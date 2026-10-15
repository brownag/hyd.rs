//! Test-only flat-ascii-v1 adapter; see SOLUTE_VERIFICATION_SPEC.md.
//! The launcher validates CASES, authorizes launches, and records the manifest/ledger.
//! Independent storage and continuous integrals belong to the metrics adapter.

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::solute::{advance_solute_step_with_processes, SoluteProcessInput};
use hydrus_core::{
    AcceptedWaterSnapshot, EquilibriumSorption, SoluteBackend, SoluteBoundaries,
    SoluteBoundaryMode as Boundary, SoluteConfig, SoluteSpecies, SoluteState, SoluteTransportInput,
    SoluteUnits, SoluteWorkspace,
};

type Result<T> = std::result::Result<T, String>;
const ENV_KEYS: [&str; 4] = [
    "HYDRUS_SOLUTE_VERIFICATION_REQUEST",
    "HYDRUS_SOLUTE_VERIFICATION_REQUEST_SHA256",
    "HYDRUS_SOLUTE_VERIFICATION_OUTPUT_ROOT",
    "HYDRUS_SOLUTE_VERIFICATION_OUT",
];
const UNITS: &str = r#"{"length":"cm","time":"hour","concentration":"C_ref","sorbed":"C_ref cm3/g","inventory":"C_ref cm","flux":"C_ref cm/hour","density":"g/cm3"}"#;

#[derive(Debug)]
struct Request {
    run_id: String,
    launch_id: String,
    case_id: String,
    cases_sha256: String,
    case_sha256: String,
    nodes: usize,
    length: f64,
    theta: f64,
    rho: f64,
    q: f64,
    dispersion: f64,
    dt: f64,
    time_end: f64,
    species: Vec<String>,
    kd: Vec<f64>,
    rates: Vec<f64>,
    inlet: Vec<f64>,
    initial: Vec<Vec<f64>>,
    steps: usize,
    print_steps: Vec<usize>,
}

fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}

fn hash_text(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn scalar(value: &str) -> Result<f64> {
    require(
        !value.is_empty()
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || b"+-.eE".contains(&b)),
        "invalid decimal token",
    )?;
    let number: f64 = value.parse().map_err(|_| "invalid number")?;
    require(number.is_finite(), "nonfinite number")?;
    Ok(number)
}

fn vector(value: &str) -> Result<Vec<f64>> {
    value.split(',').map(scalar).collect()
}

fn integer(value: &str) -> Result<usize> {
    require(
        !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
        "invalid decimal integer",
    )?;
    value.parse().map_err(|_| "integer overflow".into())
}

fn aligned_count(time: f64, dt: f64) -> Result<usize> {
    require(time > 0.0 && dt > 0.0, "nonpositive schedule value")?;
    let ratio = time / dt;
    require(
        ratio.is_finite() && (1.0..=usize::MAX as f64).contains(&ratio),
        "invalid event count",
    )?;
    // Same absolute integer-ratio tolerance as the frozen casebook checker.
    require((ratio - ratio.round()).abs() <= 1e-9, "unaligned event")?;
    Ok(ratio.round() as usize)
}

impl Request {
    fn parse(bytes: &[u8]) -> Result<Self> {
        require(
            bytes.is_ascii() && bytes.ends_with(b"\n"),
            "request must be ASCII with final newline",
        )?;
        let text = std::str::from_utf8(bytes).map_err(|_| "invalid ASCII")?;
        let mut fields = BTreeMap::new();
        for line in text.strip_suffix('\n').unwrap().split('\n') {
            require(
                !line.bytes().any(|b| b.is_ascii_whitespace()),
                "whitespace in request",
            )?;
            let (key, value) = line.split_once('=').ok_or("missing delimiter")?;
            require(
                !value.is_empty() && !value.contains('='),
                "invalid request value",
            )?;
            require(fields.insert(key, value).is_none(), "duplicate request key")?;
        }
        let get = |key: &str| {
            fields
                .get(key)
                .copied()
                .ok_or_else(|| format!("missing {key}"))
        };
        require(
            get("schema")? == "flat-ascii-v1",
            "unsupported request schema",
        )?;
        let count = integer(get("species_count")?)?;
        require((1..=3).contains(&count), "unsupported species count")?;
        let mut expected: BTreeSet<String> = "schema run_id launch_id case_id cases_sha256 case_sha256 nodes species_count length theta rho q dispersion dt time_end sampling_interval analysis_times species kd rates inlet_concentration"
            .split_whitespace().map(str::to_owned).collect();
        for i in 0..count {
            expected.insert(format!("initial_{i}"));
        }
        require(
            fields
                .keys()
                .map(|s| (*s).to_owned())
                .collect::<BTreeSet<_>>()
                == expected,
            "unknown or missing request keys",
        )?;
        for key in ["run_id", "launch_id", "case_id"] {
            require(safe_id(get(key)?), "unsafe identifier")?;
        }
        for key in ["cases_sha256", "case_sha256"] {
            require(hash_text(get(key)?), "invalid provenance hash")?;
        }
        let nodes = integer(get("nodes")?)?;
        require((3..=1001).contains(&nodes), "unsupported node count")?;
        let species: Vec<String> = get("species")?.split(',').map(str::to_owned).collect();
        require(
            species.len() == count
                && species.iter().all(|name| safe_id(name))
                && species.iter().collect::<BTreeSet<_>>().len() == count,
            "invalid species names or count",
        )?;
        let kd = vector(get("kd")?)?;
        let rates = vector(get("rates")?)?;
        let inlet = vector(get("inlet_concentration")?)?;
        for values in [&kd, &rates, &inlet] {
            require(
                values.len() == count && values.iter().all(|x| *x >= 0.0),
                "invalid species vector",
            )?;
        }
        let initial: Vec<Vec<f64>> = (0..count)
            .map(|i| vector(get(&format!("initial_{i}"))?))
            .collect::<Result<_>>()?;
        require(
            initial
                .iter()
                .all(|v| v.len() == nodes && v.iter().all(|c| *c >= 0.0)),
            "invalid initial concentrations",
        )?;
        let dt = scalar(get("dt")?)?;
        let time_end = scalar(get("time_end")?)?;
        let sampling = scalar(get("sampling_interval")?)?;
        let steps = aligned_count(time_end, dt)?;
        let sample_steps = aligned_count(sampling, dt)?;
        let prints = aligned_count(time_end, sampling)?;
        require(
            prints <= 999 && steps <= 16000,
            "schedule exceeds frozen study capacity",
        )?;
        let analysis = vector(get("analysis_times")?)?;
        let mut previous = 0.0;
        for &time in &analysis {
            require(
                time > previous && time <= time_end,
                "invalid analysis ordering/range",
            )?;
            aligned_count(time, dt)?;
            aligned_count(time, sampling)?;
            previous = time;
        }
        // The retained bridge has irregular original profile events, despite its
        // sampling_interval=50. Preserve the requested analysis times as integer
        // step events; other launches print every sampling interval.
        let print_steps = if get("launch_id")? == "benchmark-example3-bridge-rust-baseline" {
            require(
                get("case_id")? == "example3"
                    && time_end == 200.0
                    && sampling == 50.0
                    && analysis == [50.0, 100.0, 200.0],
                "bridge profile schedule differs from retained fixture",
            )?;
            analysis
                .iter()
                .map(|t| aligned_count(*t, dt))
                .collect::<Result<_>>()?
        } else {
            (1..=prints).map(|i| i * sample_steps).collect()
        };
        let request = Self {
            run_id: get("run_id")?.into(),
            launch_id: get("launch_id")?.into(),
            case_id: get("case_id")?.into(),
            cases_sha256: get("cases_sha256")?.into(),
            case_sha256: get("case_sha256")?.into(),
            nodes,
            length: scalar(get("length")?)?,
            theta: scalar(get("theta")?)?,
            rho: scalar(get("rho")?)?,
            q: scalar(get("q")?)?,
            dispersion: scalar(get("dispersion")?)?,
            dt,
            time_end,
            species,
            kd,
            rates,
            inlet,
            initial,
            steps,
            print_steps,
        };
        request.validate_mapping()?;
        Ok(request)
    }

    fn validate_mapping(&self) -> Result<()> {
        // These are the six frozen physical mappings, not adjustable solver physics.
        // The launcher checks case/launch hashes and the closed-form IC sampling.
        require(
            self.theta == 0.43 && self.rho == 0.8,
            "unsupported constant water/material",
        )?;
        type Mapping = (
            f64,
            f64,
            f64,
            &'static [&'static str],
            &'static [f64],
            &'static [f64],
            &'static [f64],
        );
        let (length, q, dispersion, names, kd, rates, inlet): Mapping = match self.case_id.as_str()
        {
            "reaction_distinct" => (
                2.0,
                0.0,
                0.0,
                &["S1", "S2", "S3"],
                &[0.5375, 0.0, 0.26875],
                &[0.2, 0.4, 0.1],
                &[0.0, 0.0, 0.0],
            ),
            "reaction_equal" => (
                2.0,
                0.0,
                0.0,
                &["S1", "S2", "S3"],
                &[0.5375, 0.0, 0.26875],
                &[0.2, 0.2, 0.0],
                &[0.0, 0.0, 0.0],
            ),
            "reaction_zero" => (
                2.0,
                0.0,
                0.0,
                &["S1", "S2", "S3"],
                &[0.5375, 0.0, 0.26875],
                &[0.0, 0.0, 0.0],
                &[0.0, 0.0, 0.0],
            ),
            "diffusion_cosine" => (2.0, 0.0, 0.2, &["S1"], &[0.5375], &[0.0], &[0.0]),
            "scalar_robin" => (2.0, 0.43, 1.0, &["S1"], &[0.26875], &[0.2], &[1.0]),
            "example3" => (
                250.0,
                0.43,
                0.18,
                &["NH4", "NO2", "NO3"],
                &[0.5375, 0.0, 0.0],
                &[0.005, 0.1, 0.0],
                &[1.0, 0.0, 0.0],
            ),
            _ => return Err("unsupported or blocked case".into()),
        };
        require(
            self.length == length
                && self.q == q
                && self.dispersion == dispersion
                && self.species == names
                && self.kd == kd
                && self.rates == rates
                && self.inlet == inlet,
            "request differs from frozen physical mapping or species order",
        )?;
        Ok(())
    }

    fn coordinates(&self) -> Vec<f64> {
        (0..self.nodes)
            .map(|i| i as f64 * self.length / (self.nodes - 1) as f64)
            .collect()
    }
}

fn half_cell_widths(coordinates: &[f64]) -> Vec<f64> {
    (0..coordinates.len())
        .map(|i| {
            let left = if i == 0 {
                coordinates[i]
            } else {
                coordinates[i - 1]
            };
            let right = if i + 1 == coordinates.len() {
                coordinates[i]
            } else {
                coordinates[i + 1]
            };
            (right - left) / 2.0
        })
        .collect()
}

fn digest(bytes: &[u8]) -> Result<String> {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("start sha256sum: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("sha256sum stdin unavailable")?
        .write_all(bytes)
        .map_err(|e| format!("hash request bytes: {e}"))?;
    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait for sha256sum: {e}"))?;
    require(output.status.success(), "sha256sum failed")?;
    let output = String::from_utf8(output.stdout).map_err(|_| "invalid sha256sum output")?;
    let hash = output
        .split_whitespace()
        .next()
        .ok_or("missing sha256sum digest")?;
    require(hash_text(hash), "invalid sha256sum digest")?;
    Ok(hash.into())
}

fn verified_request(path: &Path, expected: &str) -> Result<Request> {
    require(path.is_absolute(), "request path must be absolute")?;
    require(hash_text(expected), "invalid expected request hash")?;
    require(
        fs::metadata(path).map_err(|e| e.to_string())?.is_file(),
        "request is not a file",
    )?;
    let bytes = fs::read(path).map_err(|e| format!("read request: {e}"))?;
    require(digest(&bytes)? == expected, "request hash mismatch")?;
    Request::parse(&bytes)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn plain_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|c| matches!(c, Component::RootDir | Component::Normal(_)))
}

fn no_symlinks(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for part in path.components() {
        current.push(part.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(meta) => require(!meta.file_type().is_symlink(), "symlink in output path")?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("inspect output path: {e}")),
        }
    }
    Ok(())
}

fn output_path(root: &Path, destination: &Path) -> Result<PathBuf> {
    require(
        plain_absolute(root) && plain_absolute(destination),
        "output paths must be absolute without traversal",
    )?;
    let repo = repo_root();
    let allowed = ["13.2.2", "13.3.1", "13.4.1", "13.6.1"];
    require(
        allowed
            .iter()
            .any(|task| root.starts_with(repo.join(".development").join(task))),
        "unauthorized scratch root",
    )?;
    require(
        destination.starts_with(root)
            && destination
                .file_name()
                .is_some_and(|n| n == "observations.json"),
        "output escaped root or has wrong filename",
    )?;
    no_symlinks(root)?;
    no_symlinks(destination)?;
    match fs::symlink_metadata(destination) {
        Ok(_) => return Err("refusing existing observations".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("inspect destination: {e}")),
    }
    Ok(destination.to_path_buf())
}

fn write_observations(root: &Path, destination: &Path, json: &str) -> Result<()> {
    let destination = output_path(root, destination)?;
    fs::create_dir_all(destination.parent().ok_or("missing output parent")?)
        .map_err(|e| e.to_string())?;
    output_path(root, &destination)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|e| format!("create observations exclusively: {e}"))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("write observations: {e}"))
}

fn optional_environment(
    values: [Option<std::ffi::OsString>; 4],
) -> Result<Option<[std::ffi::OsString; 4]>> {
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    require(
        values
            .iter()
            .all(|v| v.as_ref().is_some_and(|s| !s.is_empty())),
        "all four verification environment variables are required",
    )?;
    Ok(Some(values.map(Option::unwrap)))
}

#[derive(Debug)]
struct Profile {
    time: f64,
    concentration: Vec<Vec<f64>>,
    boundary: Vec<f64>,
    reaction: Vec<f64>,
}
#[derive(Debug)]
struct Endpoint {
    time: f64,
    concentration: Vec<Vec<f64>>,
}
#[derive(Debug)]
struct Observation {
    profiles: Vec<Profile>,
    endpoints: Vec<Endpoint>,
    times: Vec<f64>,
    peclet: f64,
    courant: f64,
}

fn simulate(request: &Request) -> Result<Observation> {
    let coordinates = request.coordinates();
    let mesh = Mesh1D::new(coordinates.clone(), vec![0; request.nodes], None, 1, 0, 0.0)
        .map_err(|e| e.to_string())?;
    let species = request
        .species
        .iter()
        .enumerate()
        .map(|(i, name)| SoluteSpecies {
            name: name.clone(),
            decay_rate: request.rates[i],
            equilibrium_sorption: EquilibriumSorption::Linear {
                distribution_coefficient: request.kd[i],
            },
            kinetic_sorption: None,
        })
        .collect();
    // Mesh increases in canonical s: lower endpoint is the inlet, upper the outlet.
    let boundaries = request
        .inlet
        .iter()
        .map(|c| SoluteBoundaries {
            lower: Boundary::TotalFlux {
                flux: request.q * c,
            },
            upper: Boundary::AdvectiveOutflow,
        })
        .collect();
    let config = SoluteConfig::new_with_species_boundaries(
        SoluteUnits {
            concentration: "C_ref".into(),
            sorbed_mass: "C_ref cm3/g".into(),
        },
        species,
        boundaries,
        1,
        &[],
    )
    .map_err(|e| e.to_string())?
    .with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    let count = request.species.len();
    let mut state = SoluteState::new(
        &config,
        &mesh,
        0.0,
        request.initial.concat(),
        vec![0.0; count * request.nodes],
    )
    .map_err(|e| e.to_string())?;
    let mut workspace = SoluteWorkspace::new(request.nodes, count).map_err(|e| e.to_string())?;
    let theta = vec![request.theta; request.nodes];
    let flux = vec![request.q; request.nodes + 1];
    let dispersion = vec![request.dispersion; request.nodes - 1];
    let density = vec![request.rho; request.nodes];
    let dx = request.length / (request.nodes - 1) as f64;
    let widths = half_cell_widths(&coordinates);
    require(widths.iter().all(|w| *w > 0.0), "degenerate control volume")?;
    let mut observations = Observation {
        profiles: Vec::new(),
        endpoints: Vec::new(),
        times: Vec::new(),
        peclet: if request.q == 0.0 {
            0.0
        } else {
            request.q * dx / (request.theta * request.dispersion)
        },
        courant: request.q * request.dt / (request.theta * dx),
    };
    capture(&state, request, &mut observations, true)?;
    for step in 1..=request.steps {
        let accepted_time = step as f64 * request.dt;
        advance_solute_step_with_processes(
            &mut state,
            &mut workspace,
            &mesh,
            &config,
            SoluteTransportInput {
                accepted_water: AcceptedWaterSnapshot {
                    accepted_time,
                    water_contents: &theta,
                    face_fluxes: &flux,
                },
                start_water_contents: &theta,
                face_dispersion: &dispersion,
                timestep: request.dt,
                minimum_pivot: 1e-12,
            },
            Some(SoluteProcessInput {
                bulk_density: &density,
                max_iterations: 20,
                nonlinear_tolerance: 1e-6,
            }),
        )
        .map_err(|e| format!("step {step}: {e}"))?;
        require(
            state.time() == accepted_time && state.accounting().accepted_steps() == step as u64,
            "unexpected accepted step",
        )?;
        observations.times.push(state.time());
        capture(
            &state,
            request,
            &mut observations,
            request.print_steps.binary_search(&step).is_ok(),
        )?;
    }
    require(
        (state.time() - request.time_end).abs() <= 8.0 * f64::EPSILON * request.time_end,
        "terminal time mismatch",
    )?;
    Ok(observations)
}

fn capture(
    state: &SoluteState,
    request: &Request,
    observations: &mut Observation,
    print: bool,
) -> Result<()> {
    require(
        state.concentrations().iter().all(|c| c.is_finite()),
        "nonfinite solver concentration",
    )?;
    let concentration: Vec<Vec<f64>> = state
        .concentrations()
        .chunks_exact(request.nodes)
        .map(<[f64]>::to_vec)
        .collect();
    observations.endpoints.push(Endpoint {
        time: state.time(),
        concentration: concentration
            .iter()
            .map(|s| vec![s[0], s[request.nodes - 1]])
            .collect(),
    });
    if print {
        observations.profiles.push(Profile {
            time: state.time(),
            concentration,
            boundary: state.accounting().cumulative_boundary_mass().to_vec(),
            reaction: state.accounting().cumulative_reaction_mass().to_vec(),
        });
    }
    Ok(())
}

fn number(value: f64) -> String {
    assert!(value.is_finite(), "nonfinite observation diagnostic");
    format!("{value:.17e}")
}
fn array(values: &[f64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|v| number(*v))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn matrix(values: &[Vec<f64>]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|v| array(v))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn observation_json(request: &Request, hash: &str, observations: &Observation) -> String {
    let theta = array(&vec![request.theta; request.nodes]);
    let flux = array(&vec![request.q; request.nodes + 1]);
    let profiles = observations
        .profiles
        .iter()
        .map(|p| {
            format!(
                r#"{{"time":{},"concentration":{},"theta":{},"q_faces":{}}}"#,
                number(p.time),
                matrix(&p.concentration),
                theta,
                flux
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let steps = observations
        .times
        .iter()
        .enumerate()
        .map(|(i, t)| {
            format!(
                r#"{{"index":{},"time":{},"dt":{},"iterations":null,"peclet":{},"courant":{}}}"#,
                i + 1,
                number(*t),
                number(request.dt),
                number(observations.peclet),
                number(observations.courant)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let endpoints = observations
        .endpoints
        .iter()
        .map(|p| {
            format!(
                r#"{{"time":{},"concentration":{}}}"#,
                number(p.time),
                matrix(&p.concentration)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let reported = |boundary: bool| {
        observations
            .profiles
            .iter()
            .map(|p| {
                format!(
                    r#"{{"time":{},"values":{}}}"#,
                    number(p.time),
                    array(if boundary { &p.boundary } else { &p.reaction })
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    // Identifiers were restricted to safe ASCII; no arbitrary strings are interpolated.
    format!(concat!(
        "{{\"schema_version\":\"solute-observation-v1\",\"run_id\":\"{}\",\"launch_id\":\"{}\",\"case_id\":\"{}\",",
        "\"cases_sha256\":\"{}\",\"case_sha256\":\"{}\",\"request_sha256\":\"{}\",\"solver\":\"rust\",\"units\":{},\"species\":[{}],",
        "\"coordinates\":{},\"profiles\":[{}],\"steps\":[{}],\"endpoint_history\":[{}],",
        "\"solver_reported\":{{\"inventory\":null,\"boundary_cumulative\":[{}],\"reaction_cumulative\":[{}],\"balance\":null}},",
        "\"availability\":{{\"endpoint_history\":\"Initial and every accepted fixed step; canonical inlet,outlet order.\",",
        "\"solver_reported\":\"Core cumulative boundary and reaction mass at profile times. Inventory and cumulative balance unavailable from public accounting API; iterations unavailable. Linear equilibrium process: max_iterations=20, nonlinear_tolerance=1e-6, minimum_pivot=1e-12; backward Euler. Peclet=q*dx/(theta*D), zero at zero flow; Courant=q*dt/(theta*dx), unretarded. Independent storage and continuous integrals are computed by metrics.\",",
        "\"water\":\"Caller-supplied homogeneous theta and canonical q at all faces from hash-verified flat request; source mapping in SOLUTE_VERIFICATION_SPEC.md. These fields are prescribed, not water-solver measurements.\"}},\"warnings\":[]}}\n"),
        request.run_id, request.launch_id, request.case_id, request.cases_sha256, request.case_sha256, hash, UNITS,
        request.species.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(","),
        array(&request.coordinates()), profiles, steps, endpoints, reported(true), reported(false))
}

#[test]
fn requested_study() {
    let Some(values) =
        optional_environment(ENV_KEYS.map(env::var_os)).expect("verification environment")
    else {
        return;
    };
    let [request_path, hash, root, destination] = values;
    let hash = hash.to_str().expect("ASCII expected request hash");
    let request = verified_request(Path::new(&request_path), hash).expect("valid verified request");
    let destination =
        output_path(Path::new(&root), Path::new(&destination)).expect("confined fresh output");
    let observations = simulate(&request).expect("complete Rust verification run");
    write_observations(
        Path::new(&root),
        &destination,
        &observation_json(&request, hash, &observations),
    )
    .expect("write observations");
}

fn small_request(case: &str) -> String {
    let (length, q, dispersion, names, kd, rates, inlet, initial, time_end, dt, sampling, analysis) =
        match case {
            "reaction_zero" => (
                "2",
                "0",
                "0",
                "S1,S2,S3",
                "0.5375,0,0.26875",
                "0,0,0",
                "0,0,0",
                "initial_0=1,1,1\ninitial_1=0.25,0.25,0.25\ninitial_2=0,0,0\n",
                "4",
                "1",
                "2",
                "2,4",
            ),
            "reaction_distinct" => (
                "2",
                "0",
                "0",
                "S1,S2,S3",
                "0.5375,0,0.26875",
                "0.2,0.4,0.1",
                "0,0,0",
                "initial_0=1,1,1\ninitial_1=0,0,0\ninitial_2=0,0,0\n",
                "4",
                "1",
                "2",
                "2,4",
            ),
            "diffusion_cosine" => (
                "2",
                "0",
                "0.2",
                "S1",
                "0.5375",
                "0",
                "0",
                "initial_0=1.25,1,0.75\n",
                "1",
                "0.25",
                "0.5",
                "0.5,1",
            ),
            "scalar_robin" => (
                "2",
                "0.43",
                "1",
                "S1",
                "0.26875",
                "0.2",
                "1",
                "initial_0=1,1,1\n",
                "1",
                "0.25",
                "0.5",
                "0.5,1",
            ),
            "example3" => (
                "250",
                "0.43",
                "0.18",
                "NH4,NO2,NO3",
                "0.5375,0,0",
                "0.005,0.1,0",
                "1,0,0",
                "initial_0=0,0,0\ninitial_1=0,0,0\ninitial_2=0,0,0\n",
                "200",
                "50",
                "50",
                "50,100,200",
            ),
            _ => panic!("test case"),
        };
    format!(concat!("schema=flat-ascii-v1\nrun_id=small\nlaunch_id=test-rust\ncase_id={}\n",
        "cases_sha256={}\ncase_sha256={}\nnodes=3\nspecies_count={}\nlength={}\ntheta=0.43\nrho=0.8\nq={}\ndispersion={}\n",
        "dt={}\ntime_end={}\nsampling_interval={}\nanalysis_times={}\nspecies={}\nkd={}\nrates={}\ninlet_concentration={}\n{}"),
        case, "0".repeat(64), "1".repeat(64), names.split(',').count(), length, q, dispersion, dt, time_end, sampling, analysis,
        names, kd, rates, inlet, initial)
}

#[test]
fn strict_parser_rejects_malformed_and_nonfinite_requests() {
    let valid = small_request("reaction_zero");
    Request::parse(valid.as_bytes()).unwrap();
    let invalid = [
        valid.trim_end().to_owned(),
        format!("{valid}nodes=3\n"),
        format!("{valid}unknown=1\n"),
        valid.replace("theta=0.43\n", ""),
        valid.replace("flat-ascii-v1", "flat-ascii-v2"),
        valid.replace("theta=0.43", "theta=NaN"),
        valid.replace("theta=0.43", "theta=1e999"),
        valid.replace("theta=0.43", "theta=inf"),
        valid.replace("theta=0.43", "theta= 0.43"),
        valid.replace("theta=0.43", "theta=0.43\r"),
        valid.replace("nodes=3", "nodes=1002"),
        valid.replace("nodes=3", "nodes=+3"),
        valid.replace("nodes=3", "nodes=2"),
        valid.replace("species_count=3", "species_count=4"),
        valid.replace("run_id=small", "run_id=../small"),
        valid.replace("run_id=small", "run_id=småll"),
        valid.replace(&"0".repeat(64), &"G".repeat(64)),
        valid.replace("kd=0.5375,0,0.26875", "kd=0.5375,0"),
        valid.replace("initial_0=1,1,1", "initial_0=1,1"),
        valid.replace("initial_0=1,1,1", "initial_0=1,-1,1"),
        valid.replace("initial_0=1,1,1", "initial_0=1,NaN,1"),
        valid.replace("species=S1,S2,S3", "species=S1,S1,S3"),
    ];
    for request in invalid {
        assert!(
            Request::parse(request.as_bytes()).is_err(),
            "accepted {request}"
        );
    }
}

#[test]
fn mappings_and_order_are_explicit() {
    for case in [
        "reaction_zero",
        "reaction_distinct",
        "diffusion_cosine",
        "scalar_robin",
        "example3",
    ] {
        Request::parse(small_request(case).as_bytes()).unwrap();
    }
    let request = small_request("reaction_zero");
    for invalid in [
        request.replace("reaction_zero", "manufactured_forcing"),
        request.replace("reaction_zero", "unknown"),
        request.replace("species=S1,S2,S3", "species=S2,S1,S3"),
        request.replace("q=0\n", "q=-0.43\n"),
        request.replace("theta=0.43", "theta=0.4"),
        request.replace("rho=0.8", "rho=0.9"),
        request.replace("rates=0,0,0", "rates=0.2,0,0"),
        request.replace("length=2", "length=3"),
    ] {
        assert!(Request::parse(invalid.as_bytes()).is_err());
    }
}

#[test]
fn schedules_reject_shortened_steps_and_missing_events() {
    let request = small_request("reaction_zero");
    for invalid in [
        request.replace("dt=1\n", "dt=0\n"),
        request.replace("dt=1\n", "dt=-1\n"),
        request.replace("dt=1\n", "dt=0.3\n"),
        request.replace("sampling_interval=2", "sampling_interval=1.5"),
        request.replace("time_end=4", "time_end=5"),
        request.replace("analysis_times=2,4", "analysis_times=1,4"),
        request.replace("analysis_times=2,4", "analysis_times=4,2"),
        request.replace("analysis_times=2,4", "analysis_times=2,2"),
        request.replace("analysis_times=2,4", "analysis_times=2,6"),
        request.replace("analysis_times=2,4", "analysis_times=0,4"),
        request.replace("dt=1\n", "dt=0.00000001\n"),
    ] {
        assert!(
            Request::parse(invalid.as_bytes()).is_err(),
            "accepted {invalid}"
        );
    }
    assert_eq!(aligned_count(200.0, 0.1).unwrap(), 2000);
    assert_eq!(aligned_count(200.0, 0.0125).unwrap(), 16000);
    assert!(aligned_count(1.0, 0.3).is_err());
}

#[test]
fn bridge_keeps_original_profile_events_without_dropping_step_endpoints() {
    let ordinary = small_request("example3");
    let bridge = ordinary.replace(
        "launch_id=test-rust",
        "launch_id=benchmark-example3-bridge-rust-baseline",
    );
    let request = Request::parse(bridge.as_bytes()).unwrap();
    // Four steps on three nodes exercise event selection without a study launch.
    assert_eq!(request.print_steps, [1, 2, 4]);
    let observations = simulate(&request).unwrap();
    assert_eq!(
        observations
            .profiles
            .iter()
            .map(|p| p.time)
            .collect::<Vec<_>>(),
        [0.0, 50.0, 100.0, 200.0]
    );
    assert_eq!(
        observations
            .endpoints
            .iter()
            .map(|p| p.time)
            .collect::<Vec<_>>(),
        [0.0, 50.0, 100.0, 150.0, 200.0]
    );
    assert_eq!(
        Request::parse(ordinary.as_bytes()).unwrap().print_steps,
        [1, 2, 3, 4]
    );
    // Validate the actual bridge's event indices without executing its 2,000 steps.
    let actual_dt = Request::parse(bridge.replace("dt=50\n", "dt=0.1\n").as_bytes()).unwrap();
    assert_eq!(actual_dt.print_steps, [500, 1000, 2000]);
    assert!(Request::parse(
        bridge
            .replace("analysis_times=50,100,200", "analysis_times=50,100,150,200")
            .as_bytes()
    )
    .is_err());
}

#[test]
fn canonical_coordinates_and_half_cell_weights() {
    let request = Request::parse(small_request("example3").as_bytes()).unwrap();
    let coordinates = request.coordinates();
    assert_eq!(coordinates, [0.0, 125.0, 250.0]);
    assert_eq!(half_cell_widths(&coordinates), [62.5, 125.0, 62.5]);
    let weights = half_cell_widths(&[0.0, 0.25, 0.5, 0.75, 1.0]);
    assert_eq!(weights, [0.125, 0.25, 0.25, 0.25, 0.125]);
    assert_eq!(weights.iter().sum::<f64>(), 1.0);
}

#[test]
fn closed_zero_rate_preserves_every_species_and_reports_full_coverage() {
    let request = Request::parse(small_request("reaction_zero").as_bytes()).unwrap();
    let observation = simulate(&request).unwrap();
    assert_eq!(observation.times, [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(
        observation
            .profiles
            .iter()
            .map(|p| p.time)
            .collect::<Vec<_>>(),
        [0.0, 2.0, 4.0]
    );
    assert_eq!(observation.endpoints.len(), 5);
    for profile in &observation.profiles {
        assert_eq!(profile.concentration, request.initial);
        assert_eq!(profile.boundary, [0.0; 3]);
        assert_eq!(profile.reaction, [0.0; 3]);
    }
    assert_eq!(observation.peclet, 0.0);
    assert_eq!(observation.courant, 0.0);
}

#[test]
fn reactions_follow_declared_order_and_unequal_storage() {
    let request = Request::parse(small_request("reaction_distinct").as_bytes()).unwrap();
    let observation = simulate(&request).unwrap();
    let masses = [
        0.86 / 1.2,
        (0.2 * (0.86 / 1.2)) / 1.4,
        (0.4 * ((0.2 * (0.86 / 1.2)) / 1.4)) / 1.1,
    ];
    let storage = [0.86, 0.43, 0.645];
    for i in 0..3 {
        let expected = masses[i] / storage[i];
        assert!((observation.endpoints[1].concentration[i][0] - expected).abs() < 1e-14);
        assert!((observation.endpoints[1].concentration[i][1] - expected).abs() < 1e-14);
    }
}

#[test]
fn parent_only_inlet_and_positive_orientation_drive_the_correct_endpoint() {
    let request = Request::parse(small_request("example3").as_bytes()).unwrap();
    let observation = simulate(&request).unwrap();
    let last = observation.profiles.last().unwrap();
    for species in &last.concentration {
        assert!(species[0] > species[2] && species[2] > 0.0);
    }
    assert!(last.boundary[0] > 0.0);
    assert!(last.boundary[1] < 0.0 && last.boundary[2] < 0.0);
    assert_eq!(
        observation.endpoints[0].concentration,
        vec![vec![0.0; 2]; 3]
    );
    assert_eq!(
        observation.endpoints.last().unwrap().concentration[0],
        [last.concentration[0][0], last.concentration[0][2]]
    );
}

#[test]
fn diffusion_moves_mass_without_boundary_exchange() {
    let request = Request::parse(small_request("diffusion_cosine").as_bytes()).unwrap();
    let observation = simulate(&request).unwrap();
    let last = observation.profiles.last().unwrap();
    assert!(last.concentration[0][0] < 1.25 && last.concentration[0][2] > 0.75);
    assert!(last.boundary[0].abs() < 1e-14);
    let integral: f64 = half_cell_widths(&request.coordinates())
        .iter()
        .zip(&last.concentration[0])
        .map(|(w, c)| w * c)
        .sum();
    assert!((integral - 2.0).abs() < 1e-14);
}

#[test]
fn observation_json_has_frozen_keys_and_round_trip_values() {
    let request = Request::parse(small_request("reaction_distinct").as_bytes()).unwrap();
    let observation = simulate(&request).unwrap();
    let json = observation_json(&request, &"a".repeat(64), &observation);
    // Standard-library Python is available to the study tooling; it independently
    // parses generated JSON instead of testing string fragments as a schema proxy.
    let script = r#"
import json,sys
x=json.load(sys.stdin)
assert set(x)==set('schema_version run_id launch_id case_id cases_sha256 case_sha256 request_sha256 solver units species coordinates profiles steps endpoint_history solver_reported availability warnings'.split())
assert x['schema_version']=='solute-observation-v1' and x['solver']=='rust'
assert x['request_sha256']=='a'*64
assert x['species']==['S1','S2','S3'] and x['coordinates']==[0,1,2]
assert set(x['units'])==set('length time concentration sorbed inventory flux density'.split())
assert [p['time'] for p in x['profiles']]==[0,2,4]
for p in x['profiles']:
    assert set(p)==set('time concentration theta q_faces'.split())
    assert len(p['concentration'])==3 and all(len(s)==3 for s in p['concentration'])
    assert p['theta']==[.43]*3 and p['q_faces']==[0]*4
assert len(x['steps'])==4 and len(x['endpoint_history'])==5
for i,s in enumerate(x['steps'],1):
    assert set(s)==set('index time dt iterations peclet courant'.split())
    assert s==dict(index=i,time=i,dt=1,iterations=None,peclet=0,courant=0)
for e in x['endpoint_history']:
    assert set(e)=={'time','concentration'} and all(len(v)==2 for v in e['concentration'])
assert set(x['solver_reported'])==set('inventory boundary_cumulative reaction_cumulative balance'.split())
assert x['solver_reported']['inventory'] is None and x['solver_reported']['balance'] is None
for key in ('boundary_cumulative','reaction_cumulative'):
    assert len(x['solver_reported'][key])==3
    assert all(set(r)=={'time','values'} and len(r['values'])==3 for r in x['solver_reported'][key])
assert set(x['availability'])=={'endpoint_history','solver_reported','water'}
assert all(x['availability'].values()) and 'iterations unavailable' in x['availability']['solver_reported']
assert x['warnings']==[]
"#;
    let mut child = Command::new("python3")
        .args(["-c", script])
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(json.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
}

struct Scratch(PathBuf);
impl Scratch {
    fn new(label: &str) -> Self {
        let path = repo_root().join(".development/13.2.2").join(format!(
            "test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn request_hash_is_verified_before_parsing() {
    assert_eq!(
        digest(b"abc").unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let scratch = Scratch::new("hash");
    let path = scratch.0.join("request.txt");
    let bytes = small_request("reaction_zero");
    fs::write(&path, &bytes).unwrap();
    let expected = digest(bytes.as_bytes()).unwrap();
    verified_request(&path, &expected).unwrap();
    assert!(verified_request(&path, &"0".repeat(64)).is_err());
    assert!(verified_request(Path::new("relative.txt"), &expected).is_err());
    assert!(verified_request(&path, "wrong").is_err());
    fs::write(&path, bytes.replace("run_id=small", "run_id=changed")).unwrap();
    assert!(verified_request(&path, &expected).is_err());
}

#[test]
fn output_confinement_and_exclusive_creation() {
    let scratch = Scratch::new("paths");
    let output = scratch.0.join("nested/observations.json");
    output_path(&scratch.0, &output).unwrap();
    for invalid in [
        scratch.0.join("../observations.json"),
        repo_root().join("observations.json"),
        scratch.0.join("other.json"),
    ] {
        assert!(output_path(&scratch.0, &invalid).is_err());
    }
    assert!(output_path(Path::new("relative"), &output).is_err());
    assert!(output_path(&repo_root().join(".development/12.6.1"), &output).is_err());
    assert!(output_path(&repo_root().join(".development/13.2.2-escape"), &output).is_err());
    write_observations(&scratch.0, &output, "original").unwrap();
    assert!(write_observations(&scratch.0, &output, "replacement").is_err());
    assert_eq!(fs::read_to_string(output).unwrap(), "original");
}

#[cfg(unix)]
#[test]
fn output_rejects_symlinks_and_dangling_symlinks() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new("symlinks");
    let link = scratch.0.join("link");
    symlink(repo_root(), &link).unwrap();
    assert!(output_path(&scratch.0, &link.join("observations.json")).is_err());
    assert!(output_path(&link, &link.join("observations.json")).is_err());
    let dangling = scratch.0.join("observations.json");
    symlink(scratch.0.join("missing"), &dangling).unwrap();
    assert!(output_path(&scratch.0, &dangling).is_err());
}

#[test]
fn environment_is_all_or_none_without_global_mutation() {
    assert!(optional_environment([None, None, None, None])
        .unwrap()
        .is_none());
    for i in 0..4 {
        let mut variables = [None, None, None, None];
        variables[i] = Some("value".into());
        assert!(optional_environment(variables).is_err());
    }
    assert!(
        optional_environment(std::array::from_fn(|_| Some("value".into())))
            .unwrap()
            .is_some()
    );
    assert!(optional_environment(std::array::from_fn(|_| Some("".into()))).is_err());
}

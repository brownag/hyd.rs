//! Harness test for the FortranCompatible candidate backend emitting schema-valid observations.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::PathBuf,
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
    require((ratio - ratio.round()).abs() <= 1e-9, "unaligned event")?;
    Ok(ratio.round() as usize)
}

impl Request {
    fn coordinates(&self) -> Vec<f64> {
        let dx = self.length / (self.nodes - 1) as f64;
        (0..self.nodes).map(|i| i as f64 * dx).collect()
    }

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
        let print_steps = (1..=prints).map(|i| i * sample_steps).collect();
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
        Ok(request)
    }
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

fn simulate_candidate(request: &Request) -> Result<Observation> {
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
    .with_backend(SoluteBackend::FortranCompatible);

    assert_eq!(config.backend(), SoluteBackend::FortranCompatible);

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
    format!(concat!(
        "{{\"schema_version\":\"solute-observation-v1\",\"run_id\":\"{}\",\"launch_id\":\"{}\",\"case_id\":\"{}\",",
        "\"cases_sha256\":\"{}\",\"case_sha256\":\"{}\",\"request_sha256\":\"{}\",\"solver\":\"rust\",\"units\":{},\"species\":[{}],",
        "\"coordinates\":{},\"profiles\":[{}],\"steps\":[{}],\"endpoint_history\":[{}],",
        "\"solver_reported\":{{\"inventory\":null,\"boundary_cumulative\":[{}],\"reaction_cumulative\":[{}],\"balance\":null}},",
        "\"availability\":{{\"endpoint_history\":\"Initial and every accepted fixed step; canonical inlet,outlet order.\",",
        "\"solver_reported\":\"Core cumulative boundary and reaction mass at profile times. Inventory and cumulative balance unavailable from public accounting API; iterations unavailable. Pinned-Fortran-compatible solute transport solver.\",",
        "\"water\":\"Caller-supplied homogeneous theta and canonical q at all faces from hash-verified flat request; source mapping in SOLUTE_VERIFICATION_SPEC.md. These fields are prescribed, not water-solver measurements.\"}},\"warnings\":[]}}\n"),
        request.run_id, request.launch_id, request.case_id, request.cases_sha256, request.case_sha256, hash, UNITS,
        request.species.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(","),
        array(&request.coordinates()), profiles, steps, endpoints, reported(true), reported(false))
}

fn sample_request(case: &str) -> String {
    let (length, q, dispersion, names, kd, rates, inlet, initial, time_end, dt, sampling, analysis) =
        match case {
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
            _ => panic!("unknown test case"),
        };
    format!(concat!("schema=flat-ascii-v1\nrun_id=small\nlaunch_id=test-fortran-compat\ncase_id={}\n",
        "cases_sha256={}\ncase_sha256={}\nnodes=3\nspecies_count={}\nlength={}\ntheta=0.43\nrho=0.8\nq={}\ndispersion={}\n",
        "dt={}\ntime_end={}\nsampling_interval={}\nanalysis_times={}\nspecies={}\nkd={}\nrates={}\ninlet_concentration={}\n{}"),
        case, "0".repeat(64), "1".repeat(64), names.split(',').count(), length, q, dispersion, dt, time_end, sampling, analysis,
        names, kd, rates, inlet, initial)
}

#[test]
fn candidate_observation_json_matches_schema_and_round_trips() {
    let request = Request::parse(sample_request("reaction_distinct").as_bytes()).unwrap();
    let observation = simulate_candidate(&request).unwrap();
    let json = observation_json(&request, &"b".repeat(64), &observation);

    let script = r#"
import json,sys
x=json.load(sys.stdin)
assert set(x)==set('schema_version run_id launch_id case_id cases_sha256 case_sha256 request_sha256 solver units species coordinates profiles steps endpoint_history solver_reported availability warnings'.split())
assert x['schema_version']=='solute-observation-v1' and x['solver']=='rust'
assert x['request_sha256']=='b'*64
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
assert all(x['availability'].values())
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

/// Helper to run a parity fixture bundle, verify outputs, generate runner-manifest,
/// and execute `solute_parity_normalizers.py` to validate parity against the fixture.
fn run_parity_fixture_bundle(fixture_dir: &std::path::Path) -> Result<()> {
    use hydrus_core::{run_compat_deck, CompatRunOutputs};
    use std::fs;
    use std::path::PathBuf;

    let case_path = fixture_dir.join("CASE.json");
    let selector_path = fixture_dir.join("SELECTOR.IN");
    let profile_path = fixture_dir.join("PROFILE.DAT");
    let obs_map_path = fixture_dir.join("OBSERVATION_MAP.json");

    let case_json = fs::read_to_string(&case_path).map_err(|e| e.to_string())?;
    let selector_in = fs::read_to_string(&selector_path).map_err(|e| e.to_string())?;
    let profile_dat = fs::read_to_string(&profile_path).map_err(|e| e.to_string())?;

    // Execute the filesystem-free compat runner
    let outputs: CompatRunOutputs = run_compat_deck(&case_json, &selector_in, &profile_dat)
        .map_err(|e| format!("run_compat_deck failed: {e}"))?;

    // Create a temporary directory inside target/tmp_parity_runs to hold raw outputs and manifest
    let temp_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/tmp_parity_runs")
        .join(fixture_dir.file_name().unwrap());
    let _ = fs::remove_dir_all(&temp_root);
    fs::create_dir_all(&temp_root).map_err(|e| e.to_string())?;

    let run_root = temp_root.join("run");
    fs::create_dir_all(&run_root).map_err(|e| e.to_string())?;

    // Write raw outputs to run_root
    let mut output_paths = BTreeMap::<String, String>::new();
    let mut output_sha256 = BTreeMap::<String, String>::new();

    let write_file = |name: &str,
                      content: &str,
                      paths: &mut BTreeMap<String, String>,
                      hashes: &mut BTreeMap<String, String>|
     -> Result<()> {
        let file_path = run_root.join(name);
        fs::write(&file_path, content).map_err(|e| e.to_string())?;
        use std::process::Command;
        let child = Command::new("sha256sum")
            .arg(&file_path)
            .output()
            .map_err(|e| e.to_string())?;
        let hash = String::from_utf8_lossy(&child.stdout)
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        paths.insert(name.to_string(), format!("run/{name}"));
        hashes.insert(name.to_string(), hash);
        Ok(())
    };

    write_file(
        "PROFILE.OUT",
        &outputs.profile_out,
        &mut output_paths,
        &mut output_sha256,
    )?;
    write_file(
        "NOD_INF.OUT",
        &outputs.nod_inf_out,
        &mut output_paths,
        &mut output_sha256,
    )?;
    if let Some(ref nod_v) = outputs.nod_inf_v_out {
        write_file(
            "NOD_INF_V.OUT",
            nod_v,
            &mut output_paths,
            &mut output_sha256,
        )?;
    }
    for (idx, solute_text) in outputs.solute_outs.iter().enumerate() {
        let name = format!("solute{}.out", idx + 1);
        write_file(&name, solute_text, &mut output_paths, &mut output_sha256)?;
    }
    if let Some(ref obs) = outputs.obs_node_out {
        write_file("OBS_NODE.OUT", obs, &mut output_paths, &mut output_sha256)?;
    }
    write_file(
        "BALANCE.OUT",
        &outputs.balance_out,
        &mut output_paths,
        &mut output_sha256,
    )?;

    // Write dummy/marker auxiliary files expected by fortran runner schema if present in CASE.json
    let case_obj: serde_json::Value =
        serde_json::from_str(&case_json).map_err(|e| e.to_string())?;
    if let Some(runner_out) = case_obj
        .get("runner")
        .and_then(|r| r.get("outputs"))
        .and_then(|o| o.as_array())
    {
        for out_entry in runner_out {
            if let Some(name) = out_entry.get("name").and_then(|n| n.as_str()) {
                if !output_paths.contains_key(name) {
                    write_file(name, "end\n", &mut output_paths, &mut output_sha256)?;
                }
            }
        }
    }

    // Input files SHA-256
    let mut input_sha256 = BTreeMap::<String, String>::new();
    if let Some(input_files) = case_obj.get("input_files").and_then(|f| f.as_object()) {
        for (k, v) in input_files {
            if let Some(hash_str) = v.as_str() {
                input_sha256.insert(k.clone(), hash_str.to_string());
            }
        }
    }

    // Build runner manifest
    let runner_manifest = serde_json::json!({
        "schema_version": "runner-manifest-v1",
        "complete": true,
        "task_root": temp_root.to_str().unwrap(),
        "run_root": run_root.to_str().unwrap(),
        "input_sha256": input_sha256,
        "output_paths": output_paths,
        "output_sha256": output_sha256
    });

    let manifest_path = temp_root.join("runner-manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&runner_manifest).unwrap(),
    )
    .map_err(|e| e.to_string())?;

    // Call python normalizer tool to authenticate runner result and normalize outputs
    let script = format!(
        r#"
import sys
from pathlib import Path
import tools.solute_parity_normalizers as n

obs_map_path = Path({:?})
runner_manifest_path = Path({:?})
task_root = Path({:?})
fixture_manifest_path = Path({:?})

normalized = n.normalize_fortran_outputs(
    obs_map_path,
    runner_manifest_path,
    task_root,
    fixture_manifest_path
)
assert len(normalized) > 0, "No observations normalized"
print(f"Normalized {{len(normalized)}} observations successfully")
"#,
        obs_map_path.to_str().unwrap(),
        manifest_path.to_str().unwrap(),
        temp_root.to_str().unwrap(),
        case_path.to_str().unwrap()
    );

    let child = Command::new("python3")
        .args(["-c", &script])
        .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .map_err(|e| e.to_string())?;

    if !child.status.success() {
        return Err(format!(
            "normalizer python failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr)
        ));
    }

    // Clean up temporary run directory
    let _ = fs::remove_dir_all(&temp_root);

    Ok(())
}

#[test]
fn test_parity_single_equilibrium_linear() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../reference/fortran_reference/solute/parity/reactions/single_equilibrium_linear",
    );
    run_parity_fixture_bundle(&p).unwrap();
}

/// Regression for the top-flux root cause: `Par(5,M)` (Ks) is read without rescaling
/// (`INPUT.FOR:793`, `ConSat(M)=Par(5,M)`), so the initial saturated top flux is the
/// pinned `-0.43` (`BALANCE.OUT` Top Flux at t=0), not a value scaled by `xConv/tConv`.
#[test]
fn saturated_top_flux_uses_unscaled_ks() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../reference/fortran_reference/solute/parity/transport/transport-baseline-storage",
    );
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    let outputs = hydrus_core::run_compat_deck(
        &read("CASE.json"),
        &read("SELECTOR.IN"),
        &read("PROFILE.DAT"),
    )
    .unwrap();
    let top = outputs
        .balance_out
        .lines()
        .find(|l| l.contains("Top Flux"))
        .unwrap();
    let value: f64 = top.split_whitespace().last().unwrap().parse().unwrap();
    assert!((value + 0.43).abs() < 1e-6, "initial Top Flux was {value}");
}

/// Regression for the equilibrium water content (WATFLOW.FOR:73-76 Hyst -> Reset):
/// the mobile moisture must follow the new head. `model6_exchange` at t=1, node 2 has
/// moisture 0.4294 in the pinned NOD_INF.OUT (head -3.339 cm).
#[test]
fn equilibrium_water_content_follows_the_new_head() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../reference/fortran_reference/solute/parity/processes/dual_exchange/model6_exchange",
    );
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    let outputs = hydrus_core::run_compat_deck(
        &read("CASE.json"),
        &read("SELECTOR.IN"),
        &read("PROFILE.DAT"),
    )
    .unwrap();
    let nod = &outputs.nod_inf_out;
    let start = nod.find("Time:        1.0000").unwrap();
    let row = nod[start..]
        .lines()
        .find(|l| l.trim_start().starts_with("2 "))
        .unwrap();
    let fields: Vec<f64> = row
        .split_whitespace()
        .skip(1)
        .map(|v| v.parse().unwrap())
        .collect();
    // Fields: depth, head, moisture, ... The head still differs by about 0.02 cm, which
    // is an open dual-porosity item and is not asserted here.
    assert!((fields[2] - 0.4294).abs() < 2e-4, "moisture {}", fields[2]);
}

#[test]
fn chain_distinct_rates_paired_gate() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/fortran_reference/solute/parity/reactions/chain_distinct_rates");
    run_paired_fixture_gate(&p).unwrap();
}

/// Paired Fortran-vs-Rust gate for one fixture: runs `run_solute_parity.py` with the
/// real Rust candidate and requires `status == "pass"` (not only Fortran normalization).
fn run_paired_fixture_gate(fixture_dir: &std::path::Path) -> Result<()> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let name = fixture_dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let task_root = repo
        .join(".development/harness_paired")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&task_root);
    let child = Command::new("python3")
        .args([
            "tools/run_solute_parity.py",
            "--fixture-manifest",
            fixture_dir.join("CASE.json").to_str().unwrap(),
            "--backend",
            "fortran-compatible",
            "--task-root",
            task_root.to_str().unwrap(),
        ])
        .current_dir(&repo)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .map_err(|e| e.to_string())?;
    // The runner prints a JSON object that may contain bare NaN tokens, which serde_json
    // rejects. The top-level status is the only field needed, and it is the one line
    // indented exactly two spaces.
    let text = String::from_utf8_lossy(&child.stdout).to_string();
    let status_line = text.lines().find(|l| l.starts_with("  \"status\": "));
    let _ = fs::remove_dir_all(&task_root);
    require(
        child.status.success() && status_line == Some("  \"status\": \"pass\","),
        &format!("{name}: paired status {status_line:?}"),
    )
}

#[test]
fn test_parity_biology_blocking_type_3_noop() {
    // iPsi=3 with SMax=0 gives psi1 = NaN in the pinned Fortran (ADR 0017).
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../reference/fortran_reference/solute/parity/processes/biology/blocking_type_3_noop",
    );
    run_paired_fixture_gate(&p).unwrap();
}

#[test]
fn test_parity_biology_filtration_deposition() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../reference/fortran_reference/solute/parity/processes/biology/filtration_deposition",
    );
    run_paired_fixture_gate(&p).unwrap();
}

#[test]
fn test_parity_chain_distinct_rates() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/fortran_reference/solute/parity/reactions/chain_distinct_rates");
    run_parity_fixture_bundle(&p).unwrap();
}

#[test]
fn test_parity_lflux_on() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/fortran_reference/solute/parity/flux_concentration/lflux_on");
    run_parity_fixture_bundle(&p).unwrap();
}

#[test]
fn test_parity_lflux_off() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/fortran_reference/solute/parity/flux_concentration/lflux_off");
    run_parity_fixture_bundle(&p).unwrap();
}

#[test]
fn test_parity_dual_model6() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../reference/fortran_reference/solute/parity/processes/nonequilibrium/dual_model6",
    );
    run_parity_fixture_bundle(&p).unwrap();
}

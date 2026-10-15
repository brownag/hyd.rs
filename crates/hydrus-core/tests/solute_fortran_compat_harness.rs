//! Harness test for the FortranCompatible candidate backend emitting schema-valid observations.
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
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

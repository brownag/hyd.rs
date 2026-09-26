use std::{env, fs, path::PathBuf};

use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::solute::{advance_solute_step_with_processes, SoluteProcessInput};
use hydrus_core::{
    AcceptedWaterSnapshot, EquilibriumSorption, SoluteBoundaries, SoluteBoundaryMode as Boundary,
    SoluteConfig, SoluteSpecies, SoluteState, SoluteTransportInput, SoluteUnits, SoluteWorkspace,
};

const NODE_COUNT: usize = 1001;
const SPECIES_COUNT: usize = 3;
const STEP_COUNT: usize = 2000;
const DT: f64 = 0.1;
const THETA: f64 = 0.43;
const WATER_FLUX: f64 = -0.43;
const DISPERSION: f64 = 0.18;
const BULK_DENSITY: f64 = 0.8;
const NAMES: [&str; SPECIES_COUNT] = ["NH4", "NO2", "NO3"];
const RATES: [f64; SPECIES_COUNT] = [0.005, 0.1, 0.0];
const KD: [f64; SPECIES_COUNT] = [0.5375, 0.0, 0.0];

#[derive(Clone)]
struct Observation {
    nominal_time: f64,
    actual_time: f64,
    concentrations: Vec<Vec<f64>>,
    sorbed: Vec<Vec<f64>>,
    inventories: Vec<f64>,
    boundary_mass: Vec<f64>,
    reaction_mass: Vec<f64>,
    cumulative_balance_residual: Vec<f64>,
}

fn close(actual: f64, expected: f64, scale: f64, label: &str) {
    let tolerance = 512.0 * f64::EPSILON * scale.max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: {actual:.17e} != {expected:.17e} (tol {tolerance:.3e})"
    );
}

fn widths() -> Vec<f64> {
    (0..NODE_COUNT)
        .map(|node| {
            if node == 0 || node + 1 == NODE_COUNT {
                0.125
            } else {
                0.25
            }
        })
        .collect()
}

fn inventory(concentrations: &[f64], species: usize, widths: &[f64]) -> f64 {
    let offset = species * NODE_COUNT;
    (0..NODE_COUNT)
        .map(|node| {
            widths[node] * (THETA + BULK_DENSITY * KD[species]) * concentrations[offset + node]
        })
        .sum()
}

fn resolved_output_path() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("hydrus-core must remain inside the workspace")
        .to_path_buf();
    let scratch = repo_root.join(".development/12.6.1");
    let requested = env::var_os("HYDRUS_SOLUTE_PARITY_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| scratch.join("rust-observations.json"));
    let output = if requested.is_absolute() {
        requested
    } else {
        repo_root.join(requested)
    };
    fs::create_dir_all(&scratch).expect("create comparison scratch directory");
    let scratch = scratch.canonicalize().expect("resolve comparison scratch");
    assert!(
        output.starts_with(&scratch) || output.starts_with(repo_root.join(".development/12.6.1")),
        "Rust observations must resolve under {}",
        scratch.display()
    );
    let parent = output
        .parent()
        .expect("Rust observation path must have a parent");
    let mut existing = parent;
    while !existing.exists() {
        existing = existing
            .parent()
            .expect("observation parent has existing ancestor");
    }
    assert!(
        existing
            .canonicalize()
            .expect("resolve existing output ancestor")
            .starts_with(&scratch),
        "Rust observation path traverses outside scratch"
    );
    fs::create_dir_all(parent).expect("create Rust observation parent");
    let parent = parent
        .canonicalize()
        .expect("resolve Rust observation parent");
    assert!(
        parent.starts_with(&scratch),
        "Rust observations must stay under {}",
        scratch.display()
    );
    let destination = parent.join(output.file_name().expect("Rust observation filename"));
    match fs::symlink_metadata(&destination) {
        Ok(metadata) => assert!(
            !metadata.file_type().is_symlink(),
            "Rust observation destination may not be a symlink"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("inspect observation destination: {error}"),
    }
    destination
}

fn number(value: f64) -> String {
    assert!(value.is_finite(), "nonfinite value in Rust observations");
    format!("{value:.17e}")
}

fn number_array(values: &[f64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| number(*value))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn observation_json(observations: &[Observation]) -> String {
    let rows = observations
        .iter()
        .map(|observation| {
            let species = (0..SPECIES_COUNT)
                .map(|index| {
                    format!(
                        "{{\"name\":\"{}\",\"concentration\":{},\"equilibrium_sorbed\":{},\
                         \"inventory\":{},\"cumulative_boundary_mass\":{},\
                         \"cumulative_reaction_mass\":{},\"cumulative_balance_residual\":{}}}",
                        NAMES[index],
                        number_array(&observation.concentrations[index]),
                        number_array(&observation.sorbed[index]),
                        number(observation.inventories[index]),
                        number(observation.boundary_mass[index]),
                        number(observation.reaction_mass[index]),
                        number(observation.cumulative_balance_residual[index]),
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"time_h\":{},\"actual_time_h\":{},\"species\":[{}]}}",
                number(observation.nominal_time),
                number(observation.actual_time),
                species
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema_version\":1,\"units\":{{\"length\":\"cm\",\"time\":\"hour\",\"concentration\":\"dimensionless normalized concentration\"}},\
         \"node_count\":{},\"species_count\":{},\
         \"coordinate_start_cm\":-250.0,\"coordinate_spacing_cm\":0.25,\
         \"time_step_h\":{},\"step_count\":{},\"bulk_density\":{},\
         \"species_metadata\":[{}],\"upper_total_flux_by_species\":[0.43,0.0,0.0],\
         \"lower_boundary\":\"AdvectiveOutflow\",\"water_content\":{},\
         \"face_flux\":{},\"face_dispersion\":{},\"observations\":[{}]}}\n",
        NODE_COUNT,
        SPECIES_COUNT,
        number(DT),
        STEP_COUNT,
        number(BULK_DENSITY),
        (0..SPECIES_COUNT).map(|i| format!("{{\"name\":\"{}\",\"decay_rate\":{},\"kd\":{}}}", NAMES[i], number(RATES[i]), number(KD[i]))).collect::<Vec<_>>().join(","),
        number(THETA),
        number(WATER_FLUX),
        number(DISPERSION),
        rows
    )
}

#[test]
fn source_mapped_example3_runs_and_emits_independently_balanced_observations() {
    let coordinates: Vec<f64> = (0..NODE_COUNT)
        .map(|node| -250.0 + 0.25 * node as f64)
        .collect();
    let mesh = Mesh1D::new(coordinates, vec![0; NODE_COUNT], None, 1, 0, 0.0).unwrap();
    let species = NAMES
        .iter()
        .zip(RATES)
        .zip(KD)
        .map(|((name, decay_rate), kd)| SoluteSpecies {
            name: (*name).to_owned(),
            decay_rate,
            equilibrium_sorption: EquilibriumSorption::Linear {
                distribution_coefficient: kd,
            },
            kinetic_sorption: None,
        })
        .collect();
    let boundaries = vec![
        SoluteBoundaries {
            lower: Boundary::AdvectiveOutflow,
            upper: Boundary::TotalFlux { flux: 0.43 },
        },
        SoluteBoundaries {
            lower: Boundary::AdvectiveOutflow,
            upper: Boundary::TotalFlux { flux: 0.0 },
        },
        SoluteBoundaries {
            lower: Boundary::AdvectiveOutflow,
            upper: Boundary::TotalFlux { flux: 0.0 },
        },
    ];
    let config = SoluteConfig::new_with_species_boundaries(
        SoluteUnits {
            concentration: "normalized concentration".into(),
            sorbed_mass: "normalized concentration mass per soil mass".into(),
        },
        species,
        boundaries,
        1,
        &[],
    )
    .unwrap();
    let mut state = SoluteState::new(
        &config,
        &mesh,
        0.0,
        vec![0.0; NODE_COUNT * SPECIES_COUNT],
        vec![0.0; NODE_COUNT * SPECIES_COUNT],
    )
    .unwrap();
    let mut workspace = SoluteWorkspace::new(NODE_COUNT, SPECIES_COUNT).unwrap();
    let water_contents = vec![THETA; NODE_COUNT];
    let face_fluxes = vec![WATER_FLUX; NODE_COUNT + 1];
    let face_dispersion = vec![DISPERSION; NODE_COUNT - 1];
    let bulk_density = vec![BULK_DENSITY; NODE_COUNT];
    let control_widths = widths();
    let mut cumulative_boundary = vec![0.0; SPECIES_COUNT];
    let mut cumulative_reaction = vec![0.0; SPECIES_COUNT];
    let mut old_mass = vec![0.0; NODE_COUNT * SPECIES_COUNT];
    let mut observations = Vec::with_capacity(4);

    let capture = |nominal_time: f64,
                   actual_time: f64,
                   state: &SoluteState,
                   cumulative_boundary: &[f64],
                   cumulative_reaction: &[f64],
                   observations: &mut Vec<Observation>| {
        let concentrations = (0..SPECIES_COUNT)
            .map(|species| {
                let start = species * NODE_COUNT;
                state.concentrations()[start..start + NODE_COUNT].to_vec()
            })
            .collect::<Vec<_>>();
        let sorbed = concentrations
            .iter()
            .enumerate()
            .map(|(species, values)| values.iter().map(|value| KD[species] * value).collect())
            .collect::<Vec<Vec<f64>>>();
        let inventories: Vec<f64> = (0..SPECIES_COUNT)
            .map(|species| inventory(state.concentrations(), species, &control_widths))
            .collect();
        let cumulative_balance_residual = (0..SPECIES_COUNT)
            .map(|species| {
                inventories[species] - cumulative_boundary[species] - cumulative_reaction[species]
            })
            .collect();
        observations.push(Observation {
            nominal_time,
            actual_time,
            concentrations,
            sorbed,
            inventories,
            boundary_mass: cumulative_boundary.to_vec(),
            reaction_mass: cumulative_reaction.to_vec(),
            cumulative_balance_residual,
        });
    };
    capture(
        0.0,
        state.time(),
        &state,
        &cumulative_boundary,
        &cumulative_reaction,
        &mut observations,
    );

    for step in 1..=STEP_COUNT {
        let accepted_time = state.time() + DT;
        advance_solute_step_with_processes(
            &mut state,
            &mut workspace,
            &mesh,
            &config,
            SoluteTransportInput {
                accepted_water: AcceptedWaterSnapshot {
                    accepted_time,
                    water_contents: &water_contents,
                    face_fluxes: &face_fluxes,
                },
                start_water_contents: &water_contents,
                face_dispersion: &face_dispersion,
                timestep: DT,
                minimum_pivot: 1e-12,
            },
            Some(SoluteProcessInput {
                bulk_density: &bulk_density,
                max_iterations: 20,
                nonlinear_tolerance: 1e-6,
            }),
        )
        .unwrap_or_else(|error| panic!("Rust source-mapped step {step} failed: {error}"));

        let mut next_mass = vec![0.0; NODE_COUNT * SPECIES_COUNT];
        for species in 0..SPECIES_COUNT {
            let offset = species * NODE_COUNT;
            for node in 0..NODE_COUNT {
                next_mass[offset + node] =
                    (THETA + BULK_DENSITY * KD[species]) * state.concentrations()[offset + node];
            }
            let direct_inlet = if species == 0 { 0.43 * DT } else { 0.0 };
            let outlet = DT * WATER_FLUX * state.concentrations()[offset];
            let boundary = direct_inlet + outlet;
            let reaction: f64 = (0..NODE_COUNT)
                .map(|node| {
                    let current = -RATES[species] * next_mass[offset + node];
                    let incoming = if species == 0 {
                        0.0
                    } else {
                        RATES[species - 1] * next_mass[(species - 1) * NODE_COUNT + node]
                    };
                    DT * control_widths[node] * (current + incoming)
                })
                .sum();
            let storage: f64 = (0..NODE_COUNT)
                .map(|node| {
                    control_widths[node] * (next_mass[offset + node] - old_mass[offset + node])
                })
                .sum();
            let scale = storage.abs() + boundary.abs() + reaction.abs();
            close(
                storage,
                boundary + reaction,
                scale,
                &format!("independent species {species} balance at step {step}"),
            );
            let diagnostic = state.accounting().last_step()[species];
            close(
                diagnostic.storage_change,
                storage,
                scale,
                "reported versus independently integrated storage",
            );
            close(
                diagnostic.boundary_exchange,
                boundary,
                scale,
                "reported versus independently computed boundary exchange",
            );
            close(
                diagnostic.reaction_change,
                reaction,
                scale,
                "reported versus independently computed reaction change",
            );
            cumulative_boundary[species] += boundary;
            cumulative_reaction[species] += reaction;
        }
        old_mass = next_mass;

        if [500, 1000, 2000].contains(&step) {
            for species in 0..SPECIES_COUNT {
                let scale = cumulative_boundary[species].abs() + cumulative_reaction[species].abs();
                close(
                    state.accounting().cumulative_boundary_mass()[species],
                    cumulative_boundary[species],
                    scale,
                    "cumulative boundary accounting",
                );
                close(
                    state.accounting().cumulative_reaction_mass()[species],
                    cumulative_reaction[species],
                    scale,
                    "cumulative reaction accounting",
                );
            }
            capture(
                step as f64 * DT,
                state.time(),
                &state,
                &cumulative_boundary,
                &cumulative_reaction,
                &mut observations,
            );
        }
    }

    assert_eq!(state.accounting().accepted_steps(), STEP_COUNT as u64);
    close(state.time(), 200.0, 200.0, "terminal time");
    let path = resolved_output_path();
    fs::write(&path, observation_json(&observations))
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}

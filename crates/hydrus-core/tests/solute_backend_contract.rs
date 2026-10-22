//! Backend-agnostic contract for every `SoluteBackend`: invariants only, no backend numbers.
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, Mesh1D,
};
use hydrus_core::solute::{advance_solute_step_with_processes, SoluteProcessInput};
use hydrus_core::{
    advance_solute_step, advance_water_step, AcceptedWaterSnapshot, EquilibriumSorption,
    HydraulicError, LowerBoundaryMode, SoluteBackend, SoluteBoundaries, SoluteBoundaryMode,
    SoluteConfig, SoluteSpecies, SoluteState, SoluteTransportInput, SoluteUnits, SoluteWorkspace,
    UpperBoundary, WaterState, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

/// Linear water retention with unit conductivity, so the caller-supplied upper
/// half-cell row below is exactly the discrete balance of the last cell.
struct LinearModel;

impl HydraulicModel for LinearModel {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: 1.0 + 0.1 * head,
            conductivity: 1.0,
            capacity: 0.1,
        })
    }

    fn derivatives(&self, _head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        Ok(HydraulicDerivatives {
            dtheta_dhead: 0.1,
            dconductivity_dhead: 0.0,
            dcapacity_dhead: 0.0,
        })
    }
}

fn policy() -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations: 30,
        head_tolerance: 1e-12,
        water_tolerance: 1e-12,
        minimum_timestep: 0.001,
        retry_factor: 2.0,
        minimum_pivot: 1e-12,
    }
}

const COORDINATES: [f64; 5] = [0.0, 0.5, 1.5, 2.0, 3.0];
const HEADS: [f64; 5] = [-4.0, -2.5, -3.0, -1.0, -2.0];

fn sloped_mesh() -> Mesh1D {
    Mesh1D::new(COORDINATES.to_vec(), vec![0; 5], None, 1, 0, 0.8).unwrap()
}

fn initial_state() -> WaterState {
    let water = HEADS.iter().map(|head| 1.0 + 0.1 * head).collect();
    WaterState::new(0.0, HEADS.to_vec(), water).unwrap()
}

fn control_width(node: usize) -> f64 {
    let below = if node == 0 {
        0.0
    } else {
        COORDINATES[node] - COORDINATES[node - 1]
    };
    let above = if node == 4 {
        0.0
    } else {
        COORDINATES[node + 1] - COORDINATES[node]
    };
    (below + above) / 2.0
}

/// Upper half-cell balance for `LinearModel` (K = 1, slope cosine 0.8, dt = 0.2);
/// the transform adds the prescribed inflow to `rhs`.
fn upper_row(sink: &[f64]) -> BoundaryRow {
    let dx = COORDINATES[4] - COORDINATES[3];
    let storage = control_width(4) * 0.1 / 0.2;
    BoundaryRow {
        lower: -1.0 / dx,
        diagonal: storage + 1.0 / dx,
        upper: 0.0,
        rhs: storage * HEADS[4] - 0.8 - sink[4] * control_width(4),
    }
}

fn input<'a>(
    mesh: &'a Mesh1D,
    materials: &'a [&'a dyn HydraulicModel],
    sink: &'a [f64],
    averaging: FaceAveraging,
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials,
        sink,
        root_uptake: None,
        timestep: 0.2,
        target_time: 0.2,
        face_averaging: averaging,
        lower: LowerBoundaryMode::PrescribedFlux { flux: 0.3 },
        upper: UpperBoundary::Flux {
            flux: -0.1,
            row: upper_row(sink),
        },
    }
}

fn config(backend: SoluteBackend) -> SoluteConfig {
    SoluteConfig::new(
        SoluteUnits {
            concentration: "mass/volume".into(),
            sorbed_mass: "mass/mass".into(),
        },
        vec![
            SoluteSpecies {
                name: "a".into(),
                decay_rate: 0.0,
                equilibrium_sorption: EquilibriumSorption::None,
                kinetic_sorption: None,
            },
            SoluteSpecies {
                name: "b".into(),
                decay_rate: 0.0,
                equilibrium_sorption: EquilibriumSorption::None,
                kinetic_sorption: None,
            },
        ],
        SoluteBoundaries {
            lower: SoluteBoundaryMode::TotalFlux { flux: 0.0 },
            upper: SoluteBoundaryMode::TotalFlux { flux: 0.0 },
        },
        1,
        &[],
    )
    .unwrap()
    .with_backend(backend)
}

/// Runs a real water step and hands the published snapshot data to `body`.
fn with_water<R>(body: impl FnOnce(&Mesh1D, AcceptedWaterSnapshot<'_>, &[f64], f64) -> R) -> R {
    let mesh = sloped_mesh();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 5];
    let mut state = initial_state();
    let old = state.water_contents().to_vec();
    let mut workspace = WaterWorkspace::new(5).unwrap();
    let report = advance_water_step(
        &mut state,
        &mut workspace,
        input(&mesh, &materials, &sink, FaceAveraging::Arithmetic),
        policy(),
    )
    .unwrap();
    let snapshot = report.accepted_water();
    body(&mesh, snapshot, &old, report.timestep)
}

fn fresh_state(config: &SoluteConfig, mesh: &Mesh1D) -> SoluteState {
    SoluteState::new(config, mesh, 0.0, vec![1.0; 10], vec![0.0; 10]).unwrap()
}

fn transport<'a>(
    water: AcceptedWaterSnapshot<'a>,
    old: &'a [f64],
    dispersion: &'a [f64],
    timestep: f64,
    pivot: f64,
) -> SoluteTransportInput<'a> {
    SoluteTransportInput {
        accepted_water: water,
        start_water_contents: old,
        face_dispersion: dispersion,
        timestep,
        minimum_pivot: pivot,
    }
}

#[test]
fn default_backend_is_listed_and_selection_round_trips() {
    let default = SoluteConfig::new(
        SoluteUnits {
            concentration: "c".into(),
            sorbed_mass: "s".into(),
        },
        vec![SoluteSpecies {
            name: "a".into(),
            decay_rate: 0.0,
            equilibrium_sorption: EquilibriumSorption::None,
            kinetic_sorption: None,
        }],
        SoluteBoundaries {
            lower: SoluteBoundaryMode::TotalFlux { flux: 0.0 },
            upper: SoluteBoundaryMode::TotalFlux { flux: 0.0 },
        },
        1,
        &[],
    )
    .unwrap();
    assert_eq!(default.backend(), SoluteBackend::FortranCompatible);
    assert!(SoluteBackend::ALL.contains(&default.backend()));
    for &backend in SoluteBackend::ALL {
        assert_eq!(default.clone().with_backend(backend).backend(), backend);
    }
}

#[test]
fn accepted_step_reaches_water_time_and_preserves_counts() {
    for &backend in SoluteBackend::ALL {
        with_water(|mesh, water, old, dt| {
            let cfg = config(backend);
            let mut state = fresh_state(&cfg, mesh);
            let mut ws = SoluteWorkspace::new(5, 2).unwrap();
            let disp = [0.1; 4];
            advance_solute_step(
                &mut state,
                &mut ws,
                mesh,
                &cfg,
                transport(water, old, &disp, dt, 1e-12),
            )
            .unwrap();
            assert_eq!(state.time(), water.accepted_time);
            assert_eq!(state.concentrations().len(), 10);
            assert_eq!(state.kinetic_sorbed().len(), 10);
            assert!(state.concentrations().iter().all(|v| v.is_finite()));
            assert_eq!(state.accounting().accepted_steps(), 1);
            assert_eq!(state.accounting().cumulative_boundary_mass().len(), 2);
        });
    }
}

#[test]
fn process_entry_point_obeys_the_same_invariants() {
    for &backend in SoluteBackend::ALL {
        with_water(|mesh, water, old, dt| {
            let cfg = config(backend);
            let mut state = fresh_state(&cfg, mesh);
            let mut ws = SoluteWorkspace::new(5, 2).unwrap();
            let disp = [0.1; 4];
            advance_solute_step_with_processes(
                &mut state,
                &mut ws,
                mesh,
                &cfg,
                transport(water, old, &disp, dt, 1e-12),
                None,
            )
            .unwrap();
            assert_eq!(state.time(), water.accepted_time);
            assert_eq!(state.concentrations().len(), 10);
        });
    }
}

#[test]
fn malformed_input_is_rejected_before_mutation() {
    for &backend in SoluteBackend::ALL {
        with_water(|mesh, water, old, dt| {
            let cfg = config(backend);
            let mut state = fresh_state(&cfg, mesh);
            let before = state.clone();
            let mut ws = SoluteWorkspace::new(5, 2).unwrap();
            let disp = [0.1; 4];
            let short_fluxes = &water.face_fluxes[1..];
            let short_water = &water.water_contents[1..];
            let bad_time = AcceptedWaterSnapshot {
                accepted_time: water.accepted_time + 1.0,
                ..water
            };
            let cases: Vec<SoluteTransportInput<'_>> = vec![
                transport(
                    AcceptedWaterSnapshot {
                        face_fluxes: short_fluxes,
                        ..water
                    },
                    old,
                    &disp,
                    dt,
                    1e-12,
                ),
                transport(
                    AcceptedWaterSnapshot {
                        water_contents: short_water,
                        ..water
                    },
                    old,
                    &disp,
                    dt,
                    1e-12,
                ),
                transport(water, old, &disp[1..], dt, 1e-12),
                transport(water, old, &disp, f64::NAN, 1e-12),
                transport(bad_time, old, &disp, dt, 1e-12),
            ];
            for case in cases {
                assert!(advance_solute_step(&mut state, &mut ws, mesh, &cfg, case).is_err());
                assert_eq!(state, before);
            }
        });
    }
}

#[test]
fn any_failed_advance_leaves_state_unchanged_and_workspace_reusable() {
    for &backend in SoluteBackend::ALL {
        with_water(|mesh, water, old, dt| {
            let cfg = config(backend);
            let mut state = fresh_state(&cfg, mesh);
            let before = state.clone();
            let mut ws = SoluteWorkspace::new(5, 2).unwrap();
            let disp = [0.1; 4];
            // An unreachable pivot threshold is finite and positive, so it can
            // only fail inside the trial solve if at all.
            let failed = advance_solute_step(
                &mut state,
                &mut ws,
                mesh,
                &cfg,
                transport(water, old, &disp, dt, 1e300),
            );
            assert!(failed.is_err());
            assert_eq!(state, before);
            advance_solute_step(
                &mut state,
                &mut ws,
                mesh,
                &cfg,
                transport(water, old, &disp, dt, 1e-12),
            )
            .unwrap();
            assert_eq!(state.time(), water.accepted_time);
        });
    }
}

#[test]
fn fortran_compatible_supports_multi_material_mesh() {
    let mesh = Mesh1D::new(
        vec![0.0, 1.0, 2.0, 3.0, 4.0],
        vec![0, 0, 1, 1, 1],
        None,
        2,
        0,
        0.8,
    )
    .unwrap();
    let cfg = SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![SoluteSpecies {
            name: "tracer".into(),
            decay_rate: 0.0,
            equilibrium_sorption: EquilibriumSorption::None,
            kinetic_sorption: None,
        }],
        SoluteBoundaries {
            lower: SoluteBoundaryMode::Concentration { value: 1.0 },
            upper: SoluteBoundaryMode::Concentration { value: 0.0 },
        },
        1,
        &[],
    )
    .unwrap()
    .with_backend(SoluteBackend::FortranCompatible);

    let mut state = SoluteState::new(&cfg, &mesh, 0.0, vec![0.0; 5], vec![0.0; 5]).unwrap();
    let mut ws = SoluteWorkspace::with_materials(5, 1, 4).unwrap();
    let old_water = [0.3; 5];
    let new_water = [0.3; 5];
    let face_fluxes = [0.05; 6];
    let dispersion = [0.1; 4];

    let input = SoluteTransportInput {
        accepted_water: AcceptedWaterSnapshot {
            accepted_time: 1.0,
            water_contents: &new_water,
            face_fluxes: &face_fluxes,
        },
        start_water_contents: &old_water,
        face_dispersion: &dispersion,
        timestep: 1.0,
        minimum_pivot: 1e-12,
    };

    let proc_input = SoluteProcessInput {
        bulk_density: &[1.4, 1.4, 1.6, 1.6, 1.6],
        max_iterations: 10,
        nonlinear_tolerance: 1e-6,
    };

    advance_solute_step_with_processes(&mut state, &mut ws, &mesh, &cfg, input, Some(proc_input))
        .unwrap();
    assert_eq!(state.time(), 1.0);
    assert!(state
        .concentrations()
        .iter()
        .all(|c| c.is_finite() && *c >= 0.0));
}

#[test]
fn fortran_compatible_respects_per_species_boundary_concentrations() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 1.0).unwrap();
    let cfg = SoluteConfig::new_with_species_boundaries(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![
            SoluteSpecies {
                name: "sp1".into(),
                decay_rate: 0.0,
                equilibrium_sorption: EquilibriumSorption::None,
                kinetic_sorption: None,
            },
            SoluteSpecies {
                name: "sp2".into(),
                decay_rate: 0.0,
                equilibrium_sorption: EquilibriumSorption::None,
                kinetic_sorption: None,
            },
        ],
        vec![
            SoluteBoundaries {
                lower: SoluteBoundaryMode::Concentration { value: 10.0 },
                upper: SoluteBoundaryMode::Concentration { value: 0.0 },
            },
            SoluteBoundaries {
                lower: SoluteBoundaryMode::Concentration { value: 50.0 },
                upper: SoluteBoundaryMode::Concentration { value: 5.0 },
            },
        ],
        1,
        &[],
    )
    .unwrap()
    .with_backend(SoluteBackend::FortranCompatible);

    let mut state = SoluteState::new(&cfg, &mesh, 0.0, vec![0.0; 6], vec![0.0; 6]).unwrap();
    let mut ws = SoluteWorkspace::new(3, 2).unwrap();
    let old_water = [0.3; 3];
    let new_water = [0.3; 3];
    let face_fluxes = [0.0; 4];
    let dispersion = [0.1; 2];

    let input = SoluteTransportInput {
        accepted_water: AcceptedWaterSnapshot {
            accepted_time: 1.0,
            water_contents: &new_water,
            face_fluxes: &face_fluxes,
        },
        start_water_contents: &old_water,
        face_dispersion: &dispersion,
        timestep: 1.0,
        minimum_pivot: 1e-12,
    };

    advance_solute_step(&mut state, &mut ws, &mesh, &cfg, input).unwrap();
    assert_eq!(state.time(), 1.0);
    // Boundary conditions should produce higher concentration for sp2 than sp1 at node 0
    assert_eq!(state.concentrations()[0], 10.0);
    assert_eq!(state.concentrations()[3], 50.0);
}

#[test]
fn fortran_compatible_evolves_kinetic_sorbed_pool() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 1.0).unwrap();
    let cfg = SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![SoluteSpecies {
            name: "sorbing".into(),
            decay_rate: 0.0,
            equilibrium_sorption: EquilibriumSorption::Linear {
                distribution_coefficient: 0.5,
            },
            kinetic_sorption: Some(hydrus_core::KineticSorption {
                capacity_coefficient: 0.5,
                transfer_rate: 0.1,
            }),
        }],
        SoluteBoundaries {
            lower: SoluteBoundaryMode::Concentration { value: 1.0 },
            upper: SoluteBoundaryMode::Concentration { value: 1.0 },
        },
        1,
        &[],
    )
    .unwrap()
    .with_backend(SoluteBackend::FortranCompatible);

    let mut state = SoluteState::new(&cfg, &mesh, 0.0, vec![1.0; 3], vec![0.0; 3]).unwrap();
    let mut ws = SoluteWorkspace::new(3, 1).unwrap();
    let old_water = [0.3; 3];
    let new_water = [0.3; 3];
    let face_fluxes = [0.0; 4];
    let dispersion = [0.1; 2];

    let input = SoluteTransportInput {
        accepted_water: AcceptedWaterSnapshot {
            accepted_time: 1.0,
            water_contents: &new_water,
            face_fluxes: &face_fluxes,
        },
        start_water_contents: &old_water,
        face_dispersion: &dispersion,
        timestep: 1.0,
        minimum_pivot: 1e-12,
    };

    let proc_input = SoluteProcessInput {
        bulk_density: &[1.5; 3],
        max_iterations: 10,
        nonlinear_tolerance: 1e-6,
    };

    advance_solute_step_with_processes(&mut state, &mut ws, &mesh, &cfg, input, Some(proc_input))
        .unwrap();

    // With concentration > 0 and initial kinetic sorbed = 0, kinetic sorption should evolve > 0
    assert_eq!(state.time(), 1.0);
    for &sorbed in state.kinetic_sorbed() {
        assert!(
            sorbed > 0.0,
            "kinetic sorbed pool should evolve above zero, got {sorbed}"
        );
    }
}

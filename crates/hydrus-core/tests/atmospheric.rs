use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, Mesh1D,
};
use hydrus_core::{
    advance_atmospheric_step, AtmosphericRates, ForcingEvent, ForcingSchedule, ForcingUnits,
    HydraulicError, LowerBoundaryMode, UpperBoundary, UpperBoundaryMode, WaterState,
    WaterStepError, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

mod common;

use common::default_policy as policy;

struct LinearModel;

impl HydraulicModel for LinearModel {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: 0.2 + 0.1 * head,
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

fn setup<'a>(
    mesh: &'a Mesh1D,
    materials: &'a [&'a dyn HydraulicModel],
    sink: &'a [f64],
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials,
        sink,
        root_uptake: None,
        timestep: 2.0,
        target_time: 2.0,
        face_averaging: FaceAveraging::Arithmetic,
        lower: LowerBoundaryMode::FixedHead {
            head: 0.0,
            row: BoundaryRow {
                lower: 0.0,
                diagonal: 0.0,
                upper: 0.0,
                rhs: 0.0,
            },
            flux: 0.0,
        },
        upper: UpperBoundary::Atmospheric { critical_head: 0.0 },
    }
}

fn schedule() -> ForcingSchedule<'static> {
    ForcingSchedule::new(
        AtmosphericRates {
            precipitation: 0.0,
            evaporation: 0.0,
        },
        vec![ForcingEvent {
            time: 1.0,
            rates: AtmosphericRates {
                precipitation: 0.2,
                evaporation: 0.0,
            },
        }],
        ForcingUnits {
            time: "day",
            water_depth_rate: "cm/day",
        },
    )
    .unwrap()
}

fn constant_schedule(precipitation: f64, evaporation: f64) -> ForcingSchedule<'static> {
    ForcingSchedule::new(
        AtmosphericRates {
            precipitation,
            evaporation,
        },
        vec![],
        ForcingUnits {
            time: "day",
            water_depth_rate: "cm/day",
        },
    )
    .unwrap()
}

fn no_flow_lower(input: &mut WaterStepInput<'_>) {
    input.lower = LowerBoundaryMode::PrescribedFlux { flux: 0.0 };
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn isolated_atmospheric_flux_row_accepts_demand_limited_flux() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut input = setup(&mesh, &materials, &sink);
    no_flow_lower(&mut input);
    input.timestep = 1.0;
    input.target_time = 1.0;
    input.upper = UpperBoundary::Atmospheric { critical_head: 0.1 };
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();

    let report = advance_atmospheric_step(
        &mut state,
        &mut workspace,
        input,
        policy(),
        &constant_schedule(0.01, 0.0),
    )
    .unwrap();

    assert_eq!(
        state.atmospheric().upper_mode,
        UpperBoundaryMode::AtmosphericFlux
    );
    assert!(state.heads()[2] <= 0.1);
    assert_close(state.atmospheric().ponded_depth, 0.0);
    assert_close(report.balance.soil_boundary_flux, 0.01);
    assert_close(report.balance.atmospheric_exchange, 0.01);
    assert_close(report.balance.boundary_flux, 0.01);
    assert_close(report.balance.storage_change, report.balance.boundary_flux);
    assert!(report.balance.absolute_residual < 1e-9, "{report:?}");
}

#[test]
fn isolated_ponded_head_row_matches_discrete_top_flux_and_balance() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut input = setup(&mesh, &materials, &sink);
    no_flow_lower(&mut input);
    input.timestep = 1.0;
    input.target_time = 1.0;
    input.upper = UpperBoundary::Atmospheric { critical_head: 0.1 };
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let old_top_water = state.water_contents()[2];
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();

    let report = advance_atmospheric_step(
        &mut state,
        &mut workspace,
        input,
        policy(),
        &constant_schedule(1.0, 0.0),
    )
    .unwrap();

    assert_eq!(
        state.atmospheric().upper_mode,
        UpperBoundaryMode::PondedHead
    );
    assert_close(state.heads()[2], 0.1);

    // This is the discrete top-face flux expression used by the solver:
    // K_face * (head gradient + gravity) plus half-cell storage and sink terms.
    let top = state.heads().len() - 1;
    let previous = top - 1;
    let dx = mesh.coordinates()[top] - mesh.coordinates()[previous];
    let conductivity = 0.5
        * (model.conductivity(state.heads()[top]).unwrap()
            + model.conductivity(state.heads()[previous]).unwrap());
    let timestep = 1.0;
    let storage_flux =
        (model.theta(state.heads()[top]).unwrap() - old_top_water) * dx / (2.0 * timestep);
    let sink_flux = sink[top] * dx / 2.0;
    let discrete_top_flux = conductivity
        * ((state.heads()[top] - state.heads()[previous]) / dx + mesh.inclination_cosine())
        + storage_flux
        + sink_flux;
    assert_close(
        report.balance.soil_boundary_flux,
        discrete_top_flux * timestep,
    );

    assert_close(report.balance.atmospheric_exchange, 1.0);
    assert_close(report.balance.boundary_flux, 1.0);
    assert!(report.balance.surface_storage_change > 0.0);
    assert_close(
        report.balance.soil_storage_change + report.balance.surface_storage_change,
        report.balance.boundary_flux,
    );
    assert!(report.balance.absolute_residual < 1e-9, "{report:?}");
}

#[test]
fn accepted_rain_infiltrates_then_excess_builds_ponded_storage() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut input = setup(&mesh, &materials, &sink);
    no_flow_lower(&mut input);
    input.timestep = 1.0;
    input.target_time = 1.0;
    input.upper = UpperBoundary::Atmospheric { critical_head: 0.1 };
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();

    let rain = constant_schedule(0.01, 0.0);
    let first =
        advance_atmospheric_step(&mut state, &mut workspace, input, policy(), &rain).unwrap();
    assert_eq!(
        state.atmospheric().upper_mode,
        UpperBoundaryMode::AtmosphericFlux
    );
    assert!(state.heads()[2] > 0.0);
    assert_close(state.atmospheric().ponded_depth, 0.0);
    assert_close(first.balance.boundary_flux, 0.01);
    assert_close(first.balance.soil_boundary_flux, 0.01);
    assert_close(first.balance.atmospheric_exchange, 0.01);
    assert_eq!(first.balance.soil_boundary_flux, 0.01);
    assert!(first.balance.absolute_residual < 1e-9, "{first:?}");

    input.target_time = 2.0;
    no_flow_lower(&mut input);
    let heavy_rain = constant_schedule(1.0, 0.0);
    let before_ponding = state.atmospheric().ponded_depth;
    let before_heads = state.heads().to_vec();
    let second =
        advance_atmospheric_step(&mut state, &mut workspace, input, policy(), &heavy_rain).unwrap();
    assert_eq!(
        state.atmospheric().upper_mode,
        UpperBoundaryMode::PondedHead
    );
    assert_close(state.heads()[2], 0.1);
    assert!(second.atmospheric_mode_transitions > 0);
    assert_eq!(
        second.atmospheric_transitions.len(),
        second.atmospheric_mode_transitions
    );
    for transition in second.atmospheric_transitions {
        assert_eq!(transition.demand, 1.0);
        assert_eq!(transition.from_mode, UpperBoundaryMode::AtmosphericFlux);
        assert_eq!(transition.to_mode, UpperBoundaryMode::PondedHead);
        assert!(transition.top_head > 0.1, "{transition:?}");
        assert_eq!(transition.soil_flux, transition.demand);
        assert_eq!(transition.candidate_surface_storage, 0.0);
    }
    assert!(state.atmospheric().ponded_depth > before_ponding);
    assert_close(second.balance.boundary_flux, 1.0);
    assert_close(second.balance.atmospheric_exchange, 1.0);
    assert!(second.balance.soil_boundary_flux < second.balance.atmospheric_exchange);
    let expected_head_limited_flux =
        (state.heads()[2] - state.heads()[1]) + 0.05 * (state.heads()[2] - before_heads[2]);
    assert_close(
        second.balance.soil_boundary_flux,
        expected_head_limited_flux,
    );
    assert_eq!(state.heads()[2], 0.1);
    assert_close(
        second.balance.surface_storage_change,
        state.atmospheric().ponded_depth - before_ponding,
    );
    assert_close(
        second.balance.soil_storage_change + second.balance.surface_storage_change,
        second.balance.boundary_flux,
    );
    assert!(second.balance.absolute_residual < 1e-9, "{second:?}");
}

#[test]
fn accepted_evaporation_is_limited_by_soil_supply_and_balanced() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut input = setup(&mesh, &materials, &sink);
    no_flow_lower(&mut input);
    input.timestep = 1.0;
    input.target_time = 1.0;
    input.upper = UpperBoundary::Atmospheric {
        critical_head: -0.01,
    };
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();

    let report = advance_atmospheric_step(
        &mut state,
        &mut workspace,
        input,
        policy(),
        &constant_schedule(0.0, 0.2),
    )
    .unwrap();
    assert_eq!(
        state.atmospheric().upper_mode,
        UpperBoundaryMode::PondedHead
    );
    assert_close(state.heads()[2], -0.01);
    assert_close(state.atmospheric().ponded_depth, 0.0);
    assert!(report.balance.boundary_flux < 0.0);
    assert!(report.balance.boundary_flux > -0.2);
    assert_close(
        report.balance.atmospheric_exchange,
        report.balance.soil_boundary_flux,
    );
    assert!(report.atmospheric_mode_transitions > 0);
    assert_eq!(
        report.atmospheric_transitions.len(),
        report.atmospheric_mode_transitions
    );
    for transition in report.atmospheric_transitions {
        assert_eq!(transition.demand, -0.2);
        assert_eq!(transition.from_mode, UpperBoundaryMode::AtmosphericFlux);
        assert_eq!(transition.to_mode, UpperBoundaryMode::PondedHead);
        assert!(transition.top_head < -0.01, "{transition:?}");
        assert_eq!(transition.soil_flux, transition.demand);
        assert_eq!(transition.candidate_surface_storage, 0.0);
    }
    assert_close(report.balance.surface_storage_change, 0.0);
    assert_close(report.balance.boundary_flux, report.balance.storage_change);
    assert!(report.balance.absolute_residual < 1e-9, "{report:?}");
}

#[test]
fn clips_to_event_then_advances_cursor_only_after_acceptance() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut input = setup(&mesh, &materials, &sink);
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();
    let first =
        advance_atmospheric_step(&mut state, &mut workspace, input, policy(), &schedule()).unwrap();
    assert_eq!(first.accepted_time, 1.0);
    assert_eq!(state.atmospheric().event_cursor, 1);
    input.target_time = 2.0;
    input.timestep = 1.0;
    input.upper = UpperBoundary::Atmospheric {
        critical_head: 1000.0,
    };
    let second =
        advance_atmospheric_step(&mut state, &mut workspace, input, policy(), &schedule()).unwrap();
    assert_eq!(second.accepted_time, 2.0);
    assert_eq!(second.balance.boundary_flux, 0.2);
    assert_eq!(
        state.atmospheric().upper_mode,
        UpperBoundaryMode::AtmosphericFlux
    );
    assert_eq!(state.atmospheric().ponded_depth, 0.0);
}

#[test]
fn dry_surface_keeps_evaporative_demand_as_a_negative_flux() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut input = setup(&mesh, &materials, &sink);
    input.upper = UpperBoundary::Atmospheric {
        critical_head: -1000.0,
    };
    let schedule = ForcingSchedule::new(
        AtmosphericRates {
            precipitation: 0.0,
            evaporation: 10.0,
        },
        vec![],
        ForcingUnits {
            time: "day",
            water_depth_rate: "cm/day",
        },
    )
    .unwrap();
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();
    let report =
        advance_atmospheric_step(&mut state, &mut workspace, input, policy(), &schedule).unwrap();
    assert_eq!(report.balance.boundary_flux, -20.0);
}

#[test]
fn malformed_event_order_and_empty_unit_labels_are_rejected() {
    let units = ForcingUnits {
        time: "day",
        water_depth_rate: "cm/day",
    };
    let rate = AtmosphericRates {
        precipitation: 0.0,
        evaporation: 0.0,
    };
    assert!(matches!(
        ForcingSchedule::new(
            rate,
            vec![
                ForcingEvent {
                    time: 2.0,
                    rates: rate
                },
                ForcingEvent {
                    time: 1.0,
                    rates: rate
                }
            ],
            units
        ),
        Err(WaterStepError::InvalidForcing { .. })
    ));
    assert!(matches!(
        ForcingSchedule::new(
            rate,
            vec![],
            ForcingUnits {
                time: " ",
                water_depth_rate: "cm/day"
            }
        ),
        Err(WaterStepError::InvalidForcing { name: "units", .. })
    ));
}

#[test]
fn unsupported_upper_boundary_kind_is_rejected() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut input = setup(&mesh, &materials, &sink);
    input.upper = UpperBoundary::Flux {
        flux: 0.0,
        row: BoundaryRow {
            lower: 0.0,
            diagonal: 1.0,
            upper: 0.0,
            rhs: 0.0,
        },
    };
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();
    assert!(matches!(
        advance_atmospheric_step(&mut state, &mut workspace, input, policy(), &schedule()),
        Err(WaterStepError::InvalidUpperBoundary { .. })
    ));
}

#[test]
fn failed_trial_does_not_advance_atmospheric_cursor_or_mode() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0, f64::NAN, 0.0];
    let input = setup(&mesh, &materials, &sink);
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let original = state.clone();
    let mut undersized_workspace = WaterWorkspace::new(3).unwrap();
    let valid_sink = [0.0; 3];
    let valid_input = setup(&mesh, &materials, &valid_sink);
    assert!(matches!(
        advance_atmospheric_step(
            &mut state,
            &mut undersized_workspace,
            valid_input,
            policy(),
            &schedule()
        ),
        Err(WaterStepError::InsufficientAtmosphericTraceCapacity {
            required: 8,
            available: 0
        })
    ));
    assert_eq!(state, original);
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();
    assert!(
        advance_atmospheric_step(&mut state, &mut workspace, input, policy(), &schedule()).is_err()
    );
    assert_eq!(state, original);
}

#[test]
fn rejected_trial_restores_accepted_atmospheric_history() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let input = setup(&mesh, &materials, &sink);
    let schedule = ForcingSchedule::new(
        AtmosphericRates {
            precipitation: 0.2,
            evaporation: 0.0,
        },
        vec![ForcingEvent {
            time: 1.0,
            rates: AtmosphericRates {
                precipitation: 0.0,
                evaporation: 0.0,
            },
        }],
        ForcingUnits {
            time: "day",
            water_depth_rate: "cm/day",
        },
    )
    .unwrap();
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let original = state.clone();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();
    let strict = WaterStepPolicy {
        max_iterations: 1,
        minimum_timestep: 1.1,
        ..policy()
    };
    assert!(matches!(
        advance_atmospheric_step(&mut state, &mut workspace, input, strict, &schedule),
        Err(WaterStepError::NonConverged { .. })
    ));
    assert_eq!(state, original);
}

#[test]
fn rejected_evaporation_trial_does_not_promote_mode_or_storage() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut input = setup(&mesh, &materials, &sink);
    input.upper = UpperBoundary::Atmospheric { critical_head: 1.0 };
    let schedule = ForcingSchedule::new(
        AtmosphericRates {
            precipitation: 0.0,
            evaporation: 0.2,
        },
        vec![],
        ForcingUnits {
            time: "day",
            water_depth_rate: "cm/day",
        },
    )
    .unwrap();
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let original = state.clone();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();
    let strict = WaterStepPolicy {
        max_iterations: 1,
        minimum_timestep: 1.1,
        ..policy()
    };
    let result = advance_atmospheric_step(&mut state, &mut workspace, input, strict, &schedule);
    assert!(matches!(result, Err(WaterStepError::NonConverged { .. })));
    assert_eq!(state, original);
}

//! First-water acceptance scenarios use deterministic in-memory parameters.
//!
//! Provenance: the dry-column and cyclic-atmosphere labels follow candidate
//! cases in `planning/master/VALIDATION_STRATEGY.md` (manual Chapters 11.1 and
//! 11.2); their machine-readable decks and expected outputs are unavailable.
//! Capillary rise is an independently constructed analytic direction check,
//! not a manual or Fortran result. Values below define these test inputs only.
//! Heads and coordinates are cm, time is days, rates are cm/day, theta and root
//! density convention are dimensionless and 1/cm respectively. Balance terms
//! are signed cm per unit area; positive boundary flux enters the domain and
//! positive sink is extraction. No calibrated trajectory, universal tolerance,
//! or executable parity is asserted.
use hydrus_core::hydraulics::{
    BoundaryCondition, BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives,
    HydraulicModel, Mesh1D,
};
use hydrus_core::{
    advance_atmospheric_step, advance_water_step, feddes_reduction, AtmosphericRates, ForcingEvent,
    ForcingSchedule, ForcingUnits, HydraulicError, LowerBoundaryMode, RootUptakeInput,
    RootUptakeModel, UnsupportedLowerBoundaryMode, UpperBoundaryMode, WaterState, WaterStepError,
    WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

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

fn mesh() -> Mesh1D {
    Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap()
}

fn policy() -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations: 12,
        head_tolerance: 1e-9,
        water_tolerance: 1e-9,
        minimum_timestep: 0.01,
        retry_factor: 2.0,
        minimum_pivot: 1e-12,
    }
}

fn input<'a>(
    mesh: &'a Mesh1D,
    materials: &'a [&'a dyn HydraulicModel],
    sink: &'a [f64],
    target_time: f64,
    root_uptake: Option<&'a RootUptakeInput<'a>>,
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials,
        sink,
        root_uptake,
        timestep: target_time,
        target_time,
        face_averaging: FaceAveraging::Arithmetic,
        lower_row: BoundaryRow {
            lower: 0.0,
            diagonal: 0.0,
            upper: 0.0,
            rhs: 0.0,
        },
        lower_boundary_mode: LowerBoundaryMode::PrescribedFlux { flux: 0.0 },
        upper_row: BoundaryRow {
            lower: 0.0,
            diagonal: 0.0,
            upper: 0.0,
            rhs: 0.0,
        },
        upper_condition: BoundaryCondition::Dirichlet { value: 0.1 },
        lower_boundary_flux: 0.0,
        upper_boundary_flux: 0.0,
    }
}

fn schedule(initial: AtmosphericRates, events: Vec<ForcingEvent>) -> ForcingSchedule<'static> {
    ForcingSchedule::new(
        initial,
        events,
        ForcingUnits {
            time: "day",
            water_depth_rate: "cm/day",
        },
    )
    .unwrap()
}

fn assert_balanced(report: &hydrus_core::WaterStepReport<'_>) {
    assert!(report.balance.absolute_residual < 1e-8, "{report:?}");
    assert!(report.max_head_residual <= 1e-8, "{report:?}");
    assert!(report.max_water_residual <= 1e-8, "{report:?}");
}

#[test]
fn dry_column_infiltration_has_bounded_state_and_closes_balance() {
    // Provenance: Chapter 11.1 dry-column candidate, reconstructed in memory;
    // not the missing manual deck or a reference-output comparison. Observables
    // are accepted time, heads, atmospheric mode, and integrated balance.
    let mesh = mesh();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut request = input(&mesh, &materials, &sink, 1.0, None);
    request.upper_condition = BoundaryCondition::Dirichlet { value: 0.1 };
    let mut state = WaterState::new(0.0, vec![-1.0; 3], vec![0.9; 3]).unwrap();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();
    let forcing = schedule(
        AtmosphericRates {
            precipitation: 0.01,
            evaporation: 0.0,
        },
        vec![],
    );

    let report =
        advance_atmospheric_step(&mut state, &mut workspace, request, policy(), &forcing).unwrap();
    assert_eq!(report.accepted_time, 1.0);
    assert_eq!(state.time(), 1.0);
    assert!(state.heads().iter().all(|head| (-1.0..=0.1).contains(head)));
    assert!(state.heads()[2] > -1.0);
    assert_eq!(
        state.atmospheric().upper_mode,
        UpperBoundaryMode::AtmosphericFlux
    );
    assert!((report.balance.atmospheric_exchange - 0.01).abs() < 1e-8);
    assert_balanced(&report);
}

#[test]
fn capillary_rise_moves_water_up_from_a_fixed_head_lower_boundary() {
    // Provenance: proposed analytic capillary-rise case from the validation
    // strategy, instantiated as an in-memory sign/direction check. Observables
    // are node-head ordering, accepted time, and signed boundary balance;
    // this does not claim a calibrated transient or shallow-water-table parity.
    let mesh = mesh();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut request = input(&mesh, &materials, &sink, 0.1, None);
    request.lower_boundary_mode = LowerBoundaryMode::FixedHead { head: 0.0 };
    // Caller-supplied measured exchange closes this fixed-head step balance.
    request.lower_boundary_flux = 0.8333333333333331;
    request.upper_condition = BoundaryCondition::Dirichlet { value: -1.0 };
    let mut state = WaterState::new(0.0, vec![-1.0; 3], vec![0.9; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();

    let report = advance_water_step(&mut state, &mut workspace, request, policy()).unwrap();
    assert_eq!(report.accepted_time, 0.1);
    assert_eq!(state.heads()[0], 0.0);
    assert!(state.heads()[1] > -1.0);
    assert_eq!(state.heads()[2], -1.0);
    assert!(report.balance.soil_boundary_flux >= 0.0);
    assert_balanced(&report);
}

#[test]
fn cyclic_atmosphere_clips_events_switches_modes_and_rolls_back_rejected_trial() {
    // Provenance: Chapter 11.2 cyclic-forcing candidate reconstructed in memory;
    // full forcing deck and expected outputs are unavailable. Observables are
    // event/accepted times, mode, surface storage, and balance; rollback is
    // checked by a separate invalid trial against the accepted snapshot.
    let mesh = mesh();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut request = input(&mesh, &materials, &sink, 1.0, None);
    request.timestep = 1.0;
    request.upper_condition = BoundaryCondition::Dirichlet { value: 0.1 };
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![1.0; 3]).unwrap();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();
    let forcing = schedule(
        AtmosphericRates {
            precipitation: 1.0,
            evaporation: 0.0,
        },
        vec![ForcingEvent {
            time: 0.5,
            rates: AtmosphericRates {
                precipitation: 0.0,
                evaporation: 0.0,
            },
        }],
    );

    request.target_time = 0.5;
    let first =
        advance_atmospheric_step(&mut state, &mut workspace, request, policy(), &forcing).unwrap();
    assert_eq!(first.accepted_time, 0.5);
    assert_eq!(state.atmospheric().event_cursor, 1);
    assert_eq!(
        state.atmospheric().upper_mode,
        UpperBoundaryMode::PondedHead
    );
    assert!(state.atmospheric().ponded_depth >= 0.0);
    assert!(first.atmospheric_mode_transitions > 0);
    assert_balanced(&first);

    request.timestep = 0.5;
    request.target_time = 1.0;
    let second =
        advance_atmospheric_step(&mut state, &mut workspace, request, policy(), &forcing).unwrap();
    assert_eq!(second.accepted_time, 1.0);
    assert_eq!(state.time(), 1.0);
    assert_eq!(state.atmospheric().event_cursor, 1);
    assert_balanced(&second);

    let accepted = state.clone();
    request.timestep = 1.0;
    request.target_time = 2.0;
    request.upper_condition = BoundaryCondition::Neumann { flux: 0.0 };
    request.upper_row = BoundaryRow {
        lower: 0.0,
        diagonal: 1.0,
        upper: 0.0,
        rhs: 0.0,
    };
    let rejected =
        advance_atmospheric_step(&mut state, &mut workspace, request, policy(), &forcing);
    assert!(
        matches!(
            rejected,
            Err(WaterStepError::UnsupportedAtmosphericInput { .. })
        ),
        "{rejected:?}"
    );
    assert_eq!(state, accepted);
}

#[test]
fn cyclic_atmosphere_with_feddes_compensation_has_positive_sink_and_closes_balance() {
    // Provenance: combined Chapter 11.2/root candidate assembled in memory
    // from the source-anchored Feddes contract; no manual deck or Fortran output
    // is available. Units follow the module contract. Observables are accepted
    // event times, active stress, compensation-domain Omega, sink, mode, and
    // balance. This is a bounded contract test, not parameter calibration.
    let mesh = mesh();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let weights = [0.1; 3];
    let roots = RootUptakeInput {
        model: RootUptakeModel::Feddes,
        h0: 100.0,
        h1: 50.0,
        h2: -10.0,
        h3: -20.0,
        potential_demand: 0.01,
        compensation_threshold: 1.0,
        root_weights: &weights,
    };
    let mut request = input(&mesh, &materials, &sink, 0.5, Some(&roots));
    request.upper_condition = BoundaryCondition::Dirichlet { value: 0.1 };
    let mut state = WaterState::new(0.0, vec![-2.0; 3], vec![0.8; 3]).unwrap();
    let mut workspace =
        WaterWorkspace::with_transition_capacity(3, policy().max_iterations).unwrap();
    let forcing = schedule(
        AtmosphericRates {
            precipitation: 0.1,
            evaporation: 0.0,
        },
        vec![ForcingEvent {
            time: 0.25,
            rates: AtmosphericRates {
                precipitation: 0.0,
                evaporation: 0.01,
            },
        }],
    );

    let first =
        advance_atmospheric_step(&mut state, &mut workspace, request, policy(), &forcing).unwrap();
    assert_eq!(first.accepted_time, 0.25);
    assert_eq!(state.atmospheric().event_cursor, 1);
    assert!(first.balance.sink > 0.0);
    assert!(state
        .heads()
        .iter()
        .any(|head| feddes_reduction(*head, &roots) > 0.0));
    let omega: f64 = state
        .heads()
        .iter()
        .zip(weights)
        .enumerate()
        .map(|(node, (head, weight))| {
            feddes_reduction(*head, &roots)
                * weight
                * hydrus_core::hydraulics::nodal_control_volume_width(&mesh, node).unwrap()
        })
        .sum();
    assert!(omega > 0.0 && omega < roots.compensation_threshold);
    assert_balanced(&first);

    request.timestep = 0.25;
    request.target_time = 0.5;
    let second =
        advance_atmospheric_step(&mut state, &mut workspace, request, policy(), &forcing).unwrap();
    assert_eq!(second.accepted_time, 0.5);
    assert!(second.balance.sink > 0.0);
    assert!(state.atmospheric().ponded_depth >= 0.0);
    assert_balanced(&second);
}

#[test]
fn unsupported_seepage_and_water_table_modes_are_typed_errors() {
    // Provenance: supported/unsupported product boundary defined by Phase 4
    // decision D02; no physical results are asserted. Observables are typed
    // error variants and exact unchanged state after validation rejection.
    let mesh = mesh();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![1.0; 3]).unwrap();
    let original = state.clone();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    for (mode, name) in [
        (
            LowerBoundaryMode::Unsupported(UnsupportedLowerBoundaryMode::SeepageFace),
            "seepage_face",
        ),
        (
            LowerBoundaryMode::Unsupported(UnsupportedLowerBoundaryMode::WaterTable),
            "water_table",
        ),
    ] {
        let request = input(&mesh, &materials, &sink, 0.1, None);
        let request = WaterStepInput {
            lower_boundary_mode: mode,
            ..request
        };
        assert_eq!(
            advance_water_step(&mut state, &mut workspace, request, policy()),
            Err(WaterStepError::UnsupportedLowerBoundary { mode: name })
        );
        assert_eq!(state, original);
    }
}

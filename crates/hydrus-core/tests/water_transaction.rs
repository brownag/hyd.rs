use hydrus_core::hydraulics::{
    BoundaryCondition, BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, Mesh1D,
};
use hydrus_core::{
    advance_water_step, HydraulicError, HydraulicModel, LowerBoundaryMode, WaterState,
    WaterStepError, WaterStepInput, WaterWorkspace,
};

mod common;

use common::policy;

struct TransactionModel {
    conductivity: f64,
    capacity: f64,
}

impl HydraulicModel for TransactionModel {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: self.capacity * head,
            conductivity: self.conductivity,
            capacity: self.capacity,
        })
    }

    fn derivatives(&self, _head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        Ok(HydraulicDerivatives {
            dtheta_dhead: self.capacity,
            dconductivity_dhead: 0.0,
            dcapacity_dhead: 0.0,
        })
    }
}

static LOWER_MODEL: TransactionModel = TransactionModel {
    conductivity: 1.0,
    capacity: 0.0,
};
static INTERIOR_MODEL: TransactionModel = TransactionModel {
    conductivity: 1.0,
    capacity: 1.0,
};
static UPPER_MODEL: TransactionModel = TransactionModel {
    conductivity: 1.0,
    capacity: 0.0,
};
static MATERIALS: [&dyn HydraulicModel; 3] = [&LOWER_MODEL, &INTERIOR_MODEL, &UPPER_MODEL];

fn mesh() -> Mesh1D {
    Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0, 1, 2], None, 3, 0, 0.0).unwrap()
}
fn input<'a>(mesh: &'a Mesh1D, timestep: f64) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials: &MATERIALS,
        sink: &[0.0; 3],
        root_uptake: None,
        timestep,
        target_time: 10.0,
        face_averaging: FaceAveraging::Arithmetic,
        lower_row: BoundaryRow {
            lower: 0.0,
            diagonal: 0.0,
            upper: 0.0,
            rhs: 0.0,
        },
        lower_boundary_mode: LowerBoundaryMode::FixedHead { head: 0.0 },
        upper_row: BoundaryRow {
            lower: 0.0,
            diagonal: 0.0,
            upper: 0.0,
            rhs: 0.0,
        },
        upper_condition: BoundaryCondition::Dirichlet { value: 2.0 },
        lower_boundary_flux: 0.0,
        upper_boundary_flux: 0.0,
    }
}

#[test]
fn retries_with_reduced_dt_and_commits_only_on_acceptance() {
    let mesh = mesh();
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let report = advance_water_step(
        &mut state,
        &mut workspace,
        input(&mesh, 9.0),
        policy(1, 0.3),
    )
    .unwrap();
    assert_eq!(report.retries, 3);
    assert!((report.timestep - 1.0 / 3.0).abs() < 1e-12);
    assert!((report.max_head_residual - 0.4).abs() < 1e-12);
    assert_eq!(state.accounting().accepted_steps, 1);
    assert_eq!(state.accounting().cumulative_iterations, 1);
}

#[test]
fn retries_clip_to_target_and_never_accept_past_it() {
    let mesh = mesh();
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let request = WaterStepInput {
        target_time: 0.75,
        ..input(&mesh, 9.0)
    };
    let report = advance_water_step(&mut state, &mut workspace, request, policy(1, 0.1)).unwrap();
    assert_eq!(report.timestep, 0.25);
    assert_eq!(report.accepted_time, 0.25);
    assert_eq!(state.time(), 0.25);
    assert!(report.retries > 0);
}

#[test]
fn minimum_step_failure_restores_every_accepted_field() {
    let mesh = mesh();
    let mut state = WaterState::new(2.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let original = state.clone();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let error = advance_water_step(
        &mut state,
        &mut workspace,
        input(&mesh, 1.0),
        policy(1, 1.0),
    )
    .unwrap_err();
    assert_eq!(
        error,
        WaterStepError::NonConverged {
            timestep: 1.0,
            minimum_timestep: 1.0,
            retries: 0
        }
    );
    assert_eq!(state, original);
}

#[test]
fn invalid_input_does_not_mutate_accepted_state() {
    let mesh = mesh();
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let original = state.clone();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let sink = [0.0, f64::NAN, 0.0];
    let invalid = WaterStepInput {
        sink: &sink,
        ..input(&mesh, 1.0)
    };
    assert!(matches!(
        advance_water_step(&mut state, &mut workspace, invalid, policy(1, 0.1)),
        Err(WaterStepError::NonFiniteInput { .. })
    ));
    assert_eq!(state, original);
}

use hydrus_core::hydraulics::{
    BoundaryCondition, BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives,
    HydraulicModel, Mesh1D,
};
use hydrus_core::{
    advance_water_step, HydraulicError, LowerBoundaryMode, UnsupportedLowerBoundaryMode,
    WaterState, WaterStepError, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

mod common;

use common::default_policy as policy;

struct Model;

impl HydraulicModel for Model {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: head,
            conductivity: 1.0,
            capacity: 1.0,
        })
    }
    fn derivatives(&self, _head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        Ok(HydraulicDerivatives {
            dtheta_dhead: 1.0,
            dconductivity_dhead: 0.0,
            dcapacity_dhead: 0.0,
        })
    }
}

fn input<'a>(
    mesh: &'a Mesh1D,
    materials: &'a [&'a dyn HydraulicModel],
    sink: &'a [f64; 3],
    mode: LowerBoundaryMode,
    fixed_head_flux: f64,
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials,
        sink,
        root_uptake: None,
        timestep: 0.25,
        target_time: 0.25,
        face_averaging: FaceAveraging::Arithmetic,
        lower_row: BoundaryRow {
            lower: 0.0,
            diagonal: 0.0,
            upper: 0.0,
            rhs: 0.0,
        },
        lower_boundary_mode: mode,
        upper_row: BoundaryRow {
            lower: 0.0,
            diagonal: 0.0,
            upper: 0.0,
            rhs: 0.0,
        },
        upper_condition: BoundaryCondition::Dirichlet { value: 0.0 },
        lower_boundary_flux: fixed_head_flux,
        upper_boundary_flux: 0.0,
    }
}

#[test]
fn fixed_head_preserves_caller_supplied_signed_exchange() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 1.0).unwrap();
    let model = Model;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let report = advance_water_step(
        &mut state,
        &mut workspace,
        input(
            &mesh,
            &materials,
            &sink,
            LowerBoundaryMode::FixedHead { head: 0.0 },
            0.4,
        ),
        policy(),
    )
    .unwrap();
    assert_eq!(
        report.lower_boundary_mode,
        LowerBoundaryMode::FixedHead { head: 0.0 }
    );
    assert!((report.balance.soil_boundary_flux - 0.1).abs() < 1e-10);
}

#[test]
fn prescribed_flux_sign_is_used_in_row_and_balance() {
    for flux in [0.3, -0.2] {
        let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 1.0).unwrap();
        let model = Model;
        let materials: [&dyn HydraulicModel; 1] = [&model];
        let sink = [0.0; 3];
        let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
        let mut workspace = WaterWorkspace::new(3).unwrap();
        let report = advance_water_step(
            &mut state,
            &mut workspace,
            input(
                &mesh,
                &materials,
                &sink,
                LowerBoundaryMode::PrescribedFlux { flux },
                0.0,
            ),
            policy(),
        )
        .unwrap();
        assert!((report.balance.soil_boundary_flux - flux * 0.25).abs() < 1e-10);
        assert_eq!(
            report.lower_boundary_mode,
            LowerBoundaryMode::PrescribedFlux { flux }
        );
    }
}

#[test]
fn free_drainage_uses_source_rhs_sign_as_inward_negative_flux() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 1.0).unwrap();
    let model = Model;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let report = advance_water_step(
        &mut state,
        &mut workspace,
        input(
            &mesh,
            &materials,
            &sink,
            LowerBoundaryMode::FreeDrainage,
            0.0,
        ),
        policy(),
    )
    .unwrap();
    assert_eq!(report.lower_boundary_mode, LowerBoundaryMode::FreeDrainage);
    assert!((report.balance.soil_boundary_flux + 0.25).abs() < 1e-10);
}

#[test]
fn invalid_and_unsupported_modes_fail_before_state_mutation() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 1.0).unwrap();
    let model = Model;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let original = state.clone();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    for (mode, expected) in [
        (
            LowerBoundaryMode::PrescribedFlux { flux: f64::NAN },
            WaterStepError::InvalidLowerBoundary { name: "flux" },
        ),
        (
            LowerBoundaryMode::FixedHead {
                head: f64::INFINITY,
            },
            WaterStepError::InvalidLowerBoundary { name: "head" },
        ),
        (
            LowerBoundaryMode::Unsupported(UnsupportedLowerBoundaryMode::SeepageFace),
            WaterStepError::UnsupportedLowerBoundary {
                mode: "seepage_face",
            },
        ),
        (
            LowerBoundaryMode::Unsupported(UnsupportedLowerBoundaryMode::WaterTable),
            WaterStepError::UnsupportedLowerBoundary {
                mode: "water_table",
            },
        ),
    ] {
        assert_eq!(
            advance_water_step(
                &mut state,
                &mut workspace,
                input(&mesh, &materials, &sink, mode, 0.0),
                policy(),
            ),
            Err(expected)
        );
        assert_eq!(state, original);
    }
}

#[test]
fn rejected_lower_boundary_trial_does_not_mutate_accepted_state() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 1.0).unwrap();
    let model = Model;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let original = state.clone();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let strict = WaterStepPolicy {
        max_iterations: 1,
        head_tolerance: 1e-30,
        water_tolerance: 1e-30,
        minimum_timestep: 0.2,
        retry_factor: 2.0,
        minimum_pivot: 1e-12,
    };
    assert!(matches!(
        advance_water_step(
            &mut state,
            &mut workspace,
            input(
                &mesh,
                &materials,
                &sink,
                LowerBoundaryMode::PrescribedFlux { flux: 0.3 },
                0.0
            ),
            strict,
        ),
        Err(WaterStepError::NonConverged { .. })
    ));
    assert_eq!(state, original);
}

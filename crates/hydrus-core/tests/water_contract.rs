use hydrus_core::hydraulics::{
    BoundaryCondition, BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, Mesh1D,
};
use hydrus_core::{
    advance_water_step, HydraulicError, HydraulicModel, HydraulicModelSelector, LowerBoundaryMode,
    WaterProductConfig, WaterState, WaterStepError, WaterStepInput, WaterStepPolicy,
    WaterWorkspace,
};

struct TestModel;

impl HydraulicModel for TestModel {
    fn theta(&self, head: f64) -> Result<f64, HydraulicError> {
        Ok(head)
    }
    fn conductivity(&self, _head: f64) -> Result<f64, HydraulicError> {
        Ok(1.0)
    }
    fn capacity(&self, _head: f64) -> Result<f64, HydraulicError> {
        Ok(1.0)
    }
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

#[test]
fn accepts_only_single_domain_model_zero_configuration() {
    let config = WaterProductConfig::new(0, 1).unwrap();
    assert_eq!(
        config.hydraulic_model(),
        HydraulicModelSelector::VanGenuchten
    );
    assert_eq!(config.domain_count(), 1);
}

#[test]
fn rejects_unsupported_selector_and_domain_count_with_typed_errors() {
    assert_eq!(
        WaterProductConfig::new(8, 1),
        Err(WaterStepError::Hydraulic(
            HydraulicError::UnsupportedSelector { selector: 8 }
        ))
    );
    assert_eq!(
        WaterProductConfig::new(0, 2),
        Err(WaterStepError::UnsupportedConfiguration {
            name: "domain_count",
            value: 2
        })
    );
}

#[test]
fn invalid_dimensions_do_not_mutate_accepted_state() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap();
    let model = TestModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let input = WaterStepInput {
        mesh: &mesh,
        materials: &materials,
        sink: &sink,
        timestep: 1.0,
        target_time: 2.0,
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
    };
    let policy = WaterStepPolicy {
        max_iterations: 2,
        head_tolerance: 1e-8,
        water_tolerance: 1e-8,
        minimum_timestep: 0.1,
        retry_factor: 2.0,
        minimum_pivot: 1e-12,
    };
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let original = state.clone();
    let mut workspace = WaterWorkspace::new(4).unwrap();
    assert_eq!(
        advance_water_step(&mut state, &mut workspace, input, policy),
        Err(WaterStepError::DimensionMismatch)
    );
    assert_eq!(state, original);
}

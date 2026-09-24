use hydrus_core::hydraulics::{
    BoundaryCondition, BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, Mesh1D,
};
use hydrus_core::{
    advance_water_step, HydraulicError, HydraulicModel, WaterState, WaterStepError, WaterStepInput,
    WaterStepPolicy, WaterWorkspace,
};

struct LinearModel {
    conductivity: f64,
    conductivity_slope: f64,
    theta_offset: f64,
    capacity: f64,
}

impl HydraulicModel for LinearModel {
    fn theta(&self, head: f64) -> Result<f64, HydraulicError> {
        Ok(self.theta_offset + self.capacity * head)
    }
    fn conductivity(&self, head: f64) -> Result<f64, HydraulicError> {
        Ok(self.conductivity + self.conductivity_slope * head)
    }
    fn capacity(&self, _head: f64) -> Result<f64, HydraulicError> {
        Ok(self.capacity)
    }
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: self.theta(head)?,
            conductivity: self.conductivity(head)?,
            capacity: self.capacity(head)?,
        })
    }
    fn derivatives(&self, _head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        Ok(HydraulicDerivatives {
            dtheta_dhead: self.capacity,
            dconductivity_dhead: self.conductivity_slope,
            dcapacity_dhead: 0.0,
        })
    }
}

fn mesh() -> Mesh1D {
    Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap()
}

fn policy() -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations: 4,
        head_tolerance: 1e-12,
        water_tolerance: 1e-12,
        minimum_timestep: 0.1,
        retry_factor: 3.0,
        minimum_pivot: 1e-12,
    }
}

fn input<'a>(
    mesh: &'a Mesh1D,
    materials: &'a [&'a dyn HydraulicModel],
    sink: &'a [f64],
    timestep: f64,
    boundary_flux: f64,
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials,
        sink,
        timestep,
        face_averaging: FaceAveraging::Arithmetic,
        lower_row: BoundaryRow {
            lower: 0.0,
            diagonal: 0.0,
            upper: 0.0,
            rhs: 0.0,
        },
        lower_condition: BoundaryCondition::Dirichlet { value: 0.0 },
        upper_row: BoundaryRow {
            lower: 0.0,
            diagonal: 0.0,
            upper: 0.0,
            rhs: 0.0,
        },
        upper_condition: BoundaryCondition::Dirichlet { value: 2.0 },
        lower_boundary_flux: boundary_flux,
        upper_boundary_flux: 0.0,
    }
}

fn close(left: f64, right: f64) {
    assert!((left - right).abs() < 1e-12, "{left} != {right}");
}

#[test]
fn converges_a_deterministic_picard_step_and_reports_balance() {
    let mesh = mesh();
    let model = LinearModel {
        conductivity: 1.0,
        conductivity_slope: 0.0,
        theta_offset: 0.0,
        capacity: 1.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let report = advance_water_step(
        &mut state,
        &mut workspace,
        input(&mesh, &materials, &sink, 1.0, 8.0 / 3.0),
        policy(),
    )
    .unwrap();

    assert_eq!(report.iterations, 2);
    close(state.heads()[1], 2.0 / 3.0);
    close(state.water_contents()[1], 2.0 / 3.0);
    close(report.balance.absolute_residual, 0.0);
    close(report.balance.normalized_residual, 0.0);
    assert!(report.minimum_absolute_pivot.is_finite());
    assert_eq!(state.accounting().accepted_steps, 1);
}

#[test]
fn zero_balance_denominator_normalizes_to_the_absolute_residual() {
    let mesh = mesh();
    let model = LinearModel {
        conductivity: 1.0,
        conductivity_slope: 0.0,
        theta_offset: 0.0,
        capacity: 1.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let mut zero = policy();
    zero.max_iterations = 1;
    let stationary = WaterStepInput {
        upper_condition: BoundaryCondition::Dirichlet { value: 0.0 },
        ..input(&mesh, &materials, &sink, 1.0, 0.0)
    };
    let report = advance_water_step(&mut state, &mut workspace, stationary, zero).unwrap();
    close(report.balance.absolute_residual, 0.0);
    close(report.balance.normalized_residual, 0.0);
}

#[test]
fn material_mapping_selects_model_and_material_count_is_validated() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0, 1, 0], None, 2, 0, 0.0).unwrap();
    let first = LinearModel {
        conductivity: 1.0,
        conductivity_slope: 0.0,
        theta_offset: 0.0,
        capacity: 1.0,
    };
    let second = LinearModel {
        conductivity: 9.0,
        conductivity_slope: 0.0,
        theta_offset: 10.0,
        capacity: 2.0,
    };
    let materials: [&dyn HydraulicModel; 2] = [&first, &second];
    let sink = [0.0; 3];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let request = input(&mesh, &materials, &sink, 1.0, 0.0);
    advance_water_step(&mut state, &mut workspace, request, policy()).unwrap();
    assert!(state.water_contents()[1] >= 10.0);

    let insufficient: [&dyn HydraulicModel; 1] = [&first];
    let invalid = input(&mesh, &insufficient, &sink, 1.0, 0.0);
    assert_eq!(
        advance_water_step(&mut state, &mut workspace, invalid, policy()),
        Err(WaterStepError::DimensionMismatch)
    );
}

#[test]
fn subsequent_picard_pass_uses_trial_head_hydraulics_and_theta() {
    let mesh = mesh();
    let constant = LinearModel {
        conductivity: 1.0,
        conductivity_slope: 0.0,
        theta_offset: 0.0,
        capacity: 1.0,
    };
    let changing = LinearModel {
        conductivity: 1.0,
        conductivity_slope: 0.5,
        theta_offset: 0.0,
        capacity: 1.0,
    };
    let constant_materials: [&dyn HydraulicModel; 1] = [&constant];
    let changing_materials: [&dyn HydraulicModel; 1] = [&changing];
    let sink = [0.0; 3];
    let mut constant_state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut changing_state = constant_state.clone();
    let mut constant_workspace = WaterWorkspace::new(3).unwrap();
    let mut changing_workspace = WaterWorkspace::new(3).unwrap();
    let mut nonlinear_policy = policy();
    nonlinear_policy.max_iterations = 40;
    let constant_report = advance_water_step(
        &mut constant_state,
        &mut constant_workspace,
        input(&mesh, &constant_materials, &sink, 1.0, 8.0 / 3.0),
        nonlinear_policy,
    )
    .unwrap();
    let changing_report = advance_water_step(
        &mut changing_state,
        &mut changing_workspace,
        input(&mesh, &changing_materials, &sink, 1.0, 8.0 / 3.0),
        nonlinear_policy,
    )
    .unwrap();

    assert!(changing_report.iterations >= 3);
    assert_ne!(changing_state.heads()[1], constant_state.heads()[1]);
    close(
        changing_state.water_contents()[1],
        changing_state.heads()[1],
    );
    assert_ne!(changing_report.max_water_residual, 0.0);
    assert_eq!(constant_report.iterations, 2);
}

#[test]
fn public_contract_has_no_filesystem_surface() {
    let source = include_str!("../src/water_solver.rs");
    assert!(!source.contains("std::fs"));
    assert!(!source.contains("File::"));
}

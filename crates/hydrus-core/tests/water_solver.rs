use hydrus_core::hydraulics::{
    assemble_reset_stencil, BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, Mesh1D,
    ResetStencilInput,
};
use hydrus_core::{
    advance_water_step, HydraulicError, HydraulicModel, LowerBoundaryMode, UpperBoundary,
    WaterState, WaterStepError, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

mod common;

use common::mesh;

struct LinearModel {
    conductivity: f64,
    conductivity_slope: f64,
    theta_offset: f64,
    capacity: f64,
}

impl HydraulicModel for LinearModel {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: self.theta_offset + self.capacity * head,
            conductivity: self.conductivity + self.conductivity_slope * head,
            capacity: self.capacity,
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
        root_uptake: None,
        timestep,
        target_time: 10.0,
        face_averaging: FaceAveraging::Arithmetic,
        lower: LowerBoundaryMode::FixedHead {
            head: 0.0,
            row: BoundaryRow {
                lower: 0.0,
                diagonal: 0.0,
                upper: 0.0,
                rhs: 0.0,
            },
            flux: boundary_flux,
        },
        upper: UpperBoundary::Head {
            head: 2.0,
            row: BoundaryRow {
                lower: 0.0,
                diagonal: 0.0,
                upper: 0.0,
                rhs: 0.0,
            },
            flux: 0.0,
        },
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
        input(&mesh, &materials, &sink, 1.0, 5.0 / 3.0),
        policy(),
    )
    .unwrap();

    assert_eq!(report.iterations, 2);
    close(state.heads()[1], 2.0 / 3.0);
    close(state.water_contents()[1], 2.0 / 3.0);
    close(report.balance.storage_change, 5.0 / 3.0);
    close(report.balance.absolute_residual, 0.0);
    close(report.balance.normalized_residual, 0.0);
    assert!(report.minimum_absolute_pivot.is_finite());
    assert_eq!(state.accounting().accepted_steps, 1);
}

#[test]
fn solver_interior_row_matches_the_reset_stencil_row() {
    let mesh = mesh();
    let model = LinearModel {
        conductivity: 1.0,
        conductivity_slope: 0.0,
        theta_offset: 0.2,
        capacity: 0.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0, 0.25, 0.0];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let report = advance_water_step(
        &mut state,
        &mut workspace,
        input(&mesh, &materials, &sink, 1.0, 0.0),
        policy(),
    )
    .unwrap();

    let row = assemble_reset_stencil(
        &mesh,
        ResetStencilInput {
            conductivities: &[1.0; 3],
            capacities: &[0.0; 3],
            h_new: &[0.0; 3],
            theta_new: &[0.2; 3],
            theta_old: &[0.0; 3],
            sink: &sink,
            timestep: 1.0,
            face_averaging: FaceAveraging::Arithmetic,
        },
    )
    .unwrap()[0];
    // Both callers use lower=-1, diagonal=2, upper=-1, rhs=-0.2-0.25.
    let stencil_solution = (row.rhs - row.upper * 2.0) / row.diagonal;
    close(state.heads()[1], stencil_solution);
    close(report.balance.storage_change, 0.4);
}

#[test]
fn nonuniform_mesh_balance_uses_control_volumes() {
    let mesh = Mesh1D::new(vec![0.0, 1.0, 4.0, 8.0], vec![0, 1, 2, 3], None, 4, 0, 0.0).unwrap();
    let models = [1.0, 2.0, 3.0, 4.0].map(|theta_offset| LinearModel {
        conductivity: 1.0,
        conductivity_slope: 0.0,
        theta_offset,
        capacity: 0.0,
    });
    let materials: [&dyn HydraulicModel; 4] = [&models[0], &models[1], &models[2], &models[3]];
    let sink = [1.0, 2.0, 3.0, 4.0];
    let mut state = WaterState::new(0.0, vec![0.0; 4], vec![0.0; 4]).unwrap();
    let mut workspace = WaterWorkspace::new(4).unwrap();
    let request = WaterStepInput {
        upper: UpperBoundary::Head {
            head: 0.0,
            row: BoundaryRow {
                lower: 0.0,
                diagonal: 0.0,
                upper: 0.0,
                rhs: 0.0,
            },
            flux: -0.5,
        },
        ..input(&mesh, &materials, &sink, 2.0, 2.0)
    };
    let report = advance_water_step(&mut state, &mut workspace, request, policy()).unwrap();

    // Widths are [0.5, 2, 3.5, 2], so storage is 23 and sink is 23 * 2 = 46.
    // The old unweighted sums would be 10 for storage and 10 * 2 for sink.
    close(report.balance.storage_change, 23.0);
    close(report.balance.sink, 46.0);
    close(report.balance.boundary_flux, 3.0);
    close(report.balance.absolute_residual, 66.0);
    close(report.balance.normalized_residual, 11.0 / 12.0);
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
        upper: UpperBoundary::Head {
            head: 0.0,
            row: BoundaryRow {
                lower: 0.0,
                diagonal: 0.0,
                upper: 0.0,
                rhs: 0.0,
            },
            flux: 0.0,
        },
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

#[test]
fn clips_requested_step_to_target_and_lands_exactly() {
    let mesh = mesh();
    let model = LinearModel {
        conductivity: 1.0,
        conductivity_slope: 0.0,
        theta_offset: 0.0,
        capacity: 0.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut state = WaterState::new(2.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let request = WaterStepInput {
        target_time: 2.25,
        ..input(&mesh, &materials, &sink, 1.0, 0.0)
    };
    let report = advance_water_step(&mut state, &mut workspace, request, policy()).unwrap();
    assert_eq!(report.timestep, 0.25);
    assert_eq!(report.accepted_time, 2.25);
    assert_eq!(state.time(), 2.25);
}

#[test]
fn target_beyond_requested_step_advances_one_step_and_can_be_repeated() {
    let mesh = mesh();
    let model = LinearModel {
        conductivity: 1.0,
        conductivity_slope: 0.0,
        theta_offset: 0.0,
        capacity: 0.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let request = WaterStepInput {
        target_time: 2.5,
        ..input(&mesh, &materials, &sink, 1.0, 0.0)
    };
    let first = advance_water_step(&mut state, &mut workspace, request, policy()).unwrap();
    assert_eq!(first.timestep, 1.0);
    assert_eq!(first.accepted_time, 1.0);
    let second = advance_water_step(&mut state, &mut workspace, request, policy()).unwrap();
    assert_eq!(second.timestep, 1.0);
    let third = advance_water_step(&mut state, &mut workspace, request, policy()).unwrap();
    assert_eq!(third.timestep, 0.5);
    assert_eq!(third.accepted_time, 2.5);
}

#[test]
fn rejects_nonfuture_and_nonfinite_targets_without_mutation() {
    let mesh = mesh();
    let model = LinearModel {
        conductivity: 1.0,
        conductivity_slope: 0.0,
        theta_offset: 0.0,
        capacity: 0.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 3];
    let mut state = WaterState::new(2.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let original = state.clone();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    for target_time in [2.0, 1.0] {
        let request = WaterStepInput {
            target_time,
            ..input(&mesh, &materials, &sink, 1.0, 0.0)
        };
        assert!(matches!(
            advance_water_step(&mut state, &mut workspace, request, policy()),
            Err(WaterStepError::InvalidTargetTime { .. })
        ));
        assert_eq!(state, original);
    }
    let request = WaterStepInput {
        target_time: f64::NAN,
        ..input(&mesh, &materials, &sink, 1.0, 0.0)
    };
    assert!(matches!(
        advance_water_step(&mut state, &mut workspace, request, policy()),
        Err(WaterStepError::NonFiniteInput {
            name: "target_time"
        })
    ));
    assert_eq!(state, original);
}

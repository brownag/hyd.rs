use hydrus_core::hydraulics::{BoundaryCondition, BoundaryRow, FaceAveraging, Mesh1D};
use hydrus_core::{
    advance_water_step, WaterState, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

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

fn input<'a>(mesh: &'a Mesh1D, timestep: f64, boundary_flux: f64) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        conductivities: &[1.0, 1.0, 1.0],
        capacities: &[1.0, 1.0, 1.0],
        sink: &[0.0, 0.0, 0.0],
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
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let report = advance_water_step(
        &mut state,
        &mut workspace,
        input(&mesh, 1.0, 8.0 / 3.0),
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
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let mut zero = policy();
    zero.max_iterations = 1;
    let stationary = WaterStepInput {
        upper_condition: BoundaryCondition::Dirichlet { value: 0.0 },
        ..input(&mesh, 1.0, 0.0)
    };
    let report = advance_water_step(&mut state, &mut workspace, stationary, zero).unwrap();
    close(report.balance.absolute_residual, 0.0);
    close(report.balance.normalized_residual, 0.0);
}

#[test]
fn public_contract_has_no_filesystem_surface() {
    let source = include_str!("../src/water_solver.rs");
    assert!(!source.contains("std::fs"));
    assert!(!source.contains("File::"));
}

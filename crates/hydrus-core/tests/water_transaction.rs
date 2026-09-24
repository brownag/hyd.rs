use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use hydrus_core::hydraulics::{BoundaryCondition, BoundaryRow, FaceAveraging, Mesh1D};
use hydrus_core::{
    advance_water_step, WaterState, WaterStepError, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

struct CountingAllocator;
static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn mesh() -> Mesh1D {
    Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap()
}
fn policy(max_iterations: usize, minimum_timestep: f64) -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations,
        head_tolerance: 0.5,
        water_tolerance: 0.5,
        minimum_timestep,
        retry_factor: 3.0,
        minimum_pivot: 1e-12,
    }
}
fn input<'a>(mesh: &'a Mesh1D, timestep: f64) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        conductivities: &[1.0; 3],
        capacities: &[0.0, 1.0, 0.0],
        sink: &[0.0; 3],
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
    let invalid = WaterStepInput {
        capacities: &[1.0, f64::NAN, 1.0],
        ..input(&mesh, 1.0)
    };
    assert!(matches!(
        advance_water_step(&mut state, &mut workspace, invalid, policy(1, 0.1)),
        Err(WaterStepError::NonFiniteInput { .. })
    ));
    assert_eq!(state, original);
}

#[test]
fn fixed_size_steps_allocate_nothing_after_setup() {
    let mesh = mesh();
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let stationary = WaterStepInput {
        upper_condition: BoundaryCondition::Dirichlet { value: 0.0 },
        ..input(&mesh, 1.0)
    };
    let strict = WaterStepPolicy {
        head_tolerance: 1e-12,
        water_tolerance: 1e-12,
        ..policy(1, 0.1)
    };
    advance_water_step(&mut state, &mut workspace, stationary, strict).unwrap();
    ALLOCATIONS.store(0, Ordering::Relaxed);
    COUNTING.store(true, Ordering::Relaxed);
    let report = advance_water_step(&mut state, &mut workspace, stationary, strict).unwrap();
    COUNTING.store(false, Ordering::Relaxed);
    assert_eq!(report.retries, 0);
    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), 0);
}

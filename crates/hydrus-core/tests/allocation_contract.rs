use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use hydrus_core::hydraulics::{BoundaryCondition, BoundaryRow, FaceAveraging, Mesh1D};
use hydrus_core::{
    advance_water_step, WaterState, WaterStepInput, WaterStepPolicy, WaterWorkspace,
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
    let report = advance_water_step(&mut state, &mut workspace, stationary, strict);
    COUNTING.store(false, Ordering::Relaxed);
    let report = report.unwrap();
    assert_eq!(report.retries, 0);
    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), 0);
}

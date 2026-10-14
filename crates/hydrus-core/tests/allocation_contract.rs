use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use hydrus_core::hydraulics::{
    BoundaryCondition, BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, Mesh1D,
};
use hydrus_core::{
    advance_water_step, HydraulicError, HydraulicModel, LowerBoundaryMode, WaterState,
    WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

struct AllocationModel {
    conductivity: f64,
    capacity: f64,
}

impl HydraulicModel for AllocationModel {
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

fn input<'a>(
    mesh: &'a Mesh1D,
    materials: &'a [&'a dyn HydraulicModel],
    timestep: f64,
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials,
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
fn fixed_size_steps_allocate_nothing_after_setup() {
    let mesh = mesh();
    let model = AllocationModel {
        conductivity: 1.0,
        capacity: 1.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let stationary = WaterStepInput {
        upper_condition: BoundaryCondition::Dirichlet { value: 0.0 },
        ..input(&mesh, &materials, 1.0)
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

    let lower_model = AllocationModel {
        conductivity: 1.0,
        capacity: 0.0,
    };
    let interior_model = AllocationModel {
        conductivity: 1.0,
        capacity: 1.0,
    };
    let upper_model = AllocationModel {
        conductivity: 1.0,
        capacity: 0.0,
    };
    let retry_mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0, 1, 2], None, 3, 0, 0.0).unwrap();
    let retry_materials: [&dyn HydraulicModel; 3] = [&lower_model, &interior_model, &upper_model];
    let retry_request = input(&retry_mesh, &retry_materials, 9.0);
    let retry_policy = policy(1, 0.3);
    let mut retry_state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut retry_workspace = WaterWorkspace::new(3).unwrap();

    let mut warmup_state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut warmup_workspace = WaterWorkspace::new(3).unwrap();
    advance_water_step(
        &mut warmup_state,
        &mut warmup_workspace,
        retry_request,
        retry_policy,
    )
    .unwrap();

    ALLOCATIONS.store(0, Ordering::Relaxed);
    COUNTING.store(true, Ordering::Relaxed);
    let retry_report = advance_water_step(
        &mut retry_state,
        &mut retry_workspace,
        retry_request,
        retry_policy,
    );
    COUNTING.store(false, Ordering::Relaxed);
    let retry_report = retry_report.unwrap();
    let retry_allocations = ALLOCATIONS.load(Ordering::Relaxed);
    assert!(retry_report.retries > 0);
    assert_eq!(retry_allocations, 0);
}

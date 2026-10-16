use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, Mesh1D,
};
use hydrus_core::solute::{advance_solute_step_with_processes, SoluteProcessInput};
use hydrus_core::{
    advance_solute_step, advance_water_step, AcceptedWaterSnapshot, EquilibriumSorption,
    HydraulicError, HydraulicModel, KineticSorption, LowerBoundaryMode, SoluteBackend,
    SoluteBoundaries, SoluteBoundaryMode, SoluteConfig, SoluteSpecies, SoluteState,
    SoluteTransportInput, SoluteUnits, SoluteWorkspace, UpperBoundary, WaterState, WaterStepInput,
    WaterStepPolicy, WaterWorkspace,
};

mod common;

use common::{mesh, policy};

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
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        lower: LowerBoundaryMode::FixedHead {
            head: 0.0,
            row: BoundaryRow {
                lower: 0.0,
                diagonal: 0.0,
                upper: 0.0,
                rhs: 0.0,
            },
            flux: 0.0,
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

#[test]
fn fixed_size_steps_allocate_nothing_after_setup() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mesh = mesh();
    let model = AllocationModel {
        conductivity: 1.0,
        capacity: 1.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
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

#[test]
fn solute_steps_allocate_nothing_after_setup_for_all_backends() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0, 0, 0], None, 1, 0, 0.0).unwrap();
    let water_contents = [0.35, 0.35, 0.35];
    let face_fluxes = [0.1, 0.1, 0.1, 0.1];
    let water_snapshot = AcceptedWaterSnapshot {
        accepted_time: 1.0,
        water_contents: &water_contents,
        face_fluxes: &face_fluxes,
    };
    let start_water = [0.35, 0.35, 0.35];
    let dispersion = [0.05, 0.05];
    let transport_input = SoluteTransportInput {
        accepted_water: water_snapshot,
        start_water_contents: &start_water,
        face_dispersion: &dispersion,
        timestep: 1.0,
        minimum_pivot: 1e-12,
    };
    let bulk_density = [1.2, 1.2, 1.2];
    let process_input = SoluteProcessInput {
        bulk_density: &bulk_density,
        max_iterations: 10,
        nonlinear_tolerance: 1e-6,
    };

    for &backend in SoluteBackend::ALL {
        let plain_cfg = SoluteConfig::new(
            SoluteUnits {
                concentration: "mg/L".into(),
                sorbed_mass: "mg/kg".into(),
            },
            vec![SoluteSpecies {
                name: "tracer".into(),
                decay_rate: 0.0,
                equilibrium_sorption: EquilibriumSorption::None,
                kinetic_sorption: None,
            }],
            SoluteBoundaries {
                lower: SoluteBoundaryMode::Concentration { value: 1.0 },
                upper: SoluteBoundaryMode::Concentration { value: 0.0 },
            },
            1,
            &[],
        )
        .unwrap()
        .with_backend(backend);

        let mut state =
            SoluteState::new(&plain_cfg, &mesh, 0.0, vec![1.0; 3], vec![0.0; 3]).unwrap();
        let mut workspace = SoluteWorkspace::new(3, 1).unwrap();

        // Warmup step to ensure any one-time static caches are populated
        advance_solute_step(
            &mut state,
            &mut workspace,
            &mesh,
            &plain_cfg,
            transport_input,
        )
        .unwrap();

        // Fresh state with time=0.0 to match transport_input (accepted_time=1.0, dt=1.0)
        let mut state =
            SoluteState::new(&plain_cfg, &mesh, 0.0, vec![1.0; 3], vec![0.0; 3]).unwrap();
        ALLOCATIONS.store(0, Ordering::Relaxed);
        COUNTING.store(true, Ordering::Relaxed);
        let res = advance_solute_step(
            &mut state,
            &mut workspace,
            &mesh,
            &plain_cfg,
            transport_input,
        );
        COUNTING.store(false, Ordering::Relaxed);
        res.unwrap();
        let step_allocations = ALLOCATIONS.load(Ordering::Relaxed);
        assert_eq!(
            step_allocations, 0,
            "advance_solute_step allocated on {backend:?}"
        );

        // Process step with reaction and sorption
        let proc_cfg = SoluteConfig::new(
            SoluteUnits {
                concentration: "mg/L".into(),
                sorbed_mass: "mg/kg".into(),
            },
            vec![SoluteSpecies {
                name: "sorbing".into(),
                decay_rate: 0.01,
                equilibrium_sorption: EquilibriumSorption::Linear {
                    distribution_coefficient: 0.5,
                },
                kinetic_sorption: Some(KineticSorption {
                    capacity_coefficient: 0.2,
                    transfer_rate: 0.05,
                }),
            }],
            SoluteBoundaries {
                lower: SoluteBoundaryMode::Concentration { value: 1.0 },
                upper: SoluteBoundaryMode::Concentration { value: 0.0 },
            },
            1,
            &[],
        )
        .unwrap()
        .with_backend(backend);

        let mut proc_state =
            SoluteState::new(&proc_cfg, &mesh, 0.0, vec![1.0; 3], vec![0.0; 3]).unwrap();
        let mut proc_workspace = SoluteWorkspace::new(3, 1).unwrap();

        advance_solute_step_with_processes(
            &mut proc_state,
            &mut proc_workspace,
            &mesh,
            &proc_cfg,
            transport_input,
            Some(process_input),
        )
        .unwrap();

        let mut proc_state =
            SoluteState::new(&proc_cfg, &mesh, 0.0, vec![1.0; 3], vec![0.0; 3]).unwrap();
        ALLOCATIONS.store(0, Ordering::Relaxed);
        COUNTING.store(true, Ordering::Relaxed);
        let res_proc = advance_solute_step_with_processes(
            &mut proc_state,
            &mut proc_workspace,
            &mesh,
            &proc_cfg,
            transport_input,
            Some(process_input),
        );
        COUNTING.store(false, Ordering::Relaxed);
        res_proc.unwrap();
        let proc_allocations = ALLOCATIONS.load(Ordering::Relaxed);
        assert_eq!(
            proc_allocations, 0,
            "advance_solute_step_with_processes allocated on {backend:?}"
        );
    }
}

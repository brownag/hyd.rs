use hydrus_core::hydraulics::{
    BoundaryCondition, Evaluation, HydraulicDerivatives, HydraulicModel, Mesh1D,
};
use hydrus_core::{
    advance_water_step, feddes_reduction, LowerBoundaryMode, RootUptakeInput, RootUptakeModel,
    WaterState, WaterStepError, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

struct LinearModel {
    theta_offset: f64,
}

impl HydraulicModel for LinearModel {
    fn theta(&self, head: f64) -> Result<f64, hydrus_core::HydraulicError> {
        Ok(self.theta_offset + head)
    }
    fn conductivity(&self, _head: f64) -> Result<f64, hydrus_core::HydraulicError> {
        Ok(1.0)
    }
    fn capacity(&self, _head: f64) -> Result<f64, hydrus_core::HydraulicError> {
        Ok(1.0)
    }
    fn evaluate(&self, head: f64) -> Result<Evaluation, hydrus_core::HydraulicError> {
        Ok(Evaluation {
            theta: self.theta(head)?,
            conductivity: 1.0,
            capacity: 1.0,
        })
    }
    fn derivatives(&self, _head: f64) -> Result<HydraulicDerivatives, hydrus_core::HydraulicError> {
        Ok(HydraulicDerivatives {
            dtheta_dhead: 1.0,
            dconductivity_dhead: 0.0,
            dcapacity_dhead: 0.0,
        })
    }
}

fn config<'a>(root_weights: &'a [f64]) -> RootUptakeInput<'a> {
    RootUptakeInput {
        model: RootUptakeModel::Feddes,
        h0: 0.0,
        h1: -1.0,
        h2: -3.0,
        h3: -5.0,
        potential_demand: 12.0,
        compensation_threshold: 6.0,
        root_weights,
    }
}

fn mesh() -> Mesh1D {
    Mesh1D::new(vec![0.0, 1.0, 3.0], vec![0; 3], None, 1, 0, 0.0).unwrap()
}

fn step_input<'a>(
    mesh: &'a Mesh1D,
    materials: &'a [&'a dyn HydraulicModel],
    sink: &'a [f64],
    root_uptake: Option<&'a RootUptakeInput<'a>>,
    timestep: f64,
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials,
        sink,
        root_uptake,
        timestep,
        target_time: 2.0,
        face_averaging: hydrus_core::hydraulics::FaceAveraging::Arithmetic,
        lower_row: hydrus_core::hydraulics::BoundaryRow {
            lower: 0.0,
            diagonal: 0.0,
            upper: 0.0,
            rhs: 0.0,
        },
        lower_boundary_mode: LowerBoundaryMode::FixedHead { head: 0.0 },
        upper_row: hydrus_core::hydraulics::BoundaryRow {
            lower: 0.0,
            diagonal: 0.0,
            upper: 0.0,
            rhs: 0.0,
        },
        upper_condition: BoundaryCondition::Dirichlet { value: 0.0 },
        lower_boundary_flux: 0.0,
        upper_boundary_flux: 0.0,
    }
}

fn policy(max_iterations: usize, minimum_timestep: f64) -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations,
        head_tolerance: 1e-10,
        water_tolerance: 1e-10,
        minimum_timestep,
        retry_factor: 2.0,
        minimum_pivot: 1e-12,
    }
}

#[test]
fn feddes_curve_covers_stress_breakpoints_and_full_stress() {
    let roots = [1.0; 3];
    let input = config(&roots);
    for (head, expected) in [
        (0.0, 0.0),
        (-0.5, 0.5),
        (-1.0, 1.0),
        (-2.0, 1.0),
        (-3.0, 1.0),
        (-4.0, 0.5),
        (-5.0, 0.0),
        (-6.0, 0.0),
    ] {
        assert!((feddes_reduction(head, &input) - expected).abs() < 1e-12);
    }
}

#[test]
fn compensation_respects_nonuniform_mesh_integral_and_potential_demand() {
    let mesh = mesh();
    let model = LinearModel {
        theta_offset: 100.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let roots = [1.0; 3];
    let input = config(&roots);
    let mut sink = [0.0; 3];
    input
        .compute_into(&mesh, &materials, &[-2.0; 3], &[98.0; 3], 0.01, &mut sink)
        .unwrap();
    assert_eq!(sink, [2.0; 3]);
    // Control-volume widths are 0.5, 1.5, and 1.0; Omega=3, so
    // compensation by OmegaC=6 realizes half of the 12-unit demand.
    let weighted = sink[0] * 0.5 + sink[1] * 1.5 + sink[2];
    assert!((weighted - 6.0).abs() < 1e-12);
}

#[test]
fn available_water_caps_each_node_at_half_the_wilt_difference_per_timestep() {
    let mesh = mesh();
    let model = LinearModel { theta_offset: 8.0 };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let roots = [1.0; 3];
    let input = config(&roots);
    let mut sink = [0.0; 3];
    input
        .compute_into(&mesh, &materials, &[-2.0; 3], &[6.0; 3], 1.0, &mut sink)
        .unwrap();
    assert_eq!(sink, [1.5; 3]);
    let weighted = sink[0] * 0.5 + sink[1] * 1.5 + sink[2];
    assert!((weighted - 4.5).abs() < 1e-12);
}

#[test]
fn zero_stress_and_zero_root_distribution_produce_zero_uptake() {
    let mesh = mesh();
    let model = LinearModel {
        theta_offset: 100.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let roots = [1.0; 3];
    let mut input = config(&roots);
    let mut sink = [9.0; 3];
    input
        .compute_into(&mesh, &materials, &[1.0; 3], &[101.0; 3], 1.0, &mut sink)
        .unwrap();
    assert_eq!(sink, [0.0; 3]);

    let no_roots = [0.0; 3];
    input.root_weights = &no_roots;
    input
        .compute_into(&mesh, &materials, &[-2.0; 3], &[98.0; 3], 1.0, &mut sink)
        .unwrap();
    assert_eq!(sink, [0.0; 3]);
}

#[test]
fn invalid_breakpoints_demand_weights_and_unsupported_model_are_rejected() {
    let mesh = mesh();
    let model = LinearModel {
        theta_offset: 100.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let roots = [1.0; 3];
    let mut sink = [0.0; 3];
    let heads = [-2.0; 3];
    let water = [98.0; 3];

    let mut invalid = config(&roots);
    invalid.h1 = 1.0;
    assert!(matches!(
        invalid.compute_into(&mesh, &materials, &heads, &water, 1.0, &mut sink),
        Err(WaterStepError::InvalidRootUptake { .. })
    ));
    invalid = config(&roots);
    invalid.potential_demand = -1.0;
    assert!(matches!(
        invalid.compute_into(&mesh, &materials, &heads, &water, 1.0, &mut sink),
        Err(WaterStepError::InvalidRootUptake { .. })
    ));
    let negative_roots = [1.0, -1.0, 1.0];
    invalid = config(&negative_roots);
    assert!(matches!(
        invalid.compute_into(&mesh, &materials, &heads, &water, 1.0, &mut sink),
        Err(WaterStepError::InvalidRootUptake { .. })
    ));
    invalid = config(&roots);
    invalid.model = RootUptakeModel::Unsupported(7);
    assert!(matches!(
        invalid.compute_into(&mesh, &materials, &heads, &water, 1.0, &mut sink),
        Err(WaterStepError::UnsupportedRootUptake { option: 7 })
    ));
    assert!(matches!(
        config(&roots).compute_into(&mesh, &materials, &heads, &water, 0.0, &mut sink),
        Err(WaterStepError::InvalidRootUptake { name: "timestep" })
    ));
    assert!(matches!(
        config(&roots).compute_into(&mesh, &materials, &heads[..2], &water, 1.0, &mut sink),
        Err(WaterStepError::DimensionMismatch)
    ));
    let non_finite_roots = [1.0, f64::INFINITY, 1.0];
    assert!(matches!(
        config(&non_finite_roots).compute_into(&mesh, &materials, &heads, &water, 1.0, &mut sink),
        Err(WaterStepError::NonFiniteInput {
            name: "root_weights"
        })
    ));
}

#[test]
fn accepted_step_uses_trial_root_sink_in_weighted_balance() {
    let mesh = mesh();
    let model = LinearModel {
        theta_offset: 100.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let roots = [0.0, 1.0, 0.0];
    let root = RootUptakeInput {
        model: RootUptakeModel::Feddes,
        h0: 10.0,
        h1: 5.0,
        h2: -10.0,
        h3: -20.0,
        potential_demand: 0.01,
        compensation_threshold: 0.5,
        root_weights: &roots,
    };
    let baseline_sink = [0.0; 3];
    let input = step_input(&mesh, &materials, &baseline_sink, Some(&root), 0.1);
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![100.0; 3]).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let report = advance_water_step(&mut state, &mut workspace, input, policy(10, 0.001)).unwrap();
    assert!((report.balance.sink - 0.001).abs() < 1e-10);
    assert_eq!(state.accounting().accepted_steps, 1);
    assert!((state.accounting().cumulative_sink - report.balance.sink).abs() < 1e-12);
}

#[test]
fn rejected_root_trial_does_not_promote_state_or_sink_accounting() {
    let mesh = mesh();
    let model = LinearModel {
        theta_offset: 100.0,
    };
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let roots = [0.0, 1.0, 0.0];
    let root = RootUptakeInput {
        model: RootUptakeModel::Feddes,
        h0: 10.0,
        h1: 5.0,
        h2: -10.0,
        h3: -20.0,
        potential_demand: 10.0,
        compensation_threshold: 0.5,
        root_weights: &roots,
    };
    let baseline_sink = [0.0; 3];
    let input = step_input(&mesh, &materials, &baseline_sink, Some(&root), 0.1);
    let mut state = WaterState::new(0.0, vec![0.0; 3], vec![100.0; 3]).unwrap();
    let original = state.clone();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    assert!(matches!(
        advance_water_step(&mut state, &mut workspace, input, policy(1, 0.1)),
        Err(WaterStepError::NonConverged { .. })
    ));
    assert_eq!(state, original);
    assert_eq!(state.accounting().cumulative_sink, 0.0);
}

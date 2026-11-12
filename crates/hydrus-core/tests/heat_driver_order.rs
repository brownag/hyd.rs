//! Coupled water→heat driver: order, preflight rejection, and failure reporting.
//!
//! Water fixtures are copied from `water_solute_coupling.rs` (cm, days). Heat parameters are
//! the in-memory `TempMaterial` used by the `transport.rs` unit tests. Expected values are
//! independent recomputations, not Fortran results.

use hydrus_core::heat::capacity::TempMaterial;
use hydrus_core::heat::config::{HeatConfig, HeatConfigError, HeatConfigInput};
use hydrus_core::heat::driver::{CoupledHeatInput, HeatDriverError, HeatWaterDriver};
use hydrus_core::heat::transport::{solve_heat_transport, HeatTransportError, HeatTransportInput};
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, Mesh1D,
};
use hydrus_core::{
    advance_water_step, HydraulicError, LowerBoundaryMode, RootUptakeInput, RootUptakeModel,
    UpperBoundary, WaterState, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

const X: [f64; 5] = [0.0, 0.5, 1.5, 2.0, 3.0];
const HEADS: [f64; 5] = [-4.0, -2.5, -3.0, -1.0, -2.0];

/// Linear retention, unit conductivity, constant capacity (used for the uniform test).
struct LinearModel;

impl HydraulicModel for LinearModel {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: 1.0 + 0.1 * head,
            conductivity: 1.0,
            capacity: 0.1,
        })
    }

    fn derivatives(&self, _head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        Ok(HydraulicDerivatives {
            dtheta_dhead: 0.1,
            dconductivity_dhead: 0.0,
            dcapacity_dhead: 0.0,
        })
    }
}

/// Exponential conductivity (used for the order and failure tests).
struct ExponentialModel;

impl HydraulicModel for ExponentialModel {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: 1.0 + 0.1 * head,
            conductivity: (0.05 * head).exp(),
            capacity: 0.1,
        })
    }

    fn derivatives(&self, head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        Ok(HydraulicDerivatives {
            dtheta_dhead: 0.1,
            dconductivity_dhead: 0.05 * (0.05 * head).exp(),
            dcapacity_dhead: 0.0,
        })
    }
}

fn policy() -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations: 30,
        head_tolerance: 1e-12,
        water_tolerance: 1e-12,
        minimum_timestep: 0.001,
        retry_factor: 2.0,
        minimum_pivot: 1e-12,
    }
}

fn mesh(inclination_cosine: f64) -> Mesh1D {
    Mesh1D::new(X.to_vec(), vec![0; 5], None, 1, 0, inclination_cosine).unwrap()
}

fn control_width(node: usize) -> f64 {
    let below = if node == 0 {
        0.0
    } else {
        X[node] - X[node - 1]
    };
    let above = if node == 4 {
        0.0
    } else {
        X[node + 1] - X[node]
    };
    (below + above) / 2.0
}

/// Upper half-cell balance for the exponential fixture (gravity cosine 0.8, dt 0.2).
fn upper_row(sink: &[f64]) -> BoundaryRow {
    let dx = X[4] - X[3];
    let storage = control_width(4) * 0.1 / 0.2;
    BoundaryRow {
        lower: -1.0 / dx,
        diagonal: storage + 1.0 / dx,
        upper: 0.0,
        rhs: storage * HEADS[4] - 0.8 - sink[4] * control_width(4),
    }
}

/// Upper half-cell balance for the zero-gradient uniform column (cosine 0, H = -1, dt 0.2).
fn uniform_row() -> BoundaryRow {
    let dx = X[4] - X[3];
    let storage = control_width(4) * 0.1 / 0.2;
    BoundaryRow {
        lower: -1.0 / dx,
        diagonal: storage + 1.0 / dx,
        upper: 0.0,
        rhs: -storage,
    }
}

fn water_input<'a>(
    mesh: &'a Mesh1D,
    materials: &'a [&'a dyn HydraulicModel],
    sink: &'a [f64],
    lower_flux: f64,
    upper: UpperBoundary,
    target_time: f64,
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials,
        sink,
        root_uptake: None,
        timestep: 0.2,
        target_time,
        face_averaging: FaceAveraging::Arithmetic,
        lower: LowerBoundaryMode::PrescribedFlux { flux: lower_flux },
        upper,
    }
}

fn heat_config(campbell: bool) -> HeatConfigInput {
    HeatConfigInput {
        l_vapor: false,
        l_en_bal: false,
        i_dual_por: 0,
        l_w_dep: false,
        i_model: 0,
        domain_count: 1,
        k_top_t: 1,
        t_period: 0.0,
        k_bot_t: 1,
        campbell,
    }
}

fn material() -> TempMaterial {
    TempMaterial::from_tpar([0.5, 0.0, 0.0, 0.5, 0.0, 0.0, 2.0, 2.5, 4.0])
}

fn driver(campbell: bool, temps: Vec<f64>) -> HeatWaterDriver {
    HeatWaterDriver::new(
        heat_config(campbell),
        &X,
        &[0usize; 5],
        &[material()],
        0.0,
        temps,
        &[0.0; 5],
    )
    .unwrap()
}

fn coupled(top: f64, bottom: f64) -> CoupledHeatInput {
    CoupledHeatInput {
        top_temperature: top,
        bottom_temperature: bottom,
        x_conv: 1.0,
        t_conv: 1.0,
        minimum_pivot: 1e-12,
    }
}

fn initial_water() -> Vec<f64> {
    HEADS.iter().map(|head| 1.0 + 0.1 * head).collect()
}

#[test]
fn unsupported_config_rejected_before_water_step() {
    let base = heat_config(false);
    let temps = vec![10.0; 5];
    let try_new = |config: HeatConfigInput| {
        HeatWaterDriver::new(
            config,
            &X,
            &[0usize; 5],
            &[material()],
            0.0,
            temps.clone(),
            &[0.0; 5],
        )
    };
    assert!(matches!(
        try_new(HeatConfigInput {
            l_vapor: true,
            ..base
        }),
        Err(HeatDriverError::Config(
            HeatConfigError::VaporNotSupported { .. }
        ))
    ));
    assert!(matches!(
        try_new(HeatConfigInput {
            l_en_bal: true,
            ..base
        }),
        Err(HeatDriverError::Config(
            HeatConfigError::EnergyBalanceNotSupported { .. }
        ))
    ));
    assert!(matches!(
        try_new(HeatConfigInput {
            i_dual_por: 1,
            ..base
        }),
        Err(HeatDriverError::Config(
            HeatConfigError::DualPorosityNotSupported { .. }
        ))
    ));
    assert!(matches!(
        try_new(HeatConfigInput {
            l_w_dep: true,
            ..base
        }),
        Err(HeatDriverError::Config(
            HeatConfigError::TemperatureDependentWaterNotSupported { .. }
        ))
    ));
    assert!(matches!(
        try_new(HeatConfigInput { i_model: 1, ..base }),
        Err(HeatDriverError::Config(
            HeatConfigError::HydraulicModelNotSupported { .. }
        ))
    ));
    assert!(matches!(
        try_new(HeatConfigInput { k_top_t: 0, ..base }),
        Err(HeatDriverError::Config(
            HeatConfigError::TopBoundaryKindNotSupported { .. }
        ))
    ));
    assert!(matches!(
        try_new(HeatConfigInput {
            t_period: 1.0,
            ..base
        }),
        Err(HeatDriverError::Config(
            HeatConfigError::SinusoidalTopNotSupported { .. }
        ))
    ));
}

#[test]
fn preflight_rejections_leave_water_untouched() {
    let model = ExponentialModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 5];
    let mesh = mesh(0.8);
    let upper = UpperBoundary::Flux {
        flux: -0.1,
        row: upper_row(&sink),
    };
    let temps = vec![4.0, 6.0, 8.0, 10.0, 12.0];
    let roots = [0.0, 1.0, 1.0, 1.0, 0.0];
    let root = RootUptakeInput {
        model: RootUptakeModel::Feddes,
        h0: 0.0,
        h1: -1.0,
        h2: -3.0,
        h3: -5.0,
        potential_demand: 1.0,
        compensation_threshold: 0.5,
        root_weights: &roots,
    };
    let base = water_input(&mesh, &materials, &sink, 0.3, upper, 0.2);
    let mut heat = driver(false, temps.clone());

    // Root uptake is present.
    let mut state = WaterState::new(0.0, HEADS.to_vec(), initial_water()).unwrap();
    let original = state.clone();
    let mut workspace = WaterWorkspace::new(5).unwrap();
    let with_root = WaterStepInput {
        root_uptake: Some(&root),
        ..base
    };
    let error = heat
        .step(
            &mut state,
            &mut workspace,
            with_root,
            policy(),
            coupled(10.0, 0.0),
        )
        .unwrap_err();
    assert!(
        matches!(error, HeatDriverError::Preflight { .. }),
        "{error:?}"
    );
    assert_eq!(state, original);

    // Geometric face averaging.
    let geometric = WaterStepInput {
        face_averaging: FaceAveraging::Geometric,
        ..base
    };
    let error = heat
        .step(
            &mut state,
            &mut workspace,
            geometric,
            policy(),
            coupled(10.0, 0.0),
        )
        .unwrap_err();
    assert!(
        matches!(error, HeatDriverError::Preflight { .. }),
        "{error:?}"
    );
    assert_eq!(state, original);

    // Node count: heat column has 4 nodes, water has 5.
    let mut short = HeatWaterDriver::new(
        heat_config(false),
        &X[..4],
        &[0usize; 4],
        &[material()],
        0.0,
        temps[..4].to_vec(),
        &[0.0; 4],
    )
    .unwrap();
    let error = short
        .step(
            &mut state,
            &mut workspace,
            base,
            policy(),
            coupled(10.0, 0.0),
        )
        .unwrap_err();
    assert!(
        matches!(error, HeatDriverError::Preflight { .. }),
        "{error:?}"
    );
    assert_eq!(state, original);
}

#[test]
fn heat_uses_post_step_water() {
    let model = ExponentialModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 5];
    let mesh = mesh(0.8);
    let upper = UpperBoundary::Flux {
        flux: -0.1,
        row: upper_row(&sink),
    };
    let temps = vec![4.0, 6.0, 8.0, 10.0, 12.0];
    let pre_water = initial_water();
    let mut state = WaterState::new(0.0, HEADS.to_vec(), pre_water.clone()).unwrap();
    let mut workspace = WaterWorkspace::new(5).unwrap();
    let mut heat = driver(false, temps.clone());

    // Identical water step on a clone: source of the post-step water and face fluxes.
    let mut reference_state = state.clone();
    let mut reference_workspace = WaterWorkspace::new(5).unwrap();
    let reference = advance_water_step(
        &mut reference_state,
        &mut reference_workspace,
        water_input(&mesh, &materials, &sink, 0.3, upper, 0.2),
        policy(),
    )
    .unwrap();
    let post_water = reference.water_contents.to_vec();
    let faces = reference.face_fluxes.to_vec();
    let accepted_time = reference.accepted_time;
    let dt = reference.timestep;

    assert!(
        post_water
            .iter()
            .zip(&pre_water)
            .any(|(post, pre)| post != pre),
        "water must change for the order check to discriminate"
    );

    let report = heat
        .step(
            &mut state,
            &mut workspace,
            water_input(&mesh, &materials, &sink, 0.3, upper, 0.2),
            policy(),
            coupled(10.0, 0.0),
        )
        .unwrap();
    assert_eq!(report.accepted_time.to_bits(), accepted_time.to_bits());
    assert_eq!(heat.state().time().to_bits(), accepted_time.to_bits());

    // Node velocities, Fortran Veloc form (WATFLOW.FOR:732, :744-745, :769-771).
    let mut velocity = [0.0; 5];
    for i in 1..4 {
        let dxa = X[i + 1] - X[i];
        let dxb = X[i] - X[i - 1];
        velocity[i] = (faces[i + 1] * dxb + faces[i] * dxa) / (dxa + dxb);
    }
    velocity[0] = faces[1] + (X[1] - X[0]) / 2.0 * ((post_water[0] - pre_water[0]) / dt);
    velocity[4] = faces[4] - (X[4] - X[3]) / 2.0 * ((post_water[4] - pre_water[4]) / dt);

    let config = HeatConfig::new(heat_config(false)).unwrap();
    let mats = [material()];
    let indices = [0usize; 5];
    let zero = [0.0; 5];
    let mut expected = [0.0; 5];
    solve_heat_transport(
        config,
        HeatTransportInput {
            x: &X[..],
            dt,
            material_indices: &indices,
            materials: &mats,
            temp_old: &temps,
            water_old: &pre_water,
            water_new: &post_water,
            velocity_old: &zero,
            velocity_new: &velocity,
            sink: &sink,
            top_temperature: 10.0,
            bottom_temperature: 0.0,
            x_conv: 1.0,
            t_conv: 1.0,
            minimum_pivot: 1e-12,
        },
        &mut expected,
    )
    .unwrap();
    for (actual, want) in heat.state().temperatures().iter().zip(expected) {
        assert_eq!(actual.to_bits(), want.to_bits(), "{actual} != {want}");
    }
}

#[test]
fn uniform_column_zero_flux_stays_uniform() {
    for campbell in [false, true] {
        let model = LinearModel;
        let materials: [&dyn HydraulicModel; 1] = [&model];
        let sink = [0.0; 5];
        let mesh = mesh(0.0);
        let row = uniform_row();
        let mut state = WaterState::new(0.0, vec![-1.0; 5], vec![0.9; 5]).unwrap();
        let mut workspace = WaterWorkspace::new(5).unwrap();
        let mut heat = driver(campbell, vec![10.0; 5]);
        for step in 1..=3 {
            let target = 0.2 * f64::from(step);
            heat.step(
                &mut state,
                &mut workspace,
                water_input(
                    &mesh,
                    &materials,
                    &sink,
                    0.0,
                    UpperBoundary::Flux { flux: 0.0, row },
                    target,
                ),
                policy(),
                coupled(10.0, 10.0),
            )
            .unwrap();
        }
        for value in heat.state().temperatures() {
            assert!((value - 10.0).abs() < 1e-12, "campbell={campbell}: {value}");
        }
        assert_eq!(heat.state().time().to_bits(), state.time().to_bits());
    }
}

#[test]
fn heat_failure_after_water_success_is_reported() {
    let model = ExponentialModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 5];
    let mesh = mesh(0.8);
    let upper = UpperBoundary::Flux {
        flux: -0.1,
        row: upper_row(&sink),
    };
    let temps = vec![4.0, 6.0, 8.0, 10.0, 12.0];
    let mut state = WaterState::new(0.0, HEADS.to_vec(), initial_water()).unwrap();
    let mut workspace = WaterWorkspace::new(5).unwrap();
    let mut heat = driver(false, temps.clone());
    let failing = CoupledHeatInput {
        minimum_pivot: 1e30,
        ..coupled(10.0, 0.0)
    };

    let error = heat
        .step(
            &mut state,
            &mut workspace,
            water_input(&mesh, &materials, &sink, 0.3, upper, 0.2),
            policy(),
            failing,
        )
        .unwrap_err();
    assert!(
        matches!(
            error,
            HeatDriverError::HeatAfterWaterAccepted {
                source: HeatTransportError::Tridiagonal(_),
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(state.time().to_bits(), 0.2f64.to_bits());
    assert_eq!(heat.state().time().to_bits(), 0.0f64.to_bits());
    for (actual, want) in heat.state().temperatures().iter().zip(&temps) {
        assert_eq!(actual.to_bits(), want.to_bits());
    }
}

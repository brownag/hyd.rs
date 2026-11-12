//! Vapor coupled driver: pass order, single commit, accounting and pre-trial rejection.
//!
//! Expected values are independent recomputations from public solver entries, not Fortran results.
//! Failure injection after second water, second heat and diagnostics lives in the driver's inline tests.

use hydrus_core::heat::capacity::TempMaterial;
use hydrus_core::heat::config::HeatConfigInput;
use hydrus_core::heat::driver::CoupledHeatInput;
use hydrus_core::heat::transport::{solve_vapor_heat_transport, HeatTransportInput};
use hydrus_core::heat::vapor::adjust::VaporHeatAdjustmentInput;
use hydrus_core::heat::vapor::driver::{VaporDriverError, VaporWaterHeatDriver};
use hydrus_core::heat::vapor::state::VaporStateInput;
use hydrus_core::heat::vapor::{
    VaporConfig, VaporConfigInput, VaporError, VaporPrimitiveInput, VaporSourceMaterial,
};
use hydrus_core::hydraulics::{BoundaryRow, FaceAveraging, Mesh1D};
use hydrus_core::{
    HydraulicModel, LowerBoundaryMode, UpperBoundary, VanGenuchtenModel, VanGenuchtenParameters,
    WaterState, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

const X: [f64; 3] = [0.0, 50.0, 100.0];
const TEMPS: [f64; 3] = [10.0, 20.0, 30.0];

fn config() -> VaporConfig {
    VaporConfig::new(VaporConfigInput {
        heat: HeatConfigInput {
            l_vapor: true,
            l_en_bal: false,
            i_dual_por: 0,
            l_w_dep: false,
            i_model: 0,
            domain_count: 1,
            k_top_t: 1,
            t_period: 0.0,
            k_bot_t: 1,
            campbell: false,
        },
        water_enabled: true,
        heat_enabled: true,
        i_enhanc: 1,
        hysteresis: 0,
        chemistry: false,
        density_coupling: false,
        centrifugal: false,
        top_temperature: 30.0,
        bottom_temperature: 10.0,
        x_conv: 100.0,
        t_conv: 86400.0,
    })
    .unwrap()
}

fn row(value: f64) -> BoundaryRow {
    BoundaryRow {
        lower: 0.0,
        diagonal: 1.0,
        upper: 0.0,
        rhs: value,
    }
}

fn material() -> TempMaterial {
    TempMaterial::from_tpar([0.5, 0.0, 0.0, 0.5, 0.1, 0.2, 2.0, 0.0, 4.0])
}

fn heat() -> CoupledHeatInput {
    CoupledHeatInput {
        top_temperature: 30.0,
        bottom_temperature: 10.0,
        x_conv: 100.0,
        t_conv: 86400.0,
        minimum_pivot: 1.0e-14,
    }
}

fn run(
    sink: [f64; 3],
    f: impl FnOnce(
        &mut VaporWaterHeatDriver,
        &mut WaterState,
        &mut WaterWorkspace,
        WaterStepInput<'_>,
        &[VaporSourceMaterial<'_>],
    ),
) {
    let model =
        VanGenuchtenModel::new(VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0e-5).unwrap())
            .unwrap();
    let mesh = Mesh1D::new(X.to_vec(), vec![0; 3], None, 1, 1, 1.0).unwrap();
    let heads = [-100.0; 3];
    let e = model.evaluate(-100.0).unwrap();
    let theta = [e.theta; 3];
    let mut driver = VaporWaterHeatDriver::new(
        config(),
        VaporStateInput {
            primitive: VaporPrimitiveInput {
                heads: &heads,
                temperatures: &TEMPS,
                liquid_water_contents: &theta,
                liquid_conductivities: &[e.conductivity; 3],
                liquid_capacities: &[e.capacity; 3],
                material_indices: &[0; 3],
                saturated_water_contents: &[0.45],
                x_conv: 100.0,
                t_conv: 86400.0,
            },
            coordinates: &X,
            gravity: 1.0,
            initial_head_predictor: &heads,
            initialization_timestep: 0.1,
            time: 0.0,
        },
        &[0; 3],
        &[material()],
    )
    .unwrap();
    let mut state = WaterState::new(0.0, heads.to_vec(), theta.to_vec()).unwrap();
    let mut workspace = WaterWorkspace::new(3).unwrap();
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let input = WaterStepInput {
        mesh: &mesh,
        materials: &materials,
        sink: &sink,
        root_uptake: None,
        timestep: 0.1,
        target_time: 0.1,
        face_averaging: FaceAveraging::Arithmetic,
        lower: LowerBoundaryMode::FixedHead {
            head: -90.0,
            row: row(-90.0),
            flux: 0.0,
        },
        upper: UpperBoundary::Head {
            head: -110.0,
            row: row(-110.0),
            flux: 0.0,
        },
    };
    let source = [VaporSourceMaterial {
        model: &model,
        table: None,
    }];
    f(&mut driver, &mut state, &mut workspace, input, &source);
}

fn policy() -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations: 20,
        head_tolerance: 1.0e-8,
        water_tolerance: 1.0e-8,
        minimum_timestep: 1.0e-6,
        retry_factor: 3.0,
        minimum_pivot: 1.0e-14,
    }
}

#[test]
fn second_pass_uses_first_heat_result_and_commits_once() {
    run([0.0; 3], |driver, state, workspace, input, source| {
        let old = driver.state().snapshot();
        let (old_v, old_vv, old_theta_v, old_temps) = (
            old.liquid_velocities.to_vec(),
            old.vapor_velocities.to_vec(),
            old.vapor_contents.to_vec(),
            old.temperatures.to_vec(),
        );
        let report = driver
            .step(state, workspace, input, policy(), source, heat())
            .unwrap();
        // Pass order and temperature inputs.
        assert_eq!(report.passes[0].temperature_estimate, old_temps);
        assert_eq!(
            report.passes[1].temperature_estimate,
            report.passes[0].temperatures
        );
        assert_eq!(report.passes[1].initial_heads, report.passes[0].heads);
        assert_eq!(report.passes[0].timestep, report.passes[1].timestep);
        assert_eq!(report.restarts, 0);
        // Independent pass-two heat recomputation from published pass data and the old snapshot.
        let mut expected = vec![0.0; 3];
        let adjust = VaporHeatAdjustmentInput {
            coordinates: &X,
            temperature_old: &old_temps,
            temperature_new_estimate: &report.passes[0].temperatures,
            vapor_velocity_old: &old_vv,
            vapor_velocity_new: &report.vapor_velocities,
            vapor_content_old: &old_theta_v,
            vapor_content_new: &report.vapor_contents_new,
            timestep: report.timestep,
            x_conv: 100.0,
            t_conv: 86400.0,
        };
        solve_vapor_heat_transport(
            config(),
            HeatTransportInput {
                x: &X,
                dt: report.timestep,
                material_indices: &[0; 3],
                materials: &[material()],
                temp_old: &old_temps,
                water_old: &report.liquid_contents_old,
                water_new: &report.liquid_contents_new,
                velocity_old: &old_v,
                velocity_new: &report.liquid_velocities,
                sink: &[0.0; 3],
                top_temperature: 30.0,
                bottom_temperature: 10.0,
                x_conv: 100.0,
                t_conv: 86400.0,
                minimum_pivot: 1.0e-14,
            },
            &adjust,
            &mut expected,
        )
        .unwrap();
        assert_eq!(report.passes[1].temperatures, expected);
        // One accepted time, count and accounting increment.
        assert_eq!(state.time(), 0.1);
        let accounting = state.accounting();
        assert_eq!(accounting.accepted_steps, 1);
        assert_eq!(
            accounting.cumulative_iterations as usize,
            report.passes[0].iterations + report.passes[1].iterations
        );
        // Liquid-only volume totals are deliberately not advanced on the vapor path.
        assert_eq!(accounting.cumulative_storage_change, 0.0);
        assert_eq!(accounting.cumulative_boundary_flux, 0.0);
        assert_eq!(accounting.cumulative_sink, 0.0);
        let accepted = driver.state().snapshot();
        assert_eq!(accepted.time, 0.1);
        assert_eq!(accepted.previous_timestep, Some(0.1));
        assert_eq!(accepted.temperatures, expected.as_slice());
        assert_eq!(accepted.heads, state.heads());
        // Stored predictor is the unit-ratio extrapolation on interior nodes only.
        let h = state.heads();
        assert_eq!(accepted.head_predictor[0], h[0]);
        assert_eq!(accepted.head_predictor[2], h[2]);
        assert_eq!(accepted.head_predictor[1], h[1] + (h[1] + 100.0));
        // A second step reuses the same workspace and advances once more.
        let mut next = input;
        next.target_time = 0.2;
        driver
            .step(state, workspace, next, policy(), source, heat())
            .unwrap();
        assert_eq!(state.accounting().accepted_steps, 2);
        assert_eq!(state.time(), 0.2);
    });
}

#[test]
fn invalid_input_is_rejected_before_any_water_trial() {
    run(
        [0.0, 1.0e-6, 0.0],
        |driver, state, workspace, input, source| {
            let water = state.clone();
            let vapor = driver.state().clone();
            let error = driver
                .step(state, workspace, input, policy(), source, heat())
                .unwrap_err();
            assert!(matches!(
                error,
                VaporDriverError::Vapor(VaporError::NonzeroSink { index: 1, .. })
            ));
            assert_eq!(*state, water);
            assert_eq!(*driver.state(), vapor);
        },
    );
    run([0.0; 3], |driver, state, workspace, input, source| {
        let mut bad = heat();
        bad.top_temperature = 31.0;
        let error = driver
            .step(state, workspace, input, policy(), source, bad)
            .unwrap_err();
        assert!(matches!(error, VaporDriverError::Preflight { .. }));
        assert_eq!(state.time(), 0.0);
    });
}

#[test]
fn first_heat_failure_rolls_back_everything() {
    run([0.0; 3], |driver, state, workspace, input, source| {
        let water = state.clone();
        let vapor = driver.state().clone();
        let before = format!("{workspace:?}");
        let mut singular = heat();
        singular.minimum_pivot = 1.0e30;
        let error = driver
            .step(state, workspace, input, policy(), source, singular)
            .unwrap_err();
        assert!(matches!(error, VaporDriverError::Heat { pass: 1, .. }));
        assert_eq!(*state, water);
        assert_eq!(*driver.state(), vapor);
        assert!(!driver.state().has_staged_candidate());
        assert_eq!(format!("{workspace:?}"), before);
    });
}

//! Column balance of the closed flux/flux pair with vapor on (task 29.3.1).
//!
//! One accepted step of `VaporWaterHeatDriver` with `PrescribedFlux` below and `Flux` above.
//! Both fluxes are positive into the domain (design section 6): lower `flux = rBot`, upper
//! `flux = -rTop`. Trapezoid-weighted (liquid + vapor) storage change must equal
//! `(rBot - rTop) * dt`; with both fluxes zero it must vanish.
//!
//! Tolerances are stated at the constants below.

use hydrus_core::heat::capacity::TempMaterial;
use hydrus_core::heat::config::HeatConfigInput;
use hydrus_core::heat::driver::CoupledHeatInput;
use hydrus_core::heat::vapor::driver::VaporWaterHeatDriver;
use hydrus_core::heat::vapor::state::VaporStateInput;
use hydrus_core::heat::vapor::{
    VaporConfig, VaporConfigInput, VaporPrimitiveInput, VaporSourceMaterial,
};
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicModel, Mesh1D, VanGenuchtenModel,
    VanGenuchtenParameters,
};
use hydrus_core::{
    LowerBoundaryMode, UpperBoundary, WaterState, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

const X_CONV: f64 = 100.0;
const T_CONV: f64 = 1.0 / 3600.0;
const DT: f64 = 0.1; // hours
/// Driven case: relative to |net_in * dt|, with the absolute floor below.
const DRIVEN_REL_TOL: f64 = 1e-9; // measured residual 5.7e-17 abs (3.8e-14 rel); ~1e5x margin
const DRIVEN_ABS_FLOOR: f64 = 1e-13;
/// Control (zero flux): absolute, on the content integral (cm).
const CONTROL_ABS_TOL: f64 = 1e-12; // measured 8.9e-16

fn placeholder_row() -> BoundaryRow {
    BoundaryRow {
        lower: 0.0,
        diagonal: 1.0,
        upper: 0.0,
        rhs: 0.0,
    }
}

/// Returns (storage change, net inflow over the step) for the given fluxes.
fn run(r_bot: f64, upper_flux: f64) -> (f64, f64) {
    let n = 21;
    let x: Vec<f64> = (0..n).map(|i| -20.0 + i as f64).collect();
    let heads = vec![-100.0; n];
    let temps = vec![20.0; n];
    let indices = vec![0usize; n];
    let analytic = VanGenuchtenModel::new(
        VanGenuchtenParameters::new(0.08, 0.43, 0.015, 1.875, 0.43).unwrap(),
    )
    .unwrap();
    let models: [&dyn HydraulicModel; 1] = [&analytic];
    let init: Evaluation = analytic.evaluate(-100.0).unwrap();
    let theta = vec![init.theta; n];
    let cond = vec![init.conductivity; n];
    let cap = vec![init.capacity; n];
    let mesh = Mesh1D::new(x.clone(), indices.clone(), None, 1, 0, 1.0).unwrap();
    let materials = vec![TempMaterial::from_tpar([
        0.60,
        0.02,
        0.5,
        1.1337408e12,
        1.8335808e12,
        7.1570304e12,
        2.48832e11,
        3.25296e11,
        5.41728e11,
    ])];
    let config = VaporConfig::new(VaporConfigInput {
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
        top_temperature: 20.0,
        bottom_temperature: 20.0,
        x_conv: X_CONV,
        t_conv: T_CONV,
    })
    .unwrap();
    let mut driver = VaporWaterHeatDriver::new(
        config,
        VaporStateInput {
            primitive: VaporPrimitiveInput {
                heads: &heads,
                temperatures: &temps,
                liquid_water_contents: &theta,
                liquid_conductivities: &cond,
                liquid_capacities: &cap,
                material_indices: &indices,
                saturated_water_contents: &[0.43],
                x_conv: X_CONV,
                t_conv: T_CONV,
            },
            coordinates: &x,
            gravity: 1.0,
            initial_head_predictor: &heads,
            initialization_timestep: DT,
            time: 0.0,
        },
        &indices,
        &materials,
    )
    .unwrap();
    let source = [VaporSourceMaterial {
        model: &analytic,
        table: None,
    }];
    let policy = WaterStepPolicy {
        max_iterations: 20,
        head_tolerance: 1.0,
        water_tolerance: 1e-3,
        minimum_timestep: 0.01,
        retry_factor: 3.0,
        minimum_pivot: 1e-12,
    };
    let coupled = CoupledHeatInput {
        top_temperature: 20.0,
        bottom_temperature: 20.0,
        x_conv: X_CONV,
        t_conv: T_CONV,
        minimum_pivot: 1e-12,
    };
    let sink = vec![0.0; n];
    let mut water = WaterState::new(0.0, heads.clone(), theta.clone()).unwrap();
    let mut workspace = WaterWorkspace::new(n).unwrap();
    let report = driver
        .step(
            &mut water,
            &mut workspace,
            WaterStepInput {
                mesh: &mesh,
                materials: &models,
                sink: &sink,
                root_uptake: None,
                timestep: DT,
                target_time: DT,
                face_averaging: FaceAveraging::Arithmetic,
                lower: LowerBoundaryMode::PrescribedFlux { flux: r_bot },
                upper: UpperBoundary::Flux {
                    flux: upper_flux,
                    row: placeholder_row(),
                },
            },
            policy,
            &source,
            coupled,
        )
        .expect("closed pair accepted with vapor on");
    assert_eq!(report.restarts, 0, "timestep must not be cut");
    assert!((report.timestep - DT).abs() < 1e-12);
    let total = |liquid: &[f64], vapor: &[f64]| -> f64 {
        (0..n)
            .map(|i| {
                let w = if i == 0 {
                    (x[1] - x[0]) / 2.0
                } else if i == n - 1 {
                    (x[n - 1] - x[n - 2]) / 2.0
                } else {
                    (x[i + 1] - x[i - 1]) / 2.0
                };
                w * (liquid[i] + vapor[i])
            })
            .sum()
    };
    let old = total(&report.liquid_contents_old, &report.vapor_contents_old);
    let new = total(&report.liquid_contents_new, &report.vapor_contents_new);
    (new - old, (r_bot + upper_flux) * report.timestep)
}

#[test]
fn driven_closed_pair_storage_change_equals_net_flux() {
    let (r_bot, r_top) = (0.02, 0.005); // rTop positive out
    let (d_storage, net_in) = run(r_bot, -r_top);
    let residual = (d_storage - net_in).abs();
    let tol = (DRIVEN_REL_TOL * net_in.abs()).max(DRIVEN_ABS_FLOOR);
    println!("driven: dStorage={d_storage:e} netIn={net_in:e} residual={residual:e} tol={tol:e}");
    assert!(net_in > 1e-4, "driven case must be non-trivial");
    assert!(d_storage > 1e-4);
    assert!(residual <= tol, "residual {residual:e} > tol {tol:e}");
}

#[test]
fn zero_flux_pair_conserves_storage() {
    let (d_storage, net_in) = run(0.0, 0.0);
    println!("control: dStorage={d_storage:e}");
    assert_eq!(net_in, 0.0);
    assert!(d_storage.abs() <= CONTROL_ABS_TOL, "dStorage {d_storage:e}");
}

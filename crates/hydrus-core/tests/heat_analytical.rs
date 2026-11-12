//! Analytical comparisons for the coupled water→heat driver (analytical-only, 24.5.1).
//!
//! Each closed form is derived by hand in its test comment. Tolerances are the M6 contract
//! values (planning/master/M6_HEAT_CONTRACT.md `## Tolerances`): TempN absolute 2e-4 degC.
//!
//! Scope limit: M6 has no nonzero-gradient conduction case through the driver. The top flux is
//! advective only (`HeatFl = Cw * tTop * (vNew + vOld) / 2`), so the top-flux case here is the
//! zero-flux (zero-gradient) top. No advection-diffusion closed form is included.
//!
//! Test-only inputs: the water timestep is pinned at `DT = 1e-4` and the water policy uses
//! `minimum_timestep = 1e-5`. These are chosen for this closed-form comparison only (the
//! transient case needs dt <= 1e-4 for Crank-Nicolson time error to stay below tolerance). The
//! fixture-based parity task keeps its own policy.
//!
//! Unit factors: the analytical cases use unit-free `x_conv = t_conv = 1`. That is the correct
//! setting for these closed forms and the constants (Cap, K, D) are not scaled. The fixture
//! parity task (24.5.2) uses the source factors `xConv = 100` (cm) and `tConv = 1/3600` (hours)
//! from `INPUT.FOR:190` and `:197`, so its Campbell conductivity carries `xConv/tConv^3` (SD-006).
//!
//! Water is held static: head H = 0, theta = 0.25, zero sink, zero flux at both ends, zero
//! gravity. Then Cap and Cond are constant in space and time and water velocity is zero.

use std::f64::consts::PI;

use hydrus_core::heat::capacity::TempMaterial;
use hydrus_core::heat::config::HeatConfigInput;
use hydrus_core::heat::driver::{CoupledHeatInput, CoupledStepReport, HeatWaterDriver};
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, Mesh1D,
};
use hydrus_core::{
    HydraulicError, LowerBoundaryMode, UpperBoundary, WaterState, WaterStepInput, WaterStepPolicy,
    WaterWorkspace,
};

/// Water timestep (test-only input, see the module header).
const DT: f64 = 1e-4;
/// Contract tolerance for TempN (absolute, degC).
const TEMP_TOL: f64 = 2e-4;

/// Static water: theta = 0.25 + 0.1 H, constant capacity 0.1, unit conductivity.
/// At H = 0 (uniform, zero flux, zero gravity) the water state is an exact fixed point.
struct ThetaModel;

impl HydraulicModel for ThetaModel {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: 0.25 + 0.1 * head,
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

fn policy() -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations: 30,
        head_tolerance: 1e-12,
        water_tolerance: 1e-12,
        minimum_timestep: 1e-5,
        retry_factor: 2.0,
        minimum_pivot: 1e-12,
    }
}

/// Heat material with solid fraction 0.5, quartz 0.5, solid capacity 2.0, water capacity 4.0.
/// At theta = 0.25: Cap = 0.5*2.0 + 4.0*0.25 = 2.0. Default Cond = quartz = 0.5 (v = 0).
fn material() -> TempMaterial {
    TempMaterial::from_tpar([0.5, 0.0, 0.0, 0.5, 0.0, 0.0, 2.0, 2.5, 4.0])
}

fn heat_config(campbell: bool, k_top_t: i32) -> HeatConfigInput {
    HeatConfigInput {
        l_vapor: false,
        l_en_bal: false,
        i_dual_por: 0,
        l_w_dep: false,
        i_model: 0,
        domain_count: 1,
        k_top_t,
        t_period: 0.0,
        k_bot_t: 1,
        campbell,
    }
}

/// Upper half-cell row for a zero-flux top at H = 0. The rhs is zero because the head is zero.
fn zero_flux_row(h: f64) -> BoundaryRow {
    let storage = (h / 2.0) * 0.1 / DT;
    BoundaryRow {
        lower: -1.0 / h,
        diagonal: storage + 1.0 / h,
        upper: 0.0,
        rhs: 0.0,
    }
}

/// One accepted coupled step result: time, temperatures, and reported heat flux.
struct Snapshot {
    time: f64,
    temps: Vec<f64>,
    heat_flux: f64,
}

/// Runs the coupled driver on a uniform column of `initial.len()` nodes over [0, 1] and records
/// the accepted state after each step number listed in `snapshots` (1-based).
fn run_column(
    campbell: bool,
    k_top_t: i32,
    top_temperature: f64,
    bottom_temperature: f64,
    initial: Vec<f64>,
    snapshots: &[usize],
) -> Vec<Snapshot> {
    let n = initial.len();
    let x: Vec<f64> = (0..n).map(|i| i as f64 / (n - 1) as f64).collect();
    let h = x[1] - x[0];
    let mesh = Mesh1D::new(x.clone(), vec![0; n], None, 1, 0, 0.0).unwrap();
    let model = ThetaModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = vec![0.0; n];
    let zeros = vec![0.0; n];
    let indices = vec![0usize; n];
    let mut water = WaterState::new(0.0, vec![0.0; n], vec![0.25; n]).unwrap();
    let mut workspace = WaterWorkspace::new(n).unwrap();
    let mut heat = HeatWaterDriver::new(
        heat_config(campbell, k_top_t),
        &x,
        &indices,
        &[material()],
        0.0,
        initial,
        &zeros,
    )
    .unwrap();

    let last = *snapshots.iter().max().expect("at least one snapshot");
    let mut out = Vec::new();
    for step in 1..=last {
        let target = water.time() + DT;
        let report: CoupledStepReport = heat
            .step(
                &mut water,
                &mut workspace,
                WaterStepInput {
                    mesh: &mesh,
                    materials: &materials,
                    sink: &sink,
                    root_uptake: None,
                    timestep: DT,
                    target_time: target,
                    face_averaging: FaceAveraging::Arithmetic,
                    lower: LowerBoundaryMode::PrescribedFlux { flux: 0.0 },
                    upper: UpperBoundary::Flux {
                        flux: 0.0,
                        row: zero_flux_row(h),
                    },
                },
                policy(),
                CoupledHeatInput {
                    top_temperature,
                    bottom_temperature,
                    x_conv: 1.0,
                    t_conv: 1.0,
                    minimum_pivot: 1e-12,
                },
            )
            .expect("coupled step");
        if snapshots.contains(&step) {
            out.push(Snapshot {
                time: heat.state().time(),
                temps: heat.state().temperatures().to_vec(),
                heat_flux: report.heat_flux,
            });
        }
    }
    out
}

fn uniform_x(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 / (n - 1) as f64).collect()
}

/// Case (a), default conduction. Closed form: steady conduction with Dirichlet ends (top 20,
/// bottom 0) and uniform K has T(x) = 20 x. The discrete residual for a linear T at uniform K
/// and v = 0 is (K/h)(2T_i - T_{i-1} - T_{i+1}) = 0, and the dt/Cap term cancels because T_old =
/// T_new. The IC is already linear, so this checks the discrete fixed point (roundoff only).
#[test]
fn analytical_only_steady_linear_profile_dirichlet() {
    let x = uniform_x(11);
    let initial: Vec<f64> = x.iter().map(|&xi| 20.0 * xi).collect();
    let snaps = run_column(false, 1, 20.0, 0.0, initial, &[3]);
    let snap = &snaps[0];
    for (i, (&t, &xi)) in snap.temps.iter().zip(&x).enumerate() {
        let exact = 20.0 * xi;
        assert!(
            (t - exact).abs() <= TEMP_TOL,
            "node {i}: T = {t}, exact = {exact}, tol = {TEMP_TOL}"
        );
    }
}

/// Case (a) with Campbell conductivity. Campbell K at theta = 0.25 is hand-derived as
/// K = 347/180 (see analytical_only_transient_fourier_campbell). The steady linear profile is
/// exact for any uniform K, so this checks the Campbell branch keeps the discrete fixed point.
#[test]
fn analytical_only_steady_linear_profile_campbell() {
    let x = uniform_x(11);
    let initial: Vec<f64> = x.iter().map(|&xi| 20.0 * xi).collect();
    let snaps = run_column(true, 1, 20.0, 0.0, initial, &[3]);
    let snap = &snaps[0];
    for (i, (&t, &xi)) in snap.temps.iter().zip(&x).enumerate() {
        let exact = 20.0 * xi;
        assert!(
            (t - exact).abs() <= TEMP_TOL,
            "node {i}: T = {t}, exact = {exact}, tol = {TEMP_TOL}"
        );
    }
}

/// Case (b), zero-flux top. The top flux is advective only, HeatFl = Cw * tTop * v. With v = 0
/// the top is zero-gradient, so the closed form is the uniform profile T = tBot = 10. The
/// reported heat flux must be 0 (relative 1e-5 with absolute floor 1e-12).
#[test]
fn analytical_only_steady_zero_flux_top_is_uniform() {
    let initial = vec![10.0; 11];
    let snaps = run_column(false, -1, 10.0, 10.0, initial, &[3]);
    let snap = &snaps[0];
    for (i, &t) in snap.temps.iter().enumerate() {
        assert!(
            (t - 10.0).abs() <= TEMP_TOL,
            "node {i}: T = {t}, exact = 10, tol = {TEMP_TOL}"
        );
    }
    assert!(
        snap.heat_flux.abs() <= 1e-12,
        "heat_flux = {}, exact = 0, floor = 1e-12",
        snap.heat_flux
    );
}

/// Exact transient two-mode solution, zero Dirichlet ends on [0, 1]:
/// T(x,t) = 10 sin(pi x) exp(-D pi^2 t) + 5 sin(3 pi x) exp(-9 D pi^2 t), with D = K/Cap.
/// The IC is a finite sine series with zero end values, so the exact series has no truncation
/// error. Discrete error: spatial relative eigenvalue error (n pi h)^2 / 12 per mode and CN
/// time error z^3 / 12 per step. At N = 801 and dt = 1e-4 both are below 3e-5 (see the todo).
fn fourier_exact(x: f64, t: f64, d: f64) -> f64 {
    10.0 * (PI * x).sin() * (-d * PI * PI * t).exp()
        + 5.0 * (3.0 * PI * x).sin() * (-9.0 * d * PI * PI * t).exp()
}

fn check_fourier(campbell: bool, d: f64) {
    let x = uniform_x(801);
    let initial: Vec<f64> = x
        .iter()
        .map(|&xi| 10.0 * (PI * xi).sin() + 5.0 * (3.0 * PI * xi).sin())
        .collect();
    let snaps = run_column(campbell, 1, 0.0, 0.0, initial, &[500, 2000, 10000]);
    for snap in &snaps {
        let mut worst = 0.0_f64;
        for (&t, &xi) in snap.temps.iter().zip(&x) {
            worst = worst.max((t - fourier_exact(xi, snap.time, d)).abs());
        }
        assert!(
            worst <= TEMP_TOL,
            "t = {}: max |T - exact| = {worst}, tol = {TEMP_TOL}",
            snap.time
        );
    }
}

/// Case (c), default conduction. D = K / Cap = 0.5 / 2.0 = 0.25 (K = quartz = 0.5 at theta =
/// 0.25, v = 0; Cap = 0.5*2.0 + 4.0*0.25 = 2.0). Times: 500, 2000, 10000 steps of 1e-4.
#[test]
fn analytical_only_transient_fourier_default() {
    check_fourier(false, 0.25);
}

/// Case (c), Campbell conductivity. Hand derivation, x_conv = t_conv = 1, v = 0, s = 0.5,
/// q = 0.5, o = 0, clay = 0 (floored to 0.005), theta = 0.25:
///   aa = (0.57 + 1.73*0.5 + 0) / (1 - 0.74*0.5) - 2.8*0.5*0.5 = 1.435/0.63 - 0.7 = 71/45
///   bb = 2.8*0.5 = 1.4, cc = 1 + 2.6/sqrt(0.005) = 37.77, dd = 0.03 + 0.7*0.25 = 0.205
///   (cc*theta)^4 = (9.44)^4 = 7955, so exp(-7955) underflows to 0 in f64
///   K = aa + bb*theta = 71/45 + 0.35 = 347/180, Cap = 2.0, D = 347/360.
#[test]
fn analytical_only_transient_fourier_campbell() {
    let k = 347.0 / 180.0;
    check_fourier(true, k / 2.0);
}

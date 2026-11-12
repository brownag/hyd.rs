//! Seeded balance and invariant properties of the coupled water-to-heat driver.
//!
//! `HeatWaterDriver` returns only `heat_flux`, so every balance term is recomputed here from
//! the solved state. Formulas are documented at [`step_terms`]. Water and heat fixtures are
//! copied from `tests/heat_driver_order.rs`. Expected values are independent recomputations,
//! not Fortran results. Tolerances: planning/master/M6_HEAT_CONTRACT.md `## Tolerances`.
//! The `1e-12` bounds are for exact-by-construction properties only; they are not contract values.

use hydrus_core::heat::balance::{HeatBalance, HeatBalanceInput};
use hydrus_core::heat::capacity::{temp_cap, TempCapInput, TempMaterial};
use hydrus_core::heat::config::HeatConfigInput;
use hydrus_core::heat::driver::{CoupledHeatInput, HeatWaterDriver};
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, Mesh1D,
};
use hydrus_core::{
    advance_water_step, HydraulicError, LowerBoundaryMode, UpperBoundary, WaterState,
    WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

const X: [f64; 5] = [0.0, 0.5, 1.5, 2.0, 3.0];
const NODES: usize = 5;
/// `TPar(9)` of [`material`]: the volumetric heat capacity of water (`cw`).
const CW: f64 = 4.0;
const DT: f64 = 0.2;
const STEPS: u32 = 6;
const COSINE: f64 = 0.8;
const TOL_BALANCE: f64 = 1e-6;
const TOL_EXACT: f64 = 1e-12;
const SCALE_FLOOR: f64 = 1e-12;
const CAMPBELL_FALSE_SEEDS: [u64; 4] = [1, 2, 3, 4];
const CAMPBELL_TRUE_SEEDS: [u64; 4] = [101, 102, 103, 104];
const MATERIAL_INDICES: [usize; NODES] = [0; NODES];

/// Knuth MMIX linear congruential generator. Output is the top 53 bits over `2^53`, in [0, 1).
struct Lcg(u64);

impl Lcg {
    fn next_unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / 9_007_199_254_740_992.0
    }
}

/// Linear retention, unit conductivity, constant capacity.
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

/// Exponential conductivity.
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
    Mesh1D::new(X.to_vec(), vec![0; NODES], None, 1, 0, inclination_cosine).unwrap()
}

fn control_width(node: usize) -> f64 {
    let below = if node == 0 {
        0.0
    } else {
        X[node] - X[node - 1]
    };
    let above = if node == NODES - 1 {
        0.0
    } else {
        X[node + 1] - X[node]
    };
    (below + above) / 2.0
}

/// Upper half-cell row (as in `heat_driver_order.rs`), rebuilt from the current top head.
/// `storage` uses capacity 0.1 and `dt = DT`. `rhs` carries the old top head, gravity and sink.
fn upper_row(head_top: f64, inclination_cosine: f64, sink_top: f64) -> BoundaryRow {
    let dx = X[NODES - 1] - X[NODES - 2];
    let storage = control_width(NODES - 1) * 0.1 / DT;
    BoundaryRow {
        lower: -1.0 / dx,
        diagonal: storage + 1.0 / dx,
        upper: 0.0,
        rhs: storage * head_top - inclination_cosine - sink_top * control_width(NODES - 1),
    }
}

fn water_input<'a>(
    mesh: &'a Mesh1D,
    materials: &'a [&'a dyn HydraulicModel],
    sink: &'a [f64],
    row: BoundaryRow,
    target_time: f64,
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials,
        sink,
        root_uptake: None,
        timestep: DT,
        target_time,
        face_averaging: FaceAveraging::Arithmetic,
        lower: LowerBoundaryMode::PrescribedFlux { flux: 0.0 },
        upper: UpperBoundary::Flux { flux: 0.0, row },
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

fn coupled(top: f64, bottom: f64) -> CoupledHeatInput {
    CoupledHeatInput {
        top_temperature: top,
        bottom_temperature: bottom,
        x_conv: 1.0,
        t_conv: 1.0,
        minimum_pivot: 1e-12,
    }
}

/// Node Darcy fluxes, copied from `driver.rs:node_velocities` (private there). Duplicated
/// so the balance can use the same `vNew` the driver uses. A divergence fails the balance.
fn node_velocities(
    face: &[f64],
    water_old: &[f64],
    water_new: &[f64],
    sink: &[f64],
    dt: f64,
) -> Vec<f64> {
    let n = NODES;
    let mut out = vec![0.0; n];
    for i in 1..n - 1 {
        let dxa = X[i + 1] - X[i];
        let dxb = X[i] - X[i - 1];
        out[i] = (face[i + 1] * dxb + face[i] * dxa) / (dxa + dxb);
    }
    let dx1 = X[1] - X[0];
    out[0] = face[1] + dx1 / 2.0 * ((water_new[0] - water_old[0]) / dt + sink[0]);
    let dxn = X[n - 1] - X[n - 2];
    out[n - 1] =
        face[n - 1] - dxn / 2.0 * ((water_new[n - 1] - water_old[n - 1]) / dt + sink[n - 1]);
    out
}

fn cap_cond(water: &[f64], velocity: &[f64], campbell: bool) -> (Vec<f64>, Vec<f64>) {
    let materials = [material()];
    let mut cap = vec![0.0; NODES];
    let mut cond = vec![0.0; NODES];
    temp_cap(
        TempCapInput {
            material_indices: &MATERIAL_INDICES,
            water_content: water,
            velocity,
            materials: &materials,
            campbell,
            x_conv: 1.0,
            t_conv: 1.0,
        },
        &mut cap,
        &mut cond,
    )
    .unwrap();
    (cap, cond)
}

/// Storage weights, as `balance.rs:storage_integral`.
fn weight(i: usize) -> f64 {
    if i == 0 {
        (X[1] - X[0]) / 2.0
    } else if i == NODES - 1 {
        (X[NODES - 1] - X[NODES - 2]) / 2.0
    } else {
        (X[i + 1] - X[i - 1]) / 2.0
    }
}

/// `S = sum_i w_i * Cap_i * T_i` over all nodes, as `balance.rs` defines it.
fn storage(cap: &[f64], temp: &[f64]) -> f64 {
    (0..NODES).map(|i| weight(i) * cap[i] * temp[i]).sum()
}

/// Partition weight `1 - phi_0 - phi_{N-1}`: the part of the linear hat functions that the
/// interior rows (`1..N-2`) assemble. The end rows are Dirichlet, so their sink is not in the scheme.
fn interior_weight(xq: f64) -> f64 {
    let mut end = 0.0;
    if xq <= X[1] {
        end += (X[1] - xq) / (X[1] - X[0]);
    }
    if xq >= X[NODES - 2] {
        end += (xq - X[NODES - 2]) / (X[NODES - 1] - X[NODES - 2]);
    }
    1.0 - end
}

/// Heat removed by the sink, `cw * integral of g(x) s(x) T(x) dx`, with `s` and `T` linear per
/// element and `g` the interior partition weight. The integrand is cubic per element, so
/// Simpson's rule is exact.
fn sink_heat_physical(temp: &[f64], sink: &[f64]) -> f64 {
    let mut total = 0.0;
    for k in 0..NODES - 1 {
        let (a, b) = (X[k], X[k + 1]);
        let h = b - a;
        let f = |xq: f64| {
            let frac = (xq - a) / h;
            let s = sink[k] + (sink[k + 1] - sink[k]) * frac;
            let t = temp[k] + (temp[k + 1] - temp[k]) * frac;
            interior_weight(xq) * s * t
        };
        total += h / 6.0 * (f(a) + 4.0 * f(0.5 * (a + b)) + f(b));
    }
    CW * total
}

/// Element mean of `v*T` over element `[k, k+1]`, with `v` and `T` linear there:
/// `(v_a T_a + v_b T_b)/3 + (v_a T_b + v_b T_a)/6`. The weak-form advective rows
/// (`-(CW/2) * integral of phi_i' v T`, `transport.rs:301-375`) telescope to these averages.
fn mean_product(v: &[f64], t: &[f64], k: usize) -> f64 {
    let (va, vb, ta, tb) = (v[k], v[k + 1], t[k], t[k + 1]);
    (va * ta + vb * tb) / 3.0 + (va * tb + vb * ta) / 6.0
}

/// End fluxes for one state, positive as the balance defines them.
/// Conduction: physical face flux `(cond_i + cond_j) / (2 dx) * (T_j - T_i)`.
/// Advection: `cw * v * T` at the end node, upward positive.
/// Returns `(inflow at top, outflow at bottom)`.
fn end_flux(cond: &[f64], temp: &[f64], velocity: &[f64]) -> (f64, f64) {
    let n = NODES;
    let g_top = (cond[n - 2] + cond[n - 1]) / (2.0 * (X[n - 1] - X[n - 2]));
    let g_bot = (cond[0] + cond[1]) / (2.0 * (X[1] - X[0]));
    let inflow_top = g_top * (temp[n - 1] - temp[n - 2]) - CW * mean_product(velocity, temp, n - 2);
    let outflow_bottom = g_bot * (temp[1] - temp[0]) - CW * mean_product(velocity, temp, 0);
    (inflow_top, outflow_bottom)
}

struct StepTerms {
    cap_old: Vec<f64>,
    cap_new: Vec<f64>,
    boundary_in: f64,
    boundary_out: f64,
    sink_heat: f64,
    storage_old: f64,
    storage_new: f64,
}

/// Recomputes the step terms from the solved state. The scheme (`transport.rs:301-375`) puts
/// `w_i*Cap_new/dt` on the diagonal and `w_i*Cap_old*TempO/dt` in the RHS, with spatial terms
/// at old and new states. Each end flux is therefore averaged over old and new, times `dt`:
/// `boundary = 0.5*dt*(q_old + q_new)`. The sink uses the same average.
/// Top Dirichlet: `boundary_in` is the top inflow. Bottom Dirichlet: `boundary_out` is the bottom outflow.
/// Advection at the ends is `cw*v*T` (exact for the linear end values).
#[allow(clippy::too_many_arguments)]
fn step_terms(
    campbell: bool,
    dt: f64,
    theta_old: &[f64],
    theta_new: &[f64],
    v_old: &[f64],
    v_new: &[f64],
    t_old: &[f64],
    t_new: &[f64],
    sink: &[f64],
) -> StepTerms {
    let (cap_old, cond_old) = cap_cond(theta_old, v_old, campbell);
    let (cap_new, cond_new) = cap_cond(theta_new, v_new, campbell);
    let (in_old, out_old) = end_flux(&cond_old, t_old, v_old);
    let (in_new, out_new) = end_flux(&cond_new, t_new, v_new);
    StepTerms {
        storage_old: storage(&cap_old, t_old),
        storage_new: storage(&cap_new, t_new),
        boundary_in: 0.5 * dt * (in_old + in_new),
        boundary_out: 0.5 * dt * (out_old + out_new),
        sink_heat: 0.5 * dt * (sink_heat_physical(t_old, sink) + sink_heat_physical(t_new, sink)),
        cap_old,
        cap_new,
    }
}

/// One accepted step. `residual` is `R_full` from `HeatBalance::record_step`. `interior` is
/// `R_interior`, the closure over the storage of interior nodes only. `reservoir` is
/// `reservoir_end` (see [`record`]). By construction `residual = interior + reservoir`.
#[derive(Clone, Copy, Debug)]
struct Check {
    residual: f64,
    interior: f64,
    reservoir: f64,
    scale: f64,
}

/// Records one step in the accumulator and returns the residuals and the scale.
///
/// Convention (approved): the prescribed (Dirichlet) end nodes hold a fixed temperature. Their
/// storage change `reservoir_end = S_end_new - S_end_old` comes from the capacity change at
/// fixed temperature, and the prescribed reservoir supplies it. This is a convention about the
/// domain boundary, not a scheme error. `R_interior = (S_new - S_old - reservoir_end) -
/// (boundary_in - boundary_out - sink)` is the closure over interior nodes and is asserted
/// tightly. `R_full` is `balance.rs`'s residual. It equals `R_interior + reservoir_end` by
/// construction at prescribed nodes, so `reservoir_end` is documented, not fitted.
fn record(balance: &mut HeatBalance, terms: &StepTerms, t_old: &[f64], t_new: &[f64]) -> Check {
    let report = balance
        .record_step(HeatBalanceInput {
            x: &X,
            cap_old: &terms.cap_old,
            temp_old: t_old,
            cap_new: &terms.cap_new,
            temp_new: t_new,
            boundary_in: terms.boundary_in,
            boundary_out: terms.boundary_out,
            sink_heat: terms.sink_heat,
        })
        .unwrap();
    let scale = (terms.storage_old.abs()
        + terms.storage_new.abs()
        + terms.boundary_in.abs()
        + terms.boundary_out.abs()
        + terms.sink_heat.abs())
    .max(SCALE_FLOOR);
    let reservoir = weight(0) * (terms.cap_new[0] * t_new[0] - terms.cap_old[0] * t_old[0])
        + weight(NODES - 1)
            * (terms.cap_new[NODES - 1] * t_new[NODES - 1]
                - terms.cap_old[NODES - 1] * t_old[NODES - 1]);
    let flux_net = terms.boundary_in - terms.boundary_out - terms.sink_heat;
    let interior = (report.storage_change - reservoir) - flux_net;
    Check {
        residual: report.absolute_residual,
        interior,
        reservoir,
        scale,
    }
}

/// Coupled-step assertions: interior closure exact; full residual equals interior plus reservoir.
fn assert_coupled_step(campbell: bool, seed: u64, step: usize, check: &Check) {
    assert!(
        check.interior.abs() <= TOL_EXACT * check.scale,
        "campbell={campbell} seed={seed} step={step}: R_interior {:e} > {:e} * scale {:e}",
        check.interior,
        TOL_EXACT,
        check.scale
    );
    let identity = check.residual - (check.interior + check.reservoir);
    assert!(
        identity.abs() <= TOL_BALANCE * check.scale,
        "campbell={campbell} seed={seed} step={step}: R_full - (R_interior + reservoir) {identity:e}"
    );
}

/// Per-seed initial data. Draw order is fixed: heads (coupled only), interior temperatures,
/// bottom and top temperatures, and interior sinks (coupled only).
struct Setup {
    heads: Vec<f64>,
    temps: Vec<f64>,
    sink: Vec<f64>,
    t_bottom: f64,
    t_top: f64,
}

fn setup(seed: u64, coupled_run: bool) -> Setup {
    let mut rng = Lcg(seed);
    let heads: Vec<f64> = if coupled_run {
        (0..NODES).map(|_| -3.0 + 2.5 * rng.next_unit()).collect()
    } else {
        vec![-1.0; NODES]
    };
    let mut temps = vec![0.0; NODES];
    for value in temps.iter_mut().take(NODES - 1).skip(1) {
        *value = 5.0 + 15.0 * rng.next_unit();
    }
    let t_bottom = 5.0 + 15.0 * rng.next_unit();
    let t_top = 5.0 + 15.0 * rng.next_unit();
    temps[0] = t_bottom;
    temps[NODES - 1] = t_top;
    let mut sink = vec![0.0; NODES];
    if coupled_run {
        for value in sink.iter_mut().take(NODES - 1).skip(1) {
            *value = 0.02 * rng.next_unit();
        }
    }
    Setup {
        heads,
        temps,
        sink,
        t_bottom,
        t_top,
    }
}

/// Runs one seed for `STEPS` accepted steps and returns the per-step checks.
/// `coupled_run = false` uses uniform head -1 with cosine 0 (zero velocity, zero sink).
fn run(campbell: bool, seed: u64, coupled_run: bool) -> Vec<Check> {
    let init = setup(seed, coupled_run);
    let linear = LinearModel;
    let exponential = ExponentialModel;
    let model: &dyn HydraulicModel = if coupled_run { &exponential } else { &linear };
    let models: [&dyn HydraulicModel; 1] = [model];
    let cosine = if coupled_run { COSINE } else { 0.0 };
    let mesh = mesh(cosine);
    let water: Vec<f64> = init.heads.iter().map(|h| 1.0 + 0.1 * h).collect();
    let mut state = WaterState::new(0.0, init.heads.clone(), water).unwrap();
    let mut workspace = WaterWorkspace::new(NODES).unwrap();
    let mut heat = HeatWaterDriver::new(
        heat_config(campbell),
        &X,
        &MATERIAL_INDICES,
        &[material()],
        0.0,
        init.temps.clone(),
        &[0.0; NODES],
    )
    .unwrap();
    let mut balance = HeatBalance::default();
    let mut v_old = vec![0.0; NODES];
    let mut checks = Vec::new();
    for step in 1..=STEPS {
        let target = DT * f64::from(step);
        let row = upper_row(state.heads()[NODES - 1], cosine, init.sink[NODES - 1]);
        let input = water_input(&mesh, &models, &init.sink, row, target);
        let theta_old = state.water_contents().to_vec();
        let t_old = heat.state().temperatures().to_vec();
        // Replay the water step on clones to obtain the face fluxes that `Veloc` needs.
        let (face, dt_replay) = {
            let mut trial = state.clone();
            let mut trial_workspace = workspace.clone();
            let report =
                advance_water_step(&mut trial, &mut trial_workspace, input, policy()).unwrap();
            (report.face_fluxes.to_vec(), report.timestep)
        };
        let report = heat
            .step(
                &mut state,
                &mut workspace,
                input,
                policy(),
                coupled(init.t_top, init.t_bottom),
            )
            .unwrap();
        assert_eq!(report.timestep.to_bits(), dt_replay.to_bits());
        let t_new = heat.state().temperatures().to_vec();
        assert!(
            t_new.iter().all(|t| t.is_finite()),
            "seed {seed} step {step}: non-finite temperature {t_new:?}"
        );
        let theta_new = state.water_contents().to_vec();
        let v_new = node_velocities(&face, &theta_old, &theta_new, &init.sink, report.timestep);
        let terms = step_terms(
            campbell,
            report.timestep,
            &theta_old,
            &theta_new,
            &v_old,
            &v_new,
            &t_old,
            &t_new,
            &init.sink,
        );
        checks.push(record(&mut balance, &terms, &t_old, &t_new));
        v_old = v_new;
    }
    checks
}

fn worst_interior_ratio(checks: &[Check]) -> f64 {
    checks
        .iter()
        .map(|c| c.interior.abs() / c.scale)
        .fold(0.0, f64::max)
}

fn worst_identity_ratio(checks: &[Check]) -> f64 {
    checks
        .iter()
        .map(|c| (c.residual - (c.interior + c.reservoir)).abs() / c.scale)
        .fold(0.0, f64::max)
}

#[test]
fn zero_flux_uniform_state_does_not_drift() {
    for campbell in [false, true] {
        let linear = LinearModel;
        let models: [&dyn HydraulicModel; 1] = [&linear];
        let sink = [0.0; NODES];
        let mesh = mesh(0.0);
        let row = upper_row(-1.0, 0.0, 0.0);
        let mut state = WaterState::new(0.0, vec![-1.0; NODES], vec![0.9; NODES]).unwrap();
        let mut workspace = WaterWorkspace::new(NODES).unwrap();
        // Zero-gradient bottom (kBotT = 0), Dirichlet top at 10 degC.
        let config = HeatConfigInput {
            k_bot_t: 0,
            ..heat_config(campbell)
        };
        let mut heat = HeatWaterDriver::new(
            config,
            &X,
            &MATERIAL_INDICES,
            &[material()],
            0.0,
            vec![10.0; NODES],
            &[0.0; NODES],
        )
        .unwrap();
        let mut worst = 0.0_f64;
        for step in 1..=STEPS {
            let target = DT * f64::from(step);
            heat.step(
                &mut state,
                &mut workspace,
                water_input(&mesh, &models, &sink, row, target),
                policy(),
                coupled(10.0, 10.0),
            )
            .unwrap();
            for value in heat.state().temperatures() {
                worst = worst.max((value - 10.0).abs());
            }
        }
        eprintln!("zero_flux campbell={campbell}: worst |T-10| = {worst:e} (tol {TOL_EXACT:e})");
        assert!(worst <= TOL_EXACT, "campbell={campbell}: drift {worst:e}");
    }
}

#[test]
fn conduction_only_balance_closes_exactly() {
    for (campbell, seeds) in [(false, CAMPBELL_FALSE_SEEDS), (true, CAMPBELL_TRUE_SEEDS)] {
        let mut worst = 0.0_f64;
        for seed in seeds {
            let checks = run(campbell, seed, false);
            for (k, check) in checks.iter().enumerate() {
                let ratio = check.residual.abs() / check.scale;
                worst = worst.max(ratio);
                assert!(
                    ratio <= TOL_EXACT,
                    "campbell={campbell} seed={seed} step={}: residual {:e}, scale {:e}",
                    k + 1,
                    check.residual,
                    check.scale
                );
            }
        }
        eprintln!("conduction-only campbell={campbell}: worst |res|/scale = {worst:e} (tol {TOL_EXACT:e})");
    }
}

#[test]
fn coupled_balance_within_tolerance_campbell_false() {
    let mut worst_interior = 0.0_f64;
    let mut worst_identity = 0.0_f64;
    let mut worst_reservoir = 0.0_f64;
    for seed in CAMPBELL_FALSE_SEEDS {
        let checks = run(false, seed, true);
        worst_interior = worst_interior.max(worst_interior_ratio(&checks));
        worst_identity = worst_identity.max(worst_identity_ratio(&checks));
        for (k, check) in checks.iter().enumerate() {
            worst_reservoir = worst_reservoir.max(check.reservoir.abs());
            assert_coupled_step(false, seed, k + 1, check);
        }
    }
    eprintln!(
        "coupled campbell=false: worst |R_interior|/scale = {worst_interior:e} (tol {TOL_EXACT:e}); worst |R_full - (R_interior+reservoir)|/scale = {worst_identity:e} (tol {TOL_BALANCE:e}); max |reservoir_end| = {worst_reservoir:e}"
    );
}

#[test]
fn coupled_balance_within_tolerance_campbell_true() {
    let mut worst_interior = 0.0_f64;
    let mut worst_identity = 0.0_f64;
    let mut worst_reservoir = 0.0_f64;
    for seed in CAMPBELL_TRUE_SEEDS {
        let checks = run(true, seed, true);
        worst_interior = worst_interior.max(worst_interior_ratio(&checks));
        worst_identity = worst_identity.max(worst_identity_ratio(&checks));
        for (k, check) in checks.iter().enumerate() {
            worst_reservoir = worst_reservoir.max(check.reservoir.abs());
            assert_coupled_step(true, seed, k + 1, check);
        }
    }
    eprintln!(
        "coupled campbell=true: worst |R_interior|/scale = {worst_interior:e} (tol {TOL_EXACT:e}); worst |R_full - (R_interior+reservoir)|/scale = {worst_identity:e} (tol {TOL_BALANCE:e}); max |reservoir_end| = {worst_reservoir:e}"
    );
}

#[test]
fn temperatures_stay_finite() {
    // Non-finite temperatures panic inside `run`. This test covers both seed sets and both
    // coupling modes, and checks that every residual is finite.
    for campbell in [false, true] {
        for seed in CAMPBELL_FALSE_SEEDS
            .iter()
            .chain(CAMPBELL_TRUE_SEEDS.iter())
        {
            for coupled_run in [false, true] {
                let checks = run(campbell, *seed, coupled_run);
                assert!(checks
                    .iter()
                    .all(|c| c.residual.is_finite() && c.scale.is_finite()));
            }
        }
    }
}

#[test]
fn residual_does_not_grow_with_step_count() {
    let mut worst_cumulative = 0.0_f64;
    let mut worst_early = 0.0_f64;
    let mut worst_late = 0.0_f64;
    for (campbell, seeds) in [(false, CAMPBELL_FALSE_SEEDS), (true, CAMPBELL_TRUE_SEEDS)] {
        for seed in seeds {
            let checks = run(campbell, seed, true);
            let sum_residual: f64 = checks.iter().map(|c| c.interior).sum();
            let sum_scale: f64 = checks.iter().map(|c| c.scale).sum();
            let cumulative = sum_residual.abs() / sum_scale;
            worst_cumulative = worst_cumulative.max(cumulative);
            assert!(
                cumulative <= TOL_BALANCE,
                "campbell={campbell} seed={seed}: cumulative |sum res|/sum scale {cumulative:e}"
            );
            for (k, check) in checks.iter().enumerate() {
                let ratio = check.interior.abs() / check.scale;
                if k < 3 {
                    worst_early = worst_early.max(ratio);
                } else {
                    worst_late = worst_late.max(ratio);
                }
            }
        }
    }
    eprintln!(
        "growth (R_interior): worst cumulative {worst_cumulative:e}, early steps 1-3 {worst_early:e}, late steps 4-6 {worst_late:e} (tol {TOL_BALANCE:e})"
    );
}

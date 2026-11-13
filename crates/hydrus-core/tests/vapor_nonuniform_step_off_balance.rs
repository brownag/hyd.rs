//! Heat-only per-step interior balance for `vapor_nonuniform_step_off` (task 28.2).
//!
//! `vapor_nonuniform_step_off` has `lVapor = f`, so `vapor_balance` does not apply
//! (`UnsupportedBoolean`). This test rebuilds the `TEMPER.FOR:112-143` interior rows
//! (`do 12 i=2,N-1`; level-2 `B`, `D`, `E`, level-1 `F`) with no vapor terms, from the accepted
//! water state, and asserts `R_i = B*T[i-1] + D*T[i] + E*T[i+1] - F` closes on every accepted step.
//!
//! Scope and limits:
//! - Interior rows only. The fixed-head Dirichlet boundary flux is outside the interior balance and
//!   is not asserted.
//! - Schedule: the constant declared dt (0.1, 2400 steps), as in `vapor_nonuniform_parity.rs` for
//!   step_off. Rust's own time-step controller is not tested.
//! - Bounds come from `reference/vapor_fortran/criteria.json` `checks.heat_balance_interior`;
//!   no numbers are copied here. Ratio is `|R| / max(rel * scale, abs)`, scale `max_abs_row_term`.
//! - Closure proves the accepted `T` satisfies the rows built from the accepted water, not that the
//!   rows are right (the parity test does that). `v_new` is rebuilt here (`Veloc`,
//!   `WATFLOW.FOR:717-775`, copied from the private `heat/driver.rs:node_velocities`).
//! - The water side is NOT asserted. The `water_balance_interior` form needs `h_it`, `thL_it`,
//!   `C_eff` and iterate conductivities, none exposed by the legacy path, so it is not
//!   reproducible. The available continuum residual closes only to TolTh/TolH, so a strict bound is
//!   invalid; any asserted bound needs user-approved criteria. It is printed as a diagnostic only
//!   (finiteness is the single assertion).

use std::fs;
use std::path::PathBuf;

use hydrus_core::heat::capacity::{temp_cap, TempCapInput, TempMaterial};
use hydrus_core::heat::config::HeatConfigInput;
use hydrus_core::heat::driver::{CoupledHeatInput, HeatWaterDriver};
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, HydraulicTables,
    Mesh1D, UnitSymbol, VanGenuchtenModel, VanGenuchtenParameters,
};
use hydrus_core::{
    advance_water_step, HydraulicError, LowerBoundaryMode, UpperBoundary, WaterState,
    WaterStepInput, WaterStepPolicy, WaterWorkspace,
};
use serde_json::Value;

const CASE: &str = "vapor_nonuniform_step_off";
const X_CONV: f64 = 100.0; // INPUT.FOR:190 (cm)
const T_CONV: f64 = 1.0 / 3600.0; // INPUT.FOR:197 (hours)
const MINIMUM_PIVOT: f64 = 1e-12;
const RETRY_FACTOR: f64 = 3.0;
/// 240 h at the constant declared dt 0.1 (`TPrint` 6, 24, 72, 240).
const EXPECTED_STEPS: usize = 2400;

/// Acceptance bound pair for one balance check.
struct Bound {
    relative: f64,
    absolute: f64,
}

/// Reads `checks.<name>` from `reference/vapor_fortran/criteria.json`.
fn load_bound(name: &str) -> Bound {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../reference/vapor_fortran/criteria.json"
    );
    let bytes = fs::read(path).expect("criteria.json readable");
    let doc: Value = serde_json::from_slice(&bytes).expect("criteria.json parses");
    assert_eq!(doc["schema_version"], "vapor-criteria-v2", "schema_version");
    let c = &doc["checks"][name];
    assert_eq!(c["scale"], "max_abs_row_term", "{name} scale");
    let num = |key: &str| {
        let v = c[key]
            .as_f64()
            .unwrap_or_else(|| panic!("{name}.{key} not a number"));
        assert!(v.is_finite() && v >= 0.0, "{name}.{key} invalid: {v}");
        v
    };
    Bound {
        relative: num("relative"),
        absolute: num("absolute"),
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("hydrus-core must remain inside the workspace")
        .to_path_buf()
}

// ----------------------------------------------------------------------------------------------
// Deck and profile (copied from vapor_nonuniform_balance.rs / parity; lVapor expected off)
// ----------------------------------------------------------------------------------------------

fn after(lines: &[String], key: &str) -> Vec<String> {
    let index = lines
        .iter()
        .position(|line| line.contains(key))
        .unwrap_or_else(|| panic!("SELECTOR.IN has no line with `{key}`"));
    lines[index + 1]
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

fn numbers(words: &[String]) -> Vec<f64> {
    words
        .iter()
        .map(|word| {
            word.parse::<f64>()
                .unwrap_or_else(|_| panic!("not a number: {word}"))
        })
        .collect()
}

fn flag(word: &str) -> bool {
    match word {
        "t" => true,
        "f" => false,
        other => panic!("not a logical flag: {other}"),
    }
}

struct Deck {
    cos_alpha: f64,
    max_iterations: usize,
    tol_th: f64,
    tol_h: f64,
    h_tab_1: f64,
    h_tab_n: f64,
    vg: [f64; 6],
    dt: f64,
    dt_min: f64,
    print_times: Vec<f64>,
    t_top: f64,
    t_bot: f64,
    k_top_t: i32,
    k_bot_t: i32,
    tpar: [f64; 9],
}

fn parse_deck(text: &str) -> Deck {
    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let block1 = after(&lines, "lWat lChem lTemp");
    let block2 = after(&lines, "lSnow lHP1");
    let nmat = after(&lines, "NMat NLay CosAlpha");
    let tols = after(&lines, "MaxIt TolTh TolH");
    let top = after(&lines, "TopInf WLayer KodTop");
    let bot = after(&lines, "BotInf qGWLF");
    let htab = numbers(&after(&lines, "hTab1 hTabN"));
    let model = numbers(&after(&lines, "Model Hysteresis"));
    let vg = numbers(&after(&lines, "thr ths Alfa n Ks l"));
    let time = numbers(&after(&lines, "dt dtMin dtMax"));
    let tmax = numbers(&after(&lines, "tInit tMax"));
    let tprint = numbers(&after(&lines, "TPrint(1...MPL)"));
    let tpar = numbers(&after(&lines, "record 2 (TempIn skip)"));
    let campbell = after(&lines, "amplitude/period/Campbell");
    let bc = numbers(&after(&lines, "boundary code record"));

    assert_eq!(block1.len(), 10, "BasInf line 10 must have ten flags");
    assert_eq!(block2.len(), 6, "BasInf iVer4 line must have six flags");
    assert_eq!(model.len(), 2, "Model line");
    assert_eq!(vg.len(), 6);
    assert_eq!(htab.len(), 2, "hTab line");
    assert_eq!(tpar.len(), 9, "one TPar row of nine values");
    assert_eq!(bc.len(), 4, "boundary code record");

    // Supported selection: lVapor off, lChem/lWDep/lMeteo off, fixed heads both ends, Campbell 0.
    assert!(flag(&block1[0]) && !flag(&block1[1]), "lWat on, lChem off");
    assert!(!flag(&block1[6]) && !flag(&block2[2]), "lWDep/lMeteo off");
    assert!(!flag(&block2[3]), "lVapor off");
    assert!(!flag(&top[0]) && !flag(&bot[0]), "fixed heads at both ends");
    assert!(
        numbers(&top[2..3])[0] == 1.0 && numbers(&bot[4..5])[0] == 1.0,
        "KodTop/KodBot = 1"
    );
    assert_eq!(campbell[2].parse::<f64>().expect("iCampbell"), 0.0);
    assert_eq!(bc[0] as i32, 1, "kTopT");
    assert_eq!(bc[2] as i32, 1, "kBotT");
    assert_eq!(model[0] as i32, 0, "van Genuchten only");
    assert_eq!(model[1] as i32, 0, "no hysteresis");
    assert_eq!(vg[5], 0.5, "l equals the Mualem BPar");

    let print_times = {
        let mut times = vec![tmax[0]];
        times.extend(tprint);
        times
    };
    assert_eq!(print_times[0], 0.0, "tInit must be zero");

    Deck {
        cos_alpha: numbers(&nmat)[2],
        max_iterations: numbers(&tols)[0] as usize,
        tol_th: numbers(&tols)[1],
        tol_h: numbers(&tols)[2],
        h_tab_1: htab[0],
        h_tab_n: htab[1],
        vg: vg.try_into().expect("six vG values"),
        dt: time[0],
        dt_min: time[1],
        print_times,
        t_top: bc[1],
        t_bot: bc[3],
        k_top_t: bc[0] as i32,
        k_bot_t: bc[2] as i32,
        tpar: tpar.try_into().expect("nine TPar values"),
    }
}

struct Profile {
    depths: Vec<f64>,
    heads: Vec<f64>,
    temperatures: Vec<f64>,
    material_indices: Vec<usize>,
}

fn parse_profile(text: &str) -> Profile {
    let mut depths = Vec::new();
    let mut heads = Vec::new();
    let mut temperatures = Vec::new();
    let mut material_indices = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 10 {
            continue;
        }
        let Ok(node) = fields[0].parse::<usize>() else {
            continue;
        };
        assert_eq!(
            node,
            depths.len() + 1,
            "PROFILE nodes must be numbered 1..N"
        );
        depths.push(fields[1].parse().expect("depth"));
        heads.push(fields[2].parse().expect("head"));
        material_indices.push(fields[3].parse::<usize>().expect("material") - 1);
        temperatures.push(fields[9].parse().expect("temperature"));
    }
    assert_eq!(depths.len(), 41, "PROFILE must have 41 nodes");
    Profile {
        depths,
        heads,
        temperatures,
        material_indices,
    }
}

fn rev(values: &[f64]) -> Vec<f64> {
    values.iter().rev().copied().collect()
}

fn placeholder_row(rhs: f64) -> BoundaryRow {
    BoundaryRow {
        lower: 0.0,
        diagonal: 1.0,
        upper: 0.0,
        rhs,
    }
}

/// Table inside the `hTab` span, analytic van Genuchten outside it (`SetMat` fallback).
struct SpanModel<'a> {
    table: &'a HydraulicTables,
    analytic: &'a VanGenuchtenModel,
}

impl SpanModel<'_> {
    fn inside(&self, head: f64) -> bool {
        head >= self.table.heads()[0] && head <= *self.table.heads().last().unwrap()
    }
}

impl HydraulicModel for SpanModel<'_> {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        if self.inside(head) {
            self.table.evaluate(head)
        } else {
            self.analytic.evaluate(head)
        }
    }
    fn derivatives(&self, head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        if self.inside(head) {
            self.table.derivatives(head)
        } else {
            self.analytic.derivatives(head)
        }
    }
}

/// Initial node velocity (`vapor_nonuniform_parity.rs:initial_velocity`, copied).
fn initial_velocity(x: &[f64], head: &[f64], k: &[f64], g: f64) -> Vec<f64> {
    let n = x.len();
    let mut v = vec![0.0; n];
    for i in 1..n - 1 {
        let dxa = x[i + 1] - x[i];
        let dxb = x[i] - x[i - 1];
        let va = -(k[i] + k[i + 1]) / 2.0 * ((head[i + 1] - head[i]) / dxa + g);
        let vb = -(k[i] + k[i - 1]) / 2.0 * ((head[i] - head[i - 1]) / dxb + g);
        v[i] = (va * dxb + vb * dxa) / (dxa + dxb);
    }
    let dx1 = x[1] - x[0];
    v[0] = -(k[0] + k[1]) / 2.0 * ((head[1] - head[0]) / dx1 + g);
    let dxn = x[n - 1] - x[n - 2];
    v[n - 1] = -(k[n - 1] + k[n - 2]) / 2.0 * ((head[n - 1] - head[n - 2]) / dxn + g);
    v
}

/// `Veloc` node fluxes, copied from `heat/driver.rs:node_velocities` (private there).
fn node_velocities(
    x: &[f64],
    face: &[f64],
    water_old: &[f64],
    water_new: &[f64],
    sink: &[f64],
    dt: f64,
) -> Vec<f64> {
    let n = x.len();
    let mut out = vec![0.0; n];
    for i in 1..n - 1 {
        let dxa = x[i + 1] - x[i];
        let dxb = x[i] - x[i - 1];
        out[i] = (face[i + 1] * dxb + face[i] * dxa) / (dxa + dxb);
    }
    let dx1 = x[1] - x[0];
    out[0] = face[1] + dx1 / 2.0 * ((water_new[0] - water_old[0]) / dt + sink[0]);
    let dxn = x[n - 1] - x[n - 2];
    out[n - 1] =
        face[n - 1] - dxn / 2.0 * ((water_new[n - 1] - water_old[n - 1]) / dt + sink[n - 1]);
    out
}

fn cap_cond(
    materials: &[TempMaterial],
    indices: &[usize],
    water: &[f64],
    v: &[f64],
) -> (Vec<f64>, Vec<f64>) {
    let mut cap = vec![0.0; water.len()];
    let mut cond = vec![0.0; water.len()];
    temp_cap(
        TempCapInput {
            material_indices: indices,
            water_content: water,
            velocity: v,
            materials,
            campbell: false,
            x_conv: X_CONV,
            t_conv: T_CONV,
        },
        &mut cap,
        &mut cond,
    )
    .expect("temp_cap");
    (cap, cond)
}

/// Everything one step's rows need.
struct StepData<'a> {
    x: &'a [f64],
    dt: f64,
    cw: f64,
    sink: &'a [f64],
    t_old: &'a [f64],
    t_new: &'a [f64],
    cap_o: &'a [f64],
    cond_o: &'a [f64],
    v_old: &'a [f64],
    cap_n: &'a [f64],
    cond_n: &'a [f64],
    v_new: &'a [f64],
}

/// Interior row residual and scale at node `i` (`TEMPER.FOR:112-143`, vapor terms absent).
fn heat_only_row_residual(d: &StepData<'_>, i: usize) -> (f64, f64) {
    let (x, s, cw, dt) = (d.x, d.sink, d.cw, d.dt);
    let dxa = x[i] - x[i - 1];
    let dxb = x[i + 1] - x[i];
    let dx = (x[i + 1] - x[i - 1]) / 2.0;
    let (co, cn, vo, vn) = (d.cond_o, d.cond_n, d.v_old, d.v_new);

    let f0 = d.t_old[i - 1]
        * ((co[i] + co[i - 1]) / 4.0 / dxa + cw * (vo[i] + 2.0 * vo[i - 1]) / 12.0
            - dxa / 24.0 * cw * (s[i - 1] + s[i]));
    let f1 = d.t_old[i]
        * (-cw * (vo[i + 1] - vo[i - 1]) / 12.0 + dx / dt * d.cap_o[i]
            - (co[i + 1] + co[i]) / 4.0 / dxb
            - (co[i] + co[i - 1]) / 4.0 / dxa
            - dxa / 24.0 * cw * (s[i - 1] + 3.0 * s[i])
            - dxb / 24.0 * cw * (3.0 * s[i] + s[i + 1]));
    let f2 = d.t_old[i + 1]
        * ((co[i + 1] + co[i]) / 4.0 / dxb
            - cw * (2.0 * vo[i + 1] + vo[i]) / 12.0
            - dxb / 24.0 * cw * (s[i + 1] + s[i]));

    let b = -(cn[i] + cn[i - 1]) / 4.0 / dxa - cw * (vn[i] + 2.0 * vn[i - 1]) / 12.0
        + dxa / 24.0 * cw * (s[i - 1] + s[i]);
    let dd = (cn[i - 1] + cn[i]) / 4.0 / dxa
        + (cn[i] + cn[i + 1]) / 4.0 / dxb
        + dx / dt * d.cap_n[i]
        + cw * (vn[i + 1] - vn[i - 1]) / 12.0
        + dxa / 24.0 * cw * (s[i - 1] + 3.0 * s[i])
        + dxb / 24.0 * cw * (3.0 * s[i] + s[i + 1]);
    let e = -(cn[i] + cn[i + 1]) / 4.0 / dxb
        + cw * (2.0 * vn[i + 1] + vn[i]) / 12.0
        + dxb / 24.0 * cw * (s[i + 1] + s[i]);
    let l0 = b * d.t_new[i - 1];
    let l1 = dd * d.t_new[i];
    let l2 = e * d.t_new[i + 1];
    let f = f0 + f1 + f2;
    let scale = [l0, l1, l2, f0, f1, f2]
        .iter()
        .fold(0.0_f64, |m, t| m.max(t.abs()));
    (l0 + l1 + l2 - f, scale)
}

/// `|R| / max(rel * scale, floor)`.
fn ratio(residual: f64, scale: f64, rel: f64, floor: f64) -> f64 {
    residual.abs() / (rel * scale).max(floor)
}

#[derive(Default)]
struct Outcome {
    steps: usize,
    worst_heat: f64,
    all_finite: bool,
    water_finite: bool,
    max_scale: f64,
    max_abs_dt_temp: f64,
    max_abs_v: f64,
    water_max_abs: f64,
    water_max_ratio: f64,
    initial_temps: Vec<f64>,
    final_temps: Vec<f64>,
    violations: Vec<String>,
    /// Steps whose accepted dt differs from the declared dt (landing remainders).
    short_steps: Vec<String>,
}

fn run_step_off(bound: &Bound) -> Outcome {
    let dir = repo_root().join("reference/vapor_fortran/cases").join(CASE);
    let deck = parse_deck(&fs::read_to_string(dir.join("SELECTOR.IN")).expect("SELECTOR.IN"));
    let profile = parse_profile(&fs::read_to_string(dir.join("PROFILE.DAT")).expect("PROFILE.DAT"));
    assert!(profile.material_indices.iter().all(|&i| i == 0));

    let n = profile.depths.len();
    let x = rev(&profile.depths);
    let heads = rev(&profile.heads);
    let temps = rev(&profile.temperatures);
    let indices: Vec<usize> = profile.material_indices.iter().rev().copied().collect();
    let (head_top, head_bottom) = (heads[n - 1], heads[0]);

    let analytic = VanGenuchtenModel::new(
        VanGenuchtenParameters::new(deck.vg[0], deck.vg[1], deck.vg[2], deck.vg[3], deck.vg[4])
            .expect("vG parameters"),
    )
    .expect("vG model");
    let tables = HydraulicTables::new(
        &analytic,
        -deck.h_tab_n,
        -deck.h_tab_1,
        100,
        UnitSymbol::new("cm").unwrap(),
        UnitSymbol::new("cm/h").unwrap(),
        UnitSymbol::new("1/cm").unwrap(),
    )
    .expect("hydraulic tables");
    let span = SpanModel {
        table: &tables,
        analytic: &analytic,
    };
    let models: [&dyn HydraulicModel; 1] = [&span];
    let initial: Vec<Evaluation> = heads
        .iter()
        .map(|&h| span.evaluate(h).expect("initial evaluation"))
        .collect();
    let theta: Vec<f64> = initial.iter().map(|e| e.theta).collect();
    let conductivity: Vec<f64> = initial.iter().map(|e| e.conductivity).collect();

    let mesh = Mesh1D::new(x.clone(), vec![0; n], None, 1, 0, deck.cos_alpha).expect("mesh");
    let materials = vec![TempMaterial::from_tpar(deck.tpar)];
    let cw = materials[0].water_heat_capacity;
    let heat_input = HeatConfigInput {
        l_vapor: false,
        l_en_bal: false,
        i_dual_por: 0,
        l_w_dep: false,
        i_model: 0,
        domain_count: 1,
        k_top_t: deck.k_top_t,
        t_period: 0.0,
        k_bot_t: deck.k_bot_t,
        campbell: false,
    };
    let policy = WaterStepPolicy {
        max_iterations: deck.max_iterations,
        head_tolerance: deck.tol_h,
        water_tolerance: deck.tol_th,
        minimum_timestep: deck.dt_min,
        retry_factor: RETRY_FACTOR,
        minimum_pivot: MINIMUM_PIVOT,
    };
    let coupled = CoupledHeatInput {
        top_temperature: deck.t_top,
        bottom_temperature: deck.t_bot,
        x_conv: X_CONV,
        t_conv: T_CONV,
        minimum_pivot: MINIMUM_PIVOT,
    };
    let sink = vec![0.0; n];
    let mut water =
        WaterState::new(0.0, heads.clone(), theta.clone()).expect("initial water state");
    let mut workspace = WaterWorkspace::new(n).expect("workspace");

    let mut v_old = initial_velocity(&x, &heads, &conductivity, deck.cos_alpha);
    let mut driver = HeatWaterDriver::new(
        heat_input,
        &x,
        &indices,
        &materials,
        0.0,
        temps.clone(),
        &v_old,
    )
    .expect("legacy heat driver");

    let mut out = Outcome {
        all_finite: true,
        water_finite: true,
        initial_temps: driver.state().temperatures().to_vec(),
        ..Outcome::default()
    };
    for &print_time in &deck.print_times[1..] {
        while water.time() < print_time - 1e-12 {
            let input = WaterStepInput {
                mesh: &mesh,
                materials: &models,
                sink: &sink,
                root_uptake: None,
                timestep: deck.dt,
                target_time: print_time,
                face_averaging: FaceAveraging::Arithmetic,
                lower: LowerBoundaryMode::FixedHead {
                    head: head_bottom,
                    row: placeholder_row(head_bottom),
                    flux: 0.0,
                },
                upper: UpperBoundary::Head {
                    head: head_top,
                    row: placeholder_row(head_top),
                    flux: 0.0,
                },
            };
            let theta_old = water.water_contents().to_vec();
            let t_old = driver.state().temperatures().to_vec();
            // Replay the water step on clones to obtain the face fluxes `Veloc` needs.
            let (face, dt_replay) = {
                let mut trial = water.clone();
                let mut trial_workspace = workspace.clone();
                let report = advance_water_step(&mut trial, &mut trial_workspace, input, policy)
                    .expect("replayed water step");
                (report.face_fluxes.to_vec(), report.timestep)
            };
            let report = driver
                .step(&mut water, &mut workspace, input, policy, coupled)
                .expect("accepted legacy step");
            assert_eq!(report.timestep.to_bits(), dt_replay.to_bits(), "replay dt");
            let dt = report.timestep;
            let step = out.steps;
            if (dt - deck.dt).abs() > 1e-9 {
                out.short_steps
                    .push(format!("step {step} t={print_time} dt={dt:e}"));
            }
            out.steps += 1;

            let t_new = driver.state().temperatures().to_vec();
            let theta_new = water.water_contents().to_vec();
            let v_new = node_velocities(&x, &face, &theta_old, &theta_new, &sink, dt);
            let (cap_o, cond_o) = cap_cond(&materials, &indices, &theta_old, &v_old);
            let (cap_n, cond_n) = cap_cond(&materials, &indices, &theta_new, &v_new);
            let data = StepData {
                x: &x,
                dt,
                cw,
                sink: &sink,
                t_old: &t_old,
                t_new: &t_new,
                cap_o: &cap_o,
                cond_o: &cond_o,
                v_old: &v_old,
                cap_n: &cap_n,
                cond_n: &cond_n,
                v_new: &v_new,
            };
            for i in 1..n - 1 {
                let (r, scale) = heat_only_row_residual(&data, i);
                out.all_finite &= r.is_finite() && scale.is_finite();
                out.max_scale = out.max_scale.max(scale);
                let q = ratio(r, scale, bound.relative, bound.absolute);
                out.worst_heat = out.worst_heat.max(q);
                if q > 1.0 || !q.is_finite() {
                    out.violations.push(format!(
                        "step {step} t={print_time} heat node {i} residual {r:e} scale {scale:e} ratio {q:e}"
                    ));
                }
                // Water continuum residual (diagnostic only, never asserted).
                let dx = (x[i + 1] - x[i - 1]) / 2.0;
                let storage = dx * ((theta_new[i] - theta_old[i]) / dt + sink[i]);
                let flux = face[i + 1] - face[i];
                let rw = storage + flux;
                let sw = storage.abs().max(flux.abs());
                out.water_finite &= rw.is_finite();
                out.water_max_abs = out.water_max_abs.max(rw.abs());
                if sw > 0.0 {
                    out.water_max_ratio = out.water_max_ratio.max(rw.abs() / sw);
                }
            }
            for (a, b) in t_new.iter().zip(&t_old) {
                out.max_abs_dt_temp = out.max_abs_dt_temp.max((a - b).abs());
            }
            out.max_abs_v = v_new.iter().fold(out.max_abs_v, |m, v| m.max(v.abs()));
            v_old = v_new;
        }
        assert!(
            (water.time() - print_time).abs() < 1e-9,
            "{CASE}: time landing at {print_time}"
        );
    }
    out.final_temps = driver.state().temperatures().to_vec();
    out
}

#[test]
fn step_off_interior_heat_balance_meets_approved_bounds_every_step() {
    let bound = load_bound("heat_balance_interior");
    let out = run_step_off(&bound);
    println!(
        "{CASE}: {} steps, worst heat ratio {:.3e} (bounds rel {:.1e}, abs {:.1e}), max row scale {:.3e}, max |dT| {:.3e}, max |v| {:.3e}",
        out.steps, out.worst_heat, bound.relative, bound.absolute, out.max_scale, out.max_abs_dt_temp, out.max_abs_v
    );
    println!(
        "{CASE}: water continuum residual (DIAGNOSTIC, not asserted): max |R| {:.3e}, max |R|/scale {:.3e}",
        out.water_max_abs, out.water_max_ratio
    );
    println!("{CASE}: steps off the declared dt: {:?}", out.short_steps);
    assert_eq!(
        out.steps,
        EXPECTED_STEPS + out.short_steps.len(),
        "accepted step count at dt 0.1 plus landing remainders"
    );
    assert!(out.all_finite, "{CASE}: non-finite heat residual or scale");
    assert!(out.water_finite, "{CASE}: non-finite water diagnostic");
    assert!(out.max_scale > 0.0, "{CASE}: heat row terms are all zero");
    assert!(
        out.max_abs_dt_temp > 0.0,
        "{CASE}: temperatures never change"
    );
    assert_ne!(
        out.initial_temps, out.final_temps,
        "{CASE}: final temperatures equal initial"
    );
    assert!(
        out.violations.is_empty(),
        "{CASE}: {} interior rows exceed the criteria bounds:\n{}",
        out.violations.len(),
        out.violations.join("\n")
    );
}

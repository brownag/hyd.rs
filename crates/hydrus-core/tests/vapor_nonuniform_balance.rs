//! Per-step interior heat and water balance of the vapor driver on the nonuniform-temperature
//! step case `vapor_nonuniform_step_on` (task 27.4.2).
//!
//! Uses the existing `heat::vapor::balance::vapor_balance` helper on every accepted step. Bounds are
//! read from `reference/vapor_fortran/criteria.json` `checks` (`water_balance_interior`,
//! `heat_balance_interior`); none are copied here. Only interior rows are asserted. Endpoint
//! (Dirichlet) rows are not closed, so fixed-head boundary flux is outside the interior balance and
//! `reservoir.unclosed` is never asserted.
//!
//! Scope: zero-flux interior balance only (27.1.3 proposed no flux cases), as in
//! `vapor_heat_boundary_balance.rs`. The deck and profile are parsed from the case's own
//! `SELECTOR.IN`/`PROFILE.DAT` (helpers copied from `vapor_nonuniform_parity.rs`; not imported).
//! Criteria identity is not checked here; the parity test handles it.
//!
//! Schedule: `vapor_nonuniform_step_on` is driven at the Fortran dt/3 on every step (7200 steps), as
//! in the parity test. Rust's own time-step controller is NOT tested.
//!
//! Not covered: `vapor_nonuniform_step_off` has `lVapor = f`. The vapor driver rejects
//! `l_vapor = false` (`heat/vapor/mod.rs`, `UnsupportedBoolean`), and the legacy heat driver has no
//! balance helper, so no existing helper applies to it. Reported as a gap; not covered here.

use std::fs;
use std::path::PathBuf;

use hydrus_core::heat::capacity::TempMaterial;
use hydrus_core::heat::config::HeatConfigInput;
use hydrus_core::heat::driver::CoupledHeatInput;
use hydrus_core::heat::vapor::balance::{vapor_balance, VaporBalanceInput};
use hydrus_core::heat::vapor::driver::VaporWaterHeatDriver;
use hydrus_core::heat::vapor::state::VaporStateInput;
use hydrus_core::heat::vapor::{
    VaporConfig, VaporConfigInput, VaporPrimitiveInput, VaporSourceMaterial,
};
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, HydraulicTables,
    Mesh1D, UnitSymbol, VanGenuchtenModel, VanGenuchtenParameters,
};
use hydrus_core::{
    HydraulicError, LowerBoundaryMode, UpperBoundary, WaterState, WaterStepInput, WaterStepPolicy,
    WaterWorkspace,
};
use serde_json::Value;

const CASE: &str = "vapor_nonuniform_step_on";
const X_CONV: f64 = 100.0; // INPUT.FOR:190 (cm)
const T_CONV: f64 = 1.0 / 3600.0; // INPUT.FOR:197 (hours)
const MINIMUM_PIVOT: f64 = 1e-12;
const RETRY_FACTOR: f64 = 3.0; // source dt/3
/// Accepted source steps in `RUN_INF.OUT` for this case (vapor_nonuniform_parity.rs header).
const SOURCE_STEPS: usize = 7200;

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
// Deck and profile (copied from vapor_nonuniform_parity.rs)
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
    hysteresis: i32,
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

    // Supported selection: lVapor on, lChem/lWDep/lMeteo off, fixed heads both ends, Campbell 0.
    assert!(flag(&block1[0]) && !flag(&block1[1]), "lWat on, lChem off");
    assert!(!flag(&block1[6]) && !flag(&block2[2]), "lWDep/lMeteo off");
    assert!(flag(&block2[3]), "lVapor on");
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
        hysteresis: model[1] as i32,
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

// ----------------------------------------------------------------------------------------------
// Balance run
// ----------------------------------------------------------------------------------------------

#[derive(Default)]
struct Outcome {
    steps: usize,
    worst_water: f64,
    worst_heat: f64,
    vapor_storage_nonzero: bool,
    vapor_head_transport_nonzero: bool,
    vapor_latent_nonzero: bool,
    residuals_finite: bool,
    /// Rows whose ratio exceeds 1: `step, phase, node, residual, scale, ratio`.
    violations: Vec<String>,
}

/// Interior rows of one step: residual, scale and ratio `|R| / max(rel*scale, floor)`.
fn ratio(residual: f64, scale: f64, rel: f64, floor: f64) -> f64 {
    residual.abs() / (rel * scale).max(floor)
}

fn run_step_on(water_bound: &Bound, heat_bound: &Bound) -> Outcome {
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
    let capacity: Vec<f64> = initial.iter().map(|e| e.capacity).collect();

    let mesh = Mesh1D::new(x.clone(), vec![0; n], None, 1, 0, deck.cos_alpha).expect("mesh");
    let materials = vec![TempMaterial::from_tpar(deck.tpar)];
    let config = VaporConfig::new(VaporConfigInput {
        heat: HeatConfigInput {
            l_vapor: true,
            l_en_bal: false,
            i_dual_por: 0,
            l_w_dep: false,
            i_model: 0,
            domain_count: 1,
            k_top_t: deck.k_top_t,
            t_period: 0.0,
            k_bot_t: deck.k_bot_t,
            campbell: false,
        },
        water_enabled: true,
        heat_enabled: true,
        i_enhanc: 1, // contract: the pinned executable hardcodes enhancement on
        hysteresis: deck.hysteresis,
        chemistry: false,
        density_coupling: false,
        centrifugal: false,
        top_temperature: deck.t_top,
        bottom_temperature: deck.t_bot,
        x_conv: X_CONV,
        t_conv: T_CONV,
    })
    .expect("vapor config");
    let primitive = VaporPrimitiveInput {
        heads: &heads,
        temperatures: &temps,
        liquid_water_contents: &theta,
        liquid_conductivities: &conductivity,
        liquid_capacities: &capacity,
        material_indices: &indices,
        saturated_water_contents: &[deck.vg[1]],
        x_conv: X_CONV,
        t_conv: T_CONV,
    };
    let mut driver = VaporWaterHeatDriver::new(
        config,
        VaporStateInput {
            primitive,
            coordinates: &x,
            gravity: deck.cos_alpha,
            initial_head_predictor: &heads,
            initialization_timestep: deck.dt,
            time: 0.0,
        },
        &indices,
        &materials,
    )
    .expect("vapor driver");
    let source = [VaporSourceMaterial {
        model: &analytic,
        table: Some(&tables),
    }];

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

    // Fortran schedule: every step at dt/3 (vapor_nonuniform_parity.rs:984-992).
    let source_dt = deck.dt / RETRY_FACTOR;
    let mut out = Outcome {
        residuals_finite: true,
        ..Outcome::default()
    };
    for window in deck.print_times.windows(2) {
        let (t0, t1) = (window[0], window[1]);
        let count = ((t1 - t0) / source_dt).round() as usize;
        for _ in 0..count {
            let (old_t, old_lv, old_vv) = {
                let snap = driver.state().snapshot();
                (
                    snap.temperatures.to_vec(),
                    snap.liquid_velocities.to_vec(),
                    snap.vapor_velocities.to_vec(),
                )
            };
            let report = driver
                .step(
                    &mut water,
                    &mut workspace,
                    WaterStepInput {
                        mesh: &mesh,
                        materials: &models,
                        sink: &sink,
                        root_uptake: None,
                        timestep: source_dt,
                        target_time: t1,
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
                    },
                    policy,
                    &source,
                    coupled,
                )
                .expect("accepted vapor step");
            let b = vapor_balance(
                &VaporBalanceInput {
                    coordinates: &x,
                    gravity: deck.cos_alpha,
                    x_conv: X_CONV,
                    t_conv: T_CONV,
                    sink: &sink,
                    old_temperatures: &old_t,
                    old_liquid_velocities: &old_lv,
                    old_vapor_velocities: &old_vv,
                    heat_materials: &materials,
                    material_indices: &indices,
                    campbell: false,
                },
                &report,
            )
            .expect("balance");
            let step = out.steps;
            out.steps += 1;

            out.vapor_storage_nonzero |= b.water.vapor_storage.iter().any(|v| v.abs() > 0.0);
            out.vapor_head_transport_nonzero |=
                b.water.vapor_head_transport.iter().any(|v| v.abs() > 0.0);
            out.vapor_latent_nonzero |= b.heat.vapor_latent.iter().any(|v| v.abs() > 0.0);
            out.residuals_finite &= b
                .water
                .residual
                .iter()
                .chain(&b.heat.residual)
                .all(|v| v.is_finite());

            // Row i (interior node index i) sits at vector index i-1 (balance.rs:76, :335).
            for (phase, residual, scale, rel, floor, worst) in [
                (
                    "water",
                    &b.water.residual,
                    &b.water.scale,
                    water_bound.relative,
                    water_bound.absolute,
                    &mut out.worst_water,
                ),
                (
                    "heat",
                    &b.heat.residual,
                    &b.heat.scale,
                    heat_bound.relative,
                    heat_bound.absolute,
                    &mut out.worst_heat,
                ),
            ] {
                for (k, (&r, &s)) in residual.iter().zip(scale.iter()).enumerate() {
                    let q = ratio(r, s, rel, floor);
                    *worst = worst.max(q);
                    if q > 1.0 {
                        out.violations.push(format!(
                            "step {step} t={t1} {phase} node {} residual {r:e} scale {s:e} ratio {q:e}",
                            k + 1
                        ));
                    }
                }
            }
        }
        assert!(
            (water.time() - t1).abs() < 1e-9,
            "{CASE}: time landing at {t1}"
        );
    }
    out
}

#[test]
fn step_on_interior_balance_meets_approved_bounds_every_step() {
    let water_bound = load_bound("water_balance_interior");
    let heat_bound = load_bound("heat_balance_interior");
    let out = run_step_on(&water_bound, &heat_bound);
    println!(
        "{CASE}: {} steps, worst water ratio {:.3e}, worst heat ratio {:.3e} (bounds rel {:.1e}, abs {:.1e})",
        out.steps, out.worst_water, out.worst_heat, water_bound.relative, water_bound.absolute
    );
    assert_eq!(out.steps, SOURCE_STEPS, "accepted step count at dt/3");
    assert!(out.residuals_finite, "{CASE}: non-finite residual");
    assert!(out.vapor_storage_nonzero, "{CASE}: vapor storage is zero");
    assert!(
        out.vapor_head_transport_nonzero,
        "{CASE}: vapor head transport is zero"
    );
    assert!(
        out.vapor_latent_nonzero,
        "{CASE}: vapor latent term is zero"
    );
    assert!(
        out.violations.is_empty(),
        "{CASE}: {} interior rows exceed the criteria bounds:\n{}",
        out.violations.len(),
        out.violations.join("\n")
    );
}

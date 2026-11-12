//! Source-matched parity of the coupled heat driver against pinned-Fortran heat fixtures.
//!
//! Inputs are rebuilt from the case-folder `SELECTOR.IN` and `PROFILE.DAT`, whose sha256 must
//! match `manifest.input_sha256` in each fixture. Comparison runs on fixture values, not on a
//! live Fortran run. Pass criterion: the rounded verdict (|round(rust, 2) - fixture| <= 0.01).
//! The strict verdict (2e-4 degC, contract `## Tolerances`) is reported next to it and is not
//! hidden. Candidate temperatures are written for `tools/compare_heat_reference.py`.
//!
//! Source anchors:
//! - Units: `INPUT.FOR:190` (xConv = 100, cm), `INPUT.FOR:197` (tConv = 1/3600, hours).
//! - Water heads: `INPUT.FOR:51,56,61` (KodTop, KodBot); `INPUT.FOR:351-352` (hBot = hNew(1),
//!   hTop = hNew(NumNP) from the initial profile); `WATFLOW.FOR:316,328,338,349` (Dirichlet).
//!   `SetBC` is gated by TopInF/BotInF/AtmBC (`HYDRUS.FOR:294`), all false here.
//! - Initial vOld: `HYDRUS.FOR:380-385` calls `Veloc` on the initial head before the first step.
//! - Time-step retry: `TIME.FOR:19` (DMul2 = 0.5 reduces dt, so retry_factor = 1/DMul2).
//! - Tortuosity: the SELECTOR `l` value 0.5 equals the Fortran Mualem BPar (`MATERIAL.FOR:59`).
//!
//! Water model: the water step uses the table model, `HydraulicTables` built from the analytic
//! van Genuchten model on the Fortran GenMat grid (`INPUT.FOR:762-810`, `:1590` NTab = 100,
//! `:777` log10-spaced knots from |h| = 1e-4 to 100 cm). Knot values are the analytic functions
//! at the knots, and between knots both sides interpolate linearly in head. This reproduces the
//! Fortran K(-10) and theta(-10) to about 1e-7 (the analytic model is off by about 1.3e-2 in K,
//! which caused the earlier 0.02 degC residual).
//!
//! Known differences from `SetMat` (`WATFLOW.FOR`), not reproduced here:
//! - `SetMat` evaluates K at the mixed head `hiM = 0.1*hi1 + 0.9*hi2`. The Rust table evaluates at
//!   the trial head. The two agree when heads do not change, which is the case in these fixtures
//!   (fixed heads of -10 on a uniform column).
//! - `SetMat` falls back to analytic FK outside the table range. The Rust table rejects heads
//!   outside it. The fixtures stay inside the range.
//!
//! Campbell cases (`heat_flux_top_dir_bot_campbell`, `heat_dir_top_flux_bot_campbell`) stay at
//! 25 degC. They are weak evidence: they match exactly but do not test conduction.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use hydrus_core::heat::capacity::TempMaterial;
use hydrus_core::heat::config::HeatConfigInput;
use hydrus_core::heat::driver::{CoupledHeatInput, HeatWaterDriver};
use hydrus_core::hydraulics::{
    BoundaryRow, FaceAveraging, HydraulicModel, HydraulicTables, Mesh1D, UnitSymbol,
    VanGenuchtenModel, VanGenuchtenParameters,
};
use hydrus_core::{
    LowerBoundaryMode, UpperBoundary, WaterState, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};
use sha2::{Digest, Sha256};

const CASES: [&str; 4] = [
    "heat_dir_top_dir_bot_default",
    "heat_dir_top_flux_bot_default",
    "heat_flux_top_dir_bot_campbell",
    "heat_dir_top_flux_bot_campbell",
];
const STRICT_TOLERANCE: f64 = 2e-4;
const ROUNDED_TOLERANCE: f64 = 0.01;
const ROUNDED_EPSILON: f64 = 1e-9;
const X_CONV: f64 = 100.0; // INPUT.FOR:190 (cm)
const T_CONV: f64 = 1.0 / 3600.0; // INPUT.FOR:197 (hours)
const MINIMUM_PIVOT: f64 = 1e-12;
const TIMESTEP_KEY: &str = "dt dtMin dtMax";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("hydrus-core must remain inside the workspace")
        .to_path_buf()
}

fn fixture_dir() -> PathBuf {
    repo_root().join("reference/heat_fortran")
}

fn output_dir() -> PathBuf {
    let dir = env::var_os("HYDRUS_HEAT_PARITY_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join(".development/24.5.2"));
    fs::create_dir_all(&dir).expect("create heat parity output directory");
    dir
}

fn sha256_hex(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
    format!("{:x}", Sha256::digest(&bytes))
}

/// Words on the line after the first line that contains `key`.
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
    l_temp: bool,
    l_w_dep: bool,
    l_meteo: bool,
    l_vapor: bool,
    cos_alpha: f64,
    max_iterations: usize,
    tol_th: f64,
    tol_h: f64,
    top_inf: bool,
    k_top: i32,
    bot_inf: bool,
    k_bot: i32,
    vg: [f64; 6],
    dt: f64,
    dt_min: f64,
    dmul2: f64,
    print_times: Vec<f64>,
    t_top: f64,
    t_bot: f64,
    k_top_t: i32,
    k_bot_t: i32,
    campbell: bool,
    t_period: f64,
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
    let model = after(&lines, "Model Hysteresis");
    let vg = numbers(&after(&lines, "thr ths Alfa n Ks l"));
    let time = numbers(&after(&lines, TIMESTEP_KEY));
    let tmax = numbers(&after(&lines, "tInit tMax"));
    let tprint = numbers(&after(&lines, "TPrint(1...MPL)"));
    let tpar = numbers(&after(&lines, "record 2 (TempIn skip)"));
    let campbell = after(&lines, "amplitude/period/Campbell");
    let bc = numbers(&after(&lines, "boundary code record"));

    assert_eq!(block1.len(), 10, "BasInf line 10 must have ten flags");
    assert_eq!(block2.len(), 6, "BasInf iVer4 line must have six flags");
    assert_eq!(model.len(), 2, "Model line");
    assert_eq!(
        numbers(&model)[0],
        0.0,
        "only van Genuchten (Model 0) is supported"
    );
    assert_eq!(vg.len(), 6);
    assert_eq!(tpar.len(), 9, "one TPar row of nine values");
    assert_eq!(bc.len(), 4, "boundary code record");

    let tpar_row: [f64; 9] = tpar.try_into().expect("nine TPar values");
    let period: f64 = campbell[1].parse().expect("tPeriod");
    assert_eq!(
        period, 0.0,
        "tPeriod must be zero (contract rejects tPeriod > 0)"
    );
    let campbell_flag: f64 = campbell[2].parse().expect("iCampbell");
    assert!(
        campbell_flag == 0.0 || campbell_flag == 1.0,
        "iCampbell must be 0 or 1"
    );
    let print_times = {
        let mut times = vec![tmax[0]];
        times.extend(tprint);
        times
    };
    assert_eq!(print_times[0], 0.0, "tInit must be zero");

    Deck {
        l_temp: flag(&block1[2]),
        l_w_dep: flag(&block1[6]),
        l_meteo: flag(&block2[2]),
        l_vapor: flag(&block2[3]),
        cos_alpha: numbers(&nmat)[2],
        max_iterations: numbers(&tols)[0] as usize,
        tol_th: numbers(&tols)[1],
        tol_h: numbers(&tols)[2],
        top_inf: flag(&top[0]),
        k_top: top[2].parse().expect("KodTop"),
        bot_inf: flag(&bot[0]),
        k_bot: bot[4].parse().expect("KodBot"),
        vg: vg.try_into().expect("six vG values"),
        dt: time[0],
        dt_min: time[1],
        dmul2: time[4],
        print_times,
        t_top: bc[1],
        t_bot: bc[3],
        k_top_t: bc[0] as i32,
        k_bot_t: bc[2] as i32,
        campbell: campbell_flag == 1.0,
        t_period: period,
        tpar: tpar_row,
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

fn load_fixture(case: &str) -> serde_json::Value {
    let path = fixture_dir().join("fixtures").join(format!("{case}.json"));
    let text = fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
    serde_json::from_str(&text).expect("fixture JSON")
}

/// Fails the test when a case-folder input does not match its fixture manifest.
fn verify_inputs(case: &str, fixture: &serde_json::Value) {
    let hashes = fixture["manifest"]["input_sha256"]
        .as_object()
        .expect("manifest.input_sha256");
    for name in ["SELECTOR.IN", "PROFILE.DAT"] {
        let expected = hashes[name].as_str().expect("input hash");
        let actual = sha256_hex(&fixture_dir().join("cases").join(case).join(name));
        assert_eq!(
            actual, expected,
            "{case}/{name} sha256 differs from fixture manifest"
        );
    }
}

/// `Veloc` at t = 0 (`WATFLOW.FOR:717-775`), as called at `HYDRUS.FOR:380-385`: Sink = 0,
/// ThNew = ThOld, so the storage terms vanish. `k` is K(h) per node, `g` is CosAlpha.
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

/// Dirichlet boundary rows are replaced by the water solver (`boundary.rs`, Dirichlet branch),
/// so only the finite placeholder values reach the solve. The measured exchange is not used by
/// the heat driver, which takes face fluxes from the accepted water step.
fn placeholder_row(rhs: f64) -> BoundaryRow {
    BoundaryRow {
        lower: 0.0,
        diagonal: 1.0,
        upper: 0.0,
        rhs,
    }
}

/// Returns temperatures at the print times, top node first (fixture order), t = 0 included.
fn run_case(case: &str) -> Vec<Vec<f64>> {
    let fixture = load_fixture(case);
    verify_inputs(case, &fixture);
    let deck = parse_deck(
        &fs::read_to_string(fixture_dir().join("cases").join(case).join("SELECTOR.IN")).unwrap(),
    );
    let profile = parse_profile(
        &fs::read_to_string(fixture_dir().join("cases").join(case).join("PROFILE.DAT")).unwrap(),
    );

    let fixture_depths: Vec<f64> = fixture["node_depths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(
        profile.depths, fixture_depths,
        "PROFILE depths must match the fixture"
    );
    let fixture_times: Vec<f64> = fixture["time_points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(
        deck.print_times, fixture_times,
        "TPrint must match the fixture time points"
    );

    // Flags, exactly as SELECTOR.IN states them.
    assert!(deck.l_temp, "lTemp must be true");
    assert!(
        !deck.l_w_dep && !deck.l_vapor && !deck.l_meteo,
        "heat-only flags"
    );
    assert!(
        !deck.top_inf && !deck.bot_inf,
        "water heads are fixed (TopInF, BotInF false)"
    );
    assert!(
        deck.k_top >= 1 && deck.k_bot >= 1,
        "KodTop, KodBot = 1 (fixed head)"
    );
    assert_eq!(deck.cos_alpha, 1.0, "CosAlpha");
    assert_eq!(
        deck.vg[5], 0.5,
        "SELECTOR tortuosity l must equal the Mualem BPar (0.5)"
    );
    assert!(
        profile.material_indices.iter().all(|&i| i == 0),
        "one material"
    );

    // Columns: Fortran node 1 is the top. Driver index 0 is the bottom, x ascending.
    let n = profile.depths.len();
    let x: Vec<f64> = profile.depths.iter().rev().copied().collect();
    let heads: Vec<f64> = profile.heads.iter().rev().copied().collect();
    let temps: Vec<f64> = profile.temperatures.iter().rev().copied().collect();
    let indices: Vec<usize> = profile.material_indices.iter().rev().copied().collect();
    let head_top = heads[n - 1];
    let head_bottom = heads[0];

    let analytic = VanGenuchtenModel::new(
        VanGenuchtenParameters::new(deck.vg[0], deck.vg[1], deck.vg[2], deck.vg[3], deck.vg[4])
            .expect("vG parameters"),
    )
    .expect("vG model");
    // INPUT.FOR GenMat: 100 log10-spaced knots from h = -1e-4 to h = -100 (see header).
    let vg_model = HydraulicTables::new(
        &analytic,
        -100.0,
        -0.0001,
        100,
        UnitSymbol::new("cm").expect("head unit"),
        UnitSymbol::new("cm/h").expect("conductivity unit"),
        UnitSymbol::new("1/cm").expect("capacity unit"),
    )
    .expect("table model");
    let models: [&dyn HydraulicModel; 1] = [&vg_model];
    let theta: Vec<f64> = heads
        .iter()
        .map(|&h| vg_model.theta(h).expect("theta"))
        .collect();
    let conductivity: Vec<f64> = heads
        .iter()
        .map(|&h| vg_model.conductivity(h).expect("conductivity"))
        .collect();
    let velocity0 = initial_velocity(&x, &heads, &conductivity, deck.cos_alpha);

    let mesh = Mesh1D::new(x.clone(), vec![0; n], None, 1, 0, deck.cos_alpha).expect("mesh");
    let material = TempMaterial::from_tpar(deck.tpar);
    let heat_config = HeatConfigInput {
        l_vapor: deck.l_vapor,
        l_en_bal: false,
        i_dual_por: 0,
        l_w_dep: deck.l_w_dep,
        i_model: 0,
        domain_count: 1,
        k_top_t: deck.k_top_t,
        t_period: deck.t_period,
        k_bot_t: deck.k_bot_t,
        campbell: deck.campbell,
    };
    let mut driver = HeatWaterDriver::new(
        heat_config,
        &x,
        &indices,
        &[material],
        0.0,
        temps,
        &velocity0,
    )
    .expect("heat driver");
    let mut water = WaterState::new(0.0, heads.clone(), theta).expect("water state");
    let mut workspace = WaterWorkspace::new(n).expect("water workspace");
    let policy = WaterStepPolicy {
        max_iterations: deck.max_iterations,
        head_tolerance: deck.tol_h,
        water_tolerance: deck.tol_th,
        minimum_timestep: deck.dt_min,
        retry_factor: 1.0 / deck.dmul2,
        minimum_pivot: MINIMUM_PIVOT,
    };
    let sink = vec![0.0; n];
    let coupled = CoupledHeatInput {
        top_temperature: deck.t_top,
        bottom_temperature: deck.t_bot,
        x_conv: X_CONV,
        t_conv: T_CONV,
        minimum_pivot: MINIMUM_PIVOT,
    };

    let mut rows = vec![driver
        .state()
        .temperatures()
        .iter()
        .rev()
        .copied()
        .collect::<Vec<f64>>()];
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
            driver
                .step(&mut water, &mut workspace, input, policy, coupled)
                .unwrap_or_else(|error| panic!("{case}: coupled step failed: {error:?}"));
        }
        assert!(
            (water.time() - print_time).abs() < 1e-9,
            "{case}: accepted time {} differs from print time {print_time}",
            water.time()
        );
        rows.push(
            driver
                .state()
                .temperatures()
                .iter()
                .rev()
                .copied()
                .collect(),
        );
    }
    rows
}

/// Rounding to two decimals, ties upward (the same rule in the Python comparator).
fn round2(value: f64) -> f64 {
    (value * 100.0 + 0.5).floor() / 100.0
}

fn check_case(case: &str) {
    let fixture = load_fixture(case);
    let fixture_rows: Vec<Vec<f64>> = fixture["temperatures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        })
        .collect();
    let times: Vec<f64> = fixture["time_points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let candidate = run_case(case);

    let output = serde_json::json!({
        "case_id": case,
        "time_points": times,
        "temperatures": candidate,
    });
    let path = output_dir().join(format!("{case}.json"));
    fs::write(&path, serde_json::to_string_pretty(&output).unwrap()).expect("write candidate");

    println!(
        "{:<34} {:<6} {:>5} {:>10} {:>5} {:>10} rounded",
        "fixture", "field", "time", "max_strict", "fail", "max_round"
    );
    let mut all_pass = true;
    for (index, time) in times.iter().enumerate() {
        let mut max_strict: f64 = 0.0;
        let mut failing = 0usize;
        let mut max_rounded: f64 = 0.0;
        for (rust, reference) in candidate[index].iter().zip(&fixture_rows[index]) {
            let strict = (rust - reference).abs();
            max_strict = max_strict.max(strict);
            if strict > STRICT_TOLERANCE {
                failing += 1;
            }
            max_rounded = max_rounded.max((round2(*rust) - reference).abs());
        }
        let pass = max_rounded <= ROUNDED_TOLERANCE + ROUNDED_EPSILON;
        all_pass &= pass;
        println!(
            "{:<34} {:<6} {:>5.1} {:>10.6} {:>5} {:>10.6} {}",
            case,
            "TempN",
            time,
            max_strict,
            failing,
            max_rounded,
            if pass { "pass" } else { "fail" }
        );
    }
    assert!(all_pass, "{case}: rounded verdict failed; see table");
}

#[test]
fn heat_dir_top_dir_bot_default_matches_fixture() {
    check_case(CASES[0]);
}

#[test]
fn heat_dir_top_flux_bot_default_matches_fixture() {
    check_case(CASES[1]);
}

#[test]
fn heat_flux_top_dir_bot_campbell_matches_fixture() {
    check_case(CASES[2]);
}

#[test]
fn heat_dir_top_flux_bot_campbell_matches_fixture() {
    check_case(CASES[3]);
}

//! Structural checks and measurement harness for `T_LEVEL.OUT` boundary fluxes (`vTop`, `vBot`,
//! `sum_v_top`, `sum_v_bot`) on `vapor_met_bare_soil_on` (Plan 33 / G25).
//!
//! Authenticates the pinned `vapor_met_bare_soil_on_tlevel.json` sidecar against the committed
//! deck, verifies its structural alignment with the Rust met run, and reports per-step and
//! cumulative discrepancies for bounds selection (gate G-1).

#[path = "support/vapor_met.rs"]
mod vapor_met;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use serde_json::Value;
use sha2::{Digest, Sha256};
use vapor_met::{MetRun, RunOptions, StepTrace};

const CRITERIA_SHA256: &str = "8f9e78d1a82a5910c1fc9c0a98ac61e899d0a247ba992b3b1e70d513582ab71f";
const SIDECAR_SHA256: &str = "4f819abe86fe0cfd33b9b82e04f30da1bb2f78d8530560ce1ebc3079dbfff20b";
const SWITCH_TIMES: [f64; 3] = [42.7, 120.1, 200.3];

fn ref_dir() -> PathBuf {
    vapor_met::repo_root().join("reference/vapor_fortran")
}

fn criteria_path() -> PathBuf {
    ref_dir().join("criteria.json")
}

fn sidecar_path() -> PathBuf {
    ref_dir().join(format!("fixtures/{}_tlevel.json", vapor_met::CASE))
}

fn sha256_hex(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let digest = Sha256::digest(&bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// Sidecar and deck authentication: the sidecar SHA-256 is pinned here, and the deck files are
/// bound by `manifest.input_sha256` (matching `vapor_met_parity.rs`).
fn load_sidecar() -> &'static Value {
    static SIDECAR: OnceLock<Value> = OnceLock::new();
    SIDECAR.get_or_init(|| {
        assert_eq!(
            sha256_hex(&sidecar_path()),
            SIDECAR_SHA256,
            "tlevel sidecar SHA-256 differs from the pinned value"
        );
        let sidecar = read_json(&sidecar_path());
        assert_eq!(sidecar["case_id"], vapor_met::CASE);
        assert_eq!(sidecar["evidence_class"], "pinned-fortran-t-level");
        let hashes = sidecar["manifest"]["input_sha256"]
            .as_object()
            .expect("manifest.input_sha256");
        for name in ["SELECTOR.IN", "PROFILE.DAT", "ATMOSPH.IN", "Meteo.in"] {
            let expected = hashes[name].as_str().expect("input hash");
            assert_eq!(
                sha256_hex(&vapor_met::case_dir().join(name)),
                expected,
                "{name} sha256 differs from sidecar manifest"
            );
        }
        sidecar
    })
}

fn met_run() -> &'static MetRun {
    static RUN: OnceLock<MetRun> = OnceLock::new();
    RUN.get_or_init(|| vapor_met::run_met_case(RunOptions::default()).expect("met run"))
}

struct TLevelRow {
    t_level: usize,
    time: f64,
    r_top: f64,
    v_top: f64,
    v_bot: f64,
    sum_v_top: f64,
    sum_v_bot: f64,
}

fn parse_rows(sidecar: &Value) -> Vec<TLevelRow> {
    sidecar["rows"]
        .as_array()
        .expect("rows array")
        .iter()
        .map(|r| TLevelRow {
            t_level: r["t_level"].as_u64().expect("t_level") as usize,
            time: r["time"].as_f64().expect("time"),
            r_top: r["r_top"].as_f64().expect("r_top"),
            v_top: r["v_top"].as_f64().expect("v_top"),
            v_bot: r["v_bot"].as_f64().expect("v_bot"),
            sum_v_top: r["sum_v_top"].as_f64().expect("sum_v_top"),
            sum_v_bot: r["sum_v_bot"].as_f64().expect("sum_v_bot"),
        })
        .collect()
}

/// Half a unit in the 5th significant digit for Fortran `e13.5` output.
fn e13_5_print_floor(val: f64) -> f64 {
    let a = val.abs();
    if a == 0.0 {
        0.0
    } else {
        let exp = a.log10().floor();
        0.5 * 10_f64.powf(exp - 4.0)
    }
}

fn rel_err(diff: f64, reference: f64) -> f64 {
    let denom = reference.abs();
    if denom == 0.0 {
        if diff == 0.0 {
            0.0
        } else {
            f64::INFINITY
        }
    } else {
        diff.abs() / denom
    }
}

#[derive(Default)]
struct ModeStats {
    count: usize,
    max_abs: f64,
    max_abs_t: f64,
    max_abs_rust: f64,
    max_abs_fort: f64,
    max_rel: f64,
    max_rel_t: f64,
    max_rel_rust: f64,
    max_rel_fort: f64,
    max_floor: f64,
    max_over_floor: f64,
    max_over_floor_t: f64,
    sign_same: usize,
    sign_opp: usize,
}

impl ModeStats {
    fn update(&mut self, t: f64, rust_val: f64, fort_val: f64) {
        self.count += 1;
        let diff = (rust_val - fort_val).abs();
        let rel = rel_err(diff, fort_val);
        let floor = e13_5_print_floor(fort_val);
        if diff > self.max_abs {
            self.max_abs = diff;
            self.max_abs_t = t;
            self.max_abs_rust = rust_val;
            self.max_abs_fort = fort_val;
        }
        if rel.is_finite() && rel > self.max_rel {
            self.max_rel = rel;
            self.max_rel_t = t;
            self.max_rel_rust = rust_val;
            self.max_rel_fort = fort_val;
        }
        if floor > self.max_floor {
            self.max_floor = floor;
        }
        if floor > 0.0 {
            let ratio = diff / floor;
            if ratio > self.max_over_floor {
                self.max_over_floor = ratio;
                self.max_over_floor_t = t;
            }
        }
        if rust_val * fort_val > 0.0 || (rust_val == 0.0 && fort_val == 0.0) {
            self.sign_same += 1;
        } else if rust_val * fort_val < 0.0 {
            self.sign_opp += 1;
        }
    }
}

#[test]
fn sidecar_structure_and_time_alignment() {
    let sidecar = load_sidecar();
    let run = met_run();
    let rows = parse_rows(sidecar);
    let time_points: Vec<f64> = sidecar["time_points"]
        .as_array()
        .expect("time_points")
        .iter()
        .map(|v| v.as_f64().expect("time_point f64"))
        .collect();

    assert_eq!(rows.len(), 2400, "expected 2400 T_LEVEL rows");
    assert_eq!(time_points.len(), 2400, "expected 2400 time_points");
    assert_eq!(run.steps.len(), 2400, "expected 2400 Rust steps");

    for (i, ((row, &tp), step)) in rows
        .iter()
        .zip(time_points.iter())
        .zip(run.steps.iter())
        .enumerate()
    {
        let expected_tlevel = i + 1;
        assert_eq!(
            row.t_level, expected_tlevel,
            "t_level mismatch at index {i}"
        );
        assert!(
            (tp - step.time).abs() <= 1e-9,
            "time_points[{i}]={tp} != step.time={}",
            step.time
        );
        assert!(
            (row.time - step.time).abs() <= 1e-9,
            "row.time[{i}]={} != step.time={}",
            row.time,
            step.time
        );
    }
}

#[test]
fn measure_tlevel_flux_discrepancies() {
    let sidecar = load_sidecar();
    let run = met_run();
    let rows = parse_rows(sidecar);

    let mut vtop_flux = ModeStats::default();
    let mut vtop_head = ModeStats::default();
    let mut vbot_flux = ModeStats::default();
    let mut vbot_head = ModeStats::default();
    let mut vtop_switch_flux = ModeStats::default();
    let mut vtop_switch_head = ModeStats::default();

    let mut vtop_entries: Vec<(f64, usize, f64, i8, f64, f64, f64)> = Vec::with_capacity(2400);
    let mut vbot_entries: Vec<(f64, usize, f64, i8, f64, f64, f64)> = Vec::with_capacity(2400);

    let mut rust_sum_vtop = 0.0_f64;
    let mut rust_sum_vbot = 0.0_f64;
    let mut max_cum_vtop_abs = 0.0_f64;
    let mut max_cum_vtop_rel = 0.0_f64;
    let mut max_cum_vtop_t = 0.0_f64;
    let mut max_cum_vbot_abs = 0.0_f64;
    let mut max_cum_vbot_rel = 0.0_f64;
    let mut max_cum_vbot_t = 0.0_f64;

    let mut max_fort_vtop = 0.0_f64;
    let mut max_fort_vbot = 0.0_f64;

    for (row, step) in rows.iter().zip(run.steps.iter()) {
        let is_head = step.kod_top > 0;
        let vt_r = step.v_top_reported;
        let vb_r = step.v_bot_reported;
        let vt_f = row.v_top;
        let vb_f = row.v_bot;

        max_fort_vtop = max_fort_vtop.max(vt_f.abs());
        max_fort_vbot = max_fort_vbot.max(vb_f.abs());

        if is_head {
            vtop_head.update(step.time, vt_r, vt_f);
            vbot_head.update(step.time, vb_r, vb_f);
            vtop_switch_head.update(step.time, step.v_top, vt_f);
        } else {
            vtop_flux.update(step.time, vt_r, vt_f);
            vbot_flux.update(step.time, vb_r, vb_f);
            vtop_switch_flux.update(step.time, step.v_top, vt_f);
        }

        let dvt = (vt_r - vt_f).abs();
        let dvb = (vb_r - vb_f).abs();
        vtop_entries.push((
            dvt,
            row.t_level,
            step.time,
            step.kod_top,
            vt_r,
            vt_f,
            rel_err(dvt, vt_f),
        ));
        vbot_entries.push((
            dvb,
            row.t_level,
            step.time,
            step.kod_top,
            vb_r,
            vb_f,
            rel_err(dvb, vb_f),
        ));

        rust_sum_vtop += vt_r * step.timestep;
        rust_sum_vbot += vb_r * step.timestep;

        let cum_vt_diff = (rust_sum_vtop - row.sum_v_top).abs();
        let cum_vt_rel = rel_err(cum_vt_diff, row.sum_v_top);
        if cum_vt_diff > max_cum_vtop_abs {
            max_cum_vtop_abs = cum_vt_diff;
            max_cum_vtop_t = step.time;
        }
        if cum_vt_rel.is_finite() && cum_vt_rel > max_cum_vtop_rel {
            max_cum_vtop_rel = cum_vt_rel;
        }

        let cum_vb_diff = (rust_sum_vbot - row.sum_v_bot).abs();
        let cum_vb_rel = rel_err(cum_vb_diff, row.sum_v_bot);
        if cum_vb_diff > max_cum_vbot_abs {
            max_cum_vbot_abs = cum_vb_diff;
            max_cum_vbot_t = step.time;
        }
        if cum_vb_rel.is_finite() && cum_vb_rel > max_cum_vbot_rel {
            max_cum_vbot_rel = cum_vb_rel;
        }
    }

    let print_stats = |label: &str, s: &ModeStats| {
        println!(
            "{label:22} n={:4} | max_abs={:.6e} (t={:5.1}, R={:+.6e}, F={:+.6e}) | max_rel={:.6e} (t={:5.1}) | max_floor={:.3e} | max(|d|/floor)={:.2} (t={:5.1}) | sign same/opp={}/{}",
            s.count,
            s.max_abs,
            s.max_abs_t,
            s.max_abs_rust,
            s.max_abs_fort,
            s.max_rel,
            s.max_rel_t,
            s.max_floor,
            s.max_over_floor,
            s.max_over_floor_t,
            s.sign_same,
            s.sign_opp,
        );
    };

    println!("=== Per-step summary (cm/h) ===");
    print_stats("vTop (flux kod<0)", &vtop_flux);
    print_stats("vTop (head kod>0)", &vtop_head);
    print_stats("vBot (flux kod<0)", &vbot_flux);
    print_stats("vBot (head kod>0)", &vbot_head);
    println!("=== Reference: switching-form StepTrace::v_top vs Fortran vTop (cm/h) ===");
    print_stats("vTop_sw (flux kod<0)", &vtop_switch_flux);
    print_stats("vTop_sw (head kod>0)", &vtop_switch_head);
    println!(
        "Peak |Fortran|: vTop={:.6e} (e13.5 floor={:.3e}), vBot={:.6e} (e13.5 floor={:.3e})",
        max_fort_vtop,
        e13_5_print_floor(max_fort_vtop),
        max_fort_vbot,
        e13_5_print_floor(max_fort_vbot),
    );

    vtop_entries.sort_by(|a, b| b.0.total_cmp(&a.0));
    vbot_entries.sort_by(|a, b| b.0.total_cmp(&a.0));

    println!("\n=== Worst 10 steps: |ΔvTop| ===");
    println!("rank  t_level    time  kod        rust_vTop     fortran_vTop       |ΔvTop|         rel    e13.5_fl");
    for (rank, e) in vtop_entries.iter().take(10).enumerate() {
        println!(
            "{:4}  {:7}  {:6.1}  {:3}  {:+.6e}  {:+.6e}  {:.6e}  {:.4e}  {:.3e}",
            rank + 1,
            e.1,
            e.2,
            e.3,
            e.4,
            e.5,
            e.0,
            e.6,
            e13_5_print_floor(e.5),
        );
    }

    println!("\n=== Worst 10 steps: |ΔvBot| ===");
    println!("rank  t_level    time  kod        rust_vBot     fortran_vBot       |ΔvBot|         rel    e13.5_fl");
    for (rank, e) in vbot_entries.iter().take(10).enumerate() {
        println!(
            "{:4}  {:7}  {:6.1}  {:3}  {:+.6e}  {:+.6e}  {:.6e}  {:.4e}  {:.3e}",
            rank + 1,
            e.1,
            e.2,
            e.3,
            e.4,
            e.5,
            e.0,
            e.6,
            e13_5_print_floor(e.5),
        );
    }

    println!("\n=== Switch steps (42.7, 120.1, 200.3 h) ===");
    println!("  time  kod        rust_vTop     fortran_vTop       |ΔvTop|        rust_vBot     fortran_vBot       |ΔvBot|    rust_vTop_sw");
    for &t_sw in &SWITCH_TIMES {
        let idx = run
            .steps
            .iter()
            .position(|s| (s.time - t_sw).abs() < 1e-6)
            .expect("switch step");
        let s: &StepTrace = &run.steps[idx];
        let r = &rows[idx];
        println!(
            "{:6.1}  {:3}  {:+.6e}  {:+.6e}  {:.6e}  {:+.6e}  {:+.6e}  {:.6e}  {:+.6e}",
            s.time,
            s.kod_top,
            s.v_top_reported,
            r.v_top,
            (s.v_top_reported - r.v_top).abs(),
            s.v_bot_reported,
            r.v_bot,
            (s.v_bot_reported - r.v_bot).abs(),
            s.v_top,
        );
    }

    let rain_indices: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.r_top < 0.0)
        .map(|(i, _)| i)
        .collect();
    println!(
        "\n=== Rain steps (Fortran r_top < 0, count={}) ===",
        rain_indices.len()
    );
    if !rain_indices.is_empty() {
        let first = rain_indices[0];
        let last = *rain_indices.last().unwrap();
        let worst_rain = *rain_indices
            .iter()
            .max_by(|&&a, &&b| {
                (run.steps[a].v_top_reported - rows[a].v_top)
                    .abs()
                    .total_cmp(&(run.steps[b].v_top_reported - rows[b].v_top).abs())
            })
            .unwrap();
        let mut sample = vec![first, worst_rain, last];
        sample.dedup();
        println!("  time  kod         r_top        rust_vTop     fortran_vTop       |ΔvTop|        rust_vBot     fortran_vBot       |ΔvBot|");
        for idx in sample {
            let s = &run.steps[idx];
            let r = &rows[idx];
            println!(
                "{:6.1}  {:3}  {:+.6e}  {:+.6e}  {:+.6e}  {:.6e}  {:+.6e}  {:+.6e}  {:.6e}",
                s.time,
                s.kod_top,
                r.r_top,
                s.v_top_reported,
                r.v_top,
                (s.v_top_reported - r.v_top).abs(),
                s.v_bot_reported,
                r.v_bot,
                (s.v_bot_reported - r.v_bot).abs(),
            );
        }
    }

    let last_row = rows.last().unwrap();
    let final_vt_diff = (rust_sum_vtop - last_row.sum_v_top).abs();
    let final_vb_diff = (rust_sum_vbot - last_row.sum_v_bot).abs();
    println!("\n=== Cumulative fluxes (cm) ===");
    println!(
        "sum_v_top: final Rust={:+.6e}, Fortran={:+.6e}, |Δ|={:.6e}, rel={:.6e}, e13.5_fl={:.3e} | max_over_run |Δ|={:.6e} (t={:5.1}), max_rel={:.6e}",
        rust_sum_vtop,
        last_row.sum_v_top,
        final_vt_diff,
        rel_err(final_vt_diff, last_row.sum_v_top),
        e13_5_print_floor(last_row.sum_v_top),
        max_cum_vtop_abs,
        max_cum_vtop_t,
        max_cum_vtop_rel,
    );
    println!(
        "sum_v_bot: final Rust={:+.6e}, Fortran={:+.6e}, |Δ|={:.6e}, rel={:.6e}, e13.5_fl={:.3e} | max_over_run |Δ|={:.6e} (t={:5.1}), max_rel={:.6e}",
        rust_sum_vbot,
        last_row.sum_v_bot,
        final_vb_diff,
        rel_err(final_vb_diff, last_row.sum_v_bot),
        e13_5_print_floor(last_row.sum_v_bot),
        max_cum_vbot_abs,
        max_cum_vbot_t,
        max_cum_vbot_rel,
    );
}

fn load_criteria() -> &'static Value {
    static CRITERIA: OnceLock<Value> = OnceLock::new();
    CRITERIA.get_or_init(|| {
        let path = criteria_path();
        assert_eq!(
            sha256_hex(&path),
            CRITERIA_SHA256,
            "criteria.json SHA-256 differs from the pinned value"
        );
        read_json(&path)
    })
}

struct CheckBound {
    absolute: f64,
    relative: f64,
}

impl CheckBound {
    fn limit(&self, reference: f64) -> f64 {
        self.absolute + self.relative * reference.abs()
    }
}

fn load_check_bound(name: &str, expected_unit: &str) -> CheckBound {
    let criteria = load_criteria();
    let entry = &criteria["checks"][name];
    assert!(
        entry.is_object(),
        "criteria.json missing checks.{name} entry"
    );
    assert_eq!(
        entry["scale"].as_str(),
        Some("ref_abs"),
        "criteria.json checks.{name}.scale must be ref_abs"
    );
    assert_eq!(
        entry["unit"].as_str(),
        Some(expected_unit),
        "criteria.json checks.{name}.unit must be {expected_unit}"
    );
    CheckBound {
        absolute: entry["absolute"].as_f64().expect("check absolute bound"),
        relative: entry["relative"].as_f64().expect("check relative bound"),
    }
}

#[test]
fn t_level_per_step_flux_parity() {
    let bound = load_check_bound("t_level_boundary_flux", "cm/h");
    let sidecar = load_sidecar();
    let run = met_run();
    let rows = parse_rows(sidecar);
    assert_eq!(rows.len(), run.steps.len());

    for (row, step) in rows.iter().zip(run.steps.iter()) {
        let diff_top = (step.v_top_reported - row.v_top).abs();
        let lim_top = bound.limit(row.v_top);
        assert!(
            diff_top <= lim_top,
            "vTop mismatch at t_level={} (t={} h, kod_top={}): rust={:+.8e}, fortran={:+.8e}, |diff|={:.6e} > limit={:.6e}",
            row.t_level,
            step.time,
            step.kod_top,
            step.v_top_reported,
            row.v_top,
            diff_top,
            lim_top,
        );

        let diff_bot = (step.v_bot_reported - row.v_bot).abs();
        let lim_bot = bound.limit(row.v_bot);
        assert!(
            diff_bot <= lim_bot,
            "vBot mismatch at t_level={} (t={} h, kod_top={}): rust={:+.8e}, fortran={:+.8e}, |diff|={:.6e} > limit={:.6e}",
            row.t_level,
            step.time,
            step.kod_top,
            step.v_bot_reported,
            row.v_bot,
            diff_bot,
            lim_bot,
        );
    }
}

#[test]
fn t_level_cumulative_flux_parity() {
    let bound = load_check_bound("t_level_cumulative_flux", "cm");
    let sidecar = load_sidecar();
    let run = met_run();
    let rows = parse_rows(sidecar);
    assert_eq!(rows.len(), run.steps.len());

    let mut rust_sum_vtop = 0.0_f64;
    let mut rust_sum_vbot = 0.0_f64;

    for (row, step) in rows.iter().zip(run.steps.iter()) {
        rust_sum_vtop += step.v_top_reported * step.timestep;
        rust_sum_vbot += step.v_bot_reported * step.timestep;

        let diff_top = (rust_sum_vtop - row.sum_v_top).abs();
        let lim_top = bound.limit(row.sum_v_top);
        assert!(
            diff_top <= lim_top,
            "sum_v_top mismatch at t_level={} (t={} h): rust={:+.8e}, fortran={:+.8e}, |diff|={:.6e} > limit={:.6e}",
            row.t_level,
            step.time,
            rust_sum_vtop,
            row.sum_v_top,
            diff_top,
            lim_top,
        );

        let diff_bot = (rust_sum_vbot - row.sum_v_bot).abs();
        let lim_bot = bound.limit(row.sum_v_bot);
        assert!(
            diff_bot <= lim_bot,
            "sum_v_bot mismatch at t_level={} (t={} h): rust={:+.8e}, fortran={:+.8e}, |diff|={:.6e} > limit={:.6e}",
            row.t_level,
            step.time,
            rust_sum_vbot,
            row.sum_v_bot,
            diff_bot,
            lim_bot,
        );
    }
}

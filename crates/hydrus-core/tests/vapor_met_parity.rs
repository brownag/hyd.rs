//! D-10 gate: Rust met-driven vapor run versus the pinned Fortran trajectory of
//! `vapor_met_bare_soil_on` (task 31.4.1).
//!
//! The Rust path is `Meteo.in` records -> bare-soil ET -> forcing schedule ->
//! `advance_atmospheric_step_vapor` (the decomposition into the existing Flux/Head pairs),
//! driven by `tests/support/vapor_met.rs`. Bounds are the approved per-FIELD rules of
//! `reference/vapor_fortran/criteria.json` (SHA-256 pinned below), applied unchanged, exactly as
//! `vapor_fortran_parity.rs` applies them to `vapor_flux_fixedhead_wet_on`; the gated field set is
//! the one that case gates (head, liquid_moisture, temperature_c and the nine vapor fields).
//! A failing row is a result, not something to tune; a parity failure re-opens D-10.
//!
//! Schedule disclosure: constant dt 0.1 h (the deck dt, as in the source run), two source-ordered
//! passes per step; Rust solves each water pass to the deck tolerances, the source stops at
//! at most two Picard iterations. Source single precision is not modelled.

#[path = "support/vapor_met.rs"]
mod vapor_met;

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use serde_json::Value;
use sha2::{Digest, Sha256};
use vapor_met::{KodChange, MetRun, RunOptions};

const CRITERIA_SHA256: &str = "ba0e5d5a6a0cef5e9159c07d7c72a8e245f4de62dd43f4950bcef7115f08d3a7";
const FIXTURE_SHA256: &str = "75b4207387dff4838fba38c88eaed2527f7e450683e0f12ed418061abdd9a877";
/// Fortran `KodT` switches (RUN_INF.OUT of the pinned run, task 31.2.1 gate): the first
/// step whose printed `KodT` has the new value, as time at the end of that step.
const FORTRAN_SWITCHES: [(f64, i8, i8); 3] = [(42.7, -4, 4), (120.1, 4, -4), (200.3, -4, 4)];
const BASE_FIELDS: [&str; 3] = ["head", "liquid_moisture", "temperature_c"];
const VAPOR_FIELDS: [&str; 9] = [
    "con",
    "con_lt",
    "con_vh",
    "con_vt",
    "v_liquid",
    "v_vapor",
    "v_total",
    "v_vap_iso",
    "v_vap_term",
];
const CRITERIA_FIELDS: [&str; 15] = [
    "con",
    "con_lt",
    "con_vh",
    "con_vt",
    "head",
    "kernel_adjusted_hydraulic_capacity",
    "kernel_capacity_correction",
    "kernel_vapor_content",
    "liquid_moisture",
    "temperature_c",
    "v_liquid",
    "v_total",
    "v_vap_iso",
    "v_vap_term",
    "v_vapor",
];

fn repo_root() -> PathBuf {
    vapor_met::repo_root()
}

fn ref_dir() -> PathBuf {
    repo_root().join("reference/vapor_fortran")
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_hex(path: &Path) -> String {
    sha256_bytes(&fs::read(path).unwrap_or_else(|error| panic!("read {path:?}: {error}")))
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("parse {path:?}: {error}"))
}

#[derive(Clone, Copy, Debug)]
struct Crit {
    absolute: f64,
    relative: f64,
    fraction: Option<f64>,
    /// `ulp_multiple` of the float32 temperature-precision term (criteria v2), when the rule has it.
    ulp_multiple: Option<f64>,
}

/// Per-time reference context for the temperature-precision term.
struct Prec<'a> {
    temperature: &'a [f64],
    con_vt: &'a [f64],
    dx_min: &'a [f64],
}

/// `ulp32(x)`: the f32 spacing above `|x|`, as f64.
fn ulp32(x: f64) -> f64 {
    let a = x.abs() as f32;
    f64::from(a.next_up()) - f64::from(a)
}

fn finite_nonneg(value: &Value, what: &str) -> Result<f64, String> {
    let number = value
        .as_f64()
        .ok_or_else(|| format!("{what}: not a number"))?;
    if !number.is_finite() || number < 0.0 {
        return Err(format!("{what}: must be finite and nonnegative"));
    }
    Ok(number)
}

/// Validates the approved table and returns the per-field rule. Unknown or missing fields,
/// classes, scales or rule names are rejected.
fn parse_criteria(value: &Value) -> Result<BTreeMap<String, Crit>, String> {
    if value["schema_version"] != "vapor-criteria-v2" {
        return Err("criteria schema_version (only vapor-criteria-v2 is accepted)".into());
    }
    let fields = value["fields"].as_object().ok_or("criteria fields")?;
    for name in fields.keys() {
        if !CRITERIA_FIELDS.contains(&name.as_str()) {
            return Err(format!("unknown criteria field {name}"));
        }
    }
    let mut out = BTreeMap::new();
    for name in CRITERIA_FIELDS {
        let entry = fields
            .get(name)
            .ok_or_else(|| format!("missing criteria field {name}"))?;
        let class = entry["evidence_class"].as_str().unwrap_or("");
        let expected = if name.starts_with("kernel_") {
            "pinned-fortran-kernel"
        } else {
            "pinned-fortran-trajectory"
        };
        if class != expected {
            return Err(format!("{name}: evidence class {class:?} != {expected}"));
        }
        let absolute = finite_nonneg(&entry["absolute"], name)?;
        let relative = finite_nonneg(&entry["relative"], name)?;
        match entry["scale"].as_str() {
            Some("abs_reference") => {}
            Some("none") if relative == 0.0 => {}
            other => return Err(format!("{name}: unknown scale {other:?}")),
        }
        let unit = entry["unit"].as_str().unwrap_or("");
        let expected_unit = match name {
            "head" => "cm",
            "temperature_c" => "degC",
            "liquid_moisture" | "kernel_vapor_content" => "cm3/cm3",
            "kernel_adjusted_hydraulic_capacity" | "kernel_capacity_correction" => "1/cm",
            "con" | "con_vh" | "v_liquid" | "v_vapor" | "v_total" | "v_vap_iso" | "v_vap_term" => {
                "cm/h"
            }
            _ => "cm2/K/h", // con_lt, con_vt
        };
        if unit != expected_unit {
            return Err(format!("{name}: unit {unit:?} != {expected_unit:?}"));
        }
        let rule = entry["zero_rule"].as_object().ok_or("zero_rule")?;
        let keys_are = |expected: &[&str]| {
            let mut have: Vec<&str> = rule.keys().map(String::as_str).collect();
            have.sort_unstable();
            let mut want = expected.to_vec();
            want.sort_unstable();
            have == want
        };
        let (fraction, ulp_multiple) = match entry["zero_rule"]["name"].as_str() {
            Some("none") if keys_are(&["name"]) => (None, None),
            Some("case_time_field_max_fraction") if keys_are(&["name", "fraction"]) => (
                Some(finite_nonneg(&entry["zero_rule"]["fraction"], name)?),
                None,
            ),
            Some("case_time_field_max_fraction_with_temperature_precision")
                if keys_are(&[
                    "name",
                    "fraction",
                    "ulp_multiple",
                    "temperature_precision",
                    "conductivity_field",
                    "conductivity_neighbourhood",
                    "spacing",
                ]) && rule["temperature_precision"] == "float32"
                    && rule["conductivity_field"] == "con_vt"
                    && rule["conductivity_neighbourhood"]
                        == "node_and_mesh_neighbours_same_time"
                    && rule["spacing"] == "min_adjacent_dx" =>
            {
                (
                    Some(finite_nonneg(&entry["zero_rule"]["fraction"], name)?),
                    Some(finite_nonneg(&entry["zero_rule"]["ulp_multiple"], name)?),
                )
            }
            other => return Err(format!("{name}: unknown or malformed zero_rule {other:?}")),
        };
        out.insert(
            name.to_owned(),
            Crit {
                absolute,
                relative,
                fraction,
                ulp_multiple,
            },
        );
    }
    Ok(out)
}

fn load_criteria() -> (BTreeMap<String, Crit>, Value) {
    let path = ref_dir().join("criteria.json");
    assert_eq!(
        sha256_hex(&path),
        CRITERIA_SHA256,
        "criteria.json SHA-256 differs from the approved value"
    );
    let value = read_json(&path);
    let table = parse_criteria(&value).expect("criteria validation");
    (table, value)
}

// ----------------------------------------------------------------------------------------------
// Comparison
// ----------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct Row {
    case: String,
    field: String,
    time: f64,
    node: usize,
    candidate: f64,
    reference: f64,
    error: f64,
    bound: f64,
    pass: bool,
}

/// `|c-r| <= absolute + relative*|r|`, raised to `fraction*max|r|` over this case/time/field for
/// `case_time_field_max_fraction`. A non-finite candidate is a failing row; a non-finite
/// reference or a shape mismatch is an error.
#[allow(clippy::too_many_arguments)]
fn compare_series(
    case: &str,
    field: &str,
    time: f64,
    nodes: &[usize],
    candidate: &[f64],
    reference: &[f64],
    crit: Crit,
    prec: Option<&Prec>,
) -> Result<Vec<Row>, String> {
    if candidate.len() != reference.len() || nodes.len() != reference.len() {
        return Err(format!(
            "{case}/{field}@{time}: shape {} (candidate) vs {} (reference) vs {} (nodes)",
            candidate.len(),
            reference.len(),
            nodes.len()
        ));
    }
    if reference.iter().any(|r| !r.is_finite()) {
        return Err(format!("{case}/{field}@{time}: non-finite reference"));
    }
    let floor = crit.fraction.map_or(0.0, |fraction| {
        fraction * reference.iter().fold(0.0_f64, |m, r| m.max(r.abs()))
    });
    if let Some(multiple) = crit.ulp_multiple {
        let p = prec.ok_or_else(|| format!("{case}/{field}@{time}: precision context missing"))?;
        let n = reference.len();
        if [p.temperature.len(), p.con_vt.len(), p.dx_min.len()] != [n; 3] {
            return Err(format!("{case}/{field}@{time}: precision context shape"));
        }
        let _ = multiple;
    }
    Ok(nodes
        .iter()
        .enumerate()
        .zip(candidate.iter().zip(reference))
        .map(|((i, &node), (&c, &r))| {
            let mut bound = (crit.absolute + crit.relative * r.abs()).max(floor);
            if let (Some(multiple), Some(p)) = (crit.ulp_multiple, prec) {
                let lo = i.saturating_sub(1);
                let hi = (i + 1).min(p.con_vt.len() - 1);
                let con_vt_max = p.con_vt[lo..=hi]
                    .iter()
                    .fold(0.0_f64, |m, v| m.max(v.abs()));
                bound = bound.max(multiple * ulp32(p.temperature[i]) * con_vt_max / p.dx_min[i]);
            }
            let error = (c - r).abs();
            Row {
                case: case.to_owned(),
                field: field.to_owned(),
                time,
                node,
                candidate: c,
                reference: r,
                error,
                bound,
                pass: c.is_finite() && error <= bound,
            }
        })
        .collect())
}

fn vec_f64(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|v| v.as_f64().expect("number"))
        .collect()
}

fn matrix(value: &Value) -> Vec<Vec<f64>> {
    value
        .as_array()
        .expect("matrix")
        .iter()
        .map(vec_f64)
        .collect()
}

/// Minimum adjacent node spacing per node (fixture node depths, any order).
fn min_adjacent_dx(depths: &[f64]) -> Vec<f64> {
    let n = depths.len();
    (0..n)
        .map(|i| {
            let mut best = f64::INFINITY;
            if i > 0 {
                best = best.min((depths[i] - depths[i - 1]).abs());
            }
            if i + 1 < n {
                best = best.min((depths[i + 1] - depths[i]).abs());
            }
            best
        })
        .collect()
}

// ----------------------------------------------------------------------------------------------
// Inputs, comparison and outcome
// ----------------------------------------------------------------------------------------------

fn fixture_path() -> PathBuf {
    ref_dir().join(format!("fixtures/{}.json", vapor_met::CASE))
}

/// Fixture and deck authentication: the fixture is pinned here; the deck files are bound by
/// the fixture manifest.
fn load_fixture() -> Value {
    assert_eq!(
        sha256_hex(&fixture_path()),
        FIXTURE_SHA256,
        "fixture SHA-256 differs from the pinned value"
    );
    let fixture = read_json(&fixture_path());
    assert_eq!(fixture["case_id"], vapor_met::CASE);
    assert_eq!(fixture["vapor_output_available"], true);
    let hashes = fixture["manifest"]["input_sha256"]
        .as_object()
        .expect("manifest.input_sha256");
    for name in ["SELECTOR.IN", "PROFILE.DAT", "ATMOSPH.IN", "Meteo.in"] {
        let expected = hashes[name].as_str().expect("input hash");
        assert_eq!(
            sha256_hex(&vapor_met::case_dir().join(name)),
            expected,
            "{name} sha256 differs from fixture manifest"
        );
    }
    fixture
}

fn compare_met(
    run: &MetRun,
    fixture: &Value,
    crits: &BTreeMap<String, Crit>,
) -> Result<Vec<Row>, String> {
    let case = vapor_met::CASE;
    if vec_f64(&fixture["time_points"]) != run.time_points {
        return Err(format!("{case}: time points differ"));
    }
    if vec_f64(&fixture["node_depths"]) != run.node_depths {
        return Err(format!("{case}: node depths differ"));
    }
    let nodes: Vec<usize> = (1..=run.node_depths.len()).collect();
    type FieldRows<'a> = (&'a str, &'a Vec<Vec<f64>>, Vec<Vec<f64>>);
    let mut fields: Vec<FieldRows> = vec![
        ("head", &run.head, matrix(&fixture["head"])),
        (
            "liquid_moisture",
            &run.liquid_moisture,
            matrix(&fixture["liquid_moisture"]),
        ),
        (
            "temperature_c",
            &run.temperature_c,
            matrix(&fixture["temperature_c"]),
        ),
    ];
    for name in VAPOR_FIELDS {
        fields.push((
            name,
            &run.vapor[name],
            matrix(&fixture["vapor_fields"][name]),
        ));
    }
    if fields.iter().any(|(_, cand, r)| cand.len() != r.len()) {
        return Err(format!("{case}: time-row count mismatch"));
    }
    let temperature = matrix(&fixture["temperature_c"]);
    let con_vt = matrix(&fixture["vapor_fields"]["con_vt"]);
    let dx_min = min_adjacent_dx(&run.node_depths);
    let mut rows = Vec::new();
    for (name, cand, reference) in fields {
        let crit = *crits
            .get(name)
            .ok_or_else(|| format!("no criteria for {name}"))?;
        for (index, time) in run.time_points.iter().enumerate() {
            let prec = crit.ulp_multiple.map(|_| Prec {
                temperature: &temperature[index],
                con_vt: &con_vt[index],
                dx_min: &dx_min,
            });
            rows.extend(compare_series(
                case,
                name,
                *time,
                &nodes,
                &cand[index],
                &reference[index],
                crit,
                prec.as_ref(),
            )?);
        }
    }
    Ok(rows)
}

/// Mismatches between the Rust `KodTop` changes and the Fortran ones (empty when they agree to
/// the step: same sign change on the same 0.1 h step).
fn switch_mismatches(changes: &[KodChange], expected: &[(f64, i8, i8)]) -> Vec<String> {
    let mut out = Vec::new();
    if changes.len() != expected.len() {
        out.push(format!(
            "{} Rust switches, {} Fortran",
            changes.len(),
            expected.len()
        ));
    }
    for (c, e) in changes.iter().zip(expected) {
        if (c.time - e.0).abs() > 1e-9 || (c.from, c.to) != (e.1, e.2) {
            out.push(format!(
                "Rust {}->{} at {:.1} h, Fortran {}->{} at {:.1} h",
                c.from, c.to, c.time, e.1, e.2, e.0
            ));
        }
    }
    out
}

struct Outcome {
    run: MetRun,
    rows: Vec<Row>,
    crits: BTreeMap<String, Crit>,
    criteria: Value,
    fixture: Value,
}

fn per_field_time_max(rows: &[Row]) -> String {
    let mut report = String::from("field\ttime_h\tmax_abs_diff\tnode\tbound_at_max\tpass\n");
    let mut keys: Vec<(String, u64)> = rows
        .iter()
        .map(|r| (r.field.clone(), r.time.to_bits()))
        .collect();
    keys.sort_by(|a, b| {
        let order = |f: &str| {
            BASE_FIELDS
                .iter()
                .chain(VAPOR_FIELDS.iter())
                .position(|x| *x == f)
        };
        order(&a.0)
            .cmp(&order(&b.0))
            .then(f64::from_bits(a.1).total_cmp(&f64::from_bits(b.1)))
    });
    keys.dedup();
    for (field, bits) in keys {
        let time = f64::from_bits(bits);
        let group: Vec<&Row> = rows
            .iter()
            .filter(|r| r.field == field && r.time == time)
            .collect();
        let worst = group
            .iter()
            .max_by(|a, b| a.error.total_cmp(&b.error))
            .unwrap();
        let _ = writeln!(
            report,
            "{field}\t{time:.1}\t{:.6e}\t{}\t{:.6e}\t{}",
            worst.error,
            worst.node,
            worst.bound,
            group.iter().all(|r| r.pass)
        );
    }
    report
}

fn outcome() -> &'static Result<Outcome, String> {
    static OUTCOME: OnceLock<Result<Outcome, String>> = OnceLock::new();
    OUTCOME.get_or_init(|| {
        let (crits, criteria) = load_criteria();
        let fixture = load_fixture();
        let run = vapor_met::run_met_case(RunOptions::default())?;
        let rows = compare_met(&run, &fixture, &crits)?;
        let dir = repo_root().join(".development/31.4.1");
        let _ = fs::create_dir_all(&dir);
        let table = per_field_time_max(&rows);
        let _ = fs::write(dir.join("met_parity_max_diff.tsv"), &table);
        println!("{table}");
        println!("Rust KodTop changes: {:?}", run.kod_changes());
        Ok(Outcome {
            run,
            rows,
            crits,
            criteria,
            fixture,
        })
    })
}

fn out() -> &'static Outcome {
    match outcome() {
        Ok(o) => o,
        Err(error) => panic!("met candidate run failed: {error}"),
    }
}

fn assert_rows_pass(label: &str, rows: &[Row]) {
    let failing: Vec<&Row> = rows.iter().filter(|r| !r.pass).collect();
    assert!(
        failing.is_empty(),
        "{label}: {} of {} comparisons exceed the approved bounds; first: {:?}",
        failing.len(),
        rows.len(),
        failing.first()
    );
}

// ----------------------------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------------------------

#[test]
fn met_run_matches_fixture() {
    let o = out();
    let fields: Vec<&str> = o.rows.iter().map(|r| r.field.as_str()).collect();
    for field in BASE_FIELDS.iter().chain(VAPOR_FIELDS.iter()) {
        assert!(fields.contains(field), "no comparison for {field}");
    }
    assert_eq!(
        o.rows.len(),
        12 * 11 * 41,
        "12 fields x 11 times x 41 nodes"
    );
    assert_rows_pass(vapor_met::CASE, &o.rows);
}

#[test]
fn met_run_switches_on_the_fortran_steps() {
    let o = out();
    let mismatches = switch_mismatches(&o.run.kod_changes(), &FORTRAN_SWITCHES);
    assert!(mismatches.is_empty(), "{mismatches:?}");
    // Head mode holds hTop = hCritA = -1000 cm while active.
    let head_steps: Vec<_> = o.run.steps.iter().filter(|s| s.kod_top > 0).collect();
    assert_eq!(
        head_steps.len(),
        774 + 398,
        "head-mode steps (42.7-120.0, 200.3-240)"
    );
    assert!(head_steps
        .iter()
        .all(|s| s.h_top == -1000.0 && (s.top_head + 1000.0).abs() < 1e-6));
    // 2400 steps of 0.1 h, none shortened.
    assert_eq!(o.run.steps.len(), 2400);
    assert!(o.run.steps.iter().all(|s| (s.timestep - 0.1).abs() < 1e-9));
}

/// Negative checks: the comparator and the switch assertion reject tampered inputs.
#[test]
fn comparator_rejects_tampered_inputs() {
    let o = out();
    // A perturbed fixture head is flagged.
    let mut bad = o.fixture.clone();
    bad["head"][2][5] = Value::from(bad["head"][2][5].as_f64().unwrap() + 50.0);
    let rows = compare_met(&o.run, &bad, &o.crits).unwrap();
    assert!(rows.iter().any(|r| !r.pass && r.field == "head"));
    // A shape change is an error.
    let mut short = o.fixture.clone();
    short["head"].as_array_mut().unwrap().pop();
    assert!(compare_met(&o.run, &short, &o.crits).is_err());
    // Wrong or missing switches are reported.
    let shifted = [(42.8, -4, 4), (120.1, 4, -4), (200.3, -4, 4)];
    assert_eq!(switch_mismatches(&o.run.kod_changes(), &shifted).len(), 1);
    assert!(!switch_mismatches(&o.run.kod_changes(), &FORTRAN_SWITCHES[..2]).is_empty());
    // Criteria schema errors are rejected.
    let mut crit = o.criteria.clone();
    crit["schema_version"] = Value::from("vapor-criteria-v1");
    assert!(parse_criteria(&crit).is_err());
}

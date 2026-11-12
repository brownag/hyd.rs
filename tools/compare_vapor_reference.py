#!/usr/bin/env python3
"""Standalone comparator for the vapor reference (25.7.2).

Compares `vapor-candidate-v1` / `vapor-kernel-candidate-v1` files with the pinned-Fortran
fixtures under reference/vapor_fortran/ using the approved table in criteria.json
(`vapor-criteria-v2`). Pass rule: |c - r| <= absolute + relative*|r|, raised to at least
fraction*max_n|r_t,n| (per print time t) for `case_time_field_max_fraction` fields.

Usage:
  compare_vapor_reference.py trajectory --fixture F --candidate X --output OUT.json
  compare_vapor_reference.py kernel --fixture K --candidate X --output OUT.json
  compare_vapor_reference.py discrimination --output OUT.json
Exit status: 0 pass, 1 any FAIL verdict, 2 invalid input. OUT.json must not exist.
"""

import argparse
import hashlib
import json
import math
import struct
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
VAPOR_DIR = REPO_ROOT / "reference/vapor_fortran"
DEFAULT_CRITERIA = VAPOR_DIR / "criteria.json"
DEFAULT_FIXTURE_DIR = VAPOR_DIR / "fixtures"
DEFAULT_REQUESTS = VAPOR_DIR / "kernel_harness/requests.json"
CRITERIA_SCHEMA = "vapor-criteria-v2"
CANDIDATE_SCHEMA = "vapor-candidate-v1"
KERNEL_CANDIDATE_SCHEMA = "vapor-kernel-candidate-v1"
RESULT_SCHEMA = "vapor-comparison-result-v1"
TRAJ = "pinned-fortran-trajectory"
KERN = "pinned-fortran-kernel"
VAPOR_FIELDS = (
    "con", "con_lt", "con_vh", "con_vt", "v_liquid", "v_total", "v_vap_iso", "v_vap_term",
    "v_vapor",
)
FIELD_UNITS = {
    "head": "cm", "liquid_moisture": "cm3/cm3", "temperature_c": "degC",
    "con": "cm/h", "con_lt": "cm2/K/h", "con_vh": "cm/h", "con_vt": "cm2/K/h",
    "v_liquid": "cm/h", "v_total": "cm/h", "v_vap_iso": "cm/h", "v_vap_term": "cm/h",
    "v_vapor": "cm/h",
    "kernel_vapor_content": "cm3/cm3", "kernel_adjusted_hydraulic_capacity": "1/cm",
    "kernel_capacity_correction": "1/cm",
}
FIELD_CLASS = {name: KERN if name.startswith("kernel_") else TRAJ for name in FIELD_UNITS}
TRAJ_FIELDS = ("head", "liquid_moisture", "temperature_c") + VAPOR_FIELDS
KERNEL_FIELDS = ("kernel_vapor_content", "kernel_adjusted_hydraulic_capacity",
                 "kernel_capacity_correction")
SCALES = ("abs_reference", "none")
PRECISION_RULE = "case_time_field_max_fraction_with_temperature_precision"
ZERO_RULES = ("none", "case_time_field_max_fraction", PRECISION_RULE)
PRECISION_KEYS = {"name": PRECISION_RULE, "temperature_precision": "float32",
                  "conductivity_field": "con_vt",
                  "conductivity_neighbourhood": "node_and_mesh_neighbours_same_time",
                  "spacing": "min_adjacent_dx"}
DISC_RULES = ("any_print_time_after_initial", "max_abs_exceeds_zero_floor_each_case")
FIELD_KEYS = {"absolute", "evidence_class", "print_rule", "relative", "scale", "unit", "zero_rule"}
CHECK_KEYS = {"absolute", "assumptions", "relative", "scale", "unit"}
OPTIONAL_TOP_KEYS = {"revision_note"}
TOP_KEYS = {"approved_at", "approved_by", "checks", "contract_revision", "contract_sha256",
            "discrimination", "fields", "schema_version"}
CANDIDATE_KEYS = {"schema_version", "case_id", "evidence_class", "criteria_sha256",
                  "fixture_sha256", "input_sha256", "schedule", "time_points", "node_numbers",
                  "node_depths", "head", "liquid_moisture", "temperature_c",
                  "vapor_output_available", "vapor_fields"}
OBS_KEYS = {"request_id", "node", "vapor_content", "adjusted_hydraulic_capacity",
            "input_capacity"}
DECKS = ("SELECTOR.IN", "PROFILE.DAT")


class Reject(ValueError):
    """Invalid criteria, fixture or candidate input."""


def sha256_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_json(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle, parse_constant=_no_constant)
    except (OSError, json.JSONDecodeError) as error:
        raise Reject(f"{path}: {error}") from error


def _no_constant(name):
    raise Reject(f"non-finite JSON constant {name}")


def is_num(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def _nonneg(value, label):
    if not is_num(value) or value < 0:
        raise Reject(f"{label}: expected finite nonnegative number")


def _keys(obj, expected, label):
    if not isinstance(obj, dict):
        raise Reject(f"{label}: expected object")
    if set(obj) != set(expected):
        raise Reject(f"{label}: keys {sorted(obj)} != {sorted(expected)}")


def validate_criteria(criteria):
    if isinstance(criteria, dict) and "revision_note" in criteria:
        if not isinstance(criteria["revision_note"], str):
            raise Reject("criteria.revision_note: expected string")
        criteria = {k: v for k, v in criteria.items() if k != "revision_note"}
    _keys(criteria, TOP_KEYS, "criteria")
    if criteria["schema_version"] != CRITERIA_SCHEMA:
        raise Reject("criteria: unknown schema_version")
    fields = criteria["fields"]
    if not isinstance(fields, dict) or set(fields) != set(FIELD_UNITS):
        raise Reject("criteria.fields: unknown or missing field names")
    for name, spec in fields.items():
        label = f"criteria.fields.{name}"
        _keys(spec, FIELD_KEYS, label)
        if spec["evidence_class"] != FIELD_CLASS[name]:
            raise Reject(f"{label}: evidence_class {spec['evidence_class']!r} mismatch")
        if spec["unit"] != FIELD_UNITS[name]:
            raise Reject(f"{label}: unit {spec['unit']!r} mismatch")
        if spec["scale"] not in SCALES:
            raise Reject(f"{label}: unknown scale {spec['scale']!r}")
        if not isinstance(spec["print_rule"], str) or not spec["print_rule"]:
            raise Reject(f"{label}: print_rule missing")
        _nonneg(spec["absolute"], f"{label}.absolute")
        _nonneg(spec["relative"], f"{label}.relative")
        if spec["scale"] == "none" and spec["relative"] != 0:
            raise Reject(f"{label}: scale none requires relative == 0")
        zero = spec["zero_rule"]
        if not isinstance(zero, dict) or zero.get("name") not in ZERO_RULES:
            raise Reject(f"{label}: unknown zero_rule")
        if zero["name"] == "none":
            _keys(zero, {"name"}, f"{label}.zero_rule")
        else:
            extra = {"name", "fraction"}
            if zero["name"] == PRECISION_RULE:
                extra |= set(PRECISION_KEYS) | {"ulp_multiple"}
                if name not in ("v_vapor", "v_vap_term", "v_total"):
                    raise Reject(f"{label}: precision zero_rule not allowed here")
            _keys(zero, extra, f"{label}.zero_rule")
            if zero["name"] == PRECISION_RULE:
                for key, allowed in PRECISION_KEYS.items():
                    if zero[key] != allowed:
                        raise Reject(f"{label}.zero_rule.{key}: expected {allowed!r}")
                if not is_num(zero["ulp_multiple"]) or zero["ulp_multiple"] <= 0:
                    raise Reject(f"{label}.zero_rule.ulp_multiple: expected positive number")
            if not is_num(zero["fraction"]) or not 0 < zero["fraction"] <= 1:
                raise Reject(f"{label}.zero_rule.fraction: expected (0, 1]")
    checks = criteria["checks"]
    if not isinstance(checks, dict) or not checks:
        raise Reject("criteria.checks: expected non-empty object")
    for name, spec in checks.items():
        _keys(spec, CHECK_KEYS, f"criteria.checks.{name}")
        _nonneg(spec["absolute"], f"criteria.checks.{name}.absolute")
        _nonneg(spec["relative"], f"criteria.checks.{name}.relative")
    disc = criteria["discrimination"]
    if not isinstance(disc, dict) or not disc:
        raise Reject("criteria.discrimination: expected non-empty object")
    for name, spec in disc.items():
        label = f"criteria.discrimination.{name}"
        if not isinstance(spec, dict) or spec.get("rule") not in DISC_RULES:
            raise Reject(f"{label}: unknown rule {spec.get('rule') if isinstance(spec, dict) else spec!r}")
        if not is_num(spec.get("factor_of_tolerance")) or spec["factor_of_tolerance"] <= 0:
            raise Reject(f"{label}.factor_of_tolerance: expected positive number")
        if spec["rule"] == "any_print_time_after_initial":
            _keys(spec, {"case", "control", "factor_of_tolerance", "field", "rule", "unit"}, label)
            if spec["field"] not in fields or FIELD_CLASS[spec["field"]] != TRAJ:
                raise Reject(f"{label}: unknown field {spec['field']!r}")
            if spec["unit"] != FIELD_UNITS[spec["field"]]:
                raise Reject(f"{label}: unit mismatch")
            if not all(isinstance(spec[k], str) for k in ("case", "control")):
                raise Reject(f"{label}: case/control must be strings")
        else:
            _keys(spec, {"cases", "factor_of_tolerance", "fields", "rule", "unit"}, label)
            if not spec["cases"] or not all(isinstance(c, str) for c in spec["cases"]):
                raise Reject(f"{label}: cases must be non-empty strings")
            if not spec["fields"] or any(f not in VAPOR_FIELDS for f in spec["fields"]):
                raise Reject(f"{label}: fields must be vapor fields")
            for f in spec["fields"]:
                if fields[f]["zero_rule"]["name"] not in ZERO_RULES[1:]:
                    raise Reject(f"{label}: field {f} has no zero floor")
    return criteria


def load_criteria(path):
    criteria = validate_criteria(load_json(path))
    return criteria, sha256_file(path)


def _matrix(value, rows, cols, label):
    if not isinstance(value, list) or len(value) != rows:
        raise Reject(f"{label}: expected {rows} time rows")
    for row in value:
        if not isinstance(row, list) or len(row) != cols:
            raise Reject(f"{label}: expected {cols} nodes per row (missing node/time)")
        if not all(is_num(v) for v in row):
            raise Reject(f"{label}: non-finite or non-numeric value")


def _vector(value, label):
    if not isinstance(value, list) or not value or not all(is_num(v) for v in value):
        raise Reject(f"{label}: expected non-empty finite numeric list")


def is_trajectory_fixture(data):
    return isinstance(data, dict) and "case_id" in data and "time_points" in data


def validate_trajectory(data, label, candidate=False):
    """Validate shared fixture/candidate layout; returns (times, nodes)."""
    for key in ("time_points", "node_numbers", "node_depths"):
        _vector(data.get(key), f"{label}.{key}")
    times, nodes = len(data["time_points"]), len(data["node_numbers"])
    if len(data["node_depths"]) != nodes:
        raise Reject(f"{label}: node_depths length")
    for name in ("head", "liquid_moisture", "temperature_c"):
        _matrix(data.get(name), times, nodes, f"{label}.{name}")
    available = data.get("vapor_output_available")
    if not isinstance(available, bool):
        raise Reject(f"{label}: vapor_output_available must be boolean")
    vf = data.get("vapor_fields")
    if available:
        if not isinstance(vf, dict) or set(vf) != set(VAPOR_FIELDS):
            raise Reject(f"{label}: vapor_fields must hold exactly the nine fields")
        for name in VAPOR_FIELDS:
            _matrix(vf[name], times, nodes, f"{label}.vapor_fields.{name}")
    elif vf is not None:
        raise Reject(f"{label}: vapor_fields must be null when output unavailable")
    return times, nodes


def load_fixture(path):
    data = load_json(path)
    if not is_trajectory_fixture(data):
        raise Reject(f"{path}: not a trajectory fixture")
    if data.get("evidence_class") != TRAJ or data.get("source_matched") is not True:
        raise Reject(f"{path}: evidence_class/source_matched")
    if not isinstance(data.get("case_id"), str):
        raise Reject(f"{path}: case_id")
    validate_trajectory(data, str(path))
    digests = data.get("manifest", {}).get("input_sha256") if isinstance(data.get("manifest"), dict) else None
    if not isinstance(digests, dict):
        raise Reject(f"{path}: manifest.input_sha256 missing")
    return data, sha256_file(path)


def _is_hex(value):
    return isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value)


def validate_candidate(cand, fixture, crit_sha, fix_sha):
    _keys(cand, CANDIDATE_KEYS, "candidate")
    if cand["schema_version"] != CANDIDATE_SCHEMA:
        raise Reject("candidate: schema_version")
    if cand["evidence_class"] != "rust-candidate":
        raise Reject("candidate: evidence_class must be rust-candidate")
    if cand["case_id"] != fixture["case_id"]:
        raise Reject("candidate: case_id mismatch")
    if cand["criteria_sha256"] != crit_sha:
        raise Reject("candidate: criteria_sha256 mismatch")
    if cand["fixture_sha256"] != fix_sha:
        raise Reject("candidate: fixture_sha256 mismatch (tampered or wrong fixture)")
    inputs = cand["input_sha256"]
    manifest = fixture["manifest"]["input_sha256"]
    if not isinstance(inputs, dict) or set(inputs) != set(DECKS):
        raise Reject("candidate: input_sha256 must hold SELECTOR.IN and PROFILE.DAT")
    for deck in DECKS:
        if not _is_hex(inputs[deck]) or inputs[deck] != manifest.get(deck):
            raise Reject(f"candidate: input_sha256[{deck}] differs from fixture manifest")
    sched = cand["schedule"]
    _keys(sched, {"kind", "dt", "retry_factor", "note"}, "candidate.schedule")
    if sched["kind"] != "constant_dt" or not isinstance(sched["note"], str):
        raise Reject("candidate.schedule: kind/note")
    for key in ("dt", "retry_factor"):
        if not is_num(sched[key]) or sched[key] <= 0:
            raise Reject(f"candidate.schedule.{key}: expected finite positive")
    for key in ("time_points", "node_numbers", "node_depths"):
        if cand[key] != fixture[key]:
            raise Reject(f"candidate: {key} differ from fixture")
    validate_trajectory(cand, "candidate")
    if cand["vapor_output_available"] != fixture["vapor_output_available"]:
        raise Reject("candidate: vapor_output_available differs from fixture (missing vapor output)")


def _row(case, field, time, node, c, r, bound):
    err = abs(c - r)
    return {"case_id": case, "field": field, "time": time, "node": node, "candidate": c,
            "reference": r, "error": err, "bound": bound, "verdict": "PASS" if err <= bound else "FAIL"}


def ulp32(x):
    """float32 spacing at |x| (next float32 above |x| minus float32(|x|))."""
    try:
        base = struct.unpack("<f", struct.pack("<f", abs(x)))[0]
        bits = struct.unpack("<I", struct.pack("<f", base))[0]
        return struct.unpack("<f", struct.pack("<I", bits + 1))[0] - base
    except (OverflowError, struct.error) as error:
        raise Reject(f"ulp32({x}) not representable") from error


def precision_terms(fixture, t, n):
    """2-ulp temperature term factor: ulp32(|T_ref|) * ConVT_max / dx_min at (t, n)."""
    vf = fixture.get("vapor_fields")
    if not isinstance(vf, dict) or "con_vt" not in vf:
        raise Reject("fixture: con_vt required for temperature-precision zero rule")
    depths = fixture["node_depths"]
    gaps = [abs(depths[m] - depths[n]) for m in (n - 1, n + 1) if 0 <= m < len(depths)]
    if not gaps or min(gaps) <= 0:
        raise Reject("fixture: node_depths give no positive adjacent spacing")
    row = vf["con_vt"][t]
    convt = max(abs(row[m]) for m in (n - 1, n, n + 1) if 0 <= m < len(row))
    return ulp32(fixture["temperature_c"][t][n]) * convt / min(gaps)


def compare_matrix(case, field, spec, times, nodes, cand, ref, fixture=None):
    zero = spec["zero_rule"]
    rows = []
    for t, (crow, rrow) in enumerate(zip(cand, ref)):
        floor = 0.0  # per-time floor: fraction * max_n |r_t,n|
        if zero["name"] != "none":
            floor = zero["fraction"] * max(abs(v) for v in rrow)
        for n, (c, r) in enumerate(zip(crow, rrow)):
            rel = spec["relative"] * abs(r) if spec["scale"] == "abs_reference" else 0.0
            bound = max(spec["absolute"] + rel, floor)
            if zero["name"] == PRECISION_RULE:
                bound = max(bound, zero["ulp_multiple"] * precision_terms(fixture, t, n))
            rows.append(_row(case, field, times[t], nodes[n], c, r, bound))
    return rows


def summarize(rows):
    out = {}
    for row in rows:
        s = out.setdefault(row["field"], {"points": 0, "failures": 0, "max_error": 0.0,
                                          "max_error_over_bound": 0.0})
        s["points"] += 1
        s["failures"] += row["verdict"] == "FAIL"
        s["max_error"] = max(s["max_error"], row["error"])
        ratio = row["error"] / row["bound"] if row["bound"] > 0 else (0.0 if row["error"] == 0 else math.inf)
        s["max_error_over_bound"] = max(s["max_error_over_bound"], ratio)
    return out


def compare_trajectory(criteria, crit_sha, fixture, fix_sha, cand):
    validate_trajectory(fixture, "fixture")
    validate_candidate(cand, fixture, crit_sha, fix_sha)
    case, times, nodes = fixture["case_id"], fixture["time_points"], fixture["node_numbers"]
    rows, unavailable = [], []
    for name in TRAJ_FIELDS:
        if name in VAPOR_FIELDS:
            if not fixture["vapor_output_available"]:
                unavailable.append(name)
                continue
            c, r = cand["vapor_fields"][name], fixture["vapor_fields"][name]
        else:
            c, r = cand[name], fixture[name]
        rows += compare_matrix(case, name, criteria["fields"][name], times, nodes, c, r, fixture)
    return result_dict("trajectory", crit_sha, rows, case_id=case, fixture_sha256=fix_sha,
                       unavailable=unavailable)


def _kernel_values(obs):
    vc, adj, inp = obs["vapor_content"], obs["adjusted_hydraulic_capacity"], obs["input_capacity"]
    return {"kernel_vapor_content": vc, "kernel_adjusted_hydraulic_capacity": adj,
            "kernel_capacity_correction": inp - adj}


def _kernel_index(observations, label):
    if not isinstance(observations, list) or not observations:
        raise Reject(f"{label}: observations must be a non-empty list")
    index = {}
    for obs in observations:
        _keys(obs, OBS_KEYS, f"{label}.observation")
        if not isinstance(obs["request_id"], str) or isinstance(obs["node"], bool) \
                or not isinstance(obs["node"], int):
            raise Reject(f"{label}: request_id/node types")
        for key in ("vapor_content", "adjusted_hydraulic_capacity", "input_capacity"):
            if not is_num(obs[key]):
                raise Reject(f"{label}: non-finite {key}")
        key = (obs["request_id"], obs["node"])
        if key in index:
            raise Reject(f"{label}: duplicate observation {key}")
        index[key] = obs
    return index


def compare_kernel(criteria, crit_sha, fixture, fix_sha, cand, requests_sha=None):
    if fixture.get("evidence_class") != KERN:
        raise Reject("kernel fixture: evidence_class")
    ref = _kernel_index(fixture.get("observations"), "fixture")
    _keys(cand, {"schema_version", "evidence_class", "criteria_sha256", "fixture_sha256",
                 "requests_sha256", "observations"}, "kernel candidate")
    if cand["evidence_class"] != "rust-candidate":
        raise Reject("kernel candidate: evidence_class must be rust-candidate")
    if requests_sha is None:
        requests_sha = sha256_file(DEFAULT_REQUESTS)
    if cand["requests_sha256"] != requests_sha:
        raise Reject("kernel candidate: requests_sha256 mismatch")
    if cand["schema_version"] != KERNEL_CANDIDATE_SCHEMA:
        raise Reject("kernel candidate: schema_version")
    if cand["criteria_sha256"] != crit_sha or cand["fixture_sha256"] != fix_sha:
        raise Reject("kernel candidate: criteria/fixture hash mismatch")
    got = _kernel_index(cand["observations"], "candidate")
    if set(got) != set(ref):
        raise Reject(f"kernel candidate: observation set differs (missing {sorted(set(ref) - set(got))},"
                     f" extra {sorted(set(got) - set(ref))})")
    rows = []
    for (rid, node) in sorted(ref):
        cv, rv = _kernel_values(got[(rid, node)]), _kernel_values(ref[(rid, node)])
        for name in KERNEL_FIELDS:
            spec = criteria["fields"][name]
            rel = spec["relative"] * abs(rv[name])
            rows.append(_row(rid, name, None, node, cv[name], rv[name], spec["absolute"] + rel))
    return result_dict("kernel", crit_sha, rows, fixture_sha256=fix_sha, case_id="vapor_content")


def discrimination(criteria, crit_sha, fixtures):
    """fixtures: {case_id: fixture}. Returns result dict from the reference fixtures only."""
    checks = []
    for name, spec in sorted(criteria["discrimination"].items()):
        if spec["rule"] == "any_print_time_after_initial":
            checks.append(_dry_check(name, spec, criteria, fixtures))
        else:
            checks += _nonzero_checks(name, spec, criteria, fixtures)
    return {"schema_version": RESULT_SCHEMA, "mode": "discrimination", "criteria_sha256": crit_sha,
            "checks": checks, "verdict": "PASS" if all(c["verdict"] == "PASS" for c in checks) else "FAIL"}


def _need(fixtures, case):
    if case not in fixtures:
        raise Reject(f"discrimination: fixture for {case} not found")
    return fixtures[case]


def _dry_check(name, spec, criteria, fixtures):
    on, off = _need(fixtures, spec["case"]), _need(fixtures, spec["control"])
    for key in ("time_points", "node_numbers", "node_depths"):
        if on[key] != off[key]:
            raise Reject(f"discrimination {name}: {key} differ between case and control")
    fld = criteria["fields"][spec["field"]]
    best, count, best_ratio = 0.0, 0, 0.0
    for t in range(1, len(on["time_points"])):
        for r_on, r_off in zip(on[spec["field"]][t], off[spec["field"]][t]):
            rel = fld["relative"] * abs(r_on) if fld["scale"] == "abs_reference" else 0.0
            tol = spec["factor_of_tolerance"] * (fld["absolute"] + rel)
            diff = abs(r_on - r_off)
            best = max(best, diff)
            if diff > tol:
                count += 1
                best_ratio = max(best_ratio, diff / tol if tol > 0 else math.inf)
    return {"name": name, "rule": spec["rule"], "case_id": spec["case"], "control": spec["control"],
            "field": spec["field"], "max_abs_difference": best, "points_above_tolerance": count,
            "verdict": "PASS" if count > 0 else "FAIL"}


def _nonzero_checks(name, spec, criteria, fixtures):
    out = []
    for case in spec["cases"]:
        fx = _need(fixtures, case)
        if not fx["vapor_output_available"]:
            raise Reject(f"discrimination {name}: {case} has no vapor output")
        for field in spec["fields"]:
            frac = criteria["fields"][field]["zero_rule"]["fraction"] * spec["factor_of_tolerance"]
            data = fx["vapor_fields"][field]
            peak = max(abs(v) for row in data for v in row)
            # per-time floor: point (t, n) is above iff |r_t,n| > frac * max_n |r_t,n|
            above = sum(abs(v) > frac * max(abs(x) for x in row)
                        for row in data[1:] for v in row)
            out.append({"name": name, "rule": spec["rule"], "case_id": case, "field": field,
                        "max_abs": peak, "floor_semantics": "per_time_fraction_of_max_over_nodes",
                        "fraction": frac, "points_above_floor": above,
                        "verdict": "PASS" if peak > 0 and above > 0 else "FAIL"})
    return out


def result_dict(mode, crit_sha, rows, **extra):
    fails = [r for r in rows if r["verdict"] == "FAIL"]
    out = {"schema_version": RESULT_SCHEMA, "mode": mode, "criteria_sha256": crit_sha, **extra,
           "summary": summarize(rows), "points": len(rows), "failures": len(fails),
           "rows": rows, "verdict": "FAIL" if fails else "PASS"}
    return out


def write_new(path, payload):
    """Write JSON to a path that must not already exist."""
    try:
        with open(path, "x", encoding="utf-8") as handle:
            json.dump(payload, handle, indent=2, sort_keys=True)
            handle.write("\n")
    except FileExistsError as error:
        raise Reject(f"refusing to overwrite {path}") from error


def print_table(result, detail=False):
    print(f"mode={result['mode']} criteria_sha256={result['criteria_sha256']}")
    if result["mode"] == "discrimination":
        for c in result["checks"]:
            tail = {k: v for k, v in c.items() if k not in ("name", "rule", "verdict")}
            print(f"{c['verdict']:<4} {c['name']}: {tail}")
    else:
        print(f"{'field':<38} {'points':>7} {'fail':>5} {'max_err':>12} {'err/bound':>10}")
        for field, s in sorted(result["summary"].items()):
            print(f"{field:<38} {s['points']:>7} {s['failures']:>5} {s['max_error']:>12.4e} "
                  f"{s['max_error_over_bound']:>10.3f}")
        for field in result.get("unavailable", []):
            print(f"{field:<38} unavailable (fixture has no vapor output)")
        for row in result["rows"]:
            if detail or row["verdict"] == "FAIL":
                print(f"{row['verdict']:<4} {row['case_id']} {row['field']} t={row['time']} "
                      f"node={row['node']} cand={row['candidate']:.6e} ref={row['reference']:.6e} "
                      f"err={row['error']:.3e} bound={row['bound']:.3e}")
    print(f"verdict: {result['verdict']}")


def load_fixture_dir(directory):
    fixtures = {}
    for path in sorted(Path(directory).glob("*.json")):
        if not is_trajectory_fixture(load_json(path)):
            continue  # e.g. dry_discrimination.json summary
        data, _ = load_fixture(path)
        fixtures[data["case_id"]] = data
    return fixtures


def run(args):
    criteria, crit_sha = load_criteria(args.criteria)
    if args.command == "discrimination":
        result = discrimination(criteria, crit_sha, load_fixture_dir(args.fixture_dir))
    elif args.command == "trajectory":
        fixture, fix_sha = load_fixture(args.fixture)
        result = compare_trajectory(criteria, crit_sha, fixture, fix_sha, load_json(args.candidate))
    else:
        result = compare_kernel(criteria, crit_sha, load_json(args.fixture), sha256_file(args.fixture),
                                load_json(args.candidate), sha256_file(args.requests))
    if Path(args.output).exists():
        raise Reject(f"refusing to overwrite {args.output}")
    print_table(result, getattr(args, "detail", False))
    write_new(args.output, result)
    return 0 if result["verdict"] == "PASS" else 1


def build_parser():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("trajectory", "kernel", "discrimination"):
        p = sub.add_parser(name)
        p.add_argument("--criteria", default=str(DEFAULT_CRITERIA))
        p.add_argument("--output", required=True, help="new JSON result path (must not exist)")
        if name == "discrimination":
            p.add_argument("--fixture-dir", default=str(DEFAULT_FIXTURE_DIR))
        else:
            p.add_argument("--fixture", required=True)
            p.add_argument("--candidate", required=True)
        if name == "kernel":
            p.add_argument("--requests", default=str(DEFAULT_REQUESTS))
        if name == "trajectory":
            p.add_argument("--detail", action="store_true")
    return parser


def main(argv=None):
    args = build_parser().parse_args(argv)
    try:
        return run(args)
    except Reject as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())

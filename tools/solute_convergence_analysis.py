"""Pure convergence analysis of metrics bound to separately trusted caller evidence.

No files are read or written. Expected bindings must come from the caller's authenticated
receipt replay, not from checksums recomputed after editing the supplied records. This
module checks in-memory consistency; it cannot authenticate the underlying files.
"""

import hashlib
import math
import re

from tools import check_solute_verification_spec as contract
from tools import solute_verification_metrics as metrics

require = metrics.require
HASH = re.compile(r"[0-9a-f]{64}\Z")
NORMS = ("l1", "l2", "linf", "endpoint_linf", "interior_linf")
PHASES = ("dissolved", "equilibrium_sorbed", "total")


def digest(value) -> str:
    """Digest canonical JSON in memory, without consulting any source or artifact file."""
    return hashlib.sha256(metrics.canonical(value)).hexdigest()


def _hashes(values, label):
    require(isinstance(values, dict) and values, f"missing {label}")
    require(
        all(
            isinstance(k, str) and k and isinstance(v, str) and HASH.fullmatch(v)
            for k, v in values.items()
        ),
        f"invalid {label}",
    )


def _number(value, label, nonnegative=False):
    metrics.finite(value, label, nonnegative)
    return value


def _validate_record(book, record, trusted):
    metrics.keys(record, "result reference_values bounds", "convergence record")
    result = record["result"]
    require(isinstance(result, dict), "missing result")
    identity = result.get("launch_id")
    rows = [r for r in book["launches"] if r["id"] == identity]
    require(len(rows) == 1 and rows[0]["phase"] == "convergence", "non-convergence launch")
    row = rows[0]
    case = next(c for c in book["cases"] if c["id"] == row["case_id"])
    require(result.get("status") == "metrics_complete", "incomplete independent metrics")
    require(result.get("kind") in ("benchmark", "convergence"), "invalid result kind")
    require(
        result.get("scientific_accuracy_accepted") is False
        and result.get("thresholds") is None
        and result.get("m5v_complete") is False,
        "assigned scientific status",
    )
    _hashes(result.get("run"), "run bindings")
    run_names = {name.rsplit("/", 1)[-1] for name in result["run"]}
    require(
        run_names
        in (
            {"manifest.json", "observations.json"},
            {"manifest.json", "observations.json", "normalization.json"},
        ),
        "missing run manifest/observation bindings",
    )
    require(len(run_names) == len(result["run"]), "ambiguous run artifact bindings")
    _hashes(result.get("producer_sha256"), "producer bindings")
    receipt = result.get("reference")
    require(
        isinstance(receipt, dict) and receipt.get("status") == "admitted",
        "missing admitted reference receipt",
    )
    require(
        isinstance(receipt.get("identity"), str) and HASH.fullmatch(receipt["identity"]),
        "invalid reference identity",
    )
    _hashes(receipt.get("artifacts"), "reference artifact bindings")
    require(isinstance(trusted, dict), "missing separately trusted bindings")
    expected = {
        "book_sha256": digest(book),
        "row_sha256": digest(row),
        "record_sha256": digest(record),
        "result_sha256": digest(result),
        "run_sha256": digest(result["run"]),
        "reference_sha256": digest(receipt),
        "reference_values_sha256": digest(record["reference_values"]),
        "producer_sha256": result["producer_sha256"],
    }
    require(trusted == expected, "record differs from separately trusted provenance")
    data = result["metrics"]
    metrics.keys(data, "raw derived solver_reported", "metrics")
    raw, derived = data["raw"], data["derived"]
    _hashes(
        {name: raw.get(name) for name in ("cases_sha256", "case_sha256", "request_sha256")},
        "raw identity hashes",
    )
    require(
        raw["schema_version"] == "solute-observation-v1"
        and raw["launch_id"] == identity
        and raw["case_id"] == case["id"]
        and raw["solver"] == row["solver"]
        and raw["case_sha256"] == digest(case)
        and raw["units"] == book["units"],
        "metrics observation identity mismatch",
    )
    require(raw["species"] == [s["name"] for s in case["species"]], "species coverage")
    require(
        raw["coordinates"]
        == [i * case["length"] / (row["nodes"] - 1) for i in range(row["nodes"])],
        "mesh coverage",
    )
    profile_times, step_times = metrics.schedule(case, row)
    require([p["time"] for p in raw["profiles"]] == profile_times, "raw time coverage")
    for profile in raw["profiles"]:
        metrics.matrix(
            profile["concentration"], len(raw["species"]), row["nodes"], "raw concentration"
        )
    require([s["time"] for s in raw["steps"]] == step_times, "step coverage")
    for index, step in enumerate(raw["steps"], 1):
        require(step["index"] == index and step["dt"] == row["dt"], "accepted step history")
        for field in ("peclet", "courant"):
            _number(step[field], field, True)
    require(derived.get("thresholds") is None, "metrics threshold assigned")
    require(
        [s["species"] for s in derived["species"]] == raw["species"], "derived species coverage"
    )
    for species in derived["species"]:
        require(
            [p["time"] for p in species["profiles"]] == [0.0] + row["analysis_times"],
            "concentration norm coverage",
        )
        require(
            [p["time"] for p in species["history"]] == profile_times, "derived history coverage"
        )
        for profile in species["profiles"]:
            metrics.keys(profile["norms"], " ".join(NORMS), "concentration norms")
    _validate_reference(record["reference_values"], raw, row, profile_times)
    return case, row


def _validate_reference(reference, raw, row, profile_times):
    if reference is None:
        return
    metrics.keys(
        reference,
        "schema_version coordinates species profiles continuum uncertainty",
        "reference values",
    )
    require(
        reference["schema_version"] == "solute-metrics-reference-v1"
        and reference["coordinates"] == raw["coordinates"]
        and reference["species"] == raw["species"],
        "reference values identity",
    )
    metrics.keys(reference["uncertainty"], "profiles continuum", "reference uncertainty")
    for group, times in (("profiles", [0.0] + row["analysis_times"]), ("continuum", profile_times)):
        for uncertain, source in ((False, reference), (True, reference["uncertainty"])):
            require([v["time"] for v in source[group]] == times, "reference time coverage")
            for value in source[group]:
                if group == "profiles":
                    metrics.keys(value, "time concentration", "reference profile")
                    metrics.matrix(
                        value["concentration"],
                        len(raw["species"]),
                        len(raw["coordinates"]),
                        "reference concentration",
                        uncertain,
                    )
                else:
                    metrics.keys(
                        value, "time " + " ".join(metrics.CONTINUUM_FIELDS), "reference continuum"
                    )
                    for name in metrics.CONTINUUM_FIELDS:
                        if name in metrics.PAIR_FIELDS:
                            metrics.matrix(value[name], len(raw["species"]), 2, name, uncertain)
                        else:
                            metrics.vector(value[name], len(raw["species"]), name, uncertain)


def _bounds(record):
    require(isinstance(record["bounds"], list), "channel bounds must be a list")
    result = {}
    for entry in record["bounds"]:
        metrics.keys(
            entry,
            "species observable time arithmetic coordinate quadrature evidence",
            "channel bounds",
        )
        key = (entry["species"], entry["observable"], entry["time"])
        require(
            key not in result and isinstance(entry["evidence"], str) and entry["evidence"].strip(),
            "duplicate or unexplained channel bounds",
        )
        for name in ("arithmetic", "coordinate", "quadrature"):
            if entry[name] is not None:
                _number(entry[name], name + " bound", True)
        result[key] = entry
    return result


def _point(
    metric,
    *,
    signed=None,
    value=None,
    bounds=None,
    coordinate=True,
    quadrature=False,
    sensitivity=None,
):
    metrics.keys(
        metric,
        "absolute relative branch scale scale_name reference_uncertainty "
        "rounding_uncertainty error_interval",
        "normalized error",
    )
    for key in ("absolute", "scale", "reference_uncertainty", "rounding_uncertainty"):
        _number(metric[key], key, True)
    base = metric["reference_uncertainty"] + metric["rounding_uncertainty"]
    require(
        metric["error_interval"]
        == [max(0.0, metric["absolute"] - base), metric["absolute"] + base],
        "metric error interval",
    )
    branch = (
        "zero" if metric["scale"] == 0 else ("near_zero" if metric["scale"] <= base else "resolved")
    )
    require(metric["branch"] == branch, "metric normalization branch")
    require(
        metric["relative"]
        == (metric["absolute"] / metric["scale"] if branch == "resolved" else None),
        "metric relative normalization",
    )
    bounds = bounds or {}
    missing = []
    extra = {}
    for name, needed in (
        ("arithmetic", True),
        ("coordinate", not coordinate),
        ("quadrature", quadrature),
    ):
        if not needed:
            extra[name] = 0.0
        elif bounds.get(name) is None:
            extra[name] = 0.0
            missing.append(name + "_bound_missing")
        else:
            extra[name] = bounds[name]
    uncertainty = base + math.fsum(extra.values())
    _number(uncertainty, "combined uncertainty", True)
    if signed is not None:
        _number(signed, "signed error")
        # Both sides evaluate the same actual - reference expression from the
        # exact supplied wrapper; there is no inferred sign or tolerance.
        require(
            abs(signed) == metric["absolute"], "signed scalar disagrees with independent metric"
        )
    if value is not None:
        _number(value, "observable value")
    if sensitivity is not None:
        _number(sensitivity, "quadrature sensitivity", True)
    return {
        "error": metric["absolute"],
        "signed_error": signed,
        "value": value,
        "interval": [max(0.0, metric["absolute"] - uncertainty), metric["absolute"] + uncertainty],
        "uncertainty": uncertainty,
        "reference_uncertainty": metric["reference_uncertainty"],
        "rounding_uncertainty": metric["rounding_uncertainty"],
        "extra_bounds": extra,
        "missing": missing,
        "normalization_branch": branch,
        "bound_status": "incomplete" if missing else "supplied_bounds_complete",
        "quadrature_sensitivity": sensitivity,
    }


def _channels(record, row):
    data = record["result"]["metrics"]
    derived = data["derived"]
    reference = record["reference_values"]
    continuum = {} if reference is None else {v["time"]: v for v in reference["continuum"]}
    coordinate = derived["profile_coordinate_uncertainty"]["status"] == "exact_request_mesh"
    require(coordinate == (row["solver"] == "rust"), "coordinate availability mismatch")
    bounds = _bounds(record)
    result = {}

    def add(species, name, time, metric, **kwargs):
        key = (species, name, time)
        require(key not in result, "duplicate observable")
        result[key] = _point(metric, bounds=bounds.get(key), **kwargs)

    for index, species in enumerate(derived["species"]):
        name = species["species"]
        for profile in species["profiles"]:
            for norm in NORMS:
                add(
                    name,
                    "concentration." + norm,
                    profile["time"],
                    profile["norms"][norm],
                    coordinate=coordinate,
                )
        for history in species["history"]:
            time = history["time"]
            truth = continuum.get(time)
            for phase in PHASES:
                actual = history["inventory"][phase]
                signed = actual - truth[phase][index] if truth else None
                add(
                    name,
                    "inventory." + phase,
                    time,
                    history["inventory_error"][phase],
                    signed=signed,
                    value=actual,
                    coordinate=coordinate,
                )
            for field, truth_field in (
                ("production", "production_cumulative"),
                ("loss", "loss_cumulative"),
            ):
                reaction = history["reaction"]
                actual = reaction[field]
                add(
                    name,
                    "reaction." + field,
                    time,
                    reaction[field + "_error"],
                    signed=actual - truth[truth_field][index] if truth else None,
                    value=actual,
                    coordinate=coordinate,
                    quadrature=True,
                    sensitivity=reaction[field + "_quadrature_sensitivity"],
                )
            boundary = history["boundary"]
            require(
                boundary is not None and history["balance"] is not None,
                "independent boundary/balance coverage missing",
            )
            if boundary is not None:
                for field, truth_field in (
                    ("instantaneous", "endpoint_flux"),
                    ("cumulative", "boundary_cumulative"),
                ):
                    for side in range(2):
                        actual = boundary[field][side]
                        add(
                            name,
                            f"boundary.{field}.{side}",
                            time,
                            boundary[field + "_error"][side],
                            value=actual,
                            signed=actual - truth[truth_field][index][side] if truth else None,
                            quadrature=(field == "cumulative" and side == 1),
                            sensitivity=(
                                boundary["outlet_quadrature_sensitivity"]
                                if field == "cumulative" and side == 1
                                else None
                            ),
                        )
            balance = history["balance"]
            if balance is not None:
                add(
                    name,
                    "balance",
                    time,
                    balance["comparison"],
                    value=balance["residual"],
                    signed=balance["residual"] - balance["reference_residual"],
                    coordinate=coordinate,
                    quadrature=True,
                )
    require(set(bounds) <= set(result), "bounds for an absent observable/time")
    return result


def _diagnostics(record, case, row):
    raw = record["result"]["metrics"]["raw"]
    widths = [b - a for a, b in zip(raw["coordinates"], raw["coordinates"][1:])]
    steps = raw["steps"]
    dts = [s["dt"] for s in steps]
    velocity = abs(case["q"]) / case["theta"]
    return {
        "dx": max(widths),
        "dx_range": [min(widths), max(widths)],
        "dt": max(dts),
        "dt_range": [min(dts), max(dts)],
        "accepted_steps": [dict(s) for s in steps],
        "courant_range": [velocity * min(dts) / max(widths), velocity * max(dts) / min(widths)],
        "peclet_range": (
            [
                velocity * min(widths) / case["dispersion"],
                velocity * max(widths) / case["dispersion"],
            ]
            if case["dispersion"]
            else None
        ),
        "peclet_zero_dispersion": (
            ("zero_flow" if velocity == 0 else "unbounded") if case["dispersion"] == 0 else None
        ),
        "reported_courant_range": [
            min(s["courant"] for s in steps),
            max(s["courant"] for s in steps),
        ],
        "reported_peclet_range": [min(s["peclet"] for s in steps), max(s["peclet"] for s in steps)],
        "coefficient_policy": "water velocity q/theta; species retardation recorded separately",
        "retardation": [
            (case["theta"] + case["rho"] * s["kd"]) / case["theta"] for s in case["species"]
        ],
    }


def _control_difference(finest, control, key):
    left, right = finest["channels"][key], control["channels"][key]
    factors = [1.0, 1.0]
    if key[1].startswith("concentration."):
        raw_left = finest["record"]["result"]["metrics"]["raw"]
        raw_right = control["record"]["result"]["metrics"]["raw"]
        species = raw_left["species"].index(key[0])
        profiles = []
        meshes = []
        for raw in (raw_left, raw_right):
            profile = next(p for p in raw["profiles"] if p["time"] == key[2])
            profiles.append(dict(zip(raw["coordinates"], profile["concentration"][species])))
            meshes.append(raw["coordinates"])
        common = sorted(set(meshes[0]) & set(meshes[1]))
        require(
            len(common) >= 3
            and common[0] == meshes[0][0] == meshes[1][0]
            and common[-1] == meshes[0][-1] == meshes[1][-1],
            "control lacks exact common coordinate coverage",
        )
        norms = metrics._norms(
            [profiles[0][x] - profiles[1][x] for x in common], metrics.mesh_weights(common)
        )
        norm = key[1].split(".")[1]
        difference = norms[norm]
        if norm in ("l1", "l2"):
            # A norm bound on the finer mesh does not directly bound restriction
            # to the common mesh. Transform it with the maximum weight ratio.
            common_weights = dict(zip(common, metrics.mesh_weights(common)))
            for index, mesh in enumerate(meshes):
                own_weights = dict(zip(mesh, metrics.mesh_weights(mesh)))
                ratio = max(common_weights[x] / own_weights[x] for x in common)
                factors[index] = math.sqrt(ratio) if norm == "l2" else ratio
        coverage = {
            "common_nodes": len(common),
            "interpolation": False,
            "control_is_coarser": len(meshes[1]) < len(meshes[0]),
        }
    elif left["value"] is not None and right["value"] is not None:
        difference = abs(left["value"] - right["value"])
        coverage = {
            "common_nodes": None,
            "interpolation": False,
            "control_is_coarser": control["row"]["nodes"] < finest["row"]["nodes"],
        }
    else:
        return {"status": "unavailable", "reason": "control observable values unavailable"}
    bound = left["uncertainty"] * factors[0] + right["uncertainty"] * factors[1]
    return {
        "status": "measured",
        "absolute": difference,
        "interval": [max(0.0, difference - bound), difference + bound],
        "missing": sorted(set(left["missing"] + right["missing"])),
        "common_grid_uncertainty_factors": factors,
        **coverage,
    }


def _classify(levels, key, control):
    points = [level["channels"][key] for level in levels]
    axis = levels[0]["row"]["axis"]
    spacings = [level["diagnostics"]["dx" if axis == "space" else "dt"] for level in levels]
    reasons = sorted({reason for p in points for reason in p["missing"]})
    flags = []
    rates = []
    if len(points) < 3:
        reasons.append("fewer_than_three_levels")
    if any(p["error"] == 0 for p in points):
        flags.append("zero_error")
    if any(p["error"] > 0 and p["interval"][0] == 0 for p in points):
        flags.append("nearzero_error")
    if any(p["normalization_branch"] == "near_zero" for p in points):
        flags.append("nearzero_reference_scale")
    signed = [p["signed_error"] for p in points]
    if all(value is not None for value in signed):
        if any(a * b < 0 for a, b in zip(signed, signed[1:])):
            flags.append("sign_change")
    sign_status = "signed_scalar" if all(v is not None for v in signed) else "unavailable"
    if sign_status == "unavailable" and not key[1].startswith("concentration."):
        reasons.append("signed_reference_values_missing")
    for coarse, fine, h0, h1 in zip(points, points[1:], spacings, spacings[1:]):
        require(h0 > h1 > 0, "duplicate/non-refining actual spacing")
        lower0, upper0 = coarse["interval"]
        lower1, upper1 = fine["interval"]
        rate = {"coarse_spacing": h0, "fine_spacing": h1, "observed": None, "interval": None}
        if coarse["error"] > 0 and fine["error"] > 0:
            rate["observed"] = (math.log(coarse["error"]) - math.log(fine["error"])) / (
                math.log(h0) - math.log(h1)
            )
        if lower0 > upper1 > 0 and lower1 > 0:
            denominator = math.log(h0) - math.log(h1)
            rate["interval"] = [
                (math.log(lower0) - math.log(upper1)) / denominator,
                (math.log(upper0) - math.log(lower1)) / denominator,
            ]
        if lower1 > upper0:
            flags.append("nonmonotonicity")
        elif lower0 <= upper1:
            flags.append("plateau_or_uncertainty_overlap")
        rates.append(rate)
    for name, source in (
        ("reference_floor", "reference_uncertainty"),
        ("output_floor", "rounding_uncertainty"),
    ):
        if any(p[source] > 0 and p["error"] <= p[source] for p in points):
            flags.append(name)
    for name in ("arithmetic", "quadrature", "coordinate"):
        if any(
            p["extra_bounds"][name] > 0 and p["error"] <= p["extra_bounds"][name] for p in points
        ):
            flags.append(name + "_floor")
    if any(
        p["quadrature_sensitivity"] is not None
        and p["quadrature_sensitivity"] > 0
        and p["error"] <= p["quadrature_sensitivity"]
        for p in points
    ):
        flags.append("empirical_quadrature_floor")
    comparison = {"status": "missing", "reason": "other-axis control missing"}
    if control is None:
        reasons.append("control_missing")
    elif key not in control["channels"]:
        reasons.append("control_coverage_missing")
    else:
        finest = levels[-1]
        comparison = _control_difference(finest, control, key)
        if comparison["status"] != "measured":
            reasons.append("control_unavailable")
        else:
            reasons.extend(comparison["missing"])
            if comparison["control_is_coarser"]:
                reasons.append("coarser_control_cannot_establish_finest_mesh_adequacy")
            if len(points) > 1:
                drop = points[-2]["interval"][0] - points[-1]["interval"][1]
                comparison["resolved_last_decrease"] = drop
                comparison["finest_error_lower"] = points[-1]["interval"][0]
                comparison["policy"] = (
                    "control upper < finest error lower; also < last decrease lower when positive"
                )
                limit = points[-1]["interval"][0]
                if drop > 0:
                    limit = min(drop, limit)
                if comparison["interval"][1] != 0 and comparison["interval"][1] >= limit:
                    flags.append("control_contamination")
    flags = sorted(set(flags))
    resolved_increase = "nonmonotonicity" in flags
    fully_bounded = not any(p["missing"] for p in points)
    rate_status = (
        "failed"
        if resolved_increase and fully_bounded
        else (
            "achieved"
            if len(points) >= 3
            and fully_bounded
            and all(r["interval"] is not None for r in rates)
            and "sign_change" not in flags
            else "inconclusive"
        )
    )
    floor_reasons = sorted(
        {reason for p in points for reason in p["missing"]}
        | {flag for flag in flags if "floor" in flag}
    )
    floor_status = "inconclusive" if floor_reasons else "bounded"
    control_reasons = [
        reason
        for reason in reasons
        if reason.startswith("control_") or reason.startswith("coarser_control_")
    ]
    if comparison.get("missing"):
        control_reasons.extend(comparison["missing"])
    if "control_contamination" in flags:
        control_reasons.append("control_contamination")
    control_status = (
        "bounded" if comparison["status"] == "measured" and not control_reasons else "inconclusive"
    )
    supported = floor_status == control_status == "bounded" and not reasons
    status = rate_status if supported else "inconclusive"
    reasons = sorted(set(reasons + [flag for flag in flags if flag != "nearzero_reference_scale"]))
    if status == "achieved":
        reasons = ["resolved_observed_decrease_with_bounded_other_axis_control"]
    return {
        "species": key[0],
        "observable": key[1],
        "time": key[2],
        "status": status,
        "reasons": reasons,
        "flags": flags,
        "sign_status": sign_status,
        "rate_assessment": {"status": rate_status},
        "floor_assessment": {"status": floor_status, "reasons": floor_reasons},
        "control_assessment": {"status": control_status, "reasons": sorted(set(control_reasons))},
        "levels": [
            {"launch_id": level["row"]["id"], "spacing": spacing, **point}
            for level, spacing, point in zip(levels, spacings, points)
        ],
        "rates": rates,
        "control": comparison,
        "interpretation": "observed decrease only; no rigorous asymptotic proof",
    }


def analyze_convergence(book: dict, records: list, trusted_bindings: dict) -> dict:
    """Analyze caller-authenticated result records; never launch work or read artifact files.

    Each record has result (metrics_complete payload), reference_values (the exact
    independent metrics reference wrapper, or null), and bounds (per-channel explicit
    arithmetic/coordinate/quadrature bounds with evidence). trusted_bindings maps launch
    IDs to book/row/record/result/run/reference/reference_values SHA256 and producer map.
    The caller must authenticate those expectations independently before calling.
    """
    contract.validate(book, verify_hashes=False)
    require(isinstance(records, list) and isinstance(trusted_bindings, dict), "input containers")
    groups, diagnostics, seen, non_refinement = {}, {}, set(), []
    for record in records:
        identity = record.get("result", {}).get("launch_id")
        require(identity not in seen, "duplicate evidence launch")
        seen.add(identity)
        case, row = _validate_record(book, record, trusted_bindings.get(identity))
        channels = _channels(record, row)
        diagnostic = _diagnostics(record, case, row)
        diagnostics[identity] = diagnostic
        if row["axis"] not in ("space", "time"):
            non_refinement.append(
                {
                    "launch_id": identity,
                    "status": "inconclusive",
                    "reason": "precision sensitivity is not a refinement axis; "
                    "it supplies no rigorous arithmetic bound",
                }
            )
            continue
        group = (row["case_id"], row["solver"], row["axis"], row["precision"])
        groups.setdefault(group, []).append(
            {"record": record, "row": row, "channels": channels, "diagnostics": diagnostic}
        )
    require(set(trusted_bindings) == seen, "trusted binding coverage differs from records")
    reports = []
    for group, entries in sorted(groups.items()):
        axis = group[2]
        levels = sorted(
            (e for e in entries if e["row"]["role"] == "study"),
            key=lambda e: e["diagnostics"]["dx" if axis == "space" else "dt"],
            reverse=True,
        )
        controls = [e for e in entries if e["row"]["role"] == "control"]
        require(len(controls) <= 1, "ambiguous other-axis control")
        if not levels:
            continue
        finest = levels[-1]
        require(
            all(e["row"]["analysis_times"] == finest["row"]["analysis_times"] for e in entries),
            "non-common analysis coverage",
        )
        require(
            all(
                e["row"]["dt" if axis == "space" else "nodes"]
                == finest["row"]["dt" if axis == "space" else "nodes"]
                for e in levels
            ),
            "co-refined study axes",
        )
        control = controls[0] if controls else None
        if control:
            require(
                control["row"]["nodes" if axis == "space" else "dt"]
                == finest["row"]["nodes" if axis == "space" else "dt"],
                "control changed target axis",
            )
            require(
                (
                    control["row"]["dt"] < finest["row"]["dt"]
                    if axis == "space"
                    else control["row"]["nodes"] != finest["row"]["nodes"]
                ),
                "control did not vary other axis",
            )
        coverage = set(levels[0]["channels"])
        require(
            all(set(e["channels"]) == coverage for e in levels), "non-common observable coverage"
        )
        findings = [_classify(levels, key, control) for key in sorted(coverage)]
        statuses = [f["status"] for f in findings]
        status = (
            "failed"
            if "failed" in statuses
            else ("inconclusive" if "inconclusive" in statuses else "achieved")
        )
        counts = {
            "total": len(findings),
            "initial_time": 0,
            "independently_bounded_exact_zero": 0,
            "informative": 0,
        }
        informative_counts = {"achieved": 0, "inconclusive": 0, "failed": 0}
        for finding in findings:
            comparison = finding["control"]
            exact_zero = (
                all(
                    point["bound_status"] == "supplied_bounds_complete"
                    and point["interval"] == [0.0, 0.0]
                    for point in finding["levels"]
                )
                and comparison["status"] == "measured"
                and not comparison["missing"]
                and comparison["interval"] == [0.0, 0.0]
            )
            role = (
                "initial_time"
                if finding["time"] == 0
                else "independently_bounded_exact_zero" if exact_zero else "informative"
            )
            finding["summary_role"] = role
            counts[role] += 1
            if role == "informative":
                informative_counts[finding["status"]] += 1
        informative_status = (
            "failed"
            if informative_counts["failed"]
            else (
                "inconclusive"
                if not counts["informative"] or informative_counts["inconclusive"]
                else "achieved"
            )
        )
        reports.append(
            {
                "case": group[0],
                "solver": group[1],
                "axis": group[2],
                "precision": group[3],
                "status": status,
                "informative_status": informative_status,
                "finding_counts": {**counts, "informative_by_status": informative_counts},
                "findings": findings,
            }
        )
    return {
        "schema_version": "solute-convergence-analysis-v1",
        "tooling_status": "complete",
        "binding_status": "consistent_with_separately_trusted_caller_expectations",
        "uncertainty_interpretation": "conditional on supplied uncertainty and explicit bounds; "
        "admitted reference estimates and quadrature sensitivities may be empirical",
        "filesystem_receipts_authenticated": False,
        "groups": reports,
        "non_refinement_evidence": non_refinement,
        "diagnostics": diagnostics,
        "thresholds": None,
        "scientific_accuracy_accepted": False,
        "m5v_complete": False,
        "unmet": ["accuracy thresholds unassigned", "M5-V open"]
        + (["controlled convergence evidence absent"] if not reports else []),
    }

"""Independent synthetic error fields; no builds, inversion, or solver execution."""

import builtins
import copy
import json
import math
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from tools import solute_convergence_analysis as analysis
from tools import solute_verification_metrics as metrics

BOOK = json.loads(
    (Path(__file__).resolve().parents[1] / "reference/solute_verification/CASES.json").read_text()
)


def bindings(record):
    """Test-only trusted snapshot, captured before exercising mutation rejection."""
    result = record["result"]
    row = next(r for r in BOOK["launches"] if r["id"] == result["launch_id"])
    return {
        "book_sha256": analysis.digest(BOOK),
        "row_sha256": analysis.digest(row),
        "record_sha256": analysis.digest(record),
        "result_sha256": analysis.digest(result),
        "run_sha256": analysis.digest(result["run"]),
        "reference_sha256": analysis.digest(result["reference"]),
        "reference_values_sha256": analysis.digest(record["reference_values"]),
        "producer_sha256": copy.deepcopy(result["producer_sha256"]),
    }


def synthetic_record(row, error, *, reference_uncertainty=0.0, shape=None):
    """Author constant truth and prescribed error, not a physical benchmark solution.

    The accepted metrics calculator measures this independently specified field. The
    private sealed fixture represents already validated caller input only in this test;
    it does not establish physical mapping or filesystem receipt authentication.
    """
    case = next(c for c in BOOK["cases"] if c["id"] == row["case_id"])
    names = [s["name"] for s in case["species"]]
    coordinates = [i * case["length"] / (row["nodes"] - 1) for i in range(row["nodes"])]
    times, step_times = metrics.schedule(case, row)
    count = len(names)
    shape = shape or [1.0] * row["nodes"]

    def field(time):
        return [[1.0 + error * time / case["time_end"] * v for v in shape] for _ in names]

    raw = {
        "schema_version": "solute-observation-v1",
        "run_id": row["id"] + "-synthetic",
        "launch_id": row["id"],
        "case_id": case["id"],
        "cases_sha256": analysis.digest(BOOK),
        "case_sha256": analysis.digest(case),
        "request_sha256": "1" * 64,
        "solver": row["solver"],
        "units": BOOK["units"],
        "species": names,
        "coordinates": coordinates,
        "profiles": [
            {
                "time": t,
                "concentration": field(t),
                "theta": [case["theta"]] * row["nodes"],
                "q_faces": [case["q"]] * (row["nodes"] + 1),
            }
            for t in times
        ],
        "steps": [
            {"index": i, "time": t, "dt": row["dt"], "iterations": 1, "peclet": 0.0, "courant": 0.0}
            for i, t in enumerate(step_times, 1)
        ],
        "endpoint_history": [
            {"time": t, "concentration": [[v[0], v[-1]] for v in field(t)]}
            for t in [0.0] + step_times
        ],
        "solver_reported": {
            field: None
            for field in ("inventory", "boundary_cumulative", "reaction_cumulative", "balance")
        },
        "availability": {
            "water": "synthetic constant field",
            "endpoint_history": "complete",
            "solver_reported": "independent fixture only",
        },
        "warnings": [],
    }
    continuum = []
    for time in times:
        dissolved = [case["theta"] * case["length"]] * count
        sorbed = [case["rho"] * s["kd"] * case["length"] for s in case["species"]]
        total = [a + b for a, b in zip(dissolved, sorbed)]
        loss = [s["rate"] * m * time for s, m in zip(case["species"], total)]
        continuum.append(
            {
                "time": time,
                "dissolved": dissolved,
                "equilibrium_sorbed": sorbed,
                "total": total,
                "endpoint_flux": [[case["q"], -case["q"]] for _ in names],
                "boundary_cumulative": [[case["q"] * time, -case["q"] * time] for _ in names],
                "production_cumulative": [0.0] + loss[:-1],
                "loss_cumulative": loss,
            }
        )
    reference = {
        "schema_version": "solute-metrics-reference-v1",
        "coordinates": coordinates,
        "species": names,
        "profiles": [
            {"time": t, "concentration": [[1.0] * row["nodes"] for _ in names]}
            for t in [0.0] + row["analysis_times"]
        ],
        "continuum": continuum,
        "uncertainty": {
            "profiles": [
                {
                    "time": t,
                    "concentration": [[reference_uncertainty] * row["nodes"] for _ in names],
                }
                for t in [0.0] + row["analysis_times"]
            ],
            "continuum": [
                {
                    "time": t,
                    **{
                        name: (
                            [[reference_uncertainty] * 2 for _ in names]
                            if name in metrics.PAIR_FIELDS
                            else [reference_uncertainty] * count
                        )
                        for name in metrics.CONTINUUM_FIELDS
                    },
                }
                for t in times
            ],
        },
    }
    observations = metrics._ValidatedObservations(raw)
    observations._seal({"case": case, "launch": row, "manifest": {}}, metrics._zero_rounding(raw))
    measured = metrics.compute_metrics(observations, reference)
    result = {
        "kind": "convergence",
        "launch_id": row["id"],
        "status": "metrics_complete",
        "run": {"synthetic/manifest.json": "2" * 64, "synthetic/observations.json": "3" * 64},
        "reference": {
            "status": "admitted",
            "identity": "4" * 64,
            "artifacts": {"reference.json": "5" * 64},
        },
        "metrics": measured,
        "producer_sha256": {"synthetic-test-producer": "6" * 64},
        "scientific_accuracy_accepted": False,
        "thresholds": None,
        "m5v_complete": False,
    }
    record = {"result": result, "reference_values": reference, "bounds": []}
    for species, observable, time in analysis._channels(record, row):
        record["bounds"].append(
            {
                "species": species,
                "observable": observable,
                "time": time,
                "arithmetic": 0.0,
                "coordinate": 0.0,
                "quadrature": 0.0,
                "evidence": "Prescribed synthetic binary fields; zero extra bounds are fixture claims.",
            }
        )
    return record


def study(
    case="diffusion_cosine",
    axis="space",
    solver="rust",
    *,
    order=2,
    errors=None,
    control_error=None,
    reference_uncertainty=0
):
    rows = [
        r
        for r in BOOK["launches"]
        if r["phase"] == "convergence"
        and r["case_id"] == case
        and r["axis"] == axis
        and r["solver"] == solver
    ]
    levels = [r for r in rows if r["role"] == "study"]
    levels.sort(key=lambda r: r["nodes"] if axis == "space" else -r["dt"])
    records = []
    for index, row in enumerate(levels):
        length = next(c["length"] for c in BOOK["cases"] if c["id"] == case)
        h = length / (row["nodes"] - 1) if axis == "space" else row["dt"]
        error = errors[index] if errors is not None else h**order
        records.append(synthetic_record(row, error, reference_uncertainty=reference_uncertainty))
    if control_error != "missing":
        control = next(r for r in rows if r["role"] == "control")
        finest_error = (
            errors[-1]
            if errors is not None
            else (length / (levels[-1]["nodes"] - 1) if axis == "space" else levels[-1]["dt"])
            ** order
        )
        records.append(
            synthetic_record(
                control,
                finest_error if control_error is None else control_error,
                reference_uncertainty=reference_uncertainty,
            )
        )
    return records


def analyze(records):
    trusted = {r["result"]["launch_id"]: bindings(r) for r in records}
    return analysis.analyze_convergence(BOOK, records, trusted)


def finding(report, observable="concentration.l2", time=1, species="S1"):
    return next(
        f
        for g in report["groups"]
        for f in g["findings"]
        if f["observable"] == observable and f["time"] == time and f["species"] == species
    )


class ConvergenceTests(unittest.TestCase):
    def test_group_summary_separates_initial_and_bounded_zero_from_supported_errors(self):
        group = analyze(study())["groups"][0]
        self.assertEqual(group["status"], "inconclusive")
        self.assertEqual(group["informative_status"], "achieved")
        counts = group["finding_counts"]
        self.assertGreater(counts["initial_time"], 0)
        self.assertGreater(counts["independently_bounded_exact_zero"], 0)
        self.assertGreater(counts["informative"], 0)
        self.assertEqual(counts["informative"], counts["informative_by_status"]["achieved"])
        self.assertEqual(
            counts["total"],
            counts["initial_time"]
            + counts["independently_bounded_exact_zero"]
            + counts["informative"],
        )
        self.assertTrue(
            all(
                f["status"] == "inconclusive"
                for f in group["findings"]
                if f["summary_role"] != "informative"
            )
        )

    def test_rounded_zero_and_missing_bounds_remain_informative_inconclusive(self):
        rounded = study(errors=[0.0] * 3, reference_uncertainty=0.01)
        incomplete = study(errors=[0.0] * 3)
        for record in incomplete:
            record["bounds"] = []
        for records in (rounded, incomplete):
            with self.subTest(bounded=records is rounded):
                group = analyze(records)["groups"][0]
                self.assertEqual(group["status"], "inconclusive")
                self.assertEqual(group["informative_status"], "inconclusive")
                counts = group["finding_counts"]
                self.assertEqual(counts["independently_bounded_exact_zero"], 0)
                self.assertGreater(counts["informative_by_status"]["inconclusive"], 0)
                self.assertTrue(
                    all(
                        f["summary_role"] == "informative"
                        for f in group["findings"]
                        if f["time"] > 0
                    )
                )

    def test_known_spatial_orders_are_supported_without_fitting(self):
        for order in (1, 2, 3):
            with self.subTest(order=order):
                report = analyze(study(order=order))
                result = finding(report)
                self.assertEqual(result["status"], "achieved")
                self.assertEqual(result["sign_status"], "unavailable")
                for rate in result["rates"]:
                    self.assertAlmostEqual(rate["observed"], order)
                    self.assertEqual(rate["interval"], [rate["observed"]] * 2)
                self.assertFalse(report["scientific_accuracy_accepted"])
                self.assertFalse(report["m5v_complete"])
                self.assertIsNone(report["thresholds"])
                self.assertFalse(report["filesystem_receipts_authenticated"])

    def test_known_temporal_orders_and_finer_mesh_control(self):
        for order in (1, 2):
            result = finding(analyze(study(axis="time", order=order)))
            self.assertEqual(result["status"], "achieved")
            self.assertFalse(result["control"]["control_is_coarser"])
            self.assertEqual(result["control"]["common_nodes"], 257)
            for rate in result["rates"]:
                self.assertAlmostEqual(rate["observed"], order)

    def test_fine_mesh_norm_uncertainty_is_transformed_for_common_nodes(self):
        records = study(axis="time")
        row = next(r for r in BOOK["launches"] if r["id"] == records[-2]["result"]["launch_id"])
        control_row = next(
            r for r in BOOK["launches"] if r["id"] == records[-1]["result"]["launch_id"]
        )
        key = ("S1", "concentration.l2", 1)
        finest = {
            "record": records[-2],
            "row": row,
            "channels": analysis._channels(records[-2], row),
        }
        control = {
            "record": records[-1],
            "row": control_row,
            "channels": analysis._channels(records[-1], control_row),
        }
        # Fine-grid uncertainty concentrated on every other node has RMS sqrt(1/2).
        # Restricting to exactly those common nodes has RMS 1, so its bound must grow.
        control["channels"][key]["uncertainty"] = math.sqrt(0.5)
        comparison = analysis._control_difference(finest, control, key)
        self.assertAlmostEqual(comparison["common_grid_uncertainty_factors"][1], math.sqrt(2))
        self.assertAlmostEqual(comparison["interval"][1], 1)

    def test_plateau_is_inconclusive(self):
        result = finding(analyze(study(errors=[0.125] * 3)))
        self.assertEqual(result["status"], "inconclusive")
        self.assertIn("plateau_or_uncertainty_overlap", result["flags"])
        self.assertTrue(all(r["interval"] is None for r in result["rates"]))

    def test_zero_errors_are_not_assigned_an_order(self):
        result = finding(analyze(study(errors=[0.0] * 3)))
        self.assertEqual(result["status"], "inconclusive")
        self.assertIn("zero_error", result["flags"])
        self.assertTrue(all(r["observed"] is None for r in result["rates"]))

    def test_mixed_error_is_contaminated_by_other_axis(self):
        errors = [h * h + 0.125 for h in (0.125, 0.0625, 0.03125)]
        result = finding(analyze(study(errors=errors, control_error=0.03125**2 + 0.0625)))
        self.assertEqual(result["status"], "inconclusive")
        self.assertIn("control_contamination", result["flags"])
        self.assertGreater(
            result["control"]["interval"][1], result["control"]["resolved_last_decrease"]
        )

    def test_resolved_nonmonotonic_errors_fail_observed_decrease(self):
        result = finding(analyze(study(errors=[0.125, 0.25, 0.03125])))
        self.assertEqual(result["status"], "failed")
        self.assertIn("nonmonotonicity", result["flags"])
        self.assertAlmostEqual(result["rates"][0]["observed"], -1)

    def test_nonmonotonicity_with_missing_control_remains_inconclusive(self):
        result = finding(analyze(study(errors=[0.125, 0.25, 0.03125], control_error="missing")))
        self.assertEqual(result["rate_assessment"]["status"], "failed")
        self.assertEqual(result["control_assessment"]["status"], "inconclusive")
        self.assertEqual(result["status"], "inconclusive")

    def test_last_pair_increase_does_not_fabricate_zero_control_contamination(self):
        result = finding(analyze(study(errors=[0.125, 0.03125, 0.0625])))
        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["control"]["absolute"], 0)
        self.assertNotIn("control_contamination", result["flags"])

    def test_signed_scalar_crossing_is_reported_but_norm_sign_is_unknown(self):
        report = analyze(study(errors=[0.125, -0.0625, 0.03125]))
        scalar = finding(report, "inventory.total")
        self.assertIn("sign_change", scalar["flags"])
        self.assertEqual(scalar["status"], "inconclusive")
        self.assertNotIn("sign_change", finding(report)["flags"])

    def test_reference_uncertainty_limits_resolvability(self):
        result = finding(analyze(study(reference_uncertainty=0.02)))
        self.assertEqual(result["status"], "inconclusive")
        self.assertIn("reference_floor", result["flags"])
        self.assertIn("nearzero_error", result["flags"])
        self.assertTrue(all(r["interval"] is None for r in result["rates"]))

    def test_output_and_arithmetic_floors_are_quantified(self):
        for field in ("rounding_uncertainty", "arithmetic"):
            records = study()
            for record in records:
                if field == "arithmetic":
                    for bound in record["bounds"]:
                        if bound["observable"] == "concentration.l2":
                            bound["arithmetic"] = 0.02
                else:
                    profiles = record["result"]["metrics"]["derived"]["species"][0]["profiles"]
                    for profile in profiles:
                        value = profile["norms"]["l2"]
                        value["rounding_uncertainty"] = 0.02
                        value["error_interval"] = [
                            max(0.0, value["absolute"] - 0.02),
                            value["absolute"] + 0.02,
                        ]
            result = finding(analyze(records))
            self.assertIn(
                "arithmetic_floor" if field == "arithmetic" else "output_floor", result["flags"]
            )
            self.assertEqual(result["status"], "inconclusive")

    def test_missing_floor_bounds_are_inconclusive(self):
        records = study()
        for record in records:
            record["bounds"] = []
        result = finding(analyze(records))
        self.assertIn("arithmetic_bound_missing", result["reasons"])
        self.assertEqual(result["status"], "inconclusive")

    def test_fortran_unresolved_coordinates_prevent_achieved_claim(self):
        records = study(solver="fortran")
        for record in records:
            for bound in record["bounds"]:
                bound["coordinate"] = None
        result = finding(analyze(records))
        self.assertEqual(result["status"], "inconclusive")
        self.assertIn("coordinate_bound_missing", result["reasons"])

    def test_missing_control_and_partial_levels_remain_inconclusive(self):
        records = study(control_error="missing")[:2]
        result = finding(analyze(records))
        self.assertIn("fewer_than_three_levels", result["reasons"])
        self.assertIn("control_missing", result["reasons"])
        self.assertEqual(result["status"], "inconclusive")

    def test_actual_mesh_steps_and_dimensionless_diagnostics(self):
        records = study(case="scalar_robin", order=1)
        report = analyze(records)
        first = report["diagnostics"][records[0]["result"]["launch_id"]]
        self.assertEqual(first["dx"], 0.0625)
        self.assertEqual(first["dt_range"], [0.00390625] * 2)
        self.assertEqual(first["accepted_steps"][0]["index"], 1)
        self.assertEqual(first["courant_range"], [0.0625] * 2)
        self.assertEqual(first["peclet_range"], [0.0625] * 2)
        self.assertEqual(first["retardation"], [1.5])

    def test_scalar_channels_use_exact_supplied_truth(self):
        report = analyze(study(case="scalar_robin", order=1))
        for observable in (
            "inventory.total",
            "reaction.loss",
            "boundary.instantaneous.1",
            "boundary.cumulative.1",
            "balance",
        ):
            result = finding(report, observable)
            self.assertTrue(all(p["signed_error"] is not None for p in result["levels"]))
        outlet = finding(report, "boundary.instantaneous.1")
        self.assertLess(outlet["levels"][0]["signed_error"], 0)

    def test_missing_reference_values_does_not_invent_scalar_sign(self):
        records = study()
        for record in records:
            record["reference_values"] = None
        result = finding(analyze(records), "inventory.total")
        self.assertEqual(result["sign_status"], "unavailable")
        self.assertTrue(all(p["signed_error"] is None for p in result["levels"]))
        self.assertEqual(result["status"], "inconclusive")
        self.assertIn("signed_reference_values_missing", result["reasons"])

    def test_endpoint_and_interior_norms_are_kept_distinct(self):
        records = study(order=1)
        varied = []
        for record in records:
            row = next(r for r in BOOK["launches"] if r["id"] == record["result"]["launch_id"])
            # The same independent shape on every level, with larger endpoint errors.
            shape = [4.0] + [0.5] * (row["nodes"] - 2) + [4.0]
            h = 2 / (row["nodes"] - 1)
            varied.append(synthetic_record(row, h, shape=shape))
        report = analyze(varied)
        endpoint = finding(report, "concentration.endpoint_linf")
        interior = finding(report, "concentration.interior_linf")
        self.assertEqual(endpoint["levels"][0]["error"], 0.5)
        self.assertEqual(interior["levels"][0]["error"], 0.0625)
        for result in (endpoint, interior):
            self.assertEqual(result["status"], "achieved")
            for rate in result["rates"]:
                self.assertAlmostEqual(rate["observed"], 1)

    def test_example3_coarser_temporal_control_cannot_prove_mesh_adequacy(self):
        rows = [
            r
            for r in BOOK["launches"]
            if r["case_id"] == "example3" and r["axis"] == "time" and r["solver"] == "rust"
        ]
        levels = []
        key = ("NH4", "inventory.total", 200)
        for row in sorted(
            (r for r in rows if r["role"] == "study"), key=lambda r: r["dt"], reverse=True
        ):
            value = 1 + row["dt"]
            point = analysis._point(
                metrics.normalized_error(value, 1),
                signed=value - 1,
                value=value,
                bounds={"arithmetic": 0.0},
            )
            levels.append({"row": row, "channels": {key: point}, "diagnostics": {"dt": row["dt"]}})
        row = next(r for r in rows if r["role"] == "control")
        control = {"row": row, "channels": {key: copy.deepcopy(levels[-1]["channels"][key])}}
        result = analysis._classify(levels, key, control)
        self.assertEqual(result["status"], "inconclusive")
        self.assertEqual(result["control"]["absolute"], 0.0)
        self.assertIn("coarser_control_cannot_establish_finest_mesh_adequacy", result["reasons"])

    def test_rate_formula_does_not_overflow_for_resolved_extreme_errors(self):
        key = ("S1", "concentration.l2", 1)
        levels = []
        for spacing, error in zip((0.125, 0.0625, 0.03125), (1e300, 1.0, 1e-300)):
            point = analysis._point(metrics.normalized_error(error, 0), bounds={"arithmetic": 0.0})
            levels.append(
                {
                    "row": {"axis": "space", "id": str(spacing)},
                    "channels": {key: point},
                    "diagnostics": {"dx": spacing},
                }
            )
        result = analysis._classify(levels, key, None)
        self.assertTrue(all(math.isfinite(r["observed"]) for r in result["rates"]))

    def test_missing_independent_boundary_or_balance_is_rejected(self):
        for field in ("boundary", "balance"):
            records = study()
            records[0]["result"]["metrics"]["derived"]["species"][0]["history"][1][field] = None
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "coverage missing"):
                analyze(records)

    def test_empirical_quadrature_sensitivity_is_not_a_bound(self):
        records = study(case="scalar_robin", order=1)
        for record in records:
            for bound in record["bounds"]:
                if bound["observable"] == "reaction.loss":
                    bound["quadrature"] = None
            for history in record["result"]["metrics"]["derived"]["species"][0]["history"]:
                history["reaction"]["loss_quadrature_sensitivity"] = 1.0
        result = finding(analyze(records), "reaction.loss")
        self.assertEqual(result["status"], "inconclusive")
        self.assertIn("quadrature_bound_missing", result["reasons"])
        self.assertIn("empirical_quadrature_floor", result["flags"])

    def test_missing_or_altered_trusted_provenance_is_rejected(self):
        records = study()
        trusted = {r["result"]["launch_id"]: bindings(r) for r in records}
        for field in ("run", "producer_sha256", "reference"):
            changed = copy.deepcopy(records)
            if field == "reference":
                changed[0]["result"][field]["identity"] = "9" * 64
            else:
                key = next(iter(changed[0]["result"][field]))
                changed[0]["result"][field][key] = "9" * 64
            with self.subTest(field=field), self.assertRaisesRegex(
                ValueError, "trusted provenance"
            ):
                analysis.analyze_convergence(BOOK, changed, trusted)
        with self.assertRaisesRegex(ValueError, "trusted bindings"):
            analysis.analyze_convergence(BOOK, records, {})

    def test_missing_run_artifact_or_raw_hash_is_rejected(self):
        records = study()
        del records[0]["result"]["run"]["synthetic/manifest.json"]
        with self.assertRaisesRegex(ValueError, "manifest/observation"):
            analyze(records)
        records = study()
        del records[0]["result"]["metrics"]["raw"]["request_sha256"]
        with self.assertRaisesRegex(ValueError, "raw identity hashes"):
            analyze(records)

    def test_changed_payload_and_self_updated_hash_do_not_authenticate(self):
        records = study()
        trusted = {r["result"]["launch_id"]: bindings(r) for r in records}
        changed = copy.deepcopy(records)
        changed[0]["reference_values"]["profiles"][1]["concentration"][0][0] += 1
        self.assertNotEqual(bindings(changed[0]), trusted[changed[0]["result"]["launch_id"]])
        with self.assertRaisesRegex(ValueError, "trusted provenance"):
            analysis.analyze_convergence(BOOK, changed, trusted)

    def test_coverage_and_semantic_inconsistency_are_rejected(self):
        mutations = [
            lambda r: r["result"]["metrics"]["raw"]["steps"].pop(),
            lambda r: r["result"]["metrics"]["derived"]["species"][0]["profiles"].pop(),
            lambda r: r["reference_values"]["continuum"].pop(),
            lambda r: r["result"]["metrics"]["raw"]["coordinates"].pop(),
            lambda r: r["result"]["metrics"]["derived"]["species"][0]["profiles"][1]["norms"]["l2"][
                "error_interval"
            ].__setitem__(0, 17),
        ]
        for mutate in mutations:
            records = study()
            mutate(records[0])
            with self.subTest(mutation=mutate), self.assertRaises(ValueError):
                analyze(records)

    def test_duplicate_evidence_and_extra_trusted_binding_are_rejected(self):
        records = study()
        with self.assertRaisesRegex(ValueError, "duplicate"):
            analyze(records + [records[0]])
        trusted = {r["result"]["launch_id"]: bindings(r) for r in records}
        trusted["invented"] = copy.deepcopy(next(iter(trusted.values())))
        with self.assertRaisesRegex(ValueError, "binding coverage"):
            analysis.analyze_convergence(BOOK, records, trusted)

    def test_no_evidence_has_explicit_unmet_status(self):
        report = analysis.analyze_convergence(BOOK, [], {})
        self.assertEqual(report["groups"], [])
        self.assertIn("controlled convergence evidence absent", report["unmet"])

    def test_analysis_performs_no_io_and_does_not_mutate_inputs(self):
        records = study()
        trusted = {r["result"]["launch_id"]: bindings(r) for r in records}
        before = copy.deepcopy((BOOK, records, trusted))
        with patch.object(builtins, "open", side_effect=AssertionError("file IO")), patch.object(
            Path, "open", side_effect=AssertionError("path IO")
        ), patch.object(Path, "read_bytes", side_effect=AssertionError("file read")), patch.object(
            Path, "mkdir", side_effect=AssertionError("mkdir")
        ), patch.object(
            subprocess, "Popen", side_effect=AssertionError("process")
        ):
            report = analysis.analyze_convergence(BOOK, records, trusted)
        self.assertEqual((BOOK, records, trusted), before)
        self.assertEqual(report["tooling_status"], "complete")
        # Outputs do not alias the supplied histories.
        next(iter(report["diagnostics"].values()))["accepted_steps"][0]["time"] = -1
        self.assertEqual((BOOK, records, trusted), before)


if __name__ == "__main__":
    unittest.main()

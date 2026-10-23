"""Synthetic and authenticated read-only bridge tests. Never launch a process."""

import copy
import json
import math
import shutil
import subprocess
import tempfile
import unittest
from fractions import Fraction
from pathlib import Path
from unittest.mock import Mock, patch

from tools import solute_benchmark_bridge as bridge

CASES = bridge.ROOT / "reference/solute_verification/CASES.json"
BOOK = bridge.lifecycle.contract.validate(bridge.lifecycle.contract.load_cases(CASES))


def new_rust(old):
    """Independent schema translation for tests, not another transport calculation."""
    profiles = []
    for record in old["observations"]:
        profiles.append(
            {
                "time": record["time_h"],
                "concentration": [list(reversed(s["concentration"])) for s in record["species"]],
                "theta": [0.43] * 1001,
                "q_faces": [0.43] * 1002,
            }
        )
    reported = {"inventory": None, "balance": None}
    for field, name in (
        ("boundary_cumulative", "cumulative_boundary_mass"),
        ("reaction_cumulative", "cumulative_reaction_mass"),
    ):
        reported[field] = [
            {"time": p["time_h"], "values": [s[name] for s in p["species"]]}
            for p in old["observations"]
        ]
    return {
        "solver": "rust",
        "species": ["NH4", "NO2", "NO3"],
        "coordinates": [i / 4 for i in range(1001)],
        "profiles": profiles,
        "steps": [{"index": i, "time": i * 0.1, "dt": 0.1} for i in range(1, 2001)],
        "solver_reported": reported,
    }


class BridgeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.old_rust = bridge.load_retained_rust()
        cls.new_rust = new_rust(cls.old_rust)
        cls.case, cls.fortran_row = bridge._configuration(BOOK, "fortran")
        cls.old_fortran = bridge._fortran_records(
            bridge.ROOT / bridge.OLD_RUN, cls.case, cls.fortran_row
        )
        cls.certificate = bridge.arithmetic_certificate()

    def setUp(self):
        root = bridge.ROOT / ".development/13.3.7"
        root.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=root)
        self.addCleanup(self.temp.cleanup)
        self.scratch = Path(self.temp.name)
        no_process = patch.object(subprocess, "Popen", side_effect=AssertionError("no subprocess"))
        no_process.start()
        self.addCleanup(no_process.stop)

    def test_retained_authentication_and_mapping(self):
        for solver in ("fortran", "rust"):
            hashes = bridge.retained_artifacts(solver)
            self.assertEqual(hashes[bridge.BASELINE], bridge.BASELINE_HASH)
        self.assertEqual(len(self.old_fortran["steps"]), 2000)
        self.assertEqual(len(self.old_fortran["profiles"]), 4)
        self.assertIsNone(self.old_fortran["endpoint_history"])

    def test_baseline_tamper_rejected(self):
        target = self.scratch / bridge.BASELINE
        target.parent.mkdir(parents=True)
        target.write_text("{}\n")
        with patch.object(bridge, "ROOT", self.scratch), self.assertRaisesRegex(ValueError, "hash"):
            bridge.retained_artifacts("rust")

    def test_reviewed_source_hash_is_pinned(self):
        hashes = dict(bridge.POLICY_HASHES)
        hashes["crates/hydrus-core/src/solute.rs"] = "0" * 64
        with patch.object(bridge, "POLICY_HASHES", hashes), self.assertRaisesRegex(
            ValueError, "hash"
        ):
            bridge.retained_artifacts("rust")

    def test_case_boundaries_species_rates_are_fixed(self):
        for field, value in (
            ("boundary", {"inlet": "concentration"}),
            ("q", -0.43),
            ("theta", 0.44),
        ):
            book = copy.deepcopy(BOOK)
            next(c for c in book["cases"] if c["id"] == "example3")[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                bridge._configuration(book, "rust")
        book = copy.deepcopy(BOOK)
        case = next(c for c in book["cases"] if c["id"] == "example3")
        case["species"][0]["rate"] = 0.006
        with self.assertRaisesRegex(ValueError, "reaction"):
            bridge._configuration(book, "rust")

    def test_retained_actual_time_recurrence(self):
        old = copy.deepcopy(self.old_rust)
        old["observations"][1]["actual_time_h"] = 50.0
        with patch.object(bridge.legacy, "parse_rust_observations", return_value=old):
            with self.assertRaisesRegex(ValueError, "recurrence"):
                bridge.load_retained_rust()

    def test_outward_interval_and_underflow(self):
        third = bridge._Interval(1) / bridge._Interval(3)
        self.assertLessEqual(Fraction(third.lo), Fraction(1, 3))
        self.assertGreaterEqual(Fraction(third.hi), Fraction(1, 3))
        value = bridge._Interval(0.43) + bridge._Interval(0.8) * bridge._Interval(0.5375)
        exact = Fraction(0.43) + Fraction(0.8) * Fraction(0.5375)
        self.assertLessEqual(Fraction(value.lo), exact)
        self.assertGreaterEqual(Fraction(value.hi), exact)
        self.assertGreater(self.certificate["underflow_per_step"], 0)
        with self.assertRaises(ValueError):
            bridge._Interval(1) / bridge._Interval(0)

    def test_certificate_counts_conditioning_and_envelopes(self):
        c = self.certificate
        self.assertEqual(sum(c["operation_partitions"].values()), 128)
        self.assertEqual(c["operations_per_species_step"], 128 * 1001 + 32)
        self.assertGreater(min(c["row_margin_lower"]), 0.53)
        self.assertLess(max(c["row_norm_upper"]), 4.3)
        self.assertEqual(c["bounds"]["0"]["concentration"], [0, 0, 0])
        self.assertFalse(c["scientific_tolerance"])
        for species in range(3):
            errors = [c["bounds"][t]["concentration"][species] for t in ("0", "50", "100", "200")]
            self.assertEqual(errors, sorted(errors))
            self.assertLess(errors[-1], 1e-5)
        self.assertEqual(c["serialization_absolute_error"], 0)

    def test_rust_identity_preserves_unavailable_fields(self):
        result = bridge.compare_rust(self.old_rust, self.new_rust)
        self.assertEqual(result["status"], "passed")
        self.assertIsNone(result["availability"]["retained_endpoint_history"])
        self.assertIsNone(result["availability"]["retained_executable_identity"])
        self.assertIn("inventory", result["availability"]["derived_new_fields"])

    def test_rust_inside_and_outside_arithmetic_bound(self):
        bound = self.certificate["bounds"]["50"]["concentration"][0]
        for factor, expected in ((0.1, "passed"), (10, "failed")):
            new = copy.deepcopy(self.new_rust)
            new["profiles"][1]["concentration"][0][1] += bound * factor
            with self.subTest(factor=factor):
                self.assertEqual(bridge.compare_rust(self.old_rust, new)["status"], expected)

    def test_changed_species_and_cancelling_components_fail(self):
        new = copy.deepcopy(self.new_rust)
        new["profiles"][1]["concentration"][1][100] += 0.001
        self.assertEqual(bridge.compare_rust(self.old_rust, new)["status"], "failed")
        new = copy.deepcopy(self.new_rust)
        new["solver_reported"]["boundary_cumulative"][2]["values"][0] += 0.1
        new["solver_reported"]["reaction_cumulative"][2]["values"][0] -= 0.1
        result = bridge.compare_rust(self.old_rust, new)
        self.assertEqual(result["status"], "failed")
        self.assertGreater(result["groups"]["100/NH4/boundary"]["failures"], 0)
        self.assertGreater(result["groups"]["100/NH4/reaction"]["failures"], 0)

    def test_rust_order_orientation_schedule_and_coverage_rejected(self):
        mutations = [
            lambda n: n["species"].reverse(),
            lambda n: n["coordinates"].reverse(),
            lambda n: n["profiles"][1]["q_faces"].__setitem__(0, -0.43),
            lambda n: n["profiles"][1].__setitem__("time", 51),
            lambda n: n["steps"].pop(),
            lambda n: n["steps"][3].__setitem__("time", 0.5),
            lambda n: n["solver_reported"]["boundary_cumulative"].pop(),
            lambda n: n["solver_reported"].__setitem__("inventory", []),
        ]
        for i, mutate in enumerate(mutations):
            new = copy.deepcopy(self.new_rust)
            mutate(new)
            with self.subTest(i=i), self.assertRaises(ValueError):
                bridge.compare_rust(self.old_rust, new)
        old = copy.deepcopy(self.old_rust)
        old["observations"][1]["species"].reverse()
        with self.assertRaisesRegex(ValueError, "species"):
            bridge.compare_rust(old, self.new_rust)

    def test_comparison_exact_boundary_and_nonfinite(self):
        c = bridge._Comparison()
        c.number("x", "x", 0, 1, 1)
        c.number("x", "x", 0, math.nextafter(1, math.inf), 1)
        self.assertEqual(c.result()["failure_count"], 1)
        with self.assertRaises(ValueError):
            c.number("x", "x", 0, math.nan, 1)

    def test_fortran_retained_self_comparison(self):
        result = bridge.compare_fortran(self.old_fortran, self.old_fortran)
        self.assertEqual(result["status"], "passed")
        self.assertGreater(result["groups"]["NOD_INF.OUT"]["count"], 60000)
        self.assertIsNone(result["availability"]["new_endpoint_history"])

    def _changed_raw(self, name, transform):
        path = self.scratch / name
        path.write_text(transform(self.old_fortran["paths"][name].read_text()))
        result = dict(self.old_fortran)
        result["paths"] = dict(self.old_fortran["paths"], **{name: path})
        return result

    def test_fortran_changes_values_components_and_precision(self):
        new = self._changed_raw("NOD_INF.OUT", lambda s: s.replace("0.0000E+00", "0.1000E+00", 1))
        self.assertEqual(bridge.compare_fortran(self.old_fortran, new)["status"], "failed")
        new = self._changed_raw("solute1.out", lambda s: s.replace("0.43000E+00", "0.44000E+00", 1))
        self.assertEqual(bridge.compare_fortran(self.old_fortran, new)["status"], "failed")
        new = self._changed_raw("NOD_INF.OUT", lambda s: s.replace("0.0000E+00", "0.000E+00", 1))
        with self.assertRaisesRegex(ValueError, "precision"):
            bridge.compare_fortran(self.old_fortran, new)

    def test_fortran_missing_records_signs_and_null_endpoint(self):
        new = self._changed_raw("NOD_INF.OUT", lambda s: "\n".join(s.splitlines()[:-3]) + "\n")
        with self.assertRaisesRegex(ValueError, "completeness"):
            bridge.compare_fortran(self.old_fortran, new)
        new = self._changed_raw(
            "solute1.out", lambda s: s.replace("positive into", "positive out of")
        )
        with self.assertRaises(ValueError):
            bridge.compare_fortran(self.old_fortran, new)
        new = dict(self.old_fortran, endpoint_history=[])
        with self.assertRaisesRegex(ValueError, "unavailable"):
            bridge.compare_fortran(self.old_fortran, new)

    def test_fortran_metadata_only_exclusions(self):
        new = self._changed_raw(
            "BALANCE.OUT",
            lambda s: s.replace("0:21:25", "1:22:26").replace(
                "Calculation time [sec]", "Calculation time [sec]"
            ),
        )
        self.assertEqual(bridge.compare_fortran(self.old_fortran, new)["status"], "passed")
        self.assertEqual(bridge._token_interval("0.4300E+00")[1], 0.00005)

    def test_strict_fortran_reader_rejects_orientation_schedule_and_truncation(self):
        run = self.scratch / "raw-copy"
        shutil.copytree(bridge.ROOT / bridge.OLD_RUN, run)
        for name, column, replacement in (
            ("NOD_INF.OUT", 1, "-999.0000"),
            ("RUN_INF.OUT", 1, "0.2000000E+00"),
        ):
            path = run / ("solute-input\\" + name)
            original = path.read_text()
            lines = original.splitlines()
            for i, line in enumerate(lines):
                tokens = line.split()
                if tokens and bridge.metrics.NUMBER.fullmatch(tokens[0]):
                    tokens[column] = replacement
                    lines[i] = " ".join(tokens)
                    break
            path.write_text("\n".join(lines) + "\n")
            with self.subTest(name=name), self.assertRaises(ValueError):
                bridge._fortran_records(run, self.case, self.fortran_row)
            path.write_text(original)
        path = run / "solute-input\\solute3.out"
        path.write_text("\n".join(path.read_text().splitlines()[:-3]) + "\n")
        with self.assertRaises(ValueError):
            bridge._fortran_records(run, self.case, self.fortran_row)

    def test_printed_interval_edges(self):
        old_path, new_path = self.scratch / "old", self.scratch / "new"
        old_path.write_text("0.4300E+00\n")
        old = {"paths": {"numeric": old_path}, "endpoint_history": None}
        new = {"paths": {"numeric": new_path}, "endpoint_history": None}
        for token, status in (("0.4301E+00", "passed"), ("0.4302E+00", "failed")):
            new_path.write_text(token + "\n")
            with self.subTest(token=token):
                self.assertEqual(bridge.compare_fortran(old, new)["status"], status)

    def _mock_evidence(self):
        run = self.scratch / "new-run"
        run.mkdir()
        (run / "manifest.json").write_text("{}\n")
        (run / "observations.json").write_text("{}\n")
        manifest = {"solver": "rust", "launch_id": bridge.BRIDGES["rust"], "artifacts": {}}
        manager = Mock(book=BOOK)
        manager.verify.return_value = manifest
        return run, manager

    def test_evaluate_uses_complete_lifecycle_and_retains_artifact_bindings(self):
        run, manager = self._mock_evidence()
        with patch.object(bridge.lifecycle, "Lifecycle", return_value=manager), patch.object(
            bridge.metrics, "load_observations", return_value=self.new_rust
        ):
            report = bridge.evaluate_bridge(CASES, run)
        manager.verify.assert_called_once_with(run, require_complete=True)
        self.assertEqual(report["comparison"]["status"], "passed")
        self.assertIn(
            str(run.relative_to(bridge.ROOT) / "observations.json"), report["new_artifacts"]
        )
        self.assertIn(bridge.OLD_RUST, report["retained_artifacts"])
        self.assertIn("tools/solute_benchmark_bridge.py", report["comparator_provenance"])

    def test_report_roundtrip_tampering_and_overwrite(self):
        run, manager = self._mock_evidence()
        with patch.object(bridge.lifecycle, "Lifecycle", return_value=manager), patch.object(
            bridge.metrics, "load_observations", return_value=self.new_rust
        ):
            path = bridge.write_report(CASES, run, self.scratch / "report")
            result = bridge.verify_report(path, CASES)
            self.assertEqual(result["comparison"]["status"], "passed")
            with self.assertRaisesRegex(ValueError, "already exists"):
                bridge.write_report(CASES, run, self.scratch / "report")
            (run / "observations.json").write_text('{"changed":true}\n')
            with self.assertRaisesRegex(ValueError, "stale"):
                bridge.verify_report(path, CASES)
            report = json.loads(path.read_text())
            report["payload"]["comparison"]["status"] = "forged"
            path.write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError, "digest"):
                bridge.verify_report(path, CASES)
            report["payload_sha256"] = bridge.fortran.canonical_hash(report["payload"])
            path.write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError, "stale"):
                bridge.verify_report(path, CASES)

    def test_failed_comparison_is_persisted_and_blocks_gate(self):
        run, manager = self._mock_evidence()
        new = copy.deepcopy(self.new_rust)
        new["profiles"][1]["concentration"][0][1] += 1
        with patch.object(bridge.lifecycle, "Lifecycle", return_value=manager), patch.object(
            bridge.metrics, "load_observations", return_value=new
        ):
            path = bridge.write_report(CASES, run, self.scratch / "failed")
            report = bridge.verify_report(path, CASES)
            self.assertEqual(report["comparison"]["status"], "failed")
        passed = {"launch_id": bridge.BRIDGES["fortran"], "comparison": {"status": "passed"}}
        with patch.object(bridge, "verify_report", side_effect=[report, passed]):
            with self.assertRaisesRegex(ValueError, "failed"):
                bridge.require_bridges([path, path], CASES)

    def test_gate_needs_two_distinct_authenticated_bridges(self):
        with self.assertRaisesRegex(ValueError, "both"):
            bridge.require_bridges([], CASES)
        reports = [
            {"launch_id": bridge.BRIDGES[s], "comparison": {"status": "passed"}}
            for s in ("rust", "fortran")
        ]
        paths = [self.scratch / "r.json", self.scratch / "f.json"]
        for path in paths:
            path.write_text("{}\n")
        with patch.object(bridge, "verify_report", side_effect=reports):
            self.assertEqual(bridge.require_bridges(paths, CASES)["status"], "passed")
        with patch.object(bridge, "verify_report", return_value=reports[0]):
            with self.assertRaisesRegex(ValueError, "each exact"):
                bridge.require_bridges(paths, CASES)

    def test_report_path_escape_and_symlinks(self):
        with self.assertRaises(ValueError):
            bridge._report_path(bridge.ROOT / "reference/report.json")
        path = self.scratch / "link"
        path.symlink_to(self.scratch, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            bridge._report_path(path / "report.json")


V2_CASES = bridge.ROOT / "reference/solute_verification/CASES_V2.json"


class V2GateTests(unittest.TestCase):
    def test_v2_gate_receipt_records_reason(self):
        self.assertTrue(bridge.is_v2_casebook(V2_CASES))
        self.assertFalse(bridge.is_v2_casebook(CASES))
        receipt = bridge.v2_gate_receipt(V2_CASES)
        self.assertEqual(receipt["status"], "not_applicable_v2")
        self.assertEqual(receipt["cases_sha256"], bridge.sha256(V2_CASES))
        self.assertIn("44/44 gate (21.4.1)", receipt["reason"])
        self.assertIn("22.3.5", receipt["reason"])
        self.assertIs(receipt["scientific_accuracy_accepted"], False)

    def test_v1_still_requires_both_bridge_reports(self):
        with self.assertRaisesRegex(ValueError, "both bridge reports are required"):
            bridge.require_bridges([], CASES)


if __name__ == "__main__":
    unittest.main()

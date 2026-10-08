"""Tests for the source-only parity orchestration boundary."""

import hashlib
import json
import tempfile
import types
import unittest
from pathlib import Path
from unittest.mock import patch

from tools import run_solute_parity as parity


def digest(data):
    return hashlib.sha256(data).hexdigest()


class ParityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.fixture_dir = self.root / "fixture"
        self.fixture_dir.mkdir()
        raw = b"immutable normalized fixture input\n"
        (self.fixture_dir / "input.json").write_bytes(raw)
        self.location = {"time": 1, "species": "A", "node": 2}
        request = {
            "schema_version": "solute-normalized-request-v1",
            "fields": {
                "concentration": {"units": "C_ref", "value": 0.25},
                "status": {"units": "state", "value": 1},
            },
        }
        self.fixture = {
            "schema_version": "solute-parity-fixture-v1",
            "fixture_id": "test-fixture",
            "request": request,
            "request_sha256": parity.canonical_hash(request),
            "input_files": {"input.json": digest(raw)},
        }
        self.manifest_path = self.fixture_dir / "fixture.json"
        self.write_fixture()
        self.calls = 0

    def write_fixture(self):
        self.manifest_path.write_text(json.dumps(self.fixture), encoding="utf-8")

    def runner_result(self, task_root, **changes):
        output_data = b"raw authenticated Fortran output"
        run_root = task_root / "run"
        output_path = task_root / "outputs" / "solute1.out"
        run_root.mkdir(parents=True)
        output_path.parent.mkdir(parents=True)
        output_path.write_bytes(output_data)
        result = {
            "complete": True,
            "task_root": str(task_root),
            "run_root": str(run_root),
            "input_sha256": self.fixture["input_files"],
            "output_paths": {"solute-output": "outputs/solute1.out"},
            "output_sha256": {"solute-output": digest(output_data)},
            "source_and_archive_sha256": json.loads(
                (parity.ROOT / "planning/master/SOURCE_MANIFEST.json").read_text(encoding="utf-8")
            ),
            "patch_sha256": parity.PINNED_PATCH_SHA256,
            "executable_sha256": "a" * 64,
        }
        result.update(changes)
        return result

    def call_reference_only(self, output):
        called = []

        def run_fixture(path, task_root, timeout_seconds=None):
            self.assertFalse(task_root.exists())
            called.append((path, task_root, timeout_seconds))
            return output(task_root)

        module = types.ModuleType("tools.run_fortran_reference")
        module.run_fixture = run_fixture
        self.calls += 1
        with patch.dict("sys.modules", {"tools.run_fortran_reference": module}):
            result = parity.run_reference_only(self.manifest_path, self.root / f"task-{self.calls}")
        return result, called

    def write_group_fixture(self, directory, fixture_id):
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "input.json").write_bytes((self.fixture_dir / "input.json").read_bytes())
        fixture = {**self.fixture, "fixture_id": fixture_id}
        path = directory / "CASE.json"
        path.write_text(json.dumps(fixture), encoding="utf-8")
        return path

    def test_group_discovery_is_recursive_and_sorted(self):
        fixture_root = self.root / "parity"
        first = self.write_group_fixture(fixture_root / "transport" / "z-case", "z-case")
        second = self.write_group_fixture(fixture_root / "transport" / "a-case", "a-case")
        other = self.write_group_fixture(fixture_root / "reactions" / "only", "other")
        self.assertEqual(parity.discover_fixture_manifests("transport", fixture_root), [second, first])
        self.assertEqual(parity.discover_fixture_manifests("all", fixture_root), [other, second, first])

    def test_group_runs_serially_into_unique_safe_child_roots_and_aggregates_authenticated_results(self):
        fixture_root = self.root / "parity"
        first = self.write_group_fixture(fixture_root / "transport" / "a", "case-a")
        second = self.write_group_fixture(fixture_root / "transport" / "b", "case-b")
        calls = []

        def run_fixture(path, task_root, timeout_seconds=None):
            calls.append((path, task_root, timeout_seconds))
            return self.runner_result(task_root)

        module = types.ModuleType("tools.run_fortran_reference")
        module.run_fixture = run_fixture
        task_root = self.root / "group-task"
        with patch.dict("sys.modules", {"tools.run_fortran_reference": module}):
            result = parity.run_fixture_group("transport", task_root, fixture_root)

        self.assertEqual([call[0] for call in calls], [first, second])
        roots = [call[1] for call in calls]
        self.assertEqual(len(set(roots)), 2)
        child_roots = [root.parent for root in roots]
        self.assertTrue(all(root.parent == task_root for root in child_roots))
        self.assertEqual([root.name for root in child_roots], ["0000-case-a", "0001-case-b"])
        self.assertEqual([root.name for root in roots], ["reference-run", "reference-run"])
        self.assertTrue(all(call[2] is None for call in calls))
        self.assertEqual([item["status"] for item in result["results"]], ["authenticated", "authenticated"])
        self.assertEqual(result["result_kind"], "raw-fortran-outputs")

    def test_duplicate_or_unsafe_group_fixture_ids_fail_before_runner_invocation(self):
        fixture_root = self.root / "parity"
        self.write_group_fixture(fixture_root / "transport" / "a", "same")
        self.write_group_fixture(fixture_root / "transport" / "b", "same")
        runner = types.ModuleType("tools.run_fortran_reference")
        runner.run_fixture = unittest.mock.Mock(side_effect=AssertionError("runner must not launch"))
        task_root = self.root / "invalid-group"
        with patch.dict("sys.modules", {"tools.run_fortran_reference": runner}):
            with self.assertRaisesRegex(ValueError, "duplicate fixture_id"):
                parity.run_fixture_group("transport", task_root, fixture_root)
        self.assertFalse(task_root.exists())
        runner.run_fixture.assert_not_called()

        (fixture_root / "transport" / "b" / "CASE.json").unlink()
        self.write_group_fixture(fixture_root / "transport" / "b", "../escape")
        with patch.dict("sys.modules", {"tools.run_fortran_reference": runner}):
            with self.assertRaisesRegex(ValueError, "unsafe fixture_id"):
                parity.run_fixture_group("transport", task_root, fixture_root)
        self.assertFalse(task_root.exists())
        runner.run_fixture.assert_not_called()

    def test_reference_only_invokes_shared_api_with_unique_isolated_roots(self):
        first, called_first = self.call_reference_only(self.runner_result)
        second, called_second = self.call_reference_only(self.runner_result)
        self.assertEqual(first["status"], "authenticated")
        self.assertIsNone(first["comparison"])
        self.assertEqual(first["result_kind"], "raw-fortran-outputs")
        self.assertFalse(first["normalized_parity"])
        self.assertEqual(first["output_paths"], {"solute-output": "outputs/solute1.out"})
        self.assertEqual(first["output_sha256"], {"solute-output": digest(b"raw authenticated Fortran output")})
        self.assertEqual(called_first[0][2], None)
        outer_first = Path(first["task_root"])
        outer_second = Path(second["task_root"])
        runner_root_first = called_first[0][1]
        runner_root_second = called_second[0][1]
        self.assertTrue(outer_first.is_relative_to(self.root))
        self.assertTrue(outer_second.is_relative_to(self.root))
        self.assertNotEqual(outer_first, outer_second)
        self.assertEqual(runner_root_first, outer_first / "reference-run")
        self.assertEqual(runner_root_second, outer_second / "reference-run")
        self.assertNotEqual(runner_root_first, runner_root_second)
        self.assertEqual(Path(first["run_root"]), runner_root_first / "run")
        self.assertTrue(Path(second["run_root"]).is_relative_to(runner_root_second))
        actual_output = called_first[0][1] / first["output_paths"]["solute-output"]
        self.assertEqual(actual_output.read_bytes(), b"raw authenticated Fortran output")
        self.assertEqual(digest(actual_output.read_bytes()), first["output_sha256"]["solute-output"])

    def test_input_hash_tampering_rejected(self):
        (self.fixture_dir / "input.json").write_bytes(b"tampered")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            parity.validate_fixture(self.manifest_path)

    def test_field_identity_mismatch_rejected(self):
        ref = [{"field": "concentration", "location": self.location, "units": "C_ref", "value": 0.25}]
        candidate = json.loads(json.dumps(ref))
        candidate[0]["location"]["node"] = 3
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            parity.compare_observations(ref, candidate, {"concentration": {"atol": 0, "rtol": 0},
                                                          "status": {"atol": 0, "rtol": 0}})

    def test_exact_and_tolerance_comparisons(self):
        reference = [{"field": "concentration", "location": self.location, "units": "C_ref", "value": 1.0}]
        rule = {"concentration": {"atol": 0, "rtol": 0}}
        self.assertEqual(parity.compare_observations(reference, reference, rule)["status"], "pass")
        within = [{**reference[0], "value": 1.05}]
        self.assertEqual(parity.compare_observations(reference, within,
                                                      {"concentration": {"atol": 0.1, "rtol": 0}})["status"], "pass")
        outside = [{**reference[0], "value": 1.2}]
        result = parity.compare_observations(reference, outside,
                                             {"concentration": {"atol": 0.1, "rtol": 0}})
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["first_divergence"]["field"], "concentration")

    def test_paired_precision_intervals(self):
        reference = [{"field": "concentration", "location": self.location, "units": "C_ref", "value": 1.0}]
        overlap = parity.compare_observations(reference, reference,
                                              {"concentration": {"intervals": [[0.9, 1.1], [1.0, 1.2]]}})
        disjoint = parity.compare_observations(reference, reference,
                                               {"concentration": {"intervals": [[0.9, 1.0], [1.1, 1.2]]}})
        self.assertEqual(overlap["status"], "pass")
        self.assertEqual(disjoint["status"], "fail")

    def test_runner_hashes_completion_and_output_containment(self):
        task_root = self.root / "raw-run"
        good = self.runner_result(task_root)
        self.assertIs(parity._validate_raw_result(good, task_root, self.fixture), good)

        invalid = json.loads(json.dumps(good))
        invalid["complete"] = False
        with self.assertRaisesRegex(ValueError, "incomplete"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        invalid = json.loads(json.dumps(good))
        invalid["input_sha256"]["input.json"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "input identity/hash mismatch"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        invalid = json.loads(json.dumps(good))
        invalid["source_and_archive_sha256"]["src_reference/SOLUTE.FOR"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "source/archive hashes"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        invalid = json.loads(json.dumps(good))
        invalid["patch_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "patch hash"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        invalid = json.loads(json.dumps(good))
        invalid["executable_sha256"] = "bad"
        with self.assertRaisesRegex(ValueError, "executable SHA-256"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        invalid = json.loads(json.dumps(good))
        invalid["output_sha256"]["solute-output"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "output hash mismatch"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        invalid = json.loads(json.dumps(good))
        invalid["output_paths"] = {"missing": "outputs/missing.out"}
        invalid["output_sha256"] = {"missing": "b" * 64}
        with self.assertRaisesRegex(ValueError, "declared output missing"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        invalid = json.loads(json.dumps(good))
        invalid["output_paths"] = {"outside": "../outside.out"}
        invalid["output_sha256"] = {"outside": "0" * 64}
        with self.assertRaisesRegex(ValueError, "escapes task root"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        invalid = json.loads(json.dumps(good))
        invalid["output_paths"]["other-output"] = "outputs/solute1.out"
        with self.assertRaisesRegex(ValueError, "logical names differ"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        invalid = json.loads(json.dumps(good))
        invalid["task_root"] = str(self.root / "elsewhere")
        with self.assertRaisesRegex(ValueError, "task_root differs"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        invalid = json.loads(json.dumps(good))
        invalid["run_root"] = str(self.root / "outside-run")
        with self.assertRaisesRegex(ValueError, "run_root escapes"):
            parity._validate_raw_result(invalid, task_root, self.fixture)

        (task_root / "outputs/solute1.out").write_bytes(b"tampered output bytes")
        with self.assertRaisesRegex(ValueError, "output hash mismatch"):
            parity._validate_raw_result(good, task_root, self.fixture)


if __name__ == "__main__":
    unittest.main()

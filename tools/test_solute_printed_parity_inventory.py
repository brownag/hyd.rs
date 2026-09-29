"""Focused tests for retained printed-parity artifact inventory."""

import json
import tempfile
import unittest
from pathlib import Path

from tools import solute_printed_parity_inventory as inventory


class PrintedParityInventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = inventory.build_inventory()

    def test_exactly_six_canonical_case_pairs(self):
        self.assertEqual(
            self.data["executable_case_ids"],
            ["reaction_distinct", "reaction_equal", "reaction_zero", "diffusion_cosine", "scalar_robin", "example3"],
        )
        self.assertEqual(self.data["canonical_pair_count"], 6)
        self.assertEqual(len(self.data["canonical_pairs"]), 6)
        self.assertEqual(
            {pair["case_id"] for pair in self.data["canonical_pairs"]},
            set(self.data["executable_case_ids"]),
        )

    def test_canonical_pairs_are_hash_authenticated_and_pinned(self):
        by_case = {pair["case_id"]: pair for pair in self.data["canonical_pairs"]}
        for case_id, pair in by_case.items():
            self.assertEqual(pair["fortran"]["mapping_validation"], "passed")
            self.assertEqual(pair["rust"]["mapping_validation"], "passed")
            self.assertEqual(pair["fortran"]["nodes"], pair["rust"]["nodes"])
            self.assertEqual(pair["fortran"]["species_count"], pair["rust"]["species_count"])
            self.assertEqual(pair["fortran"]["step_count"], pair["rust"]["step_count"])
            for side in ("fortran", "rust"):
                run = pair[side]
                self.assertEqual(len(run["manifest_sha256"]), 64)
                self.assertEqual(len(run["request"]["sha256"]), 64)
                self.assertGreater(run["artifact_count"], 0)
                self.assertTrue(all(len(item["sha256"]) == 64 for item in run["artifacts"]))
            expected_batch = (
                "batch-c96c2d089400481a913a4accb6f1650f"
                if case_id == "example3"
                else "batch-043cf43e84c04448a38e452670278cba"
            )
            self.assertTrue(pair["batch_path"].endswith(expected_batch))

    def test_supplemental_records_are_separate_and_failed_raw_runs_excluded(self):
        supplemental = self.data["supplemental"]
        self.assertGreater(supplemental["run_count"], 0)
        self.assertEqual(supplemental["run_count"], len(supplemental["runs"]))
        self.assertTrue(all(run["classification"] == "supplemental_noncanonical" for run in supplemental["runs"] if run["status"] != "failed"))
        failed = [run for run in supplemental["runs"] if run["status"] == "failed"]
        self.assertEqual(len(failed), 3)
        self.assertTrue(all(run["classification"] == "failed_noncanonical_excluded" for run in failed))
        self.assertEqual(
            {item["failure_class"] for item in self.data["validation_failures"]},
            set(inventory.FAILURES),
        )
        self.assertTrue(all(item["status"] == "failed" for item in self.data["validation_failures"]))
        endpoint = next(item for item in self.data["validation_failures"] if item["failure_class"].startswith("endpoint"))
        self.assertEqual(len(endpoint["manifest_records"]), 2)
        self.assertTrue(all(record["status"] == "failed" for record in endpoint["manifest_records"]))

    def test_plan_batch_id_typo_is_resolved_by_evidence(self):
        resolution = self.data["batch_id_reconciliation"]
        self.assertEqual(resolution["plan_spelling"], "batch-043cf43e84c04448a38e452670278c4")
        self.assertEqual(resolution["authenticated_spelling"], "batch-043cf43e84c04448a38e452670278cba")
        self.assertEqual(len(resolution["basis"]), 2)
        self.assertTrue(all(len(item["sha256"]) == 64 for item in resolution["basis"]))

    def test_manifest_artifact_hash_mismatch_fails_closed(self):
        book = inventory.contract.validate(inventory.contract.load_cases(inventory.CASES_PATH))
        case = next(case for case in book["cases"] if case["id"] == "reaction_zero")
        row = next(
            row for row in book["launches"]
            if row["case_id"] == "reaction_zero" and row["solver"] == "rust"
            and row["phase"] == "benchmark"
        )
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp) / "run-fixture"
            run_dir.mkdir()
            request = inventory.fortran.request_bytes(case, row, run_dir.name, "a" * 64)
            (run_dir / "request.txt").write_bytes(request)
            payload = run_dir / "observations.json"
            payload.write_text("{}", encoding="utf-8")
            manifest = {
                "schema_version": "solute-manifest-v1",
                "run_id": run_dir.name,
                "launch_id": row["id"],
                "case_id": case["id"],
                "solver": row["solver"],
                "precision": row["precision"],
                "timeout_seconds": row["timeout_seconds"],
                "cases_sha256": "a" * 64,
                "case_sha256": inventory.canonical_hash(case),
                "status": "complete",
                "request_sha256": inventory.sha256(run_dir / "request.txt"),
                "artifacts": {
                    "observations.json": {
                        "path": "observations.json",
                        "sha256": "0" * 64,
                        "bytes": payload.stat().st_size,
                    }
                },
                "record_inventory": {"nodes": 2, "species": 1, "step_count": 1},
                "mapping_validation": {"status": "passed"},
            }
            (run_dir / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "artifact hash/size mismatch"):
                inventory.authenticate_run(run_dir, row, case, "a" * 64)

    def test_reports_are_written_from_authenticated_inventory(self):
        inventory.write_reports(self.data)
        self.assertEqual(json.loads((inventory.ROOT / inventory.REPORT_JSON).read_text())["canonical_pair_count"], 6)
        markdown = (inventory.ROOT / inventory.REPORT_MD).read_text()
        self.assertIn("## Validation failures", markdown)
        self.assertIn("batch-043cf43e84c04448a38e452670278cba", markdown)


if __name__ == "__main__":
    unittest.main()

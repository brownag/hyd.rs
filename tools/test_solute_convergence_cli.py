"""Mocked contracts for the convergence batch CLI; no study work is started."""

import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from tools import check_solute_verification_spec as contract
from tools import solute_benchmark_lifecycle as lifecycle
from tools import solute_convergence_evidence as evidence
from tools import run_solute_convergence as cli


class HistoricalLedgerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="convergence-cli-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.book = contract.validate(contract.load_cases(
            cli.ROOT / "reference/solute_verification/CASES.json"))
        self.rows = [row for row in self.book["launches"] if row["phase"] == "convergence"]
        self.assertGreaterEqual(len(self.rows), 2)
        self.task_dir = self.root / ".development/13.4.1"
        self.task_dir.mkdir(parents=True)
        self.ledger = self.task_dir / "launch-ledger.jsonl"

    def event(self, row, run, event, detail):
        return {"event": event, "run_id": run, "launch_id": row["id"],
                "utc": "2026-09-28T12:00:00+00:00", "detail": detail}

    def start(self, row, run):
        return self.event(row, run, "start", {
            "timeout_seconds": row["timeout_seconds"], "precision": row["precision"],
            "estimated_output_bytes": row["estimated_output_bytes"],
        })

    def timeout(self, row, run):
        return self.event(row, run, "timeout", {"stage": "solver", "timeout_seconds": 60})

    @mock.patch.object(lifecycle, "PHASE_TASKS", {"13.4.1": "convergence"})
    def test_historical_prefix_counts_failed_starts_without_using_later_events(self):
        first, second = self.rows[:2]
        prefix_events = [self.start(first, "run-one"), self.timeout(first, "run-one")]
        prefix = b"".join((json.dumps(event, sort_keys=True) + "\n").encode() for event in prefix_events)
        later_events = [self.start(second, "run-two"), self.timeout(second, "run-two")]
        self.ledger.write_bytes(prefix + b"".join(
            (json.dumps(event, sort_keys=True) + "\n").encode() for event in later_events))
        payload = {
            "schema_version": "solute-convergence-ledger-receipt-v1",
            "batch": str(self.root / "batch-old"),
            "launches": {"consumed": 1, "remaining": 76, "cap": 77},
            "ledgers": [{"task": "13.4.1", "phase": "convergence",
                         "sha256": __import__("hashlib").sha256(prefix).hexdigest(),
                         "bytes": prefix.decode(), "records": prefix_events}],
        }
        receipt = self.root / "receipt.json"
        receipt.write_text(json.dumps(cli.batch_artifacts.envelope(payload)))
        with mock.patch.object(cli, "ROOT", self.root), mock.patch.object(lifecycle, "ROOT", self.root):
            self.assertEqual(cli._verify_budget_receipt(receipt, self.book), payload["launches"])

    @mock.patch.object(lifecycle, "PHASE_TASKS", {"13.4.1": "convergence"})
    def test_self_consistent_forged_count_is_rejected_against_retained_prefix(self):
        row = self.rows[0]
        real = [self.start(row, "run-one"), self.timeout(row, "run-one")]
        self.ledger.write_bytes(b"".join((json.dumps(x, sort_keys=True) + "\n").encode() for x in real))
        retained_prefix = self.ledger.read_bytes()
        payload = {
            "schema_version": "solute-convergence-ledger-receipt-v1",
            "batch": str(self.root / "batch-old"),
            "launches": {"consumed": 0, "remaining": 77, "cap": 77},
            "ledgers": [{"task": "13.4.1", "phase": "convergence",
                         "sha256": __import__("hashlib").sha256(retained_prefix).hexdigest(),
                         "bytes": retained_prefix.decode(), "records": real}],
        }
        receipt = self.root / "receipt.json"
        receipt.write_text(json.dumps(cli.batch_artifacts.envelope(payload)))
        with mock.patch.object(cli, "ROOT", self.root), mock.patch.object(lifecycle, "ROOT", self.root):
            with self.assertRaisesRegex(ValueError, "historical counts do not reproduce"):
                cli._verify_budget_receipt(receipt, self.book)


    def test_summary_count_tampering_fails_against_authenticated_receipt(self):
        summary = {"launches": {"consumed": 0, "remaining": 77, "cap": 77}}
        authenticated = {"consumed": 1, "remaining": 76, "cap": 77}
        with mock.patch.object(cli, "_verify_budget_receipt", return_value=authenticated):
            with self.assertRaisesRegex(ValueError, "differ from authenticated"):
                cli._verify_summary_budget(summary, self.root / "receipt.json", self.book,
                                           self.root / "batch")

    def test_missing_or_unmet_reference_blocks_before_reconciliation_or_start(self):
        row = self.rows[-1]
        for status in ("missing", "unmet"):
            with self.subTest(status=status):
                class FakeHelper:
                    def reference(self, *_args, **_kwargs):
                        return None, {"status": status, "identity": "a" * 64}

                instance = object.__new__(cli.ConvergenceBatch)
                instance.recipe = {"mode": "execute", "rows": [row],
                                   "allow_reference_work": False}
                instance.bridge_gate = mock.Mock()
                instance.helper = mock.Mock(return_value=FakeHelper())
                instance.reconcile = mock.Mock(side_effect=AssertionError(
                    "reconciliation/start reached before reference admission"))
                with self.assertRaisesRegex(ValueError, "exact admitted references required"):
                    instance.execute(resume=False)
                instance.reconcile.assert_not_called()

    def test_reference_helper_failure_is_not_downgraded_to_a_cache_miss(self):
        class FakeHelper:
            def reference(self, *_args, **_kwargs):
                raise ValueError("invalid present prior cache entry")

        instance = object.__new__(cli.ConvergenceBatch)
        instance.recipe = {"rows": [self.rows[-1]]}
        with self.assertRaisesRegex(ValueError, "invalid present"):
            instance.references(FakeHelper(), allow_work=False, write=False)

    def test_exact_cross_root_reference_reuse_is_read_only(self):
        row = self.rows[-1]
        class FakeHelper:
            def __init__(self):
                self.call = None
            def reference(self, selected, **options):
                self.call = (selected, options)
                return {"coordinates": [0.0]}, {"status": "admitted", "identity": "a" * 64}

        helper = FakeHelper()
        instance = object.__new__(cli.ConvergenceBatch)
        instance.recipe = {"rows": [row]}
        values, receipts = instance.references(helper, allow_work=False, write=False)
        self.assertEqual(values[row["id"]], {"coordinates": [0.0]})
        self.assertEqual(receipts[row["id"]]["status"], "admitted")
        self.assertEqual(helper.call, (row, {"allow_work": False, "write": False}))

    def test_activation_is_bound_to_exact_convergence_rows(self):
        row = self.rows[0]
        activation = {"cases_sha256": cli.batch_artifacts.sha256(
                          cli.ROOT / "reference/solute_verification/CASES.json"),
                      "phase": "convergence", "launch_ids": [row["id"]],
                      "kind": "convergence", "authorization": "reviewed matrix"}
        lifecycle.validate_activation(self.book,
            cli.ROOT / "reference/solute_verification/CASES.json", "convergence", [row], activation)
        forged = dict(activation, phase="benchmark")
        with self.assertRaisesRegex(ValueError, "activation identity mismatch"):
            lifecycle.validate_activation(self.book,
                cli.ROOT / "reference/solute_verification/CASES.json", "convergence", [row], forged)

    def test_phase_selection_rejects_wrong_phase_and_duplicate_ids(self):
        benchmark = next(row for row in self.book["launches"] if row["phase"] == "benchmark")
        with self.assertRaisesRegex(ValueError, "no matching phase rows"):
            cli._rows(self.book, [benchmark["id"]])
        with self.assertRaisesRegex(ValueError, "unknown/duplicate/wrong-phase"):
            cli._rows(self.book, [self.rows[0]["id"], self.rows[0]["id"]])

    def test_bridge_gate_requires_two_reports_and_authenticates_both(self):
        instance = object.__new__(cli.ConvergenceBatch)
        instance.cases_path = cli.ROOT / "reference/solute_verification/CASES.json"
        instance.book = self.book
        instance.path = self.root / "batch-gate"
        instance.recipe = {"rows": [self.rows[0]], "bridge_reports": ["fortran.json", "rust.json"]}
        with mock.patch.object(cli.bridge, "require_bridges", return_value={"status": "passed"}) as gate, \
             mock.patch.object(cli.batch_artifacts, "read_artifact", return_value={"status": "passed"}):
            instance.bridge_gate(write=False)
        self.assertEqual(len(gate.call_args.args[0]), 2)
        instance.recipe["bridge_reports"] = ["fortran.json"]
        with self.assertRaisesRegex(ValueError, "both bridge reports required"):
            instance.bridge_gate(write=False)

    def test_incomplete_manifest_without_start_is_not_resumable(self):
        row = self.rows[0]
        instance = object.__new__(cli.ConvergenceBatch)
        instance.path = self.root / "batch-resume"
        instance.book = self.book
        instance.cases_path = cli.ROOT / "reference/solute_verification/CASES.json"
        manifest = self.root / "run" / "manifest.json"
        manifest.parent.mkdir()
        manifest.write_text(json.dumps({"launch_id": row["id"], "status": "planned"}))
        instance.recipe = {"rows": [row], "activation": {"launch_ids": [row["id"]]}}
        with mock.patch.object(lifecycle, "all_records", return_value=[]), \
             mock.patch.object(lifecycle, "Lifecycle"):
            (instance.path / "runs").mkdir(parents=True)
            (instance.path / "runs" / "run").mkdir()
            (instance.path / "runs" / "run" / "manifest.json").write_text(manifest.read_text())
            with self.assertRaisesRegex(ValueError, "automatic retry"):
                instance.reconcile(resume=True)


class NoWorkContractTests(unittest.TestCase):
    def test_default_invocation_stays_in_mocked_dry_run_path(self):
        book = contract.validate(contract.load_cases(
            cli.ROOT / "reference/solute_verification/CASES.json"))
        root = Path(".development/13.4.3/test-default-no-work").absolute()
        root.mkdir(parents=True, exist_ok=True)
        self.addCleanup(lambda: __import__("shutil").rmtree(root, ignore_errors=True))

        class Manager:
            phase = "convergence"
            def __init__(self, *_args):
                self.book, self.root = book, root
            def dry_run(self, ids):
                return [root / "planned" / str(index) for index, _ in enumerate(ids)]

        class Batch:
            def __init__(self, _cases, _output, path, _phase=None):
                self.path, self.book = Path(path), book
            def create(self, *_args, **_kwargs):
                self.path.mkdir(parents=True)
                (self.path / "batch.lock").touch()

        with mock.patch.object(cli.lifecycle, "Lifecycle", Manager), \
             mock.patch.object(cli, "ConvergenceBatch", Batch), \
             mock.patch.object(cli.batch_artifacts, "write_artifact") as write_artifact, \
             mock.patch.object(cli.batch_artifacts, "sha256", return_value="0" * 64), \
             mock.patch.object(cli.lifecycle, "Lifecycle.execute", create=True) as execute:
            code = cli.main(["--cases", "unused-cases.json", "--output-root", str(root)])
        self.assertEqual(code, 0)
        self.assertTrue(write_artifact.called)
        execute.assert_not_called()

    def test_cli_has_only_separately_explicit_execution_flags(self):
        parsed = cli.parser().parse_args([
            "--cases", "cases.json", "--output-root", ".development/13.4.1"
        ])
        self.assertFalse(parsed.execute)
        self.assertFalse(parsed.allow_reference_work)
        self.assertFalse(parsed.allow_reference_escalation)

    def test_cli_producer_is_included_in_authenticated_helper_context(self):
        actual = evidence.provenance(include_cli=True)
        self.assertEqual(actual["tools/run_solute_convergence.py"],
                         cli.batch_artifacts.sha256(Path(cli.__file__)))
        self.assertIn("tools/solute_convergence_analysis.py", actual)
        self.assertIn("tools/solute_convergence_evidence.py", actual)

    def test_historical_count_shape_is_bound_to_frozen_cap(self):
        counts = {"consumed": 77, "remaining": 0, "cap": 77}
        self.assertEqual(cli._counts_from_snapshot(
            self._small_book(), [{"records": [{"event": "start"}] * 77}]), counts)

    @staticmethod
    def _small_book():
        return {"budgets": {"launch_cap": 77}}


V2_CASES = cli.ROOT / "reference/solute_verification/CASES_V2.json"


class V2AndStateTests(unittest.TestCase):
    def _execute_batch(self, status, tmp):
        """A batch whose lifecycle returns one run with the given manifest status."""
        book = contract.validate(contract.load_cases(
            cli.ROOT / "reference/solute_verification/CASES.json"))
        row = next(r for r in book["launches"] if r["phase"] == "convergence")
        run = Path(tmp) / "run"
        run.mkdir()
        (run / "manifest.json").write_text(json.dumps({"launch_id": row["id"], "status": status}))
        seen = {}

        class Helper:
            def derive(self, *_args, **_kw):
                return ({"derived": True}, None)
            def classify(self, rows, runs, write):
                seen["runs"] = runs
                return {"ordered_evidence": []}

        class RunManager:
            def execute(self, ids, activation):
                return [run]

        batch = object.__new__(cli.ConvergenceBatch)
        batch.path, batch.book = Path(tmp) / "batch", book
        batch.recipe = {"mode": "execute", "rows": [row], "allow_reference_work": False,
                        "activation": {"launch_ids": [row["id"]]}, "activation_sha256": "0" * 64}
        batch.bridge_gate = lambda write: None
        batch.helper = lambda: Helper()
        batch.references = lambda *_a, **_k: ({}, {row["id"]: {"status": "admitted"}})
        batch.reconcile = lambda resume: ({}, [row], RunManager())
        batch._publish = lambda *_a: ({"status": "evidence_complete"}, 0)
        return batch, row, seen

    def test_cli_accepts_raw_complete_like_lifecycle_api(self):
        for status in ("complete", "raw_complete"):
            with tempfile.TemporaryDirectory() as tmp, \
                 mock.patch.object(cli.batch_artifacts, "write_artifact"), \
                 mock.patch.object(cli.lifecycle.fortran, "canonical_hash", return_value="h"):
                batch, _row, _seen = self._execute_batch(status, tmp)
                self.assertEqual(batch.execute()[1], 0, status)
        with tempfile.TemporaryDirectory() as tmp, \
             mock.patch.object(cli.batch_artifacts, "write_artifact"), \
             mock.patch.object(cli.lifecycle.fortran, "canonical_hash", return_value="h"):
            batch, _row, _seen = self._execute_batch("failed", tmp)
            with self.assertRaisesRegex(ValueError, "cannot be retried"):
                batch.execute()

    def test_rust_classification_accepts_batch_path(self):
        # classify() takes run directories; the CLI used to hand it derived record dicts.
        with tempfile.TemporaryDirectory() as tmp, \
             mock.patch.object(cli.batch_artifacts, "write_artifact"), \
             mock.patch.object(cli.lifecycle.fortran, "canonical_hash", return_value="h"):
            batch, row, seen = self._execute_batch("complete", tmp)
            batch.execute()
        self.assertIsInstance(seen["runs"][row["id"]], Path)

    def test_v1_defaults_unchanged(self):
        v1 = {"budgets": {"launch_cap": 2}}
        starts = [{"event": "start"}]
        self.assertEqual(cli._launch_counts(v1, starts), {"consumed": 1, "remaining": 1, "cap": 2})
        with self.assertRaisesRegex(ValueError, "launch cap exceeded"):
            cli._launch_counts(v1, starts * 3)
        book = contract.validate(contract.load_cases(
            cli.ROOT / "reference/solute_verification/CASES.json"))
        self.assertEqual(set(lifecycle.book_tasks(book)), {"13.3.1", "13.4.1", "13.6.1"})
        self.assertEqual(evidence.CONVERGENCE_TASKS[0], "13.4.1")

    def test_v2_roots_per_phase(self):
        book = contract.validate(contract.load_cases(V2_CASES))
        self.assertEqual(set(lifecycle.book_tasks(book)),
                         {"22-benchmark", "22-convergence", "22-holdout", "23-classify-3", "23-controls"})
        self.assertEqual(cli._launch_counts(book, [{"event": "start"}] * 5),
                         {"consumed": 5, "remaining": None, "cap": None})
        for task, phase in (("22-benchmark", "benchmark"), ("22-convergence", "convergence"),
                            ("22-holdout", "holdout")):
            root = cli.ROOT / ".development" / task / "x"
            self.assertEqual(lifecycle.Lifecycle(V2_CASES, root).phase, phase)
        self.assertEqual(cli._phase_root(cli.ROOT / ".development/22-convergence/x", V2_CASES).phase,
                         "convergence")
        with self.assertRaisesRegex(ValueError, "execution root"):
            cli._phase_root(cli.ROOT / ".development/22-holdout/x", V2_CASES)
        self.assertIn("22-convergence", evidence.CONVERGENCE_TASKS)

    def test_holdout_rows_dry_run_only_in_v2_until_reserved_status_lifted(self):
        book = contract.validate(contract.load_cases(V2_CASES))
        rows = cli._rows(book, None, "holdout")
        self.assertTrue(all(r["status"] == "reserved_until_threshold_gate" for r in rows))
        with self.assertRaisesRegex(ValueError, "reserved until the threshold gate"):
            cli._gate_holdout(rows, thresholds=cli.ROOT / "reference/absent-thresholds.json")

        class Manager:
            phase = "holdout"
            def __init__(self, *_a):
                self.book = book
            def execute(self, *_a):
                raise AssertionError("reserved holdout row must not execute")

        activation = Path(tempfile.mkdtemp()) / "act.json"
        activation.write_text(json.dumps({"launch_ids": [r["id"] for r in rows]}))
        with mock.patch.object(cli.lifecycle, "Lifecycle", Manager):
            code = cli.main(["--cases", str(V2_CASES), "--output-root", "unused", "--holdout",
                             "--execute", "--activation-file", str(activation)])
        self.assertEqual(code, 2)


class V2BridgeGateTests(unittest.TestCase):
    V2 = cli.ROOT / "reference/solute_verification/CASES_V2.json"

    def batch(self, reports):
        instance = object.__new__(cli.ConvergenceBatch)
        instance.cases_path = self.V2
        instance.book = {}
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        instance.path = Path(tmp.name)
        instance.recipe = {"rows": [{"role": "convergence"}], "bridge_reports": reports}
        return instance

    def test_v2_convergence_executes_without_bridge_reports(self):
        instance = self.batch([])
        with mock.patch.object(cli.batch_artifacts, "write_artifact",
                               lambda t, r, b: t.write_text(json.dumps(r))), \
             mock.patch.object(cli.batch_artifacts, "read_artifact",
                               lambda t: json.loads(t.read_text())), \
             mock.patch.object(cli.bridge, "require_bridges",
                               side_effect=AssertionError("v1 gate")):
            instance.bridge_gate(write=True)
            instance.bridge_gate(write=False)
        receipt = json.loads((instance.path / "bridge-gate.json").read_text())
        self.assertEqual(receipt["status"], "not_applicable_v2")

    def test_v2_rejects_bridge_reports(self):
        with self.assertRaisesRegex(ValueError, "accept no bridge reports"):
            self.batch(["a.json", "b.json"]).bridge_gate(write=False)


class HoldoutGuardTests(unittest.TestCase):
    def setUp(self):
        book = contract.validate(contract.load_cases(V2_CASES))
        self.rows = cli._rows(book, None, "holdout")
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.sha = hashlib.sha256(V2_CASES.read_bytes()).hexdigest()

    def _thresholds(self, **overrides):
        data = {"schema_version": "solute-thresholds-v2",
                "holdout_run_permitted": True, "cases_sha256": self.sha}
        data.update(overrides)
        path = Path(self.tmp.name) / "THRESHOLDS_V2.json"
        path.write_text(json.dumps(data))
        return path

    def test_holdout_refused_when_thresholds_file_absent(self):
        absent = Path(self.tmp.name) / "absent.json"
        with self.assertRaisesRegex(ValueError, "reserved until the threshold gate.*absent"):
            cli._gate_holdout(self.rows, thresholds=absent)

    def test_holdout_refused_when_not_permitted(self):
        with self.assertRaisesRegex(ValueError, "reserved until the threshold gate.*not true"):
            cli._gate_holdout(self.rows, thresholds=self._thresholds(holdout_run_permitted=False))

    def test_holdout_refused_on_sha_mismatch(self):
        with self.assertRaisesRegex(ValueError, "reserved until the threshold gate.*cases_sha256"):
            cli._gate_holdout(self.rows, thresholds=self._thresholds(cases_sha256="0" * 64))

    def test_holdout_allowed_when_permitted_and_sha_matches(self):
        cli._gate_holdout(self.rows, thresholds=self._thresholds())


if __name__ == "__main__":
    unittest.main()

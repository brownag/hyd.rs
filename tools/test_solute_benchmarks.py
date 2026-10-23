"""CLI integration with mocked study work and independently authored small reference data."""

import contextlib
import copy
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

from tools import check_solute_benchmarks as cli
from tools import solute_analytical_reference as analytical
from tools.test_solute_reference_cache import component_fixture
from tools.test_solute_verification_metrics import RunFixture

CASES_PATH = cli.SOURCE_ROOT / "reference/solute_verification/CASES.json"
BOOK = cli.lifecycle.contract.validate(cli.lifecycle.contract.load_cases(CASES_PATH))
ROWS = [r for r in BOOK["launches"] if r["phase"] == "benchmark"]
BRIDGES = [r for r in ROWS if r["role"] == "bridge"]
ZERO = [r for r in ROWS if r["case_id"] == "reaction_zero" and r["role"] == "benchmark"]
RUST_ZERO = next(r for r in ZERO if r["solver"] == "rust")


class BenchmarkTests(unittest.TestCase):
    def setUp(self):
        scratch = cli.SOURCE_ROOT / ".development/13.3.4"
        scratch.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(self.temp.cleanup)
        self.checkout = Path(self.temp.name)
        self.root = self.checkout / ".development/13.3.1/benchmarks"
        self.events = []
        self.executions = []
        self.reference_calls = []
        self.outcome = "complete"
        self.reference_status = "admitted"
        self.bridge_status = "passed"
        test = self

        class FakeLifecycle:
            def __init__(self, cases_path, root):
                self.cases_path = Path(cases_path).absolute()
                self.root = Path(root).absolute()
                self.book = BOOK
                self.phase = "convergence" if "13.4.1" in self.root.parts else "benchmark"

            def dry_run(self, ids):
                paths = []
                for identity in ids:
                    row = next(r for r in ROWS if r["id"] == identity)
                    run = self.root / (identity + "-planned")
                    run.mkdir(parents=True)
                    (run / "manifest.json").write_text(
                        json.dumps(
                            {
                                "launch_id": identity,
                                "run_id": run.name,
                                "status": "planned",
                                "solver": row["solver"],
                            }
                        )
                    )
                    paths.append(run)
                return paths

            def execute(self, ids, activation):
                test.executions.append((copy.deepcopy(ids), copy.deepcopy(activation)))
                paths = []
                for identity in ids:
                    row = next(r for r in ROWS if r["id"] == identity)
                    run = self.root / (identity + "-completed")
                    run.mkdir(parents=True)
                    if identity == RUST_ZERO["id"] and test.outcome == "complete":
                        # This accepted synthetic fixture is checked by the actual
                        # metrics loader; only the lifecycle boundary is mocked.
                        fixture = RunFixture(run)
                        fixture.manifest["run_id"] = run.name
                        fixture.data["run_id"] = run.name
                        # Keep request and observation identities consistent.
                        request = (
                            (run / "request.txt")
                            .read_text()
                            .replace("run_id=synthetic\n", f"run_id={run.name}\n")
                        )
                        (run / "request.txt").write_text(request)
                        digest = cli.sha256(run / "request.txt")
                        fixture.manifest["request_sha256"] = digest
                        fixture.manifest["provenance"]["input_sha256"]["request.txt"] = digest
                        fixture.data["request_sha256"] = digest
                        fixture.write_observation()
                        fixture.refresh()
                    else:
                        (run / "manifest.json").write_text(
                            json.dumps(
                                {
                                    "launch_id": identity,
                                    "run_id": run.name,
                                    "status": test.outcome,
                                    "solver": row["solver"],
                                }
                            )
                        )
                        (run / "observations.json").write_text('{"mock":true}')
                    (run / "activation.json").write_text(json.dumps(activation))
                    test.events.append(
                        {"event": "start", "launch_id": identity, "run_id": run.name}
                    )
                    paths.append(run)
                    if test.outcome != "complete":
                        break
                return paths

            def verify(self, run, require_complete=False):
                data = cli.metrics.json_load(Path(run) / "manifest.json")
                if require_complete and data["status"] not in ("complete", "raw_complete"):
                    raise ValueError("incomplete mocked lifecycle evidence")
                return data

        class FakeCache:
            def __init__(self, root):
                self.root = Path(root)

            def reference(
                self,
                case,
                coordinates,
                analysis,
                diagnostic,
                *,
                allow_work=False,
                allow_escalation=False,
            ):
                test.reference_calls.append(
                    (
                        copy.deepcopy(case),
                        list(coordinates),
                        list(analysis),
                        list(diagnostic),
                        allow_work,
                        allow_escalation,
                    )
                )
                reference = component_fixture(case, coordinates, analysis, diagnostic)
                admission = analytical.reference_admission(case, reference)
                admission["status"] = test.reference_status
                identity = cli.cache.reference_identity(
                    case, coordinates, analysis, diagnostic, allow_escalation
                )
                entry = self.root / cli.cache.fingerprint(identity)
                if allow_work:
                    entry.mkdir(parents=True, exist_ok=True)
                    if not (entry / "reference.json").exists():
                        (entry / "reference.json").write_text('{"mock_reference_work":true}')
                test.assertTrue(entry.exists())
                return reference, None, admission

        self.patch(cli.lifecycle, "Lifecycle", FakeLifecycle)
        self.patch(cli.lifecycle, "all_records", lambda book: copy.deepcopy(self.events))
        self.patch(cli.lifecycle, "total_storage", lambda: 0)
        self.patch(cli.cache, "ReferenceCache", FakeCache)
        self.patch(cli.cache.platform, "platform", lambda: "mock-platform")
        self.patch(cli.bridge, "ROOT", self.checkout)
        self.patch(cli.bridge, "require_bridges", self.gate)
        self.patch(cli.bridge, "write_report", self.write_bridge)
        self.patch(cli.bridge, "verify_report", self.verify_bridge)
        self.patch(
            cli.lifecycle.subprocess, "Popen", Mock(side_effect=AssertionError("solver launch"))
        )
        self.patch(cli.lifecycle.subprocess, "run", Mock(side_effect=AssertionError("build")))
        self.gate_valid = True
        self.reports = [self.checkout / "fortran-bridge.json", self.checkout / "rust-bridge.json"]
        for path in self.reports:
            path.write_text('{"mock_prerequisite":true}')

    def patch(self, target, name, value):
        patcher = patch.object(target, name, value)
        patcher.start()
        self.addCleanup(patcher.stop)

    def gate(self, reports, cases_path):
        self.assertEqual(Path(cases_path), CASES_PATH)
        if not self.gate_valid or len(reports) != 2:
            raise ValueError("both authenticated bridges must pass")
        return {
            "status": "passed",
            "reports": {str(p): cli.sha256(p) for p in reports},
            "cases_sha256": cli.sha256(CASES_PATH),
            "scientific_accuracy_accepted": False,
        }

    def write_bridge(self, cases_path, run, root):
        root.mkdir(parents=True, exist_ok=False)
        manifest = cli.metrics.json_load(run / "manifest.json")
        payload = {
            "launch_id": manifest["launch_id"],
            "run_dir": str(run.relative_to(self.checkout)),
            "comparison": {"status": self.bridge_status},
        }
        (root / "bridge.json").write_text(json.dumps(payload))
        return root / "bridge.json"

    def verify_bridge(self, path, cases_path):
        return cli.metrics.json_load(path)

    def activation(self, rows, kind):
        path = self.checkout / (kind + "-activation.json")
        data = {
            "cases_sha256": cli.sha256(CASES_PATH),
            "phase": "benchmark",
            "kind": kind,
            "launch_ids": [r["id"] for r in rows],
            "authorization": "mock explicit approval",
        }
        path.write_text(json.dumps(data, indent=2) + "\n")
        return path

    def invoke(self, extra):
        output, error = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(error):
            code = cli.main(
                ["--cases", str(CASES_PATH), "--output-root", str(self.root), *map(str, extra)]
            )
        self.last_output, self.last_error = output.getvalue(), error.getvalue()
        return code

    def batch(self):
        return next(self.root.glob("batch-*"))

    def execute_args(self, rows, stage, work=False):
        self.authorization = self.activation(rows, stage)
        args = ["--execute", "--stage", stage, "--activation-file", self.authorization]
        if stage == "benchmark":
            args += ["--bridge-report", self.reports[0], "--bridge-report", self.reports[1]]
        if work:
            args.append("--allow-reference-work")
        return args

    def test_default_dryrun_enumerates_fourteen_and_verifies_without_work(self):
        self.assertEqual(self.invoke([]), 0, self.last_error)
        batch = self.batch()
        recipe = cli.read_artifact(batch / "recipe.json")
        self.assertEqual([r["id"] for r in recipe["rows"]], [r["id"] for r in ROWS])
        self.assertEqual(len(recipe["rows"]), 14)
        self.assertEqual(recipe["budgets"]["launch_cap"], 77)
        self.assertEqual(recipe["budgets"]["phase_caps"]["benchmark"], 14)
        before = {p: p.read_bytes() for p in batch.rglob("*") if p.is_file()}
        self.assertEqual(self.invoke(["--verify-only"]), 0, self.last_error)
        self.assertEqual(before, {p: p.read_bytes() for p in before})
        self.assertEqual(self.executions, [])
        self.assertEqual(self.reference_calls, [])
        self.assertIn('"scientific_accuracy_accepted": false', self.last_output)

    def test_forged_planned_index_rejects_duplicate_paths_and_swapped_identities(self):
        self.assertEqual(self.invoke([]), 0, self.last_error)
        path = self.batch() / "planned-runs.json"
        original = cli.read_artifact(path)
        for mutation in ("duplicate", "swap"):
            with self.subTest(mutation=mutation):
                forged = copy.deepcopy(original)
                if mutation == "duplicate":
                    for key in ("run_dir", "manifest_sha256"):
                        forged[1][key] = forged[0][key]
                else:
                    for key in ("run_dir", "manifest_sha256"):
                        forged[0][key], forged[1][key] = forged[1][key], forged[0][key]
                path.write_bytes(cli.canonical(cli.envelope(forged)) + b"\n")
                self.assertEqual(self.invoke(["--verify-only"]), 2)
                self.assertIn(
                    (
                        "duplicate planned run path"
                        if mutation == "duplicate"
                        else "planned launch identity changed"
                    ),
                    self.last_error,
                )
        self.assertEqual(self.executions, [])
        self.assertEqual(self.reference_calls, [])

    def test_bridge_execute_is_reference_free_and_requires_explicit_activation(self):
        self.assertEqual(self.invoke(["--execute", "--stage", "bridge"]), 2)
        args = self.execute_args(BRIDGES, "bridge")
        self.assertEqual(self.invoke(args), 0, self.last_error)
        self.assertEqual(len(self.executions), 1)
        self.assertEqual(self.executions[0][0], [r["id"] for r in BRIDGES])
        self.assertEqual(self.reference_calls, [])
        self.assertEqual(self.invoke(["--verify-only"]), 0, self.last_error)
        self.assertEqual(len(self.executions), 1)

    def test_dependent_gate_precedes_reference_and_solver_work(self):
        args = self.execute_args([RUST_ZERO], "benchmark", work=True)
        self.gate_valid = False
        self.assertEqual(self.invoke(args), 2)
        self.assertEqual(self.reference_calls, [])
        self.assertEqual(self.executions, [])
        self.assertFalse(self.root.exists())

    def test_missing_reference_is_scientific_unmet_and_consumes_no_launch(self):
        args = self.execute_args([RUST_ZERO], "benchmark")
        self.assertEqual(self.invoke(args), 1, self.last_error)
        self.assertEqual(self.executions, [])
        self.assertEqual(self.reference_calls, [])
        summary = cli.read_artifact(next(self.batch().glob("summary-*.json")))
        self.assertEqual(summary["status"], "reference_unmet")
        self.assertEqual(summary["references"][RUST_ZERO["id"]]["status"], "missing")
        self.assertEqual(self.invoke(["--verify-only"]), 1, self.last_error)

    def test_unmet_reference_admission_is_retained_without_solver(self):
        self.reference_status = "unmet"
        args = self.execute_args([RUST_ZERO], "benchmark", work=True)
        self.assertEqual(self.invoke(args), 1, self.last_error)
        self.assertEqual(self.executions, [])
        receipt = cli.read_artifact(next((self.batch() / "references").glob("*.json")))
        self.assertEqual(receipt["admission"]["status"], "unmet")
        self.assertEqual(self.invoke(["--verify-only"]), 1, self.last_error)
        self.assertFalse(self.reference_calls[-1][4])

    def test_exact_reference_schedule_metrics_provenance_and_readonly_reproduction(self):
        args = self.execute_args([RUST_ZERO], "benchmark", work=True)
        self.assertEqual(self.invoke(args), 0, self.last_error)
        _, coordinates, analysis, diagnostic, work, escalation = self.reference_calls[0]
        self.assertEqual(coordinates, [i * 2 / 8 for i in range(9)])
        self.assertEqual(analysis, [1, 2, 4])
        self.assertEqual(diagnostic, [i * 0.125 for i in range(33)])
        self.assertTrue(work)
        self.assertFalse(escalation)
        result = cli.read_artifact(next((self.batch() / "results").glob("*.json")))
        self.assertEqual(result["status"], "metrics_complete")
        self.assertEqual(set(result["metrics"]), {"raw", "derived", "solver_reported"})
        self.assertIsNone(result["thresholds"])
        self.assertFalse(result["scientific_accuracy_accepted"])
        self.assertEqual(self.invoke(["--verify-only"]), 0, self.last_error)
        self.assertFalse(self.reference_calls[-1][4])
        self.assertEqual(len(self.executions), 1)

    def test_paired_rows_share_only_the_full_matching_reference_identity(self):
        args = self.execute_args(ZERO, "benchmark", work=True)
        with patch.object(
            cli.BenchmarkBatch, "execute", return_value=({"status": "test_recipe"}, 0)
        ):
            self.assertEqual(self.invoke(args), 0, self.last_error)
        batch = cli.BenchmarkBatch(CASES_PATH, self.root, self.batch()).load()
        _, receipts = batch.references(allow_work=True, write=True)
        self.assertEqual(len(self.reference_calls), 1)
        self.assertEqual(receipts[ZERO[0]["id"]]["identity"], receipts[ZERO[1]["id"]]["identity"])
        changed = copy.deepcopy(ZERO[1])
        changed["sampling_interval"] = 0.25
        _, identity1 = batch.reference_request(ZERO[0])
        _, identity2 = batch.reference_request(changed)
        self.assertNotEqual(cli.cache.fingerprint(identity1), cli.cache.fingerprint(identity2))

    def test_resume_recovers_missing_derived_evidence_without_solver_start(self):
        args = self.execute_args([RUST_ZERO], "benchmark", work=True)
        original = cli.BenchmarkBatch.derive
        with patch.object(cli.BenchmarkBatch, "derive", side_effect=KeyboardInterrupt):
            with self.assertRaises(KeyboardInterrupt):
                self.invoke(args)
        self.assertEqual(len(self.executions), 1)
        self.assertEqual(len(self.events), 1)
        with patch.object(cli.BenchmarkBatch, "derive", original):
            self.assertEqual(
                self.invoke([*args, "--resume-batch", self.batch()]), 0, self.last_error
            )
        self.assertEqual(len(self.executions), 1)
        self.assertEqual(len(self.events), 1)
        self.assertEqual(self.invoke(["--verify-only"]), 0, self.last_error)

    def test_resume_narrows_activation_without_changing_original_authority(self):
        args = self.execute_args(BRIDGES, "bridge")
        original_execute = cli.BenchmarkBatch.execute
        with patch.object(
            cli.BenchmarkBatch, "execute", return_value=({"status": "test_recipe"}, 0)
        ):
            self.assertEqual(self.invoke(args), 0, self.last_error)
        batch = cli.BenchmarkBatch(CASES_PATH, self.root, self.batch()).load()
        subset = copy.deepcopy(batch.activation())
        subset["launch_ids"] = [BRIDGES[0]["id"]]
        cli.write_artifact(
            batch.path / "subsets" / (cli.fortran.canonical_hash(subset) + ".json"),
            {
                "original_activation_bytes_sha256": batch.recipe["activation_bytes_sha256"],
                "activation": subset,
            },
            BOOK,
        )
        batch.runs.execute(subset["launch_ids"], subset)
        before = (batch.path / "activation.json").read_bytes()
        with patch.object(cli.BenchmarkBatch, "execute", original_execute):
            self.assertEqual(self.invoke([*args, "--resume-batch", batch.path]), 0, self.last_error)
        self.assertEqual(self.executions[-1][0], [BRIDGES[1]["id"]])
        self.assertEqual(self.executions[-1][1]["authorization"], subset["authorization"])
        self.assertEqual((batch.path / "activation.json").read_bytes(), before)
        self.assertEqual(len(self.events), 2)

    def test_failures_and_timeouts_never_repeat_consumed_rows(self):
        for outcome in ("failed", "timeout"):
            with self.subTest(outcome=outcome):
                self.root = self.checkout / f".development/13.3.1/{outcome}"
                self.events.clear()
                self.executions.clear()
                self.outcome = outcome
                args = self.execute_args(BRIDGES, "bridge")
                self.assertEqual(self.invoke(args), 2)
                batch = self.batch()
                self.assertEqual(self.invoke([*args, "--resume-batch", batch]), 2)
                self.assertEqual(len(self.executions), 1)
                self.assertEqual(len(self.events), 1)

    def test_bridge_numeric_failure_is_explicit_and_stops_new_work(self):
        self.bridge_status = "failed"
        args = self.execute_args(BRIDGES, "bridge")
        self.assertEqual(self.invoke(args), 1, self.last_error)
        self.assertEqual(self.invoke(["--verify-only"]), 1, self.last_error)
        self.assertEqual(self.reference_calls, [])

    def test_result_subset_and_reference_receipt_tampering_fail(self):
        args = self.execute_args([RUST_ZERO], "benchmark", work=True)
        self.assertEqual(self.invoke(args), 0, self.last_error)
        paths = [
            next((self.batch() / folder).glob("*.json"))
            for folder in ("results", "subsets", "references")
        ]
        paths.append(next(self.batch().glob("summary-*.json")))
        for path in paths:
            original = path.read_bytes()
            path.write_text('{"tampered":true}')
            with self.subTest(path=path):
                self.assertEqual(self.invoke(["--verify-only"]), 2)
            path.write_bytes(original)

    def test_forged_result_checksum_cannot_bypass_metric_reproduction(self):
        args = self.execute_args([RUST_ZERO], "benchmark", work=True)
        self.assertEqual(self.invoke(args), 0, self.last_error)
        path = next((self.batch() / "results").glob("*.json"))
        result = cli.read_artifact(path)
        result["metrics"]["raw"]["profiles"][-1]["concentration"][0][0] += 1
        path.write_bytes(cli.canonical(cli.envelope(result)) + b"\n")
        self.assertEqual(self.invoke(["--verify-only"]), 2)
        self.assertIn("derived evidence/provenance changed", self.last_error)

    def test_reference_request_mismatch_and_mutated_original_activation_block_resume(self):
        args = self.execute_args([RUST_ZERO], "benchmark", work=True)
        self.assertEqual(self.invoke(args), 0, self.last_error)
        batch = cli.BenchmarkBatch(CASES_PATH, self.root, self.batch()).load()
        altered = copy.deepcopy(RUST_ZERO)
        altered["sampling_interval"] = 0.25
        receipt, reference = batch.reference(altered, allow_work=False)
        self.assertEqual(receipt["status"], "missing")
        self.assertIsNone(reference)
        authority = cli.metrics.json_load(self.authorization)
        authority["launch_ids"] = [row["id"] for row in ZERO]
        self.authorization.write_text(json.dumps(authority))
        self.assertEqual(self.invoke([*args, "--resume-batch", self.batch()]), 2)
        self.assertEqual(len(self.executions), 1)

    def test_artifact_symlinks_and_conflicting_recovery_are_rejected(self):
        target = self.checkout / "target.json"
        cli.write_artifact(target, {"value": 1}, BOOK)
        link = self.checkout / "link.json"
        link.symlink_to(target)
        for operation in (
            lambda: cli.read_artifact(link),
            lambda: cli.write_artifact(link, {"value": 1}, BOOK),
        ):
            with self.assertRaisesRegex(ValueError, "symlink"):
                operation()
        with self.assertRaisesRegex(ValueError, "conflicting"):
            cli.write_artifact(target, {"value": 2}, BOOK)

    def test_verify_rejects_work_flags_and_other_phase_without_execution(self):
        self.assertEqual(self.invoke(["--verify-only", "--allow-reference-work"]), 2)
        self.root = self.checkout / ".development/13.4.1/runs"
        self.assertEqual(self.invoke([]), 2)
        self.assertEqual(self.executions, [])
        self.assertEqual(self.reference_calls, [])


class V2CapFreeTests(unittest.TestCase):
    def test_v2_counts_have_no_caps_and_storage_is_unbounded(self):
        book = cli.lifecycle.contract.validate(cli.lifecycle.contract.load_cases(
            cli.SOURCE_ROOT / "reference/solute_verification/CASES_V2.json"))
        with patch.object(cli.lifecycle, "all_records", return_value=[{"event": "start"}] * 3):
            self.assertEqual(cli.counts(book), {"consumed": 3, "remaining": None, "cap": None})
        self.assertEqual(cli.lifecycle.storage_cap(book), float("inf"))


if __name__ == "__main__":
    unittest.main()

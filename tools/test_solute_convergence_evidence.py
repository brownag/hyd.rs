"""Authenticated evidence boundaries with mocked cache/lifecycle work in task scratch."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools import solute_convergence_evidence as evidence
from tools import solute_verification_metrics as metrics
from tools.test_solute_convergence import BOOK, synthetic_record

ROWS = [
    row
    for row in BOOK["launches"]
    if row["phase"] == "convergence"
    and row["case_id"] == "diffusion_cosine"
    and row["solver"] == "rust"
    and row["axis"] == "space"
]


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        scratch = evidence.SOURCE_ROOT / ".development/13.4.4"
        scratch.mkdir(parents=True, exist_ok=True)
        temporary = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(temporary.cleanup)
        self.checkout = Path(temporary.name)
        self.cases_path = self.checkout / "reference/solute_verification/CASES.json"
        self.cases_path.parent.mkdir(parents=True)
        self.cases_path.write_bytes(metrics.canonical(BOOK) + b"\n")
        self.root = self.checkout / ".development/13.4.1/study/batch-fixture"
        self.current = self.root.parent / "reference-cache"
        self.prior = self.checkout / ".development/13.3.1/reviewed-cache"
        self.producers = evidence.provenance()
        self.calls = []
        self.entries = {}
        self.run_hashes = {}
        self.phase = "convergence"
        self.row = ROWS[0]
        test = self

        class FakeCache:
            def __init__(self, root):
                self.root = Path(root)

            def reference(self, *arguments, allow_work=False, allow_escalation=False):
                identity = evidence.cache.reference_identity(*arguments, allow_escalation)
                entry = self.root / evidence.cache.fingerprint(identity)
                test.calls.append((self.root, allow_work, allow_escalation))
                if entry not in test.entries:
                    if not allow_work:
                        raise ValueError("complete reference missing")
                    row = next(
                        r
                        for r in ROWS
                        if r["nodes"] == len(arguments[1]) and r["analysis_times"] == arguments[2]
                    )
                    test.cache_entry(self.root, row)
                fixture = test.entries[entry]
                if fixture["status"] not in ("complete", "safe_completed_stages"):
                    raise ValueError("invalid cache evidence: " + fixture["status"])
                if fixture["status"] == "safe_completed_stages":
                    if not allow_work:
                        raise ValueError("complete reference missing")
                    fixture["status"] = "complete"
                    (entry / "reference.json").write_text("completed mocked publication\n")
                return copy.deepcopy(fixture["values"]), None, copy.deepcopy(fixture["admission"])

        class FakeLifecycle:
            def __init__(self, cases_path, root):
                self.root = Path(root)
                self.phase = test.phase

            def verify(self, run, *, require_complete=False):
                if not require_complete:
                    raise AssertionError("complete verification required")
                manifest = metrics.json_load(run / "manifest.json")
                if manifest["status"] not in ("complete", "raw_complete"):
                    raise ValueError("incomplete/failed run")
                if evidence.artifacts.run_binding(run) != test.run_hashes[run]:
                    raise ValueError("mock independently trusted completion receipt changed")
                return manifest

        def load(path, manifest_path, cases_path):
            raw = metrics.json_load(path)
            row = next(r for r in ROWS if r["id"] == raw["launch_id"])
            case = next(c for c in BOOK["cases"] if c["id"] == row["case_id"])
            validated = metrics._ValidatedObservations(raw)
            validated._seal(
                {"case": case, "launch": row, "manifest": {}}, metrics._zero_rounding(raw)
            )
            return validated

        for patcher in (
            patch.object(evidence, "ROOT", self.checkout),
            patch.object(evidence.cache, "ReferenceCache", FakeCache),
            patch.object(evidence.lifecycle, "Lifecycle", FakeLifecycle),
            patch.object(evidence.lifecycle, "total_storage", return_value=0),
            patch.object(evidence.metrics, "load_observations", side_effect=load),
            patch.object(evidence.lifecycle, "build_rust", side_effect=AssertionError("build")),
            patch.object(evidence.lifecycle, "run_bounded", side_effect=AssertionError("launch")),
            patch.object(
                evidence.cache.analytical, "component_reference", side_effect=AssertionError("work")
            ),
            patch.object(
                evidence.cache.analytical,
                "finite_chain_reference",
                side_effect=AssertionError("work"),
            ),
        ):
            patcher.start()
            self.addCleanup(patcher.stop)
        self.helper = self.make_helper()

    def make_helper(self, **changes):
        parameters = {
            "cases_path": self.cases_path,
            "current_cache_root": self.current,
            "prior_cache_roots": [self.prior],
            "artifact_root": self.root,
            "producer_sha256": self.producers,
        }
        parameters.update(changes)
        return evidence.ConvergenceEvidence(**parameters)

    def cache_entry(self, root, row=None, *, status="complete", admission="admitted"):
        row = row or self.row
        _, identity = self.helper.reference_request(row)
        entry = root / evidence.cache.fingerprint(identity)
        entry.mkdir(parents=True, exist_ok=True)
        for name, content in (
            ("identity.json", metrics.canonical(identity)),
            ("analytical-work-ledger.jsonl", b"mock completion receipt\n"),
            ("reference.json", b"mock immutable reference publication\n"),
            ("stage-fixture.json", b"mock authenticated stage\n"),
        ):
            (entry / name).write_bytes(content)
        values = synthetic_record(row, 0)["reference_values"]
        self.entries[entry] = {
            "status": status,
            "values": values,
            "admission": {"status": admission, "uncertainty": "synthetic fixture only"},
        }
        if status == "safe_completed_stages":
            (entry / "reference.json").unlink()
        return entry

    def complete_run(self, row=None):
        row = row or self.row
        run = self.root / "runs" / (row["id"] + "-fixture")
        run.mkdir(parents=True, exist_ok=True)
        authored = synthetic_record(row, (1 / (row["nodes"] - 1)) ** 2)
        raw = authored["result"]["metrics"]["raw"]
        raw["solver_reported"] = authored["result"]["metrics"]["solver_reported"]
        raw["run_id"] = run.name
        raw["cases_sha256"] = evidence.sha256(self.cases_path)
        (run / "observations.json").write_bytes(metrics.canonical(raw))
        (run / "manifest.json").write_text(
            json.dumps(
                {
                    "launch_id": row["id"],
                    "run_id": run.name,
                    "solver": row["solver"],
                    "precision": row["precision"],
                    "status": "complete",
                }
            )
        )
        self.run_hashes[run] = evidence.artifacts.run_binding(run)
        return run

    def admitted(self, row=None):
        row = row or self.row
        self.cache_entry(self.prior, row)
        return self.helper.reference(row, write=True)

    def derived(self, row=None):
        row = row or self.row
        values, receipt = self.admitted(row)
        run = self.complete_run(row)
        record, trusted = self.helper.derive(row, run, values, receipt, write=True)
        return run, values, receipt, record, trusted

    def snapshot(self):
        return {str(path): path.read_bytes() for path in self.checkout.rglob("*") if path.is_file()}

    def rewrite_artifact(self, path, payload):
        path.write_bytes(metrics.canonical(evidence.artifacts.envelope(payload)) + b"\n")

    def review_bounds(self, bounds):
        source = self.root.parent / "reviewed-bounds.json"
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_bytes(
            metrics.canonical(
                {
                    "schema_version": "solute-convergence-bounds-v1",
                    "cases_sha256": evidence.sha256(self.cases_path),
                    "producer_sha256": self.producers,
                    "bounds_by_launch": bounds,
                }
            )
        )
        claim = source.parent / "bound-derivation.txt"
        claim.write_text(
            "Independently authored test-only bound claim, not scientific authority.\n"
        )
        review = source.parent / "bounds-review.json"
        review.write_bytes(
            metrics.canonical(
                {
                    "schema_version": "solute-convergence-bounds-review-v1",
                    "cases_sha256": evidence.sha256(self.cases_path),
                    "producer_sha256": self.producers,
                    "bounds_path": str(source),
                    "bounds_sha256": evidence.sha256(source),
                    "authorization": "Root review is mocked for synthetic bounds only.",
                    "evidence_sha256": {str(claim): evidence.sha256(claim)},
                }
            )
        )
        return {
            "path": str(source),
            "sha256": evidence.sha256(source),
            "review_path": str(review),
            "review_sha256": evidence.sha256(review),
        }

    def classify_fixture(self):
        runs = {}
        for row in ROWS:
            runs[row["id"]] = self.derived(row)[0]
        return runs, self.helper.classify(ROWS, runs, write=True)

    def summary(self, payload):
        path = self.root / "classification.json"
        return {
            "schema_version": "solute-convergence-summary-v1",
            "batch": str(self.root),
            "status": "evidence_complete",
            "classification": {
                "path": str(path),
                "sha256": evidence.sha256(path),
                "payload_sha256": evidence.digest(payload),
            },
            "results": [
                {
                    "launch_id": entry["launch_id"],
                    "path": entry["result_path"],
                    "sha256": entry["result_sha256"],
                }
                for entry in payload["ordered_evidence"]
            ],
            "references": [
                {
                    "launch_id": entry["launch_id"],
                    "path": entry["reference_path"],
                    "sha256": entry["reference_sha256"],
                }
                for entry in payload["ordered_evidence"]
            ],
            "launches": {"cap": 77, "consumed": 12, "remaining": 65},
            "producer_sha256": self.producers,
            "scientific_accuracy_accepted": False,
            "thresholds": None,
            "m5v_complete": False,
        }

    def test_scratch_accepts_v2_convergence_root_only_for_convergence(self):
        for task in evidence.CONVERGENCE_TASKS:
            evidence._scratch(evidence.ROOT / ".development" / task / "batch-x",
                              evidence.CONVERGENCE_TASKS)
        with self.assertRaises(ValueError):
            evidence._scratch(evidence.ROOT / ".development/22-holdout/batch-x",
                              evidence.CONVERGENCE_TASKS)

    def test_constructor_rejects_forged_producers_and_wrong_roots(self):
        forged = {**self.producers, "tools/solute_reference_cache.py": "0" * 64}
        for changes in (
            {"producer_sha256": forged},
            {"producer_sha256": {"fake": "1" * 64}},
            {"artifact_root": self.checkout / ".development/13.3.1/batch-other"},
            {"current_cache_root": self.prior},
            {"prior_cache_roots": [self.prior, self.prior]},
            {"prior_cache_roots": [self.current]},
            {"prior_cache_roots": [self.prior / "nested", self.prior]},
            {"allow_escalation": 1},
        ):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                self.make_helper(**changes)

    def test_constructor_rejects_symlinks_and_traversal(self):
        target = self.prior.parent / "real"
        target.mkdir(parents=True)
        self.prior.symlink_to(target, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.make_helper()
        with self.assertRaisesRegex(ValueError, "traversal"):
            self.make_helper(artifact_root=self.root / ".." / "batch-other")

    def test_context_rejects_changes_after_construction(self):
        self.cases_path.write_bytes(self.cases_path.read_bytes() + b" ")
        with self.assertRaisesRegex(ValueError, "casebook file changed"):
            self.helper.reference_request(self.row)

    def test_reference_request_exact_mesh_times_and_escalation(self):
        arguments, identity = self.helper.reference_request(self.row)
        case, coordinates, analysis_times, diagnostic = arguments
        self.assertEqual(coordinates, evidence.artifacts.fortran.grid(case, self.row["nodes"]))
        self.assertEqual(analysis_times, self.row["analysis_times"])
        self.assertEqual(diagnostic, metrics.schedule(case, self.row)[0])
        self.assertEqual(identity, evidence.cache.reference_identity(*arguments, False))
        escalated = self.make_helper(allow_escalation=True)
        self.assertNotEqual(
            evidence.cache.fingerprint(identity),
            evidence.cache.fingerprint(escalated.reference_request(self.row)[1]),
        )

    def test_reference_request_rejects_changed_row_and_phase(self):
        changed = copy.deepcopy(self.row)
        changed["nodes"] -= 1
        benchmark = next(r for r in BOOK["launches"] if r["phase"] == "benchmark")
        for row in (changed, benchmark, {"id": "absent"}):
            with self.subTest(row=row["id"]), self.assertRaises(ValueError):
                self.helper.reference_request(row)

    def test_missing_reference_returns_no_work_and_no_files(self):
        before = self.snapshot()
        values, receipt = self.helper.reference(self.row)
        self.assertIsNone(values)
        self.assertEqual(receipt["status"], "missing")
        self.assertEqual(self.calls, [])
        self.assertEqual(before, self.snapshot())

    def test_complete_prior_reuse_is_readonly_and_pins_all_artifacts(self):
        entry = self.cache_entry(self.prior)
        before = self.snapshot()
        values, receipt = self.helper.reference(self.row)
        self.assertEqual(receipt["cache_root"], str(self.prior))
        self.assertEqual(receipt["reference_values_sha256"], evidence.digest(values))
        self.assertEqual(set(receipt["artifacts"]), {p.name for p in entry.iterdir()})
        self.assertEqual(self.calls, [(self.prior, False, False)])
        self.assertEqual(before, self.snapshot())

    def test_present_invalid_prior_never_falls_back_to_current(self):
        for status in ("corrupt", "orphan", "interrupted", "failed", "safe_completed_stages"):
            with self.subTest(status=status):
                self.cache_entry(self.prior, status=status)
                before = self.snapshot()
                with self.assertRaises(ValueError):
                    self.helper.reference(self.row, allow_work=True, write=True)
                self.assertEqual(self.calls[-1], (self.prior, False, False))
                self.assertFalse(self.current.exists())
                self.assertEqual(before, self.snapshot())

    def test_unmet_prior_never_falls_back(self):
        self.cache_entry(self.prior, admission="unmet")
        with self.assertRaisesRegex(ValueError, "unmet"):
            self.helper.reference(self.row, allow_work=True, write=True)
        self.assertFalse(self.current.exists())

    def test_later_invalid_prior_is_not_ignored_after_valid_candidate(self):
        self.cache_entry(self.prior)
        other = self.prior.parent / "other-cache"
        self.cache_entry(other, status="interrupted")
        helper = self.make_helper(prior_cache_roots=[self.prior, other])
        with self.assertRaisesRegex(ValueError, "interrupted"):
            helper.reference(self.row, allow_work=True, write=True)
        self.assertEqual([root for root, *_ in self.calls], [self.prior, other])

    def test_exact_other_prior_identity_does_not_match(self):
        self.cache_entry(self.prior, ROWS[1])
        values, receipt = self.helper.reference(self.row)
        self.assertIsNone(values)
        self.assertEqual(receipt["status"], "missing")
        self.assertEqual(self.calls, [])

    def test_prior_partial_coverage_is_rejected(self):
        entry = self.cache_entry(self.prior)
        self.entries[entry]["values"]["profiles"].pop()
        with self.assertRaisesRegex(ValueError, "time coverage"):
            self.helper.reference(self.row, allow_work=True, write=True)
        self.assertFalse(self.current.exists())

    def test_reference_entry_file_or_symlink_is_invalid_present(self):
        _, identity = self.helper.reference_request(self.row)
        entry = self.prior / evidence.cache.fingerprint(identity)
        entry.parent.mkdir(parents=True)
        entry.write_text("orphan entry file")
        with self.assertRaisesRegex(ValueError, "entry missing"):
            self.helper.reference(self.row, allow_work=True, write=True)
        entry.unlink()
        entry.symlink_to(self.checkout / "absent")
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.helper.reference(self.row, allow_work=True, write=True)
        self.assertFalse(self.current.exists())

    def test_readonly_cache_mutation_during_replay_is_rejected(self):
        entry = self.cache_entry(self.prior)
        original = evidence.cache.ReferenceCache.reference

        def changed(store, *args, **kwargs):
            result = original(store, *args, **kwargs)
            (entry / "stage-fixture.json").write_text("changed during read-only replay")
            return result

        with patch.object(evidence.cache.ReferenceCache, "reference", changed):
            with self.assertRaisesRegex(ValueError, "changed during replay"):
                self.helper.reference(self.row)

    def test_conflicting_exact_prior_references_fail(self):
        self.cache_entry(self.prior)
        other = self.prior.parent / "other-cache"
        entry = self.cache_entry(other)
        self.entries[entry]["values"]["profiles"][-1]["concentration"][0][0] += 1
        helper = self.make_helper(prior_cache_roots=[self.prior, other])
        with self.assertRaisesRegex(ValueError, "conflicting complete exact"):
            helper.reference(self.row, allow_work=True, write=True)
        self.assertFalse(self.current.exists())

    def test_current_new_work_requires_write_and_explicit_flag(self):
        with self.assertRaisesRegex(ValueError, "read-only"):
            self.helper.reference(self.row, allow_work=True)
        values, receipt = self.helper.reference(self.row, allow_work=True, write=True)
        self.assertEqual(receipt["cache_root"], str(self.current))
        self.assertEqual(self.calls, [(self.current, True, False)])
        self.assertTrue(values)
        before = self.snapshot()
        self.helper.reference(self.row, allow_work=True, write=True)
        self.assertEqual(self.calls[-1], (self.current, False, False))
        self.assertEqual(before, self.snapshot())

    def test_current_completed_stages_resume_without_repeating(self):
        self.cache_entry(self.current, status="safe_completed_stages")
        with self.assertRaisesRegex(ValueError, "complete reference missing"):
            self.helper.reference(self.row)
        values, receipt = self.helper.reference(self.row, allow_work=True, write=True)
        self.assertTrue(values)
        self.assertEqual(receipt["status"], "admitted")
        self.assertEqual(self.calls[-1], (self.current, True, False))

    def test_invalid_current_cannot_repeat(self):
        for status in ("failed", "interrupted", "orphan", "corrupt"):
            with self.subTest(status=status):
                self.cache_entry(self.current, status=status)
                before = self.snapshot()
                with self.assertRaisesRegex(ValueError, "invalid cache"):
                    self.helper.reference(self.row, allow_work=True, write=True)
                self.assertEqual(before, self.snapshot())

    def test_current_unmet_admission_receipt_is_retained_without_recomputation(self):
        self.cache_entry(self.current, admission="unmet")
        values, receipt = self.helper.reference(self.row, allow_work=True, write=True)
        self.assertEqual(receipt["status"], "unmet")
        self.assertTrue(receipt["artifacts"])
        before = self.snapshot()
        replay, actual = self.helper.reference(self.row, allow_work=True, write=True)
        self.assertEqual(actual, receipt)
        self.assertEqual(replay, values)
        self.assertEqual(self.calls[-1], (self.current, False, False))
        self.assertEqual(before, self.snapshot())
        run = self.complete_run()
        with self.assertRaisesRegex(ValueError, "metrics require admitted"):
            self.helper.derive(self.row, run, values, receipt, write=True)

    def test_receipt_pins_source_even_if_new_prior_becomes_available(self):
        self.cache_entry(self.current)
        values, receipt = self.helper.reference(self.row, write=True)
        self.cache_entry(self.prior)
        before = self.snapshot()
        self.calls.clear()
        replay, actual = self.helper.reference(self.row, expected_receipt=receipt)
        self.assertEqual(replay, values)
        self.assertEqual(actual, receipt)
        self.assertEqual(self.calls, [(self.current, False, False)])
        self.assertEqual(before, self.snapshot())

    def test_receipt_and_actual_source_forgery_rejected(self):
        values, receipt = self.admitted()
        for key, replacement in (
            ("identity", "0" * 64),
            ("cache_root", str(self.current)),
            ("producer_sha256", {}),
            ("reference_values_sha256", "0" * 64),
        ):
            forged = {**receipt, key: replacement}
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, "receipt changed"):
                self.helper.reference(self.row, expected_receipt=forged)
        entry = self.prior / receipt["identity"]
        (entry / "stage-fixture.json").write_text("tampered\n")
        with self.assertRaisesRegex(ValueError, "binding changed"):
            self.helper.reference(self.row, expected_receipt=receipt)
        self.assertTrue(values)

    def test_rehashed_reference_receipt_still_requires_actual_replay(self):
        _, receipt = self.admitted()
        target = self.helper._reference_path(receipt["identity"])
        forged = copy.deepcopy(receipt)
        forged["reference_values_sha256"] = "0" * 64
        self.rewrite_artifact(target, forged)
        with self.assertRaisesRegex(ValueError, "binding changed"):
            self.helper.reference(self.row, expected_receipt=forged)

    def test_forged_receipt_unapproved_source_cannot_be_authenticated_by_rehash(self):
        _, receipt = self.admitted()
        target = self.helper._reference_path(receipt["identity"])
        forged = {**receipt, "cache_root": str(self.prior.parent / "unapproved-cache")}
        self.rewrite_artifact(target, forged)
        with self.assertRaisesRegex(ValueError, "unapproved cache source"):
            self.helper.reference(self.row, expected_receipt=forged)

    def test_derive_replays_metrics_and_preserves_run_reference_receipts(self):
        run, values, receipt, record, trusted = self.derived()
        before = self.snapshot()
        again, expected = self.helper.derive(self.row, run, values, receipt)
        self.assertEqual(again, record)
        self.assertEqual(expected, trusted)
        self.assertEqual(record["result"]["run"], self.run_hashes[run])
        self.assertEqual(record["result"]["reference"], receipt)
        self.assertEqual(record["bounds"], [])
        self.assertEqual(trusted["record_sha256"], evidence.digest(record))
        self.assertEqual(before, self.snapshot())

    def test_optional_normalization_receipt_is_preserved(self):
        values, receipt = self.admitted()
        run = self.complete_run()
        (run / "normalization.json").write_text("mock authenticated normalization receipt\n")
        self.run_hashes[run] = evidence.artifacts.run_binding(run)
        record, _ = self.helper.derive(self.row, run, values, receipt, write=True)
        self.assertEqual(record["result"]["run"], self.run_hashes[run])
        self.assertIn(str(run / "normalization.json"), record["result"]["run"])

    def test_derive_rejects_caller_reference_values_even_with_selfconsistent_hash(self):
        run, values, receipt, _, _ = self.derived()
        with self.assertRaisesRegex(ValueError, "reference receipt required"):
            self.helper.derive(self.row, run, values, None)
        values["profiles"][-1]["concentration"][0][0] += 1
        with self.assertRaisesRegex(ValueError, "caller reference values"):
            self.helper.derive(self.row, run, values, receipt)
        forged = {**receipt, "reference_values_sha256": evidence.digest(values)}
        with self.assertRaisesRegex(ValueError, "receipt changed"):
            self.helper.derive(self.row, run, values, forged)

    def test_derive_rejects_wrong_paths_phase_row_and_incomplete_run(self):
        run, values, receipt, _, _ = self.derived()
        with self.assertRaisesRegex(ValueError, "outside allowed root"):
            self.helper.derive(self.row, self.prior, values, receipt)
        self.phase = "benchmark"
        with self.assertRaisesRegex(ValueError, "wrong phase"):
            self.helper.derive(self.row, run, values, receipt)
        self.phase = "convergence"
        with self.assertRaisesRegex(ValueError, "identity differs"):
            self.helper.derive(ROWS[1], run, values, receipt)
        manifest = metrics.json_load(run / "manifest.json")
        manifest["status"] = "failed"
        (run / "manifest.json").write_text(json.dumps(manifest))
        with self.assertRaisesRegex(ValueError, "incomplete/failed"):
            self.helper.derive(self.row, run, values, receipt)

    def test_rehashed_perturbed_result_cannot_create_trusted_expectations(self):
        run, values, receipt, record, _ = self.derived()
        path = self.helper._result_path(self.row)
        forged = copy.deepcopy(record["result"])
        forged["metrics"]["derived"]["species"][0]["profiles"][-1]["norms"]["l2"]["absolute"] += 1
        self.rewrite_artifact(path, forged)
        with self.assertRaisesRegex(ValueError, "independent metrics/result replay"):
            self.helper.derive(self.row, run, values, receipt)

    def test_altered_run_receipts_and_observations_rejected(self):
        run, values, receipt, _, _ = self.derived()
        path = run / "observations.json"
        raw = metrics.json_load(path)
        raw["profiles"][-1]["concentration"][0][0] += 1
        path.write_bytes(metrics.canonical(raw))
        with self.assertRaisesRegex(ValueError, "completion receipt"):
            self.helper.derive(self.row, run, values, receipt)

    def test_producer_change_between_create_and_derive_rejected(self):
        run, values, receipt, _, _ = self.derived()
        with patch.object(evidence, "provenance", return_value={"forged": "1" * 64}):
            with self.assertRaisesRegex(ValueError, "actual source hashes"):
                self.helper.derive(self.row, run, values, receipt)

    def test_write_false_never_creates_missing_result(self):
        values, receipt = self.admitted()
        run = self.complete_run()
        before = self.snapshot()
        with self.assertRaises(FileNotFoundError):
            self.helper.derive(self.row, run, values, receipt)
        self.assertEqual(before, self.snapshot())

    def test_reviewed_bounds_have_source_review_and_cited_artifact_binding(self):
        bounds = synthetic_record(self.row, 0)["bounds"]
        context = self.review_bounds({self.row["id"]: bounds})
        helper = self.make_helper(bounds_source=context)
        values, receipt = self.admitted()
        run = self.complete_run()
        record, _ = helper.derive(self.row, run, values, receipt, write=True)
        self.assertEqual(record["bounds"], bounds)
        source = Path(context["path"])
        source.write_bytes(source.read_bytes() + b" ")
        with self.assertRaisesRegex(ValueError, "source changed"):
            helper.derive(self.row, run, values, receipt)

    def test_bounds_magic_boolean_and_review_forgery_rejected(self):
        with self.assertRaises(ValueError):
            self.make_helper(bounds_source={"reviewed": True})
        context = self.review_bounds({self.row["id"]: []})
        path = Path(context["review_path"])
        review = metrics.json_load(path)
        review["bounds_sha256"] = "0" * 64
        path.write_bytes(metrics.canonical(review))
        context["review_sha256"] = evidence.sha256(path)
        with self.assertRaisesRegex(ValueError, "root-review/source binding"):
            self.make_helper(bounds_source=context)

    def test_bounds_missing_claim_evidence_and_corrupt_citations_rejected(self):
        for mutation in ("missing", "changed"):
            context = self.review_bounds({self.row["id"]: []})
            review_path = Path(context["review_path"])
            review = metrics.json_load(review_path)
            if mutation == "missing":
                review["evidence_sha256"] = {}
            else:
                claim = Path(next(iter(review["evidence_sha256"])))
                claim.write_text("altered claim")
            review_path.write_bytes(metrics.canonical(review))
            context["review_sha256"] = evidence.sha256(review_path)
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.make_helper(bounds_source=context)

    def test_unknown_duplicate_negative_and_null_bounds(self):
        base = synthetic_record(self.row, 0)["bounds"][0]
        for mutation in ("observable", "duplicate", "negative", "time"):
            bound = copy.deepcopy(base)
            if mutation == "observable":
                bound["observable"] = "invented.error"
            elif mutation == "negative":
                bound["arithmetic"] = -1
            elif mutation == "time":
                bound["time"] = True
            entries = [bound, bound] if mutation == "duplicate" else [bound]
            context = self.review_bounds({self.row["id"]: entries})
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.make_helper(bounds_source=context)
        null_bound = {**base, "arithmetic": None, "coordinate": None, "quadrature": None}
        helper = self.make_helper(bounds_source=self.review_bounds({self.row["id"]: [null_bound]}))
        self.assertIsNone(helper._bounds()[self.row["id"]][0]["arithmetic"])

    def test_classification_uses_replay_trusted_bindings_and_missing_bounds_inconclusive(self):
        runs, payload = self.classify_fixture()
        self.assertTrue(payload["filesystem_receipts_authenticated"])
        self.assertFalse(payload["analysis"]["filesystem_receipts_authenticated"])
        self.assertTrue(all(g["status"] == "inconclusive" for g in payload["analysis"]["groups"]))
        self.assertFalse(payload["scientific_accuracy_accepted"])
        before = self.snapshot()
        with patch.object(
            evidence.analysis, "analyze_convergence", wraps=evidence.analysis.analyze_convergence
        ) as classifier:
            self.assertEqual(self.helper.classify(ROWS, runs), payload)
            book, records, expected = classifier.call_args.args
            self.assertEqual(book, BOOK)
            for record in records:
                identity = record["result"]["launch_id"]
                self.assertEqual(expected[identity]["record_sha256"], evidence.digest(record))
                self.assertEqual(expected[identity]["producer_sha256"], self.producers)
        self.assertEqual(before, self.snapshot())
        self.assertTrue(all(not work for _, work, _ in self.calls))

    def test_classification_rejects_duplicates_wrong_coverage_and_changed_order(self):
        runs, _ = self.classify_fixture()
        for rows, selected in (
            ([ROWS[0], ROWS[0]], {ROWS[0]["id"]: runs[ROWS[0]["id"]]}),
            (ROWS, {}),
            (list(reversed(ROWS)), runs),
        ):
            with self.subTest(ids=[r["id"] for r in rows]), self.assertRaises(ValueError):
                self.helper.classify(rows, selected)

    def test_rehashed_classification_and_forged_reference_paths_rejected(self):
        runs, payload = self.classify_fixture()
        target = self.root / "classification.json"
        for mutation in ("analysis", "reference_path"):
            forged = copy.deepcopy(payload)
            if mutation == "analysis":
                forged["analysis"]["scientific_accuracy_accepted"] = True
            else:
                forged["ordered_evidence"][0]["reference_path"] = str(self.prior / "forged")
            self.rewrite_artifact(target, forged)
            with self.subTest(mutation=mutation), self.assertRaisesRegex(
                ValueError, "classification replay"
            ):
                self.helper.classify(ROWS, runs)

    def test_summary_replay_historical_counts_and_readonly_contract(self):
        runs, payload = self.classify_fixture()
        summary = self.summary(payload)
        before = self.snapshot()
        with patch.object(
            evidence.artifacts, "write_artifact", side_effect=AssertionError("write")
        ), patch.object(
            evidence.lifecycle, "all_records", side_effect=AssertionError("current counts")
        ):
            self.assertEqual(self.helper.verify_summary(summary, ROWS, runs), payload)
        self.assertEqual(before, self.snapshot())

    def test_summary_rejects_forged_result_reference_classification_status_and_counts(self):
        runs, payload = self.classify_fixture()
        summary = self.summary(payload)
        for mutation in ("results", "references", "classification", "science", "counts", "order"):
            forged = copy.deepcopy(summary)
            if mutation in ("results", "references"):
                forged[mutation][0]["path"] = str(self.prior / "forged")
            elif mutation == "classification":
                forged["classification"]["sha256"] = "0" * 64
            elif mutation == "science":
                forged["m5v_complete"] = True
            elif mutation == "counts":
                forged["launches"]["consumed"] = 78
            else:
                forged["results"].reverse()
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.helper.verify_summary(forged, ROWS, runs)


if __name__ == "__main__":
    unittest.main()

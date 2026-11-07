#!/usr/bin/env python3
"""Run or verify explicitly activated, bounded convergence evidence batches."""

import argparse
import copy
import fcntl
import hashlib
import json
import sys
import uuid
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from tools import check_solute_benchmarks as batch_artifacts
from tools import solute_benchmark_bridge as bridge
from tools import solute_benchmark_lifecycle as lifecycle
from tools import solute_convergence_evidence as evidence
from tools import solute_reference_cache as cache
from tools import solute_verification_metrics as metrics

ROOT = Path(__file__).resolve().parents[1]
require = metrics.require
PHASE = "convergence"
HOLDOUT_PHASE = "holdout"
# Execution roots per phase come from lifecycle.PHASE_TASKS: v1 13.4.1/13.6.1, v2 22-convergence/22-holdout.
THRESHOLDS_V2 = ROOT / "reference/solute_verification/THRESHOLDS_V2.json"
CASES_V2 = ROOT / "reference/solute_verification/CASES_V2.json"


def _path(path):
    path = Path(path).absolute()
    require(path == ROOT or ROOT in path.parents, "path outside checkout")
    return path


def _phase_root(output_root, cases=None, phase=PHASE):
    manager = lifecycle.Lifecycle(
        cases or ROOT / "reference/solute_verification/CASES.json", output_root)
    require(manager.phase == phase,
            f"{phase} CLI requires the {phase} execution root")
    return manager


def _rows(book, ids=None, phase=PHASE):
    return lifecycle.selected_rows(book, phase, ids)


def _holdout_gate_reason(thresholds, cases):
    """Return why the threshold file does not lift the holdout reservation, or None."""
    try:
        data = metrics.json_load(thresholds)
    except FileNotFoundError:
        return "file absent"
    except (OSError, ValueError):
        return "file unreadable or not JSON"
    if not isinstance(data, dict) or data.get("schema_version") != "solute-thresholds-v2":
        return "schema_version is not solute-thresholds-v2"
    if data.get("holdout_run_permitted") is not True:
        return "holdout_run_permitted is not true"
    if data.get("cases_sha256") != hashlib.sha256(cases.read_bytes()).hexdigest():
        return "cases_sha256 does not match CASES_V2.json"
    return None


def _gate_holdout(rows, thresholds=THRESHOLDS_V2, cases=CASES_V2):
    """Reserved holdout rows run only after THRESHOLDS_V2.json permits them (Wave 6)."""
    if any(r.get("status") == "reserved_until_threshold_gate" for r in rows):
        reason = _holdout_gate_reason(thresholds, cases)
        require(reason is None,
                "holdout rows are reserved until the threshold gate (THRESHOLDS_V2.json); "
                f"dry-run only: {reason}")


def _launch_counts(book, starts):
    if not lifecycle.has_caps(book):
        return {"consumed": len(starts), "remaining": None, "cap": None}
    cap = book["budgets"]["launch_cap"]
    require(len(starts) <= cap, "historical launch cap exceeded")
    return {"consumed": len(starts), "remaining": cap - len(starts), "cap": cap}


def _producer_hashes():
    return evidence.provenance(include_cli=True)


def _ledger_snapshot(book):
    """Capture complete shared-ledger bytes and the validated records they encode."""
    snapshots = []
    for task, phase in lifecycle.book_tasks(book).items():
        path = ROOT / ".development" / task / "launch-ledger.jsonl"
        records = lifecycle.ledger_records(path, book, phase)
        data = path.read_bytes() if path.exists() else b""
        snapshots.append({
            "task": task,
            "phase": phase,
            "sha256": cache.hashlib.sha256(data).hexdigest(),
            "bytes": data.decode("ascii"),
            "records": records,
        })
    adapter = ROOT / ".development/13.2.1/launch-ledger.jsonl"
    adapter_records = cache.checked(adapter)
    require(not adapter_records.exists() or not adapter_records.read_bytes(),
            "producer task has unexpected scientific ledger")
    return snapshots


def _counts_from_snapshot(book, snapshots):
    records = [r for snapshot in snapshots for r in snapshot["records"]]
    starts = [r for r in records if r["event"] == "start"]
    return _launch_counts(book, starts)


def _verify_budget_receipt(path, book, expected_batch=None):
    """Authenticate historical counts by proving each saved ledger is a current prefix."""
    lifecycle.all_records(book, lifecycle.scope_task(expected_batch.parent.name) if expected_batch else None)
    payload = batch_artifacts.read_artifact(path)
    require(set(payload) == {"schema_version", "batch", "launches", "ledgers"}
            and payload["schema_version"] == "solute-convergence-ledger-receipt-v1",
            "historical ledger receipt schema")
    require(expected_batch is None or payload["batch"] == str(expected_batch),
            "historical ledger receipt belongs to another batch")
    current = {task: (phase, ROOT / ".development" / task / "launch-ledger.jsonl")
               for task, phase in lifecycle.book_tasks(book).items()}
    require(len(payload["ledgers"]) == len(current), "historical ledger inventory incomplete")
    verified = []
    seen = set()
    for saved in payload["ledgers"]:
        require(set(saved) == {"task", "phase", "sha256", "bytes", "records"},
                "historical ledger snapshot fields")
        task = saved["task"]
        require(task in current and task not in seen and current[task][0] == saved["phase"],
                "historical ledger task/phase mismatch")
        seen.add(task)
        raw = saved["bytes"].encode("ascii")
        require(cache.hashlib.sha256(raw).hexdigest() == saved["sha256"],
                "historical ledger snapshot hash mismatch")
        path_now = current[task][1]
        current_bytes = path_now.read_bytes() if path_now.exists() else b""
        require(current_bytes.startswith(raw), "historical ledger is not a retained prefix")
        # The current full ledger has passed the strict lifecycle parser; prefix
        # equality plus complete newline boundaries authenticates each saved event.
        require(not raw or raw.endswith(b"\n"), "truncated historical ledger prefix")
        records = [json.loads(line) for line in raw.decode("ascii").splitlines()]
        require(records == saved["records"], "historical ledger events changed")
        verified.append({"task": task, "records": records})
    expected = _counts_from_snapshot(book, verified)
    require(payload["launches"] == expected, "historical counts do not reproduce from events")
    return expected


def _write_budget_receipt(path, batch_path, book):
    snapshots = _ledger_snapshot(book)
    payload = {"schema_version": "solute-convergence-ledger-receipt-v1",
               "batch": str(batch_path), "launches": _counts_from_snapshot(book, snapshots),
               "ledgers": snapshots}
    batch_artifacts.write_artifact(path, payload, book)
    return payload["launches"]


def _verify_summary_budget(summary, receipt_path, book, batch_path):
    counts = _verify_budget_receipt(receipt_path, book, batch_path)
    require(summary.get("launches") == counts,
            "summary launch counts differ from authenticated ledger prefix")
    return counts


def _cache_roots(values):
    roots = [_path(value) for value in values]
    allowed = [ROOT / ".development" / task for task in cache.ALLOWED_TASKS]
    for root in roots:
        require(any(root == base or base in root.parents for base in allowed),
                "prior cache root outside accepted task roots")
    require(len(set(roots)) == len(roots), "duplicate prior cache root")
    return roots


class ConvergenceBatch:
    def __init__(self, cases_path, output_root, batch_path, phase=PHASE):
        self.manager = lifecycle.Lifecycle(cases_path, output_root)
        self.phase = phase
        require(self.manager.phase == phase,
                "wrong lifecycle phase")
        self.book, self.root, self.cases_path = self.manager.book, self.manager.root, self.manager.cases_path
        self.path = _path(batch_path)
        require(self.path.parent == self.root and self.path.name.startswith("batch-"),
                "invalid convergence batch path")
        self.recipe = None

    def create(self, rows, *, activation_path=None, bridges=(), prior_caches=(),
               allow_work=False, allow_escalation=False, bounds_source=None):
        require(not self.path.exists(), "batch already exists")
        activation_bytes = None
        activation = None
        if activation_path:
            source = _path(activation_path)
            activation_bytes = source.read_bytes()
            activation = metrics.json_load(source)
            lifecycle.validate_activation(self.book, self.cases_path, self.phase, rows, activation)
            require(activation_bytes == source.read_bytes(), "activation changed while reading")
        require(type(allow_work) is bool and type(allow_escalation) is bool,
                "reference policies must be explicit booleans")
        bridge_paths = [str(_path(p)) for p in bridges]
        recipe = {
            "schema_version": "solute-convergence-batch-v1", "batch": str(self.path),
            "root": str(self.root), "cases_path": str(self.cases_path),
            "cases_sha256": batch_artifacts.sha256(self.cases_path), "rows": rows,
            "mode": "execute" if activation is not None else "dry-run",
            "activation_source": str(_path(activation_path)) if activation_path else None,
            "activation_sha256": cache.hashlib.sha256(activation_bytes).hexdigest() if activation_bytes else None,
            "activation": activation, "bridge_reports": bridge_paths,
            "prior_cache_roots": [str(p) for p in prior_caches],
            "timeout_overrides": dict(lifecycle.TIMEOUT_OVERRIDES),
            "bounds_source": bounds_source,
            "current_cache_root": str(self.root / "reference-cache"),
            "allow_reference_work": allow_work, "allow_reference_escalation": allow_escalation,
            "producer_sha256": _producer_hashes(), "budgets": self.book["budgets"],
            "scientific_accuracy_accepted": False, "thresholds": None, "m5v_complete": False,
        }
        cache.mkdir_durable(self.path)
        cache.write_new(self.path / "batch.lock", b"")
        if activation_bytes is not None:
            cache.write_new(self.path / "activation.json", activation_bytes)
        batch_artifacts.write_artifact(self.path / "recipe.json", recipe, self.book)
        self.recipe = recipe

    def load(self):
        recipe = batch_artifacts.read_artifact(self.path / "recipe.json")
        require(recipe["schema_version"] == "solute-convergence-batch-v1"
                and recipe["batch"] == str(self.path) and recipe["root"] == str(self.root)
                and recipe["cases_path"] == str(self.cases_path)
                and recipe["cases_sha256"] == batch_artifacts.sha256(self.cases_path),
                "batch recipe identity changed")
        require(recipe["producer_sha256"] == _producer_hashes(), "producer provenance changed")
        require(recipe["budgets"] == self.book["budgets"], "batch budgets changed")
        require(recipe.get("timeout_overrides", {}) == lifecycle.TIMEOUT_OVERRIDES,
                "timeout overrides changed")
        ids = [row["id"] for row in recipe["rows"]]
        require(recipe["rows"] == _rows(self.book, ids, self.phase), "frozen rows changed")
        require(recipe["mode"] in ("dry-run", "execute")
                and type(recipe["allow_reference_work"]) is bool
                and type(recipe["allow_reference_escalation"]) is bool,
                "invalid work policy")
        require(recipe["scientific_accuracy_accepted"] is False
                and recipe["thresholds"] is None and recipe["m5v_complete"] is False,
                "scientific status changed")
        self.recipe = recipe
        if recipe["mode"] == "execute":
            path = self.path / "activation.json"
            raw = path.read_bytes()
            require(cache.hashlib.sha256(raw).hexdigest() == recipe["activation_sha256"],
                    "original activation bytes changed")
            activation = metrics.json_load(path)
            require(activation == recipe["activation"], "activation copy changed")
            require(batch_artifacts.sha256(recipe["activation_source"])
                    == recipe["activation_sha256"], "original activation source changed")
            lifecycle.validate_activation(self.book, self.cases_path, self.phase,
                                          recipe["rows"], activation)
        require(recipe["prior_cache_roots"] == [str(p) for p in _cache_roots(recipe["prior_cache_roots"])],
                "prior cache source changed")
        return self

    def helper(self):
        return evidence.ConvergenceEvidence(
            self.cases_path, self.root / "reference-cache", self.recipe["prior_cache_roots"],
            self.path, allow_escalation=self.recipe["allow_reference_escalation"],
            producer_sha256=self.recipe["producer_sha256"],
            bounds_source=self.recipe["bounds_source"],
        )

    def bridge_gate(self, *, write):
        dependent = [r for r in self.recipe["rows"] if r["role"] != "bridge"]
        if not dependent:
            require(not self.recipe["bridge_reports"], "bridge-only activation cannot claim dependent reports")
            return
        if bridge.is_v2_casebook(self.cases_path):
            require(not self.recipe["bridge_reports"], "v2 casebooks accept no bridge reports")
            payload = bridge.v2_gate_receipt(self.cases_path)
        else:
            require(len(self.recipe["bridge_reports"]) == 2, "both bridge reports required")
            payload = bridge.require_bridges([Path(p) for p in self.recipe["bridge_reports"]], self.cases_path)
        target = self.path / "bridge-gate.json"
        if write:
            batch_artifacts.write_artifact(target, payload, self.book)
        else:
            require(batch_artifacts.read_artifact(target) == payload, "bridge gate changed")

    def references(self, helper, *, allow_work, write):
        values, receipts = {}, {}
        for row in self.recipe["rows"]:
            if row["role"] == "bridge":
                continue
            ref, receipt = helper.reference(row, allow_work=allow_work,
                                            write=write)
            receipts[row["id"]], values[row["id"]] = receipt, ref
            if receipt.get("status") in ("missing", "unmet"):
                continue
        return values, receipts

    def reconcile(self, *, resume):
        all_rows = lifecycle.all_records(self.book, lifecycle.scope_task(self.path.parent.name))
        starts = [r for r in all_rows if r["event"] == "start"]
        by_id = {r["launch_id"]: r for r in starts}
        activation = self.recipe["activation"]
        done, pending = {}, []
        run_lifecycle = lifecycle.Lifecycle(self.cases_path, self.path / "runs")
        for row in self.recipe["rows"]:
            start = by_id.get(row["id"])
            if start is None:
                manifests = list((self.path / "runs").glob("*/manifest.json")) if (self.path / "runs").exists() else []
                require(not any(metrics.json_load(p).get("launch_id") == row["id"] for p in manifests),
                        "incomplete/failed attempt exists; automatic retry refused")
                pending.append(row)
                continue
            require(resume, "row already consumed; explicit authenticated resume required")
            run = self.path / "runs" / start["run_id"]
            manifest = run_lifecycle.verify(run, require_complete=True)
            subset = metrics.json_load(run / "activation.json")
            require(subset["phase"] == self.phase and row["id"] in subset["launch_ids"]
                    and set(subset["launch_ids"]) <= set(activation["launch_ids"]),
                    "completed run outside original activation")
            receipt = self.path / "subsets" / (lifecycle.fortran.canonical_hash(subset) + ".json")
            require(batch_artifacts.read_artifact(receipt) == {
                "original_activation_sha256": self.recipe["activation_sha256"], "activation": subset
            }, "activation subset receipt changed")
            done[row["id"]] = run
        return done, pending, run_lifecycle

    def verify(self):
        if self.recipe["mode"] == "dry-run":
            index = batch_artifacts.read_artifact(self.path / "planned-runs.json")
            require([x["launch_id"] for x in index] == [r["id"] for r in self.recipe["rows"]],
                    "planned row coverage changed")
            for item in index:
                path = Path(item["run_dir"])
                require(path.parent == self.path / "runs" and path.is_dir(), "forged planned run path")
                require(batch_artifacts.sha256(path / "manifest.json") == item["manifest_sha256"],
                        "planned manifest changed")
                manifest = lifecycle.Lifecycle(self.cases_path, self.path / "runs").verify(path)
                require(manifest["status"] == "planned" and manifest["launch_id"] == item["launch_id"],
                        "planned run identity changed")
            return {"status": "planned_verified", "rows": len(index)}, 0
        self.bridge_gate(write=False)
        helper = self.helper()
        done, pending, _ = self.reconcile(resume=True)
        require(not pending, "selected complete evidence missing")
        refs, receipts = self.references(helper, allow_work=False, write=False)
        require(all(x.get("status") == "admitted" for x in receipts.values()),
                "reference missing or unmet; verify-only cannot perform work")
        summary_path = self.path / "summary.json"
        summary = batch_artifacts.read_artifact(summary_path)
        result = helper.verify_summary(summary, self.recipe["rows"],
                                       {key: str(value) for key, value in done.items()})
        _verify_summary_budget(summary, self.path / "summary.ledger.json", self.book, self.path)
        return {"status": "evidence_verified", "classification": result}, 0

    def execute(self, *, resume=False):
        require(self.recipe["mode"] == "execute", "execution not activated")
        self.bridge_gate(write=True)
        helper = self.helper()
        refs, receipts = self.references(helper, allow_work=self.recipe["allow_reference_work"], write=True)
        nonbridge = [r for r in self.recipe["rows"] if r["role"] != "bridge"]
        require(all(receipts.get(r["id"], {}).get("status") == "admitted" for r in nonbridge),
                "exact admitted references required before solver starts")
        done, pending, run_manager = self.reconcile(resume=resume)
        # The accepted lifecycle owns all shared launch/storage reservations and event writes.
        results = {}
        runs = {}  # classify() takes run directories, not derived record dicts
        for row in self.recipe["rows"]:
            if row["id"] in done:
                runs[row["id"]] = done[row["id"]]
                results[row["id"]] = helper.derive(row, done[row["id"]], refs.get(row["id"]),
                                                    receipts.get(row["id"]), write=True)[0]
        if pending:
            subset = copy.deepcopy(self.recipe["activation"])
            subset["launch_ids"] = [r["id"] for r in pending]
            subset_path = self.path / "subsets" / (lifecycle.fortran.canonical_hash(subset) + ".json")
            batch_artifacts.write_artifact(subset_path,
                {"original_activation_sha256": self.recipe["activation_sha256"], "activation": subset}, self.book)
            run_paths = run_manager.execute(subset["launch_ids"], subset)
            require(len(run_paths) == len(pending), "incomplete lifecycle execution")
            for run in run_paths:
                manifest = metrics.json_load(run / "manifest.json")
                require(manifest["status"] in ("complete", "raw_complete"),
                        "failed/incomplete run cannot be retried")
                row = next(r for r in pending if r["id"] == manifest["launch_id"])
                runs[row["id"]] = run
                results[row["id"]] = helper.derive(row, run, refs.get(row["id"]),
                                                    receipts.get(row["id"]), write=True)[0]
        classification = helper.classify(self.recipe["rows"], runs, write=True)
        return self._publish("evidence_complete", results, refs, receipts, classification)

    def _publish(self, status, results, refs, receipts, classification):
        ordered = classification["ordered_evidence"]
        summary = {
            "schema_version": "solute-convergence-summary-v1", "batch": str(self.path),
            "status": status,
            "classification": {"path": str(self.path / "classification.json"),
                "sha256": batch_artifacts.sha256(self.path / "classification.json"),
                "payload_sha256": evidence.digest(classification)},
            "results": [{"launch_id": x["launch_id"], "path": x["result_path"],
                         "sha256": x["result_sha256"]} for x in ordered],
            "references": [{"launch_id": x["launch_id"], "path": x["reference_path"],
                            "sha256": x["reference_sha256"]} for x in ordered],
            "launches": _write_budget_receipt(self.path / "summary.ledger.json", self.path, self.book),
            "producer_sha256": self.recipe["producer_sha256"],
            "scientific_accuracy_accepted": False, "thresholds": None, "m5v_complete": False,
        }
        batch_artifacts.write_artifact(self.path / "summary.json", summary, self.book)
        return summary, 0


def parser():
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--cases", type=Path, required=True)
    result.add_argument("--output-root", type=Path, required=True)
    modes = result.add_mutually_exclusive_group()
    modes.add_argument("--dry-run", action="store_true")
    modes.add_argument("--verify-only", action="store_true")
    modes.add_argument("--execute", action="store_true")
    result.add_argument("--launch-id", action="append")
    result.add_argument("--activation-file", type=Path)
    result.add_argument("--bridge-report", type=Path, action="append", default=[])
    result.add_argument("--prior-cache-root", type=Path, action="append", default=[])
    result.add_argument("--bounds-source", type=Path)
    result.add_argument("--bounds-review", type=Path)
    result.add_argument("--allow-reference-work", action="store_true")
    result.add_argument("--allow-reference-escalation", action="store_true")
    result.add_argument("--batch", type=Path)
    result.add_argument("--resume-batch", type=Path)
    result.add_argument("--holdout", action="store_true",
                        help="operate on the holdout phase (22-holdout root); execution is "
                             "refused for reserved rows until THRESHOLDS_V2.json exists")
    return result


def requested_timeout_overrides(args):
    """Overrides come only from the activation (execute) or the frozen recipe (verify/resume)."""
    if args.activation_file:
        return dict(metrics.json_load(_path(args.activation_file)).get("timeout_overrides", {}))
    if args.batch or args.resume_batch:
        recipe = batch_artifacts.read_artifact(_path(args.batch or args.resume_batch) / "recipe.json")
        return dict(recipe.get("timeout_overrides", {}))
    return {}


def main(argv=None):
    args = parser().parse_args(argv)
    batch = None
    try:
        phase = HOLDOUT_PHASE if args.holdout else PHASE
        lifecycle.TIMEOUT_OVERRIDES.clear()
        lifecycle.TIMEOUT_OVERRIDES.update(requested_timeout_overrides(args))
        manager = lifecycle.Lifecycle(args.cases, args.output_root)
        require(manager.phase == phase,
                "wrong execution phase")
        require(not args.batch or args.verify_only, "--batch is verify-only")
        require(not args.resume_batch or args.execute, "--resume-batch requires --execute")
        if args.verify_only:
            require(not args.activation_file and not args.bridge_report
                    and not args.allow_reference_work and not args.allow_reference_escalation
                    and not args.prior_cache_root and not args.bounds_source
                    and not args.bounds_review and not args.launch_id,
                    "verify-only cannot activate work or alter source policy")
            paths = [args.batch] if args.batch else sorted(manager.root.glob("batch-*"))
            require(paths, "no convergence batches found")
            code = 0
            for path in paths:
                batch = ConvergenceBatch(args.cases, args.output_root, path, phase).load()
                with (batch.path / "batch.lock").open("rb") as lock:
                    fcntl.flock(lock, fcntl.LOCK_SH)
                    result, status = batch.verify()
                print(json.dumps({"batch": str(path), **result}, sort_keys=True))
                code = max(code, status)
            return code
        require(not args.execute or args.activation_file is not None,
                "execute requires an activation file")
        require(args.execute or not (args.activation_file or args.bridge_report
                    or args.allow_reference_work or args.allow_reference_escalation),
                "dry-run cannot activate work")
        activation = metrics.json_load(args.activation_file) if args.activation_file else None
        ids = args.launch_id if args.launch_id is not None else activation["launch_ids"] if activation else None
        rows = _rows(manager.book, ids, phase)
        if args.execute:
            if args.holdout:
                _gate_holdout(rows)
            lifecycle.validate_activation(manager.book, args.cases, phase, rows, activation)
            if args.holdout:
                # Holdout rows have no convergence evidence to derive: run via the lifecycle only.
                paths = manager.execute([r["id"] for r in rows], activation)
                print(json.dumps({"status": "holdout_runs_complete",
                                  "runs": [str(x) for x in paths]}, sort_keys=True))
                return 0
            if any(row["role"] != "bridge" for row in rows):
                if bridge.is_v2_casebook(args.cases):
                    require(not args.bridge_report, "v2 casebooks accept no bridge reports")
                else:
                    bridge.require_bridges(args.bridge_report, args.cases)
        prior = _cache_roots(args.prior_cache_root)
        require(bool(args.bounds_source) == bool(args.bounds_review),
                "bounds source and review must be supplied together")
        bounds = None
        if args.bounds_source:
            source, review = _path(args.bounds_source), _path(args.bounds_review)
            bounds = {"path": str(source), "sha256": batch_artifacts.sha256(source),
                      "review_path": str(review), "review_sha256": batch_artifacts.sha256(review)}
        path = args.resume_batch or manager.root / ("batch-" + uuid.uuid4().hex)
        batch = ConvergenceBatch(args.cases, args.output_root, path, phase)
        if args.resume_batch:
            batch.load()
            require(batch.recipe["rows"] == rows
                    and batch.recipe["activation_sha256"] == batch_artifacts.sha256(args.activation_file)
                    and batch.recipe["bridge_reports"] == [str(_path(p)) for p in args.bridge_report]
                    and batch.recipe["prior_cache_roots"] == [str(p) for p in prior]
                    and batch.recipe["bounds_source"] == bounds
                    and batch.recipe["allow_reference_work"] == args.allow_reference_work
                    and batch.recipe["allow_reference_escalation"] == args.allow_reference_escalation,
                    "resume must preserve original activation, source and work policy")
        else:
            batch.create(rows, activation_path=args.activation_file,
                bridges=args.bridge_report, prior_caches=prior,
                allow_work=args.allow_reference_work,
                allow_escalation=args.allow_reference_escalation,
                bounds_source=bounds)
        with (batch.path / "batch.lock").open("r+b") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            if args.execute:
                summary, code = batch.execute(resume=bool(args.resume_batch))
            else:
                paths = lifecycle.Lifecycle(args.cases, batch.path / "runs").dry_run([r["id"] for r in rows])
                index = [{"launch_id": row["id"], "run_dir": str(path),
                          "manifest_sha256": batch_artifacts.sha256(path / "manifest.json")}
                         for row, path in zip(rows, paths)]
                require(len(index) == len(rows), "incomplete planned run index")
                batch_artifacts.write_artifact(batch.path / "planned-runs.json", index, batch.book)
                summary, code = {"status": "planned", "rows": len(rows)}, 0
        print(json.dumps({"batch": str(batch.path), **summary}, sort_keys=True))
        return code
    except (OSError, ValueError, KeyError, TypeError, StopIteration) as error:
        print(f"error: {error}" + (f"; batch={batch.path}" if batch else ""), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

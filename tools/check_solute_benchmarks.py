#!/usr/bin/env python3
"""Benchmark recipes and explicitly activated evidence, with immutable resumable batches."""

import argparse
import copy
import fcntl
import json
import sys
import uuid
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from tools import run_solute_verification_reference as fortran
from tools import solute_benchmark_bridge as bridge
from tools import solute_benchmark_lifecycle as lifecycle
from tools import solute_reference_cache as cache
from tools import solute_verification_metrics as metrics

SOURCE_ROOT = Path(__file__).resolve().parents[1]
PRODUCERS = (
    "tools/check_solute_benchmarks.py",
    "tools/solute_benchmark_lifecycle.py",
    "tools/solute_benchmark_bridge.py",
    "tools/solute_reference_cache.py",
    "tools/solute_analytical_reference.py",
    "tools/solute_verification_metrics.py",
    "tools/run_solute_verification_reference.py",
    "tools/check_solute_verification_spec.py",
)
require = fortran.require
sha256 = fortran.sha256
canonical = metrics.canonical


def provenance():
    return {name: sha256(SOURCE_ROOT / name) for name in PRODUCERS}


def contained(root, path):
    path = Path(path).absolute()
    return fortran.safe_path(root, path.relative_to(Path(root).absolute()))


def envelope(payload):
    return {
        "schema_version": "solute-benchmark-artifact-v1",
        "payload": payload,
        "payload_sha256": fortran.canonical_hash(payload),
    }


def read_artifact(path):
    path = cache.checked(path)
    data = metrics.json_load(path)
    require(
        isinstance(data, dict)
        and set(data) == {"schema_version", "payload", "payload_sha256"}
        and data["schema_version"] == "solute-benchmark-artifact-v1",
        "integration artifact schema",
    )
    require(
        fortran.canonical_hash(data["payload"]) == data["payload_sha256"],
        "integration artifact hash",
    )
    return data["payload"]


def write_artifact(path, payload, book):
    path = cache.checked(path)
    data = canonical(envelope(payload)) + b"\n"
    if path.exists():
        require(path.read_bytes() == data, "conflicting immutable integration artifact")
        return path
    used = lifecycle.total_storage() if lifecycle.has_caps(book) else lifecycle.total_storage(book)
    require(
        used + len(data) <= lifecycle.storage_cap(book),
        "integration artifact exceeds global storage cap",
    )
    cache.mkdir_durable(path.parent)
    cache.write_new(path, data)
    return path


def run_binding(run):
    names = ["manifest.json", "observations.json"]
    if (run / "normalization.json").exists():
        names.append("normalization.json")
    return {str(run / name): sha256(run / name) for name in names}


def counts(book):
    starts = [r for r in lifecycle.all_records(book) if r["event"] == "start"]
    if not lifecycle.has_caps(book):  # v2 (ADR 0003): no ceilings
        return {"consumed": len(starts), "remaining": None, "cap": None}
    return {
        "consumed": len(starts),
        "remaining": book["budgets"]["launch_cap"] - len(starts),
        "cap": book["budgets"]["launch_cap"],
    }


def requested_rows(book, stage, ids=None):
    rows = lifecycle.selected_rows(book, "benchmark", ids)
    require(stage in ("all", "bridge", "benchmark"), "unknown benchmark stage")
    if stage != "all":
        rows = [r for r in rows if (r["role"] == "bridge") == (stage == "bridge")]
    require(
        rows and (ids is None or {r["id"] for r in rows} == set(ids)),
        "stage/row selection mismatch",
    )
    return rows


def commands(cases_path, root):
    base = [
        "python3",
        "tools/check_solute_benchmarks.py",
        "--cases",
        str(cases_path),
        "--output-root",
        str(root),
    ]
    return {
        "bridge": base
        + ["--execute", "--stage", "bridge", "--activation-file", "<bridge-activation.json>"],
        "benchmark": base
        + [
            "--execute",
            "--stage",
            "benchmark",
            "--activation-file",
            "<benchmark-activation.json>",
            "--bridge-report",
            "<fortran-bridge.json>",
            "--bridge-report",
            "<rust-bridge.json>",
            "--allow-reference-work",
        ],
        "verify": base + ["--verify-only"],
    }


class BenchmarkBatch:
    def __init__(self, cases_path, output_root, batch_path):
        self.manager = lifecycle.Lifecycle(cases_path, output_root)
        require(self.manager.phase == "benchmark", "benchmark CLI cannot activate other phases")
        self.book = self.manager.book
        self.root = self.manager.root
        self.cases_path = self.manager.cases_path
        self.path = contained(self.root, batch_path)
        require(
            self.path.parent == self.root and self.path.name.startswith("batch-"),
            "invalid batch path",
        )
        self.runs = lifecycle.Lifecycle(cases_path, self.path / "runs")
        self.recipe = None

    def create(
        self,
        rows,
        stage,
        *,
        activation_path=None,
        reports=(),
        allow_work=False,
        allow_escalation=False,
    ):
        require(not self.path.exists(), "batch already exists")
        activation = None
        activation_bytes = None
        if activation_path is not None:
            activation_path = Path(activation_path).absolute()
            activation_bytes = activation_path.read_bytes()
            activation = metrics.json_load(activation_path)
            lifecycle.validate_activation(self.book, self.cases_path, "benchmark", rows, activation)
            require(
                activation_bytes == activation_path.read_bytes(), "activation changed while reading"
            )
        recipe = {
            "schema_version": "solute-benchmark-batch-v1",
            "created_utc": fortran.utc(),
            "batch_path": str(self.path),
            "output_root": str(self.root),
            "cases_path": str(self.cases_path),
            "cases_sha256": sha256(self.cases_path),
            "producer_sha256": provenance(),
            "mode": "execute" if activation is not None else "dry-run",
            "stage": stage,
            "rows": rows,
            "budgets": self.book["budgets"],
            "activation_source": str(activation_path) if activation is not None else None,
            "activation_sha256": (
                fortran.canonical_hash(activation) if activation is not None else None
            ),
            "activation_bytes_sha256": (
                cache.hashlib.sha256(activation_bytes).hexdigest()
                if activation_bytes is not None
                else None
            ),
            "bridge_reports": [str(Path(p).absolute()) for p in reports],
            "reference_cache": str(self.root / "reference-cache"),
            "allow_reference_work": allow_work,
            "allow_reference_escalation": allow_escalation,
            "prospective_commands": commands(self.cases_path, self.root),
            "scientific_accuracy_accepted": False,
            "thresholds": None,
            "m5v_complete": False,
        }
        cache.mkdir_durable(self.path)
        cache.write_new(self.path / "batch.lock", b"")
        if activation_bytes is not None:
            cache.write_new(self.path / "activation.json", activation_bytes)
        write_artifact(self.path / "recipe.json", recipe, self.book)
        self.recipe = recipe

    def load(self):
        recipe = read_artifact(contained(self.path, self.path / "recipe.json"))
        require(
            recipe["schema_version"] == "solute-benchmark-batch-v1"
            and recipe["batch_path"] == str(self.path)
            and recipe["output_root"] == str(self.root)
            and recipe["cases_path"] == str(self.cases_path)
            and recipe["cases_sha256"] == sha256(self.cases_path),
            "batch identity changed",
        )
        require(recipe["producer_sha256"] == provenance(), "batch producer provenance changed")
        require(recipe["budgets"] == self.book["budgets"], "batch limits changed")
        require(
            recipe["rows"]
            == requested_rows(self.book, recipe["stage"], [r["id"] for r in recipe["rows"]]),
            "batch rows changed",
        )
        require(
            recipe["reference_cache"] == str(self.root / "reference-cache"), "cache path changed"
        )
        require(
            recipe["mode"] in ("dry-run", "execute")
            and type(recipe["allow_reference_work"]) is bool
            and type(recipe["allow_reference_escalation"]) is bool,
            "invalid batch mode",
        )
        require(
            recipe["thresholds"] is None
            and recipe["scientific_accuracy_accepted"] is False
            and recipe["m5v_complete"] is False,
            "unassigned science status changed",
        )
        self.recipe = recipe
        if recipe["mode"] == "execute":
            self.activation()
        return self

    def activation(self):
        path = contained(self.path, self.path / "activation.json")
        require(
            sha256(path) == self.recipe["activation_bytes_sha256"],
            "original activation bytes changed",
        )
        require(
            sha256(Path(self.recipe["activation_source"]))
            == self.recipe["activation_bytes_sha256"],
            "original activation source changed",
        )
        activation = metrics.json_load(path)
        require(
            fortran.canonical_hash(activation) == self.recipe["activation_sha256"],
            "activation identity changed",
        )
        lifecycle.validate_activation(
            self.book, self.cases_path, "benchmark", self.recipe["rows"], activation
        )
        return activation

    def gate(self, *, write=False):
        if self.recipe["stage"] == "bridge":
            require(
                not self.recipe["bridge_reports"],
                "bridge stage does not consume dependent gate reports",
            )
            return
        require(self.recipe["stage"] == "benchmark", "explicit execution stage required")
        if bridge.is_v2_casebook(self.cases_path):
            require(not self.recipe["bridge_reports"], "v2 casebooks accept no bridge reports")
            receipt = bridge.v2_gate_receipt(self.cases_path)
        else:
            receipt = bridge.require_bridges(
                [Path(p) for p in self.recipe["bridge_reports"]], self.cases_path
            )
        target = self.path / "bridge-gate.json"
        if write:
            write_artifact(target, receipt, self.book)
        else:
            require(read_artifact(target) == receipt, "bridge gate changed")

    def prepare(self):
        paths = self.runs.dry_run([r["id"] for r in self.recipe["rows"]])
        require(len(paths) == len(self.recipe["rows"]), "incomplete/excess dry-run rows")
        index = [
            {
                "launch_id": row["id"],
                "run_dir": str(path),
                "manifest_sha256": sha256(path / "manifest.json"),
            }
            for row, path in zip(self.recipe["rows"], paths)
        ]
        require(len(index) == len(self.recipe["rows"]), "incomplete dry-run rows")
        write_artifact(self.path / "planned-runs.json", index, self.book)
        return {"status": "planned", "rows": len(index), "scientific_accuracy_accepted": False}

    def reconcile(self, *, resume):
        records = lifecycle.all_records(self.book)
        started = {r["launch_id"]: r for r in records if r["event"] == "start"}
        complete, pending = {}, []
        activation = self.activation()
        for row in self.recipe["rows"]:
            start = started.get(row["id"])
            if start is None:
                attempts = [
                    p
                    for p in (self.path / "runs").glob("*/manifest.json")
                    if metrics.json_load(p).get("launch_id") == row["id"]
                ]
                require(
                    not attempts,
                    "existing incomplete/failed attempt requires review, not automatic retry",
                )
                pending.append(row)
                continue
            require(resume, "row already consumed; explicit original-batch resume required")
            run = contained(self.path / "runs", self.path / "runs" / start["run_id"])
            self.runs.verify(run, require_complete=True)
            original_subset = metrics.json_load(run / "activation.json")
            require(
                all(
                    original_subset[k] == activation[k]
                    for k in ("cases_sha256", "phase", "kind", "authorization")
                )
                and set(original_subset["launch_ids"]) <= set(activation["launch_ids"]),
                "completed run does not belong to original activation",
            )
            subset_path = (
                self.path / "subsets" / (fortran.canonical_hash(original_subset) + ".json")
            )
            require(
                read_artifact(subset_path)
                == {
                    "original_activation_bytes_sha256": self.recipe["activation_bytes_sha256"],
                    "activation": original_subset,
                },
                "subset authorization receipt changed",
            )
            complete[row["id"]] = run
        return complete, pending

    def reference_request(self, row):
        case = next(c for c in self.book["cases"] if c["id"] == row["case_id"])
        coordinates = fortran.grid(case, row["nodes"])
        diagnostic_times, _ = metrics.schedule(case, row)
        arguments = (case, coordinates, row["analysis_times"], diagnostic_times)
        identity = cache.reference_identity(*arguments, self.recipe["allow_reference_escalation"])
        return arguments, identity

    def reference(self, row, *, allow_work):
        arguments, identity = self.reference_request(row)
        key = cache.fingerprint(identity)
        root = contained(self.root, Path(self.recipe["reference_cache"]))
        entry = contained(root, root / key)
        if not allow_work and not entry.exists():
            return {
                "status": "missing",
                "identity": key,
                "coverage": "full matching reference absent",
            }, None
        reference, diagnostics, admission = cache.ReferenceCache(root).reference(
            *arguments,
            allow_work=allow_work,
            allow_escalation=self.recipe["allow_reference_escalation"],
        )
        bindings = {
            p.name: sha256(contained(entry, p)) for p in sorted(entry.iterdir()) if p.is_file()
        }
        receipt = {
            "status": admission["status"],
            "identity": key,
            "cache_root": str(root),
            "artifacts": bindings,
            "admission": admission,
        }
        return receipt, reference

    def references(self, *, allow_work, write):
        references, receipts, reused = {}, {}, {}
        for row in self.recipe["rows"]:
            _, identity = self.reference_request(row)
            request_key = cache.fingerprint(identity)
            if request_key not in reused:
                reused[request_key] = self.reference(row, allow_work=allow_work)
            receipt, reference = reused[request_key]
            receipts[row["id"]] = receipt
            references[row["id"]] = reference
            if receipt["status"] != "missing":
                target = self.path / "references" / (receipt["identity"] + ".json")
                if write:
                    write_artifact(target, receipt, self.book)
                else:
                    require(read_artifact(target) == receipt, "reference binding/admission changed")
        return references, receipts

    def derive(self, row, run, reference=None, reference_receipt=None, *, write):
        self.runs.verify(run, require_complete=True)
        target = self.path / "results" / (row["id"] + ".json")
        if row["role"] == "bridge" and bridge.is_v2_casebook(self.cases_path):
            result = {
                "kind": "bridge",
                "launch_id": row["id"],
                "status": "recorded",
                "run": run_binding(run),
                "scientific_accuracy_accepted": False,
            }
        elif row["role"] == "bridge":
            report = self.path / "bridges" / row["id"] / "bridge.json"
            if not report.exists():
                require(write, "bridge report missing")
                bridge.write_report(self.cases_path, run, report.parent)
            payload = bridge.verify_report(report, self.cases_path)
            require(
                payload["launch_id"] == row["id"]
                and payload["run_dir"] == str(run.relative_to(bridge.ROOT)),
                "bridge report points to another run",
            )
            result = {
                "kind": "bridge",
                "launch_id": row["id"],
                "status": payload["comparison"]["status"],
                "run": run_binding(run),
                "report": str(report),
                "report_sha256": sha256(report),
                "scientific_accuracy_accepted": False,
            }
        else:
            require(
                reference_receipt["status"] == "admitted", "metrics require admitted full reference"
            )
            data = metrics.load_observations(
                run / "observations.json", run / "manifest.json", self.cases_path
            )
            result = {
                "kind": "benchmark",
                "launch_id": row["id"],
                "status": "metrics_complete",
                "run": run_binding(run),
                "reference": reference_receipt,
                "metrics": metrics.compute_metrics(data, reference),
                "producer_sha256": provenance(),
                "scientific_accuracy_accepted": False,
                "thresholds": None,
                "m5v_complete": False,
            }
        if write:
            write_artifact(target, result, self.book)
        else:
            require(read_artifact(target) == result, "derived evidence/provenance changed")
        return result

    def execute(self, *, resume=False):
        self.gate(write=True)
        complete, pending = self.reconcile(resume=resume)
        references, receipts = {}, {}
        if self.recipe["stage"] == "benchmark":
            references, receipts = self.references(
                allow_work=self.recipe["allow_reference_work"], write=True
            )
            if any(r["status"] != "admitted" for r in receipts.values()):
                return {
                    "status": "reference_unmet",
                    "references": receipts,
                    "scientific_accuracy_accepted": False,
                }, 1
        results = {}
        for row in self.recipe["rows"]:
            if row["id"] in complete:
                results[row["id"]] = self.derive(
                    row,
                    complete[row["id"]],
                    references.get(row["id"]),
                    receipts.get(row["id"]),
                    write=True,
                )
        if any(r["status"] == "failed" for r in results.values()):
            return {"status": "bridge_failed", "results": results}, 1
        if pending:
            subset = copy.deepcopy(self.activation())
            subset["launch_ids"] = [row["id"] for row in pending]
            write_artifact(
                self.path / "subsets" / (fortran.canonical_hash(subset) + ".json"),
                {
                    "original_activation_bytes_sha256": self.recipe["activation_bytes_sha256"],
                    "activation": subset,
                },
                self.book,
            )
            paths = self.runs.execute(subset["launch_ids"], subset)
            for run in paths:
                manifest = metrics.json_load(run / "manifest.json")
                require(
                    manifest["status"] in ("complete", "raw_complete"),
                    f"solver lifecycle ended {manifest['status']}; row cannot be automatically retried",
                )
                row = next(r for r in pending if r["id"] == manifest["launch_id"])
                results[row["id"]] = self.derive(
                    row, run, references.get(row["id"]), receipts.get(row["id"]), write=True
                )
            require(len(paths) == len(pending), "incomplete lifecycle batch")
        status = (
            "bridge_failed"
            if any(r["status"] == "failed" for r in results.values())
            else "evidence_complete"
        )
        return {
            "status": status,
            "results": results,
            "scientific_accuracy_accepted": False,
            "thresholds": None,
            "m5v_complete": False,
        }, int(status == "bridge_failed")

    def verify(self):
        if self.recipe["mode"] == "dry-run":
            index = read_artifact(self.path / "planned-runs.json")
            require(
                [r["launch_id"] for r in index] == [r["id"] for r in self.recipe["rows"]],
                "planned row coverage changed",
            )
            seen_runs = set()
            for row in index:
                run = contained(self.path / "runs", Path(row["run_dir"]))
                require(run not in seen_runs, "duplicate planned run path")
                seen_runs.add(run)
                require(
                    sha256(run / "manifest.json") == row["manifest_sha256"],
                    "planned manifest changed",
                )
                manifest = self.runs.verify(run)
                require(manifest["status"] == "planned", "planned batch contains executed data")
                require(
                    manifest["launch_id"] == row["launch_id"], "planned launch identity changed"
                )
            return {
                "status": "planned_verified",
                "rows": len(index),
                "scientific_accuracy_accepted": False,
            }, 0
        self.gate(write=False)
        complete, pending = self.reconcile(resume=True)
        references, receipts = (
            ({}, {})
            if self.recipe["stage"] == "bridge"
            else self.references(allow_work=False, write=False)
        )
        if any(r["status"] != "admitted" for r in receipts.values()):
            require(not complete, "completed solver evidence lacks admitted reference")
            return {
                "status": "reference_unmet",
                "references": receipts,
                "scientific_accuracy_accepted": False,
            }, 1
        require(not pending, "selected complete evidence missing")
        results = {
            row["id"]: self.derive(
                row,
                complete[row["id"]],
                references.get(row["id"]),
                receipts.get(row["id"]),
                write=False,
            )
            for row in self.recipe["rows"]
        }
        failed = any(r["status"] == "failed" for r in results.values())
        return {
            "status": "bridge_failed" if failed else "evidence_verified",
            "rows": len(results),
            "scientific_accuracy_accepted": False,
            "thresholds": None,
            "m5v_complete": False,
        }, int(failed)

    def verify_summaries(self):
        """Authenticate retained summaries without treating historical counts as current."""
        ids = {row["id"] for row in self.recipe["rows"]}
        for path in sorted(self.path.glob("summary-*.json")):
            summary = read_artifact(path)
            require(
                summary["batch"] == str(self.path)
                and summary["scientific_accuracy_accepted"] is False
                and summary["thresholds"] is None
                and summary["m5v_complete"] is False,
                "summary identity/scientific status changed",
            )
            for identity, result in summary.get("results", {}).items():
                require(identity in ids, "summary contains an unselected row")
                target = self.path / "results" / (identity + ".json")
                require(
                    result["path"] == str(target)
                    and sha256(cache.checked(target)) == result["sha256"]
                    and read_artifact(target)["status"] == result["status"],
                    "summary result binding changed",
                )


def parser():
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--cases", type=Path, required=True)
    result.add_argument("--output-root", type=Path, required=True)
    modes = result.add_mutually_exclusive_group()
    for mode in ("dry-run", "verify-only", "execute"):
        modes.add_argument("--" + mode, action="store_true")
    result.add_argument("--stage", choices=("all", "bridge", "benchmark"), default="all")
    result.add_argument("--launch-id", action="append")
    result.add_argument("--activation-file", type=Path)
    result.add_argument("--bridge-report", type=Path, action="append", default=[])
    result.add_argument("--allow-reference-work", action="store_true")
    result.add_argument("--allow-reference-escalation", action="store_true")
    result.add_argument("--batch", type=Path)
    result.add_argument("--resume-batch", type=Path)
    return result


def main(argv=None):
    args = parser().parse_args(argv)
    batch = None
    try:
        manager = lifecycle.Lifecycle(args.cases, args.output_root)
        require(manager.phase == "benchmark", "benchmark CLI cannot select another phase")
        require(not args.batch or args.verify_only, "--batch is verify-only")
        require(not args.resume_batch or args.execute, "--resume-batch requires execute")
        if args.verify_only:
            require(
                not args.activation_file
                and not args.bridge_report
                and not args.allow_reference_work
                and not args.allow_reference_escalation
                and args.stage == "all"
                and not args.launch_id,
                "verify-only cannot activate or select new work",
            )
            paths = [args.batch] if args.batch else sorted(manager.root.glob("batch-*"))
            require(paths, "no recorded benchmark batches")
            code = 0
            for path in paths:
                batch = BenchmarkBatch(args.cases, args.output_root, path).load()
                with contained(batch.path, batch.path / "batch.lock").open("rb") as lock:
                    fcntl.flock(lock, fcntl.LOCK_SH)
                    result, status = batch.verify()
                    batch.verify_summaries()
                print(
                    json.dumps(
                        {"batch": str(path), **result, "launches": counts(manager.book)},
                        sort_keys=True,
                    )
                )
                code = max(code, status)
            return code
        require(
            args.execute
            or not any(
                (
                    args.activation_file,
                    args.bridge_report,
                    args.allow_reference_work,
                    args.allow_reference_escalation,
                )
            ),
            "dry-run cannot activate work",
        )
        activation = metrics.json_load(args.activation_file) if args.activation_file else None
        if args.execute:
            require(
                activation is not None and args.stage in ("bridge", "benchmark"),
                "execute requires an explicit stage and activation file",
            )
            require(
                args.stage != "bridge"
                or not (args.allow_reference_work or args.allow_reference_escalation),
                "bridge stage requires no reference work",
            )
        ids = (
            args.launch_id
            if args.launch_id is not None
            else activation["launch_ids"] if activation else None
        )
        rows = requested_rows(manager.book, args.stage, ids)
        if args.execute:
            lifecycle.validate_activation(manager.book, args.cases, "benchmark", rows, activation)
            if args.stage == "benchmark":
                if bridge.is_v2_casebook(args.cases):
                    require(not args.bridge_report, "v2 casebooks accept no bridge reports")
                else:
                    bridge.require_bridges(args.bridge_report, args.cases)
        path = args.resume_batch or manager.root / ("batch-" + uuid.uuid4().hex)
        batch = BenchmarkBatch(args.cases, args.output_root, path)
        if args.resume_batch:
            batch.load()
            require(
                batch.recipe["mode"] == "execute"
                and batch.recipe["stage"] == args.stage
                and batch.recipe["rows"] == rows
                and batch.recipe["activation_bytes_sha256"] == sha256(args.activation_file)
                and batch.recipe["bridge_reports"]
                == [str(p.absolute()) for p in args.bridge_report]
                and batch.recipe["allow_reference_work"] == args.allow_reference_work
                and batch.recipe["allow_reference_escalation"] == args.allow_reference_escalation,
                "resume must preserve original batch/authorization/work policy",
            )
        else:
            batch.create(
                rows,
                args.stage,
                activation_path=args.activation_file,
                reports=args.bridge_report,
                allow_work=args.allow_reference_work,
                allow_escalation=args.allow_reference_escalation,
            )
        with contained(batch.path, batch.path / "batch.lock").open("r+b") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            result, code = (
                batch.execute(resume=bool(args.resume_batch))
                if args.execute
                else (batch.prepare(), 0)
            )
            summary = {
                "batch": str(batch.path),
                **result,
                "launches": counts(manager.book),
                "scientific_accuracy_accepted": False,
                "thresholds": None,
                "m5v_complete": False,
            }
            if "results" in result:
                summary["results"] = {
                    identity: {
                        "status": item["status"],
                        "path": str(batch.path / "results" / (identity + ".json")),
                        "sha256": sha256(batch.path / "results" / (identity + ".json")),
                    }
                    for identity, item in result["results"].items()
                }
            write_artifact(
                batch.path / ("summary-" + uuid.uuid4().hex + ".json"), summary, manager.book
            )
        print(
            json.dumps(
                {k: v for k, v in summary.items() if k not in ("results", "references")},
                sort_keys=True,
            )
        )
        return code
    except (OSError, ValueError, KeyError, TypeError, StopIteration) as error:
        print(
            f"error: {error}" + (f"; batch={batch.path}" if batch is not None else ""),
            file=sys.stderr,
        )
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

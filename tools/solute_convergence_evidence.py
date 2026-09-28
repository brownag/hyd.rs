"""Authenticate convergence evidence without owning activation or numerical launches.

Caller context must come from an authenticated batch recipe. Root review remains the
authority for bound claims; matching source hashes authenticates their provenance,
not their mathematical validity. Every read-only operation replays existing evidence.
"""

import copy
from pathlib import Path

from tools import check_solute_benchmarks as artifacts
from tools import solute_benchmark_lifecycle as lifecycle
from tools import solute_convergence_analysis as analysis
from tools import solute_reference_cache as cache
from tools import solute_verification_metrics as metrics

ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOT = ROOT
PRODUCERS = (
    *artifacts.PRODUCERS,
    "tools/solute_convergence_analysis.py",
    "tools/solute_convergence_evidence.py",
)
CLI_PRODUCER = "tools/run_solute_convergence.py"
OBSERVABLES = {
    *("concentration." + name for name in ("l1", "l2", "linf", "endpoint_linf", "interior_linf")),
    *("inventory." + name for name in ("dissolved", "equilibrium_sorbed", "total")),
    "reaction.production",
    "reaction.loss",
    "boundary.instantaneous.0",
    "boundary.instantaneous.1",
    "boundary.cumulative.0",
    "boundary.cumulative.1",
    "balance",
}
require = metrics.require
digest = analysis.digest
sha256 = artifacts.sha256


def provenance(*, include_cli=False):
    """Hash actual producers; an optional future CLI is an exact named producer."""
    names = (*PRODUCERS, CLI_PRODUCER) if include_cli else PRODUCERS
    return {name: sha256(cache.checked(SOURCE_ROOT / name)) for name in names}


def _path(path, root):
    path = cache.checked(path)
    require(path == root or root in path.parents, "evidence path outside allowed root")
    return path


def _scratch(path, tasks):
    path = _path(path, ROOT)
    relative = path.relative_to(ROOT)
    require(
        len(relative.parts) >= 2
        and relative.parts[0] == ".development"
        and relative.parts[1] in tasks,
        "evidence outside authorized task scratch",
    )
    return path


def _same(left, right, message):
    require(cache.encode(left, sort_maps=True) == cache.encode(right, sort_maps=True), message)


def _cache_bindings(entry):
    require(entry.is_dir(), "reference entry missing")
    bindings = {}
    for path in sorted(entry.iterdir()):
        path = _path(path, entry)
        require(path.is_file(), "unexpected reference entry member")
        bindings[path.name] = sha256(path)
    return bindings


class ConvergenceEvidence:
    """Replay exact references, complete runs, reviewed bounds and classifications.

    ``producer_sha256`` and ``bounds_source`` are authenticated recipe context, not
    expectations supplied by an untrusted result. Bounds source is either None or
    {path, sha256, review_path, review_sha256}; its review record binds the source,
    producer map, casebook and cited evidence. The caller authenticates root authority.
    """

    def __init__(
        self,
        cases_path,
        current_cache_root,
        prior_cache_roots,
        artifact_root,
        *,
        allow_escalation=False,
        producer_sha256,
        bounds_source=None,
    ):
        require(type(allow_escalation) is bool, "explicit escalation policy required")
        self.cases_path = _path(cases_path, ROOT)
        self.cases_sha256 = sha256(self.cases_path)
        self.book = lifecycle.contract.validate(lifecycle.contract.load_cases(self.cases_path))
        self.book_sha256 = digest(self.book)
        self.root = _scratch(artifact_root, ("13.4.1",))
        require(self.root.name.startswith("batch-"), "evidence needs an original batch root")
        self.current_cache = _scratch(current_cache_root, ("13.4.1",))
        require(
            self.current_cache == self.root.parent / "reference-cache",
            "current cache differs from batch output root",
        )
        require(type(prior_cache_roots) is list, "explicit prior cache roots required")
        self.prior_caches = [_scratch(path, cache.ALLOWED_TASKS) for path in prior_cache_roots]
        roots = [self.current_cache, *self.prior_caches]
        require(len(set(roots)) == len(roots), "duplicate cache source")
        require(
            not any(a in b.parents for a in roots for b in roots if a != b),
            "overlapping cache sources",
        )
        require(isinstance(producer_sha256, dict), "producer context required")
        self.producers = copy.deepcopy(producer_sha256)
        self.allow_escalation = allow_escalation
        self.bounds_source = copy.deepcopy(bounds_source)
        self._context()
        self._bounds()

    def _context(self):
        require(sha256(self.cases_path) == self.cases_sha256, "casebook file changed")
        require(digest(self.book) == self.book_sha256, "casebook context changed")
        require(
            self.producers == provenance(include_cli=CLI_PRODUCER in self.producers),
            "producer context differs from actual source hashes",
        )
        _scratch(self.root, ("13.4.1",))
        _scratch(self.current_cache, ("13.4.1",))
        for root in self.prior_caches:
            _scratch(root, cache.ALLOWED_TASKS)

    def _row(self, row):
        self._context()
        require(isinstance(row, dict), "row must be a frozen convergence row")
        matches = [r for r in self.book["launches"] if r["id"] == row.get("id")]
        require(
            len(matches) == 1 and matches[0]["phase"] == "convergence",
            "wrong or unknown convergence phase",
        )
        _same(row, matches[0], "frozen row identity changed")
        return next(c for c in self.book["cases"] if c["id"] == row["case_id"])

    def reference_request(self, row):
        """Derive exact frozen coordinates, profile times and diagnostic times."""
        case = self._row(row)
        coordinates = artifacts.fortran.grid(case, row["nodes"])
        diagnostic_times, _ = metrics.schedule(case, row)
        arguments = (case, coordinates, row["analysis_times"], diagnostic_times)
        identity = cache.reference_identity(*arguments, self.allow_escalation)
        return arguments, identity

    def _reference_path(self, identity):
        return _path(self.root / "references" / (identity + ".json"), self.root)

    def _read_cache(self, root, arguments, identity, *, allow_work=False, require_admitted=False):
        entry = _path(root / cache.fingerprint(identity), root)
        before = _cache_bindings(entry) if not allow_work and entry.exists() else None
        # ReferenceCache authenticates stages before any further work, rejecting
        # failed/interrupted starts and orphan artifacts. Prior reads never resume.
        values, _diagnostics, admission = cache.ReferenceCache(root).reference(
            *arguments, allow_work=allow_work, allow_escalation=self.allow_escalation
        )
        require(admission.get("status") in ("admitted", "unmet"), "invalid reference admission")
        require(
            not require_admitted or admission["status"] == "admitted",
            "exact prior reference admission unmet",
        )
        case, coordinates, analysis_times, diagnostic_times = arguments
        require(
            values.get("schema_version") == "solute-metrics-reference-v1"
            and values.get("species") == [s["name"] for s in case["species"]],
            "reference schema/species coverage changed",
        )
        _same(values.get("coordinates"), coordinates, "reference coordinate coverage changed")
        require(
            [p["time"] for p in values["profiles"]] == [0, *analysis_times]
            and [p["time"] for p in values["continuum"]] == diagnostic_times,
            "reference time coverage changed",
        )
        bindings = _cache_bindings(entry)
        require(before is None or before == bindings, "reference artifacts changed during replay")
        require(
            {"identity.json", "analytical-work-ledger.jsonl", "reference.json"} <= set(bindings),
            "complete reference artifacts missing",
        )
        receipt = {
            "schema_version": "solute-convergence-reference-receipt-v1",
            "status": admission["status"],
            "identity": cache.fingerprint(identity),
            "request_sha256": cache.fingerprint(identity),
            "cache_root": str(root),
            "artifacts": bindings,
            "admission": admission,
            "reference_values_sha256": digest(values),
            "producer_sha256": self.producers,
        }
        return values, receipt

    def reference(self, row, *, allow_work=False, expected_receipt=None, write=False):
        """Replay a pinned source or select complete exact caches before authorized work."""
        require(type(allow_work) is bool and type(write) is bool, "explicit work/write policy")
        require(not allow_work or write, "read-only reference cannot authorize work")
        arguments, identity = self.reference_request(row)
        key = cache.fingerprint(identity)
        target = self._reference_path(key)
        if expected_receipt is None and target.exists():
            expected_receipt = artifacts.read_artifact(target)
        if expected_receipt is not None:
            stored = artifacts.read_artifact(target)
            _same(stored, expected_receipt, "selected reference receipt changed")
            root = cache.checked(expected_receipt.get("cache_root", ""))
            require(root in [self.current_cache, *self.prior_caches], "unapproved cache source")
            values, receipt = self._read_cache(
                root, arguments, identity, require_admitted=root in self.prior_caches
            )
            _same(receipt, expected_receipt, "reference source/artifact/admission binding changed")
        else:
            candidates = []
            for root in self.prior_caches:
                entry = _path(root / key, root)
                if entry.exists():
                    # Every present exact prior candidate is checked, even when an
                    # earlier source is valid. Invalid evidence never becomes a miss.
                    candidates.append(
                        self._read_cache(root, arguments, identity, require_admitted=True)
                    )
            if candidates:
                values, receipt = candidates[0]
                for other, _receipt in candidates[1:]:
                    _same(other, values, "conflicting complete exact prior references")
            else:
                entry = _path(self.current_cache / key, self.current_cache)
                if not entry.exists() and not allow_work:
                    return None, {
                        "status": "missing",
                        "identity": key,
                        "coverage": "complete exact reference absent",
                    }
                values, receipt = self._read_cache(
                    self.current_cache, arguments, identity, allow_work=allow_work
                )
        self._context()
        if write:
            artifacts.write_artifact(target, receipt, self.book)
        return copy.deepcopy(values), copy.deepcopy(receipt)

    def _bounds(self):
        if self.bounds_source is None:
            return {}
        metrics.keys(self.bounds_source, "path sha256 review_path review_sha256", "bounds source")
        source = _path(self.bounds_source["path"], ROOT)
        review_path = _path(self.bounds_source["review_path"], ROOT)
        require(source.is_file() and review_path.is_file(), "reviewed bounds source missing")
        require(
            sha256(source) == self.bounds_source["sha256"]
            and sha256(review_path) == self.bounds_source["review_sha256"],
            "reviewed bounds source changed",
        )
        payload = metrics.json_load(source)
        metrics.keys(
            payload,
            "schema_version cases_sha256 producer_sha256 bounds_by_launch",
            "bounds payload",
        )
        require(
            payload["schema_version"] == "solute-convergence-bounds-v1"
            and payload["cases_sha256"] == self.cases_sha256
            and payload["producer_sha256"] == self.producers,
            "bounds identity/producers changed",
        )
        review = metrics.json_load(review_path)
        metrics.keys(
            review,
            "schema_version cases_sha256 producer_sha256 bounds_path bounds_sha256 "
            "authorization evidence_sha256",
            "bounds review",
        )
        require(
            review["schema_version"] == "solute-convergence-bounds-review-v1"
            and review["cases_sha256"] == self.cases_sha256
            and review["producer_sha256"] == self.producers
            and review["bounds_path"] == str(source)
            and review["bounds_sha256"] == self.bounds_source["sha256"]
            and isinstance(review["authorization"], str)
            and review["authorization"].strip(),
            "bounds root-review/source binding changed",
        )
        evidence = review["evidence_sha256"]
        require(isinstance(evidence, dict) and evidence, "bounds review evidence absent")
        for path, expected in evidence.items():
            require(sha256(_path(path, ROOT)) == expected, "bounds review evidence changed")
        bounds = payload["bounds_by_launch"]
        require(isinstance(bounds, dict), "bounds row map required")
        for identity, entries in bounds.items():
            rows = [r for r in self.book["launches"] if r["id"] == identity]
            require(len(rows) == 1, "bounds row unknown")
            self._row(rows[0])
            require(isinstance(entries, list), "bounds entries must be a list")
            seen = set()
            case = next(c for c in self.book["cases"] if c["id"] == rows[0]["case_id"])
            times, _ = metrics.schedule(case, rows[0])
            for bound in entries:
                metrics.keys(
                    bound,
                    "species observable time arithmetic coordinate quadrature evidence",
                    "bound",
                )
                require(
                    bound["species"] in [s["name"] for s in case["species"]]
                    and bound["time"] in times
                    and isinstance(bound["observable"], str)
                    and bound["observable"] in OBSERVABLES
                    and isinstance(bound["evidence"], str)
                    and bound["evidence"].strip(),
                    "bound channel/evidence invalid",
                )
                metrics.finite(bound["time"], "bound time", nonnegative=True)
                require(
                    not bound["observable"].startswith("concentration.")
                    or bound["time"] in [0, *rows[0]["analysis_times"]],
                    "bound concentration time coverage invalid",
                )
                key = (bound["species"], bound["observable"], bound["time"])
                require(key not in seen, "duplicate bound channel")
                seen.add(key)
                for name in ("arithmetic", "coordinate", "quadrature"):
                    if bound[name] is not None:
                        metrics.finite(bound[name], name + " bound", nonnegative=True)
        return copy.deepcopy(bounds)

    def _result_path(self, row):
        return _path(self.root / "results" / (row["id"] + ".json"), self.root)

    def derive(self, row, run, reference_values, receipt, *, write=False):
        """Authenticate run/reference receipts and independently reproduce measured metrics."""
        require(type(write) is bool, "explicit result write policy required")
        require(isinstance(receipt, dict), "reference receipt required")
        self._row(row)
        run = _path(run, self.root / "runs")
        require(run.parent == self.root / "runs", "run must be a direct batch child")
        manager = lifecycle.Lifecycle(self.cases_path, self.root / "runs")
        require(manager.phase == "convergence", "complete run uses wrong phase")
        manifest = manager.verify(run, require_complete=True)
        require(
            manifest["launch_id"] == row["id"]
            and manifest["run_id"] == run.name
            and manifest["solver"] == row["solver"]
            and manifest["precision"] == row["precision"]
            and manifest["status"] in ("complete", "raw_complete"),
            "complete run identity differs from row",
        )
        values, authenticated_receipt = self.reference(row, expected_receipt=receipt)
        require(authenticated_receipt["status"] == "admitted", "metrics require admitted reference")
        _same(reference_values, values, "caller reference values differ from authenticated replay")
        before = artifacts.run_binding(run)
        observations = metrics.load_observations(
            run / "observations.json", run / "manifest.json", self.cases_path
        )
        measured = metrics.compute_metrics(observations, values)
        require(before == artifacts.run_binding(run), "run artifacts changed during metrics replay")
        require(
            measured["raw"]["launch_id"] == row["id"]
            and measured["raw"]["run_id"] == run.name
            and measured["raw"]["cases_sha256"] == self.cases_sha256,
            "loaded observation identity differs from authenticated run",
        )
        result = {
            "kind": "convergence",
            "launch_id": row["id"],
            "status": "metrics_complete",
            "run": before,
            "reference": authenticated_receipt,
            "metrics": measured,
            "producer_sha256": copy.deepcopy(self.producers),
            "scientific_accuracy_accepted": False,
            "thresholds": None,
            "m5v_complete": False,
        }
        bounds = self._bounds().get(row["id"], [])
        record = {"result": result, "reference_values": values, "bounds": bounds}
        trusted = {
            "book_sha256": self.book_sha256,
            "row_sha256": digest(row),
            "record_sha256": digest(record),
            "result_sha256": digest(result),
            "run_sha256": digest(before),
            "reference_sha256": digest(authenticated_receipt),
            "reference_values_sha256": digest(values),
            "producer_sha256": copy.deepcopy(self.producers),
        }
        self._context()
        target = self._result_path(row)
        if write:
            artifacts.write_artifact(target, result, self.book)
        else:
            _same(
                artifacts.read_artifact(target), result, "independent metrics/result replay changed"
            )
        return copy.deepcopy(record), copy.deepcopy(trusted)

    def classify(self, rows, completed_runs, *, write=False):
        """Replay exact ordered evidence; never accept caller-supplied metric expectations."""
        require(type(write) is bool and isinstance(rows, list) and rows, "classification inputs")
        ids = [r["id"] for r in rows]
        require(len(set(ids)) == len(ids), "duplicate classification rows")
        require(
            isinstance(completed_runs, dict) and set(completed_runs) == set(ids), "run coverage"
        )
        records, trusted, bindings = [], {}, []
        for row in rows:
            _, identity = self.reference_request(row)
            reference_path = self._reference_path(cache.fingerprint(identity))
            receipt = artifacts.read_artifact(reference_path)
            values, _ = self.reference(row, expected_receipt=receipt)
            record, expected = self.derive(
                row, completed_runs[row["id"]], values, receipt, write=write
            )
            records.append(record)
            trusted[row["id"]] = expected
            result_path = self._result_path(row)
            bindings.append(
                {
                    "launch_id": row["id"],
                    "row_sha256": digest(row),
                    "run_dir": str(_path(completed_runs[row["id"]], self.root / "runs")),
                    "result_path": str(result_path),
                    "result_sha256": sha256(result_path),
                    "reference_path": str(reference_path),
                    "reference_sha256": sha256(reference_path),
                    "record_sha256": expected["record_sha256"],
                }
            )
        payload = {
            "schema_version": "solute-convergence-classification-receipt-v1",
            "batch": str(self.root),
            "cases_sha256": self.cases_sha256,
            "producer_sha256": copy.deepcopy(self.producers),
            "bounds_source": copy.deepcopy(self.bounds_source),
            "ordered_evidence": bindings,
            "analysis": analysis.analyze_convergence(self.book, records, trusted),
            "filesystem_receipts_authenticated": True,
            "scientific_accuracy_accepted": False,
            "thresholds": None,
            "m5v_complete": False,
        }
        target = _path(self.root / "classification.json", self.root)
        self._context()
        if write:
            artifacts.write_artifact(target, payload, self.book)
        else:
            _same(artifacts.read_artifact(target), payload, "classification replay changed")
        return payload

    def verify_summary(self, summary, rows, completed_runs):
        """Authenticate a complete summary without consulting current launch counts."""
        metrics.keys(
            summary,
            "schema_version batch status classification results references launches "
            "producer_sha256 scientific_accuracy_accepted thresholds m5v_complete",
            "convergence summary",
        )
        require(
            summary["schema_version"] == "solute-convergence-summary-v1"
            and summary["batch"] == str(self.root)
            and summary["status"] in ("evidence_complete", "evidence_verified")
            and summary["producer_sha256"] == self.producers
            and summary["scientific_accuracy_accepted"] is False
            and summary["thresholds"] is None
            and summary["m5v_complete"] is False,
            "summary identity/scientific status changed",
        )
        payload = self.classify(rows, completed_runs)
        path = self.root / "classification.json"
        _same(
            summary["classification"],
            {"path": str(path), "sha256": sha256(path), "payload_sha256": digest(payload)},
            "summary classification binding changed",
        )
        for namespace in ("results", "references"):
            prefix = "result" if namespace == "results" else "reference"
            expected = [
                {
                    "launch_id": entry["launch_id"],
                    "path": entry[prefix + "_path"],
                    "sha256": entry[prefix + "_sha256"],
                }
                for entry in payload["ordered_evidence"]
            ]
            _same(summary[namespace], expected, "summary ordered " + namespace + " binding changed")
        counts = summary["launches"]
        metrics.keys(counts, "consumed remaining cap", "historical launch counts")
        cap = self.book["budgets"]["launch_cap"]
        require(
            all(type(value) is int for value in counts.values())
            and counts["cap"] == cap
            and 0 <= counts["consumed"] <= cap
            and counts["remaining"] == cap - counts["consumed"],
            "historical budget counts invalid",
        )
        return payload

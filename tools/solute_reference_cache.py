"""Authenticated, resumable analytical work; no solver processes or automatic retries.

ReferenceCache(root).reference(case, coordinates, analysis_times, diagnostic_times)
is read-only by default. Explicit allow_work=True permits new requests/stages,
but never repeats an interrupted stage. Returned scientific admission may be
``unmet``. Source or request changes select different identities; old evidence
is never migrated, overwritten or used as partial coverage.
"""

import copy
import datetime
import decimal
import fcntl
import hashlib
import json
import math
import os
import platform
import sys
from contextlib import contextmanager
from pathlib import Path

from tools import solute_analytical_reference as analytical

ROOT = Path(__file__).resolve().parents[1]
SOURCE_FILES = (
    "tools/solute_analytical_reference.py",
    "tools/solute_reference_cache.py",
    "tools/solute_verification_metrics.py",
    "tools/check_solute_verification_spec.py",
    "planning/master/SOLUTE_VERIFICATION_SPEC.md",
)
ALLOWED_TASKS = ("13.3.5", "13.3.1", "13.4.1", "13.6.1")
require = analytical.require


def encode(value, *, sort_maps=False):
    """Tag every type, retaining binary floats and Decimal exponent/coefficient."""
    if value is None:
        return ["none"]
    if type(value) is bool:
        return ["bool", value]
    if type(value) is int:
        return ["int", str(value)]
    if type(value) is float:
        require(math.isfinite(value), "nonfinite cache float")
        return ["float", value.hex()]
    if isinstance(value, decimal.Decimal):
        require(value.is_finite(), "nonfinite cache Decimal")
        return ["decimal", str(value)]
    if type(value) is str:
        return ["str", value]
    if type(value) in (list, tuple):
        return [
            "list" if type(value) is list else "tuple",
            [encode(x, sort_maps=sort_maps) for x in value],
        ]
    if type(value) is dict:
        pairs = [
            [encode(k, sort_maps=sort_maps), encode(v, sort_maps=sort_maps)]
            for k, v in value.items()
        ]
        # Stages retain insertion order because diagnostics iterate scalar maps.
        # Identities sort maps separately so authored JSON member order is irrelevant.
        return ["dict", sorted(pairs, key=lambda p: canonical(p[0])) if sort_maps else pairs]
    raise ValueError(f"unsupported cache type {type(value).__name__}")


def decode(value):
    require(type(value) is list and value, "invalid tagged cache value")
    tag = value[0]
    if tag == "none":
        require(len(value) == 1, "invalid none tag")
        return None
    require(len(value) == 2, "invalid cache tag arity")
    raw = value[1]
    if tag in ("str", "int", "float", "decimal"):
        require(type(raw) is str, "invalid scalar tag")
        result = {"str": str, "int": int, "float": float.fromhex, "decimal": decimal.Decimal}[tag](
            raw
        )
    elif tag == "bool":
        require(type(raw) is bool, "invalid boolean tag")
        result = raw
    elif tag in ("list", "tuple"):
        require(type(raw) is list, "invalid sequence tag")
        result = [decode(x) for x in raw]
        if tag == "tuple":
            result = tuple(result)
    elif tag == "dict":
        require(type(raw) is list, "invalid dictionary tag")
        result = {}
        for pair in raw:
            require(type(pair) is list and len(pair) == 2, "invalid dictionary pair")
            key, item = map(decode, pair)
            try:
                require(key not in result, "duplicate decoded dictionary key")
                result[key] = item
            except TypeError as error:
                raise ValueError("unhashable cache key") from error
    else:
        raise ValueError("unknown cache tag")
    require(encode(result) == value, "noncanonical cache encoding")
    return result


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("ascii")


def fingerprint(value):
    return hashlib.sha256(canonical(encode(value, sort_maps=True))).hexdigest()


def file_hash(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key")
        result[key] = value
    return result


def read_json(path):
    checked(path)
    return json.loads(
        path.read_bytes(), object_pairs_hook=unique_object, parse_constant=reject_constant
    )


def reject_constant(value):
    raise ValueError(f"nonfinite JSON value {value}")


def checked(path):
    path = Path(path).absolute()
    require(".." not in path.parts, "cache path traversal")
    require(not any(p.is_symlink() for p in (path, *path.parents)), "cache symlink rejected")
    return path


def sync_directory(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def write_new(path, data):
    checked(path)
    with path.open("xb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    sync_directory(path.parent)


def mkdir_durable(path):
    """Persist newly created directory links before any work can be recorded."""
    missing = []
    current = checked(path)
    while not current.exists():
        missing.append(current)
        current = current.parent
    require(current.is_dir(), "cache parent is not a directory")
    for directory in reversed(missing):
        checked(directory)
        directory.mkdir(exist_ok=True)
        sync_directory(directory.parent)


def runtime_identity():
    context = decimal.getcontext()
    return {
        "python": sys.version,
        "implementation": platform.python_implementation(),
        "platform": platform.platform(),
        "decimal": decimal.__version__,
        "libmpdec": decimal.__libmpdec_version__,
        "context": {
            name: getattr(context, name)
            for name in ("prec", "rounding", "Emin", "Emax", "capitals", "clamp")
        },
        "traps": {signal.__name__: enabled for signal, enabled in context.traps.items()},
    }


def reference_identity(case, coordinates, analysis_times, diagnostic_times, allow_escalation):
    analytical._validate_inputs(
        case, coordinates, analysis_times, diagnostic_times, allow_finite=True
    )
    require(type(allow_escalation) is bool, "explicit escalation flag required")
    return {
        "schema_version": "solute-reference-cache-v1",
        "case_sha256": hashlib.sha256(canonical(case)).hexdigest(),
        "case": copy.deepcopy(case),
        "coordinates": list(coordinates),
        "analysis_times": list(analysis_times),
        "diagnostic_times": list(diagnostic_times),
        "settings": copy.deepcopy(case["analytical"]),
        "allow_escalation": allow_escalation,
        "runtime": runtime_identity(),
        "source_sha256": {name: file_hash(ROOT / name) for name in SOURCE_FILES},
    }


class ReferenceCache:
    """One confined cache with serialized writers and immutable per-request evidence."""

    def __init__(self, root):
        self.root = checked(root)
        require(
            any(
                self.root == ROOT / ".development" / task
                or ROOT / ".development" / task in self.root.parents
                for task in ALLOWED_TASKS
            ),
            "cache outside authorized scratch",
        )

    @contextmanager
    def _lock(self, allow_work):
        checked(self.root)
        if allow_work:
            mkdir_durable(self.root)
        require(self.root.is_dir(), "reference cache missing")
        path = checked(self.root / "cache.lock")
        with path.open("a+b" if allow_work else "rb") as stream:
            fcntl.flock(stream, fcntl.LOCK_EX if allow_work else fcntl.LOCK_SH)
            yield

    def reference(
        self,
        case,
        coordinates,
        analysis_times,
        diagnostic_times,
        *,
        allow_work=False,
        allow_escalation=False,
    ):
        """Return (reference, diagnostics-or-None, admission), retaining unmet results."""
        require(type(allow_work) is bool, "explicit work flag required")
        identity = reference_identity(
            case, coordinates, analysis_times, diagnostic_times, allow_escalation
        )
        with self._lock(allow_work):
            entry = _Entry(self.root, identity, allow_work)
            return entry.reference()


class _Entry:
    def __init__(self, root, identity, allow_work):
        self.identity = identity
        self.key = fingerprint(identity)
        self.path = checked(root / self.key)
        self.allow_work = allow_work
        self.sequence = []
        if not self.path.exists():
            require(allow_work, "matching reference cache missing")
            self.path.mkdir()
            sync_directory(root)
            write_new(self.path / "identity.json", canonical(encode(identity)))
            write_new(self.path / "analytical-work-ledger.jsonl", b"")
        stored = decode(read_json(self.path / "identity.json"))
        require(fingerprint(stored) == self.key, "reference identity mismatch")
        self.ledger = checked(self.path / "analytical-work-ledger.jsonl")
        self.records = self._records()
        self._verify_records()

    def _records(self):
        content = self.ledger.read_bytes()
        require(not content or content.endswith(b"\n"), "truncated analytical ledger")
        records = []
        previous = None
        for line in content.splitlines():
            row = json.loads(line, object_pairs_hook=unique_object, parse_constant=reject_constant)
            require(
                isinstance(row, dict)
                and set(row) == {"sequence", "event", "stage", "utc", "detail", "previous"},
                "invalid analytical ledger fields",
            )
            require(
                type(row["sequence"]) is int and row["sequence"] == len(records) + 1,
                "invalid analytical ledger sequence",
            )
            require(row["previous"] == previous, "analytical ledger hash chain mismatch")
            require(isinstance(row["utc"], str) and row["utc"], "missing work timestamp")
            try:
                stamp = datetime.datetime.fromisoformat(row["utc"])
                require(stamp.tzinfo is not None, "work timestamp needs timezone")
            except ValueError as error:
                raise ValueError("invalid work timestamp") from error
            fields = {
                "start": {"variant", "order", "digits"},
                "complete": {"sha256"},
                "failed": {"error"},
                "reference_complete": {"sha256", "status"},
            }
            event, detail = row["event"], row["detail"]
            require(type(event) is str and event in fields, "unknown analytical work event")
            require(
                isinstance(detail, dict) and set(detail) == fields[event],
                "invalid work event detail",
            )
            if "sha256" in detail:
                require(
                    type(detail["sha256"]) is str
                    and len(detail["sha256"]) == 64
                    and all(c in "0123456789abcdef" for c in detail["sha256"]),
                    "invalid work artifact hash",
                )
            if event == "start":
                require(
                    detail["variant"]
                    in ("full", "escalation", "precision", "binary_inputs", "component"),
                    "invalid stage variant",
                )
                require(
                    all(type(detail[k]) is int or detail[k] is None for k in ("order", "digits")),
                    "invalid work order/precision",
                )
            if event == "failed":
                require(type(detail["error"]) is str and detail["error"], "missing work failure")
            if event == "reference_complete":
                require(detail["status"] in ("admitted", "unmet"), "invalid admission status")
            require(
                (
                    row["stage"] is None
                    if event == "reference_complete"
                    else type(row["stage"]) is str
                    and len(row["stage"]) == 64
                    and all(c in "0123456789abcdef" for c in row["stage"])
                ),
                "invalid stage hash",
            )
            previous = hashlib.sha256(canonical(row)).hexdigest()
            records.append(row)
        return records

    def _verify_records(self):
        starts, completed, finished = {}, set(), False
        for row in self.records:
            event, stage = row["event"], row["stage"]
            require(not finished, "events after reference completion")
            if event == "start":
                require(
                    isinstance(stage, str)
                    and len(stage) == 64
                    and all(c in "0123456789abcdef" for c in stage),
                    "invalid stage hash",
                )
                require(stage not in starts, "duplicate stage start")
                starts[stage] = row
            elif event in ("complete", "failed"):
                require(stage in starts and stage not in completed, "invalid stage completion")
                require(event != "failed", "failed analytical stage cannot be retried")
                artifact = checked(self.path / f"stage-{stage}.json")
                require(file_hash(artifact) == row["detail"]["sha256"], "stage hash mismatch")
                payload = read_json(artifact)
                require(
                    isinstance(payload, dict)
                    and set(payload) == {"identity", "request", "runtime", "result"},
                    "invalid stage artifact",
                )
                request = decode(payload["request"])
                require(isinstance(request, dict), "invalid stage request")
                require(
                    starts[stage]["detail"]
                    == {
                        "variant": request.get("variant"),
                        "order": request.get("order"),
                        "digits": request.get("digits"),
                    },
                    "stage start metadata mismatch",
                )
                require(payload["identity"] == self.key, "stage reference identity mismatch")
                require(
                    fingerprint([self.key, request, decode(payload["runtime"])]) == stage,
                    "stage request hash mismatch",
                )
                result = decode(payload["result"])
                self._validate_stage(request, result)
                completed.add(stage)
            elif event == "reference_complete":
                require(stage is None and set(starts) == completed, "incomplete reference stages")
                require(
                    file_hash(checked(self.path / "reference.json")) == row["detail"]["sha256"],
                    "reference hash mismatch",
                )
                finished = True
            else:
                raise ValueError("unknown analytical work event")
        require(
            set(starts) == completed, "interrupted analytical stage; explicit revision required"
        )
        expected = {f"stage-{stage}.json" for stage in completed}
        actual = {p.name for p in self.path.glob("stage-*.json")}
        require(actual == expected, "orphan analytical stage artifact")
        require(
            not (self.path / "reference.json").exists() or finished,
            "incomplete reference publication",
        )

    def _append(self, event, stage, detail):
        require(self.allow_work, "read-only reference work")
        row = {
            "sequence": len(self.records) + 1,
            "event": event,
            "stage": stage,
            "utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "detail": detail,
            "previous": (
                hashlib.sha256(canonical(self.records[-1])).hexdigest() if self.records else None
            ),
        }
        with checked(self.ledger).open("ab") as stream:
            stream.write(canonical(row) + b"\n")
            stream.flush()
            os.fsync(stream.fileno())
        self.records.append(row)

    def _coverage(self, reference, diagnostics):
        identity = self.identity
        require(
            encode(reference["coordinates"]) == encode(identity["coordinates"]),
            "reference coordinate coverage mismatch",
        )
        profile_times = (
            [0, *identity["analysis_times"]]
            if diagnostics is not None
            else [t for t in identity["diagnostic_times"] if t in [0, *identity["analysis_times"]]]
        )
        require(
            encode([p["time"] for p in reference["profiles"]]) == encode(profile_times),
            "reference analysis coverage mismatch",
        )
        require(
            encode([p["time"] for p in reference["continuum"]])
            == encode(identity["diagnostic_times"]),
            "reference diagnostic coverage mismatch",
        )
        return analytical.reference_admission(identity["case"], reference, diagnostics)

    def _validate_stage(self, request, result):
        if request.get("variant") == "component":
            require(
                encode(request) == encode({"variant": "component", "identity": self.key}),
                "component stage identity mismatch",
            )
            self._coverage(result, None)
        else:
            analytical.validate_chain_stage(request, result)

    def _stage(self, request, calculate):
        runtime = runtime_identity()
        key = fingerprint([self.key, request, runtime])
        self.sequence.append(key)
        found = [r for r in self.records if r["stage"] == key and r["event"] == "complete"]
        if found:
            require(len(found) == 1, "duplicate stage completion")
            return decode(read_json(self.path / f"stage-{key}.json")["result"])
        require(self.allow_work, "required analytical stage missing")
        require(not any(r["stage"] == key for r in self.records), "stage already attempted")
        artifact = self.path / f"stage-{key}.json"
        require(not artifact.exists(), "orphan analytical stage")
        self._append(
            "start",
            key,
            {
                "variant": request["variant"],
                "order": request.get("order"),
                "digits": request.get("digits"),
            },
        )
        try:
            result = calculate()
            self._validate_stage(request, result)
            payload = {
                "identity": self.key,
                "request": encode(request),
                "runtime": encode(runtime),
                "result": encode(result),
            }
            write_new(artifact, canonical(payload))
            self._append("complete", key, {"sha256": file_hash(artifact)})
            return result
        except Exception as error:
            self._append("failed", key, {"error": f"{type(error).__name__}: {error}"})
            raise

    def reference(self):
        identity = self.identity
        arguments = [
            identity[k] for k in ("case", "coordinates", "analysis_times", "diagnostic_times")
        ]
        complete = bool(self.records and self.records[-1]["event"] == "reference_complete")
        require(complete or self.allow_work, "complete reference missing")
        if complete:
            # Replay only authenticated stages to reproduce assembly/admission.
            self.allow_work = False
        if identity["case"]["family"] == "finite_chain":
            reference, diagnostics = analytical.finite_chain_reference(
                *arguments,
                allow_escalation=identity["allow_escalation"],
                stage_executor=self._stage,
            )
        else:
            reference = self._stage(
                {"variant": "component", "identity": self.key},
                lambda: analytical.component_reference(*arguments),
            )
            diagnostics = None
        admission = self._coverage(reference, diagnostics)
        require(
            self.sequence == [r["stage"] for r in self.records if r["event"] == "start"],
            "analytical stage sequence mismatch",
        )
        payload = {
            "identity": self.key,
            "reference": reference,
            "diagnostics": diagnostics,
            "admission": admission,
            "stages": self.sequence,
        }
        require(
            identity["source_sha256"] == {n: file_hash(ROOT / n) for n in SOURCE_FILES},
            "analytical source changed during work",
        )
        if complete:
            require(
                self.records[-1]["detail"]["status"] == admission["status"],
                "reference completion admission mismatch",
            )
            require(
                canonical(read_json(self.path / "reference.json")) == canonical(payload),
                "reference/admission reproduction mismatch",
            )
        else:
            write_new(self.path / "reference.json", canonical(payload))
            self._append(
                "reference_complete",
                None,
                {"sha256": file_hash(self.path / "reference.json"), "status": admission["status"]},
            )
        return reference, diagnostics, admission

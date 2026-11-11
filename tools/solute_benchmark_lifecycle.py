"""Bounded benchmark lifecycle shared by explicitly activated study phases.

No operation launches a solver except Lifecycle.execute with an exact activation
record. Compilation has separate immutable evidence and consumes no launch row.
Scientific bridge/reference gates remain the caller's responsibility.
"""

import fcntl
import hashlib
import json
import os
import platform
import re
import shutil
import signal
import subprocess
import time
import uuid
from contextlib import contextmanager
from datetime import datetime
from pathlib import Path

from tools import check_solute_verification_spec as contract
from tools import run_solute_verification_reference as fortran
from tools import solute_verification_metrics as metrics

SOURCE_ROOT = Path(__file__).resolve().parents[1]
ROOT = SOURCE_ROOT
PHASE_TASKS = dict(fortran.PHASE_TASKS)
ENV_KEYS = (
    "HYDRUS_SOLUTE_VERIFICATION_REQUEST",
    "HYDRUS_SOLUTE_VERIFICATION_REQUEST_SHA256",
    "HYDRUS_SOLUTE_VERIFICATION_OUTPUT_ROOT",
    "HYDRUS_SOLUTE_VERIFICATION_OUT",
)
PROFILE = {"debug": 0, "opt-level": 0, "debug-assertions": True, "overflow-checks": True}
RUST_FLAGS = ["-Cdebuginfo=0", "-Copt-level=0", "-Cdebug-assertions=yes", "-Coverflow-checks=yes"]
ADAPTERS = (
    "tools/solute_benchmark_lifecycle.py",
    "tools/run_solute_verification_reference.py",
    "tools/solute_verification_metrics.py",
    "tools/check_solute_verification_spec.py",
)
HASH = re.compile(r"[0-9a-f]{64}\Z")
require = fortran.require
safe = fortran.safe_path
sha256 = fortran.sha256
write_json = fortran.write_json


def output_root(value):
    require(".." not in Path(value).parts, "output path traversal")
    path = Path(value).absolute()
    try:
        relative = path.relative_to(ROOT)
    except ValueError as error:
        raise ValueError("output root outside checkout") from error
    require(
        len(relative.parts) >= 2
        and relative.parts[0] == ".development"
        and relative.parts[1] in PHASE_TASKS,
        "execution root outside scientific phase scratch",
    )
    safe(ROOT, relative)
    return path, ROOT / ".development" / relative.parts[1]


def selected_rows(book, phase, launch_ids=None):
    rows = [
        r
        for r in book["launches"]
        if r["phase"] == phase and (launch_ids is None or r["id"] in launch_ids)
    ]
    require(rows, "no matching phase rows")
    if launch_ids is not None:
        require(
            type(launch_ids) is list
            and all(type(x) is str for x in launch_ids)
            and len(set(launch_ids)) == len(launch_ids)
            and {r["id"] for r in rows} == set(launch_ids),
            "unknown/duplicate/wrong-phase row",
        )
    return rows


BASE_ACTIVATION_KEYS = frozenset({"cases_sha256", "phase", "launch_ids", "kind", "authorization"})
TIMEOUT_OVERRIDE_RANGE = (60, 300)
# Same object as the casebook loader's overrides; the CLI updates it in place.
TIMEOUT_OVERRIDES = contract.TIMEOUT_OVERRIDES


def validate_timeout_overrides(overrides, launch_ids):
    require(isinstance(overrides, dict), "timeout overrides must be a mapping")
    require(set(overrides) <= set(launch_ids), "timeout override outside activation rows")
    for launch_id, seconds in overrides.items():
        require(
            type(seconds) is int and TIMEOUT_OVERRIDE_RANGE[0] <= seconds <= TIMEOUT_OVERRIDE_RANGE[1],
            "invalid timeout override",
        )


def apply_timeout_overrides(book, overrides):
    launches = {row["id"]: row for row in book["launches"]}
    for launch_id, seconds in overrides.items():
        require(launch_id in launches, "timeout override for unknown launch")
        launches[launch_id]["timeout_seconds"] = seconds


def validate_activation(book, cases_path, phase, rows, activation):
    require(
        isinstance(activation, dict)
        and set(activation) in (BASE_ACTIVATION_KEYS, BASE_ACTIVATION_KEYS | {"timeout_overrides"}),
        "explicit activation required",
    )
    require(
        activation["cases_sha256"] == sha256(cases_path)
        and activation["phase"] == phase
        and activation["launch_ids"] == [r["id"] for r in rows],
        "activation identity mismatch",
    )
    require(
        type(activation["authorization"]) is str and activation["authorization"].strip(),
        "activation authorization reference missing",
    )
    roles = {r["role"] == "bridge" for r in rows}
    require(len(roles) == 1, "bridge and dependent rows need separate activation")
    kind = "bridge" if True in roles else phase
    require(activation["kind"] == kind, "activation kind mismatch")
    validate_timeout_overrides(activation.get("timeout_overrides", {}), activation["launch_ids"])


def ledger_records(path, book, phase):
    try:
        rows = fortran.ledger_records(path)
    except (TypeError, KeyError, AttributeError) as error:
        raise ValueError("malformed shared launch ledger") from error
    by_id = {row["id"]: row for row in book["launches"]}
    states, starts = {}, set()
    for record in rows:
        launch, run, event, detail = (record[k] for k in ("launch_id", "run_id", "event", "detail"))
        require(
            type(launch) is str and launch in by_id and by_id[launch]["phase"] == phase,
            "ledger row outside phase",
        )
        require(
            type(run) is str and re.fullmatch(r"[A-Za-z0-9_.-]+", run) and run not in (".", ".."),
            "invalid ledger run identity",
        )
        require(isinstance(detail, dict), "invalid ledger detail")
        try:
            stamp = datetime.fromisoformat(record["utc"].replace("Z", "+00:00"))
            require(stamp.tzinfo is not None, "ledger timestamp timezone missing")
        except (AttributeError, TypeError, ValueError) as error:
            raise ValueError("invalid ledger timestamp") from error
        state = states.setdefault(
            run, {"launch": launch, "started": False, "exit": None, "terminal": None}
        )
        require(state["launch"] == launch, "ledger run changed launch identity")
        if event == "start":
            require(
                not state["started"] and state["terminal"] is None and launch not in starts,
                "duplicate or repeated launch start",
            )
            require(
                detail
                == {
                    "timeout_seconds": detail.get("timeout_seconds"),
                    "precision": by_id[launch]["precision"],
                    "estimated_output_bytes": by_id[launch]["estimated_output_bytes"],
                }
                and detail.get("timeout_seconds") in {
                    by_id[launch]["timeout_seconds"],
                    contract.BASE_TIMEOUTS.get(launch, by_id[launch]["timeout_seconds"]),
                    contract.APPROVED_TIMEOUT_OVERRIDES.get(launch),
                },
                "ledger start controls mismatch",
            )
            state["started"] = True
            starts.add(launch)
        elif event == "exit":
            require(
                state["started"]
                and state["exit"] is None
                and state["terminal"] is None
                and set(detail) == {"exit_code"}
                and type(detail["exit_code"]) is int,
                "invalid exit event",
            )
            state["exit"] = detail["exit_code"]
        elif event == "complete":
            require(
                state["started"]
                and state["exit"] == 0
                and state["terminal"] is None
                and set(detail) == {"status", "manifest_sha256"}
                and detail["status"] in ("complete", "raw_complete")
                and type(detail["manifest_sha256"]) is str
                and HASH.fullmatch(detail["manifest_sha256"]),
                "invalid completion event",
            )
            state["terminal"] = detail["status"]
        else:
            require(
                state["terminal"] is None
                or state["terminal"] == "raw_complete"
                and event == "validation_failed"
                and detail.get("stage") == "normalization",
                "events after terminal failure/completion",
            )
            require(type(detail.get("stage")) is str, "failure stage missing")
            if event == "timeout":
                require(
                    type(detail.get("timeout_seconds")) in (int, float)
                    and 0 < detail["timeout_seconds"] <= 300,
                    "invalid timeout event",
                )
            else:
                require(
                    type(detail.get("error")) is str and detail["error"], "failure reason missing"
                )
            state["terminal"] = event
    return rows


def has_caps(book):
    """v1 casebooks carry launch and storage caps; v2 (ADR 0003) carries none."""
    return "launch_cap" in book["budgets"]


def row_cap(book, row):
    """v1 bounds a row at twice its estimate; v2 (ADR 0003) records estimates without a ceiling."""
    return 2 * row["estimated_output_bytes"] if has_caps(book) else float("inf")


def storage_cap(book):
    return book["budgets"]["storage_bytes_cap"] if has_caps(book) else float("inf")


def book_tasks(book):
    """Phase roots belonging to the casebook generation (v1 13.x, v2 22-*, 23-*)."""
    v2 = not has_caps(book)
    return {t: p for t, p in PHASE_TASKS.items() if t.startswith(("22-", "23-")) == v2}


def scope_task(task):
    """Campaign roots (23-*) consume launch rows only within their own ledger."""
    return task if task.startswith("23-") else None


def all_records(book, task=None):
    records = []
    tasks = book_tasks(book) if task is None else {task: PHASE_TASKS[task]}
    for name, phase in tasks.items():
        records.extend(
            ledger_records(safe(ROOT, f".development/{name}/launch-ledger.jsonl"), book, phase)
        )
    if task is None:
        adapter = safe(ROOT, ".development/13.2.1/launch-ledger.jsonl")
        require(not fortran.ledger_records(adapter), "producer task has unexpected scientific ledger")
    starts = [r for r in records if r["event"] == "start"]
    require(len({r["launch_id"] for r in starts}) == len(starts), "duplicate global launch row")
    require(len({r["run_id"] for r in starts}) == len(starts), "duplicate global run id")
    return records


def total_storage(book=None):
    tasks = PHASE_TASKS if book is None else book_tasks(book)
    return sum(fortran.directory_bytes(safe(ROOT, f".development/{task}")) for task in tasks)


def check_budget(book, row, task=None):
    records = all_records(book, task)
    starts = [r for r in records if r["event"] == "start"]
    by_id = {r["id"]: r for r in book["launches"]}
    require(not any(r["launch_id"] == row["id"] for r in starts), "launch row already consumed")
    phase_count = sum(by_id[r["launch_id"]]["phase"] == row["phase"] for r in starts)
    reserved = sum(2 * by_id[r["launch_id"]]["estimated_output_bytes"] for r in starts)
    if has_caps(book):
        require(len(starts) < book["budgets"]["launch_cap"], "global launch cap exhausted")
        require(phase_count < book["budgets"]["phase_caps"][row["phase"]], "phase cap exhausted")
        require(
            reserved + 2 * row["estimated_output_bytes"] <= storage_cap(book),
            "global storage reservation exceeded",
        )
    return {
        "consumed": len(starts),
        "remaining": book["budgets"]["launch_cap"] - len(starts) if has_caps(book) else None,
        "phase_consumed": phase_count,
        "reserved_bytes": reserved,
    }


def source_hashes(book):
    names = {
        "Cargo.toml",
        "Cargo.lock",
        "crates/hydrus-core/Cargo.toml",
        "crates/hydrus-core/tests/solute_verification_harness.rs",
    }
    names.update(
        str(p.relative_to(SOURCE_ROOT))
        for p in (SOURCE_ROOT / "crates/hydrus-core/src").rglob("*.rs")
    )
    names.update(
        name for name in ("rust-toolchain", "rust-toolchain.toml") if (SOURCE_ROOT / name).exists()
    )
    sources = dict(book["provenance"]["source_sha256"])
    sources.update({name: sha256(safe(SOURCE_ROOT, name)) for name in sorted(names)})
    return sources


def controlled_environment():
    env = os.environ.copy()
    for key in ENV_KEYS:
        env.pop(key, None)
    forbidden = {
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_RUSTFLAGS",
        "RUSTDOCFLAGS",
    }
    require(
        not any(
            k in forbidden or k.startswith(("CARGO_PROFILE_", "CARGO_TARGET_", "CARGO_BUILD_"))
            for k in env
        ),
        "unaccounted Rust build environment override",
    )
    homes = [
        SOURCE_ROOT,
        *SOURCE_ROOT.parents,
        Path(env.get("CARGO_HOME", str(Path.home() / ".cargo"))).parent,
    ]
    configs = [p / ".cargo" / name for p in homes for name in ("config", "config.toml")]
    if "CARGO_HOME" in env:
        configs += [Path(env["CARGO_HOME"]) / name for name in ("config", "config.toml")]
    require(
        not any(p.exists() or p.is_symlink() for p in configs), "unreviewed Cargo configuration"
    )
    return env


def cargo_command(target):
    return [
        "cargo",
        "test",
        "--package",
        "hydrus-core",
        "--test",
        "solute_verification_harness",
        "--no-run",
        "--message-format=json",
        "--verbose",
        "--verbose",
        "--locked",
        "--target-dir",
        str(target),
        *[
            part
            for name, value in PROFILE.items()
            for part in ("--config", f"profile.test.{name}={str(value).lower()}")
        ],
    ]


def run_bounded(
    command, cwd, storage_root, timeout, byte_cap, env, stdout_path, stderr_path, global_cap
):
    """Bound a child process group without keeping output in memory."""
    with safe(stdout_path.parent, stdout_path.name).open("xb") as stdout, safe(
        stderr_path.parent, stderr_path.name
    ).open("xb") as stderr:
        process = subprocess.Popen(
            command, cwd=cwd, env=env, stdout=stdout, stderr=stderr, start_new_session=True
        )
        deadline = time.monotonic() + timeout
        try:
            while True:
                require(
                    fortran.directory_bytes(storage_root) <= byte_cap, "local storage cap exceeded"
                )
                require(total_storage() <= global_cap, "global physical storage cap exceeded")
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise subprocess.TimeoutExpired(command, timeout)
                try:
                    code = process.wait(timeout=min(remaining, 0.1))
                    require(
                        fortran.directory_bytes(storage_root) <= byte_cap,
                        "local storage cap exceeded",
                    )
                    require(total_storage() <= global_cap, "global physical storage cap exceeded")
                    return code
                except subprocess.TimeoutExpired:
                    continue
        finally:
            # The direct child may exit while a compiler/helper descendant survives.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            if process.poll() is None:
                process.wait()


def build_rust(task_root, book, sources):
    env = controlled_environment()
    versions, binaries = {}, {}
    for name, args in (("rustc", ["-Vv"]), ("cargo", ["-V"])):
        path = shutil.which(name, path=env.get("PATH"))
        require(path is not None, f"missing {name}")
        binaries[name] = {"path": path, "sha256": sha256(Path(path))}
        versions[name] = subprocess.run(
            [path, *args],
            cwd=SOURCE_ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=True,
            timeout=60,
        ).stdout.strip()
        require(versions[name], "empty compiler version")
    identity = {
        "sources": sources,
        "versions": versions,
        "tools": binaries,
        "command": cargo_command("<build>/target"),
        "profile": PROFILE,
        "flags": RUST_FLAGS,
        "platform": platform.platform(),
        "environment": {
            k: env[k] for k in ("RUSTUP_TOOLCHAIN", "RUSTUP_HOME", "CARGO_HOME") if k in env
        },
        "adapter_sha256": {n: sha256(SOURCE_ROOT / n) for n in ADAPTERS},
    }
    build_dir = safe(task_root, "rust-builds/" + fortran.canonical_hash(identity))
    receipt_path = build_dir / "build-manifest.json"
    if build_dir.exists():
        require(receipt_path.is_file(), "incomplete Rust build; no automatic retry")
        receipt = metrics.json_load(receipt_path)
        require(
            receipt["identity"] == identity and receipt["status"] == "complete",
            "stale/failed Rust build",
        )
        for name, artifact in receipt["artifacts"].items():
            path = safe(build_dir, artifact["path"])
            require(
                sha256(path) == artifact["sha256"] and path.stat().st_size == artifact["bytes"],
                f"Rust build artifact changed: {name}",
            )
        actual = fortran.artifact_inventory(build_dir)
        actual.pop("build-manifest.json", None)
        require(actual == receipt["artifacts"], "Rust build artifact inventory changed")
        return build_dir, receipt
    build_dir.mkdir(parents=True, exist_ok=False)
    write_json(build_dir / "identity.json", identity)
    command = cargo_command(build_dir / "target")
    cap = storage_cap(book)
    receipt = {
        "identity": identity,
        "command": command,
        "status": "failed",
        "exit_code": None,
        "started_utc": fortran.utc(),
        "ended_utc": None,
        "executable": None,
        "artifacts": {},
        "error": None,
    }
    try:
        code = run_bounded(
            command,
            SOURCE_ROOT,
            build_dir,
            60,
            max(0, cap - total_storage()),
            env,
            build_dir / "cargo.jsonl",
            build_dir / "compiler.stderr",
            cap,
        )
        receipt["exit_code"] = code
        require(code == 0, f"Cargo exited {code}")
        messages = [
            json.loads(line, object_pairs_hook=contract.unique_object)
            for line in (build_dir / "cargo.jsonl").read_text().splitlines()
            if line.startswith("{")  # -vv also prints "[crate] cargo:..." build-script lines
        ]
        executables = [
            m["executable"]
            for m in messages
            if m.get("reason") == "compiler-artifact"
            and m.get("target", {}).get("name") == "solute_verification_harness"
            and m.get("target", {}).get("kind") == ["test"]
            and m.get("executable")
        ]
        require(len(executables) == 1, "missing/ambiguous Rust harness executable")
        executable = Path(executables[0])
        relative = executable.relative_to(build_dir)
        safe(build_dir, relative)
        require(
            executable.is_file() and os.access(executable, os.X_OK), "Rust build not executable"
        )
        require(source_hashes(book) == sources, "Rust source changed while compiling")
        receipt["executable"] = str(relative)
        receipt["status"] = "complete"
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        receipt["error"] = str(error)
        raise
    finally:
        receipt["ended_utc"] = fortran.utc()
        receipt["artifacts"] = fortran.artifact_inventory(build_dir)
        write_json(receipt_path, receipt)
    return build_dir, receipt


class Lifecycle:
    def __init__(self, cases_path, root):
        self.cases_path = Path(cases_path).absolute()
        self.book = contract.validate(contract.load_cases(self.cases_path))
        apply_timeout_overrides(self.book, TIMEOUT_OVERRIDES)
        self.root, self.task_root = output_root(root)
        self.phase = PHASE_TASKS[self.task_root.name]

    @contextmanager
    def _lock(self, readonly=False):
        if not readonly:
            self.root.mkdir(parents=True, exist_ok=True)
        with safe(self.task_root, "launch.lock").open("rb" if readonly else "ab") as stream:
            fcntl.flock(stream, fcntl.LOCK_SH if readonly else fcntl.LOCK_EX)
            yield

    def _prepare(self, row):
        case = next(c for c in self.book["cases"] if c["id"] == row["case_id"])
        run_id = row["id"] + "-" + uuid.uuid4().hex[:12]
        if row["solver"] == "fortran":
            run_dir, manifest, snapshot = fortran.prepare_run(
                self.book, self.cases_path, case, row, self.root, run_id
            )
            return case, run_dir, manifest, snapshot
        snapshot = fortran.protected_hashes(self.book)
        run_dir = safe(self.root, run_id)
        run_dir.mkdir(exist_ok=False)
        fortran.write_new(
            run_dir / "request.txt",
            fortran.request_bytes(case, row, run_id, sha256(self.cases_path)),
        )
        manifest = {
            "schema_version": "solute-manifest-v1",
            "run_id": run_id,
            "launch_id": row["id"],
            "case_id": case["id"],
            "cases_sha256": sha256(self.cases_path),
            "case_sha256": fortran.canonical_hash(case),
            "request_sha256": sha256(run_dir / "request.txt"),
            "solver": "rust",
            **({"backend": row["backend"]} if "backend" in row else {}),
            "precision": row["precision"],
            "status": "planned",
            "command": [
                str(run_dir / "build/solute-verification-harness"),
                "--exact",
                "requested_study",
                "--nocapture",
                "--test-threads=1",
            ],
            "cwd": str(run_dir),
            "started_utc": None,
            "ended_utc": None,
            "exit_code": None,
            "timeout_seconds": row["timeout_seconds"],
            "provenance": {
                "source_sha256": source_hashes(self.book),
                "compiler": "not built (dry-run)",
                "flags": list(RUST_FLAGS),
                "platform": platform.platform(),
                "executable_sha256": None,
                "shim_sha256": None,
                "adapter_sha256": {n: sha256(SOURCE_ROOT / n) for n in ADAPTERS},
                "input_sha256": {"request.txt": sha256(run_dir / "request.txt")},
            },
            "artifacts": {},
            "record_inventory": {
                "nodes": row["nodes"],
                "species": len(case["species"]),
                "profile_times": [],
                "step_count": 0,
                "endpoint_times": [],
                "terminal_records": {},
                "field_formats": {},
            },
            "warnings": [],
            "mapping_validation": {
                "status": "pending",
                "theta_range": [0, 0],
                "q_range": [0, 0],
                "dispersion_range": [0, 0],
                "evidence": ["No executable observations validated."],
            },
        }
        return case, run_dir, manifest, snapshot

    def dry_run(self, launch_ids=None):
        rows = selected_rows(self.book, self.phase, launch_ids)
        paths = []
        with self._lock():
            all_records(self.book, scope_task(self.task_root.name))
            for row in rows:
                _, run_dir, manifest, snapshot = self._prepare(row)
                manifest["artifacts"] = fortran.artifact_inventory(run_dir)
                write_json(run_dir / "manifest.json", manifest)
                fortran.check_preserved(snapshot)
                paths.append(run_dir)
        return paths

    def _preflight(self, row, run_dir, manifest, snapshot):
        check_budget(self.book, row, scope_task(self.task_root.name))
        fortran.check_preserved(snapshot)
        require(manifest["cases_sha256"] == sha256(self.cases_path), "casebook changed")
        provenance = manifest["provenance"]
        require(provenance["source_sha256"] == source_hashes(self.book), "source identity changed")
        for name, digest in provenance["adapter_sha256"].items():
            require(sha256(safe(SOURCE_ROOT, name)) == digest, "adapter identity changed")
        for name, digest in provenance["input_sha256"].items():
            require(sha256(safe(run_dir, name)) == digest, "input identity changed")
        require(
            sha256(run_dir / "build/solute-verification-harness")
            == provenance["executable_sha256"],
            "executable identity changed",
        )
        require(not (run_dir / "observations.json").exists(), "stale observations before start")
        cap = row_cap(self.book, row)
        size = fortran.directory_bytes(run_dir)
        require(
            size + row["estimated_output_bytes"] <= cap,
            "retained executable plus complete output reservation exceeds row cap",
        )
        require(
            total_storage() + cap - size <= storage_cap(self.book),
            "insufficient global storage for complete row reservation",
        )

    def _validate_rust(self, case, row, run_dir, manifest):
        data = metrics.json_load(safe(run_dir, "observations.json"))
        profiles = data["profiles"]
        theta = [v for p in profiles for v in p["theta"]]
        q = [v for p in profiles for v in p["q_faces"]]
        require(theta and q, "missing Rust physical observations")
        manifest["record_inventory"] = {
            "nodes": len(data["coordinates"]),
            "species": len(data["species"]),
            "profile_times": [p["time"] for p in profiles],
            "step_count": len(data["steps"]),
            "endpoint_times": [p["time"] for p in data["endpoint_history"]],
            "terminal_records": {},
            "field_formats": {"numbers": "f64 round-trip 17 digits"},
        }
        manifest["mapping_validation"] = {
            "status": "passed",
            "theta_range": [min(theta), max(theta)],
            "q_range": [min(q), max(q)],
            "dispersion_range": [case["dispersion"]] * 2,
            "evidence": [
                "Hash-verified flat request and emitted homogeneous theta/face flux profiles.",
                "Dispersion is prescribed by the validated request; no water solver measurement.",
            ],
        }
        manifest["warnings"] = list(data["warnings"])
        manifest["status"] = "complete"
        manifest["artifacts"] = fortran.artifact_inventory(run_dir)
        candidate = run_dir / "candidate-manifest.json"
        added = len(
            (json.dumps(manifest, indent=2, sort_keys=True, allow_nan=False) + "\n").encode()
        )
        require(
            fortran.directory_bytes(run_dir) + added <= row_cap(self.book, row),
            "candidate manifest exceeds row storage",
        )
        require(
            total_storage() + added <= storage_cap(self.book),
            "candidate manifest exceeds global storage",
        )
        write_json(candidate, manifest)
        try:
            metrics.load_observations(run_dir / "observations.json", candidate, self.cases_path)
        finally:
            candidate.unlink()

    def _execute_rust(self, case, row, run_dir, manifest, snapshot):
        started = False
        try:
            check_budget(self.book, row, scope_task(self.task_root.name))
            build_dir, receipt = build_rust(
                self.task_root, self.book, manifest["provenance"]["source_sha256"]
            )
            build = run_dir / "build"
            build.mkdir()
            executable = safe(build_dir, receipt["executable"])
            copied = build / "solute-verification-harness"
            with copied.open("xb") as stream, executable.open("rb") as source:
                shutil.copyfileobj(source, stream)
            copied.chmod(0o755)
            write_json(build / "rust-build.json", receipt)
            manifest["provenance"].update(
                compiler=receipt["identity"]["versions"]["rustc"], executable_sha256=sha256(copied)
            )
            self._preflight(row, run_dir, manifest, snapshot)
            env = controlled_environment()
            env.update(
                zip(
                    ENV_KEYS,
                    [
                        str(run_dir / "request.txt"),
                        manifest["request_sha256"],
                        str(self.task_root),
                        str(run_dir / "observations.json"),
                    ],
                )
            )
            manifest["status"] = "started"
            manifest["started_utc"] = fortran.utc()
            fortran.append_event(
                self.task_root,
                "start",
                manifest,
                {
                    "timeout_seconds": row["timeout_seconds"],
                    "precision": row["precision"],
                    "estimated_output_bytes": row["estimated_output_bytes"],
                },
            )
            started = True
            code = run_bounded(
                manifest["command"],
                run_dir,
                run_dir,
                row["timeout_seconds"],
                row_cap(self.book, row),
                env,
                run_dir / "stdout.txt",
                run_dir / "stderr.txt",
                storage_cap(self.book),
            )
            manifest["exit_code"] = code
            fortran.append_event(self.task_root, "exit", manifest, {"exit_code": code})
            require(code == 0, f"Rust solver exited {code}")
            manifest["ended_utc"] = fortran.utc()
            self._validate_rust(case, row, run_dir, manifest)
            require(
                fortran.directory_bytes(run_dir) <= row_cap(self.book, row),
                "validated run exceeds row storage",
            )
            require(
                total_storage() <= storage_cap(self.book),
                "validated run exceeds global storage",
            )
            fortran.check_preserved(snapshot)
        except subprocess.TimeoutExpired as error:
            manifest["status"] = "timeout"
            manifest["warnings"].append(str(error))
            fortran.append_event(
                self.task_root,
                "timeout",
                manifest,
                {"stage": "solver" if started else "build", "timeout_seconds": error.timeout},
            )
        except KeyboardInterrupt:
            manifest["status"] = "failed"
            manifest["warnings"].append("Interrupted; consumed starts cannot be retried.")
            fortran.append_event(
                self.task_root,
                "validation_failed",
                manifest,
                {"stage": "solver" if started else "build", "error": "KeyboardInterrupt"},
            )
            raise
        except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
            manifest["status"] = "failed"
            manifest["mapping_validation"]["status"] = "failed"
            manifest["mapping_validation"]["evidence"] = [str(error)]
            manifest["warnings"].append(str(error))
            fortran.append_event(
                self.task_root,
                "validation_failed",
                manifest,
                {
                    "stage": "solver_or_validation" if started else "build_or_preflight",
                    "error": str(error),
                },
            )
        finally:
            manifest["ended_utc"] = fortran.utc()
            manifest["artifacts"] = fortran.artifact_inventory(run_dir)
            write_json(run_dir / "manifest.json", manifest)
            if manifest["status"] == "complete":
                fortran.append_event(
                    self.task_root,
                    "complete",
                    manifest,
                    {"status": "complete", "manifest_sha256": sha256(run_dir / "manifest.json")},
                )
            fortran.check_preserved(snapshot)
        return manifest

    def _normalize_fortran(self, run_dir, row):
        # The accepted normalizer writes two whole files. Check their exact sizes
        # before calling it, then check both local and physical totals afterward.
        context = metrics._context(run_dir / "manifest.json", self.cases_path)
        data, _ = metrics._parse_fortran(context)
        payload = metrics.canonical(data) + b"\n"
        receipt = {
            "schema_version": "solute-normalization-v1",
            "raw_manifest_sha256": sha256(run_dir / "manifest.json"),
            "observations_sha256": hashlib.sha256(payload).hexdigest(),
            "normalizer_sha256": sha256(Path(metrics.__file__)),
            "status": "complete",
        }
        added = len(payload) + len(metrics.canonical(receipt)) + 1
        cap = row_cap(self.book, row)
        require(
            fortran.directory_bytes(run_dir) + added <= cap, "normalization exceeds row storage"
        )
        require(
            total_storage() + added <= storage_cap(self.book),
            "normalization exceeds global storage",
        )
        metrics.normalize_fortran(run_dir, self.cases_path)
        require(fortran.directory_bytes(run_dir) <= cap, "normalized row storage exceeded")
        require(
            total_storage() <= storage_cap(self.book),
            "normalized global storage exceeded",
        )
        return metrics.load_observations(
            run_dir / "observations.json", run_dir / "manifest.json", self.cases_path
        )

    def execute(self, launch_ids, activation):
        require(launch_ids is not None, "execution requires exact launch IDs")
        rows = selected_rows(self.book, self.phase, launch_ids)
        validate_activation(self.book, self.cases_path, self.phase, rows, activation)
        results = []
        with self._lock():
            for row in rows:
                check_budget(self.book, row, scope_task(self.task_root.name))
                require(
                    total_storage() + 2 * row["estimated_output_bytes"]
                    <= storage_cap(self.book),
                    "insufficient actual study storage",
                )
                case, run_dir, manifest, snapshot = self._prepare(row)
                write_json(run_dir / "activation.json", activation)
                manifest["provenance"]["input_sha256"]["activation.json"] = sha256(
                    run_dir / "activation.json"
                )
                manifest["provenance"]["adapter_sha256"]["tools/solute_benchmark_lifecycle.py"] = (
                    sha256(Path(__file__))
                )
                if row["solver"] == "rust":
                    manifest = self._execute_rust(case, row, run_dir, manifest, snapshot)
                else:
                    manifest = fortran.execute_run(
                        self.book, case, row, run_dir, manifest, snapshot, self.task_root
                    )
                    if manifest["status"] == "raw_complete":
                        try:
                            fortran.verify_run(run_dir, self.book, self.cases_path)
                            self._normalize_fortran(run_dir, row)
                        except (OSError, ValueError, KeyError, TypeError) as error:
                            fortran.append_event(
                                self.task_root,
                                "validation_failed",
                                manifest,
                                {"stage": "normalization", "error": str(error)},
                            )
                            write_json(run_dir / "normalization-error.json", {"error": str(error)})
                            results.append(run_dir)
                            break
                results.append(run_dir)
                if manifest["status"] not in ("complete", "raw_complete"):
                    break
        return results

    def verify(self, run_dir, *, require_complete=False):
        run_dir = Path(run_dir).absolute()
        safe(self.root, run_dir.relative_to(self.root))
        with self._lock(readonly=True):
            manifest = metrics.json_load(run_dir / "manifest.json")
            require(set(manifest) - {"backend"} == fortran.MANIFEST_KEYS, "manifest schema mismatch")
            row = selected_rows(self.book, self.phase, [manifest["launch_id"]])[0]
            records = all_records(self.book, scope_task(self.task_root.name))
            if row["solver"] == "fortran":
                result = fortran.verify_run(run_dir, self.book, self.cases_path)
                if result["status"] == "raw_complete":
                    metrics.load_observations(
                        run_dir / "observations.json", run_dir / "manifest.json", self.cases_path
                    )
            else:
                require(
                    manifest["schema_version"] == "solute-manifest-v1"
                    and manifest["solver"] == "rust"
                    and manifest["precision"] == row["precision"]
                    and manifest["run_id"] == run_dir.name
                    and manifest["timeout_seconds"] == row["timeout_seconds"],
                    "manifest identity mismatch",
                )
                case = next(c for c in self.book["cases"] if c["id"] == row["case_id"])
                require(
                    manifest["cases_sha256"] == sha256(self.cases_path)
                    and manifest["case_id"] == case["id"]
                    and manifest["case_sha256"] == fortran.canonical_hash(case),
                    "manifest case identity mismatch",
                )
                request_bytes = (run_dir / "request.txt").read_bytes()
                approved_requests = {
                    fortran.request_bytes(case, row, run_dir.name, sha256(self.cases_path)),
                    fortran.request_bytes(
                        case,
                        row,
                        run_dir.name,
                        sha256(self.cases_path),
                        include_upstream_weighting=False,
                    ),
                }
                require(request_bytes in approved_requests, "request differs from approved mapping")
                require(
                    manifest["request_sha256"] == sha256(run_dir / "request.txt"),
                    "request hash mismatch",
                )
                require(
                    manifest["command"]
                    == [
                        str(run_dir / "build/solute-verification-harness"),
                        "--exact",
                        "requested_study",
                        "--nocapture",
                        "--test-threads=1",
                    ]
                    and manifest["cwd"] == str(run_dir),
                    "Rust command/cwd changed",
                )
                require(
                    manifest["provenance"]["source_sha256"] == source_hashes(self.book),
                    "source identity changed",
                )
                require(manifest["provenance"]["flags"] == RUST_FLAGS, "Rust flags changed")
                require(
                    manifest["provenance"]["adapter_sha256"]
                    == {n: sha256(safe(SOURCE_ROOT, n)) for n in ADAPTERS},
                    "adapter identity changed",
                )
                inputs = ["request.txt"] + (
                    ["activation.json"] if manifest["status"] != "planned" else []
                )
                require(
                    manifest["provenance"]["input_sha256"]
                    == {n: sha256(safe(run_dir, n)) for n in inputs},
                    "input identity mismatch",
                )
                require(
                    manifest["artifacts"] == fortran.artifact_inventory(run_dir),
                    "artifact inventory changed",
                )
                require(
                    manifest["status"] in ("planned", "complete"),
                    "failed/incomplete run cannot verify",
                )
                if manifest["status"] == "complete":
                    receipt = metrics.json_load(run_dir / "build/rust-build.json")
                    require(
                        receipt["status"] == "complete"
                        and receipt["identity"]["sources"]
                        == manifest["provenance"]["source_sha256"]
                        and receipt["identity"]["versions"]["rustc"]
                        == manifest["provenance"]["compiler"],
                        "Rust build provenance mismatch",
                    )
                    metrics.load_observations(
                        run_dir / "observations.json", run_dir / "manifest.json", self.cases_path
                    )
                else:
                    require(
                        manifest["provenance"]["executable_sha256"] is None
                        and manifest["provenance"]["shim_sha256"] is None
                        and manifest["started_utc"] is None
                        and manifest["ended_utc"] is None
                        and manifest["exit_code"] is None,
                        "planned run has completed fields",
                    )
                result = manifest
            if result["status"] != "planned":
                activation = metrics.json_load(run_dir / "activation.json")
                active_rows = selected_rows(self.book, self.phase, activation.get("launch_ids"))
                validate_activation(self.book, self.cases_path, self.phase, active_rows, activation)
                require(row["id"] in activation["launch_ids"], "run absent from activation")
                completed = [
                    r
                    for r in records
                    if r["run_id"] == manifest["run_id"] and r["event"] == "complete"
                ]
                require(
                    len(completed) == 1
                    and completed[0]["detail"]["manifest_sha256"]
                    == sha256(run_dir / "manifest.json"),
                    "manifest completion is not authenticated by ledger",
                )
                require(not (run_dir / "normalization-error.json").exists(), "normalization failed")
            else:
                require(not require_complete, "complete run required")
                require(
                    not any(r["run_id"] == manifest["run_id"] for r in records),
                    "planned run has execution events",
                )
            fortran.protected_hashes(self.book)
            return result

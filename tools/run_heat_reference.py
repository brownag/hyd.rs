#!/usr/bin/env python3
"""Produce heat fixture JSON from the pinned Fortran build (source-matched only).

Each case directory under reference/heat_fortran/cases/ holds `fixture.json`
(runner format: inputs, outputs with completion markers, aliases). Each case runs
the already-verified executable in a fresh, untracked run directory, following the
isolation pattern of tools/run_fortran_reference.py (ADR 0003): inputs are copied
and byte-checked, the executable hash is checked against the build manifest, and
outputs are validated before parsing. This tool never compares against Rust
output and writes no numeric tolerance into fixtures or manifests.

run_fortran_reference.run_fixture is not used: it resolves task roots physically
and calls relative_to(REPO_ROOT), which fails under the approved worktree
`.development` symlink.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
CASES_DIR = REPO_ROOT / "reference/heat_fortran/cases"
FIXTURES_DIR = REPO_ROOT / "reference/heat_fortran/fixtures"
DEFAULT_BUILD = REPO_ROOT / ".development/24.4.3/reference-build"
NOD_INF = "NOD_INF.OUT"
# Fixed-form columns (1-based) of NodOut rows, formats 120 and 130 (OUTPUT.FOR:820, :853).
NODE_SLICE = slice(0, 4)
DEPTH_SLICE = slice(5, 15)
TEMP_SLICE = slice(103, 111)
TIME_RE = re.compile(r"^\s*Time:\s*(\S+)\s*$")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def parse_nod_inf(text: str) -> list[dict]:
    """Parse NOD_INF.OUT blocks into {time, node, depth, temp} records, top node first."""
    blocks: list[dict] = []
    current: dict | None = None
    for raw in text.splitlines():
        match = TIME_RE.match(raw)
        if match:
            if current is not None:
                raise ValueError("Time block started before previous block ended")
            current = {"time": float(match.group(1)), "node": [], "depth": [], "temp": []}
            continue
        if current is None:
            continue
        if raw.strip() == "end":
            blocks.append(current)
            current = None
            continue
        head = raw[NODE_SLICE].strip()
        if not head.isdigit():
            continue  # header lines
        if "*" in raw[NODE_SLICE.start:TEMP_SLICE.stop]:
            raise ValueError(f"Fortran field overflow in NOD_INF row: {raw!r}")
        current["node"].append(int(head))
        current["depth"].append(float(raw[DEPTH_SLICE]))
        current["temp"].append(float(raw[TEMP_SLICE]))
    if current is not None:
        raise ValueError("NOD_INF block missing 'end' marker")
    if not blocks:
        raise ValueError("NOD_INF contains no Time blocks")
    if len({len(b["node"]) for b in blocks}) != 1:
        raise ValueError("inconsistent node counts across Time blocks")
    return blocks


class NonFiniteOutput(RuntimeError):
    """Raised when the reference output holds a non-finite value; no fixture is written."""


def first_nonfinite(blocks: list[dict]) -> dict | None:
    for block in blocks:
        for node, depth, temp in zip(block["node"], block["depth"], block["temp"]):
            if not (math.isfinite(temp) and math.isfinite(depth)):
                return {"time": block["time"], "node": node, "temp": repr(temp)}
    return None


def guard_finite(case_id: str, blocks: list[dict]) -> None:
    hit = first_nonfinite(blocks)
    if hit is not None:
        raise NonFiniteOutput(
            f"{case_id}: refused fixture; non-finite Temp at time {hit['time']} "
            f"node {hit['node']} (value {hit['temp']}); Fortran output is not a valid parity target"
        )


def load_build(build_dir: Path) -> dict:
    build = json.loads((build_dir / "build-manifest.json").read_text(encoding="utf-8"))
    staged = json.loads((build_dir / "staging-manifest.json").read_text(encoding="utf-8"))
    exe = build_dir / "hydrus-reference"
    if sha256_file(exe) != build["executable_sha256"]:
        raise RuntimeError("executable hash does not match build manifest")
    return {"build": build, "staged": staged, "exe": exe,
            "build_manifest_sha256": sha256_file(build_dir / "build-manifest.json")}


def run_case(case_id: str, task_base: Path, build_dir: Path, timeout_seconds: int | None) -> Path:
    case_dir = CASES_DIR / case_id
    spec = json.loads((case_dir / "fixture.json").read_text(encoding="utf-8"))
    if spec.get("case_id") != case_id:
        raise RuntimeError(f"fixture manifest case_id mismatch: {case_id}")
    config = spec["runner"]
    info = load_build(build_dir)

    root = task_base / case_id
    if root.exists():
        raise RuntimeError(f"task root already exists; refusing to overwrite: {root}")
    run_dir, committed = root / "run", root / "run" / "committed-inputs"
    committed.mkdir(parents=True)
    input_sha = {}
    for name in config["inputs"]:
        src, dst = case_dir / name, committed / name
        shutil.copyfile(src, dst)
        if src.read_bytes() != dst.read_bytes():
            raise RuntimeError(f"input copy changed bytes: {name}")
        input_sha[name] = sha256_file(dst)
    prefix = config["data_prefix"]
    for name, alias in config.get("aliases", {}).items():
        shutil.copyfile(committed / name, run_dir / f"{prefix}\\{alias}")

    command = [str(info["exe"]), prefix]
    try:
        done = subprocess.run(command, cwd=run_dir, text=True, capture_output=True, timeout=timeout_seconds)
    except subprocess.TimeoutExpired as error:
        raise RuntimeError(f"{case_id}: run timed out; incomplete run retained at {run_dir}") from error
    (run_dir / "stdout.txt").write_text(done.stdout, encoding="utf-8")
    (run_dir / "stderr.txt").write_text(done.stderr, encoding="utf-8")
    if done.returncode:
        raise RuntimeError(f"{case_id}: exited {done.returncode}; incomplete run retained at {run_dir}")
    if (run_dir / f"{prefix}\\Error.msg").exists():
        raise RuntimeError(f"{case_id}: reference reported an error; run retained at {run_dir}")

    outputs = {}
    for entry in config["outputs"]:
        path = run_dir / f"{prefix}\\{entry['name']}"
        if not path.is_file() or path.stat().st_size == 0:
            raise RuntimeError(f"{case_id}: missing or empty output {entry['name']}")
        text = path.read_text(encoding="utf-8", errors="replace")
        if not text.endswith("\n") or entry["marker"].lower() not in text.lower():
            raise RuntimeError(f"{case_id}: malformed or truncated {entry['name']}")
        outputs[entry["name"]] = path

    blocks = parse_nod_inf(outputs[NOD_INF].read_text(encoding="utf-8"))
    try:
        guard_finite(case_id, blocks)
    except NonFiniteOutput as error:
        (root / "refusal.json").write_text(
            json.dumps({"case_id": case_id, "refused": True, "reason": str(error)}, indent=2) + "\n",
            encoding="utf-8",
        )
        raise
    selector_text = (case_dir / "SELECTOR.IN").read_text(encoding="utf-8")
    profile_text = (case_dir / "PROFILE.DAT").read_text(encoding="utf-8")
    build, staged = info["build"], info["staged"]
    fixture = {
        "schema_version": 1,
        "case_id": case_id,
        "source_matched": True,
        "evidence_class": "pinned-fortran",
        "temperature_unit": "C",
        "time_points": [b["time"] for b in blocks],
        "node_numbers": blocks[0]["node"],
        "node_depths": blocks[0]["depth"],
        "temperatures": [b["temp"] for b in blocks],
        "manifest": {
            "source_and_archive_sha256": staged["source_and_archive_sha256"],
            "patch_sha256": build["patch_sha256"],
            "executable_sha256": build["executable_sha256"],
            "build_manifest_sha256": info["build_manifest_sha256"],
            "compiler": build["gnu_fortran_version"],
            "host_os": build["host_os"],
            "host_architecture": build["host_architecture"],
            "flags": build["flags"],
            "command": ["hydrus-reference", prefix],
            "input_sha256": input_sha,
            "output_sha256": {name: sha256_file(path) for name, path in outputs.items()},
            "selector_sha256": sha256_bytes(selector_text.encode("utf-8")),
            "profile_sha256": sha256_bytes(profile_text.encode("utf-8")),
        },
    }
    FIXTURES_DIR.mkdir(parents=True, exist_ok=True)
    out = FIXTURES_DIR / f"{case_id}.json"
    out.write_text(json.dumps(fixture, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", action="append", help="case id (repeatable)")
    parser.add_argument("--all", action="store_true", help="run every case directory")
    parser.add_argument("--build-dir", default=str(DEFAULT_BUILD), help="verified reference build directory")
    parser.add_argument("--task-base", required=True, help="new parent for per-case run roots, below .development/")
    parser.add_argument("--timeout-seconds", type=int)
    args = parser.parse_args(argv)
    cases = sorted(p.name for p in CASES_DIR.iterdir() if p.is_dir()) if args.all else (args.case or [])
    if not cases:
        parser.error("give --case or --all")
    task_base = Path(args.task_base).resolve()
    development = (REPO_ROOT / ".development").resolve()
    if task_base == development or development not in task_base.parents:
        parser.error("--task-base must be below .development/")
    task_base.mkdir(parents=True, exist_ok=True)
    for case_id in cases:
        out = run_case(case_id, task_base, Path(args.build_dir).resolve(), args.timeout_seconds)
        print(f"wrote {out.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

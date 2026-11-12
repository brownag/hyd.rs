#!/usr/bin/env python3
"""Build and invoke the byte-preserved pinned VaporContent kernel only."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
SOURCE = REPO_ROOT / "src_reference/WATFLOW.FOR"
DRIVER = REPO_ROOT / "reference/vapor_fortran/kernel_harness/vapor_content_driver.f90"
SOURCE_SHA256 = "e5591cebae4cbb44e8fe07520e08fc98c1522531c67753c0cc03fe2e164b5ef9"
ROUTINE_SHA256 = "99ae3d35a4425e7d7ca740856ba80f8bf4bb4dfe741ae18c685f07bcf05c59ee"
COMPILER = "gfortran"
FLAGS = ["-std=legacy", "-ffixed-line-length-none", "-O0"]
FLOAT = re.compile(r"^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[EeDd][+-]?\d+)?$")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def extract_routine(source: bytes) -> bytes:
    if sha256(source) != SOURCE_SHA256:
        raise ValueError("pinned whole-source SHA-256 mismatch")
    lines = source.splitlines(keepends=True)
    starts = [i for i, line in enumerate(lines) if re.match(rb"^\s*subroutine\s+VaporContent\s*\(", line, re.I)]
    if len(starts) != 1:
        raise ValueError(f"expected one VaporContent subroutine boundary; found {len(starts)}")
    start = starts[0]
    ends = [i for i in range(start + 1, len(lines)) if re.match(rb"^\s*end\s*(?:!.*)?(?:\r?\n)?$", lines[i], re.I)]
    if not ends:
        raise ValueError("VaporContent has no complete END boundary")
    routine = b"".join(lines[start : ends[0] + 1])
    if sha256(routine) != ROUTINE_SHA256:
        raise ValueError("pinned VaporContent routine SHA-256 mismatch")
    return routine


def finite_number(value: Any, label: str, lower: float, upper: float, *, inclusive: bool = True) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{label} must be numeric")
    number = float(value)
    if not math.isfinite(number) or (number < lower if inclusive else number <= lower) or number > upper:
        raise ValueError(f"{label} outside finite range [{lower}, {upper}]")
    return number


def validate_requests(payload: Any) -> list[dict[str, Any]]:
    if not isinstance(payload, dict) or payload.get("schema_version") != "vapor-content-request-v1":
        raise ValueError("request schema_version must be vapor-content-request-v1")
    units = {"temperature": "degC", "head": "cm", "theta": "cm3/cm3",
             "porosity": "cm3/cm3", "capacity": "1/cm", "x_conv": "cm/m"}
    if payload.get("units") != units:
        raise ValueError("request units do not match the required units contract")
    requests = payload.get("requests")
    if not isinstance(requests, list) or not requests:
        raise ValueError("requests must be a non-empty array")
    ids: set[str] = set()
    for req in requests:
        if not isinstance(req, dict) or not isinstance(req.get("id"), str) or not re.fullmatch(r"[A-Za-z0-9_-]{1,64}", req["id"]):
            raise ValueError("each request needs a safe unique id")
        if req["id"] in ids:
            raise ValueError(f"duplicate request id {req['id']}")
        ids.add(req["id"])
        req["x_conv"] = finite_number(req.get("x_conv"), f"{req['id']}.x_conv", 0, 1e9, inclusive=False)
        materials, nodes = req.get("materials"), req.get("nodes")
        if not isinstance(materials, list) or not materials or not isinstance(nodes, list) or not nodes:
            raise ValueError(f"{req['id']} needs non-empty materials and nodes")
        material_ids = [m.get("index") if isinstance(m, dict) else None for m in materials]
        if material_ids != list(range(1, len(materials) + 1)):
            raise ValueError(f"{req['id']} material indices must be contiguous and 1-based")
        for material in materials:
            material["porosity"] = finite_number(material.get("porosity"), "porosity", 0, 1, inclusive=False)
        for node in nodes:
            if not isinstance(node, dict) or node.get("material") not in material_ids:
                raise ValueError(f"{req['id']} has an invalid material mapping")
            node["theta"] = finite_number(node.get("theta"), "theta", 0, 1)
            if node["theta"] > materials[node["material"] - 1]["porosity"]:
                raise ValueError(f"{req['id']} theta exceeds material porosity")
            node["temperature"] = finite_number(node.get("temperature"), "temperature", -100, 100)
            if node["temperature"] + 273.15 <= 0:
                raise ValueError("absolute temperature must be positive")
            node["head"] = finite_number(node.get("head"), "head", -1e7, 1e7)
            node["capacity"] = finite_number(node.get("capacity"), "capacity", -1e6, 1e6)
    return requests


def safe_new_path(path: Path, base: Path, label: str) -> None:
    if not path.is_absolute():
        raise ValueError(f"{label} must resolve from repository root")
    if base not in path.parents:
        raise ValueError(f"{label} must be below {base}")
    current = base
    parts = path.relative_to(base).parts
    for index, part in enumerate(parts):
        current = current / part
        final = index == len(parts) - 1
        if current.is_symlink():
            raise FileExistsError(f"{label} path contains symlink: {current}")
        if current.exists() and (final or not current.is_dir()):
            raise FileExistsError(f"{label} path already exists or contains non-directory: {current}")


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")


def make_input(requests: list[dict[str, Any]]) -> bytes:
    rows = [str(len(requests))]
    for req in requests:
        rows.append(f"{req['id']} {len(req['nodes'])} {len(req['materials'])} {req['x_conv']:.9g}")
        rows.extend(f"{m['porosity']:.9g}" for m in req["materials"])
        rows.extend(
            f"{node['theta']:.9g} {node['temperature']:.9g} {node['head']:.9g} "
            f"{node['material']} {node['capacity']:.9g}"
            for node in req["nodes"]
        )
    return ("\n".join(rows) + "\n").encode("ascii")


def parse_output(raw: bytes, requests: list[dict[str, Any]]) -> list[dict[str, Any]]:
    try:
        text = raw.decode("ascii")
    except UnicodeDecodeError as error:
        raise ValueError("Fortran output is not ASCII") from error
    lines = text.splitlines()
    expected = [(req["id"], i) for req in requests for i in range(1, len(req["nodes"]) + 1)]
    records: list[dict[str, Any]] = []
    marker = f"COMPLETE {len(requests)} {len(expected)}"
    if len(lines) != len(expected) + 1 or lines[-1].strip() != marker:
        raise ValueError("output has missing/extra records or invalid completion marker")
    for line, (request_id, node_index) in zip(lines[:-1], expected):
        fields = line.split()
        if len(fields) != 5 or fields[:3] != ["RECORD", request_id, str(node_index)]:
            raise ValueError("output record identity/dimensions mismatch")
        values = []
        for token in fields[3:]:
            if not FLOAT.fullmatch(token):
                raise ValueError("malformed or overflowed REAL output")
            value = float(token.replace("D", "E").replace("d", "e"))
            if not math.isfinite(value):
                raise ValueError("nonfinite REAL output")
            values.append(value)
        req = next(item for item in requests if item["id"] == request_id)
        records.append({"request_id": request_id, "node": node_index,
                        "input_capacity": req["nodes"][node_index - 1]["capacity"],
                        "vapor_content": values[0], "adjusted_hydraulic_capacity": values[1]})
    return records


def run(args: argparse.Namespace) -> Path:
    payload = json.loads(args.requests.read_text(encoding="utf-8"))
    requests = validate_requests(payload)
    dev = REPO_ROOT / ".development"
    task_root = (REPO_ROOT / args.task_root).absolute()
    output = (REPO_ROOT / args.output).absolute()
    safe_new_path(task_root, dev, "task root")
    safe_new_path(output, REPO_ROOT / "reference/vapor_fortran/kernel_fixtures", "fixture")
    task_root.mkdir(parents=True)
    logs = task_root / "attempt.json"
    manifest: dict[str, Any] = {"schema_version": "vapor-kernel-evidence-v1", "started_utc": utc_now(), "status": "started", "commands": []}
    try:
        source = SOURCE.read_bytes()
        routine = extract_routine(source)
        build = task_root / "build"
        build.mkdir()
        extracted = build / "VaporContent.FOR"
        extracted.write_bytes(routine)
        driver_copy = build / "vapor_content_driver.f90"
        shutil.copyfile(DRIVER, driver_copy)
        if driver_copy.read_bytes() != DRIVER.read_bytes():
            raise RuntimeError("driver staging byte mismatch")
        input_bytes = make_input(requests)
        input_path = task_root / "requests.input"
        input_path.write_bytes(input_bytes)
        exe = build / "vapor-content-driver"
        compile_command = [COMPILER, *FLAGS, str(extracted), str(driver_copy), "-o", str(exe)]
        manifest["commands"].append({"argv": compile_command, "started_utc": utc_now()})
        try:
            compiled = subprocess.run(compile_command, cwd=build, capture_output=True, text=True, timeout=args.timeout_seconds)
        except subprocess.TimeoutExpired as error:
            (task_root / "compiler.stdout.txt").write_text(error.stdout or "", encoding="utf-8")
            (task_root / "compiler.stderr.txt").write_text(error.stderr or "", encoding="utf-8")
            manifest["commands"][-1].update({"exited_utc": utc_now(), "returncode": None, "timed_out": True})
            raise RuntimeError("compiler timed out") from error
        (task_root / "compiler.stdout.txt").write_text(compiled.stdout, encoding="utf-8")
        (task_root / "compiler.stderr.txt").write_text(compiled.stderr, encoding="utf-8")
        manifest["commands"][-1].update({"exited_utc": utc_now(), "returncode": compiled.returncode})
        if compiled.returncode != 0 or not exe.is_file():
            raise RuntimeError("compiler failed or did not produce executable")
        exe_hash = sha256(exe.read_bytes())
        (task_root / "executable.sha256").write_text(exe_hash + "\n", encoding="ascii")
        run_command = [str(exe)]
        manifest["commands"].append({"argv": run_command, "started_utc": utc_now()})
        try:
            executed = subprocess.run(run_command, cwd=task_root, input=input_bytes, capture_output=True, timeout=args.timeout_seconds)
        except subprocess.TimeoutExpired as error:
            (task_root / "run.stdout.bin").write_bytes(error.stdout or b"")
            (task_root / "run.stderr.bin").write_bytes(error.stderr or b"")
            raise RuntimeError("kernel driver timed out") from error
        (task_root / "run.stdout.bin").write_bytes(executed.stdout)
        (task_root / "run.stderr.bin").write_bytes(executed.stderr)
        manifest["commands"][-1].update({"exited_utc": utc_now(), "returncode": executed.returncode})
        if executed.returncode != 0:
            raise RuntimeError(f"kernel driver exited {executed.returncode}")
        if sha256(exe.read_bytes()) != exe_hash:
            raise RuntimeError("executable changed during run")
        records = parse_output(executed.stdout, requests)
        fixture = {
            "schema_version": "vapor-content-kernel-fixture-v1",
            "evidence_class": "pinned-fortran-kernel",
            "claim_limit": "direct VaporContent kernel observations; not a full-program trajectory or numerical acceptance",
            "units": payload["units"],
            "requests": requests,
            "observations": records,
            "provenance": {
                "source_sha256": sha256(source), "routine_sha256": sha256(routine),
                "driver_sha256": sha256(DRIVER.read_bytes()),
                "tool_sha256": sha256(Path(__file__).read_bytes()),
                "compiler": COMPILER, "flags": FLAGS, "executable_sha256": exe_hash,
                "request_sha256": sha256(args.requests.read_bytes()),
                "staged_input_sha256": sha256(input_bytes),
                "raw_output_sha256": sha256(executed.stdout),
            },
        }
        if not any(record["vapor_content"] > 0 for record in records):
            raise RuntimeError("reference observations contain no nonzero vapor content")
        manifest.update({"status": "complete", "completed_utc": utc_now(), "returncode": 0,
                         "provenance": fixture["provenance"]})
        output.parent.mkdir(parents=True, exist_ok=True)
        # Recheck immediately before publication to refuse a raced destination.
        if output.exists() or output.is_symlink():
            raise FileExistsError(f"fixture appeared during run: {output}")
        write_json(output, fixture)
        write_json(logs, manifest)
        return output
    except Exception as error:
        manifest.update({"status": "failed", "completed_utc": utc_now(), "failure": f"{type(error).__name__}: {error}"})
        write_json(logs, manifest)
        raise


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--requests", type=Path, required=True)
    parser.add_argument("--task-root", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--timeout-seconds", type=int, default=60)
    args = parser.parse_args(argv)
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be positive")
    args.requests = (REPO_ROOT / args.requests).resolve()
    try:
        output = run(args)
    except Exception as error:
        print(f"vapor kernel reference failed: {error}", file=sys.stderr)
        return 1
    print(f"wrote {output.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

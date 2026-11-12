#!/usr/bin/env python3
"""Produce source-matched vapor trajectory fixtures from an unchanged Fortran build."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / "reference/vapor_fortran/cases"
FIXTURES = ROOT / "reference/vapor_fortran/fixtures"
TIME_RE = re.compile(r"^\s*Time:\s*(\S+)\s*$", re.I)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def finite_number(text: str, label: str) -> float:
    try:
        value = float(text.replace("D", "E").replace("d", "e"))
    except ValueError as exc:
        raise ValueError(f"malformed {label} field {text!r}") from exc
    if not math.isfinite(value):
        raise ValueError(f"nonfinite {label} field {text!r}")
    return value


def parse_blocks(text: str, vapor: bool = False) -> list[dict]:
    """Parse NodOut's fixed-width time blocks; nodes remain source top-first."""
    blocks, current = [], None
    seen_times = set()
    fields = ("con", "con_lt", "con_vh", "con_vt", "v_liquid", "v_vapor", "v_total", "v_vap_iso", "v_vap_term")
    for raw in text.splitlines():
        match = TIME_RE.match(raw)
        if match:
            if current is not None:
                raise ValueError("Time block started before previous block ended")
            t = finite_number(match.group(1), "time")
            if t in seen_times:
                raise ValueError(f"duplicate time {t}")
            seen_times.add(t)
            current = {"time": t, "nodes": []}
            continue
        if current is None:
            continue
        if raw.strip().lower() == "end":
            if not current["nodes"]:
                raise ValueError("empty time block")
            blocks.append(current)
            current = None
            continue
        node_text = raw[:4].strip()
        if not node_text.isdigit():
            continue
        if len(raw) < (133 if vapor else 111):
            raise ValueError("truncated fixed-width output row")
        if "*" in raw:
            raise ValueError("Fortran field overflow in output row")
        node = int(node_text)
        if any(r["node"] == node for r in current["nodes"]):
            raise ValueError(f"duplicate node {node} at time {current['time']}")
        depth = finite_number(raw[5:15], "depth")
        row = {"node": node, "depth": depth}
        if vapor:
            row.update(dict(zip(fields, [finite_number(raw[16 + 13*i:29 + 13*i], fields[i]) for i in range(9)])))
        else:
            row.update(head=finite_number(raw[16:27], "head"), moisture=finite_number(raw[28:34], "moisture"), temperature=finite_number(raw[103:111], "temperature"))
        current["nodes"].append(row)
    if current is not None:
        raise ValueError("time block missing end marker")
    if not blocks:
        raise ValueError("output contains no Time blocks")
    identity = [(r["node"], r["depth"]) for r in blocks[0]["nodes"]]
    if len({r["node"] for r in blocks[0]["nodes"]}) != len(identity):
        raise ValueError("duplicate node identity")
    for block in blocks[1:]:
        if [(r["node"], r["depth"]) for r in block["nodes"]] != identity:
            raise ValueError("missing, reordered, or changed node identities across times")
    return blocks


def load_build(directory: Path) -> dict:
    build_path, staged_path = directory / "build-manifest.json", directory / "staging-manifest.json"
    build, staged = json.loads(build_path.read_text()), json.loads(staged_path.read_text())
    exe = directory / "hydrus-reference"
    if sha256(exe) != build["executable_sha256"]:
        raise RuntimeError("reference executable hash mismatch")
    return {"build": build, "staged": staged, "exe": exe, "build_manifest_sha256": sha256(build_path), "staging_manifest_sha256": sha256(staged_path)}


def run_case(case_id: str, task_base: Path, build_dir: Path, timeout: int | None) -> Path:
    case_dir = CASES / case_id
    spec = json.loads((case_dir / "fixture.json").read_text())
    if spec.get("case_id") != case_id:
        raise ValueError(f"case id mismatch in {case_dir}")
    config = spec["runner"]
    output_path = FIXTURES / f"{case_id}.json"
    if output_path.exists():
        raise FileExistsError(f"refusing to overwrite fixture: {output_path}")
    run_root = task_base / case_id
    if run_root.exists():
        raise FileExistsError(f"refusing to overwrite run root: {run_root}")
    run_dir = run_root / "run"
    committed = run_root / "committed-inputs"
    run_dir.mkdir(parents=True)
    committed.mkdir()
    hashes = {}
    for name in config["inputs"]:
        src, dst = case_dir / name, committed / name
        shutil.copyfile(src, dst)
        if src.read_bytes() != dst.read_bytes():
            raise RuntimeError(f"input copy differs: {name}")
        hashes[name] = sha256(dst)
    prefix = config["data_prefix"]
    for name, alias in config.get("aliases", {}).items():
        shutil.copyfile(committed / name, run_dir / f"{prefix}\\{alias}")
    shutil.copyfile(committed / "LEVEL_01.DIR", run_dir / f"{prefix}\\LEVEL_01.DIR")
    command = [str(build_dir / "hydrus-reference"), prefix]
    done = subprocess.run(command, cwd=run_dir, text=True, capture_output=True, timeout=timeout)
    (run_root / "stdout.txt").write_text(done.stdout)
    (run_root / "stderr.txt").write_text(done.stderr)
    if done.returncode:
        raise RuntimeError(f"{case_id}: executable exited {done.returncode}; retained at {run_root}")
    if (run_dir / f"{prefix}\\Error.msg").exists():
        raise RuntimeError(f"{case_id}: source reported Error.msg; retained at {run_root}")
    outputs, parsed = {}, {}
    for entry in config["outputs"]:
        path = run_dir / f"{prefix}\\{entry['name']}"
        if not path.is_file() or not path.stat().st_size:
            raise RuntimeError(f"{case_id}: required output absent/empty: {entry['name']}")
        raw = path.read_text(errors="strict")
        if not raw.endswith("\n") or entry["marker"].lower() not in raw.lower():
            raise ValueError(f"{case_id}: malformed or truncated {entry['name']}")
        outputs[entry["name"]] = path
        parsed[entry["name"]] = parse_blocks(raw, entry["name"] == "NOD_INF_V.OUT")
    main_blocks = parsed["NOD_INF.OUT"]
    vapor_blocks = parsed.get("NOD_INF_V.OUT")
    if vapor_blocks and [b["time"] for b in vapor_blocks] != [b["time"] for b in main_blocks]:
        raise ValueError(f"{case_id}: vapor and nodal output times differ")
    selector = (case_dir / "SELECTOR.IN").read_text()
    enabled = "f f f t f f" in selector
    if enabled != bool(vapor_blocks):
        raise ValueError(f"{case_id}: selector vapor reachability disagrees with output availability")
    staged, build = load_build(build_dir)["staged"], load_build(build_dir)["build"]
    info = load_build(build_dir)
    record = {
        "schema_version": 1, "case_id": case_id, "source_matched": True,
        "evidence_class": "pinned-fortran-trajectory", "vapor_output_available": bool(vapor_blocks),
        "source_input_mapping": {"selector": "SELECTOR.IN", "profile": "PROFILE.DAT", "level_file": "LEVEL_01.DIR"},
        "time_points": [b["time"] for b in main_blocks],
        "node_numbers": [r["node"] for r in main_blocks[0]["nodes"]],
        "node_depths": [r["depth"] for r in main_blocks[0]["nodes"]],
        "head": [[r["head"] for r in b["nodes"]] for b in main_blocks],
        "liquid_moisture": [[r["moisture"] for r in b["nodes"]] for b in main_blocks],
        "temperature_c": [[r["temperature"] for r in b["nodes"]] for b in main_blocks],
        "vapor_fields": ({name: [[r[name] for r in b["nodes"]] for b in vapor_blocks] for name in vapor_blocks[0]["nodes"][0] if name not in ("node", "depth")} if vapor_blocks else None),
        "output_formats": {"NOD_INF.OUT": "source formats 120/130; head f11.3/e11.4, moisture f6.4, temperature f8.2", "NOD_INF_V.OUT": "format 140: i4, f10.4, 9e13.5" if vapor_blocks else "unavailable: vapor disabled"},
        "manifest": {"source_and_archive_sha256": staged["source_and_archive_sha256"], "patch_sha256": build["patch_sha256"], "executable_sha256": build["executable_sha256"], "build_manifest_sha256": info["build_manifest_sha256"], "staging_manifest_sha256": info["staging_manifest_sha256"], "compiler": build["gnu_fortran_version"], "host_os": build["host_os"], "host_architecture": build["host_architecture"], "flags": build["flags"], "command": command, "input_sha256": hashes, "output_sha256": {n: sha256(p) for n,p in outputs.items()}, "selector_sha256": hashlib.sha256(selector.encode()).hexdigest(), "profile_sha256": sha256(case_dir / "PROFILE.DAT"), "runner_sha256": sha256(Path(__file__))}
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")
    return output_path


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--case", action="append")
    group.add_argument("--all", action="store_true")
    parser.add_argument("--build-dir", type=Path, required=True)
    parser.add_argument("--task-base", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=int)
    args = parser.parse_args(argv)
    development = ROOT / ".development"
    if args.task_base.resolve() == development.resolve() or development.resolve() not in args.task_base.resolve().parents:
        parser.error("--task-base must be below .development/")
    cases = sorted(p.name for p in CASES.iterdir() if p.is_dir()) if args.all else args.case
    expected = {"vapor_isothermal_head_gradient", "vapor_thermal_gradient", "vapor_off_control"}
    if set(cases) != expected:
        parser.error(f"cases must be exactly: {', '.join(sorted(expected))}")
    args.task_base.mkdir(parents=True, exist_ok=False)
    for case in cases:
        print(f"wrote {run_case(case, args.task_base, args.build_dir.resolve(), args.timeout_seconds)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

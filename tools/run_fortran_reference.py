#!/usr/bin/env python3
"""Run isolated smoke or fixture cases against the pinned Fortran snapshot."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
import sys
import uuid
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SMOKE_DIR = REPO_ROOT / "reference/fortran_reference/smoke"
BUILD_DIR = REPO_ROOT / ".development/reference-build"
RUN_DIR = BUILD_DIR / "smoke-run"
RECORD = REPO_ROOT / "reference/fortran_reference/SMOKE_RUN.md"
INPUTS = ("LEVEL_01.DIR", "SELECTOR.IN", "PROFILE.DAT", "ATMOSPH.IN")
REQUIRED_OUTPUTS = ("I_CHECK.OUT", "RUN_INF.OUT", "T_LEVEL.OUT", "NOD_INF.OUT", "BALANCE.OUT", "OBS_NODE.OUT", "PROFILE.OUT")
CASE_ROOT = REPO_ROOT / "reference/fortran_reference/solute/example3"
CASE_INPUTS = ("LEVEL_01.DIR", "SELECTOR.IN", "PROFILE.DAT")
CASE_SOLUTE_OUTPUTS = ("solute1.out", "solute2.out", "solute3.out")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run_checked(command: list[str]) -> None:
    completed = subprocess.run(command, cwd=REPO_ROOT, text=True, capture_output=True)
    if completed.returncode:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(command)}\n"
            f"stdout:\n{completed.stdout}\nstderr:\n{completed.stderr}"
        )


def require_complete_text(path: Path, marker: str) -> None:
    content = path.read_text(encoding="utf-8", errors="replace")
    if not content.endswith("\n") or marker.lower() not in content.lower():
        raise RuntimeError(f"malformed or truncated completion record: {path.name}")


def markdown_list(items: dict[str, str]) -> str:
    return "\n".join(f"- `{name}`: `{value}`" for name, value in sorted(items.items()))


def require_case_inputs(case_dir: Path, input_names: tuple[str, ...] = CASE_INPUTS) -> None:
    missing = [name for name in input_names if not (case_dir / name).is_file()]
    if missing:
        raise RuntimeError(f"missing fixture input(s): {', '.join(missing)}")
    for name in input_names:
        content = (case_dir / name).read_bytes()
        if not content or not content.endswith(b"\n"):
            raise RuntimeError(f"malformed or truncated fixture input: {name}")


def validate_outputs(outputs: dict[str, Path], completion_markers: dict[str, str]) -> dict[str, str]:
    missing = [name for name, path in outputs.items() if not path.is_file()]
    if missing:
        raise RuntimeError(f"missing required output(s): {', '.join(missing)}")
    for name, path in outputs.items():
        if path.stat().st_size == 0:
            raise RuntimeError(f"empty required output: {name}")
    for name, marker in completion_markers.items():
        if name not in outputs:
            raise RuntimeError(f"completion rule references unknown output: {name}")
        require_complete_text(outputs[name], marker)
    return {name: sha256(path) for name, path in outputs.items()}


def validate_case_outputs(outputs: dict[str, Path], solute_names: tuple[str, ...]) -> dict[str, str]:
    markers = {name: "end" for name in ("I_CHECK.OUT", "RUN_INF.OUT", "T_LEVEL.OUT", "NOD_INF.OUT", "OBS_NODE.OUT", "PROFILE.OUT", *solute_names)}
    markers["BALANCE.OUT"] = "Calculation time"
    return validate_outputs(outputs, markers)


def fixture_definition(manifest_path: Path) -> tuple[Path, dict]:
    manifest_path = manifest_path.resolve()
    raw = json.loads(manifest_path.read_text(encoding="utf-8"))
    config = raw.get("runner")
    if config is None and raw.get("case_id") == "example3_sequential_decay_derived_grid":
        config = {
            "inputs": list(CASE_INPUTS),
            "outputs": [{"name": name, "marker": "Calculation time" if name == "BALANCE.OUT" else "end"} for name in (*REQUIRED_OUTPUTS, *CASE_SOLUTE_OUTPUTS)],
            "aliases": {"SELECTOR.IN": "Selector.in", "PROFILE.DAT": "Profile.dat"},
            "data_prefix": "solute-input",
        }
    if not isinstance(config, dict) or not isinstance(config.get("inputs"), list) or not config["inputs"] or not isinstance(config.get("outputs"), list) or not config["outputs"]:
        raise RuntimeError(f"fixture manifest needs runner.inputs and runner.outputs: {manifest_path}")
    for entry in config["outputs"]:
        if not isinstance(entry, dict) or not isinstance(entry.get("name"), str) or not isinstance(entry.get("marker"), str) or not entry["marker"]:
            raise RuntimeError("each runner output needs string name and completion marker")
        if Path(entry["name"]).name != entry["name"] or "\\" in entry["name"]:
            raise RuntimeError(f"invalid fixture output name: {entry['name']!r}")
    if len({entry["name"] for entry in config["outputs"]}) != len(config["outputs"]):
        raise RuntimeError("fixture manifest declares duplicate output names")
    for name in config["inputs"]:
        if not isinstance(name, str) or Path(name).is_absolute() or ".." in Path(name).parts:
            raise RuntimeError(f"invalid fixture input path: {name!r}")
    prefix = config.get("data_prefix", "fixture-input")
    if not isinstance(prefix, str) or Path(prefix).name != prefix or "\\" in prefix:
        raise RuntimeError("runner.data_prefix must be a simple filename prefix")
    for source, alias in config.get("aliases", {}).items():
        if source not in config["inputs"] or not isinstance(alias, str) or Path(alias).name != alias or "\\" in alias:
            raise RuntimeError(f"invalid fixture input alias: {source!r} -> {alias!r}")
    return manifest_path.parent, raw | {"runner": config}


def run_fixture(fixture_manifest_path: str | Path, task_root: str | Path, timeout_seconds: int | None = None) -> dict:
    """Run one manifest-described fixture; all mutable artifacts stay below task_root."""
    manifest_path = Path(fixture_manifest_path)
    if not manifest_path.is_absolute():
        manifest_path = REPO_ROOT / manifest_path
    fixture_dir, manifest = fixture_definition(manifest_path)
    config = manifest["runner"]
    root = Path(task_root).resolve()
    development = (REPO_ROOT / ".development").resolve()
    if root == development or development not in root.parents:
        raise RuntimeError("task_root must be a unique directory below .development/")
    if root.exists():
        raise RuntimeError(f"task_root already exists; refusing to overwrite: {root}")
    if not root.parent.is_dir():
        raise RuntimeError(f"task_root parent must exist: {root.parent}")
    require_case_inputs(fixture_dir, tuple(config["inputs"]))
    root.mkdir(parents=True)
    build_dir, run_dir = root / "build", root / "run"
    build_rel = str(build_dir.relative_to(REPO_ROOT))
    run_checked(["python3", "tools/stage_fortran_reference.py", "--output", build_rel])
    run_checked(["sh", "tools/build_fortran_reference.sh", "--build-dir", build_rel])
    run_dir.mkdir()
    copied_inputs = run_dir / "committed-inputs"
    copied_inputs.mkdir()
    for name in config["inputs"]:
        source, destination = fixture_dir / name, copied_inputs / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        if source.read_bytes() != destination.read_bytes():
            raise RuntimeError(f"input copy changed bytes: {name}")

    prefix = config.get("data_prefix", "fixture-input")
    aliases = config.get("aliases", {})
    for name in config["inputs"]:
        alias = aliases.get(name)
        if alias:
            shutil.copyfile(copied_inputs / name, run_dir / f"{prefix}\\{alias}")
    command = [str(build_dir / "hydrus-reference"), prefix]
    if timeout_seconds is not None and timeout_seconds <= 0:
        raise ValueError("timeout_seconds must be positive when specified")
    try:
        completed = subprocess.run(command, cwd=run_dir, text=True, capture_output=True, timeout=timeout_seconds)
    except subprocess.TimeoutExpired as error:
        def as_text(value):
            return value.decode("utf-8", errors="replace") if isinstance(value, bytes) else (value or "")

        (run_dir / "stdout.txt").write_text(as_text(error.stdout), encoding="utf-8")
        (run_dir / "stderr.txt").write_text(as_text(error.stderr), encoding="utf-8")
        raise RuntimeError(f"fixture run timed out{f' after {timeout_seconds} seconds' if timeout_seconds else ''}; incomplete run retained at {run_dir}") from error
    (run_dir / "stdout.txt").write_text(completed.stdout, encoding="utf-8")
    (run_dir / "stderr.txt").write_text(completed.stderr, encoding="utf-8")
    if completed.returncode:
        raise RuntimeError(f"fixture run exited {completed.returncode}; incomplete run retained at {run_dir}")
    if (run_dir / f"{prefix}\\Error.msg").exists():
        raise RuntimeError(f"reference reported an input/output error; incomplete run retained at {run_dir}")

    outputs = {entry["name"]: run_dir / f"{prefix}\\{entry['name']}" for entry in config["outputs"]}
    hashes = validate_outputs(outputs, {entry["name"]: entry["marker"] for entry in config["outputs"]})
    build = json.loads((build_dir / "build-manifest.json").read_text(encoding="utf-8"))
    staged = json.loads((build_dir / "staging-manifest.json").read_text(encoding="utf-8"))
    warning_lines = [line for line in (completed.stdout + "\n" + completed.stderr).splitlines() if re.search(r"warning|exception", line, re.I)]
    result = {
        "schema_version": 1,
        "fixture_manifest": str(manifest_path),
        "fixture_id": manifest.get("case_id", manifest.get("fixture_id", manifest_path.stem)),
        "complete": True,
        "task_root": str(root),
        "run_root": str(run_dir),
        "timeout_seconds": timeout_seconds,
        "command": command,
        "returncode": completed.returncode,
        "source_and_archive_sha256": staged["source_and_archive_sha256"],
        "patch_sha256": staged["patch_sha256"],
        "executable_sha256": build["executable_sha256"],
        "compiler": build["gnu_fortran_version"],
        "host_os": build["host_os"],
        "host_architecture": build["host_architecture"],
        "flags": build["flags"],
        "input_sha256": {name: sha256(copied_inputs / name) for name in config["inputs"]},
        "output_paths": {name: path.relative_to(root).as_posix() for name, path in outputs.items()},
        "output_sha256": hashes,
        "capture_sha256": {name: sha256(run_dir / name) for name in ("stdout.txt", "stderr.txt")},
        "warnings": warning_lines,
    }
    (root / "run-manifest.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def run_case(case_arg: str, timeout_seconds: int | None = None, task_root: str | None = None) -> None:
    case_dir = (REPO_ROOT / case_arg).resolve()
    if case_dir != CASE_ROOT.resolve():
        raise RuntimeError(f"only the reviewed Example 3 case is supported: {CASE_ROOT.relative_to(REPO_ROOT)}")
    task_root = Path(task_root) if task_root else REPO_ROOT / ".development/12.2.1/runs" / uuid.uuid4().hex
    task_root.parent.mkdir(parents=True, exist_ok=True)
    result = run_fixture(CASE_ROOT / "CASE.json", task_root, timeout_seconds)
    print(f"fixture run completed; run manifest: {task_root / 'run-manifest.json'}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--smoke", action="store_true", help="run the committed noncanonical smoke case")
    modes.add_argument("--case", help="run the reviewed solute fixture directory")
    modes.add_argument("--fixture-manifest", help="run one manifest-described fixture")
    parser.add_argument("--timeout-seconds", type=int, help="optional process timeout; omitted means no timeout")
    parser.add_argument("--task-root", help="unique mutable build/run root below .development/")
    args = parser.parse_args()
    if args.timeout_seconds is not None and args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be positive")
    if args.case:
        run_case(args.case, args.timeout_seconds, args.task_root)
        return 0
    if args.fixture_manifest:
        if not args.task_root:
            parser.error("--fixture-manifest requires --task-root")
        result = run_fixture(args.fixture_manifest, args.task_root, args.timeout_seconds)
        print(f"fixture run completed: {result['run_root']}")
        return 0
    missing = [name for name in INPUTS if not (SMOKE_DIR / name).is_file()]
    if missing:
        raise RuntimeError(f"missing committed smoke input(s): {', '.join(missing)}")
    for name in INPUTS:
        content = (SMOKE_DIR / name).read_bytes()
        if not content or not content.endswith(b"\n"):
            raise RuntimeError(f"malformed or truncated committed smoke input: {name}")

    run_checked(["python3", "tools/stage_fortran_reference.py", "--output", ".development/reference-build"])
    run_checked(["sh", "tools/build_fortran_reference.sh", "--build-dir", ".development/reference-build"])
    if RUN_DIR.exists():
        shutil.rmtree(RUN_DIR)
    RUN_DIR.mkdir(parents=True)
    copied_inputs = RUN_DIR / "committed-inputs"
    copied_inputs.mkdir()
    for name in INPUTS:
        source = SMOKE_DIR / name
        destination = copied_inputs / name
        shutil.copyfile(source, destination)
        if source.read_bytes() != destination.read_bytes():
            raise RuntimeError(f"input copy changed bytes: {name}")

    # The retained legacy source concatenates a literal backslash. On Linux that
    # is a filename character, so create byte-identical aliases in the run root.
    data_prefix = "smoke-input"
    legacy_names = {
        "SELECTOR.IN": "Selector.in",
        "PROFILE.DAT": "Profile.dat",
        "ATMOSPH.IN": "ATMOSPH.IN",
    }
    for name, legacy_name in legacy_names.items():
        shutil.copyfile(copied_inputs / name, RUN_DIR / f"{data_prefix}\\{legacy_name}")
    executable = BUILD_DIR / "hydrus-reference"
    command = [str(executable), data_prefix]
    try:
        completed = subprocess.run(command, cwd=RUN_DIR, text=True, capture_output=True, timeout=args.timeout_seconds)
    except subprocess.TimeoutExpired as error:
        (RUN_DIR / "stdout.txt").write_text(error.stdout or "", encoding="utf-8")
        (RUN_DIR / "stderr.txt").write_text(error.stderr or "", encoding="utf-8")
        raise RuntimeError(f"smoke run timed out after {args.timeout_seconds} seconds") from error
    (RUN_DIR / "stdout.txt").write_text(completed.stdout, encoding="utf-8")
    (RUN_DIR / "stderr.txt").write_text(completed.stderr, encoding="utf-8")
    if completed.returncode:
        raise RuntimeError(f"smoke run exited {completed.returncode}; inspect {RUN_DIR}")

    outputs = {name: RUN_DIR / f"{data_prefix}\\{name}" for name in REQUIRED_OUTPUTS}
    missing_outputs = [name for name, path in outputs.items() if not path.is_file()]
    if missing_outputs:
        raise RuntimeError(f"missing required output(s): {', '.join(missing_outputs)}")
    for name in ("I_CHECK.OUT", "RUN_INF.OUT", "T_LEVEL.OUT", "NOD_INF.OUT", "OBS_NODE.OUT", "PROFILE.OUT"):
        require_complete_text(outputs[name], "end")
    require_complete_text(outputs["BALANCE.OUT"], "Calculation time")
    if "error" in completed.stdout.lower() or "error" in completed.stderr.lower():
        raise RuntimeError("warnings/output contain an error marker")

    input_hashes = {name: sha256(SMOKE_DIR / name) for name in INPUTS}
    output_hashes = {name: sha256(path) for name, path in outputs.items()}
    build = json.loads((BUILD_DIR / "build-manifest.json").read_text(encoding="utf-8"))
    staged = json.loads((BUILD_DIR / "staging-manifest.json").read_text(encoding="utf-8"))
    warning_lines = [line for line in completed.stderr.splitlines() if line.strip()]
    warnings = "none observed" if not warning_lines else "\n".join(f"- `{line}`" for line in warning_lines)
    capture_hashes = {"stdout.txt": sha256(RUN_DIR / "stdout.txt"), "stderr.txt": sha256(RUN_DIR / "stderr.txt")}
    timeout_option = f" --timeout-seconds {args.timeout_seconds}" if args.timeout_seconds is not None else ""
    timeout_status = f"within the requested {args.timeout_seconds}-second timeout" if args.timeout_seconds is not None else "with no process timeout"
    RECORD.write_text(
        "# Noncanonical smoke run\n\n"
        "This record is execution provenance only. The inputs and outputs are not a scientific fixture, baseline, parity result, or numerical validation.\n\n"
        "## Input status\n\n"
        "All four committed smoke inputs were copied byte-for-byte into a fresh isolated run directory. They are independently constructed and noncanonical.\n\n"
        "## Command\n\n"
        f"`python3 tools/run_fortran_reference.py --smoke{timeout_option}`\n\n"
        f"Reference invocation: `{' '.join(command)}` in `.development/reference-build/smoke-run`.\n\n"
        "## Hashes\n\n"
        "### Inputs\n\n" + markdown_list(input_hashes) + "\n\n"
        "### Outputs\n\n" + markdown_list(output_hashes) + "\n\n"
        "### Captures\n\n" + markdown_list(capture_hashes) + "\n\n"
        f"- patch: `{staged['patch_sha256']}`\n"
        f"- executable: `{build['executable_sha256']}`\n\n"
        "## Environment\n\n"
        f"- GNU Fortran: `{build['gnu_fortran_version']}`\n"
        f"- OS: `{build['host_os']}`\n"
        f"- architecture: `{build['host_architecture']}`\n"
        f"- flags: `{' '.join(build['flags'])}`\n"
        f"- Python: `{platform.python_version()}`\n\n"
        "## Exit status\n\n"
        f"`0` {timeout_status}.\n\n"
        "## Completion evidence\n\n"
        "`I_CHECK.OUT`, `RUN_INF.OUT`, `T_LEVEL.OUT`, `NOD_INF.OUT`, `OBS_NODE.OUT`, and `PROFILE.OUT` each contain the legacy `end` marker; `BALANCE.OUT` contains its final `Calculation time` record; the executable returned zero.\n\n"
        "## Output inventory\n\n" + markdown_list(output_hashes) + "\n\n"
        "## Warnings\n\n"
        f"{warnings}\n\nCaptured stdout and stderr are retained untracked under `.development/reference-build/smoke-run/`.\n\n"
        "## Limitations\n\n"
        "This proves only that the patched reference executable completed this deliberately minimal, noncanonical run on this GNU Fortran/Linux environment. It establishes no physical correctness, fixture provenance, parity, tolerance, baseline, release identity, or redistribution permission.\n",
        encoding="utf-8",
    )
    record = RECORD.read_text(encoding="utf-8")
    required_headings = (
        "## Input status", "## Command", "## Hashes", "## Environment",
        "## Exit status", "## Completion evidence", "## Output inventory",
        "## Warnings", "## Limitations",
    )
    if not record.endswith("\n") or not all(heading in record for heading in required_headings):
        raise RuntimeError("malformed or truncated smoke provenance record")
    print(f"smoke run completed; provenance record: {RECORD.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)

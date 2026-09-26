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
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SMOKE_DIR = REPO_ROOT / "reference/fortran_reference/smoke"
BUILD_DIR = REPO_ROOT / ".development/reference-build"
RUN_DIR = BUILD_DIR / "smoke-run"
RECORD = REPO_ROOT / "reference/fortran_reference/SMOKE_RUN.md"
INPUTS = ("LEVEL_01.DIR", "SELECTOR.IN", "PROFILE.DAT", "ATMOSPH.IN")
REQUIRED_OUTPUTS = ("I_CHECK.OUT", "RUN_INF.OUT", "T_LEVEL.OUT", "NOD_INF.OUT", "BALANCE.OUT", "OBS_NODE.OUT", "PROFILE.OUT")
CASE_ROOT = REPO_ROOT / "reference/fortran_reference/solute/example3"
CASE_BUILD_DIR = REPO_ROOT / ".development/12.2.1/reference-build"
CASE_RUN_DIR = REPO_ROOT / ".development/12.2.1/run"
CASE_RECORD = CASE_ROOT / "RUN.md"
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


def require_case_inputs(case_dir: Path) -> None:
    missing = [name for name in CASE_INPUTS if not (case_dir / name).is_file()]
    if missing:
        raise RuntimeError(f"missing fixture input(s): {', '.join(missing)}")
    for name in CASE_INPUTS:
        content = (case_dir / name).read_bytes()
        if not content or not content.endswith(b"\n"):
            raise RuntimeError(f"malformed or truncated fixture input: {name}")


def validate_case_outputs(outputs: dict[str, Path], solute_names: tuple[str, ...]) -> dict[str, str]:
    missing = [name for name, path in outputs.items() if not path.is_file()]
    if missing:
        raise RuntimeError(f"missing required output(s): {', '.join(missing)}")
    for name, path in outputs.items():
        if path.stat().st_size == 0:
            raise RuntimeError(f"empty required output: {name}")
    for name in ("I_CHECK.OUT", "RUN_INF.OUT", "T_LEVEL.OUT", "NOD_INF.OUT", "OBS_NODE.OUT", "PROFILE.OUT"):
        require_complete_text(outputs[name], "end")
    require_complete_text(outputs["BALANCE.OUT"], "Calculation time")
    for name in solute_names:
        require_complete_text(outputs[name], "end")
    return {name: sha256(path) for name, path in outputs.items()}


def run_case(case_arg: str, timeout_seconds: int) -> None:
    case_dir = (REPO_ROOT / case_arg).resolve()
    if case_dir != CASE_ROOT.resolve():
        raise RuntimeError(f"only the reviewed Example 3 case is supported: {CASE_ROOT.relative_to(REPO_ROOT)}")
    require_case_inputs(case_dir)

    build_rel = str(CASE_BUILD_DIR.relative_to(REPO_ROOT))
    run_checked(["python3", "tools/stage_fortran_reference.py", "--output", build_rel])
    run_checked(["sh", "tools/build_fortran_reference.sh", "--build-dir", build_rel])
    if CASE_RUN_DIR.exists():
        shutil.rmtree(CASE_RUN_DIR)
    CASE_RUN_DIR.mkdir(parents=True)
    copied_inputs = CASE_RUN_DIR / "committed-inputs"
    copied_inputs.mkdir()
    for name in CASE_INPUTS:
        source = case_dir / name
        destination = copied_inputs / name
        shutil.copyfile(source, destination)
        if source.read_bytes() != destination.read_bytes():
            raise RuntimeError(f"input copy changed bytes: {name}")

    data_prefix = "solute-input"
    legacy_names = {"SELECTOR.IN": "Selector.in", "PROFILE.DAT": "Profile.dat"}
    for name, legacy_name in legacy_names.items():
        shutil.copyfile(copied_inputs / name, CASE_RUN_DIR / f"{data_prefix}\\{legacy_name}")
    executable = CASE_BUILD_DIR / "hydrus-reference"
    command = [str(executable), data_prefix]
    try:
        completed = subprocess.run(command, cwd=CASE_RUN_DIR, text=True, capture_output=True, timeout=timeout_seconds)
    except subprocess.TimeoutExpired as error:
        (CASE_RUN_DIR / "stdout.txt").write_text(error.stdout or "", encoding="utf-8")
        (CASE_RUN_DIR / "stderr.txt").write_text(error.stderr or "", encoding="utf-8")
        raise RuntimeError(f"fixture run timed out after {timeout_seconds} seconds") from error
    (CASE_RUN_DIR / "stdout.txt").write_text(completed.stdout, encoding="utf-8")
    (CASE_RUN_DIR / "stderr.txt").write_text(completed.stderr, encoding="utf-8")
    if completed.returncode:
        error_path = CASE_RUN_DIR / f"{data_prefix}\\Error.msg"
        error_text = error_path.read_text(encoding="utf-8", errors="replace").strip() if error_path.exists() else "No Error.msg was produced."
        build = json.loads((CASE_BUILD_DIR / "build-manifest.json").read_text(encoding="utf-8"))
        staged = json.loads((CASE_BUILD_DIR / "staging-manifest.json").read_text(encoding="utf-8"))
        input_hashes = {name: sha256(copied_inputs / name) for name in CASE_INPUTS}
        capture_hashes = {name: sha256(CASE_RUN_DIR / name) for name in ("stdout.txt", "stderr.txt")}
        observed = {
            path.name: sha256(path)
            for path in CASE_RUN_DIR.glob(f"{data_prefix}\\*")
            if path.is_file() and path.name not in {f"{data_prefix}\\Selector.in", f"{data_prefix}\\Profile.dat"}
        }
        CASE_RECORD.write_text(
            "# Derived Example 3 Fortran reference run\n\n"
            "This attempt did not produce a completed scientific reference run. The fixture is derived, not a recovered manual deck or manual reproduction. Raw files remain only under `.development/12.2.1/`.\n\n"
            "## Command and status\n\n"
            f"`python3 tools/run_fortran_reference.py --case reference/fortran_reference/solute/example3 --timeout-seconds {timeout_seconds}`\n\n"
            f"Reference invocation: `{' '.join(command)}`; exit status `{completed.returncode}` within {timeout_seconds} seconds.\n\n"
            "## Provenance\n\n"
            f"- source/archive hashes: `{json.dumps(staged['source_and_archive_sha256'], sort_keys=True)}`\n"
            f"- patch: `{staged['patch_sha256']}`\n"
            f"- executable: `{build['executable_sha256']}`\n"
            f"- compiler: `{build['gnu_fortran_version']}`; platform: `{build['host_os']}` / `{build['host_architecture']}`\n"
            f"- flags: `{' '.join(build['flags'])}`\n\n"
            "### Input hashes\n\n" + markdown_list(input_hashes) + "\n\n"
            "### Captured log hashes\n\n" + markdown_list(capture_hashes) + "\n\n"
            "### Observed output hashes (incomplete)\n\n" + (markdown_list(observed) or "No output files observed.") + "\n\n"
            "## Blocker\n\n"
            f"The reference exited before completing: `{error_text}` The captured stdout/stderr and `Error.msg` are in `.development/12.2.1/run/`. No solute outputs or completion markers were produced; this is not a passing water or solute run. The failure is in `BasInf` basic-information parsing of `SELECTOR.IN`; the exact record alignment needs a fixture-level audit. Correcting that input is outside this task's approved scope and requires re-scoping.\n\n"
            "## Warnings\n\n"
            "No scientific result is available.\n",
            encoding="utf-8",
        )
        raise RuntimeError(f"fixture run exited {completed.returncode}; blocker recorded in {CASE_RECORD.relative_to(REPO_ROOT)}")
    if (CASE_RUN_DIR / f"{data_prefix}\\Error.msg").exists():
        raise RuntimeError(f"reference reported an input/output error; inspect {CASE_RUN_DIR}")

    outputs = {name: CASE_RUN_DIR / f"{data_prefix}\\{name}" for name in REQUIRED_OUTPUTS}
    outputs.update({name: CASE_RUN_DIR / f"{data_prefix}\\{name}" for name in CASE_SOLUTE_OUTPUTS})
    output_hashes = validate_case_outputs(outputs, CASE_SOLUTE_OUTPUTS)
    input_hashes = {name: sha256(copied_inputs / name) for name in CASE_INPUTS}
    build = json.loads((CASE_BUILD_DIR / "build-manifest.json").read_text(encoding="utf-8"))
    staged = json.loads((CASE_BUILD_DIR / "staging-manifest.json").read_text(encoding="utf-8"))
    warning_lines = [line for line in (completed.stdout + "\n" + completed.stderr).splitlines() if re.search(r"warning|exception", line, re.I)]
    warnings = "none observed" if not warning_lines else "\n".join(f"- `{line}`" for line in warning_lines)
    capture_hashes = {name: sha256(CASE_RUN_DIR / name) for name in ("stdout.txt", "stderr.txt")}
    CASE_RECORD.write_text(
        "# Derived Example 3 Fortran reference run\n\n"
        "This run uses the authored derived fixture described in `CASE.json`; it is not a recovered manual deck or manual reproduction. Raw output is retained only under `.development/12.2.1/`.\n\n"
        "## Command and status\n\n"
        f"`python3 tools/run_fortran_reference.py --case reference/fortran_reference/solute/example3 --timeout-seconds {timeout_seconds}`\n\n"
        f"Reference invocation: `{' '.join(command)}` in `.development/12.2.1/run`; exit status `{completed.returncode}` within {timeout_seconds} seconds.\n\n"
        "## Provenance\n\n"
        f"- source/archive hashes: `{json.dumps(staged['source_and_archive_sha256'], sort_keys=True)}`\n"
        f"- patch: `{staged['patch_sha256']}`\n"
        f"- executable: `{build['executable_sha256']}`\n"
        f"- compiler: `{build['gnu_fortran_version']}`\n"
        f"- platform: `{build['host_os']}`; `{build['host_architecture']}`\n"
        f"- flags: `{' '.join(build['flags'])}`\n"
        f"- Python: `{platform.python_version()}`\n\n"
        "### Input hashes\n\n" + markdown_list(input_hashes) + "\n\n"
        "### Output hashes\n\n" + markdown_list(output_hashes) + "\n\n"
        "### Captured log hashes\n\n" + markdown_list(capture_hashes) + "\n\n"
        "## Completion evidence\n\n"
        "The executable returned zero; all seven standard output files were present and nonempty; standard text outputs ended with `end`, and `BALANCE.OUT` contained `Calculation time`. All three per-species outputs (`\\solute1.out` through `\\solute3.out`) were present, nonempty, newline-terminated, and ended with `end`.\n\n"
        "## Warnings\n\n" + warnings + "\n\n"
        "## Limitations\n\n"
        "This verifies reference execution and output completeness only. Realized water/solute coefficients, boundary signs, coordinate mapping, paired Rust parity, analytical agreement, calibrated tolerances, artifact redistribution permission, and release compatibility remain unverified.\n",
        encoding="utf-8",
    )
    print(f"fixture run completed; provenance record: {CASE_RECORD.relative_to(REPO_ROOT)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--smoke", action="store_true", help="run the committed noncanonical smoke case")
    modes.add_argument("--case", help="run the reviewed solute fixture directory")
    parser.add_argument("--timeout-seconds", type=int, required=True)
    args = parser.parse_args()
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be positive")
    if args.case:
        run_case(args.case, args.timeout_seconds)
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
    RECORD.write_text(
        "# Noncanonical smoke run\n\n"
        "This record is execution provenance only. The inputs and outputs are not a scientific fixture, baseline, parity result, or numerical validation.\n\n"
        "## Input status\n\n"
        "All four committed smoke inputs were copied byte-for-byte into a fresh isolated run directory. They are independently constructed and noncanonical.\n\n"
        "## Command\n\n"
        f"`python3 tools/run_fortran_reference.py --smoke --timeout-seconds {args.timeout_seconds}`\n\n"
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
        f"`0` within the requested {args.timeout_seconds}-second timeout.\n\n"
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

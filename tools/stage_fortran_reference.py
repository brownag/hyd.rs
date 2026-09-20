#!/usr/bin/env python3
"""Stage a hash-guarded, patched copy of the immutable Fortran reference."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = REPO_ROOT / "planning/master/SOURCE_MANIFEST.json"
ARCHIVE_PATH = REPO_ROOT / "planning/H1D_Src.zip"
SOURCE_DIR = REPO_ROOT / "src_reference"
PATCH_PATH = REPO_ROOT / "reference/fortran_reference/0001-gfortran-portability.patch"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def checked_output(value: str) -> Path:
    output = Path(value).resolve()
    development = (REPO_ROOT / ".development").resolve()
    if output == development or development not in output.parents:
        raise ValueError("--output must be a directory below .development/")
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, help="untracked directory below .development/")
    args = parser.parse_args()

    try:
        output = checked_output(args.output)
    except ValueError as error:
        parser.error(str(error))

    for required in (MANIFEST_PATH, ARCHIVE_PATH, SOURCE_DIR, PATCH_PATH):
        if not required.exists():
            print(f"error: required input is missing: {required}", file=sys.stderr)
            return 2

    try:
        expected = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"error: cannot read source manifest: {error}", file=sys.stderr)
        return 2

    input_hashes: dict[str, str] = {}
    archive_key = "planning/H1D_Src.zip"
    inputs = [(archive_key, ARCHIVE_PATH)] + [
        (f"src_reference/{name}", SOURCE_DIR / name)
        for name in sorted(path.name for path in SOURCE_DIR.glob("*.FOR"))
    ]
    for key, path in inputs:
        actual = sha256(path)
        input_hashes[key] = actual
        if expected.get(key) != actual:
            print(
                f"error: immutable input hash mismatch for {key}: expected "
                f"{expected.get(key)}, got {actual}",
                file=sys.stderr,
            )
            return 2
    source_keys = sorted(key for key in expected if key.startswith("src_reference/"))
    if sorted(key for key, _ in inputs[1:]) != source_keys:
        print("error: source snapshot does not match the pinned source inventory", file=sys.stderr)
        return 2

    archive_matches: dict[str, bool] = {}
    try:
        with zipfile.ZipFile(ARCHIVE_PATH) as archive:
            for key in source_keys:
                name = Path(key).name
                archive_matches[key] = archive.read(name) == (SOURCE_DIR / name).read_bytes()
    except (OSError, KeyError, zipfile.BadZipFile) as error:
        print(f"error: cannot compare source archive members: {error}", file=sys.stderr)
        return 2
    if not all(archive_matches.values()):
        failures = ", ".join(key for key, matched in archive_matches.items() if not matched)
        print(f"error: archive members differ from immutable source files: {failures}", file=sys.stderr)
        return 2

    output.mkdir(parents=True, exist_ok=True)
    staged_source = output / "src_reference"
    if staged_source.exists():
        shutil.rmtree(staged_source)
    shutil.copytree(SOURCE_DIR, staged_source)
    apply = subprocess.run(
        ["git", "apply", "--check", str(PATCH_PATH)],
        cwd=staged_source,
        text=True,
        capture_output=True,
    )
    if apply.returncode:
        print(f"error: portability patch does not apply:\n{apply.stderr}", file=sys.stderr)
        shutil.rmtree(staged_source)
        return apply.returncode
    apply = subprocess.run(["git", "apply", str(PATCH_PATH)], cwd=staged_source)
    if apply.returncode:
        print("error: portability patch application failed", file=sys.stderr)
        shutil.rmtree(staged_source)
        return apply.returncode

    metadata = {
        "archive_member_byte_match": archive_matches,
        "artifact_notice_review_status": "unreviewed; no license or redistribution approval inferred",
        "command": ["python3", "tools/stage_fortran_reference.py", "--output", args.output],
        "patch": "reference/fortran_reference/0001-gfortran-portability.patch",
        "patch_sha256": sha256(PATCH_PATH),
        "source_and_archive_sha256": input_hashes,
        "staged_source": "src_reference",
        "utc_date": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
    }
    (output / "staging-manifest.json").write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"staged patched reference source in {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

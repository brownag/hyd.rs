"""Authenticate raw source-only solute fixture runs and compare paired values."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PARITY_FIXTURE_ROOT = ROOT / "reference/fortran_reference/solute/parity"
FIXTURE_GROUPS = {
    "transport": "transport",
    "reactions": "reactions",
    "transport_properties": "transport_properties",
    "nonequilibrium": "processes/nonequilibrium",
    "biology": "processes/biology",
}
PINNED_PATCH_SHA256 = "622d1ea68285257e98a169f94dec83f3df1c671a9366f651e845806cb8c00d7e"
if __package__ in (None, ""):
    sys.path.insert(0, str(ROOT))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_hash(value: object) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    return hashlib.sha256(encoded).hexdigest()


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_unique_object,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(f"invalid number: {value}")))


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate_fixture(path: Path) -> tuple[dict, dict]:
    manifest = load_json(path)
    _require(manifest.get("schema_version") == "solute-parity-fixture-v1", "unsupported fixture manifest schema")
    _require(isinstance(manifest.get("fixture_id"), str) and manifest["fixture_id"], "missing fixture_id")
    request = manifest.get("request")
    _require(isinstance(request, dict), "missing normalized request")
    _require(isinstance(request.get("schema_version"), str), "normalized request requires schema_version")
    _require(manifest.get("request_sha256") == canonical_hash(request), "normalized request hash mismatch")
    fields = request.get("fields")
    _require(isinstance(fields, dict) and fields, "normalized request requires named fields")
    for name, item in fields.items():
        _require(isinstance(name, str) and name, "invalid request field identity")
        _require(isinstance(item, dict) and isinstance(item.get("units"), str) and item["units"],
                 f"field {name}: explicit units required")
        if "value" in item:
            _finite_tree(item["value"], f"field {name}")
        else:
            _require(item.get("availability") and item.get("reason"),
                     f"field {name}: missing value must have availability and reason")
    files = manifest.get("input_files")
    _require(isinstance(files, dict) and files, "input_files must be a nonempty path-to-SHA256 map")
    for relative, expected in files.items():
        _require(isinstance(expected, str) and len(expected) == 64 and all(c in "0123456789abcdef" for c in expected),
                 f"invalid input hash for {relative}")
        source = (path.parent / relative).resolve()
        _require(source.is_relative_to(path.parent.resolve()), f"input escapes fixture directory: {relative}")
        _require(source.is_file() and sha256(source) == expected, f"input missing or hash mismatch: {relative}")
    return manifest, request


def _finite_tree(value, label):
    if isinstance(value, bool) or value is None or isinstance(value, str):
        return
    if isinstance(value, (int, float)):
        _require(math.isfinite(value), f"{label}: non-finite number")
    elif isinstance(value, list):
        for item in value:
            _finite_tree(item, label)
    elif isinstance(value, dict):
        for item in value.values():
            _finite_tree(item, label)
    else:
        raise ValueError(f"{label}: unsupported value")


def _hash_string(value, label: str) -> None:
    _require(isinstance(value, str) and len(value) == 64 and
             all(char in "0123456789abcdef" for char in value), f"invalid {label} SHA-256")


def _validate_raw_result(result: dict, task_root: Path, fixture: dict) -> dict:
    """Authenticate the runner's raw manifest and every declared output artifact."""
    _require(isinstance(result, dict), "runner must return a raw result manifest")
    _require(result.get("complete") is True, "reference run is incomplete")
    root = task_root.resolve()
    _require(Path(result.get("task_root", "")).resolve() == root,
             "runner task_root differs from requested task root")
    run_root = Path(result.get("run_root", "")).resolve()
    _require(run_root.is_relative_to(root), "runner run_root escapes task root")
    _require(run_root.is_dir(), "runner run_root is missing")
    inputs = result.get("input_sha256")
    _require(inputs == fixture["input_files"], "reference run input identity/hash mismatch")

    pinned_sources = load_json(ROOT / "planning/master/SOURCE_MANIFEST.json")
    staged_sources = {
        name: digest for name, digest in pinned_sources.items()
        if name == "planning/H1D_Src.zip" or (name.startswith("src_reference/") and name.endswith(".FOR"))
    }
    sources = result.get("source_and_archive_sha256")
    _require(sources == staged_sources, "reference source/archive hashes differ from pinned manifest")
    _require(result.get("patch_sha256") == PINNED_PATCH_SHA256, "reference patch hash differs from pinned patch")
    _hash_string(result.get("executable_sha256"), "executable")

    paths = result.get("output_paths")
    outputs = result.get("output_sha256")
    _require(isinstance(paths, dict) and paths, "runner has no declared output paths")
    _require(isinstance(outputs, dict) and outputs, "runner has no declared output hashes")
    _require(paths.keys() == outputs.keys(), "output path/hash logical names differ")
    resolved_outputs = set()
    for logical_name, relative in paths.items():
        _require(isinstance(logical_name, str) and logical_name, "invalid output logical name")
        _require(isinstance(relative, str) and relative, f"invalid output path for {logical_name}")
        expected = outputs[logical_name]
        _hash_string(expected, f"output {logical_name}")
        artifact = (root / relative).resolve()
        _require(artifact.is_relative_to(root), f"output path escapes task root: {relative}")
        _require(artifact not in resolved_outputs, f"duplicate output path: {relative}")
        resolved_outputs.add(artifact)
        _require(artifact.is_file(), f"declared output missing: {relative}")
        _require(sha256(artifact) == expected, f"output hash mismatch: {logical_name}")
    return result


def compare_observations(reference: list[dict], candidate: list[dict], rules: dict) -> dict:
    """Compare paired backends only after exact field/location identity checks."""
    ref = {(item["field"], canonical_hash(item["location"])): item for item in reference}
    got = {(item["field"], canonical_hash(item["location"])): item for item in candidate}
    _require(len(ref) == len(reference) and len(got) == len(candidate), "duplicate comparison observation")
    _require(ref.keys() == got.keys(), "backend observation field identity mismatch")
    records = []
    for key, original in ref.items():
        item = got[key]
        field_name = original["field"]
        _require(original.get("units") == item.get("units"), f"field identity mismatch: {field_name} units")
        rule = rules.get(field_name)
        _require(isinstance(rule, dict), f"missing acceptance rule: {field_name}")
        left, right = original.get("value"), item.get("value")
        _require(type(left) is type(right), f"field identity mismatch: {field_name} value type")
        if isinstance(left, (int, float)) and not isinstance(left, bool):
            if "intervals" in rule:
                low_a, high_a = rule["intervals"][0]
                low_b, high_b = rule["intervals"][1]
                passed = max(low_a, low_b) <= min(high_a, high_b)
            else:
                atol, rtol = rule.get("atol"), rule.get("rtol")
                _require(isinstance(atol, (int, float)) and isinstance(rtol, (int, float)) and atol >= 0 and rtol >= 0,
                         f"field {field_name}: invalid acceptance rule")
                passed = abs(right - left) <= atol + rtol * max(abs(left), abs(right))
            records.append({"field": field_name, "location": original["location"], "status": "pass" if passed else "fail",
                            "reference": left, "candidate": right, "difference": right - left})
            if not passed:
                return {"status": "fail", "first_divergence": records[-1], "records": records}
        else:
            passed = left == right
            records.append({"field": field_name, "location": original["location"], "status": "pass" if passed else "fail",
                            "reference": left, "candidate": right})
            if not passed:
                return {"status": "fail", "first_divergence": records[-1], "records": records}
    return {"status": "pass", "first_divergence": None, "records": records}


def run_reference_only(fixture_manifest_path: Path, task_root: Path) -> dict:
    fixture, request = validate_fixture(fixture_manifest_path)
    from tools.run_fortran_reference import run_fixture

    task_root.mkdir(parents=True, exist_ok=False)
    runner_task_root = task_root / "reference-run"
    result = run_fixture(fixture_manifest_path, runner_task_root, timeout_seconds=None)
    raw_result = _validate_raw_result(result, runner_task_root, fixture)
    return {"schema_version": "solute-parity-result-v1", "mode": "reference-only", "fixture_id": fixture["fixture_id"],
            "request_sha256": fixture["request_sha256"], "task_root": str(task_root),
            "run_root": raw_result["run_root"],
            "runner_manifest_sha256": canonical_hash(raw_result), "status": "authenticated",
            "result_kind": "raw-fortran-outputs", "normalized_parity": False,
            "output_paths": raw_result["output_paths"], "output_sha256": raw_result["output_sha256"],
            "input_sha256": raw_result["input_sha256"],
            "source_and_archive_sha256": raw_result["source_and_archive_sha256"],
            "patch_sha256": raw_result["patch_sha256"],
            "executable_sha256": raw_result["executable_sha256"],
             "comparison": None}


def discover_fixture_manifests(group: str, fixture_root: Path = PARITY_FIXTURE_ROOT) -> list[Path]:
    _require(group == "all" or group in FIXTURE_GROUPS, f"unknown fixture group: {group}")
    root = fixture_root if group == "all" else fixture_root / FIXTURE_GROUPS[group]
    return sorted(root.rglob("CASE.json")) if root.is_dir() else []


def run_fixture_group(group: str, task_root: Path, fixture_root: Path = PARITY_FIXTURE_ROOT) -> dict:
    manifests = discover_fixture_manifests(group, fixture_root)
    _require(manifests, f"fixture group has no CASE.json manifests: {group}")
    fixtures = [validate_fixture(path)[0] for path in manifests]
    fixture_ids = [fixture["fixture_id"] for fixture in fixtures]
    for fixture_id in fixture_ids:
        _require(isinstance(fixture_id, str) and
                 re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", fixture_id) is not None and
                 fixture_id not in (".", ".."), f"unsafe fixture_id: {fixture_id}")
    _require(len(set(fixture_ids)) == len(fixture_ids), "duplicate fixture_id in selected group")

    task_root.mkdir(parents=True, exist_ok=False)
    results = []
    for index, manifest in enumerate(manifests):
        child_root = task_root / f"{index:04d}-{fixtures[index]['fixture_id']}"
        results.append(run_reference_only(manifest, child_root))
    return {"schema_version": "solute-parity-group-result-v1", "mode": "reference-only",
            "group": group, "status": "authenticated", "result_kind": "raw-fortran-outputs",
            "normalized_parity": False, "task_root": str(task_root), "results": results}


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference-only", action="store_true", required=True)
    selection = parser.add_mutually_exclusive_group(required=True)
    selection.add_argument("--fixture-manifest")
    selection.add_argument("--fixture-group", choices=[*FIXTURE_GROUPS, "all"])
    parser.add_argument("--task-root", required=True)
    args = parser.parse_args(argv)
    if args.fixture_manifest:
        output = run_reference_only(Path(args.fixture_manifest).resolve(), Path(args.task_root).resolve())
    else:
        output = run_fixture_group(args.fixture_group, Path(args.task_root).resolve())
    print(json.dumps(output, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

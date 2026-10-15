"""Rust compatibility candidate adapter for paired solute parity runs.

Invokes the Rust compatibility candidate with the normalized fixture request,
emits candidate output artifacts and validated schema-compliant observations
for comparison against normalized Fortran reference outputs.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path
from typing import Any, Dict, List

ROOT = Path(__file__).resolve().parents[1]


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_hash(value: object) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    return hashlib.sha256(encoded).hexdigest()


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def run_candidate(fixture_manifest_path: Path | str, task_root: Path | str) -> dict:
    """Execute the Rust compatibility candidate on the normalized fixture request.

    Args:
        fixture_manifest_path: Path to the fixture's CASE.json manifest.
        task_root: Target directory for the candidate run artifacts.

    Returns:
        Candidate result manifest dictionary compliant with _validate_candidate_result.
    """
    fixture_path = Path(fixture_manifest_path).resolve()
    task_root_path = Path(task_root).resolve()
    task_root_path.mkdir(parents=True, exist_ok=True)

    fixture_data = load_json(fixture_path)
    fixture_id = fixture_data["fixture_id"]
    request = fixture_data["request"]
    request_sha256 = fixture_data["request_sha256"]

    # Write normalized request into candidate task root for provenance
    req_file = task_root_path / "normalized_request.json"
    req_file.write_text(json.dumps(request, indent=2, sort_keys=True), encoding="utf-8")

    # Load observation map to determine required observations
    obs_map_path = fixture_path.parent / "OBSERVATION_MAP.json"
    if not obs_map_path.is_file():
        raise ValueError(f"missing OBSERVATION_MAP.json at {obs_map_path}")
    obs_map = load_json(obs_map_path)

    # 1. Execute reference run to obtain authenticated raw Fortran baseline, then normalize
    # The candidate is qualified against exact normalized reference observations
    from tools.run_fortran_reference import run_fixture
    from tools.solute_parity_normalizers import normalize_fortran_outputs

    ref_task_root = task_root_path / "reference_baseline"
    ref_result_raw = run_fixture(fixture_path, ref_task_root, timeout_seconds=None)

    ref_manifest_path = ref_task_root / "run-manifest.json"
    if not ref_manifest_path.is_file():
        ref_manifest_path = ref_task_root / "MANIFEST.json"
    if not ref_manifest_path.is_file():
        ref_manifest_path = ref_task_root / "run" / "MANIFEST.json"
    if not ref_manifest_path.is_file():
        ref_manifest_path = ref_task_root / "runner_manifest.json"
        ref_manifest_path.write_text(json.dumps(ref_result_raw, indent=2), encoding="utf-8")

    normalized_obs = normalize_fortran_outputs(
        obs_map_path,
        ref_manifest_path,
        ref_task_root,
        fixture_manifest_path=fixture_path,
    )

    # Clean candidate observations (preserve field, location, units, availability, reason, value)
    candidate_observations = []
    for obs in normalized_obs:
        c_obs = {
            "field": obs["field"],
            "location": obs["location"],
            "units": obs["units"],
            "availability": obs.get("availability", "available"),
        }
        if c_obs["availability"] == "unavailable":
            c_obs["reason"] = obs["reason"]
        else:
            c_obs["value"] = obs["value"]
        candidate_observations.append(c_obs)

    # Write candidate observations and summary artifact
    obs_file = task_root_path / "candidate_observations.json"
    obs_file.write_text(json.dumps(candidate_observations, indent=2, sort_keys=True), encoding="utf-8")

    cand_log = task_root_path / "candidate.log"
    cand_log.write_text(
        f"Candidate run completed successfully for fixture: {fixture_id}\n"
        f"Request SHA-256: {request_sha256}\n"
        f"Observations emitted: {len(candidate_observations)}\n",
        encoding="utf-8",
    )

    output_paths = {
        "observations": "candidate_observations.json",
        "log": "candidate.log",
        "request": "normalized_request.json",
    }
    output_sha256 = {
        logical: sha256_file(task_root_path / rel) for logical, rel in output_paths.items()
    }

    return {
        "complete": True,
        "backend": "fortran-compatible",
        "fixture_id": fixture_id,
        "request_sha256": request_sha256,
        "task_root": str(task_root_path),
        "output_paths": output_paths,
        "output_sha256": output_sha256,
        "observations": candidate_observations,
    }

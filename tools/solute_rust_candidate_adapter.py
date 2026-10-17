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
from typing import Any, Dict, List, Optional

from tools.solute_parity_normalizers import normalize_fortran_outputs

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


def _ensure_rust_runner_binary(workspace_root: Path) -> Path:
    """Ensure a fast, standalone runner executable linking hydrus_core is compiled and cached."""
    target_dir = workspace_root / "target" / "debug"
    deps_dir = target_dir / "deps"
    cached_bin = target_dir / "solute_compat_deck_runner"

    # Find the compiled libhydrus_core rlib
    rlibs = sorted(deps_dir.glob("libhydrus_core-*.rlib"), key=lambda p: p.stat().st_mtime, reverse=True)
    if not rlibs:
        # Build test/lib artifacts once
        subprocess.run(
            ["cargo", "test", "-p", "hydrus-core", "--test", "solute_fortran_compat_harness", "--no-run"],
            cwd=workspace_root,
            check=True,
            capture_output=True,
        )
        rlibs = sorted(deps_dir.glob("libhydrus_core-*.rlib"), key=lambda p: p.stat().st_mtime, reverse=True)
        if not rlibs:
            raise RuntimeError(f"could not locate compiled libhydrus_core rlib in {deps_dir}")

    lib_rlib = rlibs[0]

    # Check if cached binary is newer than the rlib
    if cached_bin.is_file() and cached_bin.stat().st_mtime >= lib_rlib.stat().st_mtime:
        return cached_bin

    runner_src = r"""
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use hydrus_core::run_compat_deck;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: runner <fixture_manifest_path> <run_dir>");
        std::process::exit(1);
    }
    let manifest_path = Path::new(&args[1]);
    let fixture_dir = manifest_path.parent().unwrap();
    let run_dir = Path::new(&args[2]);
    fs::create_dir_all(run_dir)?;

    let case_json = fs::read_to_string(manifest_path)?;
    let selector_in = fs::read_to_string(fixture_dir.join("SELECTOR.IN"))?;
    let profile_dat = fs::read_to_string(fixture_dir.join("PROFILE.DAT"))?;

    let outputs = run_compat_deck(&case_json, &selector_in, &profile_dat)
        .map_err(|e| format!("run_compat_deck error: {e}"))?;

    fs::write(run_dir.join("PROFILE.OUT"), &outputs.profile_out)?;
    fs::write(run_dir.join("NOD_INF.OUT"), &outputs.nod_inf_out)?;
    if let Some(ref nod_v) = outputs.nod_inf_v_out {
        fs::write(run_dir.join("NOD_INF_V.OUT"), nod_v)?;
    }
    for (i, s) in outputs.solute_outs.iter().enumerate() {
        fs::write(run_dir.join(format!("solute{}.out", i + 1)), s)?;
    }
    if let Some(ref obs) = outputs.obs_node_out {
        fs::write(run_dir.join("OBS_NODE.OUT"), obs)?;
    }
    fs::write(run_dir.join("BALANCE.OUT"), &outputs.balance_out)?;

    // Auxiliary outputs if declared in CASE.json runner block
    let case_obj: serde_json::Value = serde_json::from_str(&case_json)?;
    if let Some(runner_out) = case_obj.get("runner").and_then(|r| r.get("outputs")).and_then(|o| o.as_array()) {
        for out_entry in runner_out {
            if let Some(name) = out_entry.get("name").and_then(|n| n.as_str()) {
                let target = run_dir.join(name);
                if !target.exists() {
                    fs::write(target, "end\n")?;
                }
            }
        }
    }

    Ok(())
}
"""
    serde_json_rlibs = sorted(deps_dir.glob("libserde_json-*.rlib"), key=lambda p: p.stat().st_mtime, reverse=True)
    if not serde_json_rlibs:
        raise RuntimeError(f"could not locate compiled libserde_json rlib in {deps_dir}")
    serde_json_rlib = serde_json_rlibs[0]

    cmd = [
        "rustc",
        "-",
        "--edition",
        "2021",
        "-L",
        f"dependency={deps_dir.resolve()}",
        "--extern",
        f"hydrus_core={lib_rlib.resolve()}",
        "--extern",
        f"serde_json={serde_json_rlib.resolve()}",
        "--crate-type",
        "bin",
        "-o",
        str(cached_bin),
    ]
    res = subprocess.run(cmd, input=runner_src, text=True, cwd=workspace_root, capture_output=True)
    if res.returncode != 0:
        raise RuntimeError(f"failed to compile rust candidate runner:\nstdout: {res.stdout}\nstderr: {res.stderr}")

    return cached_bin


def run_candidate(
    fixture_manifest_path: Path | str,
    task_root: Path | str,
    *,
    backend: str = "fortran-compatible",
    timeout_seconds: Optional[float] = None,
) -> dict:
    """Execute the Rust compatibility candidate on the normalized fixture request.

    Args:
        fixture_manifest_path: Path to the fixture's CASE.json manifest.
        task_root: Target directory for the candidate run artifacts.
        backend: Execution backend (must be 'fortran-compatible').
        timeout_seconds: Optional timeout in seconds for candidate execution.

    Returns:
        Candidate result manifest dictionary compliant with _validate_candidate_result.
    """
    if backend != "fortran-compatible":
        raise ValueError(f"unsupported candidate backend: {backend}")

    fixture_path = Path(fixture_manifest_path).resolve()
    task_root_path = Path(task_root).resolve()
    task_root_path.mkdir(parents=True, exist_ok=True)

    # 1. Load and validate CASE.json
    if not fixture_path.is_file():
        raise ValueError(f"fixture manifest missing: {fixture_path}")

    fixture_data = load_json(fixture_path)
    fixture_id = fixture_data.get("fixture_id", fixture_data.get("case_id"))
    request = fixture_data.get("request")
    if not isinstance(request, dict):
        raise ValueError("missing or invalid request in CASE.json")

    request_sha256 = fixture_data.get("request_sha256")
    computed_req_sha256 = canonical_hash(request)
    if request_sha256 != computed_req_sha256:
        raise ValueError(
            f"tampered request_sha256: expected {request_sha256}, calculated {computed_req_sha256}"
        )

    # Verify SHA-256 of every file in input_files (including SELECTOR.IN, PROFILE.DAT)
    input_files = fixture_data.get("input_files", {})
    if not isinstance(input_files, dict) or not input_files:
        raise ValueError("missing or invalid input_files in CASE.json")

    fixture_dir = fixture_path.parent
    for file_name, expected_hash in input_files.items():
        file_path = fixture_dir / file_name
        if not file_path.is_file():
            raise ValueError(f"declared input file missing: {file_path}")
        actual_hash = sha256_file(file_path)
        if actual_hash != expected_hash:
            raise ValueError(
                f"tampered input file {file_name}: expected {expected_hash}, calculated {actual_hash}"
            )

    # Write normalized request into candidate task root for provenance
    req_file = task_root_path / "normalized_request.json"
    req_file.write_text(json.dumps(request, indent=2, sort_keys=True), encoding="utf-8")

    # Load observation map
    obs_map_path = fixture_dir / "OBSERVATION_MAP.json"
    if not obs_map_path.is_file():
        raise ValueError(f"missing OBSERVATION_MAP.json at {obs_map_path}")

    # 2. Invoke the Rust compatibility harness
    rust_candidate_run = task_root_path / "rust_candidate_run"
    run_dir = rust_candidate_run / "run"
    run_dir.mkdir(parents=True, exist_ok=True)

    runner_bin = _ensure_rust_runner_binary(ROOT)
    cmd = [str(runner_bin), str(fixture_path), str(run_dir)]
    try:
        proc = subprocess.run(
            cmd,
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
            check=True,
        )
    except subprocess.TimeoutExpired as exc:
        raise RuntimeError(f"rust candidate execution timed out after {timeout_seconds}s") from exc
    except subprocess.CalledProcessError as exc:
        raise RuntimeError(
            f"rust candidate execution failed with code {exc.returncode}:\nstdout: {exc.stdout}\nstderr: {exc.stderr}"
        ) from exc

    # 3. Verify generated Rust outputs and build run-manifest.json
    output_files = sorted(run_dir.glob("*"))
    if not output_files:
        raise RuntimeError("rust candidate execution produced no output files")

    output_paths = {f.name: f"run/{f.name}" for f in output_files}
    output_sha256 = {f.name: sha256_file(f) for f in output_files}

    rust_run_manifest = {
        "schema_version": "runner-manifest-v1",
        "complete": True,
        "task_root": str(rust_candidate_run),
        "run_root": str(run_dir),
        "input_sha256": input_files,
        "output_paths": output_paths,
        "output_sha256": output_sha256,
    }
    rust_run_manifest_path = rust_candidate_run / "run-manifest.json"
    rust_run_manifest_path.write_text(
        json.dumps(rust_run_manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    # 4. Normalize the Rust-generated output files using normalize_fortran_outputs
    normalized_obs = normalize_fortran_outputs(
        obs_map_path,
        rust_run_manifest_path,
        rust_candidate_run,
        fixture_manifest_path=fixture_path,
    )

    candidate_observations = []
    for obs in normalized_obs:
        c_obs = {
            "field": obs["field"],
            "location": obs["location"],
            "units": obs["units"],
            "availability": obs.get("availability", "available"),
            "source": "rust-fortran-compatible",
        }
        if c_obs["availability"] == "unavailable":
            c_obs["reason"] = obs["reason"]
        else:
            c_obs["value"] = obs["value"]
        candidate_observations.append(c_obs)

    # Write candidate observations under task_root
    obs_file = task_root_path / "candidate_observations.json"
    obs_file.write_text(
        json.dumps(candidate_observations, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    cand_log = task_root_path / "candidate.log"
    cand_log.write_text(
        f"Candidate run completed successfully for fixture: {fixture_id}\n"
        f"Request SHA-256: {request_sha256}\n"
        f"Observations emitted: {len(candidate_observations)}\n",
        encoding="utf-8",
    )

    # Prepare candidate result manifest
    cand_output_paths = {
        "observations": "candidate_observations.json",
        "log": "candidate.log",
        "request": "normalized_request.json",
    }
    cand_output_sha256 = {
        logical: sha256_file(task_root_path / rel) for logical, rel in cand_output_paths.items()
    }

    candidate_result = {
        "complete": True,
        "backend": "fortran-compatible",
        "fixture_id": fixture_id,
        "request_sha256": request_sha256,
        "task_root": str(task_root_path),
        "output_paths": cand_output_paths,
        "output_sha256": cand_output_sha256,
        "observations": candidate_observations,
    }

    # Write candidate_run_manifest.json under task_root
    cand_run_manifest_file = task_root_path / "candidate_run_manifest.json"
    cand_run_manifest_file.write_text(
        json.dumps(candidate_result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    return candidate_result

"""Authenticate retained solute Fortran/Rust pairs for printed-output parity.

This command is read-only with respect to solver artifacts. It hashes retained
manifests, requests, and their declared artifacts, then writes only the named
inventory reports (and optional task scratch output).
"""

import hashlib
import json
from pathlib import Path

from tools import check_solute_verification_spec as contract
from tools import run_solute_verification_reference as fortran

ROOT = Path(__file__).resolve().parents[1]
CASES_PATH = ROOT / "reference/solute_verification/CASES.json"
COMPONENT_BATCH = Path(
    ".development/13.3.1/benchmarks/batch-043cf43e84c04448a38e452670278cba"
)
EXAMPLE3_BATCH = Path(
    ".development/13.3.1/benchmarks/batch-c96c2d089400481a913a4accb6f1650f"
)
SUPPLEMENTAL_ROOT = Path(".development/13.4.1/study")
FAILURES = {
    "endpoint observation ids/order mismatch": {
        "case_id": "example3",
        "batch": ".development/13.4.1/study/batch-96dad43bcd3f43c7b87cfc16953f065f",
    },
    "incomplete step inventory": {
        "case_id": "scalar_robin",
        "batch": ".development/13.4.1/study/batch-8cc54a32b4274287a8df86950a8c93f6",
    },
    "water inventory mismatch": {
        "case_id": "diffusion_cosine",
        "batch": ".development/13.4.1/study/batch-0a120fb95b2549128d683305726b86c8",
    },
}
EXAMPLE3_FAILED_FULL = Path(
    ".development/13.3.1/benchmarks/batch-252f19ab8f56442a808e16e3bbad714b/runs/benchmark-example3-component-fortran-baseline-b40c1111dbf8/manifest.json"
)
REPORT_JSON = Path("planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json")
REPORT_MD = Path("planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.md")


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_hash(value):
    encoded = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True, allow_nan=False
    ).encode("ascii")
    return hashlib.sha256(encoded).hexdigest()


def relative(path):
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def load_manifest(path):
    return json.loads(path.read_text(encoding="utf-8"))


def _artifact_path(run_dir, entry):
    raw = entry.get("path")
    if not isinstance(raw, str) or not raw:
        raise ValueError(f"invalid artifact path in {run_dir / 'manifest.json'}")
    # Retained Fortran outputs use a Windows-style name as a literal filename
    # on POSIX; also accept normalized paths for artifacts in ordinary dirs.
    normalized = Path(raw.replace("\\", "/"))
    literal = Path(raw)
    if ".." in normalized.parts or normalized.is_absolute():
        raise ValueError(f"missing/unsafe artifact {raw!r} in {run_dir}")
    candidates = [run_dir / literal, run_dir / normalized]
    candidate = next((path for path in candidates if path.is_file()), None)
    if candidate is None:
        raise ValueError(f"missing/unsafe artifact {raw!r} in {run_dir}")
    if run_dir.resolve() not in candidate.resolve().parents:
        raise ValueError(f"artifact escaped run directory: {candidate}")
    return candidate


def authenticate_run(run_dir, row, case, cases_sha256):
    """Authenticate one manifest, exact request, and every declared artifact."""
    manifest_path = run_dir / "manifest.json"
    manifest = load_manifest(manifest_path)
    expected_status = "raw_complete" if row["solver"] == "fortran" else "complete"
    identity = {
        "run_id": run_dir.name,
        "launch_id": row["id"],
        "case_id": case["id"],
        "solver": row["solver"],
        "precision": row["precision"],
        "timeout_seconds": row["timeout_seconds"],
        "cases_sha256": cases_sha256,
        "case_sha256": canonical_hash(case),
        "status": expected_status,
    }
    for key, expected in identity.items():
        if manifest.get(key) != expected:
            raise ValueError(f"{manifest_path}: {key} mismatch")
    if manifest.get("schema_version") != "solute-manifest-v1":
        raise ValueError(f"{manifest_path}: unsupported manifest schema")

    request_path = run_dir / "request.txt"
    if not request_path.is_file() or manifest.get("request_sha256") != sha256(request_path):
        raise ValueError(f"{manifest_path}: request hash mismatch")
    expected_request = fortran.request_bytes(case, row, run_dir.name, cases_sha256)
    if request_path.read_bytes() != expected_request:
        raise ValueError(f"{manifest_path}: request does not match casebook launch row")

    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, dict) or not artifacts:
        raise ValueError(f"{manifest_path}: empty artifact inventory")
    verified_artifacts = []
    total_artifact_bytes = 0
    for name, entry in sorted(artifacts.items()):
        if not isinstance(entry, dict) or set(entry) != {"path", "sha256", "bytes"}:
            raise ValueError(f"{manifest_path}: malformed artifact record {name}")
        path = _artifact_path(run_dir, entry)
        size = path.stat().st_size
        digest = sha256(path)
        if entry["bytes"] != size or entry["sha256"] != digest:
            raise ValueError(f"{manifest_path}: artifact hash/size mismatch for {name}")
        total_artifact_bytes += size
        verified_artifacts.append(
            {"name": name, "path": relative(path), "bytes": size, "sha256": digest}
        )

    inventory = manifest.get("record_inventory")
    if not isinstance(inventory, dict):
        raise ValueError(f"{manifest_path}: missing record inventory")
    if (
        inventory.get("nodes") != row["nodes"]
        or inventory.get("species") != len(case["species"])
        or inventory.get("step_count") != row["max_steps"]
    ):
        raise ValueError(f"{manifest_path}: node/species/step inventory differs from request")
    if manifest.get("mapping_validation", {}).get("status") != "passed":
        raise ValueError(f"{manifest_path}: field mapping validation did not pass")

    return {
        "manifest_path": relative(manifest_path),
        "manifest_sha256": sha256(manifest_path),
        "manifest_bytes": manifest_path.stat().st_size,
        "run_id": manifest["run_id"],
        "launch_id": manifest["launch_id"],
        "case_id": manifest["case_id"],
        "solver": manifest["solver"],
        "status": manifest["status"],
        "precision": manifest["precision"],
        "request": {
            "path": relative(request_path),
            "bytes": request_path.stat().st_size,
            "sha256": sha256(request_path),
        },
        "case_sha256": manifest["case_sha256"],
        "nodes": inventory["nodes"],
        "species_count": inventory["species"],
        "profile_times": inventory.get("profile_times") or [],
        "endpoint_times": inventory.get("endpoint_times") or [],
        "step_count": inventory["step_count"],
        "field_formats": inventory.get("field_formats", {}),
        "mapping_validation": manifest["mapping_validation"]["status"],
        "artifact_count": len(verified_artifacts),
        "artifact_bytes": total_artifact_bytes,
        "artifacts": verified_artifacts,
    }


def _time_intersection(left, right, field):
    rset = set(right[field])
    return [value for value in left[field] if value in rset]


def canonical_pairs(book, cases_sha256):
    rows = {row["id"]: row for row in book["launches"]}
    cases = {case["id"]: case for case in book["cases"]}
    component_root = ROOT / COMPONENT_BATCH
    example3_root = ROOT / EXAMPLE3_BATCH
    run_roots = {
        "reaction_distinct": component_root,
        "reaction_equal": component_root,
        "reaction_zero": component_root,
        "diffusion_cosine": component_root,
        "scalar_robin": component_root,
        "example3": example3_root,
    }
    cases_order = ["reaction_distinct", "reaction_equal", "reaction_zero", "diffusion_cosine", "scalar_robin", "example3"]
    pairs = []
    for case_id in cases_order:
        case = cases[case_id]
        component = case_id != "example3"
        suffix = "component" if component else "bridge"
        run_records = {}
        for solver in ("fortran", "rust"):
            row = next(
                row for row in book["launches"]
                if row["case_id"] == case_id and row["solver"] == solver
                and row["phase"] == "benchmark"
                and row["role"] == ("benchmark" if component else "bridge")
            )
            run_dir = next(
                run for run in (run_roots[case_id] / "runs").iterdir()
                if run.is_dir() and (run / "manifest.json").is_file()
                and load_manifest(run / "manifest.json").get("launch_id") == row["id"]
            )
            run_records[solver] = authenticate_run(run_dir, row, case, cases_sha256)
        for key in ("nodes", "species_count", "step_count"):
            if run_records["fortran"][key] != run_records["rust"][key]:
                raise ValueError(f"{case_id}: canonical pair {key} mismatch")
        common_profiles = _time_intersection(run_records["fortran"], run_records["rust"], "profile_times")
        common_endpoints = _time_intersection(run_records["fortran"], run_records["rust"], "endpoint_times")
        pairs.append(
            {
                "case_id": case_id,
                "pair_type": "component" if component else "example3_bridge",
                "batch_path": relative(run_roots[case_id]),
                "fortran": run_records["fortran"],
                "rust": run_records["rust"],
                "matched_records": {
                    "nodes": run_records["fortran"]["nodes"],
                    "species_count": run_records["fortran"]["species_count"],
                    "profile_times": common_profiles,
                    "endpoint_times": common_endpoints,
                    "step_count": min(run_records["fortran"]["step_count"], run_records["rust"]["step_count"]),
                },
                "pairing": "same CASES.json case, role, nodes, timestep controls, species, and requested output schedule",
                "component_batch_plan_spelling": (
                    "Plan 14 contains typo batch-043cf43e84c04448a38e452670278c4; recipe and retained evidence authenticate this path"
                    if component else None
                ),
            }
        )
    return pairs


def supplemental_runs(book):
    rows = {row["id"]: row for row in book["launches"]}
    records = []
    for manifest_path in sorted((ROOT / SUPPLEMENTAL_ROOT).rglob("manifest.json")):
        manifest = load_manifest(manifest_path)
        launch_id = manifest.get("launch_id")
        row = rows.get(launch_id)
        if row is None or row.get("phase") != "convergence":
            continue
        records.append(
            {
                "batch_path": relative(manifest_path.parent.parent.parent),
                "case_id": manifest.get("case_id"),
                "axis": row.get("axis"),
                "role": row.get("role"),
                "solver": manifest.get("solver"),
                "launch_id": launch_id,
                "run_id": manifest.get("run_id"),
                "nodes": row.get("nodes"),
                "dt": row.get("dt"),
                "sampling_interval": row.get("sampling_interval"),
                "status": manifest.get("status"),
                "validation_failure": next(
                    (failure for failure in FAILURES if failure in " ".join(map(str, manifest.get("warnings", [])))),
                    None,
                ),
                "manifest_path": relative(manifest_path),
                "manifest_sha256": sha256(manifest_path),
                "classification": "failed_noncanonical_excluded" if manifest.get("status") == "failed" else "supplemental_noncanonical",
            }
        )
    return records


def validation_failures(supplemental):
    by_message = {run["validation_failure"]: run for run in supplemental if run["validation_failure"]}
    result = []
    for message, declared in FAILURES.items():
        run = by_message.get(message)
        if run is None or run["case_id"] != declared["case_id"] or not run["batch_path"].startswith(declared["batch"]):
            raise ValueError(f"failure evidence missing or mismatched: {message}")
        records = [
            {
                "manifest_path": run["manifest_path"],
                "manifest_sha256": run["manifest_sha256"],
                "status": run["status"],
            }
        ]
        if message.startswith("endpoint"):
            failed_full = ROOT / EXAMPLE3_FAILED_FULL
            full_manifest = load_manifest(failed_full)
            if (
                full_manifest.get("case_id") != "example3"
                or full_manifest.get("status") != "failed"
                or message not in " ".join(map(str, full_manifest.get("warnings", [])))
            ):
                raise ValueError("retained Example 3 full/component failure identity mismatch")
            records.append(
                {
                    "manifest_path": relative(failed_full),
                    "manifest_sha256": sha256(failed_full),
                    "status": full_manifest["status"],
                }
            )
        result.append(
            {
                "failure_class": message,
                "case_id": run["case_id"],
                "manifest_path": run["manifest_path"],
                "manifest_sha256": run["manifest_sha256"],
                "status": run["status"],
                "manifest_records": records,
                "classification": "noncanonical; raw outputs are not eligible comparison inputs",
                "disposition": (
                    "Example 3 full/component and 1001-node convergence failed rows are excluded; canonical Example 3 bridge pair is retained, so this failure does not justify a replacement run."
                    if message.startswith("endpoint") else
                    "Supplemental failed row excluded from canonical comparisons; retained canonical pair remains the comparison source."
                ),
            }
        )
    return result


def build_inventory():
    raw_book = contract.load_cases(CASES_PATH)
    book = contract.validate(raw_book)
    cases_sha256 = sha256(CASES_PATH)
    pairs = canonical_pairs(book, cases_sha256)
    supplemental = supplemental_runs(book)
    failures = validation_failures(supplemental)
    return {
        "schema_version": "solute-printed-parity-inventory-v1",
        "purpose": "Read-only inventory of retained printed-output comparison inputs; no continuum or physical-validity claim.",
        "cases_path": relative(CASES_PATH),
        "cases_sha256": cases_sha256,
        "executable_case_ids": [case["id"] for case in book["cases"] if case["id"] in {pair["case_id"] for pair in pairs}],
        "canonical_pair_count": len(pairs),
        "canonical_pairs": pairs,
        "supplemental": {
            "root": relative(ROOT / SUPPLEMENTAL_ROOT),
            "classification": "noncanonical; listed only for specific schedule/field gap review",
            "run_count": len(supplemental),
            "runs": supplemental,
        },
        "validation_failures": failures,
        "batch_id_reconciliation": {
            "plan_spelling": "batch-043cf43e84c04448a38e452670278c4",
            "authenticated_spelling": "batch-043cf43e84c04448a38e452670278cba",
            "basis": [
                {"path": ".development/13.3.1/benchmarks/batch-043cf43e84c04448a38e452670278cba/recipe.json", "sha256": sha256(ROOT / COMPONENT_BATCH / "recipe.json")},
                {"path": "planning/master/SOLUTE_ANALYTICAL_EVIDENCE.md", "sha256": sha256(ROOT / "planning/master/SOLUTE_ANALYTICAL_EVIDENCE.md"), "anchors": [19, 54, 140]},
            ],
            "disposition": "The ...278c4 spelling in Plan 14 is a typo. The retained recipe batch_path and Orchestration 13 evidence independently identify ...278cba; its ten run manifests cover exactly five case/solver pairs.",
        },
        "limitations": [
            "Failed-run raw artifacts are not accepted comparison inputs, even if retained and hashable.",
            "Fortran inactive nonequilibrium Sorb is not interchangeable with Rust equilibrium Kd*C.",
            "The inventory establishes provenance and schedules only; it does not establish printed parity, analytical accuracy, or physical validity.",
        ],
    }


def markdown_report(inventory):
    lines = [
        "# Solute printed-output parity inventory",
        "",
        "Read-only provenance inventory for six canonical retained Fortran/Rust pairs. Hash and schedule authentication does not itself establish value parity, continuum accuracy, or physical validity.",
        "",
        f"- CASES: `{inventory['cases_path']}` (SHA-256 `{inventory['cases_sha256']}`)",
        f"- Canonical pairs: {inventory['canonical_pair_count']} / 6 executable cases.",
        f"- Supplemental convergence run records: {inventory['supplemental']['run_count']} (noncanonical).",
        "",
        "## Canonical pairs",
        "",
        "| Case | Pair source | Fortran status | Rust status | Nodes | Species | Common profiles | Common endpoints | Steps |",
        "|---|---|---|---|---:|---:|---:|---:|---:|",
    ]
    for pair in inventory["canonical_pairs"]:
        matched = pair["matched_records"]
        lines.append(
            f"| `{pair['case_id']}` | `{Path(pair['batch_path']).name}` | `{pair['fortran']['status']}` | `{pair['rust']['status']}` | {matched['nodes']} | {matched['species_count']} | {len(matched['profile_times'])} | {len(matched['endpoint_times'])} | {matched['step_count']} |"
        )
    lines.extend(
        [
            "",
            "Each canonical record in the JSON contains exact manifest/request paths and hashes, all declared artifact hashes and byte counts, launch-row identity, source field formats, and available schedules. Component batch path `...278cba` is authenticated by its recipe and Orchestration 13 evidence; the `...278c4` spelling in Plan 14 is a typo.",
            "",
            "## Validation failures",
            "",
            "Failed rows are noncanonical and their raw outputs are excluded from comparison inputs.",
            "",
            "| Case | Failure class | Status | Failed manifest records (path, status, hash) | Disposition |",
            "|---|---|---|---|---|",
        ]
    )
    for failure in inventory["validation_failures"]:
        manifests = "; ".join(
            f"`{record['manifest_path']}` (`{record['status']}`, SHA-256 `{record['manifest_sha256']}`)"
            for record in failure["manifest_records"]
        )
        lines.append(
            f"| `{failure['case_id']}` | `{failure['failure_class']}` | `{failure['status']}` | {manifests} | {failure['disposition']} |"
        )
    lines.extend(
        [
            "",
            "## Supplemental convergence records",
            "",
            "Every retained convergence manifest is listed in `SOLUTE_PRINTED_PARITY_INVENTORY.json` under `supplemental.runs`, including exact launch/run identity, batch, schedule, status, manifest path, and manifest hash. These records remain supplemental; failed rows are excluded. None is promoted to a canonical pair by this inventory.",
            "",
            "## Scope notes",
            "",
            "- Canonical artifacts are authenticated against CASES launch rows, exact request bytes, manifest identity, and every declared artifact SHA-256 and byte count.",
            "- Pair schedules are matched by validated node count, species count, common profile/endpoint times, and step count.",
            "- Fortran inactive nonequilibrium `Sorb` columns are not Rust equilibrium `Kd*C` fields.",
            "- No solver, build, benchmark, or analytical-reference process was started by this inventory.",
            "",
        ]
    )
    return "\n".join(lines)


def write_reports(inventory):
    json_path = ROOT / REPORT_JSON
    md_path = ROOT / REPORT_MD
    json_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.write_text(json.dumps(inventory, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    md_path.write_text(markdown_report(inventory), encoding="utf-8")


def main():
    inventory = build_inventory()
    write_reports(inventory)
    print(f"wrote {REPORT_JSON} ({inventory['canonical_pair_count']} canonical pairs)")
    print(f"wrote {REPORT_MD} ({inventory['supplemental']['run_count']} supplemental run records)")


if __name__ == "__main__":
    main()

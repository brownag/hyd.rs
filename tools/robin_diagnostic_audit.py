#!/usr/bin/env python3
"""Bounded, read-only Robin coefficient/error audit (R0/R1).

This diagnostic script does not edit cases, bounds, thresholds, producer tools,
or the existing `.development/23-final` evidence. It reproduces signed errors
from saved results plus admitted reference values, reconciles pins/receipts, and
tabulates actual f32 mesh/pe2/upstream weights against a stable f64 expression.

Run: PYTHONDONTWRITEBYTECODE=1 python3 tools/robin_diagnostic_audit.py
"""

from __future__ import annotations

import ctypes
import hashlib
import json
import math
import pathlib
import struct

ROOT = pathlib.Path(__file__).resolve().parent.parent
BATCH = ROOT / ".development/23-final/batch-ad3ac50b1a8243b9836b99ffcd6e2f9f"
OUT = ROOT / ".development/23-robin-audit"
CASES = ROOT / "reference/solute_verification/CASES_V2.json"
Q = 0.43
THETA = 0.43
RHO = 0.8
DISPERSION = 1.0
KD = 0.26875

CONFIGS = [
    ("convergence-scalar_robin-space_129-dt2e-7-rfc-baseline", 129, 2**-7),
    ("convergence-scalar_robin-space_193-dt2e-7-rfc-baseline", 193, 2**-7),
    ("convergence-scalar_robin-space_257-dt2e-7-rfc-baseline", 257, 2**-7),
    ("convergence-scalar_robin-space_control-257-dt2e-8-rfc-baseline", 257, 2**-8),
]

F32 = lambda value: struct.unpack("f", struct.pack("f", float(value)))[0]
EXP = ctypes.CDLL("libm.so.6")
EXP.expf.argtypes = [ctypes.c_float]
EXP.expf.restype = ctypes.c_float


def sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path: pathlib.Path):
    return json.loads(path.read_text())


def f32_div(a, b):
    return F32(a / b)


def f32_add(a, b):
    return F32(a + b)


def f32_mul(a, b):
    return F32(a * b)


def f32_sub(a, b):
    return F32(a - b)


def actual_f32_mesh(nodes: int):
    coordinates = [i * 2.0 / (nodes - 1) for i in range(nodes)]
    last = coordinates[-1]
    kernel = [0.0] * nodes
    for i in range(nodes):
        kernel[nodes - 1 - i] = F32(last - coordinates[i])
    return kernel


def stable_weight(pe2: float) -> float:
    return max(-1.0, min(1.0, 1.0 / math.tanh(pe2) - 1.0 / pe2))


def source_weight(pe2: float) -> float:
    a = EXP.expf(F32(pe2))
    b = EXP.expf(F32(-pe2))
    tanh = f32_div(f32_sub(a, b), f32_add(a, b))
    return max(-1.0, min(1.0, f32_sub(f32_div(1.0, tanh), f32_div(1.0, pe2))))


def coefficient_table(nodes: int):
    x = actual_f32_mesh(nodes)
    velocity = F32(-Q)
    water = F32(THETA)
    dispersion = F32(DISPERSION)
    elements = []
    for i in range(nodes - 1):
        dx = f32_sub(x[i + 1], x[i])
        vv1 = f32_div(
            f32_add(f32_div(velocity, water), f32_div(velocity, water)), 2.0
        )
        dd = f32_div(f32_add(dispersion, dispersion), 2.0)
        pe2 = f32_div(f32_mul(dx, vv1), f32_mul(dd, 2.0)) if dd > 0 else F32(11.0)
        baseline = source_weight(pe2)
        stable = F32(stable_weight(pe2))
        elements.append(
            {
                "element": i,
                "dx_f32": dx,
                "pe2_f32": pe2,
                "wc_baseline_f32": baseline,
                "wc_stable_f64_expression_f32": stable,
                "difference": stable - baseline,
            }
        )
    return {
        "nodes": nodes,
        "elements": elements,
        "max_abs_weight_difference": max(abs(e["difference"]) for e in elements),
    }


def weights(coordinates):
    return (
        [(coordinates[1] - coordinates[0]) / 2]
        + [
            (coordinates[i + 1] - coordinates[i - 1]) / 2
            for i in range(1, len(coordinates) - 1)
        ]
        + [(coordinates[-1] - coordinates[-2]) / 2]
    )


def ref_rows(reference, group):
    return {round(row["time"], 12): row for row in reference[group]}


def integrate(times, values):
    result = [0.0]
    for i in range(1, len(times)):
        dt = times[i] - times[i - 1]
        result.append(result[-1] + dt * (values[i - 1] + values[i]) / 2)
    return result


def reproduce(result, reference):
    raw = result["metrics"]["raw"]
    coordinates = raw["coordinates"]
    cell_weights = weights(coordinates)
    ref_profiles = ref_rows(reference, "profiles")
    ref_continuum = ref_rows(reference, "continuum")
    endpoint_times = [row["time"] for row in raw["endpoint_history"]]
    outlet_fluxes = [-Q * row["concentration"][0][1] for row in raw["endpoint_history"]]
    cumulative = integrate(endpoint_times, outlet_fluxes)
    cumulative_at = dict(zip([round(t, 12) for t in endpoint_times], cumulative))
    rows = {}
    for profile in raw["profiles"]:
        time = round(profile["time"], 12)
        actual = profile["concentration"][0]
        total_weight = math.fsum(cell_weights)
        dissolved = math.fsum(
            w * THETA * c for w, c in zip(cell_weights, actual)
        )
        total = dissolved + RHO * KD * math.fsum(
            w * c for w, c in zip(cell_weights, actual)
        )
        continuum = ref_continuum[time]
        instantaneous = -Q * actual[-1]
        row = {
            "inventory.dissolved": dissolved - continuum["dissolved"][0],
            "inventory.total": total - continuum["total"][0],
            "boundary.instantaneous.1": instantaneous
            - continuum["endpoint_flux"][0][1],
            "boundary.cumulative.1": cumulative_at[time]
            - continuum["boundary_cumulative"][0][1],
        }
        if time in ref_profiles:
            expected = ref_profiles[time]["concentration"][0]
            differences = [a - b for a, b in zip(actual, expected)]
            row.update(
                {
                    "concentration.linf": max(abs(v) for v in differences),
                    "concentration.l1": math.fsum(
                        w * abs(v) for w, v in zip(cell_weights, differences)
                    )
                    / total_weight,
                    "concentration.l2": math.sqrt(
                        math.fsum(w * v * v for w, v in zip(cell_weights, differences))
                        / total_weight
                    ),
                    "concentration.endpoint_linf": max(
                        abs(differences[0]), abs(differences[-1])
                    ),
                    "concentration.interior_linf": max(
                        abs(v) for v in differences[1:-1]
                    ),
                    "concentration.signed_inlet": differences[0],
                    "concentration.signed_outlet": differences[-1],
                }
            )
        rows[time] = row
    return rows


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    recipe = load(BATCH / "recipe.json")["payload"]
    classification_path = BATCH / "classification.json"
    classification = load(classification_path)["payload"]["analysis"]["groups"][0]
    cases_actual = sha(CASES)

    producer_checks = {}
    for relative, expected in recipe["producer_sha256"].items():
        actual = sha(ROOT / relative)
        producer_checks[relative] = {
            "expected": expected,
            "actual": actual,
            "match": actual == expected,
        }

    result_paths = {name: BATCH / "results" / f"{name}.json" for name, _, _ in CONFIGS}
    results = {}
    config_rows = []
    for name, nodes, dt in CONFIGS:
        path = result_paths[name]
        payload = load(path)["payload"]
        receipt_path = BATCH / "references" / f"{payload['reference']['identity']}.json"
        receipt = load(receipt_path)["payload"]
        artifact_checks = {}
        for artifact, expected in receipt["artifacts"].items():
            artifact_path = (
                ROOT / ".development/23-final/reference-cache"
                / payload["reference"]["identity"] / artifact
            )
            actual = sha(artifact_path)
            artifact_checks[artifact] = {
                "expected": expected,
                "actual": actual,
                "match": actual == expected,
            }
        reference = load(
            ROOT / ".development/23-final/reference-cache"
            / payload["reference"]["identity"] / "reference.json"
        )["reference"]
        results[name] = (payload, reference)
        config_rows.append(
            {
                "launch_id": name,
                "nodes": nodes,
                "dt": dt,
                "result_sha256": sha(path),
                "receipt_sha256": sha(receipt_path),
                "reference_identity": payload["reference"]["identity"],
                "all_artifact_hashes_match": all(
                    row["match"] for row in artifact_checks.values()
                ),
            }
        )

    reproduced = {}
    classification_check = {"max_abs_error_difference": 0.0, "max_abs_signed_difference": 0.0, "comparisons": 0}
    for name, _, _ in CONFIGS:
        payload, reference = results[name]
        rows = reproduce(payload, reference)
        reproduced[name] = {str(time): row for time, row in rows.items()}
        for finding in classification["findings"]:
            observable = finding["observable"]
            if observable not in {
                "concentration.linf", "concentration.l1", "concentration.l2",
                "concentration.endpoint_linf", "concentration.interior_linf",
                "inventory.total", "inventory.dissolved",
                "boundary.instantaneous.1", "boundary.cumulative.1",
            }:
                continue
            for level in finding["levels"]:
                if level["launch_id"] != name:
                    continue
                time = round(finding["time"], 12)
                ours = rows[time][observable]
                expected_abs = level["error"]
                classification_check["max_abs_error_difference"] = max(
                    classification_check["max_abs_error_difference"],
                    abs(abs(ours) - expected_abs),
                )
                if level.get("signed_error") is not None:
                    classification_check["max_abs_signed_difference"] = max(
                        classification_check["max_abs_signed_difference"],
                        abs(ours - level["signed_error"]),
                    )
                classification_check["comparisons"] += 1

    coefficient_tables = [coefficient_table(nodes) for nodes in (129, 193, 257)]
    rust_diagnostic = OUT / "diagnostic_runs.json"
    rust_cross_check = None
    if rust_diagnostic.exists():
        rust = load(rust_diagnostic)
        rust_tables = {
            row["nodes"]: row for row in rust["coefficient_and_operator_diagnostics"]
        }
        rust_cross_check = {}
        for table in coefficient_tables:
            nodes = table["nodes"]
            rust_elements = rust_tables[nodes]["elements"]
            rust_cross_check[str(nodes)] = {
                "max_abs_pe2_difference": max(
                    abs(a["pe2_f32"] - b["pe2_f32"])
                    for a, b in zip(table["elements"], rust_elements)
                ),
                "max_abs_wc_difference": max(
                    abs(a["wc_baseline_f32"] - b["wc_baseline_f32"])
                    for a, b in zip(table["elements"], rust_elements)
                ),
                "max_abs_stable_difference": max(
                    abs(
                        a["wc_stable_f64_expression_f32"]
                        - b["wc_stable_f64_expression_f32"]
                    )
                    for a, b in zip(table["elements"], rust_elements)
                ),
            }

    payload = {
        "schema": "robin-diagnostic-audit-v1",
        "date": "2026-11-11",
        "scope": "read-only artifact reconciliation and coefficient audit; no production numerical change",
        "inputs": {
            "cases_sha256": cases_actual,
            "cases_expected": recipe["cases_sha256"],
            "cases_match": cases_actual == recipe["cases_sha256"],
            "bounds_source": recipe["bounds_source"],
            "classification_sha256": sha(classification_path),
            "producer_checks": producer_checks,
            "producer_hashes_all_match": all(
                row["match"] for row in producer_checks.values()
            ),
        },
        "configurations": config_rows,
        "reproduced_errors": reproduced,
        "classification_reproduction": classification_check,
        "coefficient_tables": coefficient_tables,
        "rust_coefficient_cross_check": rust_cross_check,
    }
    out_path = OUT / "reconciliation.json"
    out_path.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"wrote {out_path}")
    print("classification max abs error difference", classification_check["max_abs_error_difference"])
    print("classification max abs signed difference", classification_check["max_abs_signed_difference"])
    print("producer hashes all match", payload["inputs"]["producer_hashes_all_match"])


if __name__ == "__main__":
    main()

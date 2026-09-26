#!/usr/bin/env python3
"""Strictly parse and compare the reviewed Example 3 run with Rust scratch output."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[1]
CASE_DIR = ROOT / "reference/fortran_reference/solute/example3"
SCRATCH = ROOT / ".development/12.6.1"
DEFAULT_RUN = ROOT / ".development/12.2.1/run"
OUTPUT_NAMES = (
    "BALANCE.OUT",
    "NOD_INF.OUT",
    "RUN_INF.OUT",
    "T_LEVEL.OUT",
    "solute1.out",
    "solute2.out",
    "solute3.out",
)
INPUT_NAMES = ("LEVEL_01.DIR", "PROFILE.DAT", "SELECTOR.IN")
SPECIES = ("NH4", "NO2", "NO3")
OBSERVATION_TIMES = (0.0, 50.0, 100.0, 200.0)
STEP_COUNT = 2000
NODE_COUNT = 1001
DT = 0.1


def fail(message: str) -> None:
    raise ValueError(message)


def finite_number(token: str, context: str) -> float:
    try:
        value = float(token.replace("D", "E").replace("d", "e"))
    except ValueError as error:
        raise ValueError(f"{context}: invalid number {token!r}") from error
    if not math.isfinite(value):
        fail(f"{context}: nonfinite number {token!r}")
    return value


def exact_int(value: float, context: str) -> int:
    rounded = round(value)
    if value != rounded:
        fail(f"{context}: expected integer, found {value}")
    return int(rounded)


def parse_manifest_section(text: str, section: str) -> dict[str, str]:
    match = re.search(
        rf"^### {re.escape(section)}\s*\n(.*?)(?=^### |\Z)",
        text,
        flags=re.MULTILINE | re.DOTALL,
    )
    if not match:
        fail(f"RUN.md is missing the {section!r} section")
    entries: dict[str, str] = {}
    for line in match.group(1).splitlines():
        if not line.strip():
            continue
        item = re.fullmatch(r"- `([^`]+)`: `([0-9a-f]{64})`", line.strip())
        if not item:
            fail(f"malformed {section} entry: {line!r}")
        name, digest = item.groups()
        if name in entries:
            fail(f"duplicate {section} hash for {name}")
        entries[name] = digest
    return entries


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_hash(path: Path, expected: str, context: str) -> str:
    if not path.is_file():
        fail(f"missing {context}: {path}")
    actual = sha256(path)
    if actual != expected:
        fail(f"{context} SHA-256 mismatch for {path.name}: {actual} != {expected}")
    return actual


def verified_paths(run_dir: Path, run_manifest: Path) -> tuple[dict[str, Path], dict[str, str]]:
    text = run_manifest.read_text(encoding="utf-8")
    input_hashes = parse_manifest_section(text, "Input hashes")
    output_hashes = parse_manifest_section(text, "Output hashes")
    if set(INPUT_NAMES) - input_hashes.keys():
        fail("RUN.md input hashes do not cover the complete fixture input bundle")
    if set(OUTPUT_NAMES) - output_hashes.keys():
        fail("RUN.md output hashes do not cover the required reference outputs")

    for name in INPUT_NAMES:
        verify_hash(run_dir / "committed-inputs" / name, input_hashes[name], "fixture input")
        verify_hash(CASE_DIR / name, input_hashes[name], "current fixture input")
    paths: dict[str, Path] = {}
    for name in OUTPUT_NAMES:
        # The runner preserves the source program's literal Windows path separator
        # in the filename on POSIX; this is one file, not a nested directory.
        path = run_dir / f"solute-input\\{name}"
        verify_hash(path, output_hashes[name], "reference output")
        paths[name] = path
    return paths, output_hashes


def parse_float_record(line: str, width: int, context: str) -> list[float]:
    tokens = line.split()
    if len(tokens) != width:
        fail(f"{context}: expected {width} fields, found {len(tokens)}")
    return [finite_number(token, context) for token in tokens]


def require_marker(lines: list[str], marker: str, context: str) -> None:
    nonempty = [line.strip() for line in lines if line.strip()]
    if not nonempty or nonempty[-1] != marker:
        fail(f"{context}: expected exact terminal marker {marker!r}")


def parse_run_inf(path: Path) -> list[dict[str, Any]]:
    lines = path.read_text(encoding="ascii").splitlines()
    require_marker(lines, "end", path.name)
    rows: list[dict[str, Any]] = []
    for line in lines:
        fields = line.split()
        if not fields or not fields[0].isdigit():
            continue
        if len(fields) != 11:
            fail(f"{path.name}: malformed run record width {len(fields)}")
        if not re.match(r"^[+-]?[\d.]+E[+-]\d+$", fields[1], re.I):
            fail(f"{path.name}: malformed run time field {fields[1]!r}")
        if fields[8] not in ("T", "F"):
            fail(f"{path.name}: invalid convergence flag {fields[8]!r}")
        rows.append(
            {
                "step": int(fields[0]),
                "time": finite_number(fields[1], path.name),
                "dt": finite_number(fields[2], path.name),
                "converged": fields[8] == "T",
            }
        )
    if len(rows) != STEP_COUNT:
        fail(f"{path.name}: expected {STEP_COUNT} step rows, found {len(rows)}")
    for index, row in enumerate(rows, start=1):
        expected_time = index * DT
        if row["step"] != index or abs(row["time"] - expected_time) > 1e-6:
            fail(f"{path.name}: unexpected step/time at row {index}: {row}")
        if abs(row["dt"] - DT) > 1e-7 or not row["converged"]:
            fail(f"{path.name}: unexpected dt or failed water convergence at step {index}")
    return rows


def parse_t_level(path: Path, run_rows: list[dict[str, Any]]) -> list[dict[str, float]]:
    lines = path.read_text(encoding="ascii").splitlines()
    require_marker(lines, "end", path.name)
    rows: list[dict[str, float]] = []
    for line in lines:
        fields = line.split()
        if not fields or not re.match(r"^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[EeDd][+-]?\d+)?$", fields[0]):
            continue
        if len(fields) != 22:
            fail(f"{path.name}: malformed TLevel record width {len(fields)}")
        if not fields[19].isdigit():
            fail(f"{path.name}: malformed TLevel step field {fields[19]!r}")
        values = [finite_number(token, path.name) for token in fields]
        rows.append(
            {
                "time": values[0],
                "top_flux": values[3],
                "bottom_flux": values[5],
                "volume": values[16],
                "step": values[19],
            }
        )
    if len(rows) != STEP_COUNT:
        fail(f"{path.name}: expected {STEP_COUNT} strict 22-field rows, found {len(rows)}")
    for index, (row, run_row) in enumerate(zip(rows, run_rows), start=1):
        if exact_int(row["step"], path.name) != index or abs(row["time"] - run_row["time"]) > 1e-6:
            fail(f"{path.name}: TLevel/time mismatch at step {index}")
        for field in ("top_flux", "bottom_flux"):
            if abs(row[field] + 0.43) > 1e-5:
                fail(f"{path.name}: expected constant {field} -0.43 at step {index}")
        if abs(row["volume"] - 107.5) > 1e-3:
            fail(f"{path.name}: unexpected volume at step {index}: {row['volume']}")
    return rows


def parse_node_profiles(path: Path) -> dict[float, list[list[float]]]:
    lines = path.read_text(encoding="ascii").splitlines()
    require_marker(lines, "end", path.name)
    profiles: dict[float, list[list[float]]] = {}
    current_time: float | None = None
    current_rows: list[list[float]] = []
    time_re = re.compile(r"^\s*Time:\s*([+-]?[\d.]+)\s*$")
    for line in lines:
        time_match = time_re.match(line)
        if time_match:
            if current_time is not None:
                if current_time in profiles:
                    fail(f"{path.name}: duplicate profile time {current_time:g} h")
                profiles[current_time] = current_rows
            current_time = finite_number(time_match.group(1), f"{path.name} time")
            current_rows = []
            continue
        fields = line.split()
        if fields and fields[0].isdigit():
            current_rows.append(parse_float_record(line, 17, path.name))
    if current_time is not None:
        if current_time in profiles:
            fail(f"{path.name}: duplicate profile time {current_time:g} h")
        profiles[current_time] = current_rows
    if set(profiles) != set(OBSERVATION_TIMES):
        fail(f"{path.name}: unexpected profile times {sorted(profiles)}")
    normalized: dict[float, list[list[float]]] = {}
    for time, rows in profiles.items():
        if len(rows) != NODE_COUNT:
            fail(f"{path.name} at {time:g} h: expected {NODE_COUNT} nodes, got {len(rows)}")
        by_coordinate = sorted(rows, key=lambda row: row[1])
        for node, row in enumerate(by_coordinate):
            if exact_int(row[0], path.name) != NODE_COUNT - node:
                # Printed node IDs follow top-to-bottom order, while x increases
                # bottom-to-top in the Rust mesh; the ID sequence must reverse.
                fail(f"{path.name} at {time:g} h: coordinate/node order mismatch")
            expected_coordinate = -250.0 + 0.25 * node
            if abs(row[1] - expected_coordinate) > 0.001:
                fail(f"{path.name} at {time:g} h: bad coordinate at normalized node {node}")
            if abs(row[2]) > 0.001 or abs(row[3] - 0.43) > 0.0001 or abs(row[4] - 0.43) > 0.0001:
                fail(f"{path.name} at {time:g} h: water invariant mismatch at node {node}")
            if abs(row[6] + 0.43) > 0.0001 or abs(row[7]) > 0.001:
                fail(f"{path.name} at {time:g} h: flux/sink invariant mismatch at node {node}")
        normalized[time] = by_coordinate
    return normalized


def parse_solute_output(path: Path) -> list[list[float]]:
    lines = path.read_text(encoding="ascii").splitlines()
    require_marker(lines, "end", path.name)
    rows: list[list[float]] = []
    for line in lines:
        fields = line.split()
        if not fields or not re.match(r"^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$", fields[0]):
            continue
        if len(fields) != 17:
            fail(f"{path.name}: malformed solute record width {len(fields)}")
        rows.append(parse_float_record(line, 17, path.name))
    if len(rows) != STEP_COUNT:
        fail(f"{path.name}: expected {STEP_COUNT} 17-field records, found {len(rows)}")
    for index, row in enumerate(rows, start=1):
        if abs(row[0] - index * DT) > 1e-5 or exact_int(row[13], path.name) != index:
            fail(f"{path.name}: unexpected time/TLevel at row {index}")
    return rows


def parse_balance(path: Path) -> dict[float, dict[str, Any]]:
    lines = path.read_text(encoding="ascii").splitlines()
    nonempty = [line.strip() for line in lines if line.strip()]
    if not nonempty or not re.fullmatch(r"Calculation time \[sec\]\s+[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[EeDd][+-]?\d+)?", nonempty[-1]):
        fail(f"{path.name}: missing or malformed calculation-time completion record")
    finite_number(nonempty[-1].split()[-1], f"{path.name} calculation time")
    times = [
        finite_number(value, path.name)
        for value in re.findall(r"^\s*Time\s+\[T\]\s+([+-]?[\d.]+)\s*$", "\n".join(lines), re.M)
    ]
    if times != [0.0, 50.0, 100.0, 200.0]:
        fail(f"{path.name}: unexpected balance times {times}")
    blocks: dict[float, dict[str, Any]] = {}
    current_time: float | None = None
    current: dict[str, Any] | None = None
    for line in lines:
        time_match = re.match(r"^\s*Time\s+\[T\]\s+([+-]?[\d.]+)\s*$", line)
        if time_match:
            current_time = finite_number(time_match.group(1), path.name)
            current = {"conc_vol": {}, "mean_concentration": {}, "balance_total": {}, "balance_percent": {}}
            blocks[current_time] = current
            continue
        if current is None:
            continue
        match = re.match(r"^\s*ConcVol\s+\[M/L2\]\s+(\d+)\s+(\S+)\s+(\S+)\s*$", line)
        if match:
            species, left, right = match.groups()
            current["conc_vol"][int(species) - 1] = [
                finite_number(left, path.name),
                finite_number(right, path.name),
            ]
            continue
        match = re.match(r"^\s*cMean\s+\[M/L3\]\s+(\d+)\s+(\S+)\s+(\S+)\s*$", line)
        if match:
            species, left, right = match.groups()
            current["mean_concentration"][int(species) - 1] = [
                finite_number(left, path.name),
                finite_number(right, path.name),
            ]
            continue
        match = re.match(r"^\s*CncBalT\s+\[M\]\s+(\d+)\s+(\S+)\s*$", line)
        if match:
            species, value = match.groups()
            current["balance_total"][int(species) - 1] = finite_number(value, path.name)
            continue
        match = re.match(r"^\s*CncBalR\s+\[%\]\s+(\d+)\s+(\S+)\s*$", line)
        if match:
            species, value = match.groups()
            current["balance_percent"][int(species) - 1] = finite_number(value, path.name)
    for time, block in blocks.items():
        if set(block["conc_vol"]) != {0, 1, 2} or set(block["mean_concentration"]) != {0, 1, 2}:
            fail(f"{path.name} at {time:g} h: expected all three ConcVol/cMean records")
        if time > 0.0 and (
            set(block["balance_total"]) != {0, 1, 2}
            or set(block["balance_percent"]) != {0, 1, 2}
        ):
            fail(f"{path.name} at {time:g} h: expected all three concentration-balance records")
    return blocks


def inventory_from_profile(rows: list[list[float]], species: int) -> dict[str, float]:
    dissolved = 0.0
    equilibrium_sorbed = 0.0
    printed_nonequilibrium_sorbed = 0.0
    kd = (0.5375, 0.0, 0.0)[species]
    for node, row in enumerate(rows):
        width = 0.125 if node in (0, NODE_COUNT - 1) else 0.25
        dissolved += width * row[3] * row[11 + species]
        equilibrium_sorbed += width * 0.8 * kd * row[11 + species]
        printed_nonequilibrium_sorbed += width * 0.8 * row[14 + species]
    return {
        "dissolved": dissolved,
        "derived_equilibrium_sorbed": equilibrium_sorbed,
        "printed_nonequilibrium_sorbed": printed_nonequilibrium_sorbed,
        "total": dissolved + equilibrium_sorbed,
    }


def rust_profile_inventory(concentration: list[float], species: int) -> float:
    kd = (0.5375, 0.0, 0.0)[species]
    return sum(
        (0.125 if node in (0, NODE_COUNT - 1) else 0.25)
        * (0.43 + 0.8 * kd)
        * value
        for node, value in enumerate(concentration)
    )


def arithmetic_bound(*terms: float) -> float:
    scale = max(1.0, sum(abs(value) for value in terms))
    return 4.0 * NODE_COUNT * math.ulp(1.0) * scale + 4.0 * math.ulp(scale)


def require_arithmetic_match(actual: float, expected: float, label: str, *terms: float) -> None:
    bound = arithmetic_bound(*terms, actual, expected)
    if abs(actual - expected) > bound:
        fail(f"{label}: {actual:.17e} != {expected:.17e} beyond arithmetic bound {bound:.3e}")


def parse_rust_observations(path: Path) -> dict[str, Any]:
    if not path.is_file():
        fail(f"missing Rust scratch observations: {path}; run cargo test --workspace first")
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise ValueError(f"malformed Rust observations: {error}") from error
    if data.get("units") != {"length": "cm", "time": "hour", "concentration": "dimensionless normalized concentration"}:
        fail("Rust observation units do not match CASE.json")
    if data.get("upper_total_flux_by_species") != [0.43, 0.0, 0.0] or data.get("lower_boundary") != "AdvectiveOutflow":
        fail("Rust boundary configuration does not match the physical mapping")
    if (data.get("schema_version") != 1 or data.get("node_count") != NODE_COUNT
            or data.get("species_count") != 3 or data.get("coordinate_start_cm") != -250.0
            or data.get("coordinate_spacing_cm") != 0.25 or data.get("time_step_h") != DT
            or data.get("step_count") != STEP_COUNT or data.get("bulk_density") != 0.8):
        fail("Rust observations do not match the reviewed Example 3 grid/time/unit metadata")
    case = json.loads((CASE_DIR / "CASE.json").read_text(encoding="utf-8"))
    metadata = data.get("species_metadata")
    expected_meta = [
        (species["name"], species["decay_rate_per_hour"]["value"], species["linear_kd_cm3_per_g"]["value"])
        for species in case["species"]
    ]
    if not isinstance(metadata, list) or len(metadata) != 3 or any(
        (row.get("name"), row.get("decay_rate"), row.get("kd")) != expected
        for row, expected in zip(metadata, expected_meta)
    ):
        fail("Rust species coefficients/order do not match CASE.json")
    if data.get("water_content") != 0.43:
        fail("Rust observations have an unsupported water content")
    if data.get("face_flux") != -0.43 or data.get("face_dispersion") != 0.18:
        fail("Rust observations do not use the source-reconstructed water/dispersion mapping")
    observations = data.get("observations")
    if not isinstance(observations, list) or len(observations) != 4:
        fail("Rust observations must contain exactly four profile times")
    if [entry.get("time_h") for entry in observations] != list(OBSERVATION_TIMES):
        fail("Rust observation times do not match 0/50/100/200 h")
    for observation in observations:
        actual_time = observation.get("actual_time_h")
        if not isinstance(actual_time, (int, float)) or not math.isfinite(actual_time):
            fail("Rust observation has a nonfinite or missing accepted time")
        if abs(actual_time - observation.get("time_h", 0.0)) > 1e-9:
            fail("Rust accumulated accepted time differs from nominal observation time")
        species_rows = observation.get("species")
        if not isinstance(species_rows, list) or [row.get("name") for row in species_rows] != list(SPECIES):
            fail("Rust observation species order is not NH4/NO2/NO3")
        for row in species_rows:
            for field in ("concentration", "equilibrium_sorbed"):
                values = row.get(field)
                if not isinstance(values, list) or len(values) != NODE_COUNT:
                    fail(f"Rust {field} profile must have {NODE_COUNT} values")
                if any(not isinstance(value, (int, float)) or not math.isfinite(value) for value in values):
                    fail(f"Rust {field} profile contains nonfinite values")
            for field in ("inventory", "cumulative_boundary_mass", "cumulative_reaction_mass", "cumulative_balance_residual"):
                value = row.get(field)
                if not isinstance(value, (int, float)) or not math.isfinite(value):
                    fail(f"Rust observation is missing finite {field}")
    return data


def residual_metrics(reference: list[float], rust: list[float], widths: list[float]) -> dict[str, float]:
    residuals = [right - left for left, right in zip(reference, rust)]
    if len(reference) != len(rust) or len(reference) != len(widths):
        fail("residual vectors have different lengths")
    weight_sum = sum(widths)
    return {
        "max_abs": max(abs(value) for value in residuals),
        "rms": math.sqrt(sum(value * value for value in residuals) / len(residuals)),
        "mesh_weighted_l2": math.sqrt(
            sum(width * value * value for width, value in zip(widths, residuals)) / weight_sum
        ),
    }


def compare(
    profiles: dict[float, list[list[float]]],
    solute_rows: list[list[list[float]]],
    balance: dict[float, dict[str, Any]],
    rust: dict[str, Any],
    output_hashes: dict[str, str],
    provenance: dict[str, str],
) -> dict[str, Any]:
    rust_by_time = {entry["time_h"]: entry for entry in rust["observations"]}
    widths = [0.125] + [0.25] * (NODE_COUNT - 2) + [0.125]
    result: dict[str, Any] = {
        "schema_version": 1,
        "tolerances_assigned": False,
        "reference_output_hashes": {name: output_hashes[name] for name in OUTPUT_NAMES},
        "provenance_hashes": provenance,
        "mapping": {
            "water_content": 0.43,
            "face_flux_increasing_coordinate": -0.43,
            "effective_dispersion": 0.18,
            "upper_total_flux_inward_by_species": [0.43, 0.0, 0.0],
            "lower_boundary": "AdvectiveOutflow",
            "outlet_and_upper_row_discretization": "documented source/Rust differences retained for measurement",
        },
        "observables": [],
    }
    for time in (50.0, 100.0, 200.0):
        step_index = int(round(time / DT)) - 1
        ref_rows = profiles[time]
        rust_observation = rust_by_time[time]
        profile_summary = []
        inventories = []
        rust_profile_inventories = []
        for species, name in enumerate(SPECIES):
            reference_c = [row[11 + species] for row in ref_rows]
            kd = (0.5375, 0.0, 0.0)[species]
            reference_s = [kd * row[11 + species] for row in ref_rows]
            rust_species = rust_observation["species"][species]
            independent_rust_inventory = rust_profile_inventory(rust_species["concentration"], species)
            require_arithmetic_match(
                rust_species["inventory"], independent_rust_inventory,
                f"Rust {name} reported inventory versus profile integral",
                independent_rust_inventory,
            )
            rust_profile_inventories.append(independent_rust_inventory)
            profile_summary.append(
                {
                    "species": name,
                    "dissolved_concentration": residual_metrics(
                        reference_c, rust_species["concentration"], widths
                    ),
                    "equilibrium_sorbed_concentration": residual_metrics(
                        reference_s, rust_species["equilibrium_sorbed"], widths
                    ),
                    "reference_printed_nonequilibrium_sorbed_max_abs": max(
                        abs(row[14 + species]) for row in ref_rows
                    ),
                }
            )
            reference_inventory = inventory_from_profile(ref_rows, species)
            inventories.append(
                {
                    "species": name,
                    "reference_profile_integral": reference_inventory,
                    "rust_independent_integral": independent_rust_inventory,
                    "rust_reported_inventory": rust_species["inventory"],
                    "rust_minus_reference_total": independent_rust_inventory - reference_inventory["total"],
                    "reference_balance_conc_vol_records": balance[time]["conc_vol"][species],
                    "reference_balance_cnc_bal_t": balance[time]["balance_total"][species],
                    "reference_balance_cnc_bal_r_percent": balance[time]["balance_percent"][species],
                }
            )

        fluxes = []
        for species, name in enumerate(SPECIES):
            reference_boundary = solute_rows[species][step_index][3] + solute_rows[species][step_index][4]
            reference_chain_zero = solute_rows[species][step_index][5]
            reference_chain_first = solute_rows[species][step_index][6]
            rust_species = rust_observation["species"][species]
            independently_recomputed_rust_balance = (
                rust_profile_inventories[species]
                - rust_species["cumulative_boundary_mass"]
                - rust_species["cumulative_reaction_mass"]
            )
            require_arithmetic_match(
                rust_species["cumulative_balance_residual"],
                independently_recomputed_rust_balance,
                f"Rust {name} cumulative balance residual",
                rust_profile_inventories[species],
                rust_species["cumulative_boundary_mass"],
                rust_species["cumulative_reaction_mass"],
            )
            fluxes.append(
                {
                    "species": name,
                    "reference_cumulative_boundary_inward": reference_boundary,
                    "rust_cumulative_boundary_inward": rust_species["cumulative_boundary_mass"],
                    "rust_minus_reference_boundary": rust_species["cumulative_boundary_mass"] - reference_boundary,
                    "reference_cumulative_cvCh0_source": reference_chain_zero,
                    "reference_cumulative_cvCh1_reaction": reference_chain_first,
                    "reference_cumulative_chain_source_net": reference_chain_zero + reference_chain_first,
                    "rust_cumulative_reaction_change": rust_species["cumulative_reaction_mass"],
                    "rust_cumulative_balance_residual": rust_species["cumulative_balance_residual"],
                    "rust_independently_recomputed_balance_residual": independently_recomputed_rust_balance,
                    "reference_profile_storage_change": inventory_from_profile(ref_rows, species)["total"]
                    - inventory_from_profile(profiles[0.0], species)["total"],
                    "reference_independent_balance_residual": inventory_from_profile(ref_rows, species)["total"]
                    - inventory_from_profile(profiles[0.0], species)["total"]
                    - reference_boundary
                    - reference_chain_zero
                    - reference_chain_first,
                }
            )
        result["observables"].append(
            {
                "time_h": time,
                "profile_residuals": profile_summary,
                "inventories": inventories,
                "cumulative_flux_and_process_terms": fluxes,
            }
        )
    return result


def expect_failure(description: str, operation: Callable[[], Any]) -> None:
    try:
        operation()
    except (ValueError, OSError):
        return
    raise AssertionError(f"negative self-test unexpectedly passed: {description}")


def test_species_balance_isolation() -> None:
    profiles: dict[float, list[list[float]]] = {}
    for time in OBSERVATION_TIMES:
        rows = []
        for node in range(NODE_COUNT):
            row = [0.0] * 17
            row[1] = -250.0 + node * 0.25
            row[3] = 0.43
            rows.append(row)
        profiles[time] = rows
    solute_rows = [[[0.0] * 17 for _ in range(STEP_COUNT)] for _ in SPECIES]
    for species_rows in solute_rows:
        for index, row in enumerate(species_rows, start=1):
            row[0] = index * DT
    balance = {
        time: {
            "conc_vol": {species: [0.0, 0.0] for species in range(3)},
            "mean_concentration": {species: [0.0, 0.0] for species in range(3)},
            "balance_total": {species: 0.0 for species in range(3)},
            "balance_percent": {species: 0.0 for species in range(3)},
        }
        for time in OBSERVATION_TIMES
    }
    rust_observations = []
    for time in OBSERVATION_TIMES:
        species_data = []
        for species, name in enumerate(SPECIES):
            concentration = [0.001 * (species + 1)] * NODE_COUNT
            inventory = rust_profile_inventory(concentration, species)
            boundary = 0.1 * (species + 1)
            reaction = 0.01 * (species - 1)
            species_data.append({
                "name": name,
                "concentration": concentration,
                "equilibrium_sorbed": [(0.5375, 0.0, 0.0)[species] * value for value in concentration],
                "inventory": inventory,
                "cumulative_boundary_mass": boundary,
                "cumulative_reaction_mass": reaction,
                "cumulative_balance_residual": inventory - boundary - reaction,
            })
        rust_observations.append({"time_h": time, "species": species_data})
    rust = {"observations": rust_observations}
    result = compare(
        profiles, solute_rows, balance, rust,
        {name: "test" for name in OUTPUT_NAMES}, {},
    )
    flux_rows = result["observables"][0]["cumulative_flux_and_process_terms"]
    reported = [row["rust_independently_recomputed_balance_residual"] for row in flux_rows]
    expected = [
        rust_observations[1]["species"][species]["inventory"]
        - rust_observations[1]["species"][species]["cumulative_boundary_mass"]
        - rust_observations[1]["species"][species]["cumulative_reaction_mass"]
        for species in range(3)
    ]
    if reported != expected or len(set(reported)) != 3:
        raise AssertionError("compare() reused a different species' cumulative balance")


def self_test() -> None:
    expect_failure("missing marker", lambda: require_marker(["row", ""], "end", "test"))
    expect_failure("wrong shape", lambda: parse_float_record("1 2", 3, "test"))
    expect_failure("malformed value", lambda: finite_number("1e999", "test"))
    expect_failure("hash mismatch", lambda: verify_hash(Path(__file__), "0" * 64, "test"))
    expect_failure("missing file", lambda: verify_hash(Path("/definitely/missing/solute.out"), "0" * 64, "test"))
    with tempfile.TemporaryDirectory() as temporary:
        malformed = Path(temporary) / "observations.json"
        malformed.write_text("{}", encoding="utf-8")
        expect_failure("absent required fields", lambda: parse_rust_observations(malformed))
        malformed.write_text(json.dumps({
            "schema_version": 1,
            "units": {"length": "cm", "time": "hour", "concentration": "dimensionless normalized concentration"},
            "node_count": NODE_COUNT, "species_count": 3, "coordinate_start_cm": -250.0,
            "coordinate_spacing_cm": 0.25, "time_step_h": DT, "step_count": STEP_COUNT,
            "bulk_density": 0.8,
            "species_metadata": [{"name": n, "decay_rate": r, "kd": k} for n, r, k in (("NH4", .005, .5375), ("NO2", .1, 0.), ("NO3", 0., 0.))],
            "upper_total_flux_by_species": [.43, 0., 0.], "lower_boundary": "AdvectiveOutflow",
            "water_content": .43, "face_flux": -.43, "face_dispersion": .18,
            "observations": []
        }), encoding="utf-8")
        expect_failure("wrong Rust observation shape", lambda: parse_rust_observations(malformed))
        complete = json.loads(malformed.read_text(encoding="utf-8"))
        complete["observations"] = []
        for time in OBSERVATION_TIMES:
            complete["observations"].append({
                "time_h": time, "actual_time_h": time,
                "species": [{
                    "name": name, "concentration": [0.0] * NODE_COUNT,
                    "equilibrium_sorbed": [0.0] * NODE_COUNT,
                    "inventory": 0.0, "cumulative_boundary_mass": 0.0,
                    "cumulative_reaction_mass": 0.0, "cumulative_balance_residual": 0.0,
                } for name in SPECIES],
            })
        complete["observations"][1]["actual_time_h"] = float("nan")
        malformed.write_text(json.dumps(complete), encoding="utf-8")
        expect_failure("nonfinite actual time", lambda: parse_rust_observations(malformed))
        bad_solute = Path(temporary) / "solute.out"
        bad_solute.write_text("malformed row\nend\n", encoding="ascii")
        expect_failure("malformed solute row/missing records", lambda: parse_solute_output(bad_solute))
        bad_tlevel = Path(temporary) / "T_LEVEL.OUT"
        bad_tlevel.write_text("not a TLevel record\nend\n", encoding="ascii")
        expect_failure("malformed T_LEVEL shape", lambda: parse_t_level(bad_tlevel, []))
        raw_profile = DEFAULT_RUN / "solute-input\\NOD_INF.OUT"
        if raw_profile.is_file():
            mutated_profile = Path(temporary) / "NOD_INF.OUT"
            profile_text = raw_profile.read_text(encoding="ascii")
            marker = re.search(r"^\s*Time:\s*[+-]?[\d.]+\s*$", profile_text, re.M)
            assert marker
            mutated_profile.write_text(profile_text[:marker.end()] + "\n" + profile_text[marker.start():marker.end()] + profile_text[marker.end():], encoding="ascii")
            expect_failure("duplicate time in complete reference profile", lambda: parse_node_profiles(mutated_profile))
    test_species_balance_isolation()
    print("parser negative and comparator regression self-tests passed (12 cases)")


def run(case_dir: Path) -> dict[str, Any]:
    if case_dir.resolve() != CASE_DIR.resolve():
        fail(f"only the reviewed Example 3 case is accepted: {case_dir}")
    run_dir_value = os.environ.get("HYDRUS_REFERENCE_RUN_DIR")
    run_dir = (ROOT / run_dir_value).resolve() if run_dir_value else DEFAULT_RUN.resolve()
    allowed_reference = (ROOT / ".development/12.2.1").resolve()
    if not run_dir.is_relative_to(allowed_reference):
        fail(f"reference run directory must remain under {allowed_reference}")
    paths, output_hashes = verified_paths(run_dir, CASE_DIR / "RUN.md")
    run_manifest = CASE_DIR / "RUN.md"
    provenance = {
        "CASE.json": sha256(CASE_DIR / "CASE.json"),
        "RUN.md": sha256(run_manifest),
    }
    for input_name in INPUT_NAMES:
        provenance[f"input:{input_name}"] = sha256(CASE_DIR / input_name)
    run_rows = parse_run_inf(paths["RUN_INF.OUT"])
    parse_t_level(paths["T_LEVEL.OUT"], run_rows)
    profiles = parse_node_profiles(paths["NOD_INF.OUT"])
    solute_rows = [parse_solute_output(paths[f"solute{index}.out"]) for index in range(1, 4)]
    balance = parse_balance(paths["BALANCE.OUT"])

    rust_value = os.environ.get("HYDRUS_RUST_OBSERVATIONS")
    rust_path = Path(rust_value) if rust_value else SCRATCH / "rust-observations.json"
    rust_path = rust_path if rust_path.is_absolute() else ROOT / rust_path
    scratch_resolved = SCRATCH.resolve()
    if not rust_path.resolve().is_relative_to(scratch_resolved):
        fail(f"Rust observations must remain under {scratch_resolved}")
    rust = parse_rust_observations(rust_path.resolve())
    provenance["rust_observations.json"] = sha256(rust_path.resolve())
    comparison = compare(profiles, solute_rows, balance, rust, output_hashes, provenance)

    SCRATCH.mkdir(parents=True, exist_ok=True)
    normalized_path = SCRATCH / "normalized-reference.json"
    normalized = {
        "schema_version": 1,
        "raw_hashes": {name: output_hashes[name] for name in OUTPUT_NAMES},
        "provenance_hashes": provenance,
        "times_h": list(OBSERVATION_TIMES),
        "profiles": {
            str(time): [
                {
                    "coordinate_cm": row[1],
                    "water_content": row[3],
                    "flux_cm_per_hour": row[6],
                    "concentration": row[11:14],
                    "derived_equilibrium_sorbed": [kd * row[11 + species] for species, kd in enumerate((0.5375, 0.0, 0.0))],
                    "printed_nonequilibrium_sorbed": row[14:17],
                }
                for row in rows
            ]
            for time, rows in profiles.items()
        },
    }
    normalized_path.write_text(json.dumps(normalized, indent=2) + "\n", encoding="utf-8")
    report_path = SCRATCH / "residual-report.json"
    report_path.write_text(json.dumps(comparison, indent=2) + "\n", encoding="utf-8")
    return {
        "normalized_reference": str(normalized_path),
        "residual_report": str(report_path),
        "species": list(SPECIES),
        "times_h": [50.0, 100.0, 200.0],
        "tolerances_assigned": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("case", nargs="?", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    try:
        if args.self_test:
            self_test()
            return 0
        if args.case is None:
            parser.error("case path is required unless --self-test is used")
        result = run(args.case)
    except (OSError, ValueError, AssertionError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

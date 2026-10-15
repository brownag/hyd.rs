"""Manifest-driven normalizer for raw Fortran solute parity fixture runs.

This module validates fixture OBSERVATION_MAP.json declarations, authenticates raw
runner output artifacts and manifests, parses raw Fortran outputs (PROFILE.OUT,
NOD_INF.OUT, solute1..NS.out, OBS_NODE.OUT, BALANCE.OUT), and emits normalized
solute observations in the exact order declared by each fixture map.
"""

from __future__ import annotations

import hashlib
import json
import math
import re
from decimal import Decimal, InvalidOperation, localcontext
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple, Union

OBSERVATION_MAP_SCHEMA = "solute-observation-map-v1"
NORMALIZED_OBSERVATION_SCHEMA = "solute-observation-v1"


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    return json.loads(
        path.read_text(encoding="utf-8"),
        object_pairs_hook=_unique_object,
        parse_constant=lambda value: (_ for _ in ()).throw(ValueError(f"invalid number: {value}")),
    )


def canonical_hash(value: object) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def parse_decimal_token(token: str) -> Decimal:
    """Parse Fortran numeric token with D/d or E/e exponent, or fixed decimal."""
    try:
        norm = token.strip().replace("D", "E").replace("d", "e")
        return Decimal(norm)
    except InvalidOperation as err:
        raise ValueError(f"invalid numeric token: {token!r}") from err


def compute_half_width(token: str, fmt: Optional[str] = None) -> Decimal:
    """Compute exact precision interval half-width (half one unit in the last printed decimal place)."""
    norm = token.strip().replace("D", "E").replace("d", "e")
    with localcontext() as ctx:
        ctx.prec = 1200
        dec = parse_decimal_token(token)
        # Check scientific notation
        match_sci = re.search(r"[eE]([+-]?\d+)\Z", norm)
        if match_sci:
            exp_val = int(match_sci.group(1))
            mantissa = norm[: match_sci.start()]
            if "." in mantissa:
                decimals = len(mantissa.partition(".")[2])
            else:
                decimals = 0
            unit_exp = exp_val - decimals
        else:
            if "." in norm:
                decimals = len(norm.partition(".")[2])
            else:
                decimals = 0
            unit_exp = -decimals
        return Decimal(5).scaleb(unit_exp - 1)


def validate_observation_map(data: dict) -> dict:
    """Strictly validate an OBSERVATION_MAP.json against solute-observation-map-v1 contract."""
    _require(isinstance(data, dict), "observation map must be a JSON object")
    _require(data.get("schema_version") == OBSERVATION_MAP_SCHEMA, f"unsupported observation map schema: {data.get('schema_version')}")
    observations = data.get("observations")
    _require(isinstance(observations, list) and len(observations) > 0, "observations must be a non-empty list")

    seen_keys = set()
    for idx, obs in enumerate(observations):
        _require(isinstance(obs, dict), f"observation at index {idx} must be an object")
        field = obs.get("field") or obs.get("field_identity")
        _require(isinstance(field, str) and field, f"observation at index {idx} missing field identity")
        avail = obs.get("availability")
        units = obs.get("units")
        _require(isinstance(units, str) and (units or avail == "unavailable"), f"observation {field} at index {idx} missing units")

        raw_file = obs.get("raw_file") or obs.get("file") or obs.get("raw_output_file")
        reason = obs.get("reason") or obs.get("unavailability_reason")

        if avail == "unavailable" or raw_file in (None, "none", "None"):
            _require(isinstance(reason, str) and reason.strip(), f"observation {field} is unavailable but lacks an explicit reason")

        location = obs.get("location") or obs.get("location_identity") or {
            "species": obs.get("species"),
            "node": obs.get("node"),
            "time": obs.get("time"),
        }
        loc_repr = canonical_hash(location)
        obs_key = (field, loc_repr)
        _require(obs_key not in seen_keys, f"duplicate observation field/location identity: {field}")
        seen_keys.add(obs_key)

    return data


def authenticate_runner_result(runner_manifest: dict, task_root: Path, expected_input_files: Optional[dict] = None) -> dict:
    """Authenticate runner raw manifest and verify output artifact files, hashes, and directory paths."""
    _require(isinstance(runner_manifest, dict), "runner manifest must be an object")
    _require(runner_manifest.get("complete") is True, "runner manifest indicates run was not complete")

    root = task_root.resolve()
    manifest_task_root = Path(runner_manifest.get("task_root", "")).resolve()
    _require(manifest_task_root == root, f"manifest task root {manifest_task_root} does not match requested {root}")

    run_root = Path(runner_manifest.get("run_root", "")).resolve()
    _require(run_root.is_relative_to(root), f"run_root escapes task_root: {run_root}")
    _require(run_root.is_dir(), f"run_root is not a directory: {run_root}")

    if expected_input_files is not None:
        _require(runner_manifest.get("input_sha256") == expected_input_files, "input_sha256 in manifest does not match fixture inputs")

    output_paths = runner_manifest.get("output_paths")
    output_hashes = runner_manifest.get("output_sha256")
    _require(isinstance(output_paths, dict) and output_paths, "runner manifest missing output_paths")
    _require(isinstance(output_hashes, dict) and output_hashes, "runner manifest missing output_sha256")
    _require(output_paths.keys() == output_hashes.keys(), "keys of output_paths and output_sha256 differ")

    for logical_name, rel_path in output_paths.items():
        _require(isinstance(rel_path, str) and rel_path, f"invalid output path for {logical_name}")
        expected_hash = output_hashes[logical_name]
        _require(isinstance(expected_hash, str) and len(expected_hash) == 64, f"invalid output hash for {logical_name}")

        artifact_path = (root / rel_path).resolve()
        _require(artifact_path.is_relative_to(root), f"output path escapes task_root: {rel_path}")
        _require(artifact_path.is_file(), f"output artifact file missing: {rel_path}")
        actual_hash = sha256_file(artifact_path)
        _require(actual_hash == expected_hash, f"hash mismatch for output artifact {logical_name}")

    return runner_manifest


class RawFortranOutputBundle:
    """Loads and caches raw Fortran output files from a run directory."""

    def __init__(self, run_root: Path):
        self.run_root = run_root.resolve()
        self._parsed_profiles: Optional[List[Dict[str, Any]]] = None
        self._parsed_nod_inf: Optional[List[Dict[str, Any]]] = None
        self._parsed_solute_files: Dict[str, List[List[str]]] = {}
        self._parsed_obs_node: Optional[List[List[str]]] = None
        self._parsed_balance: Optional[List[Dict[str, Any]]] = None

    def read_file_lines(self, filename: str) -> List[str]:
        # Try direct file and case variations (solute1.out, PROFILE.OUT, etc.)
        candidates = [self.run_root / filename, self.run_root / filename.upper(), self.run_root / filename.lower()]
        target = None
        for c in candidates:
            if c.is_file():
                target = c
                break
        if target is None:
            fn_lower = filename.lower()
            for f in self.run_root.iterdir():
                if f.is_file():
                    name_lower = f.name.lower()
                    if name_lower == fn_lower or name_lower.endswith("\\" + fn_lower) or name_lower.endswith("/" + fn_lower):
                        target = f
                        break
        if target is None:
            raise ValueError(f"raw output file not found in run directory: {filename}")
        return target.read_text(encoding="utf-8", errors="strict").splitlines()

    def get_profile_blocks(self) -> List[Dict[str, Any]]:
        """Parse PROFILE.OUT blocks, falling back to NOD_INF.OUT if PROFILE.OUT contains only mesh data."""
        if self._parsed_profiles is not None:
            return self._parsed_profiles
        lines = self.read_file_lines("PROFILE.OUT")
        blocks = []
        current_block: Optional[Dict[str, Any]] = None
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("Time:"):
                time_token = stripped.partition(":")[2].strip()
                current_block = {"time_token": time_token, "rows": []}
                blocks.append(current_block)
            elif current_block is not None and stripped and not stripped.startswith(("[", "Node")) and stripped != "end":
                tokens = stripped.split()
                # Check that first token is integer node index
                if tokens and tokens[0].isdigit():
                    current_block["rows"].append(tokens)
        if not blocks:
            # Pinned Fortran outputs nodal time histories to NOD_INF.OUT (unit 75)
            # while PROFILE.OUT (unit 78) writes the static material/grid profile.
            blocks = self.get_nod_inf_blocks()
        self._parsed_profiles = blocks
        return blocks

    def get_nod_inf_blocks(self, filename: str = "NOD_INF.OUT") -> List[Dict[str, Any]]:
        """Parse NOD_INF.OUT / NOD_INF_V.OUT blocks."""
        fn_key = filename.upper()
        if fn_key == "NOD_INF.OUT" and self._parsed_nod_inf is not None:
            return self._parsed_nod_inf
        lines = self.read_file_lines(filename)
        blocks = []
        current_block: Optional[Dict[str, Any]] = None
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("Time:"):
                time_token = stripped.partition(":")[2].strip()
                current_block = {"time_token": time_token, "rows": []}
                blocks.append(current_block)
            elif current_block is not None and stripped and not stripped.startswith(("[", "Node")) and stripped != "end":
                tokens = stripped.split()
                if tokens and tokens[0].isdigit():
                    current_block["rows"].append(tokens)
        if fn_key == "NOD_INF.OUT":
            self._parsed_nod_inf = blocks
        return blocks

    def get_solute_rows(self, filename: str) -> List[List[str]]:
        """Parse solute1..NS.out data rows."""
        canonical_name = filename.lower()
        if canonical_name in self._parsed_solute_files:
            return self._parsed_solute_files[canonical_name]
        lines = self.read_file_lines(filename)
        rows = []
        for line in lines:
            stripped = line.strip()
            if not stripped or stripped.startswith("All") or stripped.startswith("Time") or stripped.startswith("["):
                continue
            tokens = stripped.split()
            # Row begins with time token (float / scientific)
            try:
                parse_decimal_token(tokens[0])
                rows.append(tokens)
            except ValueError:
                continue
        self._parsed_solute_files[canonical_name] = rows
        return rows

    def get_obs_node_rows(self) -> List[List[str]]:
        """Parse OBS_NODE.OUT rows."""
        if self._parsed_obs_node is not None:
            return self._parsed_obs_node
        lines = self.read_file_lines("OBS_NODE.OUT")
        rows = []
        for line in lines:
            stripped = line.strip()
            if not stripped or stripped.startswith("Time") or stripped.startswith("[") or stripped.startswith("h"):
                continue
            tokens = stripped.split()
            try:
                parse_decimal_token(tokens[0])
                rows.append(tokens)
            except ValueError:
                continue
        self._parsed_obs_node = rows
        return rows

    def get_balance_records(self) -> List[Dict[str, Any]]:
        """Parse BALANCE.OUT blocks."""
        if self._parsed_balance is not None:
            return self._parsed_balance
        lines = self.read_file_lines("BALANCE.OUT")
        records = []
        current: Optional[Dict[str, Any]] = None
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("Time") and "[T]" in stripped:
                time_token = stripped.partition("[T]")[2].strip()
                current = {"time_token": time_token, "entries": {}}
                records.append(current)
            elif current is not None:
                # E.g. "W-volumeI[L]      0.12345E+01"
                for label in ("W-volumeI[L]", "W-volume [L]", "WatBalT  [L]", "CncBalT  [M]"):
                    if label in stripped:
                        parts = stripped.partition(label)
                        tokens = parts[2].split()
                        if tokens:
                            current["entries"][label] = tokens
        self._parsed_balance = records
        return records


def normalize_fortran_outputs(
    observation_map_path: Path,
    runner_manifest_path: Path,
    task_root: Path,
    fixture_manifest_path: Optional[Path] = None,
) -> List[dict]:
    """Parse raw Fortran outputs according to OBSERVATION_MAP.json and emit normalized observation records."""
    map_data = load_json(observation_map_path)
    validate_observation_map(map_data)

    runner_manifest = load_json(runner_manifest_path)
    expected_inputs = None
    if fixture_manifest_path is not None:
        fixture_data = load_json(fixture_manifest_path)
        expected_inputs = fixture_data.get("input_files")

    authenticate_runner_result(runner_manifest, task_root, expected_inputs)

    run_root = Path(runner_manifest["run_root"]).resolve()
    bundle = RawFortranOutputBundle(run_root)

    normalized_observations: List[dict] = []

    for obs in map_data["observations"]:
        field = obs.get("field") or obs.get("field_identity")
        units = obs["units"]
        raw_file = obs.get("raw_file") or obs.get("file") or obs.get("raw_output_file")
        avail = obs.get("availability")
        reason = obs.get("reason") or obs.get("unavailability_reason")
        selector = obs.get("row_selector", {})
        selector_kind = selector.get("kind") if isinstance(selector, dict) else None

        # Build canonical location dict
        loc_spec = obs.get("location") or obs.get("location_identity")
        if loc_spec is not None and isinstance(loc_spec, dict):
            location = dict(loc_spec)
        else:
            location = {}
            if "species" in obs or "species_position" in obs:
                location["species"] = obs.get("species_position", obs.get("species"))
            if "node" in obs:
                location["node"] = obs.get("node")
            if "time" in obs:
                location["time"] = obs.get("time")

        # Handle explicit unavailable
        if avail == "unavailable" or raw_file in (None, "none", "None") or selector_kind == "unavailable":
            _require(bool(reason), f"observation {field} is unavailable but has no reason")
            normalized_observations.append({
                "field": field,
                "location": location,
                "units": units,
                "value": None,
                "availability": "unavailable",
                "reason": reason,
            })
            continue

        # Extract value from raw output file
        rf_upper = raw_file.upper()
        source_col = obs.get("source_column")
        if source_col is None and "column" in obs:
            source_col = obs["column"]
        if source_col is None and "source_columns" in obs:
            cols = obs["source_columns"]
            if isinstance(cols, list) and len(cols) == 1:
                source_col = cols[0]

        value = None
        precision_interval = None
        source_format = obs.get("source_format") or obs.get("source_numeric_format")

        if rf_upper == "PROFILE.OUT":
            blocks = bundle.get_profile_blocks()
            _require(len(blocks) > 0, "PROFILE.OUT has no time blocks")
            _require(source_col is not None, f"field {field} in PROFILE.OUT missing source_column")
            col_idx = int(source_col) - 1

            # Extract profile array
            profile_values = []
            interval_widths = []
            for block in blocks:
                time_tok = block["time_token"]
                time_val = float(parse_decimal_token(time_tok))
                # Check coordinate reversal: nodes are printed from surface (N) down to bottom (1)
                # or reversed. We extract the column value for each node row.
                step_values = []
                step_intervals = []
                for row in block["rows"]:
                    _require(col_idx < len(row), f"column {source_col} out of range in PROFILE.OUT row (len {len(row)})")
                    tok = row[col_idx]
                    dec = parse_decimal_token(tok)
                    half_w = compute_half_width(tok, source_format)
                    step_values.append(float(dec))
                    step_intervals.append(float(half_w))
                profile_values.append(step_values)
                interval_widths.append(step_intervals)

            # If location specified single time/node vs profile
            if len(profile_values) == 1:
                value = profile_values[0]
                precision_interval = interval_widths[0]
            else:
                value = profile_values
                precision_interval = interval_widths

        elif rf_upper in ("NOD_INF.OUT", "NOD_INF_V.OUT"):
            blocks = bundle.get_nod_inf_blocks(raw_file)
            _require(len(blocks) > 0, f"{raw_file} has no time blocks")
            if source_col is None and (selector_kind == "Time header records" or field == "time"):
                profile_values = []
                interval_widths = []
                for block in blocks:
                    tok = block["time_token"]
                    dec = parse_decimal_token(tok)
                    half_w = compute_half_width(tok, source_format)
                    profile_values.append(float(dec))
                    interval_widths.append(float(half_w))
                value = profile_values
                precision_interval = interval_widths
            else:
                _require(source_col is not None, f"field {field} in {raw_file} missing source_column")
                col_idx = int(source_col) - 1

                profile_values = []
                interval_widths = []
                for block in blocks:
                    step_values = []
                    step_intervals = []
                    for row in block["rows"]:
                        _require(col_idx < len(row), f"column {source_col} out of range in {raw_file} row")
                        tok = row[col_idx]
                        dec = parse_decimal_token(tok)
                        half_w = compute_half_width(tok, source_format)
                        step_values.append(float(dec))
                        step_intervals.append(float(half_w))
                    profile_values.append(step_values)
                    interval_widths.append(step_intervals)

                if len(profile_values) == 1:
                    value = profile_values[0]
                    precision_interval = interval_widths[0]
                else:
                    value = profile_values
                    precision_interval = interval_widths

        elif rf_upper.startswith("SOLUTE") and rf_upper.endswith(".OUT"):
            rows = bundle.get_solute_rows(raw_file)
            _require(len(rows) > 0, f"{raw_file} has no data rows")
            _require(source_col is not None, f"field {field} in {raw_file} missing source_column")
            col_idx = int(source_col) - 1

            col_values = []
            intervals = []
            for row in rows:
                _require(col_idx < len(row), f"column {source_col} out of range in {raw_file} (len {len(row)})")
                tok = row[col_idx]
                dec = parse_decimal_token(tok)
                half_w = compute_half_width(tok, source_format)
                col_values.append(float(dec))
                intervals.append(float(half_w))

            if len(col_values) == 1:
                value = col_values[0]
                precision_interval = intervals[0]
            else:
                value = col_values
                precision_interval = intervals

        elif rf_upper == "OBS_NODE.OUT":
            rows = bundle.get_obs_node_rows()
            _require(len(rows) > 0, "OBS_NODE.OUT has no data rows")
            _require(source_col is not None, f"field {field} in OBS_NODE.OUT missing source_column")
            col_idx = int(source_col) - 1

            col_values = []
            intervals = []
            for row in rows:
                _require(col_idx < len(row), f"column {source_col} out of range in OBS_NODE.OUT")
                tok = row[col_idx]
                dec = parse_decimal_token(tok)
                half_w = compute_half_width(tok, source_format)
                col_values.append(float(dec))
                intervals.append(float(half_w))

            if len(col_values) == 1:
                value = col_values[0]
                precision_interval = intervals[0]
            else:
                value = col_values
                precision_interval = intervals

        elif rf_upper == "BALANCE.OUT":
            records = bundle.get_balance_records()
            _require(len(records) > 0, "BALANCE.OUT has no summary records")
            label = selector.get("label") if isinstance(selector, dict) else None
            _require(bool(label), f"field {field} in BALANCE.OUT requires a row_selector label")

            col_idx = (int(source_col) - 1) if source_col is not None else 0
            rec_values = []
            intervals = []
            for rec in records:
                entries = rec["entries"].get(label)
                if entries is None:
                    continue
                if col_idx >= len(entries) and len(entries) == 1:
                    tok = entries[0]
                else:
                    _require(col_idx < len(entries), f"column {source_col} out of range in BALANCE.OUT label {label}")
                    tok = entries[col_idx]
                dec = parse_decimal_token(tok)
                half_w = compute_half_width(tok, source_format)
                rec_values.append(float(dec))
                intervals.append(float(half_w))

            if not rec_values:
                # If label not found in this run
                _require(bool(reason), f"BALANCE.OUT label {label} missing and no reason provided")
                normalized_observations.append({
                    "field": field,
                    "location": location,
                    "units": units,
                    "value": None,
                    "availability": "unavailable",
                    "reason": reason,
                })
                continue
            elif len(rec_values) == 1:
                value = rec_values[0]
                precision_interval = intervals[0]
            else:
                value = rec_values
                precision_interval = intervals
        else:
            raise ValueError(f"unsupported raw output file: {raw_file}")

        obs_record = {
            "field": field,
            "location": location,
            "units": units,
            "value": value,
            "availability": "available",
            "precision_interval": precision_interval,
        }
        normalized_observations.append(obs_record)

    return normalized_observations

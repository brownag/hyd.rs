#!/usr/bin/env python3
"""Compare HYDRUS defect probe output pairs against a probe MANIFEST.json.

This tool extracts target output variables across probe deck pairs (defect pair
and control pair) and evaluates verdicts on whether a suspected source defect is
present, fixed, or inconclusive in a given HYDRUS build.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple, Union

DEFAULT_RELATIVE_THRESHOLD = 1e-4


def parse_decimal_token(token: str) -> Decimal:
    """Parse Fortran-style floating point token into Decimal."""
    clean = token.strip().replace("D", "E").replace("d", "e")
    return Decimal(clean)


def parse_tokens(line: str) -> List[str]:
    """Split line by whitespace, ignoring empty parts."""
    return line.strip().split()


def extract_from_nod_inf(
    lines: Sequence[str],
    target_node: Optional[int] = None,
    target_time: Optional[float] = None,
    col_index: Optional[int] = None,
    header_offset: int = 0,
    col_offset: int = 0,
) -> List[Decimal]:
    """Extract values from NOD_INF.OUT or PROFILE.OUT text lines.

    Lines contain blocks headed by 'Time: <val>'.
    Nodes typically listed with 'Node' or directly numbers.
    header_offset: line offset after the Time: header to begin table rows if needed.
    col_offset: column index offset added to col_index.
    """
    effective_col = col_index + col_offset if col_index is not None else None
    blocks: List[Dict[str, Any]] = []
    current_block: Optional[Dict[str, Any]] = None

    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("Time:") or stripped.startswith("time:"):
            time_token = stripped.partition(":")[2].strip().split()[0]
            try:
                time_val = float(parse_decimal_token(time_token))
            except Exception:
                time_val = 0.0
            current_block = {"time": time_val, "lines": []}
            blocks.append(current_block)
        elif current_block is not None:
            if stripped.startswith("[") or stripped.startswith("Node") or stripped.startswith("node") or stripped.startswith("end") or stripped.startswith("End"):
                continue
            current_block["lines"].append(stripped)

    if not blocks:
        # If no explicit Time: blocks found, treat entire file as single block at time 0.0
        current_block = {"time": 0.0, "lines": []}
        for line in lines:
            stripped = line.strip()
            if not stripped or stripped.startswith("[") or stripped.startswith("Node") or stripped.startswith("node"):
                continue
            current_block["lines"].append(stripped)
        blocks.append(current_block)

    # Filter by time if specified, else use the first block (t=0 or first printed time)
    selected_block = None
    if target_time is not None:
        for b in blocks:
            if math.isclose(b["time"], target_time, rel_tol=1e-5, abs_tol=1e-5):
                selected_block = b
                break
        if selected_block is None:
            raise ValueError(f"target time {target_time} not found in NOD_INF output (available: {[b['time'] for b in blocks]})")
    else:
        selected_block = blocks[0]

    data_lines = selected_block["lines"][header_offset:]
    extracted: List[Decimal] = []

    # Node ID can be in col 0 or shifted by node_col_offset / col_offset if extra columns exist
    for row_line in data_lines:
        toks = parse_tokens(row_line)
        if not toks:
            continue
        # Find which token is the integer node ID
        node_id = None
        node_tok_idx = -1
        # Try col 0 first, or try tokens within the first few columns
        for idx in range(min(len(toks), 5)):
            try:
                node_id = int(toks[idx])
                node_tok_idx = idx
                break
            except ValueError:
                continue

        if node_id is None:
            continue

        if target_node is not None and node_id != target_node:
            continue

        if effective_col is not None:
            if effective_col >= len(toks):
                raise ValueError(f"column index {effective_col} out of range in row: {row_line!r}")
            extracted.append(parse_decimal_token(toks[effective_col]))
        else:
            # If no col_index specified, collect all tokens after node id
            for tok in toks[node_tok_idx + 1:]:
                extracted.append(parse_decimal_token(tok))


    if not extracted:
        raise ValueError(f"no matching data found for node={target_node}, time={target_time}")
    return extracted


def extract_from_obs_node(
    lines: Sequence[str],
    target_node_idx: Optional[int] = None,
    target_time: Optional[float] = None,
    col_index: Optional[int] = None,
    header_offset: int = 0,
    col_offset: int = 0,
) -> List[Decimal]:
    """Extract values from OBS_NODE.OUT text lines.

    Lines contain observation node time series.
    header_offset skips preliminary header lines.
    """
    effective_col = col_index + col_offset if col_index is not None else None
    data_lines = [l.strip() for l in lines if l.strip()]
    if header_offset < len(data_lines):
        data_lines = data_lines[header_offset:]

    extracted: List[Decimal] = []
    for line in data_lines:
        if line.startswith("time") or line.startswith("Time") or line.startswith("[") or line.startswith("node"):
            continue
        toks = parse_tokens(line)
        if not toks:
            continue
        try:
            time_val = float(parse_decimal_token(toks[0]))
        except Exception:
            continue

        if target_time is not None and not math.isclose(time_val, target_time, rel_tol=1e-5, abs_tol=1e-5):
            continue

        if effective_col is not None:
            if effective_col >= len(toks):
                raise ValueError(f"column index {effective_col} out of range in OBS_NODE row: {line!r}")
            extracted.append(parse_decimal_token(toks[effective_col]))
        else:
            # Default to first non-time token
            if len(toks) > 1:
                extracted.append(parse_decimal_token(toks[1]))

        if target_time is not None:
            break

    if not extracted:
        raise ValueError(f"no matching OBS_NODE data found for time={target_time}")
    return extracted


def extract_output_values(deck_dir: Path, output_spec: Dict[str, Any]) -> List[Decimal]:
    """Extract compared values from a deck's output directory according to output_spec."""
    filename = output_spec.get("file") or output_spec.get("output_file")
    if not filename:
        raise ValueError("output spec missing 'file'")

    target_file = None
    for cand in [deck_dir / filename, deck_dir / filename.upper(), deck_dir / filename.lower()]:
        if cand.is_file():
            target_file = cand
            break
    if target_file is None:
        # Check subdirectories (e.g. deck_dir might contain outputs/ or similar)
        for f in deck_dir.rglob("*"):
            if f.is_file() and f.name.lower() == filename.lower():
                target_file = f
                break

    if target_file is None:
        raise FileNotFoundError(f"output file {filename!r} not found in {deck_dir}")

    lines = target_file.read_text(encoding="utf-8", errors="strict").splitlines()

    # Determine extraction parameters from manifest
    target_node = output_spec.get("node")
    if isinstance(target_node, list) and len(target_node) == 1:
        target_node = target_node[0]

    target_time = output_spec.get("time")
    if target_time is not None:
        target_time = float(target_time)

    # Column can be specified by index (1-based or 0-based), or variable name
    col_idx = output_spec.get("column_index")
    if col_idx is None and "column" in output_spec:
        col_idx = int(output_spec["column"])
    # If column was given 1-based in manifest, convert to 0-based
    one_based = output_spec.get("one_based", True)
    if col_idx is not None and one_based:
        col_idx = col_idx - 1

    header_offset = int(output_spec.get("header_offset", 0))
    col_offset = int(output_spec.get("col_offset", output_spec.get("column_offset", 0)))

    fn_upper = filename.upper()
    if "NOD_INF" in fn_upper or "PROFILE" in fn_upper:
        return extract_from_nod_inf(
            lines,
            target_node=target_node,
            target_time=target_time,
            col_index=col_idx,
            header_offset=header_offset,
            col_offset=col_offset,
        )
    elif "OBS_NODE" in fn_upper:
        return extract_from_obs_node(
            lines,
            target_node_idx=target_node,
            target_time=target_time,
            col_index=col_idx,
            header_offset=header_offset,
            col_offset=col_offset,
        )
    else:
        # Generic tabular extractor
        effective_col = col_idx + col_offset if col_idx is not None else 0
        data_lines = lines[header_offset:]
        vals = []
        for l in data_lines:
            toks = parse_tokens(l)
            if not toks:
                continue
            if effective_col < len(toks):
                try:
                    vals.append(parse_decimal_token(toks[effective_col]))
                except InvalidOperation:
                    continue
        if not vals:
            raise ValueError(f"could not extract values from {filename}")
        return vals


def classify_pair(
    values_a: List[Decimal],
    values_b: List[Decimal],
    is_defect_pair: bool,
    relative_threshold: float = DEFAULT_RELATIVE_THRESHOLD,
) -> Tuple[str, Dict[str, Any]]:
    """Classify a pair comparison as 'identical' or 'different'.

    For defect pair: exact match is expected if defect is present.
    For control pair: must differ by more than relative_threshold.
    """
    if len(values_a) != len(values_b):
        return "different", {
            "length_a": len(values_a),
            "length_b": len(values_b),
            "reason": "length mismatch",
        }

    exact_equal = (values_a == values_b)
    if is_defect_pair:
        if exact_equal:
            return "identical", {"diff": 0.0, "exact": True}
        else:
            max_rel_diff = 0.0
            for a, b in zip(values_a, values_b):
                denom = max(abs(a), abs(b))
                if denom > 0:
                    diff = float(abs(a - b) / denom)
                else:
                    diff = float(abs(a - b))
                if diff > max_rel_diff:
                    max_rel_diff = diff
            return "different", {"max_relative_diff": max_rel_diff, "exact": False}
    else:
        # Control pair: compare against relative_threshold
        if exact_equal:
            return "identical", {"diff": 0.0, "exact": True}
        max_rel_diff = 0.0
        for a, b in zip(values_a, values_b):
            denom = max(abs(a), abs(b))
            if denom > 0:
                diff = float(abs(a - b) / denom)
            else:
                diff = float(abs(a - b))
            if diff > max_rel_diff:
                max_rel_diff = diff

        if max_rel_diff > relative_threshold:
            return "different", {"max_relative_diff": max_rel_diff, "threshold": relative_threshold}
        else:
            return "identical", {"max_relative_diff": max_rel_diff, "threshold": relative_threshold}


def evaluate_entry(
    entry: Dict[str, Any],
    outputs_dir: Path,
    relative_threshold: float = DEFAULT_RELATIVE_THRESHOLD,
) -> Dict[str, Any]:
    """Evaluate probe results for a single SD register entry."""
    sd_id = entry.get("sd_id") or entry.get("id")
    output_spec = entry.get("compared_output", {})
    pairs = entry.get("pairs", {})

    result: Dict[str, Any] = {
        "sd_id": sd_id,
        "verdict": "inconclusive",
        "defect_pair": None,
        "control_pair": None,
        "error": None,
    }

    defect_pair_info = pairs.get("defect")
    control_pair_info = pairs.get("control")

    if not defect_pair_info or not control_pair_info:
        result["error"] = "manifest entry missing defect or control pair declaration"
        return result

    # Resolve paths for defect pair decks
    try:
        deck_a_path = outputs_dir / defect_pair_info["deck_a"]
        deck_b_path = outputs_dir / defect_pair_info["deck_b"]
        vals_def_a = extract_output_values(deck_a_path, output_spec)
        vals_def_b = extract_output_values(deck_b_path, output_spec)
        def_class, def_details = classify_pair(
            vals_def_a, vals_def_b, is_defect_pair=True, relative_threshold=relative_threshold
        )
        result["defect_pair"] = {
            "classification": def_class,
            "details": def_details,
            "values_a": [str(v) for v in vals_def_a],
            "values_b": [str(v) for v in vals_def_b],
        }
    except Exception as err:
        result["defect_pair"] = {"classification": "error", "error": str(err)}
        result["error"] = f"defect pair error: {err}"
        return result

    # Resolve paths for control pair decks
    try:
        ctrl_a_path = outputs_dir / control_pair_info["deck_a"]
        ctrl_b_path = outputs_dir / control_pair_info["deck_b"]
        vals_ctrl_a = extract_output_values(ctrl_a_path, output_spec)
        vals_ctrl_b = extract_output_values(ctrl_b_path, output_spec)
        ctrl_class, ctrl_details = classify_pair(
            vals_ctrl_a, vals_ctrl_b, is_defect_pair=False, relative_threshold=relative_threshold
        )
        result["control_pair"] = {
            "classification": ctrl_class,
            "details": ctrl_details,
            "values_a": [str(v) for v in vals_ctrl_a],
            "values_b": [str(v) for v in vals_ctrl_b],
        }
    except Exception as err:
        result["control_pair"] = {"classification": "error", "error": str(err)}
        result["error"] = f"control pair error: {err}"
        return result

    # Determine verdict
    # defect present: defect pair identical AND control pair different
    # defect fixed: both differ
    # inconclusive: control identical, or values missing/unparsable
    if ctrl_class != "different":
        result["verdict"] = "inconclusive"
        result["reason"] = "control pair output was identical (insufficient sensitivity or non-responsive run)"
    elif def_class == "identical":
        result["verdict"] = "defect present"
    elif def_class == "different":
        result["verdict"] = "defect fixed"
    else:
        result["verdict"] = "inconclusive"

    return result


def compare_probes(
    manifest_path: Path,
    outputs_dir: Path,
    sd_filter: Optional[str] = None,
    relative_threshold: float = DEFAULT_RELATIVE_THRESHOLD,
) -> Dict[str, Any]:
    """Run probe comparison across all entries in manifest."""
    manifest_text = manifest_path.read_text(encoding="utf-8")
    manifest = json.loads(manifest_text)

    entries = manifest.get("entries", [])
    if isinstance(manifest, list):
        entries = manifest

    results = []
    for entry in entries:
        sd_id = entry.get("sd_id") or entry.get("id")
        if sd_filter and sd_id != sd_filter:
            continue
        eval_res = evaluate_entry(entry, outputs_dir, relative_threshold=relative_threshold)
        results.append(eval_res)

    return {
        "manifest": str(manifest_path),
        "outputs_dir": str(outputs_dir),
        "results": results,
    }


def format_table(summary: Dict[str, Any]) -> str:
    """Format evaluation results as a human-readable table."""
    headers = ["SD ID", "Defect Pair", "Control Pair", "Verdict", "Notes/Error"]
    widths = [10, 15, 15, 18, 40]
    sep_line = "+" + "+".join("-" * (w + 2) for w in widths) + "+"

    lines = [sep_line]
    row_fmt = "| " + " | ".join(f"{{:<{w}}}" for w in widths) + " |"
    lines.append(row_fmt.format(*headers))
    lines.append(sep_line)

    for item in summary.get("results", []):
        sd_id = item.get("sd_id", "N/A")
        def_pair = (item.get("defect_pair") or {}).get("classification", "error")
        ctrl_pair = (item.get("control_pair") or {}).get("classification", "error")
        verdict = item.get("verdict", "inconclusive")
        note = item.get("error") or item.get("reason") or ""
        lines.append(row_fmt.format(sd_id, def_pair, ctrl_pair, verdict, note[:40]))

    lines.append(sep_line)
    return "\n".join(lines)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Compare HYDRUS defect probe output pairs and report defect status verdicts."
    )
    parser.add_argument("--manifest", required=True, type=Path, help="Path to defect probes MANIFEST.json")
    parser.add_argument("--outputs", required=True, type=Path, help="Directory containing run outputs arranged by deck")
    parser.add_argument("--sd", required=False, type=str, default=None, help="Optional SD-NNN filter")
    parser.add_argument("--json-out", required=False, type=Path, default=None, help="Optional output JSON path")
    parser.add_argument(
        "--threshold",
        required=False,
        type=float,
        default=DEFAULT_RELATIVE_THRESHOLD,
        help="Relative difference threshold for control pair (default: 1e-4)",
    )
    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if not args.manifest.is_file():
        sys.stderr.write(f"Error: manifest file not found: {args.manifest}\n")
        return 1
    if not args.outputs.is_dir():
        sys.stderr.write(f"Error: outputs directory not found: {args.outputs}\n")
        return 1

    try:
        report = compare_probes(
            args.manifest,
            args.outputs,
            sd_filter=args.sd,
            relative_threshold=args.threshold,
        )
    except Exception as exc:
        sys.stderr.write(f"Error executing comparison: {exc}\n")
        return 2

    # Print table to stdout
    table_text = format_table(report)
    print(table_text)

    # Write JSON output if requested, otherwise print JSON to stdout if json flag
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(f"\nWrote JSON report to: {args.json_out}")

    return 0


if __name__ == "__main__":
    sys.exit(main())

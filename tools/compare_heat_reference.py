#!/usr/bin/env python3
"""Compare heat candidate temperatures with pinned-Fortran heat fixtures (24.5.2).

Reads each fixture JSON under reference/heat_fortran/fixtures/ and the candidate JSON
`<case_id>.json` in --candidate-dir (written by crates/hydrus-core/tests/heat_fortran_parity.rs).
Two verdicts are printed for every (fixture, time):

- strict: max |candidate - fixture| over nodes, with the count of nodes above the approved
  TempN tolerance (2e-4 degC, planning/master/M6_HEAT_CONTRACT.md `## Tolerances`).
- rounded: the candidate is rounded to 2 decimals (ties upward), and the verdict passes when
  |round(candidate, 2) - fixture| <= 0.01 degC for every node. This is an output-rounding
  comparison, separately labelled. It is not the 2e-4 tolerance.

The parity pass criterion is the rounded verdict. The exit status is 1 when any rounded verdict
fails. Use --detail to list every node above the strict tolerance.
"""

import argparse
import json
import math
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE_DIR = REPO_ROOT / "reference/heat_fortran/fixtures"
STRICT_TOLERANCE = 2e-4
ROUNDED_TOLERANCE = 0.01
ROUNDED_EPSILON = 1e-9
NODE_COUNT = 41
HEADER = (
    f"{'fixture':<34} {'field':<6} {'time':>5} {'max_strict':>10} {'fail':>5} "
    f"{'max_round':>10} rounded"
)


def round2(value):
    """Round to two decimals, ties upward (same rule as the Rust test)."""
    return math.floor(value * 100.0 + 0.5) / 100.0


def load_json(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def _matrix(value, label):
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label}: expected a non-empty list of rows")
    for row in value:
        if not isinstance(row, list) or len(row) != NODE_COUNT:
            raise ValueError(f"{label}: each row needs {NODE_COUNT} nodes")
        for number in row:
            if not isinstance(number, (int, float)) or not math.isfinite(number):
                raise ValueError(f"{label}: non-finite or non-numeric value")
    return value


def validate_fixture(fixture):
    if fixture.get("source_matched") is not True:
        raise ValueError("fixture is not source_matched")
    if fixture.get("temperature_unit") != "C":
        raise ValueError("fixture temperature_unit must be C")
    times = fixture["time_points"]
    temps = _matrix(fixture["temperatures"], "fixture temperatures")
    if len(temps) != len(times):
        raise ValueError("fixture rows differ from time_points")
    if len(fixture["node_numbers"]) != NODE_COUNT:
        raise ValueError("fixture node count must be 41")
    return times, temps


def compare_case(fixture, candidate):
    """Return one verdict row per fixture time point."""
    times, reference = validate_fixture(fixture)
    if candidate.get("case_id") != fixture["case_id"]:
        raise ValueError("candidate case_id differs from fixture")
    if candidate["time_points"] != times:
        raise ValueError("candidate time_points differ from fixture")
    rows_candidate = _matrix(candidate["temperatures"], "candidate temperatures")
    if len(rows_candidate) != len(reference):
        raise ValueError("candidate rows differ from fixture")

    rows = []
    for index, time in enumerate(times):
        failing_nodes = []
        max_strict = 0.0
        max_rounded = 0.0
        for node, (value, expected) in enumerate(zip(rows_candidate[index], reference[index])):
            strict = abs(value - expected)
            max_strict = max(max_strict, strict)
            if strict > STRICT_TOLERANCE:
                failing_nodes.append((fixture["node_numbers"][node], value, expected, strict))
            max_rounded = max(max_rounded, abs(round2(value) - expected))
        rounded_pass = max_rounded <= ROUNDED_TOLERANCE + ROUNDED_EPSILON
        rows.append(
            {
                "fixture": fixture["case_id"],
                "time": time,
                "max_strict": max_strict,
                "failing": len(failing_nodes),
                "failing_nodes": failing_nodes,
                "max_rounded": max_rounded,
                "rounded_pass": rounded_pass,
            }
        )
    return rows


def format_row(row):
    verdict = "pass" if row["rounded_pass"] else "fail"
    return (
        f"{row['fixture']:<34} {'TempN':<6} {row['time']:>5.1f} {row['max_strict']:>10.6f} "
        f"{row['failing']:>5d} {row['max_rounded']:>10.6f} {verdict}"
    )


def run(fixture_dir, candidate_dir, detail):
    rows = []
    for fixture_path in sorted(Path(fixture_dir).glob("*.json")):
        fixture = load_json(fixture_path)
        candidate_path = Path(candidate_dir) / f"{fixture['case_id']}.json"
        if not candidate_path.is_file():
            raise FileNotFoundError(f"missing candidate {candidate_path}")
        rows.extend(compare_case(fixture, load_json(candidate_path)))

    lines = [HEADER] + [format_row(row) for row in rows]
    if detail:
        lines.append("")
        lines.append("strict failures (node number, time, candidate, fixture, error):")
        for row in rows:
            for node, value, expected, error in row["failing_nodes"]:
                lines.append(
                    f"  {row['fixture']} t={row['time']:.1f} node={node:>2d} "
                    f"candidate={value:.6f} fixture={expected:.2f} error={error:.6f}"
                )
    return rows, "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-dir", required=True, type=Path)
    parser.add_argument("--fixture-dir", type=Path, default=FIXTURE_DIR)
    parser.add_argument("--detail", action="store_true")
    args = parser.parse_args(argv)
    rows, text = run(args.fixture_dir, args.candidate_dir, args.detail)
    print(text)
    return 0 if all(row["rounded_pass"] for row in rows) else 1


if __name__ == "__main__":
    sys.exit(main())

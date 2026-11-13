#!/usr/bin/env python3
"""Heat-boundary (criteria v3) comparator for the vapor reference (26.4.2).

Thin layer over tools/compare_vapor_reference.py (25.7.2). It selects the
vapor_heatbc_* cases and heatbc_* discrimination checks, and reads every limit
from the criteria file. No numeric limit is defined here.

  python3 -m tools.compare_vapor_heat_boundary discrimination --output OUT.json
  python3 -m tools.compare_vapor_heat_boundary trajectory --fixture FX.json --candidate CAND.json --output OUT.json
"""

import argparse
import sys
from pathlib import Path

import tools.compare_vapor_reference as cmp

HEATBC_PREFIX = "vapor_heatbc_"
RESULT_SCHEMA = "vapor-heat-boundary-result-v1"


def discrimination(criteria, crit_sha, fixtures):
    """Heat-boundary discrimination: only heatbc_* entries of criteria v3; verdict over those alone."""
    heat = {n: s for n, s in criteria["discrimination"].items() if n.startswith("heatbc_")}
    if not heat:
        raise cmp.Reject("discrimination: no heatbc_* entries in criteria")
    sub = dict(criteria, discrimination=heat)
    full = cmp.discrimination(sub, crit_sha, fixtures)
    cases = sorted({k for k in fixtures if k.startswith(HEATBC_PREFIX)})
    return {"schema_version": RESULT_SCHEMA, "mode": "discrimination", "criteria_sha256": crit_sha,
            "cases": cases, "checks": full["checks"], "verdict": full["verdict"]}


def trajectory(criteria, crit_sha, fixture, fix_sha, cand):
    """Per-field, per-row verdicts (err/bound) for one heat-boundary trajectory fixture."""
    if not fixture["case_id"].startswith(HEATBC_PREFIX):
        raise cmp.Reject(f"trajectory: {fixture['case_id']} is not a heat-boundary case")
    res = cmp.compare_trajectory(criteria, crit_sha, fixture, fix_sha, cand)
    res["schema_version"] = RESULT_SCHEMA
    res["mode"] = "trajectory"
    return res


def run(args):
    criteria, crit_sha = cmp.load_criteria(args.criteria)
    if args.command == "discrimination":
        result = discrimination(criteria, crit_sha, cmp.load_fixture_dir(args.fixture_dir))
    else:
        fixture, fix_sha = cmp.load_fixture(args.fixture)
        result = trajectory(criteria, crit_sha, fixture, fix_sha, cmp.load_json(args.candidate))
    if Path(args.output).exists():
        raise cmp.Reject(f"refusing to overwrite {args.output}")
    cmp.write_new(args.output, result)
    for c in result.get("checks", []):
        print(f"{c['name']:<32} {c.get('case_id', ''):<28} {c.get('field', ''):<18} {c['verdict']}")
    print(f"verdict: {result['verdict']}")
    return 0 if result["verdict"] == "PASS" else 1


def build_parser():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("discrimination", "trajectory"):
        p = sub.add_parser(name)
        p.add_argument("--criteria", default=str(cmp.DEFAULT_CRITERIA))
        p.add_argument("--output", required=True, help="new JSON result path (must not exist)")
        if name == "discrimination":
            p.add_argument("--fixture-dir", default=str(cmp.DEFAULT_FIXTURE_DIR))
        else:
            p.add_argument("--fixture", required=True)
            p.add_argument("--candidate", required=True)
    return parser


def main(argv=None):
    args = build_parser().parse_args(argv)
    try:
        return run(args)
    except cmp.Reject as error:
        print(f"REJECT: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())

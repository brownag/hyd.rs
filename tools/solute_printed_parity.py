"""Compare retained Fortran concentration prints with validated Rust observations.

Concentration and theta profile values have established one-to-one semantic
mappings here. Other fields remain unavailable until their mapping is proven.
"""

import argparse
import json
import re
import sys
from decimal import Decimal, InvalidOperation, localcontext
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if __package__ in (None, ""):
    sys.path.insert(0, str(ROOT))

from tools import solute_printed_parity_inventory as inventory
from tools import solute_verification_metrics as metrics

CONCENTRATION_FORMAT = "e12.4"
COORDINATE_FORMAT = "f10.4"
TIME_FORMAT = "f14.4"
RAW_NODE_FIELDS = {
    "head": (2, "f11.3"),
    # _node_profiles authenticates these formats for head, K, and flux;
    # inactive capacity/sink are only checked for zero, so their format is unknown.
    "moisture_capacity": (5, None),
    "hydraulic_conductivity": (4, "e12.4"),
    "water_flux": (6, "e12.4"),
    "sink": (7, None),
    "kappa": (8, None),
    "v_over_ks_top": (9, None),
    "temperature": (10, None),
}
UNAVAILABLE_REASONS = {
    "head": "Rust observations do not retain a mapped nodal head field",
    "moisture_capacity": "Rust observations do not retain the Fortran C field",
    "hydraulic_conductivity": "Rust observations do not retain a mapped nodal K field",
    "water_flux": "Rust face fluxes do not establish the Fortran nodal Flux mapping",
    "sink": "Rust observations do not retain a mapped nodal sink field",
    "kappa": "Rust observations do not retain a mapped nodal Kappa field",
    "v_over_ks_top": "Rust observations do not retain a mapped v/KsTop field",
    "temperature": "Rust observations do not retain a mapped temperature field",
    "Sorb": "Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage",
}


def _decimal(token):
    try:
        return Decimal(token.replace("D", "E").replace("d", "e"))
    except InvalidOperation as error:
        raise ValueError(f"invalid numeric token {token!r}") from error


def _half_width(token, fmt):
    """Exact decimal half-unit for Fortran E/F tokens, including tiny exponents."""
    match = re.fullmatch(r"([eEfF])(\d+)\.(\d+)", fmt)
    if not match:
        raise ValueError(f"unsupported printed format {fmt!r}")
    kind, _width, digits_text = match.groups()
    digits = int(digits_text)
    value = _decimal(token)
    if kind.lower() == "e":
        exp_match = re.search(r"[EeDd]([+-]?\d+)\Z", token)
        if not exp_match or len(re.split(r"[EeDd]", token)[0].partition(".")[2]) != digits:
            raise ValueError(f"token {token!r} does not match {fmt}")
        unit_exponent = int(exp_match.group(1)) - digits
    else:
        if re.search(r"[EeDd]", token) or len(token.partition(".")[2]) != digits:
            raise ValueError(f"token {token!r} does not match {fmt}")
        unit_exponent = -digits
    return Decimal(5).scaleb(unit_exponent - 1)


def _token_records(path, species_count):
    lines = path.read_text(encoding="utf-8", errors="strict").splitlines()
    blocks = []
    current = None
    for line in lines:
        text = line.strip()
        if text.startswith("Time:"):
            current = {"time_token": text.partition(":")[2].strip(), "rows": []}
            blocks.append(current)
        elif current is not None and text and not text.startswith(("[", "Node")) and text != "end":
            fields = text.split()
            if len(fields) == 11 + 2 * species_count:
                current["rows"].append(fields)
    return blocks


def _artifact(run, name):
    entries = [item for item in run["artifacts"] if item["name"] == name]
    if len(entries) != 1:
        raise ValueError(f"{run['manifest_path']}: expected one {name} artifact")
    return ROOT / entries[0]["path"]


def _rust_observations(pair, cases_path):
    run = pair["rust"]
    manifest_path = ROOT / run["manifest_path"]
    data_path = _artifact(run, "observations.json")
    return metrics.load_observations(data_path, manifest_path, cases_path)


def _fortran_context(pair, cases_path):
    run = pair["fortran"]
    return metrics._context(ROOT / run["manifest_path"], cases_path)


def _record(field, status, token, rust_value, fmt, location, reason=None):
    with localcontext() as context:
        context.prec = 1200
        printed = _decimal(token)
        half_width = _half_width(token, fmt)
        rust_decimal = Decimal.from_float(float(rust_value))
        lower, upper = printed - half_width, printed + half_width
        passed = lower <= rust_decimal <= upper
        signed_difference = rust_decimal - printed
        low_information = rust_decimal.is_zero() or half_width >= abs(rust_decimal)
    if status == "comparable":
        result_status = "pass" if passed else "fail"
        result_reason = None if passed else "Rust binary64 value lies outside the Fortran print interval"
    else:
        result_status, result_reason = "unavailable", reason
    return {
        **location,
        "field": field,
        "status": result_status,
        "fortran_token": token,
        "fortran_value": str(printed),
        "rust_value": float(rust_value),
        "signed_difference_rust_minus_fortran": str(signed_difference),
        "interval": {"lower": str(lower), "upper": str(upper), "half_width": str(half_width)},
        "source_format": fmt,
        "low_information": low_information,
        "reason": result_reason,
    }


def _unavailable_record(field, token, fmt, location, reason):
    printed = _decimal(token)
    if fmt is None:
        interval = None
    else:
        with localcontext() as context:
            context.prec = 1200
            half_width = _half_width(token, fmt)
            lower, upper = printed - half_width, printed + half_width
        interval = {"lower": str(lower), "upper": str(upper), "half_width": str(half_width)}
    return {
        **location, "field": field, "status": "unavailable", "fortran_token": token,
        "fortran_value": str(printed), "rust_value": None,
        "signed_difference_rust_minus_fortran": None,
        "interval": interval, "source_format": fmt, "low_information": None, "reason": reason,
    }


def _validate_node_mapping(fields, expected_node, rust_coordinate):
    if int(fields[0]) != expected_node:
        raise ValueError(f"node order mismatch: expected {expected_node}, found {fields[0]}")
    token = fields[1]
    with localcontext() as context:
        context.prec = 1200
        coordinate = -_decimal(token)
        half_width = _half_width(token, COORDINATE_FORMAT)
        lower, upper = coordinate - half_width, coordinate + half_width
        matched = lower <= Decimal.from_float(float(rust_coordinate)) <= upper
    if not matched:
        raise ValueError(f"coordinate mapping mismatch at node {expected_node}")
    return token


def compare_pair(pair, cases_path):
    """Compare concentration and theta after both sides pass retained validation."""
    context = _fortran_context(pair, cases_path)
    if context["manifest"]["solver"] != "fortran":
        raise ValueError("expected authenticated Fortran manifest")
    # Run the established full raw-output checks before consuming raw tokens.
    metrics._parse_fortran(context)
    rust = _rust_observations(pair, cases_path)
    case, launch = context["case"], context["launch"]
    files = context["paths"]
    blocks = _token_records(files["NOD_INF.OUT"], len(case["species"]))
    if len(blocks) != len(rust["profiles"]):
        raise ValueError(f"{pair['case_id']}: profile schedule mismatch")
    records = []
    for profile_index, (block, rust_profile) in enumerate(zip(blocks, rust["profiles"])):
        if len(block["rows"]) != launch["nodes"]:
            raise ValueError(f"{pair['case_id']}: incomplete raw node block")
        time = rust_profile["time"]
        _half_width(block["time_token"], TIME_FORMAT)
        for node_index, fields in enumerate(block["rows"]):
            coordinate_token = _validate_node_mapping(
                fields, node_index + 1, rust["coordinates"][node_index]
            )
            # The validator proves Fortran's negative depth and Rust's positive
            # coordinate represent the same physical orientation.
            location = {
                "case_id": pair["case_id"], "coordinate": rust["coordinates"][node_index],
                "fortran_coordinate_token": coordinate_token, "time": time,
                "fortran_time_token": block["time_token"], "profile_index": profile_index,
                "node_index": node_index,
            }
            records.append(_record(
                "theta", "comparable", fields[3], rust_profile["theta"][node_index], "f6.4",
                location,
            ))
            for field, (column, fmt) in RAW_NODE_FIELDS.items():
                records.append(_unavailable_record(
                    field, fields[column], fmt, location, UNAVAILABLE_REASONS[field]
                ))
            for species_index, species in enumerate(rust["species"]):
                token = fields[11 + species_index]
                rust_value = rust_profile["concentration"][species_index][node_index]
                species_location = {**location, "species": species}
                records.append(_record("concentration", "comparable", token, rust_value,
                                       CONCENTRATION_FORMAT, species_location))
                sorb_token = fields[11 + len(rust["species"]) + species_index]
                records.append(_unavailable_record(
                    "Sorb", sorb_token, CONCENTRATION_FORMAT, species_location,
                    UNAVAILABLE_REASONS["Sorb"],
                ))
    return {
        "case_id": pair["case_id"],
        "fortran_manifest_sha256": pair["fortran"]["manifest_sha256"],
        "rust_manifest_sha256": pair["rust"]["manifest_sha256"],
        "records": records,
        "unavailable_fields": sorted(UNAVAILABLE_REASONS),
    }


def compare_inventory(cases_path, inventory_path):
    supplied = json.loads(Path(inventory_path).read_text(encoding="utf-8"))
    current = inventory.build_inventory()
    if supplied != current:
        raise ValueError("inventory differs from currently authenticated retained artifacts")
    if supplied.get("canonical_pair_count") != 6:
        raise ValueError("expected exactly six authenticated canonical pairs")
    results = [compare_pair(pair, Path(cases_path)) for pair in supplied["canonical_pairs"]]
    return {"schema_version": "solute-printed-parity-v1", "cases_sha256": supplied["cases_sha256"],
            "inventory_sha256": inventory.sha256(Path(inventory_path)), "comparisons": results}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", required=True)
    parser.add_argument("--inventory", required=True)
    parser.add_argument("--output-root", required=True)
    args = parser.parse_args(argv)
    output_root = ROOT / args.output_root
    output_root.mkdir(parents=True, exist_ok=True)
    output = output_root / "printed_parity.json"
    if output.exists():
        raise FileExistsError(f"refusing to overwrite {output}")
    result = compare_inventory(ROOT / args.cases, ROOT / args.inventory)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(output.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

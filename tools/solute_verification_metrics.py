#!/usr/bin/env python3
"""Strict observation loading and independent solute verification metrics.

Public loaders return a dict with the frozen observation keys and private validated
case/launch/rounding context. compute_metrics rejects plain or subsequently mutated
observations. It never treats solver-reported balances as analytical truth.

Reference v1 has exact keys schema_version, coordinates, species, profiles,
continuum, uncertainty. Profiles ({time, concentration}, S by N) cover zero and
analysis times. Continuum records cover every diagnostic profile time and contain
S-valued dissolved, equilibrium_sorbed, total, production_cumulative,
loss_cumulative and S by 2 inward endpoint_flux/boundary_cumulative. Uncertainty
has profiles and continuum of identical shapes containing absolute bounds.
"""

import copy
import hashlib
import json
import math
import os
import re
from datetime import datetime
from pathlib import Path

from tools.check_solute_verification_spec import load_cases, validate

ROOT = Path(__file__).resolve().parents[1]
OBS_KEYS = (
    "schema_version run_id launch_id case_id cases_sha256 case_sha256 request_sha256 "
    "solver units species coordinates profiles steps endpoint_history solver_reported "
    "availability warnings"
)
MANIFEST_KEYS = (
    "schema_version run_id launch_id case_id cases_sha256 case_sha256 request_sha256 "
    "solver precision status command cwd started_utc ended_utc exit_code timeout_seconds "
    "provenance artifacts record_inventory warnings mapping_validation"
)
CONTINUUM_FIELDS = (
    "dissolved",
    "equilibrium_sorbed",
    "total",
    "endpoint_flux",
    "boundary_cumulative",
    "production_cumulative",
    "loss_cumulative",
)
PAIR_FIELDS = {"endpoint_flux", "boundary_cumulative"}
FORMATS = {
    "concentration": "e12.4",
    "sorbed": "e12.4",
    "coordinate": "f10.4",
    "theta": "f6.4",
    "head": "f11.3",
    "nodal_flux": "e12.4",
    "solute_flux": "e13.5",
    "profile_time": "f14.4",
    "run_time": "e15.7",
    "run_dt": "e13.5",
    "endpoint_time": "f14.4",
    "endpoint_concentration": "e12.4",
}
NUMBER = re.compile(r"[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[EeDd][+-]?\d+)?\Z")
SAFE_ID = re.compile(r"[A-Za-z0-9_.-]+\Z")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def keys(value, expected, label):
    require(isinstance(value, dict) and set(value) == set(expected.split()), f"{label}: wrong keys")


def finite(value, label="number", nonnegative=False):
    require(type(value) in (int, float) and math.isfinite(value), f"{label}: nonfinite number")
    require(not nonnegative or value >= 0, f"{label}: negative value")
    return value


def vector(value, size, label, nonnegative=False):
    require(isinstance(value, list) and len(value) == size, f"{label}: wrong length")
    for item in value:
        finite(item, label, nonnegative)


def matrix(value, rows, columns, label, nonnegative=False):
    require(isinstance(value, list) and len(value) == rows, f"{label}: wrong species count")
    for row in value:
        vector(row, columns, label, nonnegative)


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True, allow_nan=False
    ).encode("ascii")


def digest_file(path):
    with Path(path).open("rb") as stream:
        digest = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def json_load(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, f"duplicate JSON key {key}")
            result[key] = value
        return result

    def reject(value):
        raise ValueError(f"nonfinite JSON token {value}")

    return json.loads(
        Path(path).read_text(encoding="utf-8"), object_pairs_hook=unique, parse_constant=reject
    )


def contained(root, relative):
    require(
        isinstance(relative, str) and relative and not Path(relative).is_absolute(),
        "artifact path must be relative",
    )
    require(".." not in Path(relative).parts, "artifact path traversal")
    path = root / relative
    require(
        not any(p.is_symlink() for p in (path, *path.parents) if p != root.parent),
        "symlink artifact path",
    )
    require(
        path.resolve() == root.resolve() or root.resolve() in path.resolve().parents,
        "artifact path escapes run",
    )
    return path


def hash_match(path, expected):
    require(
        isinstance(expected, str) and re.fullmatch(r"[0-9a-f]{64}", expected), "invalid SHA-256"
    )
    require(path.is_file() and digest_file(path) == expected, f"hash mismatch: {path}")


def numeric(token):
    require(bool(NUMBER.fullmatch(token)), f"malformed numeric token {token!r}")
    return finite(float(token.replace("D", "E").replace("d", "e")), token)


def rounding_half_width(token, fmt):
    """Return half a printed unit, including Fortran's 0.xxxx E12.4 notation."""
    numeric(token)
    match = re.fullmatch(r"([eEfF])\d+\.(\d+)", fmt)
    require(match is not None, f"unsupported field format {fmt}")
    kind, digits = match.groups()
    normalized = token.upper().replace("D", "E")
    if kind.lower() == "e":
        require("E" in normalized, f"expected exponent token for {fmt}")
        mantissa, exponent = normalized.split("E")
        require(len(mantissa.partition(".")[2]) == int(digits), "wrong printed precision")
        return 0.5 * 10.0 ** (int(exponent) - int(digits))
    require("E" not in normalized, f"expected fixed token for {fmt}")
    require(len(normalized.partition(".")[2]) == int(digits), "wrong printed precision")
    return 0.5 * 10.0 ** -int(digits)


def printed_match(token, expected, fmt, label):
    width = rounding_half_width(token, fmt)
    require(
        abs(numeric(token) - expected) <= width + 8 * math.ulp(float(expected)),
        f"{label}: printed value does not contain expected value",
    )
    return width


def schedule(case, launch):
    if launch["role"] == "bridge":
        profiles = [0.0, 50.0, 100.0, 200.0]
    else:
        profiles = [
            i * launch["sampling_interval"]
            for i in range(round(case["time_end"] / launch["sampling_interval"]) + 1)
        ]
    steps = [i * launch["dt"] for i in range(1, launch["max_steps"] + 1)]
    return profiles, steps


def _artifact(paths, name):
    found = {
        path for key, path in paths.items() if key == name or path.name.split("\\")[-1] == name
    }
    require(len(found) == 1, f"missing or ambiguous artifact {name}")
    return found.pop()


def _request(path, manifest, case, launch, book):
    text = path.read_text(encoding="ascii")
    require(text.endswith("\n"), "request missing final newline")
    request = {}
    for line in text.splitlines():
        require(line.count("=") == 1 and not any(c.isspace() for c in line), "bad request line")
        name, value = line.split("=")
        require(name not in request and value, "duplicate or empty request field")
        request[name] = value
    count = len(case["species"])
    expected_keys = (
        "schema run_id launch_id case_id cases_sha256 case_sha256 nodes species_count length "
        "theta rho q dispersion dt time_end sampling_interval analysis_times species kd rates "
        "inlet_concentration " + " ".join(f"initial_{j}" for j in range(count))
    )
    keys(request, expected_keys, "request")
    require(request["schema"] == "flat-ascii-v1", "request schema")
    for key in ("run_id", "launch_id", "case_id", "cases_sha256", "case_sha256"):
        require(request[key] == manifest[key], f"request {key} mismatch")
    require(request["species"].split(",") == [s["name"] for s in case["species"]], "species order")
    for key, expected in {"nodes": launch["nodes"], "species_count": count}.items():
        require(request[key] == str(expected), f"request {key} mismatch")
    for key in ("length", "theta", "rho", "q", "dispersion", "time_end"):
        require(numeric(request[key]) == case[key], f"request {key} mismatch")
    for key in ("dt", "sampling_interval"):
        require(numeric(request[key]) == launch[key], f"request {key} mismatch")
    vectors = {
        "analysis_times": launch["analysis_times"],
        "kd": [s["kd"] for s in case["species"]],
        "rates": [s["rate"] for s in case["species"]],
        "inlet_concentration": case["boundary"]["inlet_concentration"],
    }
    for key, expected in vectors.items():
        require([numeric(t) for t in request[key].split(",")] == expected, f"request {key}")
    initial = [[numeric(t) for t in request[f"initial_{j}"].split(",")] for j in range(count)]
    matrix(initial, count, launch["nodes"], "request initial")
    expected_initial = _initial_arrays(case, launch["nodes"])
    for actual, expected in zip(initial, expected_initial):
        require(
            all(math.isclose(a, b, rel_tol=2e-14, abs_tol=2e-14) for a, b in zip(actual, expected)),
            "request mathematical initial condition mismatch",
        )
    return initial


def _initial_arrays(case, nodes):
    initial = case["initial"]
    if initial["kind"] == "uniform":
        return [[value] * nodes for value in initial["values"]]
    coordinates = [i * case["length"] / (nodes - 1) for i in range(nodes)]
    if initial["kind"] == "cosine":
        return [
            [
                initial["mean"]
                + initial["amplitude"] * math.cos(initial["mode"] * math.pi * s / case["length"])
                for s in coordinates
            ]
        ]
    require(initial["kind"] == "steady_plus_mode", "unsupported initial condition")
    velocity = case["q"] / case["theta"]
    dispersion = case["dispersion"]
    retardation = 1 + case["rho"] * case["species"][0]["kd"] / case["theta"]
    z = retardation * case["species"][0]["rate"]
    root = math.sqrt(velocity * velocity + 4 * dispersion * z)
    plus = (velocity + root) / (2 * dispersion)
    minus = -2 * z / (velocity + root)
    ratio = minus / plus
    amplitude = velocity / (
        dispersion * plus * (1 - ratio * ratio * math.exp((minus - plus) * case["length"]))
    )
    return [
        [
            amplitude
            * (
                math.exp(minus * s)
                - ratio * math.exp(minus * case["length"] + plus * (s - case["length"]))
            )
            + initial["amplitude"]
            * math.exp(velocity / (2 * dispersion) * (s - case["length"] / 2))
            * math.cos(initial["root"] * (s - case["length"] / 2))
            for s in coordinates
        ]
    ]


def _context(manifest_path, cases_path):
    manifest_path = Path(manifest_path)
    root = manifest_path.parent.resolve()
    book = validate(load_cases(Path(cases_path)))
    manifest = json_load(manifest_path)
    keys(manifest, MANIFEST_KEYS, "manifest")
    require(manifest["schema_version"] == "solute-manifest-v1", "manifest schema")
    for key in ("run_id", "launch_id", "case_id"):
        require(
            isinstance(manifest[key], str) and SAFE_ID.fullmatch(manifest[key]), "unsafe identity"
        )
    require(manifest["cases_sha256"] == digest_file(cases_path), "casebook hash mismatch")
    candidates = [row for row in book["launches"] if row["id"] == manifest["launch_id"]]
    require(len(candidates) == 1, "unknown launch")
    launch = candidates[0]
    case = next(c for c in book["cases"] if c["id"] == launch["case_id"])
    require(
        manifest["case_id"] == case["id"]
        and manifest["case_sha256"] == digest_bytes(canonical(case)),
        "case hash mismatch",
    )
    require(
        manifest["solver"] == launch["solver"] and manifest["precision"] == launch["precision"],
        "solver identity mismatch",
    )
    require(
        manifest["status"] in ("raw_complete", "complete") and manifest["exit_code"] == 0,
        "run is not successfully complete",
    )
    require(manifest["timeout_seconds"] == launch["timeout_seconds"], "timeout mismatch")
    require(
        isinstance(manifest["command"], list)
        and manifest["command"]
        and all(isinstance(x, str) and x for x in manifest["command"]),
        "missing command",
    )
    require(isinstance(manifest["cwd"], str) and Path(manifest["cwd"]).is_absolute(), "missing cwd")
    try:
        start = datetime.fromisoformat(manifest["started_utc"].replace("Z", "+00:00"))
        end = datetime.fromisoformat(manifest["ended_utc"].replace("Z", "+00:00"))
        require(
            start.tzinfo is not None and end.tzinfo is not None and end >= start, "bad run dates"
        )
    except (AttributeError, TypeError) as error:
        raise ValueError("bad run dates") from error
    provenance = manifest["provenance"]
    keys(
        provenance,
        "source_sha256 compiler flags platform executable_sha256 shim_sha256 "
        "adapter_sha256 input_sha256",
        "provenance",
    )
    for key in ("compiler", "platform"):
        require(isinstance(provenance[key], str) and provenance[key].strip(), f"missing {key}")
    require(
        isinstance(provenance["flags"], list)
        and all(isinstance(v, str) for v in provenance["flags"]),
        "bad compiler flags",
    )
    for key in ("executable_sha256", "shim_sha256"):
        value = provenance[key]
        require(
            (key == "shim_sha256" and manifest["solver"] == "rust" and value is None)
            or isinstance(value, str)
            and re.fullmatch(r"[0-9a-f]{64}", value),
            f"bad {key}",
        )
    for group in ("source_sha256", "adapter_sha256", "input_sha256"):
        entries = provenance[group]
        require(isinstance(entries, dict) and entries, f"missing {group}")
        for name, digest in entries.items():
            # Repository identities and retained run-relative copies are both checked.
            local = contained(root, name) if group == "input_sha256" else contained(ROOT, name)
            if not local.is_file() and group != "input_sha256":
                local = contained(root, name)
            hash_match(local, digest)
    source_hashes = provenance["source_sha256"]
    for name, expected in book["provenance"]["source_sha256"].items():
        require(source_hashes.get(name) == expected, f"unreviewed source identity {name}")
    if manifest["solver"] == "fortran":
        source_manifest = json_load(ROOT / "planning/master/SOURCE_MANIFEST.json")
        for name, digest in source_manifest.items():
            if name.startswith("src_reference/") or name == "planning/H1D_Src.zip":
                require(source_hashes.get(name) == digest, f"missing source authority {name}")
        expected_flags = ["-std=legacy", "-ffixed-line-length-none"]
        if manifest["precision"] == "promoted-real":
            expected_flags += ["-fdefault-real-8", "-fdefault-double-8"]
        require(provenance["flags"] == expected_flags, "unreviewed compiler flags")
    paths = {}
    require(isinstance(manifest["artifacts"], dict) and manifest["artifacts"], "missing artifacts")
    for name, artifact in manifest["artifacts"].items():
        contained(root, name)
        keys(artifact, "path sha256 bytes", "artifact")
        path = contained(root, artifact["path"])
        hash_match(path, artifact["sha256"])
        require(
            type(artifact["bytes"]) is int and artifact["bytes"] == path.stat().st_size,
            "artifact byte count mismatch",
        )
        paths[name] = path
    executable_matches = [
        p
        for p in paths.values()
        if os.access(p, os.X_OK) and digest_file(p) == provenance["executable_sha256"]
    ]
    require(
        executable_matches, "executable identity is not bound to a retained executable artifact"
    )
    if manifest["solver"] == "fortran":
        hash_match(_artifact(paths, "gnu_compatibility.FOR"), provenance["shim_sha256"])
    request = _artifact(paths, "request.txt")
    hash_match(request, manifest["request_sha256"])
    for name in ("stdout.txt", "stderr.txt"):
        _artifact(paths, name)
    initial = _request(request, manifest, case, launch, book)
    inventory = manifest["record_inventory"]
    keys(
        inventory,
        "nodes species profile_times step_count endpoint_times terminal_records " "field_formats",
        "record inventory",
    )
    times, steps = schedule(case, launch)
    require(
        inventory["nodes"] == launch["nodes"]
        and inventory["species"] == len(case["species"])
        and inventory["profile_times"] == times
        and inventory["step_count"] == len(steps),
        "record inventory mismatch",
    )
    endpoint_times = (
        None if launch["role"] == "bridge" and launch["solver"] == "fortran" else [0.0] + steps
    )
    require(inventory["endpoint_times"] == endpoint_times, "endpoint inventory mismatch")
    mapping = manifest["mapping_validation"]
    keys(mapping, "status theta_range q_range dispersion_range evidence", "mapping")
    require(mapping["status"] == "passed", "unvalidated physical mapping")
    require(
        isinstance(mapping["evidence"], list)
        and mapping["evidence"]
        and all(isinstance(v, str) and v.strip() for v in mapping["evidence"]),
        "missing mapping evidence",
    )
    for name, expected in (
        ("theta", case["theta"]),
        ("q", case["q"]),
        ("dispersion", case["dispersion"]),
    ):
        bounds = mapping[name + "_range"]
        vector(bounds, 2, name + " range")
        require(bounds[0] <= bounds[1], "reversed mapping range")
        # Ranges report actual sampled/reconstructed values, bounded by printed precision.
        tolerance = 5e-5 * max(1.0, abs(expected)) if launch["solver"] == "fortran" else 1e-12
        require(all(abs(v - expected) <= tolerance for v in bounds), "physical mapping mismatch")
    require(
        isinstance(manifest["warnings"], list)
        and all(isinstance(v, str) for v in manifest["warnings"]),
        "invalid warnings",
    )
    return {
        "root": root,
        "book": book,
        "case": case,
        "launch": launch,
        "manifest": manifest,
        "manifest_path": manifest_path,
        "paths": paths,
        "initial": initial,
    }


def _lines(path, terminal="end"):
    text = path.read_text(encoding="ascii")
    require(text.endswith("\n"), f"{path.name}: truncated final record")
    lines = text.splitlines()
    while lines and not lines[-1].strip():
        lines.pop()
    nonempty = [line.strip() for line in lines if line.strip()]
    require(
        nonempty and (terminal is None or nonempty[-1] == terminal),
        f"{path.name}: missing exact terminal record",
    )
    return lines


def _table(path, header, width):
    lines = _lines(path)
    positions = [i for i, line in enumerate(lines) if " ".join(line.split()) == header]
    require(len(positions) == 1, f"{path.name}: missing/malformed header")
    rows = []
    for line in lines[positions[0] + 1 : -1]:
        fields = line.split()
        if not fields or fields[0].startswith("["):
            continue
        require(len(fields) == width, f"{path.name}: malformed columns")
        rows.append(fields)
    return rows


def _run_rows(path, case, launch):
    header = "TLevel Time dt ItrW ItrC ItCum KodT KodB Converg Peclet Courant"
    rows = _table(path, header, 11)
    _, times = schedule(case, launch)
    require(len(rows) == len(times), "RUN_INF: incomplete step inventory")
    result = []
    for index, (fields, time) in enumerate(zip(rows, times), 1):
        for column in (0, 3, 4, 5, 6, 7):
            require(re.fullmatch(r"[+-]?\d+", fields[column]), "RUN_INF: integer field")
        require(int(fields[0]) == index and fields[8] == "T", "RUN_INF: index/convergence")
        printed_match(fields[1], time, "e15.7", "RUN_INF time")
        printed_match(fields[2], launch["dt"], "e13.5", "RUN_INF dt")
        result.append(
            {
                "index": index,
                "time": time,
                "dt": launch["dt"],
                "iterations": int(fields[4]),
                "peclet": numeric(fields[9]),
                "courant": numeric(fields[10]),
            }
        )
    return result


def _water_rows(path, case, launch):
    header = (
        "Time rTop rRoot vTop vRoot vBot sum(rTop) sum(rRoot) sum(vTop) sum(vRoot) "
        "sum(vBot) hTop hRoot hBot RunOff sum(RunOff) Volume sum(Infil) sum(Evap) "
        "TLevel Cum(WTrans) SnowLayer"
    )
    rows = _table(path, header, 22)
    _, times = schedule(case, launch)
    require(len(rows) == len(times), "T_LEVEL: incomplete step inventory")
    for index, (fields, time) in enumerate(zip(rows, times), 1):
        values = [numeric(t) for t in fields]
        require(values[19] == index, "T_LEVEL: step index")
        printed_match(fields[0], time, "f13.4", "T_LEVEL time")
        for column in (3, 5):
            printed_match(fields[column], -case["q"], "e13.5", "T_LEVEL flux")
        for column in (11, 13):
            printed_match(fields[column], case["hydraulics"]["head"], "e13.5", "T_LEVEL head")
        printed_match(fields[16], case["length"] * case["theta"], "e13.5", "T_LEVEL volume")
    return rows


def _solute_rows(path, case, launch):
    lines = _lines(path)
    header = (
        "Time cvTop cvBot Sum(cvTop) Sum(cvBot) cvCh0 cvCh1 cTop cRoot cBot cvRoot "
        "Sum(cvRoot) Sum(cvNEql) TLevel cGWL cRunOff Sum(cRunOff) "
        "(cv(i), Sum(cv(i)), i=1,NObs)"
    )
    require(
        lines[0].strip()
        == "All solute fluxes and cumulative solute fluxes are positive into the region",
        "solute: missing sign convention",
    )
    rows = _table(path, header, 17)
    _, times = schedule(case, launch)
    require(len(rows) == len(times), "solute: incomplete step inventory")
    for index, (fields, time) in enumerate(zip(rows, times), 1):
        values = [numeric(t) for t in fields]
        require(values[13] == index, "solute: step index")
        printed_match(fields[0], time, "f14.4", "solute time")
        for column in list(range(1, 13)) + [14, 15, 16]:
            rounding_half_width(fields[column], "e13.5")
    return [[numeric(t) for t in fields] for fields in rows]


def _node_profiles(path, case, launch):
    lines = _lines(path)
    header = "Node Depth Head Moisture K C Flux Sink Kappa v/KsTop Temp " "Conc(1..NS) Sorb(1...NS)"
    times, _ = schedule(case, launch)
    blocks = []
    current = None
    seen_header = False
    for line in lines:
        text = line.strip()
        if text.startswith("Time:"):
            require(len(blocks) < len(times), "NOD_INF: excess profile")
            token = text.partition(":")[2].strip()
            printed_match(token, times[len(blocks)], "f14.4", "NOD_INF time")
            current = []
            blocks.append(current)
            seen_header = False
        elif text.startswith("Node"):
            require(
                current is not None and not seen_header and " ".join(text.split()) == header,
                "NOD_INF: malformed/duplicate header",
            )
            seen_header = True
        elif current is not None and text and not text.startswith("[") and text != "end":
            require(seen_header, "NOD_INF: missing header")
            fields = text.split()
            require(len(fields) == 11 + 2 * len(case["species"]), "NOD_INF: malformed columns")
            current.append(fields)
    require(len(blocks) == len(times), "NOD_INF: missing profile times")
    count, species_count = launch["nodes"], len(case["species"])
    profiles, uncertainty, raw_sorb = [], [], []
    coordinates = [i * case["length"] / (count - 1) for i in range(count)]
    for time, rows in zip(times, blocks):
        require(len(rows) == count, "NOD_INF: truncated node profile")
        values = [[numeric(t) for t in row] for row in rows]
        for index, (fields, row) in enumerate(zip(rows, values)):
            require(row[0] == index + 1, "NOD_INF: duplicate/missing node")
            printed_match(fields[1], -coordinates[index], "f10.4", "NOD_INF coordinate")
            printed_match(fields[2], case["hydraulics"]["head"], "f11.3", "NOD_INF head")
            printed_match(fields[3], case["theta"], "f6.4", "NOD_INF theta")
            printed_match(fields[4], case["hydraulics"]["ks"], "e12.4", "NOD_INF K")
            printed_match(fields[6], -case["q"], "e12.4", "NOD_INF flux")
            require(row[5] == 0 and row[7] == 0, "NOD_INF: active capacity/sink")
            for column in range(11, 11 + 2 * species_count):
                rounding_half_width(fields[column], "e12.4")
            require(
                all(v == 0 for v in row[11 + species_count :]),
                "NOD_INF: inactive nonequilibrium Sorb is nonzero",
            )
        concentrations = [[row[11 + j] for row in values] for j in range(species_count)]
        profiles.append(
            {
                "time": time,
                "concentration": concentrations,
                "theta": [row[3] for row in values],
                "q_faces": [case["q"]] * (count + 1),
            }
        )
        uncertainty.append(
            {
                "concentration": [
                    [rounding_half_width(row[11 + j], "e12.4") for row in rows]
                    for j in range(species_count)
                ],
                "theta": [5e-5] * count,
                "coordinate": [5e-5] * count,
            }
        )
        raw_sorb.append(
            [[row[11 + species_count + j] for row in values] for j in range(species_count)]
        )
    return coordinates, profiles, uncertainty, raw_sorb


def _balance(path, case, launch):
    lines = _lines(path, None)
    terminal = next(line.strip() for line in reversed(lines) if line.strip())
    require(re.fullmatch(r"Calculation time \[sec\]\s+\S+", terminal), "BALANCE: terminal")
    finite(numeric(terminal.split()[-1]), "calculation time", True)
    times, _ = schedule(case, launch)
    blocks = []
    block = None
    patterns = {
        "inventory": r"ConcVol\s+\[M/L2\]\s+(\d+)\s+(\S+)\s+(\S+)",
        "mean": r"cMean\s+\[M/L3\]\s+(\d+)\s+(\S+)\s+(\S+)",
        "balance": r"CncBalT\s+\[M\]\s+(\d+)\s+(\S+)",
        "percent": r"CncBalR\s+\[%\]\s+(\d+)\s+(\S+)",
    }
    for line in lines:
        text = line.strip()
        match = re.fullmatch(r"Time\s+\[T\]\s+(\S+)", text)
        if match:
            require(len(blocks) < len(times), "BALANCE: extra time")
            printed_match(match.group(1), times[len(blocks)], "f14.4", "BALANCE time")
            block = {key: {} for key in patterns}
            blocks.append(block)
        elif text.startswith(tuple(("ConcVol", "cMean", "CncBalT", "CncBalR"))):
            require(block is not None, "BALANCE: record before time")
            matches = [(key, re.fullmatch(pattern, text)) for key, pattern in patterns.items()]
            matches = [(key, match) for key, match in matches if match]
            require(len(matches) == 1, "BALANCE: malformed species record")
            key, match = matches[0]
            species = int(match.group(1)) - 1
            require(
                0 <= species < len(case["species"]) and species not in block[key],
                "BALANCE: duplicate/unknown species",
            )
            block[key][species] = [numeric(t) for t in match.groups()[1:]]
    require(len(blocks) == len(times), "BALANCE: missing times")
    inventory, balance = [], []
    for time, block in zip(times, blocks):
        expected = set(range(len(case["species"])))
        require(
            set(block["inventory"]) == expected and set(block["mean"]) == expected,
            "BALANCE: incomplete species inventory",
        )
        # CncBalR is legitimately absent when the source denominator is zero.
        require(
            set(block["balance"]) == (expected if time > 0 else set()),
            "BALANCE: incomplete absolute balance",
        )
        inventory.append({"time": time, "values": [block["inventory"][j][0] for j in expected]})
        if time > 0:
            balance.append({"time": time, "values": [block["balance"][j][0] for j in expected]})
    return inventory, balance


def _endpoints(path, case, launch, initial):
    lines = _lines(path)
    # INPUT.FOR prints a grouped Node(...) header followed by h/theta/Temp/Conc columns.
    headers = [line for line in lines if "Node(" in line.replace(" ", "")]
    require(len(headers) == 1, "OBS_NODE: missing node header")
    node_ids = [int(value) for value in re.findall(r"Node\(\s*(\d+)\s*\)", headers[0])]
    require(node_ids == [1, launch["nodes"]], "OBS_NODE: wrong endpoint ordering")
    header_index = next(i for i, line in enumerate(lines) if line == headers[0])
    require(
        any(
            "time" in line.lower() and "h" in line and "theta" in line.lower()
            for line in lines[header_index + 1 : header_index + 5]
        ),
        "OBS_NODE: malformed header",
    )
    _, times = schedule(case, launch)
    rows = []
    for line in lines[header_index + 1 :]:
        fields = line.split()
        if not fields or fields[0].lower() in ("time", "end") or fields[0].startswith("["):
            continue
        require(NUMBER.fullmatch(fields[0]), "OBS_NODE: malformed record")
        rows.append(fields)
    # The source may emit an initial observation as well as every accepted step.
    expected_times = [0.0] + times if len(rows) == len(times) + 1 else times
    require(len(rows) == len(expected_times), "OBS_NODE: incomplete step inventory")
    count = len(case["species"])
    history = [{"time": 0.0, "concentration": [[row[0], row[-1]] for row in initial]}]
    widths = [[[0.0, 0.0] for _ in range(count)]]
    for fields, time in zip(rows, expected_times):
        require(len(fields) == 1 + 2 * (3 + count), "OBS_NODE: malformed columns")
        [numeric(t) for t in fields]
        printed_match(fields[0], time, "f14.4", "OBS_NODE time")
        concentrations, errors = [[] for _ in range(count)], [[] for _ in range(count)]
        for endpoint in range(2):
            offset = 1 + endpoint * (3 + count)
            printed_match(fields[offset], case["hydraulics"]["head"], "f12.2", "OBS_NODE head")
            printed_match(fields[offset + 1], case["theta"], "f8.4", "OBS_NODE theta")
            printed_match(fields[offset + 2], 20.0, "f9.3", "OBS_NODE temperature")
            for j in range(count):
                token = fields[offset + 3 + j]
                concentrations[j].append(numeric(token))
                errors[j].append(rounding_half_width(token, "e12.4"))
        if time == 0:
            for j in range(count):
                for endpoint in range(2):
                    require(
                        abs(concentrations[j][endpoint] - history[0]["concentration"][j][endpoint])
                        <= errors[j][endpoint],
                        "OBS_NODE initial mismatch",
                    )
            history[0], widths[0] = {"time": time, "concentration": concentrations}, errors
        else:
            history.append({"time": time, "concentration": concentrations})
            widths.append(errors)
    return history, widths


def _ancillary(paths, case, launch, initial):
    header = "n depth THr THs hs Ks Ks/KsTop Beta Ah AK ATh"
    rows = _table(_artifact(paths, "PROFILE.OUT"), header, 11)
    require(len(rows) == launch["nodes"], "PROFILE: incomplete nodes")
    for index, row in enumerate(rows):
        values = [numeric(t) for t in row]
        require(values[0] == index + 1, "PROFILE: node order")
        printed_match(
            row[1], index * case["length"] / (launch["nodes"] - 1), "f10.2", "PROFILE coordinate"
        )
        printed_match(row[3], case["theta"], "f8.3", "PROFILE saturation")
        printed_match(row[5], case["hydraulics"]["ks"], "e12.3", "PROFILE conductivity")
    lines = _lines(_artifact(paths, "I_CHECK.OUT"), None)
    nonempty = [line.strip() for line in lines if line.strip()]
    require(nonempty[-1].startswith("tPulse ="), "I_CHECK: missing terminal tPulse")
    printed_match(nonempty[-1].split()[-1], case["time_end"], "f15.3", "I_CHECK tPulse")
    positions = [i for i, line in enumerate(lines) if line.strip().startswith("node      x")]
    require(len(positions) == 1, "I_CHECK: nodal header")
    nodal = []
    for line in lines[positions[0] + 1 :]:
        if line.strip() == "end":
            break
        if line.strip():
            nodal.append(line.split())
    require(len(nodal) == launch["nodes"], "I_CHECK: incomplete nodes")
    for index, row in enumerate(nodal):
        require(len(row) == 10 + len(case["species"]), "I_CHECK: malformed node")
        values = [numeric(t) for t in row]
        require(values[0] == index + 1 and values[3:5] == [1, 1], "I_CHECK: node/material")
        for j in range(len(case["species"])):
            printed_match(row[10 + j], initial[j][index], "e12.4", "I_CHECK initial")
    bulk_headers = [i for i, line in enumerate(lines) if "Bulk.D." in line]
    require(len(bulk_headers) == 1, "I_CHECK: bulk header")
    bulk = lines[bulk_headers[0] + 1].split()
    require(len(bulk) == 5 and numeric(bulk[0]) == 1, "I_CHECK: bulk columns")
    for token, expected in zip(bulk[1:], [case["rho"], case["chemistry"]["dispersivity"], 1, 0]):
        printed_match(token, expected, "f13.4", "I_CHECK bulk/diffusion mapping")
    species_headers = [
        (i, int(match.group(1)))
        for i, line in enumerate(lines)
        if (match := re.search(r"\(\s*(\d+)\.solute\)", line))
    ]
    require(
        [j for _, j in species_headers] == list(range(1, len(case["species"]) + 1)),
        "I_CHECK: incomplete chemical species",
    )
    for position, species in species_headers:
        molecular = lines[position + 1].split()
        require(len(molecular) == 2, "I_CHECK: molecular coefficients")
        printed_match(
            molecular[0], case["chemistry"]["molecular_diffusion"], "e12.4", "I_CHECK diffusion"
        )
        printed_match(molecular[1], 0.0, "e12.4", "I_CHECK gas diffusion")
        record = lines[position + 3].split()
        require(len(record) == 15 and numeric(record[0]) == 1, "I_CHECK: chemistry columns")
        material = case["species"][species - 1]
        loss = material["rate"] if species == len(case["species"]) else 0.0
        transfer = material["rate"] if species < len(case["species"]) else 0.0
        expected = [material["kd"], 0, 1, 0, loss, loss, 0, transfer, transfer, 0, 0, 0, 0, 0]
        for token, target in zip(record[1:], expected):
            printed_match(token, target, "e11.4", "I_CHECK chemistry")
    for label, concentrations in (
        ("kTopCh", case["boundary"]["inlet_concentration"]),
        ("kBotCh", [0] * len(case["species"])),
    ):
        positions = [i for i, line in enumerate(lines) if line.strip().startswith(label)]
        require(len(positions) == 1, "I_CHECK: boundary header")
        row = lines[positions[0] + 1].split()
        require(len(row) == 1 + len(concentrations) and row[0] == "-1", "I_CHECK boundary")
        for token, expected in zip(row[1:], concentrations):
            printed_match(token, expected, "e10.3", "I_CHECK inlet concentration")


def _validate_observation(data, context, rounding):
    case, launch, manifest = (context[name] for name in ("case", "launch", "manifest"))
    keys(data, OBS_KEYS, "observation")
    require(data["schema_version"] == "solute-observation-v1", "observation schema")
    for name in (
        "run_id",
        "launch_id",
        "case_id",
        "cases_sha256",
        "case_sha256",
        "request_sha256",
        "solver",
    ):
        require(data[name] == manifest[name], f"observation {name} mismatch")
    require(data["units"] == context["book"]["units"], "observation units")
    require(data["species"] == [s["name"] for s in case["species"]], "observation species")
    count, species_count = launch["nodes"], len(case["species"])
    vector(data["coordinates"], count, "coordinates")
    expected_coordinates = [i * case["length"] / (count - 1) for i in range(count)]
    require(data["coordinates"] == expected_coordinates, "unexpected canonical mesh")
    times, step_times = schedule(case, launch)
    require(
        isinstance(data["profiles"], list)
        and [row.get("time") for row in data["profiles"]] == times,
        "profile time inventory",
    )
    for index, row in enumerate(data["profiles"]):
        keys(row, "time concentration theta q_faces", "profile")
        matrix(row["concentration"], species_count, count, "concentration")
        vector(row["theta"], count, "theta")
        vector(row["q_faces"], count + 1, "q faces")
        for node, value in enumerate(row["theta"]):
            require(
                abs(value - case["theta"]) <= rounding["profiles"][index]["theta"][node],
                "profile water mismatch",
            )
        require(all(value == case["q"] for value in row["q_faces"]), "profile face flux mismatch")
        if index == 0:
            for j in range(species_count):
                for node, value in enumerate(row["concentration"][j]):
                    require(
                        abs(value - context["initial"][j][node])
                        <= rounding["profiles"][0]["concentration"][j][node] + 1e-14,
                        "profile initial mismatch",
                    )
    require(
        isinstance(data["steps"], list) and len(data["steps"]) == len(step_times),
        "step count mismatch",
    )
    for index, (row, time) in enumerate(zip(data["steps"], step_times), 1):
        keys(row, "index time dt iterations peclet courant", "step")
        require(
            type(row["index"]) is int
            and row["index"] == index
            and row["time"] == time
            and row["dt"] == launch["dt"],
            "step sequence mismatch",
        )
        require(
            row["iterations"] is None or type(row["iterations"]) is int and row["iterations"] >= 0,
            "iterations",
        )
        finite(row["peclet"], "peclet", True)
        finite(row["courant"], "courant", True)
    keys(data["availability"], "endpoint_history solver_reported water", "availability")
    require(
        all(isinstance(v, str) and v.strip() for v in data["availability"].values()),
        "missing availability reason",
    )
    if any(row["iterations"] is None for row in data["steps"]):
        require(
            "iteration" in " ".join(data["availability"].values()).lower(),
            "missing iterations reason",
        )
    history = data["endpoint_history"]
    if history is None:
        require(
            launch["role"] == "bridge"
            and launch["solver"] == "fortran"
            and data["availability"]["endpoint_history"] == "legacy_bridge_no_obs",
            "missing endpoint history",
        )
    else:
        require(
            isinstance(history, list)
            and [row.get("time") for row in history] == [0.0] + step_times,
            "endpoint time inventory",
        )
        for row in history:
            keys(row, "time concentration", "endpoint record")
            matrix(row["concentration"], species_count, 2, "endpoint concentration")
        by_time = {row["time"]: (row, error) for row, error in zip(history, rounding["endpoints"])}
        for index, profile in enumerate(data["profiles"]):
            # Different source output routines may round at different places.
            endpoint, error = by_time[profile["time"]]
            for j in range(species_count):
                for side, node in enumerate((0, count - 1)):
                    bound = error[j][side] + rounding["profiles"][index]["concentration"][j][node]
                    require(
                        abs(endpoint["concentration"][j][side] - profile["concentration"][j][node])
                        <= bound + 1e-14,
                        "endpoint/profile mismatch",
                    )
    keys(
        data["solver_reported"],
        "inventory boundary_cumulative reaction_cumulative balance",
        "solver reported",
    )
    for field, records in data["solver_reported"].items():
        if records is None:
            continue
        require(isinstance(records, list) and records, "empty solver-reported history")
        reported_times = []
        for row in records:
            keys(row, "time values", "solver-reported record")
            finite(row["time"], "reported time", True)
            require(row["time"] in [0.0] + step_times, "unaccepted solver-reported time")
            vector(row["values"], species_count, field)
            reported_times.append(row["time"])
        require(reported_times == sorted(set(reported_times)), "duplicate solver-reported time")
        allowed = (times, times[1:], step_times, [0.0] + step_times)
        require(reported_times in allowed, "incomplete solver-reported history")
    require(
        isinstance(data["warnings"], list) and all(isinstance(v, str) for v in data["warnings"]),
        "warnings",
    )
    require(
        all(warning in data["warnings"] for warning in manifest["warnings"]),
        "dropped manifest warnings",
    )


def _parse_fortran(context):
    case, launch, manifest, paths = (context[k] for k in ("case", "launch", "manifest", "paths"))
    require(manifest["solver"] == "fortran", "expected Fortran manifest")
    formats = manifest["record_inventory"]["field_formats"]
    require(
        isinstance(formats, dict)
        and all(formats.get(k, "").lower() == v for k, v in FORMATS.items()),
        "field format mismatch",
    )
    required = [
        "NOD_INF.OUT",
        "RUN_INF.OUT",
        "T_LEVEL.OUT",
        "BALANCE.OUT",
        "PROFILE.OUT",
        "I_CHECK.OUT",
    ]
    required += [f"solute{j + 1}.out" for j in range(len(case["species"]))]
    if launch["role"] != "bridge":
        required.append("OBS_NODE.OUT")
    terminals = manifest["record_inventory"]["terminal_records"]
    require(isinstance(terminals, dict), "missing terminal inventory")
    for name in required:
        path = _artifact(paths, name)
        actual = next(line.strip() for line in reversed(_lines(path, None)) if line.strip())
        require(terminals.get(name) == actual, f"terminal inventory mismatch {name}")
    steps = _run_rows(_artifact(paths, "RUN_INF.OUT"), case, launch)
    _water_rows(_artifact(paths, "T_LEVEL.OUT"), case, launch)
    coordinates, profiles, errors, raw_sorb = _node_profiles(
        _artifact(paths, "NOD_INF.OUT"), case, launch
    )
    _ancillary(paths, case, launch, context["initial"])
    inventory, balance = _balance(_artifact(paths, "BALANCE.OUT"), case, launch)
    solute = [
        _solute_rows(_artifact(paths, f"solute{j + 1}.out"), case, launch)
        for j in range(len(case["species"]))
    ]
    history, endpoint_errors = (None, None)
    if launch["role"] != "bridge":
        history, endpoint_errors = _endpoints(
            _artifact(paths, "OBS_NODE.OUT"), case, launch, profiles[0]["concentration"]
        )
    reported = {
        "inventory": inventory,
        "balance": balance,
        "boundary_cumulative": [],
        "reaction_cumulative": [],
    }
    for index, step in enumerate(steps):
        reported["boundary_cumulative"].append(
            {"time": step["time"], "values": [rows[index][3] + rows[index][4] for rows in solute]}
        )
        reported["reaction_cumulative"].append(
            {"time": step["time"], "values": [rows[index][5] + rows[index][6] for rows in solute]}
        )
    data = {
        key: manifest[key]
        for key in (
            "run_id",
            "launch_id",
            "case_id",
            "cases_sha256",
            "case_sha256",
            "request_sha256",
            "solver",
        )
    }
    data.update(
        {
            "schema_version": "solute-observation-v1",
            "units": context["book"]["units"],
            "species": [s["name"] for s in case["species"]],
            "coordinates": coordinates,
            "profiles": profiles,
            "steps": steps,
            "endpoint_history": history,
            "solver_reported": reported,
            "warnings": manifest["warnings"],
            "availability": {
                "endpoint_history": (
                    "legacy_bridge_no_obs"
                    if history is None
                    else "OBS_NODE dissolved endpoint samples; t=0 NOD_INF initial profile"
                ),
                "solver_reported": "BALANCE ConcVol/CncBalT; solute cumulative boundary and "
                "cvCh0+cvCh1 reaction, retained separately from independent integrals",
                "water": "NOD_INF head/theta/K/flux and all-step T_LEVEL head/flux/volume "
                "validated; q_faces reconstructed q=Ks*CosAlpha from INPUT and OUTPUT "
                "constant saturated heads; D=alpha*abs(q)/theta+Dw, TauW=1",
            },
        }
    )
    rounding = {
        "profiles": errors,
        "endpoints": endpoint_errors,
        "time": 5e-5,
        "inactive_sorb": raw_sorb,
    }
    _validate_observation(data, context, rounding)
    return data, rounding


class _ValidatedObservations(dict):
    """Internal loader result; private context never becomes observation JSON."""

    def _seal(self, context, rounding):
        self._context = copy.deepcopy({key: context[key] for key in ("case", "launch", "manifest")})
        self._rounding = copy.deepcopy(rounding)
        self._fingerprint = digest_bytes(canonical([self, self._context, self._rounding]))
        return self


def _zero_rounding(data):
    return {
        "profiles": [
            {
                "concentration": [[0.0 for _ in row] for row in p["concentration"]],
                "theta": [0.0] * len(p["theta"]),
                "coordinate": [0.0] * len(p["theta"]),
            }
            for p in data["profiles"]
        ],
        "endpoints": (
            None
            if data["endpoint_history"] is None
            else [[[0.0, 0.0] for _ in data["species"]] for _ in data["endpoint_history"]]
        ),
        "time": 0.0,
        "inactive_sorb": None,
    }


def normalize_fortran(run_dir: Path, cases_path: Path) -> dict:
    """Validate raw output, write new immutable observations/receipt, and return it."""
    root = Path(run_dir).absolute()
    require(not any(p.is_symlink() for p in (root, *root.parents)), "symlink normalization path")
    allowed = {"13.2.3", "13.3.1", "13.4.1", "13.6.1"}
    require(
        any((ROOT / ".development" / task) in root.parents for task in allowed),
        "normalization outside task scratch",
    )
    require(root == root.resolve(), "noncanonical normalization path")
    require(
        not any(
            (root / name).exists() or (root / name).is_symlink()
            for name in ("observations.json", "normalization.json")
        ),
        "normalization refuses existing artifacts",
    )
    context = _context(root / "manifest.json", cases_path)
    raw_hash = digest_file(root / "manifest.json")
    data, rounding = _parse_fortran(context)
    payload = canonical(data) + b"\n"
    receipt = {
        "schema_version": "solute-normalization-v1",
        "raw_manifest_sha256": raw_hash,
        "observations_sha256": digest_bytes(payload),
        "normalizer_sha256": digest_file(__file__),
        "status": "complete",
    }
    require(
        digest_file(root / "manifest.json") == raw_hash, "raw manifest changed during normalization"
    )
    with (root / "observations.json").open("xb") as stream:
        stream.write(payload)
    with (root / "normalization.json").open("xb") as stream:
        stream.write(canonical(receipt) + b"\n")
    return _ValidatedObservations(data)._seal(context, rounding)


def load_observations(path: Path, manifest_path: Path, cases_path: Path) -> dict:
    """Load only provenance-verified, physically mapped, complete observations."""
    path = Path(path)
    context = _context(manifest_path, cases_path)
    require(path.resolve().parent == context["root"], "observation outside run directory")
    data = json_load(path)
    if context["manifest"]["solver"] == "fortran":
        receipt = json_load(context["root"] / "normalization.json")
        keys(
            receipt,
            "schema_version raw_manifest_sha256 observations_sha256 normalizer_sha256 status",
            "normalization receipt",
        )
        require(
            receipt["schema_version"] == "solute-normalization-v1"
            and receipt["status"] == "complete",
            "incomplete normalization",
        )
        hash_match(Path(manifest_path), receipt["raw_manifest_sha256"])
        hash_match(path, receipt["observations_sha256"])
        hash_match(Path(__file__), receipt["normalizer_sha256"])
        expected, rounding = _parse_fortran(context)
        require(data == expected, "normalized observations differ from raw records")
    else:
        require(context["manifest"]["status"] == "complete", "Rust manifest not complete")
        require(
            _artifact(context["paths"], "observations.json").resolve() == path.resolve(),
            "observation artifact mismatch",
        )
        rounding = _zero_rounding(data)
        _validate_observation(data, context, rounding)
    return _ValidatedObservations(data)._seal(context, rounding)


def mesh_weights(coordinates: list) -> list:
    """Return conservative half-cell weights on any strictly increasing mesh."""
    require(isinstance(coordinates, list) and len(coordinates) >= 3, "mesh needs three nodes")
    for value in coordinates:
        finite(value, "coordinate")
    require(all(b > a for a, b in zip(coordinates, coordinates[1:])), "nonincreasing mesh")
    return (
        [(coordinates[1] - coordinates[0]) / 2]
        + [(coordinates[i + 1] - coordinates[i - 1]) / 2 for i in range(1, len(coordinates) - 1)]
        + [(coordinates[-1] - coordinates[-2]) / 2]
    )


def normalized_error(
    actual: float,
    expected: float,
    reference_uncertainty: float = 0,
    rounding_uncertainty: float = 0,
    scale_name: str = "absolute reference",
) -> dict:
    """Normalize only against a resolved, named scale in the same units."""
    for value in (actual, expected):
        finite(value)
    finite(reference_uncertainty, "reference uncertainty", True)
    finite(rounding_uncertainty, "rounding uncertainty", True)
    return _normalized_absolute(
        abs(actual - expected),
        abs(expected),
        reference_uncertainty,
        rounding_uncertainty,
        scale_name,
    )


def _normalized_absolute(error, scale, reference_uncertainty, rounding_uncertainty, scale_name):
    uncertainty = reference_uncertainty + rounding_uncertainty
    branch = "zero" if scale == 0 else "near_zero" if scale <= uncertainty else "resolved"
    return {
        "absolute": error,
        "relative": error / scale if branch == "resolved" else None,
        "branch": branch,
        "scale": scale,
        "scale_name": scale_name,
        "reference_uncertainty": reference_uncertainty,
        "rounding_uncertainty": rounding_uncertainty,
        "error_interval": [max(0.0, error - uncertainty), error + uncertainty],
    }


def _norms(values, weights):
    length = math.fsum(weights)
    return {
        "l1": math.fsum(w * abs(v) for w, v in zip(weights, values)) / length,
        "l2": math.sqrt(math.fsum(w * v * v for w, v in zip(weights, values)) / length),
        "linf": max(abs(v) for v in values),
        "endpoint_linf": max(abs(values[0]), abs(values[-1])),
        "interior_linf": max(abs(v) for v in values[1:-1]),
    }


def _inventory(concentration, theta, rho_kd, weights, error=None):
    dissolved = math.fsum(w * t * c for w, t, c in zip(weights, theta, concentration))
    sorbed = rho_kd * math.fsum(w * c for w, c in zip(weights, concentration))
    result = {"dissolved": dissolved, "equilibrium_sorbed": sorbed, "total": dissolved + sorbed}
    if error is None:
        return result
    ce, te, xe = error
    we = (
        [(xe[0] + xe[1]) / 2]
        + [(xe[i - 1] + xe[i + 1]) / 2 for i in range(1, len(xe) - 1)]
        + [(xe[-2] + xe[-1]) / 2]
    )
    d_error = math.fsum(
        (w + ew) * (abs(t) + et) * (abs(c) + ec) - w * abs(t) * abs(c)
        for w, ew, t, et, c, ec in zip(weights, we, theta, te, concentration, ce)
    )
    s_error = rho_kd * math.fsum(
        (w + ew) * (abs(c) + ec) - w * abs(c)
        for w, ew, c, ec in zip(weights, we, concentration, ce)
    )
    return result, {
        "dissolved": max(0, d_error),
        "equilibrium_sorbed": max(0, s_error),
        "total": max(0, d_error + s_error),
    }


def _integral_history(times, values, uncertainties, time_uncertainty):
    integrals, errors = [0.0], [0.0]
    for i in range(1, len(times)):
        dt = times[i] - times[i - 1]
        require(dt > 0, "nonincreasing integration time")
        integrals.append(integrals[-1] + dt * (values[i - 1] + values[i]) / 2)
        errors.append(
            errors[-1]
            + dt * (uncertainties[i - 1] + uncertainties[i]) / 2
            + time_uncertainty
            * (abs(values[i - 1]) + abs(values[i]) + uncertainties[i - 1] + uncertainties[i])
        )
    return integrals, errors


def _coarsening_sensitivity(times, values, integrals):
    """Return coarse-minus-fine prefix differences, not a certified error bound."""
    coarse = [0.0] * len(times)
    result = [0.0, None] if len(times) > 1 else [0.0]
    for i in range(2, len(times)):
        coarse[i] = coarse[i - 2] + (times[i] - times[i - 2]) * (values[i] + values[i - 2]) / 2
        if i % 2:
            coarse[i] += (times[1] - times[0]) * (values[1] + values[0]) / 2 if i == 3 else 0
        result.append(abs(coarse[i] - integrals[i]))
    return result


def _reference(reference, observations):
    keys(
        reference, "schema_version coordinates species profiles continuum uncertainty", "reference"
    )
    require(reference["schema_version"] == "solute-metrics-reference-v1", "reference schema")
    require(
        reference["coordinates"] == observations["coordinates"]
        and reference["species"] == observations["species"],
        "reference mesh/species mismatch",
    )
    keys(reference["uncertainty"], "profiles continuum", "reference uncertainty")
    times = [p["time"] for p in observations["profiles"]]
    analysis_times = [0.0] + observations._context["launch"]["analysis_times"]
    size, count = len(observations["coordinates"]), len(observations["species"])
    for group, expected_times in (("profiles", analysis_times), ("continuum", times)):
        for uncertain, container in ((False, reference), (True, reference["uncertainty"])):
            records = container[group]
            require(
                isinstance(records, list) and [p.get("time") for p in records] == expected_times,
                f"reference {group}: missing/duplicate time",
            )
            for row in records:
                if group == "profiles":
                    keys(row, "time concentration", "reference profile")
                    matrix(row["concentration"], count, size, "reference concentration", uncertain)
                else:
                    keys(row, "time " + " ".join(CONTINUUM_FIELDS), "reference continuum")
                    for field in CONTINUUM_FIELDS:
                        if field in PAIR_FIELDS:
                            matrix(row[field], count, 2, field, uncertain)
                        else:
                            vector(row[field], count, field, uncertain)
    return {key: {row["time"]: row for row in reference[key]} for key in ("profiles", "continuum")}


def compute_metrics(observations: dict, reference: dict) -> dict:
    """Compute independent inventories, integrals, norms, and uncertainty branches."""
    require(
        isinstance(observations, _ValidatedObservations), "observations must come from a loader"
    )
    require(
        observations._fingerprint
        == digest_bytes(canonical([observations, observations._context, observations._rounding])),
        "validated observations changed",
    )
    refs = _reference(reference, observations)
    case = observations._context["case"]
    launch = observations._context["launch"]
    rounding = observations._rounding
    weights = mesh_weights(observations["coordinates"])
    profiles = observations["profiles"]
    times = [p["time"] for p in profiles]
    count = len(observations["species"])
    inventories, inventory_errors = [], []
    for index, profile in enumerate(profiles):
        values, errors = [], []
        for j, species in enumerate(case["species"]):
            error = rounding["profiles"][index]
            value, bound = _inventory(
                profile["concentration"][j],
                profile["theta"],
                case["rho"] * species["kd"],
                weights,
                (error["concentration"][j], error["theta"], error["coordinate"]),
            )
            values.append(value)
            errors.append(bound)
        inventories.append(values)
        inventory_errors.append(errors)
    losses, loss_errors, loss_sensitivity = [], [], []
    for j, species in enumerate(case["species"]):
        rate_values = [species["rate"] * row[j]["total"] for row in inventories]
        rate_errors = [species["rate"] * row[j]["total"] for row in inventory_errors]
        values, errors = _integral_history(times, rate_values, rate_errors, rounding["time"])
        losses.append(values)
        loss_errors.append(errors)
        loss_sensitivity.append(_coarsening_sensitivity(times, rate_values, values))
    endpoint_history = observations["endpoint_history"]
    endpoint_indices, outlets, outlet_errors, outlet_sensitivity = {}, [], [], []
    if endpoint_history is not None:
        endpoint_times = [p["time"] for p in endpoint_history]
        endpoint_indices = {time: i for i, time in enumerate(endpoint_times)}
        for j in range(count):
            fluxes = [-case["q"] * row["concentration"][j][1] for row in endpoint_history]
            errors = [case["q"] * row[j][1] for row in rounding["endpoints"]]
            values, bounds = _integral_history(endpoint_times, fluxes, errors, rounding["time"])
            outlets.append(values)
            outlet_errors.append(bounds)
            outlet_sensitivity.append(_coarsening_sensitivity(endpoint_times, fluxes, values))
    reference_errors = {
        group: {r["time"]: r for r in reference["uncertainty"][group]}
        for group in ("profiles", "continuum")
    }
    species_results = []
    for j, species in enumerate(case["species"]):
        history, profile_metrics = [], []
        for index, time in enumerate(times):
            truth = refs["continuum"][time]
            uncertainty = reference_errors["continuum"][time]
            mass = inventories[index][j]
            mass_error = inventory_errors[index][j]
            inventory_metrics = {
                phase: normalized_error(
                    mass[phase],
                    truth[phase][j],
                    uncertainty[phase][j],
                    mass_error[phase],
                    f"{species['name']} continuum {phase} inventory",
                )
                for phase in ("dissolved", "equilibrium_sorbed", "total")
            }
            inlet = case["q"] * case["boundary"]["inlet_concentration"][j]
            production = losses[j - 1][index] if j else 0.0
            production_error = loss_errors[j - 1][index] if j else 0.0
            loss = losses[j][index]
            loss_error = loss_errors[j][index]
            reaction = {
                "production": production,
                "loss": loss,
                "production_error": normalized_error(
                    production,
                    truth["production_cumulative"][j],
                    uncertainty["production_cumulative"][j],
                    production_error,
                    f"{species['name']} cumulative production",
                ),
                "loss_error": normalized_error(
                    loss,
                    truth["loss_cumulative"][j],
                    uncertainty["loss_cumulative"][j],
                    loss_error,
                    f"{species['name']} cumulative loss",
                ),
                "production_quadrature_sensitivity": loss_sensitivity[j - 1][index] if j else 0,
                "loss_quadrature_sensitivity": loss_sensitivity[j][index],
            }
            row = {
                "time": time,
                "inventory": mass,
                "inventory_error": inventory_metrics,
                "inventory_rounding": mass_error,
                "reaction": reaction,
                "boundary": None,
                "balance": None,
            }
            if endpoint_history is not None:
                eindex = endpoint_indices[time]
                outlet_flux = -case["q"] * endpoint_history[eindex]["concentration"][j][1]
                flux_error = case["q"] * rounding["endpoints"][eindex][j][1]
                cumulative = [inlet * time, outlets[j][eindex]]
                cumulative_error = [0.0, outlet_errors[j][eindex]]
                row["boundary"] = {
                    "instantaneous": [inlet, outlet_flux],
                    "cumulative": cumulative,
                    "instantaneous_error": [
                        normalized_error(
                            value,
                            truth["endpoint_flux"][j][side],
                            uncertainty["endpoint_flux"][j][side],
                            [0.0, flux_error][side],
                            f"{species['name']} inward endpoint {side} flux",
                        )
                        for side, value in enumerate((inlet, outlet_flux))
                    ],
                    "cumulative_error": [
                        normalized_error(
                            value,
                            truth["boundary_cumulative"][j][side],
                            uncertainty["boundary_cumulative"][j][side],
                            cumulative_error[side],
                            f"{species['name']} cumulative inward endpoint {side}",
                        )
                        for side, value in enumerate(cumulative)
                    ],
                    "outlet_quadrature_sensitivity": outlet_sensitivity[j][eindex],
                }
                residual = (
                    mass["total"] - inventories[0][j]["total"] - sum(cumulative) - production + loss
                )
                residual_error = (
                    mass_error["total"]
                    + inventory_errors[0][j]["total"]
                    + sum(cumulative_error)
                    + production_error
                    + loss_error
                )
                reference_initial = refs["continuum"][0.0]
                ref_residual = (
                    truth["total"][j]
                    - reference_initial["total"][j]
                    - sum(truth["boundary_cumulative"][j])
                    - truth["production_cumulative"][j]
                    + truth["loss_cumulative"][j]
                )
                ref_bound = (
                    uncertainty["total"][j]
                    + reference_errors["continuum"][0.0]["total"][j]
                    + sum(uncertainty["boundary_cumulative"][j])
                    + uncertainty["production_cumulative"][j]
                    + uncertainty["loss_cumulative"][j]
                )
                row["balance"] = {
                    "residual": residual,
                    "rounding_uncertainty": residual_error,
                    "reference_residual": ref_residual,
                    "reference_uncertainty": ref_bound,
                    "comparison": normalized_error(
                        residual, ref_residual, ref_bound, residual_error, "reference residual"
                    ),
                }
            if time in refs["profiles"]:
                expected = refs["profiles"][time]["concentration"][j]
                actual = profiles[index]["concentration"][j]
                errors = _norms([a - b for a, b in zip(actual, expected)], weights)
                scales = _norms(expected, weights)
                ref_bounds = _norms(reference_errors["profiles"][time]["concentration"][j], weights)
                round_bounds = _norms(rounding["profiles"][index]["concentration"][j], weights)
                metrics = {}
                for name, absolute in errors.items():
                    metrics[name] = _normalized_absolute(
                        absolute,
                        scales[name],
                        ref_bounds[name],
                        round_bounds[name],
                        f"{species['name']} reference {name}",
                    )
                mesh_truth = _inventory(
                    expected, [case["theta"]] * len(weights), case["rho"] * species["kd"], weights
                )
                row["inventory_mesh_truth"] = mesh_truth
                row["inventory_mesh_error"] = {
                    phase: mass[phase] - mesh_truth[phase] for phase in mesh_truth
                }
                row["inventory_mesh_quadrature"] = {
                    phase: mesh_truth[phase] - truth[phase][j] for phase in mesh_truth
                }
                profile_metrics.append({"time": time, "norms": metrics})
            history.append(row)
        selected = [p for p in profile_metrics if p["time"] in launch["analysis_times"]]
        analysis_times = [p["time"] for p in selected]
        temporal_weights = mesh_weights(analysis_times)
        temporal_rms = math.sqrt(
            math.fsum(
                w * p["norms"]["l2"]["absolute"] ** 2 for w, p in zip(temporal_weights, selected)
            )
            / (analysis_times[-1] - analysis_times[0])
        )
        species_results.append(
            {
                "species": species["name"],
                "profiles": profile_metrics,
                "history": history,
                "temporal_l2": temporal_rms,
            }
        )
    raw = copy.deepcopy(dict(observations))
    reported = raw.pop("solver_reported")
    return {
        "raw": raw,
        "derived": {
            "species": species_results,
            "mesh_weights": weights,
            "rounding": copy.deepcopy(rounding),
            "interpolation_uncertainty": 0.0,
            "coordinate_policy": "requested canonical mesh; printed coordinate intervals propagated "
            "through inventory weights; no interpolation",
            "profile_coordinate_uncertainty": {
                "status": (
                    "unresolved" if observations["solver"] == "fortran" else "exact_request_mesh"
                ),
                "coordinate_half_width": 5e-5 if observations["solver"] == "fortran" else 0.0,
                "reason": "reference wrapper supplies no independent spatial derivative bound; "
                "profile uncertainty excludes this unresolved coordinate effect",
            },
            "integration": "trapezoidal state integrals; coarsening sensitivities are empirical, "
            "not rigorous quadrature bounds; solver terms never used for closure",
            "availability": (
                "bridge lacks independent endpoint integrals/balance"
                if endpoint_history is None
                else "independent sampled endpoint and reaction integrals"
            ),
            "thresholds": None,
        },
        "solver_reported": reported,
    }

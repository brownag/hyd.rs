#!/usr/bin/env python3
"""Isolated, finite-matrix Fortran adapter; default invocation only prepares recipes.

Wave 3 can reuse initial_arrays(case, nodes), request_bytes(case, row, run_id,
cases_sha256), generate_decks(case, row), and validate_outputs(run_dir, case,
row, started_ns=None). No function derives forcing from a solver output.
"""

import argparse
import datetime
import fcntl
import hashlib
import json
import math
import os
import platform
import re
import shutil
import subprocess
import sys
import time
import uuid
import zipfile
from pathlib import Path

try:
    from tools import check_solute_verification_spec as contract
except ModuleNotFoundError:
    import check_solute_verification_spec as contract

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "reference/fortran_reference/solute/example3"
PATCH = "reference/fortran_reference/0001-gfortran-portability.patch"
SOURCE_MANIFEST = "planning/master/SOURCE_MANIFEST.json"
FLAGS = ["-std=legacy", "-ffixed-line-length-none"]
PRECISION_FLAGS = {
    "baseline": FLAGS,
    "promoted-real": FLAGS + ["-fdefault-real-8", "-fdefault-double-8"],
}
PHASE_TASKS = {
    "13.3.1": "benchmark",
    "13.4.1": "convergence",
    "13.6.1": "holdout",
    "22-benchmark": "benchmark",
    "22-convergence": "convergence",
    "22-holdout": "holdout",
    "23-classify-3": "convergence",
    "23-controls": "convergence",
}
OUTPUTS = (
    "I_CHECK.OUT",
    "RUN_INF.OUT",
    "T_LEVEL.OUT",
    "NOD_INF.OUT",
    "BALANCE.OUT",
    "OBS_NODE.OUT",
    "PROFILE.OUT",
)
INPUTS = ("SELECTOR.IN", "PROFILE.DAT", "LEVEL_01.DIR")
ALIASES = {"SELECTOR.IN": "solute-input\\Selector.in", "PROFILE.DAT": "solute-input\\Profile.dat"}
FIELD_FORMATS = {
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
    "endpoint_head": "f12.2",
    "endpoint_theta": "f8.4",
    "endpoint_temperature": "f9.3",
}
NUMBER = re.compile(r"[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[EeDd][+-]?\d+)?\Z")
MANIFEST_KEYS = set(
    (
        "schema_version run_id launch_id case_id cases_sha256 case_sha256 "
        "request_sha256 solver precision status command cwd started_utc ended_utc "
        "exit_code timeout_seconds provenance artifacts record_inventory warnings "
        "mapping_validation"
    ).split()
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(path):
    with Path(path).open("rb") as stream:
        digest = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_hash(value):
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, separators=(",", ":"), ensure_ascii=True, allow_nan=False
        ).encode("ascii")
    ).hexdigest()


def utc():
    return datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z")


def safe_path(root, relative):
    """Reject traversal and every symlink component, including dangling links."""
    relative = Path(relative)
    require(not relative.is_absolute() and ".." not in relative.parts, "unsafe relative path")
    target = Path(root) / relative
    for part in (target, *target.parents):
        require(not part.is_symlink(), f"symlink forbidden: {part}")
    require(
        target.resolve() == Path(root).resolve()
        or Path(root).resolve() in target.resolve().parents,
        "path escaped root",
    )
    return target


def output_root(value):
    path = Path(os.path.abspath(value))
    require(".." not in Path(value).parts, "output path traversal")
    relative = path.relative_to(ROOT)
    require(
        len(relative.parts) >= 2
        and relative.parts[0] == ".development"
        and relative.parts[1] in {"13.2.1", *PHASE_TASKS},
        "output root outside task scratch",
    )
    safe_path(ROOT, relative)
    return path, ROOT / ".development" / relative.parts[1]


def write_new(path, data):
    safe_path(path.parent, path.name)
    with path.open("xb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def write_json(path, data):
    write_new(path, (json.dumps(data, indent=2, sort_keys=True, allow_nan=False) + "\n").encode())


def protected_hashes(book):
    """Check the archive authority and snapshot, then hash existing accepted evidence."""
    pinned = contract.load_cases(ROOT / SOURCE_MANIFEST)
    expected = dict(pinned)
    expected.update(book["provenance"]["source_sha256"])
    source_names = {str(p.relative_to(ROOT)) for p in (ROOT / "src_reference").glob("*.FOR")}
    require(
        source_names == {k for k in pinned if k.startswith("src_reference/")},
        "source inventory differs from authority",
    )
    for name, digest in expected.items():
        require(sha256(safe_path(ROOT, name)) == digest, f"protected hash mismatch: {name}")
    with zipfile.ZipFile(ROOT / "planning/H1D_Src.zip") as archive:
        for name in source_names:
            require(
                archive.read(Path(name).name) == (ROOT / name).read_bytes(),
                f"archive member mismatch: {name}",
            )
    snapshot = {str(ROOT / name): digest for name, digest in expected.items()}
    checkouts = [ROOT]
    if (ROOT / ".git").is_file():
        gitdir = (ROOT / ".git").read_text().strip().split("gitdir: ", 1)[1]
        shared = Path(gitdir).resolve().parents[2]
        if (shared / "src_reference").is_dir():
            checkouts.append(shared)
    for checkout in checkouts:
        bases = [
            checkout / "reference/fortran_reference/solute/example3",
            *sorted((checkout / ".development").glob("12.*")),
        ]
        for base in bases:
            for path in base.rglob("*"):
                if path.is_file():
                    snapshot[str(path)] = sha256(path)
    return snapshot


def check_preserved(snapshot):
    for name, digest in snapshot.items():
        path = Path(name)
        require(
            sha256(safe_path(path.parent, path.name)) == digest,
            f"protected artifact changed: {name}",
        )


def grid(case, nodes):
    require(type(nodes) is int and 2 <= nodes <= 1001, "unsupported node capacity")
    return [i * case["length"] / (nodes - 1) for i in range(nodes)]


def initial_arrays(case, nodes):
    """Sample the frozen analytical IC only; return species-major canonical arrays."""
    coordinates = grid(case, nodes)
    initial = case["initial"]
    if initial["kind"] == "uniform":
        return [[value] * nodes for value in initial["values"]]
    if initial["kind"] == "cosine":
        return [
            [
                initial["mean"]
                + initial["amplitude"] * math.cos(initial["mode"] * math.pi * s / case["length"])
                for s in coordinates
            ]
        ]
    require(initial["kind"] == "steady_plus_mode", "unsupported initial condition")
    species = case["species"][0]
    velocity, dispersion, length = case["q"] / case["theta"], case["dispersion"], case["length"]
    z = (1 + case["rho"] * species["kd"] / case["theta"]) * species["rate"]
    radical = math.sqrt(velocity * velocity + 4 * dispersion * z)
    rp, rm = (velocity + radical) / (2 * dispersion), -2 * z / (velocity + radical)
    coefficient = velocity / (dispersion * rp * (1 - (rm / rp) ** 2 * math.exp((rm - rp) * length)))
    alpha = velocity / (2 * dispersion)
    return [
        [
            coefficient
            * (math.exp(rm * s) - rm / rp * math.exp(rm * length + rp * (s - length)))
            * case["boundary"]["inlet_concentration"][0]
            + initial["amplitude"]
            * math.exp(alpha * (s - length / 2))
            * math.cos(initial["root"] * (s - length / 2))
            for s in coordinates
        ]
    ]


def schedule(case, row):
    require(row["dt"] > 0 and row["sampling_interval"] > 0, "nonpositive schedule")
    for value in (row["dt"], row["sampling_interval"]):
        require(contract.aligned(case["time_end"], value), "nonintegral schedule")
    steps = [i * row["dt"] for i in range(1, round(case["time_end"] / row["dt"]) + 1)]
    prints = (
        list(row["analysis_times"])
        if row["role"] == "bridge"
        else [
            i * row["sampling_interval"]
            for i in range(1, round(case["time_end"] / row["sampling_interval"]) + 1)
        ]
    )
    require(len(steps) <= row["max_steps"] and len(prints) <= 999, "schedule capacity exceeded")
    require(
        all(any(math.isclose(t, p, abs_tol=1e-12) for p in prints) for t in row["analysis_times"]),
        "missing analysis print",
    )
    return steps, [0.0] + prints


def request_bytes(case, row, run_id, cases_sha256):
    """Create the flat-ascii-v1 shared request with round-trip decimal numbers."""
    require(re.fullmatch(r"[A-Za-z0-9_.-]+", run_id), "unsafe run id")
    schedule(case, row)
    values = {
        "schema": "flat-ascii-v1",
        "run_id": run_id,
        "launch_id": row["id"],
        "case_id": case["id"],
        "cases_sha256": cases_sha256,
        "case_sha256": canonical_hash(case),
        "nodes": str(row["nodes"]),
        "species_count": str(len(case["species"])),
    }
    for key in ("length", "theta", "rho", "q", "dispersion", "time_end"):
        values[key] = format(case[key], ".17g")
    for key in ("dt", "sampling_interval"):
        values[key] = format(row[key], ".17g")
    vectors = {
        "analysis_times": row["analysis_times"],
        "kd": [s["kd"] for s in case["species"]],
        "rates": [s["rate"] for s in case["species"]],
        "inlet_concentration": case["boundary"]["inlet_concentration"],
    }
    vectors.update(
        {f"initial_{i}": vector for i, vector in enumerate(initial_arrays(case, row["nodes"]))}
    )
    for key, vector in vectors.items():
        require(all(math.isfinite(v) for v in vector), "nonfinite request data")
        values[key] = ",".join(format(v, ".17g") for v in vector)
    values["species"] = ",".join(s["name"] for s in case["species"])
    if "backend" in row:
        values["backend"] = row["backend"]
    return "".join(f"{key}={value}\n" for key, value in values.items()).encode("ascii")


def chemical_coefficients(case, index):
    species = case["species"][index]
    ordinary = species["rate"] if index == len(case["species"]) - 1 else 0
    transfer = species["rate"] if index < len(case["species"]) - 1 else 0
    return [species["kd"], 0, 1, 0, ordinary, ordinary, 0, transfer, transfer, 0, 0, 0, 0, 0]


def generate_decks(case, row):
    """Generate native top-first x=-s inputs, preserving all bridge input bytes."""
    require(case["status"] == "executable", "blocked case")
    grid(case, row["nodes"])
    _, prints = schedule(case, row)
    if row["role"] == "bridge":
        require(
            case["id"] == "example3" and row["nodes"] == 1001 and row["dt"] == 0.1,
            "unsupported bridge change",
        )
        return {name: (FIXTURE / name).read_bytes() for name in INPUTS}
    require(
        row["time_controls"]
        == {
            "policy": "fixed",
            "dt_min": row["dt"],
            "dt_max": row["dt"],
            "increase": 1,
            "decrease": 1,
        },
        "nonfixed study controls",
    )
    lines = (FIXTURE / "SELECTOR.IN").read_text().splitlines()

    def replace_after(label, text):
        lines[lines.index(label) + 1] = text

    def nums(values):
        return " ".join(format(v, ".17g") for v in values)

    h, c = case["hydraulics"], case["chemistry"]
    lines[2:4] = [
        "Authored solute verification case " + case["id"],
        "Constant-water verification; frozen v1 source mapping.",
    ]
    replace_after("NMat NLay CosAlpha", nums([1, 1, h["cos_alpha"]]))
    replace_after(
        "thr ths Alfa n Ks l",
        nums([h["theta_r"], case["theta"], h["alpha"], h["n"], h["ks"], h["pore_connectivity"]]),
    )
    replace_after(
        "dt dtMin dtMax DMul DMul2 ItMin ItMax MPL",
        nums([row["dt"]] * 3 + [1, 1, 1, 20, len(prints) - 1]),
    )
    replace_after("tInit tMax", nums([0, case["time_end"]]))
    replace_after(
        "lPrintD nPrintSteps tPrintInterval lEnter",
        "f 1 " + format(row["sampling_interval"], ".17g") + " f",
    )
    replace_after("TPrint(1...MPL)", nums(prints[1:]))
    replace_after(
        "epsi lUpW lArtD lTDep cTolA cTolR MaxItC PeCr NS lTort iBact lFiltr",
        f"{c['epsi']} {'t' if c['upstream_weighting'] else 'f'} f f "
        f"0.000001 0.000001 20 100 {len(case['species'])} f 0 f",
    )
    replace_after(
        "BulkDensity Dispersivity Fraction ImmobileWaterContent",
        nums([case["rho"], c["dispersivity"], 1, 0]),
    )
    start = lines.index("Species 1: Dw Dg")
    lines = lines[:start]
    for index, species in enumerate(case["species"]):
        lines += [
            f"Species {index+1}: Dw Dg",
            nums([c["molecular_diffusion"], 0]),
            f"Species {index+1}: Ks Nu Beta Henry GamL GamS GamG GamL1 GamS1 GamG1 xMuL xMuS xMuG Omega",
            nums(chemical_coefficients(case, index)),
        ]
    lines += [
        "kTopCh cTop kBotCh cBot",
        nums([-1] + case["boundary"]["inlet_concentration"] + [-1] + [0] * len(case["species"])),
        "tPulse",
        format(case["time_end"], ".17g"),
        "*** END OF INPUT FILE 'SELECTOR.IN' ***********************************",
    ]
    profile = [
        "Pcp_File_Version=4",
        "1",
        "Authored constant-water verification profile",
        f"{row['nodes']} 0 {len(case['species'])}",
    ]
    initial = initial_arrays(case, row["nodes"])
    for i, s in enumerate(grid(case, row["nodes"])):
        profile.append(
            nums([i + 1, -s, h["head"], 1, 1, 0, 1, 1, 1, 20] + [values[i] for values in initial])
        )
    profile += ["2", f"1 {row['nodes']}"]
    return {
        "SELECTOR.IN": ("\n".join(lines) + "\n").encode("ascii"),
        "PROFILE.DAT": ("\n".join(profile) + "\n").encode("ascii"),
        "LEVEL_01.DIR": (FIXTURE / "LEVEL_01.DIR").read_bytes(),
    }


def build_reference(run_dir, precision, sources):
    """Build only a new isolated copy using the pinned patch and retained GNU shim."""
    require(precision in PRECISION_FLAGS, "unsupported arithmetic mode")
    build = run_dir / "build"
    build.mkdir()
    staged = build / "src_reference"
    shutil.copytree(ROOT / "src_reference", staged)
    for name in sources:
        if name.startswith("src_reference/"):
            require(sha256(staged / Path(name).name) == sources[name], "staging hash mismatch")
    patch_command = ["git", "apply", str(ROOT / PATCH)]
    patched = subprocess.run(patch_command, cwd=staged, capture_output=True, check=True)
    write_new(build / "patch.stdout", patched.stdout)
    write_new(build / "patch.stderr", patched.stderr)
    builder = (ROOT / "tools/build_fortran_reference.sh").read_text()
    shim = builder.split("<<'EOF'\n", 1)[1].split("\nEOF\n", 1)[0] + "\n"
    shim_path = build / "gnu_compatibility.FOR"
    write_new(shim_path, shim.encode("ascii"))
    compiler = subprocess.run(
        ["gfortran", "--version"], capture_output=True, text=True, check=True
    ).stdout.splitlines()[0]
    executable = build / "hydrus-reference"
    command = [
        "gfortran",
        *PRECISION_FLAGS[precision],
        *[str(p) for p in sorted(staged.glob("*.FOR"))],
        str(shim_path),
        "-o",
        str(executable),
    ]
    with (build / "compiler.stdout").open("xb") as stdout, (build / "compiler.stderr").open(
        "xb"
    ) as stderr:
        subprocess.run(command, cwd=build, stdout=stdout, stderr=stderr, check=True, timeout=60)
    require(executable.is_file() and os.access(executable, os.X_OK), "build not executable")
    staged_hashes = {str(p.relative_to(run_dir)): sha256(p) for p in sorted(staged.glob("*.FOR"))}
    write_json(
        build / "recipe.json",
        {
            "command": command,
            "patch_command": patch_command,
            "compiler": compiler,
            "source_sha256": staged_hashes,
        },
    )
    return {
        "compiler": compiler,
        "executable_sha256": sha256(executable),
        "shim_sha256": sha256(shim_path),
        "staged_sha256": staged_hashes,
    }


def number(token):
    require(bool(NUMBER.fullmatch(token)), f"invalid numeric token: {token}")
    value = float(token.replace("D", "E").replace("d", "e"))
    require(math.isfinite(value), "nonfinite output")
    return value


def matches(token, expected, places=None):
    """Test against the source format's half-last-place printing interval."""
    value = number(token)
    mantissa, *exponent = re.split("[EeDd]", token)
    decimals = len(mantissa.split(".")[1]) if "." in mantissa else 0
    quantum = 10.0 ** (
        (int(exponent[0]) if exponent else 0) - (decimals if places is None else places)
    )
    return abs(value - expected) <= quantum / 2 + 1e-12 * max(1, abs(expected))


def printed_format(token, kind, decimals):
    """Reject tokens whose precision differs from the selected source declaration."""
    suffix = r"[EeDd][+-]\d{2,3}" if kind == "e" else ""
    pattern = rf"[+-]?\d*\.\d{{{decimals}}}{suffix}"
    require(re.fullmatch(pattern, token), f"malformed {kind}*.{decimals} token: {token}")


def numeric_rows(lines, width, context):
    rows = []
    for line in lines:
        fields = line.split()
        if fields and (
            NUMBER.fullmatch(fields[0])
            or fields[0].startswith("*")
            and not line.lstrip().startswith("*******")
        ):
            require(len(fields) == width, f"{context}: incorrect record width")
            for token in fields:
                number(token)
            rows.append(fields)
    return rows


def require_header(lines, marker, name):
    require(any(marker in line for line in lines), f"{name}: missing {marker} header")


def validate_outputs(run_dir, case, row, started_ns=None):
    """Validate complete native record inventories and sampled/reconstructed water."""
    species_count, nodes = len(case["species"]), row["nodes"]
    steps, times = schedule(case, row)
    names = list(OUTPUTS) + [f"solute{i+1}.out" for i in range(species_count)]
    texts, terminals = {}, {}
    require(not (run_dir / "solute-input\\Error.msg").exists(), "legacy Error.msg exists")
    for name in names:
        path = safe_path(run_dir, "solute-input\\" + name)
        require(path.is_file() and path.stat().st_size > 0, f"missing/empty output: {name}")
        if started_ns is not None:
            require(path.stat().st_mtime_ns >= started_ns, f"stale output: {name}")
        content = path.read_text(encoding="ascii")
        require(content.endswith("\n"), f"truncated output: {name}")
        lines = content.splitlines()
        if not name.startswith("solute"):
            require_header(lines, "Program HYDRUS", name)
        last = next((line.strip() for line in reversed(lines) if line.strip()), "")
        if name == "BALANCE.OUT":
            match = re.fullmatch(r"Calculation time \[sec\]\s+(\S+)", last)
            require(match and number(match[1]) >= 0, "missing calculation-time terminal record")
        elif name == "I_CHECK.OUT":
            match = re.fullmatch(r"tPulse\s*=\s*(\S+)", last)
            require(
                match and matches(match[1], case["time_end"], 3), "invalid input terminal record"
            )
        else:
            require(last == "end", f"{name}: missing exact terminal end")
        texts[name], terminals[name] = lines, last
    require_header(texts["RUN_INF.OUT"], "TLevel", "RUN_INF.OUT")
    run_rows = []
    for line in texts["RUN_INF.OUT"]:
        fields = line.split()
        if fields and NUMBER.fullmatch(fields[0]):
            require(len(fields) == 11 and fields[8] == "T", "malformed/unconverged run record")
            require(all(number(fields[j]) >= 0 for j in (3, 4, 5, 9, 10)), "invalid diagnostics")
            for j in (0, 1, 2, 6, 7):
                number(fields[j])
            printed_format(fields[1], "e", 7)
            printed_format(fields[2], "e", 5)
            run_rows.append(fields)
    require(len(run_rows) == len(steps), "incomplete step inventory")
    for i, (fields, t) in enumerate(zip(run_rows, steps), 1):
        require(
            number(fields[0]) == i and matches(fields[1], t) and matches(fields[2], row["dt"]),
            f"unexpected step/time/dt at {i}",
        )
    require_header(texts["T_LEVEL.OUT"], "vTop", "T_LEVEL.OUT")
    water_rows = numeric_rows(texts["T_LEVEL.OUT"], 22, "T_LEVEL.OUT")
    require(len(water_rows) == len(steps), "incomplete water step inventory")
    theta_values, flux_values = [], []
    for i, (fields, t) in enumerate(zip(water_rows, steps), 1):
        printed_format(fields[0], "f", 4)
        for token in fields[1:19]:
            printed_format(token, "e", 5)
        require(number(fields[19]) == i and matches(fields[0], t, 4), "water step/time mismatch")
        for j in (3, 5):
            require(matches(fields[j], -case["q"]), "realized water flux mismatch")
            flux_values.append(-number(fields[j]))
        for j in (11, 13):
            require(matches(fields[j], case["hydraulics"]["head"]), "boundary head mismatch")
        require(matches(fields[16], case["theta"] * case["length"]), "water inventory mismatch")
    require_header(texts["NOD_INF.OUT"], "Conc(1..NS)", "NOD_INF.OUT")
    profiles, current = [], None
    for line in texts["NOD_INF.OUT"]:
        match = re.fullmatch(r"\s*Time:\s*(\S+)\s*", line)
        if match:
            current = [match[1], []]
            profiles.append(current)
        elif line.split() and NUMBER.fullmatch(line.split()[0]):
            require(current is not None, "node row outside profile")
            current[1].extend(numeric_rows([line], 11 + 2 * species_count, "NOD_INF.OUT"))
    require(len(profiles) == len(times), "missing/extra profile times")
    initial = initial_arrays(case, nodes)
    coordinates = grid(case, nodes)
    for (token, rows), t in zip(profiles, times):
        require(matches(token, t, 4) and len(rows) == nodes, "profile time/node inventory mismatch")
        for i, fields in enumerate(rows):
            for j, digits in ((1, 4), (2, 3), (3, 4)):
                printed_format(fields[j], "f", digits)
            for token in fields[4:8] + fields[11:]:
                printed_format(token, "e", 4)
            require(
                number(fields[0]) == i + 1 and matches(fields[1], -coordinates[i], 4),
                "node id/coordinate orientation mismatch",
            )
            for j, value in (
                (2, case["hydraulics"]["head"]),
                (3, case["theta"]),
                (4, case["hydraulics"]["ks"]),
                (6, -case["q"]),
                (7, 0),
            ):
                require(matches(fields[j], value), "nodal water invariant mismatch")
            require(
                all(number(v) == 0 for v in fields[11 + species_count :]),
                "inactive sorbed pool nonzero",
            )
            if t == 0:
                require(
                    all(matches(fields[11 + j], initial[j][i]) for j in range(species_count)),
                    "initial concentration mismatch",
                )
            theta_values.append(number(fields[3]))
            flux_values.append(-number(fields[6]))
    endpoint_times = None
    obs = texts["OBS_NODE.OUT"]
    obs_rows = numeric_rows(obs, 1 + 2 * (3 + species_count), "OBS_NODE.OUT")
    if row["role"] == "bridge":
        require(not obs_rows, "bridge acquired unreviewed observation nodes")
    else:
        require_header(obs, "Temp", "OBS_NODE.OUT")
        obs_ids = re.findall(r"Node\(\s*(\d+)\)", "\n".join(obs))
        require(obs_ids == ["1", str(nodes)], "endpoint observation ids/order mismatch")
        require(len(obs_rows) == len(steps), "incomplete endpoint history")
        for fields, t in zip(obs_rows, steps):
            printed_format(fields[0], "f", 4)
            for base in (1, 4 + species_count):
                for j, digits in ((0, 2), (1, 4), (2, 3)):
                    printed_format(fields[base + j], "f", digits)
                for token in fields[base + 3 : base + 3 + species_count]:
                    printed_format(token, "e", 4)
            require(matches(fields[0], t, 4), "endpoint time mismatch")
            for base in (1, 4 + species_count):
                require(
                    matches(fields[base], case["hydraulics"]["head"], 2)
                    and matches(fields[base + 1], case["theta"], 4)
                    and matches(fields[base + 2], 20, 3),
                    "endpoint water invariant mismatch",
                )
                theta_values.append(number(fields[base + 1]))
        endpoint_times = [0.0] + steps
    for j in range(species_count):
        name = f"solute{j+1}.out"
        require_header(texts[name], "cvTop", name)
        rows = numeric_rows(texts[name], 17, name)
        require(len(rows) == len(steps), f"{name}: incomplete species inventory")
        for i, (fields, t) in enumerate(zip(rows, steps), 1):
            printed_format(fields[0], "f", 4)
            for token in fields[1:13] + fields[14:]:
                printed_format(token, "e", 5)
            require(number(fields[13]) == i and matches(fields[0], t, 4), "solute step mismatch")
    balance = texts["BALANCE.OUT"]
    balance_times = [
        m[1] for line in balance if (m := re.fullmatch(r"\s*Time\s+\[T\]\s+(\S+)\s*", line))
    ]
    require(
        len(balance_times) == len(times)
        and all(matches(a, b, 4) for a, b in zip(balance_times, times)),
        "balance time inventory mismatch",
    )
    for label in ("ConcVol", "cMean"):
        records = [
            m
            for line in balance
            if (m := re.fullmatch(rf"\s*{label}\s+\[M/L[23]\]\s+(\d+)\s+(\S+)\s+(\S+)\s*", line))
        ]
        require(
            [int(m[1]) for m in records] == list(range(1, species_count + 1)) * len(times),
            f"incomplete balance {label} inventory",
        )
        for match in records:
            number(match[2])
            number(match[3])
    residuals = [
        m for line in balance if (m := re.fullmatch(r"\s*CncBalT\s+\[M\]\s+(\d+)\s+(\S+)\s*", line))
    ]
    require(
        [int(m[1]) for m in residuals] == list(range(1, species_count + 1)) * (len(times) - 1),
        "incomplete solute balance residual inventory",
    )
    for residual in residuals:
        number(residual[2])
    profile = texts["PROFILE.OUT"]
    require_header(profile, "Ks/KsTop", "PROFILE.OUT")
    rows = numeric_rows(profile, 11, "PROFILE.OUT")
    require(len(rows) == nodes, "incomplete hydraulic profile")
    for i, fields in enumerate(rows):
        require(
            number(fields[0]) == i + 1
            and matches(fields[1], coordinates[i], 2)
            and matches(fields[3], case["theta"], 3)
            and matches(fields[5], case["hydraulics"]["ks"]),
            "hydraulic profile mismatch",
        )
    check_input_echo(texts["I_CHECK.OUT"], case, row)
    # Source SOLUTE.FOR Disper: no gas, TauW=1, D=alpha*abs(q)/theta+Dw.
    c = case["chemistry"]
    dispersion = [
        c["dispersivity"] * abs(q) / th + c["molecular_diffusion"]
        for q in (min(flux_values), max(flux_values))
        for th in (min(theta_values), max(theta_values))
    ]
    require(
        all(math.isclose(d, case["dispersion"], rel_tol=2e-4, abs_tol=1e-12) for d in dispersion),
        "realized dispersion mismatch",
    )
    return (
        {
            "nodes": nodes,
            "species": species_count,
            "profile_times": times,
            "step_count": len(steps),
            "endpoint_times": endpoint_times,
            "terminal_records": terminals,
            "field_formats": dict(FIELD_FORMATS),
        },
        {
            "status": "passed",
            "theta_range": [min(theta_values), max(theta_values)],
            "q_range": [min(flux_values), max(flux_values)],
            "dispersion_range": [min(dispersion), max(dispersion)],
            "evidence": [
                "INPUT.FOR NodInf/ChemIn input echoes and hydraulic PROFILE validated",
                "NOD_INF theta/head/conductivity/flux and every T_LEVEL flux/head/volume validated",
                (
                    "OBS_NODE endpoint head/theta validated; lFlux=false, third field temperature"
                    if endpoint_times
                    else "legacy_bridge_no_obs"
                ),
                "SOLUTE.FOR Disper: D=dispersivity*abs(q)/theta+Dw, lTort=false; "
                "face q reconstructed from homogeneous saturated water, not solute flux",
            ],
        },
    )


def check_input_echo(lines, case, row):
    require_header(lines, "Nodal point information", "I_CHECK.OUT")
    begin = next((i for i, line in enumerate(lines) if line.strip().startswith("node ")), None)
    require(begin is not None, "input echo node header missing")
    end = next((i for i in range(begin + 1, len(lines)) if lines[i].strip() == "end"), None)
    require(end is not None, "input echo node terminal missing")
    rows = numeric_rows(lines[begin + 1 : end], 10 + len(case["species"]), "I_CHECK nodal")
    require(len(rows) == row["nodes"], "input echo node inventory mismatch")
    for i, fields in enumerate(rows):
        require(
            number(fields[0]) == i + 1 and matches(fields[1], -grid(case, row["nodes"])[i], 3),
            "input echo node identity mismatch",
        )
    bulk = next((i for i, line in enumerate(lines) if "Bulk.D." in line), None)
    require(bulk is not None and bulk + 1 < len(lines), "input echo bulk header missing")
    values = numeric_rows([lines[bulk + 1]], 5, "I_CHECK bulk")[0]
    require(
        all(
            matches(token, expected)
            for token, expected in zip(
                values, [1, case["rho"], case["chemistry"]["dispersivity"], 1, 0]
            )
        ),
        "bulk density/dispersivity echo mismatch",
    )
    for header, expected in (
        ("kTopCh", [-1] + case["boundary"]["inlet_concentration"]),
        ("kBotCh", [-1] + [0] * len(case["species"])),
    ):
        index = next((i for i, line in enumerate(lines) if line.strip().startswith(header)), None)
        require(index is not None and index + 1 < len(lines), "input echo boundary missing")
        values = numeric_rows([lines[index + 1]], len(expected), "I_CHECK boundary")[0]
        require(
            all(matches(token, value) for token, value in zip(values, expected)),
            "chemical boundary echo mismatch",
        )
    echoes = [i for i, line in enumerate(lines) if "Dif.w." in line]
    require(len(echoes) == len(case["species"]), "input echo species inventory mismatch")
    for j, i in enumerate(echoes):
        require(i + 3 < len(lines), "truncated chemical echo")
        molecular = numeric_rows([lines[i + 1]], 2, "I_CHECK diffusion")[0]
        require(
            matches(molecular[0], case["chemistry"]["molecular_diffusion"])
            and number(molecular[1]) == 0,
            "diffusion echo mismatch",
        )
        coefficients = numeric_rows([lines[i + 3]], 15, "I_CHECK chemistry")[0]
        require(
            number(coefficients[0]) == 1
            and all(
                matches(token, value)
                for token, value in zip(coefficients[1:], chemical_coefficients(case, j))
            ),
            "chemical coefficient echo mismatch",
        )


def artifact_inventory(run_dir):
    artifacts = {}
    for path in sorted(run_dir.rglob("*")):
        safe_path(run_dir, path.relative_to(run_dir))
        if not path.is_file() or path.name == "manifest.json":
            continue
        relative = str(path.relative_to(run_dir))
        key = (
            relative.split("\\", 1)[1]
            if relative.startswith("solute-input\\")
            and relative.split("\\", 1)[1] in OUTPUTS + tuple(f"solute{i}.out" for i in range(1, 4))
            else path.name if path.parent == run_dir / "inputs" else relative
        )
        artifacts[key] = {"path": relative, "sha256": sha256(path), "bytes": path.stat().st_size}
    return artifacts


def directory_bytes(root):
    total = 0
    for path in root.rglob("*"):
        safe_path(root, path.relative_to(root))
        if path.is_file():
            total += path.stat().st_size
    return total


def ledger_records(path):
    if not path.exists():
        return []
    safe_path(path.parent, path.name)
    content = path.read_text(encoding="ascii")
    require(not content or content.endswith("\n"), "truncated launch ledger")
    rows = [
        json.loads(line, object_pairs_hook=contract.unique_object) for line in content.splitlines()
    ]
    for row in rows:
        require(
            set(row) == {"event", "run_id", "launch_id", "utc", "detail"}, "invalid ledger keys"
        )
        require(
            row["event"] in {"start", "exit", "timeout", "validation_failed", "complete"},
            "unknown ledger event",
        )
    return rows


def append_event(task_root, event, manifest, detail):
    path = safe_path(task_root, "launch-ledger.jsonl")
    record = {
        "event": event,
        "run_id": manifest["run_id"],
        "launch_id": manifest["launch_id"],
        "utc": utc(),
        "detail": detail,
    }
    with path.open("a", encoding="ascii") as stream:
        stream.write(json.dumps(record, sort_keys=True, allow_nan=False) + "\n")
        stream.flush()
        os.fsync(stream.fileno())


def check_launch_budget(book, row):
    """Count failed starts too, across all study phase ledgers in this checkout."""
    starts = []
    for task in ("13.2.1", *PHASE_TASKS):
        starts.extend(
            r
            for r in ledger_records(ROOT / ".development" / task / "launch-ledger.jsonl")
            if r["event"] == "start"
        )
    by_id = {r["id"]: r for r in book["launches"]}
    require(all(r["launch_id"] in by_id for r in starts), "ledger launch absent from matrix")
    require(len(starts) < book["budgets"]["launch_cap"], "launch cap exhausted")
    require(not any(r["launch_id"] == row["id"] for r in starts), "launch row already consumed")
    require(
        sum(by_id[r["launch_id"]]["phase"] == row["phase"] for r in starts)
        < book["budgets"]["phase_caps"][row["phase"]],
        "phase launch cap exhausted",
    )
    reserved = sum(2 * by_id[r["launch_id"]]["estimated_output_bytes"] for r in starts)
    require(
        reserved + 2 * row["estimated_output_bytes"] <= book["budgets"]["storage_bytes_cap"],
        "matrix storage reservation exceeded",
    )


def run_solver(command, run_dir, timeout_seconds, byte_cap):
    """Capture streams directly and bound time/storage while the child is running."""
    with (run_dir / "stdout.txt").open("xb") as stdout, (run_dir / "stderr.txt").open(
        "xb"
    ) as stderr:
        process = subprocess.Popen(command, cwd=run_dir, stdout=stdout, stderr=stderr)
        deadline = time.monotonic() + timeout_seconds
        try:
            while True:
                require(directory_bytes(run_dir) <= byte_cap, "run storage cap exceeded")
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise subprocess.TimeoutExpired(command, timeout_seconds)
                try:
                    return process.wait(timeout=min(0.1, remaining))
                except subprocess.TimeoutExpired:
                    continue
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()


def new_manifest(case, row, run_dir, cases_digest, sources):
    return {
        "schema_version": "solute-manifest-v1",
        "run_id": run_dir.name,
        "launch_id": row["id"],
        "case_id": case["id"],
        "cases_sha256": cases_digest,
        "case_sha256": canonical_hash(case),
        "request_sha256": sha256(run_dir / "request.txt"),
        "solver": "fortran",
        "precision": row["precision"],
        "status": "planned",
        "command": [str(run_dir / "build/hydrus-reference"), "solute-input"],
        "cwd": str(run_dir),
        "started_utc": None,
        "ended_utc": None,
        "exit_code": None,
        "timeout_seconds": row["timeout_seconds"],
        "provenance": {
            "source_sha256": sources,
            "compiler": "not built (dry-run)",
            "flags": PRECISION_FLAGS[row["precision"]],
            "platform": platform.platform(),
            "executable_sha256": None,
            "shim_sha256": None,
            "adapter_sha256": {
                name: sha256(ROOT / name)
                for name in (
                    "tools/run_solute_verification_reference.py",
                    "tools/check_solute_verification_spec.py",
                    "tools/build_fortran_reference.sh",
                )
            },
            "input_sha256": {
                str(path.relative_to(run_dir)): sha256(path)
                for path in [
                    run_dir / "request.txt",
                    *[run_dir / "inputs" / n for n in INPUTS],
                    *[run_dir / n for n in ALIASES.values()],
                ]
            },
        },
        "artifacts": {},
        "record_inventory": {
            "nodes": row["nodes"],
            "species": len(case["species"]),
            "profile_times": [],
            "step_count": 0,
            "endpoint_times": [],
            "terminal_records": {},
            "field_formats": FIELD_FORMATS.copy(),
        },
        "warnings": [],
        "mapping_validation": {
            "status": "pending",
            "theta_range": [0, 0],
            "q_range": [0, 0],
            "dispersion_range": [0, 0],
            "evidence": ["No executable output validated."],
        },
    }


def prepare_run(book, cases_path, case, row, root, run_id):
    require(re.fullmatch(r"[A-Za-z0-9_.-]+", run_id) and run_id not in {".", ".."}, "unsafe run id")
    require(
        row["solver"] == "fortran" and row["precision"] in PRECISION_FLAGS, "unsupported solver"
    )
    require(
        row in book["launches"] and case in book["cases"] and case["id"] == row["case_id"],
        "case/launch absent from approved matrix",
    )
    output_root(root)
    snapshot = protected_hashes(book)
    sources = dict(contract.load_cases(ROOT / SOURCE_MANIFEST))
    sources.update(book["provenance"]["source_sha256"])
    run_dir = safe_path(root, run_id)
    run_dir.mkdir(parents=False, exist_ok=False)
    (run_dir / "inputs").mkdir()
    write_new(run_dir / "request.txt", request_bytes(case, row, run_id, sha256(cases_path)))
    for name, data in generate_decks(case, row).items():
        write_new(run_dir / "inputs" / name, data)
        if name in ALIASES:
            write_new(run_dir / ALIASES[name], data)
    manifest = new_manifest(case, row, run_dir, sha256(cases_path), sources)
    return run_dir, manifest, snapshot


def execute_run(book, case, row, run_dir, manifest, snapshot, task_root):
    """Finalize this new run once, retaining every failed attempt and its raw files."""
    started = False
    try:
        check_launch_budget(book, row)
        build = build_reference(run_dir, row["precision"], manifest["provenance"]["source_sha256"])
        manifest["provenance"]["source_sha256"].update(build.pop("staged_sha256"))
        manifest["provenance"].update(build)
        check_preserved(snapshot)
        check_launch_budget(book, row)
        require(
            not any((run_dir / ("solute-input\\" + n)).exists() for n in OUTPUTS),
            "stale outputs before launch",
        )
        for name, digest in manifest["provenance"]["input_sha256"].items():
            require(sha256(safe_path(run_dir, name)) == digest, "input changed before launch")
        for name, digest in manifest["provenance"]["adapter_sha256"].items():
            require(sha256(safe_path(ROOT, name)) == digest, "adapter changed before launch")
        for name, digest in manifest["provenance"]["source_sha256"].items():
            if name.startswith("build/"):
                require(
                    sha256(safe_path(run_dir, name)) == digest,
                    "staged source changed before launch",
                )
        require(
            sha256(run_dir / "build/hydrus-reference") == build["executable_sha256"],
            "executable hash changed before launch",
        )
        require(
            directory_bytes(run_dir) < 2 * row["estimated_output_bytes"], "insufficient run storage"
        )
        manifest["started_utc"] = utc()
        manifest["status"] = "started"
        append_event(
            task_root,
            "start",
            manifest,
            {
                "timeout_seconds": row["timeout_seconds"],
                "precision": row["precision"],
                "estimated_output_bytes": row["estimated_output_bytes"],
            },
        )
        started = True
        started_ns = time.time_ns()
        code = run_solver(
            manifest["command"], run_dir, row["timeout_seconds"], 2 * row["estimated_output_bytes"]
        )
        manifest["exit_code"] = code
        append_event(task_root, "exit", manifest, {"exit_code": code})
        require(code == 0, f"solver exited {code}")
        manifest["record_inventory"], manifest["mapping_validation"] = validate_outputs(
            run_dir, case, row, started_ns
        )
        require(
            directory_bytes(run_dir) <= 2 * row["estimated_output_bytes"],
            "run storage cap exceeded",
        )
        check_preserved(snapshot)
        manifest["status"] = "raw_complete"
    except subprocess.TimeoutExpired as error:
        manifest["status"] = "timeout"
        manifest["warnings"].append(str(error))
        append_event(
            task_root,
            "timeout",
            manifest,
            {"stage": "solver" if started else "build", "timeout_seconds": error.timeout},
        )
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        manifest["status"] = "failed"
        manifest["mapping_validation"]["status"] = "failed"
        manifest["mapping_validation"]["evidence"] = [str(error)]
        manifest["warnings"].append(str(error))
        append_event(
            task_root,
            "validation_failed",
            manifest,
            {
                "stage": "solver_or_validation" if started else "build_or_preflight",
                "error": str(error),
            },
        )
    finally:
        manifest["ended_utc"] = utc()
        for name in ("stdout.txt", "stderr.txt"):
            path = run_dir / name
            if path.exists():
                manifest["warnings"].extend(
                    line
                    for line in path.read_text(errors="replace").splitlines()
                    if re.search(r"warning|exception|underflow|denormal", line, re.I)
                )
        manifest["artifacts"] = artifact_inventory(run_dir)
        write_json(run_dir / "manifest.json", manifest)
        if manifest["status"] == "raw_complete":
            append_event(
                task_root,
                "complete",
                manifest,
                {"status": "raw_complete", "manifest_sha256": sha256(run_dir / "manifest.json")},
            )
        check_preserved(snapshot)
    return manifest


def verify_run(run_dir, book, cases_path):
    """Read-only verification; never stage, compile, execute or rewrite a manifest."""
    manifest = contract.load_cases(safe_path(run_dir, "manifest.json"))
    require(
        set(manifest) - {"backend"} == MANIFEST_KEYS and manifest["schema_version"] == "solute-manifest-v1",
        "invalid manifest schema",
    )
    rows = [r for r in book["launches"] if r["id"] == manifest["launch_id"]]
    require(len(rows) == 1, "unknown manifest launch")
    row = rows[0]
    case = next(c for c in book["cases"] if c["id"] == row["case_id"])
    require(
        manifest["case_id"] == case["id"]
        and manifest["run_id"] == run_dir.name
        and manifest["cases_sha256"] == sha256(cases_path)
        and manifest["case_sha256"] == canonical_hash(case),
        "manifest identity mismatch",
    )
    require(
        manifest["solver"] == "fortran"
        and manifest["precision"] == row["precision"]
        and manifest["timeout_seconds"] == row["timeout_seconds"],
        "manifest solver controls mismatch",
    )
    protected_hashes(book)
    sources = dict(contract.load_cases(ROOT / SOURCE_MANIFEST))
    sources.update(book["provenance"]["source_sha256"])
    provenance = manifest["provenance"]
    require(provenance["flags"] == PRECISION_FLAGS[row["precision"]], "unreviewed flags")
    for name, digest in sources.items():
        require(provenance["source_sha256"].get(name) == digest, "source provenance mismatch")
    for name, digest in provenance["source_sha256"].items():
        root = run_dir if name.startswith("build/") else ROOT
        require(sha256(safe_path(root, name)) == digest, f"source hash mismatch: {name}")
    for name, digest in provenance["adapter_sha256"].items():
        require(sha256(safe_path(ROOT, name)) == digest, f"adapter hash mismatch: {name}")
    current_artifacts = artifact_inventory(run_dir)
    for name in ("observations.json", "normalization.json"):
        if name not in manifest["artifacts"]:
            current_artifacts.pop(name, None)
    require(manifest["artifacts"] == current_artifacts, "artifact inventory/hash mismatch")
    for name, digest in provenance["input_sha256"].items():
        require(sha256(safe_path(run_dir, name)) == digest, "input hash mismatch")
    expected_request = request_bytes(case, row, run_dir.name, sha256(cases_path))
    require(
        (run_dir / "request.txt").read_bytes() == expected_request
        and manifest["request_sha256"] == sha256(run_dir / "request.txt"),
        "request mismatch",
    )
    for name, data in generate_decks(case, row).items():
        require((run_dir / "inputs" / name).read_bytes() == data, "deck byte mismatch")
        if name in ALIASES:
            require((run_dir / ALIASES[name]).read_bytes() == data, "runtime alias byte mismatch")
    require(
        manifest["status"] in {"planned", "raw_complete"}, "failed/incomplete run cannot verify"
    )
    if manifest["status"] == "raw_complete":
        require(
            manifest["exit_code"] == 0 and manifest["started_utc"] and manifest["ended_utc"],
            "invalid completed lifecycle",
        )
        require(
            sha256(run_dir / "build/hydrus-reference") == provenance["executable_sha256"]
            and sha256(run_dir / "build/gnu_compatibility.FOR") == provenance["shim_sha256"],
            "executable/shim hash mismatch",
        )
        inventory, mapping = validate_outputs(run_dir, case, row)
        require(
            inventory == manifest["record_inventory"] and mapping == manifest["mapping_validation"],
            "raw validation differs from manifest",
        )
    return manifest


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output-root", required=True)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--dry-run", action="store_true")
    modes.add_argument("--execute", action="store_true")
    modes.add_argument("--verify-only", action="store_true")
    parser.add_argument("--launch-id", action="append")
    args = parser.parse_args(argv)
    try:
        book = contract.validate(contract.load_cases(args.cases))
        root, task_root = output_root(args.output_root)
        selected = [
            r
            for r in book["launches"]
            if r["solver"] == "fortran" and (args.launch_id is None or r["id"] in args.launch_id)
        ]
        require(
            selected and (args.launch_id is None or len(selected) == len(set(args.launch_id))),
            "unknown/non-Fortran launch selection",
        )
        if args.execute:
            require(task_root.name in PHASE_TASKS, "solver execution forbidden in adapter task")
            require(
                args.launch_id is not None,
                "execution requires explicit launch ids after producer gates",
            )
            require(
                all(r["phase"] == PHASE_TASKS[task_root.name] for r in selected),
                "launch phase does not match executing task",
            )
        if args.verify_only:
            require(root.is_dir(), "verification root missing")
            manifests = sorted(root.glob("*/manifest.json"))
            require(manifests, "no manifests to verify")
            verified = 0
            for path in manifests:
                if contract.load_cases(path)["launch_id"] in {r["id"] for r in selected}:
                    verify_run(path.parent, book, args.cases)
                    verified += 1
            require(verified, "no selected manifests to verify")
            print(f"Verified {verified} immutable manifests; no solver launches.")
            return 0
        root.mkdir(parents=True, exist_ok=True)
        # One process may consume a phase ledger at a time, including failed attempts.
        with safe_path(task_root, "launch.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            failed = False
            for row in selected:
                case = next(c for c in book["cases"] if c["id"] == row["case_id"])
                run_id = row["id"] + "-" + uuid.uuid4().hex[:12]
                run_dir, manifest, snapshot = prepare_run(book, args.cases, case, row, root, run_id)
                if args.execute:
                    manifest = execute_run(book, case, row, run_dir, manifest, snapshot, task_root)
                    failed = manifest["status"] != "raw_complete"
                else:
                    manifest["artifacts"] = artifact_inventory(run_dir)
                    write_json(run_dir / "manifest.json", manifest)
                    check_preserved(snapshot)
                print(f"{manifest['status']}: {run_dir}")
                if failed:
                    break
        return 1 if failed else 0
    except (OSError, ValueError, KeyError, StopIteration) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

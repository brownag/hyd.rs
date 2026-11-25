#!/usr/bin/env python3
"""Produce source-matched vapor trajectory fixtures from an unchanged Fortran build."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import math
import re
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / "reference/vapor_fortran/cases"
FIXTURES = ROOT / "reference/vapor_fortran/fixtures"
TIME_RE = re.compile(r"^\s*Time:\s*(\S+)\s*$", re.I)
BAL_TIME_RE = re.compile(r"^\s*Time\s+\[T\]\s+(\S+)\s*$", re.I)
CALC_TIME_RE = re.compile(r"^\s*Calculation time \[sec\]\s+(\S+)\s*$", re.I)

BAL_PATTERNS = [
    ("area", re.compile(r"^\s*Area\s+\[L\]\s+(.*)$", re.I)),
    ("w_volume", re.compile(r"^\s*W-volume\s+\[L\]\s+(.*)$", re.I)),
    ("w_volume_im", re.compile(r"^\s*W-volumeI\s*\[L\]\s+(.*)$", re.I)),
    ("in_flow", re.compile(r"^\s*In-flow\s+\[L/T\]\s+(.*)$", re.I)),
    ("h_mean", re.compile(r"^\s*h\s+Mean\s+\[L\]\s+(.*)$", re.I)),
    ("heat_vol", re.compile(r"^\s*HeatVol\s+\[M/T2\]\s+(.*)$", re.I)),
    ("t_mean", re.compile(r"^\s*tMean\s+\[K\]\s+(.*)$", re.I)),
    ("top_flux", re.compile(r"^\s*Top\s+Flux\s+\[L/T\]\s+(.*)$", re.I)),
    ("bot_flux", re.compile(r"^\s*Bot\s+Flux\s+\[L/T\]\s+(.*)$", re.I)),
    ("wat_bal_t", re.compile(r"^\s*WatBalT\s+\[L\]\s+(.*)$", re.I)),
    ("wat_bal_r", re.compile(r"^\s*WatBalR\s+\[%\]\s+(.*)$", re.I)),
]

T_LEVEL_FORMAT_170 = "format 170: f13.4,11e13.5,2e13.5,5e13.5,i7,e13.5,f11.3"
T_LEVEL_HEADER_RE = re.compile(
    r"^\s*Time\s+rTop\s+rRoot\s+vTop\s+vRoot\s+vBot\b.*\bTLevel\b.*\bSnowLayer\s*$"
)
T_LEVEL_FIELDS = (
    ("time", 0, 13, "float"),
    ("r_top", 13, 26, "float"),
    ("r_root", 26, 39, "float"),
    ("v_top", 39, 52, "float"),
    ("v_root", 52, 65, "float"),
    ("v_bot", 65, 78, "float"),
    ("sum_r_top", 78, 91, "float"),
    ("sum_r_root", 91, 104, "float"),
    ("sum_v_top", 104, 117, "float"),
    ("sum_v_root", 117, 130, "float"),
    ("sum_v_bot", 130, 143, "float"),
    ("h_top", 143, 156, "float"),
    ("h_root", 156, 169, "float"),
    ("h_bot", 169, 182, "float"),
    ("run_off", 182, 195, "float"),
    ("sum_run_off", 195, 208, "float"),
    ("volume", 208, 221, "float"),
    ("sum_infil", 221, 234, "float"),
    ("sum_evap", 234, 247, "float"),
    ("t_level", 247, 254, "int"),
    ("cum_wtrans", 254, 267, "float"),
    ("snow_layer", 267, 278, "float"),
)
T_LEVEL_ROW_WIDTH = 278


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def deterministic_gzip(payload: bytes) -> bytes:
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as stream:
        stream.write(payload)
    return output.getvalue()


def finite_number(text: str, label: str) -> float:
    try:
        value = float(text.replace("D", "E").replace("d", "e"))
    except ValueError as exc:
        raise ValueError(f"malformed {label} field {text!r}") from exc
    if not math.isfinite(value):
        raise ValueError(f"nonfinite {label} field {text!r}")
    return value


def parse_blocks(text: str, vapor: bool = False) -> list[dict]:
    """Parse NodOut's fixed-width time blocks; nodes remain source top-first."""
    blocks, current = [], None
    seen_times = set()
    fields = ("con", "con_lt", "con_vh", "con_vt", "v_liquid", "v_vapor", "v_total", "v_vap_iso", "v_vap_term")
    for raw in text.splitlines():
        match = TIME_RE.match(raw)
        if match:
            if current is not None:
                raise ValueError("Time block started before previous block ended")
            t = finite_number(match.group(1), "time")
            if t in seen_times:
                raise ValueError(f"duplicate time {t}")
            seen_times.add(t)
            current = {"time": t, "nodes": []}
            continue
        if current is None:
            continue
        if raw.strip().lower() == "end":
            if not current["nodes"]:
                raise ValueError("empty time block")
            blocks.append(current)
            current = None
            continue
        node_text = raw[:4].strip()
        if not node_text.isdigit():
            continue
        if len(raw) < (133 if vapor else 111):
            raise ValueError("truncated fixed-width output row")
        if "*" in raw:
            raise ValueError("Fortran field overflow in output row")
        node = int(node_text)
        if any(r["node"] == node for r in current["nodes"]):
            raise ValueError(f"duplicate node {node} at time {current['time']}")
        depth = finite_number(raw[5:15], "depth")
        row = {"node": node, "depth": depth}
        if vapor:
            row.update(dict(zip(fields, [finite_number(raw[16 + 13*i:29 + 13*i], fields[i]) for i in range(9)])))
        else:
            row.update(head=finite_number(raw[16:27], "head"), moisture=finite_number(raw[28:34], "moisture"), temperature=finite_number(raw[103:111], "temperature"))
        current["nodes"].append(row)
    if current is not None:
        raise ValueError("time block missing end marker")
    if not blocks:
        raise ValueError("output contains no Time blocks")
    identity = [(r["node"], r["depth"]) for r in blocks[0]["nodes"]]
    if len({r["node"] for r in blocks[0]["nodes"]}) != len(identity):
        raise ValueError("duplicate node identity")
    for block in blocks[1:]:
        if [(r["node"], r["depth"]) for r in block["nodes"]] != identity:
            raise ValueError("missing, reordered, or changed node identities across times")
    return blocks


def parse_balance(source: str | Path) -> list[dict]:
    """Parse BALANCE.OUT blocks; returns per-time-step balance rows."""
    if isinstance(source, Path):
        if not source.is_file() or not source.stat().st_size:
            raise RuntimeError(f"required output absent/empty: {source.name}")
        text = source.read_text(errors="strict")
    else:
        text = source
    if not text or not text.endswith("\n"):
        raise ValueError("malformed or truncated BALANCE.OUT")

    lines = text.splitlines()
    nonempty = [line.strip() for line in lines if line.strip()]
    if not nonempty:
        raise ValueError("empty BALANCE.OUT")
    m_term = CALC_TIME_RE.match(nonempty[-1])
    if not m_term:
        raise ValueError("missing or malformed calculation-time completion record in BALANCE.OUT")
    calc_time = finite_number(m_term.group(1), "calculation time")
    if calc_time < 0:
        raise ValueError("negative calculation time")

    blocks, current, seen_times = [], None, set()
    for line in lines:
        stripped = line.strip()
        m_time = BAL_TIME_RE.match(line)
        if m_time:
            if current is not None:
                raise ValueError("Time block started before previous block ended")
            t = finite_number(m_time.group(1), "time")
            if t in seen_times:
                raise ValueError(f"duplicate balance time {t}")
            seen_times.add(t)
            current = {"time": t, "raw": {}, "sub_regions": {}}
            continue
        if current is None:
            continue
        if "*" in line:
            raise ValueError("Fortran field overflow in output row")
        if stripped.startswith("Sub-region num."):
            current["sub_regions_idx"] = [int(tok) for tok in stripped.split()[2:] if tok.isdigit()]
            continue
        if stripped.startswith("---"):
            if current["raw"]:
                req = ("area", "w_volume", "in_flow", "h_mean", "top_flux", "bot_flux")
                for k in req:
                    if k not in current["raw"]:
                        raise ValueError(f"truncated balance block at time {current['time']}: missing {k}")
                if current["time"] > 0 and "wat_bal_t" not in current["raw"]:
                    raise ValueError(f"truncated balance block at time {current['time']}: missing wat_bal_t")

                row = {
                    "time": current["time"],
                    "area": current["raw"]["area"][0],
                    "w_volume": current["raw"]["w_volume"][0],
                    "in_flow": current["raw"]["in_flow"][0],
                    "h_mean": current["raw"]["h_mean"][0],
                    "top_flux": current["raw"]["top_flux"][0],
                    "bot_flux": current["raw"]["bot_flux"][0],
                    "wat_bal_t": current["raw"]["wat_bal_t"][0] if "wat_bal_t" in current["raw"] else None,
                    "wat_bal_r": current["raw"]["wat_bal_r"][0] if "wat_bal_r" in current["raw"] else None,
                }
                if "heat_vol" in current["raw"]:
                    row["heat_vol"] = current["raw"]["heat_vol"][0]
                if "t_mean" in current["raw"]:
                    row["t_mean"] = current["raw"]["t_mean"][0]
                if "w_volume_im" in current["raw"]:
                    row["w_volume_im"] = current["raw"]["w_volume_im"][0]
                row["sub_regions"] = current["sub_regions"]
                blocks.append(row)
                current = None
            continue

        for key, pat in BAL_PATTERNS:
            m = pat.match(line)
            if m:
                tokens = [finite_number(tok, key) for tok in m.group(1).split()]
                if not tokens:
                    raise ValueError(f"empty tokens for {key} at time {current['time']}")
                current["raw"][key] = tokens
                if len(tokens) > 1:
                    current["sub_regions"][key] = tokens[1:]
                break

    if current is not None:
        raise ValueError("balance block missing end marker")
    if not blocks:
        raise ValueError("BALANCE.OUT contains no Time blocks")
    return blocks


def parse_t_level(source: str | Path) -> list[dict]:
    """Parse T_LEVEL.OUT format-170 rows after the format-130 header."""
    if isinstance(source, Path):
        if not source.is_file() or not source.stat().st_size:
            raise RuntimeError(f"required output absent/empty: {source.name}")
        text = source.read_text(errors="strict")
    else:
        text = source
    if not text:
        raise ValueError("empty T_LEVEL.OUT")

    rows: list[dict] = []
    in_table = False
    ended = False
    prev_time: float | None = None
    prev_t_level: int | None = None
    for raw in text.splitlines():
        stripped = raw.strip()
        if not in_table:
            if T_LEVEL_HEADER_RE.match(raw):
                in_table = True
            continue
        if not stripped or stripped.startswith("[T]"):
            continue
        if stripped.lower() == "end":
            ended = True
            break
        if "*" in raw:
            raise ValueError("Fortran field overflow in T_LEVEL.OUT row")
        if len(raw) < T_LEVEL_ROW_WIDTH:
            raise ValueError(f"truncated T_LEVEL.OUT row (len={len(raw)} < {T_LEVEL_ROW_WIDTH})")
        row: dict[str, float | int] = {}
        for name, start, end, kind in T_LEVEL_FIELDS:
            cell = raw[start:end]
            if kind == "int":
                try:
                    row[name] = int(cell.strip())
                except ValueError as exc:
                    raise ValueError(f"malformed {name} field {cell!r}") from exc
            else:
                row[name] = finite_number(cell, name)
        t = row["time"]
        tl = row["t_level"]
        if prev_time is not None and t <= prev_time:
            raise ValueError(f"duplicate or non-increasing time {t} after {prev_time}")
        if prev_t_level is not None and tl != prev_t_level + 1:
            raise ValueError(f"t_level must increase by 1: got {tl} after {prev_t_level}")
        prev_time = float(t)
        prev_t_level = int(tl)
        rows.append(row)

    if not in_table:
        raise ValueError("missing T_LEVEL.OUT format-130 header")
    if not ended:
        raise ValueError("T_LEVEL.OUT missing standalone end marker")
    if not rows:
        raise ValueError("T_LEVEL.OUT contains no data rows")
    return rows


def load_build(directory: Path) -> dict:
    build_path, staged_path = directory / "build-manifest.json", directory / "staging-manifest.json"
    build, staged = json.loads(build_path.read_text()), json.loads(staged_path.read_text())
    exe = directory / "hydrus-reference"
    if sha256(exe) != build["executable_sha256"]:
        raise RuntimeError("reference executable hash mismatch")
    return {"build": build, "staged": staged, "exe": exe, "build_manifest_sha256": sha256(build_path), "staging_manifest_sha256": sha256(staged_path)}


MET_CASE_ID = "vapor_met_bare_soil_on"
# Daily records: tAtm [h], Rad [MJ/m2/d], TMax, TMin [C], RHMean [%], Wind [km/d], SunHours [h].
MET_DAYS = [(24 * (i + 1), *row) for i, row in enumerate([
    (24.0, 34.0, 20.0, 35.0, 250.0, 11.0), (25.0, 35.0, 21.0, 32.0, 250.0, 11.5),
    (26.0, 36.0, 22.0, 30.0, 260.0, 12.0), (25.0, 35.0, 21.0, 32.0, 250.0, 11.5),
    (24.0, 34.0, 20.0, 35.0, 240.0, 11.0), (10.0, 24.0, 18.0, 85.0, 150.0, 2.0),
    (22.0, 30.0, 19.0, 50.0, 200.0, 9.0), (25.0, 34.0, 20.0, 35.0, 250.0, 11.5),
    (26.0, 36.0, 22.0, 30.0, 260.0, 12.0), (25.0, 35.0, 21.0, 32.0, 250.0, 11.5)])]
MET_PREC_CM_H = {144: 0.1}  # rain on day 6 only; all other records dry


def generate_met_case(dest: Path) -> None:
    """Write the bare-soil Penman-Monteith case deck (lMeteo=t, lVapor=t, atmospheric top BC)."""
    wet = CASES / "vapor_flux_fixedhead_wet_on"
    sel = (wet / "SELECTOR.IN").read_text()
    for old, new in (("Heat-only derived deck for source-matched fixture production. Not a manual deck.", "Meteo bare-soil derived deck for source-matched fixture production. Not a manual deck."),
                     ("t f t f f f f f f f", "t f t f f f f f t f"), ("f f f t f f", "f f t t f f"),
                     ("TopInf WLayer KodTop InitCond\nf f -1 f", "TopInf WLayer KodTop InitCond\nt f -4 f"),
                     ("rTop rBot rRoot\n-1.0e-4 0.0 0.0\n", ""), ("MPL\n0.1 0.01 0.5 1.2 0.5 1 20 4", "MPL\n0.1 0.01 0.5 1.2 0.5 1 20 10"),
                     ("lEnter\nf 1 6 f", "lEnter\nf 1 24 f"), ("6 24 72 240", " ".join(str(24 * i) for i in range(1, 11)))):
        if sel.count(old) != 1:
            raise ValueError(f"template drift: {old!r}")
        sel = sel.replace(old, new)
    atm = ["Pcp_File_Version=4", "*** BLOCK I: ATMOSPHERIC INFORMATION ***********************************", "MaxAL", str(len(MET_DAYS)),
           "DailyVar SinusVar lLAI lBCCycles lInterc", "f f f f f", "hCritS", "0",
           "tAtm Prec rSoil rRoot hCritA rB hB ht tTop tBot Ampl"]
    atm += [f"{d[0]} {MET_PREC_CM_H.get(d[0], 0.0)} 0 0 1000 0 0 0 25 15 0" for d in MET_DAYS] + ["end", "*** END OF INPUT FILE 'ATMOSPH.IN' **********************************"]
    met = ["Pcp_File_Version=4", "*** BLOCK A: METEOROLOGICAL INFORMATION ****************************",
           "MaxAL iRadiation lHargr", f"{len(MET_DAYS)} 1 f", "lEnBal lMetDaily", "f f", "Latitude Altitude", "-30.0 100.0",
           "ShortWaveRadA ShortWaveRadB", "0.25 0.5", "LongWaveRadA LongWaveRadB", "0.9 0.1", "LongWaveRadA1 LongWaveRadB1", "0.34 -0.139",
           "WindHeight TempHeight [cm]", "200. 190.", "iCrop iSunSh iRelHum", "0 0 0", "Albedo", "0.23",
           "*** BLOCK B: DAILY METEOROLOGICAL RECORDS ****************************", "tAtm[h] Rad[MJ/m2/d] TMax[C] TMin[C] RHMean[%] Wind[km/d] SunHours[h]", "records follow"]
    met += [" ".join(str(v) for v in d) for d in MET_DAYS] + ["end", "*** END OF INPUT FILE 'Meteo.in' **********************************"]
    spec = {"case_id": MET_CASE_ID, "runner": {
        "inputs": ["LEVEL_01.DIR", "SELECTOR.IN", "PROFILE.DAT", "ATMOSPH.IN", "Meteo.in"],
        "outputs": [{"name": "NOD_INF.OUT", "marker": "end"}, {"name": "NOD_INF_V.OUT", "marker": "end"}],
        "aliases": {"SELECTOR.IN": "Selector.in", "PROFILE.DAT": "Profile.dat", "ATMOSPH.IN": "ATMOSPH.IN", "Meteo.in": "Meteo.in"},
        "data_prefix": MET_CASE_ID}}
    dest.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(wet / "LEVEL_01.DIR", dest / "LEVEL_01.DIR")
    shutil.copyfile(wet / "PROFILE.DAT", dest / "PROFILE.DAT")
    (dest / "SELECTOR.IN").write_text(sel)
    (dest / "ATMOSPH.IN").write_text("\n".join(atm) + "\n")
    (dest / "Meteo.in").write_text("\n".join(met) + "\n")
    (dest / "fixture.json").write_text(json.dumps(spec, indent=1) + "\n")


def run_case(case_id: str, task_base: Path, build_dir: Path, timeout: int | None, balance: bool = False, tlevel: bool = False) -> Path:
    case_dir = CASES / case_id
    spec = json.loads((case_dir / "fixture.json").read_text())
    if spec.get("case_id") != case_id:
        raise ValueError(f"case id mismatch in {case_dir}")
    config = spec["runner"]
    produce_balance = balance or any(e["name"] == "BALANCE.OUT" for e in config["outputs"])
    produce_tlevel = tlevel or any(e["name"] == "T_LEVEL.OUT" for e in config["outputs"])
    sidecar_mode = produce_balance or produce_tlevel
    output_path = FIXTURES / f"{case_id}.json"
    balance_path = FIXTURES / f"{case_id}_balance.json"
    tlevel_path = FIXTURES / f"{case_id}_tlevel.json.gz"
    if produce_balance and balance_path.exists():
        raise FileExistsError(f"refusing to overwrite balance fixture: {balance_path}")
    if produce_tlevel and tlevel_path.exists():
        raise FileExistsError(f"refusing to overwrite tlevel fixture: {tlevel_path}")
    if not sidecar_mode and output_path.exists():
        raise FileExistsError(f"refusing to overwrite fixture: {output_path}")
    run_root = task_base / case_id
    if run_root.exists():
        raise FileExistsError(f"refusing to overwrite run root: {run_root}")
    run_dir = run_root / "run"
    committed = run_root / "committed-inputs"
    run_dir.mkdir(parents=True)
    committed.mkdir()
    hashes = {}
    for name in config["inputs"]:
        src, dst = case_dir / name, committed / name
        shutil.copyfile(src, dst)
        if src.read_bytes() != dst.read_bytes():
            raise RuntimeError(f"input copy differs: {name}")
        hashes[name] = sha256(dst)
    prefix = config["data_prefix"]
    for name, alias in config.get("aliases", {}).items():
        shutil.copyfile(committed / name, run_dir / f"{prefix}\\{alias}")
    shutil.copyfile(committed / "LEVEL_01.DIR", run_dir / f"{prefix}\\LEVEL_01.DIR")
    command = [str(build_dir / "hydrus-reference"), prefix]
    done = subprocess.run(command, cwd=run_dir, text=True, capture_output=True, timeout=timeout)
    (run_root / "stdout.txt").write_text(done.stdout)
    (run_root / "stderr.txt").write_text(done.stderr)
    if done.returncode:
        raise RuntimeError(f"{case_id}: executable exited {done.returncode}; retained at {run_root}")
    if (run_dir / f"{prefix}\\Error.msg").exists():
        raise RuntimeError(f"{case_id}: source reported Error.msg; retained at {run_root}")
    outputs, parsed = {}, {}
    outputs_to_check = list(config["outputs"])
    if produce_balance and not any(e["name"] == "BALANCE.OUT" for e in outputs_to_check):
        outputs_to_check.append({"name": "BALANCE.OUT", "marker": "Calculation time"})
    if produce_tlevel and not any(e["name"] == "T_LEVEL.OUT" for e in outputs_to_check):
        outputs_to_check.append({"name": "T_LEVEL.OUT", "marker": "end"})
    for entry in outputs_to_check:
        path = run_dir / f"{prefix}\\{entry['name']}"
        if not path.is_file() or not path.stat().st_size:
            raise RuntimeError(f"{case_id}: required output absent/empty: {entry['name']}")
        raw = path.read_text(errors="strict")
        if not raw.endswith("\n") or entry["marker"].lower() not in raw.lower():
            raise ValueError(f"{case_id}: malformed or truncated {entry['name']}")
        outputs[entry["name"]] = path
        if entry["name"] == "BALANCE.OUT":
            parsed[entry["name"]] = parse_balance(raw)
        elif entry["name"] == "T_LEVEL.OUT":
            parsed[entry["name"]] = parse_t_level(raw)
        else:
            parsed[entry["name"]] = parse_blocks(raw, entry["name"] == "NOD_INF_V.OUT")
    main_blocks = parsed["NOD_INF.OUT"]
    vapor_blocks = parsed.get("NOD_INF_V.OUT")
    if vapor_blocks and [b["time"] for b in vapor_blocks] != [b["time"] for b in main_blocks]:
        raise ValueError(f"{case_id}: vapor and nodal output times differ")
    selector = (case_dir / "SELECTOR.IN").read_text()
    enabled = "f f f t f f" in selector or "f f t t f f" in selector
    if enabled != bool(vapor_blocks):
        raise ValueError(f"{case_id}: selector vapor reachability disagrees with output availability")
    staged, build = load_build(build_dir)["staged"], load_build(build_dir)["build"]
    info = load_build(build_dir)
    outputs_for_main = {n: p for n, p in outputs.items() if n not in ("BALANCE.OUT", "T_LEVEL.OUT")}
    if not output_path.exists():
        record = {
            "schema_version": 1, "case_id": case_id, "source_matched": True,
            "evidence_class": "pinned-fortran-trajectory", "vapor_output_available": bool(vapor_blocks),
            "source_input_mapping": {"selector": "SELECTOR.IN", "profile": "PROFILE.DAT", "level_file": "LEVEL_01.DIR"},
            "time_points": [b["time"] for b in main_blocks],
            "node_numbers": [r["node"] for r in main_blocks[0]["nodes"]],
            "node_depths": [r["depth"] for r in main_blocks[0]["nodes"]],
            "head": [[r["head"] for r in b["nodes"]] for b in main_blocks],
            "liquid_moisture": [[r["moisture"] for r in b["nodes"]] for b in main_blocks],
            "temperature_c": [[r["temperature"] for r in b["nodes"]] for b in main_blocks],
            "vapor_fields": ({name: [[r[name] for r in b["nodes"]] for b in vapor_blocks] for name in vapor_blocks[0]["nodes"][0] if name not in ("node", "depth")} if vapor_blocks else None),
            "output_formats": {"NOD_INF.OUT": "source formats 120/130; head f11.3/e11.4, moisture f6.4, temperature f8.2", "NOD_INF_V.OUT": "format 140: i4, f10.4, 9e13.5" if vapor_blocks else "unavailable: vapor disabled"},
            "manifest": {"source_and_archive_sha256": staged["source_and_archive_sha256"], "patch_sha256": build["patch_sha256"], "executable_sha256": build["executable_sha256"], "build_manifest_sha256": info["build_manifest_sha256"], "staging_manifest_sha256": info["staging_manifest_sha256"], "compiler": build["gnu_fortran_version"], "host_os": build["host_os"], "host_architecture": build["host_architecture"], "flags": build["flags"], "command": command, "input_sha256": hashes, "output_sha256": {n: sha256(p) for n,p in outputs_for_main.items()}, "selector_sha256": hashlib.sha256(selector.encode()).hexdigest(), "profile_sha256": sha256(case_dir / "PROFILE.DAT"), "runner_sha256": sha256(Path(__file__))}
        }
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")
    if produce_balance:
        balance_blocks = parsed["BALANCE.OUT"]
        balance_record = {
            "schema_version": 1, "case_id": case_id, "source_matched": True,
            "evidence_class": "pinned-fortran-balance",
            "time_points": [b["time"] for b in balance_blocks],
            "balance_rows": balance_blocks,
            "manifest": {"source_and_archive_sha256": staged["source_and_archive_sha256"], "patch_sha256": build["patch_sha256"], "executable_sha256": build["executable_sha256"], "build_manifest_sha256": info["build_manifest_sha256"], "staging_manifest_sha256": info["staging_manifest_sha256"], "compiler": build["gnu_fortran_version"], "host_os": build["host_os"], "host_architecture": build["host_architecture"], "flags": build["flags"], "command": command, "input_sha256": hashes, "output_sha256": {"BALANCE.OUT": sha256(outputs["BALANCE.OUT"])}, "selector_sha256": hashlib.sha256(selector.encode()).hexdigest(), "profile_sha256": sha256(case_dir / "PROFILE.DAT"), "runner_sha256": sha256(Path(__file__))}
        }
        balance_path.parent.mkdir(parents=True, exist_ok=True)
        balance_path.write_text(json.dumps(balance_record, indent=2, sort_keys=True) + "\n")
    if produce_tlevel:
        tlevel_rows = parsed["T_LEVEL.OUT"]
        tlevel_outputs = {n: sha256(outputs[n]) for n in ("T_LEVEL.OUT", "NOD_INF.OUT", "NOD_INF_V.OUT") if n in outputs}
        tlevel_record = {
            "schema_version": 1, "case_id": case_id, "source_matched": True,
            "evidence_class": "pinned-fortran-t-level",
            "output_format": T_LEVEL_FORMAT_170,
            "rows": tlevel_rows,
            "manifest": {"source_and_archive_sha256": staged["source_and_archive_sha256"], "patch_sha256": build["patch_sha256"], "executable_sha256": build["executable_sha256"], "build_manifest_sha256": info["build_manifest_sha256"], "staging_manifest_sha256": info["staging_manifest_sha256"], "compiler": build["gnu_fortran_version"], "host_os": build["host_os"], "host_architecture": build["host_architecture"], "flags": build["flags"], "command": command, "input_sha256": hashes, "output_sha256": tlevel_outputs, "selector_sha256": hashlib.sha256(selector.encode()).hexdigest(), "profile_sha256": sha256(case_dir / "PROFILE.DAT"), "runner_sha256": sha256(Path(__file__))}
        }
        tlevel_path.parent.mkdir(parents=True, exist_ok=True)
        payload = (json.dumps(tlevel_record, indent=2, sort_keys=True) + "\n").encode()
        tlevel_path.write_bytes(deterministic_gzip(payload))
    if produce_tlevel and not produce_balance:
        return tlevel_path
    return balance_path if (produce_balance and output_path.exists()) else output_path


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--case", action="append")
    group.add_argument("--all", action="store_true")
    parser.add_argument("--build-dir", type=Path, required=True)
    parser.add_argument("--task-base", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=int)
    parser.add_argument("--balance", action="store_true", help="parse BALANCE.OUT and write <case>_balance.json")
    parser.add_argument("--tlevel", action="store_true", help="parse T_LEVEL.OUT and write <case>_tlevel.json.gz")
    args = parser.parse_args(argv)
    development = ROOT / ".development"
    if args.task_base.resolve() == development.resolve() or development.resolve() not in args.task_base.resolve().parents:
        parser.error("--task-base must be below .development/")
    cases = sorted(p.name for p in CASES.iterdir() if p.is_dir()) if args.all else args.case
    expected = {"vapor_isothermal_head_gradient", "vapor_thermal_gradient", "vapor_off_control", "vapor_dry_thermal_gradient", "vapor_dry_off_control"}
    expected |= {f"vapor_heatbc_{pair}_{state}" for pair in ("df", "dz", "fd", "ff", "fz") for state in ("on", "off")}
    expected |= {f"vapor_nonuniform_step_{state}" for state in ("on", "off")}
    expected |= {f"vapor_{pair}_{state}" for pair in ("closed_flux", "flux_known") for state in ("on", "off")}
    expected |= {f"vapor_{pair}_{state}" for pair in ("dry_isothermal", "free_drainage") for state in ("on", "off")}
    expected |= {"vapor_flux_fixedhead_wet_on", MET_CASE_ID}
    if not set(cases) <= expected:
        parser.error(f"cases must be a subset of: {', '.join(sorted(expected))}")
    args.task_base.mkdir(parents=True, exist_ok=False)
    for case in cases:
        print(f"wrote {run_case(case, args.task_base, args.build_dir.resolve(), args.timeout_seconds, args.balance, args.tlevel)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

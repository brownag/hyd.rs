"""Focused parser and format tests for the full vapor reference producer."""
import io
import json
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from unittest import mock

from tools import run_vapor_reference as vapor


def main_row(node, depth, temp="  25.00"):
    row = f"{node:4d} {depth:10.4f} {-10.0:11.3f} {0.4254:6.4f} "
    row += "".join(f"{0.282:12.4e}" for _ in range(4))
    row += f"{-1:8d} {-.656:11.3e}{temp:>8}"
    return row


def vapor_row(node, depth, fields=None):
    values = fields or [float(i + 1) / 10 for i in range(9)]
    return f"{node:4d} {depth:10.4f} " + "".join(f"{v:13.5e}" for v in values)


def blocks(rows, times=(6.0,)):
    text = []
    for time in times:
        text.extend((f" Time:{time:14.4f}", " Node Depth output"))
        text.extend(rows)
        text.append("end")
    return "\n".join(text) + "\n"


class FixedWidthParserTests(unittest.TestCase):
    def test_parses_nodal_blocks_and_values(self):
        result = vapor.parse_blocks(blocks([main_row(1, 0), main_row(2, -1)], (6, 12)))
        self.assertEqual([b["time"] for b in result], [6, 12])
        self.assertEqual([r["node"] for r in result[0]["nodes"]], [1, 2])
        self.assertEqual(result[1]["nodes"][1]["temperature"], 25.0)

    def test_parses_nine_vapor_fields(self):
        result = vapor.parse_blocks(blocks([vapor_row(1, 0)]), vapor=True)
        self.assertEqual(result[0]["nodes"][0]["v_vap_term"], 0.9)

    def test_rejects_truncated_rows(self):
        with self.assertRaisesRegex(ValueError, "truncated"):
            vapor.parse_blocks(blocks(["   1 short"]))

    def test_rejects_field_overflow(self):
        with self.assertRaisesRegex(ValueError, "overflow"):
            vapor.parse_blocks(blocks([main_row(1, 0).replace("  25.00", "********")]))

    def test_rejects_missing_end(self):
        with self.assertRaisesRegex(ValueError, "missing end"):
            vapor.parse_blocks(" Time: 6.0\n" + main_row(1, 0) + "\n")

    def test_rejects_duplicate_times(self):
        with self.assertRaisesRegex(ValueError, "duplicate time"):
            vapor.parse_blocks(blocks([main_row(1, 0)], (6, 6)))

    def test_rejects_duplicate_nodes(self):
        with self.assertRaisesRegex(ValueError, "duplicate node"):
            vapor.parse_blocks(blocks([main_row(1, 0), main_row(1, -1)]))

    def test_rejects_changed_node_identity_between_times(self):
        text = blocks([main_row(1, 0)], (6,)) + blocks([main_row(2, 0)], (12,))
        with self.assertRaisesRegex(ValueError, "node identities"):
            vapor.parse_blocks(text)

    def test_rejects_nonfinite_output(self):
        with self.assertRaisesRegex(ValueError, "nonfinite"):
            vapor.parse_blocks(blocks([main_row(1, 0, "     NaN")]))

    def test_rejects_overflow_in_vapor_output(self):
        with self.assertRaisesRegex(ValueError, "overflow"):
            vapor.parse_blocks(blocks([vapor_row(1, 0)[:-13] + "*************"]), vapor=True)


class CaseSelectionTests(unittest.TestCase):
    BASE = str(vapor.ROOT / ".development/25.6.3/unit-test-never-created")

    def run_main(self, *cases):
        argv = [a for c in cases for a in ("--case", c)] + ["--build-dir", "unused", "--task-base", self.BASE]
        with mock.patch.object(vapor, "run_case", side_effect=lambda c, *a: Path(c)), mock.patch.object(Path, "mkdir"):
            return vapor.main(argv)

    def test_rejects_unknown_case_id(self):
        with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            self.run_main("vapor_unknown")

    def test_accepts_new_dry_cases_alone(self):
        with redirect_stderr(io.StringIO()), mock.patch("builtins.print"):
            self.assertEqual(self.run_main("vapor_dry_thermal_gradient", "vapor_dry_off_control"), 0)

    HEATBC = {
        "df": ("f f f t f f", "1 45 -1 0.0"),
        "dz": ("f f f t f f", "1 45 0 5"),
        "fd": ("f f f t f f", "-1 0.0 1 5"),
        "ff": ("f f f t f f", "-1 0.0 -1 0.0"),
        "fz": ("f f f t f f", "-1 0.0 0 5"),
    }

    def test_heatbc_decks_differ_only_in_lvapor_and_boundary_rows(self):
        base = (vapor.CASES / "vapor_dry_off_control/SELECTOR.IN").read_text().splitlines()
        for pair, (_, boundary) in self.HEATBC.items():
            for state, row in (("on", "f f f t f f"), ("off", "f f f f f f")):
                lines = (vapor.CASES / f"vapor_heatbc_{pair}_{state}/SELECTOR.IN").read_text().splitlines()
                self.assertEqual(lines[11], row, f"{pair}_{state} lVapor row")
                self.assertEqual(lines[42], boundary, f"{pair}_{state} boundary row")
                self.assertEqual(len(lines), len(base))
                self.assertEqual([i for i, (a, b) in enumerate(zip(lines, base)) if a != b and i not in (11, 42)], [])

    def test_accepts_new_heatbc_cases_and_existing_ids(self):
        ids = [f"vapor_heatbc_{p}_{s}" for p in self.HEATBC for s in ("on", "off")]
        existing = ["vapor_isothermal_head_gradient", "vapor_thermal_gradient", "vapor_off_control", "vapor_dry_thermal_gradient", "vapor_dry_off_control"]
        with redirect_stderr(io.StringIO()), mock.patch("builtins.print"):
            self.assertEqual(self.run_main(*ids, *existing), 0)

    def test_all_lists_new_heatbc_ids(self):
        called = []
        argv = ["--all", "--build-dir", "unused", "--task-base", self.BASE]
        with mock.patch.object(vapor, "run_case", side_effect=lambda c, *a: called.append(c) or Path(c)), \
             mock.patch.object(Path, "mkdir"), mock.patch("builtins.print"):
            self.assertEqual(vapor.main(argv), 0)
        for pair in self.HEATBC:
            for state in ("on", "off"):
                self.assertIn(f"vapor_heatbc_{pair}_{state}", called)
        self.assertIn("vapor_dry_off_control", called)


class NonuniformStepTests(unittest.TestCase):
    STEP = ("vapor_nonuniform_step_on", "vapor_nonuniform_step_off")
    TEMPLATE = {"vapor_nonuniform_step_on": "vapor_dry_thermal_gradient", "vapor_nonuniform_step_off": "vapor_dry_off_control"}

    def test_step_profile_is_45_then_5_at_node_20(self):
        rows = [f for f in (line.split() for line in (vapor.CASES / "vapor_nonuniform_step_on/PROFILE.DAT").read_text().splitlines()[4:]) if len(f) == 10]
        self.assertEqual(len(rows), 41)
        temps = {int(r[0]): float(r[-1]) for r in rows}
        self.assertTrue(all(temps[n] == 45.0 for n in range(1, 21)))
        self.assertTrue(all(temps[n] == 5.0 for n in range(21, 42)))

    def test_step_decks_match_dry_templates(self):
        for case in self.STEP:
            template = self.TEMPLATE[case]
            self.assertEqual((vapor.CASES / case / "SELECTOR.IN").read_bytes(), (vapor.CASES / template / "SELECTOR.IN").read_bytes(), case)
            self.assertEqual((vapor.CASES / case / "LEVEL_01.DIR").read_bytes(), (vapor.CASES / template / "LEVEL_01.DIR").read_bytes(), case)
            self.assertEqual((vapor.CASES / case / "PROFILE.DAT").read_text().splitlines()[:5], (vapor.CASES / template / "PROFILE.DAT").read_text().splitlines()[:5], case)

    def test_step_fixture_ids_match_directories(self):
        for case in self.STEP:
            spec = json.loads((vapor.CASES / case / "fixture.json").read_text())
            self.assertEqual(spec["case_id"], case)
            self.assertEqual(spec["runner"]["data_prefix"], case)

    def test_accepts_step_cases_and_existing_ids(self):
        base = str(vapor.ROOT / ".development/27.2.1/unit-test-never-created")
        for cases in (self.STEP, (*self.STEP, "vapor_thermal_gradient", "vapor_dry_off_control")):
            argv = [a for c in cases for a in ("--case", c)] + ["--build-dir", "unused", "--task-base", base]
            with redirect_stderr(io.StringIO()), mock.patch.object(vapor, "run_case", side_effect=lambda c, *a: Path(c)), \
                 mock.patch.object(Path, "mkdir"), mock.patch("builtins.print"):
                self.assertEqual(vapor.main(argv), 0)

    def test_all_lists_step_ids(self):
        called = []
        argv = ["--all", "--build-dir", "unused", "--task-base", CaseSelectionTests.BASE]
        with mock.patch.object(vapor, "run_case", side_effect=lambda c, *a: called.append(c) or Path(c)), \
             mock.patch.object(Path, "mkdir"), mock.patch("builtins.print"):
            self.assertEqual(vapor.main(argv), 0)
        for case in self.STEP:
            self.assertIn(case, called)
        self.assertIn("vapor_thermal_gradient", called)


class ClosedPairTests(unittest.TestCase):
    CASES = {"vapor_closed_flux_on": ("vapor_dry_thermal_gradient", "0.0 0.0 0.0"), "vapor_closed_flux_off": ("vapor_dry_off_control", "0.0 0.0 0.0"),
             "vapor_flux_known_on": ("vapor_dry_thermal_gradient", "-1.0e-4 -5.0e-5 0.0"), "vapor_flux_known_off": ("vapor_dry_off_control", "-1.0e-4 -5.0e-5 0.0")}

    def test_decks_match_templates_except_flux_edits(self):
        for case, (template, flux) in self.CASES.items():
            new = (vapor.CASES / case / "SELECTOR.IN").read_text().splitlines()
            base = (vapor.CASES / template / "SELECTOR.IN").read_text().splitlines()
            base[18], base[20] = "f f -1 f", "f f f f -1 f 0"
            base[21:21] = ["rTop rBot rRoot", flux]
            self.assertEqual(new, base, case)
            self.assertEqual((vapor.CASES / case / "LEVEL_01.DIR").read_bytes(), (vapor.CASES / template / "LEVEL_01.DIR").read_bytes(), case)
            # Initial head adopted from candidate D (2026-11-14): only the head column differs.
            profile_template = (vapor.CASES / template / "PROFILE.DAT").read_bytes()
            self.assertEqual((vapor.CASES / case / "PROFILE.DAT").read_bytes(), profile_template.replace(b"-10000.000", b"-100.000"), case)

    def test_pairs_differ_only_in_lvapor(self):
        for pair in ("closed_flux", "flux_known"):
            on = (vapor.CASES / f"vapor_{pair}_on/SELECTOR.IN").read_text().splitlines()
            off = (vapor.CASES / f"vapor_{pair}_off/SELECTOR.IN").read_text().splitlines()
            self.assertEqual([i for i, (a, b) in enumerate(zip(on, off)) if a != b], [11])
            self.assertEqual(len(on), len(off))

    def test_fixture_specs_bind_ids_and_carry_no_outputs(self):
        for case in self.CASES:
            spec = json.loads((vapor.CASES / case / "fixture.json").read_text())
            self.assertEqual(spec["case_id"], case)
            self.assertEqual(spec["runner"]["data_prefix"], case)
            self.assertNotIn("hashes", json.dumps(spec))

    def test_accepts_closed_pair_ids_and_all_lists_them(self):
        base = str(vapor.ROOT / ".development/29.3.2/unit-test-never-created")
        argv = [a for c in self.CASES for a in ("--case", c)] + ["--build-dir", "unused", "--task-base", base]
        called = []
        with mock.patch.object(vapor, "run_case", side_effect=lambda c, *a: called.append(c) or Path(c)), \
             mock.patch.object(Path, "mkdir"), mock.patch("builtins.print"):
            self.assertEqual(vapor.main(argv), 0)
            called.clear()
            self.assertEqual(vapor.main(["--all", "--build-dir", "unused", "--task-base", base]), 0)
        for case in self.CASES:
            self.assertIn(case, called)


def balance_block(time=0.0, area=40.0, w_vol=11.409, in_flow=-0.001592, h_mean=-100.0,
                  heat_vol=3.7009e16, t_mean=25.0, top_flux=-0.0091726, bot_flux=-0.0088832,
                  wat_bal_t=None, wat_bal_r=None):
    lines = [
        "----------------------------------------------------------",
        f" Time       [T]{time:14.4f}",
        "----------------------------------------------------------",
        " Sub-region num.                     1",
        "----------------------------------------------------------",
        f" Area     [L]        {area:12.5e} {area:12.5e}",
        f" W-volume [L]        {w_vol:12.5e} {w_vol:12.5e}",
        f" In-flow  [L/T]     {in_flow:12.5e} {in_flow:12.5e}",
        f" h Mean   [L]       {h_mean:12.5e} {h_mean:12.5e}",
        f" HeatVol  [M/T2]     {heat_vol:12.5e} {heat_vol:12.5e}",
        f" tMean    [K]             {t_mean:6.3f}       {t_mean:6.3f}",
        f" Top Flux [L/T]     {top_flux:12.5e}",
        f" Bot Flux [L/T]     {bot_flux:12.5e}",
    ]
    if wat_bal_t is not None:
        lines.append(f" WatBalT  [L]        {wat_bal_t:12.5e}")
    if wat_bal_r is not None:
        lines.append(f" WatBalR  [%]              {wat_bal_r:5.3f}")
    lines.append("----------------------------------------------------------")
    return "\n".join(lines)


def sample_balance_text(blocks=None, calc_time=0.0):
    header = [
        " ******* Program HYDRUS",
        " ******* Physical values are chosen inputs; no physical claim is made.",
        " Date:  14.**.    Time:  11:36:11",
        " Units: L = cm   , T = hours, M = kg   ",
        "",
    ]
    if blocks is None:
        blocks = [
            balance_block(time=0.0),
            balance_block(time=6.0, wat_bal_t=1.6497e-4, wat_bal_r=0.156),
        ]
    return "\n".join(header) + "\n\n".join(blocks) + f"\n\n Calculation time [sec]   {calc_time:18.16f}     \n"


class BalanceParserTests(unittest.TestCase):
    def test_parses_balance_blocks_and_values(self):
        text = sample_balance_text()
        rows = vapor.parse_balance(text)
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0]["time"], 0.0)
        self.assertEqual(rows[0]["area"], 40.0)
        self.assertEqual(rows[0]["w_volume"], 11.409)
        self.assertEqual(rows[0]["in_flow"], -0.001592)
        self.assertEqual(rows[0]["h_mean"], -100.0)
        self.assertEqual(rows[0]["top_flux"], -0.0091726)
        self.assertEqual(rows[0]["bot_flux"], -0.0088832)
        self.assertIsNone(rows[0]["wat_bal_t"])
        self.assertIsNone(rows[0]["wat_bal_r"])
        self.assertEqual(rows[0]["sub_regions"]["area"], [40.0])

        self.assertEqual(rows[1]["time"], 6.0)
        self.assertAlmostEqual(rows[1]["wat_bal_t"], 1.6497e-4, places=8)
        self.assertEqual(rows[1]["wat_bal_r"], 0.156)

    def test_parses_balance_from_path(self):
        scratch = vapor.ROOT / ".development/30.3.1"
        scratch.mkdir(parents=True, exist_ok=True)
        path = scratch / "test_balance.out"
        path.write_text(sample_balance_text())
        rows = vapor.parse_balance(path)
        self.assertEqual(len(rows), 2)

    def test_rejects_absent_or_empty_file(self):
        scratch = vapor.ROOT / ".development/30.3.1"
        scratch.mkdir(parents=True, exist_ok=True)
        nonexistent = scratch / "nonexistent.out"
        with self.assertRaisesRegex(RuntimeError, "absent/empty"):
            vapor.parse_balance(nonexistent)
        empty = scratch / "empty.out"
        empty.write_text("")
        with self.assertRaisesRegex(RuntimeError, "absent/empty"):
            vapor.parse_balance(empty)

    def test_rejects_missing_final_newline(self):
        text = sample_balance_text().rstrip("\n")
        with self.assertRaisesRegex(ValueError, "malformed or truncated"):
            vapor.parse_balance(text)

    def test_rejects_missing_calculation_time(self):
        text = sample_balance_text().rsplit("Calculation time", 1)[0].rstrip() + "\n"
        with self.assertRaisesRegex(ValueError, "calculation-time"):
            vapor.parse_balance(text)

    def test_rejects_negative_calculation_time(self):
        text = sample_balance_text(calc_time=-5.0)
        with self.assertRaisesRegex(ValueError, "negative calculation time"):
            vapor.parse_balance(text)

    def test_rejects_unclosed_block(self):
        text = sample_balance_text()
        truncated = text[:text.rfind("----------------------------------------------------------")]
        with self.assertRaisesRegex(ValueError, "missing end marker"):
            vapor.parse_balance(truncated + "\n Calculation time [sec] 0.0\n")

    def test_rejects_missing_required_fields(self):
        block = [
            "----------------------------------------------------------",
            " Time       [T]        0.0000",
            "----------------------------------------------------------",
            " Sub-region num.                     1",
            "----------------------------------------------------------",
            " W-volume [L]        0.11409E+02",
            " In-flow  [L/T]     -0.15920E-02",
            " h Mean   [L]       -0.10000E+03",
            " Top Flux [L/T]     -0.0091726",
            " Bot Flux [L/T]     -0.0088832",
            "----------------------------------------------------------",
            "",
            " Calculation time [sec] 0.0",
            "",
        ]
        with self.assertRaisesRegex(ValueError, "truncated balance block"):
            vapor.parse_balance("\n".join(block))

    def test_rejects_stepping_block_missing_wat_bal(self):
        block = balance_block(time=6.0, wat_bal_t=None)
        text = sample_balance_text(blocks=[block])
        with self.assertRaisesRegex(ValueError, "missing wat_bal_t"):
            vapor.parse_balance(text)

    def test_rejects_field_overflow(self):
        bad_block = balance_block(time=0.0).replace(" 4.00000e+01", " ***********")
        text = sample_balance_text(blocks=[bad_block])
        with self.assertRaisesRegex(ValueError, "overflow"):
            vapor.parse_balance(text)

    def test_rejects_nonfinite_values(self):
        bad_block = balance_block(time=0.0).replace(" 4.00000e+01", "         NaN")
        text = sample_balance_text(blocks=[bad_block])
        with self.assertRaisesRegex(ValueError, "nonfinite"):
            vapor.parse_balance(text)

    def test_rejects_duplicate_times(self):
        blocks = [balance_block(time=0.0), balance_block(time=0.0)]
        text = sample_balance_text(blocks=blocks)
        with self.assertRaisesRegex(ValueError, "duplicate balance time"):
            vapor.parse_balance(text)


class BalanceRunnerIntegrationTests(unittest.TestCase):
    BASE = str(vapor.ROOT / ".development/30.3.1/unit-test-never-created")

    def test_main_accepts_balance_flag(self):
        called = []
        argv = ["--case", "vapor_closed_flux_on", "--build-dir", "unused", "--task-base", self.BASE, "--balance"]
        with mock.patch.object(vapor, "run_case", side_effect=lambda c, *a: called.append((c, a)) or Path(c)), \
             mock.patch.object(Path, "mkdir"), mock.patch("builtins.print"), redirect_stderr(io.StringIO()):
            self.assertEqual(vapor.main(argv), 0)
        self.assertEqual(len(called), 1)
        self.assertEqual(called[0][0], "vapor_closed_flux_on")
        self.assertTrue(called[0][1][3])

    def test_run_case_refuses_to_overwrite_existing_fixture(self):
        with mock.patch.object(Path, "exists", return_value=True):
            with self.assertRaisesRegex(FileExistsError, "refusing to overwrite fixture"):
                vapor.run_case("vapor_closed_flux_on", Path("tb"), Path("bd"), None, balance=False)

    def test_run_case_refuses_to_overwrite_existing_balance_fixture(self):
        def mock_exists(self):
            return self.name.endswith("_balance.json")
        with mock.patch.object(Path, "exists", mock_exists):
            with self.assertRaisesRegex(FileExistsError, "refusing to overwrite balance fixture"):
                vapor.run_case("vapor_closed_flux_on", Path("tb"), Path("bd"), None, balance=True)

    def test_run_case_produces_balance_without_overwriting_existing_fixture(self):
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            base = Path(td)
            task_base = base / "tb"
            build_dir = base / "bd"
            fixtures_dir = base / "fixtures"
            fixtures_dir.mkdir()
            existing_fixture = fixtures_dir / "vapor_closed_flux_on.json"
            existing_fixture.write_text('{"existing": true}\n')

            def fake_run(cmd, cwd, text, capture_output, timeout):
                prefix = "vapor_closed_flux_on"
                (cwd / f"{prefix}\\NOD_INF.OUT").write_text(blocks([main_row(1, 0), main_row(2, -1)], (0, 6)))
                (cwd / f"{prefix}\\NOD_INF_V.OUT").write_text(blocks([vapor_row(1, 0), vapor_row(2, -1)], (0, 6)))
                (cwd / f"{prefix}\\BALANCE.OUT").write_text(sample_balance_text())
                return mock.Mock(returncode=0, stdout="", stderr="")

            mock_build = {
                "build": {"executable_sha256": "e", "patch_sha256": "p", "gnu_fortran_version": "g", "host_os": "l", "host_architecture": "x", "flags": []},
                "staged": {"source_and_archive_sha256": "s"},
                "exe": build_dir / "hydrus-reference",
                "build_manifest_sha256": "bm",
                "staging_manifest_sha256": "sm",
            }
            with mock.patch.object(vapor, "FIXTURES", fixtures_dir), \
                 mock.patch.object(vapor, "load_build", return_value=mock_build), \
                 mock.patch("subprocess.run", side_effect=fake_run):
                out = vapor.run_case("vapor_closed_flux_on", task_base, build_dir, None, balance=True)
                self.assertEqual(out, fixtures_dir / "vapor_closed_flux_on_balance.json")
                self.assertEqual(existing_fixture.read_text(), '{"existing": true}\n')
                bal = json.loads(out.read_text())
                self.assertEqual(bal["case_id"], "vapor_closed_flux_on")
                self.assertEqual(bal["evidence_class"], "pinned-fortran-balance")
                self.assertEqual(len(bal["balance_rows"]), 2)


class MetCaseTests(unittest.TestCase):
    CASE = "vapor_met_bare_soil_on"

    def test_generator_reproduces_committed_deck(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            vapor.generate_met_case(Path(tmp))
            for name in ("LEVEL_01.DIR", "SELECTOR.IN", "PROFILE.DAT", "ATMOSPH.IN", "Meteo.in", "fixture.json"):
                self.assertEqual((Path(tmp) / name).read_bytes(), (vapor.CASES / self.CASE / name).read_bytes(), name)

    def test_selector_flags_and_no_unconsumed_flux_record(self):
        sel = (vapor.CASES / self.CASE / "SELECTOR.IN").read_text().splitlines()
        self.assertEqual(sel[9], "t f t f f f f f t f")  # AtmBC=t
        self.assertEqual(sel[11], "f f t t f f")  # lMeteo=t lVapor=t
        self.assertEqual(sel[18], "t f -4 f")
        self.assertNotIn("rTop rBot rRoot", sel)

    def test_meteo_deck_is_bare_soil_with_heights_in_cm(self):
        met = (vapor.CASES / self.CASE / "Meteo.in").read_text().splitlines()
        self.assertEqual(met[met.index("iCrop iSunSh iRelHum") + 1], "0 0 0")
        self.assertEqual(met[met.index("lEnBal lMetDaily") + 1], "f f")
        self.assertEqual(met[met.index("MaxAL iRadiation lHargr") + 1], "10 1 f")
        self.assertEqual(met[met.index("WindHeight TempHeight [cm]") + 1], "200. 190.")
        records = [l for l in met[met.index("records follow") + 1:] if l and l[0].isdigit()]
        atm = [l for l in (vapor.CASES / self.CASE / "ATMOSPH.IN").read_text().splitlines() if l and l[0].isdigit() and len(l.split()) == 11]
        self.assertEqual([r.split()[0] for r in records], [a.split()[0] for a in atm])
        self.assertEqual([r.split()[0] for r in records], [str(24 * i) for i in range(1, 11)])
        self.assertEqual(met[-1], "*** END OF INPUT FILE 'Meteo.in' **********************************")
        self.assertEqual(met[-2], "end")

    def test_fixture_spec_lists_all_met_inputs(self):
        spec = json.loads((vapor.CASES / self.CASE / "fixture.json").read_text())
        self.assertEqual(spec["case_id"], self.CASE)
        self.assertEqual(spec["runner"]["data_prefix"], self.CASE)
        self.assertEqual(set(spec["runner"]["aliases"]) | {"LEVEL_01.DIR"}, set(spec["runner"]["inputs"]))

    def test_allowlist_accepts_met_case_and_all_lists_it(self):
        with redirect_stderr(io.StringIO()), mock.patch("builtins.print"):
            self.assertEqual(CaseSelectionTests.run_main(CaseSelectionTests(), self.CASE), 0)
        called = []
        argv = ["--all", "--build-dir", "unused", "--task-base", CaseSelectionTests.BASE]
        with mock.patch.object(vapor, "run_case", side_effect=lambda c, *a: called.append(c) or Path(c)), \
             mock.patch.object(Path, "mkdir"), mock.patch("builtins.print"):
            self.assertEqual(vapor.main(argv), 0)
        self.assertIn(self.CASE, called)


if __name__ == "__main__":
    unittest.main()

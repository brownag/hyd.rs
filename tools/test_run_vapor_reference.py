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


if __name__ == "__main__":
    unittest.main()

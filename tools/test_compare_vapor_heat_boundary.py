"""Unit tests for tools/compare_vapor_heat_boundary.py (26.4.2)."""

import copy
import io
import json
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

import tools.compare_vapor_heat_boundary as hb
import tools.compare_vapor_reference as cmp
from tools.test_compare_vapor_reference import fixture

STEMS = ("df", "dz", "fd", "ff", "fz")


def heatbc_fixtures(vapor=1.0):
    out = {}
    for stem in STEMS:
        on = fixture(f"vapor_heatbc_{stem}_on", vapor=vapor)
        on["head"][1][0] += 5.0
        on["liquid_moisture"][1][0] += 0.05
        on["temperature_c"][1][0] += 1.0
        out[f"vapor_heatbc_{stem}_on"] = on
        out[f"vapor_heatbc_{stem}_off"] = fixture(f"vapor_heatbc_{stem}_off", available=False)
    return out


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)
        self.criteria, self.csha = cmp.load_criteria(cmp.DEFAULT_CRITERIA)


class CriteriaV3Tests(Base):
    def test_real_criteria_has_heatbc_checks(self):
        names = [n for n in self.criteria["discrimination"] if n.startswith("heatbc_")]
        self.assertEqual(len(names), 13 + 1)  # 13 dry entries + 1 nonzero entry
        dry = [n for n in names if self.criteria["discrimination"][n]["rule"] == "any_print_time_after_initial"]
        self.assertEqual(len(dry), 13)

    def test_ff_and_fz_have_no_moisture_check(self):
        disc = self.criteria["discrimination"]
        for stem in ("ff", "fz"):
            self.assertNotIn(f"heatbc_{stem}_liquid_moisture", disc)
        for stem in ("df", "dz", "fd"):
            self.assertIn(f"heatbc_{stem}_liquid_moisture", disc)


class DiscriminationTests(Base):
    def run_disc(self, fixtures, criteria=None):
        crit = self.criteria if criteria is None else criteria
        return hb.discrimination(crit, self.csha, fixtures)

    def test_only_heatbc_checks_and_pass(self):
        res = self.run_disc(heatbc_fixtures(vapor=1.0))
        self.assertTrue(all(c["name"].startswith("heatbc_") for c in res["checks"]))
        self.assertEqual(len([c for c in res["checks"] if c["rule"] == "any_print_time_after_initial"]), 13)
        self.assertEqual(len([c for c in res["checks"] if c["rule"] == "max_abs_exceeds_zero_floor_each_case"]), 25)
        self.assertEqual(res["verdict"], "PASS")

    def test_equal_on_off_fails(self):
        fx = heatbc_fixtures()
        fx["vapor_heatbc_df_on"] = copy.deepcopy(fx["vapor_heatbc_df_on"])
        for f in ("head", "liquid_moisture", "temperature_c"):
            fx["vapor_heatbc_df_on"][f] = copy.deepcopy(fx["vapor_heatbc_df_off"][f])
        fx["vapor_heatbc_df_on"]["vapor_output_available"] = True
        res = self.run_disc(fx)
        verdicts = {c["name"]: c["verdict"] for c in res["checks"]}
        self.assertEqual(verdicts["heatbc_df_head"], "FAIL")
        self.assertEqual(res["verdict"], "FAIL")

    def test_ff_fz_have_no_moisture_check_in_result(self):
        res = self.run_disc(heatbc_fixtures())
        names = {c["name"] for c in res["checks"]}
        self.assertNotIn("heatbc_ff_liquid_moisture", names)
        self.assertNotIn("heatbc_fz_liquid_moisture", names)

    def test_limits_come_from_criteria(self):
        # Raising a limit in the criteria copy must flip the verdict: no hardcoded bound.
        crit = copy.deepcopy(self.criteria)
        crit["discrimination"]["heatbc_df_head"]["factor_of_tolerance"] = 1e9
        res = self.run_disc(heatbc_fixtures(), criteria=crit)
        verdicts = {c["name"]: c["verdict"] for c in res["checks"]}
        self.assertEqual(verdicts["heatbc_df_head"], "FAIL")
        self.assertEqual(res["verdict"], "FAIL")

    def test_missing_heatbc_fixture_rejected(self):
        fx = heatbc_fixtures()
        fx.pop("vapor_heatbc_fz_off")
        with self.assertRaises(cmp.Reject):
            self.run_disc(fx)


class TrajectoryTests(Base):
    def test_non_heatbc_case_rejected(self):
        fx = fixture("vapor_dry_thermal_gradient")
        with self.assertRaises(cmp.Reject):
            hb.trajectory(self.criteria, self.csha, fx, "f" * 64, {})


class CliTests(Base):
    def test_cli_discrimination_writes_new_file_only(self):
        fdir = self.dir / "fx"
        fdir.mkdir()
        for name, data in heatbc_fixtures().items():
            (fdir / f"{name}.json").write_text(json.dumps(data), encoding="utf-8")
        out = self.dir / "out.json"
        with redirect_stdout(io.StringIO()):
            rc = hb.main(["discrimination", "--fixture-dir", str(fdir), "--output", str(out)])
        self.assertEqual(rc, 0)
        self.assertEqual(json.loads(out.read_text())["verdict"], "PASS")
        with redirect_stdout(io.StringIO()):
            self.assertEqual(hb.main(["discrimination", "--fixture-dir", str(fdir), "--output", str(out)]), 2)


if __name__ == "__main__":
    unittest.main()

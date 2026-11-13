"""Unit tests for tools/compare_vapor_reference.py (25.7.2)."""

import copy
import hashlib
import io
import json
import math
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

import tools.compare_vapor_reference as cmp

REQ_SHA = "e" * 64
DECK_HASH = {"SELECTOR.IN": "a" * 64, "PROFILE.DAT": "b" * 64}


def matrix(value, times=2, nodes=3):
    return [[value] * nodes for _ in range(times)]


def fixture(case="case", available=True, head=-10.0, vapor=1.0):
    fx = {
        "case_id": case, "evidence_class": cmp.TRAJ, "source_matched": True, "schema_version": 1,
        "time_points": [0.0, 6.0], "node_numbers": [1, 2, 3], "node_depths": [0.0, 1.0, 2.0],
        "head": matrix(head), "liquid_moisture": matrix(0.3), "temperature_c": matrix(15.0),
        "vapor_output_available": available,
        "vapor_fields": {n: matrix(vapor) for n in cmp.VAPOR_FIELDS} if available else None,
        "manifest": {"input_sha256": dict(DECK_HASH, **{"LEVEL_01.DIR": "c" * 64})},
    }
    return fx


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        self.criteria_path = cmp.DEFAULT_CRITERIA
        self.criteria, self.csha = cmp.load_criteria(self.criteria_path)

    def write(self, name, data):
        path = self.dir / name
        path.write_text(json.dumps(data), encoding="utf-8")
        return path

    def setup_case(self, **kw):
        fx = fixture(**kw)
        path = self.write("fx.json", fx)
        fx, fsha = cmp.load_fixture(path)
        cand = {
            "schema_version": cmp.CANDIDATE_SCHEMA, "case_id": fx["case_id"],
            "evidence_class": "rust-candidate", "criteria_sha256": self.csha,
            "fixture_sha256": fsha, "input_sha256": dict(DECK_HASH),
            "schedule": {"kind": "constant_dt", "dt": 0.1, "retry_factor": 3.0, "note": "n"},
            "time_points": copy.deepcopy(fx["time_points"]),
            "node_numbers": copy.deepcopy(fx["node_numbers"]),
            "node_depths": copy.deepcopy(fx["node_depths"]),
            "head": copy.deepcopy(fx["head"]), "liquid_moisture": copy.deepcopy(fx["liquid_moisture"]),
            "temperature_c": copy.deepcopy(fx["temperature_c"]),
            "vapor_output_available": fx["vapor_output_available"],
            "vapor_fields": copy.deepcopy(fx["vapor_fields"]),
        }
        return fx, fsha, cand

    def run_traj(self, fx, fsha, cand):
        return cmp.compare_trajectory(self.criteria, self.csha, fx, fsha, cand)


class CriteriaTests(Base):
    def test_real_criteria_validate(self):
        self.assertEqual(len(self.csha), 64)

    def mutate(self, fn):
        data = copy.deepcopy(self.criteria)
        fn(data)
        with self.assertRaises(cmp.Reject):
            cmp.validate_criteria(data)

    def test_v1_and_revision_note(self):
        self.mutate(lambda d: d.update(schema_version="vapor-criteria-v1"))
        self.mutate(lambda d: d.update(revision_note=5))
        ok = copy.deepcopy(self.criteria)
        ok["revision_note"] = "note"
        cmp.validate_criteria(ok)

    def test_precision_rule_keys(self):
        z = lambda d: d["fields"]["v_vapor"]["zero_rule"]
        self.mutate(lambda d: z(d).update(temperature_precision="float64"))
        self.mutate(lambda d: z(d).update(spacing="other"))
        self.mutate(lambda d: z(d).update(conductivity_field="con"))
        self.mutate(lambda d: z(d).update(ulp_multiple=0.0))
        self.mutate(lambda d: z(d).pop("ulp_multiple"))
        self.mutate(lambda d: z(d).update(extra=1))
        self.mutate(lambda d: d["fields"]["head"].update(zero_rule=dict(z(d))))

    def test_unknown_missing_wrong(self):
        self.mutate(lambda d: d["fields"].pop("head"))
        self.mutate(lambda d: d["fields"].update(bogus=d["fields"]["head"]))
        self.mutate(lambda d: d["fields"]["head"].update(unit="m"))
        self.mutate(lambda d: d["fields"]["head"].update(evidence_class=cmp.KERN))
        self.mutate(lambda d: d["fields"]["head"].update(scale="weird"))
        self.mutate(lambda d: d["fields"]["head"].update(absolute=-1.0))
        self.mutate(lambda d: d["fields"]["head"].update(absolute=float("nan")))
        self.mutate(lambda d: d["fields"]["head"].pop("print_rule"))
        self.mutate(lambda d: d["fields"]["con"]["zero_rule"].update(name="other"))
        self.mutate(lambda d: d["fields"]["liquid_moisture"].update(relative=0.1))
        self.mutate(lambda d: d["discrimination"]["vapor_fields_nonzero"].update(rule="nope"))
        self.mutate(lambda d: d.update(schema_version="x"))


class TrajectoryTests(Base):
    def test_pass_and_unrounded_identity(self):
        res = self.run_traj(*self.setup_case())
        self.assertEqual(res["verdict"], "PASS")
        self.assertEqual(res["points"], 12 * 2 * 3)

    def test_tolerance_edge_head(self):
        # head bound = 0.5 + 0.02*10 = 0.7 at r = -10
        fx, fsha, cand = self.setup_case()
        cand["head"][1][0] = -10.0 + 0.5 + 0.02 * 10.0
        res = self.run_traj(fx, fsha, cand)
        row = [r for r in res["rows"] if r["field"] == "head" and r["node"] == 1 and r["time"] == 6.0][0]
        self.assertEqual(row["error"] <= row["bound"], row["verdict"] == "PASS")
        cand["head"][1][0] = -10.0 + 0.7 + 1e-6
        self.assertEqual(self.run_traj(fx, fsha, cand)["verdict"], "FAIL")
        cand["head"][1][0] = -10.0 + 0.69
        self.assertEqual(self.run_traj(fx, fsha, cand)["verdict"], "PASS")

    def test_scale_none_absolute_only(self):
        fx, fsha, cand = self.setup_case()
        cand["temperature_c"][0][0] = 15.06
        self.assertEqual(self.run_traj(fx, fsha, cand)["verdict"], "FAIL")

    def test_nan_rejected(self):
        fx, fsha, cand = self.setup_case()
        cand["head"][0][0] = float("nan")
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)

    def test_missing_node_and_time(self):
        fx, fsha, cand = self.setup_case()
        cand["head"][0].pop()
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)
        fx, fsha, cand = self.setup_case()
        cand["head"].pop()
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)
        fx, fsha, cand = self.setup_case()
        cand["node_numbers"] = [1, 2, 4]
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)
        fx, fsha, cand = self.setup_case()
        cand["time_points"][1] = 6.0000001
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)

    def test_tampered_hashes_and_ids(self):
        for key, value in (("fixture_sha256", "0" * 64), ("criteria_sha256", "0" * 64),
                           ("case_id", "other")):
            fx, fsha, cand = self.setup_case()
            cand[key] = value
            with self.assertRaises(cmp.Reject):
                self.run_traj(fx, fsha, cand)
        fx, fsha, cand = self.setup_case()
        cand["input_sha256"]["PROFILE.DAT"] = "d" * 64
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)

    def test_tampered_fixture_file(self):
        fx, fsha, cand = self.setup_case()
        path = self.write("fx.json", dict(fixture(), head=matrix(-11.0)))
        fx2, fsha2 = cmp.load_fixture(path)
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx2, fsha2, cand)

    def test_schedule_validation(self):
        fx, fsha, cand = self.setup_case()
        cand["schedule"]["kind"] = "adaptive"
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)
        fx, fsha, cand = self.setup_case()
        cand["schedule"]["dt"] = 0.0
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)

    def test_zero_floor(self):
        # reference con: 0 at node 1, 10 at node 2; floor = 0.001*10 = 0.01 even at r=0
        fx, fsha, cand = self.setup_case()
        fx["vapor_fields"]["con"] = [[0.0, 10.0, 0.0]] * 2
        cand["vapor_fields"]["con"] = [[0.009, 10.0, 0.0]] * 2
        self.assertEqual(self.run_traj(fx, fsha, cand)["verdict"], "PASS")
        cand["vapor_fields"]["con"] = [[0.011, 10.0, 0.0]] * 2
        res = self.run_traj(fx, fsha, cand)
        self.assertEqual(res["summary"]["con"]["failures"], 2)

    def test_zero_floor_is_per_time(self):
        # t=0 max 10 (floor 0.01), t=1 max 0.01 (floor 1e-5). A global floor would pass 0.001 at t=1.
        fx, fsha, cand = self.setup_case()
        fx["vapor_fields"]["con"] = [[0.0, 10.0, 0.0], [0.0, 0.01, 0.0]]
        cand["vapor_fields"]["con"] = [[0.009, 10.0, 0.0], [0.001, 0.01, 0.0]]
        res = self.run_traj(fx, fsha, cand)
        rows = {(r["time"], r["node"]): r for r in res["rows"] if r["field"] == "con"}
        self.assertEqual(rows[(0.0, 1)]["verdict"], "PASS")
        self.assertAlmostEqual(rows[(6.0, 1)]["bound"], 1e-5)
        self.assertEqual(rows[(6.0, 1)]["verdict"], "FAIL")

    def test_precision_term_dominates(self):
        # T=15 -> ulp32 = 2**-20; ConVT neighbourhood max 1e6, dx=1 -> term = 2*2**-20*1e6
        fx, fsha, cand = self.setup_case()
        fx["vapor_fields"]["v_vapor"] = matrix(0.0)
        fx["vapor_fields"]["con_vt"] = [[0.0, 1e6, 0.0], [0.0, 0.0, 0.0]]
        cand["vapor_fields"]["v_vapor"] = matrix(1.0)
        res = self.run_traj(fx, fsha, cand)
        rows = {(r["time"], r["node"]): r for r in res["rows"] if r["field"] == "v_vapor"}
        term = 2.0 * 2.0 ** -20 * 1e6
        for node in (1, 2, 3):  # neighbours of node 2 share its ConVT
            self.assertAlmostEqual(rows[(0.0, node)]["bound"], term, places=9)
            self.assertEqual(rows[(0.0, node)]["verdict"], "PASS")
        self.assertEqual(rows[(6.0, 1)]["bound"], 0.0)  # same-time neighbourhood only
        self.assertEqual(rows[(6.0, 1)]["verdict"], "FAIL")
        self.assertEqual(self.run_traj(fx, fsha, cand)["summary"]["v_vapor"]["failures"], 3)
        # other vapor fields keep the plain rule
        self.assertEqual(res["summary"]["con"]["failures"], 0)

    def test_precision_term_requires_con_vt(self):
        fx, fsha, cand = self.setup_case()
        fx["vapor_fields"].pop("con_vt")
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)
        with self.assertRaises(cmp.Reject):
            cmp.precision_terms(fx, 0, 0)

    def test_ulp32(self):
        self.assertEqual(cmp.ulp32(25.0), 2.0 ** -19)
        self.assertEqual(cmp.ulp32(-25.0), 2.0 ** -19)
        self.assertEqual(cmp.ulp32(1.0), 2.0 ** -23)
        self.assertAlmostEqual(cmp.ulp32(25.0), 1.9073486e-06, places=12)

    def test_vapor_unavailable_vs_missing(self):
        fx, fsha, cand = self.setup_case(available=False)
        res = self.run_traj(fx, fsha, cand)
        self.assertEqual(res["verdict"], "PASS")
        self.assertEqual(res["unavailable"], list(cmp.VAPOR_FIELDS))
        self.assertNotIn("con", res["summary"])
        # candidate lacks vapor output although fixture has it: reject
        fx, fsha, cand = self.setup_case()
        cand["vapor_output_available"] = False
        cand["vapor_fields"] = None
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)
        # candidate carries vapor output where fixture says unavailable: reject
        fx, fsha, cand = self.setup_case(available=False)
        cand["vapor_output_available"] = True
        cand["vapor_fields"] = {n: matrix(1.0) for n in cmp.VAPOR_FIELDS}
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)

    def test_missing_vapor_field(self):
        fx, fsha, cand = self.setup_case()
        cand["vapor_fields"].pop("con_vt")
        with self.assertRaises(cmp.Reject):
            self.run_traj(fx, fsha, cand)


def kernel_fixture():
    obs = [{"request_id": "a", "node": 1, "vapor_content": 1e-6,
            "adjusted_hydraulic_capacity": 0.0199, "input_capacity": 0.02},
           {"request_id": "a", "node": 2, "vapor_content": 2e-6,
            "adjusted_hydraulic_capacity": 0.0299, "input_capacity": 0.03}]
    return {"evidence_class": cmp.KERN, "observations": obs}


class KernelTests(Base):
    def setup_kernel(self):
        path = self.write("k.json", kernel_fixture())
        fx = cmp.load_json(path)
        fsha = cmp.sha256_file(path)
        cand = {"schema_version": cmp.KERNEL_CANDIDATE_SCHEMA, "criteria_sha256": self.csha,
                "evidence_class": "rust-candidate", "requests_sha256": REQ_SHA,
                "fixture_sha256": fsha, "observations": copy.deepcopy(fx["observations"])}
        return fx, fsha, cand

    def run_kernel(self, fx, fsha, cand):
        return cmp.compare_kernel(self.criteria, self.csha, fx, fsha, cand, REQ_SHA)

    def test_requests_hash_and_evidence_class(self):
        for key, value in (("requests_sha256", "0" * 64), ("evidence_class", "other")):
            fx, fsha, cand = self.setup_kernel()
            cand[key] = value
            with self.assertRaises(cmp.Reject):
                self.run_kernel(fx, fsha, cand)
        fx, fsha, cand = self.setup_kernel()
        cand.pop("requests_sha256")
        with self.assertRaises(cmp.Reject):
            self.run_kernel(fx, fsha, cand)

    def test_pass(self):
        self.assertEqual(self.run_kernel(*self.setup_kernel())["verdict"], "PASS")

    def test_capacity_correction_semantics(self):
        # reference correction = 1e-4, rel bound 1e-2 -> 1e-6. Shift adjusted by 5e-7:
        # adjusted passes (rel 1e-6*0.0199 = 2e-8 fails!), so shift adjusted by 1e-8 only.
        fx, fsha, cand = self.setup_kernel()
        cand["observations"][0]["adjusted_hydraulic_capacity"] += 1e-8
        res = self.run_kernel(fx, fsha, cand)
        by = {r["field"]: r for r in res["rows"] if r["node"] == 1}
        row = by["kernel_capacity_correction"]
        self.assertAlmostEqual(row["reference"], 1e-4, places=12)
        self.assertAlmostEqual(row["candidate"], 1e-4 - 1e-8, places=12)
        self.assertEqual(row["verdict"], "PASS")
        cand["observations"][0]["adjusted_hydraulic_capacity"] += 5e-6
        res = self.run_kernel(fx, fsha, cand)
        by = {r["field"]: r["verdict"] for r in res["rows"] if r["node"] == 1}
        self.assertEqual(by["kernel_capacity_correction"], "FAIL")
        self.assertEqual(by["kernel_adjusted_hydraulic_capacity"], "FAIL")

    def test_missing_extra_nan(self):
        fx, fsha, cand = self.setup_kernel()
        cand["observations"].pop()
        with self.assertRaises(cmp.Reject):
            self.run_kernel(fx, fsha, cand)
        fx, fsha, cand = self.setup_kernel()
        cand["observations"][0]["vapor_content"] = float("inf")
        with self.assertRaises(cmp.Reject):
            self.run_kernel(fx, fsha, cand)
        fx, fsha, cand = self.setup_kernel()
        cand["fixture_sha256"] = "0" * 64
        with self.assertRaises(cmp.Reject):
            self.run_kernel(fx, fsha, cand)

    def test_real_kernel_fixture_self_compare(self):
        path = cmp.VAPOR_DIR / "kernel_fixtures/vapor_content.json"
        fx = cmp.load_json(path)
        cand = {"schema_version": cmp.KERNEL_CANDIDATE_SCHEMA, "criteria_sha256": self.csha,
                "evidence_class": "rust-candidate", "requests_sha256": cmp.sha256_file(cmp.DEFAULT_REQUESTS),
                "fixture_sha256": cmp.sha256_file(path), "observations": fx["observations"]}
        res = cmp.compare_kernel(self.criteria, self.csha, fx, cmp.sha256_file(path), cand)
        self.assertEqual(res["verdict"], "PASS")


class DiscriminationTests(Base):
    def fixtures(self, off_head=-10.0, vapor=1.0):
        out = {
            "vapor_dry_thermal_gradient": fixture("vapor_dry_thermal_gradient", vapor=vapor),
            "vapor_dry_off_control": fixture("vapor_dry_off_control", available=False, head=off_head),
            "vapor_isothermal_head_gradient": fixture("vapor_isothermal_head_gradient", vapor=vapor),
            "vapor_thermal_gradient": fixture("vapor_thermal_gradient", vapor=vapor),
        }
        # heat-boundary (criteria v3) on/off pairs; on-case differs from off beyond
        # 2x tolerance at a print time after t=0 for every dry-checked field
        for stem in ("df", "dz", "fd", "ff", "fz"):
            on = fixture(f"vapor_heatbc_{stem}_on", vapor=vapor)
            on["head"][1][0] += 5.0
            on["liquid_moisture"][1][0] += 0.05
            on["temperature_c"][1][0] += 1.0
            out[f"vapor_heatbc_{stem}_on"] = on
            out[f"vapor_heatbc_{stem}_off"] = fixture(f"vapor_heatbc_{stem}_off", available=False)
        return out

    def disc(self, fixtures):
        return cmp.discrimination(self.criteria, self.csha, fixtures)

    def test_fail_when_on_equals_off(self):
        res = self.disc(self.fixtures())
        names = {c["name"]: c["verdict"] for c in res["checks"] if c["field"] == "head"}
        self.assertEqual(names["dry_on_off_head"], "FAIL")
        self.assertEqual(res["verdict"], "FAIL")

    def test_pass_when_difference_exceeds_twice_tolerance(self):
        # head tol at r=-10: 0.7, factor 2 -> 1.4. Only time index >= 1 counts.
        fixtures = self.fixtures()
        off = fixtures["vapor_dry_off_control"]
        for field in ("head", "liquid_moisture", "temperature_c"):
            off[field] = copy.deepcopy(fixtures["vapor_dry_thermal_gradient"][field])
        off["head"][1][0] += 1.41
        off["liquid_moisture"][1][0] += 0.0022
        off["temperature_c"][1][0] += 0.11
        self.assertEqual(self.disc(fixtures)["verdict"], "PASS")
        off["head"][1][0] -= 0.02  # 1.39 < 1.4
        self.assertEqual(self.disc(fixtures)["verdict"], "FAIL")

    def test_initial_time_difference_ignored(self):
        fixtures = self.fixtures()
        off = fixtures["vapor_dry_off_control"]
        for field in ("head", "liquid_moisture", "temperature_c"):
            off[field] = copy.deepcopy(fixtures["vapor_dry_thermal_gradient"][field])
        off["head"][0][0] += 100.0
        res = self.disc(fixtures)
        head = [c for c in res["checks"] if c["field"] == "head"][0]
        self.assertEqual(head["verdict"], "FAIL")

    def test_nonzero_rule(self):
        res = self.disc(self.fixtures(vapor=1.0))
        nz = [c for c in res["checks"] if c["rule"] == "max_abs_exceeds_zero_floor_each_case"
              and c["name"] == "vapor_fields_nonzero"]
        self.assertEqual(len(nz), 15)
        self.assertTrue(all(c["verdict"] == "PASS" for c in nz))
        res = self.disc(self.fixtures(vapor=0.0))
        nz = [c for c in res["checks"] if c["rule"] == "max_abs_exceeds_zero_floor_each_case"
              and c["name"] == "vapor_fields_nonzero"]
        self.assertTrue(all(c["verdict"] == "FAIL" for c in nz))

    def test_nonzero_floor_is_per_time(self):
        fixtures = self.fixtures(vapor=0.0)
        # t=0 huge, t=1 tiny but its own max point is above its own per-time floor
        fixtures["vapor_thermal_gradient"]["vapor_fields"]["con_vh"] = [[5.0, 0.0, 0.0], [1e-9, 0.0, 0.0]]
        res = self.disc(fixtures)
        c = [c for c in res["checks"] if c["case_id"] == "vapor_thermal_gradient" and c["field"] == "con_vh"][0]
        self.assertEqual(c["verdict"], "PASS")
        self.assertEqual(c["floor_semantics"], "per_time_fraction_of_max_over_nodes")

    def test_nonzero_requires_point_after_initial(self):
        fixtures = self.fixtures(vapor=0.0)
        for name in ("vapor_thermal_gradient",):
            fixtures[name]["vapor_fields"]["con_vh"][0][0] = 5.0  # only at t = 0
        res = self.disc(fixtures)
        con = [c for c in res["checks"] if c["case_id"] == "vapor_thermal_gradient" and c["field"] == "con_vh"][0]
        self.assertEqual(con["verdict"], "FAIL")

    def test_unknown_rule_rejected_and_missing_case(self):
        crit = copy.deepcopy(self.criteria)
        crit["discrimination"]["dry_on_off_head"]["rule"] = "bogus"
        with self.assertRaises(cmp.Reject):
            cmp.validate_criteria(crit)
        fixtures = self.fixtures()
        fixtures.pop("vapor_dry_off_control")
        with self.assertRaises(cmp.Reject):
            self.disc(fixtures)

    def test_dry_discrimination_summary_excluded_by_schema(self):
        d = Path(self.tmp.name) / "fx"
        d.mkdir()
        (d / "summary.json").write_text(json.dumps({"schema_version": 1, "cases": []}))
        (d / "zz.json").write_text(json.dumps(fixture("only")))
        self.assertEqual(list(cmp.load_fixture_dir(d)), ["only"])

    def test_real_fixtures_load(self):
        fixtures = cmp.load_fixture_dir(cmp.DEFAULT_FIXTURE_DIR)
        self.assertIn("vapor_dry_off_control", fixtures)
        self.assertNotIn("dry_discrimination", fixtures)
        res = cmp.discrimination(self.criteria, self.csha, fixtures)
        self.assertIn(res["verdict"], ("PASS", "FAIL"))


class CliTests(Base):
    def call(self, argv):
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = cmp.main(argv)
        return code, out.getvalue(), err.getvalue()

    def test_cli_trajectory_and_overwrite_refusal(self):
        fx, fsha, cand = self.setup_case()
        cpath = self.write("cand.json", cand)
        out = self.dir / "out.json"
        argv = ["trajectory", "--fixture", str(self.dir / "fx.json"), "--candidate", str(cpath),
                "--output", str(out)]
        code, text, _ = self.call(argv)
        self.assertEqual(code, 0)
        self.assertIn("verdict: PASS", text)
        self.assertEqual(json.loads(out.read_text())["criteria_sha256"], self.csha)
        code, _, err = self.call(argv)
        self.assertEqual(code, 2)
        self.assertIn("overwrite", err)

    def test_cli_failure_exit_and_invalid(self):
        fx, fsha, cand = self.setup_case()
        cand["head"][1][1] = 0.0
        cpath = self.write("cand.json", cand)
        argv = ["trajectory", "--fixture", str(self.dir / "fx.json"), "--candidate", str(cpath),
                "--output", str(self.dir / "o1.json")]
        self.assertEqual(self.call(argv)[0], 1)
        cand["head"][1][1] = math.nan
        cpath.write_text(json.dumps(cand))  # NaN literal rejected at load
        argv[-1] = str(self.dir / "o2.json")
        self.assertEqual(self.call(argv)[0], 2)


DECKS = ("SELECTOR.IN", "PROFILE.DAT")


def traj_candidate(fx, fsha, csha):
    """Minimal valid trajectory candidate built from a fixture (27.4.1 optional-key tests)."""
    return {
        "schema_version": cmp.CANDIDATE_SCHEMA, "case_id": fx["case_id"],
        "evidence_class": "rust-candidate", "criteria_sha256": csha, "fixture_sha256": fsha,
        "input_sha256": {d: fx["manifest"]["input_sha256"][d] for d in DECKS},
        "schedule": {"kind": "constant_dt", "dt": 0.1, "retry_factor": 3.0, "note": "n"},
        "time_points": copy.deepcopy(fx["time_points"]),
        "node_numbers": copy.deepcopy(fx["node_numbers"]),
        "node_depths": copy.deepcopy(fx["node_depths"]),
        "head": copy.deepcopy(fx["head"]), "liquid_moisture": copy.deepcopy(fx["liquid_moisture"]),
        "temperature_c": copy.deepcopy(fx["temperature_c"]),
        "vapor_output_available": fx["vapor_output_available"],
        "vapor_fields": copy.deepcopy(fx["vapor_fields"]),
    }


class CandidateOptionalKeyTests(Base):
    """27.4.1: requests_sha256 (top level) and schedule.step_on_dt are optional; others still rejected."""

    def setUp(self):
        super().setUp()
        self.fx = fixture("vapor_nonuniform_step_on")
        self.fsha = "9" * 64
        self.cand = traj_candidate(self.fx, self.fsha, self.csha)

    def run_cand(self, cand=None):
        return cmp.compare_trajectory(self.criteria, self.csha, self.fx, self.fsha,
                                      self.cand if cand is None else cand)

    def test_without_optional_keys_accepted(self):
        self.assertEqual(self.run_cand()["verdict"], "PASS")

    def test_requests_sha256_accepted_when_hex(self):
        self.cand["requests_sha256"] = "e" * 64
        self.assertEqual(self.run_cand()["verdict"], "PASS")

    def test_requests_sha256_must_be_hex(self):
        for bad in ("E" * 64, "e" * 63, "zz" * 32, 7):
            self.cand["requests_sha256"] = bad
            with self.assertRaises(cmp.Reject):
                self.run_cand()

    def test_step_on_dt_accepted_when_nonempty_string(self):
        self.cand["schedule"]["step_on_dt"] = "deck dt / retry_factor"
        self.assertEqual(self.run_cand()["verdict"], "PASS")

    def test_step_on_dt_must_be_nonempty_string(self):
        for bad in ("", None, 0.0333):
            self.cand["schedule"]["step_on_dt"] = bad
            with self.assertRaises(cmp.Reject):
                self.run_cand()

    def test_unknown_top_level_and_schedule_keys_still_rejected(self):
        self.cand["extra"] = 1
        with self.assertRaises(cmp.Reject):
            self.run_cand()
        del self.cand["extra"]
        self.cand["schedule"]["extra"] = "x"
        with self.assertRaises(cmp.Reject):
            self.run_cand()

    def test_required_keys_still_required(self):
        for key in ("criteria_sha256", "time_points"):
            bad = copy.deepcopy(self.cand)
            del bad[key]
            with self.assertRaises(cmp.Reject):
                self.run_cand(bad)
        bad = copy.deepcopy(self.cand)
        del bad["schedule"]["note"]
        with self.assertRaises(cmp.Reject):
            self.run_cand(bad)

    def test_evidence_class_must_be_rust_candidate(self):
        self.cand["evidence_class"] = "candidate-rust"
        with self.assertRaises(cmp.Reject):
            self.run_cand()


if __name__ == "__main__":
    unittest.main()

"""Unit tests for the solute parity output normalizers.

Tests verify fixed/scientific/D exponents, coordinate reversal, print intervals,
unavailability reasons, rejection of malformed/duplicate/missing rows, and validates
every fixture OBSERVATION_MAP.json across Wave 3 and Wave 7 without launching solvers.
"""

from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from decimal import Decimal
from pathlib import Path

from tools import solute_parity_normalizers as normalizers

ROOT = Path(__file__).resolve().parents[1]
PARITY_FIXTURE_ROOT = ROOT / "reference/fortran_reference/solute/parity"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class TestSoluteParityNormalizers(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def test_fortran_numeric_parsing_and_d_exponents(self):
        # Standard decimal
        self.assertEqual(normalizers.parse_decimal_token("12.345"), Decimal("12.345"))
        self.assertEqual(normalizers.parse_decimal_token("-0.001"), Decimal("-0.001"))
        # E scientific
        self.assertEqual(normalizers.parse_decimal_token("1.2345E+02"), Decimal("123.45"))
        self.assertEqual(normalizers.parse_decimal_token("-5.4321e-03"), Decimal("-0.0054321"))
        # D Fortran exponent
        self.assertEqual(normalizers.parse_decimal_token("3.14159D+00"), Decimal("3.14159"))
        self.assertEqual(normalizers.parse_decimal_token("-2.71828d-02"), Decimal("-0.0271828"))
        # Malformed
        with self.assertRaises(ValueError):
            normalizers.parse_decimal_token("invalid_num")

    def test_precision_interval_half_width(self):
        # "0.4300E+00" -> decimals=4, exp=0 -> unit_exp = -4 -> half_width = 5e-5 = 0.00005
        hw1 = normalizers.compute_half_width("0.4300E+00")
        self.assertEqual(hw1, Decimal("0.00005"))

        # Fixed decimal: "12.345" -> decimals=3 -> unit_exp = -3 -> half_width = 5e-4 = 0.0005
        hw2 = normalizers.compute_half_width("12.345")
        self.assertEqual(hw2, Decimal("0.0005"))

        # D-exponent: "1.00000D-01" -> decimals=5, exp=-1 -> unit_exp = -6 -> half_width = 5e-7
        hw3 = normalizers.compute_half_width("1.00000D-01")
        self.assertEqual(hw3, Decimal("0.0000005"))

    def test_all_fixture_observation_maps_validate_against_contract(self):
        maps = sorted(PARITY_FIXTURE_ROOT.rglob("OBSERVATION_MAP.json"))
        self.assertGreater(len(maps), 40, f"Expected > 40 observation maps, found {len(maps)}")
        for map_path in maps:
            with self.subTest(map_path=str(map_path.relative_to(ROOT))):
                data = normalizers.load_json(map_path)
                validated = normalizers.validate_observation_map(data)
                self.assertEqual(validated["schema_version"], normalizers.OBSERVATION_MAP_SCHEMA)
                self.assertIsInstance(validated["observations"], list)
                self.assertGreater(len(validated["observations"]), 0)

    def test_duplicate_observation_identity_rejected(self):
        map_data = {
            "schema_version": normalizers.OBSERVATION_MAP_SCHEMA,
            "observations": [
                {
                    "field": "Conc",
                    "location": {"node": 1, "time": 0.1, "species": 1},
                    "units": "C_ref",
                    "raw_file": "solute1.out",
                    "source_column": 1,
                },
                {
                    "field": "Conc",
                    "location": {"node": 1, "time": 0.1, "species": 1},
                    "units": "C_ref",
                    "raw_file": "solute1.out",
                    "source_column": 2,
                },
            ],
        }
        with self.assertRaisesRegex(ValueError, "duplicate observation"):
            normalizers.validate_observation_map(map_data)

    def test_unavailable_field_without_reason_rejected(self):
        map_data = {
            "schema_version": normalizers.OBSERVATION_MAP_SCHEMA,
            "observations": [
                {
                    "field": "Sorb2",
                    "location": {"node": 1},
                    "units": "C_ref",
                    "availability": "unavailable",
                    # Missing reason
                }
            ],
        }
        with self.assertRaisesRegex(ValueError, "explicit reason"):
            normalizers.validate_observation_map(map_data)

    def test_authenticate_runner_result(self):
        task_root = self.root / "task"
        task_root.mkdir()
        run_root = task_root / "run"
        run_root.mkdir()
        output_file = task_root / "solute1.out"
        content = b"sample output"
        output_file.write_bytes(content)

        manifest = {
            "complete": True,
            "task_root": str(task_root),
            "run_root": str(run_root),
            "input_sha256": {"SELECTOR.IN": "a" * 64},
            "output_paths": {"solute1": "solute1.out"},
            "output_sha256": {"solute1": digest(content)},
        }

        # Successful authentication
        normalizers.authenticate_runner_result(manifest, task_root, {"SELECTOR.IN": "a" * 64})

        # Tampered output file
        output_file.write_bytes(b"tampered")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            normalizers.authenticate_runner_result(manifest, task_root, {"SELECTOR.IN": "a" * 64})

        # Restored content, but run not complete
        output_file.write_bytes(content)
        bad_manifest = dict(manifest, complete=False)
        with self.assertRaisesRegex(ValueError, "not complete"):
            normalizers.authenticate_runner_result(bad_manifest, task_root, {"SELECTOR.IN": "a" * 64})

    def test_normalize_synthetic_profile_and_solute_outputs(self):
        task_root = self.root / "task_synth"
        task_root.mkdir()
        run_root = task_root / "run"
        run_root.mkdir()

        # PROFILE.OUT with 2 nodes (node 2 top, node 1 bottom)
        profile_content = (
            " Time:        0.1000\n\n"
            " Node      Depth      Head Moisture       K          C         Flux        Sink         Kappa   v/KsTop   Temp   Conc(1..NS) Sorb(1...NS)\n"
            "   2     0.0000    -10.000  0.3000   1.0000E-01  1.0000E-02  1.0000E-01  0.0000E+00       1   1.000E+00   20.00   1.5000D-01  2.5000D-02\n"
            "   1    -5.0000    -15.000  0.2800   8.0000E-02  1.1000E-02  1.0000E-01  0.0000E+00       1   1.000E+00   20.00   5.0000D-02  1.0000D-02\n"
            "end\n"
        ).encode("utf-8")
        (run_root / "PROFILE.OUT").write_bytes(profile_content)

        # solute1.out with 1 record
        solute_content = (
            "       Time         cvTop        cvBot      Sum(cvTop)   Sum(cvBot)     cvCh0        cvCh1         cTop        cRoot         cBot\n"
            "        0.1000   1.23450E-01  5.43210E-02  1.23450E-02  5.43210E-03  0.00000E+00  0.00000E+00  1.50000E-01  0.00000E+00  5.00000E-02\n"
        ).encode("utf-8")
        (run_root / "solute1.out").write_bytes(solute_content)

        map_data = {
            "schema_version": normalizers.OBSERVATION_MAP_SCHEMA,
            "observations": [
                {
                    "field": "Conc",
                    "location": {"node": "profile", "time": 0.1, "species": 1},
                    "units": "C_ref",
                    "raw_file": "PROFILE.OUT",
                    "source_column": 12,
                    "source_format": "e12.4",
                },
                {
                    "field": "Sorb",
                    "location": {"node": "profile", "time": 0.1, "species": 1},
                    "units": "C_ref",
                    "raw_file": "PROFILE.OUT",
                    "source_column": 13,
                    "source_format": "e12.4",
                },
                {
                    "field": "Sorb2",
                    "location": {"node": "profile", "time": 0.1, "species": 1},
                    "units": "C_ref",
                    "availability": "unavailable",
                    "reason": "dual-nonequilibrium path not active",
                },
                {
                    "field": "cvTop",
                    "location": {"time": 0.1, "species": 1},
                    "units": "M/L2/T",
                    "raw_file": "solute1.out",
                    "source_column": 2,
                    "source_format": "e13.5",
                },
            ],
        }
        map_path = task_root / "OBSERVATION_MAP.json"
        map_path.write_text(json.dumps(map_data), encoding="utf-8")

        manifest = {
            "complete": True,
            "task_root": str(task_root),
            "run_root": str(run_root),
            "output_paths": {
                "profile": "run/PROFILE.OUT",
                "solute1": "run/solute1.out",
            },
            "output_sha256": {
                "profile": digest(profile_content),
                "solute1": digest(solute_content),
            },
        }
        manifest_path = task_root / "manifest.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

        norm_obs = normalizers.normalize_fortran_outputs(map_path, manifest_path, task_root)
        self.assertEqual(len(norm_obs), 4)

        # Conc
        self.assertEqual(norm_obs[0]["field"], "Conc")
        self.assertEqual(norm_obs[0]["value"], [0.15, 0.05])
        self.assertEqual(norm_obs[0]["availability"], "available")

        # Sorb
        self.assertEqual(norm_obs[1]["field"], "Sorb")
        self.assertEqual(norm_obs[1]["value"], [0.025, 0.01])
        self.assertEqual(norm_obs[1]["availability"], "available")

        # Sorb2 unavailable
        self.assertEqual(norm_obs[2]["field"], "Sorb2")
        self.assertIsNone(norm_obs[2]["value"])
        self.assertEqual(norm_obs[2]["availability"], "unavailable")
        self.assertEqual(norm_obs[2]["reason"], "dual-nonequilibrium path not active")

        # cvTop
        self.assertEqual(norm_obs[3]["field"], "cvTop")
        self.assertAlmostEqual(norm_obs[3]["value"], 0.12345)
        self.assertEqual(norm_obs[3]["availability"], "available")

    def test_normalize_synthetic_nod_inf_obs_node_balance(self):
        task_root = self.root / "task_all"
        task_root.mkdir()
        run_root = task_root / "run"
        run_root.mkdir()

        nod_inf_content = (
            " Time:        0.1000\n\n"
            " Node      Depth      Head Moisture       K          C         Flux        Sink         Kappa   v/KsTop   Temp   Conc(1..NS) Sorb(1...NS)\n"
            "   1    -5.0000    -15.000  0.2800   8.0000E-02  1.1000E-02  1.0000E-01  0.0000E+00       1   1.000E+00   20.00   5.0000E-02  1.0000E-02\n"
        ).encode("utf-8")
        (run_root / "NOD_INF.OUT").write_bytes(nod_inf_content)

        obs_node_content = (
            "   Time       h(1)      Th(1)     Temp(1)     h(2)      Th(2)     Temp(2)\n"
            "   0.1000   -10.000    0.3000    20.000   -15.000    0.2800    20.000\n"
        ).encode("utf-8")
        (run_root / "OBS_NODE.OUT").write_bytes(obs_node_content)

        balance_content = (
            "----------------------------------------------------------\n"
            " Time       [T]        0.1000\n"
            "----------------------------------------------------------\n"
            " Area     [L]        1.00000E+00\n"
            " W-volume [L]        2.80000E+00\n"
            " W-volumeI[L]        5.00000E-01\n"
        ).encode("utf-8")
        (run_root / "BALANCE.OUT").write_bytes(balance_content)

        map_data = {
            "schema_version": normalizers.OBSERVATION_MAP_SCHEMA,
            "observations": [
                {
                    "field": "nod_head",
                    "location": {"node": 1, "time": 0.1},
                    "units": "cm",
                    "raw_file": "NOD_INF.OUT",
                    "source_column": 3,
                },
                {
                    "field": "obs1_temp",
                    "location": {"node": 1, "time": 0.1},
                    "units": "degC",
                    "raw_file": "OBS_NODE.OUT",
                    "source_column": 4,
                },
                {
                    "field": "dual_immobile_water_volume",
                    "location": {"node": "whole-profile", "time": 0.1},
                    "units": "cm",
                    "raw_file": "BALANCE.OUT",
                    "row_selector": {"kind": "labeled_record", "label": "W-volumeI[L]"},
                    "source_column": 1,
                },
            ],
        }
        map_path = task_root / "OBSERVATION_MAP.json"
        map_path.write_text(json.dumps(map_data), encoding="utf-8")

        manifest = {
            "complete": True,
            "task_root": str(task_root),
            "run_root": str(run_root),
            "output_paths": {
                "nod_inf": "run/NOD_INF.OUT",
                "obs_node": "run/OBS_NODE.OUT",
                "balance": "run/BALANCE.OUT",
            },
            "output_sha256": {
                "nod_inf": digest(nod_inf_content),
                "obs_node": digest(obs_node_content),
                "balance": digest(balance_content),
            },
        }
        manifest_path = task_root / "manifest.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

        norm_obs = normalizers.normalize_fortran_outputs(map_path, manifest_path, task_root)
        self.assertEqual(len(norm_obs), 3)
        self.assertEqual(norm_obs[0]["field"], "nod_head")
        self.assertEqual(norm_obs[0]["value"], [-15.0])
        self.assertEqual(norm_obs[1]["field"], "obs1_temp")
        self.assertEqual(norm_obs[1]["value"], 20.0)
        self.assertEqual(norm_obs[2]["field"], "dual_immobile_water_volume")
        self.assertEqual(norm_obs[2]["value"], 0.5)

    def test_rejection_of_malformed_and_out_of_range_columns(self):
        task_root = self.root / "task_err"
        task_root.mkdir()
        run_root = task_root / "run"
        run_root.mkdir()

        solute_content = b" 0.1000 1.0\n"
        (run_root / "solute1.out").write_bytes(solute_content)

        map_data = {
            "schema_version": normalizers.OBSERVATION_MAP_SCHEMA,
            "observations": [
                {
                    "field": "invalid_col",
                    "location": {"time": 0.1},
                    "units": "M/L2/T",
                    "raw_file": "solute1.out",
                    "source_column": 99,  # out of range
                }
            ],
        }
        map_path = task_root / "OBSERVATION_MAP.json"
        map_path.write_text(json.dumps(map_data), encoding="utf-8")

        manifest = {
            "complete": True,
            "task_root": str(task_root),
            "run_root": str(run_root),
            "output_paths": {"solute1": "run/solute1.out"},
            "output_sha256": {"solute1": digest(solute_content)},
        }
        manifest_path = task_root / "manifest.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

        with self.assertRaisesRegex(ValueError, "out of range"):
            normalizers.normalize_fortran_outputs(map_path, manifest_path, task_root)


if __name__ == "__main__":
    unittest.main()

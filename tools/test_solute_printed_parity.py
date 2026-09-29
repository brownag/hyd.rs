"""Focused tests for field-aware retained printed-output comparison."""

import unittest
from decimal import Decimal

from tools import solute_printed_parity as parity


class PrintedParityTests(unittest.TestCase):
    def test_native_scientific_and_fixed_format_intervals(self):
        self.assertEqual(parity._half_width("1.2345E+00", "e12.4"), Decimal("0.00005"))
        self.assertEqual(parity._half_width("-1.23456D-120", "e13.5"), Decimal("5E-126"))
        self.assertEqual(parity._half_width("-12.3456", "f10.4"), Decimal("0.00005"))

    def test_exact_interval_edges_are_included_by_comparator(self):
        lower = parity._record("fixed", "comparable", "1", 0.5, "f10.0", {})
        upper = parity._record("fixed", "comparable", "1", 1.5, "f10.0", {})
        outside = parity._record("fixed", "comparable", "1", 1.5000001, "f10.0", {})
        self.assertEqual(lower["status"], "pass")
        self.assertEqual(upper["status"], "pass")
        self.assertEqual(outside["status"], "fail")
        self.assertEqual(lower["interval"]["lower"], "0.5")
        self.assertEqual(upper["interval"]["upper"], "1.5")

    def test_small_underflow_scale_and_d_exponent(self):
        self.assertEqual(parity._half_width("1.0000D-320", "e12.4"), Decimal("5E-325"))
        self.assertEqual(parity._decimal("-2.5000D-300"), Decimal("-2.5000E-300"))

    def test_record_preserves_signed_difference_and_resolution_flag(self):
        record = parity._record(
            "concentration", "comparable", "1.0000E+00", 1.00001, "e12.4", {}
        )
        self.assertEqual(record["status"], "pass")
        self.assertGreater(Decimal(record["signed_difference_rust_minus_fortran"]), 0)
        low = parity._record("concentration", "comparable", "0.0000E+00", 0.0, "e12.4", {})
        self.assertTrue(low["low_information"])
        self.assertEqual(low["status"], "pass")

    def test_wrong_format_and_node_mapping_order_fail_closed(self):
        with self.assertRaises(ValueError):
            parity._half_width("1.23456E+00", "e12.4")
        with self.assertRaises(ValueError):
            parity._half_width("1.2345E+00", "e13.5")
        with self.assertRaisesRegex(ValueError, "node order mismatch"):
            parity._validate_node_mapping(["2", "-0.0000"], 1, 0.0)
        with self.assertRaisesRegex(ValueError, "coordinate mapping mismatch"):
            parity._validate_node_mapping(["1", "-1.0000"], 1, 0.0)

    def test_missing_counterpart_and_candidate_field_coverage_are_unavailable(self):
        missing = parity._unavailable_record(
            "head", "0.000", "f11.3", {}, parity.UNAVAILABLE_REASONS["head"]
        )
        self.assertEqual(missing["status"], "unavailable")
        self.assertIsNone(missing["rust_value"])
        self.assertIn("do not retain a mapped nodal head", missing["reason"])
        semantic = parity._unavailable_record(
            "Sorb", "0.0000E+00", "e12.4", {}, parity.UNAVAILABLE_REASONS["Sorb"]
        )
        self.assertEqual(semantic["status"], "unavailable")
        self.assertIn("not equivalent", semantic["reason"])
        self.assertEqual(
            set(parity.RAW_NODE_FIELDS) | {"Sorb"},
            set(parity.UNAVAILABLE_REASONS),
        )

    def test_unauthenticated_field_formats_keep_unknown_intervals(self):
        for field in ("moisture_capacity", "sink", "kappa", "v_over_ks_top", "temperature"):
            with self.subTest(field=field):
                _, fmt = parity.RAW_NODE_FIELDS[field]
                self.assertIsNone(fmt)
                record = parity._unavailable_record(
                    field, "0.0", fmt, {}, parity.UNAVAILABLE_REASONS[field]
                )
                self.assertEqual(record["status"], "unavailable")
                self.assertEqual(record["fortran_token"], "0.0")
                self.assertIsNone(record["source_format"])
                self.assertIsNone(record["interval"])


if __name__ == "__main__":
    unittest.main()

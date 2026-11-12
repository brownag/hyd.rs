"""Unit tests for tools/compare_heat_reference.py (24.5.2)."""

import copy
import io
import json
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

import tools.compare_heat_reference as comparator


def _fixture(temps=None, times=(0.0, 6.0)):
    row = [15.0] * comparator.NODE_COUNT
    return {
        "case_id": "synthetic_case",
        "source_matched": True,
        "temperature_unit": "C",
        "time_points": list(times),
        "node_numbers": list(range(1, comparator.NODE_COUNT + 1)),
        "temperatures": temps if temps is not None else [row, list(row)],
    }


def _candidate(fixture, offset=0.0, index=0):
    temps = copy.deepcopy(fixture["temperatures"])
    temps[1][index] += offset
    return {
        "case_id": fixture["case_id"],
        "time_points": fixture["time_points"],
        "temperatures": temps,
    }


class StrictAndRoundedVerdicts(unittest.TestCase):
    def test_exact_match_passes_both(self):
        fixture = _fixture()
        row = comparator.compare_case(fixture, _candidate(fixture))[1]
        self.assertEqual(row["failing"], 0)
        self.assertEqual(row["max_strict"], 0.0)
        self.assertTrue(row["rounded_pass"])

    def test_strict_boundary_counts_nodes_above_2e4(self):
        fixture = _fixture()
        cand = _candidate(fixture, offset=2e-4, index=0)
        row = comparator.compare_case(fixture, cand)[1]
        self.assertEqual(row["failing"], 0, "exactly 2e-4 is within tolerance")
        cand = _candidate(fixture, offset=2.5e-4, index=0)
        row = comparator.compare_case(fixture, cand)[1]
        self.assertEqual(row["failing"], 1)
        self.assertEqual(row["failing_nodes"][0][0], 1)
        self.assertAlmostEqual(row["max_strict"], 2.5e-4)

    def test_rounded_rule_passes_within_one_hundredth(self):
        fixture = _fixture()
        # 15.004 rounds to 15.00, which is within 0.01 of the fixture (15.0).
        cand = _candidate(fixture, offset=0.004, index=3)
        row = comparator.compare_case(fixture, cand)[1]
        self.assertTrue(row["rounded_pass"])
        self.assertEqual(row["failing"], 1, "strict verdict still reports the miss")

    def test_rounded_rule_fails_above_one_hundredth(self):
        fixture = _fixture()
        cand = _candidate(fixture, offset=0.02, index=3)
        row = comparator.compare_case(fixture, cand)[1]
        self.assertFalse(row["rounded_pass"])
        self.assertAlmostEqual(row["max_rounded"], 0.02)

    def test_round_ties_upward(self):
        self.assertEqual(comparator.round2(15.005), 15.01)
        self.assertEqual(comparator.round2(15.004), 15.0)


class SchemaRejection(unittest.TestCase):
    def test_rejects_non_source_matched_fixture(self):
        fixture = _fixture()
        fixture["source_matched"] = False
        with self.assertRaises(ValueError):
            comparator.compare_case(fixture, _candidate(_fixture()))

    def test_rejects_time_point_mismatch(self):
        fixture = _fixture()
        cand = _candidate(fixture)
        cand["time_points"] = [0.0, 7.0]
        with self.assertRaises(ValueError):
            comparator.compare_case(fixture, cand)

    def test_rejects_wrong_node_count(self):
        fixture = _fixture(temps=[[15.0] * 40, [15.0] * 40])
        with self.assertRaises(ValueError):
            comparator.validate_fixture(fixture)

    def test_rejects_non_finite_candidate(self):
        fixture = _fixture()
        cand = _candidate(fixture)
        cand["temperatures"][1][0] = float("nan")
        with self.assertRaises(ValueError):
            comparator.compare_case(fixture, cand)


class TableAndExit(unittest.TestCase):
    def _write(self, directory, fixture, candidate):
        Path(directory, "fixtures").mkdir()
        Path(directory, "candidates").mkdir()
        Path(directory, "fixtures", "synthetic_case.json").write_text(json.dumps(fixture))
        Path(directory, "candidates", "synthetic_case.json").write_text(json.dumps(candidate))

    def test_table_lists_both_verdicts_and_exit_status(self):
        fixture = _fixture()
        with tempfile.TemporaryDirectory() as tmp:
            self._write(tmp, fixture, _candidate(fixture, offset=0.004, index=0))
            buffer = io.StringIO()
            with redirect_stdout(buffer):
                status = comparator.main(
                    ["--fixture-dir", f"{tmp}/fixtures", "--candidate-dir", f"{tmp}/candidates"]
                )
            text = buffer.getvalue()
        self.assertEqual(status, 0)
        self.assertIn("max_strict", text)
        self.assertIn("rounded", text)
        self.assertIn("synthetic_case", text)
        self.assertTrue(text.splitlines()[1].endswith("pass"))

    def test_exit_status_one_on_rounded_failure(self):
        fixture = _fixture()
        with tempfile.TemporaryDirectory() as tmp:
            self._write(tmp, fixture, _candidate(fixture, offset=0.5, index=0))
            with redirect_stdout(io.StringIO()):
                status = comparator.main(
                    ["--fixture-dir", f"{tmp}/fixtures", "--candidate-dir", f"{tmp}/candidates"]
                )
        self.assertEqual(status, 1)


if __name__ == "__main__":
    unittest.main()

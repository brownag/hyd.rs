"""Focused parser and format tests for the full vapor reference producer."""
import io
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


if __name__ == "__main__":
    unittest.main()

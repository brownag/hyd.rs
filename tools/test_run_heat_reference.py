"""Focused checks for the heat fixture producer: parser, finiteness refusal, and manifest shape."""

import json
import unittest

from tools import run_heat_reference as heat


def row(node: int, depth: float, temp_text: str) -> str:
    """One NodOut row in the format-120 column layout (OUTPUT.FOR:820-821)."""
    head = f"{node:4d} {depth:10.4f} {-10.0:11.3f} {0.4254:6.4f} "
    k_block = "".join(f"{0.282:12.4e}" for _ in range(4))
    tail = f"{-1:8d} {-0.656:11.3e}{temp_text:>8}"
    line = head + k_block + tail
    assert len(line) == 111, len(line)
    return line


def nod_inf(blocks: list[tuple[float, list[str]]]) -> str:
    out = []
    for time, rows in blocks:
        out.append(f" Time:{time:14.4f}")
        out.append(" Node      Depth      Head Moisture")
        out.extend(rows)
        out.append("end")
    return "\n".join(out) + "\n"


class ParserTests(unittest.TestCase):
    def test_parses_two_time_blocks_in_order(self):
        text = nod_inf([
            (6.0, [row(1, 0.0, "  25.00"), row(2, -1.0, "  15.00")]),
            (12.0, [row(1, 0.0, "  25.00"), row(2, -1.0, "  14.50")]),
        ])
        blocks = heat.parse_nod_inf(text)
        self.assertEqual([b["time"] for b in blocks], [6.0, 12.0])
        self.assertEqual(blocks[0]["node"], [1, 2])
        self.assertEqual(blocks[0]["depth"], [0.0, -1.0])
        self.assertEqual(blocks[1]["temp"], [25.0, 14.5])

    def test_rejects_field_overflow(self):
        bad = row(1, 0.0, "  25.00").replace("  25.00", "********")
        with self.assertRaisesRegex(ValueError, "overflow"):
            heat.parse_nod_inf(nod_inf([(6.0, [bad])]))

    def test_rejects_missing_end_marker(self):
        text = " Time:        6.0000\n" + row(1, 0.0, "  25.00") + "\n"
        with self.assertRaisesRegex(ValueError, "end"):
            heat.parse_nod_inf(text)


class FinitenessRefusalTests(unittest.TestCase):
    def test_finite_output_passes_guard(self):
        blocks = heat.parse_nod_inf(nod_inf([(6.0, [row(1, 0.0, "  25.00")])]))
        heat.guard_finite("case_x", blocks)  # must not raise
        self.assertIsNone(heat.first_nonfinite(blocks))

    def test_nan_is_refused_with_time_and_node(self):
        blocks = heat.parse_nod_inf(nod_inf([
            (6.0, [row(1, 0.0, "  25.00"), row(2, -1.0, "  15.00")]),
            (12.0, [row(1, 0.0, "  25.00"), row(2, -1.0, "     NaN")]),
        ]))
        hit = heat.first_nonfinite(blocks)
        self.assertEqual((hit["time"], hit["node"]), (12.0, 2))
        with self.assertRaisesRegex(heat.NonFiniteOutput, "refused fixture.*time 12.0.*node 2"):
            heat.guard_finite("case_x", blocks)

    def test_refusal_reason_is_serializable_and_has_no_tolerance(self):
        blocks = heat.parse_nod_inf(nod_inf([(6.0, [row(1, 0.0, "     NaN")])]))
        with self.assertRaises(heat.NonFiniteOutput) as caught:
            heat.guard_finite("case_x", blocks)
        record = json.dumps({"case_id": "case_x", "refused": True, "reason": str(caught.exception)})
        self.assertIn("refused", record)
        self.assertNotIn("tolerance", record.lower())


if __name__ == "__main__":
    unittest.main()

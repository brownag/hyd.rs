"""Every casebook loader must see the same per-row timeout override."""
import os
import unittest
from pathlib import Path

from tools import check_solute_verification_spec as contract
from tools import solute_benchmark_lifecycle as lifecycle

REPO = Path(__file__).resolve().parents[1]
CASES = REPO / "reference" / "solute_verification" / "CASES_V2.json"
ROW = "convergence-example3-space_1001-rfc-baseline"
ROW_TIME_CONTROL = "convergence-example3-time_control-1001-dt0.025-rfc-baseline"


class TimeoutOverrideTests(unittest.TestCase):
    def setUp(self):
        self.saved = dict(contract.TIMEOUT_OVERRIDES)

    def tearDown(self):
        contract.TIMEOUT_OVERRIDES.clear()
        contract.TIMEOUT_OVERRIDES.update(self.saved)

    def row_timeout(self, book, rid=ROW):
        return next(r for r in book["launches"] if r["id"] == rid)["timeout_seconds"]

    def test_default_is_casebook_value(self):
        contract.TIMEOUT_OVERRIDES.clear()
        self.assertEqual(self.row_timeout(contract.validate(contract.load_cases(CASES))), 60)

    def test_every_loader_sees_the_override(self):
        contract.TIMEOUT_OVERRIDES.clear()
        contract.TIMEOUT_OVERRIDES.update({ROW: 180})
        self.assertIs(lifecycle.TIMEOUT_OVERRIDES, contract.TIMEOUT_OVERRIDES)
        direct = contract.validate(contract.load_cases(CASES))
        self.assertEqual(self.row_timeout(direct), 180)
        root = REPO / ".development" / "23-controls-4"
        managed = lifecycle.Lifecycle(CASES, root).book
        self.assertEqual(self.row_timeout(managed), 180)
        self.assertEqual(self.row_timeout(contract.validate(contract.load_cases(CASES), False)), 180)

    def test_rejects_unknown_launch_and_out_of_range(self):
        contract.TIMEOUT_OVERRIDES.clear()
        contract.TIMEOUT_OVERRIDES.update({"convergence-no-such-row": 180})
        with self.assertRaises(ValueError):
            contract.validate(contract.load_cases(CASES))
        contract.TIMEOUT_OVERRIDES.clear()
        contract.TIMEOUT_OVERRIDES.update({ROW: 900})
        with self.assertRaises(ValueError):
            contract.validate(contract.load_cases(CASES))
        contract.TIMEOUT_OVERRIDES.clear()
        contract.TIMEOUT_OVERRIDES.update({"convergence-diffusion_cosine-space_17-rfc-baseline": 120})
        with self.assertRaises(ValueError):
            contract.validate(contract.load_cases(CASES))

    def test_patched_book_revalidates_against_v1_source(self):
        contract.TIMEOUT_OVERRIDES.clear()
        contract.TIMEOUT_OVERRIDES.update({ROW: 180})
        patched = contract.validate(contract.load_cases(CASES))
        self.assertEqual(self.row_timeout(patched), 180)
        again = contract.validate(patched, verify_hashes=False)
        self.assertEqual(self.row_timeout(again), 180)

    def test_example3_time_control_override_approved(self):
        contract.TIMEOUT_OVERRIDES.clear()
        self.assertEqual(180, contract.APPROVED_TIMEOUT_OVERRIDES[ROW_TIME_CONTROL])
        book = contract.load_cases(CASES)
        self.assertEqual(self.row_timeout(book, ROW_TIME_CONTROL), 180)
        contract.TIMEOUT_OVERRIDES.update({ROW_TIME_CONTROL: 180})
        patched = contract.validate(book)
        self.assertEqual(self.row_timeout(patched, ROW_TIME_CONTROL), 180)
        self.assertEqual(self.row_timeout(patched, ROW), 60)
        contract.TIMEOUT_OVERRIDES.clear()
        contract.TIMEOUT_OVERRIDES.update({ROW_TIME_CONTROL: 120})
        with self.assertRaises(ValueError):
            contract.validate(contract.load_cases(CASES))


if __name__ == "__main__":
    unittest.main()

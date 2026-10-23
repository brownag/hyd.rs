"""Mocked analytical work tests; no scientific solver or inversion runs."""

import copy
import decimal
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools import solute_analytical_reference as analytical
from tools import solute_reference_cache as cache
from tools.test_solute_analytical_reference import CASES, synthetic_chain_order


def component_fixture(case, coordinates, analysis_times, diagnostic_times):
    """A small independently authored zero-rate homogeneous reference."""
    values = case["initial"]["values"]
    profiles = [
        {"time": t, "concentration": [[v] * len(coordinates) for v in values]}
        for t in diagnostic_times
        if t in [0, *analysis_times]
    ]
    inventories = {
        "dissolved": [case["theta"] * case["length"] * c for c in values],
        "equilibrium_sorbed": [
            case["rho"] * s["kd"] * case["length"] * c for s, c in zip(case["species"], values)
        ],
    }
    inventories["total"] = [a + b for a, b in zip(*inventories.values())]
    continuum = [
        {
            "time": t,
            **copy.deepcopy(inventories),
            "endpoint_flux": [[0, 0] for _ in values],
            "boundary_cumulative": [[0, 0] for _ in values],
            "production_cumulative": [0] * len(values),
            "loss_cumulative": [0] * len(values),
        }
        for t in diagnostic_times
    ]
    uncertainty = copy.deepcopy({"profiles": profiles, "continuum": continuum})
    for rows in uncertainty.values():
        for row in rows:
            for key, value in row.items():
                if key != "time":
                    row[key] = [[0] * len(v) if isinstance(v, list) else 0 for v in value]
    return {
        "schema_version": "solute-metrics-reference-v1",
        "coordinates": coordinates,
        "species": [s["name"] for s in case["species"]],
        "profiles": profiles,
        "continuum": continuum,
        "uncertainty": uncertainty,
    }


class EncodingTests(unittest.TestCase):
    def test_exact_decimal_binary_float_and_keys(self):
        values = {
            ("profiles", 0.0, "x", 1, 2): decimal.Decimal("-0.00000"),
            "decimals": [
                decimal.Decimal("1.234567890123456789E-450"),
                decimal.Decimal("0E+200"),
                decimal.Decimal("1.00"),
            ],
            "floats": [-0.0, 0.1],
            "types": [True, False, None, 123, "text"],
        }
        encoded = cache.encode(values)
        with decimal.localcontext() as context:
            context.prec = 2
            decoded = cache.decode(json.loads(cache.canonical(encoded)))
        self.assertEqual(cache.encode(decoded), encoded)
        self.assertEqual(decoded["decimals"][2].as_tuple(), decimal.Decimal("1.00").as_tuple())
        self.assertNotEqual(cache.fingerprint([0]), cache.fingerprint([0.0]))
        self.assertNotEqual(cache.fingerprint([0.0]), cache.fingerprint([-0.0]))

    def test_rejects_invalid_encodings(self):
        values = [
            ["surprise", 1],
            ["int", "01"],
            ["float", "nan"],
            ["decimal", "Infinity"],
            ["bool", 1],
            ["none", None],
            ["dict", [[["str", "a"], ["int", "1"]], [["str", "a"], ["int", "2"]]]],
        ]
        for value in values:
            with self.subTest(value=value), self.assertRaises(ValueError):
                cache.decode(value)
        for value in (float("nan"), decimal.Decimal("NaN"), object()):
            with self.assertRaises(ValueError):
                cache.encode(value)
        with self.assertRaises(ValueError):
            cache.unique_object([("a", 1), ("a", 2)])


class CacheTests(unittest.TestCase):
    def setUp(self):
        scratch = cache.ROOT / ".development/13.3.5"
        scratch.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "cache"
        self.store = cache.ReferenceCache(self.root)
        self.arguments = (copy.deepcopy(CASES["example3"]), [0, 125, 250], [50], [0, 50, 200])

    def generate(self, **kwargs):
        with patch.object(analytical, "_chain_order", side_effect=synthetic_chain_order) as calls:
            result = self.store.reference(*self.arguments, allow_work=True, **kwargs)
        return result, calls.call_count

    def entry(self):
        return next(p for p in self.root.iterdir() if p.is_dir())

    def ledger(self):
        return self.entry() / "analytical-work-ledger.jsonl"

    def rows(self):
        return [json.loads(line) for line in self.ledger().read_text().splitlines()]

    def rewrite_rows(self, rows):
        previous = None
        for index, row in enumerate(rows):
            row["sequence"] = index + 1
            row["previous"] = previous
            previous = cache.hashlib.sha256(cache.canonical(row)).hexdigest()
        self.ledger().write_bytes(b"".join(cache.canonical(r) + b"\n" for r in rows))

    def test_cold_reuse_exact_results_and_unmet_admission(self):
        expected, count = self.generate()
        self.assertEqual(count, 5)
        self.assertEqual(expected[2]["status"], "unmet")
        before = {p: p.read_bytes() for p in self.entry().iterdir() if p.is_file()}
        with patch.object(analytical, "_chain_order", side_effect=AssertionError("recalculated")):
            actual = self.store.reference(*self.arguments)
            self.assertEqual(actual, expected)
            # The mathematical request is shared by different solver/dt rows.
            for _solver, _dt in (("rust", 0.1), ("fortran", 0.05)):
                self.assertEqual(self.store.reference(*self.arguments), expected)
        self.assertEqual(before, {p: p.read_bytes() for p in before})
        self.assertEqual(len([r for r in self.rows() if r["event"] == "start"]), 5)

    def test_explicit_escalation_admission_and_reuse(self):
        result, count = self.generate(allow_escalation=True)
        self.assertEqual(count, 6)
        self.assertEqual(result[2]["status"], "admitted")
        with patch.object(analytical, "_chain_order", side_effect=AssertionError("recalculated")):
            self.assertEqual(self.store.reference(*self.arguments, allow_escalation=True), result)

    def test_resume_completed_stage_without_repeating(self):
        original = cache._Entry._append

        def interrupt_before_second_start(entry, event, stage, detail):
            if event == "start" and any(r["event"] == "complete" for r in entry.records):
                raise KeyboardInterrupt("between stages")
            return original(entry, event, stage, detail)

        with patch.object(cache._Entry, "_append", interrupt_before_second_start):
            with self.assertRaises(KeyboardInterrupt):
                self.generate()
        self.assertEqual([r["event"] for r in self.rows()], ["start", "complete"])
        _, count = self.generate()
        self.assertEqual(count, 4)
        self.assertEqual(len([r for r in self.rows() if r["event"] == "start"]), 5)

    def test_interrupted_started_stage_never_repeats(self):
        with patch.object(analytical, "_chain_order", side_effect=KeyboardInterrupt):
            with self.assertRaises(KeyboardInterrupt):
                self.store.reference(*self.arguments, allow_work=True)
        with patch.object(analytical, "_chain_order") as calculation:
            with self.assertRaisesRegex(ValueError, "interrupted"):
                self.store.reference(*self.arguments, allow_work=True)
            calculation.assert_not_called()

    def test_failed_stage_never_repeats(self):
        with patch.object(analytical, "_chain_order", side_effect=ValueError("work failed")):
            with self.assertRaisesRegex(ValueError, "work failed"):
                self.store.reference(*self.arguments, allow_work=True)
        self.assertEqual([r["event"] for r in self.rows()], ["start", "failed"])
        with patch.object(analytical, "_chain_order") as calculation:
            with self.assertRaisesRegex(ValueError, "failed analytical stage"):
                self.store.reference(*self.arguments, allow_work=True)
            calculation.assert_not_called()

    def test_crash_between_artifact_and_completion_blocks(self):
        original = cache._Entry._append

        def crash(entry, event, stage, detail):
            if event == "complete":
                raise KeyboardInterrupt("after durable artifact")
            return original(entry, event, stage, detail)

        with patch.object(cache._Entry, "_append", crash):
            with self.assertRaises(KeyboardInterrupt):
                self.generate()
        self.assertEqual(len(list(self.entry().glob("stage-*.json"))), 1)
        with patch.object(analytical, "_chain_order") as calculation:
            with self.assertRaisesRegex(ValueError, "interrupted"):
                self.store.reference(*self.arguments, allow_work=True)
            calculation.assert_not_called()

    def test_complete_stage_and_reference_tampering_fail(self):
        self.generate()
        for path in (
            next(self.entry().glob("stage-*.json")),
            self.entry() / "reference.json",
            self.entry() / "identity.json",
        ):
            original = path.read_bytes()
            path.write_bytes(b"{}")
            try:
                with patch.object(analytical, "_chain_order") as calculation:
                    with self.assertRaises(ValueError):
                        self.store.reference(*self.arguments, allow_work=True)
                    calculation.assert_not_called()
            finally:
                path.write_bytes(original)

    def test_missing_stage_file_fails_without_recalculation(self):
        self.generate()
        next(self.entry().glob("stage-*.json")).unlink()
        with patch.object(analytical, "_chain_order") as calculation:
            with self.assertRaises(FileNotFoundError):
                self.store.reference(*self.arguments, allow_work=True)
            calculation.assert_not_called()

    def test_truncated_duplicate_unknown_and_orphan_ledger(self):
        self.generate()
        original = self.ledger().read_bytes()
        rows = self.rows()
        mutations = [rows[:1] + rows[:1], rows[:2] + [dict(rows[2], event="unknown")]]
        for mutated in mutations:
            self.rewrite_rows(copy.deepcopy(mutated))
            with self.assertRaises(ValueError):
                self.store.reference(*self.arguments, allow_work=True)
        self.ledger().write_bytes(original[:-1])
        with self.assertRaisesRegex(ValueError, "truncated"):
            self.store.reference(*self.arguments)
        self.ledger().write_bytes(b"")
        with self.assertRaisesRegex(ValueError, "orphan"):
            self.store.reference(*self.arguments, allow_work=True)

    def test_missing_reference_completion_blocks(self):
        self.generate()
        self.rewrite_rows(self.rows()[:-1])
        with self.assertRaisesRegex(ValueError, "incomplete reference publication"):
            self.store.reference(*self.arguments, allow_work=True)

    def test_malformed_ledger_types_fail_cleanly(self):
        self.generate()
        original = self.rows()
        for mutation in (
            [],
            {"event": "start"},
            dict(original[0], detail=[]),
            dict(original[0], detail={}),
            dict(original[0], stage=[]),
            dict(original[0], event=[]),
            dict(original[0], utc="yesterday"),
        ):
            self.ledger().write_bytes(cache.canonical(mutation) + b"\n")
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.store.reference(*self.arguments)

    def test_final_stage_sequence_tampering_fails(self):
        self.generate()
        path = self.entry() / "reference.json"
        data = cache.read_json(path)
        data["stages"] = list(reversed(data["stages"]))
        path.write_bytes(cache.canonical(data))
        rows = self.rows()
        rows[-1]["detail"]["sha256"] = cache.file_hash(path)
        self.rewrite_rows(rows)
        with self.assertRaisesRegex(ValueError, "reproduction mismatch"):
            self.store.reference(*self.arguments)

    def test_start_metadata_and_completion_admission_are_bound(self):
        self.generate()
        original = self.rows()
        rows = copy.deepcopy(original)
        rows[0]["detail"]["order"] = 256
        self.rewrite_rows(rows)
        with self.assertRaisesRegex(ValueError, "start metadata mismatch"):
            self.store.reference(*self.arguments)
        rows = copy.deepcopy(original)
        rows[-1]["detail"]["status"] = "admitted"
        self.rewrite_rows(rows)
        with self.assertRaisesRegex(ValueError, "completion admission mismatch"):
            self.store.reference(*self.arguments)

    def test_final_admission_is_reproduced_even_if_file_hash_is_updated(self):
        self.generate()
        path = self.entry() / "reference.json"
        data = cache.read_json(path)
        data["admission"]["status"] = "admitted"
        path.write_bytes(cache.canonical(data))
        rows = self.rows()
        rows[-1]["detail"]["sha256"] = cache.file_hash(path)
        self.rewrite_rows(rows)
        with self.assertRaisesRegex(ValueError, "reproduction mismatch"):
            self.store.reference(*self.arguments)

    def test_stage_coverage_revalidated_even_if_file_hash_is_updated(self):
        self.generate()
        path = next(self.entry().glob("stage-*.json"))
        data = cache.read_json(path)
        result = cache.decode(data["result"])
        result["values"].pop(next(iter(result["values"])))
        data["result"] = cache.encode(result)
        path.write_bytes(cache.canonical(data))
        rows = self.rows()
        for row in rows:
            if row["event"] == "complete" and path.stem == "stage-" + row["stage"]:
                row["detail"]["sha256"] = cache.file_hash(path)
        self.rewrite_rows(rows)
        with self.assertRaisesRegex(ValueError, "coverage"):
            self.store.reference(*self.arguments)

    def test_changed_inputs_do_not_reuse_partial_coverage(self):
        self.generate()
        mutations = []
        changed = copy.deepcopy(self.arguments)
        changed[0]["provenance"] += " altered"
        mutations.append(changed)
        mutations.extend(
            [
                (self.arguments[0], [0.0, 125.0, 250.0], [50], [0, 50, 200]),
                (self.arguments[0], [0, 250 / 3, 500 / 3, 250], [50], [0, 50, 200]),
                (self.arguments[0], [0, 125, 250], [50, 200], [0, 50, 200]),
                (self.arguments[0], [0, 125, 250], [50], [0, 25, 50, 200]),
            ]
        )
        for arguments in mutations:
            with self.subTest(arguments=arguments[1:]), self.assertRaisesRegex(
                ValueError, "missing"
            ):
                self.store.reference(*arguments)
        with self.assertRaisesRegex(ValueError, "missing"):
            self.store.reference(*self.arguments, allow_escalation=True)
        with decimal.localcontext() as context:
            context.prec += 1
            with self.assertRaisesRegex(ValueError, "missing"):
                self.store.reference(*self.arguments)
        with patch.object(cache, "runtime_identity", return_value={"python": "other"}):
            with self.assertRaisesRegex(ValueError, "missing"):
                self.store.reference(*self.arguments)
        original_hash = cache.file_hash
        with patch.object(
            cache,
            "file_hash",
            side_effect=lambda p: (
                "0" * 64 if str(p).endswith("solute_reference_cache.py") else original_hash(p)
            ),
        ):
            with self.assertRaisesRegex(ValueError, "missing"):
                self.store.reference(*self.arguments)

    def test_read_only_missing_cache_has_no_side_effects(self):
        with self.assertRaisesRegex(ValueError, "cache missing"):
            self.store.reference(*self.arguments)
        self.assertFalse(self.root.exists())
        self.generate()
        self.rewrite_rows(self.rows()[:-1])
        (self.entry() / "reference.json").unlink()
        with patch.object(analytical, "_chain_order") as calculation:
            with self.assertRaisesRegex(ValueError, "complete reference missing"):
                self.store.reference(*self.arguments)
            calculation.assert_not_called()

    def test_v2_phase_roots_allowed_and_others_rejected(self):
        for task in ("22-benchmark", "22-convergence", "22-holdout"):
            cache.ReferenceCache(cache.ROOT / ".development" / task / "cache")
        with self.assertRaisesRegex(ValueError, "outside"):
            cache.ReferenceCache(cache.ROOT / ".development/99-other/cache")

    def test_paths_exclusive_writes_and_lock_modes(self):
        with self.assertRaisesRegex(ValueError, "outside"):
            cache.ReferenceCache(Path(self.temp.name).parents[3] / "wrong")
        with self.assertRaisesRegex(ValueError, "traversal"):
            cache.ReferenceCache(Path(self.temp.name) / ".." / "wrong")
        link = Path(self.temp.name) / "link"
        link.symlink_to(Path(self.temp.name), target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            cache.ReferenceCache(link / "cache")
        path = Path(self.temp.name) / "exclusive"
        cache.write_new(path, b"original")
        with self.assertRaises(FileExistsError):
            cache.write_new(path, b"overwrite")
        self.assertEqual(path.read_bytes(), b"original")
        with patch.object(cache.fcntl, "flock", wraps=cache.fcntl.flock) as locks:
            self.generate()
            self.store.reference(*self.arguments)
        self.assertEqual(
            [call.args[1] for call in locks.call_args_list],
            [cache.fcntl.LOCK_EX, cache.fcntl.LOCK_SH],
        )
        path = next(self.entry().glob("stage-*.json"))
        path.unlink()
        path.symlink_to(Path(self.temp.name) / "exclusive")
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.store.reference(*self.arguments)

    def test_component_cache_and_exact_float_schedule(self):
        arguments = (CASES["reaction_zero"], [0.0, 1.0, 2.0], [1, 4], [0.0, 1.0, 4.0])
        with patch.object(
            analytical, "component_reference", side_effect=component_fixture
        ) as calls:
            expected = self.store.reference(*arguments, allow_work=True)
            self.assertEqual(calls.call_count, 1)
        with patch.object(
            analytical, "component_reference", side_effect=AssertionError("repeated")
        ):
            self.assertEqual(self.store.reference(*arguments), expected)
        self.assertEqual(expected[1], None)
        self.assertEqual(expected[2]["status"], "admitted")


if __name__ == "__main__":
    unittest.main()

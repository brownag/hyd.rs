"""Focused checks for fixture input and output validation."""

import tempfile
import unittest
from pathlib import Path

from tools import run_fortran_reference as runner


class CaseValidationTests(unittest.TestCase):
    def test_reviewed_fixture_matches_equilibrium_profile_layout(self):
        selector = (runner.CASE_ROOT / "SELECTOR.IN").read_text().splitlines()
        self.assertEqual(selector[0], "Pcp_File_Version=4")
        # BasInf reads positional values; the ninth label says lVariabBC,
        # but the corresponding source variable is AtmBC.
        flags = selector[9].split()
        self.assertEqual(len(flags), 10)
        self.assertEqual(flags[1], "t")  # lChem
        self.assertEqual(flags[9], "t")  # initial lEquil
        optional_flags = selector[11].split()
        self.assertEqual(optional_flags[2], "f")  # lMeteo
        self.assertEqual(optional_flags[5], "f")  # lFlux
        self.assertEqual(flags[8], "f")  # AtmBC
        self.assertEqual(selector[18].split()[0], "f")  # TopInF
        self.assertEqual(selector[20].split()[0], "f")  # BotInF
        self.assertEqual(selector[24].split()[0], "0")  # iModel: no Mater.in
        self.assertEqual(selector[40].split()[2], "f")  # lDualNEq

        profile = (runner.CASE_ROOT / "PROFILE.DAT").read_text().splitlines()
        description_count = int(profile[1])
        header_index = 2 + description_count
        node_count, _, species_count = map(int, profile[header_index].split())
        self.assertEqual((node_count, species_count), (1001, 3))
        nodes = profile[header_index + 1 : header_index + 1 + node_count]
        self.assertEqual(len(nodes), node_count)
        for index, line in enumerate(nodes, start=1):
            with self.subTest(node=index):
                fields = [float(value) for value in line.split()]
                self.assertEqual(len(fields), 10 + species_count)
                self.assertEqual(fields[0], index)
        self.assertEqual(profile[header_index + 1 + node_count :], ["0"])

    def test_fixture_inputs_are_complete(self):
        with tempfile.TemporaryDirectory() as temporary:
            case_dir = Path(temporary)
            for name in runner.CASE_INPUTS:
                (case_dir / name).write_text("input\n", encoding="utf-8")
            runner.require_case_inputs(case_dir)

    def test_missing_fixture_input_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(RuntimeError, "missing fixture input"):
                runner.require_case_inputs(Path(temporary))

    def test_empty_or_truncated_fixture_input_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            case_dir = Path(temporary)
            for name in runner.CASE_INPUTS:
                (case_dir / name).write_text("input\n", encoding="utf-8")
            (case_dir / "PROFILE.DAT").write_bytes(b"truncated")
            with self.assertRaisesRegex(RuntimeError, "malformed or truncated"):
                runner.require_case_inputs(case_dir)

    def test_only_reviewed_case_is_accepted(self):
        with self.assertRaisesRegex(RuntimeError, "only the reviewed"):
            runner.run_case("reference/fortran_reference/smoke", 60)

    def test_output_inventory_requires_all_per_species_files_and_markers(self):
        names = (*runner.REQUIRED_OUTPUTS, *runner.CASE_SOLUTE_OUTPUTS)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            outputs = {name: root / name for name in names}
            for name, path in outputs.items():
                marker = "Calculation time" if name == "BALANCE.OUT" else "end"
                path.write_text(f"record\n{marker}\n", encoding="utf-8")
            hashes = runner.validate_case_outputs(outputs, runner.CASE_SOLUTE_OUTPUTS)
            self.assertEqual(set(hashes), set(names))

            outputs[runner.CASE_SOLUTE_OUTPUTS[1]].unlink()
            with self.assertRaisesRegex(RuntimeError, "missing required output"):
                runner.validate_case_outputs(outputs, runner.CASE_SOLUTE_OUTPUTS)

    def test_output_completion_marker_is_required(self):
        names = (*runner.REQUIRED_OUTPUTS, *runner.CASE_SOLUTE_OUTPUTS)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            outputs = {name: root / name for name in names}
            for name, path in outputs.items():
                marker = "Calculation time" if name == "BALANCE.OUT" else "end"
                path.write_text(f"record\n{marker}\n", encoding="utf-8")
            outputs[runner.CASE_SOLUTE_OUTPUTS[0]].write_text("incomplete\n", encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "completion record"):
                runner.validate_case_outputs(outputs, runner.CASE_SOLUTE_OUTPUTS)


if __name__ == "__main__":
    unittest.main()

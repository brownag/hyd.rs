"""Focused checks for fixture input and output validation."""

import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

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


class ManifestRunnerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.development = runner.REPO_ROOT / ".development"
        cls.created_development = not cls.development.exists()
        cls.development.mkdir(exist_ok=True)

    @classmethod
    def tearDownClass(cls):
        if cls.created_development:
            cls.development.rmdir()

    def make_fixture(self, base, name, output_names=("RESULT.OUT",)):
        fixture = base / name
        fixture.mkdir()
        (fixture / "INPUT.DAT").write_text(f"{name}\n", encoding="utf-8")
        manifest = {
            "fixture_id": name,
            "runner": {
                "inputs": ["INPUT.DAT"],
                "outputs": [{"name": output, "marker": "done"} for output in output_names],
                "data_prefix": name,
            },
        }
        path = fixture / "fixture.json"
        path.write_text(json.dumps(manifest), encoding="utf-8")
        return path

    def run_with_fakes(self, manifest, task_root, *, missing=(), wrong=()):
        def fake_checked(command):
            if command[0] == "sh":
                build = runner.REPO_ROOT / command[-1]
                build.mkdir(parents=True, exist_ok=True)
                (build / "build-manifest.json").write_text(json.dumps({
                    "executable_sha256": "exe-hash", "gnu_fortran_version": "GNU Fortran test",
                    "host_os": "test-os", "host_architecture": "test-arch", "flags": ["-test"],
                }), encoding="utf-8")
                (build / "staging-manifest.json").write_text(json.dumps({
                    "source_and_archive_sha256": {"source": "source-hash"}, "patch_sha256": "patch-hash",
                }), encoding="utf-8")

        def fake_run(command, cwd, **kwargs):
            prefix = command[-1]
            config = json.loads(manifest.read_text(encoding="utf-8"))["runner"]
            for item in config["outputs"]:
                name = item["name"]
                if name not in missing:
                    marker = "incorrect" if name in wrong else item["marker"]
                    (cwd / f"{prefix}\\{name}").write_text(f"body\n{marker}\n", encoding="utf-8")
            return SimpleNamespace(returncode=0, stdout="ok\n", stderr="")

        with patch.object(runner, "run_checked", side_effect=fake_checked), patch.object(runner.subprocess, "run", side_effect=fake_run) as execute:
            result = runner.run_fixture(manifest, task_root)
            self.last_timeout_seconds = execute.call_args.kwargs["timeout"]
            return result

    def test_multiple_manifests_run_in_isolated_roots(self):
        with tempfile.TemporaryDirectory() as temp, tempfile.TemporaryDirectory(dir=self.development) as scratch:
            base = Path(temp)
            first = self.make_fixture(base, "first", ("A.OUT", "B.OUT"))
            second = self.make_fixture(base, "second", ("OTHER.OUT",))
            root = Path(scratch)
            result_a = self.run_with_fakes(first, root / "a")
            result_b = self.run_with_fakes(second, root / "b")
            self.assertTrue(result_a["complete"] and result_b["complete"])
            self.assertEqual(set(result_a["output_sha256"]), {"A.OUT", "B.OUT"})
            self.assertEqual(set(result_b["output_sha256"]), {"OTHER.OUT"})
            self.assertNotEqual(result_a["run_root"], result_b["run_root"])
            self.assertIsNone(self.last_timeout_seconds)

    def test_result_matches_wave_two_contract_and_output_paths(self):
        with tempfile.TemporaryDirectory() as temp, tempfile.TemporaryDirectory(dir=self.development) as scratch:
            manifest = self.make_fixture(Path(temp), "contract", ("A.OUT", "B.OUT"))
            root = Path(scratch) / "task"
            result = self.run_with_fakes(manifest, root)

            contract_fields = {
                "complete",
                "task_root",
                "run_root",
                "input_sha256",
                "output_paths",
                "output_sha256",
                "source_and_archive_sha256",
                "patch_sha256",
                "executable_sha256",
            }
            self.assertTrue(contract_fields.issubset(result))
            self.assertEqual(result["task_root"], str(root.resolve()))
            self.assertEqual(result["run_root"], str((root / "run").resolve()))
            self.assertEqual(set(result["output_paths"]), {"A.OUT", "B.OUT"})
            self.assertEqual(set(result["output_sha256"]), set(result["output_paths"]))
            for logical_name, relative_path in result["output_paths"].items():
                with self.subTest(output=logical_name):
                    self.assertFalse(Path(relative_path).is_absolute())
                    output = (root / relative_path).resolve()
                    self.assertTrue(output.is_relative_to(root.resolve()))
                    self.assertTrue(output.is_file())
                    self.assertEqual(result["output_sha256"][logical_name], runner.sha256(output))
                    self.assertEqual(output.name, f"contract\\{logical_name}")

            persisted = json.loads((root / "run-manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(persisted, result)

    def test_example3_cli_preserves_retained_run_manifest(self):
        before = (runner.CASE_ROOT / "RUN.md").read_bytes()
        task_root = self.development / "18.2.1-test-example3"
        with patch.object(runner, "run_fixture", return_value={"run_root": str(task_root / "run")} ) as execute:
            runner.run_case("reference/fortran_reference/solute/example3", task_root=str(task_root))
        execute.assert_called_once_with(runner.CASE_ROOT / "CASE.json", task_root, None)
        self.assertEqual((runner.CASE_ROOT / "RUN.md").read_bytes(), before)

    def test_manifest_cli_defaults_to_no_timeout(self):
        with patch.object(sys, "argv", ["run_fortran_reference.py", "--fixture-manifest", "fixture.json", "--task-root", ".development/18.2.1-cli-test"]), patch.object(
            runner, "run_fixture", return_value={"run_root": "some/run"}
        ) as execute:
            self.assertEqual(runner.main(), 0)
        self.assertEqual(execute.call_args.args[2], None)

    def test_missing_and_wrong_outputs_never_write_completion_manifest(self):
        for mode in ("missing", "wrong"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temp, tempfile.TemporaryDirectory(dir=self.development) as scratch:
                manifest = self.make_fixture(Path(temp), mode)
                root = Path(scratch) / "run"
                with self.assertRaisesRegex(RuntimeError, "missing required output|completion record"):
                    self.run_with_fakes(manifest, root, **{mode: ("RESULT.OUT",)})
                self.assertFalse((root / "run-manifest.json").exists())

    def test_existing_task_root_is_preserved(self):
        with tempfile.TemporaryDirectory() as temp, tempfile.TemporaryDirectory(dir=self.development) as scratch:
            manifest = self.make_fixture(Path(temp), "preserve")
            root = Path(scratch) / "existing"
            root.mkdir()
            sentinel = root / "keep.txt"
            sentinel.write_text("keep", encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "refusing to overwrite"):
                runner.run_fixture(manifest, root)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")

    def test_timeout_is_configurable_and_never_records_completion(self):
        with tempfile.TemporaryDirectory() as temp, tempfile.TemporaryDirectory(dir=self.development) as scratch:
            manifest = self.make_fixture(Path(temp), "timeout")
            root = Path(scratch) / "timed-out"

            def fake_checked(command):
                if command[0] == "sh":
                    build = runner.REPO_ROOT / command[-1]
                    build.mkdir(parents=True, exist_ok=True)
                    (build / "build-manifest.json").write_text("{}", encoding="utf-8")
                    (build / "staging-manifest.json").write_text("{}", encoding="utf-8")

            with patch.object(runner, "run_checked", side_effect=fake_checked), patch.object(
                runner.subprocess, "run", side_effect=runner.subprocess.TimeoutExpired(["reference"], 3, output=b"partial")
            ) as execute:
                with self.assertRaisesRegex(RuntimeError, "timed out"):
                    runner.run_fixture(manifest, root, timeout_seconds=3)
            self.assertEqual(execute.call_args.kwargs["timeout"], 3)
            self.assertFalse((root / "run-manifest.json").exists())

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

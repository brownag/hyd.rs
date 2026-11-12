"""Focused validation for the isolated pinned vapor-kernel runner."""

from __future__ import annotations

import argparse
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools import run_vapor_kernel_reference as runner


class VaporKernelReferenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.scratch = runner.REPO_ROOT / ".development/25.1.2/test-tmp"
        cls.scratch.mkdir(parents=True, exist_ok=True)

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(dir=self.scratch)
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_extracts_exact_pinned_routine(self) -> None:
        routine = runner.extract_routine(runner.SOURCE.read_bytes())
        self.assertIn(b"     !                        ths,Cap,xConv)\r\n", routine)
        self.assertEqual(runner.sha256(routine), runner.ROUTINE_SHA256)

    def test_rejects_altered_source_hash(self) -> None:
        with self.assertRaisesRegex(ValueError, "whole-source"):
            runner.extract_routine(runner.SOURCE.read_bytes() + b"\n")

    def test_rejects_ambiguous_routine_boundary(self) -> None:
        source = runner.SOURCE.read_bytes() + b"\n      subroutine VaporContent(X)\n      end\n"
        with patch.object(runner, "SOURCE_SHA256", runner.sha256(source)):
            with self.assertRaisesRegex(ValueError, "found 2"):
                runner.extract_routine(source)

    def test_rejects_request_material_mapping_and_nonfinite_values(self) -> None:
        payload = json.loads((runner.REPO_ROOT / "reference/vapor_fortran/kernel_harness/requests.json").read_text())
        payload["requests"][0]["nodes"][0]["material"] = 2
        with self.assertRaisesRegex(ValueError, "material mapping"):
            runner.validate_requests(payload)
        payload = json.loads((runner.REPO_ROOT / "reference/vapor_fortran/kernel_harness/requests.json").read_text())
        payload["requests"][0]["nodes"][0]["temperature"] = float("nan")
        with self.assertRaisesRegex(ValueError, "finite range"):
            runner.validate_requests(payload)

    def test_rejects_malformed_dimensions_and_output_values(self) -> None:
        reqs = runner.validate_requests(json.loads((runner.REPO_ROOT / "reference/vapor_fortran/kernel_harness/requests.json").read_text()))
        wrong_identity = [b"RECORD wrong 1 1.0 2.0"] + [
            f"RECORD {req['id']} {i} 1.0 2.0".encode()
            for req in reqs for i in range(1, len(req["nodes"]) + 1)
        ][1:]
        with self.assertRaisesRegex(ValueError, "identity/dimensions"):
            runner.parse_output(b"\n".join(wrong_identity + [b"COMPLETE 3 6"]), reqs)
        bad = b"\n".join([b"RECORD dry-cold 1 NaN 2.0"] + [b"RECORD dry-cold 2 1.0 2.0"] + [b"RECORD moist-warm 1 1.0 2.0", b"RECORD moist-warm 2 1.0 2.0", b"RECORD length-conversion 1 1.0 2.0", b"RECORD length-conversion 2 1.0 2.0", b"COMPLETE 3 6"])
        with self.assertRaisesRegex(ValueError, "malformed"):
            runner.parse_output(bad, reqs)

    def test_rejects_truncated_extra_and_nonfinite_output(self) -> None:
        reqs = runner.validate_requests(json.loads((runner.REPO_ROOT / "reference/vapor_fortran/kernel_harness/requests.json").read_text()))
        with self.assertRaisesRegex(ValueError, "completion"):
            runner.parse_output(b"COMPLETE 3 6\n", reqs)
        good = self._valid_output(reqs)
        with self.assertRaisesRegex(ValueError, "missing/extra"):
            runner.parse_output(good + b"EXTRA\n", reqs)
        with self.assertRaisesRegex(ValueError, "malformed or overflowed"):
            runner.parse_output(good.replace(b"1.0", b"********************", 1), reqs)
        with self.assertRaisesRegex(ValueError, "malformed or overflowed"):
            runner.parse_output(good.replace(b"1.0", b"NaN", 1), reqs)

    @staticmethod
    def _valid_output(reqs: list[dict]) -> bytes:
        records = [f"RECORD {req['id']} {i} 1.0 2.0" for req in reqs for i in range(1, len(req["nodes"]) + 1)]
        records.append(f"COMPLETE {len(reqs)} {len(records)}")
        return ("\n".join(records) + "\n").encode()

    def _sandbox_runner(self) -> tuple[Path, Path, Path]:
        base = self.root / "repo"
        development = base / ".development"
        fixtures = base / "reference/vapor_fortran/kernel_fixtures"
        development.mkdir(parents=True, exist_ok=True)
        fixtures.mkdir(parents=True, exist_ok=True)
        source = self.root / "source.for"
        driver = self.root / "driver.f90"
        source.write_bytes(runner.SOURCE.read_bytes())
        driver.write_text("program placeholder\nend program\n")
        return base, source, driver

    def test_failed_compiler_retains_failure_record(self) -> None:
        base, source, driver = self._sandbox_runner()
        request_path = runner.REPO_ROOT / "reference/vapor_fortran/kernel_harness/requests.json"
        args = argparse.Namespace(requests=request_path, task_root=".development/25.1.2/failure-compiler", output="reference/vapor_fortran/kernel_fixtures/out.json", timeout_seconds=5)
        with patch.object(runner, "REPO_ROOT", base), patch.object(runner, "SOURCE", source), patch.object(runner, "DRIVER", driver), patch.object(runner.subprocess, "run", return_value=runner.subprocess.CompletedProcess([], 1, "", "compile failed")):
            with self.assertRaisesRegex(RuntimeError, "compiler failed"):
                runner.run(args)
        record = json.loads((base / args.task_root / "attempt.json").read_text())
        self.assertEqual(record["status"], "failed")
        self.assertTrue((base / args.task_root / "compiler.stderr.txt").is_file())

    def test_compiler_timeout_retains_failure_record(self) -> None:
        base, source, driver = self._sandbox_runner()
        request_path = runner.REPO_ROOT / "reference/vapor_fortran/kernel_harness/requests.json"
        args = argparse.Namespace(requests=request_path, task_root=".development/25.1.2/failure-timeout", output="reference/vapor_fortran/kernel_fixtures/out.json", timeout_seconds=1)
        timeout = runner.subprocess.TimeoutExpired(["gfortran"], 1, output="partial", stderr="diagnostic")
        with patch.object(runner, "REPO_ROOT", base), patch.object(runner, "SOURCE", source), patch.object(runner, "DRIVER", driver), patch.object(runner.subprocess, "run", side_effect=timeout):
            with self.assertRaisesRegex(RuntimeError, "compiler timed out"):
                runner.run(args)
        root = base / args.task_root
        record = json.loads((root / "attempt.json").read_text())
        self.assertTrue(record["commands"][0]["timed_out"])
        self.assertIn("diagnostic", (root / "compiler.stderr.txt").read_text())

    def test_successful_run_publishes_provenance(self) -> None:
        base, source, driver = self._sandbox_runner()
        request_path = runner.REPO_ROOT / "reference/vapor_fortran/kernel_harness/requests.json"
        output_rel = "reference/vapor_fortran/kernel_fixtures/vapor_content.json"
        args = argparse.Namespace(requests=request_path, task_root=".development/25.1.2/success", output=output_rel, timeout_seconds=5)
        requests = runner.validate_requests(json.loads(request_path.read_text()))
        stdout = self._valid_output(requests)

        def fake_run(command: list[str], **kwargs: object) -> runner.subprocess.CompletedProcess:
            if "-o" in command:
                Path(command[command.index("-o") + 1]).write_bytes(b"fake executable")
                return runner.subprocess.CompletedProcess(command, 0, "compiled", "")
            return runner.subprocess.CompletedProcess(command, 0, stdout, b"")

        with patch.object(runner, "REPO_ROOT", base), patch.object(runner, "SOURCE", source), patch.object(runner, "DRIVER", driver), patch.object(runner.subprocess, "run", side_effect=fake_run):
            output = runner.run(args)
        fixture = json.loads(output.read_text())
        attempt = json.loads((base / args.task_root / "attempt.json").read_text())
        self.assertEqual(fixture["evidence_class"], "pinned-fortran-kernel")
        self.assertEqual(fixture["provenance"]["raw_output_sha256"], runner.sha256(stdout))
        self.assertEqual(attempt["status"], "complete")
        self.assertNotEqual(fixture["observations"][0]["input_capacity"], fixture["observations"][0]["adjusted_hydraulic_capacity"])

    def test_failed_run_and_missing_output_are_retained(self) -> None:
        for name, returncode, stdout, message in (
            ("failed-run", 7, b"partial", "exited 7"),
            ("missing-output", 0, b"", "missing/extra records"),
        ):
            with self.subTest(name=name):
                base, source, driver = self._sandbox_runner()
                request_path = runner.REPO_ROOT / "reference/vapor_fortran/kernel_harness/requests.json"
                args = argparse.Namespace(requests=request_path, task_root=f".development/25.1.2/{name}", output="reference/vapor_fortran/kernel_fixtures/out.json", timeout_seconds=5)

                def fake_run(command: list[str], **kwargs: object) -> runner.subprocess.CompletedProcess:
                    if "-o" in command:
                        Path(command[command.index("-o") + 1]).write_bytes(b"fake executable")
                        return runner.subprocess.CompletedProcess(command, 0, "", "")
                    return runner.subprocess.CompletedProcess(command, returncode, stdout, b"failure detail")

                with patch.object(runner, "REPO_ROOT", base), patch.object(runner, "SOURCE", source), patch.object(runner, "DRIVER", driver), patch.object(runner.subprocess, "run", side_effect=fake_run):
                    error_type = ValueError if name == "missing-output" else RuntimeError
                    with self.assertRaisesRegex(error_type, message):
                        runner.run(args)
                root = base / args.task_root
                self.assertEqual(json.loads((root / "attempt.json").read_text())["status"], "failed")
                self.assertTrue((root / "run.stderr.bin").is_file())

    def test_refuses_existing_root_or_fixture_destination(self) -> None:
        base, _, _ = self._sandbox_runner()
        (base / ".development/existing").mkdir()
        with self.assertRaises(FileExistsError):
            runner.safe_new_path(base / ".development/existing", base / ".development", "task root")
        (base / "reference/vapor_fortran/kernel_fixtures/out.json").symlink_to(self.root / "elsewhere")
        with self.assertRaises(FileExistsError):
            runner.safe_new_path(base / "reference/vapor_fortran/kernel_fixtures/out.json", base / "reference/vapor_fortran/kernel_fixtures", "fixture")


if __name__ == "__main__":
    unittest.main()

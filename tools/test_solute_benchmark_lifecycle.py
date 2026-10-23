"""Independent synthetic observations and mocked processes; no real builds or solvers."""

import copy
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

from tools import solute_benchmark_lifecycle as lifecycle

BOOK_PATH = lifecycle.SOURCE_ROOT / "reference/solute_verification/CASES.json"
BOOK = lifecycle.contract.validate(lifecycle.contract.load_cases(BOOK_PATH))
RUST = next(
    r
    for r in BOOK["launches"]
    if r["solver"] == "rust" and r["case_id"] == "reaction_zero" and r["role"] == "benchmark"
)
FORTRAN = next(
    r
    for r in BOOK["launches"]
    if r["solver"] == "fortran" and r["case_id"] == "reaction_zero" and r["role"] == "benchmark"
)
CASE = next(c for c in BOOK["cases"] if c["id"] == "reaction_zero")


def observations(run_dir, row=RUST, case=CASE):
    """Zero-rate homogeneous fields, authored independently of launcher/metrics helpers."""
    request = dict(
        line.split("=", 1) for line in (run_dir / "request.txt").read_text().splitlines()
    )
    data = {
        name: request[name]
        for name in ("run_id", "launch_id", "case_id", "cases_sha256", "case_sha256")
    }
    data.update(
        schema_version="solute-observation-v1",
        solver="rust",
        units=BOOK["units"],
        request_sha256=lifecycle.sha256(run_dir / "request.txt"),
        species=[s["name"] for s in case["species"]],
        coordinates=[i * case["length"] / (row["nodes"] - 1) for i in range(row["nodes"])],
    )
    concentrations = [[value] * row["nodes"] for value in case["initial"]["values"]]
    times = [
        i * row["sampling_interval"]
        for i in range(round(case["time_end"] / row["sampling_interval"]) + 1)
    ]
    steps = [i * row["dt"] for i in range(1, row["max_steps"] + 1)]
    data["profiles"] = [
        {
            "time": t,
            "concentration": copy.deepcopy(concentrations),
            "theta": [case["theta"]] * row["nodes"],
            "q_faces": [0.0] * (row["nodes"] + 1),
        }
        for t in times
    ]
    data["steps"] = [
        {"index": i, "time": t, "dt": row["dt"], "iterations": None, "peclet": 0.0, "courant": 0.0}
        for i, t in enumerate(steps, 1)
    ]
    data["endpoint_history"] = [
        {"time": t, "concentration": [[v, v] for v in case["initial"]["values"]]}
        for t in [0.0, *steps]
    ]
    data["solver_reported"] = {
        k: None for k in ("inventory", "boundary_cumulative", "reaction_cumulative", "balance")
    }
    data["availability"] = {
        "endpoint_history": "Initial and every fixed step.",
        "water": "Independent homogeneous constant inputs.",
        "solver_reported": "Accounting and iterations unavailable in synthetic fixture.",
    }
    data["warnings"] = []
    return data


class LifecycleTests(unittest.TestCase):
    def setUp(self):
        scratch = lifecycle.SOURCE_ROOT / ".development/13.3.6"
        scratch.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(self.temp.cleanup)
        self.checkout = Path(self.temp.name)
        patcher = patch.object(lifecycle, "ROOT", self.checkout)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.root = self.checkout / ".development/13.3.1/runs"
        self.runner = lifecycle.Lifecycle(BOOK_PATH, self.root)

    def activation(self, rows=(RUST,), kind="benchmark", phase="benchmark"):
        return {
            "cases_sha256": lifecycle.sha256(BOOK_PATH),
            "phase": phase,
            "launch_ids": [r["id"] for r in rows],
            "kind": kind,
            "authorization": "mock orchestrator activation; no real execution",
        }

    def mock_build(self, task_root, book, sources):
        build = task_root / "rust-builds/synthetic"
        build.mkdir(parents=True, exist_ok=True)
        executable = build / "synthetic-harness"
        if not executable.exists():
            executable.write_bytes(b"synthetic executable; never launched")
            executable.chmod(0o755)
        return build, {
            "status": "complete",
            "executable": executable.name,
            "identity": {"sources": sources, "versions": {"rustc": "synthetic rustc"}},
        }

    def mock_run(self, command, cwd, storage_root, timeout, cap, env, stdout, stderr, global_cap):
        self.assertEqual(
            command,
            [
                str(cwd / "build/solute-verification-harness"),
                "--exact",
                "requested_study",
                "--nocapture",
                "--test-threads=1",
            ],
        )
        self.assertEqual(timeout, 60)
        self.assertEqual(cap, 2 * RUST["estimated_output_bytes"])
        self.assertEqual(env[lifecycle.ENV_KEYS[0]], str(cwd / "request.txt"))
        self.assertEqual(env[lifecycle.ENV_KEYS[1]], lifecycle.sha256(cwd / "request.txt"))
        self.assertEqual(env[lifecycle.ENV_KEYS[2]], str(self.runner.task_root))
        self.assertEqual(env[lifecycle.ENV_KEYS[3]], str(cwd / "observations.json"))
        self.assertEqual(self.events()[-1]["event"], "start")
        stdout.write_text("")
        stderr.write_text("")
        (cwd / "observations.json").write_text(json.dumps(observations(cwd)))
        return 0

    def events(self):
        return lifecycle.fortran.ledger_records(self.runner.task_root / "launch-ledger.jsonl")

    def execute(self, run=None, build=None):
        with patch.object(
            lifecycle, "build_rust", side_effect=build or self.mock_build
        ), patch.object(lifecycle, "run_bounded", side_effect=run or self.mock_run):
            return self.runner.execute([RUST["id"]], self.activation())[0]

    def test_success_passes_real_metrics_and_readonly_verification(self):
        run = self.execute()
        manifest = lifecycle.metrics.json_load(run / "manifest.json")
        self.assertEqual(manifest["status"], "complete")
        self.assertEqual([r["event"] for r in self.events()], ["start", "exit", "complete"])
        self.assertEqual(manifest["record_inventory"]["step_count"], 128)
        self.assertEqual(len(manifest["record_inventory"]["profile_times"]), 33)
        data = lifecycle.metrics.load_observations(
            run / "observations.json", run / "manifest.json", BOOK_PATH
        )
        self.assertEqual(data["profiles"][-1]["concentration"][1], [0.25] * 9)
        with patch.object(
            lifecycle.subprocess, "Popen", side_effect=AssertionError("spawn")
        ), patch.object(lifecycle.subprocess, "run", side_effect=AssertionError("build")):
            self.assertEqual(self.runner.verify(run, require_complete=True), manifest)
        remaining_row = next(
            r for r in BOOK["launches"] if r["solver"] == "rust" and r["role"] == "bridge"
        )
        self.assertEqual(lifecycle.check_budget(BOOK, remaining_row)["remaining"], 76)

    def test_dryrun_is_unique_and_launch_free(self):
        with patch.object(
            lifecycle.subprocess, "Popen", side_effect=AssertionError("spawn")
        ), patch.object(lifecycle.subprocess, "run", side_effect=AssertionError("build")):
            one = self.runner.dry_run([RUST["id"]])[0]
            two = self.runner.dry_run([RUST["id"]])[0]
            self.assertNotEqual(one, two)
            self.assertEqual(self.runner.verify(one)["status"], "planned")
            with self.assertRaisesRegex(ValueError, "complete run required"):
                self.runner.verify(one, require_complete=True)
        self.assertEqual(self.events(), [])
        with patch.object(lifecycle.uuid, "uuid4", return_value=Mock(hex="a" * 32)):
            self.runner.dry_run([RUST["id"]])
            with self.assertRaises(FileExistsError):
                self.runner.dry_run([RUST["id"]])

    def test_solver_failures_consume_start_without_retry(self):
        for mode in ("spawn", "nonzero", "timeout", "storage"):
            with self.subTest(mode=mode):
                # Each failure has an isolated simulated checkout/phase ledger.
                with tempfile.TemporaryDirectory(dir=self.checkout) as sub, patch.object(
                    lifecycle, "ROOT", Path(sub)
                ):
                    self.runner = lifecycle.Lifecycle(
                        BOOK_PATH, Path(sub) / ".development/13.3.1/runs"
                    )

                    def fail(*args):
                        args[6].write_text("")
                        args[7].write_text("")
                        if mode == "spawn":
                            raise OSError("spawn denied")
                        if mode == "timeout":
                            raise subprocess.TimeoutExpired(args[0], 60)
                        if mode == "storage":
                            raise ValueError("local storage cap exceeded")
                        return 7

                    run = self.execute(run=fail)
                    manifest = lifecycle.metrics.json_load(run / "manifest.json")
                    self.assertEqual(
                        manifest["status"], "timeout" if mode == "timeout" else "failed"
                    )
                    self.assertEqual(sum(r["event"] == "start" for r in self.events()), 1)
                    with patch.object(lifecycle, "build_rust") as build:
                        with self.assertRaisesRegex(ValueError, "consumed"):
                            self.runner.execute([RUST["id"]], self.activation())
                        build.assert_not_called()

    def test_build_failure_and_oversized_executable_do_not_consume_start(self):
        def large(task_root, book, sources):
            build, receipt = self.mock_build(task_root, book, sources)
            (build / receipt["executable"]).write_bytes(b"x" * RUST["estimated_output_bytes"])
            return build, receipt

        for failure in (lambda *args: (_ for _ in ()).throw(ValueError("compile failed")), large):
            run = self.execute(build=failure)
            self.assertEqual(lifecycle.metrics.json_load(run / "manifest.json")["status"], "failed")
        self.assertFalse(any(r["event"] == "start" for r in self.events()))

    def test_independent_observation_corruptions_fail_actual_metrics(self):
        mutations = {
            "species": lambda d: d["species"].reverse(),
            "profile": lambda d: d["profiles"].pop(),
            "step": lambda d: d["steps"].pop(),
            "endpoints": lambda d: d["endpoint_history"].pop(),
            "water": lambda d: d["profiles"][-1]["theta"].__setitem__(0, 0.5),
            "concentration_shape": lambda d: d["profiles"][-1]["concentration"].pop(),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory(
                dir=self.checkout
            ) as sub, patch.object(lifecycle, "ROOT", Path(sub)):
                self.runner = lifecycle.Lifecycle(BOOK_PATH, Path(sub) / ".development/13.3.1/runs")

                def malformed(*args):
                    self.mock_run(*args)
                    path = args[1] / "observations.json"
                    data = json.loads(path.read_text())
                    mutate(data)
                    path.write_text(json.dumps(data))
                    return 0

                run = self.execute(run=malformed)
                self.assertEqual(
                    lifecycle.metrics.json_load(run / "manifest.json")["status"], "failed"
                )
                self.assertFalse(any(r["event"] == "complete" for r in self.events()))

    def test_stale_preflight_input_source_executable_blocks_start(self):
        original = self.runner._preflight
        for target in ("request.txt", "build/solute-verification-harness", "source"):

            def stale(row, run_dir, manifest, snapshot):
                if target == "source":
                    manifest["provenance"]["source_sha256"]["Cargo.toml"] = "0" * 64
                else:
                    (run_dir / target).write_bytes(b"tampered")
                original(row, run_dir, manifest, snapshot)

            with patch.object(self.runner, "_preflight", side_effect=stale):
                run = self.execute()
            self.assertEqual(lifecycle.metrics.json_load(run / "manifest.json")["status"], "failed")
        self.assertFalse(any(r["event"] == "start" for r in self.events()))

    def test_manifest_and_executable_tampering_fail_verification(self):
        run = self.execute()
        path = run / "manifest.json"
        original = path.read_bytes()
        for field, value in (
            ("request_sha256", "0" * 64),
            ("command", ["wrong"]),
            ("cwd", "/wrong"),
        ):
            data = json.loads(original)
            data[field] = value
            path.write_text(json.dumps(data))
            with self.assertRaises(ValueError):
                self.runner.verify(run)
        path.write_bytes(original)
        data = json.loads(original)
        data["provenance"]["adapter_sha256"].pop("tools/solute_benchmark_lifecycle.py")
        path.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, "adapter identity"):
            self.runner.verify(run)
        path.write_bytes(original)
        (run / "build/solute-verification-harness").write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "inventory"):
            self.runner.verify(run)

    def test_activation_selection_and_future_phase_confinement(self):
        for value in (
            None,
            {},
            dict(self.activation(), authorization=""),
            dict(self.activation(), cases_sha256="0" * 64),
            dict(self.activation(), kind="bridge"),
        ):
            with self.assertRaises(ValueError):
                self.runner.execute([RUST["id"]], value)
        bridges = [r for r in BOOK["launches"] if r["role"] == "bridge"]
        lifecycle.validate_activation(
            BOOK, BOOK_PATH, "benchmark", bridges, self.activation(bridges, "bridge")
        )
        with self.assertRaisesRegex(ValueError, "separate activation"):
            lifecycle.validate_activation(
                BOOK, BOOK_PATH, "benchmark", [*bridges, RUST], self.activation([*bridges, RUST])
            )
        for task, phase in (("13.4.1", "convergence"), ("13.6.1", "holdout")):
            runner = lifecycle.Lifecycle(BOOK_PATH, self.checkout / f".development/{task}/runs")
            row = next(r for r in BOOK["launches"] if r["phase"] == phase)
            lifecycle.validate_activation(
                BOOK, BOOK_PATH, phase, [row], self.activation([row], phase, phase)
            )
            with self.assertRaises(ValueError):
                runner.execute([RUST["id"]], self.activation())
        self.assertFalse(self.root.exists())

    def test_shared_fortran_rust_budget_and_ledger_corruption(self):
        self.runner.task_root.mkdir(parents=True)
        manifest = {"run_id": "prior-fortran", "launch_id": FORTRAN["id"]}
        lifecycle.fortran.append_event(
            self.runner.task_root,
            "start",
            manifest,
            {
                "timeout_seconds": 60,
                "precision": FORTRAN["precision"],
                "estimated_output_bytes": FORTRAN["estimated_output_bytes"],
            },
        )
        lifecycle.fortran.append_event(self.runner.task_root, "exit", manifest, {"exit_code": 1})
        lifecycle.fortran.append_event(
            self.runner.task_root,
            "validation_failed",
            manifest,
            {"stage": "solver", "error": "synthetic failed attempt"},
        )
        self.assertEqual(lifecycle.check_budget(BOOK, RUST)["remaining"], 76)
        self.execute()
        other = next(r for r in BOOK["launches"] if r["role"] == "bridge")
        self.assertEqual(lifecycle.check_budget(BOOK, other)["remaining"], 75)
        ledger = self.runner.task_root / "launch-ledger.jsonl"
        original = ledger.read_bytes()
        ledger.write_bytes(original + original.splitlines()[0] + b"\n")
        with self.assertRaisesRegex(ValueError, "duplicate|repeated"):
            lifecycle.check_budget(BOOK, other)
        ledger.write_bytes(original[:-1])
        with self.assertRaisesRegex(ValueError, "truncated"):
            lifecycle.check_budget(BOOK, other)
        ledger.write_text("null\n")
        with self.assertRaisesRegex(ValueError, "malformed"):
            lifecycle.check_budget(BOOK, other)
        wrong = json.loads(original.splitlines()[0])
        wrong["launch_id"] = next(r["id"] for r in BOOK["launches"] if r["phase"] == "holdout")
        ledger.write_text(json.dumps(wrong) + "\n")
        with self.assertRaisesRegex(ValueError, "outside phase"):
            lifecycle.check_budget(BOOK, other)

    def test_actual_global_storage_blocks_before_build(self):
        with patch.object(
            lifecycle, "total_storage", return_value=BOOK["budgets"]["storage_bytes_cap"]
        ), patch.object(lifecycle, "build_rust") as build:
            with self.assertRaisesRegex(ValueError, "actual study storage"):
                self.runner.execute([RUST["id"]], self.activation())
            build.assert_not_called()
        self.assertEqual(self.events(), [])

    def test_fortran_dispatch_reuses_adapter_and_shared_ledger(self):
        def prepare(book, cases_path, case, row, root, run_id):
            run = root / run_id
            run.mkdir()
            manifest = {
                "run_id": run_id,
                "launch_id": row["id"],
                "status": "planned",
                "provenance": {"input_sha256": {}, "adapter_sha256": {}},
            }
            return run, manifest, {}

        def execute(book, case, row, run, manifest, snapshot, task_root):
            self.assertEqual(task_root, self.runner.task_root)
            self.assertEqual(lifecycle.check_budget(book, row)["consumed"], 0)
            lifecycle.fortran.append_event(
                task_root,
                "start",
                manifest,
                {
                    "timeout_seconds": 60,
                    "precision": row["precision"],
                    "estimated_output_bytes": row["estimated_output_bytes"],
                },
            )
            lifecycle.fortran.append_event(task_root, "exit", manifest, {"exit_code": 0})
            manifest["status"] = "raw_complete"
            lifecycle.write_json(run / "manifest.json", manifest)
            lifecycle.fortran.append_event(
                task_root,
                "complete",
                manifest,
                {
                    "status": "raw_complete",
                    "manifest_sha256": lifecycle.sha256(run / "manifest.json"),
                },
            )
            return manifest

        with patch.object(
            lifecycle.fortran, "prepare_run", side_effect=prepare
        ) as prepared, patch.object(
            lifecycle.fortran, "execute_run", side_effect=execute
        ) as executed, patch.object(
            lifecycle.fortran, "verify_run"
        ) as verified, patch.object(
            self.runner, "_normalize_fortran"
        ) as normalized, patch.object(
            lifecycle.subprocess, "Popen", side_effect=AssertionError("spawn")
        ):
            run = self.runner.execute([FORTRAN["id"]], self.activation([FORTRAN]))[0]
            for mock in (prepared, executed, verified, normalized):
                mock.assert_called_once()
            self.assertEqual(
                lifecycle.metrics.json_load(run / "manifest.json")["status"], "raw_complete"
            )
        self.assertEqual(lifecycle.check_budget(BOOK, RUST)["remaining"], 76)

    def test_fortran_normalization_preflight_and_postcheck_caps(self):
        run = self.checkout / "normalization"
        run.mkdir()
        (run / "manifest.json").write_text("{}")
        cap = 2 * FORTRAN["estimated_output_bytes"]
        with patch.object(lifecycle.metrics, "_context", return_value={}), patch.object(
            lifecycle.metrics, "_parse_fortran", return_value=({"fixture": 1}, {})
        ), patch.object(lifecycle.metrics, "normalize_fortran") as normalize, patch.object(
            lifecycle.metrics, "load_observations", return_value={}
        ), patch.object(
            lifecycle, "total_storage", return_value=0
        ):
            with patch.object(lifecycle.fortran, "directory_bytes", return_value=cap - 1):
                with self.assertRaisesRegex(ValueError, "normalization exceeds row"):
                    self.runner._normalize_fortran(run, FORTRAN)
                normalize.assert_not_called()
            with patch.object(lifecycle.fortran, "directory_bytes", side_effect=[0, cap + 1]):
                with self.assertRaisesRegex(ValueError, "normalized row storage"):
                    self.runner._normalize_fortran(run, FORTRAN)
                normalize.assert_called_once()

    def test_path_escape_symlink_and_unknown_rows(self):
        for path in (
            self.checkout / "outside",
            self.checkout / ".development/13.3.6/run",
            self.root / ".." / "escape",
        ):
            with self.assertRaises(ValueError):
                lifecycle.Lifecycle(BOOK_PATH, path)
        self.runner.task_root.mkdir(parents=True)
        self.root.symlink_to(self.checkout, target_is_directory=True)
        with self.assertRaises(ValueError):
            lifecycle.Lifecycle(BOOK_PATH, self.root)
        for rows in ([RUST["id"], RUST["id"]], ["unknown"]):
            with self.assertRaises(ValueError):
                lifecycle.selected_rows(BOOK, "benchmark", rows)


class ProcessTests(unittest.TestCase):
    def test_process_group_cleanup_even_when_direct_child_exits(self):
        scratch = lifecycle.SOURCE_ROOT / ".development/13.3.6"
        scratch.mkdir(parents=True, exist_ok=True)
        for mode in ("success", "timeout", "storage"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory(dir=scratch) as sub:
                root = Path(sub)
                process = Mock(pid=999999)
                process.wait.return_value = 0
                process.poll.return_value = 0 if mode == "success" else None
                times = [0, 61] if mode == "timeout" else [0, 1]
                with patch.object(
                    lifecycle.subprocess, "Popen", return_value=process
                ) as popen, patch.object(lifecycle.os, "killpg") as kill, patch.object(
                    lifecycle.time, "monotonic", side_effect=times
                ), patch.object(
                    lifecycle, "total_storage", return_value=0
                ), patch.object(
                    lifecycle.fortran,
                    "directory_bytes",
                    return_value=101 if mode == "storage" else 0,
                ):

                    def call():
                        return lifecycle.run_bounded(
                            ["mock"], root, root, 60, 100, {}, root / "out", root / "err", 1000
                        )

                    if mode == "success":
                        self.assertEqual(call(), 0)
                    else:
                        with self.assertRaises(
                            subprocess.TimeoutExpired if mode == "timeout" else ValueError
                        ):
                            call()
                    kill.assert_called_once_with(999999, lifecycle.signal.SIGKILL)
                    self.assertTrue(popen.call_args.kwargs["start_new_session"])
                    process.wait.assert_called()


class BuildTests(unittest.TestCase):
    def setUp(self):
        scratch = lifecycle.SOURCE_ROOT / ".development/13.3.6"
        scratch.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.tool = self.root / "mock-tool"
        self.tool.write_bytes(b"fake compiler")
        self.tool.chmod(0o755)
        self.sources = lifecycle.source_hashes(BOOK)

    def compile(self, command, cwd, storage_root, timeout, cap, env, stdout, stderr, global_cap):
        self.assertEqual(command, lifecycle.cargo_command(storage_root / "target"))
        self.assertEqual(timeout, 60)
        self.assertEqual(
            lifecycle.PROFILE,
            {"debug": 0, "opt-level": 0, "debug-assertions": True, "overflow-checks": True},
        )
        exe = storage_root / "target/harness"
        exe.parent.mkdir()
        exe.write_bytes(b"fake built executable")
        exe.chmod(0o755)
        stdout.write_text(
            json.dumps(
                {
                    "reason": "compiler-artifact",
                    "target": {"name": "solute_verification_harness", "kind": ["test"]},
                    "executable": str(exe),
                }
            )
            + "\n"
        )
        stderr.write_text("")
        return 0

    def test_compile_reuse_and_stale_executable(self):
        with patch.object(lifecycle, "controlled_environment", return_value={}), patch.object(
            lifecycle.shutil, "which", return_value=str(self.tool)
        ), patch.object(
            lifecycle.subprocess, "run", return_value=Mock(stdout="mock compiler v1")
        ), patch.object(
            lifecycle, "total_storage", return_value=0
        ), patch.object(
            lifecycle, "run_bounded", side_effect=self.compile
        ) as compile:
            build, receipt = lifecycle.build_rust(self.root, BOOK, self.sources)
            self.assertEqual(lifecycle.build_rust(self.root, BOOK, self.sources), (build, receipt))
            self.assertEqual(compile.call_count, 1)
            self.assertFalse(list(self.root.rglob("launch-ledger.jsonl")))
            (build / receipt["executable"]).write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "artifact changed"):
                lifecycle.build_rust(self.root, BOOK, self.sources)
            self.assertEqual(compile.call_count, 1)

    def test_failed_or_ambiguous_build_never_automatically_retries(self):
        def ambiguous(*args):
            self.compile(*args)
            args[6].write_text(args[6].read_text() * 2)
            return 0

        with patch.object(lifecycle, "controlled_environment", return_value={}), patch.object(
            lifecycle.shutil, "which", return_value=str(self.tool)
        ), patch.object(
            lifecycle.subprocess, "run", return_value=Mock(stdout="mock compiler v1")
        ), patch.object(
            lifecycle, "total_storage", return_value=0
        ), patch.object(
            lifecycle, "run_bounded", side_effect=ambiguous
        ) as compile:
            with self.assertRaisesRegex(ValueError, "ambiguous"):
                lifecycle.build_rust(self.root, BOOK, self.sources)
            with self.assertRaisesRegex(ValueError, "failed"):
                lifecycle.build_rust(self.root, BOOK, self.sources)
            self.assertEqual(compile.call_count, 1)

    def test_unreviewed_build_override_rejected(self):
        with patch.dict(os.environ, {"RUSTFLAGS": "-Copt-level=3"}):
            with self.assertRaisesRegex(ValueError, "override"):
                lifecycle.controlled_environment()


class V2LifecycleTests(unittest.TestCase):
    V2_PATH = lifecycle.SOURCE_ROOT / "reference/solute_verification/CASES_V2.json"

    def setUp(self):
        scratch = lifecycle.SOURCE_ROOT / ".development/22.3.2"
        scratch.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(self.temp.cleanup)
        self.checkout = Path(self.temp.name)
        patcher = patch.object(lifecycle, "ROOT", self.checkout)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.book = lifecycle.contract.validate(lifecycle.contract.load_cases(self.V2_PATH))

    def test_v2_phase_roots_accepted(self):
        for phase in ("benchmark", "convergence", "holdout"):
            runner = lifecycle.Lifecycle(
                self.V2_PATH, self.checkout / f".development/22-{phase}/runs"
            )
            self.assertEqual(runner.phase, phase)
            self.assertFalse(lifecycle.has_caps(runner.book))

    def test_v2_has_no_launch_or_storage_ceiling(self):
        row = next(r for r in self.book["launches"] if r["phase"] == "benchmark")
        info = lifecycle.check_budget(self.book, row)
        self.assertIsNone(info["remaining"])
        self.assertEqual(lifecycle.storage_cap(self.book), float("inf"))
        # Estimates are still recorded for v2 rows.
        self.assertGreater(row["estimated_output_bytes"], 0)
        with patch.object(lifecycle, "total_storage", return_value=10**15):
            lifecycle.check_budget(self.book, row)

    def test_v2_dry_run_manifest_records_backend(self):
        for phase, case_id in (("benchmark", "reaction_zero"), ("convergence", "diffusion_cosine")):
            row = next(
                r for r in self.book["launches"] if r["phase"] == phase and r["case_id"] == case_id
            )
            runner = lifecycle.Lifecycle(
                self.V2_PATH, self.checkout / f".development/22-{phase}/runs"
            )
            (path,) = runner.dry_run([row["id"]])
            manifest = json.loads((path / "manifest.json").read_text())
            self.assertEqual(manifest["backend"], "fortran-compatible")
            self.assertIn(b"backend=fortran-compatible\n", (path / "request.txt").read_bytes())

    def test_v1_budget_caps_still_enforced(self):
        book = copy.deepcopy(BOOK)
        self.assertTrue(lifecycle.has_caps(book))
        book["budgets"]["launch_cap"] = 0
        with self.assertRaisesRegex(ValueError, "launch cap"):
            lifecycle.check_budget(book, RUST)
        book = copy.deepcopy(BOOK)
        book["budgets"]["storage_bytes_cap"] = 1
        with self.assertRaisesRegex(ValueError, "storage reservation"):
            lifecycle.check_budget(book, RUST)

    def test_v1_manifest_has_no_backend(self):
        runner = lifecycle.Lifecycle(BOOK_PATH, self.checkout / ".development/13.3.1/runs")
        (path,) = runner.dry_run([RUST["id"]])
        self.assertNotIn("backend", json.loads((path / "manifest.json").read_text()))


if __name__ == "__main__":
    unittest.main()

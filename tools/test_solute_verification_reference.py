"""Mock-only tests for deck mapping, strict inventories, and launch lifecycle."""

import copy
import math
import os
import shutil
import stat
import subprocess
import tempfile
import time
import unittest
from pathlib import Path
from unittest.mock import patch

from tools import run_solute_verification_reference as adapter

REAL_ROOT = adapter.ROOT


def scientific(value, decimals=4):
    if value == 0:
        return f"{0:.{decimals}f}E+00"
    exponent = int(math.floor(math.log10(abs(value)))) + 1
    return f"{value/10**exponent:.{decimals}f}E{exponent:+03d}"


def fake_outputs(run_dir, case, row):
    """Author a small native-layout transcript; values are not solver evidence."""
    nodes, species = row["nodes"], len(case["species"])
    steps, times = adapter.schedule(case, row)
    coordinates = adapter.grid(case, nodes)
    initial = adapter.initial_arrays(case, nodes)
    header = [" ******* Program HYDRUS", " ******* Authored mock records"]
    run, water, obs = header + ["TLevel Time dt ItrW ItrC"], header + ["Time vTop"], header[:]
    if row["role"] != "bridge":
        obs += [f"Node( 1) Node( {nodes})", "time h theta Temp Conc"]
    solutes = [
        [
            "All solute fluxes and cumulative solute fluxes are positive into the region",
            "time cvTop cvBot",
        ]
        for _ in range(species)
    ]
    for i, t in enumerate(steps, 1):
        run.append(f'{i} {scientific(t, 7)} {scientific(row["dt"], 5)} 1 1 {i} 1 1 T 0.000 0.000')
        fields = [0.0] * 22
        fields[0], fields[3], fields[5] = t, -case["q"], -case["q"]
        fields[16], fields[19] = case["theta"] * case["length"], i
        water.append(
            f"{t:.4f} "
            + " ".join(scientific(v, 5) for v in fields[1:19])
            + f" {i} "
            + " ".join(scientific(v, 5) for v in fields[20:])
        )
        if row["role"] != "bridge":
            fields = [f"{t:.4f}"]
            for node in (0, nodes - 1):
                fields += ["0.00", f'{case["theta"]:.4f}', "20.000"]
                fields += [scientific(initial[j][node]) for j in range(species)]
            obs.append(" ".join(fields))
        for j in range(species):
            fields = [f"{t:.4f}"] + [scientific(0, 5)] * 12 + [str(i)] + [scientific(0, 5)] * 3
            solutes[j].append(" ".join(fields))
    profiles, balance = header[:], header[:]
    for t in times:
        profiles += [f"Time: {t:.4f}", "Node Depth Head Moisture Conc(1..NS) Sorb(1...NS)"]
        balance += [f"Time [T] {t:.4f}"]
        for j in range(species):
            balance += [
                f"ConcVol [M/L2] {j+1} 0.00000E+00 0.00000E+00",
                f"cMean [M/L3] {j+1} 0.00000E+00 0.00000E+00",
            ]
        if t:
            balance += [f"CncBalT [M] {j+1} 0.00000E+00" for j in range(species)]
        for i, s in enumerate(coordinates):
            fields = [
                str(i + 1),
                f"{-s:.4f}",
                "0.000",
                f'{case["theta"]:.4f}',
                scientific(case["hydraulics"]["ks"]),
                scientific(0),
                scientific(-case["q"]),
                scientific(0),
                "-1",
                scientific(-case["q"] / case["hydraulics"]["ks"], 3),
                "20.00",
            ]
            fields += [scientific(initial[j][i]) for j in range(species)]
            fields += [scientific(0)] * species
            profiles.append(" ".join(fields))
    hydraulic = header + ["n depth THr THs hs Ks Ks/KsTop Beta Ah AK ATh"]
    echo = header + ["Nodal point information", "node x hOld MatN LayN Beta Ah AK ATh Temp Conc"]
    for i, s in enumerate(coordinates):
        hydraulic.append(
            f"{i+1} {s:.2f} .080 .430 0.0 "
            + scientific(case["hydraulics"]["ks"], 3)
            + " 1.000 0.000 1.000 1.000 1.000"
        )
        echo.append(
            f"{i+1} {-s:.3f} 0.000 1 1 0.000 1.000 1.000 1.000 20.00 "
            + " ".join(scientific(initial[j][i]) for j in range(species))
        )
    echo += [
        "end",
        "Solute transport information",
        "Mat. Bulk.D. DispL Fraction Immobile WC",
        f"1 {case['rho']:.4f} {case['chemistry']['dispersivity']:.4f} 1.0000 0.0000",
    ]
    for j in range(species):
        echo += [
            f"Dif.w. Dif.g. ({j+1}.solute)",
            scientific(case["chemistry"]["molecular_diffusion"]) + " " + scientific(0),
            "Mat. KS Nu Beta Henry SinkL1 SinkS1",
            "1 " + " ".join(scientific(v) for v in adapter.chemical_coefficients(case, j)),
        ]
    echo += [
        "kTopCh cTop(1...NS)",
        "-1 " + " ".join(scientific(v, 3) for v in case["boundary"]["inlet_concentration"]),
        "kBotCh cBot(1...NS)",
        "-1 " + " ".join(scientific(0, 3) for _ in range(species)),
        f'tPulse = {case["time_end"]:.3f}',
    ]
    outputs = {
        "RUN_INF.OUT": run + ["end"],
        "T_LEVEL.OUT": water + ["end"],
        "NOD_INF.OUT": profiles + ["end"],
        "OBS_NODE.OUT": obs + ["end"],
        "PROFILE.OUT": hydraulic + ["end"],
        "I_CHECK.OUT": echo,
        "BALANCE.OUT": balance + ["Calculation time [sec] 0.01"],
    }
    outputs.update({f"solute{j+1}.out": records + ["end"] for j, records in enumerate(solutes)})
    for name, lines in outputs.items():
        (run_dir / ("solute-input\\" + name)).write_text("\n".join(lines) + "\n", encoding="ascii")


class ReferenceTests(unittest.TestCase):
    def setUp(self):
        scratch = REAL_ROOT / ".development/13.2.1/tests"
        scratch.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "repo"
        self.root.mkdir()
        for name in (
            "src_reference",
            "reference/fortran_reference",
            "reference/solute_verification",
        ):
            shutil.copytree(REAL_ROOT / name, self.root / name)
        for name in (
            "planning/H1D_Src.zip",
            "planning/master/SOURCE_MANIFEST.json",
            "tools/run_solute_verification_reference.py",
            "tools/check_solute_verification_spec.py",
            "tools/build_fortran_reference.sh",
        ):
            destination = self.root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(REAL_ROOT / name, destination)
        for name in adapter.contract.load_cases(REAL_ROOT / adapter.SOURCE_MANIFEST):
            destination = self.root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            if not destination.exists():
                shutil.copy2(REAL_ROOT / name, destination)
        for target in (
            patch.object(adapter, "ROOT", self.root),
            patch.object(adapter.contract, "ROOT", self.root),
            patch.object(
                adapter, "FIXTURE", self.root / "reference/fortran_reference/solute/example3"
            ),
        ):
            target.start()
            self.addCleanup(target.stop)
        self.cases_path = self.root / "reference/solute_verification/CASES.json"
        self.book = adapter.contract.validate(adapter.contract.load_cases(self.cases_path))
        self.case = next(c for c in self.book["cases"] if c["id"] == "reaction_distinct")
        self.row = next(
            r
            for r in self.book["launches"]
            if r["case_id"] == self.case["id"] and r["solver"] == "fortran"
        )
        self.task_root = self.root / ".development/13.2.1"
        self.output = self.task_root / "runs"
        self.output.mkdir(parents=True)

    def prepare(self, run_id="test-run"):
        return adapter.prepare_run(
            self.book, self.cases_path, self.case, self.row, self.output, run_id
        )

    def mock_build(self, run_dir, precision, sources):
        build = run_dir / "build"
        build.mkdir()
        exe = build / "hydrus-reference"
        exe.write_bytes(b"not a solver executable\n")
        exe.chmod(0o755)
        shim = build / "gnu_compatibility.FOR"
        shim.write_bytes(b"mock shim\n")
        return {
            "compiler": "mock GNU Fortran",
            "executable_sha256": adapter.sha256(exe),
            "shim_sha256": adapter.sha256(shim),
            "staged_sha256": {},
        }

    def mock_success(self, command, run_dir, timeout, cap):
        fake_outputs(run_dir, self.case, self.row)
        (run_dir / "stdout.txt").write_text("")
        (run_dir / "stderr.txt").write_text("Warning: IEEE_UNDERFLOW_FLAG IEEE_DENORMAL\n")
        return 0

    def test_deterministic_requests_and_native_orientation(self):
        case = next(c for c in self.book["cases"] if c["id"] == "diffusion_cosine")
        row = next(
            r
            for r in self.book["launches"]
            if r["case_id"] == case["id"] and r["solver"] == "fortran"
        )
        first = adapter.generate_decks(case, row)
        self.assertEqual(first, adapter.generate_decks(case, row))
        rows = first["PROFILE.DAT"].decode().splitlines()[4 : 4 + row["nodes"]]
        self.assertEqual(float(rows[0].split()[1]), 0)
        self.assertEqual(float(rows[-1].split()[1]), -case["length"])
        self.assertEqual(float(rows[0].split()[10]), 1.25)
        self.assertEqual(float(rows[-1].split()[10]), 0.75)
        request = adapter.request_bytes(case, row, "run-1", "a" * 64)
        self.assertEqual(request, adapter.request_bytes(case, row, "run-1", "a" * 64))
        values = dict(line.split("=") for line in request.decode().splitlines())
        self.assertEqual(values["initial_0"].split(",")[0], "1.25")
        self.assertEqual(values["initial_0"].split(",")[-1], "0.75")
        self.assertTrue(first["PROFILE.DAT"].endswith(f"2\n1 {row['nodes']}\n".encode()))

    def test_scalar_initial_formula_has_both_boundaries(self):
        case = next(c for c in self.book["cases"] if c["id"] == "scalar_robin")
        initial = adapter.initial_arrays(case, 1001)[0]
        dx = case["length"] / 1000
        derivative_in = (-3 * initial[0] + 4 * initial[1] - initial[2]) / (2 * dx)
        derivative_out = (3 * initial[-1] - 4 * initial[-2] + initial[-3]) / (2 * dx)
        self.assertAlmostEqual(
            case["q"] * initial[0] - case["theta"] * case["dispersion"] * derivative_in,
            case["q"],
            places=5,
        )
        self.assertAlmostEqual(derivative_out, 0, places=5)

    def test_chemical_switches_and_terminal_loss(self):
        deck = adapter.generate_decks(self.case, self.row)["SELECTOR.IN"].decode()
        self.assertIn("1 1 0\n", deck)
        self.assertIn("0.5 f f f", deck)
        self.assertIn("0 f f f f f", deck)
        self.assertEqual(adapter.chemical_coefficients(self.case, 0)[4:9], [0, 0, 0, 0.2, 0.2])
        self.assertEqual(adapter.chemical_coefficients(self.case, 2)[4:9], [0.1, 0.1, 0, 0, 0])
        self.assertIn("0.03125 0.03125 0.03125 1 1 1 20 32", deck)
        self.assertIn("f f f f f f", deck)  # lFluxes stays false.

    def test_bridge_decks_are_exact_and_keep_print_schedule(self):
        case = next(c for c in self.book["cases"] if c["id"] == "example3")
        row = self.book["launches"][0]
        for name, data in adapter.generate_decks(case, row).items():
            self.assertEqual(data, (adapter.FIXTURE / name).read_bytes())
        self.assertEqual(adapter.schedule(case, row)[1], [0, 50, 100, 200])

    def test_capacity_and_schedule_rejected(self):
        for nodes in (1, 1002):
            with self.assertRaises(ValueError):
                adapter.initial_arrays(self.case, nodes)
        row = copy.deepcopy(self.row)
        row["dt"] = 0.03
        with self.assertRaisesRegex(ValueError, "nonintegral"):
            adapter.schedule(self.case, row)
        row["dt"] = 0
        with self.assertRaises(ValueError):
            adapter.schedule(self.case, row)

    def test_confinement_and_symlinks(self):
        for root in (self.root / ".development/12.2.1/new", self.root / "elsewhere", "/tmp/no"):
            with self.assertRaises(ValueError):
                adapter.output_root(str(root))
        (self.task_root / "link").symlink_to(self.output, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            adapter.output_root(str(self.task_root / "link/child"))
        with self.assertRaises(ValueError):
            adapter.safe_path(self.output, "../escape")
        (self.output / "dangling").symlink_to(self.output / "missing")
        with self.assertRaises(ValueError):
            adapter.safe_path(self.output, "dangling")

    def test_existing_run_and_unsafe_id_refused(self):
        self.prepare()
        with self.assertRaises(FileExistsError):
            self.prepare()
        with self.assertRaises(ValueError):
            self.prepare("../escape")

    def test_source_and_archive_hash_guards(self):
        original = REAL_ROOT / "src_reference/SOLUTE.FOR"
        original_mode, original_hash = original.stat().st_mode, adapter.sha256(original)
        path = self.root / "src_reference/SOLUTE.FOR"
        # Reproduce copy2 preserving the pinned source's read-only permissions.
        path.chmod(path.stat().st_mode & ~0o222)
        self.assertEqual(path.stat().st_mode & 0o222, 0)
        path.chmod(path.stat().st_mode | stat.S_IWUSR)
        path.write_bytes(path.read_bytes() + b"changed")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            self.prepare()
        self.assertEqual(original.stat().st_mode, original_mode)
        self.assertEqual(adapter.sha256(original), original_hash)

    def test_mock_raw_complete_and_read_only_verify(self):
        run_dir, manifest, snapshot = self.prepare()
        with patch.object(adapter, "build_reference", side_effect=self.mock_build), patch.object(
            adapter, "run_solver", side_effect=self.mock_success
        ):
            result = adapter.execute_run(
                self.book, self.case, self.row, run_dir, manifest, snapshot, self.task_root
            )
        self.assertEqual(result["status"], "raw_complete", result["warnings"])
        self.assertIn("Warning: IEEE_UNDERFLOW_FLAG IEEE_DENORMAL", result["warnings"])
        self.assertEqual(set(result), adapter.MANIFEST_KEYS)
        events = adapter.ledger_records(self.task_root / "launch-ledger.jsonl")
        self.assertEqual([e["event"] for e in events], ["start", "exit", "complete"])
        with patch.object(
            adapter.subprocess, "run", side_effect=AssertionError("subprocess")
        ), patch.object(adapter.subprocess, "Popen", side_effect=AssertionError("solver")):
            digest = adapter.sha256(run_dir / "manifest.json")
            adapter.verify_run(run_dir, self.book, self.cases_path)
            self.assertEqual(adapter.sha256(run_dir / "manifest.json"), digest)
            (run_dir / "observations.json").write_text("{}\n")
            (run_dir / "normalization.json").write_text("{}\n")
            adapter.verify_run(run_dir, self.book, self.cases_path)
            self.assertEqual(adapter.sha256(run_dir / "manifest.json"), digest)
            (run_dir / "unexpected.raw").write_text("unrecorded\n")
            with self.assertRaisesRegex(ValueError, "inventory/hash"):
                adapter.verify_run(run_dir, self.book, self.cases_path)
        with self.assertRaisesRegex(ValueError, "consumed"):
            adapter.check_launch_budget(self.book, self.row)

    def test_mock_nonzero_exit_consumes_row(self):
        run_dir, manifest, snapshot = self.prepare()
        with patch.object(adapter, "build_reference", side_effect=self.mock_build), patch.object(
            adapter, "run_solver", return_value=3
        ):
            result = adapter.execute_run(
                self.book, self.case, self.row, run_dir, manifest, snapshot, self.task_root
            )
        self.assertEqual(result["status"], "failed")
        events = adapter.ledger_records(self.task_root / "launch-ledger.jsonl")
        self.assertEqual([e["event"] for e in events], ["start", "exit", "validation_failed"])
        with self.assertRaisesRegex(ValueError, "consumed"):
            adapter.check_launch_budget(self.book, self.row)

    def test_mock_timeout_and_launch_error_consume_row(self):
        for failure, status, event in (
            (subprocess.TimeoutExpired(["mock"], 60), "timeout", "timeout"),
            (OSError("cannot launch"), "failed", "validation_failed"),
        ):
            with self.subTest(status=status):
                ledger = self.task_root / "launch-ledger.jsonl"
                if ledger.exists():
                    ledger.unlink()  # Isolated mock ledger only.
                run_dir, manifest, snapshot = self.prepare(status)
                with patch.object(
                    adapter, "build_reference", side_effect=self.mock_build
                ), patch.object(adapter, "run_solver", side_effect=failure):
                    result = adapter.execute_run(
                        self.book, self.case, self.row, run_dir, manifest, snapshot, self.task_root
                    )
                self.assertEqual(result["status"], status)
                self.assertEqual(
                    [e["event"] for e in adapter.ledger_records(ledger)], ["start", event]
                )
                with self.assertRaisesRegex(ValueError, "consumed"):
                    adapter.check_launch_budget(self.book, self.row)

    def test_stale_missing_truncated_and_malformed_output(self):
        run_dir, _, _ = self.prepare()
        fake_outputs(run_dir, self.case, self.row)
        adapter.validate_outputs(run_dir, self.case, self.row)
        path = run_dir / "solute-input\\RUN_INF.OUT"
        original = path.read_bytes()
        for malformed in (
            original[:-1],
            original.replace(b"\nend\n", b"\nnot-end\n"),
            original.replace(b" T ", b" F ", 1),
            original.replace(b"1 ", b"2 ", 1),
        ):
            path.write_bytes(malformed)
            with self.assertRaises(ValueError):
                adapter.validate_outputs(run_dir, self.case, self.row)
        path.write_bytes(original)
        with self.assertRaisesRegex(ValueError, "stale"):
            adapter.validate_outputs(run_dir, self.case, self.row, time.time_ns() + 10**9)
        path.unlink()
        with self.assertRaisesRegex(ValueError, "missing"):
            adapter.validate_outputs(run_dir, self.case, self.row)

    def test_missing_endpoint_water_and_species_records(self):
        run_dir, _, _ = self.prepare()
        fake_outputs(run_dir, self.case, self.row)
        for name in ("OBS_NODE.OUT", "solute2.out", "T_LEVEL.OUT", "NOD_INF.OUT"):
            path = run_dir / ("solute-input\\" + name)
            original = path.read_text()
            lines = original.splitlines()
            numeric_index = next(
                i
                for i, line in enumerate(lines)
                if line.split() and adapter.NUMBER.fullmatch(line.split()[0])
            )
            del lines[numeric_index]
            path.write_text("\n".join(lines) + "\n")
            with self.assertRaises(ValueError):
                adapter.validate_outputs(run_dir, self.case, self.row)
            path.write_text(original)
        path = run_dir / "solute-input\\OBS_NODE.OUT"
        path.write_text(path.read_text().replace("0.4300", "0.4200", 1))
        with self.assertRaisesRegex(ValueError, "water invariant"):
            adapter.validate_outputs(run_dir, self.case, self.row)

    def test_small_dt_rounding_interval_and_endpoint_coverage(self):
        self.case = next(c for c in self.book["cases"] if c["id"] == "diffusion_cosine")
        self.row = next(
            r
            for r in self.book["launches"]
            if r["case_id"] == self.case["id"] and r["solver"] == "fortran"
        )
        run_dir, _, _ = self.prepare()
        fake_outputs(run_dir, self.case, self.row)
        inventory, _ = adapter.validate_outputs(run_dir, self.case, self.row)
        self.assertEqual(inventory["endpoint_times"][1], self.row["dt"])
        self.assertTrue(adapter.matches("0.0001", 0.0001220703125, 4))
        self.assertFalse(adapter.matches("0.0002", 0.0001220703125, 4))

    def test_default_and_verify_only_never_launch(self):
        args = [
            "--cases",
            str(self.cases_path),
            "--output-root",
            str(self.output),
            "--launch-id",
            self.row["id"],
        ]
        with patch.object(
            adapter.subprocess, "Popen", side_effect=AssertionError("solver")
        ), patch.object(adapter.subprocess, "run", side_effect=AssertionError("build")):
            self.assertEqual(adapter.main(args), 0)
            self.assertEqual(adapter.main(args + ["--verify-only"]), 0)
            self.assertEqual(adapter.main(args + ["--execute"]), 2)
        self.assertFalse((self.task_root / "launch-ledger.jsonl").exists())

    def test_artifact_hash_mismatch_and_escape_rejected(self):
        run_dir, manifest, _ = self.prepare()
        manifest["artifacts"] = adapter.artifact_inventory(run_dir)
        adapter.write_json(run_dir / "manifest.json", manifest)
        (run_dir / "request.txt").write_bytes(b"tampered\n")
        with self.assertRaisesRegex(ValueError, "inventory/hash"):
            adapter.verify_run(run_dir, self.book, self.cases_path)
        (run_dir / "request.txt").unlink()
        (run_dir / "request.txt").symlink_to(self.cases_path)
        with self.assertRaisesRegex(ValueError, "symlink"):
            adapter.verify_run(run_dir, self.book, self.cases_path)

    def test_precision_flags_and_shim_build_recipe(self):
        for precision, extra in (
            ("baseline", []),
            ("promoted-real", ["-fdefault-real-8", "-fdefault-double-8"]),
        ):
            run_dir = self.output / precision
            run_dir.mkdir()

            def fake_subprocess(command, **kwargs):
                if command[:2] == ["gfortran", "--version"]:
                    return subprocess.CompletedProcess(command, 0, stdout="GNU Fortran mock\n")
                if command[0] == "gfortran":
                    exe = Path(command[-1])
                    exe.write_bytes(b"mock executable\n")
                    exe.chmod(0o755)
                return subprocess.CompletedProcess(command, 0, stdout=b"", stderr=b"")

            with patch.object(adapter.subprocess, "run", side_effect=fake_subprocess) as call:
                result = adapter.build_reference(
                    run_dir, precision, self.book["provenance"]["source_sha256"]
                )
            compile_command = call.call_args_list[-1].args[0]
            self.assertEqual(compile_command[1 : 3 + len(extra)], adapter.FLAGS + extra)
            self.assertEqual(
                result["executable_sha256"], adapter.sha256(run_dir / "build/hydrus-reference")
            )
            builder = (self.root / "tools/build_fortran_reference.sh").read_text()
            expected = builder.split("<<'EOF'\n")[1].split("\nEOF\n")[0] + "\n"
            self.assertEqual((run_dir / "build/gnu_compatibility.FOR").read_text(), expected)
            self.assertTrue(os.access(run_dir / "build/hydrus-reference", os.X_OK))

    def test_bridge_raw_terminal_and_null_endpoint_inventory(self):
        self.case = next(c for c in self.book["cases"] if c["id"] == "example3")
        self.row = self.book["launches"][0]
        run_dir, _, _ = self.prepare()
        fake_outputs(run_dir, self.case, self.row)
        inventory, mapping = adapter.validate_outputs(run_dir, self.case, self.row)
        self.assertIsNone(inventory["endpoint_times"])
        self.assertEqual(inventory["profile_times"], [0, 50, 100, 200])
        self.assertEqual(inventory["terminal_records"]["I_CHECK.OUT"], "tPulse = 200.000")
        self.assertIn("legacy_bridge_no_obs", mapping["evidence"])

    def test_changed_input_fails_before_launch(self):
        run_dir, manifest, snapshot = self.prepare()
        (run_dir / "solute-input\\Selector.in").write_text("changed\n")
        with patch.object(adapter, "build_reference", side_effect=self.mock_build), patch.object(
            adapter, "run_solver", side_effect=AssertionError("must not launch")
        ):
            result = adapter.execute_run(
                self.book, self.case, self.row, run_dir, manifest, snapshot, self.task_root
            )
        self.assertEqual(result["status"], "failed")
        events = adapter.ledger_records(self.task_root / "launch-ledger.jsonl")
        self.assertEqual([e["event"] for e in events], ["validation_failed"])

    def test_mock_process_monitor_timeout_and_storage(self):
        from unittest.mock import Mock

        for mode in ("timeout", "storage"):
            run_dir = self.output / mode
            run_dir.mkdir()
            process = Mock()
            process.poll.return_value = None
            process.wait.return_value = 0
            with patch.object(adapter.subprocess, "Popen", return_value=process), patch.object(
                adapter.time, "monotonic", side_effect=[0, 61]
            ):
                exception = subprocess.TimeoutExpired if mode == "timeout" else ValueError
                with self.assertRaises(exception):
                    adapter.run_solver(["mock"], run_dir, 60, 100 if mode == "timeout" else -1)
            process.kill.assert_called_once()
            self.assertTrue((run_dir / "stdout.txt").exists())
            self.assertTrue((run_dir / "stderr.txt").exists())

    def test_matrix_budget_and_failed_attempt_count(self):
        _, manifest, _ = self.prepare()
        book = copy.deepcopy(self.book)
        book["budgets"]["launch_cap"] = 0
        with self.assertRaisesRegex(ValueError, "launch cap"):
            adapter.check_launch_budget(book, self.row)
        book = copy.deepcopy(self.book)
        book["budgets"]["phase_caps"]["benchmark"] = 0
        with self.assertRaisesRegex(ValueError, "phase launch cap"):
            adapter.check_launch_budget(book, self.row)
        book = copy.deepcopy(self.book)
        book["budgets"]["storage_bytes_cap"] = 1
        with self.assertRaisesRegex(ValueError, "storage"):
            adapter.check_launch_budget(book, self.row)
        adapter.append_event(self.task_root, "start", manifest, {})
        adapter.append_event(self.task_root, "timeout", manifest, {})
        with self.assertRaisesRegex(ValueError, "consumed"):
            adapter.check_launch_budget(self.book, self.row)

    def test_truncated_printed_precision_is_rejected(self):
        run_dir, _, _ = self.prepare()
        fake_outputs(run_dir, self.case, self.row)
        path = run_dir / "solute-input\\NOD_INF.OUT"
        path.write_text(path.read_text().replace("0.4300", "0.43", 1))
        with self.assertRaisesRegex(ValueError, "malformed"):
            adapter.validate_outputs(run_dir, self.case, self.row)

    def test_symlink_output_and_chemical_echo_mismatch(self):
        run_dir, _, _ = self.prepare()
        fake_outputs(run_dir, self.case, self.row)
        path = run_dir / "solute-input\\I_CHECK.OUT"
        path.write_text(path.read_text().replace("0.8000", "0.7000", 1))
        with self.assertRaisesRegex(ValueError, "bulk density"):
            adapter.validate_outputs(run_dir, self.case, self.row)
        path.unlink()
        path.symlink_to(self.cases_path)
        with self.assertRaisesRegex(ValueError, "symlink"):
            adapter.validate_outputs(run_dir, self.case, self.row)


if __name__ == "__main__":
    unittest.main()

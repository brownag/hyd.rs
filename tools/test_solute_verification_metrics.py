"""Synthetic, hand-computed parser and accounting checks; never launch a solver."""

import json
import copy
import math
import tempfile
import unittest
from pathlib import Path

from tools import solute_verification_metrics as metrics

BOOK_PATH = metrics.ROOT / "reference/solute_verification/CASES.json"
BOOK = metrics.load_cases(BOOK_PATH)
SCRATCH = metrics.ROOT / ".development/13.2.3"


def fexp(value, digits=4):
    if value == 0:
        return "0." + "0" * digits + "E+00"
    exponent = math.floor(math.log10(abs(value))) + 1
    mantissa = value / 10**exponent
    return f"{mantissa:.{digits}f}E{exponent:+03d}"


def make_request(case, launch, manifest):
    fields = {"schema": "flat-ascii-v1"}
    fields.update(
        {
            key: manifest[key]
            for key in ("run_id", "launch_id", "case_id", "cases_sha256", "case_sha256")
        }
    )
    fields.update(nodes=str(launch["nodes"]), species_count=str(len(case["species"])))
    for key in ("length", "theta", "rho", "q", "dispersion", "time_end"):
        fields[key] = format(case[key], ".17g")
    for key in ("dt", "sampling_interval"):
        fields[key] = format(launch[key], ".17g")
    vectors = {
        "analysis_times": launch["analysis_times"],
        "kd": [s["kd"] for s in case["species"]],
        "rates": [s["rate"] for s in case["species"]],
        "inlet_concentration": case["boundary"]["inlet_concentration"],
    }
    vectors.update(
        {
            f"initial_{j}": values
            for j, values in enumerate(metrics._initial_arrays(case, launch["nodes"]))
        }
    )
    fields.update(
        {key: ",".join(format(v, ".17g") for v in values) for key, values in vectors.items()}
    )
    fields["species"] = ",".join(s["name"] for s in case["species"])
    return "".join(f"{key}={value}\n" for key, value in fields.items()).encode("ascii")


class RunFixture:
    def __init__(self, root, case_id="reaction_zero", solver="rust", slopes=None):
        self.root = root
        self.case = next(c for c in BOOK["cases"] if c["id"] == case_id)
        self.launch = next(
            r
            for r in BOOK["launches"]
            if r["case_id"] == case_id and r["solver"] == solver and r["role"] == "benchmark"
        )
        case, launch = self.case, self.launch
        times, steps = metrics.schedule(case, launch)
        self.initial = metrics._initial_arrays(case, launch["nodes"])
        self.slopes = slopes or [0.0] * len(case["species"])
        manifest = {
            "schema_version": "solute-manifest-v1",
            "run_id": "synthetic",
            "launch_id": launch["id"],
            "case_id": case_id,
            "cases_sha256": metrics.digest_file(BOOK_PATH),
            "case_sha256": metrics.digest_bytes(metrics.canonical(case)),
            "solver": solver,
            "precision": launch["precision"],
            "status": "complete" if solver == "rust" else "raw_complete",
            "command": ["synthetic-no-execution"],
            "cwd": str(root),
            "started_utc": "2026-09-27T00:00:00+00:00",
            "ended_utc": "2026-09-27T00:00:01+00:00",
            "exit_code": 0,
            "timeout_seconds": launch["timeout_seconds"],
            "warnings": [],
            "mapping_validation": {
                "status": "passed",
                "theta_range": [case["theta"]] * 2,
                "q_range": [case["q"]] * 2,
                "dispersion_range": [case["dispersion"]] * 2,
                "evidence": ["synthetic test water history"],
            },
            "record_inventory": {
                "nodes": launch["nodes"],
                "species": len(case["species"]),
                "profile_times": times,
                "step_count": len(steps),
                "endpoint_times": [0.0] + steps,
                "terminal_records": {},
                "field_formats": dict(metrics.FORMATS),
            },
            "artifacts": {},
        }
        request = make_request(case, launch, manifest)
        (root / "request.txt").write_bytes(request)
        manifest["request_sha256"] = metrics.digest_bytes(request)
        (root / "stdout.txt").write_text("")
        (root / "stderr.txt").write_text("")
        executable = root / "build" / "synthetic-executable"
        executable.parent.mkdir()
        executable.write_bytes(b"synthetic fixture; never executed\n")
        executable.chmod(0o755)
        source = dict(BOOK["provenance"]["source_sha256"])
        if solver == "fortran":
            source.update(
                {
                    k: v
                    for k, v in metrics.json_load(
                        metrics.ROOT / "planning/master/SOURCE_MANIFEST.json"
                    ).items()
                    if k.startswith("src_reference/") or k == "planning/H1D_Src.zip"
                }
            )
            (root / "build/gnu_compatibility.FOR").write_text("synthetic shim\n")
        manifest["provenance"] = {
            "source_sha256": source,
            "compiler": "synthetic",
            "platform": "test",
            "flags": ["-std=legacy", "-ffixed-line-length-none"] if solver == "fortran" else [],
            "executable_sha256": metrics.digest_file(executable),
            "shim_sha256": (
                metrics.digest_file(root / "build/gnu_compatibility.FOR")
                if solver == "fortran"
                else None
            ),
            "adapter_sha256": {
                "tools/solute_verification_metrics.py": metrics.digest_file(metrics.__file__)
            },
            "input_sha256": {"request.txt": manifest["request_sha256"]},
        }
        data = {
            key: manifest[key]
            for key in (
                "run_id",
                "launch_id",
                "case_id",
                "cases_sha256",
                "case_sha256",
                "request_sha256",
                "solver",
            )
        }
        data.update(
            schema_version="solute-observation-v1",
            units=BOOK["units"],
            species=[s["name"] for s in case["species"]],
            coordinates=[
                i * case["length"] / (launch["nodes"] - 1) for i in range(launch["nodes"])
            ],
            profiles=[
                {
                    "time": t,
                    "concentration": self.concentration(t),
                    "theta": [case["theta"]] * launch["nodes"],
                    "q_faces": [case["q"]] * (launch["nodes"] + 1),
                }
                for t in times
            ],
            steps=[
                {
                    "index": i,
                    "time": t,
                    "dt": launch["dt"],
                    "iterations": 1,
                    "peclet": 0.0,
                    "courant": 0.0,
                }
                for i, t in enumerate(steps, 1)
            ],
            endpoint_history=[
                {"time": t, "concentration": [[v[0], v[-1]] for v in self.concentration(t)]}
                for t in [0.0] + steps
            ],
            solver_reported={
                k: None
                for k in ("inventory", "boundary_cumulative", "reaction_cumulative", "balance")
            },
            availability={
                "water": "synthetic constant input water",
                "endpoint_history": "all synthetic steps",
                "solver_reported": "unavailable in synthetic test",
            },
            warnings=[],
        )
        self.data, self.manifest = data, manifest
        if solver == "rust":
            self.write_observation()
        else:
            self.write_fortran()
        self.refresh()

    def concentration(self, time):
        return [
            [v + slope * time for v in initial] for slope, initial in zip(self.slopes, self.initial)
        ]

    def write_observation(self):
        (self.root / "observations.json").write_bytes(metrics.canonical(self.data))

    def refresh(self):
        self.manifest["artifacts"] = {
            str(path.relative_to(self.root)): {
                "path": str(path.relative_to(self.root)),
                "sha256": metrics.digest_file(path),
                "bytes": path.stat().st_size,
            }
            for path in self.root.rglob("*")
            if path.is_file() and path.name not in ("manifest.json", "normalization.json")
        }
        (self.root / "manifest.json").write_bytes(metrics.canonical(self.manifest))

    def load(self):
        return metrics.load_observations(
            self.root / "observations.json", self.root / "manifest.json", BOOK_PATH
        )

    def reference(self):
        case, launch = self.case, self.launch
        times, _ = metrics.schedule(case, launch)
        profiles = [
            {"time": t, "concentration": self.concentration(t)}
            for t in [0.0] + launch["analysis_times"]
        ]
        continuum = []
        for t in times:
            dissolved, sorbed, total, losses = [], [], [], []
            for j, species in enumerate(case["species"]):
                # Tests using this reference use spatially uniform ICs.
                c = self.initial[j][0] + self.slopes[j] * t
                integral_c = self.initial[j][0] * t + self.slopes[j] * t * t / 2
                dissolved.append(case["length"] * case["theta"] * c)
                sorbed.append(case["length"] * case["rho"] * species["kd"] * c)
                total.append(dissolved[-1] + sorbed[-1])
                losses.append(
                    case["length"]
                    * (case["theta"] + case["rho"] * species["kd"])
                    * species["rate"]
                    * integral_c
                )
            continuum.append(
                {
                    "time": t,
                    "dissolved": dissolved,
                    "equilibrium_sorbed": sorbed,
                    "total": total,
                    "production_cumulative": [0.0] + losses[:-1],
                    "loss_cumulative": losses,
                    "endpoint_flux": [[0.0, 0.0] for _ in total],
                    "boundary_cumulative": [[0.0, 0.0] for _ in total],
                }
            )

        def zero(records):
            result = copy.deepcopy(records)
            for row in result:
                for key, values in row.items():
                    if key != "time":
                        row[key] = (
                            [[0.0] * len(v) for v in values]
                            if isinstance(values[0], list)
                            else [0.0] * len(values)
                        )
            return result

        return {
            "schema_version": "solute-metrics-reference-v1",
            "coordinates": self.data["coordinates"],
            "species": self.data["species"],
            "profiles": profiles,
            "continuum": continuum,
            "uncertainty": {"profiles": zero(profiles), "continuum": zero(continuum)},
        }

    def write_fortran(self):
        case, launch = self.case, self.launch
        n, species_count = launch["nodes"], len(case["species"])
        times, steps = metrics.schedule(case, launch)

        def write(name, lines, terminal="end"):
            (self.root / ("solute-input\\" + name)).write_text("\n".join(lines + [terminal]) + "\n")
            self.manifest["record_inventory"]["terminal_records"][name] = terminal

        run = ["TLevel Time dt ItrW ItrC ItCum KodT KodB Converg Peclet Courant"]
        water = [
            "Time rTop rRoot vTop vRoot vBot sum(rTop) sum(rRoot) sum(vTop) sum(vRoot) sum(vBot) hTop hRoot hBot RunOff sum(RunOff) Volume sum(Infil) sum(Evap) TLevel Cum(WTrans) SnowLayer"
        ]
        solutes = [
            [
                "All solute fluxes and cumulative solute fluxes are positive into the region",
                "Time cvTop cvBot Sum(cvTop) Sum(cvBot) cvCh0 cvCh1 cTop cRoot cBot cvRoot Sum(cvRoot) Sum(cvNEql) TLevel cGWL cRunOff Sum(cRunOff) (cv(i), Sum(cv(i)), i=1,NObs)",
            ]
            for _ in case["species"]
        ]
        for i, t in enumerate(steps, 1):
            run.append(f"{i} {fexp(t, 7)} {fexp(launch['dt'], 5)} 2 1 {2*i} 1 1 T 0.000 0.000")
            row = [fexp(0, 5)] * 22
            row[0], row[3], row[5] = f"{t:.4f}", fexp(-case["q"], 5), fexp(-case["q"], 5)
            row[16], row[19], row[21] = fexp(case["length"] * case["theta"], 5), str(i), "0.000"
            water.append(" ".join(row))
            for species_rows in solutes:
                row = [fexp(0, 5)] * 17
                row[0], row[13] = f"{t:.4f}", str(i)
                species_rows.append(" ".join(row))
        write("RUN_INF.OUT", run)
        write("T_LEVEL.OUT", water)
        for j, rows in enumerate(solutes, 1):
            write(f"solute{j}.out", rows)
        nodal = []
        balance = []
        for t in times:
            nodal += [
                f"Time: {t:.4f}",
                "Node Depth Head Moisture K C Flux Sink Kappa v/KsTop Temp Conc(1..NS) Sorb(1...NS)",
            ]
            c = self.concentration(t)
            for i in range(n):
                row = [
                    str(i + 1),
                    f"{-i * case['length'] / (n-1):.4f}",
                    "0.000",
                    "0.4300",
                    fexp(case["hydraulics"]["ks"]),
                    fexp(0),
                    fexp(-case["q"]),
                    fexp(0),
                    "-1",
                    "0.000",
                    "20.00",
                ]
                row += [fexp(c[j][i]) for j in range(species_count)] + [fexp(0)] * species_count
                nodal.append(" ".join(row))
            balance.append(f"Time [T] {t:.4f}")
            for j, species in enumerate(case["species"], 1):
                mass = case["length"] * (case["theta"] + case["rho"] * species["kd"]) * c[j - 1][0]
                balance += [
                    f"ConcVol [M/L2] {j} {fexp(mass, 5)} {fexp(mass, 5)}",
                    f"cMean [M/L3] {j} {fexp(c[j-1][0], 5)} {fexp(c[j-1][0], 5)}",
                ]
                if t:
                    balance.append(f"CncBalT [M] {j} {fexp(0, 5)}")
        write("NOD_INF.OUT", nodal)
        write("BALANCE.OUT", balance, "Calculation time [sec] 1.0")
        obs = [f"Node(1) Node({n})", "time h theta Temp Conc"]
        for t in steps:
            c = self.concentration(t)
            fields = [f"{t:.4f}"]
            for node in (0, n - 1):
                fields += ["0.00", "0.4300", "20.000"] + [
                    fexp(c[j][node]) for j in range(species_count)
                ]
            obs.append(" ".join(fields))
        write("OBS_NODE.OUT", obs)
        profile = ["n depth THr THs hs Ks Ks/KsTop Beta Ah AK ATh"]
        check = ["node      x         hOld    MatN LayN Beta Ah AK ATh Temp Conc"]
        for i in range(n):
            x = i * case["length"] / (n - 1)
            profile.append(
                f"{i+1} {x:.2f} 0.080 0.430 -0.0 {fexp(case['hydraulics']['ks'],3)} 1.000 0.000 1.000 1.000 1.000"
            )
            check.append(
                f"{i+1} {-x:.3f} 0.000 1 1 0.000 1.000 1.000 1.000 20.00 "
                + " ".join(fexp(self.initial[j][i]) for j in range(species_count))
            )
        check += [
            "end",
            "Solute transport information",
            "Mat. Bulk.D. DispL Fraction Immobile WC",
            f"1 {case['rho']:.4f} {case['chemistry']['dispersivity']:.4f} 1.0000 0.0000",
        ]
        for j, species in enumerate(case["species"], 1):
            loss = species["rate"] if j == species_count else 0
            transfer = species["rate"] if j < species_count else 0
            coefficients = [
                species["kd"],
                0,
                1,
                0,
                loss,
                loss,
                0,
                transfer,
                transfer,
                0,
                0,
                0,
                0,
                0,
            ]
            check += [
                f"Dif.w. Dif.g. ----- ({j}.solute)",
                f"{fexp(case['chemistry']['molecular_diffusion'])} {fexp(0)}",
                "Mat. KS Nu Beta Henry SinkL1 SinkS1 SinkG1 SinkL1` SinkS1` SinkG1` SinkL0 SinkS0 SinkG0 Alfa",
                "1 " + " ".join(fexp(v) for v in coefficients),
            ]
        check += [
            "kTopCh cTop(1...NS)",
            "-1 " + " ".join(fexp(v, 3) for v in case["boundary"]["inlet_concentration"]),
            "kBotCh cBot(1...NS)",
            "-1 " + " ".join(fexp(0, 3) for _ in case["species"]),
        ]
        write("PROFILE.OUT", profile)
        write("I_CHECK.OUT", check, f"tPulse = {case['time_end']:.3f}")


class MetricsTests(unittest.TestCase):
    def setUp(self):
        SCRATCH.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=SCRATCH)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def test_unequal_mesh_weights_and_phase_inventory(self):
        weights = metrics.mesh_weights([0, 1, 3])
        self.assertEqual(weights, [0.5, 1.5, 1.0])
        actual = metrics._inventory([1, 2, 4], [0.5] * 3, 0.25, weights)
        self.assertEqual(actual, {"dissolved": 3.75, "equilibrium_sorbed": 1.875, "total": 5.625})
        norms = metrics._norms([1, -2, 4], weights)
        self.assertEqual(norms["l1"], 2.5)
        self.assertAlmostEqual(norms["l2"], math.sqrt(7.5))
        self.assertEqual(norms["interior_linf"], 2)

    def test_zero_near_zero_and_resolved_scales(self):
        for target, uncertainty, branch in [
            (0, 0, "zero"),
            (1e-6, 2e-6, "near_zero"),
            (2, 0.1, "resolved"),
        ]:
            with self.subTest(branch=branch):
                actual = metrics.normalized_error(1, target, uncertainty)
                self.assertEqual(actual["branch"], branch)
                self.assertEqual(actual["relative"] is None, branch != "resolved")
        self.assertEqual(metrics.normalized_error(1, -2)["relative"], 1.5)

    def test_printed_rounding_four_significant_digits(self):
        self.assertEqual(metrics.rounding_half_width("0.4300E+00", "e12.4"), 5e-5)
        self.assertEqual(metrics.rounding_half_width("0.4300E-08", "e12.4"), 5e-13)
        self.assertEqual(metrics.rounding_half_width("0.0000E+00", "e12.4"), 5e-5)
        with self.assertRaises(ValueError):
            metrics.rounding_half_width("4.30000E-01", "e12.4")
        metrics.printed_match("0.0001", 0.0001220703125, "f14.4", "tiny dt")
        with self.assertRaises(ValueError):
            metrics.printed_match("0.0000", 0.0001220703125, "f14.4", "tiny dt")

    def test_load_rust_and_exact_zero_reaction_metrics(self):
        fixture = RunFixture(self.root)
        result = metrics.compute_metrics(fixture.load(), fixture.reference())
        first = result["derived"]["species"][0]
        self.assertEqual(first["history"][-1]["inventory"]["total"], 1.72)
        self.assertEqual(first["history"][-1]["balance"]["residual"], 0)
        self.assertEqual(first["temporal_l2"], 0)
        self.assertIsNone(result["solver_reported"]["balance"])
        self.assertIsNone(result["derived"]["thresholds"])

    def test_independent_reaction_integrals_and_parent_transfer(self):
        fixture = RunFixture(self.root, "reaction_distinct", slopes=[0.1, 0.2, 0.3])
        result = metrics.compute_metrics(fixture.load(), fixture.reference())
        history = [species["history"][-1] for species in result["derived"]["species"]]
        self.assertAlmostEqual(history[0]["reaction"]["loss"], 0.2 * 1.72 * (4 + 0.1 * 8))
        self.assertAlmostEqual(history[1]["reaction"]["production"], history[0]["reaction"]["loss"])
        self.assertAlmostEqual(history[1]["reaction"]["loss"], 0.4 * 0.86 * 0.2 * 8)
        self.assertAlmostEqual(history[2]["reaction"]["loss"], 0.1 * 1.29 * 0.3 * 8)
        self.assertNotEqual(history[0]["balance"]["residual"], 0)

    def test_species_isolation_without_parent_coupling(self):
        fixture = RunFixture(self.root)
        baseline = metrics.compute_metrics(fixture.load(), fixture.reference())["derived"][
            "species"
        ][0]
        fixture.slopes[1] = 0.25
        for p in fixture.data["profiles"]:
            p["concentration"] = fixture.concentration(p["time"])
        for p in fixture.data["endpoint_history"]:
            p["concentration"] = [[r[0], r[-1]] for r in fixture.concentration(p["time"])]
        fixture.write_observation()
        fixture.refresh()
        changed = metrics.compute_metrics(fixture.load(), fixture.reference())["derived"][
            "species"
        ][0]
        self.assertEqual(changed, baseline)

    def test_continuum_and_mesh_error_are_separate(self):
        fixture = RunFixture(self.root)
        reference = fixture.reference()
        for row in reference["continuum"]:
            row["total"][0] += 0.125
        result = metrics.compute_metrics(fixture.load(), reference)
        last = result["derived"]["species"][0]["history"][-1]
        self.assertEqual(last["inventory_mesh_error"]["total"], 0)
        self.assertEqual(last["inventory_mesh_quadrature"]["total"], -0.125)
        self.assertEqual(last["inventory_error"]["total"]["absolute"], 0.125)

    def test_solver_reported_values_never_close_balance(self):
        fixture = RunFixture(self.root)
        fixture.data["solver_reported"]["balance"] = [
            {"time": p["time"], "values": [1e9] * 3} for p in fixture.data["profiles"]
        ]
        fixture.write_observation()
        fixture.refresh()
        result = metrics.compute_metrics(fixture.load(), fixture.reference())
        self.assertEqual(result["derived"]["species"][0]["history"][-1]["balance"]["residual"], 0)
        self.assertEqual(result["solver_reported"]["balance"][-1]["values"][0], 1e9)

    def test_reject_unvalidated_and_mutated_observations(self):
        fixture = RunFixture(self.root)
        observations = fixture.load()
        with self.assertRaises(ValueError):
            metrics.compute_metrics(dict(observations), fixture.reference())
        observations["profiles"][-1]["concentration"][0][0] += 1
        with self.assertRaises(ValueError):
            metrics.compute_metrics(observations, fixture.reference())

    def test_reject_reference_shape_time_and_nonfinite_errors(self):
        fixture = RunFixture(self.root)
        observations = fixture.load()
        for mutate in (
            lambda r: r["profiles"].pop(),
            lambda r: r["continuum"].append(r["continuum"][-1]),
            lambda r: r["profiles"][0]["concentration"][0].pop(),
            lambda r: r["uncertainty"]["profiles"][0]["concentration"][0].__setitem__(0, -1),
            lambda r: r["continuum"][0]["total"].__setitem__(0, float("nan")),
        ):
            reference = fixture.reference()
            mutate(reference)
            with self.assertRaises(ValueError):
                metrics.compute_metrics(observations, reference)

    def test_bad_artifact_hash_and_executable_binding(self):
        fixture = RunFixture(self.root)
        (self.root / "observations.json").write_text("{}")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            fixture.load()
        fixture.write_observation()
        fixture.refresh()
        fixture.manifest["provenance"]["executable_sha256"] = "f" * 64
        fixture.refresh()
        with self.assertRaisesRegex(ValueError, "retained executable"):
            fixture.load()

    def test_bad_request_initial_even_with_rehashed_manifest(self):
        fixture = RunFixture(self.root)
        path = self.root / "request.txt"
        text = path.read_text().replace("initial_0=1,", "initial_0=2,")
        path.write_text(text)
        fixture.manifest["request_sha256"] = metrics.digest_file(path)
        fixture.manifest["provenance"]["input_sha256"]["request.txt"] = metrics.digest_file(path)
        fixture.refresh()
        with self.assertRaisesRegex(ValueError, "initial condition"):
            fixture.load()

    def test_manifest_backend_key_is_optional_and_exact(self):
        fixture = RunFixture(self.root)
        fixture.manifest["backend"] = "fortran-compatible"
        fixture.refresh()
        fixture.load()
        fixture.manifest["backend"] = "other"
        fixture.refresh()
        with self.assertRaisesRegex(ValueError, "manifest backend"):
            fixture.load()
        del fixture.manifest["backend"]
        fixture.manifest["extra"] = 1
        fixture.refresh()
        with self.assertRaisesRegex(ValueError, "wrong keys"):
            fixture.load()

    def test_request_backend_key_is_optional_and_matches_manifest(self):
        fixture = RunFixture(self.root)
        path = self.root / "request.txt"

        def rebind(path):
            digest = metrics.digest_file(path)
            fixture.manifest["request_sha256"] = digest
            fixture.manifest["provenance"]["input_sha256"]["request.txt"] = digest
            obs = metrics.json_load(self.root / "observations.json")
            obs["request_sha256"] = digest
            (self.root / "observations.json").write_text(json.dumps(obs))

        path.write_text(path.read_text() + "backend=fortran-compatible\n")
        rebind(path)
        fixture.manifest["backend"] = "fortran-compatible"
        fixture.refresh()
        fixture.load()
        del fixture.manifest["backend"]
        fixture.refresh()
        with self.assertRaisesRegex(ValueError, "request backend"):
            fixture.load()
        path.write_text(path.read_text().replace("backend=fortran-compatible", "backend=other"))
        rebind(path)
        fixture.manifest["backend"] = "fortran-compatible"
        fixture.refresh()
        with self.assertRaisesRegex(ValueError, "request backend"):
            fixture.load()

    def test_request_upstream_weighting_matches_case_and_accepts_legacy_omission(self):
        fixture = RunFixture(self.root, case_id="scalar_robin")
        path = self.root / "request.txt"

        def rebind(path):
            digest = metrics.digest_file(path)
            fixture.manifest["request_sha256"] = digest
            fixture.manifest["provenance"]["input_sha256"]["request.txt"] = digest
            obs = metrics.json_load(self.root / "observations.json")
            obs["request_sha256"] = digest
            (self.root / "observations.json").write_text(json.dumps(obs))

        old_request = path.read_text()
        fixture.manifest["backend"] = "fortran-compatible"
        path.write_text(old_request + "backend=fortran-compatible\n")
        rebind(path)
        fixture.refresh()
        fixture.load()

        path.write_text(
            old_request + "backend=fortran-compatible\nupstream_weighting=true\n"
        )
        rebind(path)
        fixture.refresh()
        fixture.load()

        path.write_text(
            old_request + "backend=fortran-compatible\nupstream_weighting=false\n"
        )
        rebind(path)
        fixture.refresh()
        with self.assertRaisesRegex(ValueError, "request upstream weighting"):
            fixture.load()

    def test_json_duplicate_nonfinite_and_symlink(self):
        path = self.root / "bad.json"
        for text in ('{"a":1,"a":2}', '{"a":NaN}'):
            path.write_text(text)
            with self.assertRaises(ValueError):
                metrics.json_load(path)
        (self.root / "alias").symlink_to(path)
        with self.assertRaises(ValueError):
            metrics.contained(self.root, "alias")
        with self.assertRaises(ValueError):
            metrics.contained(self.root, "../outside")

    def test_duplicate_missing_times_and_nonfinite_observations(self):
        fixture = RunFixture(self.root)
        original = copy.deepcopy(fixture.data)
        mutations = [
            lambda d: d["profiles"].pop(),
            lambda d: d["profiles"].append(d["profiles"][-1]),
            lambda d: d["steps"][0].__setitem__("index", 2),
            lambda d: d["profiles"][0]["theta"].__setitem__(0, 0.2),
            lambda d: d["endpoint_history"].pop(),
            lambda d: d["profiles"][0]["concentration"][0].pop(),
        ]
        for mutate in mutations:
            fixture.data = copy.deepcopy(original)
            mutate(fixture.data)
            fixture.write_observation()
            fixture.refresh()
            with self.assertRaises(ValueError):
                fixture.load()

    def test_fortran_normalization_receipt_roundtrip_and_no_overwrite(self):
        fixture = RunFixture(self.root, solver="fortran")
        before = (self.root / "manifest.json").read_bytes()
        observations = metrics.normalize_fortran(self.root, BOOK_PATH)
        self.assertEqual(observations, fixture.load())
        self.assertEqual(before, (self.root / "manifest.json").read_bytes())
        self.assertGreater(observations._rounding["profiles"][0]["concentration"][0][0], 0)
        result = metrics.compute_metrics(observations, fixture.reference())
        self.assertAlmostEqual(
            result["derived"]["species"][0]["history"][-1]["inventory"]["equilibrium_sorbed"], 0.86
        )
        with self.assertRaises(ValueError):
            metrics.normalize_fortran(self.root, BOOK_PATH)

    def test_fortran_missing_duplicate_truncated_and_malformed_profiles(self):
        fixture = RunFixture(self.root, solver="fortran")
        path = self.root / "solute-input\\NOD_INF.OUT"
        original = path.read_text()
        for text in (
            original.rsplit("end", 1)[0],
            original.replace("1 0.0000", "2 0.0000", 1),
            original.replace(" 20.00 ", " ", 1),
            original.replace("Time: 0.0000", "Time: 0.1250", 1),
        ):
            path.write_text(text)
            with self.assertRaises(ValueError):
                metrics._node_profiles(path, fixture.case, fixture.launch)
        path.write_text(original.replace("0.1000E+01", "NaN", 1))
        with self.assertRaises(ValueError):
            metrics._node_profiles(path, fixture.case, fixture.launch)

    def test_inactive_sorb_rejected_and_never_counted_as_equilibrium(self):
        fixture = RunFixture(self.root, solver="fortran")
        path = self.root / "solute-input\\NOD_INF.OUT"
        lines = path.read_text().splitlines()
        fields = lines[2].split()
        fields[-1] = fexp(1)
        lines[2] = " ".join(fields)
        path.write_text("\n".join(lines) + "\n")
        with self.assertRaisesRegex(ValueError, "Sorb"):
            metrics._node_profiles(path, fixture.case, fixture.launch)

    def test_multiple_node_counts(self):
        for nodes in (3, 5, 17):
            with self.subTest(nodes=nodes):
                case = next(c for c in BOOK["cases"] if c["id"] == "reaction_zero")
                launch = copy.deepcopy(
                    next(
                        r
                        for r in BOOK["launches"]
                        if r["case_id"] == "reaction_zero" and r["solver"] == "fortran"
                    )
                )
                launch["nodes"] = nodes
                times, _ = metrics.schedule(case, launch)
                lines = []
                for time in times:
                    lines += [
                        f"Time: {time:.4f}",
                        "Node Depth Head Moisture K C Flux Sink Kappa v/KsTop Temp Conc(1..NS) Sorb(1...NS)",
                    ]
                    for i in range(nodes):
                        lines.append(
                            f"{i+1} {-i*2/(nodes-1):.4f} 0.000 0.4300 {fexp(.43)} {fexp(0)} {fexp(0)} {fexp(0)} -1 0.000 20.00 "
                            + " ".join(fexp(v) for v in (1, 0.25, 0, 0, 0, 0))
                        )
                path = self.root / str(nodes)
                path.write_text("\n".join(lines + ["end"]) + "\n")
                coordinates, profiles, _, _ = metrics._node_profiles(path, case, launch)
                self.assertEqual(len(coordinates), nodes)
                self.assertEqual(len(profiles), len(times))

    def test_normalization_rejects_foreign_root_and_dangling_output(self):
        with tempfile.TemporaryDirectory() as other:
            foreign = Path(other) / ".development/13.3.1/run"
            foreign.mkdir(parents=True)
            with self.assertRaisesRegex(ValueError, "outside task scratch"):
                metrics.normalize_fortran(foreign, BOOK_PATH)
        (self.root / "observations.json").symlink_to(self.root / "missing")
        with self.assertRaisesRegex(ValueError, "existing artifacts"):
            metrics.normalize_fortran(self.root, BOOK_PATH)

    def test_tiny_norm_error_preserves_relative_and_interval(self):
        result = metrics._normalized_absolute(1e-20, 1.0, 0, 0, "reference l2")
        self.assertEqual(result["relative"], 1e-20)
        self.assertEqual(result["error_interval"], [1e-20, 1e-20])

    def test_flowing_case_signed_inlet_and_outlet(self):
        fixture = RunFixture(self.root, "scalar_robin")
        result = metrics.compute_metrics(fixture.load(), fixture.reference())
        last = result["derived"]["species"][0]["history"][-1]
        self.assertEqual(last["boundary"]["instantaneous"][0], 0.43)
        outlet = -0.43 * fixture.initial[0][-1]
        self.assertEqual(last["boundary"]["instantaneous"][1], outlet)
        self.assertAlmostEqual(last["boundary"]["cumulative"][1], outlet)
        self.assertEqual(last["boundary"]["cumulative"][0], 0.43)

    def test_signed_endpoint_integral_and_quadrature_sensitivity(self):
        times = [0, 1, 2, 3, 4]
        values = [-0.5 * t * t for t in times]
        result, error = metrics._integral_history(times, values, [0] * 5, 0)
        self.assertEqual(result[-1], -11)
        self.assertEqual(error[-1], 0)
        sensitivity = metrics._coarsening_sensitivity(times, values, result)
        self.assertEqual(sensitivity[-1], 1)


if __name__ == "__main__":
    unittest.main()

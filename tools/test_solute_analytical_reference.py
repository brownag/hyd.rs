"""Independent component identities and uncertainty checks; no solver launches."""

import copy
import math
import unittest
from decimal import Decimal, localcontext
from pathlib import Path

from tools import solute_analytical_reference as analytical
from tools import solute_verification_metrics as metrics
from tools.check_solute_verification_spec import load_cases, validate

BOOK = validate(
    load_cases(Path(__file__).resolve().parents[1] / "reference/solute_verification/CASES.json")
)
CASES = {case["id"]: case for case in BOOK["cases"]}
I = analytical.Interval
D = Decimal


def configuration(case_id):
    case = copy.deepcopy(CASES[case_id])
    launch = next(
        r for r in BOOK["launches"] if r["case_id"] == case_id and r["role"] == "benchmark"
    )
    coordinates = [i * case["length"] / (launch["nodes"] - 1) for i in range(launch["nodes"])]
    times = [
        k * launch["sampling_interval"]
        for k in range(round(case["time_end"] / launch["sampling_interval"]) + 1)
    ]
    return case, coordinates, launch["analysis_times"], times


def evaluate(case_id):
    return analytical.component_reference(*configuration(case_id))


def simpson(function, low, high, intervals=1000):
    step = (high - low) / intervals
    return (
        step
        / 3
        * math.fsum(
            function(low + k * step) * (1 if k in (0, intervals) else 4 if k % 2 else 2)
            for k in range(intervals + 1)
        )
    )


class ArithmeticTests(unittest.TestCase):
    def test_outward_arithmetic_and_serialization(self):
        with localcontext() as context:
            context.prec = 120
            exact = (D(2).sqrt() + D("0.1").exp()) / 3
            result = (I(2).sqrt() + I("0.1").exp()) / 3
            self.assertTrue(result.contains(exact))
            value, error = result.serialized()
            self.assertLessEqual(abs(D.from_float(value) - exact), D.from_float(error))
        self.assertEqual(I(0).serialized(), (0.0, 0.0))
        self.assertTrue((I(-2, 3) * I(-4, 5)).contains(-12))
        with self.assertRaisesRegex(ValueError, "includes zero"):
            _ = I(1) / I(-1, 1)

    def test_pi_and_trigonometric_bounds(self):
        pi_digits = D("3.141592653589793238462643383279502884197169399375105820974944592307816406")
        self.assertTrue(analytical.pi_interval().contains(pi_digits))
        for x in (D("-3.1"), D("-0.2"), D(0), D("0.7"), D("3.9")):
            with localcontext() as context:
                context.prec = 110
                # Independent direct powers/factorials, rather than term recurrence.
                sine = sum(
                    (-1) ** k * x ** (2 * k + 1) / D(math.factorial(2 * k + 1)) for k in range(100)
                )
                cosine = 1 + sum(
                    (-1) ** k * x ** (2 * k) / D(math.factorial(2 * k)) for k in range(1, 100)
                )
            actual_sine, actual_cosine = analytical.sincos(x)
            self.assertTrue(actual_sine.contains(sine), str(x))
            self.assertTrue(actual_cosine.contains(cosine), str(x))
        with self.assertRaises(ValueError):
            analytical.sincos(5)


class ReactionTests(unittest.TestCase):
    def test_distinct_rates_and_independent_integrals(self):
        rates = list(map(D, (".2", ".4", ".1")))
        for t in (D(".125"), D(1), D(4)):
            actual, integrated, tail = analytical.reaction_masses([I(".86"), I(0), I(0)], rates, t)
            with localcontext() as context:
                context.prec = 110
                factor = D(".86")
                for j in range(3):
                    subset = rates[: j + 1]
                    denominators = [
                        math.prod(r - rate for r in subset if r != rate) for rate in subset
                    ]
                    expected = factor * sum(
                        (-rate * t).exp() / den for rate, den in zip(subset, denominators)
                    )
                    integral = factor * sum(
                        (1 - (-rate * t).exp()) / rate / den
                        for rate, den in zip(subset, denominators)
                    )
                    self.assertTrue(actual[j].contains(expected), (t, j))
                    self.assertTrue(integrated[j].contains(integral), (t, j))
                    factor *= rates[j]
            self.assertLess(tail.high, D("1e-70"))

    def test_equal_rate_and_zero_terminal_limits(self):
        masses, integrated, _ = analytical.reaction_masses([1, 0, 0], [D(".2"), D(".2"), 0], 4)
        with localcontext() as context:
            context.prec = 110
            exponential = D("-.8").exp()
            expected = [exponential, D(".8") * exponential, 1 - D("1.8") * exponential]
            integrals = [(1 - exponential) / D(".2"), (1 - D("1.8") * exponential) / D(".2")]
            integrals.append(4 - sum(integrals))  # Independent identity used only in this test.
            for actual, value in zip(masses, expected):
                self.assertTrue(actual.contains(value))
            for actual, value in zip(integrated, integrals):
                self.assertTrue(actual.contains(value))
        self.assertTrue(sum(masses, I(0)).contains(1))

    def test_near_equal_rates_do_not_cancel(self):
        rates = [D(".2"), D(".200000000000000000000000000001"), D(0)]
        masses, integrated, _ = analytical.reaction_masses([1, 0, 0], rates, 4)
        with localcontext() as context:
            context.prec = 130
            a, b = rates[:2]
            daughter = a * ((-a * 4).exp() - (-b * 4).exp()) / (b - a)
            integral = a * ((1 - (-a * 4).exp()) / a - (1 - (-b * 4).exp()) / b) / (b - a)
        self.assertTrue(masses[1].contains(daughter))
        self.assertTrue(integrated[1].contains(integral))

    def test_zero_rates_initial_daughters_and_terminal_loss(self):
        masses, integrated, tail = analytical.reaction_masses([2, 3, 4], [0, 0, 0], 4)
        self.assertEqual([x.serialized()[0] for x in masses], [2, 3, 4])
        self.assertEqual([x.serialized()[0] for x in integrated], [8, 12, 16])
        self.assertEqual(tail.high, 0)
        actual, integrals, _ = analytical.reaction_masses([0, 0, 4], [D(".2"), D(".4"), D(".1")], 4)
        with localcontext() as context:
            context.prec = 100
            self.assertTrue(actual[2].contains(4 * D("-.4").exp()))
            self.assertTrue(integrals[2].contains(40 * (1 - D("-.4").exp())))

    def test_poisson_bound_covers_coarse_truncation(self):
        coarse, integrated, tail = analytical.reaction_masses(
            [1, 0, 0], [D(".2"), D(".4"), 0], 4, terms=2
        )
        fine, fine_integrals, _ = analytical.reaction_masses([1, 0, 0], [D(".2"), D(".4"), 0], 4)
        self.assertGreater(tail.high, D(".05"))
        for a, b in zip(coarse + integrated, fine + fine_integrals):
            self.assertLessEqual(a.low, b.low)
            self.assertGreaterEqual(a.high, b.high)
        with self.assertRaisesRegex(ValueError, "tail requires"):
            analytical.reaction_masses([1], [1], 4, terms=2)

    def test_initial_daughter_superposition(self):
        rates = list(map(D, (".2", ".4", ".1")))
        initial = list(map(D, (".86", ".3", ".7")))
        actual, integrated, _ = analytical.reaction_masses(initial, rates, 4)
        with localcontext() as context:
            context.prec = 110
            for j in range(3):
                expected, cumulative = D(0), D(0)
                for parent in range(j + 1):
                    subset = rates[parent : j + 1]
                    factor = initial[parent] * math.prod(rates[parent:j])
                    for rate in subset:
                        denominator = math.prod(r - rate for r in subset if r != rate)
                        expected += factor * (-rate * 4).exp() / denominator
                        cumulative += factor * (1 - (-rate * 4).exp()) / rate / denominator
                self.assertTrue(actual[j].contains(expected))
                self.assertTrue(integrated[j].contains(cumulative))

    def test_unequal_storage_and_independent_reaction_balance(self):
        for case_id in ("reaction_distinct", "reaction_equal", "reaction_zero"):
            case = CASES[case_id]
            reference = evaluate(case_id)
            initial = reference["continuum"][0]["total"]
            for row in reference["continuum"]:
                for j, s in enumerate(case["species"]):
                    mass = row["total"][j]
                    concentration = next(
                        (
                            r["concentration"][j][0]
                            for r in reference["profiles"]
                            if r["time"] == row["time"]
                        ),
                        None,
                    )
                    if concentration is not None:
                        self.assertAlmostEqual(
                            mass,
                            (case["theta"] + case["rho"] * s["kd"])
                            * case["length"]
                            * concentration,
                        )
                    self.assertAlmostEqual(
                        mass
                        - initial[j]
                        - row["production_cumulative"][j]
                        + row["loss_cumulative"][j],
                        0,
                        places=14,
                    )
            if case_id == "reaction_distinct":
                self.assertLess(sum(reference["continuum"][-1]["total"]), sum(initial))


class DiffusionTests(unittest.TestCase):
    def test_profile_pde_and_boundary_identities(self):
        case, coordinates, analysis, times = configuration("diffusion_cosine")
        reference = analytical.component_reference(case, coordinates, analysis, times)
        rate = case["dispersion"] * math.pi**2 / 8
        for profile, uncertainty in zip(
            reference["profiles"], reference["uncertainty"]["profiles"]
        ):
            t = profile["time"]
            for s, value, error in zip(
                coordinates, profile["concentration"][0], uncertainty["concentration"][0]
            ):
                expected = 1 + 0.25 * math.cos(math.pi * s / 2) * math.exp(-rate * t)
                self.assertLessEqual(abs(value - expected), error + 3 * math.ulp(expected))
            # Independently differentiate the closed form at interior locations.
            for s in (0.25, 0.75, 1.5):
                temporal = -0.25 * rate * math.cos(math.pi * s / 2) * math.exp(-rate * t)
                spatial = (
                    -0.25 * (math.pi / 2) ** 2 * math.cos(math.pi * s / 2) * math.exp(-rate * t)
                )
                self.assertAlmostEqual(2 * temporal, 0.2 * spatial, places=15)
            for s in (0, 2):
                self.assertAlmostEqual(
                    -0.25 * math.pi / 2 * math.sin(math.pi * s / 2), 0, places=15
                )

    def test_exact_inventory_and_zero_terms(self):
        reference = evaluate("diffusion_cosine")
        for row, uncertainty in zip(reference["continuum"], reference["uncertainty"]["continuum"]):
            self.assertLessEqual(abs(row["total"][0] - 1.72), uncertainty["total"][0])
            self.assertEqual(row["endpoint_flux"], [[0, 0]])
            self.assertEqual(row["boundary_cumulative"], [[0, 0]])
            self.assertEqual(row["loss_cumulative"], [0])
            self.assertEqual(row["production_cumulative"], [0])


class ScalarTests(unittest.TestCase):
    def test_independently_recomputed_root_and_frozen_parameter(self):
        root = analytical.scalar_root(2, 1, 1)
        self.assertLess(root.high - root.low, D("1e-44"))
        known = D("0.65327118709440310111436")
        # The quoted short decimal need not lie in the much tighter root interval.
        self.assertLess(abs(root.low - known), D("1e-22"))
        self.assertLess(root.low, D(str(CASES["scalar_robin"]["initial"]["root"])))
        sine, cosine = analytical.sincos(root)
        self.assertTrue((root * sine - cosine / 2).contains(0))

    def test_transfer_pde_robin_neumann_and_zero_limit(self):
        z, velocity, dispersion, length = 0.3, 1.0, 1.0, 2.0
        root = math.sqrt(velocity**2 + 4 * dispersion * z)
        plus, minus = (velocity + root) / (2 * dispersion), -2 * z / (velocity + root)
        ratio = minus / plus
        amplitude = velocity / (
            dispersion * plus * (1 - ratio**2 * math.exp((minus - plus) * length))
        )

        def derivatives(s):
            first, second = math.exp(minus * s), ratio * math.exp(
                minus * length + plus * (s - length)
            )
            return [amplitude * (minus**n * first - plus**n * second) for n in range(3)]

        for s in (0, 0.25, 1, 2):
            h, derivative, second = derivatives(s)
            evaluated, _ = analytical.steady_transfer(D(".3"), D(str(s)), 2, 1, 1)
            value, error = evaluated.serialized()
            self.assertLessEqual(abs(h - value), error + 5 * math.ulp(h))
            self.assertAlmostEqual(
                z * h - dispersion * second + velocity * derivative, 0, places=15
            )
        inlet, derivative, _ = derivatives(0)
        self.assertAlmostEqual(velocity * inlet - dispersion * derivative, velocity, places=15)
        self.assertAlmostEqual(derivatives(2)[1], 0, places=15)
        zero, integral = analytical.steady_transfer(0, 1, 2, 1, 1)
        self.assertEqual((zero.low, integral.low), (1, 2))

    def test_scalar_mode_pde_and_boundary_residuals(self):
        root = float(analytical.scalar_root(2, 1, 1).low)
        alpha, retardation, decay = 0.5, 1.5, 0.2
        beta = (root**2 + alpha**2) / retardation + decay
        for s in (0, 0.3, 1, 2):
            y = s - 1
            mode = math.exp(alpha * y) * math.cos(root * y)
            derivative = math.exp(alpha * y) * (
                alpha * math.cos(root * y) - root * math.sin(root * y)
            )
            second = math.exp(alpha * y) * (
                (alpha**2 - root**2) * math.cos(root * y) - 2 * alpha * root * math.sin(root * y)
            )
            self.assertAlmostEqual(
                -retardation * beta * mode,
                second - derivative - decay * retardation * mode,
                places=14,
            )
            if s == 0:
                self.assertAlmostEqual(mode - derivative, 0, places=14)
            if s == 2:
                self.assertAlmostEqual(derivative, 0, places=14)

    def test_scalar_spatial_and_time_integrals_independently(self):
        case, mesh, analysis, times = configuration("scalar_robin")
        reference = analytical.component_reference(case, mesh, analysis, times)
        # Independent double-precision expression evaluated with Simpson rules.
        k = float(analytical.scalar_root(2, 1, 1).low)
        beta = (k * k + 0.25) / 1.5 + 0.2

        def concentration(s, t):
            d = math.sqrt(2.2)
            rp, rm = (1 + d) / 2, (1 - d) / 2
            ratio = rm / rp
            a = 1 / (rp * (1 - ratio * ratio * math.exp((rm - rp) * 2)))
            h = a * (math.exp(rm * s) - ratio * math.exp(2 * rm + rp * (s - 2)))
            return h + 0.1 * math.exp(0.5 * (s - 1)) * math.cos(k * (s - 1)) * math.exp(-beta * t)

        row = reference["continuum"][-1]
        for profile, uncertainty in zip(
            reference["profiles"], reference["uncertainty"]["profiles"]
        ):
            for s, actual, error in zip(
                mesh, profile["concentration"][0], uncertainty["concentration"][0]
            ):
                expected = concentration(s, profile["time"])
                self.assertLessEqual(abs(actual - expected), error + 8 * math.ulp(expected))
        adapter_initial = metrics._initial_arrays(case, len(mesh))[0]
        for value, adapter, error in zip(
            reference["profiles"][0]["concentration"][0],
            adapter_initial,
            reference["uncertainty"]["profiles"][0]["concentration"][0],
        ):
            self.assertLessEqual(abs(value - adapter), error + 8 * math.ulp(adapter))
        mass = 0.645 * simpson(lambda s: concentration(s, 1), 0, 2)
        loss = (
            0.2
            * 0.645
            * simpson(lambda t: simpson(lambda s: concentration(s, t), 0, 2, 40), 0, 1, 40)
        )
        outlet = -0.43 * simpson(lambda t: concentration(2, t), 0, 1)
        self.assertAlmostEqual(row["total"][0], mass, delta=2e-12)
        self.assertAlmostEqual(row["loss_cumulative"][0], loss, delta=2e-9)
        self.assertAlmostEqual(row["boundary_cumulative"][0][1], outlet, delta=2e-12)
        initial = reference["continuum"][0]["total"][0]
        self.assertAlmostEqual(
            row["total"][0]
            - initial
            - sum(row["boundary_cumulative"][0])
            + row["loss_cumulative"][0],
            0,
            places=14,
        )

    def test_root_input_discrepancy_is_propagated(self):
        case, mesh, analysis, times = configuration("scalar_robin")
        reference = analytical.component_reference(case, mesh, analysis, times)
        altered = copy.deepcopy(case)
        altered["initial"]["root"] += 1e-5
        altered["initial"]["root_bracket"] = [
            altered["initial"]["root"] - 1e-8,
            altered["initial"]["root"] + 1e-8,
        ]
        changed = analytical.component_reference(altered, mesh, analysis, times)
        self.assertEqual(analytical.reference_admission(altered, changed)["status"], "unmet")
        for row, widened in zip(
            reference["uncertainty"]["continuum"], changed["uncertainty"]["continuum"]
        ):
            self.assertGreater(widened["total"][0], row["total"][0])
            if row["time"] > 0:
                self.assertGreater(widened["loss_cumulative"][0], row["loss_cumulative"][0])
                self.assertGreater(
                    widened["boundary_cumulative"][0][1], row["boundary_cumulative"][0][1]
                )


class ContractTests(unittest.TestCase):
    def test_every_component_meets_frozen_budgets_and_wrapper(self):
        for case_id in (
            "reaction_distinct",
            "reaction_equal",
            "reaction_zero",
            "diffusion_cosine",
            "scalar_robin",
        ):
            case, mesh, analysis, times = configuration(case_id)
            reference = analytical.component_reference(case, mesh, analysis, times)
            self.assertEqual(
                set(reference),
                {
                    "schema_version",
                    "coordinates",
                    "species",
                    "profiles",
                    "continuum",
                    "uncertainty",
                },
            )
            admission = analytical.reference_admission(case, reference)
            self.assertEqual(admission["status"], "admitted", case_id)
            self.assertEqual([r["time"] for r in reference["profiles"]], [0, *analysis])
            self.assertEqual([r["time"] for r in reference["continuum"]], times)
            self.assertEqual(reference["coordinates"], mesh)

    def test_metrics_rejects_species_order_and_missing_coverage(self):
        case, mesh, analysis, times = configuration("reaction_distinct")
        reference = analytical.component_reference(case, mesh, analysis, times)

        class Observation(dict):
            pass

        observation = Observation(
            coordinates=mesh, species=reference["species"], profiles=[{"time": t} for t in times]
        )
        observation._context = {"launch": {"analysis_times": analysis}}
        metrics._reference(reference, observation)
        for mutation in ("species", "profiles", "continuum", "coordinates"):
            bad = copy.deepcopy(reference)
            if mutation == "species":
                bad[mutation].reverse()
            else:
                bad[mutation].pop()
            with self.assertRaises(ValueError, msg=mutation):
                metrics._reference(bad, observation)

    def test_wrong_inward_boundary_sign_and_species_fail_admission(self):
        case = CASES["scalar_robin"]
        reference = evaluate("scalar_robin")
        for field in analytical.PAIR_FIELDS:
            bad = copy.deepcopy(reference)
            bad["continuum"][-1][field][0][1] *= -1
            with self.assertRaisesRegex(ValueError, "boundary sign"):
                analytical.reference_admission(case, bad)
        bad = copy.deepcopy(reference)
        bad["species"] = ["wrong"]
        with self.assertRaisesRegex(ValueError, "species mismatch"):
            analytical.reference_admission(case, bad)

    def test_reference_uncertainty_is_not_a_solver_tolerance(self):
        case = copy.deepcopy(CASES["reaction_distinct"])
        reference = evaluate("reaction_distinct")
        case["analytical"]["budget"]["concentration_absolute"] = 0
        decision = analytical.reference_admission(case, reference)
        self.assertEqual(decision["status"], "unmet")
        self.assertTrue(any(row["status"] == "unmet" for row in decision["observables"]))

    def test_invalid_inputs_and_unavailable_full_chain(self):
        case, mesh, analysis, times = configuration("diffusion_cosine")
        for args in (
            (case, mesh[::-1], analysis, times),
            (case, mesh, [0, 1], times),
            (case, mesh, analysis, times[1:]),
            (case, [0, float("nan"), 2], analysis, times),
        ):
            with self.assertRaises(ValueError):
                analytical.component_reference(*args)
        unsupported = copy.deepcopy(case)
        unsupported["boundary"]["outlet"] = "wrong"
        with self.assertRaisesRegex(ValueError, "boundary law"):
            analytical.component_reference(unsupported, mesh, analysis, times)
        with self.assertRaisesRegex(NotImplementedError, "finite-chain"):
            analytical.component_reference(*configuration("example3"))


if __name__ == "__main__":
    unittest.main()

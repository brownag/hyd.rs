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


class FiniteKernelTests(unittest.TestCase):
    def test_jet_derivatives_and_zero_limit(self):
        with localcontext() as context:
            context.prec = 100
            x = analytical._Jet(2, 1)
            self.assertEqual((x * x).c, (D(4), D(4), D(1)))
            self.assertEqual((1 / x).c, (D(".5"), D("-.25"), D(".125")))
            self.assertEqual(x.exp().c, (D(2).exp(), D(2).exp(), D(2).exp() / 2))
            self.assertLess(abs(x.sqrt().c[1] - 1 / (2 * D(2).sqrt())), D("1e-98"))
            for s in (D(0), D(1), D(2)):
                h, integral = analytical._finite_transfer(D(0), s, D(1), D(".5"), D(2), jets=True)
                first = D(".5") * ((2 * (s - 2)).exp() - 1) - s
                integrated_first = D(".25") * (1 - D(-4).exp()) - 2 - 1
                self.assertLess(abs(h.c[0] - 1), D("1e-98"))
                self.assertLess(abs(h.c[1] - first), D("1e-98"))
                self.assertLess(abs(integral.c[0] - 2), D("1e-98"))
                self.assertLess(abs(integral.c[1] - integrated_first), D("1e-98"))
                self.assertNotEqual(h.c[2], 0)
                self.assertNotEqual(integral.c[2], 0)

    def test_transfer_derivatives_and_integral_identity(self):
        with localcontext() as context:
            context.prec = 100
            z, s, v, d, length = map(D, (".3", ".7", "1", ".5", "2"))
            h, integral = analytical._finite_transfer(z, s, v, d, length, jets=True)
            eps = D("1e-20")
            below = analytical._finite_transfer(z - eps, s, v, d, length)
            above = analytical._finite_transfer(z + eps, s, v, d, length)
            for jet, a, b in zip((h, integral), below, above):
                self.assertLess(abs((b - a) / (2 * eps) - jet.c[1]), D("1e-37"))
                self.assertLess(
                    abs((a + b - 2 * jet.c[0]) / (2 * eps * eps) - jet.c[2]), D("1e-37")
                )
            outlet, total = analytical._finite_transfer(z, length, v, d, length)
            self.assertLess(abs(z * total - v * (1 - outlet)), D("1e-97"))
            # Differencing in space is independent of the differentiated z jets.
            center = h.c[0]
            left = analytical._finite_transfer(z, s - eps, v, d, length)[0]
            right = analytical._finite_transfer(z, s + eps, v, d, length)[0]
            residual = (
                z * center
                - d * (left + right - 2 * center) / eps**2
                + v * (right - left) / (2 * eps)
            )
            self.assertLess(abs(residual), D("1e-37"))

    def test_confluent_and_permuted_divided_differences(self):
        import itertools

        with localcontext() as context:
            context.prec = 100

            def derivative(z, order):
                return (-1) ** order / (1 + z) ** (order + 1)

            for nodes in ([D(0)] * 3, [D(0), D(0), D(2)], [D(1), D(1), D(1)], [D(1), D(2), D(3)]):
                expected = 1 / math.prod(1 + z for z in nodes)
                for permutation in itertools.permutations(nodes):
                    actual, error, condition = analytical._divided_difference(
                        permutation, [1 / (1 + z) for z in permutation], derivative
                    )
                    self.assertLessEqual(abs(actual - expected), error)
                    self.assertGreater(error, 0)
                    self.assertGreaterEqual(condition, 1)

    def test_unequal_retardation_confluent_and_near_confluent(self):
        case = CASES["example3"]
        with localcontext() as context:
            context.prec = 400
            for p in (D(".09"), D(".09") + D("1e-80")):
                values, errors, conditioning, guarded = analytical._transform_values(
                    case, p, [D(10), D(250)]
                )
                self.assertEqual(guarded, p != D(".09"))
                self.assertGreater(conditioning, 1)
                self.assertTrue(all(e > 0 for row in errors for e in row))
                # Integrate each transformed PDE, independently of closure construction.
                theta, storage, _, rates, _, _, _ = analytical._chain_parameters(case)
                masses = [a * c for a, c in zip(storage, values[-1])]
                for j in range(3):
                    inlet = D(".43") / p if j == 0 else D(0)
                    production = rates[j - 1] * masses[j - 1] if j else D(0)
                    residual = (
                        (p + rates[j]) * masses[j] - production - inlet + D(".43") * values[1][j]
                    )
                    self.assertLess(abs(residual), D("1e-200"))

    def test_zero_transfer_removes_daughters_exactly(self):
        case = copy.deepcopy(CASES["example3"])
        case["species"][0]["rate"] = 0
        with localcontext() as context:
            context.prec = 400
            values, errors, _, _ = analytical._transform_values(case, D(".1"), [D(0), D(250)])
        self.assertTrue(all(row[1:] == [0, 0] for row in values))
        self.assertTrue(all(row[1:] == [0, 0] for row in errors))

    def test_inversion_weights_known_transforms_and_limits(self):
        self.assertEqual(analytical._stehfest_weights(2, 400), (D(2), D(-2)))
        constant = analytical._invert_transform(lambda p: 1 / p, 2, 64, 400)
        exponential = analytical._invert_transform(lambda p: 1 / (p + D(".2")), 2, 64, 400)
        self.assertLess(abs(constant - 1), D("1e-300"))
        self.assertLess(abs(exponential - D("-.4").exp()), D("1e-24"))
        for order, digits in ((3, 400), (514, 650), (64, 800)):
            with self.assertRaises(ValueError):
                analytical._stehfest_weights(order, digits)


def synthetic_chain_order(
    case,
    coordinates,
    analysis_times,
    diagnostic_times,
    order,
    digits,
    profile_indices=None,
    direct=False,
):
    values, errors = {}, {}
    offset = {192: D("1e-4"), 256: D("1e-6"), 384: D("1e-10"), 512: D("1e-12")}[order]
    for group, times, fields in (
        ("profiles", [0, *analysis_times], ["concentration"]),
        ("continuum", diagnostic_times, analytical.FIELDS),
    ):
        for time in times:
            for field in fields:
                if group == "profiles":
                    indices = (
                        range(len(coordinates))
                        if profile_indices is None
                        else profile_indices[time]
                    )
                else:
                    indices = range(2) if field in analytical.PAIR_FIELDS else [-1]
                for j in range(len(case["species"])):
                    for index in indices:
                        key = (group, time, field, j, index)
                        if field in analytical.PAIR_FIELDS and index == 0:
                            value = analytical._decimal(case["q"]) if j == 0 else D(0)
                            if field == "boundary_cumulative":
                                value *= analytical._decimal(time)
                        elif analytical._known_exact(case, key):
                            value = D(0)
                        else:
                            value = 1 + offset
                            if field in analytical.PAIR_FIELDS:
                                value = -value
                        values[key], errors[key] = value, D(0)
    return {
        "values": values,
        "arithmetic": errors,
        "recurrence": errors,
        "order": order,
        "digits": digits,
        "maximum_conditioning": "1",
        "guarded_samples": 0,
    }


class FiniteEvaluationTests(unittest.TestCase):
    def test_recurrence_and_independent_cumulative_channels(self):
        case = CASES["example3"]
        with localcontext() as context:
            context.prec = 400
            positions = [D(25) * i for i in range(11)]
            recurrent, _, _, _ = analytical._transform_values(
                case, D(".1"), positions, recurrence=True
            )
            direct, _, _, _ = analytical._transform_values(case, D(".1"), positions)
            self.assertLess(
                max(
                    abs(a - b) for row, other in zip(recurrent, direct) for a, b in zip(row, other)
                ),
                D("1e-395"),
            )
        run = analytical._chain_order(case, [0, 125, 250], [50], [0, 50], 32, 400)
        self.assertEqual(run["values"][("continuum", 50, "boundary_cumulative", 0, 0)], D("21.50"))
        for j in range(3):

            def value(field, side=-1):
                return run["values"][("continuum", 50, field, j, side)]

            residual = (
                value("total")
                - value("boundary_cumulative", 0)
                - value("boundary_cumulative", 1)
                - value("production_cumulative")
                + value("loss_cumulative")
            )
            self.assertLess(abs(residual), D("1e-12"))

    def test_transform_chain_pde_and_endpoint_derivatives(self):
        with localcontext() as context:
            context.prec = 100
            case, p, x, eps = CASES["example3"], D(".2"), D(20), D("1e-15")
            rows = analytical._transform_values(case, p, [x - eps, x, x + eps])[0]
            theta, storage, _, rates, velocity, dispersion, length = analytical._chain_parameters(
                case
            )
            for j in range(3):
                first = (rows[2][j] - rows[0][j]) / (2 * eps)
                second = (rows[2][j] + rows[0][j] - 2 * rows[1][j]) / eps**2
                production = rates[j - 1] * storage[j - 1] * rows[1][j - 1] if j else 0
                residual = (
                    storage[j] * (p + rates[j]) * rows[1][j]
                    - theta * dispersion * second
                    + D(".43") * first
                    - production
                )
                self.assertLess(abs(residual), D("1e-27"))
            for position in (D(0), length):
                rows = analytical._transform_values(
                    case, p, [position - eps, position, position + eps]
                )[0]
                for j in range(3):
                    derivative = (rows[2][j] - rows[0][j]) / (2 * eps)
                    residual = (
                        velocity * rows[1][j]
                        - dispersion * derivative
                        - (velocity / p if j == 0 else 0)
                        if position == 0
                        else derivative
                    )
                    self.assertLess(abs(residual), D("1e-26"))

    def test_order_diagnostics_reject_plateau_and_nonmonotonicity(self):
        for values in ([D(1), D(1), D(1)], [D(1), D(2), D(3)], [D(1), D(2), D(4)]):
            self.assertFalse(analytical._difference_diagnostic(values)[2])
        self.assertTrue(analytical._difference_diagnostic([D(1)] * 3, exact=True)[2])
        self.assertTrue(analytical._difference_diagnostic([D("1e-3"), D("-1e-5"), D("-1e-7")])[2])

    def test_full_coverage_and_bounded_escalation_with_synthetic_values(self):
        from unittest.mock import patch

        case, mesh, analysis, times = configuration("example3")
        with patch.object(analytical, "_chain_order", side_effect=synthetic_chain_order) as calls:
            reference, diagnostics = analytical.finite_chain_reference(
                case, mesh, analysis, times, allow_escalation=True
            )
        self.assertEqual(diagnostics["orders"], [192, 256, 384, 512])
        self.assertEqual(diagnostics["precision_comparison_digits"], [650, 400])
        self.assertEqual(calls.call_count, 6)  # Four full orders; precision and input subsets.
        self.assertEqual(len(reference["continuum"]), 201)
        self.assertEqual(len(reference["profiles"]), 4)
        self.assertEqual(len(reference["profiles"][0]["concentration"][0]), 1001)
        self.assertEqual(
            analytical.reference_admission(case, reference, diagnostics)["status"], "admitted"
        )
        for mutation in ("missing_observable", "missing_time", "precision", "hash"):
            bad = copy.deepcopy(diagnostics)
            if mutation == "missing_observable":
                bad["observables"].pop()
            elif mutation == "missing_time":
                bad["diagnostic_times"].pop()
            elif mutation == "precision":
                bad["precision_comparison_digits"] = [650, 650]
            else:
                bad["reference_sha256"] = "0" * 64
            with self.assertRaises(ValueError, msg=mutation):
                analytical.reference_admission(case, reference, bad)
        for mutation in (
            "uncertainty",
            "budget",
            "exact_identity",
            "decreasing",
            "last_order_difference",
            "arithmetic_estimate",
            "status",
            "summary",
        ):
            bad = copy.deepcopy(diagnostics)
            row = next(
                r for r in bad["observables"] if r["group"] == "profiles" and r["time"] == 50
            )
            if mutation in ("uncertainty", "budget"):
                row[mutation] *= 2
            elif mutation in ("exact_identity", "decreasing"):
                row[mutation] = not row[mutation]
            elif mutation == "last_order_difference":
                row[mutation] = "1"
            elif mutation == "arithmetic_estimate":
                row[mutation] = "-1"
            elif mutation == "status":
                row[mutation] = "unmet"
            else:
                bad["status"] = "unmet"
            with self.assertRaises(ValueError, msg=mutation):
                analytical.reference_admission(case, reference, bad)

    def test_no_implicit_escalation_and_unmet_budgets(self):
        from unittest.mock import patch

        case, mesh, analysis, times = configuration("example3")
        mesh = [0, 125, 250]
        with patch.object(analytical, "_chain_order", side_effect=synthetic_chain_order) as calls:
            reference, diagnostics = analytical.finite_chain_reference(case, mesh, analysis, times)
        self.assertEqual(diagnostics["orders"], [192, 256, 384])
        self.assertEqual(calls.call_count, 5)
        self.assertEqual(
            analytical.reference_admission(case, reference, diagnostics)["status"], "unmet"
        )
        with self.assertRaises((ValueError, NotImplementedError)):
            analytical.reference_admission(case, reference)
        case["analytical"]["settings"]["maximum_order"] = 640
        with self.assertRaisesRegex(ValueError, "settings"):
            analytical.finite_chain_reference(case, mesh, analysis, times)

    def test_mesh_validation_and_coordinate_deviation(self):
        analytical._checked_uniform_mesh([0, 250 / 3, 500 / 3, 250], 250)
        with self.assertRaisesRegex(ValueError, "uniform mesh"):
            analytical._checked_uniform_mesh([0, 100, 250], 250)

    def test_negative_near_zero_estimate_is_not_clipped(self):
        from unittest.mock import patch

        def small_negative(*args, **kwargs):
            run = synthetic_chain_order(*args, **kwargs)
            for key in run["values"]:
                if key[0] == "profiles" and key[1] > 0:
                    run["values"][key] = (
                        D("-9e-10")
                        + {192: D("1e-5"), 256: D("1e-7"), 384: D("1e-10"), 512: D("1e-12")}[
                            run["order"]
                        ]
                    )
            return run

        case = copy.deepcopy(CASES["example3"])
        with patch.object(analytical, "_chain_order", side_effect=small_negative):
            reference, diagnostics = analytical.finite_chain_reference(
                case, [0, 125, 250], [50, 100, 200], [0, 50, 100, 200], allow_escalation=True
            )
        self.assertLess(reference["profiles"][-1]["concentration"][0][0], 0)
        self.assertGreater(
            reference["uncertainty"]["profiles"][-1]["concentration"][0][0],
            abs(reference["profiles"][-1]["concentration"][0][0]),
        )


class StageExecutorTests(unittest.TestCase):
    def test_hook_and_default_reproduce_all_arguments_and_results(self):
        from unittest.mock import patch

        case = copy.deepcopy(CASES["example3"])
        arguments = (case, [0, 125, 250], [50], [0, 50, 200])
        for escalation in (False, True):
            with self.subTest(escalation=escalation):
                with patch.object(
                    analytical, "_chain_order", side_effect=synthetic_chain_order
                ) as calls:
                    expected = analytical.finite_chain_reference(
                        *arguments, allow_escalation=escalation
                    )
                    expected_calls = copy.deepcopy(
                        [(c.args, c.kwargs) for c in calls.call_args_list]
                    )
                variants = []

                def execute(request, calculate):
                    variants.append(request["variant"])
                    # Request mutation cannot alter the callback's original arguments.
                    request["case"]["theta"] = -1
                    request["coordinates"].clear()
                    return calculate()

                with patch.object(
                    analytical, "_chain_order", side_effect=synthetic_chain_order
                ) as calls:
                    actual = analytical.finite_chain_reference(
                        *arguments, allow_escalation=escalation, stage_executor=execute
                    )
                    self.assertEqual(
                        [(c.args, c.kwargs) for c in calls.call_args_list], expected_calls
                    )
                self.assertEqual(actual, expected)
                self.assertEqual(
                    variants,
                    ["full"] * 3
                    + (["escalation"] if escalation else [])
                    + ["precision", "binary_inputs"],
                )

    def test_rejects_executor_metadata_coverage_and_value_corruption(self):
        from unittest.mock import patch

        mutations = {
            "order": lambda r: r.update(order=512),
            "digits": lambda r: r.update(digits=20),
            "missing": lambda r: r["values"].pop(next(iter(r["values"]))),
            "extra": lambda r: r["recurrence"].update({("bad", 0, "x", 0, 0): D(0)}),
            "nonfinite": lambda r: r["values"].update({next(iter(r["values"])): D("NaN")}),
            "float": lambda r: r["values"].update({next(iter(r["values"])): 0.0}),
            "negative": lambda r: r["arithmetic"].update({next(iter(r["arithmetic"])): D(-1)}),
            "conditioning": lambda r: r.update(maximum_conditioning="invalid"),
            "guarded": lambda r: r.update(guarded_samples=-1),
        }
        for name, mutate in mutations.items():
            with self.subTest(mutation=name):

                def execute(request, calculate):
                    result = copy.deepcopy(calculate())
                    mutate(result)
                    return result

                with patch.object(analytical, "_chain_order", side_effect=synthetic_chain_order):
                    with self.assertRaises(ValueError):
                        analytical.finite_chain_reference(
                            CASES["example3"],
                            [0, 125, 250],
                            [50],
                            [0, 50, 200],
                            stage_executor=execute,
                        )


if __name__ == "__main__":
    unittest.main()

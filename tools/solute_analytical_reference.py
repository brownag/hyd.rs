"""Independent component references with outward-rounded Decimal enclosures.

``component_reference(case, coordinates, analysis_times, diagnostic_times)``
returns exactly solute-metrics-reference-v1. Analysis times exclude zero;
diagnostic times include zero. ``reference_admission(case, reference)`` returns
separate per-observable budget decisions. Callers validate the casebook with
check_solute_verification_spec.validate before selecting a case or launch.

Reaction truncation uses a Poisson-tail bound. Trigonometric Taylor remainders,
Decimal arithmetic, the scalar root interval, decimal/binary input conversion,
and final binary serialization are enclosed. These are component formula bounds,
not solver thresholds. No solver matrices, outputs, quadrature closure, or third
party numerical packages enter the calculation. Finite-chain requests are
explicitly unavailable pending their separately reviewed implementation.
"""

import hashlib
import json
import math
from decimal import Decimal, ROUND_CEILING, ROUND_FLOOR, getcontext, localcontext
from functools import lru_cache

PRECISION = 70
FIELDS = (
    "dissolved",
    "equilibrium_sorbed",
    "total",
    "endpoint_flux",
    "boundary_cumulative",
    "production_cumulative",
    "loss_cumulative",
)
PAIR_FIELDS = {"endpoint_flux", "boundary_cumulative"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def _decimal(value):
    result = value if isinstance(value, Decimal) else Decimal(str(value))
    require(result.is_finite(), "nonfinite Decimal input")
    return result


def _rounded(operation, upward=False):
    with localcontext() as context:
        context.prec = PRECISION
        context.rounding = ROUND_CEILING if upward else ROUND_FLOOR
        return operation()


class Interval:
    """Closed Decimal interval; all arithmetic rounds outwards at 70 digits."""

    def __init__(self, low=0, high=None):
        self.low = _decimal(low)
        self.high = self.low if high is None else _decimal(high)
        require(self.low <= self.high, "reversed interval")

    def __add__(self, other):
        other = interval(other)
        return Interval(
            _rounded(lambda: self.low + other.low),
            _rounded(lambda: self.high + other.high, True),
        )

    __radd__ = __add__

    def __neg__(self):
        return Interval(self.high.copy_negate(), self.low.copy_negate())

    def __sub__(self, other):
        return self + -interval(other)

    def __rsub__(self, other):
        return interval(other) + -self

    def __mul__(self, other):
        other = interval(other)
        pairs = [(a, b) for a in (self.low, self.high) for b in (other.low, other.high)]
        return Interval(
            min(_rounded(lambda: a * b) for a, b in pairs),
            max(_rounded(lambda: a * b, True) for a, b in pairs),
        )

    __rmul__ = __mul__

    def __truediv__(self, other):
        other = interval(other)
        require(other.low > 0 or other.high < 0, "interval division includes zero")
        reciprocal = Interval(
            _rounded(lambda: Decimal(1) / other.high),
            _rounded(lambda: Decimal(1) / other.low, True),
        )
        return self * reciprocal

    def __rtruediv__(self, other):
        return interval(other) / self

    def __pow__(self, exponent):
        require(type(exponent) is int and exponent >= 0, "nonnegative integer power required")
        result = Interval(1)
        for _ in range(exponent):
            result = result * self
        return result

    def exp(self):
        # Decimal.exp/sqrt use correctly rounded nearest, regardless of context
        # rounding. Adjacent representable numbers enclose their exact values.
        with localcontext() as context:
            context.prec = PRECISION
            return Interval(self.low.exp().next_minus(), self.high.exp().next_plus())

    def sqrt(self):
        require(self.low >= 0, "negative square root")
        with localcontext() as context:
            context.prec = PRECISION
            return Interval(
                max(Decimal(0), self.low.sqrt().next_minus()),
                self.high.sqrt().next_plus(),
            )

    def magnitude(self):
        return max(self.low.copy_abs(), self.high.copy_abs())

    def widened(self, radius):
        radius = interval(radius).high
        require(radius >= 0, "negative uncertainty")
        return self + Interval(radius.copy_negate(), radius)

    def contains(self, value):
        value = _decimal(value)
        return self.low <= value <= self.high

    def serialized(self):
        """Return binary64 midpoint and upward binary64 absolute enclosure."""
        midpoint = _rounded(lambda: (self.low + self.high) / 2)
        value = float(midpoint)
        require(math.isfinite(value), "reference exceeds binary64 output range")
        binary = Decimal.from_float(value)
        radius = max(
            _rounded(lambda: binary - self.low, True),
            _rounded(lambda: self.high - binary, True),
        )
        error = float(radius)
        require(math.isfinite(error), "uncertainty exceeds binary64 output range")
        if Decimal.from_float(error) < radius:
            error = math.nextafter(error, math.inf)
        return value, error


def interval(value):
    return value if isinstance(value, Interval) else Interval(value)


def input_interval(value):
    """Enclose both the authored decimal and the binary64 adapter parameter."""
    authored = _decimal(value)
    binary = Decimal.from_float(value) if type(value) is float else authored
    return Interval(min(authored, binary), max(authored, binary))


def sincos(value):
    """Taylor enclosures on [-4,4], with an explicit absolute next-term bound."""
    value = interval(value)
    require(value.magnitude() <= 4, "trigonometric argument outside component range")
    square = value * value
    sine, cosine = value, Interval(1)
    sin_term, cos_term = value, Interval(1)
    for k in range(1, 101):
        sin_term = -sin_term * square / ((2 * k) * (2 * k + 1))
        cos_term = -cos_term * square / ((2 * k - 1) * (2 * k))
        sine, cosine = sine + sin_term, cosine + cos_term
        # For every fixed x the terms now decrease; alternating-series
        # remainders are bounded by these next omitted terms.
        sin_tail = (sin_term * square / ((2 * k + 2) * (2 * k + 3))).magnitude()
        cos_tail = (cos_term * square / ((2 * k + 1) * (2 * k + 2))).magnitude()
        if max(sin_tail, cos_tail) < Decimal("1e-65"):
            return sine.widened(sin_tail), cosine.widened(cos_tail)
    raise ValueError("trigonometric series did not reach its bounded target")


@lru_cache(maxsize=1)
def pi_interval():
    """Machin's identity with independently bounded alternating atan series."""

    def atan_inverse(n):
        x = Interval(1) / n
        power, total = x, x
        for k in range(1, 110):
            power = -power * x * x
            total = total + power / (2 * k + 1)
        tail = (power * x * x / 221).magnitude()
        return total.widened(tail)

    return 16 * atan_inverse(5) - 4 * atan_inverse(239)


def reaction_masses(initial, rates, time, terms=64):
    """Stored mass densities and their independent time integrals, enclosed.

    Each sum contains k=0,...,terms. The substochastic uniformization matrix
    bounds both omitted sums by ||m0||_1 times the Poisson tail (times t for
    the integral). Integrated weights use (1-PoissonCDF(k))/nu, never closure.
    """
    require(len(initial) == len(rates) and 1 <= len(rates) <= 3, "reaction shape")
    initial, rates = list(map(interval, initial)), list(map(interval, rates))
    time = interval(time)
    require(time.low >= 0 and all(x.low >= 0 for x in initial + rates), "negative reaction input")
    require(type(terms) is int and terms >= 1, "invalid uniformization truncation")
    nu = max(x.high for x in rates)
    if not nu or not time.high:
        return initial, [x * time for x in initial], Interval(0)
    x = time * nu
    require(x.high < terms + 2, "Poisson tail requires terms+2 > nu*t")
    poisson = (-x).exp()
    cdf, power = Interval(0), initial
    masses = [Interval(0) for _ in rates]
    integrated = [Interval(0) for _ in rates]
    for k in range(terms + 1):
        cdf = cdf + poisson
        weight = (1 - cdf) / nu
        # Known positivity permits intersection when cancellation straddles zero.
        weight = Interval(max(Decimal(0), weight.low), max(Decimal(0), weight.high))
        masses = [m + poisson * p for m, p in zip(masses, power)]
        integrated = [m + weight * p for m, p in zip(integrated, power)]
        power = [
            (1 - rate / nu) * power[j] + (rates[j - 1] / nu * power[j - 1] if j else 0)
            for j, rate in enumerate(rates)
        ]
        poisson = poisson * x / (k + 1)
    tail = sum(initial, Interval(0)) * poisson / (1 - x / (terms + 2))
    return (
        [m.widened(tail.high) for m in masses],
        [m.widened((time * tail).high) for m in integrated],
        tail,
    )


def steady_transfer(z, position, length, velocity, dispersion):
    """Stable finite-domain scalar transfer H and spatial integral I."""
    z, position, length, velocity, dispersion = map(
        interval, (z, position, length, velocity, dispersion)
    )
    require(z.low >= 0 and velocity.low > 0 and dispersion.low > 0, "transfer parameters")
    if z.high == 0:
        return Interval(1), length
    require(z.low > 0, "transfer z interval straddles removable zero limit")
    root = (velocity * velocity + 4 * dispersion * z).sqrt()
    plus = (velocity + root) / (2 * dispersion)
    minus = -2 * z / (velocity + root)
    ratio = minus / plus
    amplitude = velocity / (
        dispersion * plus * (1 - ratio * ratio * ((minus - plus) * length).exp())
    )
    value = amplitude * (
        (minus * position).exp() - ratio * (minus * length + plus * (position - length)).exp()
    )
    integral = amplitude * (
        ((minus * length).exp() - 1) / minus
        - ratio * (minus * length).exp() * (1 - (-plus * length).exp()) / plus
    )
    return value, integral


def scalar_root(length, velocity, dispersion):
    """Isolate k*sin(kL/2)-alpha*cos(kL/2)=0 independently of the frozen IC."""
    length, velocity, dispersion = map(interval, (length, velocity, dispersion))
    require(length.low > 0 and velocity.low > 0 and dispersion.low > 0, "root parameters")
    alpha = velocity / (2 * dispersion)
    low, high = Decimal(0), (pi_interval() / length).high
    for _ in range(240):
        middle = _rounded(lambda: (low + high) / 2)
        sine, cosine = sincos(Interval(middle) * length / 2)
        residual = middle * sine - alpha * cosine
        if residual.high < 0:
            low = middle
        elif residual.low > 0:
            high = middle
        else:
            # Input intervals can limit sign resolution. The existing bracket
            # already encloses all roots, and must not be collapsed to midpoint.
            break
        if high - low < Decimal("1e-45"):
            break
    return Interval(low, high)


def _parameters(case):
    theta, rho = input_interval(case["theta"]), input_interval(case["rho"])
    kd = [input_interval(s["kd"]) for s in case["species"]]
    rates = [input_interval(s["rate"]) for s in case["species"]]
    storage = [theta + rho * value for value in kd]
    return theta, rho, kd, rates, storage


def _continuum(case, spatial, integrated, outlet, integrated_outlet, time):
    theta, rho, kd, rates, storage = _parameters(case)
    q = input_interval(case["q"])
    inlet = [q * input_interval(c) for c in case["boundary"]["inlet_concentration"]]
    return {
        "dissolved": [theta * x for x in spatial],
        "equilibrium_sorbed": [rho * k * x for k, x in zip(kd, spatial)],
        "total": [a * x for a, x in zip(storage, spatial)],
        "endpoint_flux": [[a, -q * b] for a, b in zip(inlet, outlet)],
        "boundary_cumulative": [[a * time, -q * b] for a, b in zip(inlet, integrated_outlet)],
        "production_cumulative": [Interval(0)]
        + [rates[j - 1] * storage[j - 1] * integrated[j - 1] for j in range(1, len(kd))],
        "loss_cumulative": [r * a * x for r, a, x in zip(rates, storage, integrated)],
    }


def _component(case):
    theta, _, _, rates, storage = _parameters(case)
    length = input_interval(case["length"])
    family = case["family"]
    if family == "reaction":
        initial = [a * input_interval(c) for a, c in zip(storage, case["initial"]["values"])]

        def evaluate(time, coordinates):
            masses, integrated, _ = reaction_masses(initial, rates, time)
            values = [m / a for m, a in zip(masses, storage)]
            integrals = [m / a for m, a in zip(integrated, storage)]
            return [[c for _ in coordinates] for c in values], _continuum(
                case,
                [length * c for c in values],
                [length * c for c in integrals],
                values,
                integrals,
                time,
            )

        return evaluate
    if family == "diffusion":
        mean, amplitude = map(
            input_interval, (case["initial"]["mean"], case["initial"]["amplitude"])
        )
        frequency = pi_interval() * case["initial"]["mode"] / length
        decay = input_interval(case["dispersion"]) * theta / storage[0] * frequency * frequency

        def evaluate(time, coordinates):
            values = [
                mean + amplitude * sincos(frequency * s)[1] * (-decay * time).exp()
                for s in coordinates
            ]
            return [values], _continuum(
                case, [mean * length], [mean * length * time], [Interval(0)], [Interval(0)], time
            )

        return evaluate
    velocity = input_interval(case["q"]) / theta
    dispersion = input_interval(case["dispersion"])
    alpha, retardation = velocity / (2 * dispersion), storage[0] / theta
    root = scalar_root(length, velocity, dispersion)
    frozen = [
        input_interval(x) for x in [case["initial"]["root"], *case["initial"]["root_bracket"]]
    ]
    frozen_interval = Interval(min(x.low for x in frozen), max(x.high for x in frozen))
    root_difference = (root - frozen_interval).magnitude()
    beta = (
        dispersion * root * root + velocity * velocity / (4 * dispersion)
    ) / retardation + rates[0]
    amplitude = input_interval(case["initial"]["amplitude"])
    inlet = input_interval(case["boundary"]["inlet_concentration"][0])
    # |d phi/dk| <= (L/2)*exp(alpha*L/2). The homogeneous Robin/Neumann
    # advection-diffusion semigroup is a sup-norm contraction with decay lambda:
    # a positive boundary maximum at the inlet contradicts C_s=(v/D)C.
    # This bounds propagation of the frozen, slightly incompatible IC; merely
    # substituting its root in beta would not solve the prescribed boundary law.
    initial_error = amplitude * length / 2 * (alpha * length / 2).exp() * root_difference

    def steady(s):
        value, integral = steady_transfer(retardation * rates[0], s, length, velocity, dispersion)
        return inlet * value, inlet * integral

    def mode(s):
        y = s - length / 2
        return (alpha * y).exp() * sincos(root * y)[1]

    def primitive(y):
        sine, cosine = sincos(root * y)
        return (alpha * y).exp() * (alpha * cosine + root * sine) / (alpha * alpha + root * root)

    mode_integral = primitive(length / 2) - primitive(-length / 2)
    end_steady, steady_integral = steady(length)
    end_mode = mode(length)

    def evaluate(time, coordinates):
        exponential = (-beta * time).exp()
        time_integral = (1 - exponential) / beta if time.high else Interval(0)
        error = initial_error * (-rates[0] * time).exp()
        cumulative_error = initial_error * time  # Conservative integral of the decaying bound.
        values = [
            (steady(s)[0] + amplitude * mode(s) * exponential).widened(error.high)
            for s in coordinates
        ]
        spatial = (steady_integral + amplitude * mode_integral * exponential).widened(
            (length * error).high
        )
        integrated = (steady_integral * time + amplitude * mode_integral * time_integral).widened(
            (length * cumulative_error).high
        )
        outlet = (end_steady + amplitude * end_mode * exponential).widened(error.high)
        integrated_outlet = (end_steady * time + amplitude * end_mode * time_integral).widened(
            cumulative_error.high
        )
        return [values], _continuum(
            case, [spatial], [integrated], [outlet], [integrated_outlet], time
        )

    return evaluate


def _validate_inputs(case, coordinates, analysis_times, diagnostic_times, allow_finite=False):
    if case.get("family") == "finite_chain" and not allow_finite:
        raise NotImplementedError("finite-chain reference unavailable pending task 13.3.3")
    require(case.get("status") == "executable", "blocked analytical case")
    require(
        case.get("family") in {"reaction", "diffusion", "scalar_robin", "finite_chain"},
        "unsupported family",
    )
    for field in ("length", "theta", "rho", "q", "dispersion", "time_end"):
        require(
            type(case[field]) in (int, float) and math.isfinite(case[field]), f"invalid {field}"
        )
        require(case[field] >= 0, f"negative {field}")
    require(case["length"] > 0 and case["theta"] > 0 and case["time_end"] > 0, "degenerate domain")
    species = case["species"]
    require(1 <= len(species) <= 3, "unsupported species count")
    require(len({s["name"] for s in species}) == len(species), "duplicate species")
    for s in species:
        for key in ("kd", "rate"):
            require(
                type(s[key]) in (int, float) and math.isfinite(s[key]) and s[key] >= 0,
                f"invalid species {key}",
            )
    boundary = case["boundary"]
    require(
        boundary["inlet"] == "total_flux" and boundary["outlet"] == "zero_gradient", "boundary law"
    )
    require(len(boundary["inlet_concentration"]) == len(species), "inlet species count")
    require(
        all(
            type(x) in (int, float) and math.isfinite(x) and x >= 0
            for x in boundary["inlet_concentration"]
        ),
        "invalid inlet",
    )
    require(len(coordinates) >= 3 and len(coordinates) <= 1001, "mesh size")
    for values, label in (
        (coordinates, "mesh"),
        (analysis_times, "analysis"),
        (diagnostic_times, "diagnostic"),
    ):
        require(values and all(type(x) in (int, float) and math.isfinite(x) for x in values), label)
        require(all(a < b for a, b in zip(values, values[1:])), f"unordered {label}")
    require(coordinates[0] == 0 and coordinates[-1] == case["length"], "mesh endpoints")
    require(analysis_times[0] > 0 and analysis_times[-1] <= case["time_end"], "analysis interval")
    require(
        diagnostic_times[0] == 0 and diagnostic_times[-1] == case["time_end"], "diagnostic interval"
    )
    require(set(analysis_times) <= set(diagnostic_times), "missing analysis diagnostic times")
    initial = case["initial"]
    family = case["family"]
    if family == "reaction":
        require(
            case["q"] == case["dispersion"] == 0 and initial["kind"] == "uniform",
            "reaction mapping",
        )
        require(len(initial["values"]) == len(species), "reaction initial species count")
        require(
            all(type(x) in (int, float) and math.isfinite(x) and x >= 0 for x in initial["values"]),
            "reaction initial values",
        )
    elif family == "diffusion":
        require(
            len(species) == 1
            and case["q"] == species[0]["rate"] == 0
            and case["dispersion"] > 0
            and initial["kind"] == "cosine",
            "diffusion mapping",
        )
        require(initial["mode"] == 1, "only the frozen fundamental diffusion mode is supported")
        require(0 <= initial["amplitude"] <= initial["mean"], "diffusion positivity")
    elif family == "finite_chain":
        require(case["q"] > 0 and case["dispersion"] > 0, "finite-chain transport")
        require(
            initial["kind"] == "uniform" and initial["values"] == [0] * len(species),
            "finite-chain reference requires zero initial data",
        )
        require(
            boundary["inlet_concentration"][1:] == [0] * (len(species) - 1),
            "finite-chain reference supports parent-only inlet",
        )
    else:
        require(
            len(species) == 1
            and case["q"] > 0
            and case["dispersion"] > 0
            and initial["kind"] == "steady_plus_mode",
            "scalar mapping",
        )
        bracket = initial["root_bracket"]
        require(
            len(bracket) == 2 and 0 < bracket[0] <= initial["root"] <= bracket[1],
            "scalar root bracket",
        )
        require(initial["amplitude"] >= 0, "scalar initial amplitude")


def component_reference(
    case: dict, coordinates: list, analysis_times: list, diagnostic_times: list
) -> dict:
    """Evaluate independently on the supplied mesh and exact requested schedules."""
    _validate_inputs(case, coordinates, analysis_times, diagnostic_times)
    evaluate = _component(case)
    reference = {
        "schema_version": "solute-metrics-reference-v1",
        "coordinates": list(coordinates),
        "species": [s["name"] for s in case["species"]],
        "profiles": [],
        "continuum": [],
        "uncertainty": {"profiles": [], "continuum": []},
    }
    mesh = list(map(input_interval, coordinates))

    def serialize(value):
        if isinstance(value, list):
            pairs = [serialize(x) for x in value]
            return [x[0] for x in pairs], [x[1] for x in pairs]
        return value.serialized()

    for time in diagnostic_times:
        profiles, continuum = evaluate(
            input_interval(time), mesh if time in [0, *analysis_times] else []
        )
        if time in [0, *analysis_times]:
            values, uncertainty = serialize(profiles)
            reference["profiles"].append({"time": time, "concentration": values})
            reference["uncertainty"]["profiles"].append(
                {"time": time, "concentration": uncertainty}
            )
        row, error = {"time": time}, {"time": time}
        for field in FIELDS:
            row[field], error[field] = serialize(continuum[field])
        reference["continuum"].append(row)
        reference["uncertainty"]["continuum"].append(error)
    return reference


def reference_admission(case: dict, reference: dict, diagnostics=None) -> dict:
    """Validate reference coverage/signs and report each frozen uncertainty budget.

    This checks reference uncertainty, never the discrepancy of a numerical
    solver. It cannot authenticate externally supplied numerical truth; callers
    retain the evaluator/source hashes alongside generated references.
    """
    from tools import solute_verification_metrics as metrics

    class Context(dict):
        pass

    require(
        isinstance(reference, dict)
        and isinstance(reference.get("profiles"), list)
        and reference["profiles"],
        "reference profiles missing",
    )
    analysis = [r["time"] for r in reference["profiles"][1:]]
    diagnostic_times = [r["time"] for r in reference["continuum"]]
    # Finite-chain diagnostics are checked by the separate entry point below;
    # its wrapper is intentionally identical to the component wrapper.
    _validate_inputs(
        case,
        reference["coordinates"],
        analysis,
        diagnostic_times,
        allow_finite=diagnostics is not None,
    )
    observation = Context(
        coordinates=reference["coordinates"],
        species=[s["name"] for s in case["species"]],
        profiles=[{"time": t} for t in diagnostic_times],
    )
    observation._context = {"launch": {"analysis_times": analysis}}
    metrics._reference(reference, observation)
    length = case["length"]
    storage = [case["theta"] + case["rho"] * s["kd"] for s in case["species"]]
    mass_scale = length * max(storage)
    flux_scale = max(case["q"], case["theta"] * case["dispersion"] / length)
    cumulative_scale = mass_scale + case["q"] * case["time_end"]
    budgets = case["analytical"]["budget"]
    limits = {"concentration": budgets["concentration_absolute"]}
    for field in FIELDS:
        if field == "endpoint_flux":
            limits[field] = budgets["instantaneous_flux_scaled"] * flux_scale
        elif field.endswith("cumulative"):
            limits[field] = budgets["cumulative_scaled"] * cumulative_scale
        else:
            limits[field] = budgets["inventory_scaled"] * mass_scale
    decisions = []
    for group in ("profiles", "continuum"):
        for row, uncertainty in zip(reference[group], reference["uncertainty"][group]):
            for field in ("concentration",) if group == "profiles" else FIELDS:
                for j, (value, error) in enumerate(zip(row[field], uncertainty[field])):
                    values = value if isinstance(value, list) else [value]
                    errors = error if isinstance(error, list) else [error]
                    if field in PAIR_FIELDS:
                        require(
                            values[0] >= -errors[0] and values[1] <= errors[1],
                            f"{field}: wrong inward boundary sign",
                        )
                        expected = case["q"] * case["boundary"]["inlet_concentration"][j]
                        if field == "boundary_cumulative":
                            expected *= row["time"]
                        # The analytical inlet is prescribed, not inferred from mass closure.
                        expected_error = abs(expected) * 4 * math.ulp(1.0)
                        require(
                            abs(values[0] - expected) <= errors[0] + expected_error,
                            f"{field}: inlet does not match prescribed law",
                        )
                    maximum = max(errors)
                    decisions.append(
                        {
                            "group": group,
                            "time": row["time"],
                            "field": field,
                            "species": reference["species"][j],
                            "maximum_uncertainty": maximum,
                            "budget": limits[field],
                            "status": "admitted" if maximum <= limits[field] else "unmet",
                        }
                    )
    result = {
        "status": "admitted" if all(d["status"] == "admitted" for d in decisions) else "unmet",
        "basis": "component interval arithmetic, analytical truncation and serialization bounds",
        "observables": decisions,
    }
    if case["family"] == "finite_chain":
        _check_chain_diagnostics(case, reference, diagnostics)
        result["basis"] = diagnostics["basis"]
        by_observable = {}
        for row in diagnostics["observables"]:
            key = tuple(row[k] for k in ("group", "time", "field", "species"))
            by_observable[key] = by_observable.get(key, True) and row["status"] == "admitted"
        for decision in decisions:
            key = tuple(decision[k] for k in ("group", "time", "field", "species"))
            if not by_observable.get(key, False):
                decision["status"] = "unmet"
        result["status"] = (
            "admitted" if all(r["status"] == "admitted" for r in decisions) else "unmet"
        )
    return result


# Explicit local contexts keep the finite-chain calculation separate from the
# component evaluator's 70-digit outward interval arithmetic.
class _Jet:
    """Taylor coefficients (f, f', f''/2) in the current Decimal context."""

    def __init__(self, value, first=0, second=0):
        self.c = tuple(map(_decimal, (value, first, second)))

    def __add__(self, other):
        other = _jet(other)
        return _Jet(*(a + b for a, b in zip(self.c, other.c)))

    __radd__ = __add__

    def __neg__(self):
        return _Jet(*(-x for x in self.c))

    def __sub__(self, other):
        return self + -_jet(other)

    def __rsub__(self, other):
        return _jet(other) - self

    def __mul__(self, other):
        other = _jet(other)
        return _Jet(*(sum(self.c[k] * other.c[n - k] for k in range(n + 1)) for n in range(3)))

    __rmul__ = __mul__

    def __truediv__(self, other):
        a, b, c = _jet(other).c
        require(a != 0, "singular jet division")
        return self * _Jet(1 / a, -b / (a * a), b * b / (a * a * a) - c / (a * a))

    def __rtruediv__(self, other):
        return _jet(other) / self

    def exp(self):
        a, b, c = self.c
        e = a.exp()
        return _Jet(e, e * b, e * (c + b * b / 2))

    def sqrt(self):
        a, b, c = self.c
        root = a.sqrt()
        return _Jet(root, b / (2 * root), c / (2 * root) - b * b / (8 * root**3))


def _jet(value):
    return value if isinstance(value, _Jet) else _Jet(value)


def _exprel(value):
    """(exp(x)-1)/x, retaining the nonzero Taylor derivatives at x=0."""
    is_jet = isinstance(value, _Jet)
    base = value.c[0] if is_jet else value
    if abs(base) >= Decimal(".5"):
        return (value.exp() - 1) / value
    term = total = _Jet(1) if is_jet else Decimal(1)
    target = Decimal(10) ** (-getcontext().prec - 5)
    for n in range(1, 2 * getcontext().prec + 20):
        term = term * value / (n + 1)
        total = total + term
        magnitude = max(map(abs, term.c)) if is_jet else abs(term)
        if n >= 3 and magnitude < target:
            return total
    raise ValueError("exprel series failed its finite work bound")


@lru_cache(maxsize=4096)
def _transfer_coefficients(z, velocity, dispersion, length, digits, jets=False):
    # Precision belongs to the key: 400-digit roots must not be reused at650.
    with localcontext() as context:
        context.prec = digits
        argument = _Jet(z, 1) if jets else z
        root = (velocity * velocity + 4 * dispersion * argument).sqrt()
        plus = (velocity + root) / (2 * dispersion)
        minus = -2 * argument / (velocity + root)
        ratio = minus / plus
        amplitude = velocity / (
            dispersion * plus * (1 - ratio * ratio * ((minus - plus) * length).exp())
        )
        return amplitude, minus, plus, ratio


def _finite_transfer(z, position, velocity, dispersion, length, jets=False):
    """Return H and I without replacing their z=0 jets by constants."""
    a, minus, plus, ratio = _transfer_coefficients(
        z, velocity, dispersion, length, getcontext().prec, jets
    )
    h = a * ((minus * position).exp() - ratio * (minus * length + plus * (position - length)).exp())
    spatial = a * (
        length * _exprel(minus * length)
        - ratio * (minus * length).exp() * length * _exprel(-plus * length)
    )
    return h, spatial


def _divided_difference(nodes, values, derivative):
    """Reversed difference with a visible empirical arithmetic error estimate.

    derivative(z, order) supplies the ordinary Taylor coefficient at repeated z.
    Near-coincident nodes are guarded at650 digits by the caller. Error seeds
    expose cancellation amplification; they are estimates, not interval bounds.
    """
    ordered = sorted(zip(nodes, values), key=lambda pair: pair[0])
    nodes, values = map(list, zip(*ordered))
    epsilon = Decimal(10) ** (12 - getcontext().prec)
    errors = [epsilon * max(abs(v), Decimal(1)) for v in values]
    conditioning = Decimal(1)
    for level in range(1, len(nodes)):
        output, uncertainty = [], []
        for i in range(len(values) - 1):
            gap = nodes[i + level] - nodes[i]
            if gap == 0:
                value = (-1) ** level * derivative(nodes[i], level)
                error = epsilon * max(abs(value), Decimal(1))
            else:
                numerator = values[i] - values[i + 1]
                value = numerator / gap
                error = (
                    errors[i]
                    + errors[i + 1]
                    + epsilon * abs(value) * (abs(nodes[i]) + abs(nodes[i + level]))
                ) / abs(gap)
                error += epsilon * max(abs(value), Decimal(1))
                if numerator:
                    conditioning = max(
                        conditioning, (abs(values[i]) + abs(values[i + 1])) / abs(numerator)
                    )
                elif values[i] or values[i + 1]:
                    conditioning = max(conditioning, 1 / epsilon)
            output.append(value)
            uncertainty.append(error)
        values, errors = output, uncertainty
    return values[0], errors[0], conditioning


def _chain_parameters(case):
    theta, rho = _decimal(case["theta"]), _decimal(case["rho"])
    storage = [theta + rho * _decimal(s["kd"]) for s in case["species"]]
    return (
        theta,
        storage,
        [a / theta for a in storage],
        [_decimal(s["rate"]) for s in case["species"]],
        _decimal(case["q"]) / theta,
        _decimal(case["dispersion"]),
        _decimal(case["length"]),
    )


def _transform_values(case, p, positions, recurrence=False):
    """Concentration and independent spatial-integral transforms for all species."""
    theta, storage, retardation, rates, velocity, dispersion, length = _chain_parameters(case)
    nodes = [r * (p + rate) for r, rate in zip(retardation, rates)]
    close = any(
        a != b and abs(a - b) < Decimal("1e-30") * max(abs(a), abs(b), Decimal(1))
        for j, a in enumerate(nodes)
        for b in nodes[j + 1 :]
    )
    if close and getcontext().prec < 650:
        with localcontext() as context:
            context.prec = 650
            return _transform_values(case, p, positions, recurrence)
    h_values, integrals = [], []
    for z in nodes:
        _, integral = _finite_transfer(z, length, velocity, dispersion, length)
        integrals.append(integral)
        if recurrence and len(positions) > 1:
            a, minus, plus, ratio = _transfer_coefficients(
                z, velocity, dispersion, length, getcontext().prec
            )
            step = length / (len(positions) - 1)
            left, right = [Decimal(1)], [(minus * length).exp()]
            left_ratio, right_ratio = (minus * step).exp(), (-plus * step).exp()
            for _ in range(len(positions) - 1):
                left.append(left[-1] * left_ratio)
                right.append(right[-1] * right_ratio)
            h_values.append([a * (u - ratio * v) for u, v in zip(left, reversed(right))])
        else:
            a, minus, plus, ratio = _transfer_coefficients(
                z, velocity, dispersion, length, getcontext().prec
            )
            h_values.append(
                [
                    a * ((minus * x).exp() - ratio * (minus * length + plus * (x - length)).exp())
                    for x in positions
                ]
            )
    output, errors = [], []
    maximum_condition = Decimal(1)
    for column in range(len(positions) + 1):
        values = integrals if column == len(positions) else [row[column] for row in h_values]
        position = length if column == len(positions) else positions[column]

        def derivative(z, order):
            h, mass = _finite_transfer(z, position, velocity, dispersion, length, jets=True)
            return (mass if column == len(positions) else h).c[order]

        factor = _decimal(case["boundary"]["inlet_concentration"][0]) / p
        result, error = [], []
        for j in range(len(nodes)):
            if factor == 0:
                value, uncertainty, conditioning = Decimal(0), Decimal(0), Decimal(1)
            else:
                value, uncertainty, conditioning = _divided_difference(
                    nodes[: j + 1], values[: j + 1], derivative
                )
            result.append(factor * value)
            error.append(abs(factor) * uncertainty)
            factor *= rates[j] * retardation[j]
            maximum_condition = max(maximum_condition, conditioning)
        output.append(result)
        errors.append(error)
    return output, errors, maximum_condition, close


@lru_cache(maxsize=12)
def _stehfest_weights(order, digits):
    require(
        type(order) is int and order > 0 and order % 2 == 0 and order <= 512,
        "invalid inversion order",
    )
    require(digits in (400, 650), "unsupported inversion precision")
    with localcontext() as context:
        context.prec = digits
        half = order // 2
        factorial = [math.factorial(k) for k in range(order + 1)]
        values = []
        for k in range(1, order + 1):
            terms = []
            for j in range((k + 1) // 2, min(k, half) + 1):
                numerator = j**half * factorial[2 * j]
                denominator = (
                    factorial[half - j]
                    * factorial[j]
                    * factorial[j - 1]
                    * factorial[k - j]
                    * factorial[2 * j - k]
                )
                terms.append(Decimal(numerator) / denominator)
            values.append((-1) ** (k + half) * sum(terms, Decimal(0)))
        return tuple(values)


def _invert_transform(transform, time, order, digits):
    """Diagnostic inverse transform, independent of either solver discretization."""
    with localcontext() as context:
        context.prec = digits
        step = Decimal(2).ln() / _decimal(time)
        return step * sum(
            (
                weight * transform(step * k)
                for k, weight in enumerate(_stehfest_weights(order, digits), 1)
            ),
            Decimal(0),
        )


def _diagnostic_indices(case, coordinates, time, values=None):
    indices = {0, len(coordinates) // 2, len(coordinates) - 1}
    theta = case["theta"]
    for j, species in enumerate(case["species"]):
        front = case["q"] * time / (theta + case["rho"] * species["kd"])
        indices.add(min(range(len(coordinates)), key=lambda i: abs(coordinates[i] - front)))
        if values is not None:
            row = [
                values[("profiles", time, "concentration", j, i)] for i in range(len(coordinates))
            ]
            indices.update(
                (
                    min(range(len(row)), key=lambda i: row[i]),
                    max(range(len(row)), key=lambda i: row[i]),
                    min(range(len(row)), key=lambda i: abs(row[i])),
                )
            )
    return sorted(indices)


def _chain_order(
    case,
    coordinates,
    analysis_times,
    diagnostic_times,
    order,
    digits,
    profile_indices=None,
    direct=False,
):
    """One bounded evaluation; sparse direct diagnostics keep original node indices."""
    with localcontext() as context:
        context.prec = digits
        theta, storage, retardation, rates, velocity, dispersion, length = _chain_parameters(case)
        q = _decimal(case["q"])
        inlet = [q * _decimal(c) for c in case["boundary"]["inlet_concentration"]]
        species_count = len(storage)
        result, uncertainty, recurrence_errors = {}, {}, {}
        condition, guarded = Decimal(1), 0
        weights = _stehfest_weights(order, digits)
        epsilon = Decimal(10) ** (12 - digits)
        for raw_time in diagnostic_times:
            time = _decimal(raw_time)
            indices = (
                (
                    list(range(len(coordinates)))
                    if profile_indices is None
                    else profile_indices.get(raw_time, [])
                )
                if raw_time in [0, *analysis_times]
                else []
            )
            if indices and indices[-1] != len(coordinates) - 1:
                raise ValueError("profile diagnostic subset must contain outlet")
            positions = [_decimal(coordinates[i]) for i in indices] if indices else [length]
            size = len(positions)
            sums = [[Decimal(0)] * species_count for _ in range(size + 3)]
            errors = [[Decimal(0)] * species_count for _ in sums]
            absolute_sums = [[Decimal(0)] * species_count for _ in sums]
            check_indices = (
                _diagnostic_indices(case, coordinates, raw_time) if indices and not direct else []
            )
            check_positions = [_decimal(coordinates[i]) for i in check_indices]
            differences = [[Decimal(0)] * species_count for _ in check_indices]
            if time:
                laplace_step = Decimal(2).ln() / time
                for k, weight in enumerate(weights, 1):
                    p = laplace_step * k
                    transformed, arithmetic, conditioning, guarded_node = _transform_values(
                        case, p, positions, recurrence=bool(indices) and not direct
                    )
                    condition = max(condition, conditioning)
                    guarded += int(guarded_node)
                    # Last transform is I. Append H(L)/p and I/p independently.
                    transformed.extend(
                        [[c / p for c in transformed[size - 1]], [c / p for c in transformed[size]]]
                    )
                    arithmetic.extend(
                        [[c / p for c in arithmetic[size - 1]], [c / p for c in arithmetic[size]]]
                    )
                    for n, row in enumerate(transformed):
                        for j, value in enumerate(row):
                            term = weight * value
                            sums[n][j] += term
                            absolute_sums[n][j] += abs(term)
                            errors[n][j] += abs(weight) * arithmetic[n][j]
                    if check_indices:
                        direct_values, _, _, _ = _transform_values(case, p, check_positions)
                        for n, index in enumerate(check_indices):
                            for j in range(species_count):
                                differences[n][j] += weight * (
                                    transformed[index][j] - direct_values[n][j]
                                )
                sums = [[laplace_step * x for x in row] for row in sums]
                errors = [
                    [laplace_step * (e + order * epsilon * a) for e, a in zip(er, ar)]
                    for er, ar in zip(errors, absolute_sums)
                ]
                differences = [[laplace_step * x for x in row] for row in differences]
            for n, index in enumerate(indices):
                for j in range(species_count):
                    key = ("profiles", raw_time, "concentration", j, index)
                    result[key], uncertainty[key] = sums[n][j], errors[n][j]
                    recurrence_errors[key] = max(
                        (abs(row[j]) for row in differences), default=Decimal(0)
                    )
            for j in range(species_count):

                def put(field, value, error=Decimal(0), side=-1):
                    key = ("continuum", raw_time, field, j, side)
                    result[key], uncertainty[key], recurrence_errors[key] = value, error, Decimal(0)

                spatial, integrated = sums[size][j], sums[size + 2][j]
                spatial_error, integrated_error = errors[size][j], errors[size + 2][j]
                put("dissolved", theta * spatial, theta * spatial_error)
                put(
                    "equilibrium_sorbed",
                    (storage[j] - theta) * spatial,
                    (storage[j] - theta) * spatial_error,
                )
                put("total", storage[j] * spatial, storage[j] * spatial_error)
                put("endpoint_flux", inlet[j], side=0)
                put("endpoint_flux", -q * sums[size - 1][j], q * errors[size - 1][j], side=1)
                put("boundary_cumulative", inlet[j] * time, side=0)
                put("boundary_cumulative", -q * sums[size + 1][j], q * errors[size + 1][j], side=1)
                put(
                    "production_cumulative",
                    rates[j - 1] * storage[j - 1] * sums[size + 2][j - 1] if j else Decimal(0),
                    rates[j - 1] * storage[j - 1] * errors[size + 2][j - 1] if j else Decimal(0),
                )
                put(
                    "loss_cumulative",
                    rates[j] * storage[j] * integrated,
                    rates[j] * storage[j] * integrated_error,
                )
        return {
            "values": result,
            "arithmetic": uncertainty,
            "recurrence": recurrence_errors,
            "order": order,
            "digits": digits,
            "maximum_conditioning": str(condition),
            "guarded_samples": guarded,
        }


def _chain_budget(case, field):
    budget = case["analytical"]["budget"]
    theta, storage, _, _, _, dispersion, length = _chain_parameters(case)
    q = _decimal(case["q"])
    if field == "concentration":
        return _decimal(budget["concentration_absolute"])
    if field == "endpoint_flux":
        return _decimal(budget["instantaneous_flux_scaled"]) * max(q, theta * dispersion / length)
    if field.endswith("cumulative"):
        return _decimal(budget["cumulative_scaled"]) * (
            length * max(storage) + q * _decimal(case["time_end"])
        )
    return _decimal(budget["inventory_scaled"]) * length * max(storage)


def _known_exact(case, key):
    group, time, field, species, index = key
    if time == 0 or (field in PAIR_FIELDS and index == 0):
        return True
    if field == "production_cumulative" and species == 0:
        return True
    if field == "loss_cumulative" and case["species"][species]["rate"] == 0:
        return True
    if field == "equilibrium_sorbed" and case["species"][species]["kd"] == 0:
        return True
    return any(s["rate"] == 0 for s in case["species"][:species])


def _difference_diagnostic(values, exact=False):
    previous, latest = abs(values[-2] - values[-3]), abs(values[-1] - values[-2])
    decreasing = exact or latest < previous
    return previous, latest, decreasing


def _fingerprint(value):
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()
    ).hexdigest()


def _binary_case(case):
    """Diagnostic perturbation from authored decimals to exact binary64 inputs."""

    def convert(value):
        if isinstance(value, dict):
            return {k: convert(v) for k, v in value.items()}
        if isinstance(value, list):
            return [convert(v) for v in value]
        return Decimal.from_float(value) if type(value) is float else value

    return convert(case)


def _checked_uniform_mesh(coordinates, length):
    # Wave 2 nodes are binary64 evaluations of i*L/(N-1). Recurrences use
    # Decimal ideal nodes; their difference is measured by direct diagnostics.
    for i, value in enumerate(coordinates):
        ideal = i * length / (len(coordinates) - 1)
        require(
            abs(value - ideal) <= 4 * math.ulp(ideal),
            "finite-chain recurrence requires uniform mesh",
        )


def _make_chain_reference(case, coordinates, analysis_times, diagnostic_times, values, errors):
    reference = {
        "schema_version": "solute-metrics-reference-v1",
        "coordinates": list(coordinates),
        "species": [s["name"] for s in case["species"]],
        "profiles": [],
        "continuum": [],
        "uncertainty": {"profiles": [], "continuum": []},
    }
    for group, times, fields in (
        ("profiles", [0, *analysis_times], ("concentration",)),
        ("continuum", diagnostic_times, FIELDS),
    ):
        for time in times:
            row, error = {"time": time}, {"time": time}
            for field in fields:
                indices = (
                    range(len(coordinates))
                    if group == "profiles"
                    else range(2) if field in PAIR_FIELDS else [-1]
                )
                rows, uncertainties = [], []
                for j in range(len(case["species"])):
                    pairs = [
                        (values[(group, time, field, j, i)], errors[(group, time, field, j, i)])
                        for i in indices
                    ]
                    rows.append(
                        [p[0] for p in pairs]
                        if group == "profiles" or field in PAIR_FIELDS
                        else pairs[0][0]
                    )
                    uncertainties.append(
                        [p[1] for p in pairs]
                        if group == "profiles" or field in PAIR_FIELDS
                        else pairs[0][1]
                    )
                row[field], error[field] = rows, uncertainties
            reference[group].append(row)
            reference["uncertainty"][group].append(error)
    return reference


def finite_chain_reference(
    case: dict,
    coordinates: list,
    analysis_times: list,
    diagnostic_times: list,
    *,
    allow_escalation=False,
) -> tuple:
    """Return (frozen reference wrapper, empirical admission diagnostics).

    Three full orders (192/256/384 at400 digits) are mandatory. Explicit
    allow_escalation permits at most one additional full order512 at650 digits.
    Final-order sparse checks compare400/650 digits, authored/binary parameters,
    and direct/recurrence coordinates. Every continuum field is checked at every
    diagnostic time; sparse profile differences are empirical estimates extended
    by their per-species/time maxima to unsampled nodes, not rigorous bounds.
    """
    _validate_inputs(case, coordinates, analysis_times, diagnostic_times, allow_finite=True)
    require(case["family"] == "finite_chain", "finite-chain case required")
    require(type(allow_escalation) is bool, "explicit escalation flag required")
    _checked_uniform_mesh(coordinates, case["length"])
    settings = case["analytical"].get("settings", {})
    require(
        settings
        == {
            "orders": [192, 256, 384],
            "maximum_order": 512,
            "decimal_digits": [400, 650],
            "uncertainty_safety_factor": 10,
            "maximum_full_evaluations": 4,
        },
        "unreviewed finite-chain settings",
    )
    runs = [
        _chain_order(case, coordinates, analysis_times, diagnostic_times, n, 400)
        for n in settings["orders"]
    ]
    with localcontext() as context:
        context.prec = 650
        need_more = False
        for key in runs[-1]["values"]:
            _, last, decreasing = _difference_diagnostic(
                [r["values"][key] for r in runs], _known_exact(case, key)
            )
            if not decreasing or 10 * last + runs[-1]["arithmetic"][key] > _chain_budget(
                case, key[2]
            ):
                need_more = True
        if need_more and allow_escalation:
            runs.append(_chain_order(case, coordinates, analysis_times, diagnostic_times, 512, 650))
        final = runs[-1]
        subset = {
            t: _diagnostic_indices(case, coordinates, t, final["values"])
            for t in [0, *analysis_times]
        }
        opposite_digits = 650 if final["digits"] == 400 else 400
        precision = _chain_order(
            case,
            coordinates,
            analysis_times,
            diagnostic_times,
            final["order"],
            opposite_digits,
            subset,
            direct=True,
        )
        # A same-650 repetition is never presented as independent precision evidence.
        inputs = _chain_order(
            _binary_case(case),
            [Decimal.from_float(x) if type(x) is float else x for x in coordinates],
            analysis_times,
            diagnostic_times,
            final["order"],
            final["digits"],
            subset,
            direct=True,
        )
        precision_max, input_max = {}, {}
        for key in precision["values"]:
            if key[0] == "profiles":
                bucket = key[:4]
                precision_max[bucket] = max(
                    precision_max.get(bucket, Decimal(0)),
                    abs(precision["values"][key] - final["values"][key]),
                )
                input_max[bucket] = max(
                    input_max.get(bucket, Decimal(0)),
                    abs(inputs["values"][key] - final["values"][key]),
                )
        values, errors, records = {}, {}, []
        for key, value in final["values"].items():
            previous, last, decreasing = _difference_diagnostic(
                [r["values"][key] for r in runs], _known_exact(case, key)
            )
            precision_error = (
                precision_max[key[:4]]
                if key[0] == "profiles"
                else abs(precision["values"][key] - value)
            )
            input_error = (
                input_max[key[:4]] if key[0] == "profiles" else abs(inputs["values"][key] - value)
            )
            recurrence_error = final["recurrence"][key]
            arithmetic = max(
                final["arithmetic"][key],
                precision["arithmetic"].get(key, Decimal(0)),
                inputs["arithmetic"].get(key, Decimal(0)),
            )
            estimated = 10 * last + precision_error + input_error + recurrence_error + arithmetic
            binary = float(value)
            require(math.isfinite(binary), "nonfinite finite-chain reference value")
            serialization = abs(Decimal.from_float(binary) - value)
            uncertainty = estimated + serialization
            error = float(uncertainty)
            require(math.isfinite(error), "finite-chain uncertainty exceeds finite output range")
            if Decimal.from_float(error) < uncertainty:
                error = math.nextafter(error, math.inf)
            values[key], errors[key] = binary, error
            group, time, field, species, index = key
            records.append(
                {
                    "group": group,
                    "time": time,
                    "field": field,
                    "species": case["species"][species]["name"],
                    "index": index,
                    "previous_order_difference": str(previous),
                    "last_order_difference": str(last),
                    "decreasing": decreasing,
                    "exact_identity": _known_exact(case, key),
                    "precision": str(precision_error),
                    "input_coordinate": str(input_error),
                    "recurrence": str(recurrence_error),
                    "arithmetic_estimate": str(arithmetic),
                    "serialization": str(serialization),
                    "uncertainty": error,
                    "budget": float(_chain_budget(case, field)),
                    "status": (
                        "admitted"
                        if decreasing and Decimal.from_float(error) <= _chain_budget(case, field)
                        else "unmet"
                    ),
                }
            )
        reference = _make_chain_reference(
            case, coordinates, analysis_times, diagnostic_times, values, errors
        )
        diagnostics = {
            "schema_version": "solute-chain-diagnostics-v1",
            "case_sha256": _fingerprint(case),
            "reference_sha256": _fingerprint(reference),
            "basis": "empirical inversion-order, precision, recurrence and input sensitivity estimates; not rigorous remainder bounds",
            "orders": [r["order"] for r in runs],
            "digits": [r["digits"] for r in runs],
            "precision_comparison_digits": [final["digits"], opposite_digits],
            "profile_diagnostic_indices": {str(t): indices for t, indices in subset.items()},
            "analysis_times": list(analysis_times),
            "diagnostic_times": list(diagnostic_times),
            "work": [
                {
                    "order": r["order"],
                    "digits": r["digits"],
                    "maximum_conditioning": r["maximum_conditioning"],
                    "guarded_samples": r["guarded_samples"],
                }
                for r in [*runs, precision, inputs]
            ],
            "observables": records,
            "status": "admitted" if all(r["status"] == "admitted" for r in records) else "unmet",
        }
        return reference, diagnostics


def _check_chain_diagnostics(case, reference, diagnostics):
    require(
        isinstance(diagnostics, dict)
        and diagnostics.get("schema_version") == "solute-chain-diagnostics-v1",
        "finite-chain admission requires diagnostics",
    )
    require(
        diagnostics["case_sha256"] == _fingerprint(case)
        and diagnostics["reference_sha256"] == _fingerprint(reference),
        "finite-chain diagnostics hash mismatch",
    )
    require(
        diagnostics["orders"] in ([192, 256, 384], [192, 256, 384, 512]),
        "unreviewed inversion orders",
    )
    require(
        diagnostics["digits"] == ([400] * 3 + ([650] if len(diagnostics["orders"]) == 4 else [])),
        "unreviewed inversion precision",
    )
    require(
        set(diagnostics["precision_comparison_digits"]) == {400, 650},
        "missing independent precision diagnostic",
    )
    require(
        [r["time"] for r in reference["profiles"]] == [0, *diagnostics["analysis_times"]]
        and [r["time"] for r in reference["continuum"]] == diagnostics["diagnostic_times"],
        "finite-chain diagnostic time coverage",
    )
    expected = {}
    for group in ("profiles", "continuum"):
        for row, errors in zip(reference[group], reference["uncertainty"][group]):
            for field in ("concentration",) if group == "profiles" else FIELDS:
                for j, name in enumerate(reference["species"]):
                    indices = (
                        range(len(reference["coordinates"]))
                        if group == "profiles"
                        else range(2) if field in PAIR_FIELDS else [-1]
                    )
                    for index in indices:
                        expected[(group, row["time"], field, name, index)] = (
                            errors[field][j][index] if index != -1 else errors[field][j]
                        )
    actual = [
        (r["group"], r["time"], r["field"], r["species"], r["index"])
        for r in diagnostics["observables"]
    ]
    require(
        len(actual) == len(set(actual)) and set(actual) == set(expected),
        "finite-chain diagnostic observable coverage",
    )
    require(
        len(diagnostics["work"]) == len(diagnostics["orders"]) + 2, "finite-chain work cap mismatch"
    )
    with localcontext() as context:
        context.prec = 650
        for row in diagnostics["observables"]:
            label = (row["group"], row["time"], row["field"], row["species"], row["index"])
            j = reference["species"].index(row["species"])
            key = (row["group"], row["time"], row["field"], j, row["index"])
            components = {
                name: _decimal(row[name])
                for name in (
                    "previous_order_difference",
                    "last_order_difference",
                    "precision",
                    "input_coordinate",
                    "recurrence",
                    "arithmetic_estimate",
                    "serialization",
                )
            }
            require(
                all(value >= 0 for value in components.values()),
                "negative diagnostic error component",
            )
            require(
                type(row["uncertainty"]) in (int, float)
                and math.isfinite(row["uncertainty"])
                and row["uncertainty"] >= 0
                and row["uncertainty"] == expected[label],
                "diagnostic uncertainty differs from reference",
            )
            budget = _chain_budget(case, row["field"])
            require(row["budget"] == float(budget), "diagnostic budget differs from frozen budget")
            exact = _known_exact(case, key)
            decreasing = (
                exact
                or components["last_order_difference"] < components["previous_order_difference"]
            )
            require(
                type(row["exact_identity"]) is bool and row["exact_identity"] == exact,
                "incorrect exact-identity claim",
            )
            require(
                type(row["decreasing"]) is bool and row["decreasing"] == decreasing,
                "incorrect decreasing-difference claim",
            )
            minimum = 10 * components["last_order_difference"] + sum(
                components[name]
                for name in (
                    "precision",
                    "input_coordinate",
                    "recurrence",
                    "arithmetic_estimate",
                    "serialization",
                )
            )
            require(
                Decimal.from_float(float(row["uncertainty"])) >= minimum,
                "uncertainty below reported error components",
            )
            expected_status = (
                "admitted"
                if decreasing and Decimal.from_float(float(row["uncertainty"])) <= budget
                else "unmet"
            )
            require(row["status"] == expected_status, "inconsistent empirical admission")
    expected_status = (
        "admitted"
        if all(row["status"] == "admitted" for row in diagnostics["observables"])
        else "unmet"
    )
    require(diagnostics["status"] == expected_status, "inconsistent diagnostic summary status")

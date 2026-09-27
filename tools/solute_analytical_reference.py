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

import math
from decimal import Decimal, ROUND_CEILING, ROUND_FLOOR, localcontext
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


def _validate_inputs(case, coordinates, analysis_times, diagnostic_times):
    if case.get("family") == "finite_chain":
        raise NotImplementedError("finite-chain reference unavailable pending task 13.3.3")
    require(case.get("status") == "executable", "blocked analytical case")
    require(case.get("family") in {"reaction", "diffusion", "scalar_robin"}, "unsupported family")
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


def reference_admission(case: dict, reference: dict) -> dict:
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
    diagnostics = [r["time"] for r in reference["continuum"]]
    _validate_inputs(case, reference["coordinates"], analysis, diagnostics)
    observation = Context(
        coordinates=reference["coordinates"],
        species=[s["name"] for s in case["species"]],
        profiles=[{"time": t} for t in diagnostics],
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
    return {
        "status": "admitted" if all(d["status"] == "admitted" for d in decisions) else "unmet",
        "basis": "component interval arithmetic, analytical truncation and serialization bounds",
        "observables": decisions,
    }

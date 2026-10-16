"""Authenticated same-solver bridges; no build, solver, or activation operations.

Arithmetic bounds protect the identity of the discrete Rust calculation only.
They are deliberately conservative and are not scientific accuracy tolerances.
"""

import math
import re
from decimal import Decimal, ROUND_CEILING, ROUND_FLOOR, localcontext
from fractions import Fraction
from functools import lru_cache
from pathlib import Path

from tools import compare_solute_reference as legacy
from tools import run_solute_verification_reference as fortran
from tools import solute_benchmark_lifecycle as lifecycle
from tools import solute_verification_metrics as metrics

ROOT = Path(__file__).resolve().parents[1]
BASELINE = ".development/13-baseline/protected-hashes.json"
BASELINE_HASH = "018323fc8049ba44ecfdd8ab4d453407566b81130464b74cb234b76a7ba0b7fd"
FIXTURE = "reference/fortran_reference/solute/example3"
OLD_RUN = ".development/12.2.1/run"
OLD_RUST = ".development/12.6.1/rust-observations.json"
POLICY_HASHES = {
    # Re-pinned under Orchestration 20 D6 (Wave 1 / 20.1.1): default SoluteBackend reverted
    # to FiniteVolumeBackwardEuler pending genuine 44-fixture Rust-vs-Fortran parity verification.
    "crates/hydrus-core/src/solute.rs": "7b0912e5a536b6e1851602eaa05d33de13b6f04d2d59f2d400ba347c2e7ffc7d",
    "crates/hydrus-core/src/hydraulics/linear.rs": "6b5649011b1c605e77d55500fa08bd5bfbb708d3548abd225c3dbc11f3e0196e",
    "crates/hydrus-core/tests/solute_fixture_parity.rs": "890bb77171f189402299cf9ceaf8b4809ffa15e3c52a82355016e92b6893bba9",
    "crates/hydrus-core/tests/solute_verification_harness.rs": "df60e942863d08738ec60945aec548815a3ab021d70ff1b7d2b0469e73beb3f1",
}
COMPARATORS = (
    "tools/solute_benchmark_bridge.py",
    "tools/compare_solute_reference.py",
    *lifecycle.ADAPTERS,
)
BRIDGES = {s: f"benchmark-example3-bridge-{s}-baseline" for s in ("fortran", "rust")}
TIMES = [0.0, 50.0, 100.0, 200.0]
SPECIES = ["NH4", "NO2", "NO3"]
NODES = 1001
STEPS = 2000
DT = 0.1
KD = [0.5375, 0.0, 0.0]
RATES = [0.005, 0.1, 0.0]
require = fortran.require
safe = fortran.safe_path
sha256 = fortran.sha256


def _bindings(paths):
    return {str(p.relative_to(ROOT)): sha256(p) for p in sorted(set(paths))}


def retained_artifacts(solver):
    """Authenticate the fixed original inventory, never a regression override."""
    require(solver in BRIDGES, "unknown bridge solver")
    metrics.hash_match(safe(ROOT, BASELINE), BASELINE_HASH)
    inventory = metrics.json_load(ROOT / BASELINE)
    prefixes = (FIXTURE + "/",)
    if solver == "fortran":
        prefixes += (OLD_RUN + "/", ".development/12.2.1/reference-build/")
    names = [n for n in inventory if n.startswith(prefixes)]
    if solver == "rust":
        names.append(OLD_RUST)
    require(names and OLD_RUST in inventory, "incomplete retained inventory")
    for name in names:
        metrics.hash_match(safe(ROOT, name), inventory[name])
    if solver == "fortran":
        legacy.verified_paths(ROOT / OLD_RUN, ROOT / FIXTURE / "RUN.md")
        build = metrics.json_load(ROOT / ".development/12.2.1/reference-build/build-manifest.json")
        require(build["flags"] == fortran.FLAGS, "retained arithmetic flags")
        require(
            build["executable_sha256"]
            == inventory[".development/12.2.1/reference-build/hydrus-reference"],
            "retained executable identity",
        )
    for name, digest in POLICY_HASHES.items():
        metrics.hash_match(safe(ROOT, name), digest)
    return {BASELINE: BASELINE_HASH, **{n: inventory[n] for n in names}, **POLICY_HASHES}


def _configuration(book, solver):
    row = next(r for r in book["launches"] if r["id"] == BRIDGES[solver])
    case = next(c for c in book["cases"] if c["id"] == row["case_id"])
    require(
        row["role"] == "bridge"
        and row["nodes"] == NODES
        and row["dt"] == DT
        and row["max_steps"] == STEPS
        and case["id"] == "example3"
        and case["length"] == 250
        and case["theta"] == 0.43
        and case["q"] == 0.43
        and case["dispersion"] == 0.18
        and case["rho"] == 0.8,
        "arithmetic policy applies only to frozen Example 3",
    )
    require([s["name"] for s in case["species"]] == SPECIES, "species mapping")
    require([s["kd"] for s in case["species"]] == KD, "sorption mapping")
    require([s["rate"] for s in case["species"]] == RATES, "reaction mapping")
    require(
        case["initial"] == {"kind": "uniform", "values": [0, 0, 0]}
        and case["boundary"]
        == {"inlet": "total_flux", "inlet_concentration": [1, 0, 0], "outlet": "zero_gradient"},
        "boundary/initial interpretation",
    )
    return case, row


def load_retained_rust():
    data = legacy.parse_rust_observations(ROOT / OLD_RUST)
    # The old harness accumulates accepted time; the new harness uses step*dt.
    accumulated = 0.0
    actual = {0.0: 0.0}
    for step in range(1, STEPS + 1):
        accumulated += DT
        if step in (500, 1000, 2000):
            actual[step * DT] = accumulated
    for profile in data["observations"]:
        require(
            profile["actual_time_h"] == actual[profile["time_h"]],
            "retained accepted-time recurrence changed",
        )
    return data


def _fortran_records(root, case, row):
    """Strict raw readers validate structure, water, signs, and native ordering."""
    inventory, mapping = fortran.validate_outputs(root, case, row)
    paths = {
        n: safe(root, "solute-input\\" + n)
        for n in (*fortran.OUTPUTS, "solute1.out", "solute2.out", "solute3.out")
    }
    steps = metrics._run_rows(paths["RUN_INF.OUT"], case, row)
    metrics._water_rows(paths["T_LEVEL.OUT"], case, row)
    coordinates, profiles, rounding, _ = metrics._node_profiles(paths["NOD_INF.OUT"], case, row)
    inventories, balances = metrics._balance(paths["BALANCE.OUT"], case, row)
    solutes = [metrics._solute_rows(paths[f"solute{j}.out"], case, row) for j in (1, 2, 3)]
    metrics._ancillary(paths, case, row, fortran.initial_arrays(case, NODES))
    return dict(
        paths=paths,
        inventory=inventory,
        mapping=mapping,
        steps=steps,
        coordinates=coordinates,
        profiles=profiles,
        rounding=rounding,
        inventories=inventories,
        balances=balances,
        solutes=solutes,
        endpoint_history=None,
    )


class _Interval:
    """Small directed Decimal interval for the scalar error certificate only."""

    def __init__(self, value, upper=None):
        if isinstance(value, float):
            value = Decimal.from_float(value)
        self.lo = Decimal(value)
        self.hi = self.lo if upper is None else upper

    def _op(self, other, operation):
        other = other if isinstance(other, _Interval) else _Interval(other)
        if operation == "div":
            require(other.lo > 0, "nonpositive arithmetic-bound denominator")
        endpoints = []
        for rounding in (ROUND_FLOOR, ROUND_CEILING):
            with localcontext() as context:
                context.prec = 60
                context.rounding = rounding
                values = []
                for a in (self.lo, self.hi):
                    for b in (other.lo, other.hi):
                        values.append(
                            {
                                "add": lambda: a + b,
                                "sub": lambda: a - b,
                                "mul": lambda: a * b,
                                "div": lambda: a / b,
                            }[operation]()
                        )
                endpoints.append(min(values) if rounding == ROUND_FLOOR else max(values))
        return _Interval(endpoints[0], endpoints[1])

    def __add__(self, other):
        return self._op(other, "add")

    __radd__ = __add__

    def __sub__(self, other):
        return self._op(other, "sub")

    def __mul__(self, other):
        return self._op(other, "mul")

    __rmul__ = __mul__

    def __truediv__(self, other):
        return self._op(other, "div")

    def upper(self):
        return 0.0 if self.hi == 0 else math.nextafter(float(self.hi), math.inf)


def _gamma(count):
    nu = _Interval(2.0**-53) * count
    return nu / (_Interval(1) - nu)


@lru_cache(maxsize=1)
def arithmetic_certificate():
    """Outward bounds for both orderings of the same discrete M-matrix solve.

    S is equilibrium storage, not the spec's retardation R. Thomas uses <8N
    operations; flux/widths <=24N, two local mass/intercept evaluations <=32N,
    source/row assembly <=16N, remaining boundary/setup <=24N, and postsolve
    mass conversion <=24N. Add 32 scalar operations: 128N+32 in total.
    |L||U|=|A| for exact M-matrix LU. Positive pivots and the rounded factor
    identities bound the assembly/LU perturbation by 4*gamma*a. RHS expansion
    is bounded by 8*gamma*H, including both cancelling intercept terms.
    See the approved TODO derivation; no observation values enter this function.
    """
    interval = _Interval
    theta, rho, q, dt, length = map(interval, (0.43, 0.8, 0.43, DT, 250))
    storage = [theta + rho * interval(kd) for kd in KD]
    rates = list(map(interval, RATES))
    g = [interval(1) / dt + rate for rate in rates]
    d = theta * interval(0.18) / interval(0.25)
    margins = [interval(0.125) * s * rate for s, rate in zip(storage, g)]
    # The interior has the largest absolute row sum; endpoints are smaller.
    norms = [interval(0.25) * s * rate + 2 * q + 4 * d for s, rate in zip(storage, g)]
    operation_count = 128 * NODES + 32
    delta = _gamma(operation_count)
    alpha = [4 * delta * a for a in norms]
    require(all(a.hi < m.lo / 4 for a, m in zip(alpha, margins)), "lost diagonal dominance")
    beta = [interval(1) / (interval(1) + dt * r) for r in rates]
    eta = [interval(0)] + [
        dt * rates[j - 1] * storage[j - 1] / (storage[j] * (interval(1) + dt * rates[j]))
        for j in (1, 2)
    ]
    daughter = rates[0] * storage[0] / (rates[1] * storage[1])

    def envelopes(step):
        return [interval(1), daughter, step * dt * rates[1] * storage[1] * daughter / storage[2]]

    errors = [interval(0), interval(0), interval(0)]
    boundary_error, reaction_error = [interval(0) for _ in KD], [interval(0) for _ in KD]
    boundary_abs, reaction_abs = [interval(0) for _ in KD], [interval(0) for _ in KD]
    underflow = interval(math.ulp(0.0)) * operation_count
    rows = {
        "0": {
            k: [0.0] * 3
            for k in ("concentration", "sorbed", "inventory", "boundary", "reaction", "balance")
        }
    }
    for step in range(1, STEPS + 1):
        old = errors
        errors = []
        before, bound = envelopes(step - 1), envelopes(step)
        for j in range(3):
            parent = interval(0) if j == 0 else bound[j - 1] + errors[j - 1]
            incoming = interval(0) if j == 0 else rates[j - 1] * storage[j - 1] * parent
            c_old = before[j] + old[j]
            # Expanded intercept: mass - derivative*C. Both absolute terms
            # matter despite the exact linear intercept being zero.
            h = interval(0.25) * (
                storage[j] * c_old / dt + incoming + 2 * g[j] * storage[j] * c_old
            )
            h = h + (q if j == 0 else interval(0))
            propagated = beta[j] * old[j] + eta[j] * (interval(0) if j == 0 else errors[j - 1])
            error = margins[j] * propagated + alpha[j] * bound[j] + 8 * delta * h + underflow
            errors.append(error / (margins[j] - alpha[j]))
            c = bound[j] + errors[j]
            b_abs = q * dt * ((1 if j == 0 else 0) + c)
            b_error = q * dt * errors[j]
            r_abs = length * dt * (rates[j] * storage[j] * c + incoming)
            r_error = (
                length
                * dt
                * (
                    rates[j] * storage[j] * errors[j]
                    + (interval(0) if j == 0 else rates[j - 1] * storage[j - 1] * errors[j - 1])
                )
            )
            boundary_abs[j] = boundary_abs[j] + b_abs
            reaction_abs[j] = reaction_abs[j] + r_abs
            boundary_error[j] = boundary_error[j] + b_error + _gamma(8) * b_abs + underflow
            reaction_error[j] = (
                reaction_error[j] + r_error + _gamma(16 * NODES + 16) * r_abs + underflow
            )
        if step in (500, 1000, 2000):
            result = {k: [] for k in rows["0"]}
            for j in range(3):
                c = bound[j] + errors[j]
                inv = length * storage[j] * errors[j] + _gamma(8 * NODES) * length * storage[j] * c
                b = boundary_error[j] + _gamma(step) * boundary_abs[j]
                r = reaction_error[j] + _gamma(step) * reaction_abs[j]
                balance = (
                    inv
                    + b
                    + r
                    + _gamma(4) * (length * storage[j] * c + boundary_abs[j] + reaction_abs[j])
                )
                values = [
                    errors[j],
                    interval(KD[j]) * errors[j] + _gamma(1) * interval(KD[j]) * c,
                    inv,
                    b,
                    r,
                    balance,
                ]
                for key, value in zip(result, values):
                    result[key].append((2 * value).upper())
            rows[f"{step*DT:g}"] = result
    return {
        "policy": "example3-discrete-arithmetic-v1",
        "scientific_tolerance": False,
        "operations_per_species_step": operation_count,
        "operation_partitions": {
            "thomas": 8,
            "flux_width": 24,
            "local_mass": 32,
            "row_source": 16,
            "setup": 24,
            "postsolve_mass": 24,
        },
        "matrix_error_partitions": 4,
        "rhs_absolute_term_partitions": 8,
        "gamma": delta.upper(),
        "row_margin_lower": [math.nextafter(float(m.lo), -math.inf) for m in margins],
        "row_norm_upper": [a.upper() for a in norms],
        "serialization_absolute_error": 0,
        "serialization_reason": "Both pinned writers use .17e, round-tripping finite binary64.",
        "underflow_per_step": underflow.upper(),
        "bounds": rows,
        "scope": "Same discrete calculation only; conservative arithmetic guards, not accuracy budgets.",
    }


class _Comparison:
    def __init__(self):
        self.groups = {}
        self.failures = []
        self.failure_count = 0

    def number(self, group, label, old, new, bound):
        for value in (old, new, bound):
            metrics.finite(value, label)
        require(bound >= 0, "negative comparison allowance")
        # Exact rational difference avoids rounding a just-outside difference down.
        difference = abs(Fraction.from_float(float(old)) - Fraction.from_float(float(new)))
        passed = difference <= Fraction.from_float(float(bound))
        stat = self.groups.setdefault(
            group, {"count": 0, "maximum_difference": 0.0, "maximum_bound": 0.0, "failures": 0}
        )
        stat["count"] += 1
        stat["maximum_difference"] = max(stat["maximum_difference"], float(difference))
        stat["maximum_bound"] = max(stat["maximum_bound"], bound)
        if not passed:
            stat["failures"] += 1
            self.failure_count += 1
            if len(self.failures) < 32:
                self.failures.append(
                    dict(
                        field=label,
                        retained=old,
                        new=new,
                        difference=float(difference),
                        bound=bound,
                    )
                )

    def result(self):
        return {
            "status": "passed" if self.failure_count == 0 else "failed",
            "groups": self.groups,
            "failure_count": self.failure_count,
            "first_failures": self.failures,
        }


def _semantic_lines(path):
    """Exclude only the source's date header and elapsed runtime record."""
    lines = []
    for line in path.read_text(encoding="ascii").splitlines():
        line = line.strip()
        if re.fullmatch(r"Date:.*Time:.*", line) or line.startswith("Calculation time [sec]"):
            continue
        if line:
            lines.append(line.split())
    return lines


def _token_interval(token):
    value = metrics.numeric(token)
    if "." not in token and "E" not in token.upper():
        return value, 0.0, "integer"
    mantissa = re.split("[EeDd]", token)[0]
    digits = len(mantissa.partition(".")[2])
    fmt = ("e" if re.search("[EeDd]", token) else "f") + f"99.{digits}"
    return value, metrics.rounding_half_width(token, fmt), fmt


def compare_fortran(old, new):
    comparison = _Comparison()
    formats = {}
    require(
        old["endpoint_history"] is None and new["endpoint_history"] is None,
        "legacy endpoint history must stay unavailable",
    )
    for name in old["paths"]:
        left, right = _semantic_lines(old["paths"][name]), _semantic_lines(new["paths"][name])
        require(len(left) == len(right), f"{name}: changed record completeness")
        formats[name] = set()
        for index, (a, b) in enumerate(zip(left, right)):
            require(len(a) == len(b), f"{name}: changed column count")
            for column, (x, y) in enumerate(zip(a, b)):
                label = f"{name}:record{index}:column{column}"
                if metrics.NUMBER.fullmatch(x):
                    xv, xe, xf = _token_interval(x)
                    yv, ye, yf = _token_interval(y)
                    require(xf == yf, f"{label}: changed printed precision")
                    formats[name].add(xf)
                    allowance = (
                        0.0
                        if xf == "integer"
                        else math.nextafter(xe + ye + math.ulp(xv) + math.ulp(yv), math.inf)
                    )
                    comparison.number(name, label, xv, yv, allowance)
                else:
                    require(x == y, f"{label}: changed label/sign convention")
    result = comparison.result()
    result["precision"] = {
        "policy": "paired source-printed half-unit intervals plus conversion rounding",
        "concentration": "e12.4 (four significant digits)",
        "solute_flux_and_cumulative": "e13.5",
        "balance_inventory_and_absolute": "e13.5",
        "formats_by_file": {k: sorted(v) for k, v in formats.items()},
        "excluded_metadata": ["Date: ... Time: ...", "Calculation time [sec] ..."],
    }
    result["availability"] = {
        "retained_endpoint_history": None,
        "new_endpoint_history": None,
        "reason": "legacy_bridge_no_obs; no independent endpoint integral claim",
        "retained_steps": STEPS,
        "profiles": TIMES,
        "comparison": "All raw scientific fields, including separate boundary/reaction components.",
    }
    return result


def compare_rust(old, new):
    """Compare mapped observations; derived inventories remain separately labelled."""
    require(new["solver"] == "rust" and new["species"] == SPECIES, "Rust species ordering")
    require(new["coordinates"] == [i * 0.25 for i in range(NODES)], "Rust coordinate orientation")
    require([p["time"] for p in new["profiles"]] == TIMES, "Rust profile schedule")
    require([p["time_h"] for p in old["observations"]] == TIMES, "retained profile schedule")
    require(len(new["steps"]) == STEPS, "Rust step completeness")
    for i, row in enumerate(new["steps"], 1):
        require((row["index"], row["time"], row["dt"]) == (i, i * DT, DT), "Rust step schedule")
    reported = new["solver_reported"]
    require(
        reported["inventory"] is None and reported["balance"] is None,
        "Rust unavailable solver fields must remain null",
    )
    for key in ("boundary_cumulative", "reaction_cumulative"):
        require(
            isinstance(reported[key], list) and [r["time"] for r in reported[key]] == TIMES,
            f"Rust {key} coverage",
        )
        for record in reported[key]:
            metrics.vector(record["values"], 3, key)
    certificate = arithmetic_certificate()
    comparison = _Comparison()
    for p, previous in zip(new["profiles"], old["observations"]):
        time = p["time"]
        bounds = certificate["bounds"][f"{time:g}"]
        require(
            p["theta"] == [0.43] * NODES and p["q_faces"] == [0.43] * (NODES + 1),
            "Rust water/orientation mapping",
        )
        metrics.matrix(p["concentration"], 3, NODES, "Rust profile")
        require([r["name"] for r in previous["species"]] == SPECIES, "retained species ordering")
        at = TIMES.index(time)
        for j, retained in enumerate(previous["species"]):
            group = f"{time:g}/{SPECIES[j]}"
            # Native old x increases from outlet to inlet. Canonical s reverses it.
            native = list(reversed(p["concentration"][j]))
            metrics.vector(retained["concentration"], NODES, "retained profile")
            metrics.vector(retained["equilibrium_sorbed"], NODES, "retained sorbed")
            for i, value in enumerate(native):
                comparison.number(
                    group + "/concentration",
                    f"{group}/native-node-{i}",
                    retained["concentration"][i],
                    value,
                    bounds["concentration"][j],
                )
                comparison.number(
                    group + "/equilibrium_sorbed",
                    f"{group}/sorbed-node-{i}",
                    retained["equilibrium_sorbed"][i],
                    KD[j] * value,
                    bounds["sorbed"][j],
                )
            # Reproduce the legacy product/reduction order; this is derived storage.
            inventory = sum(
                (0.125 if i in (0, NODES - 1) else 0.25) * (0.43 + 0.8 * KD[j]) * v
                for i, v in enumerate(native)
            )
            boundary = reported["boundary_cumulative"][at]["values"][j]
            reaction = reported["reaction_cumulative"][at]["values"][j]
            for key, field, value in (
                ("inventory", "inventory", inventory),
                ("boundary", "cumulative_boundary_mass", boundary),
                ("reaction", "cumulative_reaction_mass", reaction),
                ("balance", "cumulative_balance_residual", inventory - boundary - reaction),
            ):
                comparison.number(
                    group + "/" + key, group + "/" + field, retained[field], value, bounds[key][j]
                )
    result = comparison.result()
    result["precision"] = certificate
    result["availability"] = {
        "retained_endpoint_history": None,
        "retained_per_step_records": None,
        "retained_executable_identity": None,
        "reason": "Original Rust artifact records only four profiles and declared 2000 steps; "
        "no original executable/build manifest or endpoint history was retained.",
        "new_steps_validated": STEPS,
        "profiles": TIMES,
        "derived_new_fields": ["equilibrium_sorbed", "inventory", "balance"],
        "comparison": "Retained independent boundary/reaction against new core diagnostics; "
        "this is a source-parity bridge, not independent continuous balance evidence.",
    }
    return result


def evaluate_bridge(cases_path: Path, run_dir: Path) -> dict:
    """Read-only authenticated comparison. Invalid evidence raises; numeric failure is returned."""
    cases_path, run_dir = Path(cases_path).absolute(), Path(run_dir).absolute()
    safe(ROOT, cases_path.relative_to(ROOT))
    safe(ROOT, run_dir.relative_to(ROOT))
    manager = lifecycle.Lifecycle(cases_path, run_dir.parent)
    manifest = manager.verify(run_dir, require_complete=True)
    solver = manifest["solver"]
    require(solver in BRIDGES and manifest["launch_id"] == BRIDGES[solver], "not a bridge row")
    case, row = _configuration(manager.book, solver)
    old_hashes = retained_artifacts(solver)
    data = metrics.load_observations(
        run_dir / "observations.json", run_dir / "manifest.json", cases_path
    )
    inputs = [run_dir / "manifest.json", run_dir / "observations.json", cases_path]
    inputs += [safe(run_dir, entry["path"]) for entry in manifest["artifacts"].values()]
    if solver == "fortran":
        inputs.append(run_dir / "normalization.json")
    bindings = _bindings(inputs)
    if solver == "rust":
        comparison = compare_rust(load_retained_rust(), data)
    else:
        old = _fortran_records(ROOT / OLD_RUN, case, row)
        new = _fortran_records(run_dir, case, row)
        require(data["endpoint_history"] is None, "normalized legacy endpoint history changed")
        comparison = compare_fortran(old, new)
    require(bindings == _bindings(inputs), "new artifact changed during comparison")
    require(old_hashes == retained_artifacts(solver), "retained artifact changed during comparison")
    return {
        "schema_version": "solute-bridge-v1",
        "solver": solver,
        "launch_id": row["id"],
        "case_id": case["id"],
        "cases_path": str(cases_path.relative_to(ROOT)),
        "run_dir": str(run_dir.relative_to(ROOT)),
        "retained_artifacts": old_hashes,
        "new_artifacts": bindings,
        "comparator_provenance": _bindings([safe(ROOT, n) for n in COMPARATORS]),
        "comparison": comparison,
        "scientific_accuracy_accepted": False,
    }


def _report_path(path):
    path = Path(path).absolute()
    relative = path.relative_to(ROOT)
    require(
        len(relative.parts) >= 3
        and relative.parts[0] == ".development"
        and relative.parts[1] in {"13.3.7", "13.3.1", "13.4.1", "13.6.1"},
        "bridge report outside allowed task scratch",
    )
    return safe(ROOT, relative)


def write_report(cases_path: Path, run_dir: Path, output_root: Path) -> Path:
    """Create an immutable report in an explicit fresh directory; refuse overwrite."""
    root = _report_path(output_root)
    require(not root.exists(), "bridge report output root already exists")
    payload = evaluate_bridge(cases_path, run_dir)
    envelope = {
        "schema_version": "solute-bridge-report-v1",
        "payload": payload,
        "payload_sha256": fortran.canonical_hash(payload),
    }
    root.mkdir(parents=True, exist_ok=False)
    destination = root / "bridge.json"
    fortran.write_json(destination, envelope)
    return destination


def verify_report(report_path: Path, cases_path: Path) -> dict:
    """Recompute the bound comparison, authenticating reports beyond their self-checksum."""
    report_path = _report_path(report_path)
    report = metrics.json_load(report_path)
    require(
        set(report) == {"schema_version", "payload", "payload_sha256"}
        and report["schema_version"] == "solute-bridge-report-v1",
        "bridge report schema",
    )
    payload = report["payload"]
    require(report["payload_sha256"] == fortran.canonical_hash(payload), "bridge report digest")
    require(
        payload["cases_path"] == str(Path(cases_path).absolute().relative_to(ROOT)),
        "bridge report casebook path",
    )
    fresh = evaluate_bridge(cases_path, safe(ROOT, payload["run_dir"]))
    require(
        fortran.canonical_hash(fresh) == report["payload_sha256"],
        "bridge report is stale, changed, or not reproducible",
    )
    return payload


def require_bridges(report_paths: list, cases_path: Path) -> dict:
    """Admission prerequisite for callers; this never activates a launch itself."""
    require(len(report_paths) == 2, "both bridge reports are required")
    reports = [verify_report(p, cases_path) for p in report_paths]
    require(
        {r["launch_id"] for r in reports} == set(BRIDGES.values()),
        "one report for each exact bridge row is required",
    )
    require(all(r["comparison"]["status"] == "passed" for r in reports), "bridge comparison failed")
    return {
        "status": "passed",
        "reports": _bindings([_report_path(p) for p in report_paths]),
        "cases_sha256": sha256(cases_path),
        "scientific_accuracy_accepted": False,
    }

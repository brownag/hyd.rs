#!/usr/bin/env python3
"""Validate the frozen solute study contract without launching either solver."""

import argparse
import copy
import hashlib
import json
import math
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CASES = ROOT / 'reference/solute_verification/CASES.json'
V1 = 'solute-verification-v1'
V2 = 'solute-verification-v2'
V2_BACKEND = 'fortran-compatible'
V2_TIMEOUT_EXCEPTIONS = {
    'convergence-example3-space_control-rfc-baseline': 180,
    'convergence-example3-time_control-1001-dt0.025-rfc-baseline': 180,
}
# v2 relabel (D4, 23.3.1): every rfc row is f32 except these four, which stay f64 by decision.
# The bridge row is not edited (D11). The three holdouts are not touched.
V2_PRECISION_F64_EXCEPTIONS = {
    'benchmark-example3-bridge-rfc-baseline',
    'holdout-diffusion_cosine-holdout-rfc-baseline',
    'holdout-scalar_robin-holdout-rfc-baseline',
    'holdout-example3-holdout-rfc-baseline',
}
V2_HOLDOUT_ID = {'diffusion_cosine': 'holdout-diffusion_cosine-holdout-rfc-baseline',
                 'scalar_robin': 'holdout-scalar_robin-holdout-rfc-baseline',
                 'example3': 'holdout-example3-holdout-rfc-baseline'}
# 15 isolating study rows (M5V_ISOLATION_DESIGN_23.md): id -> (case_id, axis, nodes, dt).
V2_ISOLATION = {
    'convergence-diffusion_cosine-space_65-dt2e-7-rfc-baseline': ('diffusion_cosine', 'space', 65, 2**-7),
    'convergence-diffusion_cosine-space_97-dt2e-7-rfc-baseline': ('diffusion_cosine', 'space', 97, 2**-7),
    'convergence-diffusion_cosine-space_129-dt2e-7-rfc-baseline': ('diffusion_cosine', 'space', 129, 2**-7),
    'convergence-diffusion_cosine-time_97-dt2e-6-rfc-baseline': ('diffusion_cosine', 'time', 97, 2**-6),
    'convergence-diffusion_cosine-time_97-dt2e-8-rfc-baseline': ('diffusion_cosine', 'time', 97, 2**-8),
    'convergence-scalar_robin-space_129-dt2e-7-rfc-baseline': ('scalar_robin', 'space', 129, 2**-7),
    'convergence-scalar_robin-space_193-dt2e-7-rfc-baseline': ('scalar_robin', 'space', 193, 2**-7),
    'convergence-scalar_robin-space_257-dt2e-7-rfc-baseline': ('scalar_robin', 'space', 257, 2**-7),
    'convergence-scalar_robin-time_193-dt2e-6-rfc-baseline': ('scalar_robin', 'time', 193, 2**-6),
    'convergence-scalar_robin-time_193-dt2e-8-rfc-baseline': ('scalar_robin', 'time', 193, 2**-8),
    'convergence-example3-space_501-dt0.04-rfc-baseline': ('example3', 'space', 501, 0.04),
    'convergence-example3-space_751-dt0.04-rfc-baseline': ('example3', 'space', 751, 0.04),
    'convergence-example3-space_1001-dt0.04-rfc-baseline': ('example3', 'space', 1001, 0.04),
    'convergence-example3-time_751-dt0.05-rfc-baseline': ('example3', 'time', 751, 0.05),
    'convergence-example3-time_751-dt0.025-rfc-baseline': ('example3', 'time', 751, 0.025),
}
# Five other-axis control rows (23R.1): id -> (case_id, axis, nodes, dt).
V2_CONTROLS = {
    'convergence-diffusion_cosine-space_control-129-dt2e-8-rfc-baseline':
        ('diffusion_cosine', 'space', 129, 2**-8),
    'convergence-diffusion_cosine-time_control-193-dt2e-8-rfc-baseline':
        ('diffusion_cosine', 'time', 193, 2**-8),
    'convergence-scalar_robin-space_control-257-dt2e-8-rfc-baseline':
        ('scalar_robin', 'space', 257, 2**-8),
    'convergence-scalar_robin-time_control-257-dt2e-8-rfc-baseline':
        ('scalar_robin', 'time', 257, 2**-8),
    'convergence-example3-time_control-1001-dt0.025-rfc-baseline':
        ('example3', 'time', 1001, 0.025),
}
V2_ADDED_ROWS = set(V2_ISOLATION) | set(V2_CONTROLS)
CEILING_KEYS = {'launch_cap', 'failed_attempts_consume_cap', 'storage_bytes_cap', 'phase_caps',
                'automatic_retries'}
METRICS = {
    'profile_linf', 'profile_l2', 'profile_l1', 'endpoint_linf', 'interior_linf',
    'inventory', 'boundary', 'reaction', 'balance', 'temporal_l2', 'normalization', 'order',
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def number(value, name, minimum=None, positive=False):
    require(type(value) in (int, float) and math.isfinite(value), f'{name}: finite number required')
    require(not positive or value > 0, f'{name}: must be positive')
    require(minimum is None or value >= minimum, f'{name}: below minimum')
    return value


def integer(value, name, low, high):
    require(type(value) is int and low <= value <= high, f'{name}: integer outside [{low}, {high}]')


def keys(value, expected, name):
    require(isinstance(value, dict) and set(value) == set(expected.split()), f'{name}: wrong keys')


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f'duplicate JSON key: {key}')
        result[key] = value
    return result


def load_cases(path: Path = DEFAULT_CASES) -> dict:
    """Load a casebook while rejecting duplicate JSON keys."""
    return json.loads(Path(path).read_text(), object_pairs_hook=unique_object)


def aligned(time, dt):
    ratio = time / dt
    return abs(ratio - round(ratio)) <= 1e-9


# Recorded per-row timeout overrides. Set in place by the CLI from an activation (execute)
# or a frozen recipe (verify/resume). Applied in validate(), the one loader every path uses,
# so no path can see the casebook timeout while another sees the override.
TIMEOUT_OVERRIDES = {}
TIMEOUT_OVERRIDE_RANGE = (60, 300)
# Approved, recorded overrides. An activation may only request these values, and the
# ledger accepts a start timeout equal to the casebook default or to the approved value.
# Casebook timeouts before any override, kept outside the book so validate_v2's exact key sets still hold.
BASE_TIMEOUTS = {}
APPROVED_TIMEOUT_OVERRIDES = {
    "convergence-example3-space_1001-rfc-baseline": 180,
    # Same 1001-node / 0.025 dt configuration as the space study row; the time control
    # needs the same approved deviation (23R.1).
    "convergence-example3-time_control-1001-dt0.025-rfc-baseline": 180,
}


def apply_timeout_overrides(book: dict, overrides: dict) -> dict:
    launches = {row['id']: row for row in book.get('launches', [])}
    for launch_id, seconds in overrides.items():
        if launch_id not in launches:
            raise ValueError('timeout override for unknown launch')
        if type(seconds) is not int or not TIMEOUT_OVERRIDE_RANGE[0] <= seconds <= TIMEOUT_OVERRIDE_RANGE[1]:
            raise ValueError('invalid timeout override')
        if APPROVED_TIMEOUT_OVERRIDES.get(launch_id) != seconds:
            raise ValueError('timeout override not approved')
        BASE_TIMEOUTS.setdefault(launch_id, launches[launch_id]['timeout_seconds'])
        launches[launch_id]['timeout_seconds'] = seconds
    return book


def validate(data: dict, verify_hashes: bool = True) -> dict:
    """Validate a casebook, dispatching on its schema version."""
    if isinstance(data, dict) and data.get('schema_version') == V2:
        book = validate_v2(data, verify_hashes)
    else:
        book = validate_v1(data, verify_hashes)
    return apply_timeout_overrides(book, TIMEOUT_OVERRIDES)


def validate_v1(data: dict, verify_hashes: bool = True) -> dict:
    """Validate mappings and the finite matrix, returning the unchanged casebook."""
    keys(data, 'schema_version units limits provenance contracts metrics cases launches budgets', 'root')
    require(data['schema_version'] == 'solute-verification-v1', 'unsupported schema')
    require(data['contracts'] == {
        'request': 'flat-ascii-v1', 'observation': 'solute-observation-v1',
        'manifest': 'solute-manifest-v1', 'spec': 'planning/master/SOLUTE_VERIFICATION_SPEC.md',
    }, 'contract identities changed')
    require(data['limits'] == {
        'nodes': 1001, 'print_times': 999, 'timeout_seconds': 60, 'automatic_retries': 0,
    }, 'unsupported source capacity, timeout or retry limit')
    keys(data['units'], 'length time concentration sorbed inventory flux density', 'units')
    require(all(isinstance(v, str) and v for v in data['units'].values()), 'missing unit labels')
    require(set(data['metrics']) == METRICS, 'missing or unknown metric definitions')
    require(all(isinstance(v, str) and v.strip() for v in data['metrics'].values()), 'empty metric')
    keys(data['provenance'], 'authorship source_sha256', 'provenance')
    require(bool(data['provenance']['authorship']), 'absent provenance authorship')
    hashes = data['provenance']['source_sha256']
    require(isinstance(hashes, dict) and len(hashes) >= 7, 'absent source provenance')
    for name, digest in hashes.items():
        path = Path(name)
        require(not path.is_absolute() and '..' not in path.parts, 'unsafe source path')
        require(isinstance(digest, str) and len(digest) == 64
                and all(c in '0123456789abcdef' for c in digest), 'bad source hash')
        if verify_hashes:
            resolved = (ROOT / path).resolve()
            require(ROOT in resolved.parents, 'source path escaped repository')
            require(hashlib.sha256(resolved.read_bytes()).hexdigest() == digest, 'source hash mismatch')
    require(isinstance(data['cases'], list) and data['cases'], 'no cases')
    cases = {}
    for case in data['cases']:
        keys(case, 'id family status blocked_reason length theta rho q dispersion species initial '
             'boundary time_end analysis_times hydraulics chemistry analytical provenance', 'case')
        cid = case['id']
        require(isinstance(cid, str) and re.fullmatch(r'[A-Za-z0-9_.-]+', cid)
                and cid not in cases, 'duplicate/invalid case id')
        cases[cid] = case
        require(case['status'] in ('executable', 'blocked'), 'invalid case status')
        require((case['status'] == 'blocked') == bool(case['blocked_reason']), 'missing blocked reason')
        require(bool(case['provenance']), 'missing case provenance')
        for field in ('length', 'theta', 'rho', 'time_end'):
            number(case[field], field, positive=True)
        require(case['theta'] <= 1, 'theta exceeds porosity bound')
        for field in ('q', 'dispersion'):
            number(case[field], field, minimum=0)
        species = case['species']
        require(isinstance(species, list) and 1 <= len(species) <= 3, 'unsupported species count')
        names = set()
        for item in species:
            keys(item, 'name kd rate', 'species')
            require(isinstance(item['name'], str) and item['name'] and item['name'] not in names,
                    'duplicate/invalid species name')
            names.add(item['name'])
            number(item['kd'], 'kd', minimum=0)
            number(item['rate'], 'rate', minimum=0)
        boundary = case['boundary']
        keys(boundary, 'inlet inlet_concentration outlet', 'boundary')
        require(boundary['inlet'] == 'total_flux' and boundary['outlet'] == 'zero_gradient',
                'unsupported boundary law')
        inlet = boundary['inlet_concentration']
        require(isinstance(inlet, list) and len(inlet) == len(species), 'inlet species mismatch')
        for value in inlet:
            number(value, 'inlet', minimum=0)
        times = case['analysis_times']
        validate_times(times, case['time_end'])
        initial = case['initial']
        require(isinstance(initial, dict), 'invalid initial profile')
        kind = initial.get('kind')
        if kind == 'uniform':
            keys(initial, 'kind values', 'uniform initial')
            require(len(initial['values']) == len(species), 'initial species mismatch')
            for value in initial['values']:
                number(value, 'initial concentration', minimum=0)
        elif kind == 'cosine':
            keys(initial, 'kind mean amplitude mode', 'cosine initial')
            require(len(species) == 1, 'cosine requires scalar species')
            number(initial['mean'], 'mean', minimum=0)
            number(initial['amplitude'], 'amplitude')
            integer(initial['mode'], 'mode', 1, 1)
            require(initial['mean'] >= abs(initial['amplitude']), 'negative cosine profile')
        elif kind == 'steady_plus_mode':
            keys(initial, 'kind amplitude root root_bracket', 'Robin initial')
            require(len(species) == 1 and case['q'] > 0 and case['dispersion'] > 0,
                    'Robin mapping requires scalar flow and diffusion')
            number(initial['amplitude'], 'amplitude', minimum=0)
            k = number(initial['root'], 'root', positive=True)
            lo, hi = initial['root_bracket']
            a = case['q'] / (2 * case['theta'] * case['dispersion'])
            require(0 < lo <= k <= hi < math.pi / case['length'], 'invalid eigenroot bracket')
            require(hi - lo < 1e-13 and abs(k * math.tan(k * case['length'] / 2) - a) < 1e-13,
                    'incorrect eigenroot')
        else:
            raise ValueError('unsupported initial profile')
        chem = case['chemistry']
        keys(chem, 'dispersivity molecular_diffusion tortuosity artificial_dispersion '
             'upstream_weighting epsi fraction immobile_water', 'chemistry')
        require(chem['tortuosity'] is False and chem['artificial_dispersion'] is False,
                'unsupported dispersion switches')
        require(chem['fraction'] == 1 and chem['immobile_water'] == 0 and chem['epsi'] == .5,
                'unsupported sorption or time weighting')
        require(type(chem['upstream_weighting']) is bool, 'upstream switch must be boolean')
        for field in ('dispersivity', 'molecular_diffusion'):
            number(chem[field], field, minimum=0)
        effective = chem['dispersivity'] * case['q'] / case['theta'] + chem['molecular_diffusion']
        require(math.isclose(effective, case['dispersion'], rel_tol=1e-13, abs_tol=1e-15),
                'effective dispersion mapping differs')
        hydraulic = case['hydraulics']
        keys(hydraulic, 'cos_alpha head ks theta_r alpha n pore_connectivity', 'hydraulics')
        require(hydraulic['head'] == 0 and hydraulic['cos_alpha'] in (0, 1), 'unsupported water mapping')
        number(hydraulic['ks'], 'Ks', positive=True)
        require(math.isclose(hydraulic['ks'] * hydraulic['cos_alpha'], case['q'], abs_tol=1e-15),
                'water flux mapping differs')
        for field in ('head', 'cos_alpha', 'theta_r', 'alpha', 'n', 'pore_connectivity'):
            number(hydraulic[field], field)
        require(0 <= hydraulic['theta_r'] < case['theta'] and hydraulic['alpha'] > 0
                and hydraulic['n'] > 1, 'invalid hydraulic material')
        analytic = case['analytical']
        require(isinstance(analytic, dict) and analytic.get('status') in
                ('feasible_pending_wave3_validation', 'blocked'), 'missing evaluator status')
        require(bool(analytic.get('method')) and bool(analytic.get('budget_meaning')),
                'missing reference method/budget meaning')
        keys(analytic['budget'], 'concentration_absolute inventory_scaled instantaneous_flux_scaled '
             'cumulative_scaled', 'reference budget')
        for value in analytic['budget'].values():
            number(value, 'reference budget', positive=True)
        expected_kind = {'reaction': 'uniform', 'diffusion': 'cosine',
                         'scalar_robin': 'steady_plus_mode', 'finite_chain': 'uniform'}
        if case['status'] == 'executable':
            require(case['family'] in expected_kind and kind == expected_kind[case['family']],
                    'unsupported executable family/initial mapping')
            require(analytic['status'] != 'blocked', 'executable case lacks evaluator path')
            if case['family'] in ('reaction', 'diffusion'):
                require(case['q'] == 0, 'closed component has flow')
            if case['family'] == 'reaction':
                require(case['dispersion'] == 0, 'reaction-only case has dispersion')
            if case['family'] == 'diffusion':
                require(species[0]['rate'] == 0 and case['dispersion'] > 0, 'diffusion mapping')
            if case['family'] == 'finite_chain':
                require(len(species) == 3 and inlet == [1, 0, 0]
                        and initial['values'] == [0, 0, 0], 'full-chain forcing mismatch')
    ids = set()
    require(isinstance(data['launches'], list) and data['launches'], 'empty launch matrix')
    for row in data['launches']:
        keys(row, 'id phase case_id solver precision role axis nodes dt sampling_interval '
             'analysis_times timeout_seconds max_steps estimated_output_bytes estimated_work_units '
             'status time_controls', 'launch')
        require(isinstance(row['id'], str) and row['id'] and row['id'] not in ids, 'duplicate launch')
        ids.add(row['id'])
        require(row['case_id'] in cases, 'unknown launch case')
        case = cases[row['case_id']]
        require(case['status'] == 'executable', 'blocked case scheduled')
        require(row['solver'] in ('fortran', 'rust'), 'unknown solver')
        modes = ('baseline', 'promoted-real') if row['solver'] == 'fortran' else ('f64',)
        require(row['precision'] in modes, 'unsupported precision')
        require(row['phase'] in ('benchmark', 'convergence', 'holdout'), 'unknown phase')
        require(row['role'] in ('bridge', 'benchmark', 'study', 'control', 'sensitivity', 'holdout'),
                'unknown launch role')
        require((row['role'] == 'holdout') == (row['phase'] == 'holdout'), 'holdout leakage')
        require(row['status'] == 'approved_pending_adapter_and_evaluator_gates', 'invalid launch gate')
        require(row['axis'] in ('bridge', 'component', 'space', 'time', 'precision', 'holdout'),
                'unknown study axis')
        require(row['role'] == 'holdout' or row['analysis_times'] == case['analysis_times'],
                'analysis schedule differs from case')
        controls = row['time_controls']
        keys(controls, 'policy dt_min dt_max increase decrease', 'time controls')
        expected_controls = ({'policy': 'legacy_bridge', 'dt_min': .01, 'dt_max': .5,
                              'increase': 1.2, 'decrease': .5} if row['role'] == 'bridge' else
                             {'policy': 'fixed', 'dt_min': row['dt'], 'dt_max': row['dt'],
                              'increase': 1, 'decrease': 1})
        require(controls == expected_controls, 'time controls differ from approved policy')
        integer(row['nodes'], 'nodes', 3, 1001)
        number(row['dt'], 'dt', positive=True)
        number(row['sampling_interval'], 'sampling_interval', positive=True)
        require(aligned(case['time_end'], row['dt']), 'end not aligned with dt')
        require(aligned(row['sampling_interval'], row['dt']), 'sampling not aligned with dt')
        require(aligned(case['time_end'], row['sampling_interval']), 'end not aligned with sampling')
        require(case['time_end'] / row['sampling_interval'] <= 999, 'print-time capacity exceeded')
        require(row['max_steps'] == round(case['time_end'] / row['dt']), 'step count mismatch')
        validate_times(row['analysis_times'], case['time_end'])
        require(all(aligned(t, row['sampling_interval']) for t in row['analysis_times']),
                'analysis time missing from profiles')
        require(row['timeout_seconds'] == 60, 'unapproved timeout')
        for field in ('estimated_output_bytes', 'estimated_work_units'):
            integer(row[field], field, 1, 10**12)
    budgets = data['budgets']
    keys(budgets, 'launch_cap failed_attempts_consume_cap storage_bytes_cap cost_estimate phase_caps',
         'budgets')
    require(budgets['launch_cap'] == len(ids) and budgets['failed_attempts_consume_cap'] is True,
            'launch ledger cap mismatch')
    require(budgets['storage_bytes_cap'] >= sum(r['estimated_output_bytes'] for r in data['launches']),
            'insufficient storage budget')
    require(bool(budgets['cost_estimate']), 'missing cost estimate')
    require(budgets['phase_caps'] == {
        phase: sum(r['phase'] == phase for r in data['launches'])
        for phase in ('benchmark', 'convergence', 'holdout')
    }, 'phase caps mismatch')
    solvers = sorted({r['solver'] for r in data['launches']})
    for solver in solvers:
        rows = [r for r in data['launches'] if r['case_id'] == 'example3'
                and r['solver'] == solver and r['role'] == 'study']
        require(sorted(r['nodes'] for r in rows if r['axis'] == 'space') == [251, 501, 1001],
                'missing derived spatial levels')
        require(len({r['dt'] for r in rows if r['axis'] == 'time'}) >= 3,
                'missing derived temporal levels')
    for cid in cases:
        for solver in solvers:
            rows = [r for r in data['launches'] if r['case_id'] == cid
                    and r['solver'] == solver and r['role'] == 'study']
            spatial = [r for r in rows if r['axis'] == 'space']
            temporal = [r for r in rows if r['axis'] == 'time']
            require(not spatial or len({r['dt'] for r in spatial}) == 1,
                    'spatial study changes timestep')
            require(not temporal or len({r['nodes'] for r in temporal}) == 1,
                    'temporal study changes mesh')
            for axis, group in (('space', spatial), ('time', temporal)):
                if group:
                    require(len(group) >= 3 and any(
                        r['case_id'] == cid and r['solver'] == solver
                        and r['axis'] == axis and r['role'] == 'control'
                        for r in data['launches']), 'missing refinement levels/control')
    return data


def canonical(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def row_signature(row):
    skip = {'id', 'solver', 'backend', 'status', 'precision'}
    return canonical({k: v for k, v in row.items() if k not in skip})


def load_v1_source(data):
    derived = data['provenance']['derived_from']
    keys(derived, 'path sha256', 'derived_from')
    path = Path(derived['path'])
    require(not path.is_absolute() and '..' not in path.parts, 'unsafe derived_from path')
    raw = (ROOT / path).read_bytes()
    require(hashlib.sha256(raw).hexdigest() == derived['sha256'], 'derived_from hash mismatch')
    return json.loads(raw, object_pairs_hook=unique_object)


def validate_v2(data: dict, verify_hashes: bool = True) -> dict:
    """Validate a v2 casebook: v1 cases unchanged, one backend-tagged row per v1 rust row."""
    keys(data, 'schema_version units limits provenance contracts metrics cases launches budgets', 'root')
    require(data['schema_version'] == V2, 'unsupported schema')
    require(isinstance(data['limits'], dict) and isinstance(data['budgets'], dict)
            and isinstance(data['provenance'], dict), 'malformed root sections')
    require(not (CEILING_KEYS & (set(data['limits']) | set(data['budgets']))), 'ceiling key present')
    require(data['limits'] == {'nodes': 1001, 'print_times': 999, 'timeout_seconds': 60,
                              'max_row_timeout_seconds': 300},
            'unsupported source capacity or timeout limit')
    keys(data['budgets'], 'cost_estimate policy', 'budgets')
    require(data['budgets']['policy'] == 'no launch or storage ceilings (ADR 0003)'
            and bool(data['budgets']['cost_estimate']), 'budget policy mismatch')
    keys(data['provenance'], 'authorship source_sha256 derived_from', 'provenance')
    v1 = load_v1_source(data)
    require(v1['schema_version'] == V1, 'derived_from is not a v1 casebook')
    for section in ('cases', 'units', 'contracts', 'metrics'):
        require(canonical(data[section]) == canonical(v1[section]), f'{section} differ from v1')
    require(isinstance(data['launches'], list), 'launches must be a list')
    rows = data['launches']
    for row in rows:
        require(isinstance(row, dict) and row.get('backend') == V2_BACKEND, 'row lacks backend')
        require(row.get('solver') == 'rust', 'v2 rows must be solver rust')
        require(isinstance(row.get('id'), str) and '-rfc-' in row['id']
                and '-rust-' not in row['id'], 'row id lacks rfc tag')
        expected_precision = 'f64' if row['id'] in V2_PRECISION_F64_EXCEPTIONS else 'f32'
        require(row.get('precision') == expected_precision, f"{row['id']}: precision must be {expected_precision}")
        reserved = row.get('role') == 'holdout' or row.get('phase') == 'holdout'
        require(row.get('status') == ('reserved_until_threshold_gate' if reserved
                                      else 'approved_pending_adapter_and_evaluator_gates'),
                'holdout rows must be reserved; others approved')
    validate_isolation(rows)
    validate_controls(rows)
    base = [r for r in rows if r['id'] not in V2_ADDED_ROWS]
    v1_rust = [r for r in v1['launches'] if r['solver'] == 'rust']
    require(len(base) == len(v1_rust), 'v2 rows do not cover v1 rust rows')
    by_id = {r['id']: r for r in rows}
    require(len(by_id) == len(rows), 'duplicate launch')
    for old in v1_rust:
        new = by_id.get(old['id'].replace('-rust-', '-rfc-'))
        require(new is not None, 'v1 rust row has no rfc counterpart')
        # Recorded per-row timeout deviations; every other row keeps the v1 60 s.
        old = {**old, 'timeout_seconds': V2_TIMEOUT_EXCEPTIONS.get(new['id'], old['timeout_seconds'])}
        # Compare the casebook value, not an in-memory approved override applied to this row.
        casebook_new = {**new, 'timeout_seconds': BASE_TIMEOUTS.get(new['id'], new['timeout_seconds'])}
        require(row_signature(casebook_new) == row_signature(old), 'rfc row differs from v1 rust row')
    # Reuse every v1 row, matrix and case check on a v1-shaped view of the casebook.
    # The isolating rows are validated above, not by the v1 group rules; the v1 view
    # labels every rfc row f64 because the v1 validator accepts only f64 for rust rows.
    view = copy.deepcopy(data)
    view['schema_version'] = V1
    view['limits']['automatic_retries'] = 0
    del view['limits']['max_row_timeout_seconds']
    del view['provenance']['derived_from']
    view['launches'] = [r for r in view['launches'] if r['id'] not in V2_ADDED_ROWS]
    for row in view['launches']:
        del row['backend']
        row['precision'] = 'f64'
        row['timeout_seconds'] = BASE_TIMEOUTS.get(row['id'], row['timeout_seconds'])
        if row['id'] in V2_TIMEOUT_EXCEPTIONS:
            row['timeout_seconds'] = 60  # v1 view: the deviation is validated above
        if row['status'] == 'reserved_until_threshold_gate':
            row['status'] = 'approved_pending_adapter_and_evaluator_gates'
    view['budgets'] = {
        'launch_cap': len(view['launches']), 'failed_attempts_consume_cap': True,
        'storage_bytes_cap': sum(r['estimated_output_bytes'] for r in view['launches']),
        'cost_estimate': data['budgets']['cost_estimate'],
        'phase_caps': {p: sum(r['phase'] == p for r in view['launches'])
                       for p in ('benchmark', 'convergence', 'holdout')},
    }
    validate_v1(view, verify_hashes)
    return data


def validate_isolation(rows):
    """The 15 isolating rows: table values, study role, and bracket-plus-centre geometry."""
    by_id = {r['id']: r for r in rows}
    for rid, (case, axis, nodes, dt) in V2_ISOLATION.items():
        require(rid in by_id, f'missing isolating row {rid}')
        row = by_id[rid]
        require((row['case_id'], row['axis'], row['nodes'], row['dt']) == (case, axis, nodes, dt),
                f'{rid}: values differ from the isolation table')
        require(row['role'] == 'study' and row['phase'] == 'convergence' and row['solver'] == 'rust'
                and row['precision'] == 'f32', f'{rid}: role, phase or precision')
    for case, hold_id in V2_HOLDOUT_ID.items():
        hold = by_id[hold_id]
        group = [r for r in rows if r['id'] in V2_ISOLATION and r['case_id'] == case]
        space = [r for r in group if r['axis'] == 'space']
        temporal = [r for r in group if r['axis'] == 'time']
        require(space and all(r['dt'] == hold['dt'] for r in space),
                f'{case}: space rows must hold the holdout dt')
        nodes = {r['nodes'] for r in space}
        require(hold['nodes'] in nodes and any(n < hold['nodes'] for n in nodes)
                and any(n > hold['nodes'] for n in nodes), f'{case}: space rows do not bracket holdout mesh')
        require(temporal and all(r['nodes'] == hold['nodes'] for r in temporal),
                f'{case}: time rows must hold the holdout mesh')
        dts = {r['dt'] for r in temporal}
        require(any(d < hold['dt'] for d in dts) and any(d > hold['dt'] for d in dts),
                f'{case}: time rows do not bracket holdout dt')
        require(any(r['nodes'] == hold['nodes'] and r['dt'] == hold['dt'] for r in space),
                f'{case}: centre row missing')


def validate_controls(rows):
    """The five 23R.1 other-axis controls: table values and fixed schedule."""
    by_id = {r['id']: r for r in rows}
    expected_keys = ('id phase case_id solver precision role axis nodes dt sampling_interval '
                     'analysis_times timeout_seconds max_steps estimated_output_bytes '
                     'estimated_work_units status time_controls backend')
    schedule = {
        'diffusion_cosine': ([0.25, 0.5, 1], 0.0625),
        'scalar_robin': ([0.25, 0.5, 1], 0.0625),
        'example3': ([50, 100, 200], 1),
    }
    time_end = {'diffusion_cosine': 1, 'scalar_robin': 1, 'example3': 200}
    for rid, (case, axis, nodes, dt) in V2_CONTROLS.items():
        require(rid in by_id, f'missing control row {rid}')
        row = by_id[rid]
        keys(row, expected_keys, f'control row {rid}')
        require((row['case_id'], row['axis'], row['nodes'], row['dt']) == (case, axis, nodes, dt),
                f'{rid}: values differ from the control table')
        require(row['role'] == 'control' and row['phase'] == 'convergence'
                and row['solver'] == 'rust' and row['precision'] == 'f32'
                and row['backend'] == V2_BACKEND, f'{rid}: role, phase, precision or backend')
        require(row['status'] == 'approved_pending_adapter_and_evaluator_gates',
                f'{rid}: invalid launch gate')
        require(row['timeout_seconds'] == V2_TIMEOUT_EXCEPTIONS.get(rid, 60),
                f'{rid}: unapproved timeout')
        analysis, sampling = schedule[case]
        require(row['analysis_times'] == analysis and row['sampling_interval'] == sampling,
                f'{rid}: schedule differs from the study siblings')
        require(row['time_controls'] == {'policy': 'fixed', 'dt_min': dt, 'dt_max': dt,
                                         'increase': 1, 'decrease': 1},
                f'{rid}: time controls differ from approved policy')
        require(row['max_steps'] == round(time_end[case] / dt), f'{rid}: step count mismatch')
        for field in ('estimated_output_bytes', 'estimated_work_units'):
            integer(row[field], field, 1, 10**12)


def validate_times(times, end):
    require(isinstance(times, list) and times, 'empty times')
    for time in times:
        number(time, 'time', positive=True)
    require(times == sorted(set(times)) and times[-1] <= end, 'mismatched or duplicate times')


def self_test_v2(data):
    validate(data)
    checks = 1
    mutations = [
        lambda d: d['launches'][0].pop('backend'),
        lambda d: d['launches'][0].update(backend='fv-be'),
        lambda d: d['launches'].pop(),
        lambda d: d['cases'][0].update(length=d['cases'][0]['length'] + 1),
        lambda d: d['budgets'].update(launch_cap=10),
        lambda d: d['limits'].update(automatic_retries=0),
        lambda d: d['launches'][-1].update(status='approved_pending_adapter_and_evaluator_gates'),
        lambda d: d['launches'][0].update(solver='fortran'),
        lambda d: d['launches'][0].update(id='x-rust-baseline'),
        lambda d: d['launches'][0].update(nodes=1002),
    ]
    for index, mutate in enumerate(mutations):
        bad = copy.deepcopy(data)
        mutate(bad)
        try:
            validate(bad)
        except (ValueError, KeyError, TypeError, OSError):
            checks += 1
        else:
            raise AssertionError(f'v2 negative test {index + 1} was accepted')
    print(f'{checks} v2 contract checks passed; zero solver launches')


def self_test(data):
    if data.get('schema_version') == V2:
        return self_test_v2(data)
    validate(data)
    checks = 1
    def alter_axis(data, axis):
        row = next(r for r in data['launches'] if r['case_id'] == 'scalar_robin'
                   and r['solver'] == 'rust' and r['role'] == 'study' and r['axis'] == axis)
        if axis == 'space':
            row['dt'] *= 2
            row['time_controls']['dt_min'] = row['dt']
            row['time_controls']['dt_max'] = row['dt']
            row['max_steps'] //= 2
        else:
            row['nodes'] += 2

    mutations = [
        lambda d: d.pop('provenance'),
        lambda d: d['metrics'].pop('reaction'),
        lambda d: d['launches'][0].update(nodes=1002),
        lambda d: d['launches'][0].update(nodes=True),
        lambda d: d['launches'][0].update(dt=0),
        lambda d: d['launches'][0].update(dt=-1),
        lambda d: d['launches'][0].update(dt=float('nan')),
        lambda d: d['launches'][0].update(analysis_times=[50, 50]),
        lambda d: d['launches'][0].update(analysis_times=[51]),
        lambda d: d['cases'][0]['initial'].update(values=[1]),
        lambda d: d['cases'][0]['boundary'].update(inlet_concentration=[1]),
        lambda d: d['cases'][0]['species'][0].update(rate=-1),
        lambda d: d['cases'][0]['species'][0].update(kd=float('inf')),
        lambda d: d['cases'][0]['chemistry'].update(molecular_diffusion=1),
        lambda d: d['cases'][0]['hydraulics'].update(cos_alpha=1),
        lambda d: d['cases'][0].update(status='blocked', blocked_reason=None),
        lambda d: d['launches'][0].update(precision='quad'),
        lambda d: d['launches'][0].update(timeout_seconds=61),
        lambda d: d['launches'][0].update(sampling_interval=.1),
        lambda d: d['launches'][0].update(max_steps=1),
        lambda d: d['launches'][0].update(role='holdout'),
        lambda d: d['launches'][0].update(case_id='manufactured_forcing'),
        lambda d: d['budgets'].update(launch_cap=10000),
        lambda d: d['budgets'].update(storage_bytes_cap=1),
        lambda d: d['provenance']['source_sha256'].update({'../escape': '0' * 64}),
        lambda d: d['provenance']['source_sha256'].update({'src_reference/SOLUTE.FOR': '0' * 64}),
        lambda d: d['cases'][4]['initial'].update(root=.8),
        lambda d: d['cases'][0]['hydraulics'].update(pore_connectivity=float('nan')),
        lambda d: d['launches'][0].update(axis='unknown'),
        lambda d: d['launches'][0]['time_controls'].update(dt_max=.1),
        lambda d: alter_axis(d, 'space'),
        lambda d: alter_axis(d, 'time'),
    ]
    for index, mutate in enumerate(mutations):
        bad = copy.deepcopy(data)
        mutate(bad)
        try:
            validate(bad)
        except (ValueError, KeyError, TypeError, OSError):
            checks += 1
        else:
            raise AssertionError(f'negative test {index + 1} was accepted')
    try:
        json.loads('{"x":1,"x":2}', object_pairs_hook=unique_object)
    except ValueError:
        checks += 1
    else:
        raise AssertionError('duplicate keys accepted')
    print(f'{checks} contract checks passed; zero solver launches')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('cases', nargs='?', type=Path, default=DEFAULT_CASES)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    try:
        data = load_cases(args.cases)
        if args.self_test:
            self_test(data)
        else:
            validate(data)
            print(f"Valid {data['schema_version']}: {len(data['cases'])} cases, "
                  f"{len(data['launches'])} conditional launches; no scientific thresholds assigned")
    except (ValueError, KeyError, TypeError, OSError, OverflowError) as error:
        parser.exit(2, f'Invalid solute verification contract: {error}\n')


if __name__ == '__main__':
    main()

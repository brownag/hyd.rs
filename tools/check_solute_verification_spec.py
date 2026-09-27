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


def validate(data: dict, verify_hashes: bool = True) -> dict:
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
    for solver in ('fortran', 'rust'):
        rows = [r for r in data['launches'] if r['case_id'] == 'example3'
                and r['solver'] == solver and r['role'] == 'study']
        require(sorted(r['nodes'] for r in rows if r['axis'] == 'space') == [251, 501, 1001],
                'missing derived spatial levels')
        require(len({r['dt'] for r in rows if r['axis'] == 'time'}) >= 3,
                'missing derived temporal levels')
    for cid in cases:
        for solver in ('fortran', 'rust'):
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


def validate_times(times, end):
    require(isinstance(times, list) and times, 'empty times')
    for time in times:
        number(time, 'time', positive=True)
    require(times == sorted(set(times)) and times[-1] <= end, 'mismatched or duplicate times')


def self_test(data):
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

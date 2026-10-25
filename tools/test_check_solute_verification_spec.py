import copy
import unittest

from tools import check_solute_verification_spec as spec

V1 = spec.ROOT / 'reference/solute_verification/CASES.json'
V2 = spec.ROOT / 'reference/solute_verification/CASES_V2.json'


class SpecCheckerTests(unittest.TestCase):
    def setUp(self):
        self.v1 = spec.load_cases(V1)
        self.v2 = spec.load_cases(V2)

    def rejects(self, mutate):
        bad = copy.deepcopy(self.v2)
        mutate(bad)
        with self.assertRaises((ValueError, KeyError, TypeError)):
            spec.validate(bad)

    def test_v2_cases_match_v1(self):
        spec.validate(self.v2)
        for old, new in zip(self.v1['cases'], self.v2['cases']):
            self.assertEqual(spec.canonical(old), spec.canonical(new), old['id'])
        for key in ('units', 'contracts', 'metrics'):
            self.assertEqual(spec.canonical(self.v1[key]), spec.canonical(self.v2[key]))
        self.rejects(lambda d: d['cases'][0].update(length=d['cases'][0]['length'] + 1))

    def test_v2_launch_rows_cover_v1_rust_rows(self):
        rust = [r for r in self.v1['launches'] if r['solver'] == 'rust']
        base = [r for r in self.v2['launches'] if r['id'] not in spec.V2_ISOLATION]
        self.assertEqual(len(rust), len(base))
        ids = {r['id'] for r in self.v2['launches']}
        self.assertTrue(all(r['id'].replace('-rust-', '-rfc-') in ids for r in rust))
        self.assertFalse(any(r['solver'] == 'fortran' for r in self.v2['launches']))
        self.rejects(lambda d: d['launches'].pop())

    def test_v2_rows_carry_backend(self):
        self.assertTrue(all(r['backend'] == 'fortran-compatible' and r['solver'] == 'rust'
                            for r in self.v2['launches']))
        self.rejects(lambda d: d['launches'][0].pop('backend'))
        self.rejects(lambda d: d['launches'][0].update(backend='fv-be'))

    def test_v2_holdouts_reserved(self):
        hold = [r for r in self.v2['launches'] if r['role'] == 'holdout']
        self.assertEqual(3, len(hold))
        self.assertTrue(all(r['status'] == 'reserved_until_threshold_gate' for r in hold))
        self.rejects(lambda d: next(r for r in d['launches'] if r['role'] == 'holdout').update(
            status='approved_pending_adapter_and_evaluator_gates'))
        self.rejects(lambda d: d['budgets'].update(launch_cap=1))
        self.rejects(lambda d: d['limits'].update(automatic_retries=0))

    def test_v2_row_timeout_exception(self):
        sc = 'convergence-example3-space_control-rfc-baseline'
        row = lambda d, i: next(r for r in d['launches'] if r['id'] == i)
        self.assertEqual(180, row(self.v2, sc)['timeout_seconds'])
        self.assertEqual(300, self.v2['limits']['max_row_timeout_seconds'])
        self.rejects(lambda d: row(d, sc).update(timeout_seconds=60))
        self.rejects(lambda d: row(d, sc).update(timeout_seconds=301))
        self.rejects(lambda d: d['launches'][0].update(timeout_seconds=180))
        self.rejects(lambda d: d['limits'].update(max_row_timeout_seconds=301))
        self.rejects(lambda d: d['limits'].pop('max_row_timeout_seconds'))

    def test_v2_relabel_f32_is_only_precision_change(self):
        v1_rust = {r['id']: r for r in self.v1['launches'] if r['solver'] == 'rust'}
        for row in self.v2['launches']:
            if row['id'] in spec.V2_ISOLATION:
                continue
            old = v1_rust[row['id'].replace('-rfc-', '-rust-')]
            fields = {k for k in set(row) | set(old) if row.get(k) != old.get(k)}
            allowed = {'id', 'solver', 'backend', 'status', 'precision'}
            if row['id'] in spec.V2_TIMEOUT_EXCEPTIONS:
                allowed.add('timeout_seconds')
            self.assertLessEqual(fields, allowed, row['id'])
            expected = 'f64' if row['id'] in spec.V2_PRECISION_F64_EXCEPTIONS else 'f32'
            self.assertEqual(expected, row['precision'], row['id'])
            self.assertEqual(old['precision'], 'f64', row['id'])
        self.assertEqual(34, sum(r['precision'] == 'f32' and r['id'] not in spec.V2_ISOLATION
                                 for r in self.v2['launches']))
        self.assertEqual(spec.V2_PRECISION_F64_EXCEPTIONS,
                         {r['id'] for r in self.v2['launches'] if r['precision'] == 'f64'})

    def test_v2_new_rows_have_backend_and_rfc_id(self):
        by_id = {r['id']: r for r in self.v2['launches']}
        self.assertEqual(15, len(spec.V2_ISOLATION))
        for rid in spec.V2_ISOLATION:
            row = by_id[rid]
            self.assertEqual('fortran-compatible', row['backend'], rid)
            self.assertIn('-rfc-', rid)
            self.assertEqual('rust', row['solver'], rid)
        self.rejects(lambda d: next(r for r in d['launches']
                                    if r['id'] in spec.V2_ISOLATION).pop('backend'))
        self.rejects(lambda d: next(r for r in d['launches']
                                    if r['id'] in spec.V2_ISOLATION).update(nodes=66))

    def test_v2_holdout_rows_still_reserved(self):
        hold = [r for r in self.v2['launches'] if r['role'] == 'holdout']
        self.assertEqual(sorted(spec.V2_HOLDOUT_ID.values()), sorted(r['id'] for r in hold))
        self.assertTrue(all(r['status'] == 'reserved_until_threshold_gate' for r in hold))
        self.assertTrue(all(r['precision'] == 'f64' for r in hold))
        self.assertTrue(all(r['phase'] == 'holdout' for r in hold))

    def test_v2_new_rows_not_holdout(self):
        for row in self.v2['launches']:
            if row['id'] in spec.V2_ISOLATION:
                self.assertEqual('study', row['role'], row['id'])
                self.assertEqual('convergence', row['phase'], row['id'])
                self.assertNotEqual('holdout', row['role'])
        self.rejects(lambda d: next(r for r in d['launches']
                                    if r['id'] in spec.V2_ISOLATION).update(role='holdout'))

    def test_bridge_print_rule_selects_only_bridge_rows(self):
        # Harness rule (23.3.1): case example3, time_end 200, sampling 50, analysis [50,100,200].
        # The Rust harness reads only rust requests, so the rule is scoped to solver rust.
        # CASES.json also holds the fortran bridge row; the rule selects it too, but it is never
        # a Rust request. Across both casebooks the rust selection is exactly the two bridge rows.
        time_end = {c['id']: c['time_end'] for c in self.v1['cases']}
        selected = set()
        for casebook in (self.v1, self.v2):
            for r in casebook['launches']:
                if (r['solver'] == 'rust' and r['case_id'] == 'example3'
                        and time_end[r['case_id']] == 200 and r['sampling_interval'] == 50
                        and r['analysis_times'] == [50, 100, 200]):
                    selected.add(r['id'])
        self.assertEqual({'benchmark-example3-bridge-rust-baseline',
                          'benchmark-example3-bridge-rfc-baseline'}, selected)

    def test_v1_still_validates(self):
        spec.validate(self.v1)
        spec.self_test(self.v1)
        bad = copy.deepcopy(self.v1)
        bad['budgets']['launch_cap'] = 10000
        with self.assertRaises(ValueError):
            spec.validate(bad)


if __name__ == '__main__':
    unittest.main()

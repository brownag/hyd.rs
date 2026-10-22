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
        self.assertEqual(len(rust), len(self.v2['launches']))
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

    def test_v1_still_validates(self):
        spec.validate(self.v1)
        spec.self_test(self.v1)
        bad = copy.deepcopy(self.v1)
        bad['budgets']['launch_cap'] = 10000
        with self.assertRaises(ValueError):
            spec.validate(bad)


if __name__ == '__main__':
    unittest.main()

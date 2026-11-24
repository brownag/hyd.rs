# M6 vapor ORCH30 criteria proposal (30.5.2)

Status: PROPOSAL for review at G30.6. Nothing here is written to `reference/vapor_fortran/criteria.json`. No tolerance is changed. Every pair entry uses factor 2.0, the factor of the model entries `dry_on_off_*`.

## Model and rule
- Pair entries copy `dry_on_off_*`: rule `any_print_time_after_initial`, keys `case, control, factor_of_tolerance, field, rule, unit` (validator `tools/compare_vapor_reference.py`, `validate_criteria`).
- `_on` nonzero entries copy `vapor_fields_nonzero`: rule `max_abs_exceeds_zero_floor_each_case`, factor 1.0, keys `cases, factor_of_tolerance, fields, rule, unit`. New ids are used. The existing `vapor_fields_nonzero.cases` is not extended.
- The field tolerances are head abs 0.5 cm (rel 0.02), liquid_moisture abs 0.00105, temperature_c abs 0.05. The headroom column below uses the absolute part only (2.0 x tolerance: head 1.0 cm, liquid_moisture 0.0021, temperature_c 0.1 degC).

## Evidence
On/off max abs difference over all nodes and print times after t=0 (0, 6, 24, 72, 240 h). Computed from `reference/vapor_fortran/fixtures/<pair>_{on,off}.json` with a scratchpad script. The wet-pair values match `.development/29.4.2/onoff.txt`.

| Pair (case / control) | head (cm) | liquid_moisture | temperature_c (degC) |
|---|---|---|---|
| vapor_dry_isothermal_on / _off | 0.010 | 0.0 | 0.0 |
| vapor_free_drainage_on / _off | 3.215 | 0.0024 | 0.40 |
| vapor_closed_flux_on / _off | 0.897 | 0.0009 | 0.27 |
| vapor_flux_known_on / _off | 0.877 | 0.0008 | 0.27 |
| threshold (2.0 x tol) | 1.0 | 0.0021 | 0.1 |

## Flagged shortfalls for G30.6 (measured difference below 2.0 x tolerance)
Entries are still proposed at factor 2.0, as directed. They would fail the check as written, so G30.6 must decide whether to accept, drop or revisit each one.
1. `dry_iso_on_off_head`: 0.010 vs 1.0
2. `dry_iso_on_off_liquid_moisture`: 0.0 vs 0.0021 (no on/off difference)
3. `dry_iso_on_off_temperature_c`: 0.0 vs 0.1 (no on/off difference)
4. `closed_flux_on_off_head`: 0.897 vs 1.0
5. `closed_flux_on_off_liquid_moisture`: 0.0009 vs 0.0021
6. `flux_known_on_off_head`: 0.877 vs 1.0
7. `flux_known_on_off_liquid_moisture`: 0.0008 vs 0.0021

Entries that pass: all three `free_drain_*` entries, and `closed_flux_on_off_temperature_c` and `flux_known_on_off_temperature_c`.
Note: the head threshold ignores the 0.02 relative term, which only raises the real threshold further. The dry isothermal pair has no thermal drive, so it is expected to show no discrimination.

## Proposed pair entries
```json
{
  "dry_iso_on_off_head": {"case": "vapor_dry_isothermal_on", "control": "vapor_dry_isothermal_off", "factor_of_tolerance": 2.0, "field": "head", "rule": "any_print_time_after_initial", "unit": "cm"},
  "dry_iso_on_off_liquid_moisture": {"case": "vapor_dry_isothermal_on", "control": "vapor_dry_isothermal_off", "factor_of_tolerance": 2.0, "field": "liquid_moisture", "rule": "any_print_time_after_initial", "unit": "cm3/cm3"},
  "dry_iso_on_off_temperature_c": {"case": "vapor_dry_isothermal_on", "control": "vapor_dry_isothermal_off", "factor_of_tolerance": 2.0, "field": "temperature_c", "rule": "any_print_time_after_initial", "unit": "degC"},
  "free_drain_on_off_head": {"case": "vapor_free_drainage_on", "control": "vapor_free_drainage_off", "factor_of_tolerance": 2.0, "field": "head", "rule": "any_print_time_after_initial", "unit": "cm"},
  "free_drain_on_off_liquid_moisture": {"case": "vapor_free_drainage_on", "control": "vapor_free_drainage_off", "factor_of_tolerance": 2.0, "field": "liquid_moisture", "rule": "any_print_time_after_initial", "unit": "cm3/cm3"},
  "free_drain_on_off_temperature_c": {"case": "vapor_free_drainage_on", "control": "vapor_free_drainage_off", "factor_of_tolerance": 2.0, "field": "temperature_c", "rule": "any_print_time_after_initial", "unit": "degC"},
  "closed_flux_on_off_head": {"case": "vapor_closed_flux_on", "control": "vapor_closed_flux_off", "factor_of_tolerance": 2.0, "field": "head", "rule": "any_print_time_after_initial", "unit": "cm"},
  "closed_flux_on_off_liquid_moisture": {"case": "vapor_closed_flux_on", "control": "vapor_closed_flux_off", "factor_of_tolerance": 2.0, "field": "liquid_moisture", "rule": "any_print_time_after_initial", "unit": "cm3/cm3"},
  "closed_flux_on_off_temperature_c": {"case": "vapor_closed_flux_on", "control": "vapor_closed_flux_off", "factor_of_tolerance": 2.0, "field": "temperature_c", "rule": "any_print_time_after_initial", "unit": "degC"},
  "flux_known_on_off_head": {"case": "vapor_flux_known_on", "control": "vapor_flux_known_off", "factor_of_tolerance": 2.0, "field": "head", "rule": "any_print_time_after_initial", "unit": "cm"},
  "flux_known_on_off_liquid_moisture": {"case": "vapor_flux_known_on", "control": "vapor_flux_known_off", "factor_of_tolerance": 2.0, "field": "liquid_moisture", "rule": "any_print_time_after_initial", "unit": "cm3/cm3"},
  "flux_known_on_off_temperature_c": {"case": "vapor_flux_known_on", "control": "vapor_flux_known_off", "factor_of_tolerance": 2.0, "field": "temperature_c", "rule": "any_print_time_after_initial", "unit": "degC"}
}
```

## Proposed `_on` nonzero entries (new ids)
```json
{
  "closed_vapor_fields_nonzero": {"cases": ["vapor_closed_flux_on", "vapor_flux_known_on"], "factor_of_tolerance": 1.0, "fields": ["con_vh", "con_vt", "v_vapor", "v_vap_iso", "v_vap_term"], "rule": "max_abs_exceeds_zero_floor_each_case", "unit": "field"},
  "dry_iso_vapor_fields_nonzero": {"cases": ["vapor_dry_isothermal_on"], "factor_of_tolerance": 1.0, "fields": ["con_vh", "con_vt", "v_vapor", "v_vap_iso", "v_vap_term"], "rule": "max_abs_exceeds_zero_floor_each_case", "unit": "field"},
  "free_drain_vapor_fields_nonzero": {"cases": ["vapor_free_drainage_on"], "factor_of_tolerance": 1.0, "fields": ["con_vh", "con_vt", "v_vapor", "v_vap_iso", "v_vap_term"], "rule": "max_abs_exceeds_zero_floor_each_case", "unit": "field"}
}
```
Evidence: max abs of each vapor field in the `_on` fixtures (units as in the fixtures).

| Case | con_vh | con_vt | v_vapor | v_vap_iso | v_vap_term |
|---|---|---|---|---|---|
| vapor_dry_isothermal_on | 2.33e-09 | 1.906e-03 | 7.28e-09 | 1.82e-11 | 7.27e-09 |
| vapor_free_drainage_on | 1.05e-09 | 8.61e-04 | 8.03e-04 | 1.36e-09 | 8.03e-04 |
| vapor_closed_flux_on | 6.55e-10 | 5.48e-04 | 5.29e-04 | 7.02e-10 | 5.29e-04 |
| vapor_flux_known_on | 6.48e-10 | 5.42e-04 | 5.23e-04 | 6.80e-10 | 5.23e-04 |

Caveat for G30.6: the zero floors are `case_time_field_max_fraction` (0.001 of the case/time/field max), and `v_vapor` and `v_vap_term` carry a temperature-precision floor. The rule can only be confirmed by running the comparer on these fixtures. The dry isothermal `v_vapor`, `v_vap_iso` and `v_vap_term` values are about 1e-8 to 1e-11, so the pass should be confirmed before the entry is accepted.

## Not changed
`dry_on_off_*`, `vapor_dry_off_control`, `vapor_fields_nonzero`, `heatbc_*`, all tolerances, and `contract_sha256`. Adding entries will require the usual criteria revision and hash update under G30.6 approval.

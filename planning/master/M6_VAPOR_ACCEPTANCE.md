# M6 vapor capability acceptance

Status: **COMPLETE WITH EXCEPTION** for the bounded first increment (G19).
Decision date: 2026-11-12. Vapor flow coupled to heat, first-increment selection only.

This record states what the vapor-aware path supports, what it rejects, the
evidence for each claim, and what is not claimed. It follows the structure of
the historical [heat acceptance](M6_HEAT_ACCEPTANCE.md), which is unchanged. It
accepts the capability for the stated selection and evidence only. It does not
establish universal accuracy, and no failed check is converted into a pass.
Authority for the selection, equations and criteria is
[M6_VAPOR_CONTRACT.md](M6_VAPOR_CONTRACT.md); the full capability map is
[M6_VAPOR_COVERAGE.md](M6_VAPOR_COVERAGE.md). Evidence source: `planning/STATE_25.md`.

## Scope and capability

Supported selection (contract `## Supported selection`): water and heat with
`lVapor=true`, one domain, van Genuchten `iModel=0`, `iEnhanc=1`, default thermal
conductivity `iCampbell=0`, constant Dirichlet temperature at both ends, fixed
water pressure heads at both ends, two source-ordered water-to-heat passes per
step, atomic acceptance with retry at `dt/3`. The vapor-aware entry accepts only
this selection; heat-only entries reject `lVapor=true` before mutation.

## Exception

Eighteen isothermal `v_vapor`/`v_vap_term` rows (case `vapor_isothermal_head_gradient`,
9 rows each, worst err/bound 1.76, errors up to 1.97e-12) fail criteria v2
against the pinned fixture. They are pinned as `KNOWN_SOURCE_PRECISION_FAILURES`
in `crates/hydrus-core/tests/vapor_fortran_parity.rs`; the test passes only if
the failing set equals that list exactly, and fails if any listed row heals.
The cause is the source single-precision `TempN` (`TEMPER.FOR:55`), diagnosed in
[M6_VAPOR_PRECISION_DIAGNOSIS.md](M6_VAPOR_PRECISION_DIAGNOSIS.md) (commit
`b4913c3`): Rust matches a double-precision rebuild of the source (<= 1e-9
relative dT). These rows are **not claimed as matched**.

**Criteria v2 was a post-failure, owner-approved revision of v1.** The user
approved it on 2026-11-12 after seeing Wave 7 failures, as an explicit override
of the no-widening rule (contract `### Criteria revision v2`). It changes only
the `v_vapor`, `v_vap_term` and `v_total` bounds (a source-precision floor).
`reference/vapor_fortran/criteria.json` v1 (introduced in `bde9c7f`) has SHA-256
`d645cae2a0ba44219eed7b53185abb09a1815ee481041f4e7a3b9fbc0cbbf008`; v2
(introduced in `21101ca`) has SHA-256
`1d8dc73eac5fe66c1f8bbfd8c5108781c5efe92509c9689467c3dbf19cc4a7c0`. Contract
revisions: `8e561a5` (v1), `a4545e4` (v2). The v1 verdicts were retained. Even
under v2, 9 + 9 rows failed by up to 1.76x the bound; the user then chose to
record them as explained rather than widen again.

## Supported and rejected

Supported: the selection above. Rejected with typed errors (tests in
`crates/hydrus-core/tests/vapor_config_contract.rs`): enhancement off, Campbell,
heat flux / zero-gradient / sinusoidal boundaries, non-head water boundaries,
root uptake or nonzero sink, `lWDep`, `lEnBal`, multi-domain, other hydraulic
models, hysteresis, dual porosity, chemistry, density and centrifugal options.
Each row's status and follow-on goal is in the coverage matrix.

## Evidence

Four classes are kept separate.

### Independent checks (no production outputs as expectations)

- `cargo test -p hydrus-core --test vapor_analytical`: 6 tests (hand-worked
  kernel values, zero limits, conversions).
- `cargo test -p hydrus-core --test vapor_balance`: 3 tests. Interior water
  residual max 3.4e-16 and heat 2.6e-16 relative (bound 1e-9); fresh-coefficient
  closure 6.1e-13 is reported only; vapor/liquid transport ~1.06e3 and storage
  ~1.66e-3 asserted >= 1e-6. Commit `9a864c2`.

### Pinned-kernel comparison (kernel only, not trajectory)

- `vapor_kernel_matches_pinned_fortran` in `crates/hydrus-core/tests/vapor_fortran_parity.rs`
  against `reference/vapor_fortran/kernel_fixtures/vapor_content.json`, built from
  `reference/vapor_fortran/kernel_harness/` (`reference/vapor_fortran/kernel_harness/vapor_content_driver.f90`,
  `reference/vapor_fortran/kernel_harness/requests.json`) by `tools/run_vapor_kernel_reference.py`. Three request points
  (dry-cold, moist-warm, length-conversion): content, adjusted capacity and
  capacity correction all pass; worst err/bound 0.199 (correction), 0.136
  (content), 0.0329 (capacity). Full-program vapor content output does not exist
  in the source, so content is **not** compared in trajectories.

### Full trajectories against the pinned fixtures

Fixtures: `reference/vapor_fortran/fixtures/` (`vapor_isothermal_head_gradient`,
`vapor_thermal_gradient`, `vapor_off_control`, `vapor_dry_thermal_gradient`,
`vapor_dry_off_control`, `dry_discrimination`), decks under
`reference/vapor_fortran/cases/`, producer `tools/run_vapor_reference.py`.
Rust runs: `vapor_fortran_parity` (10 tests, commit `95ee096`); standalone
comparator `tools/compare_vapor_reference.py`; both consumers agree exactly on
the failing rows. Latest verdicts: `.development/25.7.1/attempt-20261112T205416Z/verdicts.tsv`
(untracked; 7521 rows), summarized as worst err/bound per field:

| Case | Fields | Worst err/bound | Failing rows |
|---|---|---|---|
| vapor_isothermal_head_gradient | head, moisture, temperature, con, con_lt, con_vh, con_vt, v_liquid, v_total, v_vap_iso | <= 0.062 | 0 |
| vapor_isothermal_head_gradient | `v_vapor`, `v_vap_term` | 1.76 | 9 + 9 (the exception) |
| vapor_thermal_gradient | all fields | 0.099 (temperature); all other fields <= 0.003 | 0 |
| vapor_off_control | head, moisture, temperature | <= 0.0995 | 0 |
| vapor_dry_thermal_gradient | all fields | 0.183 (`v_liquid`); temperature 0.0996; all other fields <= 0.132 | 0 |
| vapor_dry_off_control | head, moisture, temperature | <= 0.034 | 0 |

Discrimination (nonzero effect): dry fixtures discriminate; reference and
candidate ratios to 2x tolerance are head 21.4, moisture 9.3, temperature 28.8,
and the second coupling pass changes temperature by up to 6.3e-3 C
(`dry_discrimination_and_nonzero_vapor_fields`, `second_pass_changes_temperatures`).
Wet fixtures are non-discriminating for head, moisture and temperature (only
vapor fields discriminate there); they are regression evidence only. Vapor-only
fields are nonzero above their zero floor in every vapor-enabled case.

### Unit, configuration, state, driver and output tests

`cargo test -p hydrus-core --test <name>`: `vapor_config_contract` (6),
`vapor_kernels` (4), `vapor_water_rows` (4), `vapor_heat_transport` (4),
`vapor_state` (2), `vapor_driver_order` (3), `vapor_output` (6); inline
`cargo test -p hydrus-core --lib water_solver::vapor_trial_tests` and
`heat::vapor::driver`. Python: `tools/test_run_vapor_kernel_reference.py`,
`tools/test_run_vapor_reference.py`, `tools/test_compare_vapor_reference.py`
(`python3 -m unittest discover -s tools -p 'test_*.py'`).

Integration commits: `7849c64`, `1790b62`, `29392a6`, `29f4942`, `232ab4e`,
`2f6a94c`, `6c5776a`, `b8e0544`, `49fc1b8`, `9a864c2`, `494c040`, `366566f`,
`1a57124`, `4560583`, `95ee096`; record `08590ca`.

Wave 7 serial gates (after `95ee096`): build PASS; workspace tests PASS (51
suites, 584 passed, 0 failed, 2 ignored); fmt PASS; Clippy PASS;
`hydraulic_curves` 9 passed; Python PASS (447 tests).

## Known limits and disclosures

- The source accepts a non-converged water solution once `dt <= dtMin`; Rust
  returns a typed `NonConverged` error instead.
- Rust parity runs use a constant declared `dt` of 0.1 h. Task 25.7.3 found the
  source also took 240 steps of 0.1 h in the isothermal case.
- Retry factor is 3 (source `dt/3`).
- `hSat` is computed in f64 without the source's `sngl` rounding; the D2
  saturated-branch test cannot separate that rule from `EpsTh` (weak).
- The committed 25.2.2 case directories lack `LEVEL_01.DIR`, which the producer
  requires, so they cannot be rerun by the producer as committed.
- Kernel evidence is pinned-kernel only; there is no trajectory content evidence
  and no full-program vapor content output.
- `verdicts.tsv` and the 25.7.3 diagnostic roots are untracked.
- Arithmetic is f64; no identical-arithmetic or bitwise claim.

## Next triggers

- **G22** stays open: select the next dependency-ready coverage row by
  dependencies and user priority; no execution is authorized here.
- **G20** may start only conditional on the vapor interactions it requires
  (atmospheric/flux boundaries are not accepted by this increment).
- **G23** remains separately deferred.

## Not claimed

Complete vapor coverage (G22), energy balance (G20), `lWDep` (G23); Campbell,
enhancement-off, flux/zero-gradient/sinusoidal heat boundaries, flux water
boundaries, root uptake; the 18 isothermal rows as matched; full-program vapor
content parity; bitwise or whole-program parity; physical experimental
validation; acceptance of ADR 0021.

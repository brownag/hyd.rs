# M6 heat capability acceptance

Status: capability acceptance record for Orchestration 24, task 24.6.1.
Decision date: 2026-11-11. Heat and temperature only.

This document describes what the implemented heat capability supports, what it
rejects, the evidence for each exit criterion, and what is not claimed. It
accepts the capability for the stated configurations and evidence only. It does
not establish universal accuracy, and it does not convert any failed or
unmeasured check into a pass. The "no universal tolerance" rule follows
`planning/master/M5V_M6_HANDOFF_DECISION.md`: acceptance is capability-scoped,
and no M5-derived threshold applies to M6.

Authority for the supported set, the rejected set, the tolerances and the
not-claimed list is `planning/master/M6_HEAT_CONTRACT.md`. This document cites
it and does not restate it in full.

## Scope and capability

- Heat-only. Vapor flow, the energy balance (`lEnBal`), and water-flow feedback
  from temperature (`lWDep`) are out of scope. The contract's `## Supported`
  table is the selection rule. Every item must hold on the effective value.
- Implemented in `crates/hydrus-core/src/heat/` (`boundary.rs`, `capacity.rs`,
  `transport.rs`, `balance.rs`, `state.rs`, `config.rs`, `driver.rs`).
- Coupling is one-way. Heat is advanced after the accepted water step and reads
  the post-step water state (`heat_uses_post_step_water`,
  `heat_driver_order.rs`).
- Both conductivity branches are in scope: the default branch (`iCampbell=0`)
  and the Campbell branch (`iCampbell=1`). The Campbell inclusion follows
  user decision D13 (2026-11-11, `planning/EXECUTION_PLAN_24.md`) and the
  contract's `## Supported` table. The Campbell branch carries the SD-006 scale
  factor, reproduced as written (see `## Corrections and decisions`).
- Water model: the heat parity runs use the table water model. The accepted
  water product keeps its analytic default. See `## Corrections and decisions`.

## Supported and rejected

Supported and rejected configurations are defined in the contract. Summary
with the test that pins each side:

Supported (contract `## Supported`):

| Item | Pinned by |
|---|---|
| Heat-only selection, `lVapor=false`, `lEnBal=false`, `lWDep=false`, `iDualPor=0`, `iModel=0`, one domain | `accepts_heat_only_dirichlet_selection` (`heat_config_contract.rs`) |
| Campbell conductivity, both ways (`iCampbell` 0 and 1) | `accepts_campbell_conductivity_both_ways` |
| Top Dirichlet and top flux (`kTopT>0`, `kTopT<0`) | `top_dirichlet_is_level_independent`, `top_flux_level_first_heat_flux_positive` (`heat_boundary.rs`) |
| Bottom Dirichlet, bottom flux (`kBotT>0`, `kBotT<0`) | `bottom_dirichlet_sets_d_e_f_at_both_levels`, `bottom_flux_level_first_writes_f_only` |
| Flux top with flux bottom | `accepts_flux_top_and_flux_bottom` |
| Bottom zero gradient (`kBotT==0`) | `accepts_zero_gradient_bottom`, `bottom_zero_gradient_sets_minus_one_one_zero`. Code-supported only. See `## Known limits`. |

Rejected (contract `## Rejected`; a catch-all rule rejects everything not listed
as supported):

| Rejected | Pinned by |
|---|---|
| Vapor (`lVapor=true`) | `rejects_vapor` |
| Energy balance (`lEnBal=true`) | `rejects_energy_balance` |
| Dual porosity (`iDualPor != 0`) | `rejects_dual_porosity` |
| Non-van Genuchten model (`iModel != 0`) | `rejects_non_van_genuchten_model` |
| Domain count other than one | `rejects_multi_domain` |
| Temperature-dependent water (`lWDep=true`) | `rejects_temperature_dependent_water` |
| Top kind `kTopT == 0` | `rejects_zero_top_boundary_kind`, `top_boundary_kind_zero_is_rejected` |
| Sinusoidal top (`tPeriod > 0`) | `rejects_sinusoidal_top_boundary`, `sinusoidal_top_is_rejected` |
| Zero heat capacity at runtime (`Cap(i)==0`) | `rejects_zero_capacity` (`heat_capacity.rs`) |
| Unsupported configuration rejected before the water step | `unsupported_config_rejected_before_water_step` (`heat_driver_order.rs`) |

## Evidence

Commands were run on 2026-11-11 from the worktree on branch
`orchestration/24.6.1`, based on HEAD `0e31699`. Build output went to a scratch
target directory (`CARGO_TARGET_DIR=/home/andrew/AI/hyd.rs/.development/24.6.1/target`);
see `## Known limits` for why. Results were not re-run on the full workspace
for this document.

### Analytical

Command: `cargo test -p hydrus-core --test heat_analytical`. Result: 5 passed, 0 failed.

- `analytical_only_steady_linear_profile_dirichlet`
- `analytical_only_steady_linear_profile_campbell`
- `analytical_only_steady_zero_flux_top_is_uniform`: the zero-flux top case. It
  stands in for a nonzero-gradient conduction case, which the driver cannot
  produce because the top flux is advective only.
- `analytical_only_transient_fourier_default`
- `analytical_only_transient_fourier_campbell`

### Source-matched parity

Command: `cargo test -p hydrus-core --test heat_fortran_parity -- --nocapture --test-threads=1`.
Result: 4 passed, 0 failed. The pass criterion is the rounded verdict: the
value rounded to 0.01 degC must be within 0.01 degC of the fixture. The strict
2e-4 degC check is reported next to it. Strict results are in the table below.

Fixtures: `reference/heat_fortran/fixtures/`.

- `heat_dir_top_dir_bot_default_matches_fixture` (fixture `heat_dir_top_dir_bot_default.json`)
- `heat_dir_top_flux_bot_default_matches_fixture` (fixture `heat_dir_top_flux_bot_default.json`)
- `heat_flux_top_dir_bot_campbell_matches_fixture` (fixture `heat_flux_top_dir_bot_campbell.json`)
- `heat_dir_top_flux_bot_campbell_matches_fixture` (fixture `heat_dir_top_flux_bot_campbell.json`)

Measured TempN results from the `--nocapture` output (degC; `fail` is the count
of the 41 nodes over the strict 2e-4 limit):

| Case | t (h) | max strict deviation | strict fails | max rounded deviation | rounded verdict |
|---|---|---|---|---|---|
| `heat_dir_top_dir_bot_default` | 0 | 0.000000 | 0 | 0.000000 | pass |
| | 6 | 0.004913 | 20 | 0.000000 | pass |
| | 12 | 0.004840 | 29 | 0.000000 | pass |
| | 24 | 0.005098 | 35 | 0.010000 | pass |
| `heat_dir_top_flux_bot_default` | 0 | 0.000000 | 0 | 0.000000 | pass |
| | 6 | 0.004928 | 21 | 0.000000 | pass |
| | 12 | 0.004822 | 30 | 0.000000 | pass |
| | 24 | 0.005335 | 36 | 0.010000 | pass |
| `heat_dir_top_flux_bot_campbell` | 0, 6, 12, 24 | 0.000000 | 0 | 0.000000 | pass |
| `heat_flux_top_dir_bot_campbell` | 0, 6, 12, 24 | 0.000000 | 0 | 0.000000 | pass |

The per-case node counts above were measured in this run. The 2e-4 degC strict
check fails at 20 to 36 nodes per default-conductivity time point, and this
failure is reported in full, not hidden. It is output rounding at the 2e-4 limit
and is not a rounded-verdict failure. The default cases reach a rounded
deviation of exactly 0.010000 at 24 h, at the pass boundary.

The two Campbell cases are flat at 25 degC and match exactly. They are weak
evidence for conduction (see `## Known limits`).

### Balance

Command: `cargo test -p hydrus-core --test heat_balance`. Result: 6 passed, 0 failed.

- `conduction_only_balance_closes_exactly`
- `coupled_balance_within_tolerance_campbell_false`
- `coupled_balance_within_tolerance_campbell_true`
- `residual_does_not_grow_with_step_count`
- `temperatures_stay_finite`
- `zero_flux_uniform_state_does_not_drift`

The coupled full residual holds by construction at Dirichlet ends, under the
reservoir convention (see `## Corrections and decisions`). The interior closure
is the non-trivial check. The unit tests of the balance module
(`heat::balance::tests::known_flux_step_matches_hand_computed_storage_and_residual`,
`heat::balance::tests::zero_flux_closed_column_has_zero_storage_change_and_residual`,
`heat::balance::tests::non_finite_input_is_rejected_and_leaves_state_unchanged`)
cover the storage and residual formulas.

### Unit and config tests

Each command below is `cargo test -p hydrus-core --test <name>`. All passed, 0 failed.

| Test file | Count | Examples |
|---|---|---|
| `heat_capacity` | 10 | `default_branch_matches_hand_values_for_two_materials`, `campbell_branch_at_zero_water_content_uses_dd`, `rejects_zero_capacity` |
| `heat_boundary` | 16 | `bottom_zero_gradient_sets_minus_one_one_zero`, `top_flux_level_second_with_advection`, `sinusoidal_top_is_rejected` |
| `heat_config_contract` | 12 | `accepts_heat_only_dirichlet_selection`, `rejects_vapor`, `rejects_energy_balance`, `rejects_dual_porosity` |
| `heat_driver_order` | 5 | `heat_uses_post_step_water`, `unsupported_config_rejected_before_water_step`, `heat_failure_after_water_success_is_reported` |
| `heat_state` | 7 | `accept_commits_trial_and_time`, `reject_leaves_accepted_state_unchanged` |
| `heat_analytical` | 5 | see `## Evidence`, Analytical |
| `heat_balance` | 6 | see `## Evidence`, Balance |
| `heat_fortran_parity` | 4 | see `## Evidence`, Source-matched parity |

Source-defect pinning test (unit, library):
`cargo test -p hydrus-core --lib source_defect_sd006`. Result: 1 passed, 0 failed.
Test: `heat::capacity::tests::source_defect_sd006_campbell_scale_applies_xconv_tconv_cubed`.

Python tooling: `python3 -m unittest tools.test_run_heat_reference tools.test_compare_heat_reference`.
Result: 17 tests, OK. The same 17 tests were selected by
`python3 -m unittest discover -s tools -p 'test_*heat*.py'`.

## Known limits

- **`kBotT == 0` (zero-gradient bottom) is supported in code and not validated
  against the Fortran.** The contract lists it as Supported. Code support is
  pinned by `accepts_zero_gradient_bottom` and
  `bottom_zero_gradient_sets_minus_one_one_zero`. There is no source-matched
  fixture for it. Its only Fortran deck, `heat_flux_top_zero_bot_campbell`, is
  refused by the producer because it reproduces SD-006 (the Campbell scale
  produces NaN from `t=6`, see `docs/FORTRAN_SOURCE_DEFECTS.md`). The refusal
  is recorded in that case directory (`refusal.json`); no fixture was written.
  The contract's `kBotT == 0` item is a source-reading claim only. The open
  question of whether to keep it Supported is not resolved here.
- **Campbell fixtures are flat at 25 degC.** `heat_dir_top_flux_bot_campbell`
  and `heat_flux_top_dir_bot_campbell` match exactly, but they do not test
  conduction. They are weak evidence for the Campbell branch.
- **Analytical conduction stand-in.** The driver cannot produce a nonzero-gradient
  conduction case, because the top flux is advective only. The zero-flux top case
  (`analytical_only_steady_zero_flux_top_is_uniform`) stands in for it. No
  nonzero-gradient conduction case is claimed as analytically validated at
  driver level.
- **Strict 2e-4 degC check not met.** Measured in this run: 20 to 36 nodes per
  default-conductivity time point (t=6, 12 and 24 h) exceed 2e-4 degC, with
  maximum deviations of about 0.0049 to 0.0053 degC. The rounded verdict passes.
  The Campbell cases pass the strict check at every point.
- **Rounded verdict at the boundary.** At 24 h, both default-conductivity cases
  reach a rounded deviation of exactly 0.01 degC, which passes (the rule is
  within 0.01 degC) but leaves no margin.
- **Table versus analytic water model.** The analytic water model misses the
  rounded rule by about 0.02 degC at 24 h for `heat_dir_top_flux_bot_default`
  (source: STATE_24 and the parity test header). Heat parity uses the table
  model. Differences from `SetMat` that are not reproduced: the mixed head
  `hiM`, and the analytic fallback outside the table range. The fixtures stay
  inside the range.
- **Tolerances not separately measured.** The contract's `HeatFl`, `Cap`/`Cond`,
  matrix-coefficient and per-step balance-residual tolerances are not reported
  with measured values in this document. They are not claimed as met.
- **Build directory.** The worktree's `target` symlink points to
  `.development/24-target/24.6.1`, which does not exist, so cargo failed with
  "Not a directory". The runs used `CARGO_TARGET_DIR` outside the checkout.

## Corrections and decisions

- **Van Genuchten conductivity exponent (dated correction, 2026-11-11).** The
  conductivity used a hardcoded `Se^1.5`. The Fortran (`MATERIAL.FOR`, `iModel 0`)
  uses the Mualem pore-connectivity parameter `l` (0.5 in the heat decks). Commit
  `1d4131a` (`hydraulics/van_genuchten.rs`, `hydraulics/types.rs`) adds
  `pore_connectivity` to `VanGenuchtenParameters`, with a default of 0.5 and
  validation in [0, 10]. Pinning tests in `crates/hydrus-core/tests/hydraulic_curves.rs`:
  `conductivity_uses_pore_connectivity_exponent`,
  `pore_connectivity_changes_unsaturated_conductivity`,
  `conductivity_derivative_matches_central_difference`,
  `pore_connectivity_is_validated`. The solute conductivity paths already used
  the Fortran `BPar` and were not changed.
- **Parity water model (2026-11-11, commit `31b9ad0`).** Heat parity uses the
  table water model, which reproduces the Fortran GenMat table (100
  log-spaced knots). The accepted water product keeps its analytic default. The
  difference is recorded here so that the two models are not conflated.
- **Tolerance approval (2026-11-11).** The user approved the values in the
  contract's `## Tolerances` (option "Derived from REAL precision"). The pass
  criterion for the parity tests is the rounded verdict (within 0.01 degC of
  the fixture). The strict TempN limit of 2e-4 degC is reported beside it and is
  not met in full (see `## Known limits`).
- **SD-006 (reproduced as written).** The Campbell conductivity carries an
  `xConv/tConv^3` factor that the default branch lacks (`TEMPER.FOR:300` against
  `TEMPER.FOR:286`). It is reproduced, not corrected. Register entry:
  `docs/FORTRAN_SOURCE_DEFECTS.md`, SD-006. Port: `heat/capacity.rs`
  (`campbell_conductivity`). Pinning test, also cited under the unit evidence
  above: `heat::capacity::tests::source_defect_sd006_campbell_scale_applies_xconv_tconv_cubed`.
  The fixed alternative is deferred. The decision on the scale is the user's.
- **Reservoir convention (2026-11-11, commits `5dfdf20` and `c94c808`).** At
  Dirichlet ends, the coupled full residual holds by construction, since the end
  nodes act as reservoirs. The interior closure is the test that carries weight.

## Not claimed

- Vapor flow (`lVapor`, `TempAdj`, vapor output). Goal G19.
- Energy balance (`lEnBal`, `UpdateEnergy`, `dtMaxT` under `lEnBal`, the `iTemp`
  loop). Goal G20.
- Heat-to-water feedback (`lWDep`, `lWTDep`). Rejected in M6.
- Hysteresis (M7).
- Dual porosity (M8).
- Root uptake with heat coupling.
- Chemistry.
- Universal accuracy, or any universal tolerance. Acceptance is capability-scoped.
- Any tolerance the evidence did not meet, including the strict 2e-4 degC TempN
  check on the default-conductivity cases.
- Correctness of boundary kinds outside `## Supported`, or of integer boundary
  codes (manual-pending).
- Correctness of `kBotT == 0` against the Fortran (see `## Known limits`).

# Phase 5 solute acceptance evidence

> **Current disposition (2026-11-11):** The user accepted the M5-V result with
> limitations and unblocked M6 planning and implementation. Original campaign
> classifications and holdout status are unchanged. See the
> [M5-V/M6 decision](M5V_M6_HANDOFF_DECISION.md).

> [!WARNING]
> **Orchestration 20 Correction Banner:**
> The `18.15.1` parity run invoked the Fortran reference runner inside `tools/solute_rust_candidate_adapter.py` rather than the Rust compatibility solver (`crates/hydrus-core/src/solute_fortran_compat/**`).
> Consequently, the default solute backend has been reset to `FiniteVolumeBackwardEuler` until genuine Rust-vs-Fortran parity passes across all 44 fixtures in Orchestration 20.
> Additionally, `iPsi=3` (`SOLUTE.FOR:1570-1579`) is an active column-6 continuation formula rather than a commented-out no-op.

## Purpose and evidence standard

This record assesses M5 from the accepted Orchestration 10 task reports and Rust contract tests. It distinguishes implemented contract behavior and bounded internal consistency checks from scientific validation. The source snapshot now has a reviewed derived-case executable run and diagnostic paired observations, detailed below. No canonical manual deck or calibrated scientific acceptance threshold is available.

The solute fixtures are deterministic, independent in-memory cases. Manual Chapters 11.3–11.5 are provenance candidates, but their machine-readable decks and expected outputs are absent. Fixture numbers are constructed inputs, not recovered manual outputs. The tests establish selected API, transaction, sign, and conservation invariants; they do not establish calibrated trajectories, physical correctness, or source-version parity.

## Delivered M5 contract

The `hydrus-core` solute API supports one water domain. `SoluteUnits` requires nonempty caller-declared concentration and sorbed-mass labels, and performs no conversion or universal physical-basis assumption. Callers must provide mutually compatible units for concentrations, sorbed pools, rates, fluxes, bulk density, and time.

Species vector order defines the sequential first-order chain. For species `i`, the configured rate transfers mass out of `i` and into `i+1`; decay of the terminal species leaves the configured chain. The selected process set includes linear and nonlinear equilibrium sorption and one persistent kinetic sorbed pool per species. Kinetic exchange updates that pool; the excluded second source pool is not represented.

The supported boundary selectors are prescribed dissolved concentration, prescribed total-solute flux, and `AdvectiveOutflow` at either endpoint, configured independently for each species in chain order. The outlet mode imposes zero dispersive endpoint flux and solution-dependent advective export using the accepted endpoint water flux; inward water flow returns `InwardFlowAtOutlet` before trial state begins, and zero flow is valid. `SoluteConfig::new` broadcasts one pair to every species; `new_with_species_boundaries` requires exactly one pair per species, and `boundaries()` exposes the pairs as a species-ordered slice. Nonfinite values and negative prescribed concentrations in any pair, malformed pair counts, and unsupported feature selections return typed errors. Deferred selections include heat/vapor and temperature dependence, density-dependent transport, bacteria, filtration, special chemistry, dual-porosity water, mobile-immobile transport, artificial dispersion, and moisture-dependent transport. The node-wise solid bulk-density input used to convert sorbed storage is required for selected sorption processes; it does not enable density-dependent transport.

## Equations, units, signs, and water coupling

The source-anchored selected equations are:

- Linear equilibrium sorption: `S_eq = Kd * C`, with `Kd` dimensions sorbed-mass/concentration.
- Nonlinear equilibrium sorption: `S_eq = f * K * C^n / (1 + nu * C^n)`, where `f` and `n` are dimensionless, `K` has dimensions sorbed-mass/concentration^n, and `nu` has concentration^-n dimensions.
- Kinetic nonequilibrium exchange: `dS_kin/dt = omega * (Kd * C - S_kin)`, with `omega` in inverse time and the partition coefficient dimensionally compatible with the declared labels.
- Ordered chain reaction: `dM_i/dt = -k_i*M_i` and `dM_(i+1)/dt = +k_i*M_i`, with `k_i` in inverse time. The final species has no configured successor, so its decay is net loss from the chain.

These equations are anchored to `SOLUTE.FOR` species sequencing and the `Coeff`, `NEquil`, and `MassTran` routines. They define the selected Rust contract and are not a claim that every source branch is reproduced. Nonlinear iteration uses caller-supplied positive iteration and concentration-change limits and is deterministic and bounded; the tolerance is not calibrated. A nonlinear exponent below one at zero concentration produces a nonfinite derivative and a typed rejection.

For node `j` and species `i`, the reported integrated inventory is assembled as:

`M_i = sum_j(dx_j * [theta_j*C_ij + rho_b,j*(S_eq,ij + S_kin,ij)])`.

Here `theta` is dimensionless, `C` and `S` use the caller's labels, and `rho_b*S` must have units compatible with `theta*C`. The control-volume-weighted total is a mass inventory per column cross-sectional area under those compatible labels. Storage includes dissolved, equilibrium-sorbed, and kinetic-sorbed mass exactly once.

The signed step balance is `delta_M_i = B_i + R_i`, where species-specific boundary exchange `B_i` is positive into that species inventory and reaction change `R_i` is signed positive for net production into that species and negative for net loss. Explicit total-flux and outlet contributions use only that species’ endpoint pair; outlet exchange is computed independently as `dt * (qL*CnewL - qU*CnewU)` for configured outlet ends, then subtracted before any concentration-boundary remainder inference. Concentration-boundary remainder inference is species-specific. The residual is `delta_M_i - B_i - R_i`; the normalized residual divides its absolute value by `abs(delta_M_i)+abs(B_i)+abs(R_i)`. When that denominator is zero, the reported normalized residual is the absolute residual. Per-species cumulative boundary/reaction terms and last-accepted-step diagnostics promote with accepted state. Rust uses backward-Euler endpoint quadrature and a conservative half-cell finite-volume row; the frozen Fortran lower outlet row and its old/new weighted accounting differ. This implementation adds outlet support but does not establish fixture parity, full water-history mapping, or scientific tolerances.

Transport borrows the accepted water-content vector and `node_count + 1` water-face fluxes ordered lower boundary, internal faces, upper boundary. Water-face flux signs are positive toward increasing mesh coordinates and are supplied by the caller; the solute solver does not reconstruct fluxes or mutate water state. The configured total-solute boundary flux is separately defined positive inward at either endpoint, so its mass contribution is `timestep * (lower_inward_flux + upper_inward_flux)`. Concentration rows have no independent solute-flux observation: their net exchange is inferred internally as the exchange needed to close the discrete storage/reaction balance after all known total-flux and advective-outlet contributions have been subtracted. This inferred term is not independent validation of closure.

The outlet regressions in [`solute_outlet.rs`](../../crates/hydrus-core/tests/solute_outlet.rs) independently reconstruct weighted inventories, outlet exchanges, and process-chain reaction changes. They cover both endpoints and both stepping entry paths, zero flow, parent-only direct inlet, species-specific chain export, mixed concentration/outlet accounting, broadcast/explicit configuration equivalence, and in-trial rollback after a later species fails nonlinear convergence.

Before trial mutation, accepted water time must match `solute_state.time + timestep` within `8 * f64::EPSILON * max(abs(start_time), abs(end_time))`; this is a floating-point roundoff bound, not a physical or calibration tolerance. Snapshot and process lengths, finite values, and required dimensions are validated before `begin_trial`.

## Transactions and fixture inventory

The accepted/trial state and setup-allocated workspace cover dissolved concentration, one kinetic sorbed pool, time, and per-species accounting. Rejected/error trials restore the accepted solute pools and accounting. Accepted water state remains externally owned and is not changed by solute stepping.

| Fixture | Provenance and observed contract | Exact command and accepted result |
|---|---|---|
| Capability contract | Independent in-memory configurations; checks unit/species ordering, sorption and boundary selection, malformed/nonfinite inputs, and typed errors for deferred selections. | `cargo test -p hydrus-core --test solute_contract` — 4 passed, 0 failed. |
| Transaction lifecycle | Independent state/workspace inputs; checks dimension rejection, full promotion, rollback identity across pools/accounting, and workspace reuse. | `cargo test -p hydrus-core --test solute_transaction` — 3 passed, 0 failed. |
| Conservative transport | Constructed finite-volume cases; checks closed-column diffusion mass, both signed advection directions, concentration/total-flux endpoint rows, invalid-input/pivot rejection, and state identity. | `cargo test -p hydrus-core --test solute_transport` — 5 passed, 0 failed. |
| Reaction and sorption | Constructed process cases; checks sequential species transfer/terminal loss, linear and nonlinear sorption conservation/retardation, kinetic transfer direction/conservation, limiting coefficients, invalid/deferred inputs, and failed-trial atomicity. | `cargo test -p hydrus-core --test solute_reactions` — 6 passed, 0 failed. |
| Accepted-step balance | Constructed cases; checks explicit inward total-flux exchange, dissolved/equilibrium/kinetic storage and signed reaction closure, internally inferred concentration-boundary exchange, accepted time, time/shape rejection identity, and typed unsupported-mode error. | `cargo test -p hydrus-core --test solute_acceptance` — 4 passed, 0 failed. |

The contract fixtures above use deterministic in-memory values. Orchestration 12 additionally supplies a derived Fortran comparison fixture; canonical Chapter 11.3–11.5 decks, independent analytical trajectories and calibrated tolerances remain unavailable. See [`solute_contract.rs`](../../crates/hydrus-core/tests/solute_contract.rs), [`solute_transaction.rs`](../../crates/hydrus-core/tests/solute_transaction.rs), [`solute_transport.rs`](../../crates/hydrus-core/tests/solute_transport.rs), [`solute_reactions.rs`](../../crates/hydrus-core/tests/solute_reactions.rs), and [`solute_acceptance.rs`](../../crates/hydrus-core/tests/solute_acceptance.rs) for exact inputs and assertions.

## M5 exit assessment

| Criterion | Status | Evidence and limit |
|---|---|---|
| Validated single-domain solute configuration, selected species/process/boundary contract, and typed rejection of unsupported combinations | **Met (contract evidence)** | 10.1.1 report and 4/4 contract tests. No broad source-option or release-compatibility claim. |
| Accepted/trial lifecycle, fixed-size workspace, and rollback | **Met (contract evidence)** | 10.2.1 report and 3/3 transaction tests cover all stored mutable solute pools and accounting fields. |
| Accepted-water-coupled advection/dispersion and selected endpoint modes | **Met (bounded contract coverage)** | 10.3.1 report and 5/5 transport tests check signs, closed-column invariants, boundary rows, validation, and pivot rejection. No independent numerical reference. |
| Ordered reaction, equilibrium sorption, and one kinetic sorbed pool | **Met (bounded contract coverage)** | 10.4.1 report and 6/6 reaction tests check selected equations and invariant cases. The nonlinear exponent-below-one/zero-concentration path has a typed rejection edge case. |
| Per-species accepted-step accounting and integrated acceptance fixtures | **Evidence-limited contract coverage** | 10.5.1 and 11.1.1 reports and the balance fixtures check selected terms, rejection identity, and a hand-computed changing-water-content inventory difference. Callers supply start-of-interval water contents corresponding to the current solute-state time and accepted end-of-interval contents; pre-step dissolved storage uses start theta while accepted storage uses end theta. Concentration-boundary exchange is inferred from the same discrete balance, so its closure is internally imposed rather than independently observed. |
| Physical validation, manual/reference comparison, calibrated tolerance, and canonical fixture provenance | **Unmet / unavailable** | Manual decks and analytical arrays are absent. A derived executable baseline and diagnostic paired comparison now exist, but refinement evidence and calibration remain missing. Constructed invariant cases do not establish physical correctness or parity. |
| Workspace quality gates | **Passed at accepted 10.5.1 wave gate; final serial gate belongs to orchestrator** | Accepted report records `cargo test --workspace` (110 integration tests, 0 failures), `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passing. The orchestrator runs these again after this final evidence task. |

M5 reaches its planned implementation and contract-evidence endpoint for the selected scope, while scientific validation remains unmet. This status does not imply a calibrated model, manual-case reproduction, Fortran parity, universal tolerance, release compatibility, or performance result.

## Limitations and later-milestone handoff

The largest evidence limits are absent manual decks/analytical outputs and paired mesh/time convergence evidence; caller-declared rather than universally fixed unit bases; and a concentration-boundary exchange inferred from the same closure equation. Interval accounting now takes caller-owned start-of-interval water contents and the accepted end snapshot explicitly; start-water values are dimension, finiteness, and positivity checked before trial mutation, and are contractually associated with the current solute-state time. Test tolerances are invariant-check thresholds, not calibrated physical tolerances. The nonlinear solver's caller-selected tolerance is not calibrated. The source snapshot's lineage and release compatibility remain unverified.

The M5 scientific-comparison work retains limitations. Per the [comparison research](SCIENTIFIC_COMPARISON_RESEARCH.md), Orchestration 12 delivered a derived fixture, solute reference-run support, diagnostic paired observables, and a tolerance review that leaves thresholds unassigned. The user accepted the existing result for M6 progression; no additional M5 campaign is required by that decision. Phase 0's noncanonical smoke output is not a solute oracle.

The next physics milestones remain distinct and are not authorized by this evidence:

- **M6 heat/vapor:** requires a separate temperature/energy and, if approved, vapor coupling order, supported-feature contract, energy/water balance evidence, rollback, and fixture provenance.
- **M7 hysteresis:** requires explicit reversal/history state, accepted/rejected history rollback, and drainage/imbibition fixtures.
- **M8 dual porosity:** requires separate mobile/immobile water state and exchange equations, balance and rollback evidence. It must remain distinct from this M5 single-domain water model, the one supported kinetic solute sorption pool, and Durner bimodal retention; dual permeability remains a separate rejected mode.

Density-dependent transport, bacteria/filtration, special chemistry, and other optional branches remain explicitly deferred and require separate capability decisions and evidence. No future milestone follows automatically from M5 completion.

## Sources

- [Orchestration 10 execution plan](../EXECUTION_PLAN_10.md), [accepted task reports](../STATE_10.md), and per-task TODO reports: [10.1.1](../tasks/10.1.1.todo.md), [10.2.1](../tasks/10.2.1.todo.md), [10.3.1](../tasks/10.3.1.todo.md), [10.4.1](../tasks/10.4.1.todo.md), and [10.5.1](../tasks/10.5.1.todo.md).
- [Phase 4 water acceptance evidence](PHASE_4_WATER_ACCEPTANCE_EVIDENCE.md) for evidence structure and bounded-provenance conventions.
- [Roadmap and risks](ROADMAP_AND_RISKS.md), [feature matrix](FEATURE_MATRIX.md), and [validation strategy](VALIDATION_STRATEGY.md) for M5–M8 sequencing and scientific-validation limits.
- [`SOLUTE.FOR`](../../src_reference/SOLUTE.FOR): species sequence (`:6-230`), `Coeff` (`:241-625`), `Disper` (`:1025-1076`), `NEquil` (`:1078-1200`), `MassTran` (`:1319-1450`), and `BanSol` (`:945-957`). These are source-algebra anchors only.

## Derived comparison and M5-V handoff — 2026-09-26

The [fixture README](../../reference/fortran_reference/solute/example3/README.md), [pinned run record](../../reference/fortran_reference/solute/example3/RUN.md), and [repeatability/tolerance review](SCIENTIFIC_COMPARISON_RESEARCH.md#repeatability-precision-and-tolerance-review--2026-09-26) establish completed derived-case execution, audited input/water/process mapping, strict parsed observables, independent inventory/accounting diagnostics, and two repeat executions matching the original numeric output exactly. One verified build with promoted default-real precision also completed; its measured concentration/inventory/balance sensitivity is documented in the review. All runs retain underflow/denormal warnings. The intermediate water field is source-reconstructed, and endpoint/time-quadrature differences remain explicit.

| M5-V claim | Reviewed status |
|---|---|
| Derived fixture provenance and pinned executable completion | Met for the single named derived case; input/source/patch/executable/output hashes in RUN.md and scratch characterization manifests |
| Diagnostic source comparison and independent observables | Met for audited common space/time points; paired concentration maxima reach `0.0432242`, total inventory differences range `-0.00505994` to `+0.0117295` |
| Baseline repeatability and bounded precision sensitivity | Met on GNU Fortran 13.3.0/Linux x86_64 only; two baseline runs plus one precision variant, no wider compiler/platform claim |
| Calibrated source-parity acceptance | **Unmet**: no justified per-field absolute/relative tolerances, balance threshold or near-zero cutoff; comparator stays diagnostic |
| Independent analytical/physical validation | **Unmet**: missing both-code mesh/time convergence and reviewed analytical solution for this finite-domain boundary problem; contract invariants alone are insufficient |
| Manual reproduction, general text/release compatibility and redistribution | **Unavailable/unreviewed**; derived-case parser success does not establish these claims |

M5-V remains open. Follow-up work must quantify endpoint and temporal-quadrature convergence, provide an independent analytical/manufactured reference on common coordinates/times, and review storage/flux/reaction error budgets including zero/near-zero balance cases before freezing thresholds. This handoff does not authorize Phase 6 or promote the precision variant to an oracle.

> Superseded by [M5-V closure (22.x)](#m5-v-closure-22x--partial--open): M5-V is still partial / open.

## Complete 44-fixture Fortran parity suite execution (Orchestration 18 / Task 18.15.1) — 2026-10-15

Task 18.15.1 completed full serial execution of all 44 parity fixtures across all six process groups (`transport`, `reactions`, `transport_properties`, `nonequilibrium`, `dual_exchange`, `biology`, `flux_concentration`), comparing the pinned Fortran reference executable (`GNU Fortran 13.3.0`, Linux x86_64) against the Rust Fortran-compatible solver (`SoluteBackend::FortranCompatible`).

- **Total Fixtures:** 44
- **Passed:** 44 (100%)
- **Failed / Diverged:** 0
- **Runner Evidence:** Pinned executable and GNU compatibility patch authenticated for all runs. Normalized inputs and outputs matched within declared half-width precision intervals and field rules.
- **FV/BE Solver Preservation:** The Finite-Volume / Backward-Euler alternative solver remains intact as an explicit candidate (`SoluteBackend::FiniteVolumeBackwardEuler`), passing all 6/6 tests in `tests/solute_acceptance.rs` and 4/4 tests in `tests/solute_solver_selection.rs`.
- **Documentation:** Full audit record, active modes, inactive helpers, and scientific gaps are recorded in [SOLUTE_FORTRAN_PARITY_18.md](SOLUTE_FORTRAN_PARITY_18.md).
- **Parity vs. Validity:** This result confirms numerical parity against the pinned legacy Fortran source algorithms; it does not claim continuum physical correctness or empirical validation against physical experimental data.

## 20.6.1 all-mode Rust-vs-Fortran parity run (round 5, 2026-10-20T10:47:56Z) — FAIL, 35/44

> **Superseded** by the 21.4.1 run below (2026-10-22, `17d9d9c`): 44/44 PASS. This section is kept as the round-5 record.

- **Command:** `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group all --backend fortran-compatible --task-root .development/20.6.1/r5_final`, plus one run per named group (`.development/20.6.1/r5_final_<group>`).
- **Commit:** `62c4b37` plus uncommitted 20.6.1 changes.
- **Result:** 35 of 44 fixtures pass. The gate does not pass, so 44/44 is not claimed.

| Group | Fixtures | Pass | Fail |
|---|---|---|---|
| transport | 18 | 18 | 0 |
| reactions | 4 | 4 | 0 |
| transport_properties | 5 | 0 | 5 |
| nonequilibrium | 5 | 5 | 0 |
| dual_exchange | 4 | 0 | 4 |
| biology | 6 | 6 | 0 |
| flux_concentration | 2 | 2 | 0 |

- **Divergence metric:** the runner has no `divergent_count` field. Per-fixture `status` and `first_divergence` are the metric; fail counts above are the substitute.
- **Digests:** 44 fixtures. 315 distinct Fortran reference output SHA-256 values and 121 distinct Rust candidate values. Shared reference/candidate digests: 0. Pinned patch SHA-256 `622d1ea6...`.
- **Coverage:** active `iPsi=3` is exercised by one fixture, `biology/blocking_type_3_noop`. Coupled `WatFlow`/`DualPor` for `W-volumeI[L]` is exposed by nine fixtures; the coupling is not yet confirmed from `SELECTOR.IN`, so it remains a stated gap.
- **Fixed in round 5:**
  - `reactions/chain_distinct_rates` now passes. The cumulative solute accumulators (`CumCh`) are default REAL in `OUTPUT.FOR`, so they must accumulate in single precision. The Rust runner used f64, which moved a printed value across a rounding interval. Regression: `chain_distinct_rates_paired_gate`.
  - Equilibrium water content now follows the new head. `WATFLOW.FOR:73-76` (Hyst) runs after SetMat and sets `ThEq` at `hNew`, and `Reset` uses it (`WATFLOW.FOR:194`). The Rust reset used `ThEq = ThOld`, which froze the mobile moisture. Regression: `equilibrium_water_content_follows_the_new_head`. Moisture now matches the pinned value (0.4294).
  - Con/Cap are now evaluated at `hNew` after SetMat, as the pinned Hyst overwrite does (`WATFLOW.FOR:73-76`). No measured effect on the gate.
- **Open, root cause localized:** Hyst (`HYSTER.FOR`, `HysterIni` lines 124-240, `DRY` 535-594) is not ported. The pinned code evaluates `Con`, `Cap`, and `ThEq` from tabulated Lenhard k-S-P curves on every iteration, with per-node history. Rust uses analytic van Genuchten values.
  - `dual_exchange` (4): the first divergent quantity is the step-1 water iteration count (Fortran `ItrW`=8, Rust 9). Head at node 2 at t=1 is -3.319 vs -3.339. Mobile moisture and immobile moisture match.
  - `transport_properties` (5): pinned `Con` varies between nodes at identical head (Kappa history), so the top flux drifts in time. Rust keeps it constant.
  - Two attempts were made on this cause. The porting work is the next step.
- **Disclaimer (unchanged):** `FortranCompatible` establishes pinned-source numerical parity, while continuum and mathematical accuracy are tracked separately by the FV/BE analytical and convergence suites.

## 21.4.1 all-mode Rust-vs-Fortran parity run (2026-10-22T04:45:25Z) — PASS, 44/44

- **Command:** `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group all --backend fortran-compatible --task-root .development/21.4.1/all_parity`, then one run per named group with its own task root (`.development/21.4.1/<group>_parity`), run serially.
- **Timestamp (UTC):** all-mode run 2026-10-22T04:45:25Z to 04:47:42Z; group runs finished 2026-10-22T04:50:03Z.
- **Commit:** `17d9d9c` (clean tree; the last code change is merge `99fe2a6`).
- **Result:** 44 of 44 fixtures pass. The all-mode run and all seven group runs exit 0 with top-level `"status": "pass"`. No fixture has status `fail`. The seven groups together hold the same 44 fixtures as the all-mode run.

| Group | Fixtures | Pass | Fail |
|---|---|---|---|
| transport | 18 | 18 | 0 |
| reactions | 4 | 4 | 0 |
| transport_properties | 5 | 5 | 0 |
| nonequilibrium | 5 | 5 | 0 |
| dual_exchange | 4 | 4 | 0 |
| biology | 6 | 6 | 0 |
| flux_concentration | 2 | 2 | 0 |

- **Divergence metric:** the runner has no `divergent_count` field. Per-fixture `status` is the metric, and the fail counts above stand in for it.
- **Digests:** 44 fixtures. 349 distinct Fortran reference output SHA-256 values and 122 distinct Rust candidate values. Shared reference/candidate digests: 0, so the Rust candidate produced its own outputs. Pinned patch SHA-256 `622d1ea6...` on every fixture. Candidate runner `solute_compat_deck_runner` SHA-256 `320a4ad9...`, compiled fresh against `libhydrus_core` rlib `61ba7811...` in a new target directory.
- **Reproducibility:** checked against an independent all-mode run at the same code (`99fe2a6`). Every candidate output digest is identical in all 44 fixtures, and every per-fixture status matches. Every Fortran reference digest differs. Across all 493 reference output files, the only lines that differ are the wall-clock `Date: … Time:` header line and the `BALANCE.OUT` `Calculation time [sec]` line. The Fortran executable is rebuilt for each fixture task root, and its SHA-256 differs from build to build (44 distinct values in each run).
- **Coverage:**
  - Active `iPsi = 3` is exercised by one fixture, `biology/blocking_type_3_noop` (`CASE.json`, `blocking_iPsi: 3`).
  - `W-volumeI[L]` is output by nine fixtures (`CASE.json`/`OBSERVATION_MAP.json`). From `SELECTOR.IN`, all nine have `lWat = t`. Six of them run coupled `WatFlow`/`DualPor`: `Model Hysteresis` = `6 0` in `dual_exchange/model6_exchange`, `dual_exchange/model6_no_exchange` and `nonequilibrium/dual_model6`, and `7 0` in `dual_exchange/model7_exchange`, `dual_exchange/model7_no_exchange` and `nonequilibrium/dual_model7`. The other three have `0 0` (no `DualPor`): `nonequilibrium/dual_single_domain_control`, `mobile_control` and `mobile_immobile`.
  - From `SELECTOR.IN`, all 44 fixtures have `iHyst = 0`.
- **What changed since 20.6.1 (35/44):** pass counts went 35 → 39 → 43 → 44.
  - The 20.6.1 attribution to an unported `Hyst` (`HYSTER.FOR`) was wrong. With `iHyst = 0` that code is unreachable. The water divergence came from the differences listed below, outside the solver equations.
  - Deck-derived `lTable`, applying the `INPUT.FOR:560-580` `hTab` negation; `lExtrap = .true.` (`INPUT.FOR:1712`); `lMinStep = .true.` (`INPUT.FOR:1584`).
  - `wat_flow` makes one blended `SetMat` call (`WATFLOW.FOR:65`). The second, unblended call (commit `72d2544`, citing the unreachable `iHyst = 3` branch at `WATFLOW.FOR:73-76`) is removed.
  - Two pinned-source defects are reproduced in the Rust port and registered in `docs/FORTRAN_SOURCE_DEFECTS.md` with pinning tests:
    - SD-003: `Solute` overwrites `hTemp` through its `vCorr` argument (`HYDRUS.FOR:510-511`).
    - SD-004: `SetMat` table interpolation reads row `NTab+1` (`WATFLOW.FOR:482-484,515-518`).
  - The pinned Fortran is not modified to reproduce SD-003/SD-004. The Rust port reproduces them, as ADR 0001 and the defect register require.
  - With `lChem` set and `iDualPor > 0`, `lEquil` is set to `.false.` (`HYDRUS.FOR:278`), so `NEquil`/`SorbConc` run for the dual-porosity decks.
  - Model 7 immobile `n`/`Ks` are keyed on `iDualPor = 2`; before, they were dropped and the exchange call failed silently. `WatFlow` exchange errors now propagate. `STrans` is copied back for `NOD_INF`.
  - `format_fortran_e` rounds exact ties to even, as gfortran does.
  - Vapor: `iEnhanc = 1` (`INPUT.FOR:1718`). `NOD_INF_V` reports an f32 `vTotal` and `vVapIso`/`vVapTerm` as `OUTPUT.FOR:769-816` does.
- **Known gaps not covered by the gate:** these output differences are not observed by the parity comparison and are open follow-ups:
  - gfortran prints `-0.0…E+00` where Rust prints ` 0.0…E+00`. Parity parses values numerically.
  - The `NOD_INF` node depth sign at t=1: Rust prints `0.2500`, Fortran `-0.2500`.
  - `NodOut` recomputes `vi`/`vVi`, while Rust prints the solver's `v`/`vv`.
- **Disclaimer:** `FortranCompatible` establishes pinned-source numerical parity, while continuum and mathematical accuracy are tracked separately by the FV/BE analytical and convergence suites.

## M5-V closure (22.x) — partial / open

At the time of the 22.x closure, M5-V was recorded partial/open: study-level thresholds (`reference/solute_verification/THRESHOLDS_V2.json`, `cases_sha256` `c236eb7c…`, `holdout_run_permitted: false`) were approved under D10, holdouts were not run (no holdout-level thresholds assigned; holdout mesh/dt not isolated), and that record did not unblock M6. The later [2026-11-11 decision](M5V_M6_HANDOFF_DECISION.md) accepts the result with limitations for project progression.

Decisions: D2 (re-scope: source parity from the 44/44 gate, physical and analytical validation on the default `FortranCompatible` backend), D3 (four analytical families; FV/BE not re-studied), D8 (error budgets, user threshold gate, then holdouts), D10 (2026-10-24: approve study-level ceilings only, no holdouts). ADR: [0019](../../docs/decisions/0019-validate-scientific-accuracy-on-the-default-backend.md).

### Claims

Rows are copied from [SOLUTE_CONVERGENCE_EVIDENCE_22.md](SOLUTE_CONVERGENCE_EVIDENCE_22.md) "M5-V claims". The original column header ("Status if the proposals are approved") assumed holdouts pass; no holdout ran, so read each row as study-level only.

| M5-V claim | Status under D10 (study-level only) |
|---|---|
| Independent analytical reference for closed reactions, time axis (distinct/equal/zero) | Would be **met for `profile_linf` at the listed species/times on `reaction_distinct`/`reaction_equal`** (one level for `equal`); no holdout row exists for it; inventory and reaction rates remain **unassigned** |
| Diffusion cosine space accuracy (`linf`, `l2`) | **Conditionally met** at 65 nodes, dt 2.44e-4 only; the 97-node/dt 0.0078125 holdout candidate is **unassigned**, so the holdout cannot confirm it |
| Diffusion cosine inventory / mass conservation | **Unmet: unassigned, known defect candidate** |
| Scalar Robin space accuracy (`linf`, boundary outflow) | **Conditionally met** at 129 nodes, dt 2^-8 only; holdout candidate **unassigned**; `l1`, `l2`, inventory **unassigned** |
| Temporal convergence (diffusion, Robin, Example 3) | **Unmet**: not isolated on any non-reaction case |
| Example 3 full-chain NH4/NO2 concentration norms | **Conditionally met** at 1001 nodes, dt .025 (scratch-replay evidence); NO3, inventory, balance, node-1 closure, outlet tail, reaction rates **unassigned** |
| Example 3 inventory, balance, node-1 closure, temporal error | **Unmet: unassigned** |
| Classifier verdict (achieved/failed) | **Unmet**: no bounds source; every finding `inconclusive` |
| M5-V at 22.x closure | **Partial; open.** Unassigned fields remained unmet (`SOLUTE_VERIFICATION_SPEC.md:409`); see the later disposition update at the top of this document. |

### Source-parity evidence (21.4.1)

Rust `FortranCompatible` matched the pinned Fortran (GNU Fortran 13.3.0, Linux x86_64) on all 44 fixtures (`17d9d9c`; [SOLUTE_FORTRAN_PARITY_18.md](SOLUTE_FORTRAN_PARITY_18.md) section 21.4.1). This ties the 22.x evidence to the pinned Fortran; no 22.x row launched Fortran.

### Analytical evidence (22.4.x, 22.5.1)

- Closed reactions: [M5V_STUDY_22_REACTIONS.md](M5V_STUDY_22_REACTIONS.md).
- `diffusion_cosine`: [M5V_STUDY_22_DIFFUSION.md](M5V_STUDY_22_DIFFUSION.md).
- `scalar_robin`: [M5V_STUDY_22_ROBIN.md](M5V_STUDY_22_ROBIN.md).
- Example 3 chain: [M5V_STUDY_22_EXAMPLE3.md](M5V_STUDY_22_EXAMPLE3.md) (22.4.4, partial) and [M5V_STUDY_22_EXAMPLE3_SPACE.md](M5V_STUDY_22_EXAMPLE3_SPACE.md) (22.4.5).
- Error budgets, observed orders and proposed thresholds (22.5.1): [SOLUTE_CONVERGENCE_EVIDENCE_22.md](SOLUTE_CONVERGENCE_EVIDENCE_22.md).

### Thresholds

[`THRESHOLDS_V2.json`](../../reference/solute_verification/THRESHOLDS_V2.json): SHA-256 `c15a4dc0ba80e92994fea1f618774e1428a13c8f95c70261d0f04ee4967ca8ab`; `cases_sha256` `c236eb7cd42dee691a36787625865457df62c0bb4a72cb0388a786cbe1e1b43b` (`CASES_V2.json`); 33 entries; study-level ceilings only; `holdout_run_permitted: false`. Margin rule: 1.5 x observed error, 2 significant figures, applied once (ADR 0009).

### Holdouts

Not run (task 22.7.1 was not executed). Every combined holdout-level candidate is unassigned because no study isolated the holdout mesh or dt. The convergence CLI lifts its holdout guard when the thresholds file exists, so holdout rows must not be run against it.

### Remaining honest gaps

- Holdouts were not run.
- Unassigned fields: diffusion inventory (known kernel end-row mass-loss defect candidate; no SD registered; reachability unproven); Robin `l1`/`l2`/inventory/balance; all temporal error; Example 3 inventory, balance, node-1 closure, NO3, outlet tail and reaction rates.
- Example 3 space numbers are scratch replay, not authenticated batch evidence.
- The bridge gate was waived for v2, and the v2 bridge row failed ("record inventory mismatch").
- The precision label "f64" in `CASES_V2` is schema-only; the Fortran-compatible solver is f32.
- O13 `--verify-only` limit: the copied 22.4.4 batches no longer verify under the new casebook (exit 2).
- The classifier has no bounds source, so every finding is `inconclusive`.
- The checker SHA is part of the reference-cache identity, so each checker edit forces a ~3 h reference recompute.
- The v1 Fortran-only precision-sensitivity row was dropped and has no v2 counterpart.
- The FV/BE study was not re-run; manufactured forcing is still blocked.
- All claims hold for one compiler and platform (GNU Fortran 13.3.0, Linux x86_64).

## M5-V final campaign (23.x) — partial / open

At campaign close, M5-V was recorded partial / open. The campaign ran six verified batches with a signed combined bounds source; no holdout row was run and Gate C was not requested. See the current disposition at the top of this document.

- Diffusion space: 12 `achieved` concentration fields, 178 inconclusive.
- Scalar Robin space: 40 `failed` S1 fields (`nonmonotonicity` between the fine pair), 150 inconclusive.
- Diffusion time, Robin time, Example 3 space `dt .04`, and Example 3 time: all inconclusive (12,560 findings in these four groups; 12,888 across all six). Every time finding reports `fewer_than_three_levels`; the design's centre rows are space-axis only.
- Example 3 space is dominated by control contamination and arithmetic floors; it does not establish the claimed norms.
- No holdout-level threshold was defensible from these results. `THRESHOLDS_V2.json` remains the stale study-level file (`cases_sha256` `c236eb7c…`, `holdout_run_permitted: false`) and no holdout row ran. At campaign close, M6 was not unblocked; the later decision accepts the result with limitations and unblocks M6 progression.
- Evidence: [M5V_FINAL_23.md](M5V_FINAL_23.md). ADR [0019](../../docs/decisions/0019-validate-scientific-accuracy-on-the-default-backend.md) remains "Accepted; M5-V open".

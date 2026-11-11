# Validation and performance strategy

> **Current disposition (2026-11-11):** M5-V is accepted with limitations for
> project progression, and M6 is unblocked. No further M5 campaign work is
> required by default. Preserve the observed classifications and revisit a
> numerical anomaly only if a real supported-use case shows that it could
> change an implementation or use decision. See the
> [M5-V/M6 decision](M5V_M6_HANDOFF_DECISION.md).

> [!WARNING]
> **Orchestration 20 Correction Banner:**
> The `18.15.1` parity run invoked the Fortran reference runner inside `tools/solute_rust_candidate_adapter.py` rather than the Rust compatibility solver (`crates/hydrus-core/src/solute_fortran_compat/**`).
> Consequently, the default solute backend has been reset to `FiniteVolumeBackwardEuler` until genuine Rust-vs-Fortran parity passes across all 44 fixtures in Orchestration 20.
> Additionally, `iPsi=3` (`SOLUTE.FOR:1570-1579`) is an active column-6 continuation formula rather than a commented-out no-op.

## Fortran reference harness

The Fortran reference executable is a separately identified copy of the pinned ten-file snapshot. Phase 0 preserves the originals, applies a reviewable GNU Fortran/Linux portability patch for the known `MSFLIB` and `iMetHour` blockers, and records source/archive, patch, compiler, flags, executable, host, and date. It does not silently promote precision or rewrite solute algorithms. The patched executable passed the explicitly noncanonical water smoke, so the runner/build path works; no canonical scientific solute run or expected output exists yet. See [Phase 0 evidence](PHASE_0_REFERENCE_EVIDENCE.md).

Each fixture must run in its own temporary output directory with immutable input copies, an explicit wall-clock timeout, captured stdout/stderr, exit status, expected completion marker (`CloseOutput`/`end` where applicable), file inventory, and no stale outputs. A manifest records fixture ID, input provenance, source and patch hashes, compiler/toolchain, flags, precision mode, platform, command, timeout, output hashes, and observed warnings. A run is incomplete if it times out, exits nonzero, omits required outputs, or writes malformed/truncated records. The current runner also supports the derived Example 3 solute case; strict fixture parsing and diagnostic comparison now exist. M5-V remains open pending analytical/refinement evidence and calibrated tolerances. (Superseded by [M5-V closure (22.x)](#m5-v-closure-22x--partial--open): M5-V is still partial / open.)

## Fixture matrix

| Fixture | Input provenance status | Observables and checks | Producer/prerequisite |
|---|---|---|---|
| Dry-column infiltration | 4.08 Chapter 11.1 parameters are documented; machine-readable deck absent | `h`, water content, surface flux, cumulative balance, accepted times and iterations | construct/recover deck; Fortran-reference bootstrap + water solver |
| Capillary rise / shallow water table | proposed independent fixture, not a claimed manual example | equilibrium gradient, transient heads, gravity/sign convention, balance and refinement | fixture design + mesh/water path |
| Cyclic atmospheric forcing with roots | manual Chapter 11.2 provenance; complete deck absent | atmospheric mode switches, ponding, root uptake/stress/compensation, `SubReg` balance and outputs | forcing/root adapter + water acceptance |
| Solute reaction/sorption | manual Chapters 11.3–11.5 decks/analytical arrays absent; derived Example 3 deck exists | species concentration profiles, supported sorbed pools, boundary flux histories, species inventories and independent balance | M5 scientific-comparison catch-up; derived Example 3 has completed execution and diagnostic pairing; calibration remains open |
| Coupled heat/vapor | manual Chapter 11.11; deck absent | temperature, vapor/energy terms, water/heat coupling order and balance | later heat/vapor milestone |
| Hydrostatic equilibrium | proposed analytic fixture | residual gradient, flux≈0, conservation under no forcing | mesh/hydraulic core |
| Layered/inclined mesh | proposed fixture; source anchors `CosAlf`, layers | interface flux continuity, orientation/sign, refinement | mesh metrics + boundary contract |
| Event/switch/retry | proposed forcing with discontinuities and forced difficult steps | target-time alignment, boundary mode, iteration count, retry `dt`, rollback equality | scheduler/rollback implementation |
| Conservation/refinement | proposed manufactured or analytical fixture | storage/inflow/outflow/sink closure under space/time refinement | balance implementation |
| Hysteresis | manual Chapter 11.10; deck absent | reversal history, drainage/imbibition path, accepted/rejected trial state | later hysteresis milestone |
| Dual porosity | manual/source branch evidence; deck absent | mobile/immobile water and transfer mass, coupling and rollback | later dual-porosity milestone |
| Host equivalence | same canonical fixture through each selected adapter | accepted state, diagnostics, errors, cancellation/resource release | API and each host binding |

Input recovery or construction must preserve the source of every value: manual page/section, supplied artifact hash, or clearly labeled independent design. A reproduced chart is not a golden output. Expected results remain unknown until the producer milestone creates and reviews them.

## M5 solute scientific-comparison gate

M5's contract and invariant tests do not complete all possible scientific claims. Orchestration 12 delivered the derived comparison fixture and repeatability/precision review. The user accepted the M5-V result with limitations for M6 progression; the remaining evidence gaps are follow-up caveats, not a requirement for another M5 campaign before M6. Use [SCIENTIFIC_COMPARISON_RESEARCH.md](SCIENTIFIC_COMPARISON_RESEARCH.md) for historical evidence and fixture context. The first candidate is 4.08 manual Example 3 (sequential decay) because its process maps most directly to the selected Rust chain and the manual reports an analytical CHAIN comparison; a clearly labeled derived deck supplies executable input, while independent analytical output arrays remain missing.

The catch-up must establish a complete, provenance-reviewed fixture; solute-specific reference execution with output inventory/completion checks; normalized Rust/reference observables at matching space and time points; independent inventory/balance checks and an analytical cross-check where feasible; and reviewed field-specific tolerances. Preserve source-snapshot parity, independent analytical/physical validation, text-layout compatibility, and release compatibility as separate evidence claims. If any prerequisite remains unavailable, report the blocker and keep the M5 scientific gate open. Do not use the noncanonical Phase 0 smoke as an M5 baseline.

## Component and property tests

Future test files should include `tests/hydraulic_curves.rs`, `tests/root_uptake.rs`, `tests/mesh_equilibrium.rs`, `tests/time_events.rs`, and `tests/fixture_parity.rs`, plus `benches/solver.rs`; names are proposed. Test `FK`, `FC`, `FQ`, `FH`, `FAlfa`, table interpolation and derivatives across finite physical domains, saturated limits, dry limits, selector rejection, monotonicity where mathematically required, and NaN/overflow boundaries. Proptest domains must constrain positive conductivity/length, valid material parameters, ordered nodes and finite heads while including boundary-adjacent values; generated cases must never treat invalid input as a passing physical result.

Property checks should cover hydrostatic equilibrium, conservation, refinement trends, root uptake bounds, and rollback identity. Compare analytical derivatives to finite differences only where the function is differentiable and use a separately recorded kink policy. `Gauss` and `BanSol` require pivot, boundary-row, singular/near-singular, and residual checks against independently assembled small systems.

## Tolerance policy

Report per-variable units before comparison: pressure head and coordinates in the configured length unit, water content dimensionless, flux and cumulative water in configured length/time and length, concentration and sorbed mass in their declared units, temperature in degrees, and relative balance as dimensionless. For each scalar/vector field use paired acceptance `abs(a-b) <= atol + rtol * max(abs(a), abs(b))`, plus normalized L-infinity, RMS, and mesh-weighted L2 where applicable. State vector normalization and node weights explicitly; do not compare rounded text as if it were full precision.

Use absolute residual criteria when a mass-balance denominator is zero or near zero, and record the denominator and normalization branch. Compare accepted times, event times, boundary modes, iteration counts, retries, timestep history, termination status, and balances as diagnostics even when physical-result tolerances permit trajectory differences. The prompt's `1e-6` is a calibration candidate, not a universal or achieved threshold. Calibrate per field after a real Fortran-reference executable and fixture baseline, including mixed-precision versus f64 compiler comparisons.

The [2026-09-26 tolerance review](SCIENTIFIC_COMPARISON_RESEARCH.md#repeatability-precision-and-tolerance-review--2026-09-26) found exact baseline repeatability at printed precision, but precision changes up to `1e-4` in dissolved profiles and `0.00665` in reported balance. No field-specific scientific thresholds were approved. The comparator remains diagnostic (`tolerances_assigned: false`); its schema/invariant checks are not calibrated trajectory tolerances. Missing paired mesh/time convergence and an independent finite-domain analytical cross-check prevent treating current residual maxima as acceptable error bounds.

For balance reporting, retain dimensional residual and the explicitly defined normalization denominator. At exactly zero denominator, relative error is undefined: report an absolute residual and identify the absolute branch, never substitute zero percent or divide by an arbitrary epsilon. Do not turn every recorded metric into a gate. Select outputs relevant to the real use case, use limits only when justified for that use, and run further tests only when their result could change the claim or implementation. A numerical anomaly or nonmonotone refinement is not automatically a failure if it has no demonstrated practical consequence. This guidance introduces no universal threshold.

Text-layout checks are separate: verify filenames, headers, column count, delimiters, units, end markers, line endings, numeric width/precision, and deterministic ordering. A text mismatch does not by itself prove scientific mismatch; a text match does not prove numerical correctness.

## Host and platform tests

Run the same in-memory fixture through the core API and each selected Python/R/JavaScript/Wasm/C host only after those adapters are approved. Compare state and diagnostics at accepted event boundaries, invalid-input error classes, cancellation, resource release, serialization round trips, and output encoding. Interleave two independent simulations and run them concurrently to detect shared COMMON/static state, accidental global caches, data races, and cross-instance contamination. Matrix platform/compiler claims only after a release host decision; record OS, architecture, compiler, optimization, and math/runtime details.

## Allocation and performance

Allocation counting is a future implementation test, not an assertion about the current repository. Instrument the inner time/Picard loop and verify zero allocations for fixed-size stepping, while separately measuring setup, retries, output, error construction, and dynamic resizing. A failing allocation budget must report call site and phase. Criterion experiments should benchmark equivalent numerical work: same mesh, physics, forcing, accepted steps, iteration limits, output disabled/enabled as separate cases, and recorded iteration/retry counts. Compare against the patched Fortran reference executable only after parity is established, separating initialization, stepping, formatting, wrapper copies, and ensemble throughput. Report distributions, hardware, compiler flags, precision, and problem size; claim no speedup until workloads and results are comparable.

## Future commands and prerequisites

The following are proposed commands, not executable checks today:

| Command | Purpose | Prerequisite |
|---|---|---|
| `python3 tools/run_reference.py --fixture <id> --manifest <path>` | isolated Fortran-reference run and provenance capture | Fortran-reference bootstrap, tool script and fixture |
| `cargo test -p hydrus-core --test hydraulic_curves` | curve/property tests | Rust core and test files |
| `cargo test -p hydrus-core --test fixture_parity -- --nocapture` | paired reference comparison | reference outputs, parser and tolerance policy |
| `cargo test -p hydrus-core --test state_isolation` | interleaved/threaded independence | re-entrant core and host decision |
| `cargo bench -p hydrus-core --bench solver` | Criterion performance | benchmark harness and parity-approved case |
| `python3 tools/check_outputs.py --layout ...` | text-layout validation | canonical writer and expected schema |

The current document structural check validates documentation only; it is not numerical validation.

## Acceptance gates

1. Provenance gate: immutable source/archive hashes, artifact notices, patch manifest, and reproducible Fortran-reference command exist.
2. Fixture gate: each required scenario has a reviewed input source, expected observables, and isolated run contract; missing decks remain explicitly blocked.
3. Water gate: single-domain water, atmosphere, root uptake, mesh, events, rollback, and balance pass independent and Fortran-reference comparisons after tolerance calibration.
4. Later-physics gates: solute/heat, hysteresis, and required dual porosity each receive separate fixtures and coupling/rollback evidence; no feature is inferred from inventory alone.
5. Host gate: selected bindings pass equivalence, invalid-input, concurrency, cancellation, and resource tests.
6. Performance gate: allocation and benchmark results use comparable numerical work and are reported with reproducible environment metadata.

Recommendations become implementation commitments only after the corresponding gate, fixture provenance, and user scope decision are recorded. Manual/platform verification remains distinct from source-snapshot parity and does not establish release identity.

## Evidence

- `SOURCE_AUDIT.md` records ten immutable source files, the failed syntax baseline, output/source anchors, and no numerical results.
- `STATE_AND_CALLS.md` records accepted/trial state, rollback obligations, mixed precision, and output lifecycle.
- `FEATURE_MATRIX.md` records first acceptance, later solute/heat/hysteresis/dual-porosity ordering, and optional scope gates.
- `HYDRUS_RESEARCH.md` §§4 and 8 identify manual Chapter 11 candidates, missing machine-readable decks, per-field tolerance recommendations, host tests, and performance separation.
- Direct anchors inspected: `OUTPUT.FOR:TLInf/NodOut/SubReg`, `WATFLOW.FOR:Gauss`, `SOLUTE.FOR:BanSol`, `MATERIAL.FOR:FK/FC/FQ`, and `SINK.FOR:FAlfa`.
- The local manual is evidence for documented cases; no golden output, passing numerical comparison, universal tolerance, or benchmark speedup is claimed.
- `SOLUTE_FORTRAN_PARITY_18.md` records complete empirical numerical parity across 44 fixtures in all 6 process groups comparing pinned Fortran (`GNU Fortran 13.3.0`, Linux x86_64) against `SoluteBackend::FortranCompatible` using declared precision intervals and strict input/output authentication. Explicit FV/BE regressions remain preserved. Physical/continuum validity claims remain strictly disclaimed.

## Open decisions

1. Who owns construction/recovery and review of missing decks and expected results, and which independent fixtures are acceptable?
2. Which compiler, precision modes, operating systems, and host bindings are release targets?
3. What per-field tolerances and mass-balance zero-denominator policy are accepted after Fortran-reference calibration?
4. Which optional physics and output formats receive performance and host-equivalence commitments?
5. Which exact patched-reference semantics are permitted for MSFLIB and the `iMetHour` blocker?

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

At the time of the 22.x closure, M5-V was partial/open: study-level thresholds (`reference/solute_verification/THRESHOLDS_V2.json`, `cases_sha256` `c236eb7c…`, `holdout_run_permitted: false`) were approved under D10, holdouts were not run (no holdout-level thresholds assigned; holdout mesh/dt not isolated), and that record did not unblock M6. See the current disposition at the top of this document.

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
| M5-V at 22.x closure | **Partial; open.** Unassigned fields remained unmet (`SOLUTE_VERIFICATION_SPEC.md:409`); the later user decision accepts the result with limitations and unblocks M6 progression. |

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

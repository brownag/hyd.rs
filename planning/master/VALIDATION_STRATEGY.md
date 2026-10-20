# Validation and performance strategy

> [!WARNING]
> **Orchestration 20 Correction Banner:**
> The `18.15.1` parity run invoked the Fortran reference runner inside `tools/solute_rust_candidate_adapter.py` rather than the Rust compatibility solver (`crates/hydrus-core/src/solute_fortran_compat/**`).
> Consequently, the default solute backend has been reset to `FiniteVolumeBackwardEuler` until genuine Rust-vs-Fortran parity passes across all 44 fixtures in Orchestration 20.
> Additionally, `iPsi=3` (`SOLUTE.FOR:1570-1579`) is an active column-6 continuation formula rather than a commented-out no-op.

## Fortran reference harness

The Fortran reference executable is a separately identified copy of the pinned ten-file snapshot. Phase 0 preserves the originals, applies a reviewable GNU Fortran/Linux portability patch for the known `MSFLIB` and `iMetHour` blockers, and records source/archive, patch, compiler, flags, executable, host, and date. It does not silently promote precision or rewrite solute algorithms. The patched executable passed the explicitly noncanonical water smoke, so the runner/build path works; no canonical scientific solute run or expected output exists yet. See [Phase 0 evidence](PHASE_0_REFERENCE_EVIDENCE.md).

Each fixture must run in its own temporary output directory with immutable input copies, an explicit wall-clock timeout, captured stdout/stderr, exit status, expected completion marker (`CloseOutput`/`end` where applicable), file inventory, and no stale outputs. A manifest records fixture ID, input provenance, source and patch hashes, compiler/toolchain, flags, precision mode, platform, command, timeout, output hashes, and observed warnings. A run is incomplete if it times out, exits nonzero, omits required outputs, or writes malformed/truncated records. The current runner also supports the derived Example 3 solute case; strict fixture parsing and diagnostic comparison now exist. M5-V remains open pending analytical/refinement evidence and calibrated tolerances.

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

M5's contract and invariant tests do not complete its scientific validation. Orchestration 12 delivered the derived comparison fixture and repeatability/precision review; further M5 evidence is required before Phase 6 heat/temperature planning. Use [SCIENTIFIC_COMPARISON_RESEARCH.md](SCIENTIFIC_COMPARISON_RESEARCH.md) as the evidence and fixture-selection brief. The first candidate is 4.08 manual Example 3 (sequential decay) because its process maps most directly to the selected Rust chain and the manual reports an analytical CHAIN comparison; a clearly labeled derived deck now supplies executable input, while independent analytical output arrays remain missing.

The catch-up must establish a complete, provenance-reviewed fixture; solute-specific reference execution with output inventory/completion checks; normalized Rust/reference observables at matching space and time points; independent inventory/balance checks and an analytical cross-check where feasible; and reviewed field-specific tolerances. Preserve source-snapshot parity, independent analytical/physical validation, text-layout compatibility, and release compatibility as separate evidence claims. If any prerequisite remains unavailable, report the blocker and keep the M5 scientific gate open. Do not use the noncanonical Phase 0 smoke as an M5 baseline.

## Component and property tests

Future test files should include `tests/hydraulic_curves.rs`, `tests/root_uptake.rs`, `tests/mesh_equilibrium.rs`, `tests/time_events.rs`, and `tests/fixture_parity.rs`, plus `benches/solver.rs`; names are proposed. Test `FK`, `FC`, `FQ`, `FH`, `FAlfa`, table interpolation and derivatives across finite physical domains, saturated limits, dry limits, selector rejection, monotonicity where mathematically required, and NaN/overflow boundaries. Proptest domains must constrain positive conductivity/length, valid material parameters, ordered nodes and finite heads while including boundary-adjacent values; generated cases must never treat invalid input as a passing physical result.

Property checks should cover hydrostatic equilibrium, conservation, refinement trends, root uptake bounds, and rollback identity. Compare analytical derivatives to finite differences only where the function is differentiable and use a separately recorded kink policy. `Gauss` and `BanSol` require pivot, boundary-row, singular/near-singular, and residual checks against independently assembled small systems.

## Tolerance policy

Report per-variable units before comparison: pressure head and coordinates in the configured length unit, water content dimensionless, flux and cumulative water in configured length/time and length, concentration and sorbed mass in their declared units, temperature in degrees, and relative balance as dimensionless. For each scalar/vector field use paired acceptance `abs(a-b) <= atol + rtol * max(abs(a), abs(b))`, plus normalized L-infinity, RMS, and mesh-weighted L2 where applicable. State vector normalization and node weights explicitly; do not compare rounded text as if it were full precision.

Use absolute residual criteria when a mass-balance denominator is zero or near zero, and record the denominator and normalization branch. Compare accepted times, event times, boundary modes, iteration counts, retries, timestep history, termination status, and balances as diagnostics even when physical-result tolerances permit trajectory differences. The prompt's `1e-6` is a calibration candidate, not a universal or achieved threshold. Calibrate per field after a real Fortran-reference executable and fixture baseline, including mixed-precision versus f64 compiler comparisons.

The [2026-09-26 tolerance review](SCIENTIFIC_COMPARISON_RESEARCH.md#repeatability-precision-and-tolerance-review--2026-09-26) found exact baseline repeatability at printed precision, but precision changes up to `1e-4` in dissolved profiles and `0.00665` in reported balance. No field-specific scientific thresholds were approved. The comparator remains diagnostic (`tolerances_assigned: false`); its schema/invariant checks are not calibrated trajectory tolerances. Missing paired mesh/time convergence and an independent finite-domain analytical cross-check prevent treating current residual maxima as acceptable error bounds.

For balance reporting, retain dimensional residual and the explicitly defined normalization denominator. At exactly zero denominator, relative error is undefined: report an absolute residual and identify the absolute branch, never substitute zero percent or divide by an arbitrary epsilon. A scientifically justified absolute acceptance bound and near-zero transition scale remain unassigned; zero/near-zero independent cases and precision/refinement error budgets must establish them. This reporting rule introduces no new regression acceptance threshold.

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

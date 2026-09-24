# Phase 4 water acceptance evidence

## Purpose and evidence standard

This record assesses the first-water scope in [the sequential phase plan](../HYDRUS1D_RUST_PORT_PLAN.md#sequential-phase-plan) using the accepted Orchestration 9 implementation reports and Rust contract tests. It records what those artifacts demonstrate and separates it from scientific validation. Phase 0's reviewed GNU Fortran smoke executable is noncanonical; Phase 1–3 evidence is source-algebra and Rust-contract evidence. None supplies calibrated HYDRUS outputs or a parity oracle.

The Phase 4 scenarios are deterministic, independent in-memory tests. The dry-column and cyclic atmosphere labels follow candidate cases in the validation strategy (manual Chapters 11.1 and 11.2), whose machine-readable decks and expected outputs are unavailable. Capillary rise is a proposed analytic direction check. The combined atmosphere/root case is assembled from the Feddes source-anchored contract. These are bounded contract checks, not recovered manual cases, calibrated trajectories, or Fortran comparisons.

## Delivered water contract

The filesystem-free `hydrus-core` water stepping API remains `f64` and uses validated `Mesh1D` material mapping, caller-owned/preallocated workspaces, typed errors, and accepted/trial state promotion. `WaterStepInput` selects a supported physical lower boundary and may provide Feddes root uptake. `ForcingSchedule` supplies rates and event times with explicit time and water-depth-rate unit labels. Atmospheric stepping clips to forcing events and the requested target time.

Supported lower modes are fixed head, signed prescribed flux, and free drainage. Positive lower flux is into the domain. Fixed-head measured exchange is caller-supplied; free-drainage sign mapping is derived from source row algebra and Rust balance conventions. Upper atmospheric behavior supports precipitation/evaporation demand, flux/head switching, and ponded surface storage. Root uptake supports the source-anchored piecewise Feddes reduction with compensation, trial-dependent realized sinks, and an available-water cap. With no optional root input, the existing caller-sink behavior remains.

Seepage-face and water-table lower modes are represented as explicitly unsupported selections and return typed errors; they do not silently fall back to supported modes. Solute, heat, vapor/energy, hysteresis, dual porosity, density, snow/interception, tile drainage, inverse methods, special chemistry, broad legacy I/O, host bindings, and release compatibility are outside this first-water acceptance.

## Atmospheric iteration, rollback, and accounting

The accepted 9.1.1 report records trial-local upper mode and candidate ponding, convergence against the mode used by the final solve, and promotion only after acceptance. Retries/errors discard the trial candidate. Transition records include demand, mode, top head, soil flux, and candidate surface storage; they are held in a workspace-preallocated buffer with explicit capacity rather than allocated in each solve/retry. The atmospheric tests assert transition traces for the supplied rain and evaporation cases, event cursor promotion, accepted ponding, and state equality after rejected trials.

The accepted report separates soil storage change, surface storage change, soil boundary flux, and external atmospheric exchange; `storage_change` is total storage. Test conventions use positive boundary flux into the domain, positive sink as extraction, and signed balance quantities per area. Atmospheric exchange is the external forcing contribution; it can differ from soil boundary flux when surface storage changes. The root sink is timestep-integrated and mesh-control-volume weighted. Tests assert near-zero residuals for the exercised bounded cases. This verifies internal accounting consistency for those tests; it does not establish physical calibration.

Specific rollback evidence includes the atmospheric exact rain/evaporation tests and cyclic acceptance scenario's invalid-input rejection, which checks the complete accepted state remains unchanged. The 9.1.1 report also records failed-trial/retry rollback behavior. No evidence here resolves the physical interpretation of evaporation critical-head semantics beyond the implemented contract, nor does it establish Fortran parity.

## Fixture inventory and exact targeted tests

| Scenario | Provenance and observed contract | Exact command and recorded result |
|---|---|---|
| Dry-column infiltration | Chapter 11.1 candidate reconstructed in memory; checks accepted time, bounded heads, atmospheric flux mode, atmospheric exchange and balance closure. | `cargo test -p hydrus-core --test water_acceptance -- --nocapture` — 5 passed, 0 failed (whole acceptance binary). |
| Capillary rise | Independently constructed analytic direction/sign check; checks fixed-head lower boundary, head ordering, accepted time, signed lower exchange, and balance closure. Not a calibrated transient or shallow-water-table result. | Same acceptance command — 5 passed, 0 failed. |
| Cyclic atmospheric ponding/switching | Chapter 11.2 candidate reconstructed in memory; checks event clipping/cursor, mode transition, nonnegative ponding, balance closure, and unchanged accepted state after rejection. | Same acceptance command — 5 passed, 0 failed. |
| Cyclic atmosphere with Feddes stress/compensation | In-memory combined candidate; checks event progression, positive realized sink, active stress, compensation domain, nonnegative ponding, and balance closure. | Same acceptance command — 5 passed, 0 failed. |

The same acceptance binary has a fifth test for typed seepage-face and water-table rejection and unchanged state on validation error. Scenario inputs use cm for heads/coordinates, days for time, cm/day for rates, dimensionless theta, root density per length, and signed balance quantities in cm per area. Test values are inputs to independent contract checks, not expected reference outputs. See [`water_acceptance.rs`](../../crates/hydrus-core/tests/water_acceptance.rs) for per-case assertions and provenance comments.

Other exact targeted commands and results from accepted reports:

- `cargo test -p hydrus-core --test atmospheric` — 9 passed, 0 failed; transition, rain/evaporation balance, event, and rollback contracts. See [`atmospheric.rs`](../../crates/hydrus-core/tests/atmospheric.rs).
- `cargo test -p hydrus-core --test lower_boundary` — 5 passed, 0 failed; supported modes, signed flux, invalid/unsupported values, and rollback.
- `cargo test -p hydrus-core --test root_uptake` — 7 passed, 0 failed; Feddes breakpoints, stress, compensation, caps, mesh weighting, validation, and transaction behavior.
- `cargo test -p hydrus-core --test water_acceptance -- --nocapture` — 5 passed, 0 failed.

The 9.1.1–9.4.1 reports each record successful `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` at their respective wave gates. The reports respectively record workspace totals of 71, 76, 83, and 88 integration tests, all passed. These are per-wave runs, not a single combined test count. The final Orchestration 9 workspace gate is run by the orchestrator after this evidence task.

## Phase 4 exit assessment

Status labels apply to evidence in this repository, not to a claim of scientific correctness.

| Exit criterion | Status | Evidence and limit |
|---|---|---|
| Single-domain `iModel=0` water foundation | **Met (contract evidence)** | Phase 1–3 API, property, mesh, stencil, solver, transaction, and typed-error evidence; no calibrated physical validation. |
| Atmospheric precipitation/evaporation, event alignment, ponding, and upper flux/head switching | **Met (contract evidence)** | 9.1.1 report, 9/9 atmospheric tests, and cyclic fixtures cover transitions, event clipping, storage, and rollback. Evaporation critical-head physical semantics remain unresolved. |
| Lower fixed-head, prescribed-flux, and free-drainage behavior | **Met (contract evidence)** | 9.2.1 report and 5/5 lower-boundary tests cover supported modes, signs, diagnostics, and rollback. Free-drainage mapping is source-algebra-derived, not executable parity. |
| Feddes stress with compensation and realized root sink | **Met (contract evidence)** | 9.3.1 report and 7/7 root tests cover curve, compensation, caps, mesh weighting, and accepted/trial behavior; combined cyclic/root fixture exercises stress, compensation, sink, and balance. No independent calibrated root trajectory exists. |
| Dry-column infiltration, capillary rise, and cyclic atmosphere/root acceptance scenarios | **Met (bounded contract coverage)** | Four deterministic in-memory scenarios pass the 5/5 acceptance binary and record provenance, observables, units, balance convention, relevant times, and limitations. Chapter 11 decks/outputs are absent; capillary rise is an independent direction check. |
| Typed rejection of unsupported seepage-face and water-table selections | **Met (contract evidence)** | Acceptance test checks typed errors and unchanged state for both selectors. |
| Water balance sign-off and conservation | **Evidence-limited** | Separate storage, surface storage, soil flux, atmospheric exchange, and sink terms close within asserted test tolerances in constructed cases. This is internal bounded accounting evidence, not independent physical mass-balance validation or calibrated tolerance sign-off. |
| Independent/reference comparison, calibrated tolerance, and scientific fixture provenance | **Unmet / unavailable** | No canonical Phase 4 deck or reviewed expected output, no independently run/reference comparison, and no calibrated tolerance. Phase 0 smoke execution is explicitly noncanonical and not a scientific oracle. |
| Full workspace quality gates after completion of Orchestration 9 | **Pending final gate** | Prior wave reports pass all four workspace checks; the orchestrator must record the final serial gate after this task. |

## Limitations and Phase 5 handoff

There is no Phase 4 Fortran parity result, calibrated numerical baseline/tolerance, canonical manual fixture, universal tolerance, physical-correctness proof, release-compatibility claim, or performance claim. Phase 0's GNU Fortran executable and smoke run establish narrow build/run provenance only. Phase 1–3 source anchors are not executable reference results. Phase 4 tests establish behavior of the implemented Rust contracts over selected inputs.

The plan's Phase 5 sequence is solute transport, which requires an accepted Phase 4 water state plus a solute fixture/provenance plan. This evidence supports **Phase 5 planning readiness only**, conditional on the orchestrator's final workspace gate and review of the evidence-limited Phase 4 criteria. It does not authorize Phase 5 implementation or characterize water behavior as scientifically validated. A Phase 5 plan must define selected solute/reaction/transfer scope, input and fixture provenance, concentration and solute-balance observables, and typed unsupported behavior before implementation; independent comparison should be added if a reviewed oracle becomes available.

## Sources

- [Sequential phase plan](../HYDRUS1D_RUST_PORT_PLAN.md#sequential-phase-plan), [Phase 0 evidence](PHASE_0_REFERENCE_EVIDENCE.md), [Phase 1 evidence](PHASE_1_HYDRAULIC_EVIDENCE.md), [Phase 2 evidence](PHASE_2_MESH_STENCIL_EVIDENCE.md), and [Phase 3 evidence](PHASE_3_WATER_SOLVER_EVIDENCE.md).
- [Orchestration 9 execution plan](../EXECUTION_PLAN_9.md) and [accepted task reports](../STATE_9.md#task-reports), including the detailed [9.1.1](../tasks/9.1.1.todo.md), [9.2.1](../tasks/9.2.1.todo.md), [9.3.1](../tasks/9.3.1.todo.md), and [9.4.1](../tasks/9.4.1.todo.md) records.
- [`atmospheric.rs`](../../crates/hydrus-core/tests/atmospheric.rs), [`lower_boundary.rs`](../../crates/hydrus-core/tests/lower_boundary.rs), [`root_uptake.rs`](../../crates/hydrus-core/tests/root_uptake.rs), and [`water_acceptance.rs`](../../crates/hydrus-core/tests/water_acceptance.rs).

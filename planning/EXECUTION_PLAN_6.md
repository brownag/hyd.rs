# Orchestration 6 — Phase 4 first water acceptance

Status: **renumbered from the prior orchestration-5 draft on 2026-09-23; execution not started.**
Master authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 4.
Entry evidence: Phase 0 provenance, Phase 1 hydraulic evidence, Phase 2 mesh/stencil evidence, and future Phase 3 solver evidence.

## Goal and scope

Implement and validate the first accepted single-domain water product over the completed Phase 3 solver: event-aligned atmospheric forcing, ponding and upper switching, supported lower boundary policies, one documented root stress/compensation model, water balances, typed unsupported combinations, and provenance-labeled in-memory fixtures.

Out of scope: solute, heat, vapor/energy, hysteresis, dual porosity, density, snow/interception, drainage/tile extensions, inverse methods, special chemistry, legacy I/O/CLI/host bindings, release compatibility, universal tolerances, and parity claims without reference fixtures.

## Entry gate and decisions

Phase 3 evidence, solver paths, public stepping API, transaction snapshot, retry behavior, boundary-row contract, and balance diagnostics must be present before Wave 1. If absent, stop without implementation changes.

| ID | Decision |
|---|---|
| D01 | Keep `iModel=0` as the only hydraulic family and reject unsupported selectors. |
| D02 | Support fixed-head, prescribed-flux, and free-drainage lower modes; reject un-evidenced seepage/water-table combinations. |
| D03 | Use event-aligned forcing and accepted-step-only accounting; rejected trials restore all mutable state. |
| D04 | Keep fixture provenance explicit and do not manufacture Fortran expected values. |

## Wave table

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 1 | 6.1.1 | Phase 3 handoff | Water product configuration and capability validation |
| 2 | 6.2.1 | 6.1.1 | Atmospheric forcing, event alignment, ponding, upper switching |
| 3 | 6.3.1 | 6.2.1 | Lower boundaries and root stress/compensation |
| 4 | 6.4.1 | 6.3.1 | Water fixtures, balances, and acceptance tests |
| 5 | 6.5.1 | 6.4.1 | Phase 4 evidence and Phase 5 handoff |

Tasks are serial because each extends the shared water API/state boundary. Use the existing worker approval and escalation policy; full workspace checks run serially after the final wave.

## Task specifications

### 6.1.1 — Integrate the accepted water product contract

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** Phase 3 evidence and verified solver.
- **Files:** `crates/hydrus-core/src/water_state.rs`, `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/lib.rs`, `crates/hydrus-core/src/error.rs`; new `crates/hydrus-core/tests/water_contract.rs`; `planning/tasks/6.1.1.todo.md`; scratch `.development/6.1.1/`. If the Phase 3 handoff chooses different public paths, rescope before execution.
- **Read/anchors:** `master/API_AND_IO.md#core-signatures`, `master/STATE_AND_CALLS.md#state-inventory`, `master/NUMERICAL_DESIGN.md#time-and-nonlinear-solve`; `rg -n 'State|Workspace|step|advance_to|Balance|Capability' crates/hydrus-core/src`.
- **Spec:** Define validated water configuration and accepted-step results around the Phase 3 solver, rejecting unsupported selectors before allocation or stepping. Do not add forcing physics.
- **Acceptance:** targeted water contract tests, fmt, and Clippy.

### 6.2.1 — Add atmospheric forcing and upper-boundary switching

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 6.1.1.
- **Files:** new `crates/hydrus-core/src/forcing.rs`, `crates/hydrus-core/src/atmospheric.rs`, required `lib.rs`/`error.rs` exports; new atmospheric tests; TODO; scratch `.development/6.2.1/`.
- **Read/anchors:** `TIME.FOR:3-30`, `WATFLOW.FOR:359-430`, `master/FEATURE_MATRIX.md#first-acceptance`, `master/API_AND_IO.md#lifecycle-and-forcing`; `rg -n 'TmCont|Shift|pond|precip|evap|Atmos|event' src_reference crates/hydrus-core/src`.
- **Spec:** Implement forcing schedules, event alignment, precipitation/evaporation, ponding, upper switching, validation, and rollback-safe cursors.
- **Acceptance:** targeted atmospheric tests, fmt, and Clippy.

### 6.3.1 — Add lower boundaries and root stress/compensation

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 6.2.1.
- **Files:** new `crates/hydrus-core/src/lower_boundary.rs`, `crates/hydrus-core/src/root_uptake.rs`, required `lib.rs`/`error.rs` exports; lower/root tests; TODO; scratch `.development/6.3.1/`.
- **Read/anchors:** `master/FEATURE_MATRIX.md#capability-matrix`, `master/VALIDATION_STRATEGY.md#fixture-matrix`; `rg -n 'qGWLF|FreeD|SeepF|fixed.head|flux|Feddes|stress|compensation|root' src_reference crates/hydrus-core/src`.
- **Spec:** Implement supported lower modes and one documented root stress/compensation model with typed rejection and bounded sinks. Keep un-evidenced seepage/water-table combinations rejected.
- **Acceptance:** targeted lower-boundary/root tests, fmt, and Clippy.

### 6.4.1 — Add fixture-backed water acceptance evidence

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 6.3.1.
- **Files:** new `crates/hydrus-core/tests/water_acceptance.rs`, test-only fixtures under `crates/hydrus-core/tests/fixtures/` or `.development/6.4.1/`, and no production source except explicitly approved balance helpers; TODO; scratch `.development/6.4.1/`.
- **Read/anchors:** `master/VALIDATION_STRATEGY.md#fixture-matrix`, `#tolerance-policy`, `master/ROADMAP_AND_RISKS.md#phase-checklists`; `rg -n 'balance|accepted|retry|event|fixture|root|pond' crates/hydrus-core/tests planning/master`.
- **Spec:** Build deterministic dry-column, cyclic atmospheric/ponding, and root-stress/compensation cases. Record provenance, observables, units, balance policy, accepted times, iterations, retries, and unsupported behavior without inventing reference values.
- **Acceptance:** targeted acceptance with `--nocapture`, workspace tests, fmt, Clippy, and diff checks.

### 6.5.1 — Record Phase 4 evidence and Phase 5 handoff

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 6.4.1.
- **Files:** `planning/master/PHASE_4_WATER_ACCEPTANCE_EVIDENCE.md`, TODO, scratch `.development/6.5.1/`.
- **Read/anchors:** all Phase 4 reports, master Phase 4/5 rows, feature matrix, validation strategy, state/call audit, and Phase 0–3 evidence; `rg -n 'atmospheric|pond|switch|fixed.head|free.drainage|root|balance|unsupported|solute' crates/hydrus-core/src crates/hydrus-core/tests planning/master`.
- **Spec:** Record supported behavior, fixture provenance, transaction evidence, limitations, and the Phase 5 solute handoff with the established evidence boundaries.
- **Acceptance:** workspace tests, fmt, Clippy, diff checks, and required evidence headings.

## Execution policy

Workers produce at-most-12-step micro-plans, stop for approval, follow the escalation ladder, and never backfill Phase 3. One conventional commit per accepted task.

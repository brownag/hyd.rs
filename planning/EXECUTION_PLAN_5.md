# Orchestration 5 — Phase 3 Richards/Picard water solver

Status: **approved by the user on 2026-09-23; execution not started.**
Master authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 3.
Entry evidence: [Phase 2 mesh/stencil evidence](master/PHASE_2_MESH_STENCIL_EVIDENCE.md).

## Goal and scope

Implement the Phase 3 single-domain Richards water-solver foundation: generic boundary-row assembly, an in-house tridiagonal water solver, Picard iteration, strict adaptive retry/rollback transactions, accepted-step balance diagnostics, and a filesystem-free public numerical contract. Use the current Rust `f64` hydraulic and mesh APIs and preserve the observed Phase 2 `Reset` stencil.

Physical boundary policies are not implemented here. Atmospheric forcing, ponding and upper switching, lower fixed-head/flux/free-drainage policies, root uptake, and first-water fixtures belong to Phase 4 (orchestration 6). No solute, heat, vapor, hysteresis, dual porosity, legacy I/O, CLI, host bindings, or release compatibility is included.

### Verbatim constraints

> “Plan only the next incomplete phase in the sequence below.”
>
> “Unsupported selector or feature combinations must fail validation rather than fall back to a nearby model.”
>
> “Do not widen the first water acceptance to include solute, heat, hysteresis, dual porosity, or optional physics.”

## Decisions

| ID | Decision |
|---|---|
| D01 | Phase 3 owns generic boundary-row/solver plumbing; physical forcing and boundary policies remain Phase 4. (2026-09-23) |
| D02 | Use `f64` throughout the Phase 3 Rust contract; mixed-precision compatibility requires later fixture calibration. (2026-09-23) |
| D03 | Rejected trials restore all mutable state, reduce `dt`, retry, and return typed non-convergence at `dt_min`. (2026-09-23) |
| D04 | Use an in-house preallocated Thomas-style tridiagonal kernel; add no numerical dependency or transport solver. (2026-09-23) |
| D05 | Execute four serial waves because solver layers share the core API/state boundary; retain the three-worker ceiling but do not parallelize these tasks. (2026-09-23) |
| D06 | Require source-algebra/property, convergence/retry/rollback, balance, and scoped fixed-size allocation tests; none is a Fortran parity result. (2026-09-23) |

## Reconnaissance

- Current green baseline: `cargo test --workspace` passes 29 tests; `cargo fmt --check`, workspace Clippy with `-D warnings`, and `git diff --check` pass.
- Current implementation is Phase 1/2 only: `crates/hydrus-core/src/hydraulics/{mod,types,geometry,stencil,table,van_genuchten}.rs`; no solver/state/stepper module exists.
- The core is filesystem-free and currently exposes typed hydraulic errors, `Mesh1D`, geometry helpers, and `assemble_reset_stencil`.
- Source anchors: `WATFLOW.FOR:173-353` (`Reset`/`Gauss`), `WATFLOW.FOR:3-168` (`WatFlow`), `WATFLOW.FOR:1079-1125` (`Update`), `TIME.FOR:3-30`, and `HYDRUS.FOR:417-445`.
- Phase 2 explicitly defers boundary transformation, elimination, convergence, retries, rollback, and balances.
- Scientific baseline is unavailable; current tests establish Rust contracts and source algebra, not parity, tolerances, release compatibility, or performance.

## Wave table

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 1 | 5.1.1 | Phase 2 handoff | Generic boundary-row contract and typed validation |
| 2 | 5.2.1 | 5.1.1 | Preallocated water tridiagonal kernel and pivot diagnostics |
| 3 | 5.3.1 | 5.2.1 | Picard stepping, transactions, retry, balance, allocation boundary |
| 4 | 5.4.1 | 5.3.1 | Phase 3 evidence and Phase 4 handoff |

All waves are serial. The orchestrator runs workspace checks after each accepted task and commits one conventional commit per task.

## Pre-wave gate

Record in `STATE_5.md`: `git status --short`; `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check`. Confirm the Phase 2 handoff, current exports, no existing solver module, and the source anchors above.

## Shared constraints

- Preserve source hashes, Phase 0–2 evidence, hydraulic behavior, and mesh/stencil behavior.
- Keep `hydrus-core` filesystem-free; no fixture reader, clock, callback, or host binding enters the solver loop.
- Use `f64` in all new public and internal solver paths.
- Allocate or resize only during setup; fixed-size accepted steps, retries, restore, assembly, elimination, and diagnostics use preallocated workspace storage.
- Generic row transformation must not encode atmospheric, ponding, seepage, root, or lower-boundary policy.
- No silent selector fallback, invented Fortran expected values, or parity/tolerance/performance claim.
- Workers write only approved files and task TODOs; the orchestrator owns state updates, full-suite checks, and commits.

## Task specifications

### 5.1.1 — Define generic boundary-row assembly

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** Phase 2 handoff.
- **Must edit:** `crates/hydrus-core/src/error.rs`, `crates/hydrus-core/src/hydraulics/mod.rs` only for exports, and new `crates/hydrus-core/src/hydraulics/boundary.rs`.
- **Can create:** `crates/hydrus-core/tests/boundary_rows.rs`, `planning/tasks/5.1.1.todo.md`.
- **Scratch only:** `.development/5.1.1/`.
- **Read:** `master/NUMERICAL_DESIGN.md#discretization`, `#linear-solver`; `master/API_AND_IO.md#core-signatures`; `master/PHASE_2_MESH_STENCIL_EVIDENCE.md`; `WATFLOW.FOR:308-430`.
- **Anchors:** `rg -n 'Reset|Gauss|Shift|Boundary|ResetRow|HydraulicError' crates/hydrus-core/src src_reference/WATFLOW.FOR`.

Define a filesystem-free typed row/boundary contract for generic Dirichlet, Neumann/flux, and natural/free row transformation. Validate finite coefficients, dimensions, row location, and pivot-relevant values. Preserve explicit signs and reject unsupported variants. Do not implement physical forcing or boundary policy.

**Acceptance:** `cargo test -p hydrus-core --test boundary_rows`; fmt, Clippy, and diff checks. Tests cover algebra, edge rows, finite/domain failures, and unsupported policy rejection.

### 5.2.1 — Implement the water tridiagonal kernel

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 5.1.1.
- **Must edit:** new `crates/hydrus-core/src/hydraulics/linear.rs`; `mod.rs` only for exports.
- **Can create:** `crates/hydrus-core/tests/tridiagonal.rs`, `planning/tasks/5.2.1.todo.md`.
- **Scratch only:** `.development/5.2.1/`.
- **Read:** `master/NUMERICAL_DESIGN.md#linear-solver`, `#memory-and-precision`; `WATFLOW.FOR:308-353`; `SOLUTE.FOR:945-957` only to distinguish transport behavior.
- **Anchors:** `rg -n 'Gauss|BanSol|pivot|Singular|workspace|lower|upper' crates/hydrus-core/src src_reference/WATFLOW.FOR src_reference/SOLUTE.FOR`.

Implement an in-place Thomas-style solver over caller-owned lower, diagonal, upper, and RHS buffers. Return typed finite-input and singular/near-zero-pivot failures, preserve the solution in the RHS/output buffer, and report minimum-pivot diagnostics. Add no dependency and no transport/general-band solver.

**Acceptance:** `cargo test -p hydrus-core --test tridiagonal`; fmt, Clippy, and diff checks. Tests cover hand-solved and multiple-size systems, pivot thresholds, non-finite/dimension failures, and buffer reuse.

### 5.3.1 — Implement transactional Picard water stepping

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 5.2.1.
- **Must edit:** new `crates/hydrus-core/src/water_state.rs`, `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/lib.rs`, and `crates/hydrus-core/src/error.rs`; `hydraulics/mod.rs` only for required solver exports.
- **Can create:** `crates/hydrus-core/tests/water_solver.rs`, `crates/hydrus-core/tests/water_transaction.rs`, `planning/tasks/5.3.1.todo.md`.
- **Scratch only:** `.development/5.3.1/`.
- **Read:** `master/NUMERICAL_DESIGN.md#time-and-nonlinear-solve`, `#balances-and-coupling`, `#memory-and-precision`; `master/API_AND_IO.md#core-signatures`, `#lifecycle-and-forcing`; `master/STATE_AND_CALLS.md#state-inventory`; `WATFLOW.FOR:3-168,1079-1125`; `TIME.FOR:3-30`.
- **Anchors:** `rg -n 'Simulation|State|Workspace|Step|Picard|ConvgF|ItCum|Update|Balance|retry|snapshot' crates/hydrus-core/src crates/hydrus-core/tests src_reference/WATFLOW.FOR src_reference/HYDRUS.FOR`.

Add the minimum filesystem-free state, preallocated workspace, convergence policy, accepted-step report, and water-balance diagnostic needed to run the Phase 2 stencil through Picard iterations. Implement accepted/trial ownership, strict rollback of mutable step/accounting state, adaptive `dt` retry, typed `dt_min` failure, accepted-only promotion, and deterministic test-only hydraulic/stencil inputs. Do not add physical forcing or boundary policies.

**Acceptance:** `cargo test -p hydrus-core --test water_solver` and `cargo test -p hydrus-core --test water_transaction`; fmt, Clippy, diff checks, and a scoped allocation test. Cover convergence, rejected-trial identity, retry `dt`, minimum-step failure, balance residual/zero-denominator policy, accepted-only counters, finite diagnostics, and no filesystem dependency.

### 5.4.1 — Record Phase 3 evidence and Phase 4 handoff

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 5.3.1.
- **Must edit:** none.
- **Can create:** `planning/master/PHASE_3_WATER_SOLVER_EVIDENCE.md`, `planning/tasks/5.4.1.todo.md`.
- **Scratch only:** `.development/5.4.1/`.
- **Read:** all 5.1.1–5.3.1 reports; `master/NUMERICAL_DESIGN.md`; `master/API_AND_IO.md`; `master/STATE_AND_CALLS.md`; `master/VALIDATION_STRATEGY.md`; `master/PHASE_2_MESH_STENCIL_EVIDENCE.md`; roadmap Phase 3/4 rows.
- **Anchors:** `rg -n 'boundary|pivot|Picard|retry|rollback|balance|allocation|unsupported|parity|Phase 4' crates/hydrus-core/src crates/hydrus-core/tests planning/master`.

Record the generic boundary contract, kernel, Picard lifecycle, transaction/retry behavior, balance denominator policy, allocation boundary, typed unsupported behavior, and exact test evidence. Distinguish source-algebra/contract evidence from parity and calibration. Hand off Phase 4 atmospheric, physical-boundary, root, fixture, and first-water scope.

**Acceptance:** `cargo test --workspace`; fmt; workspace Clippy; `git diff --check`; and a structure assertion requiring `## Boundary contract`, `## Linear solver`, `## Picard stepping`, `## Transactions and retries`, `## Balances and allocation`, `## Unsupported behavior`, `## Exit checklist`, `## Phase 4 handoff`, `not a Fortran parity result`, and `provenance`.

## Execution policy

Workers produce at-most-12-step micro-plans and stop for orchestrator approval. Full suites run serially after each wave. One conventional no-phase/wave commit per accepted task. One failed targeted verification receives one retry, then escalation to `gpt-5.6-terra` medium effort. Out-of-scope needs require plan rescoping. This orchestration authorizes no implementation until separately approved and executed by wave-orchestration.

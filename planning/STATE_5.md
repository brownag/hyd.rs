# State — Orchestration 5: Phase 3 Richards/Picard water solver

Plan status: **approved by the user on 2026-09-23.**
Execution status: **not started.**
Master authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 3.
Resume from [EXECUTION_PLAN_5.md](EXECUTION_PLAN_5.md); worker prompt: [PROMPT_TEMPLATE_5.md](PROMPT_TEMPLATE_5.md).

## Baseline checklist

- [x] Phase 0 provenance handoff exists: `master/PHASE_0_REFERENCE_EVIDENCE.md`.
- [x] Phase 1 hydraulic handoff exists: `master/PHASE_1_HYDRAULIC_EVIDENCE.md`.
- [x] Phase 2 mesh/stencil handoff exists: `master/PHASE_2_MESH_STENCIL_EVIDENCE.md`.
- [x] Current branch contains Phase 1/2 implementation only; no solver/state/stepper module exists.
- [x] `cargo test --workspace` passes: 29 tests, 0 failed.
- [x] `cargo fmt --check` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [x] `git diff --check` passes.
- [x] Source anchors reviewed: `WATFLOW.FOR:3-168,173-353,1079-1125`, `TIME.FOR:3-30`, `HYDRUS.FOR:417-445`.

Scientific baseline: **unavailable.** Phase 3 tests are source-algebra and Rust contract evidence; they do not establish Fortran parity, universal tolerances, release compatibility, or performance.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 5.1.1 | Generic boundary-row contract | done | pending |
| 2 | 5.2.1 | Preallocated water tridiagonal kernel | done | pending |
| 3 | 5.3.1 | Transactional Picard stepping and balances | done | pending |
| 4 | 5.4.1 | Phase 3 evidence and Phase 4 handoff | pending | — |

## Task reports

<!-- Orchestrator appends concise worker reports here. -->

## 5.1.1

- Status: done; generic endpoint row contract implemented with typed validation.
- Files: `error.rs`, `hydraulics/mod.rs`, `hydraulics/boundary.rs`, `tests/boundary_rows.rs`, task TODO.
- Tests: `cargo test -p hydrus-core --test boundary_rows` (6 passed); workspace (35 passed).
- Checks: fmt, scoped Clippy, workspace Clippy, and `git diff --check` passed.
- Deviations: none; physical boundary policy remains deferred to Phase 4.
- Gotcha: caller supplies signed flux; natural/free preserves its prepared row.

## 5.2.1

- Status: done; in-place Thomas kernel and pivot diagnostics implemented.
- Files: `hydraulics/linear.rs`, `hydraulics/mod.rs`, `tests/tridiagonal.rs`, task TODO.
- Tests: `cargo test -p hydrus-core --test tridiagonal` (5 passed); workspace (40 passed).
- Checks: fmt, scoped Clippy, workspace Clippy, and `git diff --check` passed.
- Deviations: one exact pivot comparison corrected to tolerance-based; no scope change.
- Gotcha: `TridiagonalError` is exported from `hydraulics`; RHS and diagonal are mutated.

## 5.3.1

- Status: done after mid-tier escalation; transactional Picard step and balances implemented.
- Files: `water_state.rs`, `water_solver.rs`, `lib.rs`, `error.rs`, both water tests, task TODO.
- Tests: `water_solver` (3 passed); `water_transaction` (4 passed); workspace (47 passed).
- Checks: fmt, workspace Clippy, and `git diff --check` passed.
- Deviation: initial worker failed the retry test twice; escalation excluded prescribed Dirichlet jumps from nonlinear head residuals.
- Gotcha: reported head residual covers free nodes; endpoint water residuals remain part of convergence.

## Escalations and amendments

<!-- Record retries, escalations, scope amendments, and pre-wave blockers here. -->

- 5.3.1 escalated once after two identical targeted transaction-test failures; mid-tier correction passed all checks.

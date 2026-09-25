# State — Orchestration 10: M5 single-domain solute transport

Plan status: **approved by the user on 2026-09-24.**
Execution status: **Waves 1–4 complete; Waves 5–6 pending.**
Plan: [EXECUTION_PLAN_10.md](EXECUTION_PLAN_10.md); worker prompt: [PROMPT_TEMPLATE_10.md](PROMPT_TEMPLATE_10.md).

## Baseline checklist

- [x] Orchestration 9 Waves 1–7 complete; Phase 4 evidence records planning readiness for M5 only.
- [x] Working tree clean before plan drafting.
- [x] `cargo test --workspace`: passed (88 integration tests; 0 failures).
- [x] `cargo fmt --check`: passed.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- [x] `git diff --check`: passed.
- [x] Scientific baseline remains unavailable: no canonical solute deck, calibrated tolerance, or reviewed Fortran output.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 10.1.1 | Validated solute capability contract | done | pending |
| 2 | 10.2.1 | Transactional state/workspace | done | pending |
| 3 | 10.3.1 | Advection/dispersion and boundary solve | done | pending |
| 4 | 10.4.1 | Species reaction, sorption and transfer | done | pending |
| 5 | 10.5.1 | Solute balance and acceptance fixtures | pending | — |
| 6 | 10.6.1 | M5 evidence and handoff | pending | — |

## Task reports

<!-- Append each accepted worker report here, at most 15 lines per task. -->

## 10.1.1
- **Status:** done; contract types, typed config errors, exports, and contract tests implemented.
- **Files:** `error.rs`, `lib.rs`, new `solute.rs`, new `tests/solute_contract.rs`, `tasks/10.1.1.todo.md`.
- **Tests:** `cargo test -p hydrus-core --test solute_contract` — 4 passed, 0 failed; `cargo test --workspace` — 92 passed, 0 failed.
- **Gates:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- **Deviations:** none. Formatting was corrected within scope after the first format gate.
- **Gotcha:** ordered species define the chain; each species transfers to the next, while terminal-species decay leaves the configured chain.

## 10.2.1
- **Status:** done; accepted/trial state and fixed-size workspace cover dissolved concentration, one kinetic sorbed pool, time, and per-species accounting.
- **Files:** `solute.rs`, `lib.rs`, new `tests/solute_transaction.rs`, `tasks/10.2.1.todo.md`.
- **Tests:** `cargo test -p hydrus-core --test solute_transaction` — 3 passed, 0 failed; `cargo test --workspace` — 95 passed, 0 failed.
- **Gates:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- **Deviations:** none; accounting vector copies were changed to preallocated slices after review to avoid trial-time allocation.
- **Gotcha:** 10.4/10.5 must define signs and update semantics for cumulative per-species boundary and reaction mass.

## 10.3.1
- **Status:** done; implicit finite-volume dissolved transport uses accepted water snapshots and preallocated tridiagonal buffers.
- **Files:** `solute.rs`, `lib.rs`, new `tests/solute_transport.rs`, `tasks/10.3.1.todo.md`.
- **Tests:** `cargo test -p hydrus-core --test solute_transport` — 5 passed, 0 failed; `cargo test --workspace` — 100 passed, 0 failed.
- **Gates:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- **Deviations:** none; one initial targeted run failed on a missing test config argument and was corrected; formatting was fixed within scope.
- **Gotcha:** accepted water faces are `node_count + 1`, lower/internal/upper, signed toward +x; prescribed solute boundary flux is positive inward at either endpoint.

## 10.4.1
- **Status:** done; sequential decay, equilibrium/kinetic sorption, bulk-density conversion, nonlinear bounded iteration, and rollback are implemented.
- **Files:** `solute.rs`, new `tests/solute_reactions.rs`, `tasks/10.4.1.todo.md`.
- **Tests:** `cargo test -p hydrus-core --test solute_reactions` — 6 passed, 0 failed; `cargo test --workspace` — 106 passed, 0 failed.
- **Gates:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- **Deviations:** none; escalated from small worker to mid-tier after two failed compile attempts. Test assertions were corrected to compare nonlinear retardation with a nonsorbing case and to record signed net species reaction mass.
- **Gotchas:** reaction accounting is signed net mass change by species; nonlinear exponents below 1 at zero concentration produce a typed infinite-derivative rejection. 10.5.1 must include all supported sorbed pools and use these signed terms.

## Amendments and blockers

- User approved the plan on 2026-09-24. Waves 1–4 passed verification; no active blockers.

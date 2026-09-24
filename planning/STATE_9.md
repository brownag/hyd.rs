# State — Orchestration 9: Phase 4 atmospheric remediation and completion

Plan status: **approved by the user on 2026-09-24.**
Execution status: **Waves 1–3 complete; Waves 4–5 pending.**
Plan: [EXECUTION_PLAN_9.md](EXECUTION_PLAN_9.md); worker prompt: [PROMPT_TEMPLATE_9.md](PROMPT_TEMPLATE_9.md).
Diagnosis: [ATMOSPHERIC_NONCONVERGENCE_FINDINGS.md](ATMOSPHERIC_NONCONVERGENCE_FINDINGS.md).
Historical blocker: [STATE_8.md](STATE_8.md), task 8.2.1.

## Baseline checklist

- [x] Orchestration 8.1.1 water capability is complete; Phase 3 handoff exists.
- [x] Atmospheric failure diagnosis reproduced both exact failures at `dt=0.015625`, `dt_min=0.01`, after six retries; full atmospheric suite was 7 passed / 2 failed.
- [x] Findings identify active-set alternation caused by switching against accepted pond storage and list unresolved sign/balance science.
- [x] Before Wave 1: recorded `git status --short`; initial workspace run reproduced the known atmospheric 7 passed / 2 failed; initial fmt check reported drift in the Wave 1 files; initial Clippy and diff check passed. Wave 1 resolved the test and formatting failures.
- [x] User approved this plan; implementation began after the Wave 1 entry gate passed.

Scientific baseline: **unavailable.** No Fortran executable, calibrated tolerance, or reviewed oracle outputs exist.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 9.1.1 | Atmospheric active-set and balance remediation | done | committed |
| 2 | 9.2.1 | Supported lower boundary modes | done | committed |
| 3 | 9.3.1 | Feddes stress and compensation | done | committed |
| 4 | 9.4.1 | Four first-water acceptance fixtures | pending | — |
| 5 | 9.5.1 | Phase 4 evidence and Phase 5 handoff | pending | — |

## Task reports

<!-- Append each accepted worker report here, at most 15 lines per task. -->

## 9.1.1 — Atmospheric active-set and balance remediation

- Status: complete after one mid-tier escalation to remove per-step transition-trace allocation.
- Changed `water_solver.rs`, `water_state.rs`, `atmospheric.rs`, `error.rs`, `tests/atmospheric.rs`; task micro-plan in `tasks/9.1.1.todo.md`.
- Active mode and candidate ponding are trial-local; final mode must match the final solve; accepted state updates only on acceptance.
- Reports expose per-transition demand, mode, head, soil flux, and candidate surface storage using preallocated workspace capacity; insufficient capacity errors before state mutation.
- Balance exposes soil storage, surface storage, soil boundary flux, and atmospheric exchange separately; `storage_change` remains total storage.
- `cargo test -p hydrus-core --test atmospheric`: 9 passed, 0 failed.
- `cargo test --workspace`: all tests passed (71 integration tests; 0 failed).
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`: passed.
- Deviations: added explicit trace-capacity provisioning and a borrowed report trace to preserve preallocated-step behavior.
- Gotcha: callers must finish reading `WaterStepReport.atmospheric_transitions` before mutably reusing its workspace.
- Evaporation critical-head physical semantics and Fortran parity remain unresolved; no tolerances or iteration limits changed.

## 9.2.1 — Supported lower boundary modes

- Status: complete after a task-scope amendment and one mid-tier Clippy cleanup.
- Changed `lower_boundary.rs`, `water_solver.rs`, `error.rs`, `lib.rs`, `tests/lower_boundary.rs`, and existing input fixtures in allocation, atmospheric, water contract, solver, and transaction tests.
- Added explicit fixed-head, prescribed-flux, and free-drainage modes; seepage-face and water-table selections return typed unsupported errors.
- Positive lower flux is into the domain. Free-drainage exchange follows the source row sign mapping; this is algebra-derived behavior, not executable parity.
- Fixed-head mode preserves caller-supplied measured exchange; accepted reports include selected mode and signed boundary balance.
- `cargo test -p hydrus-core --test lower_boundary`: 5 passed, 0 failed.
- `cargo test --workspace`: all tests passed (76 integration tests; 0 failed).
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`: passed.
- Scope amendment added `WaterStepInput` construction call sites after the original input type had no physical lower-mode selector.
- Clippy cleanup grouped row and balance arguments in private input structs; behavior unchanged.
- No new gotchas for dependents beyond selecting an explicit `LowerBoundaryMode` at every `WaterStepInput` construction.

## 9.3.1 — Feddes root stress and compensation

- Status: complete after one mid-tier Clippy cleanup.
- Changed `root_uptake.rs`, `water_solver.rs`, `water_state.rs`, `error.rs`, `lib.rs`, `tests/root_uptake.rs`, and all six scoped existing `WaterStepInput` fixture files.
- Added optional Feddes inputs; absent configuration leaves caller sink behavior unchanged.
- Trial sink is rebuilt each Picard pass from current heads and water contents, then used consistently in boundary/interior rows, atmospheric flux, and weighted balance.
- Compensation follows the source `Omega`/`Compen` accumulation; node uptake is capped at half the water above the Feddes wilting content per timestep.
- `cargo test -p hydrus-core --test root_uptake`: 7 passed, 0 failed.
- `cargo test --workspace`: all tests passed (83 integration tests; 0 failed).
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`: passed.
- Deviations: added the optional input, workspace buffer, existing caller fixtures, and two Clippy-driven private argument/iteration cleanups after plan amendments/review.
- Root weights are per-length densities, potential uptake is positive depth/time, and sink rates are positive extraction; source algebra does not establish executable parity.

## Amendments and blockers

- 9.2.1 scope amended after the first plan review found `WaterStepInput` has no explicit physical lower-mode selector. The task now includes the existing struct-literal test call sites that must retain behavior after the new field is added. No implementation changes were made during the blocked attempt.
- 9.3.1 scope amended at plan review because trial-dependent uptake requires an optional root input and workspace-owned trial-sink storage. The task now includes `water_state.rs` and all existing `WaterStepInput` test call sites; no implementation changes were made before approval.

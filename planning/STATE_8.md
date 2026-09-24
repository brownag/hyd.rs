# State — Orchestration 8: Phase 4 first water acceptance

Plan status: **approved by the user on 2026-09-24.**
Execution status: **Wave 1 in progress; task 8.1.1 micro-plan requested.**
Master authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 4.
Phase 3 entry evidence: [PHASE_3_WATER_SOLVER_EVIDENCE.md](master/PHASE_3_WATER_SOLVER_EVIDENCE.md).
Plan: [EXECUTION_PLAN_8.md](EXECUTION_PLAN_8.md); worker prompt: [PROMPT_TEMPLATE_8.md](PROMPT_TEMPLATE_8.md).

## Baseline checklist

- [x] Phase 0 provenance process, Phase 1 hydraulic, and Phase 2 mesh/stencil evidence exist.
- [x] Phase 3 solver, public stepping API, transaction/retry, generic boundary rows, target-time clipping, balances, and corrected evidence are present per Orchestration 7.
- [x] Latest recorded workspace baseline: 59 passed, 0 failed at `26ae9bb` and `2a3b422`; formatting, warning-denied Clippy, and diff checks passed.
- [x] Before Wave 1, `git status --short` showed only the three untracked Orchestration 8 planning artifacts.
- [x] Current entry gate passed: `cargo test --workspace` (59 passed, 0 failed); `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- [x] User approved the plan; task 8.1.1 is in micro-planning and no implementation changes have begun.

Scientific baseline: **unavailable.** Fixture provenance and numerical parity limitations must remain explicit.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 8.1.1 | Validated water product configuration | done | pending local commit |
| 2 | 8.2.1 | Event-aligned atmospheric forcing and upper switching | pending | — |
| 3 | 8.3.1 | Supported lower boundary modes | pending | — |
| 4 | 8.4.1 | Feddes root stress and compensation | pending | — |
| 5 | 8.5.1 | Four first-water acceptance fixtures | pending | — |
| 6 | 8.6.1 | Phase 4 evidence and Phase 5 handoff | pending | — |

## Task reports

### 8.1.1

- Status: done; added public single-domain/model-0 configuration and typed rejection coverage.
- Files: `error.rs`, `lib.rs`, `water_solver.rs`, new `tests/water_contract.rs`, and `tasks/8.1.1.todo.md`.
- Targeted: `cargo test -p hydrus-core --test water_contract` — 3 passed, 0 failed.
- Wave gate: `cargo test --workspace` — 62 passed, 0 failed; fmt, warning-denied Clippy, and `git diff --check` passed.
- Retry: first wave gate found rustfmt differences in the new test; one correction applied, all gates then passed.
- Deviations: none. Downstream: capability boundary exists; event and physical boundary logic remains for later tasks.
- Commit: pending.

## Escalations and amendments

<!-- Record retries, escalations, scope amendments, and pre-wave blockers here. -->

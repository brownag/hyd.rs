# State — Orchestration 7: Phase 3 solver correction and closure

Plan status: **approved by the user on 2026-09-23.**
Execution status: **Wave 1 in progress; pre-wave baseline recorded.**
Planning baseline revision: `5066dad`, reviewed 2026-09-23. The worktree was clean before these three draft artifacts were created.
Master authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 3.
Resume from [EXECUTION_PLAN_7.md](EXECUTION_PLAN_7.md); worker prompt: [PROMPT_TEMPLATE_7.md](PROMPT_TEMPLATE_7.md).

## Baseline checklist

- [x] Worktree was clean during planning reconnaissance.
- [x] Phase 0 provenance, Phase 1 hydraulics, and Phase 2 mesh/stencil evidence exist.
- [x] Phase 3 implementation and evidence exist from Orchestration 5; review found the specific gaps listed in `EXECUTION_PLAN_7.md`.
- [x] Earlier review-session `cargo test --workspace` passed; the fresh pre-wave run at `5066dad` passed 46 tests and failed `fixed_size_steps_allocate_nothing_after_setup` (counter observed 1 allocation). Another workspace run counted 2, and a targeted rerun previously passed. Wave 1 includes isolating this counter.
- [x] At `5066dad`, fresh `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all passed.
- [x] Pre-wave commands completed and exact results are recorded above; the sole known baseline failure is the allocation assertion assigned to 7.1.3.

Scientific baseline: **unavailable.** This orchestration must not claim Fortran parity, calibrated tolerance, release compatibility, or performance.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 7.1.1 | Private validated van Genuchten parameters | pending | — |
| 1 | 7.1.2 | Unit-tagged tables, correct derivatives, no failing constructors | pending | — |
| 1 | 7.1.3 | Isolated accepted-step allocation test binary | pending | — |
| 2 | 7.2.1 | Trial-head, per-material hydraulic Picard evaluation | pending | — |
| 3 | 7.3.1 | Weighted balances and shared row assembly | pending | — |
| 4 | 7.4.1 | Caller target-time alignment | pending | — |
| 5 | 7.5.1 | Stable allocation evidence across retries and accepted steps | pending | — |
| 6 | 7.6.1 | Corrected Phase 3 evidence and Phase 4 gate | pending | — |

## Wave verification

| Wave | Revision | Targeted tests | Workspace tests | Fmt | Clippy | Diff check |
|---|---|---|---|---|---|---|
| 1 | — | pending | pending | pending | pending | pending |
| 2 | — | pending | pending | pending | pending | pending |
| 3 | — | pending | pending | pending | pending | pending |
| 4 | — | pending | pending | pending | pending | pending |
| 5 | — | pending | pending | pending | pending | pending |
| 6 | — | pending | pending | pending | pending | pending |

## Task reports

<!-- Orchestrator appends concise worker reports here. -->

## Escalations and amendments

<!-- Record retries, escalations, scope amendments, and pre-wave blockers here. -->

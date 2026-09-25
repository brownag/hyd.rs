# State — Orchestration 12: M5 scientific-comparison catch-up

Plan status: **drafted; execution has not started.**
Plan: [EXECUTION_PLAN_12.md](EXECUTION_PLAN_12.md); worker prompt: [PROMPT_TEMPLATE_12.md](PROMPT_TEMPLATE_12.md).

## Baseline checklist

- [x] Orchestration 11 complete; M5 implementation/contract evidence is recorded.
- [x] Working tree was clean after Orchestration 11 commit `73712ef` before planning this catch-up.
- [x] Latest recorded baseline: `cargo test --workspace` — 112 passed, 0 failed; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- [ ] Re-run the four gates serially before Wave 1; they were not run during plan drafting.
- [x] Scientific evidence remains unavailable: no approved M5 fixture, paired solute oracle output, or calibrated per-field tolerance.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 12.1.1 | Complete, provenance-labeled M5 comparison fixture | planned | — |
| 2 | 12.2.1 | Isolated Fortran solute run and verified output manifest | planned | — |
| 3 | 12.3.1 | Normalized observables and paired Rust/reference comparison | planned | — |
| 4 | 12.4.1 | Reviewed tolerance decision and M5 evidence handoff | planned | — |

## Task reports

<!-- Append each accepted worker report here, at most 15 lines per task. -->

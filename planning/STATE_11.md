# State — Orchestration 11: M5 solute review follow-up

Plan status: **drafted; implementation not started.**
Plan: [EXECUTION_PLAN_11.md](EXECUTION_PLAN_11.md); worker prompt: [PROMPT_TEMPLATE_11.md](PROMPT_TEMPLATE_11.md).

## Baseline checklist

- [x] Orchestration 10 complete; M5 evidence documents the review limitation that storage closure under changing water content lacks prior theta.
- [x] Working tree clean before Orchestration 11 plan drafting.
- [x] Latest recorded baseline from `STATE_10.md`: `cargo test --workspace` — 110 passed, 0 failed; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- [ ] Re-run the four gates serially before Wave 1; they were not run during plan drafting.
- [x] Scientific validation remains unavailable; no canonical solute deck, calibrated tolerance, reviewed Fortran output, or parity evidence.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 11.1.1 | Correct storage accounting across changing water content | planned | — |
| 2 | 11.2.1 | Shared transport and boundary assembly | planned | — |

## Task reports

<!-- Append each accepted worker report here, at most 15 lines per task. -->

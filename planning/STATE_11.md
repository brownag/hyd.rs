# State — Orchestration 11: M5 solute review follow-up

Plan status: **approved; execution in progress.**
Plan: [EXECUTION_PLAN_11.md](EXECUTION_PLAN_11.md); worker prompt: [PROMPT_TEMPLATE_11.md](PROMPT_TEMPLATE_11.md).

## Baseline checklist

- [x] Orchestration 10 complete; M5 evidence documents the review limitation that storage closure under changing water content lacks prior theta.
- [x] Working tree clean before Orchestration 11 plan drafting.
- [x] Latest recorded baseline from `STATE_10.md`: `cargo test --workspace` — 110 passed, 0 failed; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- [x] Before Wave 1, `cargo test --workspace` passed (110 tests, 0 failures); `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed serially.
- [x] Scientific validation remains unavailable; no canonical solute deck, calibrated tolerance, reviewed Fortran output, or parity evidence.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 11.1.1 | Correct storage accounting across changing water content | done | this commit |
| 2 | 11.2.1 | Shared transport and boundary assembly | planned | — |

## Task reports

<!-- Append each accepted worker report here, at most 15 lines per task. -->

## 11.1.1

- **Status:** done; start theta is validated before mutation and storage uses interval endpoint contents.
- **Changed:** `solute.rs` (602–605, 689–700, 748–810, 890–912, 946–960); acceptance tests (50–78, 139–164, 298–327); transport/reaction constructors; M5 evidence; task TODO.
- **Tests:** `cargo test -p hydrus-core --test solute_acceptance` — 6 passed, 0 failed; `cargo test -p hydrus-core --test solute_transport` — 5 passed, 0 failed; `cargo test -p hydrus-core --test solute_reactions` — 6 passed, 0 failed.
- **Gates:** `cargo test --workspace` — 112 passed, 0 failed; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` passed.
- **Deviations:** none. Start-content dimension, NaN, and zero-value rejection preserve accepted-state identity; changing-theta fixture checks storage change and residual. Initial fmt check required one formatting correction.
- **Dependent-task note:** all `SoluteTransportInput` constructors in the three scoped targets now provide `start_water_contents`.

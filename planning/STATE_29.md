# Orchestration 29 state — G22.2 closed prescribed-flux pair with vapor on

Created: 2026-11-14. Status: **Plan approved 2026-11-14. Wave 0 (baseline) in progress.**
Plan: [EXECUTION_PLAN_29.md](EXECUTION_PLAN_29.md). Goal: [GOALS.md](GOALS.md) G22.2 (Circled).

## Baseline (wave 0)

| Suite | Command | Result |
|---|---|---|
| Build | `cargo build --workspace --locked` | pass |
| Rust | `cargo test --workspace --no-fail-fast` | pass: 620 passed, 0 failed, 4 ignored |
| Format | `cargo fmt --all -- --check` | pass |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| Python | `python3 -m unittest discover -s tools -p 'test_*.py'` | pass: 479 OK |

## Gates

- [x] Wave 0 baseline green (2026-11-14, HEAD d4b44f4).
- [ ] GATE 29.1 (user): fixture design, deck choice, guard shape. Decision: pending.
- [ ] GATE 29.2 (user): fixture and balance evidence against the exit criterion. Decision: pending.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 0 | — | Commit plan; baseline | done |
| 1 | 29.1.1 | Fortran closed-pair deck and fixture design | pending |
| 1 | 29.1.2 | Rust design for vapor-aware flux rows and typed guard | pending |
| — | GATE 29.1 | Design approval | pending |
| 2 | 29.2.1 | Vapor-aware flux row builders | pending |
| 2 | 29.2.2 | Lift rejection for closed pair only | pending |
| 3 | 29.3.1 | Column balance check | pending |
| 3 | 29.3.2 | Deck and fixture generation and comparison | pending |
| — | GATE 29.2 | Exit criterion decision | pending |

## Wave gates

_Filled in as each wave completes._

## Task reports

_Appended by the orchestrator from each worker's `planning/tasks/<id>.report.md` (plan decision D9)._

## Discovered

_Empty at planning. Unlisted work goes here and waits for approval._

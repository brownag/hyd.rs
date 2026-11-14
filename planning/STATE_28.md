# Orchestration 28 state — G22.2 water-flux and free-drainage audit (with vapor)

Created: 2026-11-13. Status: **Plan approved 2026-11-13. Not yet executed (wave 0 pending).**
Plan: [EXECUTION_PLAN_28.md](EXECUTION_PLAN_28.md).
Worker: [PROMPT_TEMPLATE_28.md](PROMPT_TEMPLATE_28.md).
Goal: [GOALS.md](GOALS.md) Circled goal G22.2. Coverage: [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md) row 22.

## Baseline and pre-wave-1 checklist

- [ ] Working tree clean before wave 1 (`git status --short`).
- [ ] Repeat the five suites at the planning HEAD:

| Suite | Command | Result |
|---|---|---|
| Build | `cargo build --workspace --locked` | pending |
| Rust | `cargo test --workspace --no-fail-fast` | pending (expected 620 passed, 0 failed, 4 ignored) |
| Format | `cargo fmt --all -- --check` | pending |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pending |
| Python | `python3 -m unittest discover -s tools -p 'test_*.py'` | pending (expected 479 OK) |

- [ ] `planning/tasks/` present for worker micro-TODOs.
- [x] User approved this plan (2026-11-13).

## Gates

- [ ] GATE W1 (user): disposition per water form after wave 1. Decision: pending.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 1 | 28.1.1 | Free drainage with vapor on: source and coupling audit | pending |
| 1 | 28.1.2 | Prescribed water flux with vapor on: audit | pending |
| 1 | 28.1.3 | Seepage face and water table with vapor on: audit | pending |
| 1 | 28.1.4 | G7 free-drainage end-row assessment | pending |
| — | GATE W1 | Disposition per form | pending |
| 2 | 28.2.1 | Coverage matrix, GOALS and G22.2 handoff | pending |

Max two workers in parallel. The orchestrator owns shared state and the serial gate at each wave end, one commit at a time.

## Wave gates

_Filled in as each wave completes (commits, serial gate results, deviations)._

## Task reports

_Appended by workers under `## <task id>` as tasks complete._

## Discovered

_Empty at planning. Any unlisted work goes here and waits for approval._

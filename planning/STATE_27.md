# Orchestration 27 state — G22.4 remaining G22 rows (nonuniform T, ConVh, sink/root)

Created: 2026-11-13. Status: **Plan approved 2026-11-13. Pre-wave-1 baseline green (2026-11-13). Wave 1 starting.**
Plan: [EXECUTION_PLAN_27.md](EXECUTION_PLAN_27.md).
Worker: [PROMPT_TEMPLATE_27.md](PROMPT_TEMPLATE_27.md).
Goal: [GOALS.md](GOALS.md) sequenced goal G22.4. Coverage: [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md).

## Baseline and pre-wave-1 checklist

- [ ] Working tree clean before wave 1 (`git status --short`). Tree was clean at `603b4e4` when planned.
- [x] Repeat the five Quick Start suites at `603b4e4`:

| Suite | Command | Result |
|---|---|---|
| Build | `cargo build --workspace --locked` | PASS (exit 0) |
| Rust | `cargo test --workspace --no-fail-fast` | PASS: 607 passed, 0 failed, 3 ignored |
| Format | `cargo fmt --all -- --check` | PASS (exit 0) |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | PASS (exit 0) |
| Python | `python3 -m unittest discover -s tools -p 'test_*.py'` | PASS: 459 tests OK |

- [ ] Reference executable present: `.development/26.2.2/build/hydrus-reference`, SHA-256 begins `e6d69dd3` (checked at planning, 2026-11-13). Re-check before 27.2.2.
- [ ] `planning/tasks/` directory present for worker micro-TODOs.
- [x] User approved this plan (Phase 6). Approval recorded 2026-11-13.

## Gates

- [ ] GATE S (user): disposition of sink and root uptake after 27.1.1. Decision: pending.
- [ ] GATE N2 (user, conditional): bounds for any new case not covered by criteria v3. Needed: pending (decided at 27.2.2).

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 1 | 27.1.1 | Sink and root-uptake source/coupling audit | pending |
| 1 | 27.1.2 | ConVh scalar helper location and reachability audit | pending |
| 1 | 27.1.3 | Nonuniform-temperature coverage audit | pending |
| — | GATE S | Sink and root disposition | pending |
| 2 | 27.2.1 | Nonuniform case specs and producer | pending |
| 2 | 27.2.2 | Reference-only fixture run | pending |
| — | GATE N2 | Conditional bounds | pending |
| 3 | 27.3.1 | ConVh helper removed or guarded | pending |
| 3 | 27.3.2 | Nonuniform-T Rust parity test | pending |
| 4 | 27.4.1 | Python comparator for nonuniform cases | pending |
| 4 | 27.4.2 | Independent balance checks for nonuniform cases | pending |
| 5 | 27.5.1 | Acceptance, coverage and goal handoff | pending |

Max two workers. Separate worktrees with absolute target and run roots. The orchestrator owns shared state and serial full-suite gates, one task commit at a time.

## Wave gates

_Filled in as each wave completes (commits, serial gate results, deviations)._

## Task reports

_Appended by workers under `## <task id>` as tasks complete._

## Discovered

_Empty at planning. Any unlisted work goes here and waits for approval._

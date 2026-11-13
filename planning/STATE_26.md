# Orchestration 26 state — G22.1 heat top/bottom boundary rows with vapor

Created: 2026-11-12. Status: **PLAN DRAFT — awaiting user approval. Nothing executed.**
Plan: [EXECUTION_PLAN_26.md](EXECUTION_PLAN_26.md).
Worker: [PROMPT_TEMPLATE_26.md](PROMPT_TEMPLATE_26.md).
Goal: [GOALS.md](GOALS.md) circled goal G22.1. Coverage: [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md).

## Baseline and pre-wave-1 checklist

- [ ] Working tree clean before wave 1 (`git status --short`). Tree was clean at `237642f` when planned.
- [ ] Repeat the five Quick Start suites on `237642f` (orchestration 25 baseline was on `a8fc200`):

| Suite | Command | Result |
|---|---|---|
| Build | `cargo build --workspace --locked` | pass (exit 0) |
| Rust | `cargo test --workspace --no-fail-fast` | pass: 584 passed, 0 failed, 2 ignored (matches STATE_25) |
| Format | `cargo fmt --all -- --check` | pass (exit 0) |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pass (exit 0) |
| Python | `python3 -m unittest discover -s tools -p 'test_*.py'` | pass: 447 tests, OK |

- [ ] Reference executable located by SHA-256 `824179ab…` under `.development/25.2.2/`, or rebuild recorded (decision D8). Path: pending.
- [ ] `planning/tasks/` directory present for worker micro-TODOs.
- [ ] User approved this plan (Phase 6). Approval recorded: pending.

## Numeric approval gate (GATE N1)

- [ ] Wave 2 reference-only evidence complete (26.2.2).
- [ ] Orchestrator proposes criteria v3: per-field tolerances, print rules, zero floor, discrimination (vapor on/off) per new form.
- [ ] User approves the numbers before any candidate comparison (decision D6).
- [ ] Criteria v3 written to `reference/vapor_fortran/criteria.json` and frozen with a contract revision. Hash: pending.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 1 | 26.1.1 | Heat-boundary code audit (source and Rust) | done (audit committed) |
| 2 | 26.2.1 | Vapor heat-boundary case specs and producer | pending |
| 2 | 26.2.2 | Reference-only fixture run | pending |
| — | GATE N1 | User approves numeric criteria v3 | pending |
| 3 | 26.3.1 | Vapor config accepts audited forms, rejects others | pending |
| 3 | 26.3.2 | TempAdj and top/bottom vapor heat terms | pending |
| 4 | 26.4.1 | Rust candidate runs and parity test | pending |
| 4 | 26.4.2 | Python comparator against criteria v3 | pending |
| 5 | 26.5.1 | Independent balance checks for accepted forms | pending |
| 6 | 26.6.1 | Acceptance, coverage and goal handoff | pending |

Max two workers. Separate worktrees with absolute target and run roots. The
orchestrator owns shared state and serial full-suite gates, one task commit at a time.

## Wave gates

_Filled in as each wave completes (commits, serial gate results, deviations)._

## Task reports

## 26.1.1
- Status: done. Audit: `planning/master/M6_VAPOR_HEAT_BOUNDARY_AUDIT.md`. Micro-TODO: `planning/tasks/26.1.1.todo.md`.
- Files changed: the audit file only (created). No code, test or fixture changed.
- Tests: none run (doc task). Orchestrator spot-checked TEMPER.FOR:249-255, :220, INPUT.FOR:1236, config.rs:162-172. All match.
- Deviations: worker model was Sonnet for this task, not Haiku as chosen by the user. Later workers use Haiku.
- Decisions taken: Q1 yes (kTopT=-1 only top code proposed for acceptance). Q2 lEnBal out of scope for vapor.
- Findings: H1 confirmed with nuance (kTopT < -1 omits the TEMPER.FOR:255 term; reject-with-trigger). H2, H3, H4 confirmed. Candidates: DF, DZ, FD, FF, FZ; kBotT==0 and kBotT<0 are candidates; kTopT<-1 is reject-with-trigger.
- Gotchas for 26.2.1: the candidate set includes kBotT==0 (DZ, FZ) and kBotT<0 variants.

## Open execution blockers / discoveries

- Provisional accepted-form set in 26.3.1 depends on the Wave 1 audit and the Wave 2 evidence; confirm at Phase 6.
- Hazard H1: if a negative `kTopT` code other than `-1` is reachable, Rust's all-negative-is-flux mapping is unfaithful for vapor. Must be settled in 26.1.1.

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

## 27.1.2 — ConVh helper location and reachability audit (done, read-only)

- Status: done. Disposition: **reachable, output-neutral**. Audit: `planning/master/M6_VAPOR_CONVH_AUDIT.md`.
- Rust helper `con_vh` at `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs:196-217` (hardcoded 20 °C at `:202`). Sole caller `:281` inside `gen_mat`, result bound to `_conv` and discarded.
- `gen_mat` is reached unconditionally from `run_compat_deck` (`compat_runner.rs:112`, exported at `lib.rs:34`). Not gated on vapor.
- Fortran anchor correction: `WATFLOW.FOR:968-995` is the scalar `ConVh`; array routine is `ConVapor` at `:849-929`. Only scalar caller is `INPUT.FOR:804`, writing to unit 50 only.
- Vapor path computes its own inline value (`kernels.rs:52-53`), not via the helper.
- Gotcha for 27.3.1: remove-or-guard is a user decision (D4 assumed "unreachable"). Candidate file set `water_compat.rs` needs approval before any edit.
- No code changed; no commit.

## 27.1.1 -- Sink and root-uptake source/coupling audit (done, read-only)

- Audit: `planning/master/M6_VAPOR_SINK_ROOT_AUDIT.md` (untracked). No code, fixture, or run. Not committed.
- (a) Prescribed sink, SinkF on, no root: candidate for bounded case at Gate S, conditional on confirming the Beta source when lRoot=0 (Open item 1).
- (b) Root uptake (TPot=rRoot via SetSnk, Beta via SetRG): reject; trigger = root uptake requested with vapor enabled (new gate).
- (c) Heat sink terms (TEMPER.FOR SetUpHeat) are active; TempAdj vapor terms do not take Sink. Reject; trigger = sink-bearing vapor case requested with a closed energy balance.
- (d) Water step with vapor: Sink enters liquid rows (WATFLOW.FOR:221,254,285) and liquid boundary v (Veloc:717); ConVapor/VaporContent take no Sink. Reject; trigger = Gate S picks (ii) plus an approved sink+vapor water balance criterion.
- Guards at vapor/mod.rs:180 and :185 are unchanged. driver.rs passes input.sink into heat passes, so the guard is the only barrier.
- Extra read-only sources: SINK.FOR, TIME.FOR, WATFLOW.FOR (per orchestrator answers).
- Git note: planning/STATE_27.md is staged by another task; this append is unstaged and not committed.

## 27.1.1 — Sink and root-uptake source/coupling audit (done, read-only)

- Audit: `planning/master/M6_VAPOR_SINK_ROOT_AUDIT.md` (untracked; not committed yet).
- Dispositions (proposed, then decided at Gate S): (a) prescribed sink, no root: candidate for a bounded case, conditional on confirming the `Beta` source when `lRoot=0`. (b) root uptake: reject. (c) heat sink coupling: reject. (d) water step under sink: reject. Each has a trigger in the audit.
- Guards `vapor/mod.rs:180` and `:185` unchanged. `vapor/driver.rs` passes `input.sink` into both heat passes, so the guard is the only barrier.
- Open items: (1) `Beta` source when `lRoot=0`; (2) driver/guard coupling; (3) `balance.rs` sink accounting read for citation only.
- Note: STATE_27.md now has unstaged changes from this append; commit with the planning batch.

## GATE S — decided by user (2026-11-13)

- Decision: **reject all rows (a)–(d) with triggers**. No implementation in this orchestration; orchestration 28 not opened.
- Row (a) keeps its trigger text (a sink-only, vapor-on case with `SinkF=1, lRoot=0`, after the `Beta` source is confirmed and an explicit balance criterion is approved).
- Rows (b)–(d) keep their triggers as written in the sink/root audit.
- 27.5.1 handoff: record these as rejected-with-trigger in `M6_VAPOR_COVERAGE.md`.

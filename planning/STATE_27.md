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

- [x] GATE S (user): decided 2026-11-13: reject all rows (a)–(d) with triggers.
- [ ] GATE N2 (user, conditional): bounds for any new case not covered by criteria v3. Needed: pending (decided at 27.2.2).

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 1 | 27.1.1 | Sink and root-uptake source/coupling audit | done |
| 1 | 27.1.2 | ConVh scalar helper location and reachability audit | done |
| 1 | 27.1.3 | Nonuniform-temperature coverage audit | done |
| — | GATE S | Sink and root disposition | done: reject all with triggers |
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

- Wave 1 (2026-11-13), serial gate after 27.1.3: Rust 607 passed, 0 failed, 3 ignored; fmt OK; clippy OK; Python 459 OK. Commits: a850e69 (baseline), ddc8ef6 (27.1.1, 27.1.2 audits), 33c786d (27.1.3 audit).

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

## 27.1.3 -- Nonuniform-temperature coverage audit (done, read-only)

- Audit: `planning/master/M6_VAPOR_NONUNIFORM_T_AUDIT.md` (untracked). Micro-TODO: `planning/tasks/27.1.3.todo.md`. No fixture, producer edit, executable run, or commit.
- Existing thermal cases: all iCampbell=0 and fixed-head both ends. Wet cases are uniform initial T (15 C or 25 C). Dry cases (`vapor_dry_thermal_gradient`, 10 `vapor_heatbc_*`) use a linear 45 to 5 C ramp.
- Gap (M6 row 12): no non-linear initial T profile in any case, and no wet case with a nonuniform initial profile.
- Proposed pair (orchestrator answers applied): `vapor_nonuniform_step_on` / `vapor_nonuniform_step_off`, dry h=-1e4 cm, step initial profile (nodes 1-20 at 45 C, 21-41 at 5 C), boundary 45/5 Dirichlet, fixed head, iCampbell=0.
- Producer schema: expressible with no new field. Needs the `expected` set at `tools/run_vapor_reference.py:20-21` extended in 27.2.1, and the same LEVEL_01.DIR content as the dry case.
- Fortran: `TempIn` at `INPUT.FOR:1205-1245`; `iCampbell` read at `:1223`; boundary record at `:1236`.
- Resolved: kBotT=0 is the zero-gradient branch (`TEMPER.FOR:109-111`), so the "z" heatbc codes mean zero flux and tB is unused.
- Risk: `vapor_thermal_gradient`, `vapor_off_control`, `vapor_isothermal_head_gradient` list LEVEL_01.DIR in fixture inputs but have no such file on disk or in git history; `run_case` copies it unconditionally (`:123`). Do not reuse those directories as templates.
- Git: planning/STATE_27.md was already staged by another task; this append is unstaged and not committed.

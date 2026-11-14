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
| Build | `cargo build --workspace --locked` | pass (2026-11-13, HEAD 3d222f9) |
| Rust | `cargo test --workspace --no-fail-fast` | pass: 620 passed, 0 failed, 4 ignored |
| Format | `cargo fmt --all -- --check` | pass |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| Python | `python3 -m unittest discover -s tools -p 'test_*.py'` | pass: 479 tests, OK |

- [x] Working tree clean before wave 1 (`git status --short` empty at 3d222f9).
- [x] Baseline green at planning HEAD (table above).
- [x] `planning/tasks/` present for worker micro-TODOs.
- [x] User approved this plan (2026-11-13).

## Gates

- [x] GATE W1 (user): disposition per water form after wave 1. Decision recorded 2026-11-13 (see below).

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 1 | 28.1.1 | Free drainage with vapor on: source and coupling audit | done |
| 1 | 28.1.2 | Prescribed water flux with vapor on: audit | done |
| 1 | 28.1.3 | Seepage face and water table with vapor on: audit | done |
| 1 | 28.1.4 | G7 free-drainage end-row assessment | done |
| — | GATE W1 | Disposition per form | pending |
| 2 | 28.2.1 | Coverage matrix, GOALS and G22.2 handoff | done |

Max two workers in parallel. The orchestrator owns shared state and the serial gate at each wave end, one commit at a time.

## Wave gates

### Wave 1 serial gate (2026-11-13)

- Build, Rust (620 passed, 0 failed, 4 ignored), fmt, clippy, Python (479 OK): all pass.
- Changed paths: only the wave's new audit and task docs, plus `planning/STATE_28.md` (orchestrator edits). No code, fixture, criteria or pinned source changed.
- Micro-plans approved with scope limits: no reads of `driver.rs`, `vapor/balance.rs`, `vapor/water.rs`, `FqDrain` or drain parameters; these are recorded as "not read in scope". 28.1.3 approved with its two stated defaults.
- Deviations: none that change a disposition. 28.1.1's report has a placeholder acceptance line (its results are in this gate). 28.1.4 corrected several line numbers in its own TODO notes; the assessment carries the corrected ones.

### GATE W1 summary (input for the user decision)

| Form / pair | Source evidence | Vapor term affected | Disposition | Named trigger | Balance check (named, not run) |
|---|---|---|---|---|---|
| Free drainage, lower (`FreeD`, `KodBot=-5`) | `WATFLOW.FOR:202-213, 221-222`; `INPUT.FOR:56-61, 112` | Yes: `ConVh` added to `ConB` before `rBot` (`:211,213`); `ConTB`/`ThV` storage in `PB` (`:224-229`) | Candidate for a bounded case | Accept only with an explicit bottom-row design matching `:211-229`; otherwise `vapor/mod.rs:217` stays | Bottom exchange equals vapor-augmented drainage flux; vapor storage in stored-water change |
| Upper prescribed flux (`KodTop=-1`) | `INPUT.FOR:51-52, 63-69`; `WATFLOW.FOR:276, 287-294, 327-334` | Yes: `ConVh` in `ConA`; `ConTA` and `ThV` in `PT` | Candidate for a bounded case | No vapor-aware upper flux row source in scope | Liquid plus vapor column storage change equals `rTop`·dt plus lower exchange |
| Lower prescribed flux (`KodBot=-1`) | `INPUT.FOR:56-61, 63-69`; `WATFLOW.FOR:211, 221-228, 315-322` | Yes: `ConVh` in `ConB`; `ConTB`/`ThV` in `PB` | Candidate for a bounded case | Lower flux row (`lower_boundary.rs:43-86`) is water-only | Same column balance with `rBot` as lower exchange |
| Lower `qGWLF` (`KodBot=-7`) | `INPUT.FOR:77-81, 109`; `WATFLOW.FOR:217, 572-574` | Yes, plus `rBot` depends on `hNew(1)` | Reject with trigger | No Rust mode; needs new mode and per-iteration `Fqh` update | Exchange equals `Fqh(h-GWL0L)` at accepted head |
| Seepage face (`SeepF`, `KodBot=-2`) | `WATFLOW.FOR:360-400`, bottom at `:379-387` | Shared bottom row only (`:211`); none in the `vBot` switch | Reject with trigger | Nonlinear head switch (zero flux vs `hSeep`); needs variable-head boundary support | Outflow accumulation against liquid plus vapor storage per step |
| Water table (`qDrain`) | `INPUT.FOR:56, 60`; `WATFLOW.FOR:218-221`; `FqDrain` not read in scope | Shared bottom row (`:211`); none in the drain call | Reject with trigger | Drain flux depends on `hNew(1)`; needs water-only drain support | Drain outflow against liquid plus vapor storage change, flux at converged `hNew(1)` |
| Atmospheric (`KodTop=-4`, `AtmBC`) | `INPUT.FOR:107-110` | n/a | G22.3, not decided here | n/a | n/a |
| Pair: flux upper with fixed-head lower | `WATFLOW.FOR:294, 315-322` | Follows the upper-flux row | Follows the two singles | Upper flux needs vapor-aware row | Upper flux plus fixed-head lower exchange |
| Pair: flux upper with flux lower | `WATFLOW.FOR:221-222, 294, 315-334` | Follows the two singles | Follows the two singles; closed pair is the cleanest bounded case | As singles | Net imposed flux equals storage change |
| Pair: flux upper with `qGWLF` | `WATFLOW.FOR:217` | Yes | Reject (`qGWLF` rejected) | As `qGWLF` | As `qGWLF` |
| Pair: flux upper with free drainage | `WATFLOW.FOR:211, 213, 294` | Yes; same `ConB` point | Conditional on free-drainage verdict | Reconcile at GATE W1 | As free drainage |
| Pair: head upper with free drainage | `vapor/mod.rs:219-225`; `WATFLOW.FOR:213` | Lower end only | Conditional on free-drainage verdict | Reconcile at GATE W1 | As free drainage |

**G7 end-row verdict (input, 28.1.4): not established.** `WATFLOW.FOR:202-213` builds `ConB` from liquid conductance for the gravity term `B` (`:205`), then adds vapor conductance before `rBot` (`:211,213`). Vapor has no matching term in `B`, so the source is internally inconsistent with vapor on. Intent cannot be shown without a run, so no defect is claimed. Upgrade path: SD-007 with a tagged pinning test, out of scope here.

**Decision requested at GATE W1 (user).** Per form and pair: (i) reject with trigger, (ii) accept for orchestration 29 (implementation, fixture and balance check), or (iii) other. Also decide G7: accept "not established" as recorded, or direct a defect investigation.

### GATE W1 decision (user, 2026-11-13)

- Free drainage, lower (`FreeD`): **accepted for orchestration 29** (implementation, fixture, balance check; bottom-row design must match `WATFLOW.FOR:211-229`).
- Prescribed flux, upper (`KodTop=-1`) and lower (`KodBot=-1`): **both accepted for orchestration 29**; the closed flux-flux pair is the first candidate.
- Seepage face (`SeepF`) and water table (`qDrain`): **rejected, triggers kept** as recorded above.
- `qGWLF` (`KodBot=-7`), `TopInF`/`BotInF` (±3), atmospheric (G22.3): not in the answers. Recorded as the audit's disposition (`qGWLF` reject with trigger; ±3 reject with trigger; atmospheric G22.3, not decided here). Flag for the user to confirm at wave 2 close.
- Pairs: follow the singles. Free-drainage pairs are accepted with free drainage; flux-with-`qGWLF` rejected.
- G7: **recorded as not established.** No SD-007 entry now.

G22.2 stays Circled: at least one form is accepted for orchestration 29.



## Task reports

_Appended by the orchestrator from each worker's `planning/tasks/<id>.report.md` (plan decision D9)._

### 28.1.1

# 28.1.1 report
Status: DONE (read-only audit written; no code, fixture or run).
Files created: planning/master/M6_VAPOR_FREE_DRAINAGE_AUDIT.md, planning/tasks/28.1.1.todo.md, planning/tasks/28.1.1.report.md.
Acceptance: see results appended below by the final check.
Disposition: free drainage + vapor = candidate for a bounded case at GATE W1. Trigger: user selects it for orchestration 29 and approves a bottom-row design matching WATFLOW.FOR:211-229; else stays rejected (vapor/mod.rs:217). Balance check: bottom exchange equals vapor-augmented drainage flux, vapor storage in stored-water change; existing lower-boundary balance support not read in scope.
Key finding: WATFLOW.FOR:211 adds ConVh to ConB before :213 rBot, so vapor enters the free-drainage flux and row; Rust lower_boundary.rs:161-182 uses liquid K only.
Deviations: none (per coordinator answers, driver.rs/balance.rs/water.rs claims excluded; water_solver.rs cited by grep only).
Gotchas: whether the vapor trial overwrites the lower row (water_solver.rs) and the lower-boundary balance are "not read in scope"; 28.1.4 must reconcile on WATFLOW.FOR:213.

### 28.1.2

# 28.1.2 report
Status: DONE. Audit: planning/master/M6_VAPOR_WATER_FLUX_AUDIT.md (only new files: it, 28.1.2.todo.md, this report).
Acceptance: `grep -c "^|"` = 27 (>=6); `grep -c "INPUT.FOR\|WATFLOW.FOR"` = 28 (>=4); `grep -c "ADR 0021"` = 4 (>=1); git status shows only new files; `git diff --name-only` lists only the pre-existing `planning/STATE_28.md` modification (not touched by this task).
Dispositions (trigger; balance check named, not run):
- Upper flux KodTop=-1: candidate; trigger = no vapor-aware upper flux row in scope; check = liquid+vapor column storage vs rTop*dt + lower exchange.
- Lower flux KodBot=-1: candidate; trigger = lower_flux_row water-only (lower_boundary.rs:43-86); same column check with rBot.
- qGWLF (-7): reject; trigger = no Rust mode, head-dependent Fqh.
- TopInF/BotInF (+-3): reject; time-variable input not audited.
- FreeD, SeepF/qDrain: cross-ref 28.1.1 / 28.1.3. Atmospheric: G22.3, undecided.
Pairs: flux x FixedHead and flux x flux follow the singles (no new interaction); flux x qGWLF reject; flux x FreeD and head x FreeD conditional on 28.1.1, reconcile at GATE W1.
Deviations: none. Per coordinator, vapor/water.rs and vapor/driver.rs are not cited (marked not read in scope).
Gotchas: Fortran FreeD rBot uses vapor-augmented ConB (WATFLOW.FOR:211,213); Rust free drainage uses liquid K only (lower_boundary.rs:161-178). Relevant to 28.1.1.

### 28.1.3

# 28.1.3 report
Status: DONE (audit-only; no commit).
Files: created planning/master/M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md, planning/tasks/28.1.3.todo.md, planning/tasks/28.1.3.report.md. Nothing else touched.
Acceptance: grep -c "Seepage|Water table" = 7 (>=2); grep -c "WATFLOW.FOR|INPUT.FOR" = 7 (>=4); grep -c "ADR 0021" = 2 (>=1).
git status --short (my paths): the audit and todo are untracked (?? ); the report was written after this check.
git diff --name-only: planning/STATE_28.md only, modified before this task; none of my files appear.
Disposition, seepage face: reject. Trigger: nonlinear head condition (KodBot switch on vBot sign and hNew(1)>=hSeep, WATFLOW.FOR:379-387) in a vapor case. Balance check: cumulative bottom outflow vs liquid+vapor storage change per step across switch events, plus hNew(1)<=hSeep when active.
Disposition, water table (qDrain): reject. Trigger: drain flux rBot(hNew(1)) (WATFLOW.FOR:218-221) with vapor on; needs water-only support first. Balance check: drain accumulation vs liquid+vapor storage change at the converged hNew(1).
Both are compliant with ADR 0021 (typed rejection, D7/D9); Rust rejects both in the water-only path too (lower_boundary.rs:108-116).
Deviations: none.
Gotchas: FqDrain body, zBotDr/BaseGW/rSpacing, KodBot consumption and Rust solver hooks were "not read in scope". Neither vBot nor the drain call has a vapor term, but the shared bottom row adds ConVh at WATFLOW.FOR:211. The spec's "git diff empty" check cannot hold while STATE_28.md is modified.

### 28.1.4

# 28.1.4 report
- Status: done. Read-only; no commit.
- Files created: planning/master/G7_FREE_DRAINAGE_END_ROW_ASSESSMENT.md, planning/tasks/28.1.4.todo.md, planning/tasks/28.1.4.report.md.
- Acceptance 1: `grep -Ec '^Verdict: ...'` on the assessment = 1.
- Acceptance 2: `git status --short` for the scoped paths shows only new (untracked) files. `git diff --name-only` is NOT empty: it lists planning/STATE_28.md, modified before this task (not touched by me).
- Verdict: not established. WATFLOW.FOR:211-213 adds vapor ConVh to ConB before rBot=-ConB*Grav*fRE, while B (:205) is liquid only; vapor has no Grav term at :405-407, :736, :770. Top-row lDensity (:279) also uses post-vapor ConA, so the source is inconsistent, intent unprovable without a run.
- Deviations: line refs in the todo/plan were re-checked and corrected in the assessment (bottom ConB :202-203, B :205, interior B :239, vapor :246-249, top B :276, vapor :277, lDensity :278-280). Todo notes still carry the earlier numbers.
- Gotchas: Rust conductivity_a/b vapor content and heat/vapor/mod.rs:213-217 not read in scope. Vapor-off free drainage is unaffected. If a defect is later claimed: SD-007 plus pinning test source_defect_sd007_*.


### Wave 2 serial gate (2026-11-13)

- Build, Rust (620 passed, 0 failed, 4 ignored), fmt, clippy, Python (479 OK): all pass.
- Changed paths: `planning/master/M6_VAPOR_COVERAGE.md`, `planning/GOALS.md`, new `planning/master/M6_VAPOR_G22_2_ACCEPTANCE.md`, and the 28.2.1 task files. `git diff --check` clean. No code, fixture, criteria, ADR or SD-register change.
- G22.2 stays Circled, with a Result line in GOALS.
- Open items for the user at wave 2 close: `qGWLF`, `TopInF`/`BotInF` (±3) rejections, and the flux/`qGWLF` pair were not explicitly answered at GATE W1. Atmospheric is G22.3.

### 28.2.1

# 28.2.1 report
Status: DONE (docs only; no commit).
Files: planning/master/M6_VAPOR_COVERAGE.md (line 3; row 22 replaced by lines 22-32, parent plus 10 sub-rows); planning/GOALS.md (new Result line after G22.2 Depends, line 13); created planning/master/M6_VAPOR_G22_2_ACCEPTANCE.md, planning/tasks/28.2.1.todo.md, this report.
Acceptance: `git diff --check` clean; `git status --short` for the 3 paths shows two ` M` and one `??`; `git diff --name-only` lists only the two must-edit files; grep "Water flux" shows non-empty Status on every water-flux row.
Row 22 final Status: FreeD lower, upper flux, lower flux = accepted for orchestration 29; seepage face, water table = rejected with trigger; qGWLF, TopInF/BotInF = rejected with trigger (flagged not explicitly answered); atmospheric = not decided (G22.3, row 23 unchanged).
Pairs: flux/flux (closed first), flux/FixedHead, flux/FreeD, head/FreeD = accepted for orchestration 29; pairs with seepage or drain = rejected; flux/qGWLF = rejected (flagged).
G22.2 stays Circled. G7 not established, no SD-007.
Deviations: none. Trigger text for accepted forms copied from the audits' "trigger" cells verbatim.

## Discovered

_Empty at planning. Any unlisted work goes here and waits for approval._

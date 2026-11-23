# State 31: close G22.2 and deliver G22.3 with meteorology

Plan: [EXECUTION_PLAN_31.md](EXECUTION_PLAN_31.md). Worker prompt: [PROMPT_TEMPLATE_31.md](PROMPT_TEMPLATE_31.md).
Branch: `feat/vapor-flux-fixedhead`. Tracked planning (D-5).

## Baseline (before wave 1, 2026-11-15)

- [x] `cargo test --workspace --no-fail-fast`: all suites ok (no FAILED result lines).
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- [x] `cargo fmt --all -- --check`: clean.
- [x] `python3 -m unittest discover -s tools -p 'test_*.py'`: 499 tests, OK.
- [x] `vapor_fortran_parity`: 19/19 pass.
- Code head at baseline: `5ed1df2` (after `880c6c2`).

## Wave board

| Wave | Task | Status | Commit(s) |
| --- | --- | --- | --- |
| 1 | 31.1.1 coverage matrix fix | done | `ffa1af1` |
| 1 | 31.1.2 C5 criteria proposal | done | `ac05c44` |
| 1 | 31.1.3 G22.3 + met contract | done | `4a76818` |
| GATE A | user approves C5 and G22.3 contract | approved 2026-11-15 | see Gate A decisions |
| 2 | 31.2.1 met reference harness | done | `bcefbc5` |
| 2 | 31.2.2 radiation Rn/Rns/Rnl port | done | `b7acedf` |
| 2 | 31.2.3 C5 criteria entry (M1) | done | `d2d1f32` |
| 3 | 31.3.1 evaporation port | pending | |
| 3 | 31.3.2 vapor-aware atmospheric switching | pending | |
| 3 | 31.3.3 Meteo.in reader (D-11) | pending | |
| 4 | 31.4.1 met parity | pending | |
| 4 | 31.4.2 met water balance | pending | |
| 5 | 31.5.1 record G22.2 close and G22.3 acceptance | pending | |

## Gate A decisions (2026-11-15, user)

- C5 head: Option A (no closed-pair head entries; rely on exact fixture parity).
- C5 moisture: Option M1 (add `flux_known_on_off_liquid_moisture`, factor 0.75). `criteria.json` edit is task 31.2.3 (wave 2, user decision).
- G22.3 Q1/Q2: bare soil (`iCrop=0`), daily records only for first met fixture.
- G22.3 Q3/Q4: follow Fortran source for `hCritA` with vapor; atmospheric step decomposes into existing Flux/Head pairs (no new `VaporBoundaryPair` variant). Must retain Fortran parity (plan D-10); 31.4.1 is the gate, and a parity failure re-opens this decision.

## Magic Wand refresh (2026-11-22, user)

- 31.2.1 is the Circled Goal (GOALS G22.3-M1). Its worker worktree (`subagent-Worker-31-2-1-met-harness-...`, base `e4fd0ae`) holds only an untracked `31.2.1.todo.md`. Answers to its questions: case name `vapor_met_bare_soil_on`; 240 h with 10 daily records. Rebase onto `49ebb43` before resuming.
- Task 31.3.3 (`Meteo.in` reader) added per plan D-11; 31.4.1 now depends on it.

## Wave 2 end verification (2026-11-22, orchestrator, at `33164ea`)

- 31.2.1 gate checked independently from `RUN_INF.OUT`: 2400 steps reaching 240 h; non-convergence 0; `KodT` switches 3 (-4 to 4 at 42.7 h, 4 to -4 at 120.1 h, -4 to 4 at 200.3 h); no NaN in `T_LEVEL.OUT` or `NOD_INF.OUT`. `criteria.json`, `src_reference/` and `crates/` are unchanged.
- Deck deviation for review: `TopInf=t` (needed for `SetBC` to apply `hCritA`), with daily print times 24 to 240 h. The first switch (42.7 h) falls between print times.
- `cargo test --workspace --no-fail-fast`: 658 passed, 0 failed. clippy `-D warnings`: clean. fmt: clean. Python tools: 504 tests, OK.

## Task reports

## 31.1.1 Fix G22.2 coverage matrix
- Status: complete
- Files changed:
  - `planning/master/M6_VAPOR_COVERAGE.md` (lines 24-25, 31)
  - `planning/master/M6_VAPOR_G22_2_ACCEPTANCE.md` (lines 24-25, 102-110)
  - `planning/tasks/31.1.1.todo.md` (lines 1-23)
- Commit ID: `ffa1af1` (docs commit, no code modified)
- Tests:
  - `grep -n "still rejects" planning/master/M6_VAPOR_COVERAGE.md`: 0 matches
  - `cargo test -p hydrus-core --test vapor_fortran_parity flux_fixedhead_wet_on_matches_fixture`: passed (1 passed, 0 failed)
- Deviations from approved plan: none
- Gotchas for dependent tasks: none; code support pre-landed in `880c6c2` (`FixedHeadLowerFluxUpper`).

## 31.1.2
- Status: done
- Files changed: planning/master/M6_VAPOR_C5_PROPOSAL.md (lines 1-180, created), planning/tasks/31.1.2.todo.md, planning/tasks/31.1.2.report.md
- Docs commit: `ac05c44`
- Tests: cargo test -p hydrus-core --test vapor_fortran_parity (19 passed, 0 failed); python3 -m unittest tools.test_compare_vapor_reference (49 tests, OK)
- Deviations: none
- Gotchas for dependent tasks: criteria.json was NOT modified; GATE A requires explicit user decision on head (Options A/B/C) and moisture (Options M1/M2/M3) before any criteria.json update in subsequent waves.

## 31.2.3 C5 criteria entry (Option M1)
- Status: complete
- Files changed:
  - `reference/vapor_fortran/criteria.json` (lines 116-123, 492)
  - `crates/hydrus-core/tests/vapor_fortran_parity.rs` (line 51)
  - `crates/hydrus-core/tests/vapor_heat_boundary_parity.rs` (line 51)
  - `crates/hydrus-core/tests/vapor_nonuniform_parity.rs` (line 56)
  - `crates/hydrus-core/tests/vapor_nonuniform_timestep_controller.rs` (line 65)
- Code commit ID: `d2d1f32`
- Tests:
  - `python3 -m unittest tools.test_compare_vapor_reference`: 43 passed, 0 failed
  - `python3 tools/compare_vapor_reference.py discrimination --output <tmp>`: PASS (flux_known moisture r_max 0.7619, 4 points above tol)
  - `cargo test -p hydrus-core --test vapor_fortran_parity`: 19 passed, 0 failed
  - `cargo test -p hydrus-core --test vapor_heat_boundary_parity --test vapor_nonuniform_parity --test vapor_nonuniform_timestep_controller`: 15 passed, 0 failed (2 ignored)
- Deviations from approved plan: none
- Gotchas for dependent tasks: criteria.json SHA-256 updated to `b03047a6e0091b227d50da205de08c731e825ce192d88e8d4cadd9fc3b8a8541` across the four pinning test files.


## 31.2.2 Port radiation Rn, Rns, Rnl
- Status: done
- Files changed:
  - `crates/hydrus-core/src/meteo/mod.rs` (created, lines 1-270)
  - `crates/hydrus-core/src/meteo/radiation.rs` (created, lines 1-755)
  - `crates/hydrus-core/src/lib.rs` (lines 13, 28-35)
- Code commit: `b7acedf`
- Tests: `cargo test -p hydrus-core meteo::radiation` (9 passed, 0 failed); `cargo clippy -p hydrus-core --all-targets -- -D warnings` (clean); `cargo fmt --all -- --check` (clean)
- Deviations from approved plan: none
- Gotchas for dependent tasks: Latitude inputs use degrees-and-minutes decimal format (`50.30` = 50°30'); `meteo::radiation` computes strictly in `f32` per ADR 0010/0015; task 31.3.1 (evaporation) can consume `NetRadiation`, `ExtraterrestrialRadiation`, and `CloudinessResult` directly.

## 31.2.1 Met reference harness
- Status: done; gate passed (Fortran reached 240 h, 0 non-convergence, 3 upper-BC switches).
- Files: `tools/run_vapor_reference.py` (MET_* constants and `generate_met_case`, lines ~269-330; allowlist and `f f t t f f` check, ~1 line each); `tools/test_run_vapor_reference.py` (`MetCaseTests`, appended); new `reference/vapor_fortran/cases/vapor_met_bare_soil_on/` (6 files) and `fixtures/vapor_met_bare_soil_on.json`. `criteria.json` untouched.
- Code commit: see `git log` subject "feat(vapor): add bare-soil Penman-Monteith met reference case and fixture".
- Tests: `python3 -m unittest tools.test_run_vapor_reference` -> 45 run, 45 pass.
- Run: `python3 tools/run_vapor_reference.py --case vapor_met_bare_soil_on --build-dir .development/31.2.1/reference-build --task-base .development/31.2.1/runs`.
- Measured from RUN_INF.OUT (2400 steps, dt=0.1 h, max 2 iterations, Convergency column all T; no Error.msg) and KodT column: switches at 42.7 h (-4 to 4, hTop reaches hCritA=-1000), 120.1 h (4 to -4, rain), 200.3 h (-4 to 4). T_LEVEL.OUT hTop confirms (-912 at 42 h, -1000 from 42.7 h, -58 at 144 h).
- Deviations: TopInf=t with KodTop=-4 (TopInf=f never applies hCritA in SetBC); MPL=10 daily print times 24..240 instead of the template's 6/24/72/240.
- Gotchas for 31.3.3/31.4.1: Meteo.in record = `tAtm[h] Rad[MJ/m2/d] TMax TMin RHMean[%] Wind[km/d] SunHours`; tAtm must equal ATMOSPH.IN times (24..240). ATMOSPH.IN records have 11 columns (lTemp adds tTop tBot Ampl); with lMeteo only Prec (cm/h) and hCritA (positive, stored negative) are used, rSoil is overwritten by Meteo (EvapP). Records apply to the interval ending at tAtm. Heights are cm (200/190); bare soil CropHeight=0.1 cm. Latitude -30.0 deg.min format, altitude 100 m. Day 6 rain = 0.1 cm/h (24 mm/d); PET about 5.5 mm/d dry days.

## 31.3.2 Vapor-aware atmospheric switching
- Status: done (code d96fcb7); water_solver.rs and heat/vapor/mod.rs untouched.
- Files: `atmospheric.rs` 84-120 (`clip_to_next_event`, shared with the water-only fn), 360-640 (`VaporAtmosphericState`, `fortran_top_switch`, `advance_atmospheric_step_vapor`, `VaporAtmosphericError`), tests 846+; `forcing.rs` 18-45 (`VaporAtmosphericCoupling`).
- Tests: `cargo test -p hydrus-core atmospheric` -> 12 unit + 3 + 2 integration pass, 0 fail; `cargo clippy -p hydrus-core --all-targets -D warnings` clean; fmt applied.
- Design: sibling fn; Flux/Head pairs only. Order: SetBC rain reset, Shift on start iterate (initial_heads predictor), solve, Shift on converged trial, one re-solve on change; second would-switch flags `oscillation` (list times in 31.4.1). Caller commits state/trial.
- Deviation/risk (D-10): Fortran switches inside the Picard loop (<=2 it/step); here a changed mode re-solves the whole step, so one iteration of history differs. 31.4.1 decides.
- Gotchas: rTop sign is Fortran (out positive); see Fix below. hCritA taken from `UpperBoundary::Atmospheric{critical_head}` as -abs. Pass `initial_heads` already extrapolated. Lower must be FixedHead. WLayer/hCritS rejected with `SurfacePondingUnsupported`. 90%-depth clamp not implemented. Test fixture needs mild heads (about -100 cm) and small rates (K=1e-5); dry heads give singular pivot/inf ConVh.
- Fix (code commit after d96fcb7): rTop and rTopOld now live in `VaporAtmosphericState` and change only at a forcing-record boundary (`record_cursor != event_cursor`), unchanged by mid-record Shift flips; the same rTop feeds the pre- and post-solve Shift. Source order: SetBC interim rTop=-|Prec| (TIME.FOR:131) with the rain reset (TIME.FOR:133-141, so under lMeteo it cannot fire: rTop>0 is false), then met update (TIME.FOR:418 rTop=|rSoil|-|Prec|), then HYDRUS.FOR:627-628. Correction to the orchestrator note: line 628 is unconditional for `.not.lVarBC`, so at every boundary rTop = |rSoil|-|Prec| = E-P in head mode too, not -|Prec|. This matches the pinned run (head mode held 42.7-120.1 h, which a head-mode rTop of 0 would flip every step).
- `oscillation` renamed `second_shift_would_flip` (expected on dry head steps, not an error). The accepted pair is always the one used by the last solve (the second solve when a re-solve occurred); persisted kod_top is that pair, as in Fortran where the last iteration's KodTop persists.
- Tests: `cargo test -p hydrus-core atmospheric` 13 unit + 3 + 2 pass; clippy and fmt clean.

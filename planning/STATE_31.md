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
| 2 | 31.2.1 met reference harness | pending | |
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

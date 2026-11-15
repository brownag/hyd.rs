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
| 2 | 31.2.2 radiation Rn/Rns/Rnl port | pending | |
| 3 | 31.3.1 evaporation port | pending | |
| 3 | 31.3.2 vapor-aware atmospheric switching | pending | |
| 4 | 31.4.1 met parity | pending | |
| 4 | 31.4.2 met water balance | pending | |
| 5 | 31.5.1 record G22.2 close and G22.3 acceptance | pending | |

## Gate A decisions (2026-11-15, user)

- C5 head: Option A (no closed-pair head entries; rely on exact fixture parity).
- C5 moisture: Option M1 (add `flux_known_on_off_liquid_moisture`, factor 0.75). `criteria.json` edit still pending; assign to a wave 2 task or 31.5.1 scope.
- G22.3 Q1/Q2: bare soil (`iCrop=0`), daily records only for first met fixture.
- G22.3 Q3/Q4: follow Fortran source for `hCritA` with vapor; atmospheric step decomposes into existing Flux/Head pairs (no new `VaporBoundaryPair` variant).

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

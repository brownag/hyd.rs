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
| 1 | 31.1.1 coverage matrix fix | pending | |
| 1 | 31.1.2 C5 criteria proposal | pending | |
| 1 | 31.1.3 G22.3 + met contract | complete | 4a76818 |
| GATE A | user approves C5 and G22.3 contract | pending | |
| 2 | 31.2.1 met reference harness | pending | |
| 2 | 31.2.2 radiation Rn/Rns/Rnl port | pending | |
| 3 | 31.3.1 evaporation port | pending | |
| 3 | 31.3.2 vapor-aware atmospheric switching | pending | |
| 4 | 31.4.1 met parity | pending | |
| 4 | 31.4.2 met water balance | pending | |
| 5 | 31.5.1 record G22.2 close and G22.3 acceptance | pending | |

## Task reports

## 31.1.3
- Status: complete.
- Files changed: `planning/master/M6_VAPOR_G22_3_CONTRACT.md` (lines 1-233, created), `planning/tasks/31.1.3.todo.md` (created).
- Commit ID: `4a76818` (docs commit, no code modified).
- Tests: N/A (specification/contract task; verified mapping table and named rejections with triggers).
- Deviations from approved plan: none.
- Gotchas for dependent tasks:
  - 31.2.1: Reference case runner must supply valid `Meteo.in` with $z_w, z_t > 0.667 \cdot h_{\text{crop}}$ to satisfy Error 33 guard.
  - 31.2.2: Fortran `Latitude` uses degrees + minutes decimal encoding (`TIME.FOR:679`); port conversion formula faithfully.
  - 31.3.2: `heat/vapor/mod.rs:312` rejects `Atmospheric` upper boundary; 31.3.2 must coordinate vapor flux/head active-set mapping.

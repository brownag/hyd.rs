# State 32: production met run path in `src/` (G26, with G28)

Plan: [EXECUTION_PLAN_32.md](EXECUTION_PLAN_32.md). Worker prompt: [PROMPT_TEMPLATE_32.md](PROMPT_TEMPLATE_32.md).
Branch: `feat/met-run-path` (from `main` `3f328fc`). Tracked planning (D-5).

## Baseline (before wave 1)

- [x] `cargo test -p hydrus-core --release --test vapor_met_parity --test vapor_met_balance`: 5 + 3 pass (2026-11-24, `3f328fc`).
- [x] `cargo test --workspace --no-fail-fast`: 694 passed, 0 failed, 4 ignored (2026-11-24, `3fc4f2a`).
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- [x] `cargo fmt --all -- --check`: clean.
- [x] `python3 -m unittest discover -s tools -p 'test_*.py'`: 504 tests OK.
- Code head at baseline: `3f328fc` (branch tip `3fc4f2a`, planning only on top).

## Wave board

| Wave | Task | Status | Commit(s) |
| --- | --- | --- | --- |
| 1 | 32.1.1 ADR 0022 (G28) | done | `e8bbb3f` |
| 1 | 32.1.2 `MetForcing` builder | done | `fc16698` |
| 1 | 32.1.3 `step_atmospheric` | done | `5beb4ad`, `a6a2211`, `123ac63` |
| 2 | 32.2.1 rewire test driver (bit-identical) | done | `a6c2ee4` |
| 3 | 32.3.1 record G26/G28, add G31 | done | `7a9321b` |

## Orchestrator log

- 2026-11-24 wave 1: the Agent worktrees were cut from `main` (`3f328fc`), not from `feat/met-run-path`. Each worker rebased onto the feature branch before execution, and each branch was rebased again and fast-forwarded on integration. Commit IDs in the worker reports under `planning/tasks/` predate the integration rebase. The IDs on this board are authoritative.
- Deviation from the template: parallel workers wrote `planning/tasks/<id>.report.md` instead of appending here, to avoid merge conflicts. The orchestrator consolidates the reports below.
- 32.1.2 answers: the `schedule` error is `MetForcingError::Schedule(WaterStepError)`; `NoRecords` added; `AtmosphericRecord`, `MetForcing` and `MetForcingError` re-exported from `lib.rs`; the hCritA check stays in the test wiring.
- 32.1.3 answers: preflight uses a new `pub(crate) VaporConfig::validate_atmospheric_water_input` (mod.rs). It shares a helper with `validate_water_input`, whose public behaviour is unchanged, and classifies the pair with a Flux stand-in. Errors from `advance_atmospheric_step_vapor` surface as `VaporDriverError::Atmospheric { pass, source }`, including `Water(..)`. Orchestrator review nit: a duplicate lint allow, fixed in `123ac63`.
- Wave 1 end (head `123ac63`), serial: workspace tests 704 passed, 0 failed, 4 ignored (+6 forcing, +4 driver); clippy clean; fmt clean; Python 504 OK. Forbidden paths (`water_solver.rs`, `atmospheric.rs`, `reference/`, `tests/`) have no diff since `3fc4f2a`. Wave 1 done.
- 2026-11-24 wave 2: 32.2.1 ran in the main checkout (single task, no worktree). Orchestrator verification:
  - Re-ran `cmp` of `before.json` and `after.json`: OK.
  - `before.json` (17:31:33) predates the code commit (17:32:09).
  - The scratch test is gone, the spec grep returns nothing, and the forbidden paths show no diff.
- Wave 2 end (head `f954c47`), serial: workspace tests 704 passed, 0 failed, 4 ignored; clippy clean; fmt clean; Python 504 OK. Wave 2 done.
- 2026-11-24 wave 3: 32.3.1 (docs only) was reviewed. The worker's report names its report commit as `d679497`; that commit was amended to `9494a5a`. Orchestrator fix in the closing commit: the M6 coverage row had listed the `src/` met path under "Exceptions", so it now reads as the original exception marked resolved.
- Wave 3 end: `git diff 5a7fb87 HEAD -- crates/ tools/ reference/ Cargo.*` is empty, so the wave 2 serial suites (704/0/4, clippy, fmt, Python 504 OK) apply to the final code tree. Wave 3 done.
- **Orchestration 32 complete.** Exit criteria met:
  - `vapor_met_parity` (5) and `vapor_met_balance` (3) pass through `src/` with unchanged tolerances and criteria. `reference/` has no diff, and the MetRun is bit-identical.
  - ADR 0022 records D-10.
  - The full suites are green.
  - Leftover: three merged wave 1 worktrees under `.claude/worktrees/agent-*` (branches `worktree-agent-*`), kept for the user to remove (G22.2-C7 gate).

## Task reports

## 32.1.1
- Status: done. Docs only. ADR commit `e8bbb3f`. Details in [tasks/32.1.1.report.md](tasks/32.1.1.report.md).
- Files: `docs/decisions/0022-re-solve-the-step-on-atmospheric-switches-with-vapor.md` (new), `docs/decisions/README.md` (row 0022).
- Acceptance: the ADR file exists, the README row is present, and there is no diff under `crates/` or `reference/`.
- Deviation: cites `atmospheric.rs:517-523` (the doc comment and fn) instead of 511-522.

## 32.1.2
- Status: done. Code commit `fc16698`. Details in [tasks/32.1.2.report.md](tasks/32.1.2.report.md).
- Files: `src/meteo/forcing.rs` (new), `src/meteo/mod.rs`, `src/lib.rs`.
- Tests: `cargo test -p hydrus-core --lib meteo::forcing` 6/6 (oracle compares f64 bits on `vapor_met_bare_soil_on`). Clippy and fmt clean.
- Gotchas for 32.2.1: keep the hCritA assert in the driver. The API is `MetForcing::new(&cfg, &meteo, &atm, X_CONV, T_CONV)`, then `records()`, `top_temperatures()` and `schedule(units)`.

## 32.1.3
- Status: done. Code commits `5beb4ad` (pure refactor), `a6a2211` (feat) and `123ac63` (style). Details in [tasks/32.1.3.report.md](tasks/32.1.3.report.md).
- Files: `src/heat/vapor/driver.rs`, `src/heat/vapor/mod.rs`.
- Tests: driver lib 11/11 (7 before). Release `vapor_met_parity` 5, `vapor_met_balance` 3, `vapor_fortran_parity` 19, `vapor_boundary_guard` 2, all pass.
- Gotchas for 32.2.1:
  - The pass-2 state is `{ event_cursor: atmo.event_cursor, ..pass1.state }`, built inside the driver.
  - The predictor uses the unclamped `water_input.timestep`.
  - `top_fixed` is taken from pass-2 `kod_top > 0`.
  - `heat_input.top_temperature` must equal the `top_temperature` argument.
  - The report carries `step`, `state`, `switches: Vec<(pass, record)>` and `below_critical_head`.


## 32.2.1
- Status: done. Code commit `a6c2ee4`.
- Files: `crates/hydrus-core/tests/support/vapor_met.rs` only (forcing, driver build, step loop and imports rewritten; module doc updated; predictors and heat closure removed).
- Dump: scratch test (deleted) wrote the full MetRun (all fields via `{:?}`, whole `StepTrace` Debug, kod_changes). Before was deterministic across two runs.
- `cmp .development/32.2.1/before.json .development/32.2.1/after.json`: identical (16391862 bytes).
- Tests: `cargo test -p hydrus-core --release --test vapor_met_parity --test vapor_met_balance`: 5 + 3 pass. Spec grep empty; reference/parity/balance diff clean; fmt and `clippy --workspace --all-targets -D warnings` clean.
- Deviations: none.
- Gotchas for 32.3.1: `AtmosphericRecord` has only `t_atm` and `precipitation`. The driver is built once with `top_temperatures[0]`, and `step_atmospheric` swaps in the per-step value. The caller keeps the hCritA assert and moves `water`/`workspace` in place.

## 32.3.1
- Status: done. Docs only. Plan approved with amendments (Circled bullet per 30a96ed; G26/G28 placed above G24 in Completed; G31 in Deferred; line 8 updated; no wave-board edit by worker).
- Files: `planning/GOALS.md`, `planning/master/M6_VAPOR_G22_3_ACCEPTANCE.md` (dated update section), `planning/master/M6_VAPOR_COVERAGE.md` (row 34), `planning/tasks/32.3.1.todo.md`.
- Commits: plan `c430403`; GOALS and M6 docs `7a9321b`; report `d679497`.
- Acceptance: `grep -n "G31" planning/GOALS.md` matches (line 56). `git diff --quiet -- crates/ reference/` clean.
- Deviations: none.

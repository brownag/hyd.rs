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
| 2 | 32.2.1 rewire test driver (bit-identical) | pending | |
| 3 | 32.3.1 record G26/G28, add G31 | pending | |

## Orchestrator log

- 2026-11-24 wave 1: the Agent worktrees were cut from `main` (`3f328fc`), not from `feat/met-run-path`. Each worker rebased onto the feature branch before execution, and each branch was rebased again and fast-forwarded on integration. Commit IDs in the worker reports under `planning/tasks/` predate the integration rebase. The IDs on this board are authoritative.
- Deviation from the template: parallel workers wrote `planning/tasks/<id>.report.md` instead of appending here, to avoid merge conflicts. The orchestrator consolidates the reports below.
- 32.1.2 answers: the `schedule` error is `MetForcingError::Schedule(WaterStepError)`; `NoRecords` added; `AtmosphericRecord`, `MetForcing` and `MetForcingError` re-exported from `lib.rs`; the hCritA check stays in the test wiring.
- 32.1.3 answers: preflight uses a new `pub(crate) VaporConfig::validate_atmospheric_water_input` (mod.rs). It shares a helper with `validate_water_input`, whose public behaviour is unchanged, and classifies the pair with a Flux stand-in. Errors from `advance_atmospheric_step_vapor` surface as `VaporDriverError::Atmospheric { pass, source }`, including `Water(..)`. Orchestrator review nit: a duplicate lint allow, fixed in `123ac63`.
- Wave 1 end (head `123ac63`), serial: workspace tests 704 passed, 0 failed, 4 ignored (+6 forcing, +4 driver); clippy clean; fmt clean; Python 504 OK. Forbidden paths (`water_solver.rs`, `atmospheric.rs`, `reference/`, `tests/`) have no diff since `3fc4f2a`. Wave 1 done.

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


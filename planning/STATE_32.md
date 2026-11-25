# State 32: production met run path in `src/` (G26, with G28)

Plan: [EXECUTION_PLAN_32.md](EXECUTION_PLAN_32.md). Worker prompt: [PROMPT_TEMPLATE_32.md](PROMPT_TEMPLATE_32.md).
Branch: `feat/met-run-path` (from `main` `3f328fc`). Tracked planning (D-5).

## Baseline (before wave 1)

- [x] `cargo test -p hydrus-core --release --test vapor_met_parity --test vapor_met_balance`: 5 + 3 pass (2026-11-24, `3f328fc`).
- [ ] `cargo test --workspace --no-fail-fast`: all suites ok.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- [ ] `cargo fmt --all -- --check`: clean.
- [ ] `python3 -m unittest discover -s tools -p 'test_*.py'`: count recorded.
- Code head at baseline: `3f328fc`.

## Wave board

| Wave | Task | Status | Commit(s) |
| --- | --- | --- | --- |
| 1 | 32.1.1 ADR 0022 (G28) | pending | |
| 1 | 32.1.2 `MetForcing` builder | pending | |
| 1 | 32.1.3 `step_atmospheric` | pending | |
| 2 | 32.2.1 rewire test driver (bit-identical) | pending | |
| 3 | 32.3.1 record G26/G28, add G31 | pending | |

## Orchestrator log

## Task reports

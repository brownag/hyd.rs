# State 33: reported head-end boundary fluxes for the met path (G25)

Plan: [EXECUTION_PLAN_33.md](EXECUTION_PLAN_33.md). Worker prompt: [PROMPT_TEMPLATE_33.md](PROMPT_TEMPLATE_33.md).
Branch: `feat/tlevel-head-end-flux` (stacked on `feat/met-run-path` at `c7c294c`; rebase onto `main` after the G32 merge, D-10). Tracked planning (D-5).

## Baseline (before wave 1)

- [x] `cargo test --workspace --no-fail-fast`: 704 passed, 0 failed, 4 ignored (verified 2026-11-24).
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: clean (verified 2026-11-24).
- [x] `cargo fmt --all -- --check`: clean (verified 2026-11-24).
- [x] `python3 -m unittest discover -s tools -p 'test_*.py'`: 504 OK (verified 2026-11-24).
- [x] The 31.2.1 reference build still exists and its `hydrus-reference` hash is `87969711f9241b836ad04e24c1987c46e58de2c035d09270ef5c0a33b0519638` (verified 2026-11-24).
- Code head at baseline: `c7c294c`.

## Wave board

| Wave | Task | Status | Commit(s) |
| --- | --- | --- | --- |
| 1 | 33.1.1 ADR 0023 (`TLInf` reported form) | pending | |
| 1 | 33.1.2 runner `--tlevel` mode | pending | |
| 1 | 33.1.3 `TLInf` vT/vB port | pending | |
| 2 | 33.2.1 `T_LEVEL.OUT` sidecar | pending | |
| 2 | 33.2.2 reported fluxes in met trace; balance uses `src/` | pending | |
| 3 | 33.3.1 structural checks and measurement report | pending | |
| gate | G-1 user bounds approval | pending | |
| 4 | 33.4.1 criteria v8, hash rebind, asserting tests | pending | |
| 5 | 33.5.1 record G25 done | pending | |

## Orchestrator log

## Task reports

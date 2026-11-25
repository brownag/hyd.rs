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
| 1 | 33.1.1 ADR 0023 (`TLInf` reported form) | done | `ba96556` |
| 1 | 33.1.2 runner `--tlevel` mode | done | `56b842a` |
| 1 | 33.1.3 `TLInf` vT/vB port | done | `a8bc3fc` |
| 2 | 33.2.1 `T_LEVEL.OUT` sidecar | pending | |
| 2 | 33.2.2 reported fluxes in met trace; balance uses `src/` | pending | |
| 3 | 33.3.1 structural checks and measurement report | pending | |
| gate | G-1 user bounds approval | pending | |
| 4 | 33.4.1 criteria v8, hash rebind, asserting tests | pending | |
| 5 | 33.5.1 record G25 done | pending | |

## Orchestrator log
- 2026-11-24: Wave 1 complete (`ba96556`, `56b842a`, `a8bc3fc`). Serial verification green: Rust 709 passed / 0 failed / 4 ignored, Python 515 OK, clippy/fmt clean.

## Task reports

### 33.1.1
- Status: done (`ba96556`). Created `docs/decisions/0023-report-end-boundary-fluxes-in-the-tlinf-output-form.md` (L1-57) and added README row (L27). Acceptance checks passed; no deviations.
- Gotchas (33.5.1): G20 must decide whether `UpdateEnergy` (`HYDRUS.FOR:462`) reads `trial_v_top` or `TLInf` (`:530`).

### 33.1.2
- Status: done (`56b842a`). Edited `tools/run_vapor_reference.py` (`T_LEVEL_FORMAT_170`, `parse_t_level`, `--tlevel`, generalised sidecar guards, `<case>_tlevel.json` writer) and `tools/test_run_vapor_reference.py` (+11 tests). Targeted suite: 56 passed; full tools suite on main checkout: 515 OK. No deviations.
- Gotchas (33.2.1): `parse_t_level` expects format-130 header, 278-char format-170 rows, and `end` marker; `run_case(..., tlevel=True)` refuses to overwrite an existing `_tlevel.json`.

### 33.1.3
- Status: done (`a8bc3fc`). Edited `crates/hydrus-core/src/heat/vapor/output.rs` (L235-652: `TLevelFluxInput`, `TLevelFluxes`, `t_level_fluxes`, `t_level_fluxes_for_step`, 5 unit tests). Targeted suite: 5 passed; clippy/fmt clean. No deviations.
- Gotchas (33.2.2): `hydrus_core::heat::vapor::output::{TLevelFluxInput, TLevelFluxes, t_level_fluxes, t_level_fluxes_for_step}` is already public. `v_top` and `v_bot` are positive upward (out of soil at top, into soil at bottom).

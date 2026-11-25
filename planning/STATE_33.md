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
| 2 | 33.2.1 `T_LEVEL.OUT` sidecar | done | `9d35728` |
| 2 | 33.2.2 reported fluxes in met trace; balance uses `src/` | done | `7a1fec8` |
| 3 | 33.3.1 structural checks and measurement report | done | `b72d5a7`, `0486164` |
| gate | G-1 user bounds approval | in_progress | |
| 4 | 33.4.1 criteria v8, hash rebind, asserting tests | pending | |
| 5 | 33.5.1 record G25 done | pending | |

## Orchestrator log
- 2026-11-24: Wave 1 complete (`ba96556`, `56b842a`, `a8bc3fc`). Serial verification green: Rust 709 passed / 0 failed / 4 ignored, Python 515 OK, clippy/fmt clean.
- 2026-11-24: Wave 2 complete (`9d35728`, `7a1fec8`, plus `661ec91` fixing `is_trajectory_fixture` to skip `_tlevel.json`). Serial verification green: Rust 709 passed / 0 failed / 4 ignored, Python 515 OK, clippy/fmt clean.
- 2026-11-24: Wave 3 complete (`b72d5a7`, `0486164`). Targeted parity test 2 passed, clippy/fmt clean. Stopped at hard Gate G-1 for user bounds approval.

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

### 33.2.1
- Status: done (`9d35728`). Created `reference/vapor_fortran/fixtures/vapor_met_bare_soil_on_tlevel.json` (SHA-256 `4f819abe86fe0cfd33b9b82e04f30da1bb2f78d8530560ce1ebc3079dbfff20b`). Verified exact match on `executable_sha256` (`87969711…`), `input_sha256`, `NOD_INF_V.OUT` (`a80e702f…`), 2400 rows, `t_level` 1..2400, last `time` = 240.0.
- Deviations (user-approved Option A): `NOD_INF.OUT` (`ca49c9ff…`) is byte-identical to 31.2.1 (`22dc9a1c…`) modulo line 3 (HYDRUS wall-clock `Date: ... Time: ...`).
- Gotchas (33.3.1): Pin `SIDECAR_SHA256 = "4f819abe86fe0cfd33b9b82e04f30da1bb2f78d8530560ce1ebc3079dbfff20b"`.

### 33.2.2
- Status: done (`7a1fec8`). Edited `crates/hydrus-core/tests/support/vapor_met.rs` (added `v_top_reported`, `v_bot_reported`, `v_top_liquid_reported`, `v_top_vapor_reported` to `StepTrace`) and `crates/hydrus-core/tests/vapor_met_balance.rs` (documented `net_vtop`). Byte-identity `cmp before.json after.json` passed (15 MB). Targeted tests: 5 + 3 passing; clippy/fmt clean.
- Deviations (user-approved D-9 amendment, Option 1): kept `net_vtop` in `vapor_met_balance.rs` because `t_level_fluxes_for_step` uses post-heat-solve temperatures (matching Fortran `TLInf`), differing from water-solve `total_face_fluxes` (pre-heat-solve temperatures) by ~9.4e-6 cm/h.
- Gotchas (33.3.1): `StepTrace` exposes `v_top_reported`, `v_bot_reported`, `v_top_liquid_reported`, `v_top_vapor_reported` (cm/h, positive upward).

### 33.3.1
- Status: done (`b72d5a7`, `0486164`). Created `crates/hydrus-core/tests/vapor_met_tlevel_parity.rs` and `planning/master/M6_VAPOR_G25_TLEVEL_MEASUREMENT.md`. Targeted test: 2 passed; clippy/fmt clean. No deviations.
- Key measurements: 2400/2400 sign agreement (positive upward); `max |ΔvTop| = 5.40e-7 cm/h` (`1.59x` `e13.5` floor), `max |ΔvBot| = 1.11e-6 cm/h`; cumulative `max |Δsum_v_top| = 5.39e-5 cm` (final `2.55e-5`), `max |Δsum_v_bot| = 8.25e-6 cm` (final `1.06e-7`). Recommended Option B bounds await Gate G-1 approval.

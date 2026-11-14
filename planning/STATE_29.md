# Orchestration 29 state — G22.2 closed prescribed-flux pair with vapor on

Created: 2026-11-14. Status: **Plan approved 2026-11-14. Wave 0 (baseline) in progress.**
Plan: [EXECUTION_PLAN_29.md](EXECUTION_PLAN_29.md). Goal: [GOALS.md](GOALS.md) G22.2 (Circled).

## Baseline (wave 0)

| Suite | Command | Result |
|---|---|---|
| Build | `cargo build --workspace --locked` | pass |
| Rust | `cargo test --workspace --no-fail-fast` | pass: 620 passed, 0 failed, 4 ignored |
| Format | `cargo fmt --all -- --check` | pass |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| Python | `python3 -m unittest discover -s tools -p 'test_*.py'` | pass: 479 OK |

## Gates

- [x] Wave 0 baseline green (2026-11-14, HEAD d4b44f4).
- [x] GATE 29.1 (user, 2026-11-14): design approved as written, with lGeom implemented in wave 2 (not rejected). Wave 3 whitelist amendment approved. Criteria (D5) deferred to GATE 29.2.
- [ ] GATE 29.2 (user): fixture and balance evidence against the exit criterion. Decision: pending.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 0 | — | Commit plan; baseline | done |
| 1 | 29.1.1 | Fortran closed-pair deck and fixture design | pending |
| 1 | 29.1.2 | Rust design for vapor-aware flux rows and typed guard | pending |
| — | GATE 29.1 | Design approval | done |
| 2 | 29.2.1 | Vapor-aware flux row builders | done (5af819d, ce6dde6) |
| 2 | 29.2.2 | Lift rejection for closed pair only | done (4194d27, 2c56c2e) |
| 3 | 29.3.1 | Column balance check | pending |
| 3 | 29.3.2 | Deck and fixture generation and comparison | pending |
| — | GATE 29.2 | Exit criterion decision | pending |

## Wave gates

### Wave 2 serial gate (2026-11-14, HEAD 2c56c2e)

- Build, Rust (633 passed, 0 failed, 4 ignored; was 620, +13 new unit and guard tests), fmt, clippy, Python (479 OK): all pass.
- Changed paths: `heat/vapor/water.rs`, `water_solver.rs`, `heat/vapor/mod.rs`, new `tests/vapor_boundary_guard.rs`, and planning docs. `lower_boundary.rs`, `criteria.json`, fixtures and `src_reference/` unchanged.
- **Behaviour change to note:** the closed flux/flux pair is now accepted by `validate_water_input` and runs through `solve_vapor_water_trial`. No end-to-end test exists yet. Wave 3 must cover it before GATE 29.2.
- Every other lower or upper combination still returns its `Unsupported*` error. Head/head is unchanged.

## Task reports

_Appended by the orchestrator from each worker's `planning/tasks/<id>.report.md` (plan decision D9)._

## Gate 29.1 decision (2026-11-14)

- Designs approved: `planning/master/M6_VAPOR_CLOSED_PAIR_ROW_DESIGN.md` and `planning/master/M6_VAPOR_CLOSED_PAIR_FIXTURE_DESIGN.md`.
- Open item A: **implement geometric face averaging with vapor in wave 2** (scope widened; see EXECUTION_PLAN_29 wave 2). A fixture for lGeom is not in this orchestration unless approved later.
- Open items B (rTop/rBot sign) and D (no double application of the Neumann flux) are checked in wave 2 against the Fortran; C (lDensity, iDualPor, lWTDep off) is asserted in the guard.
- New criteria entries (D5) decided at GATE 29.2 with the run.

## Wave 3 status (2026-11-14)

- 29.3.1 (balance test) done and committed (`b0a7f26`, `04cc432`): driven residual ~4e-14 relative; zero-flux control passes.
- 29.2/29.3.2 (decks and whitelist) done and committed (`5de3b20`, `70350a5`).
- **29.3.3 BLOCKED.** Both vapor-on closed-pair decks diverge in the Fortran reference (NaN head; many non-convergence messages; hTop -1.35e4 to -7.4e4 within 0.4 h). The vapor-off `vapor_closed_flux_off` fixture is complete (0-240 h). The vapor-off `vapor_flux_known_off` run stopped at 72 h, so it was moved to `.development/29.3.3/quarantine/` and not committed.
- **User decision (2026-11-14):** design a milder vapor-on deck (smaller flux, less dry start, or smaller initial step), re-approved at a gate before any reference run.

## Discovered

- **Correction (29.2.2):** `iDualPor` and `lWTDep` exist in `HeatConfigInput` and are rejected at `heat/config.rs:139-146`. They are off by construction on the vapor path (earlier note said no field).

- **Scope amendment (2026-11-14, user-approved via orchestrator):** 29.1.2 may read `src_reference/WATFLOW.FOR` lines 202-229, 272-294, 315-334 and `crates/hydrus-core/src/heat/vapor/water.rs` lines 1-120, read-only. Reason: the design must cite the Fortran terms and the builder inputs. Plan 29.1.2 scope is otherwise unchanged.
- **Scope amendment (2026-11-14, 29.1.1):** the fixture design must include a nonzero-flux deck as a required second case, not optional. Runner and comparator whitelist edits (`tools/run_vapor_reference.py:185-187`, comparator, criteria case binding) are named as a Wave 3 task to be scoped at GATE 29.1. New criteria entries are a D5 user-approval item.
- **Anchor correction:** the vapor rejections are `vapor/mod.rs:211-218` (lower) and `:219-226` (upper), not `:213-217` and `:219-225` as the plan states. Use the corrected anchors.

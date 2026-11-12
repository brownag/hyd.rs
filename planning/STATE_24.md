# Orchestration 24 state — G12 M6 heat/temperature (through G18)

Plan: [EXECUTION_PLAN_24.md](EXECUTION_PLAN_24.md) · Worker template: [PROMPT_TEMPLATE_24.md](PROMPT_TEMPLATE_24.md)
Status: **RUNNING — approved by user 2026-11-11 ("run plan 24"). Wave 1 done; Wave 2 next. Tolerances (contract `## Tolerances`) await user review before 24.5.2.**
Created: 2026-11-11 · Source goal: [G12 in GOALS.md](GOALS.md)

## Pre-wave-1 gate (must clear before Wave 1)

- [x] **Uncommitted work (cleared 2026-11-11).** Working tree clean on `feat/heat1`; the planning and doc files are committed (HEAD `a155e06`). Original note:  At recon, `git status --short` showed 16 entries (planning/master/*, planning/GOALS.md, planning/STATE_23.md, README.md, TODO.md, docs/decisions/*, plus new untracked M5V_* and DECISION_RECORDING_POLICY_DRAFT.md). Worktrees branch from `HEAD` and will not see them. **User decision:** commit or stash before Wave 1.
- [ ] **Baseline green** (AGENTS.md Quick Start, serial, recorded below). Pending, to run after the gate clears.
- [x] **Branch:** base is `feat/heat1` at `a155e06` (plan recon referenced `fix/robin-upstream-mapping1`; superseded). Task branches `orchestration/<id>` branch from this HEAD.
- [ ] **Fortran reference build** available in `.development/reference-build/` for task 24.4.3 (`tools/stage_fortran_reference.py`, `tools/build_fortran_reference.sh`). Pending.

## Baseline (pre-wave-1)

Run 2026-11-11 on `feat/heat1` at `a155e06`, serial.

| Suite | Command | Result |
|---|---|---|
| Workspace build | `cargo build --workspace --locked` | pass (exit 0) |
| Workspace tests | `cargo test --workspace --no-fail-fast` | pass: 452 passed, 0 failed across 33 test binaries |
| Format | `cargo fmt --all -- --check` | pass (exit 0) |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pass (exit 0) |
| Python tools | `python3 -m unittest discover -s tools -p 'test_*.py'` | **FAIL: 372 run, 4 errors, all in `tools/test_solute_benchmark_bridge.py`** (`ValueError: hash mismatch: crates/hydrus-core/src/solute.rs`) |

Python baseline failure (four `test_solute_benchmark_bridge` hash errors) was a stale `POLICY_HASHES` pin left by commit `075dd71`, which changed `solute.rs` and the harness without a re-pin. Fixed on `feat/heat1` (see Wave 1 gate below): both pins re-pinned with a justification comment. Python suite now passes (372 OK).

Prior reference (from STATE_23 / ADR 0019, not re-run here): parity 44/44; M5-V accepted with limitations 2026-11-11.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 1 | 24.1.1 | Heat source contract | done (9d199a1; Campbell scope amended d2df282) |
| 1 | 24.1.2 | Heat module skeleton and TempCap kernel | done (15d7f99) |
| 2 | 24.2.1 | Heat-only configuration and rejection | done (ef41a44) |
| 2 | 24.2.2 | SetUpHeat boundary kernel | done (9634b4e) |
| 3 | 24.3.1 | Temper transport solve (heat-only) | done (9426aa7) |
| 3 | 24.3.2 | Heat state and accepted-step snapshot | done (d5f8f11) |
| 4 | 24.4.1 | Coupled water→heat step driver | done (757be7d → c4ccc8a) |
| 4 | 24.4.2 | Heat-energy balance accounting | done (2224109 → 2d082a3) |
| 4 | 24.4.3 | Pinned-Fortran heat decks and fixture producer | done (7bb479e, 7a3b662; SD-006 added; 4th deck refused) |
| 5 | 24.5.1 | Analytical comparisons | done (fd2bdb0 → 5770a7e) |
| 5 | 24.5.2 | Source-matched Fortran parity | done (6a18626 → 31b9ad0; rounded verdict pass, strict 2e-4 fails at 20–36 nodes; table water model) |
| 5 | 24.5.3 | Balance and invariant properties | done (5dfdf20 → c94c808; reservoir convention) |
| 6 | 24.6.1 | Capability acceptance document | pending |
| 6 | 24.6.2 | Draft ADR for heat coupling and rejection | pending |

Wave-end gate after each wave: serial AGENTS.md Quick Start suite; counts recorded in the wave's section below.

## Wave gates

### Wave 5 (2026-11-11, HEAD after `31b9ad0`)
- Build, clippy `--workspace --all-targets -D warnings`, fmt: pass.
- Workspace tests: 529 passed, 0 failed (+5 analytical, +4 parity, +6 balance, +4 VG correction tests).
- Python: 389 OK (includes compare_heat_reference tests).
- Verdict: pass for the wave's commits. Parity criterion is the approved rounded rule. Strict 2e-4 verdict fails at 20–36 nodes per default-conductivity time point (output rounding), reported in full.

### Dated correction: van Genuchten conductivity exponent (`1d4131a`, 2026-11-11)
- Finding: `hydraulics/van_genuchten.rs` computed `Se^1.5` hardcoded. The Fortran (`MATERIAL.FOR:29`, `:76`, iModel 0) computes `Se^l * [1-(1-Se^(1/m))^m]^2`, with `l` = `Par(6)` = 0.5 in the heat decks.
- Fix: `l` added as `VanGenuchtenParameters::pore_connectivity` (default 0.5; `with_pore_connectivity` validated in [0, 10]); derivative updated; zero-derivative guard at Se <= 0.
- Effect: no existing test expectation changed (521 → 525 workspace tests, all pass). Water results for unsaturated cases change. This correction is to the accepted M4 water product and is recorded here.
- Why it was not caught: the Fortran-compatible solute parity path uses prescribed water and does not use this hydraulics module.

### Heat parity water model (decision 2026-11-11, user)
- Heat parity uses the table water model, which reproduces the Fortran GenMat table (100 log-spaced knots, `INPUT.FOR:1590`, `:777`). Known differences from SetMat, documented in the test: mixed head `hiM` and the analytic fallback outside the table range. These do not affect the fixtures (heads fixed at -10).
- The accepted water product keeps the analytic default.
- With the analytic model, `heat_dir_top_flux_bot_default` misses the rounded rule by 0.02 degC at 24 h. With the table model, all four pass.

### Open items for Wave 6 and the close
- Strict 2e-4 TempN verdict fails at many nodes in the default cases (output rounding). Documented; not a pass.
- Campbell fixtures are flat at 25 degC: weak evidence for conduction.
- Campbell scaling SD-006 is reproduced as-is; fixed alternative later.
- `kBotT==0` has no fixture (refused case is SD-006 evidence only), but the contract still lists it as Supported. Decide whether to keep it Supported in the acceptance document.
- Balance full residual holds by construction at Dirichlet ends (reservoir convention); interior closure is the real test.
- Analytical nonzero-gradient conduction case is not available through the driver (top flux advective only); zero-flux case used.

### Wave 4 (2026-11-11, HEAD after `eda010b`)
- Build, clippy `--workspace --all-targets -D warnings`, fmt: pass.
- Workspace tests: 510 passed, 0 failed (Wave 3: 501; +5 `heat_driver_order`, +3 `heat::balance`, +1 SD-006 pin).
- Python: 378 OK (includes `tools/test_run_heat_reference.py`).
- Verdict: pass for the wave.
- Open items carried forward: tolerances still unassigned (user review gate before 24.5.2). SD-006 (Campbell scale, reproduced as-is; fixed alternative later). The `kBotT==0` contract entry has no fixture: its only deck is refused (SD-006 evidence). The bottom-flux Campbell fixture is weakly sensitive (profile equilibrates by t=6 h).

### Wave 3 (2026-11-11, HEAD after `9426aa7`)
- Build: pass. Clippy `--workspace --all-targets -D warnings`: pass. fmt check: pass.
- Workspace tests: 501 passed, 0 failed (Wave 2: 490; +4 `heat::transport`, +7 `heat_state`).
- Python: 372 OK.
- Verdict: pass for the wave.

### Wave 2 (2026-11-11, HEAD after `9634b4e`)
- Build: pass. Clippy `--workspace --all-targets -D warnings`: pass. fmt check: pass.
- Workspace tests: 490 passed, 0 failed (Wave 1: 462; +12 `heat_config_contract`, +16 `heat_boundary`).
- Python: 372 OK.
- Verdict: pass for the wave.

### Wave 1 (2026-11-11, HEAD after `d2df282`)
- Build: pass. Clippy `--workspace --all-targets -D warnings`: pass. fmt check: pass.
- Workspace tests: 462 passed, 0 failed (baseline 452; +10 from `heat_capacity`).
- Python: 372 run, 4 errors from the stale `solute.rs` pin (baseline). Re-pinned `solute.rs` and `solute_verification_harness.rs` in `tools/solute_benchmark_bridge.py` after `075dd71`. Re-run: 372 OK.
- Verdict: pass for the wave.

## Task reports

## 24.1.2
- Status: done. Commit `15d7f99` (`feat(heat): add thermal capacity kernel`), fast-forwarded into `feat/heat1`.
- Files: `crates/hydrus-core/src/lib.rs` (+1, `pub mod heat;`); `heat/mod.rs` (new); `heat/capacity.rs` (new, 221 lines); six placeholders; `tests/heat_capacity.rs` (new, 10 tests).
- Tests: `cargo test -p hydrus-core --test heat_capacity --locked`: 10 passed, 0 failed. Clippy `-p hydrus-core --all-targets -D warnings`: pass. fmt check: pass (one mod-ordering fix in `heat/mod.rs`).
- Orchestrator re-ran the same checks in the worktree: 10 passed, clippy pass, fmt pass.
- Deviations: 10-step micro-plan after one revision (14 to 10 steps). Two extra rejection tests added (Campbell `t_conv`, non-finite material parameter).
- Discovered: none.
- Gotchas: `temp_cap(input: TempCapInput, cap: &mut [f64], cond: &mut [f64]) -> Result<(), TempCapError>`. Material indices are zero-based. `TempMaterial::from_tpar([f64; 9])`. Rows 2, 3, 7, 8 of TPar are inferred from usage. Outputs are unspecified after an error.
- `TempCapError` variants: LengthMismatch, MaterialIndexOutOfRange, NonFiniteInput, NonFiniteParameter, InvalidConversion, NegativeWaterContent, NonPositiveCapacity.

## 24.4.1
- Status: done. Commit `757be7d`, cherry-picked to `feat/heat1` as `c4ccc8a`.
- Files: `heat/driver.rs`, `tests/heat_driver_order.rs` (5 tests).
- Signature: `HeatWaterDriver::new(HeatConfigInput, x, material_indices, materials, initial_time, initial_temperatures, initial_velocity)`; `step(&mut self, water_state, water_workspace, WaterStepInput, WaterStepPolicy, CoupledHeatInput) -> Result<CoupledStepReport, HeatDriverError>`.
- Failure behavior: preflight errors return before water runs. Water errors leave heat unchanged. Heat errors after water success return `HeatAfterWaterAccepted`, with water kept advanced (must not retry water alone).
- Deviations accepted: `initial_time` in constructor; `x_conv`/`t_conv` in `CoupledHeatInput`.
- Balance terms (boundary_out, sink) are not computed by the driver; 24.5.3 owns them.

## 24.4.2
- Status: done. Commit `2224109`, cherry-picked to `feat/heat1` as `2d082a3`. `heat/balance.rs`, 3 tests.
- Caller supplies boundary_in, boundary_out, sink_heat; record_step derives none.

## 24.4.3
- Status: done with a scope amendment. Commits `7bb479e` (decks, fixtures, SD-006 pin in `heat/capacity.rs`, `docs/FORTRAN_SOURCE_DEFECTS.md`) and `7a3b662` (second Campbell fixture). Merged as `e513172`, `eda010b`.
- Fixtures: `heat_dir_top_dir_bot_default` (iCampbell 0), `heat_dir_top_flux_bot_default` (0), `heat_flux_top_dir_bot_campbell` (1), `heat_dir_top_flux_bot_campbell` (1).
- Refused (reproduction only, SD-006): `heat_flux_top_zero_bot_campbell` (non-finite output; `refusal.json`).
- Executable sha256 `5a3734bee…`; build manifest `94bf5b96c…`.
- Deviations accepted: `dtMin=0.01`; `run_fixture` not used because it fails under a symlinked `.development` (documented).

## 24.3.1 Commit `5cd9647`, cherry-picked to `feat/heat1` as `9426aa7`.
- Files: `heat/transport.rs` (+569). Tests: `heat::transport` 4/4. Clippy pass. fmt pass after one formatting fix.
- Deviations: `HeatTransportInput` has an extra `temp_old` field (TempO), needed by SetUpHeat for the level-1 F row. Accepted. `HeatBoundaryError` has no Display, so transport formats it with Debug.
- Gotchas for 24.4.1: `solve_heat_transport(config: HeatConfig, input: HeatTransportInput<'_>, temp_new: &mut [f64]) -> Result<HeatTransportReport, HeatTransportError>`. `HeatTransportReport { heat_flux, minimum_absolute_pivot }`. dtMaxT is not used in heat-only mode (one solve per call). The driver must check the mesh length against N. t_period is passed as 0.0, relying on HeatConfig rejecting tPeriod > 0. TempN is stored as f64 without sngl rounding (TEMPER.FOR:55).

## 24.2.1
- Status: done. Commit `ef41a44` (`feat(heat): add heat-only configuration validation`), fast-forwarded into `feat/heat1`.
- Files: `heat/config.rs` (+193), `tests/heat_config_contract.rs` (new, 12 tests), `planning/tasks/24.2.1.todo.md`.
- Tests: `heat_config_contract` 12/12. Clippy pass. fmt pass after one formatting fix.
- Deviations: none. `BoundaryKindNotSupported` is a catch-all with its reject test waived (documented in the test header). `campbell: bool`; the caller maps iCampbell 0/1 and rejects other values.
- Gotchas: `HeatConfig::new(input: HeatConfigInput) -> Result<HeatConfig, HeatConfigError>`. `HeatConfigError` variants are the contract names. Accessors: `top_boundary()`, `bottom_boundary()`, `campbell()`. `HeatConfig` is `Copy`.

## 24.2.2
- Status: done. Commit `f0ab4ef` (`feat(heat): add SetUpHeat boundary kernel`), cherry-picked to `feat/heat1` as `9634b4e` (branch had moved past its base).
- Files: `heat/boundary.rs` (+277), `tests/heat_boundary.rs` (new, 16 tests).
- Tests: `heat_boundary` 16/16. Clippy pass. fmt pass after one formatting fix.
- Deviations: none. Guards beyond the Fortran (documented): N>=2, dt>0 for all kinds, positive spacing, finite inputs.
- Gotchas: `set_up_heat_boundary(level: HeatLevel, input: HeatBoundaryInput<'_>) -> Result<HeatBoundaryRows, HeatBoundaryError>`. `HeatBoundaryRows { bottom, top: BoundaryCoefficients }`, `BoundaryCoefficients { b, d, e, f: Option<f64> }`. Apply only `Some` entries. Level 1 supplies `f`; Level 2 supplies `b`, `d`, `e`. `kTopT == 0` and `tPeriod > 0` return typed errors.

## 24.1.1
- Status: done. Commit `0190035` (`docs: add heat source contract`), cherry-picked to `feat/heat1` as `9d199a1` (branch had moved past its base).
- Files: `planning/master/M6_HEAT_CONTRACT.md` (new, 166 lines, seven headings in spec order). Nothing else.
- Deviations: `kBotT==0` branch cited at `TEMPER.FOR:108-111`. `iCampbell=1` (`TEMPER.FOR:279-300`) added to Rejected under the catch-all rule. Not in the approved question list. Needs user confirmation.
- Discovered: `lWDep=true` is heat-to-water feedback (`WATFLOW.FOR:224-227`, `:404-405`, `:458-464`); rejected as added beyond D7. `WatFlow` resets `TempN=TempO` on retry (`WATFLOW.FOR:139-140`). `Cap==0` calls `stop` after stdin read (`TEMPER.FOR:281-284`); port returns a typed error.
- GATE: numeric tolerances need user review before 24.5.2 runs. Written in the contract.
- Gotchas: proposed Rejected error names are listed in the contract. Integer boundary codes are "manual-pending".

## Follow-on goals to log at wave close (not executed in 24)

- **G19** Vapor flow for the M6 capability (`lVapor`, `TempAdj`, `ConVapor`, `VaporContent`, `ConVh`, `NOD_INF_V` output). Trigger: user selects vapor as the next capability.
- **G20** Energy balance (`lEnBal`, `UpdateEnergy`, surface energy terms). Trigger: user selects energy balance as the next capability; depends on the atmospheric boundary path.

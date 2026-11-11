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

Pre-existing Python failure: the four `test_solute_benchmark_bridge` errors predate orchestration 24 and are not caused by it. They are a source-hash binding for `solute.rs` that no longer matches its recorded value. Orchestration 24 does not touch `solute.rs` or the bridge. Each wave-end Python gate must show exactly these four errors and no others. Any new error is a regression. Recording it here does not make it acceptable: it needs its own fix or a campaign re-bind outside orchestration 24.

Prior reference (from STATE_23 / ADR 0019, not re-run here): parity 44/44; M5-V accepted with limitations 2026-11-11.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 1 | 24.1.1 | Heat source contract | done (9d199a1; Campbell scope amended d2df282) |
| 1 | 24.1.2 | Heat module skeleton and TempCap kernel | done (15d7f99) |
| 2 | 24.2.1 | Heat-only configuration and rejection | pending |
| 2 | 24.2.2 | SetUpHeat boundary kernel | pending |
| 3 | 24.3.1 | Temper transport solve (heat-only) | pending |
| 3 | 24.3.2 | Heat state and accepted-step snapshot | pending |
| 4 | 24.4.1 | Coupled water→heat step driver | pending |
| 4 | 24.4.2 | Heat-energy balance accounting | pending |
| 4 | 24.4.3 | Pinned-Fortran heat decks and fixture producer | pending |
| 5 | 24.5.1 | Analytical comparisons | pending |
| 5 | 24.5.2 | Source-matched Fortran parity | pending |
| 5 | 24.5.3 | Balance and invariant properties | pending |
| 6 | 24.6.1 | Capability acceptance document | pending |
| 6 | 24.6.2 | Draft ADR for heat coupling and rejection | pending |

Wave-end gate after each wave: serial AGENTS.md Quick Start suite; counts recorded in the wave's section below.

## Wave gates

### Wave 1 (2026-11-11, HEAD after `d2df282`)
- Build: pass. Clippy `--workspace --all-targets -D warnings`: pass. fmt check: pass.
- Workspace tests: 462 passed, 0 failed (baseline 452; +10 from `heat_capacity`).
- Python: 372 run, 4 errors, the same four known `test_solute_benchmark_bridge` hash errors as baseline. No new failures.
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

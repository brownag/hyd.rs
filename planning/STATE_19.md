# State — Orchestration 19

Plan status: **Approved by user on 2026-10-14 (“yes”).**
Execution status: **Waves 0–1 done; Wave 2 next.** 18.7.8 was reviewed, fixed and merged first (D10, `60ee1b5`), and Orchestration 18 is paused after it.
Plan: [EXECUTION_PLAN_19.md](EXECUTION_PLAN_19.md). Worker template: [PROMPT_TEMPLATE_19.md](PROMPT_TEMPLATE_19.md).

## Baseline and guardrails

- [x] Reconnaissance 2026-10-14 at `1747e0c`. `cargo test --workspace`: 202 passed, 0 failed. `cargo test -p hydrus-core --lib solute_fortran_compat`: 57 passed.
- [x] Python: system `/usr/bin/python3` 3.12.3. `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` ran 291 tests, OK, in 147 s, and left the tree clean. The `STATE_18.md` interpreter blocker is stale.
- [x] Wave 0 baseline at `60ee1b5` (after the 18.7.8 merge), 2026-10-14: `cargo test --workspace` 226 passed / 0 failed; `cargo test -p hydrus-core --lib solute_fortran_compat` 81 passed; Python 291 OK (99 s, tree clean); fmt clean; clippy `-D warnings` clean; `git diff --check` clean. The pre-merge baseline at `8716f06` was 202 Rust / 291 Python, all green.
- [x] Wave 0: `STATE_18.md` records 18.7.8 accepted (fix `fc8bf8d`: `SinkS` cleared once per `SetSSnk` call, `SINK.FOR:174-176`), 18 paused after 18.7.8, and the Python blocker corrected. Plan amended by D10.
- [x] User approves this execution plan.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 0 | — | Baseline; review+merge 18.7.8; pause 18 | done |
| 1 | 19.1.1 | Compat adapter delegates shape checks to kernels | done (`a0d607f`) |
| 1 | 19.1.2 | `HydraulicModel` provided projections | done (`c518fba`) |
| 1 | 19.1.3 | ADRs 0013–0016 and `CONTEXT.md` | done (`4ed74fa`) |
| 2 | 19.2.1 | Compat hygiene: dead code, error alias, visibility, stub docs | pending |
| 2 | 19.2.2 | Deepen the atmospheric upper-boundary module | pending |
| 3 | 19.3.1 | Split the request adapter into submodules | pending |
| 3 | 19.3.2 | Shared water test fixtures | pending |
| 3 | 19.3.3 | Remove public solute workspace mutators | pending |
| 4 | 19.4.1 | serde decoding: dependency + transport group | pending |
| 4 | 19.4.2 | One lower-boundary value in `WaterStepInput` | pending |
| 5 | 19.5.1 | serde decoding: reactions + transport properties | pending |
| 5 | 19.5.2 | serde decoding: nonequilibrium + biology + source coupling | pending |
| 5 | 19.5.3 | One upper-boundary value in `WaterStepInput` | pending |
| 6 | 19.6.1 | Remove orphaned adapter decode helpers | pending |
| 6 | 19.6.2 | Water step publishes the accepted water snapshot | pending |
| 7 | 19.7.1 | Closed `SoluteBackend` dispatch + contract tests | pending |
| 7 | 19.7.2 | Amend Orchestration 18 for 19's changes | pending |

## Public renames (D3)

_None yet. Tasks append old → new here via the orchestrator._

## Verification

- Wave 1 (at `4ed74fa`): `cargo test --workspace` 226 passed / 0 failed (baseline 226); Python 291 OK; fmt, clippy `-D warnings` and `git diff --check` clean.

## Task reports

### 19.1.1 — done (`a0d607f`)
- `adapter.rs` now calls `boundary::validate`, `dual_domain::validate` (after the kept iDualPor, models-6/7 and model-7 ParD cross-checks), and `source_coupling::validate_uptake`/`validate_exclusion` (at the end of each guard block in `validate_required_mode_inputs`). All four are `pub(super)`.
- The epsi range, alf vs 1−epsi and bottom_resistance checks moved verbatim into `boundary::validate`. Three kernel messages now name their fields.
- When a boundary code is missing, the endpoint validation runs with placeholder code 0. A missing code is still rejected by `validate_required_mode_inputs`.
- Newly rejected at parse time (approved under D6/ADR-0014, no tests added): out-of-range boundary codes; f32 overflow; and invalid dual-porosity state (dual_porosity∉{1,2}, hydraulic_model≠0, dt≤0, non-finite hCritA/arrays/ParD, non-increasing coordinates, ParD ordering and iDualPor=2 bounds). The coupling bundles fail their validators once their guards pass. JSON non-finites were already rejected by `finite_tree`.
- `--lib solute_fortran_compat` 81 → 81. Three test substrings changed (dual-porosity prefix). No fixture rejected.

### 19.1.2 — done (`c518fba`)
- `HydraulicModel::{theta,conductivity,capacity}` default to `Ok(self.evaluate(head)?.field)`. All overrides were deleted from `VanGenuchtenModel`, `HydraulicTables` and the eight list-W test models; none was kept.
- In five test models, `evaluate` used to call `self.theta()` etc. The exact override expressions are now inlined there (otherwise the calls would recurse).
- List W 46 → 46 (alloc 1, atm 11, lower 5, root 7, accept 5, contract 3, solver 10, transaction 4); curves 5, tables 6.
- Gotcha: a new implementer's `evaluate` must not call the provided projections.

### 19.1.3 — done (`4ed74fa`)
- ADRs 0013–0016 added, plus four README rows and `CONTEXT.md` (10 terms). 0014 states the parse-time widening above. 0015 records `van_genuchten.rs` as f64 and the `FH`/`FK` ports as narrowing like the source.
- The orchestrator fixed the heading numbers (`013.` → `13.`). The todo file is a completion stub.

# State — Orchestration 19

Plan status: **Approved by user on 2026-10-14 (“yes”).**
Execution status: **Waves 0–3 done; Wave 4 next.** 18.7.8 was reviewed, fixed and merged first (D10, `60ee1b5`), and Orchestration 18 is paused after it.
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
| 2 | 19.2.1 | Compat hygiene: dead code, error alias, visibility, stub docs | done (`074a627`) |
| 2 | 19.2.2 | Deepen the atmospheric upper-boundary module | done (`cfa0741`) |
| 3 | 19.3.1 | Split the request adapter into submodules | done (`a98c777`) |
| 3 | 19.3.2 | Shared water test fixtures | done (`46d6194`) |
| 3 | 19.3.3 | Remove public solute workspace mutators | done (`4f09c77`) |
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
- Wave 2 (at `cfa0741`): `cargo test --workspace` 229 passed / 0 failed (226 + 3 new `atmospheric::tests`); Python 291 OK; fmt, clippy and diff-check clean.
- Wave 3 (at `882fa87`): `cargo test --workspace` 229 passed / 0 failed / 1 ignored (`dump_parsed_fixtures`). Python first failed 4 tests in `test_solute_benchmark_bridge` with a `solute.rs` hash mismatch. Re-pinned under D11 in `882fa87` (bed1cd17… → 63f83901…); justification: the non-test `solute.rs` diff is five visibility/`#[cfg(test)]` lines and `tests/solute_*.rs` passes 53/0. Then Python 291 OK; fmt, clippy and diff-check clean.

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

### 19.2.1 — done (`074a627`)
- The crate-level `#![allow(dead_code)]` is replaced by per-`mod` allowances, only on the modules that warn when it is removed: `adapter`, `types` (`wired by 18.12.1`); `accounting`, `assembly`, `boundary`, `dual_domain`, `source_coupling`, `transport` (`wired by 18.10.1`). The stubs `biology`, `deposition`, `driver`, `iteration`, `nonequilibrium` and `processes` have no dead items and get no allowance.
- `pub(crate) type KernelError = &'static str` is in `types.rs`; 21 `Result` sites now use it. The `types.rs` top-level items and their methods are `pub(crate)`.
- Stub docs now point to their owners: processes 18.9.1, nonequilibrium 18.7.2 + 18.9.2, biology 18.9.4, deposition 18.9.5, driver 18.10.1, iteration 18.11.1. The ADR-0015 pointer is on `FH`/`FK`.
- `--lib solute_fortran_compat` 81 → 81. The diff touches only attributes, visibility, the alias and docs (orchestrator-filtered).
- Gotcha for 18 kernel tasks: a new module that warns needs its own `#[allow(dead_code)] // wired by 18.10.1` on its `mod` line.

### 19.2.2 — done (`cfa0741`)
- `atmospheric.rs` now owns `AtmosphericAssembly` (pub(crate)) and `AtmosphericAssembly::prepare_step(workspace,state,input,timestep) -> AtmosphericUpperPlan{condition,row,mode}`, called from `assemble`.
- It also owns `decide_transition(rates,upper_mode,workspace,state,input,timestep) -> AtmosphericTransitionDecision{soil_flux,ponded_depth,atmospheric_exchange,stable}`, called from `solve_trial`. It writes the transition trace; `solve_trial` still resets the count.
- Private helpers `top_flux`, `top_flux_for_heads`, `select_mode` and `top_row` were moved verbatim. The two top-flux functions are not merged: their inputs and fallibility differ. The two `hCritA` matches became one `critical_head`, called at the same point at both sites.
- The orchestrator compared the removed and added lines: identical arithmetic, reformatted only.
- Targeted: atm 11, acceptance 5, solver 10, alloc 1 (unchanged). There are 3 new `atmospheric::tests`.
- For 19.5.3: replace the body of `critical_head` (`atmospheric.rs`) with reading `UpperBoundary::Atmospheric{critical_head}`.

### 19.3.1 — done (`a98c777`)
- `adapter.rs` was split into `adapter/`: mod 665, json_guard 176, fields 164, transport 313, reactions 199, transport_properties 94, nonequilibrium 240, biology 70, source_coupling 225, tests 1197 lines.
- New `pub(super)` wrappers hold verbatim blocks: `reactions::{validate_ordered_species, validate_material_species_chpar}`, `transport_properties::validate_dimensions`, `nonequilibrium::validate_dual_domain_dimensions`, `transport::validate_endpoint_state` (called in original order from `validate_cross_field_dimensions`), and `source_coupling::validate_source_coupling_guards(fields, switches)` (called from `validate_required_mode_inputs`).
- Orchestrator check: every item of the old file appears whitespace-insensitively in the new files, up to rustfmt trailing commas. The wrapper bodies are verbatim substrings and cover the old dispatch exactly (5654/5654 chars).
- `#[ignore]` `dump_parsed_fixtures` writes `## parsed_fixtures` (38 CASE.json, sorted) and `## rejection_texts` (5 literals), tab-separated, one record per line.
- `--lib solute_fortran_compat` 81 → 81 (+1 ignored). Insertions were 5.1% over deletions (wrappers plus dump test); accepted.
- Gotchas: group files `use super::*`. The kernel module is reached as `super::super::source_coupling` (name clash). `species_count_value`, `available_len`, `unavailable` and `explicit_from_option_vec` stay `pub(super)` in `mod.rs`.

### 19.3.2 — done (`46d6194`)
- `tests/common/mod.rs` holds `mesh()`, `policy(max_iterations, minimum_timestep)` and `default_policy()`, all proven byte-identical by hash. `atmospheric`/`lower_boundary` import `default_policy as policy`, so call sites are unchanged.
- `input`, the test models, `close`, `assert_close` and `no_flow_lower` stay local: they differ between files or live in only one. List W 46 → 46; `wc -l` 2267 → 2266.
- For 19.4.2/19.5.3: each list-W file's local `input` helper needs its own migration.

### 19.3.3 — done (`4f09c77`)
- `trial_reaction_mass_mut` is now `pub(crate)`. The other four mutators are `#[cfg(test)] pub(crate)`.
- Two mutator tests moved verbatim into `solute.rs` `mod transaction_tests`. The spec was amended from 3: the public-API dimension test stays in `tests/solute_transaction.rs` (3 → 1); `--lib transaction_tests` has 2.
- The first attempt dropped the last 4 lines of `rejected_trial_restores_…`. The orchestrator caught it; the worker restored them on its FAILED VERIFICATION retry, now verified identical.
- Solute integration suites 55 → 53. This edit triggered D11.

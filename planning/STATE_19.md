# State — Orchestration 19

Plan status: **Approved by user on 2026-10-14 (“yes”).**
Execution status: not started. Orchestration 18 is to be paused before 18.7.8 (Wave 0).
Plan: [EXECUTION_PLAN_19.md](EXECUTION_PLAN_19.md). Worker template: [PROMPT_TEMPLATE_19.md](PROMPT_TEMPLATE_19.md).

## Baseline and guardrails

- [x] Reconnaissance 2026-10-14 at `1747e0c`. `cargo test --workspace`: 202 passed, 0 failed. `cargo test -p hydrus-core --lib solute_fortran_compat`: 57 passed.
- [x] Python: system `/usr/bin/python3` 3.12.3. `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` ran 291 tests, OK, in 147 s, and left the tree clean. The `STATE_18.md` interpreter blocker is stale.
- [ ] Wave 0: re-record the baseline at the approved HEAD (workspace tests, Python suite, `cargo fmt --all -- --check`, `cargo clippy -p hydrus-core --all-targets -- -D warnings`, `git diff --check`).
- [ ] Wave 0: mark `STATE_18.md` paused before 18.7.8 and correct its Python blocker.
- [x] User approves this execution plan.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 0 | — | Baseline; pause 18 | pending |
| 1 | 19.1.1 | Compat adapter delegates shape checks to kernels | pending |
| 1 | 19.1.2 | `HydraulicModel` provided projections | pending |
| 1 | 19.1.3 | ADRs 0013–0016 and `CONTEXT.md` | pending |
| 2 | 19.2.1 | Compat hygiene: dead code, error alias, visibility, stub docs | pending |
| 2 | 19.2.2 | Deepen the atmospheric upper-boundary module | pending |
| 3 | 19.3.1 | Split the request adapter into submodules | pending |
| 3 | 19.3.2 | Shared water test fixtures | pending |
| 3 | 19.3.3 | Remove public solute workspace mutators | pending |
| 4 | 19.4.1 | serde decoding: dependency + transport group | pending |
| 4 | 19.4.2 | One lower-boundary value in `WaterStepInput` | pending |
| 5 | 19.5.1 | serde decoding: reactions + transport properties | pending |
| 5 | 19.5.2 | serde decoding: nonequilibrium + biology | pending |
| 5 | 19.5.3 | One upper-boundary value in `WaterStepInput` | pending |
| 6 | 19.6.1 | Remove orphaned adapter decode helpers | pending |
| 6 | 19.6.2 | Water step publishes the accepted water snapshot | pending |
| 7 | 19.7.1 | Closed `SoluteBackend` dispatch + contract tests | pending |
| 7 | 19.7.2 | Amend Orchestration 18 for 19's changes | pending |

## Public renames (D3)

_None yet. Tasks append old → new here via the orchestrator._

## Verification

_Wave-end results go here._

## Task reports

_Distilled reports (≤15 lines each) go here._

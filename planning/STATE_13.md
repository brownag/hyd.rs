# State — Orchestration 13

Plan status: **User approved on 2026-09-26; awaiting execution instruction. No task execution.**
Scientific status: **M5-V open; convergence, independent full-case analytical evidence and justified tolerances remain unmet.**
Plan: [EXECUTION_PLAN_13.md](EXECUTION_PLAN_13.md). Workers: [PROMPT_TEMPLATE_13.md](PROMPT_TEMPLATE_13.md).

## Baseline — 2026-09-26

- [x] Orchestration 12 scoped evidence work completed; its scientific gate remains open.
- [x] Branch `phase4-water-acceptance`, baseline commit `f86a028`; clean tree before planning.
- [x] `cargo test --workspace --quiet`: 130 tests passed, zero failures.
- [x] `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_fortran_reference tools.test_solute_outlet_contract`: 13 passed.
- [x] `PYTHONDONTWRITEBYTECODE=1 python3 tools/compare_solute_reference.py --self-test`: 12 checks passed.
- [x] `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check`: passed.
- [x] No new Fortran execution during planning; no solver changes.
- [ ] Refresh baseline and record protected source/fixture/prior-artifact hashes before execution. Set `HYDRUS_SOLUTE_PARITY_OUT` to a fresh `.development/12.6.1/13-regression/<unique-id>.json` for this and every later full Cargo suite; preserve the accepted original.
- [x] Independent spec review: passed; clarified reporting of measurement floors within user-selected build limits.
- [x] Independent standards review: passed after correcting legacy parity-output preservation; reviewer rechecked the fix.
- [x] User approved the complete plan on 2026-09-26 (“approve”).

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 13.1.1 | Mathematical benchmark/case/schema contract | pending | — |
| 2 | 13.2.1 | Isolated Fortran adapter | pending | — |
| 2 | 13.2.2 | Configurable Rust adapter | pending | — |
| 2 | 13.2.3 | Strict metrics and normalization | pending | — |
| 3 | 13.3.1 | Independent analytical evaluator and evidence | pending | — |
| 4 | 13.4.1 | Separate spatial and temporal convergence | pending | — |
| 5 | 13.5.1 | Discrepancy attribution/remediation specifications | pending | — |
| 6 | 13.6.1 | Budgets, regression gates and M5-V assessment | pending | — |

## Resume point

Independent plan reviews are complete, findings resolved, and user approval recorded. Next task upon a wave-orchestration execution instruction: 13.1.1; refresh the baseline using the protected-output override first. No numerical threshold or new build variant is approved. User chose: “Keep existing limits and report any measurement floor.” Retain the 1001-node capacity and current output formats; variants changing them are outside this plan. Wait for a wave-orchestration execution instruction; plan approval alone has not started any task.

## Task reports

<!-- Empty by design. Orchestrator appends accepted reports, at most 15 lines per task. -->

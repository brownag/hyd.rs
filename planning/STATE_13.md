# State — Orchestration 13

Plan status: **Execution started on 2026-09-26 by user wave-orchestration instruction; Wave 1 accepted; Wave 2 next.**
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
- [x] Refreshed all baseline checks: 130 Rust tests, 13 Python tests, 12 comparator checks, fmt/Clippy/diff passed. Protected 207 files in `.development/13-baseline/protected-hashes.json`; hashes unchanged after checks. Cargo output override: `.development/12.6.1/13-regression/baseline-20260926-01.json`.
- [x] Independent spec review: passed; clarified reporting of measurement floors within user-selected build limits.
- [x] Independent standards review: passed after correcting legacy parity-output preservation; reviewer rechecked the fix.
- [x] User approved the complete plan on 2026-09-26 (“approve”).

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 13.1.1 | Mathematical benchmark/case/schema contract | done | task commit |
| 2 | 13.2.1 | Isolated Fortran adapter | pending | — |
| 2 | 13.2.2 | Configurable Rust adapter | pending | — |
| 2 | 13.2.3 | Strict metrics and normalization | pending | — |
| 3 | 13.3.1 | Independent analytical evaluator and evidence | pending | — |
| 4 | 13.4.1 | Separate spatial and temporal convergence | pending | — |
| 5 | 13.5.1 | Discrepancy attribution/remediation specifications | pending | — |
| 6 | 13.6.1 | Budgets, regression gates and M5-V assessment | pending | — |

## Resume point

Wave 1 accepted after independent equation, boundary, equal-rate and feasibility review. Next: isolated workers for 13.2.1/13.2.2/13.2.3. Contract frozen; 77 conditional runs remain gated on adapters and evaluator. No solver threshold approved; M5-V remains open. Preserve 207 hashes in `.development/13-baseline/protected-hashes.json`. Latest full-suite parity output: `.development/12.6.1/13-regression/wave1-20260926-01.json`.

## Task reports

## 13.1.1
- Status: accepted; mathematical contract and schemas frozen, scientific accuracy not yet established.
- Created spec (593 lines), CASES (2849), README (41), checker (371), and completed TODO (38).
- Independently checked H Robin/Neumann boundaries, resolvent signs, scalar eigenmode, and equal-rate unequal-storage chain (total mass 2 preserved).
- Decimal full-case inversion feasible provisionally; latest tested differences ~3e-10 concentration / 6e-10 integral; empirical, not rigorous remainder bounds.
- Checker self-tests: 34 passed; CASES validation and diff checks passed.
- Wave-end baseline: 130 Rust tests, 13 Python tests, 12 comparator cases; fmt/Clippy passed.
- Protected 207 source/fixture/prior-artifact hashes unchanged; no new Fortran/study solver launches.
- Reviewed matrix: 77 conditional launches, 60 seconds each, 2699462776-byte aggregate cap; activation awaits downstream producer gates.
- Corrected printed concentration precision to four significant digits; original bridge controls retained.
- Deviations: none. Worker recovered from transient service usage error; no incomplete output accepted.
- Remaining: Wave 3 per-observable evaluator admission; capacity-limited temporal contamination and M5-V remain unresolved.

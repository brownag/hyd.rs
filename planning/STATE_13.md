# State — Orchestration 13

Plan status: **Execution started on 2026-09-26 by user wave-orchestration instruction; Wave 1 accepted; Wave 2 Fortran/Rust accepted; metrics worker executing.**
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
| 1 | 13.1.1 | Mathematical benchmark/case/schema contract | done | 490a41b |
| 2 | 13.2.1 | Isolated Fortran adapter | done | task commit |
| 2 | 13.2.2 | Configurable Rust adapter | done | 0023eb4 |
| 2 | 13.2.3 | Strict metrics and normalization | in progress | — |
| 3 | 13.3.1 | Independent analytical evaluator and evidence | pending | — |
| 4 | 13.4.1 | Separate spatial and temporal convergence | pending | — |
| 5 | 13.5.1 | Discrepancy attribution/remediation specifications | pending | — |
| 6 | 13.6.1 | Budgets, regression gates and M5-V assessment | pending | — |

## Resume point

Wave 1 accepted after independent equation, boundary, equal-rate and feasibility review. All Wave 2 micro-plans approved; 13.2.1/13.2.2 accepted, 13.2.3 implementing in `.development/worktrees/<task-id>` on `solute-<task-id>` branches. Contract frozen; 77 conditional runs remain gated on adapters and evaluator. No solver threshold approved; M5-V remains open. Preserve 207 hashes in `.development/13-baseline/protected-hashes.json`. Latest full-suite parity output: `.development/12.6.1/13-regression/wave1-20260926-01.json`.

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

## Wave 2 interface decision

Approved the metrics validated-dict context and exact reference wrapper in the spec. Analytical concentration arrays cover initial/analysis times; continuum observables cover all diagnostic times. Shared artifact names and source formats frozen. Adapter task prose now explicitly leaves independent derived integrals to metrics. No added JSON observation keys, dependencies, or solver launches.

## 13.2.2
- Status: accepted after one review correction; Rust harness and completed TODO integrated from isolated worktree.
- Strict flat request/hash/path validation, fixed-step observations, explicit frozen physics, and solver/derived separation implemented.
- Corrected bridge profile schedule to 0/50/100/200 while retaining all step endpoints.
- Orchestrator reran targeted harness: 15 passed; formatting, targeted Clippy and whitespace passed.
- No core/Cargo/old-fixture edits; no study launches (77 conditional slots remain).
- Wave-end global suite deferred until all Wave 2 producers are integrated.
- Launcher owns casebook/launch authorization and mathematical IC validation; metrics owns independent integrals.

## 13.2.1
- Status: accepted; isolated Fortran adapter, 23 mocked tests and completed TODO integrated.
- Orchestrator reran 23 tests, fresh dry-run and verify-only: all 39 planned Fortran manifests verified.
- Fresh preview: worktree `.development/13.2.1/orchestrator-preview-20260927`; no builds/solver launches.
- Read-only retained bridge validates 2000 steps, four profiles, exact terminals and water mapping.
- Reviewed source/deck/hash preservation, process timeout/storage caps, ledger accounting, no-overwrite and symlink rejection.
- Raw manifests stay verifiable after normalization; bridge unavailable endpoints use null.
- Corrected file-specific solute header and complete mock source-authority fixture during review.
- Protected main 207 hashes unchanged; worker additionally checked 211 across its checkout/shared evidence.
- Deviations: none; all 77 conditional launch slots remain unconsumed.
- Downstream APIs: initial_arrays, request_bytes, generate_decks, validate_outputs; actual build/run bridge remains Wave 3.

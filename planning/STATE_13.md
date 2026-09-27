# State — Orchestration 13

Plan status: **Execution started on 2026-09-26 by user wave-orchestration instruction; Wave 1 accepted; Wave 2 accepted; Wave 3 split into serial tasks; component references accepted; finite-chain evaluator accepted; benchmark integration next.**
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
| 2 | 13.2.1 | Isolated Fortran adapter | done | 4a40220 |
| 2 | 13.2.2 | Configurable Rust adapter | done | 0023eb4 |
| 2 | 13.2.3 | Strict metrics and normalization | done | 15b21b5 |
| 3a | 13.3.2 | Component analytical references | done | 6285bca |
| 3b | 13.3.3 | Finite-domain chain reference | done | pending commit |
| 3c | 13.3.4 | Bounded benchmark integration | pending | — |
| 3d | 13.3.1 | Analytical executions and evidence | pending | — |
| 4 | 13.4.1 | Separate spatial and temporal convergence | pending | — |
| 5 | 13.5.1 | Discrepancy attribution/remediation specifications | pending | — |
| 6 | 13.6.1 | Budgets, regression gates and M5-V assessment | pending | — |

## Resume point

Waves 1 and 2 accepted. Next task: 13.3.4 integration, then original 13.3.1 executions/evidence. Read frozen spec including approved Wave 2 interface appendix and dependency reports. All 77 conditional launch rows remain unconsumed; actual runs require explicit orchestrator activation. Preserve 207 hashes in `.development/13-baseline/protected-hashes.json`. Latest full-suite parity output: `.development/12.6.1/13-regression/wave3b-20260927-01.json`. No solver threshold approved; M5-V remains open.

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

## 13.2.3
- Status: accepted; strict metrics/normalization module, 22 tests and completed TODO integrated.
- Independent per-species norms, phase/continuum/mesh inventories, signed endpoint/reaction integrals and balance implemented.
- Executable/shim hashes bind retained artifacts; initial arrays match declared formulas; strict path and complete record validation.
- Worker cross-adapter synthetic check: 33 profiles, 128 steps, 129 endpoints; retained raw bridge readers independently passed.
- Fixed tiny norm error cancellation; explicit zero/near-zero scales retain uncertainties.
- Profile-coordinate uncertainty remains explicitly unresolved without independent gradient bounds; quadrature sensitivities remain empirical.
- Orchestrator wave checks: 145 Rust tests; 58 Python tests; 12 comparator self-checks; 34 contract self-checks; fmt/Clippy/diff passed.
- Fixed Fortran tamper test to make only isolated copy writable; pinned main sources and all 207 protected hashes unchanged.
- Full Cargo output used fresh `.development/12.6.1/13-regression/wave2-20260927-01.json`.
- Deviations: none; clarified species independence applies without transfer, with separate legitimate chain-coupling coverage.
- Study launches: zero; thresholds unassigned and M5-V remains open.

## Wave 3 decomposition decision

Worker identified four clusters exceeding twelve small steps. Approved serial subwaves3a–3d with separate task gates and commits; preserved original13.3.1 execution root and downstream IDs. Added producer test module `tools.test_solute_benchmarks` to future wave-end suites. Matrix, limits and scientific scope unchanged; no solver execution activated.

## 13.3.2
- Status: accepted; independent component evaluator, 21 tests and completed TODO.
- Reaction uniformization, diffusion cosine and scalar Robin/Neumann formulas use independent integrals and outward Decimal enclosures.
- Reviewed reaction tail indexing, integrated weights, transfer integral and inward signs; scalar frozen-root IC error uses a maximum-principle bound.
- All five component reference budgets admitted; maximum concentration uncertainty 1.36e-16, inventory uncertainty 2.62e-16.
- APIs: component_reference(case, coordinates, analysis_times, diagnostic_times), reference_admission(case, reference); wrapper unchanged.
- Orchestrator checks: 21 targeted tests; 145 Rust tests; 79 Python tests; 12 comparator and 34 contract checks; fmt/Clippy/diff passed.
- Cargo used fresh .development/12.6.1/13-regression/wave3a-20260927-01.json; 207 protected hashes unchanged.
- No study launches, core/source edits or solver thresholds; finite-chain remains explicitly unavailable pending next task.
- Deviation: serial decomposition already recorded above; scientific M5-V remains open.

## Finite-chain feasibility activation

Approved task 13.3.3 micro-plan: reference-only 11-node mesh, analysis times 50/100/200 and ten listed continuum times, orders192/256/384 at400 digits, optional single512 at650 if needed. Precision subset and two fixed parent PDE residual stencils are declared in TODO. This is reduced evaluator feasibility, not full-case scientific admission. No solver launch activation or extra matrix rows.

## Finite-chain interim verification

Implementation review and orchestrator reruns passed:34 targeted tests,145 Rust tests,92 Python tests,12 comparator checks,34 contract checks,fmt/Clippy/diff. Cargo output:.development/12.6.1/13-regression/wave3b-20260927-01.json. Reduced numerical feasibility is still running at .development/13.3.3/feasibility/finite-chain-1d00b2c17a85; do not repeat its logged orders. Task13.3.3 is not yet accepted.

## 13.3.3
- Status: accepted; finite-domain chain evaluator, 34 targeted tests and completed TODO.
- Reviewed stable H/I kernels, confluent divided differences, independent cumulative transforms and strict admission diagnostics.
- Reduced exercise: 11 nodes, 3 profile times, 10 continuum times; all 402 scalar entries admitted empirically.
- Orders 192/256/384; optional 512 unused; maximum uncertainty 2.948e-9; precision difference below 9.724e-144.
- Parent PDE residual reduced 3.9967 times when stencil spacings halved; diagnostic only.
- Runtime 825.87 seconds; rough full-time projection about 5 hours, not a measured cost or bound.
- Artifacts and source snapshots: .development/13.3.3/feasibility/finite-chain-1d00b2c17a85; reuse rules in HANDOFF.md.
- Orchestrator reproduced retained admission, matched current source hashes and verified all 207 protected hashes.
- Orchestrator checks: 34 targeted, 145 Rust, 92 Python, 12 comparator and 34 contract checks; fmt/Clippy/diff passed.
- API: finite_chain_reference returns reference/diagnostics; reference_admission requires bound diagnostics for chain.
- No solver launches; full coverage pending; uncertainty empirical, scientific M5-V remains open.
- Deviations: none beyond recorded decomposition; no core/source/dependency edits.

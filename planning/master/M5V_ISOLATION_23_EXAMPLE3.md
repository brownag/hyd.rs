# M5-V isolation 23 — Example 3 space re-run and dt 0.04 space isolation

Status: evidence collected and verified; unclassified. No accuracy threshold is assigned, no run is accepted, and M5-V stays open.

## Run identity

- Worktree: `orchestration/23.4.3`, base `31fb096`.
- Cases: `reference/solute_verification/CASES_V2.json`, SHA `ceddd59db73e1f54abbfd4198b0b5923ef9838b365bf8c96b691f5ad9718cab7`.
- Output root (shared by all batches): `.development/22-convergence/example3-23`.
- Authorization on every activation: `Orchestration 23 Wave 4, plan approved by the user 2026-10-25`.
- Precision `f32`, backend `fortran-compatible` (reference) and `rust` (solver). Timeout 60 s per row; `space_control` at dt 0.0125 with 180 s, as in CASES_V2.
- No holdout row was run. `--holdout` was not used.

## Lifecycle and verify status

| Batch | Rows | Lifecycle result | `--verify-only` | Notes |
|---|---|---|---|---|
| `batch-8756dae6…` (first B1) | 251 (run), 501, 1001, control (planned) | exit 2, `incomplete lifecycle execution` | not run | The 251 run ended before the solver with `mapping_validation: failed`, `unaccounted Rust build environment override`, `exit_code: null`. No other row ran. |
| `batch-36f7b3c2…` (B1 rerun) | 501 dt .025, 1001 dt .025, control dt .0125 | exit 0 | exit 0, `evidence_verified` | 251 was not in this batch. `m5v_complete: false`. |
| `batch-17ae1e52…` (251 rerun) | 251 dt .025 | exit 0, run `exit_code 0`, mapping `passed` | exit 0, `evidence_verified` | Activated after the first attempt was refused by the environment check. |
| `batch-50d6fe8f…` (first B2) | 501/751/1001 dt .04 | refused: `exact admitted references required before solver starts` | not run | No `runs/` directory, so no solver ran. Recipe had `allow_reference_work: False`. |
| `batch-9b4698e4…` (B2 retry) | 501/751/1001 dt .04 | exit 0, `evidence_complete` | exit 0, `evidence_verified` | Recipe had `allow_reference_work: True`. |

Launch ledger for the output root: 7 consumed in total (3 from B1 rerun, 3 from B2 retry, 1 from the 251 rerun). The ledger was at 6 before the 251 rerun. The failed first 251 attempt does not appear in that count.

Every verified batch reports `scientific_accuracy_accepted: false`, `thresholds: null`, `m5v_complete: false`, and `unmet: [accuracy thresholds unassigned, M5-V open]`. Analysis tooling status was checked as `complete` for the B2 retry and the 251 rerun.

## Deviation (for the record)

The first B2 attempt was run without `--allow-reference-work`. TODO steps 5 and 6 specify that flag for B2 (step 6 says "same command" as B1), so the first attempt did not follow the plan. The refusal message (`tools/run_solute_convergence.py:362`) does not mention the flag; it requires exact admitted references before the solver starts. The retry added the flag, which matches the plan. The refusal happened before any solver ran, and the dry-run manifests show no rows consumed.

## Metrics table (row × field, unclassified)

Each value is the maximum absolute error across all findings and times for that row, taken from `classification.json` per batch. Bounds are missing for every finding, so no value is classified.

| Field | 251 dt .025 | 501 dt .025 | 1001 dt .025 | 501 dt .04 | 751 dt .04 | 1001 dt .04 |
|---|---|---|---|---|---|---|
| balance | 1.25e-2 | 5.03e-3 | 1.57e-2 | 4.25e-3 | 1.38e-2 | 1.01e-2 |
| boundary.cumulative.0 | 1.42e-14 | 1.42e-14 | 1.42e-14 | 1.42e-14 | 1.42e-14 | 1.42e-14 |
| boundary.cumulative.1 | 1.54e-8 | 4.49e-10 | 7.77e-11 | 7.16e-10 | 2.41e-10 | 1.50e-10 |
| boundary.instantaneous.0 | 0 | 0 | 0 | 0 | 0 | 0 |
| boundary.instantaneous.1 | 7.75e-9 | 2.96e-10 | 5.72e-11 | 4.61e-10 | 1.67e-10 | 1.07e-10 |
| concentration.endpoint_linf | 6.17e-5 | 2.21e-5 | 3.33e-5 | 4.05e-5 | 5.24e-5 | 5.65e-5 |
| concentration.interior_linf | 4.33e-2 | 1.27e-2 | 4.32e-3 | 1.33e-2 | 7.33e-3 | 5.41e-3 |
| concentration.l1 | 1.38e-3 | 3.72e-4 | 1.78e-4 | 3.95e-4 | 2.22e-4 | 1.79e-4 |
| concentration.l2 | 5.81e-3 | 1.61e-3 | 5.97e-4 | 1.72e-3 | 1.00e-3 | 7.86e-4 |
| concentration.linf | 4.33e-2 | 1.27e-2 | 4.32e-3 | 1.33e-2 | 7.33e-3 | 5.41e-3 |
| inventory.dissolved | 4.18e-3 | 5.19e-3 | 6.70e-3 | 3.19e-3 | 4.68e-3 | 3.31e-3 |
| inventory.equilibrium_sorbed | 4.18e-3 | 7.93e-4 | 5.51e-3 | 1.09e-3 | 4.68e-3 | 3.31e-3 |
| inventory.total | 8.36e-3 | 5.19e-3 | 1.10e-2 | 3.19e-3 | 9.36e-3 | 6.62e-3 |
| reaction.loss | 4.14e-3 | 3.87e-4 | 4.63e-3 | 1.71e-3 | 4.49e-3 | 3.48e-3 |
| reaction.production | 4.14e-3 | 3.87e-4 | 4.63e-3 | 1.71e-3 | 4.49e-3 | 3.48e-3 |

`space_control` (dt 0.0125) is not in the table because its findings are used only as the other-axis control. Its control assessment is `inconclusive` (`arithmetic_bound_missing`, `quadrature_bound_missing`).

## Old-vs-new comparison

No old-number source exists. The 22.4.5 scratch replay has no space metrics for `space_251`, `space_501`, or `space_1001`. The table therefore has no comparison column.

## Refinement and fitted orders

No order is fitted. Every finding is `inconclusive`, with 0 `achieved` and 0 `failed`, so the spec does not allow a fit. The `rates` entries have `observed: null`.

## Floors and uncertainty

- The `boundary.cumulative.0` values are about 1.42e-14 and `boundary.instantaneous.0` is exactly 0 on every row. These are near-zero values with no bounds, so they are not an established floor and not evidence of convergence.
- The uncertainty fields are zero in every finding. The bounds are missing (`arithmetic_bound_missing`, `quadrature_bound_missing`), so no floor or uncertainty interval is established.

## Observations against the spec

- All 6,090 findings per batch are `inconclusive`. No row reaches a decision.
- Within the dt .025 set, the 251 row has the largest interior, cumulative-boundary, and reaction errors. The 501 row is lower than 251 on every field except `inventory.dissolved`, where 501 (5.19e-3) is higher than 251 (4.18e-3).
- Within the dt .04 set, `concentration.interior_linf` and `concentration.linf` decrease with space refinement (501 1.33e-2, 751 7.33e-3, 1001 5.41e-3).
- For `balance`, the error is not monotone in space: 251 1.25e-2, 501 5.03e-3, 1001 1.57e-2 at dt .025; 501 4.25e-3, 751 1.38e-2, 1001 1.01e-2 at dt .04.
- The B2 analysis block reports `filesystem_receipts_authenticated: false`, while its top level reports `true`. The 251 rerun reports `true` in both places. The B1 rerun's analysis block was not checked. The cause of the B2 mismatch is not established.

## Inputs for threshold proposal

No threshold is proposed. Inputs for a later decision: the missing arithmetic and quadrature bounds, the control row status, and the non-monotone balance result above.

## Additions

`space_control` (dt 0.0125, 180 s) was run in the B1 rerun. Its findings serve only as the other-axis control and are `inconclusive`.

## Discovered

- Timeout: CASES_V2 gives `space_control` a 180 s timeout, but the spec ceiling is 60 s (`SOLUTE_VERIFICATION_SPEC.md:386-387`), and 22.4.4 timed out at 60 s. The timeout was not changed in this run. The `space_control` row completed in batch `batch-36f7…` and passed verify. The discrepancy is left for the orchestrator to decide.
- Centre-row holdout caveat: the 751 dt .04 row is the single centre row, run once in B2 and reused for time isolation (23.4.4). Holdout rows were not run.
- Batch-ID naming: the runner assigns batch IDs. The first B1 batch is `batch-8756…` (incomplete); its rerun is `batch-36f7…`; the first B2 batch is `batch-50d6…` (refused); its retry is `batch-9b46…`; the 251 rerun is `batch-17ae…`.
- Retry conflict: the 251 row's first attempt failed the environment check, and the rerun was started after the check was clear. The B2 retry used the reference-work flag, which the first B2 attempt omitted against the plan.
- Activation path: the first 251 rerun execute used an activation file outside the worktree and exited 2 with `path outside checkout`. No batch was created and no launch was consumed. The rerun used the copy inside the worktree.

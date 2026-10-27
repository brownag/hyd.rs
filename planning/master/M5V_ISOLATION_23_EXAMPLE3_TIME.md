# M5-V isolation 23 — Example 3 time-axis isolation at dt 0.04, and bridge re-run

Status: evidence collected and verified; unclassified. No accuracy threshold is assigned, no run is accepted, and M5-V stays open.

## Run identity

- Worktree: `orchestration/23.4.4`, base `196079d`, with `orchestration/23.4.3` merged in (`03fec1b`) so the 23.4.3 report and centre-row evidence are available.
- Cases: `reference/solute_verification/CASES_V2.json`, SHA `ceddd59db73e1f54abbfd4198b0b5923ef9838b365bf8c96b691f5ad9718cab7`.
- Time rows: `convergence-example3-time_751-dt0.05-rfc-baseline` and `convergence-example3-time_751-dt0.025-rfc-baseline` (study, f32, 751 nodes).
- Centre row (not run here): `convergence-example3-space_751-dt0.04-rfc-baseline`, from 23.4.3 B2 batch `batch-9b4698e478424952bb485e9927fdbe54`. It is the dt 0.04 point on the time axis.
- Bridge row: `benchmark-example3-bridge-rfc-baseline` (role `bridge`, f64, 1001 nodes, dt 0.1, informational).
- Holdout rows: not touched. No `holdout-*` row was run.
- Activation files: `.development/23.4.4/activation-time.json` (phase `convergence`, kind `convergence`) and `.development/23.4.4/activation-bridge.json` (phase `benchmark`, kind `bridge`). Both carry the frozen CASES_V2 SHA and the authorization string.

## Lifecycle and verify status

| Batch | Rows | Runner | Exit | `--verify-only` | Notes |
|---|---|---|---|---|---|
| `batch-0b82b0506b7f4b9bb7ae66bd23dcb841` | time dt 0.05, time dt 0.025 | convergence | 0 (`evidence_complete`, 2 launches consumed) | 0, `evidence_verified` | Cold reference cache in this root; executed with `--allow-reference-work`. |
| `batch-b60b26b1083948688d8b0882dbc0350d` | bridge | benchmark, `--stage bridge` | 0 (`evidence_complete`) | 0, `evidence_verified` | First execute with `--allow-reference-work` exited 2: `bridge stage requires no reference work`. Rerun without the flag exited 0. |

Dry runs planned but did not execute: `batch-f88a4380ae184db38b012eba081dab4b` (time rows) and `batch-3e223dec9335485086bb0afdb2412f39` (bridge, 0 launches consumed).

Every verified batch reports `scientific_accuracy_accepted: false`, `thresholds: null`, `m5v_complete: false`, and `unmet: [accuracy thresholds unassigned, M5-V open]`.

Bridge inventory check (D11): profile, boundary-cumulative, and reaction-cumulative times are `[0, 50, 100, 200]`. The bridge recorded 2000 steps and 2001 endpoint samples.

## Metrics table (row × field, unclassified)

Each value is the maximum absolute error across all findings and times for that row, from `classification.json` of the time batch. The centre column is the 23.4.3 B2 value (dt 0.04), included for comparison. Bounds are missing for every finding, so no value is classified.

| Field | dt 0.05 | dt 0.04 (centre) | dt 0.025 |
|---|---|---|---|
| balance | 1.53e-2 | 1.38e-2 | 1.77e-2 |
| boundary.cumulative.0 | 1.42e-14 | 1.42e-14 | 1.42e-14 |
| boundary.cumulative.1 | 3.45e-10 | 2.41e-10 | 1.34e-10 |
| boundary.instantaneous.0 | 0 | 0 | 0 |
| boundary.instantaneous.1 | 2.35e-10 | 1.67e-10 | 9.58e-11 |
| concentration.endpoint_linf | 6.78e-5 | 5.24e-5 | 2.91e-5 |
| concentration.interior_linf | 7.99e-3 | 7.33e-3 | 6.41e-3 |
| concentration.l1 | 2.48e-4 | 2.22e-4 | 1.89e-4 |
| concentration.l2 | 1.12e-3 | 1.00e-3 | 8.42e-4 |
| concentration.linf | 7.99e-3 | 7.33e-3 | 6.41e-3 |
| inventory.dissolved | 5.09e-3 | 4.68e-3 | 6.26e-3 |
| inventory.equilibrium_sorbed | 5.09e-3 | 4.68e-3 | 6.26e-3 |
| inventory.total | 1.02e-2 | 9.36e-3 | 1.25e-2 |
| reaction.loss | 5.18e-3 | 4.49e-3 | 5.21e-3 |
| reaction.production | 5.18e-3 | 4.49e-3 | 5.21e-3 |

## Refinement and fitted orders

No order is fitted. The spec (`SOLUTE_VERIFICATION_SPEC.md:373-381`) allows rate estimates only where positive resolved errors decrease and the other-axis control is small. The time axis has no other-axis control at 751 nodes, and balance is non-monotone.

Raw ratios, for observation only:

- `concentration.linf` decreases monotonically with dt: 7.99e-3 (dt 0.05), 7.33e-3 (dt 0.04), 6.41e-3 (dt 0.025). The two-step ratio from dt 0.05 to dt 0.025 is about 1.25, which is not evidence of a stable order.
- `balance` does not: 1.53e-2 (dt 0.05), 1.38e-2 (dt 0.04), 1.77e-2 (dt 0.025).
- `inventory.total` and `reaction.loss` are not monotone across the three levels.

## Floors and uncertainty

- `boundary.cumulative.0` is about 1.42e-14 and `boundary.instantaneous.0` is exactly 0 on every row. These are near-zero values with no bounds, so they are not an established floor.
- The uncertainty fields are zero in every finding. Bounds are missing (`arithmetic_bound_missing`, `quadrature_bound_missing`), so no floor or uncertainty interval is established.

## Bridge row

The bridge row is informational and carries no claimed field. It verified (`evidence_verified`) and its inventory times match D11. No metric from the bridge is reported here.

## Observations against the spec

- All 6,090 findings are `inconclusive`. No row reaches a decision.
- Concentration linf falls as dt falls, across the three time levels. This is consistent with time-step error, but the spec's conditions for an order estimate are not met, so it is not reported as one.
- Balance is non-monotone in dt. The dt 0.025 row has a larger balance error than dt 0.05. This is recorded as an observation and not explained here.
- The time-control gap remains: no dt below 0.025 is run at 751 nodes, so the finest time level is 0.025.

## Inputs for threshold proposal

No threshold is proposed. Inputs for a later decision: the missing arithmetic and quadrature bounds, the absence of an other-axis control on the time axis, and the non-monotone balance result.

## Additions

None. The time batch and bridge row are the planned rows only.

## Discovered

- Bridge stage refused `--allow-reference-work` (`bridge stage requires no reference work`). The rerun without the flag matches the stage's rule. This is a plan-text issue, since TODO step 6 says "same rule" and does not say the bridge stage forbids the flag.
- The 23.4.4 worktree has its own output root, `.development/22-convergence/example3-23`, with only the two time batches. The 23.4.3 root is separate, so no shared ledger is implied.
- Dry-run batches `batch-f88a4380…` and `batch-3e223dec…` remain as planned-only directories, consistent with earlier dry runs.
- The first bridge execute exit code was recorded in `bridge-status.txt` as 2, from the runner, not from the shell wrapper.

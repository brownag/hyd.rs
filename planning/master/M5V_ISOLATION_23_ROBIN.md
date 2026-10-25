# M5-V isolating study: scalar_robin at holdout mesh and dt (23.4.2)

Status: unclassified. Gate B has not been given, so no finding is classified. The verify output reports all 380 findings as `inconclusive`. No holdout row was run or read (A6).

## Run identity

- Commit at run: `31fb096` (branch `orchestration/23.4.2`, worktree `.development/23-worktrees/23.4.2`). This report is committed on top.
- CASES_V2 SHA-256: `ceddd59db73e1f54abbfd4198b0b5923ef9838b365bf8c96b691f5ad9718cab7` (matches the frozen value).
- Output root: `.development/22-convergence` (approved; a registered `PHASE_TASKS` key). `23-convergence` is not registered.
- Runner SHA-256: `tools/run_solute_convergence.py` `f9e20b0eea24d618d464f5fbd1929de4be6840bdc9a59711baef496eeb3beec6`; `tools/solute_benchmark_lifecycle.py` `fcb9094e443670a7c0e736158de47ffe5e59f6c938ffefdb9a5be5ed794c0195`.
- Binary: not hashed in this report. The runner's build cache is `.development/22-convergence/rust-builds`; the batch recipes record the build identities.
- Activation authorization: `Orchestration 23 Wave 4, plan approved by the user 2026-10-25`. Files: `.development/23.4.2/activation-space.json`, `.development/23.4.2/activation-time.json`.
- Commands (from the worktree root):
  - Space: `python3 tools/run_solute_convergence.py --cases reference/solute_verification/CASES_V2.json --output-root .development/22-convergence --execute --allow-reference-work --activation-file .development/23.4.2/activation-space.json --launch-id convergence-scalar_robin-space_129-dt2e-7-rfc-baseline --launch-id convergence-scalar_robin-space_193-dt2e-7-rfc-baseline --launch-id convergence-scalar_robin-space_257-dt2e-7-rfc-baseline`
  - Time: same form with `activation-time.json` and the two time launch IDs.
  - Verify: `… --verify-only --batch <batch path>`.

## Lifecycle and verify status

| Batch | Rows | Step | Lifecycle state | Verify exit |
|---|---|---|---|---|
| `batch-f1b13c8d198c4d02b72e8d639afc89d4` | space 129, 193, 257 | dry-run (superseded) | planned | 0 |
| (none) | space | execute attempt 1 | refused: activation outside checkout | n/a |
| `batch-eaf85df7c9f449659ebe460ad164b9cb` | space | execute attempt 2 | refused: admitted references required (no `--allow-reference-work`) | 2 (`selected complete evidence missing`) |
| `batch-771f357ed2d54b5787170d0c267c82e6` | space 129, 193, 257 | execute with `--allow-reference-work` (approved rerun) | `evidence_complete` | 0 (`evidence_verified`) |
| `batch-f521346fd1334a45ad9501ee7e11980e` | time 193 (dt 2^-6, 2^-8) | dry-run (superseded) | planned | n/a |
| `batch-a5475a714c964eb7a283c191ae5c883a` | time 193 (dt 2^-6, 2^-8) | execute with `--allow-reference-work` | `evidence_complete` | 0 (`evidence_verified`) |

- Ledger check before the rerun: `.development/22-convergence/launch-ledger.jsonl` did not exist. Refused attempt 2 left no start record, so the space rows were not consumed. The rerun was therefore allowed.
- The space row `convergence-scalar_robin-space_193-dt2e-7-rfc-baseline` was run once and used for both the space centre and the time centre (same nodes and dt).
- Copies: `.development/23.4.2/batches/batch-771f357ed2d54b5787170d0c267c82e6`, `.development/23.4.2/batches/batch-a5475a714c964eb7a283c191ae5c883a`, and `.development/23.4.2/batches/launch-ledger.jsonl`.
- Verify output: `.development/23.4.2/space-verify.out`, `.development/23.4.2/time-verify.out`.
- Study outputs: `.development/23.4.2/metrics-all.csv` (all 380 findings, both axes).

## Metrics table (row × field)

Errors against the analytical reference, species S1, final analysis time t = 1. Scale and branch as recorded by the tool. Uncertainty is the tool's `uncertainty` field. No finding is classified.

| Axis | Field | Levels: error | Branch | Flags |
|---|---|---|---|---|
| space | `concentration.linf` | 129: 7.03e-4; 193: 4.91e-4; 257: 8.88e-4 | resolved | nonmonotonicity |
| space | `concentration.l2` | 129: 3.08e-4; 193: 3.11e-4; 257: 6.35e-4 | resolved | none |
| space | `concentration.l1` | 129: 2.67e-4; 193: 3.08e-4; 257: 6.24e-4 | resolved | none |
| space | `concentration.endpoint_linf` | 129: 7.03e-4; 193: 4.91e-4; 257: 8.88e-4 | resolved | none |
| space | `concentration.interior_linf` | 129: 6.87e-4; 193: 4.84e-4; 257: 8.84e-4 | resolved | none |
| space | `inventory.total` | 129: 3.46e-4; 193: 3.98e-4; 257: 8.06e-4 | resolved | nonmonotonicity |
| space | `balance` | 129: 5.86e-4; 193: 5.72e-4; 257: 1.12e-3 | nearzero_reference_scale | nonmonotonicity |
| space | `boundary.cumulative.1` | 129: 1.99e-4; 193: 1.28e-4; 257: 2.23e-4 | resolved | nonmonotonicity |
| space | `boundary.cumulative.0` | 0 at all levels | zero | reference_floor, zero_error |
| space | `reaction.loss` | 129: 4.20e-5; 193: 4.68e-5; 257: 9.30e-5 | resolved | nonmonotonicity |
| time | `concentration.linf` | 2^-6: 7.13e-4; 2^-8: 5.29e-4 (2^-7 centre: 4.91e-4 from the space-193 row) | resolved | none |
| time | `concentration.l2` | 2^-6: 3.95e-4; 2^-8: 2.88e-4 | resolved | none |
| time | `concentration.l1` | 2^-6: 3.72e-4; 2^-8: 2.76e-4 | resolved | none |
| time | `concentration.endpoint_linf` | 2^-6: 7.13e-4; 2^-8: 5.30e-4 | resolved | none |
| time | `concentration.interior_linf` | 2^-6: 7.06e-4; 2^-8: 5.22e-4 | resolved | none |
| time | `inventory.total` | 2^-6: 4.81e-4; 2^-8: 3.57e-4 | resolved | none |
| time | `balance` | 2^-6: 7.37e-4; 2^-8: 5.40e-4 | nearzero_reference_scale | none |
| time | `boundary.cumulative.1` | 2^-6: 1.98e-4; 2^-8: 1.42e-4 | resolved | none |
| time | `reaction.loss` | 2^-6: 5.86e-5; 2^-8: 4.21e-5 | resolved | none |

- Uncertainty per field is about 1e-16 to 2e-16 (1e-17 for `boundary.cumulative.0`). Every error in the table is 10^12 or more times larger, so the uncertainty does not overlap the error for any non-zero field.
- Quadrature split: the tool reports `quadrature_bound_missing` and `arithmetic_bound_missing` for every finding, so the quadrature split is not established. The quadrature sensitivity (`quadrature_sensitivity`) is not supplied, and the bounds needed for the spec's split are missing.
- Other-axis control: `control_missing` for every finding. The other-axis control rows are not part of this study, so no control is available.
- Reaction closed-form fields are outside this design (per the design file).

## Refinement and fitted orders

No order is claimed. The tool reports `rates` for each finding, but every rate is `inconclusive`. Observed rates from the tool, for transparency only:

- Space `concentration.linf`: 129→193 observed 0.886; 193→257 observed -2.06. The sign changes between the two pairs.
- Time `concentration.linf`: 2^-6→2^-8 observed 0.215 (over two levels). The 2^-7 centre is the space-193 value, so the 2^-7→2^-8 step is 4.91e-4 → 5.29e-4, an increase.

Sign changes and non-monotonicity mean the spec's rate rule (positive resolved errors that decrease, with small other-axis control) is not satisfied. No order is fitted through any of these points.

## Floors and uncertainty

- `boundary.cumulative.0` is zero at every level, flagged `reference_floor` and `zero_error`. Its rate is not defined.
- The initial-time findings are flagged `reference_floor` and `zero_error` throughout.
- The `balance` field is on a `nearzero_reference_scale` branch, so no relative error is reported. Its absolute error is reported.
- Uncertainty is about 1e-16 for all fields, which is far below the errors. The f32 unit roundoff (about 6e-8) is also below the errors. No floor is established from these data. The f32 floor question stays open because the tool has no floor bound (`arithmetic_bound_missing`, `quadrature_bound_missing`).
- Known limit, recorded and not classified: the study rows use analysis times `[0.25, 0.5, 1]` and f32. The holdout row uses different analysis times and f64. Matching the holdout needs a frozen CASES change, so this study isolates mesh and dt at study conditions only.

## Observations against the spec's expected behaviour

- Space axis is non-monotonic for `concentration.linf`, `inventory.total`, `balance`, `boundary.cumulative.1` and `reaction.loss`. All fields have the same shape: the 193-node error is the smallest, and the 257-node error is larger than the 129-node error (`concentration.linf` 8.88e-4 against 7.03e-4). This is a sign change in the refinement, not a convergence.
- Time axis is non-monotonic at 2^-8 for `concentration.linf` (4.91e-4 at 2^-7, 5.29e-4 at 2^-8). The 2^-6 to 2^-7 step decreases; the 2^-7 to 2^-8 step increases.
- The spec expects backward Euler (first order in time) and at most first-order upwind space. The data show neither: the space rates are not monotone, and the time rate is not established.
- The space-257 error exceeds the 129 error. This is a candidate for the rounding floor, quadrature sensitivity, or a boundary-row effect. The study does not establish which.

## Inputs for threshold proposal

- No thresholds are proposed. No ceiling is assigned.
- Inputs: the 20-value table above for both axes at t = 1, the non-monotonicity at space 257 and time 2^-8, the zero boundary-0 field, and the missing quadrature and arithmetic bounds.
- Known limits to carry into the threshold review: study analysis times and f32 against the holdout's analysis times and f64; `control_missing` for every finding; `arithmetic_bound_missing` and `quadrature_bound_missing` for every finding; the non-monotone space and time behaviour.

## Discovered

1. **Activation path.** The runner requires the activation file inside the checkout. Attempt 1 used the main checkout and was refused (`path outside checkout`). The worktree copy passed.
2. **Reference admission gate.** Attempt 2 was refused before the solver with `exact admitted references required before solver starts` (`tools/run_solute_convergence.py:361-362`). The approved `--allow-reference-work` flag resolved it on the rerun.
3. **Ledger check.** `.development/22-convergence/launch-ledger.jsonl` did not exist before the rerun, so refused attempt 2 consumed no rows.
4. **Status field.** Informational. Neither execute nor verify gates on launch-row `status` (`tools/run_solute_convergence.py:69`).
5. **Design reading.** The time-axis centre coincides with the space-axis centre (193 nodes, dt 2^-7). The earlier "missing row" reading was wrong and is retracted.
6. **Concurrent worker.** Worker 23.4.3 ran in its own worktree during this task. It uses a different root and was not touched.

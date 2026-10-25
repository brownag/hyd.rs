# M5-V isolating study: scalar_robin at holdout mesh and dt (23.4.2)

Status: unclassified. Gate B has not been given. No solver batch completed, so no metrics were computed. No holdout row was run or read (A6).

## Run identity

- Commit: `31fb096` (branch `orchestration/23.4.2`, worktree `.development/23-worktrees/23.4.2`).
- CASES_V2 SHA-256: `ceddd59db73e1f54abbfd4198b0b5923ef9838b365bf8c96b691f5ad9718cab7` (matches the frozen value).
- Runner SHA-256: `tools/run_solute_convergence.py` `f9e20b0eea24d618d464f5fbd1929de4be6840bdc9a59711baef496eeb3beec6`; `tools/solute_benchmark_lifecycle.py` `fcb9094e443670a7c0e736158de47ffe5e59f6c938ffefdb9a5be5ed794c0195`. No solver binary was built or run.
- Output root: `.development/22-convergence` (approved; a registered `PHASE_TASKS` key). `23-convergence` is not registered.
- Activation authorization string: `Orchestration 23 Wave 4, plan approved by the user 2026-10-25`. Activation files: `.development/23.4.2/activation-space.json`, `.development/23.4.2/activation-time.json`.
- Commands run (from the worktree root):
  - `python3 tools/run_solute_convergence.py --cases reference/solute_verification/CASES_V2.json --output-root .development/22-convergence --dry-run --launch-id <space ids>`
  - same with `--execute --activation-file .development/23.4.2/activation-space.json` (attempted twice; see Lifecycle).
  - `… --verify-only --batch <batch path>` for each batch.

## Lifecycle and verify status

| Batch | Rows | Step | Result | Verify exit code |
|---|---|---|---|---|
| `batch-f1b13c8d198c4d02b72e8d639afc89d4` | space 129, 193, 257 (dt 2^-7) | dry-run | `planned`, 3 rows | 0 (`planned_verified`) |
| (none) | space | execute attempt 1 | refused: activation file outside checkout (`path outside checkout`); no batch directory created | n/a |
| `batch-eaf85df7c9f449659ebe460ad164b9cb` | space | execute attempt 2 (retry) | refused before solver start: `exact admitted references required before solver starts` | 2 (`selected complete evidence missing`) |
| (not run) | time 193 at dt 2^-6, 2^-8 | — | not attempted; see Discovered | — |

- The space batch failed twice. The second failure has the same cause (no admitted Fortran references), so the task stops per the ground rules. The time batch was not run, because it would hit the same gate. This is a decision for the coordinator.
- No launch reached the solver. No `run_dir`, no manifest and no metrics exist.
- Logs: `.development/23.4.2/space-execute.out`.

## Metrics table (row × field)

Not computed. No solver output exists for any of the five study rows. The fields `profile_linf`, `profile_l2`, `profile_l1`, `endpoint_linf`, `interior_linf`, inventory, boundary, reaction, balance and `temporal_l2` are all pending.

Planned rows (from `CASES_V2.json`, role `study`, `fortran-compatible`, `f32`):

| Row | Axis | Nodes | dt | Status |
|---|---|---|---|---|
| `convergence-scalar_robin-space_129-dt2e-7-rfc-baseline` | space | 129 | 2^-7 | not run |
| `convergence-scalar_robin-space_193-dt2e-7-rfc-baseline` | space (and time centre) | 193 | 2^-7 | not run |
| `convergence-scalar_robin-space_257-dt2e-7-rfc-baseline` | space | 257 | 2^-7 | not run |
| `convergence-scalar_robin-time_193-dt2e-6-rfc-baseline` | time | 193 | 2^-6 | not run |
| `convergence-scalar_robin-time_193-dt2e-8-rfc-baseline` | time | 193 | 2^-8 | not run |

## Refinement and fitted orders

None. No order was fitted, because no solver output exists.

## Floors and uncertainty

None measured. Quadrature split and f32 floors are unknown until batches run.

## Observations against the spec's expected behaviour

- No behaviour is observed yet. The expected behaviours in the spec (first-order time, at most first-order upwind space, possible second-order centred diffusion) cannot be checked.
- Known limit (recorded, not classified): the study rows use the design's analysis times `[0.25, 0.5, 1]` and `f32`. The holdout row uses different analysis times and `f64`. Matching the holdout would need a frozen CASES change, so this limit stands for this study, as for diffusion. This study therefore isolates the mesh and dt at study conditions, not at holdout conditions.
- The design reads the 193-node, dt 2^-7 space row as the time-axis centre too. This is consistent with the approved reading. It is one run, not a missing row.

## Inputs for threshold proposal

- No thresholds are proposed here.
- Inputs not yet available: the five study rows' metrics, the orders, and the floors.
- Limits to carry forward: study analysis times `[0.25, 0.5, 1]` and `f32` versus holdout analysis times and `f64` (known limit, needs a frozen CASES change); the time-axis centre is the space-193 row, so its error is reported on both axes.

## Discovered

1. **Activation file path.** The runner requires the activation file to lie inside the checkout. The first attempt used `.development/23.4.2/` in the main checkout and was refused (`path outside checkout`). The retry used the worktree copy and passed that check.
2. **Reference admission gate.** Both execute attempts stop before the solver with `exact admitted references required before solver starts` (`tools/run_solute_convergence.py:361-362`). The execute path calls `self.references(…, allow_work=self.recipe["allow_reference_work"])`. The batch was created without `--allow-reference-work` or `--prior-cache-root`. Neither was in the approved plan, and the run was stopped rather than chased. Coordinator decision needed on whether to add those flags or supply a prior cache.
3. **Root.** `23-convergence` is not a registered root. `22-convergence` was used as approved.
4. **Status field.** Informational. Neither execute nor verify gates on launch-row `status` (checked earlier: `tools/run_solute_convergence.py:69` tests only `reserved_until_threshold_gate`).
5. **Design reading.** The time-axis centres (diffusion 97 @ 2^-7, Robin 193 @ 2^-7, Example 3 751 @ 0.04) coincide with the space-axis centres. The earlier "missing row" reading was wrong and is retracted.

# M5-V isolation study 23.4.1: diffusion_cosine at holdout mesh and dt

Report only. Results are unclassified: gate B (bounds review and signature) is pending. No thresholds are proposed. The holdout row `holdout-diffusion_cosine-holdout-rfc-baseline` (97 nodes, dt 2^-7) was not run or read (A6).

## Run identity

- Commit at execution: `31fb096` (branch `phase4-water-acceptance`); worktree `.development/23-worktrees/23.4.1`, branch `orchestration/23.4.1`.
- CASES_V2 SHA-256: `ceddd59db73e1f54abbfd4198b0b5923ef9838b365bf8c96b691f5ad9718cab7` (matches the frozen value).
- Output root: `.development/22-convergence/23.4.1` (the 22.4.4 precedent; `23-convergence` is not a registered phase root).
- Harness binary (`solute_verification_harness-ec7dfff3fd1b6e1e`): SHA-256 `8f3a97dd96bb2b5e0f3065acb0505ccd04bdb1c824fe714fec89d7d73330a6ca`, built with `CARGO_TARGET_DIR=.development/23-target/23.4.1`.
- Runner `tools/run_solute_convergence.py` SHA-256: `f9e20b0eea24d618d464f5fbd1929de4be6840bdc9a59711baef496eeb3beec6`.
- Batch IDs (both verified; see the batch table): space `batch-160463d79070463db5d2788339450de9`, time `batch-b9356e1aac134e4993266b5e18d70338`.
- Commands (from the worktree root):
  - Space: `env -u CARGO_TARGET_DIR python3 tools/run_solute_convergence.py --cases reference/solute_verification/CASES_V2.json --output-root .development/22-convergence/23.4.1 --execute --allow-reference-work --activation-file .development/23.4.1/activation-space.json --launch-id <S65> --launch-id <S97> --launch-id <S129>`
  - Time: the same, with `activation-time.json` and the two time launch IDs.
  - Verify: `python3 tools/run_solute_convergence.py --cases … --output-root .development/22-convergence/23.4.1 --verify-only --batch <batch>`.
- Metric extraction: `.development/23.4.1/extract_metrics.py` (read-only over the batch `classification.json` files; output `.development/23.4.1/metrics.json`).
- Activation authorization: "Worker 23.4.1 derived activation; orchestrator approval 2026-10-25 (APPROVED - execute the plan as written, with answers)".

## Lifecycle and verify status

| Batch | Rows | Launch IDs | Lifecycle state | Verify exit code |
|---|---|---|---|---|
| `batch-160463d79070463db5d2788339450de9` (space) | 3 | S65, S97 (centre), S129 | `evidence_complete`, all 3 `metrics_complete` | 0 |
| `batch-b9356e1aac134e4993266b5e18d70338` (time) | 2 | T97a (dt 2^-6), T97b (dt 2^-8) | `evidence_complete`, 2 rows | 0 |
| `batch-dee15d156eab4e7bb5f61f5b8ea0a689` | 0 run | (space, first attempt) | refused before the solver: "exact admitted references required before solver starts" (`tools/run_solute_convergence.py:362`); no `runs/` directory | not verified (not run) |

- The refused attempt is recorded and not counted as a run. The retry used `--allow-reference-work` (the cold-cache recompute the Wave 4 plan accepts); no `--prior-cache-root` and no `--resume-batch` were used.
- Ledger after both batches: 5 launches consumed (`consumed: 5`, `cap: null`).
- Each batch wrote a `classification.json` through the CLI. These are not used as classifications: gate B is pending, and the tool's per-finding status is `inconclusive` in every case (no other-axis control; floor and arithmetic bounds missing).
- No holdout row was launched.

## Metrics table (row x field, analysis times 0.25 / 0.5 / 1; absolute errors, unclassified)

Single species (`S1`). Rows are the five study launches: S65 (65 nodes, dt 2^-7), S97 (97 nodes, dt 2^-7, centre), S129 (129 nodes, dt 2^-7), T6 (97 nodes, dt 2^-6), T8 (97 nodes, dt 2^-8).

| Field | t | S65 | S97 | S129 | T6 | T8 |
|---|---|---|---|---|---|---|
| profile_linf (`concentration.linf`) | 0.25 | 1.771e-3 | 1.145e-3 | 8.482e-4 | 1.123e-3 | 1.151e-3 |
| | 0.5 | 2.339e-3 | 1.526e-3 | 1.134e-3 | 1.488e-3 | 1.534e-3 |
| | 1 | 2.984e-3 | 1.962e-3 | 1.462e-3 | 1.898e-3 | 1.975e-3 |
| profile_l2 (`concentration.l2`) | 1 | 8.513e-4 | 5.612e-4 | 4.174e-4 | 5.335e-4 | 5.659e-4 |
| profile_l1 (`concentration.l1`) | 1 | 4.504e-4 | 3.084e-4 | 2.368e-4 | 2.980e-4 | 3.015e-4 |
| endpoint_linf (`concentration.endpoint_linf`) | 1 | 2.984e-3 | 1.962e-3 | 1.462e-3 | 1.898e-3 | 1.975e-3 |
| interior_linf (`concentration.interior_linf`) | 1 | 2.748e-3 | 1.857e-3 | 1.404e-3 | 1.793e-3 | 1.870e-3 |
| inventory (`inventory.total`, signed) | 1 | -7.748e-4 | -5.304e-4 | -4.073e-4 | -5.126e-4 | -5.187e-4 |
| balance (absolute, zero reference) | 1 | 7.748e-4 | 5.304e-4 | 4.073e-4 | 5.126e-4 | 5.187e-4 |
| boundary (`boundary.cumulative.0/1`) | all | 0 | 0 | 0 | 0 | 0 |
| reaction (`reaction.loss/production`) | all | 0 | 0 | 0 | 0 | 0 |
| temporal_l2 | - | not emitted by the analysis (see below) | | | | |

Values at t = 0.25 and 0.5 for the l1, l2, interior and inventory fields are in `.development/23.4.1/metrics.json`. The inventory error is negative at every time (signed).

Scale and branch: every concentration field and the inventory error carry branch `resolved` (relative); the named scale value is not in the analysis output and is not extracted here (gap). Balance, boundary and reaction carry branch `zero`, so the relative value is null and the absolute error is reported.

Quadrature split (spec `:356-362`): the analysis reports `quadrature_sensitivity: null` for every level (not computed for this case). The exact-continuum versus mesh-weighted split is therefore unavailable in this study.

## Refinement and fitted orders

Observed orders are the tool's rates between adjacent levels. They are descriptive only: the control status is "missing" for every finding, so no order here is a rigorous asymptotic claim.

Space axis, dt 2^-7 (65 to 97 to 129 nodes), at t = 1:

| Field | Order (65 to 97) | Order (97 to 129) |
|---|---|---|
| profile_linf | 1.034 | 1.021 |
| profile_l2 | 1.028 | 1.029 |
| interior_linf | 0.967 | 0.973 |
| inventory | 0.935 | 0.918 |

- The space order is close to 1, not 2. Interior values sit slightly below 1. This is below the spec's "may show second order" for smooth interior fields.
- Inventory order is below 1 and decreasing toward the coarse-mesh level: the mass defect shrinks with refinement but does not change order.
- Errors decrease monotonically at every time: no plateau, no sign change in the space rows.

Time axis, 97 nodes (dt 2^-6, 2^-7 centre, 2^-8):

- Observed time rates are negative (linf: -0.0286; l2: -0.0426 at t = 1), and the tool flags `nonmonotonicity`. Inventory rate is -0.0085.
- Interpretation: at 97 nodes the spatial error (about 2e-3 at t = 1) dominates. The total error moves toward the spatial limit as dt is refined, so the time-driven part has the opposite sign to the total change. The time order is not fitted (no other-axis control; see Questions).
- Result for the time axis: no fit, "other-axis control missing" (approved answer 1).

## Floors and uncertainty

- Reference uncertainty: about 1e-16 for every concentration and inventory observable; 0 for reaction and boundary. All errors (1e-4 and above) sit far above the reference uncertainty and the f32 rounding scale (about 1e-7 relative). No floor is reached in these rows, and no order is fitted through one.
- The time-axis differences at t = 1 (T6 to T8: 7.7e-5 in linf) are far above the reference uncertainty (about 1e-16) and about 770 times the f32 rounding scale (about 1e-7). The sign pattern is noted above.
- Control status is `missing` for all findings (no other-axis control at dt 2^-7 for space, and no 97-node fine-space control for time). The analysis reports `inconclusive` for every finding for this reason, and the floor assessment lacks arithmetic and quadrature bounds.
- `temporal_l2` is not emitted by the current analysis (it is not in the findings list). It is reported as unavailable, not as zero.

## Observations against the spec's expected behaviour

- Spatial accuracy: first order in the profile norms (about 1.02 to 1.03) and about 0.97 in the interior. The spec allows second order in smooth interior fields for centred diffusion, but endpoint conditions and inventory can reduce it. The observed behaviour is consistent with that reduction, but not proven (no control).
- Endpoint: `endpoint_linf` equals `profile_linf` in every row, so the worst error is at the endpoint.
- Inventory and balance: negative signed inventory error (mass deficit) that grows in time and shrinks with refinement, with order 0.92 to 0.94. This matches the known mass loss at the end rows (SD-005, `SOLUTE.FOR:803-827`). It is recorded as a known defect, not as a blocking result.
- Boundary and reaction: all zero. For diffusion this is expected for reaction; the boundary being exactly zero is recorded, not explained.
- Time: the total error is dominated by the spatial error at 97 nodes. The time order is not measurable in this design (see Inputs).
- Isolation limits (for gate C, per the coordinator's answer 2): the study rows use analysis times [0.25, 0.5, 1] at f32 precision. The holdout uses analysis times [0.375, 0.75, 1] at f64. These differences are a known limit of this isolation: the study does not exercise the holdout's analysis times or its precision. Changing either needs a CASES change, which the freeze forbids.

## Inputs for threshold proposal (no thresholds proposed)

- Per-field errors at holdout mesh and dt: profile_linf 1.96e-3 (t = 1), profile_l2 5.61e-4, profile_l1 3.08e-4, interior_linf 1.86e-3, inventory -5.30e-4, with spatial 65 and 129 bracketing it.
- Space order is about 1 (not 2); the threshold proposal should not assume second order.
- Time axis has no fit. Any temporal ceiling needs an other-axis control or must be marked inconclusive.
- Quadrature split and `temporal_l2` are unavailable for this case; they cannot support a ceiling as-is.
- Limit (report only): study analysis times and precision differ from the holdout (see Observations).
- Unclassified: gate B pending. No classification is claimed in this report.

## Discovered

- The lifecycle refuses `23-convergence` as a root (only `13.*` and `22-*` are registered); the 22.4.4 root was used.
- The first space execute was refused before the solver because the admitted reference was not present; the cold-cache recompute (`--allow-reference-work`) was needed. Refused attempt recorded above.
- The status field `approved_pending_adapter_and_evaluator_gates` is not read by the execute path. The only status check is for `reserved_until_threshold_gate` (`tools/run_solute_convergence.py:69`).
- The analysis does not emit `temporal_l2` and does not compute the quadrature split for this case.
- The scale value for relative metrics is not in the analysis output.

# M5V study 22.4.3: `scalar_robin` on the default FortranCompatible backend

Study only: no thresholds are proposed and no code changed. Errors are against the analytical reference (`tools/solute_analytical_reference.py`); expected values never come from solver output. The reserved holdout row was not run.

Superseded first pass: an earlier run on the defective public step (outlet pile-up, inlet/outlet orientation; errors 0.3-0.8, independent of refinement, outlet above c_in) is discarded. Its roots, ledgers, batches and old report are preserved intact in `.development/22.4.3/superseded/` (`firstpass-roots/`, `firstpass-roots.filelist.sha256` with 1478 files, ledger SHA-256 `b75920d2…` benchmark and `7775cd3e…` convergence). Nothing below uses it.

## Run identity

- Worktree HEAD `d2a8bef` (includes 22.3.8); `CASES_V2.json` SHA-256 `644c4845956d48cc27e0f45387a06317dbbf9aa671667617fc6487e4b3ad8a3d`; backend `fortran-compatible`, `solver: rust`.
- Activation files (derived by the worker from `lifecycle.validate_activation` and the O13 activations, in `.development/22.4.3/`): benchmark `ad41be2c5bfdbd6ca707ec88cc37dda7578aa3389bb1c96e4df3659c41725c0b` (1 row), convergence `f0681c9b3ee966b636859f3ccc86333b17fee8a0fa812cd9778856b6abbfa90d` (8 rows). The bridge gate wrote the `not_applicable_v2` receipt.
- Pre-built harness `solute_verification_harness-ec7dfff3fd1b6e1e`, SHA-256 `b88644938d0b37e7ee73f0a569f45ae234948f566ce80d3d0463a64aabec8031`; the lifecycle's own runner executable SHA-256 is `b52169f9bc015892fa658bbe39d792f3620f3459f6fcc490afb94f6ad96b6af4` (manifest provenance).
- Roots: fresh `.development/22-benchmark` and `.development/22-convergence` (old ones archived), output roots `robin-v2`. Fresh ledger SHA-256: benchmark `df7016c6…`, convergence `a04ac673…` (not edited by hand).
- Commands (from the worktree, `CARGO_TARGET_DIR` unset, `target` symlink present):
  - `check_solute_benchmarks.py --cases reference/solute_verification/CASES_V2.json --output-root .development/22-benchmark/robin-v2 --execute --stage benchmark --allow-reference-work --activation-file .development/22.4.3/activation-benchmark.json --launch-id benchmark-scalar_robin-component-rfc-baseline`
  - `run_solute_convergence.py --cases ... --output-root .development/22-convergence/robin-v2 --execute --allow-reference-work --activation-file .development/22.4.3/activation-convergence.json --launch-id <8 ids>`
  - no `--allow-reference-escalation`, no `--bounds-source`.
- Batches: benchmark `batch-bd9a83ddd31a4ab98ae8c9fdacfb32e3`; convergence `batch-d8607f2416c84a2385ba6d3f521363c1`.
- Analytical reference (admitted, cache under the roots): identity `1fbb35ab…` (benchmark row and space_129; `reference.json` `0a5ea7f8ed1c61e8abd301242c188a31b8af1d24bbb5b19d91ed7769bc7c5f1a`, `identity.json` `232ad8d7…`); `bde1a9af…` (space_65, `reference.json` `0e296ed6…`); `492d9f45…`, `6984afc9…`, `df8a50b3…` (full values in each `reference-cache/<id>/`).
- Manifest SHA-256 (9 runs): `.development/22.4.3/manifest-sha256.txt`. Convergence `classification.json` `260d34db528cbfe3499bf53dce950b76e07e6cfbb122d0167b33dffe11650ed1`; `summary.json` `6120e788989f2983ac2de501fe17991f5a6f9e7658b6f69b3913167a5b7f19cd`; benchmark summary `eb9f21a3bb97ecf88f1a80fbe585a05a26d31ffb3c1f3bd9605f7d28c17d3402`.

## Lifecycle and verify status

Both batches `evidence_complete`; all 9 manifests `complete`, exit 0, `mapping_validation: passed`; 9 launches consumed on the fresh ledgers; no solver row failed and no retry was used.

| batch | `--verify-only --batch` exit | status |
|---|---|---|
| benchmark `batch-bd9a83dd…` | 0 | `evidence_verified` |
| convergence `batch-d8607f24…` | 0 | `evidence_verified` |

## Metrics table

Source: `classification.json` groups, formatted by `.development/22.4.3/fmt_tables.py` into `.development/22.4.3/tables.md` (all 17 times, all fields; shown here: t = 0.25, 0.5, 1, without `boundary.instantaneous`). L1-L3 are the errors at the three study levels (space: 33, 65, 129 nodes at dt 2^-8; time: dt 0.0625, 0.03125, 0.015625 at 257 nodes); branch is `zero`, `near_zero` or `resolved` (relative error); rate1/rate2 are the tool's observed rates between successive levels (not fitted orders). Species S1. Every finding is `inconclusive` because the tool has no arithmetic/quadrature bounds (see Floors and uncertainty).


### axis=space status=inconclusive

| observable | t | role | err L1 | err L2 | err L3 | branch | rate1 | rate2 | status | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| balance | 0.25 | informative | 3.692e-04 | 1.818e-04 | 1.895e-04 | near_zero | 1.022 | -0.060 | inconclusive | nearzero_reference_scale,nonmonotonicity |
| balance | 0.5 | informative | 7.617e-04 | 3.776e-04 | 3.835e-04 | near_zero | 1.012 | -0.022 | inconclusive | nearzero_reference_scale,nonmonotonicity |
| balance | 1.0 | informative | 1.634e-03 | 8.095e-04 | 7.838e-04 | near_zero | 1.013 | 0.047 | inconclusive | control_contamination,nearzero_reference_scale |
| boundary.cumulative.0 | 0.25 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | resolved | - | - | inconclusive | control_contamination,plateau_or_uncertainty_overlap,reference_floor,zero_error |
| boundary.cumulative.0 | 0.5 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | resolved | - | - | inconclusive | control_contamination,plateau_or_uncertainty_overlap,reference_floor,zero_error |
| boundary.cumulative.0 | 1.0 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | resolved | - | - | inconclusive | control_contamination,plateau_or_uncertainty_overlap,reference_floor,zero_error |
| boundary.cumulative.1 | 0.25 | informative | 9.398e-05 | 4.500e-05 | 2.589e-05 | resolved | 1.062 | 0.798 | inconclusive |  |
| boundary.cumulative.1 | 0.5 | informative | 2.569e-04 | 1.248e-04 | 7.654e-05 | resolved | 1.042 | 0.705 | inconclusive |  |
| boundary.cumulative.1 | 1.0 | informative | 7.099e-04 | 3.477e-04 | 2.289e-04 | resolved | 1.030 | 0.603 | inconclusive |  |
| concentration.endpoint_linf | 0.25 | informative | 1.261e-03 | 6.138e-04 | 3.739e-04 | resolved | 1.038 | 0.715 | inconclusive |  |
| concentration.endpoint_linf | 0.5 | informative | 1.744e-03 | 8.564e-04 | 5.605e-04 | resolved | 1.026 | 0.612 | inconclusive |  |
| concentration.endpoint_linf | 1.0 | informative | 2.427e-03 | 1.196e-03 | 8.407e-04 | resolved | 1.021 | 0.509 | inconclusive |  |
| concentration.interior_linf | 0.25 | informative | 1.063e-03 | 5.640e-04 | 3.613e-04 | resolved | 0.915 | 0.642 | inconclusive |  |
| concentration.interior_linf | 0.5 | informative | 1.526e-03 | 8.011e-04 | 5.466e-04 | resolved | 0.929 | 0.551 | inconclusive |  |
| concentration.interior_linf | 1.0 | informative | 2.174e-03 | 1.132e-03 | 8.247e-04 | resolved | 0.941 | 0.457 | inconclusive |  |
| concentration.l1 | 0.25 | informative | 2.049e-04 | 1.022e-04 | 1.229e-04 | resolved | 1.004 | -0.266 | inconclusive | nonmonotonicity |
| concentration.l1 | 0.5 | informative | 3.658e-04 | 1.838e-04 | 2.246e-04 | resolved | 0.993 | -0.289 | inconclusive | nonmonotonicity |
| concentration.l1 | 1.0 | informative | 6.351e-04 | 3.185e-04 | 3.848e-04 | resolved | 0.995 | -0.273 | inconclusive | nonmonotonicity |
| concentration.l2 | 0.25 | informative | 3.709e-04 | 1.800e-04 | 1.425e-04 | resolved | 1.043 | 0.336 | inconclusive | control_contamination |
| concentration.l2 | 0.5 | informative | 5.808e-04 | 2.850e-04 | 2.503e-04 | resolved | 1.027 | 0.188 | inconclusive | control_contamination |
| concentration.l2 | 1.0 | informative | 8.986e-04 | 4.435e-04 | 4.204e-04 | resolved | 1.019 | 0.077 | inconclusive | control_contamination |
| concentration.linf | 0.25 | informative | 1.261e-03 | 6.138e-04 | 3.739e-04 | resolved | 1.038 | 0.715 | inconclusive |  |
| concentration.linf | 0.5 | informative | 1.744e-03 | 8.564e-04 | 5.605e-04 | resolved | 1.026 | 0.612 | inconclusive |  |
| concentration.linf | 1.0 | informative | 2.427e-03 | 1.196e-03 | 8.407e-04 | resolved | 1.021 | 0.509 | inconclusive |  |
| inventory.dissolved | 0.25 | informative | 1.964e-04 | 9.291e-05 | 1.070e-04 | resolved | 1.080 | -0.203 | inconclusive | nonmonotonicity |
| inventory.dissolved | 0.5 | informative | 3.356e-04 | 1.633e-04 | 1.945e-04 | resolved | 1.039 | -0.252 | inconclusive | nonmonotonicity |
| inventory.dissolved | 1.0 | informative | 5.685e-04 | 2.795e-04 | 3.323e-04 | resolved | 1.024 | -0.250 | inconclusive | nonmonotonicity |
| inventory.equilibrium_sorbed | 0.25 | informative | 9.819e-05 | 4.646e-05 | 5.348e-05 | resolved | 1.080 | -0.203 | inconclusive | nonmonotonicity |
| inventory.equilibrium_sorbed | 0.5 | informative | 1.678e-04 | 8.167e-05 | 9.723e-05 | resolved | 1.039 | -0.252 | inconclusive | nonmonotonicity |
| inventory.equilibrium_sorbed | 1.0 | informative | 2.843e-04 | 1.398e-04 | 1.662e-04 | resolved | 1.024 | -0.250 | inconclusive | nonmonotonicity |
| inventory.total | 0.25 | informative | 2.946e-04 | 1.394e-04 | 1.604e-04 | resolved | 1.080 | -0.203 | inconclusive | nonmonotonicity |
| inventory.total | 0.5 | informative | 5.034e-04 | 2.450e-04 | 2.917e-04 | resolved | 1.039 | -0.252 | inconclusive | nonmonotonicity |
| inventory.total | 1.0 | informative | 8.528e-04 | 4.193e-04 | 4.985e-04 | resolved | 1.024 | -0.250 | inconclusive | nonmonotonicity |
| reaction.loss | 0.25 | informative | 9.349e-06 | 4.651e-06 | 4.996e-06 | resolved | 1.007 | -0.103 | inconclusive | nonmonotonicity |
| reaction.loss | 0.5 | informative | 3.009e-05 | 1.500e-05 | 1.706e-05 | resolved | 1.004 | -0.186 | inconclusive | nonmonotonicity |
| reaction.loss | 1.0 | informative | 9.958e-05 | 4.961e-05 | 5.817e-05 | resolved | 1.005 | -0.230 | inconclusive | nonmonotonicity |
| reaction.production | 0.25 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | zero | - | - | inconclusive | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | 0.5 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | zero | - | - | inconclusive | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | 1.0 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | zero | - | - | inconclusive | plateau_or_uncertainty_overlap,zero_error |

### axis=time status=inconclusive

| observable | t | role | err L1 | err L2 | err L3 | branch | rate1 | rate2 | status | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| balance | 0.25 | informative | 3.325e-04 | 3.985e-04 | 6.732e-05 | near_zero | -0.261 | 2.565 | inconclusive | control_contamination,nearzero_reference_scale,nonmonotonicity |
| balance | 0.5 | informative | 6.167e-04 | 7.714e-04 | 1.267e-04 | near_zero | -0.323 | 2.606 | inconclusive | control_contamination,nearzero_reference_scale,nonmonotonicity |
| balance | 1.0 | informative | 1.066e-03 | 1.454e-03 | 2.260e-04 | near_zero | -0.449 | 2.686 | inconclusive | control_contamination,nearzero_reference_scale,nonmonotonicity |
| boundary.cumulative.0 | 0.25 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | resolved | - | - | inconclusive | control_contamination,plateau_or_uncertainty_overlap,reference_floor,zero_error |
| boundary.cumulative.0 | 0.5 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | resolved | - | - | inconclusive | control_contamination,plateau_or_uncertainty_overlap,reference_floor,zero_error |
| boundary.cumulative.0 | 1.0 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | resolved | - | - | inconclusive | control_contamination,plateau_or_uncertainty_overlap,reference_floor,zero_error |
| boundary.cumulative.1 | 0.25 | informative | 2.391e-05 | 2.521e-05 | 1.194e-05 | resolved | -0.076 | 1.078 | inconclusive | control_contamination,nonmonotonicity |
| boundary.cumulative.1 | 0.5 | informative | 7.475e-05 | 8.406e-05 | 3.348e-05 | resolved | -0.169 | 1.328 | inconclusive | control_contamination,nonmonotonicity |
| boundary.cumulative.1 | 1.0 | informative | 2.192e-04 | 2.740e-04 | 8.946e-05 | resolved | -0.321 | 1.615 | inconclusive | control_contamination,nonmonotonicity |
| concentration.endpoint_linf | 0.25 | informative | 3.880e-04 | 4.052e-04 | 1.669e-04 | resolved | -0.063 | 1.280 | inconclusive | control_contamination,nonmonotonicity |
| concentration.endpoint_linf | 0.5 | informative | 5.467e-04 | 6.722e-04 | 2.264e-04 | resolved | -0.298 | 1.570 | inconclusive | control_contamination,nonmonotonicity |
| concentration.endpoint_linf | 1.0 | informative | 7.449e-04 | 1.061e-03 | 2.839e-04 | resolved | -0.511 | 1.903 | inconclusive | control_contamination,nonmonotonicity |
| concentration.interior_linf | 0.25 | informative | 3.832e-04 | 4.020e-04 | 1.637e-04 | resolved | -0.069 | 1.296 | inconclusive | control_contamination,nonmonotonicity |
| concentration.interior_linf | 0.5 | informative | 5.432e-04 | 6.688e-04 | 2.229e-04 | resolved | -0.300 | 1.585 | inconclusive | control_contamination,nonmonotonicity |
| concentration.interior_linf | 1.0 | informative | 7.409e-04 | 1.057e-03 | 2.799e-04 | resolved | -0.513 | 1.918 | inconclusive | control_contamination,nonmonotonicity |
| concentration.l1 | 0.25 | informative | 2.324e-04 | 2.814e-04 | 4.117e-05 | resolved | -0.276 | 2.773 | inconclusive | control_contamination,nonmonotonicity |
| concentration.l1 | 0.5 | informative | 3.968e-04 | 5.044e-04 | 6.718e-05 | resolved | -0.346 | 2.909 | inconclusive | control_contamination,nonmonotonicity |
| concentration.l1 | 1.0 | informative | 5.816e-04 | 8.184e-04 | 9.172e-05 | resolved | -0.493 | 3.157 | inconclusive | control_contamination,nonmonotonicity |
| concentration.l2 | 0.25 | informative | 2.395e-04 | 2.841e-04 | 5.608e-05 | resolved | -0.246 | 2.341 | inconclusive | control_contamination,nonmonotonicity |
| concentration.l2 | 0.5 | informative | 4.019e-04 | 5.074e-04 | 8.404e-05 | resolved | -0.336 | 2.594 | inconclusive | control_contamination,nonmonotonicity |
| concentration.l2 | 1.0 | informative | 5.848e-04 | 8.242e-04 | 1.105e-04 | resolved | -0.495 | 2.899 | inconclusive | control_contamination,nonmonotonicity |
| concentration.linf | 0.25 | informative | 3.880e-04 | 4.052e-04 | 1.669e-04 | resolved | -0.063 | 1.280 | inconclusive | control_contamination,nonmonotonicity |
| concentration.linf | 0.5 | informative | 5.467e-04 | 6.722e-04 | 2.264e-04 | resolved | -0.298 | 1.570 | inconclusive | control_contamination,nonmonotonicity |
| concentration.linf | 1.0 | informative | 7.449e-04 | 1.061e-03 | 2.839e-04 | resolved | -0.511 | 1.903 | inconclusive | control_contamination,nonmonotonicity |
| inventory.dissolved | 0.25 | informative | 2.002e-04 | 2.423e-04 | 3.572e-05 | resolved | -0.275 | 2.762 | inconclusive | control_contamination,nonmonotonicity |
| inventory.dissolved | 0.5 | informative | 3.416e-04 | 4.342e-04 | 5.811e-05 | resolved | -0.346 | 2.901 | inconclusive | control_contamination,nonmonotonicity |
| inventory.dissolved | 1.0 | informative | 5.005e-04 | 7.041e-04 | 7.923e-05 | resolved | -0.492 | 3.152 | inconclusive | control_contamination,nonmonotonicity |
| inventory.equilibrium_sorbed | 0.25 | informative | 1.001e-04 | 1.212e-04 | 1.786e-05 | resolved | -0.275 | 2.762 | inconclusive | control_contamination,nonmonotonicity |
| inventory.equilibrium_sorbed | 0.5 | informative | 1.708e-04 | 2.171e-04 | 2.905e-05 | resolved | -0.346 | 2.901 | inconclusive | control_contamination,nonmonotonicity |
| inventory.equilibrium_sorbed | 1.0 | informative | 2.503e-04 | 3.521e-04 | 3.961e-05 | resolved | -0.492 | 3.152 | inconclusive | control_contamination,nonmonotonicity |
| inventory.total | 0.25 | informative | 3.003e-04 | 3.635e-04 | 5.358e-05 | resolved | -0.275 | 2.762 | inconclusive | control_contamination,nonmonotonicity |
| inventory.total | 0.5 | informative | 5.124e-04 | 6.512e-04 | 8.716e-05 | resolved | -0.346 | 2.901 | inconclusive | control_contamination,nonmonotonicity |
| inventory.total | 1.0 | informative | 7.508e-04 | 1.056e-03 | 1.188e-04 | resolved | -0.492 | 3.152 | inconclusive | control_contamination,nonmonotonicity |
| reaction.loss | 0.25 | informative | 8.712e-06 | 1.023e-05 | 2.246e-06 | resolved | -0.232 | 2.187 | inconclusive | control_contamination,empirical_quadrature_floor,nonmonotonicity |
| reaction.loss | 0.5 | informative | 3.000e-05 | 3.653e-05 | 6.499e-06 | resolved | -0.284 | 2.491 | inconclusive | control_contamination,nonmonotonicity |
| reaction.loss | 1.0 | informative | 9.593e-05 | 1.246e-04 | 1.817e-05 | resolved | -0.377 | 2.777 | inconclusive | control_contamination,nonmonotonicity |
| reaction.production | 0.25 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | zero | - | - | inconclusive | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | 0.5 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | zero | - | - | inconclusive | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | 1.0 | informative | 0.000e+00 | 0.000e+00 | 0.000e+00 | zero | - | - | inconclusive | plateau_or_uncertainty_overlap,zero_error |

Quadrature split (quadrature inventory against exact continuum mass vs same-mesh-weight truth): not available. The tool reports `quadrature_sensitivity: null` and `quadrature_bound_missing` for every finding, so no split was computed by hand.

## Refinement and fitted orders

No order is fitted: the tool classifies all 175 informative findings per axis as `inconclusive` (0 achieved, 0 failed), and the spec's conditions for a fit (resolved errors with a small control on the other axis) are not met.
- Space axis: errors fall with refinement; `concentration.linf` at t = 1 is 2.43e-3, 1.20e-3, 8.41e-4, observed rates 1.02 then 0.51. The first rate is about 1 (spec: at most first order in space). The second drops because the dt = 2^-8 time error is no longer small next to the 129-node space error; the space control (129 nodes at half dt) has linf error 1.41e-4 at t = 1, so temporal error contaminates the finest level. `inventory.total` is non-monotone (8.5e-4, 4.2e-4, 5.0e-4), flagged `nonmonotonicity`.
- Time axis: not monotone in this window. `concentration.linf` at t = 1 is 7.4e-4, 1.06e-3, 2.8e-4 (rates -0.51, +1.90). The 513-node control at the same dt 0.015625 has error 1.10e-3, so the 257-node spatial error (about 1e-3) is comparable to the temporal error and the two partially cancel; every time-axis field carries `control_contamination`. Temporal error is not isolated.
- t = 0 rows: `inventory.*` rate about 2 (initial-condition quadrature) and round-off level errors for L1/L2.

## Floors and uncertainty

- Smallest nonzero errors are 1e-5 to 1e-4, far above the f32 resolution (about 1e-7 relative); no f32 floor or plateau is reached, so none explains the non-monotone time axis.
- t = 0 errors are 1e-16 to 2e-16 (`reference_floor`); `boundary.cumulative.0` and `reaction.production` errors are exactly 0 (`zero` branch, no relative error).
- Analytical reference uncertainty is about 5e-16 and not limiting.
- Classifier blocker: every finding lists `arithmetic_bound_missing` and `quadrature_bound_missing`. `run_solute_convergence.py` accepts `--bounds-source` and `--bounds-review` only together. Required: (1) a bounds file with `schema_version: solute-convergence-bounds-v1`, `cases_sha256`, `producer_sha256` (must equal the current tool hashes) and `bounds_by_launch` (launch id -> list of entries with keys `species observable time arithmetic coordinate quadrature evidence`; times from the row schedule; `evidence` non-empty text; numeric bounds nonnegative or null); (2) a review file with `schema_version: solute-convergence-bounds-review-v1`, `cases_sha256`, `producer_sha256`, `bounds_path`, `bounds_sha256`, a non-empty `authorization` string and an `evidence_sha256` map of file path to SHA-256 that must still match. Both bind to the exact tool hashes, so any `tools/` change invalidates them. The authorization must come from the root reviewer/orchestrator; the worker did not produce either file.

## Observations against the spec's expected behavior

- Physically sound after the 22.3.8 fix: at t = 1 the profile maximum is 0.840 (inlet node) and the minimum 0.664 (outlet), all below c_in = 1; there is no constant offset. Values at all levels agree to 3 digits (space_33 0.8402/0.6653, space_129 0.8404/0.6637).
- Spatial convergence near first order is visible at coarse levels (rate 1.0-1.1 for `concentration.linf/l2`, `boundary.cumulative.1`, `inventory.total`), consistent with the spec's "at most first order in space".
- The spec's Fortran epsi = 0.5 note (global second order not guaranteed) is consistent with first-order spatial behavior; the observed time-axis rates (up to 3.2 on `inventory.total` between the two finest levels) come from cancellation with spatial error and must not be read as an order.
- Prior FV/BE evidence (`SOLUTE_CONVERGENCE_EVIDENCE.md`) was not used for numbers.

## Inputs for threshold proposal

No thresholds are proposed. For Wave 5: (1) finest-level errors at t = 1 are 8.4e-4 (`concentration.linf`, space) and 2.8e-4 (time), `inventory.total` 5.0e-4 / 1.2e-4; (2) bounds files are needed before any finding can leave `inconclusive`; (3) the time axis needs a spatial control finer than 513 nodes (or a bounded spatial term) to isolate temporal error; (4) reference uncertainty (about 5e-16) is negligible; (5) errors are about 1e-3 levels at the study resolutions.

## Spot-check list

- `concentration.linf` t = 1.0 space levels 2.427e-3, 1.196e-3, 8.407e-4: `.development/22-convergence/robin-v2/batch-d8607f2416c84a2385ba6d3f521363c1/classification.json`, group axis=space.
- Profile extremes at t = 1: `results/convergence-scalar_robin-space_129-rfc-baseline.json`, `payload.metrics.raw.profiles[-1].concentration`.
- Batch IDs and `--verify-only` exit codes: "Lifecycle and verify status".

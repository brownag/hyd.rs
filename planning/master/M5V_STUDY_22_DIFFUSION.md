# M5-V study 22.4.2: diffusion_cosine, FortranCompatible backend

Status: run and verified on the fixed code; no thresholds proposed. M5-V remains open.

Superseded first pass: an earlier run on HEAD 66a26ed (before the 22.3.8 chain-production and flow-direction fixes) is kept as superseded scratch (`.development/22.4.2/superseded/firstpass-roots/`, with `firstpass-filelist.txt` and `ledger-sha256.txt`). Its one failed attempt came from an exported `CARGO_TARGET_DIR`. The re-run is numerically identical to the first pass at the printed digits (this case has q=0 and no chain), so the fixes did not change `diffusion_cosine`; only the harness binary SHA changed. First-pass ledger SHA-256: benchmark `0a701958…`, convergence `37441e3c…`. The phase roots were moved aside, not edited, so the per-root launch ledger started empty.

## Run identity

- Worktree HEAD d2a8bef (includes the 22.3.8 chain-production and flow-direction fixes) (branch orchestration/22.4.2), `reference/solute_verification/CASES_V2.json` SHA-256 `644c4845956d48cc27e0f45387a06317dbbf9aa671667617fc6487e4b3ad8a3d`; case SHA `3ebf84b5e214c3cfaa1aa12cb7965dcd303ef9306123d49720087c176b9c1fd0`.
- Backend `fortran-compatible` (rows carry `solver: rust`, `backend: fortran-compatible`, f32 arithmetic inside; `precision: f64` is a schema label).
- Compiler: rustc 1.96.0-nightly (c75612477 2026-04-07). Harness executable SHA-256 `4e591448acaf1f7224a51097c3a1e250b258fae414235676b16fd5e15703209c` (from run manifests). This task uses the harness, not the cached `solute_compat_deck_runner` (none was present under the task target dir).
- Activation files (derived from `lifecycle.validate_activation` and the 13.3.1/13.4.1 examples; scratch `.development/22.4.2/`): `benchmark-activation.json` `53985137856c438bf5f75efee8cbf7b168f714cc1fdd45d328814bf7cd90103e`; `convergence-activation.json` `a6b9702f2783db19d391016553bf428df678db125578089ff022f31fa9b7f766`.
- Commands (cwd worktree, temporary untracked `target` symlink to the 22.4.2 target dir, `CARGO_TARGET_DIR` unset):
  - `python3 tools/check_solute_benchmarks.py --cases reference/solute_verification/CASES_V2.json --output-root .development/22-benchmark/diff-v2 --execute --stage benchmark --activation-file .development/22.4.2/benchmark-activation.json --allow-reference-work --launch-id benchmark-diffusion_cosine-component-rfc-baseline`
  - `python3 tools/run_solute_convergence.py --cases ... --output-root .development/22-convergence/diff-v2 --execute --activation-file .development/22.4.2/convergence-activation.json --allow-reference-work` with the 8 `--launch-id convergence-diffusion_cosine-{space_17,space_33,space_65,space_control,time_0.0625,time_0.03125,time_0.015625,time_control}-rfc-baseline` (47 s wall).
- Bridge gate: v2 receipt `bridge-gate.json` (not_applicable_v2); no `--bridge-report`.
- Reference: analytical, built into the per-root caches (`--allow-reference-work`; no Fortran output). Reference identity per cache entry, `reference.json` SHA-256:
  - space_65, space_control, benchmark (id `979d1675…`): `fabd2190fd8e994dcacd4eb3c781c0e25f383b08c45874bc46f68de4ea08f969`
  - space_17 (`7600b703…`): `6f4a5a393d718262ff69c42707cf4d26ba89b2d3edce40dbf052fb29ae9735bf`
  - space_33 (`e3e78d08…`): `1318b025d4754d8f1df50a5ff8923b8011e70bb6aeef3dced8827c6018be8343`
  - time_0.0625/0.03125/0.015625 (`46ae67a0…`): `9f5418cd0496c7462c8cad32ba0ba9323dcc1bad680604a912ebb042a25fbfa9`
  - time_control (`b114b4e9…`): `2eb4b7ceab70f024888dd92696537bae1adb4401414c7eaffda1d13ff10b74f9`
- Artifact SHA-256: benchmark result `5c6b653cc3d07400f4f25c9e89a6a663bb00e73a6b649e9aa4e6e65d60276eca`, benchmark summary `64fc8c7c8f7b05b3ecd2186d4a8a3d3f8302594df71b493511f93b8b8a93f260`; convergence summary `9f881617058a2666dd8dfdc48e418755ad547f80672e8575399cd3b78f0c65f7`, classification `f73aa2112034b6b180858bc4f48275d319050430ab0680e9eb83f7e084889b65`.
- The holdout row `holdout-diffusion_cosine-holdout-rfc-baseline` was not run and `--holdout` was never passed.

## Lifecycle and verify status

| Batch | Rows | State | `--verify-only` exit |
|---|---|---|---|
| `22-benchmark/diff-v2/batch-620d5ab3d85d490983522bf5e55999b5` | benchmark (1) | evidence_complete | 0 (`--batch` form) |
| `22-convergence/diff-v2/batch-6ca52a89c6fb48ec89c51c90b92e9cf5` | 8 study/control | evidence_complete | 0 (status evidence_verified) |

- No row failed in this pass; no retry used. Benchmark verified with `--batch`; the convergence root verified without it (one batch).
- Convergence classification: both axes `inconclusive` (175 informative findings, 0 achieved, 0 failed); control assessments list `arithmetic_bound_missing` and `quadrature_bound_missing`; `bounds_source` was null (not supplied). Unmet: "accuracy thresholds unassigned", "M5-V open".

## Metrics table

Absolute errors against the analytical reference (species S1; t=0 rows omitted). Every norm row used branch `resolved` (scale = reference norm, 1.195 to 1.235 for linf). Boundary, reaction and balance errors are all `scale=0`, `branch=zero`, `relative=null`, absolute 0. At t=0 all norms are at most 1.1e-16.

Benchmark row (same 65-node grid and dt as space_65): identical to space_65 below (linf 1.763e-3 / 2.307e-3 / 2.893e-3 at t=0.25/0.5/1.0; l2 3.514e-4 / 5.427e-4 / 8.093e-4; temporal_l2 6.214e-4).

| row | t | linf | l2 | l1 | endpoint_linf | interior_linf | branch(linf) | scale(linf) | temporal_l2 |
|---|---|---|---|---|---|---|---|---|---|
| space_17 | 0.25 | 9.055e-03 | 2.026e-03 | 7.626e-04 | 9.055e-03 | 4.539e-03 | resolved | 1.235 | 3.196e-03 |
| space_17 | 0.5 | 1.116e-02 | 2.851e-03 | 1.281e-03 | 1.116e-02 | 6.913e-03 | resolved | 1.221 | 3.196e-03 |
| space_17 | 1.0 | 1.353e-02 | 4.050e-03 | 2.183e-03 | 1.353e-02 | 9.781e-03 | resolved | 1.195 | 3.196e-03 |
| space_33 | 0.25 | 3.873e-03 | 8.018e-04 | 2.915e-04 | 3.873e-03 | 2.741e-03 | resolved | 1.235 | 1.386e-03 |
| space_33 | 0.5 | 4.987e-03 | 1.211e-03 | 5.189e-04 | 4.987e-03 | 3.923e-03 | resolved | 1.221 | 1.386e-03 |
| space_33 | 1.0 | 6.236e-03 | 1.801e-03 | 9.226e-04 | 6.236e-03 | 5.295e-03 | resolved | 1.195 | 1.386e-03 |
| space_65 | 0.25 | 1.763e-03 | 3.514e-04 | 1.350e-04 | 1.763e-03 | 1.480e-03 | resolved | 1.235 | 6.214e-04 |
| space_65 | 0.5 | 2.307e-03 | 5.427e-04 | 2.418e-04 | 2.307e-03 | 2.041e-03 | resolved | 1.221 | 6.214e-04 |
| space_65 | 1.0 | 2.893e-03 | 8.093e-04 | 4.173e-04 | 2.893e-03 | 2.657e-03 | resolved | 1.195 | 6.214e-04 |
| space_control | 0.25 | 1.762e-03 | 3.503e-04 | 1.317e-04 | 1.762e-03 | 1.479e-03 | resolved | 1.235 | 6.074e-04 |
| space_control | 0.5 | 2.273e-03 | 5.335e-04 | 2.319e-04 | 2.273e-03 | 2.007e-03 | resolved | 1.221 | 6.074e-04 |
| space_control | 1.0 | 2.817e-03 | 7.864e-04 | 3.955e-04 | 2.817e-03 | 2.581e-03 | resolved | 1.195 | 6.074e-04 |
| time_0.0625 | 0.25 | 2.995e-04 | 8.023e-05 | 6.575e-05 | 2.995e-04 | 2.818e-04 | resolved | 1.235 | 1.567e-04 |
| time_0.0625 | 0.5 | 3.557e-04 | 1.321e-04 | 1.111e-04 | 3.557e-04 | 3.391e-04 | resolved | 1.221 | 1.567e-04 |
| time_0.0625 | 1.0 | 3.778e-04 | 2.103e-04 | 1.725e-04 | 3.778e-04 | 3.778e-04 | resolved | 1.195 | 1.567e-04 |
| time_0.03125 | 0.25 | 3.653e-04 | 7.386e-05 | 4.577e-05 | 3.653e-04 | 3.476e-04 | resolved | 1.235 | 1.347e-04 |
| time_0.03125 | 0.5 | 4.716e-04 | 1.161e-04 | 7.884e-05 | 4.716e-04 | 4.550e-04 | resolved | 1.221 | 1.347e-04 |
| time_0.03125 | 1.0 | 5.753e-04 | 1.774e-04 | 1.310e-04 | 5.753e-04 | 5.605e-04 | resolved | 1.195 | 1.347e-04 |
| time_0.015625 | 0.25 | 4.034e-04 | 8.076e-05 | 4.137e-05 | 4.034e-04 | 3.857e-04 | resolved | 1.235 | 1.553e-04 |
| time_0.015625 | 0.5 | 5.402e-04 | 1.307e-04 | 7.982e-05 | 5.402e-04 | 5.235e-04 | resolved | 1.221 | 1.553e-04 |
| time_0.015625 | 1.0 | 6.961e-04 | 2.084e-04 | 1.524e-04 | 6.961e-04 | 6.814e-04 | resolved | 1.195 | 1.553e-04 |
| time_control | 0.25 | 1.924e-04 | 4.155e-05 | 2.669e-05 | 1.924e-04 | 1.880e-04 | resolved | 1.235 | 8.431e-05 |
| time_control | 0.5 | 2.554e-04 | 6.914e-05 | 5.074e-05 | 2.554e-04 | 2.512e-04 | resolved | 1.221 | 8.431e-05 |
| time_control | 1.0 | 3.265e-04 | 1.153e-04 | 9.800e-05 | 3.265e-04 | 3.228e-04 | resolved | 1.195 | 8.431e-05 |

| row | t | inv_total_abs_err | inv_mesh_quadrature | inv_mesh_error | boundary cum err branch | reaction branch | balance branch |
|---|---|---|---|---|---|---|---|
| space_17 | 0.25 | 1.312e-03 | 0.000e+00 | -1.312e-03 | zero | zero | zero |
| space_17 | 0.5 | 2.203e-03 | 0.000e+00 | -2.203e-03 | zero | zero | zero |
| space_17 | 1.0 | 3.755e-03 | 0.000e+00 | -3.755e-03 | zero | zero | zero |
| space_33 | 0.25 | 4.948e-04 | 0.000e+00 | -4.948e-04 | zero | zero | zero |
| space_33 | 0.5 | 8.908e-04 | 0.000e+00 | -8.908e-04 | zero | zero | zero |
| space_33 | 1.0 | 1.587e-03 | 0.000e+00 | -1.587e-03 | zero | zero | zero |
| space_65 | 0.25 | 1.866e-04 | 0.000e+00 | -1.866e-04 | zero | zero | zero |
| space_65 | 0.5 | 3.437e-04 | 0.000e+00 | -3.437e-04 | zero | zero | zero |
| space_65 | 1.0 | 6.153e-04 | 0.000e+00 | -6.153e-04 | zero | zero | zero |
| space_control | 0.25 | 1.884e-04 | 0.000e+00 | -1.884e-04 | zero | zero | zero |
| space_control | 0.5 | 3.444e-04 | 0.000e+00 | -3.444e-04 | zero | zero | zero |
| space_control | 1.0 | 6.109e-04 | 0.000e+00 | -6.109e-04 | zero | zero | zero |
| time_0.0625 | 0.25 | 5.599e-05 | 0.000e+00 | -5.599e-05 | zero | zero | zero |
| time_0.0625 | 0.5 | 1.082e-04 | 0.000e+00 | -1.082e-04 | zero | zero | zero |
| time_0.0625 | 1.0 | 2.039e-04 | 0.000e+00 | -2.039e-04 | zero | zero | zero |
| time_0.03125 | 0.25 | 6.099e-05 | 0.000e+00 | -6.099e-05 | zero | zero | zero |
| time_0.03125 | 0.5 | 1.181e-04 | 0.000e+00 | -1.181e-04 | zero | zero | zero |
| time_0.03125 | 1.0 | 2.236e-04 | 0.000e+00 | -2.236e-04 | zero | zero | zero |
| time_0.015625 | 0.25 | 7.060e-05 | 0.000e+00 | -7.060e-05 | zero | zero | zero |
| time_0.015625 | 0.5 | 1.373e-04 | 0.000e+00 | -1.373e-04 | zero | zero | zero |
| time_0.015625 | 1.0 | 2.620e-04 | 0.000e+00 | -2.620e-04 | zero | zero | zero |
| time_control | 0.25 | 4.451e-05 | 0.000e+00 | -4.451e-05 | zero | zero | zero |
| time_control | 0.5 | 8.727e-05 | 0.000e+00 | -8.727e-05 | zero | zero | zero |
| time_control | 1.0 | 1.686e-04 | 0.000e+00 | -1.686e-04 | zero | zero | zero |

Quadrature split: `inv_mesh_quadrature` (numerical inventory minus analytic truth integrated with the same mesh weights) is exactly 0 at the reported times, and `inv_mesh_error` equals the total inventory error. So the negative inventory error (loss against exact aL = 1.72) is not a quadrature artefact; it is solution mass loss.

## Refinement and fitted orders

No order is fitted: the tool classification is inconclusive and no bounds were supplied. Descriptive ratios only, linf at t=1.0:
- Space (dt 2.44e-4): 17 -> 33 -> 65 nodes gives 1.353e-2, 6.236e-3, 2.893e-3 (ratios 2.17, 2.16; about 1.1 in log2). Same pattern at t=0.25 (2.34, 2.20) and t=0.5 (2.24, 2.16). l2 ratios 2.25, 2.23 at t=1.0; l1 2.37, 2.21; inventory error 3.755e-3, 1.587e-3, 6.153e-4 (2.37, 2.58).
- Space control (65 nodes, half dt) differs from space_65 by 2.6 percent in linf at t=1.0 (2.817e-3 vs 2.893e-3), so the temporal contribution on this axis is small but not zero.
- Time (257 nodes): linf at t=1.0 is 3.778e-4, 5.753e-4, 6.961e-4 for dt 0.0625, 0.03125, 0.015625. Error grows as dt shrinks (non-monotonic with respect to refinement), and `resolved_last_decrease` is negative in the findings. Re-examined with the 513-node control, it gives 3.265e-4 at dt 0.015625, below every 257-node row and about half the 257-node value at the same dt: spatial error at 257 nodes is the same size as or larger than the temporal signal, so the time axis is not isolated. A temporal order cannot be fitted or claimed.
- Errors concentrate at the endpoints (endpoint_linf = linf in every row).

## Floors and uncertainty

- Smallest errors seen in studies are 1e-4 to 1e-3; f32 resolution (about 1e-7 relative) is 3 to 4 orders below, so no plateau at the f32 floor is observed on the space axis. The time axis behaviour above is not an f32 floor effect by magnitude, but was not isolated further.
- Reference uncertainties are 1e-16 to 1e-17; rounding uncertainty reported 0; coordinate uncertainty none (exact request mesh).
- Missing: arithmetic and quadrature bounds (hence inconclusive).
- Bounds source not produced (out of scope). The classifier (`solute_convergence_evidence.py:254-330`; CLI `--bounds-source` and `--bounds-review`, required together) needs a bounds file with `schema_version solute-convergence-bounds-v1`, matching `cases_sha256` and `producer_sha256`, and `bounds_by_launch` mapping each launch id to entries with keys `species observable time arithmetic coordinate quadrature evidence` (non-empty independent evidence text). It also needs a review file `solute-convergence-bounds-review-v1` with `cases_sha256 producer_sha256 bounds_path bounds_sha256 authorization evidence_sha256` (non-empty authorization reference and evidence file hashes). The review is root-authorized, so the orchestrator or user must authorize it; bounds must come from independent derivation, never from solver output (ADR 0004, 0009).

## Observations against the spec's expected behavior

- Spec (`SOLUTE_VERIFICATION_SPEC.md:135-146`): exact finite-domain Neumann cosine solution; inventory exactly aL = 1.72 (0.86 dissolved plus 0.86 sorbed), boundary and reaction zero. Boundary, reaction, balance: zero as expected. Inventory: not conserved; this is a feature of the Fortran kernel's end-row c1=c2 treatment (`SOLUTE.FOR:803-827`, confirmed by the coordinator), not a public-path defect (error -3.8e-3 at 17 nodes, -6.2e-4 at 65 nodes, t=1.0), shrinking about 2.4x per halving of dx. The spec does not state an order for this case (the methodology section says it does not prescribe one).
- Observed spatial convergence ratio about 2.2 per halving, below the 4 expected of a second-order scheme; errors peak at the endpoints. This is reported as observed, not as a verdict. The 22.3.5 report noted a hand-run FortranCompatible vs FV/BE maximum absolute difference of 0.0499 on this case; that is an observation only and was not re-measured here.
- FV/BE prior evidence (`SOLUTE_CONVERGENCE_EVIDENCE.md:19-22`) classified this case inconclusive for the same reason (null bounds); not numerically compared in this task.

## Inputs for threshold proposal

No thresholds are proposed. For 22.5.1:
- Candidate quantities: linf/l2/l1/endpoint/interior per time; inventory error; temporal_l2 (space 3.2e-3, 1.39e-3, 6.2e-4; time 1.6e-4, 1.3e-4, 1.6e-4; benchmark 6.2e-4).
- Space axis has 3 resolved levels with a small control (2.6 percent), usable for ratio discussion; time axis is spatially limited and non-monotonic at 257 nodes.
- Bounds were not supplied (`--bounds-source`), so classification stays inconclusive until they are.
- Table scratch: `.development/22.4.2/table.py`, `tables-v2.md`.

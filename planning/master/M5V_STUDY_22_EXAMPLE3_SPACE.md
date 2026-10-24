# M5-V study 22.4.5: Example 3 space axis completed (space_control at 1001 nodes, dt .0125)

Study only: no scientific code changed, no holdout row run, no thresholds proposed. Completes the space axis left open in `M5V_STUDY_22_EXAMPLE3.md`.

## Run identity

- Branch `orchestration/22.4.5` (HEAD `5e476b1` plus this task's edits); backend `fortran-compatible`, `solver: rust`, f32 arithmetic. Harness build recorded in the run manifest (`build/solute-verification-harness` SHA-256 `cb8e8946…` prefix; test binary rebuilt from the unchanged `crates/`, `solute_verification_harness-ec7dfff3fd1b6e1e`, SHA-256 `21a4ca0d…`).
- **New `CASES_V2.json` SHA-256 `c236eb7cd42dee691a36787625865457df62c0bb4a72cb0388a786cbe1e1b43b`** (was `644c4845956d48cc27e0f45387a06317dbbf9aa671667617fc6487e4b3ad8a3d`). The old SHA is pinned only in docs (STATE_22, earlier study reports, task todos); no tool or test pins it. Those records stay historically correct and were not edited.
- Edited tool hashes: `tools/check_solute_verification_spec.py` `17afaf8e…`, `tools/solute_benchmark_lifecycle.py` `fcb9094e…`; `tools/solute_verification_metrics.py` unchanged (`609a18e0…`). `POLICY_HASHES` in `tools/solute_benchmark_bridge.py` pins only `crates/...` files and the protected baseline, none of which changed, so **no POLICY_HASHES re-pin is needed**.
- Fresh phase root `.development/22-convergence` (worktree), output root `example3-v2`; launch ledger SHA-256 `90161109…`, never edited; the consumed 22.4.4 batch was not resumed.
- Batch of the new attempt: `batch-df5e1531ffb047b39db83a099ff5333a` (one row: `convergence-example3-space_control-rfc-baseline`, run `convergence-example3-space_control-rfc-baseline-fa43a73bd073`, manifest SHA-256 `284dab38…`; `classification.json` file SHA-256 `132e1ed9…`; `summary.json` `98c88ab8…`). Activation: `.development/22.4.5/activation-space-control.json` (orchestrator approval 2026-10-24).
- Command (`CARGO_TARGET_DIR` unset, `--allow-reference-work` only): `run_solute_convergence.py --cases reference/solute_verification/CASES_V2.json --output-root .development/22-convergence/example3-v2 --execute --allow-reference-work --activation-file .development/22.4.5/activation-space-control.json --launch-id convergence-example3-space_control-rfc-baseline`. The Laplace reference for 1001 nodes (400-digit N=192/256/384 plus 650-digit precision stage) was recomputed: about 3 h 30 min wall, almost all of it the reference; the solver run itself took 76 s.

## Timeout deviation (recorded)

- Measured solver time, direct harness runs on a scratch copy of the 22.4.4 request (no ledger): 84.03, 84.20, 82.90 s (3 runs, exit 0). In the lifecycle the new attempt ran 2026-10-24T20:38:57Z to 20:40:13Z (75.9 s).
- Chosen per-row timeout: **180 s** (about 2.1x the slowest measurement), `limits.max_row_timeout_seconds: 300` as the cap. Default 60 s unchanged for every other row including holdouts. The checker (`V2_TIMEOUT_EXCEPTIONS`) allows exactly one id; 61 on any other row, 301, or a missing cap fail. The lifecycle timeout-event bound is now `<= 300`; the `start` event still has to equal the row's own `timeout_seconds`.
- Tests: `tools.test_check_solute_verification_spec` (new `test_v2_row_timeout_exception`), `tools.test_solute_benchmark_lifecycle` (new `test_ledger_timeout_event_bound_is_row_ceiling`), `tools.test_solute_verification_metrics`: 56 tests OK. `python3 tools/check_solute_verification_spec.py` valid on CASES_V2 and CASES. Wider run of `test_solute_benchmarks`, `test_solute_convergence_cli`, `test_solute_verification_reference`, `test_solute_benchmark_bridge`: 69 tests, 1 error, which is the missing worktree-local `.development/12.6.1/rust-observations.json` ("run cargo test --workspace first"), unrelated to this change.

## Lifecycle and verify status

| Batch | Content | `--verify-only --batch` |
|---|---|---|
| `batch-df5e1531…` (new) | space_control, run `complete`, `evidence_complete` | exit 0 (`evidence_verified`) |
| `batch-66f7e9ba…` (22.4.4, copied) | space 251/501/1001 complete, space_control timeout | exit 2: "batch recipe identity changed" (recipe is bound to the old CASES_V2 SHA and old tool hashes). Originally also exit 2 in 22.4.4 (incomplete batch) |
| `batch-0cf92fa6…` (22.4.4 time batch, copied then removed) | time rows | exit 2 "batch recipe identity changed" (same cause; original exit 0 under the old SHA). Not used in this report |

Failures recorded, nothing chased. The space-axis metrics of the old batch could not go through `--verify-only`/`classify` of the new tools: (1) the copied run manifests carry the 22.4.4 worktree `command`/`cwd`, so `Lifecycle.verify` refuses them ("Rust command/cwd changed"); (2) the 22.4.4 reference-cache identities differ from today's by exactly one input, the SHA of the edited `check_solute_verification_spec.py`, so the 251 and 501 references could not be replayed by identity without a roughly 3 h recompute each. Instead, the scratch scripts `.development/22.4.5/space_old.py` (run in an unedited archive of HEAD, `.development/22.4.5/oldtree`, against the old casebook and the copied 22.4.4 caches, which replay as `admitted`) and `space_analyze.py` produced the metrics records, and the classification library `analyze_convergence` (unchanged) combined them with the new control record. The trusted bindings for the three old levels were computed from the records themselves, so this is a replay of the documented analysis, not an authenticated filesystem receipt chain. The control result was replayed and matched the stored result.

## Metrics table

Space axis, dt .025 for the three levels (251/501/1001 nodes, dx = 1, .5, .25 relative), absolute error against the Laplace reference per species (NH4, NO2, NO3), `t` = 50/100/200 (history times 1..200 are in `.development/22.4.5/tables.md`, 6045 rows). "local p" is `log2(e_coarse/e_fine)` of the observed errors, descriptive only. "ctrl abs" is the 1001-node dt=.0125 control's error against the reference. Flags are the classification tool's.

| observable | sp | t | e(251) | e(501) | e(1001) | local p (251-501, 501-1001) | ctrl abs | branch | ref uncert | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| balance | NH4 | 50 | 1.91e-03 | 4.73e-04 | 1.81e-03 | 2.01, -1.94 | 7.93e-04 | near_zero | 2.02e-15 | nearzero_reference_scale,nonmonotonicity |
| balance | NH4 | 100 | 4.90e-03 | 3.24e-04 | 4.95e-03 | 3.92, -3.94 | 1.05e-03 | near_zero | 4.92e-15 | nearzero_reference_scale,nonmonotonicity,sign_change |
| balance | NH4 | 200 | 1.25e-02 | 1.97e-03 | 1.57e-02 | 2.66, -2.99 | 1.41e-03 | near_zero | 6.22e-15 | nearzero_reference_scale,nonmonotonicity,sign_change |
| balance | NO2 | 50 | 3.80e-04 | 4.01e-04 | 4.03e-04 | -0.08, -0.01 | 8.24e-05 | near_zero | 3.33e-16 | nearzero_reference_scale,nonmonotonicity |
| balance | NO2 | 100 | 2.60e-04 | 2.86e-04 | 3.11e-04 | -0.13, -0.12 | 2.12e-05 | resolved | 8.32e-16 | nonmonotonicity |
| balance | NO2 | 200 | 3.07e-05 | 2.13e-04 | 4.44e-05 | -2.80, 2.26 | 1.66e-05 | zero | 1.19e-15 | nonmonotonicity |
| balance | NO3 | 50 | 1.22e-03 | 8.64e-04 | 7.63e-04 | 0.50, 0.18 | 5.46e-04 | zero | 1.11e-16 | control_contamination |
| balance | NO3 | 100 | 2.71e-03 | 2.12e-03 | 1.18e-03 | 0.35, 0.85 | 1.63e-03 | zero | 2.75e-16 | control_contamination |
| balance | NO3 | 200 | 5.59e-03 | 5.03e-03 | 2.58e-03 | 0.15, 0.96 | 7.63e-03 | zero | 5.13e-09 | control_contamination,sign_change |
| boundary.cumulative.1 | NH4 | 50 | 6.95e-113 | 6.95e-113 | 6.95e-113 | 0.00, 0.00 | 0.00e+00 | near_zero | 9.81e-77 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NH4 | 100 | 1.68e-63 | 1.68e-63 | 1.68e-63 | 0.00, 0.00 | 0.00e+00 | near_zero | 1.01e-40 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NH4 | 200 | 5.44e-31 | 5.44e-31 | 5.44e-31 | 0.00, 0.00 | 0.00e+00 | near_zero | 1.95e-18 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NO2 | 50 | 3.01e-73 | 3.01e-73 | 3.01e-73 | 0.00, 0.00 | 0.00e+00 | near_zero | 3.38e-51 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NO2 | 100 | 1.47e-41 | 1.47e-41 | 1.47e-41 | 0.00, 0.00 | 0.00e+00 | near_zero | 3.40e-29 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NO2 | 200 | 2.95e-17 | 9.42e-19 | 1.66e-19 | 4.97, 2.50 | 6.42e-20 | near_zero | 1.47e-19 | control_contamination,nearzero_reference_scale |
| boundary.cumulative.1 | NO3 | 50 | 3.29e-64 | 3.29e-64 | 3.29e-64 | 0.00, 0.00 | 0.00e+00 | near_zero | 1.25e-42 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NO3 | 100 | 1.96e-32 | 1.96e-32 | 1.96e-32 | 0.00, 0.00 | 0.00e+00 | near_zero | 6.52e-20 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NO3 | 200 | 1.54e-08 | 4.49e-10 | 7.77e-11 | 5.10, 2.53 | 3.66e-11 | near_zero | 2.56e-09 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| concentration.endpoint_linf | NH4 | 50 | 1.44e-05 | 1.99e-05 | 1.89e-05 | -0.47, 0.08 | 1.11e-05 | resolved | 6.69e-17 | control_contamination,nonmonotonicity |
| concentration.endpoint_linf | NH4 | 100 | 1.44e-05 | 1.99e-05 | 1.89e-05 | -0.47, 0.08 | 1.11e-05 | resolved | 7.44e-17 | control_contamination,nonmonotonicity |
| concentration.endpoint_linf | NH4 | 200 | 1.44e-05 | 1.99e-05 | 1.89e-05 | -0.47, 0.08 | 1.11e-05 | resolved | 6.96e-17 | control_contamination,nonmonotonicity |
| concentration.endpoint_linf | NO2 | 50 | 4.68e-05 | 1.73e-05 | 3.33e-05 | 1.44, -0.95 | 1.93e-05 | resolved | 3.91e-18 | nonmonotonicity |
| concentration.endpoint_linf | NO2 | 100 | 4.68e-05 | 1.73e-05 | 3.33e-05 | 1.44, -0.95 | 1.93e-05 | resolved | 5.22e-18 | nonmonotonicity |
| concentration.endpoint_linf | NO2 | 200 | 4.68e-05 | 1.73e-05 | 3.33e-05 | 1.44, -0.95 | 1.93e-05 | resolved | 4.79e-18 | nonmonotonicity |
| concentration.endpoint_linf | NO3 | 50 | 6.17e-05 | 2.21e-05 | 7.59e-06 | 1.48, 1.54 | 1.38e-06 | resolved | 7.01e-18 |  |
| concentration.endpoint_linf | NO3 | 100 | 6.17e-05 | 2.21e-05 | 7.59e-06 | 1.48, 1.54 | 1.38e-06 | resolved | 1.14e-17 |  |
| concentration.endpoint_linf | NO3 | 200 | 6.17e-05 | 2.21e-05 | 7.59e-06 | 1.48, 1.54 | 1.38e-06 | resolved | 2.95e-09 |  |
| concentration.interior_linf | NH4 | 50 | 4.33e-02 | 1.27e-02 | 4.32e-03 | 1.77, 1.55 | 1.24e-03 | resolved | 9.71e-17 |  |
| concentration.interior_linf | NH4 | 100 | 2.52e-02 | 7.19e-03 | 2.77e-03 | 1.81, 1.38 | 1.01e-03 | resolved | 1.03e-16 |  |
| concentration.interior_linf | NH4 | 200 | 1.13e-02 | 3.24e-03 | 1.49e-03 | 1.80, 1.12 | 6.72e-04 | resolved | 1.59e-12 |  |
| concentration.interior_linf | NO2 | 50 | 1.62e-03 | 4.82e-04 | 1.77e-04 | 1.75, 1.44 | 5.76e-05 | resolved | 1.07e-17 |  |
| concentration.interior_linf | NO2 | 100 | 1.25e-03 | 3.69e-04 | 1.58e-04 | 1.76, 1.22 | 6.31e-05 | resolved | 1.21e-17 |  |
| concentration.interior_linf | NO2 | 200 | 7.16e-04 | 2.11e-04 | 1.08e-04 | 1.76, 0.96 | 5.12e-05 | resolved | 3.02e-14 |  |
| concentration.interior_linf | NO3 | 50 | 1.31e-03 | 5.91e-04 | 4.28e-04 | 1.15, 0.46 | 2.00e-04 | resolved | 1.99e-17 | control_contamination |
| concentration.interior_linf | NO3 | 100 | 1.41e-03 | 7.07e-04 | 5.64e-04 | 1.00, 0.33 | 2.74e-04 | resolved | 5.00e-14 | control_contamination |
| concentration.interior_linf | NO3 | 200 | 1.53e-03 | 8.66e-04 | 7.40e-04 | 0.82, 0.23 | 3.75e-04 | resolved | 5.44e-09 | control_contamination |
| concentration.l1 | NH4 | 50 | 1.38e-03 | 3.72e-04 | 1.32e-04 | 1.89, 1.49 | 4.63e-05 | resolved | 4.51e-17 |  |
| concentration.l1 | NH4 | 100 | 1.10e-03 | 3.06e-04 | 1.29e-04 | 1.84, 1.24 | 5.55e-05 | resolved | 5.34e-17 |  |
| concentration.l1 | NH4 | 200 | 6.86e-04 | 2.03e-04 | 1.34e-04 | 1.76, 0.60 | 6.47e-05 | resolved | 1.79e-13 |  |
| concentration.l1 | NO2 | 50 | 6.55e-05 | 1.81e-05 | 7.07e-06 | 1.86, 1.35 | 2.70e-06 | resolved | 4.12e-18 |  |
| concentration.l1 | NO2 | 100 | 6.61e-05 | 1.92e-05 | 8.39e-06 | 1.78, 1.20 | 4.17e-06 | resolved | 5.95e-18 |  |
| concentration.l1 | NO2 | 200 | 5.12e-05 | 1.57e-05 | 1.11e-05 | 1.71, 0.49 | 5.86e-06 | resolved | 3.45e-15 | control_contamination |
| concentration.l1 | NO3 | 50 | 1.10e-04 | 3.57e-05 | 2.60e-05 | 1.62, 0.46 | 1.27e-05 | resolved | 7.63e-18 | control_contamination |
| concentration.l1 | NO3 | 100 | 1.58e-04 | 5.83e-05 | 5.76e-05 | 1.44, 0.02 | 2.30e-05 | resolved | 5.90e-15 | control_contamination |
| concentration.l1 | NO3 | 200 | 2.22e-04 | 9.36e-05 | 1.78e-04 | 1.25, -0.92 | 7.13e-05 | resolved | 7.55e-10 | nonmonotonicity |
| concentration.l2 | NH4 | 50 | 5.81e-03 | 1.61e-03 | 5.97e-04 | 1.85, 1.43 | 1.94e-04 | resolved | 4.63e-17 |  |
| concentration.l2 | NH4 | 100 | 3.92e-03 | 1.10e-03 | 4.70e-04 | 1.83, 1.23 | 1.83e-04 | resolved | 5.50e-17 |  |
| concentration.l2 | NH4 | 200 | 2.05e-03 | 5.97e-04 | 3.15e-04 | 1.78, 0.92 | 1.44e-04 | resolved | 3.98e-13 |  |
| concentration.l2 | NO2 | 50 | 2.41e-04 | 6.54e-05 | 2.48e-05 | 1.88, 1.40 | 8.95e-06 | resolved | 4.24e-18 |  |
| concentration.l2 | NO2 | 100 | 2.09e-04 | 5.86e-05 | 2.66e-05 | 1.84, 1.14 | 1.14e-05 | resolved | 6.13e-18 |  |
| concentration.l2 | NO2 | 200 | 1.35e-04 | 3.99e-05 | 2.32e-05 | 1.77, 0.78 | 1.13e-05 | resolved | 7.61e-15 |  |
| concentration.l2 | NO3 | 50 | 2.85e-04 | 1.04e-04 | 7.77e-05 | 1.46, 0.42 | 3.74e-05 | resolved | 7.84e-18 | control_contamination |
| concentration.l2 | NO3 | 100 | 3.44e-04 | 1.46e-04 | 1.26e-04 | 1.24, 0.21 | 5.92e-05 | resolved | 1.28e-14 | control_contamination |
| concentration.l2 | NO3 | 200 | 4.05e-04 | 2.06e-04 | 2.38e-04 | 0.97, -0.21 | 1.08e-04 | resolved | 1.60e-09 | nonmonotonicity |
| concentration.linf | NH4 | 50 | 4.33e-02 | 1.27e-02 | 4.32e-03 | 1.77, 1.55 | 1.24e-03 | resolved | 9.71e-17 |  |
| concentration.linf | NH4 | 100 | 2.52e-02 | 7.19e-03 | 2.77e-03 | 1.81, 1.38 | 1.01e-03 | resolved | 1.03e-16 |  |
| concentration.linf | NH4 | 200 | 1.13e-02 | 3.24e-03 | 1.49e-03 | 1.80, 1.12 | 6.72e-04 | resolved | 1.59e-12 |  |
| concentration.linf | NO2 | 50 | 1.62e-03 | 4.82e-04 | 1.77e-04 | 1.75, 1.44 | 5.76e-05 | resolved | 1.07e-17 |  |
| concentration.linf | NO2 | 100 | 1.25e-03 | 3.69e-04 | 1.58e-04 | 1.76, 1.22 | 6.31e-05 | resolved | 1.21e-17 |  |
| concentration.linf | NO2 | 200 | 7.16e-04 | 2.11e-04 | 1.08e-04 | 1.76, 0.96 | 5.12e-05 | resolved | 3.02e-14 |  |
| concentration.linf | NO3 | 50 | 1.31e-03 | 5.91e-04 | 4.28e-04 | 1.15, 0.46 | 2.00e-04 | resolved | 1.99e-17 | control_contamination |
| concentration.linf | NO3 | 100 | 1.41e-03 | 7.07e-04 | 5.64e-04 | 1.00, 0.33 | 2.74e-04 | resolved | 5.00e-14 | control_contamination |
| concentration.linf | NO3 | 200 | 1.53e-03 | 8.66e-04 | 7.40e-04 | 0.82, 0.23 | 3.75e-04 | resolved | 5.44e-09 | control_contamination |
| inventory.total | NH4 | 50 | 1.67e-03 | 3.48e-04 | 1.59e-03 | 2.26, -2.19 | 6.96e-04 | resolved | 1.46e-15 | nonmonotonicity |
| inventory.total | NH4 | 100 | 3.93e-03 | 4.69e-04 | 4.02e-03 | 3.07, -3.10 | 7.55e-04 | resolved | 3.74e-15 | nonmonotonicity,sign_change |
| inventory.total | NH4 | 200 | 8.36e-03 | 1.59e-03 | 1.10e-02 | 2.40, -2.80 | 1.62e-03 | resolved | 4.69e-15 | nonmonotonicity,sign_change |
| inventory.total | NO2 | 50 | 7.54e-05 | 2.35e-05 | 6.41e-05 | 1.68, -1.45 | 3.88e-05 | resolved | 5.47e-17 | nonmonotonicity |
| inventory.total | NO2 | 100 | 2.01e-04 | 1.38e-05 | 1.96e-04 | 3.87, -3.83 | 4.96e-05 | resolved | 1.74e-16 | nonmonotonicity |
| inventory.total | NO2 | 200 | 4.33e-04 | 1.55e-05 | 5.54e-04 | 4.80, -5.16 | 6.71e-05 | resolved | 2.66e-16 | nonmonotonicity,sign_change |
| inventory.total | NO3 | 50 | 1.44e-03 | 1.16e-03 | 1.01e-03 | 0.31, 0.21 | 5.70e-04 | resolved | 5.56e-17 | control_contamination |
| inventory.total | NO3 | 100 | 2.20e-03 | 2.28e-03 | 7.51e-04 | -0.05, 1.60 | 1.41e-03 | resolved | 1.37e-16 | control_contamination,nonmonotonicity |
| inventory.total | NO3 | 200 | 1.85e-03 | 5.19e-03 | 6.70e-03 | -1.48, -0.37 | 7.37e-03 | resolved | 2.56e-09 | control_contamination,nonmonotonicity,sign_change |
| reaction.loss | NH4 | 50 | 2.36e-04 | 1.25e-04 | 2.20e-04 | 0.92, -0.82 | 9.73e-05 | resolved | 2.23e-16 | nonmonotonicity |
| reaction.loss | NH4 | 100 | 9.68e-04 | 1.45e-04 | 9.36e-04 | 2.74, -2.69 | 2.94e-04 | resolved | 5.20e-16 | empirical_quadrature_floor,nonmonotonicity |
| reaction.loss | NH4 | 200 | 4.14e-03 | 3.87e-04 | 4.63e-03 | 3.42, -3.58 | 2.14e-04 | resolved | 1.98e-16 | nonmonotonicity,sign_change |
| reaction.loss | NO2 | 50 | 2.19e-04 | 2.99e-04 | 2.46e-04 | -0.45, 0.28 | 2.39e-05 | resolved | 5.56e-17 | empirical_quadrature_floor,nonmonotonicity |
| reaction.loss | NO2 | 100 | 5.07e-04 | 1.54e-04 | 4.29e-04 | 1.72, -1.48 | 2.23e-04 | resolved | 1.37e-16 | empirical_quadrature_floor,nonmonotonicity,sign_change |
| reaction.loss | NO2 | 200 | 3.74e-03 | 1.59e-04 | 4.12e-03 | 4.56, -4.70 | 2.64e-04 | resolved | 7.23e-16 | empirical_quadrature_floor,nonmonotonicity,sign_change |
| reaction.loss | NO3 | 50 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.00e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.loss | NO3 | 100 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.00e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.loss | NO3 | 200 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.00e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | NH4 | 50 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.00e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | NH4 | 100 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.00e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | NH4 | 200 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.00e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | NO2 | 50 | 2.36e-04 | 1.25e-04 | 2.20e-04 | 0.92, -0.82 | 9.73e-05 | resolved | 2.23e-16 | nonmonotonicity |
| reaction.production | NO2 | 100 | 9.68e-04 | 1.45e-04 | 9.36e-04 | 2.74, -2.69 | 2.94e-04 | resolved | 5.20e-16 | empirical_quadrature_floor,nonmonotonicity |
| reaction.production | NO2 | 200 | 4.14e-03 | 3.87e-04 | 4.63e-03 | 3.42, -3.58 | 2.14e-04 | resolved | 1.98e-16 | nonmonotonicity,sign_change |
| reaction.production | NO3 | 50 | 2.19e-04 | 2.99e-04 | 2.46e-04 | -0.45, 0.28 | 2.39e-05 | resolved | 5.56e-17 | empirical_quadrature_floor,nonmonotonicity |
| reaction.production | NO3 | 100 | 5.07e-04 | 1.54e-04 | 4.29e-04 | 1.72, -1.48 | 2.23e-04 | resolved | 1.37e-16 | empirical_quadrature_floor,nonmonotonicity,sign_change |
| reaction.production | NO3 | 200 | 3.74e-03 | 1.59e-04 | 4.12e-03 | 4.56, -4.70 | 2.64e-04 | resolved | 7.23e-16 | empirical_quadrature_floor,nonmonotonicity,sign_change |

## Refinement and fitted orders

- No order was fitted. Classification of all 6045 informative findings: `inconclusive` (0 achieved, 0 failed), as in 22.4.4: no `solute-convergence-bounds-v1` source exists, so every finding carries `arithmetic_bound_missing` (and `quadrature_bound_missing` where applicable).
- Concentration norms (`linf`, `l2`, `l1`, `interior_linf`) for NH4 and NO2 decrease monotonically with local rates about 1.8 (251 to 501) and about 1.1 to 1.55 (501 to 1001); the second rate is lower and the control (dt .0125) error is a third to a half of the 1001-node error, so time error is already comparable at 1001 nodes (flag `control_contamination` appears on the NO3 rows). NO3 rates are 1.0 to 1.5, then 0.0 to 0.5.
- Endpoint (`concentration.endpoint_linf`) is non-monotone for NH4 and NO2 (1.44e-5, 1.99e-5, 1.89e-5 for NH4); its t=50/100/200 values are identical because the tool reports the endpoint maximum over the available history.

## Floors and uncertainty

- Reference uncertainty is at most 5.4e-9 (NO3 concentration norms at t=200), and about 1e-12 or smaller elsewhere; it is far below every resolved error. f32 rounding floors (about 1e-7 relative) are not reached by the concentration norms (smallest error 7.6e-6 for NO3 endpoint), but boundary cumulative outflow rows sit on the reference floor (`reference_floor`, `nearzero_reference_scale`: 1577 and 1391 findings) and are not interpretable as rates.
- `empirical_quadrature_floor` (710 findings) marks reaction rows; `zero_error`/`plateau_or_uncertainty_overlap` mark rows that are exactly zero (for example NH4 production, inlet-side species).

## Observations against the spec

1. **Node-1 outflow closure.** Outlet cumulative mass at t=200 is 5e-31 (NH4), 1.7e-19 to 2.9e-17 (NO2) and 7.8e-11 to 1.5e-8 (NO3), against inventory totals of 2.6 to 54 and inventory errors of 1e-4 to 1e-2. The outlet front lies beyond t=200 (as predicted from Peclet), so the node-1 closure cannot be tested by outflow at these times and **cannot explain the inventory error**; the closure loss is not observable on this horizon. NO2/NO3 outlet quantities converge (local p about 2.5 to 5) but are at the reference floor.
2. **Inventory and balance errors are not space errors at fixed dt.** `inventory.total` NH4 is 2.7e-5, 2.65e-5, 2.69e-5 at t=1 for 251/501/1001 nodes (flat in dx), and the dt=.0125 control is about half of that (1.4e-5), the signature of an error dominated by dt, not dx. At t=50 to 200 the three levels are non-monotone with sign changes (NH4 t=200: -8.4e-3, +1.6e-3, -1.1e-2; 501 nodes is the smallest) while the control is 1.6e-3. The balance residual has the same pattern (NH4 t=200: -1.25e-2, +1.97e-3, -1.57e-2; control 1.41e-3). Not chased; the 501-node level is the odd one and the cause (time error aliasing, cancellation, or a mesh-dependent term) is not isolated here. The 22.4.4 time axis found balance NH4 t=200 errors of 5.67e-3 at dt .05 (1001 nodes), so balance at dt .025, 1001 nodes (1.57e-2) is larger than at dt .05: the balance/inventory trajectory is not monotone in dt either.
3. The space and time axes are not separable at 1001 nodes for inventory/balance/reaction. The space control (dt .0125) bounds temporal sensitivity of concentration norms but its error is the same order as the 1001-node error, so spatial order of the concentration norms is "inconclusive" under spec :390-395, consistent with the lower second rate.
4. Component-case successes do not replace this full-chain trajectory (spec :594-595).

## Inputs for threshold proposal

No thresholds are proposed. For 22.5.1: (1) a bounds source is still the gate for any `achieved`/`failed` label; (2) the space axis now has all four rows with complete metrics, but only through the scratch replay above, not authenticated batch evidence; a reference-cache identity bound to a per-file SHA of the checker means any later edit of the checker forces recomputing 3 h Laplace references, so reference reuse needs a stable identity; (3) inventory/balance/reaction rows need an absolute floor and a note that they are dt-dominated at dt .025; (4) the per-row 180 s timeout for space_control is a recorded deviation, with measured need 83 to 84 s direct (76 s in lifecycle).

## Spot-check list

- `.development/22.4.5/exec.json` (execution receipt), `verify-df5e.json` (exit 0), `verify-66f7e9ba.err`/`verify-0cf92fa6.err` (recorded failures).
- `.development/22.4.5/meas/times.txt` (timing), `space-analysis.json` (full analysis), `tables.md`.
- Ledger: `.development/22-convergence/launch-ledger.jsonl` (one start, exit 0, complete for the control; `timeout_seconds` 180).

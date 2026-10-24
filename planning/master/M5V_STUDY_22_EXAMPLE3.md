# M5-V study 22.4.4: Example 3 chain on the default FortranCompatible backend

Study only: no code changed, no holdout row run, no thresholds proposed. The space axis is incomplete (see Lifecycle and verify status).

## Run identity

- Worktree branch `orchestration/22.4.4`; `CASES_V2.json` SHA-256 `644c4845956d48cc27e0f45387a06317dbbf9aa671667617fc6487e4b3ad8a3d`; backend `fortran-compatible`, `solver: rust`, f32 arithmetic.
- Harness `solute_verification_harness-ec7dfff3fd1b6e1e` SHA-256 `5f90722384dbfdcd8da7622f5fafd4529e0384c4c2e5eddbf61b50e49b07bc2e` (`.development/22.4.4/harness.sha`). Fresh roots `.development/22-benchmark` and `.development/22-convergence`, output roots `example3-v2`. Ledger SHA-256: benchmark `27a03577…`, convergence `4ef4e3f0…` (never edited).
- Activation files (derived from `lifecycle.validate_activation`, as 22.4.3): `.development/22.4.4/activation-benchmark-{component,bridge}.json`, `activation-convergence{,-b,-c}.json` (`-b` and `-c` are subsets re-derived after the timeout, below). The bridge gate wrote the `not_applicable_v2` receipt (`bridge-gate.json`).
- Commands (`CARGO_TARGET_DIR` unset, `target` symlink present, `--allow-reference-work` only; no escalation, no `--bounds-source`):
  - `check_solute_benchmarks.py --cases reference/solute_verification/CASES_V2.json --output-root .development/22-benchmark/example3-v2 --execute --stage benchmark --allow-reference-work --activation-file .development/22.4.4/activation-benchmark-component.json --launch-id benchmark-example3-component-rfc-baseline`
  - bridge: same with `--stage bridge`, activation kind `bridge`, no `--allow-reference-work` (the tool refuses reference work for the bridge stage), `--launch-id benchmark-example3-bridge-rfc-baseline`.
  - `run_solute_convergence.py --cases ... --output-root .development/22-convergence/example3-v2 --execute --allow-reference-work --activation-file ... --launch-id <ids>` (4 calls, below).
- Batches:
  - benchmark component `batch-f1ecd5dabb944eaead4dff422d1b3969` (summary SHA-256 `0668f9de…`);
  - benchmark bridge `batch-9c2c024908a84c3eb67b429e39e8e5fc`;
  - convergence first call (space 251/501/1001 complete, space_control timeout) `batch-66f7e9bacfbd47c299ec0ae191344c31`;
  - convergence re-call attempts refused as "row already consumed" `batch-4a24d3b04801463bb9c26f1b53b3e030`, `batch-5cbde83caa0a44caa5ab29f46ce76f2a` (no runs);
  - convergence time rows `batch-0cf92fa649f04e268b86b79c1e0208c5` (`classification.json` SHA-256 `64223d3c…`, `summary.json` `2ae81a42…`).
- Run manifest SHA-256 prefixes: `.development/22.4.4/manifest-sha256.txt`. Laplace reference: admitted for the benchmark batch and the time batch (4233 observables, all `admitted`).

## Lifecycle and verify status

| Row | State | Notes |
|---|---|---|
| benchmark component | `evidence_complete`, `--verify-only --batch` exit 0 | about 20 min wall, nearly all the Laplace reference build |
| benchmark bridge (`legacy_bridge` policy) | run exit 0, harness test passed, lifecycle `failed` (`mapping_validation`: "record inventory mismatch") | tool: "row cannot be automatically retried"; `--verify-only` exit 2 ("failed/incomplete run cannot verify"); not retried |
| space_251, space_501, space_1001 | run `complete`, exit 0 | batch `66f7e9ba…` is incomplete, so classify and `--verify-only` exit 2 and no metrics were computed |
| space_control (dt .0125, 16000 steps) | `timeout` at the 60 s ceiling | retry as a new attempt was refused ("row already consumed; explicit authenticated resume required"); a resume is outside this task, so the row stays failed |
| time_0.2, time_0.1, time_0.05, time_control | complete, `evidence_complete`, `--verify-only` exit 0 | batch `0cf92fa6…` |

Verify-only exit codes: `f1ecd5da…` 0; `0cf92fa6…` 0; `9c2c0249…` 2; `66f7e9ba…` 2; `4a24d3b0…` 2 and `5cbde83c…` 2 (empty batches from the refused calls). The failures are recorded per the failed-rows rule; nothing was chased. CASES_V2 and the timeouts were not edited.

## Metrics table

Time axis (1001 nodes, dt .2/.1/.05), absolute error against the Laplace reference per species (NH4, NO2, NO3 = the chain), `t` = 50/100/200. "local p" is `log2(e_coarse/e_fine)` of the observed errors, descriptive only. "ctrl" is the 501-node dt=.05 control's error. Branches are as the metrics tool assigned them (`resolved`/`near_zero`/`zero`). Not tabulated: `profile_*` at t=0 and `temporal_l2` (present in the batch metrics). Full per-finding data: `batch-0cf92fa6…/classification.json`.

| observable | sp | t | e(.2) | e(.1) | e(.05) | local p (.2-.1, .1-.05) | ctrl abs | branch | ref uncert | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| balance | NH4 | 50 | 9.73e-03 | 4.75e-03 | 2.16e-03 | 1.04, 1.13 | 8.53e-05 | near_zero | 2.0e-15 | nearzero_reference_scale |
| balance | NH4 | 100 | 1.77e-02 | 8.46e-03 | 3.62e-03 | 1.07, 1.22 | 5.11e-04 | near_zero | 4.9e-15 | nearzero_reference_scale |
| balance | NH4 | 200 | 3.00e-02 | 1.39e-02 | 5.67e-03 | 1.11, 1.30 | 1.47e-03 | near_zero | 6.2e-15 | nearzero_reference_scale |
| boundary.cumulative.0 | NH4 | 50 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | resolved | 3.3e-16 | control_contamination,plateau_or_uncertainty_overlap,reference_floor,zero_error |
| boundary.cumulative.0 | NH4 | 100 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | resolved | 6.7e-16 | control_contamination,plateau_or_uncertainty_overlap,reference_floor,zero_error |
| boundary.cumulative.0 | NH4 | 200 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | resolved | 1.3e-15 | control_contamination,plateau_or_uncertainty_overlap,reference_floor,zero_error |
| boundary.cumulative.1 | NH4 | 50 | 6.95e-113 | 6.95e-113 | 6.95e-113 | 0.00, 0.00 | 0.00e+00 | near_zero | 9.8e-77 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NH4 | 100 | 1.68e-63 | 1.68e-63 | 1.68e-63 | 0.00, 0.00 | 0.00e+00 | near_zero | 1.0e-40 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NH4 | 200 | 5.44e-31 | 5.44e-31 | 5.44e-31 | 0.00, 0.00 | 0.00e+00 | near_zero | 1.9e-18 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.instantaneous.1 | NH4 | 50 | 2.59e-113 | 2.59e-113 | 2.59e-113 | 0.00, 0.00 | 0.00e+00 | near_zero | 7.8e-77 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.instantaneous.1 | NH4 | 100 | 5.73e-63 | 5.73e-63 | 5.73e-63 | 0.00, 0.00 | 0.00e+00 | near_zero | 1.0e-41 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.instantaneous.1 | NH4 | 200 | 1.06e-31 | 1.06e-31 | 1.06e-31 | 0.00, 0.00 | 0.00e+00 | near_zero | 3.5e-19 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| concentration.endpoint_linf | NH4 | 50 | 1.65e-04 | 8.22e-05 | 4.06e-05 | 1.00, 1.02 | 9.54e-07 | resolved | 6.7e-17 |  |
| concentration.endpoint_linf | NH4 | 100 | 1.65e-04 | 8.22e-05 | 4.06e-05 | 1.00, 1.02 | 9.54e-07 | resolved | 7.4e-17 |  |
| concentration.endpoint_linf | NH4 | 200 | 1.65e-04 | 8.22e-05 | 4.06e-05 | 1.00, 1.02 | 9.54e-07 | resolved | 7.0e-17 |  |
| concentration.interior_linf | NH4 | 50 | 1.88e-02 | 1.04e-02 | 6.20e-03 | 0.85, 0.75 | 8.26e-03 | resolved | 9.7e-17 | control_contamination |
| concentration.interior_linf | NH4 | 100 | 1.44e-02 | 7.76e-03 | 4.33e-03 | 0.89, 0.84 | 4.60e-03 | resolved | 1.0e-16 | control_contamination |
| concentration.interior_linf | NH4 | 200 | 8.89e-03 | 4.69e-03 | 2.49e-03 | 0.92, 0.92 | 1.99e-03 | resolved | 1.6e-12 |  |
| concentration.l1 | NH4 | 50 | 7.01e-04 | 3.72e-04 | 2.06e-04 | 0.92, 0.85 | 2.47e-04 | resolved | 4.5e-17 | control_contamination |
| concentration.l1 | NH4 | 100 | 7.67e-04 | 4.00e-04 | 2.12e-04 | 0.94, 0.91 | 1.96e-04 | resolved | 5.3e-17 | control_contamination |
| concentration.l1 | NH4 | 200 | 6.63e-04 | 3.42e-04 | 1.76e-04 | 0.95, 0.96 | 1.23e-04 | resolved | 1.8e-13 |  |
| concentration.l2 | NH4 | 50 | 2.97e-03 | 1.61e-03 | 9.19e-04 | 0.88, 0.81 | 1.06e-03 | resolved | 4.6e-17 | control_contamination |
| concentration.l2 | NH4 | 100 | 2.70e-03 | 1.43e-03 | 7.78e-04 | 0.91, 0.88 | 7.00e-04 | resolved | 5.5e-17 | control_contamination |
| concentration.l2 | NH4 | 200 | 1.94e-03 | 1.02e-03 | 5.33e-04 | 0.93, 0.93 | 3.59e-04 | resolved | 4.0e-13 |  |
| concentration.linf | NH4 | 50 | 1.88e-02 | 1.04e-02 | 6.20e-03 | 0.85, 0.75 | 8.26e-03 | resolved | 9.7e-17 | control_contamination |
| concentration.linf | NH4 | 100 | 1.44e-02 | 7.76e-03 | 4.33e-03 | 0.89, 0.84 | 4.60e-03 | resolved | 1.0e-16 | control_contamination |
| concentration.linf | NH4 | 200 | 8.89e-03 | 4.69e-03 | 2.49e-03 | 0.92, 0.92 | 1.99e-03 | resolved | 1.6e-12 |  |
| inventory.total | NH4 | 50 | 8.54e-03 | 4.14e-03 | 1.86e-03 | 1.04, 1.15 | 8.34e-05 | resolved | 1.5e-15 |  |
| inventory.total | NH4 | 100 | 1.37e-02 | 6.47e-03 | 2.70e-03 | 1.08, 1.26 | 4.41e-04 | resolved | 3.7e-15 |  |
| inventory.total | NH4 | 200 | 1.78e-02 | 8.13e-03 | 3.20e-03 | 1.13, 1.34 | 1.02e-03 | resolved | 4.7e-15 |  |
| reaction.loss | NH4 | 50 | 1.19e-03 | 6.04e-04 | 3.00e-04 | 0.98, 1.01 | 1.85e-06 | resolved | 2.2e-16 |  |
| reaction.loss | NH4 | 100 | 4.06e-03 | 1.99e-03 | 9.18e-04 | 1.03, 1.12 | 6.93e-05 | resolved | 5.2e-16 |  |
| reaction.loss | NH4 | 200 | 1.22e-02 | 5.80e-03 | 2.47e-03 | 1.07, 1.23 | 4.42e-04 | resolved | 2.0e-16 |  |
| reaction.production | NH4 | 50 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | NH4 | 100 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | NH4 | 200 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| balance | NO2 | 50 | 1.92e-03 | 1.06e-03 | 6.01e-04 | 0.86, 0.81 | 2.92e-05 | near_zero | 3.3e-16 | nearzero_reference_scale |
| balance | NO2 | 100 | 1.55e-03 | 8.66e-04 | 4.21e-04 | 0.84, 1.04 | 1.24e-04 | resolved | 8.3e-16 |  |
| balance | NO2 | 200 | 1.01e-03 | 5.61e-04 | 7.85e-05 | 0.84, 2.84 | 4.32e-04 | zero | 1.2e-15 | control_contamination,sign_change |
| boundary.cumulative.0 | NO2 | 50 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| boundary.cumulative.0 | NO2 | 100 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| boundary.cumulative.0 | NO2 | 200 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| boundary.cumulative.1 | NO2 | 50 | 3.01e-73 | 3.01e-73 | 3.01e-73 | 0.00, 0.00 | 0.00e+00 | near_zero | 3.4e-51 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NO2 | 100 | 1.47e-41 | 1.47e-41 | 1.47e-41 | 0.00, 0.00 | 0.00e+00 | near_zero | 3.4e-29 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NO2 | 200 | 7.16e-18 | 1.24e-18 | 3.67e-19 | 2.53, 1.76 | 1.23e-18 | near_zero | 1.5e-19 | control_contamination,nearzero_reference_scale |
| boundary.instantaneous.1 | NO2 | 50 | 1.43e-72 | 1.43e-72 | 1.43e-72 | 0.00, 0.00 | 0.00e+00 | near_zero | 2.9e-51 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.instantaneous.1 | NO2 | 100 | 5.40e-42 | 5.40e-42 | 5.40e-42 | 0.00, 0.00 | 0.00e+00 | near_zero | 5.4e-30 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.instantaneous.1 | NO2 | 200 | 3.56e-18 | 7.16e-19 | 2.30e-19 | 2.32, 1.64 | 6.68e-19 | near_zero | 4.6e-20 | control_contamination,nearzero_reference_scale |
| concentration.endpoint_linf | NO2 | 50 | 3.06e-04 | 1.50e-04 | 7.20e-05 | 1.03, 1.06 | 1.61e-05 | resolved | 3.9e-18 |  |
| concentration.endpoint_linf | NO2 | 100 | 3.06e-04 | 1.50e-04 | 7.20e-05 | 1.03, 1.06 | 1.61e-05 | resolved | 5.2e-18 |  |
| concentration.endpoint_linf | NO2 | 200 | 3.06e-04 | 1.50e-04 | 7.20e-05 | 1.03, 1.06 | 1.61e-05 | resolved | 4.8e-18 |  |
| concentration.interior_linf | NO2 | 50 | 8.47e-04 | 4.59e-04 | 2.65e-04 | 0.88, 0.79 | 3.08e-04 | resolved | 1.1e-17 | control_contamination |
| concentration.interior_linf | NO2 | 100 | 8.82e-04 | 4.67e-04 | 2.55e-04 | 0.92, 0.88 | 2.28e-04 | resolved | 1.2e-17 | control_contamination |
| concentration.interior_linf | NO2 | 200 | 6.67e-04 | 3.48e-04 | 1.82e-04 | 0.94, 0.94 | 1.24e-04 | resolved | 3.0e-14 |  |
| concentration.l1 | NO2 | 50 | 4.18e-05 | 2.17e-05 | 1.17e-05 | 0.94, 0.89 | 1.21e-05 | resolved | 4.1e-18 | control_contamination |
| concentration.l1 | NO2 | 100 | 5.85e-05 | 3.01e-05 | 1.56e-05 | 0.96, 0.95 | 1.24e-05 | resolved | 6.0e-18 |  |
| concentration.l1 | NO2 | 200 | 6.17e-05 | 3.14e-05 | 1.58e-05 | 0.97, 1.00 | 9.82e-06 | resolved | 3.5e-15 |  |
| concentration.l2 | NO2 | 50 | 1.36e-04 | 7.19e-05 | 3.96e-05 | 0.92, 0.86 | 4.42e-05 | resolved | 4.2e-18 | control_contamination |
| concentration.l2 | NO2 | 100 | 1.64e-04 | 8.54e-05 | 4.53e-05 | 0.94, 0.91 | 3.76e-05 | resolved | 6.1e-18 |  |
| concentration.l2 | NO2 | 200 | 1.47e-04 | 7.58e-05 | 3.90e-05 | 0.96, 0.96 | 2.39e-05 | resolved | 7.6e-15 |  |
| concentration.linf | NO2 | 50 | 8.47e-04 | 4.59e-04 | 2.65e-04 | 0.88, 0.79 | 3.08e-04 | resolved | 1.1e-17 | control_contamination |
| concentration.linf | NO2 | 100 | 8.82e-04 | 4.67e-04 | 2.55e-04 | 0.92, 0.88 | 2.28e-04 | resolved | 1.2e-17 | control_contamination |
| concentration.linf | NO2 | 200 | 6.67e-04 | 3.48e-04 | 1.82e-04 | 0.94, 0.94 | 1.24e-04 | resolved | 3.0e-14 |  |
| inventory.total | NO2 | 50 | 3.69e-04 | 1.80e-04 | 8.98e-05 | 1.04, 1.00 | 1.27e-05 | resolved | 5.5e-17 |  |
| inventory.total | NO2 | 100 | 7.11e-04 | 3.41e-04 | 1.68e-04 | 1.06, 1.02 | 4.07e-05 | resolved | 1.7e-16 |  |
| inventory.total | NO2 | 200 | 9.24e-04 | 4.30e-04 | 2.14e-04 | 1.10, 1.00 | 8.47e-05 | resolved | 2.7e-16 |  |
| reaction.loss | NO2 | 50 | 1.10e-03 | 6.33e-04 | 3.91e-04 | 0.80, 0.70 | 1.84e-05 | resolved | 5.6e-17 | empirical_quadrature_floor |
| reaction.loss | NO2 | 100 | 1.79e-03 | 7.84e-04 | 3.29e-04 | 1.19, 1.25 | 1.53e-04 | resolved | 1.4e-16 | empirical_quadrature_floor |
| reaction.loss | NO2 | 200 | 1.03e-02 | 4.81e-03 | 2.33e-03 | 1.10, 1.05 | 7.90e-04 | resolved | 7.2e-16 |  |
| reaction.production | NO2 | 50 | 1.19e-03 | 6.04e-04 | 3.00e-04 | 0.98, 1.01 | 1.85e-06 | resolved | 2.2e-16 |  |
| reaction.production | NO2 | 100 | 4.06e-03 | 1.99e-03 | 9.18e-04 | 1.03, 1.12 | 6.93e-05 | resolved | 5.2e-16 |  |
| reaction.production | NO2 | 200 | 1.22e-02 | 5.80e-03 | 2.47e-03 | 1.07, 1.23 | 4.42e-04 | resolved | 2.0e-16 |  |
| balance | NO3 | 50 | 7.61e-03 | 3.73e-03 | 1.81e-03 | 1.03, 1.04 | 1.28e-05 | zero | 1.1e-16 |  |
| balance | NO3 | 100 | 1.53e-02 | 7.55e-03 | 3.91e-03 | 1.02, 0.95 | 1.37e-04 | zero | 2.7e-16 |  |
| balance | NO3 | 200 | 2.54e-02 | 1.21e-02 | 7.45e-03 | 1.07, 0.70 | 1.19e-03 | zero | 5.1e-09 |  |
| boundary.cumulative.0 | NO3 | 50 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| boundary.cumulative.0 | NO3 | 100 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| boundary.cumulative.0 | NO3 | 200 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| boundary.cumulative.1 | NO3 | 50 | 3.29e-64 | 3.29e-64 | 3.29e-64 | 0.00, 0.00 | 0.00e+00 | near_zero | 1.2e-42 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NO3 | 100 | 1.96e-32 | 1.96e-32 | 1.96e-32 | 0.00, 0.00 | 0.00e+00 | near_zero | 6.5e-20 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.cumulative.1 | NO3 | 200 | 1.45e-08 | 1.19e-09 | 2.22e-10 | 3.61, 2.42 | 7.41e-10 | near_zero | 2.6e-09 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.instantaneous.1 | NO3 | 50 | 2.57e-64 | 2.57e-64 | 2.57e-64 | 0.00, 0.00 | 0.00e+00 | near_zero | 4.2e-42 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.instantaneous.1 | NO3 | 100 | 1.44e-32 | 1.44e-32 | 1.44e-32 | 0.00, 0.00 | 0.00e+00 | near_zero | 1.1e-19 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| boundary.instantaneous.1 | NO3 | 200 | 7.90e-09 | 7.60e-10 | 1.56e-10 | 3.38, 2.28 | 4.53e-10 | near_zero | 1.3e-09 | control_contamination,nearzero_error,nearzero_reference_scale,plateau_or_uncertainty_overlap,reference_floor |
| concentration.endpoint_linf | NO3 | 50 | 2.53e-05 | 1.55e-05 | 1.03e-05 | 0.70, 0.59 | 1.45e-05 | resolved | 7.0e-18 | control_contamination |
| concentration.endpoint_linf | NO3 | 100 | 2.53e-05 | 1.55e-05 | 1.03e-05 | 0.70, 0.59 | 1.45e-05 | resolved | 1.1e-17 | control_contamination |
| concentration.endpoint_linf | NO3 | 200 | 2.53e-05 | 1.55e-05 | 1.03e-05 | 0.70, 0.59 | 1.45e-05 | resolved | 2.9e-09 | control_contamination |
| concentration.interior_linf | NO3 | 50 | 3.01e-03 | 1.57e-03 | 8.15e-04 | 0.94, 0.95 | 1.93e-04 | resolved | 2.0e-17 |  |
| concentration.interior_linf | NO3 | 100 | 4.07e-03 | 2.12e-03 | 1.10e-03 | 0.94, 0.95 | 1.93e-04 | resolved | 5.0e-14 |  |
| concentration.interior_linf | NO3 | 200 | 5.45e-03 | 2.85e-03 | 1.47e-03 | 0.94, 0.96 | 2.05e-04 | resolved | 5.4e-09 |  |
| concentration.l1 | NO3 | 50 | 2.03e-04 | 1.02e-04 | 5.08e-05 | 1.00, 1.00 | 1.97e-05 | resolved | 7.6e-18 |  |
| concentration.l1 | NO3 | 100 | 3.97e-04 | 1.99e-04 | 9.88e-05 | 1.00, 1.01 | 2.77e-05 | resolved | 5.9e-15 |  |
| concentration.l1 | NO3 | 200 | 8.05e-04 | 4.07e-04 | 1.93e-04 | 0.99, 1.08 | 3.91e-05 | resolved | 7.5e-10 |  |
| concentration.l2 | NO3 | 50 | 5.80e-04 | 2.96e-04 | 1.50e-04 | 0.97, 0.98 | 5.02e-05 | resolved | 7.8e-18 |  |
| concentration.l2 | NO3 | 100 | 9.24e-04 | 4.72e-04 | 2.39e-04 | 0.97, 0.98 | 5.80e-05 | resolved | 1.3e-14 |  |
| concentration.l2 | NO3 | 200 | 1.45e-03 | 7.41e-04 | 3.72e-04 | 0.97, 0.99 | 6.49e-05 | resolved | 1.6e-09 |  |
| concentration.linf | NO3 | 50 | 3.01e-03 | 1.57e-03 | 8.15e-04 | 0.94, 0.95 | 1.93e-04 | resolved | 2.0e-17 |  |
| concentration.linf | NO3 | 100 | 4.07e-03 | 2.12e-03 | 1.10e-03 | 0.94, 0.95 | 1.93e-04 | resolved | 5.0e-14 |  |
| concentration.linf | NO3 | 200 | 5.45e-03 | 2.85e-03 | 1.47e-03 | 0.94, 0.96 | 2.05e-04 | resolved | 5.4e-09 |  |
| inventory.total | NO3 | 50 | 8.71e-03 | 4.36e-03 | 2.20e-03 | 1.00, 0.98 | 5.59e-06 | resolved | 5.6e-17 |  |
| inventory.total | NO3 | 100 | 1.35e-02 | 6.76e-03 | 3.58e-03 | 1.00, 0.92 | 1.55e-05 | resolved | 1.4e-16 |  |
| inventory.total | NO3 | 200 | 1.51e-02 | 7.31e-03 | 5.12e-03 | 1.05, 0.51 | 3.97e-04 | resolved | 2.6e-09 |  |
| reaction.loss | NO3 | 50 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.loss | NO3 | 100 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.loss | NO3 | 200 | 0.00e+00 | 0.00e+00 | 0.00e+00 | n/a, n/a | 0.00e+00 | zero | 0.0e+00 | plateau_or_uncertainty_overlap,zero_error |
| reaction.production | NO3 | 50 | 1.10e-03 | 6.33e-04 | 3.91e-04 | 0.80, 0.70 | 1.84e-05 | resolved | 5.6e-17 | empirical_quadrature_floor |
| reaction.production | NO3 | 100 | 1.79e-03 | 7.84e-04 | 3.29e-04 | 1.19, 1.25 | 1.53e-04 | resolved | 1.4e-16 | empirical_quadrature_floor |
| reaction.production | NO3 | 200 | 1.03e-02 | 4.81e-03 | 2.33e-03 | 1.10, 1.05 | 7.90e-04 | resolved | 7.2e-16 |  |

Quadrature split, benchmark row (dt .1, 1001 nodes) at t=200, total inventory:

| species | solver total | err vs exact continuum (dissolved) | err vs mesh-weighted truth (total) | quadrature (total) |
|---|---|---|---|---|
| NH4 | 54.354235 | 4.067e-03 | -8.178e-03 | 4.463e-05 |
| NO2 | 2.634432 | 4.299e-04 | -4.083e-04 | -2.155e-05 |
| NO3 | 29.010078 | 7.307e-03 | 7.308e-03 | -7.685e-07 |

The quadrature term is 2 to 4 orders below the inventory error, so the inventory error is not a quadrature artifact. The exact-continuum figure is for the dissolved phase and the mesh figure for the total (sorbed equals dissolved for NH4), and their signs differ for NH4 and NO2; read them as the tool's two definitions, not one signed series.

## Refinement and fitted orders

No order was fitted. The classifier labels all 6090 time findings `inconclusive` (reasons `arithmetic_bound_missing`, `quadrature_bound_missing`, `coarser_control_cannot_establish_finest_mesh_adequacy`). Observed local rates, as context:

- Time axis, concentration `linf`/`l1`/`l2`: 0.75 to 1.0 per halving of dt (NO3 l1/l2 0.97 to 1.08, NH4 interior linf 0.75 to 0.92 and rising); inventory 1.0 to 1.3, with NO3 inventory 1.05 then 0.51 at t=200; reaction terms 0.7 to 1.25. The spec's expected temporal order is first order (backward Euler); the observed rates approach 1 from below.
- Temporal error is not isolated. The 501-node control at dt=.05 has errors comparable to or larger than the 1001-node dt=.05 error for NH4/NO2 concentration at t=50 and 100 (e.g. NH4 linf 8.26e-3 against 6.20e-3), so spatial error at 501 nodes contaminates the time axis ("control_contamination"). Per spec :390-395 and the plan row, **temporal error cannot be isolated: inconclusive.**
- Space axis (251/501/1001, control): not analyzed, as the control timed out and the batch cannot be classified or verified. The planned control (dt .0125) was never obtained.
- Sign change and non-monotone cases: NO2 balance at t=200 (rate 0.84 then 2.84, flag `sign_change`, control contamination); NO3 inventory t=200 and NO3 balance t=200 slow at the finest level (0.51, 0.70).

## Floors and uncertainty

- Laplace reference: all 4233 observables `admitted` by the frozen budget (largest uncertainty/budget ratio 0.054). Uncertainties in the table are about 1e-17 for concentrations at t=50 and 100, rising to 1e-9 to 1e-13 at t=200 (NO3, NH4 interior), and 1e-77 to 1e-18 for outlet quantities. Reference uncertainty is far below every resolved error (smallest resolved error about 1e-5), except the outlet tail.
- Outlet tail (`boundary.*.1`): at t=50 and 100 the reference is 1e-30 to 1e-113 and the solver error equals that scale for all dt (plateau, `reference_floor`, flat across dt); at t=200 NO3 outlet error 1.45e-8, 1.19e-9, 2.22e-10 sits at or below its reference uncertainty (2.6e-9 cumulative, 1.3e-9 flux), so it is not a resolved error and gives no rate.
- Endpoint `linf` is identical at all three times per species (inlet cell error), 1.65e-4/8.22e-5/4.06e-5 for NH4: this is the inlet-node value, not tied to the front.
- f32 floor: no finding reaches the f32 resolution (about 1e-7 relative of the unit inlet value); the smallest resolved error in the table is of order 1e-6 to 1e-5 (NO2 endpoint control 1.6e-5, NO3 control profile errors 2e-4). No plateau at the f32 floor is claimed.
- Missing for a bounds source: the classifier needs a `solute-convergence-bounds-v1` plus `-review-v1` with a root authorization (arithmetic and quadrature bounds). None exists and none was produced.

## Observations against the spec's expected behavior

- Component success does not replace this full-chain trajectory (spec :594-595); this study is that trajectory for the time axis only. The bridge row did not run to completion in the lifecycle.
- Inventory is not conserved: balance residuals are 9.7e-3 (NH4, t=50) rising to 3.0e-2 (t=200) at dt .2, halving with dt, so first order in dt for the residual; the 501-node control is far smaller (1.5e-3 at t=200), which points at a space-dependent part of the balance, not just a time part. This was measured, but no claim is made on the node-1 outflow closure because the space axis was not run.
- The chain is produced: NO2 and NO3 profile errors are nonzero and the Bateman-type production and loss terms agree to first order (reaction production/loss of NO2 and NO3 coincide, as the chain implies); NH4 production and NO3 loss are exactly zero as expected.
- Outlet values stay below the reference noise until t=200 for NO3 (large Peclet number, vL/2D about 700), so the breakthrough tail is not testable on this schedule.

## Inputs for threshold proposal

No thresholds are proposed. For 22.5.1: (1) a bounds source is the gate for any `achieved`/`failed` classification; (2) the time-axis errors above are real and unambiguous at t=50 to 200 (reference uncertainty at least 3 orders smaller), but are not isolated from spatial error by the 501-node control; (3) the space axis needs a retry of `space_control` under an authenticated resume (the 60 s ceiling cut dt=.0125 at 1001 nodes) or a decision on the ceiling, not decided here; (4) the bridge row fails lifecycle mapping validation ("record inventory mismatch") with the `legacy_bridge` time policy, a lifecycle matter to review; (5) reference outlet quantities before t=200 are below f32 resolution and need an absolute floor, not a relative one.

## Spot-check list

- NH4 concentration.linf t=200, dt .2/.1/.05: 8.89e-3, 4.69e-3, 2.49e-3: table row, `classification.json` finding `concentration.linf`/NH4/200.
- NO3 inventory.total t=100: 1.35e-2, 6.76e-3, 3.58e-3.
- Balance NO2 t=200 sign change: rows in table.
- Quadrature split: `batch-f1ecd5da…/results/benchmark-example3-component-rfc-baseline.json`, species history at time 200.
- Lifecycle: `22-convergence/launch-ledger.jsonl` timeout event for `convergence-example3-space_control-rfc-baseline`.

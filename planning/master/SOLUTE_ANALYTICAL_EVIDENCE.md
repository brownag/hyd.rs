# Solute analytical benchmark evidence — Orchestration 13

**Evidence status, 2026-09-28 America/Los_Angeles:** two retained-output bridges and ten component rows completed and verified; the full Example 3 reference was admitted after all five authorized stages. The single full-case Fortran start failed endpoint-observation validation, and the Rust row did not start, so the full solver comparison remains unmet. 13/77 starts consumed (64 remain). No solver accuracy thresholds have been assigned; M5-V remains open.

## Scope and scientific interpretation

This task uses the independently authored constant-water cases and frozen fourteen-row benchmark matrix in [CASES.json](../../reference/solute_verification/CASES.json), under the [verification contract](SOLUTE_VERIFICATION_SPEC.md). The bridge comparisons establish consistency with retained discrete results. The component comparisons use independent continuum formulas and integrals. They provide measured errors at one mesh and timestep per case; convergence and justified solver thresholds require the later controlled studies.

All five component references and the full transient three-species Example 3 reference passed per-observable admission with complete requested coverage. This admits reference uncertainty under the frozen budgets; it does not accept a solver error. The full transient solver comparison remains unmet because its Fortran output failed adapter validation and Rust was not started.

## Activation, commands, and preservation

The orchestrator separately activated bridge, component, and full Example 3 batches after reviewing the preceding results. Activation and reference-work policies are retained in `.development/13.3.1/{bridge,component,example3}-activation.json`, `component-reference-policy.json`, and `example3-reference-policy.json`. Commands were invoked once; no scientific retry or added row has occurred. The full-case policy permits one optional order 512 at 650 digits if admission requires it.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_solute_benchmarks.py --cases reference/solute_verification/CASES.json --execute --stage bridge --activation-file .development/13.3.1/bridge-activation.json --output-root .development/13.3.1/benchmarks
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_solute_benchmarks.py --cases reference/solute_verification/CASES.json --execute --stage benchmark --activation-file .development/13.3.1/component-activation.json --bridge-report .development/13.3.1/benchmarks/batch-c96c2d089400481a913a4accb6f1650f/bridges/benchmark-example3-bridge-fortran-baseline/bridge.json --bridge-report .development/13.3.1/benchmarks/batch-c96c2d089400481a913a4accb6f1650f/bridges/benchmark-example3-bridge-rust-baseline/bridge.json --allow-reference-work --output-root .development/13.3.1/benchmarks
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_solute_benchmarks.py --cases reference/solute_verification/CASES.json --execute --stage benchmark --activation-file .development/13.3.1/example3-activation.json --bridge-report .development/13.3.1/benchmarks/batch-c96c2d089400481a913a4accb6f1650f/bridges/benchmark-example3-bridge-fortran-baseline/bridge.json --bridge-report .development/13.3.1/benchmarks/batch-c96c2d089400481a913a4accb6f1650f/bridges/benchmark-example3-bridge-rust-baseline/bridge.json --allow-reference-work --allow-reference-escalation --output-root .development/13.3.1/benchmarks
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_solute_benchmarks.py --cases reference/solute_verification/CASES.json --verify-only --output-root .development/13.3.1/benchmarks --batch .development/13.3.1/benchmarks/batch-043cf43e84c04448a38e452670278cba
```

CASES SHA-256: `d3b06531c06545ca89956b95c487bb1616ace1470ba1a8c02a3f1209ff4e8acf`. The 207 hashes in `.development/13-baseline/protected-hashes.json` were checked before each batch and after completed numerical work; all match. Original source, portability patch, Example 3 fixture and Orchestration 12 artifacts are unchanged. The accepted fresh fourteen-row dry-run `batch-e749c2682b8f4fb48814ec1b3dcbf290` and archived superseded planned batch are preserved.

Producer identities used by the accepted fresh recipe:

| Producer | SHA-256 |
|---|---|
| `tools/check_solute_benchmarks.py` | `ec25618870da808cb1733380bcd1802d6c6ccee624b11a88462f1f45b86c450f` |
| `tools/check_solute_verification_spec.py` | `19adeb6c299cc30b52cedc3350c20c5258e6ec7db7079caaaa3c27f4c06c97e3` |
| `tools/run_solute_verification_reference.py` | `66acb845b447faecc48e8d7feb832290433548887927ba419d7d55602c5e8c08` |
| `tools/solute_analytical_reference.py` | `873d0df6ae69d40c432dd0b39c7cf401f9ff04f3069fb9187e321bc36e219b06` |
| `tools/solute_benchmark_bridge.py` | `60379dcc692b2a4999ee8162b5a0dba38bec5c7c35f9a8bcbbb4808d2efd42aa` |
| `tools/solute_benchmark_lifecycle.py` | `541f10ea76d4c7d53de2817971b25ba04cb3dcabec17adb5f64734a27bbeabec` |
| `tools/solute_reference_cache.py` | `d0021a886bc6054571ab9d8a86716e408380883804fae4c14d288337887e5ce9` |
| `tools/solute_verification_metrics.py` | `0fa6ab1bd172fa1bbe5ad1503bd8ba9734282e443ee235ab82633e8f574cd2c3` |

## Bridge results

Batch: `.development/13.3.1/benchmarks/batch-c96c2d089400481a913a4accb6f1650f`. Both reports were independently authenticated and reproduced by the worker and orchestrator. CLI exit 0 reported `evidence_complete`; this is tooling completion. Both runs completed 2,000 steps and profiles at 0/50/100/200.

| Solver | Compared fields | Maximum field difference | Executable bytes | Runtime seconds | Report SHA-256 |
|---|---:|---:|---:|---:|---|
| fortran | 259142 | 0 | 490736 | 2.477468 | `87ee71a06ede315c58b1b400acca02df9f43f2233b38b29201a15e24eb5eb1ba` |
| rust | 24072 | 1.1084467e-12 | 2172336 | 1.553705 | `fded6f3d9ca86266d95e53d08a9b6b570fbebfc04ce761067d23cdae42357fd7` |

Reports reside at `bridges/<exact-launch-ID>/bridge.json` in the batch; complete run/build provenance and hashes are retained in manifests. The Fortran baseline compiler is GNU Fortran 13.3.0 with `-std=legacy -ffixed-line-length-none`; its underflow/denormal warning was retained. The Rust compiler is rustc 1.96.0-nightly, commit c756124775121dea0e640652c5ee3c89e3dd0eb4. Its approved unoptimized test build uses `-Cdebuginfo=0 -Copt-level=0 -Cdebug-assertions=yes -Coverflow-checks=yes`; no runtime warning occurred. Recorded runtimes include lifecycle handling and are not performance benchmarks.

All Fortran printed scientific fields matched the retained originals exactly; date and calculation-time metadata are excluded by the accepted policy. Rust maximum concentration difference was 1.2656542480726785e-14. Its bridge arithmetic guards are directed discrete-arithmetic bounds, not continuum-accuracy budgets.

The Fortran bridge has no independent endpoint history (`legacy_bridge_no_obs`). The original Rust artifact has no retained executable/build manifest or all-step/endpoint history. New Rust equilibrium sorbed storage, inventory and balance bridge fields are explicitly derived. These unavailable quantities limit bridge claims.

## Component reference admission

Ten rows completed in `batch-043cf43e84c04448a38e452670278cba`; the exact batch passed read-only `--verify-only --batch` with exit 0 and `evidence_verified`. Five complete references were shared across paired solvers without using solver observations as input. Reaction cases cover nine nodes and 33 diagnostic times; diffusion covers 65 nodes and 17 times; scalar Robin covers 129 nodes and 17 times. All cases include initial and three frozen analysis profiles.

| Case | Reference identity | Admitted observable records | Maximum profile uncertainty | Maximum continuum uncertainty |
|---|---|---:|---:|---:|
| diffusion_cosine | `a25ab11db0941cf0f9b2c64e06cf5f3d618f9c84f6f46e4ecb01d6f66ed08323` | 123 | 1.1195805e-16 | 7.4384943e-17 |
| reaction_distinct | `0cc23a6e6fad9f2cea9d1dd668cbdbc24ce5133d40a077b46664d25e4653b78c` | 705 | 8.3660587e-17 | 2.3281784e-16 |
| reaction_equal | `75760660aebbc095b01d176c7c4e656fb74ed5d422b39d1095431d19ab1b6294` | 705 | 1.1623677e-16 | 2.3281784e-16 |
| reaction_zero | `a6eac95416d9af8e27d188e2aeecdac1a2d3feb7b638a887035357ed919a346e` | 705 | 5.615663e-17 | 1.7097435e-16 |
| scalar_robin | `afb0c7e18f48bbbd9816bba8f87161ccb64e19c6e7284547adfdf609d1be013d` | 123 | 1.3523105e-16 | 2.6134419e-16 |

Admission uses component interval arithmetic, analytical truncation and serialization bounds. All concentration uncertainties are below the frozen absolute 1e-7 reference budget; other observable budgets use their declared dimensional scales. Complete per-observable admissions and immutable stage/reference hashes are retained in the cache and batch reference receipts.

## Component solver observations

All ten runs have complete authenticated output records and no runtime warnings. Reaction cases completed 128 steps with 33 profiles and 129 endpoint records; diffusion completed 4,096 steps with 17 profiles and 4,097 endpoints; scalar Robin completed 256 steps with 17 profiles and 257 endpoints. Executable identities match the bridge builds. L1 and L2 use the specified length-normalized mesh weights; maxima below range over initial and analysis profiles and preserve species separately.

| Case | Solver | Species | Maximum L1 | Maximum L2 | Maximum L∞ | Endpoint L∞ | Interior L∞ |
|---|---|---|---:|---:|---:|---:|---:|
| diffusion_cosine | fortran | S1 | 0.00063158795 | 0.00094391504 | 0.0031359326 | 0.0031359326 | 0.002900642 |
| diffusion_cosine | rust | S1 | 7.0831789e-06 | 7.8690138e-06 | 1.1128466e-05 | 1.1128466e-05 | 1.1115061e-05 |
| reaction_distinct | fortran | S1 | 3.0753078e-05 | 3.0753078e-05 | 3.0753078e-05 | 3.0753078e-05 | 3.0753078e-05 |
| reaction_distinct | fortran | S2 | 3.5107755e-05 | 3.5107755e-05 | 3.5107755e-05 | 3.5107755e-05 | 3.5107755e-05 |
| reaction_distinct | fortran | S3 | 3.3835175e-05 | 3.3835175e-05 | 3.3835175e-05 | 3.3835175e-05 | 3.3835175e-05 |
| reaction_distinct | rust | S1 | 0.0011200574 | 0.0011200574 | 0.0011200574 | 0.0011200574 | 0.0011200574 |
| reaction_distinct | rust | S2 | 0.0027973706 | 0.0027973706 | 0.0027973706 | 0.0027973706 | 0.0027973706 |
| reaction_distinct | rust | S3 | 0.00090932365 | 0.00090932365 | 0.00090932365 | 0.00090932365 | 0.00090932365 |
| reaction_equal | fortran | S1 | 3.0753078e-05 | 3.0753078e-05 | 3.0753078e-05 | 3.0753078e-05 | 3.0753078e-05 |
| reaction_equal | fortran | S2 | 4.3963171e-05 | 4.3963171e-05 | 4.3963171e-05 | 4.3963171e-05 | 4.3963171e-05 |
| reaction_equal | fortran | S3 | 4.3819452e-05 | 4.3819452e-05 | 4.3819452e-05 | 4.3819452e-05 | 4.3819452e-05 |
| reaction_equal | rust | S1 | 0.0011200574 | 0.0011200574 | 0.0011200574 | 0.0011200574 | 0.0011200574 |
| reaction_equal | rust | S2 | 0.0026844201 | 0.0026844201 | 0.0026844201 | 0.0026844201 | 0.0026844201 |
| reaction_equal | rust | S3 | 0.00066472439 | 0.00066472439 | 0.00066472439 | 0.00066472439 | 0.00066472439 |
| reaction_zero | fortran | S1 | 0 | 0 | 0 | 0 | 0 |
| reaction_zero | fortran | S2 | 0 | 0 | 0 | 0 | 0 |
| reaction_zero | fortran | S3 | 0 | 0 | 0 | 0 | 0 |
| reaction_zero | rust | S1 | 0 | 0 | 0 | 0 | 0 |
| reaction_zero | rust | S2 | 0 | 0 | 0 | 0 | 0 |
| reaction_zero | rust | S3 | 0 | 0 | 0 | 0 | 0 |
| scalar_robin | fortran | S1 | 0.00023054089 | 0.00028141197 | 0.00068353907 | 0.00064741324 | 0.00068353907 |
| scalar_robin | rust | S1 | 0.00021947031 | 0.00025450657 | 0.00053788583 | 0.00053788583 | 0.00052682941 |

Final inventory and cumulative terms below are absolute errors against independent continuum observables. Phase-separated dissolved/equilibrium-sorbed inventory, same-mesh truth, mesh-quadrature error, instantaneous signed flux, temporal RMS and all diagnostic histories remain in the complete result artifacts. Production and loss are integrated separately from sampled states, never inferred from a closure residual.

| Case | Solver | Species | Final total inventory error | Boundary cumulative error inlet/outlet | Production error | Loss error | Maximum independent balance error |
|---|---|---|---:|---|---:|---:|---:|
| diffusion_cosine | fortran | S1 | 0.0009916875 | 0 / 0 | 0 | 0 | 0.0009379375 |
| diffusion_cosine | rust | S1 | 3.8191672e-14 | 0 / 0 | 0 | 0 | 3.8191672e-14 |
| reaction_distinct | fortran | S1 | 4.9818282e-05 | 0 / 0 | 0 | 4.3368282e-05 | 0.0001118 |
| reaction_distinct | fortran | S2 | 3.0192669e-05 | 0 / 0 | 4.3368282e-05 | 0.00018677439 | 0.00024295 |
| reaction_distinct | fortran | S3 | 4.3647375e-05 | 0 / 0 | 0.00018677439 | 1.575915e-05 | 0.00024618091 |
| reaction_distinct | rust | S1 | 0.0019264988 | 0 / 0 | 0 | 0.0010733473 | 0.0029998461 |
| reaction_distinct | rust | S2 | 0.0015345856 | 0 / 0 | 0.0010733473 | 0.0032159692 | 0.0058239022 |
| reaction_distinct | rust | S3 | 0.0011730275 | 0 / 0 | 0.0032159692 | 0.00010078728 | 0.0023439596 |
| reaction_equal | fortran | S1 | 4.9818282e-05 | 0 / 0 | 0 | 4.3368282e-05 | 0.0001118 |
| reaction_equal | fortran | S2 | 2.2654625e-05 | 0 / 0 | 4.3368282e-05 | 7.5017093e-05 | 0.00014104 |
| reaction_equal | fortran | S3 | 5.6527093e-05 | 0 / 0 | 7.5017093e-05 | 0 | 0.00012599 |
| reaction_equal | rust | S1 | 0.0019264988 | 0 / 0 | 0 | 0.0010733473 | 0.0029998461 |
| reaction_equal | rust | S2 | 0.0023086013 | 0 / 0 | 0.0010733473 | 0.0016186457 | 0.0050005944 |
| reaction_equal | rust | S3 | 0.00038210257 | 0 / 0 | 0.0016186457 | 0 | 0.0020007483 |
| reaction_zero | fortran | S1 | 0 | 0 / 0 | 0 | 0 | 0 |
| reaction_zero | fortran | S2 | 0 | 0 / 0 | 0 | 0 | 0 |
| reaction_zero | fortran | S3 | 0 | 0 / 0 | 0 | 0 | 0 |
| reaction_zero | rust | S1 | 0 | 0 / 0 | 0 | 0 | 0 |
| reaction_zero | rust | S2 | 0 | 0 / 0 | 0 | 0 | 0 |
| reaction_zero | rust | S3 | 0 | 0 / 0 | 0 | 0 | 0 |
| scalar_robin | fortran | S1 | 0.00029949509 | 0 / 0.00018918813 | 0 | 3.5346595e-05 | 0.00052465039 |
| scalar_robin | rust | S1 | 9.2293575e-06 | 0 / 8.4469701e-05 | 0 | 4.7574765e-06 | 7.8202953e-05 |

Distinct/equal reaction cases have small Fortran concentration differences that are often masked by printed precision, while the Rust backward-Euler trajectories show larger errors at this timestep. Both zero-reaction cases reproduce constant concentrations and zero balance residuals in available values; this provides a zero-limit check, not a nontrivial accuracy threshold. Diffusion and Robin discrepancies need the controlled mesh/timestep studies before attributing causes or assigning orders.

## Uncertainty and remaining gates

Fortran concentration columns have four significant digits (`e12.4`); independent metrics retain printed half-unit intervals. Printed coordinates have a 5e-5 half-width. The metrics wrapper has no independent spatial derivative bound, so profile coordinate uncertainty remains explicitly unresolved. Rust evaluates the exact requested mesh. Concentration errors, endpoint errors and continuum-inventory errors must not be treated as interchangeable.

Trapezoidal boundary/reaction integration and history coarsening provide empirical quadrature sensitivity, not a rigorous error bound. Reaction production/loss coarsening differences reach approximately 5.64e-4 in these records; this diagnostic can exceed a reported balance residual. A small core-reported residual or source bridge agreement cannot replace independent balance evidence. Zero/near-zero normalization branches retain absolute error and null relative error; no arbitrary near-zero epsilon was assigned.

Reference admission, schema completeness, scientific solver accuracy, convergence, and M5-V are separate decisions. Solver thresholds remain null. Controlled convergence, attribution of persistent differences, reviewed error budgets and independent holdout evidence remain pending. No physical validation or general HYDRUS release-compatibility claim is made.

## Full Example 3 reference and solver outcome

The one-time activated batch is `.development/13.3.1/benchmarks/batch-252f19ab8f56442a808e16e3bbad714b`; its authenticated reference identity is `22a684a825245213ea48b0afab6ac34524dce0dedd1ae05d4c0b228ac0789916`. The durable ledger records `reference_complete`, status `admitted`, at 2026-09-28T04:43:52.970196Z. `reference.json` SHA-256 is `a816584a7314c7ec65026f29204fe073314be4786e4b553632568747430f679c`. All 4,233 grouped profile/continuum observable records were admitted. Maximum empirical uncertainty was 5.4426446242266274e-9 (5.44% of its applicable observable budget); the largest concentration uncertainty was 5.4426446242266274e-9 against 1e-7. Admission is based on inversion-order, precision, recurrence and input sensitivity, not rigorous remainder bounds. This admits reference uncertainty only, not solver accuracy.

The ledger shows exactly five full-case evaluations, with no repeated or interrupted stage and no order-512 escalation: 192/400 full (start 01:06:21.635470, complete 01:22:25.900143 UTC; checkpoint SHA-256 `f2980a9144bb9cc0a2b32c18bf569c4cdce2265b8696c0d7d0f0d58af3fae74d`); 256/400 full (start 01:22:25.931526, complete 01:43:58.199347; `ce16a2f697ddf906cf74fa2db44d9cee5c0834ba08686d0aafc32ef28234e176`); 384/400 full (start 01:43:58.230971, complete 02:16:21.150080; `241301fc63b68af7fb3333953872807482372e138917ad7393a47bfe223ff2bf`); 384/650 precision (start 02:16:21.308942, complete 04:13:11.452855; `fe056ef8904e967a17a0d021939d825caf7d633b7b003cbe63a7e6b48aa00ad2`); 384/400 binary inputs (start 04:13:11.468625, complete 04:43:52.114176; `958b899bf530546e556dbfd9c95eea1140e9b33f591704cb47d8a04778d80828`). Stage files are retained under `benchmarks/reference-cache/<identity>/`; final admission receipt and hash-chained work ledger are alongside them.

The full-case benchmark command then made its single authorized Fortran start, `benchmark-example3-component-fortran-baseline-b40c1111dbf8`, at 04:43:54.237249Z. GNU Fortran 13.3.0 used `-std=legacy -ffixed-line-length-none`; executable SHA-256 `39cd3ecfb3c4971d1a3376f1c324883bdfee0e203157920c5231ec7613983c4e`; retained IEEE underflow/denormal warning. Process exit was 0, but manifest status is `failed`: adapter validation reported `endpoint observation ids/order mismatch`; endpoint/profile records were not accepted and no scientific metrics were produced. Manifest SHA-256 is `175c7b265a71796ef84232f6bd0e861ecc2210af523ae6423c903fd755d6727b` at `runs/benchmark-example3-component-fortran-baseline-b40c1111dbf8/manifest.json`. The Rust row has no start. No retry or producer/core/case change was made. The full transient solver comparison is therefore unmet, despite the admitted reference.

The allowed read-only command `PYTHONDONTWRITEBYTECODE=1 python3 tools/check_solute_benchmarks.py --cases reference/solute_verification/CASES.json --verify-only --output-root .development/13.3.1/benchmarks` was run once by the orchestrator; it exited 2 on the failed full-case batch with the endpoint observation mismatch. In the same invocation it reported component batch `batch-043cf43e84c04448a38e452670278cba` as `evidence_verified`, ten rows, and launch accounting 13/77 (64 remaining). The earlier component-only `--verify-only --batch .../batch-043cf43e84c04448a38e452670278cba` had exited 0; the whole output root is no longer fully verifiable because it includes the failed full batch. No command was rerun here.

Protected-hash verification after all numerical work found 207/207 original files present and unchanged. No additional full-case solver metrics can be inferred from the raw failed run. The sole failed start consumes one slot: 13/77 consumed, 64 remaining; no convergence or holdout starts are in this task. Solver thresholds remain null. Coordinate uncertainty for Fortran profiles, empirical quadrature/coarsening sensitivity, controlled convergence, discrepancy attribution, reviewed error budgets and independent holdout evidence remain unresolved; M5-V remains open.

## Budget and artifact index

At final verification, 13 of 77 scientific slots were consumed and 64 remained. Twelve bridge/component starts completed; one full-case Fortran start exited 0 but failed validation; no retry occurred and the Rust row did not start. The bridge/component nominal reservations total 89,571,744 bytes; the full two-row reservation is 236,487,472 bytes. Each solver retains a 60-second ceiling and aggregate storage cap is 2,699,462,776 bytes. Analytical stage work is recorded separately and does not consume solver slots.

Task-local preflight/review records:

- `.development/13.3.1/preflight-{bridge,component,example3}-20260927.json`: protected hashes, activation/policy identities and launch/storage checks.
- `.development/13.3.1/bridge-review-20260927.json`: authenticated bridge reports, all comparison groups, warnings, executable sizes/hashes and provenance.
- `.development/13.3.1/component-review-20260927.json`: per-case/species metrics, all result/run/reference hashes and completeness.
- `.development/13.3.1/launch-ledger.jsonl`: every solver start, exit and validated completion.
- `.development/13.3.1/benchmarks/reference-cache/<identity>/`: exact request identities, mathematical-stage ledgers/checkpoints, admissions and published references.

Batch recipes, activation bytes, narrowed receipts, reference bindings, manifests, raw outputs, observations, derived results and summaries remain immutable in task scratch. Only this independently authored report and its TODO are intended for the task commit. Final budget and verification status will be added after full work concludes.

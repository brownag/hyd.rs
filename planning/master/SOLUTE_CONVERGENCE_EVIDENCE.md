# Solute convergence evidence

## Current status

The first authorized batch, spatial refinement for `diffusion_cosine` with the Fortran baseline solver, produced four retained raw runs. Each process exited 0 and lifecycle recorded `raw_complete`, but the convergence CLI then stopped at its completeness gate: `require(manifest["status"] == "complete", "failed/incomplete run cannot be retried")`. The accepted Fortran adapter records validated runs as `raw_complete`; the accepted `ConvergenceEvidence.derive` path accepts that state. This CLI/lifecycle state mismatch blocked batch derivation after execution.

No accepted normalized metric results, classification, or batch summary were produced. No convergence rates or scientific findings can be reported from this batch. Scientific status is **unmet**; thresholds remain null and M5-V remains open. The four runs and failed batch are preserved. No retry, resume, verify-only, or subsequent batch was attempted after the failure.

## Activation, command, and budget

Root reviewed the exact 57-row matrix and 15 batch activations in `.development/13.4.1/activation-review.json` (SHA-256 `41a7c3c3ed56fc2591bdacf3c0e164b42dc03dad1928fb6af8b2cd7db3ad4d03`). The first activation `.development/13.4.1/activations/diffusion_cosine-space-fortran.json` has SHA-256 `9215a5c8d1d83f4947cf76c2835e793356e206e00920bec6bc5f09cb20453ae3` and authorizes these four IDs only:

- `convergence-diffusion_cosine-space_17-fortran-baseline`
- `convergence-diffusion_cosine-space_33-fortran-baseline`
- `convergence-diffusion_cosine-space_65-fortran-baseline`
- `convergence-diffusion_cosine-space_control-fortran-baseline`

Exact invocation:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_convergence.py --cases reference/solute_verification/CASES.json --output-root .development/13.4.1/study --execute --activation-file .development/13.4.1/activations/diffusion_cosine-space-fortran.json --bridge-report .development/13.3.1/benchmarks/batch-c96c2d089400481a913a4accb6f1650f/bridges/benchmark-example3-bridge-fortran-baseline/bridge.json --bridge-report .development/13.3.1/benchmarks/batch-c96c2d089400481a913a4accb6f1650f/bridges/benchmark-example3-bridge-rust-baseline/bridge.json --prior-cache-root .development/13.3.1/benchmarks/reference-cache --allow-reference-work
```

Escalation was false and not enabled. The prior 13.3.1 cache was read-only. All four IDs are consumed; no automatic retry or resume is allowed. The ledger `.development/13.4.1/launch-ledger.jsonl` contains four starts, four exit-0 events, and four `complete` events with `raw_complete` manifests (SHA-256 `887b0a9c2b602b892ae6381978255168b5a4a243cc1e880b46ea3bb45170e2ac`, 3,539 bytes). Combined with 13 earlier starts in 13.3.1, the global count is 17/77 and 60 remain; convergence has consumed 4/57. The prior failed Fortran Example 3 benchmark slot remains among the original 13 consumed starts.

At the recorded storage check, `total_storage()` measured 239,017,858 bytes across lifecycle phase roots, against the 2,699,462,776-byte global cap. Root's activation review recorded 214,850,214 bytes before this batch and a 250,000,000-byte aggregate ceiling for new reference/batch evidence plus outstanding output reservations. This storage figure is an operational accounting snapshot, not a scientific result.

## Retained batch and references

Batch: `.development/13.4.1/study/batch-e92dd5b995e443acbe7c231c962548c4`.

The recipe binds the activation, producer hashes, both reviewed bridge reports, `allow_reference_work: true`, `allow_reference_escalation: false`, the read-only prior cache root, null bounds, null thresholds, and `scientific_accuracy_accepted: false`. Recipe SHA-256: `8fa3106dc613d1b6d6a1288e594e1ba276f16642c38c2db7fdb15baf0d736452`. The authenticated bridge gate is passed with false scientific acceptance (SHA-256 `477a814b3ee9e8cc96c80a5789c2a6537d8eb2cd70a19b89b3ee6f7e223fc9cc`).

Three exact analytical reference identities were materialized and admitted before the solver starts. Their immutable wrapper paths and hashes are:

| Identity | Wrapper SHA-256 | Reference values SHA-256 |
|---|---|---|
| `00dc8d0afa0ebc198b70bcf1bf0a1d7e93ff46563f1c745ec43d5c35972779ad` | `2b9357f84c1608f0ce40845b9441054d5b8dcfa5af6b236ba0e47f06f4ebefc0` | `23704f86c14021edf42dc17753d6671cc3c6a0c928649f83b227e1aaf78d60e5` |
| `77448d76084707aa49826bbc45e70a8f1fecd76626b8ab138bd8009350b73a01` | `8cf66d4529c6f262c7d25f7f00b8705f33c6b47e762087802fa6e80a4aaf368d` | `29fbe102610a17baa15049b09e1f7cf8d414fbc3fb0ea2e6e94ae4c5064914b5` |
| `a25ab11db0941cf0f9b2c64e06cf5f3d618f9c84f6f46e4ecb01d6f66ed08323` | `79d97ab06caa1adf5c6f31c06073e91054ff207a06e2d6ca3b88603a3fd9dc2d` | `b1ca88d7463af68df5f261177ff15ea7b3fe5d38eeaa46762e70819f0c770725` |

Each wrapper has `status: admitted` and `admission.status: admitted`; these are reference admissions, not solver acceptance. The references remain under the batch's `references/` directory.

## Raw run artifacts

The run manifests, raw observations, and normalization request artifacts are retained under the batch's `runs/` directory. Every manifest records `status: raw_complete` and exit code 0. Manifest and observations hashes:

| Launch ID | Run ID | Manifest SHA-256 | Observations SHA-256 |
|---|---|---|---|
| `convergence-diffusion_cosine-space_17-fortran-baseline` | `...-b55356dd0307` | `1b41f0d889667f4fc949774ea49a03e1de4787358e1f39994ffba1decab18118` | `42b333c395beceb7a1fa528a049eb43a400bf6db317f9047c12b7c013369463e` |
| `convergence-diffusion_cosine-space_33-fortran-baseline` | `...-ce7dffdff459` | `e2f44fdc65310bd057e1bbf9f7559f6d63d10d9faa4291f0a2f4adf7bbba7154` | `04409864ec905035fcc26fee9d1acd1c4ae0285ae338794854bfc6f463b9e53b` |
| `convergence-diffusion_cosine-space_65-fortran-baseline` | `...-98d39de28deb` | `a6b86529ac71f6df3038fe9bd16c084899d54ebf9bc5231542a4ca1a40241209` | `906be9119d3af287087f5e28debb7e962f9399ef751cd0da7165bf551319833a` |
| `convergence-diffusion_cosine-space_control-fortran-baseline` | `...-2c880d97d3e1` | `72479cad27d76b5ae76c0cb342da4044674c40ac5f716613c7f2256dc40483e8` | `af48536061afbc617cf7dbeb6f5c6573d6999fef13458b0eaa9897310b91141d` |

The hashes authenticate retained raw artifacts only. They do not establish analytical error, expected-order behavior, or convergence.

## Scoped checks

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_convergence`: 32 passed. These synthetic classifier tests do not exercise or repair the CLI/lifecycle status mismatch.
- `git diff --check`: passed for tracked changes; explicit trailing-whitespace scan of this untracked report, task TODO, and preactivation handoff found none.
- `--verify-only` was not run against the failed batch: it cannot provide the missing accepted metrics/classification/summary, and the orchestrator explicitly stopped execution after this failure.

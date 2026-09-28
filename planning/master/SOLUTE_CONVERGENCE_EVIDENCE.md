# Solute convergence evidence

## Status

The 13.4.1 wave is **incomplete and stopped at the storage ceiling**. Thirty-two of its 57 authorized convergence starts were consumed across eight activated batches. Seven batches have accepted classification summaries and passed CLI `--verify-only`; one Fortran time batch stopped on lifecycle validation failure. The final Example 3 spatial batch was classified and verified, including its Fortran failure and all four Rust runs. No other batch was started after the activation bundle's tenth group was processed.

This is retained numerical evidence, not scientific acceptance. Bounds and thresholds remain null, `scientific_accuracy_accepted` remains false, M5-V remains open, and the six holdout rows were not activated or used.

## Activation and frozen scope

The frozen `CASES.json` SHA-256 is `d3b06531c06545ca89956b95c487bb1616ace1470ba1a8c02a3f1209ff4e8acf`. Root reviewed the exact 57-row, 15-batch activation bundle in `.development/13.4.1/activation-review.json` (SHA-256 `41a7c3c3ed56fc2591bdacf3c0e164b42dc03dad1928fb6af8b2cd7db3ad4d03`). The storage amendment `.development/13.4.1/storage-amendment.json` raised only new evidence growth from 250,000,000 to 450,000,000 bytes above its recorded baseline of 214,850,214 bytes. It did not change launch identities, reference permissions, solver limits, global caps, or holdout reservation.

All starts used exact rows from the reviewed activations, 60-second timeouts, and no retries. Reference escalation remained false; the accepted 13.3.1 cache remained read-only. The 13.4.1 batches are retained at `.development/13.4.1/study/batch-*`; exact per-run manifests and observations are beneath their `runs/` directories. Execution and verify logs are in `.development/13.4.1/`.

## Batch outcomes

| Case / axis / solver | Started | Outcome |
|---|---:|---|
| `diffusion_cosine` / space / Fortran | 4/4 | All processes exited 0 and produced validated `raw_complete` runs. Recovered through the accepted evidence API after the CLI rejected Fortran's `raw_complete` state at its `complete` gate. Classified inconclusive; null bounds and thresholds prevent supported rates. Verify-only passed. Batch `batch-e92dd5b995e443acbe7c231c962548c4`. |
| `diffusion_cosine` / space / Rust | 4/4 | All complete. CLI classification hit a helper path/dict argument error; classification was recovered through the accepted API. Verify-only passed. `batch-0b6f80fccad0460580a5c42cd2c8135b`. |
| `diffusion_cosine` / time / Fortran | 4/4 | Three runs are `raw_complete`; control failed validation with `water inventory mismatch`. Preserved without retry. No accepted batch classification. `batch-0a120fb95b2549128d683305726b86c8`. |
| `diffusion_cosine` / time / Rust | 4/4 | All complete, classified through the accepted API after the CLI helper path/dict error, and verify-only passed. `batch-c6e8e54eea484e3ea24053adba30a5c3`. |
| `scalar_robin` / space / Fortran | 4/4 | All `raw_complete`; classified through the accepted API after the CLI rejected the manifest state. Verify-only passed. `batch-d5138e6f0afd47d598aa87a215a18646`. |
| `scalar_robin` / space / Rust | 4/4 | All complete, classified through the accepted API after the CLI helper path/dict error, and verify-only passed. `batch-cf4926cc1252462da27bc054932f8ab0`. |
| `scalar_robin` / time / Fortran | 1/4 | First run failed lifecycle validation with `incomplete step inventory`; the remaining three rows were not started. Preserved without retry. `batch-8cc54a32b4274287a8df86950a8c93f6`. |
| `scalar_robin` / time / Rust | 4/4 | All complete, classified through the accepted API after the CLI helper path/dict error, and verify-only passed. `batch-f4fae0ba6ce84e4789c8446855068080`. |
| Example 3 / space / Fortran | 3/4 | 251- and 501-node runs are `raw_complete`; 1001-node run failed validation with `endpoint observation ids/order mismatch`; control was not started. Preserved without retry. No accepted classification. `batch-96dad43bcd3f43c7b87cfc16953f065f`. |
| Example 3 / space / Rust | 4/4 | All complete, classified through the accepted API after the CLI helper path/dict error, and verify-only passed. `batch-991d7748895f445ab6a2b41323a4a783`. |

The seven batches with classification summaries are the first, second, fourth, fifth, sixth, eighth, and tenth rows. Classification or verify-only establishes artifact consistency only; it does not establish convergence or accuracy.

## Accounting and stop condition

The shared ledger records 49/77 global starts: 13 benchmark starts plus 36 convergence starts, of which four were from the prior 13.3.1 component wave and 32 were consumed by this 13.4.1 wave. Thus 28 global slots and 21 convergence rows remain unstarted. Six holdout rows remain reserved. Started-row 2x output reservations total 1,164,835,816 bytes.

The lifecycle `total_storage()` measurement across phase roots is 914,651,636 bytes. Against the amended growth baseline of 214,850,214 bytes, current growth is 699,801,422 bytes, exceeding the amended 450,000,000-byte ceiling by 249,801,422 bytes. The 2,150,694,968-byte frozen 2x reservation for all convergence rows was already part of activation review; remaining exact reservations cannot make more work fit under the ceiling. No further numerical or reference stage was started after this condition was identified. This storage overrun is an operational accounting breach; it is not scientific evidence.

## Tooling and scientific limits

- Seven retained successful batches passed CLI `--verify-only`; the Example 3 Rust verification log ends with `evidence_verified`. The Fortran batches with raw-complete status required accepted API recovery because the CLI expects `complete`; Rust CLI classification attempts encountered a path/dict argument mismatch. No producer source was changed.
- The accepted classifier unit suite had 32 passing tests earlier in the wave. These tests and verify-only runs check tooling/artifact integrity, not scientific accuracy.
- No fitted convergence order, assigned threshold, or accuracy acceptance is reported. The authorized matrix is unfinished, failed rows remain consumed, and unresolved uncertainty/error floors and M5-V prevent acceptance.
- Activations 11–15 were not started; six holdout rows were untouched. No automatic retry or resume was used.

## Next work

This wave cannot continue under its current storage amendment. Any further numerical execution requires a fresh root review of lifecycle storage, reservations, and an amended bounded plan before activating work. Preserve all existing run artifacts and failed attempts; do not reuse consumed IDs.

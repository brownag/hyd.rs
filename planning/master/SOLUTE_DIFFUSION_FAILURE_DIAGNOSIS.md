# Solute pure-diffusion diagnosis

## Scope and execution record

This bounded reproduction used only `diffusion_cosine`, with 65 nodes and S1. Its earliest failing retained concentration profile is profile 1 at `t = 0.0625 h`; the authenticated retained pair has Fortran manifest SHA-256 `63160c89f6f27f39923c99181a229671d87023342e3dece3b20fcb6fd5b77893` and Rust manifest SHA-256 `bed3099a49394176e3aba0dfb799c6ca7c713b7b64755407636a25845243e914`. Retained input hashes were checked before and after the diagnostic; task-local validation verified all 58 ledger entries unchanged. Prepared Rust and Fortran initial arrays matched exactly (maximum absolute difference 0). Both task-local inputs were bounded at 256 fixed steps to 0.0625 h (`dt = 0.000244140625 h`); no retained 4096-step schedule was run.

Preparation command (zero launches):

```text
python3 .development/15.2.1/trace_runner.py --prepare --cases reference/solute_verification/CASES.json --inventory planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json --profile-time 0.0625 --task-root .development/15.2.1 --max-artifact-bytes 104857600
```

Build command: `python3 .development/15.2.1/trace_runner.py --build --task-root .development/15.2.1 --max-artifact-bytes 104857600` — Rust and Fortran builds exited 0. Rust binary SHA-256: `dd4ca411d8ad646b25b131401e4b07759b3218eafa40cacdca9ade5e8a2301d1`; Fortran binary SHA-256: `ec18700e2332c8d7c1c80b8719fb34f21c358e1f48656f90c972e9e68c1f04a6`.

The separately invoked Rust command was `python3 .development/15.2.1/trace_runner.py --run --solver rust --case diffusion_cosine --task-root .development/15.2.1 --max-total-launches 2 --timeout-seconds 60 --max-artifact-bytes 104857600`. It exited 0; the harness reports 1 passed and produced profiles at 0 and 0.0625 h. Its first accepted timestep was `0.000244140625 h`.

The single Fortran command was `python3 .development/15.2.1/trace_runner.py --run --solver fortran --case diffusion_cosine --task-root .development/15.2.1 --max-total-launches 2 --timeout-seconds 60 --max-artifact-bytes 104857600`. It exited 2 after 0.072192 s. `Error.msg` says `Open file error in file: .\Selector.in`; stderr then reports EOF at `HYDRUS.FOR:771` while waiting at `Press Enter to continue`. The prepared task-local deck included `SELECTOR.IN` and `solute-input\Selector.in`, but not the exact `.\Selector.in` Linux filename that this legacy path requested. Fortran emitted no matrix trace and did not reach its first solute solve. Its actual first accepted timestep and its profile at 0.0625 h were therefore not observed.

The launch ledger preserves both process attempts. Rust exited 0 but the runner failed while finalizing its ledger row: `FileExistsError: [Errno 17] File exists: '/home/andrew/AI/hyd.rs/.development/15.2.1/launch-results.json'`. The process had completed and its stdout, stderr, and observations were present. The row preserves that runner-finalization failure and was reconciled from those captured outputs without rerunning Rust. The task-local runner's only post-Rust correction was atomic replacement in `writej`; it did not change launch selection or attempt counting. No solver process was launched after the failed Fortran attempt.

## Rust trace and retained profile evidence

Rust trace values use cm, hours, and `C_ref`; node indices are zero-based. For this case `q = 0`, `upstream_weighting = false`, and reaction rates are zero, so advective/upstream weighting is inactive. The Rust request uses `D = 0.2 cm²/h` on every face and `theta = 0.43`. The first-step trace contains one pre-solve row for each of 65 nodes. At zero-based interior node 32, `dt = 0.000244140625 h`, `D_left = D_right = 0.2`, lower/upper are `-2.752`, diagonal is `115.584`, and RHS is `110.08`. At endpoint node 0, lower is 0, diagonal `57.792`, upper `-2.752`, and RHS `68.8`; at node 64, lower is `-2.752`, diagonal `57.792`, upper 0, and RHS `41.28`. The matrix already differs from a purely aqueous storage coefficient because the Rust mass derivative includes equilibrium sorption (`theta + rho*Kd = 0.86`); these Rust-only values do not establish a Fortran coefficient difference.

In the retained printed-parity records at 0.0625 h, 10 of 65 concentration values fail their Fortran print intervals. Every failed signed difference (`Rust - Fortran`) is negative, ranging from `-5.1458058e-5` to `-9.7513698e-4 C_ref`; failed nodes are 41 and 53, and 57–64. The other 55 records pass their printed intervals. This reproduces the asymmetric negative tail signature described in the research, but it is profile-level evidence and does not isolate the cause. The new Rust reproduction reaches the same profile time, but no same-task Fortran result exists to align with it.

## Finding, limits, and next discriminator

Supported: the Rust effective face dispersion is uniform at 0.2 cm²/h, water content is 0.43, q is zero, upstream weighting is disabled, and its first-step assembly is logged. The retained first failing profile contains a negative residual tail at the lower-concentration end.

Not established: Fortran's effective dispersion, finite-element coefficients, endpoint rows, RHS, solved state, or advancement for this bounded input. No causal attribution to the spatial stencil, mass weighting, boundary rows, or solve is supported by this failed reproduction. In particular, the observed residual sign alone does not prove which implementation is incorrect.

Smallest next step: after a new run authorization, prepare the legacy Fortran input under the exact Linux path `.\Selector.in` (and verify all related runtime path aliases against the reference program's `cDataPath` convention), then run only the single bounded Fortran reproduction through 0.0625 h. Compare its first assembled rows and accepted time to the preserved Rust trace. This task has already consumed its two authorized process attempts; no replacement launch was made.

## Provenance and checks

- Original Rust solver source SHA-256: `bed1cd1756427f0120c60958f9bfb5cd0bde332e7225f61481c454d92b6564b0`; instrumented task-local Rust source SHA-256: `3c4868f6b108ab8d7b98a7ea20f474305653cc522668577b3941a857831147bb`. Original Fortran `SOLUTE.FOR` SHA-256: `81fed6adbbd18aabe53385151166ab36d0df504a7939b54e10460c2254488910`; task-local patched/instrumented `SOLUTE.FOR` SHA-256: `b03289769d44652e61ba860a3e68f187485cb96dc7af40803ca748eed7091b1c`. Other original Fortran sources and canonical retained request/deck/profile hashes are in `.development/15.2.1/provenance.json`; task-local copied source, executable, and input hashes are in `.development/15.2.1/prelaunch-verification.json`.
- Corrected task-local runner SHA-256: `8a7e140cff7439ae4527fa6f3eaaaa1c86882c232f72e2218b518ee67f9a79fd`. It atomically finalizes the existing launch ledger row and validates recorded input/source/log hashes; the prior runner hash and captured finalization exception are preserved in `.development/15.2.1/launch-results.json` and the orchestration worker report.
- Validation command: `python3 .development/15.2.1/trace_runner.py --verify --task-root .development/15.2.1 --max-artifact-bytes 104857600` — passed integrity checks, recording status `validated_with_fortran_preassembly_failure`, 2 process attempts, 58 retained hashes verified, 65 Rust trace rows, and 0 Fortran matrix rows.
- Final task artifact size was 27,692,887 bytes, below the 104,857,600-byte cap. No broad test suite ran, and no production solver or reference source was changed.

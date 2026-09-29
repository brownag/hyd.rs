# State — Orchestration 15

Plan status: **Approved by user on 2026-09-29.**
Execution status: **Waves 1–3 have run within task bounds. Reaction and scalar Robin reports are complete. Diffusion and Example 3 reports record missing Fortran operator traces; no retries were made. Wave 4 synthesis is pending. No production solver changes are authorized or made.**
Plan: [EXECUTION_PLAN_15.md](EXECUTION_PLAN_15.md). Worker template: [PROMPT_TEMPLATE_15.md](PROMPT_TEMPLATE_15.md).

## Baseline and guardrails

- [x] Read-only planning reconnaissance completed 2026-09-29.
- [x] Last recorded baseline from Orchestration 14: 145 Rust tests; 13 runner/outlet Python tests; 17 benchmark tests; 86 convergence tests; fmt, Clippy with warnings denied, and diff checks passed. Not rerun for this plan.
- [x] Authenticated parity evidence reports 9,092 concentration failures in five cases and zero in `reaction_zero`.
- [x] Production solver/reference sources, casebook, retained `.development/13.*` and `.development/14.*` artifacts, and prior orchestration records are immutable under this plan.
- [x] User approved the bounded diagnostic run protocol on 2026-09-29.
- [x] Exact per-task run commands, counts, timeouts, and storage bounds reviewed before each execution wave.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 1 | 15.1.1 | Reaction first-step trace and cause classification | completed_with_launch_overrun |
| 2 | 15.2.1 | Pure-diffusion diagnosis | completed_with_fortran_preassembly_blocker |
| 2 | 15.2.2 | Scalar Robin diagnosis | done |
| 3 | 15.3.1 | Example 3 mapping/checkpoint diagnosis | completed_with_missing_fortran_operator_trace |
| 4 | 15.4.1 | Evidence synthesis and targeted-fix pathways | pending |

## Task reports

## 15.1.1 — Reaction-only first-step trace
- Status: preflight passed after one user-approved correction; the first attempt assumed all three reaction cases shared the same S2 initial profile, but `reaction_zero` uses S2 = 0.25.
- First command failed before builds/runs: `python3 .development/15.1.1/trace_runner.py --prepare --cases reference/solute_verification/CASES.json --inventory planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json --task-root .development/15.1.1 --max-artifact-bytes 104857600`.
- Amendment: user approved correcting and rerunning the non-launching preflight once; no build or solver retries are authorized.
- Corrected preflight: same command — exit 0; six one-step task-local inputs prepared at `dt=0.03125 h`.
- Hashes: `provenance.json` `66a765cfa1848a06e3976ddac386527d383e97fe92aa0273ad0adde94fa6d75c`; `prepared_runs.json` `4a4d7709f79bd88a247b24c0b231f491f5f772cf0d31f5583da1ae36ab685f13`.
- Pre-build artifacts: 686,973 bytes / 104,857,600 cap. All 23 retained manifest/request/source/deck/profile hashes rechecked unchanged; no diagnosis report yet.
- Instrumentation diffs: Rust `rust-solute-instrumentation.patch` SHA-256 `dfdc21f5c7b854242c5ce509eb7abec661a6c52b90c07a756a850bc01f254a34`; Fortran `fortran-SOLUTE-instrumentation.patch` SHA-256 `5cc9d338677e89acd6fe2654a7884238842e02912fb2e83e1b80057133a23b06`.
- Build command: `python3 .development/15.1.1/trace_runner.py --build --task-root .development/15.1.1 --max-artifact-bytes 104857600` — pass; Rust and Fortran builds both exit 0.
- Build binaries: Rust SHA-256 `2a23d12ea4c76c14150211fffdf11cf0388dc9ded0547afd9e50cd6d0a9b7fd5`; Fortran SHA-256 `b1b35df616ffd57d2493f98a8425c47c3e073b03db2a45c8959b83f4d82613b8`.
- Build peak: 43,991,479 bytes / 104,857,600 cap; both compiler commands exited 0.
- First run attempt: the Rust harness process exited 101 before simulation because `HYDRUS_SOLUTE_VERIFICATION_OUT=observations.json` was relative; the harness requires an absolute destination.
- Failed command: `timeout --signal=TERM 60s /home/andrew/AI/hyd.rs/.development/15.1.1/rust-target/debug/deps/solute_verification_harness-9a32a5c4b9de6ece --exact requested_study --nocapture`; elapsed 0.003784 s; request hash `56735a9829814ca4a3c0b3a339c57112bef1bd7c3106619dba0ff4399e4a4b3d`.
- Outcome: one harness attempt failed before solver simulation; no other launch ran and no trace was produced. User then authorized correction of this self-authored configuration issue and one rerun of `reaction_equal/rust`; the original six successful solver simulations remain the cap, and any later failure stops without retry.
- Current task artifacts: 43,994,419 bytes / 104,857,600 cap. All 23 retained hashes rechecked unchanged; `git diff --check` — pass.
- Further launches require a user-approved amendment; the prior amendment authorized only one preflight correction/rerun.
- Corrected harness record: `rust-harness-output-correction.patch` SHA-256 `6452064001a9581ebe53d87bf19c3babc031d0fe56b3093d955d4430188a77ee`; runner SHA-256 `76efa4e2562ecbc5d8586750ae2cc10ad8202ef53addecf0172b9c37abe4b1f0`; provenance SHA-256 `934a27a4071d684e0ab858e5d8f2924cd1d0e278c8d95e5a6a4e0da1dafdf8a0`.
- Rust-only rebuild passed with binary SHA-256 `7c3145a7bbcc2e7c941881acb771d3bbc92cc65e35c3a20aea269bbfee4f39e1`; peak task artifacts during rebuild were 67,529,382 bytes.
- After the one authorized pre-solver retry, six simulations succeeded. Runner accounting defect: it launched a seventh process (distinct/Fortran); no more launches were run. All six intended case/solver combinations completed, plus this unapproved extra run. Preserve all seven rows in `launch-results.json`; do not treat the extra result as part of the approved six.
- Final task artifacts: 59,477,395 bytes / 104,857,600 cap. Corrected harness config now writes Rust observations inside the copied task-local workspace under its allowlisted `.development/15.1.1` subtree.
- Diagnosis: `planning/master/SOLUTE_REACTION_FAILURE_DIAGNOSIS.md`. Inactive `reaction_zero` traces agree nodewise; reaction-enabled cases first diverge in the S1 assembled equation, with larger S2/S3 discrepancies. The report records formulation and print-precision limits; no production fix is made.
- `git diff --check` passes. No broad tests were run.

## 15.2.1 — Pure-diffusion trace
- Status: bounded task complete with an unresolved Fortran pre-assembly input-path failure; user chose the no-retry limit.
- Report: `planning/master/SOLUTE_DIFFUSION_FAILURE_DIAGNOSIS.md`; task-local validation passed, 58 retained hashes unchanged.
- Prep/build passed; Rust produced the 0.0625 h profile and first-step trace. Fortran exited 2 before assembly because `.`/`Selector.in` runtime alias was missing; no operator comparison is claimed.
- Exactly two process attempts are retained (Rust exit 0; Fortran exit 2). The Rust ledger finalization issue and atomic-writer correction are recorded; Rust was not rerun.
- Final task artifacts: 27,692,887 bytes / 104,857,600 cap. No production or retained source changed.
- Next discriminator documented in the report: a separately authorized Fortran-only bounded reproduction with the exact legacy path alias.

## 15.2.2 — Scalar Robin trace
- Status: complete; task-local provenance, source-backed boundary mapping, aligned trace, and diagnosis report accepted.
- Report: `planning/master/SOLUTE_ROBIN_FAILURE_DIAGNOSIS.md`; Rust and Fortran builds passed, exactly two serial solver attempts exited 0.
- First aligned interval is 0.00390625 h; effective D, flux magnitude, storage, and decay agree. The first solve differs in interior stencil and endpoint rows.
- Reverse Fortran node order for comparison: its inlet is i=129 and outlet i=1; Rust inlet is node 0 and outlet node 128.
- Evidence does not isolate upstream weighting from finite-element weighting and boundary assembly; the report gives a common-form discriminator.
- Task-local trace verifier and `git diff --check` passed; 13 authenticated retained inputs remained unchanged.
- Final task artifacts: 48,659,350 bytes / 104,857,600 cap. No production source changed.

## 15.3.1 — Example 3 mapping and checkpoint diagnosis
- Status: bounded run/report complete with a missing Fortran first-step operator trace; no retry under the approved two-attempt limit.
- Report: `planning/master/SOLUTE_EXAMPLE3_FAILURE_DIAGNOSIS.md`; 60 retained files rehashed with zero mismatches.
- Task-local Rust and Fortran builds passed; exactly two serial solver attempts reached the shared 50 h profile and exited 0.
- The 0/50 h profile check reproduces large mixed-sign differences; source-to-field mapping and Fortran/Rust coordinate reversal are documented.
- Rust logged nine representative assembly and nine solved rows. Fortran logged no operator rows because the task-local time guard missed the `Solute` call time; first-step operator divergence remains unproven.
- Verification status: `verified_incomplete_fortran_first_step_trace`; no retry or extra run. The separate 15.2.1 pre-assembly blocker also remains unresolved.
- Final task artifacts: 45,878,333 bytes / 104,857,600 cap. No production source changed.

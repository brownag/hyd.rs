# State — Orchestration 16

Plan status: **Original plan approved 2026-09-30; amendment approved by the user 2026-10-05.**
Execution status: **Waves 1–6 closed; no production code changes.** Both conditional correction branches closed as `no_fix_cause_unisolated`. No task commits were made; planning/evidence artifacts are being preserved in a separate documentation recovery commit.
Plan: [EXECUTION_PLAN_16.md](EXECUTION_PLAN_16.md). Worker template: [PROMPT_TEMPLATE_16.md](PROMPT_TEMPLATE_16.md).

## Baseline and guardrails

- [x] Read-only reconnaissance completed 2026-09-30.
- [x] Committed baseline: `ec90ba6dcff06de1326670d6a0af46c60a0cbde2` on `phase4-water-acceptance`.
- [x] Working tree clean at planning start (`git status --porcelain=v1` empty).
- [x] Prior recorded baseline only (not rerun): 145 Rust; 13 runner/outlet Python; 17 benchmark; 86 convergence tests; fmt, Clippy with warnings denied, diff check passed (STATE_15).
- [x] Read Orchestration 15 plan/state, synthesis, four family reports, printed-parity evidence, Rust/Fortran anchors, casebook, parity tests and repository instructions search.
- [x] Wave 1 reaction and Robin equation normalizations completed; both recorded no-fix because no unique Rust defect was isolated.
- [x] Historical limitations: diffusion Fortran previously failed before assembly; Example 3 Fortran rows were absent and internal endpoints are not common; unintended seventh reaction-distinct launch remains excluded.
- [x] User approved retries and follow-up diagnostic runs for diffusion and Example 3 on 2026-10-04; fixed timeouts and artifact caps must not block work or cause repeated user-input loops.
- [x] User approved conditional implementation where a Rust defect is supported; no-fix is acceptable, with closer investigation deferred to a later session.
- [x] Amendment approved by the user on 2026-10-05. Diagnostic runs use task-local provenance and attempt logs, with no fixed timeout/artifact cap as an acceptance gate.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 1 | 16.1.1 | Reaction equation normalization | done_no_fix_cause_unisolated |
| 1 | 16.1.2 | Scalar Robin row normalization | done_no_fix_cause_unisolated |
| 2 | 16.2.1 | Fortran diagnostic runner setup | complete |
| 3 | 16.3.1 | Diffusion Fortran discriminator | complete; no Rust defect established |
| 4 | 16.4.1 | Example 3 Fortran discriminator | complete; measured first-step rows; no Rust defect established |
| 5 | 16.5.1 | Conditional diffusion correction/no-fix | done_no_fix_cause_unisolated |
| 6 | 16.6.1 | Conditional Example 3 correction/no-fix | done_no_fix_cause_unisolated |

## Task reports

Task reports follow.

## 16.2.1 — Reusable Fortran diagnostic runner
- Status: complete; task-local Fortran-only runner, usage/recovery notes, and 8 offline tests added under `.development/16.2.1/`.
- Retry-safe preparation reserves each attempt and preserves prior/failed staging under `preparations/`; builds and launches reserve unique attempt outputs before staging or execution. No fixed timeout, retry-count, or artifact-size rejection caps.
- Authentication, copy staging, literal Linux aliases (`.\\Selector.in`, `solute-input\\Selector.in`, `solute-input\\Profile.dat`), input/source hashes, ledger output hashes, and actual-time Example 3 `Solute` entry instrumentation are verified.
- `PYTHONDONTWRITEBYTECODE=1 python3 .development/16.2.1/test_trace_runner.py` — pass (8 tests).
- `python3 .development/16.2.1/trace_runner.py prepare --case diffusion_cosine --task-root .development/16.3.1 --dry-run` and `... --case example3 --task-root .development/16.4.1 --dry-run` — pass; matching `verify` commands pass for both fixtures.
- `git diff --check` — pass. No build or solver was run. Two corrected setup attempts remain preserved in `.development/16.2.1/failed-prepare-*`.
- Closed gap: the reviewed diagnostic waves compiled and exercised task-local Fortran instrumentation in both the diffusion and Example 3 cases. Their build and launch outputs are retained in `.development/16.3.1/` and `.development/16.4.1/`.

## 16.3.1 — Diffusion Fortran first-step discriminator
- Status: complete; normalized 65-node S1 Fortran matrix, RHS, timestep, endpoint rows, and solved vector against the retained Rust trace. Full report: `planning/master/SOLUTE_DIFFUSION_NORMALIZED_TRACE.md`.
- The reviewed task-local schedule ended at 0.0625 h with fixed `dt=0.000244140625 h` and 256 steps. Preparation 0007, build 0008, and launch 0009 are preserved. Launch exited 0; 65 matrix rows and 65 post-`BanSol` values were captured. Runner verification passed (57 retained files, 4 aliases, 9 attempts, artifact hashes verified).
- Fortran coordinates were reversed into Rust inlet-to-outlet order. All 63 interior Fortran rows differ from Rust rows by `(+1.37599995327, -2.75199926758, +1.37599995327)` in `(lower, diagonal, upper)`; endpoint closures also differ. The Rust first-step solution was reconstructed from retained pre-solve tridiagonal rows. The maximum state difference is `0.0003047828` at the outlet.
- Disposition: `no_fix_cause_unisolated`. The trace establishes different operator and endpoint formulations but does not show which formulation is required for Rust. Conditional diffusion correction task 16.5.1 was closed as no-fix; no numerical change is supported from parity evidence alone.
- `python3 .development/16.2.1/trace_runner.py verify --task-root .development/16.3.1` — pass. `git diff --check` — pass.

## 16.4.1 — Example 3 Fortran first-interval discriminator
- Status: complete; measured the actual Fortran `Solute` entry time and captured 9 assembly rows and 9 post-solve values across three species and three physical locations. Full report: `planning/master/SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md`.
- Preparation 0005 preserves the full 1001-node, 250 cm, three-species case and 0/50/100/200 h profile schedule. Build 0006 succeeded; launch 0007 exited 0 in 2.049 s after 2000 accepted steps. All preparation/build/launch attempts remain in `.development/16.4.1/attempt-ledger.json`.
- Measured Fortran time was `t=0.10000000149011612 h`, `dt=0.100000001 h`. Coordinate normalization is Fortran `i=1, x=-250` → Rust node 1000/outlet; `i=501, x=-125` → node 500/midpoint; `i=1001, x≈0` → node 0/inlet. In Rust row order, Fortran coefficients map as `(lower=E, diagonal=D, upper=B, rhs=F)`.
- The first-interval Fortran rows and inlet solved values differ from Rust at matched physical locations. The measured Fortran weighting/finite-element rows and outlet closure differ from Rust's finite-volume assembly and endpoint row; this locates the divergence but does not establish which formulation Rust must implement.
- Disposition: `no_fix_cause_unisolated`. Conditional task 16.6.1 was closed as no-fix; no numerical correction is supported from this parity evidence alone.
- `python3 .development/16.2.1/trace_runner.py verify --task-root .development/16.4.1` — pass (59 retained files, 4 aliases, 7 unique attempts, artifact hashes verified). `git diff --check` — pass.

## 16.1.1 — Active-reaction equation normalization
- Status: complete; dimensional Rust nodal balance and Fortran FE assembly comparison recorded.
- Report: `planning/master/SOLUTE_REACTION_NORMALIZED_EQUATIONS.md`; `reaction_zero` retained as inactive control; unintended seventh distinct/Fortran run excluded.
- Rust S1 coefficients reproduce the accepted Rust trace, but retained Fortran coefficients cannot be mapped into the same nodal basis from available evidence.
- Disposition: `no_fix_cause_unisolated`; no production correction justified. Missing discriminator: full FE row/source and mapped phase-specific reaction parameters.
- `git diff --check` passed; no build, test, numerical execution, or solver launch.

## 16.1.3 — Diffusion first-step discriminator
- Historical status: stopped before copies/build/launch because the former one-launch ledger requirements conflicted with the existing task-15 runner.
- The old `.development/16.1.3/prelaunch-ledger.md` and Rust-only report are retained as history; the amended execution is task 16.3.1 using the new task-local driver.
- No solver launch occurred in the prior resume.

## 16.1.4 — Example 3 discriminator
- Historical status: deferred because the prior driver assumed a two-launch limit and 300 s timeout.
- No new run occurred; the amended task is 16.4.1 and will use the shared task-local driver after diffusion completes.
- Existing 50 h profile evidence remains; no first-step Fortran rows or common internal endpoints are available.

## Conditional implementation findings
- Reaction: no-fix because 16.1.1 identified no unique Rust defect; no code/tests/parity run.
- Robin: no-fix because 16.1.2 identified no unique Rust defect; no code/tests/parity run.
- Diffusion task 16.3.1 and Example 3 task 16.4.1 have discriminators, but neither establishes a specific Rust defect. Conditional tasks 16.5.1 and 16.6.1 are closed as `no_fix_cause_unisolated`; no production source or tests were changed and no implementation-branch test or parity rerun was applicable.
- No physical/continuum claim is made.

## 16.5.1 — Conditional diffusion correction/no-fix
- Status: closed `no_fix_cause_unisolated`; no Rust source or test edits.
- The normalized first-step trace shows a consistent interior row difference and different endpoint closures, but does not establish which formulation Rust is required to implement. See `planning/master/SOLUTE_DIFFUSION_NORMALIZED_TRACE.md` and the 16.3.1 evidence above.
- The correction-only regression, unit test, and parity commands were not run because the discriminator did not identify a Rust defect. Existing interval and negative-tail evidence remains recorded in the 16.3.1 report.
- `git diff --check` — pass.

## 16.6.1 — Conditional Example 3 correction/no-fix
- Status: closed `no_fix_cause_unisolated`; no Rust source or test edits.
- The actual-entry trace shows first-interval row and state differences at matched physical locations. The measured Fortran two-level finite-element assembly and outlet closure differ from Rust finite-volume rows, but the trace does not isolate a Rust translation or assembly error. See `planning/master/SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md` and the 16.4.1 evidence above.
- The correction-only regression, unit test, and parity commands were not run because the discriminator did not identify a Rust defect. Existing comparison is limited to the common 0/50/100/200 h printed profiles; no internal-endpoint parity is inferred.
- `git diff --check` — pass.

## 16.1.2 — Scalar Robin row normalization
- Status: complete; normalized retained inlet, midpoint, and outlet rows in physical inlet-to-outlet order.
- Report: `planning/master/SOLUTE_ROBIN_NORMALIZED_ROWS.md`; Fortran index reversal and coefficient mapping are explicit.
- Separated known transport/storage values and attributed remaining differences to unresolved upstream weighting, FE/FV spatial/time assembly, and endpoint closure interactions.
- Disposition: `no_fix_cause_unisolated`; no unique Rust defect follows from the two-run trace.
- `git diff --check` passed. No builds, tests, scripts, solvers, or production changes.

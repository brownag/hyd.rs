# TODO

## Goal
Prove the mechanism of the vapor-on head drift in `vapor_heatbc_fd_on` (and FF) by locating the first step at which the Rust vapor driver diverges from the instrumented Fortran reference, and naming the code path that produces it. No source change in `src/` or `src_reference/`.

## Decisions (user, 2026-11-13)
- Scope: prove the mechanism only. Any fix is a separate, later approval.
- Fortran instrumentation: staged copy under `.development/26-drift/` built with the existing harness. Pinned source and committed fixtures untouched.
- Rust instrumentation: env-gated, in a scratch worktree of the branch. Not committed to the feature code.
- Tracking: planning artifacts are **tracked**; planning-only commits use `docs(planning):`.
- Branch: new branch `orchestration/26-drift` created from `feat/heat1` at `71ba3d3`. `feat/heat1` is not modified.
- If step counts do not align, report divergence at output times 6, 24, 72, 240 instead of every step.

## Tasks

### 1. Branch and scratch layout
- [x] Create branch `orchestration/26-drift` from `feat/heat1` (`71ba3d3`). Confirm `git status --short` shows only the two pre-existing untracked files (`EXECUTION_PLAN_26.md` and `PROMPT_TEMPLATE_26.md` are now tracked, so expect a clean tree).
- [x] Create scratch root `.development/26-drift/` (untracked by `.gitignore`; verify with `git check-ignore`).
- [x] Create a detached scratch worktree of the branch at `.development/26-drift/rust-wt` for Rust instrumentation. Verify `git worktree list` shows it.

### 2. Instrumented Fortran reference build
- [x] Stage the pinned source with `python3 tools/stage_fortran_reference.py --output .development/26-drift/fortran-ref`. Record the staging manifest hash.
- [x] Add a per-iteration dump to the staged copy only: in `WATFLOW.FOR` after each solve and in `HYDRUS.FOR` at the coupling loop, write time, dt, iteration index, and `hNew`, `TempN` at nodes 2, 3, 4, 5 to `DUMP_ITER.OUT`. Gate the dump by a unit that is opened only when a flag file `DUMP_ON` exists in the run directory, so the default output is unchanged.
- [x] Build with `sh tools/build_fortran_reference.sh --build-dir .development/26-drift/fortran-ref`. Record the executable SHA-256 and compare with the harness build used for the fixtures (`e6d69dd3…`). Expect a different hash because of the dump; record both.
- [x] Non-intrusion check: run the instrumented executable on `vapor_heatbc_fd_on` with the dump flag off. Its standard outputs must match `reference/vapor_fortran/fixtures/vapor_heatbc_fd_on.json` at the printed times to the printed precision (compare parsed values, not file hashes, because `NOD_INF.OUT` carries a timestamp).

### 3. Reference per-step trace
- [x] Run the instrumented executable on `vapor_heatbc_fd_on` and `vapor_heatbc_ff_on` with the dump on. Output `DUMP_ITER.OUT` under `.development/26-drift/fortran-run/fd_on/` and `ff_on/`.
- [x] Parse each dump into a table: step, time, dt, Fortran iteration index, and head/temperature at nodes 2–5. Record step count (expect 2400 at constant 0.1 h per `RUN_INF.OUT`).
- [x] Verify the dump's head and temperature at the printed output times equal the fixture values to printed precision. Stop and report if they do not.

### 4. Rust per-step trace
- [x] In `rust-wt`, add env-gated dumps (`HYD_DRIFT_DUMP=<path>`) of time, dt, and heads and temperatures at nodes 2–5 for each accepted vapor step, plus pass-one and pass-two heads and temperatures, in `crates/hydrus-core/src/heat/vapor/driver.rs` at the step commit (around the `stage` call). Dump only; no change to values.
- [x] Verify the Rust dump is inert: run `cargo test -p hydrus-core --test vapor_heat_boundary_parity` in `rust-wt` with and without the variable. Same pass/fail and same candidate values.
- [x] Run the Rust fd_on case with the dump on. Output under `.development/26-drift/rust-run/`.
- [x] Record Rust step count and dt schedule; compare with the Fortran schedule from task 3.

### 5. Alignment and first-divergence search
- [x] Align Fortran and Rust steps by time. If counts or dt differ, fall back to output-time comparison (decision above) and record why.
- [x] Compute per-step max |Δh| and |ΔT| over nodes 2–5 for fd_on and ff_on, and over the vapor-off controls.
- [x] Define divergence as the first step where |Δh| > 1e-8 cm or |ΔT| > 1e-6 °C (about 10× the vapor-off control noise of ~4e-9 cm). Record the first such step for each case in a table.
- [x] Confirm the vapor-off FD control never crosses the threshold (control for the method). **Deviation:** no Rust vapor-off dump exists (the legacy driver does not use the vapor commit). Replaced by the double-precision check on the vapor-off Fortran, which agrees with its fixture at printed precision.

### 6. Localize the first divergence
- [x] At the first divergence step, compare the step's inputs in order: old temperatures, old heads, predictor, pass-one heads, pass-one temperatures, pass-two heads, post-solve temperatures. The first quantity that differs is the candidate source.
- [x] Map that quantity to the code path: **Done** (pass-one WatFlow head, same inputs; see the mechanism document). Fortran `WATFLOW.FOR` / `TEMPER.FOR` / `HYDRUS.FOR` line and the Rust function in `heat/vapor/driver.rs`, `water_solver.rs`, `heat/vapor/adjust.rs` or `water.rs`. Cite file:line on both sides.
- [x] Test the candidate: a scratch-only change on the Rust side that copies the Fortran value at that point into the Rust step. If the drift vanishes at that step, the mechanism is confirmed. Record the result. **Deviation:** not run as written. Replaced by a real*8 Fortran build (`fortran-ref-dbl`), which tests the same hypothesis more directly: the drift vanishes. that copies the Fortran value at that point into the Rust step. If the drift vanishes at that step, the mechanism is confirmed. Record the result.

### 7. Verdict and record
- [x] Write `planning/master/D-26.2_DRIFT_MECHANISM.md` with: the first divergence step per case; the localized quantity; file:line on both sides; the scratch-test result; and an explicit statement of what is proven and what is not.
- [x] If the mechanism is not proven, state what was ruled out and what remains, and do not claim a cause.
- [x] Update exception 1 in `planning/master/M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md` with a dated result line. Do not change the acceptance decision.
- [x] Commit the planning-only changes with `docs(planning):` (separate from any code commit). No code commit is expected: the Rust and Fortran instrumentation stay in scratch.

### 8. Archive
- [x] When all tasks are `[x]` and validation is complete, write "All tasks in TODO.md are complete." Archive this file to `planning/TODO_YYYYMMDD-N.md` with the next free sequence number and commit the move with `docs(planning):`.

## Notes
- Root `TODO.md` is a tracked plan for the unrelated Robin spatial-refinement campaign. It is not touched by this plan. This plan lives at `planning/TODO_26_DRIFT.md` for that reason.
- Reference executable provenance: fixtures were produced by the harness build with SHA-256 `e6d69dd3…`. The earlier pinned hash `824179ab…` is not the fixture source. Disclosed in the acceptance doc.
- Scratch outputs are under `.development/` and are not committed. Only planning docs and the verdict are committed.
- Known constraint: `NOD_INF.OUT` header carries a run timestamp. Compare parsed values only.
- Hazard: the Fortran per-iteration (`Iter`) structure and the Rust pass structure (pass one / pass two) are not the same unit. Alignment is by step time, not by iteration count.
- Known numbers to reproduce, not to trust blindly: FD max absolute head error 5.70 cm at t=240, node 4; FF 4.41 cm.

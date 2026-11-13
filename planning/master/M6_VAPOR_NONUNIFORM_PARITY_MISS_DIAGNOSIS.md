# M6 vapor nonuniform step_on parity miss: diagnosis (task 27.3.4)

Date: 2026-11-13. Read-only diagnosis. No production code, test, fixture or criteria file was edited.
Scratch: `.development/27.3.4-diag/` (Fortran re-run in `run/`, `runoff/`; instrumented copy of the test in `tree/`; output in `diag.txt`).

## Verdict

Cause established: a test-harness schedule mismatch, not a Rust defect. The untracked test
`crates/hydrus-core/tests/vapor_nonuniform_parity.rs` requests a constant dt = 0.1 each step. The pinned Fortran
fails its first dt = 0.1 attempt, retries at dt/3, and then keeps dt = 0.1/3 for the whole run (7200 steps). Run with the Fortran step
sequence, the same Rust code passes **0 failures of 2460** comparisons against the frozen criteria v3.

## Reproduction

    cargo test -p hydrus-core --test vapor_nonuniform_parity nonuniform_step_on_matches_fixture
    -> 147 of 2460 comparisons exceed bounds; first: head t=6 node 17, err 682.18, bound 246.0

Diagnostic run (scratch copy only): `.development/27.3.4-diag/tree`, test `diag_schedules`
(`CARGO_TARGET_DIR=.development/27.3.4-diag/target cargo test -p hydrus-core --test vapor_nonuniform_parity diag_schedules -- --nocapture`).
Fortran RUN_INF regenerated with the pinned `.development/26.2.2/build/hydrus-reference` (sha e6d69dd3...), inputs from
`reference/vapor_fortran/cases/vapor_nonuniform_step_on/`. It matches the 27.3.3 `run2` RUN_INF except for the date line.

## dt schedule comparison (step_on)

| Side | Accepted steps | dt sequence |
|---|---|---|
| Fortran (RUN_INF.OUT) | 7200 | every step 0.33333E-01 (10 late rows print 0.33332E-01, float rounding); first attempt at 0.1 failed and was retried at dt/3; dt never regrows (Iter = 2 > ItMin = 1) |
| Rust test (constant) | 2402 | 1 step at 0.1/3, then 2399 steps at 0.1, plus 2 shortened landing steps; the harness asks for 0.1 each step and the Rust attempt at 0.1 converges after the first step |
| Rust with Fortran schedule | 7200 | every step 0.1/3 |

The Fortran keeps its reduced dt as state; the harness re-offers dt = 0.1 every step, so Rust takes 3x larger steps through the
sharp 45/5 C step, where the solution is dt-sensitive.

## Error pattern (constant dt)

- Errors are zero at the Dirichlet-pinned end nodes (head -10000 matches at nodes 1 and 41) and concentrated at the step. Head at t = 6,
  node: error: 17: 682; 18: 1480; 19: 1185; 20: 381; 21: 568; 22: 262; away from the step errors are 2-15 cm, within bounds.
  The largest err/bound ratios are at nodes 17-22.
- Failing fields (all at t = 6, 24, 72; t = 240 mostly closes): head (6.4x bound worst), con, con_lt (up to 39x), v_liquid (87x), v_vap_iso (82x).
  Fields tied to the temperature/moisture step, as expected for a dt-dependent transient.
- Not a uniform offset, not a reversed profile.

## Hypotheses

| Hypothesis | Result | Evidence |
|---|---|---|
| Constant dt in the test differs from Fortran adaptive dt | **Ruled in, sufficient** | Fortran-schedule run: 0/2460 failures (`diag.txt`) |
| Initial-condition mismatch / node reversal | Ruled out | Errors are 0 at pinned ends and small away from the step; and the full pass with the same IC under the source schedule |
| Single-precision reference (H6) | Ruled out | Misses are up to 80x bound, localized at the step; with matching schedule everything passes within the same criteria |
| Remaining Rust formula difference (5 C side, clamp) | Ruled out | Same Rust code gives 0 failures under the Fortran schedule |

## Do other parity tests use the same policy?

Fortran RUN_INF for `vapor_heatbc_fd_on`, `vapor_heatbc_ff_off` and `vapor_nonuniform_step_off` is 2400 steps at dt = 0.1 each. Constant dt = 0.1
is therefore the source schedule for those, so their tests are valid. Only step_on differs (dt/3 retry on step 1, persisting).
(`vapor_heat_boundary_parity.rs` already carries an `#[ignore]` source-schedule investigation, `source_schedule`/`SourceStep`;
the step test file has the same `schedule: Option<&[SourceStep]>` hook in `run_case` but no parser for it.)

## Proposed next action (needs approval; harness change, no Rust fix)

1. In `vapor_nonuniform_parity.rs`, drive `vapor_nonuniform_step_on` with the source-derived schedule: dt = deck dt / 3 for every step
   (7200 steps, print targets 6/24/72/240 land on step indices 180/720/2160/7200), and state in the header that the schedule is taken from the
   source RUN_INF (adaptive TmCont still not reproduced; the constant dt/3 is what the source produced). Prefer an exact `dt/RETRY_FACTOR`
   over parsing the 5-digit RUN_INF values (parsed values break the 1e-9 time-landing assert; the last 10 rows print 0.33332E-01).
   Alternative: parse RUN_INF (exact values need rounding to dt/3) and keep it as an authenticated input hash.
2. Keep `step_off` at constant 0.1 (source schedule).
3. Update the header "Schedule disclosure" comment accordingly; do not change criteria, tolerances or fixtures.

A cleaner, less hard-coded alternative is to teach the Rust driver/harness to carry the retried dt forward like `TmCont` (dt persists, regrows with DMul); that is a larger change and is not
required for parity.

## Regression risk

Low. Test-only change to an uncommitted file; production code (27.3.3 clamp) untouched. The 147-row miss disappears with the matching schedule, with the same criteria.
Residual risk: the step_on case verifies parity only under the Fortran-chosen dt; sensitivity to dt (the constant-0.1 run differs by up to ~87x bound in v_liquid) is a
real behaviour difference of the two time-stepping policies, not a fixed defect, and should be recorded as a known limitation of the unreproduced adaptive schedule.

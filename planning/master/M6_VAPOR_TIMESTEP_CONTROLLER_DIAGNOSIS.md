# M6 vapor time-step controller diagnosis (task 28.1)

Date: 2026-11-13. Read-only diagnosis. No production code, test, fixture or criteria file was edited.
Scratch: `.development/28.1-diag/` (cargo crate `harness/` with an instrumented copy of
`vapor_nonuniform_parity.rs`; run with `CARGO_TARGET_DIR=../target cargo test --offline --test probe probe -- --exact --nocapture`).

## Verdict

Classification (c): expected, not a defect in anything that exists. Rust does reproduce the Fortran
first-attempt failure at dt = 0.1 and the retry at dt/3. The visible difference is that Rust has no
`TmCont`/`dtOpt` step-size memory: the caller supplies dt every step, and the parity harness supplies 0.1
every time. Fortran keeps `dtOpt = dt/3` after the retry and `TmCont` never grows it. The earlier statement
"Rust converges at dt = 0.1 where Fortran fails" is true only for steps 2..2400, which Fortran never attempts.

## Fortran rule (src_reference/)

- Inputs (`INPUT.FOR:880,893`): `dt dtMin dtMax DMul DMul2 ItMin ItMax MPL` = 0.1 0.01 0.5 1.2 0.5 1 20 4; `dtOpt = dt`. `MaxIt` = 20 (SELECTOR.IN block B).
- Retry is inside `WatFlow` (`WATFLOW.FOR:~123-149`): after `Iter=Iter+1` the convergence test (`:105-127`) runs. If not converged, or `Iter <= 1`
  (`:128`, a second solve is forced), and `Iter >= MaxIt`: if `dt <= dtMin` fail (`:131`), else restore `hNew=hTemp=hOld`, `dt = max(dt/3, dtMin)`,
  `dtOpt = dt` (`:144-145`), `t = tOld+dt`, `goto 11` (Iter reset to 0).
- Next-step control is `TmCont` (`TIME.FOR:3-30`, called at `HYDRUS.FOR:659`) with `Iter = max(IterW, IterC)` (`HYDRUS.FOR:655`):
  `dtOpt` grows by `DMul` only if `Iter <= ItMin` (`TIME.FOR:16`), shrinks by `DMul2` if `Iter >= ItMax` (`:18`). `dt = min(dtOpt, tFix-t)`, then an even-split
  adjustment (`:21-29`, `iStep`).
- Because `WatFlow` forces at least 2 iterations, `Iter >= 2 > ItMin = 1` always, so `dtOpt` never grows. After the first retry `dtOpt = 0.1/3` for the whole run.
- Observed in the pinned run (`.development/27.2.2/runs/vapor_nonuniform_step_on/run/...RUN_INF.OUT`): 7200 steps, all dt = 0.33333E-01 (10 rows print 0.33332E-01,
  single-precision landing), Iter = 2 on 7199 rows and 7 on row 1; ItCum 41 after row 1, then +4 per row (two passes x 2).

## Rust rule

- No `TmCont` equivalent. Water step (`water_solver.rs:602,676-688`): the step takes the caller's dt (clipped to the target) and retries with `dt/retry_factor` only on
  non-convergence; it errors (`:677`) if `next < minimum_timestep`. There is no persistent `dtOpt`.
- Vapor driver (`heat/vapor/driver.rs:384-423`): sets `local.minimum_timestep = dt` (`:391`) so water fails fast, then the driver restarts both passes at `dt/retry_factor`
  (`:410`), resetting the guess to old heads (`:420`); it errors if `dt/3 < policy.minimum_timestep`.
- The parity test passes `deck.dt` = 0.1 every step on the `None` schedule (`vapor_nonuniform_parity.rs:893`) and a fixed dt/3 plan for `step_on` (`:983-990`).
- Convergence test (`water_solver.rs:386-417`): same form as `WATFLOW.FOR:105-127` (EpsTh/EpsH split at `hSat`, rMax 0.999 -> failure, minimum 2 iterations, `max_iterations` = MaxIt = 20).

## Differences

1. No `TmCont`/`dtOpt`: no growth by `DMul`, no shrink by `DMul2`, no `iStep` even split, no persistence of the reduced dt. This is the whole observed schedule difference.
2. Retry floor: Fortran clamps `dt = max(dt/3, dtMin)` and fails only when `dt <= dtMin`; Rust errors when `dt/3 < dtMin` without clamping. Not reached in this case (0.1 -> 0.0333 > 0.01).
3. Retry granularity: Fortran retries inside each `WatFlow` call (per pass); Rust restarts the whole two-pass step. Not exercised: only pass one retried here.
4. Single-precision `dt` in Fortran (the ten 0.33332E-01 rows); Rust is f64.

## First-step reproduction

Scratch run of the Rust vapor driver at constant dt = 0.1 (no schedule), first step:

    step 1 t=0.0333333 dt=0.033333 restarts=1 it1=14 it2=7
    step 2 t=0.1333333 dt=0.100000 restarts=0 it1=4  it2=4
    step 3.. dt=0.1, it1=2 it2=2

The first dt = 0.1 attempt is rejected and the retry at dt/3 is accepted, as in Fortran. Iteration bookkeeping matches Fortran row 1 exactly:
Iter 7 = Rust pass-two 7; ItCum 41 = 20 (failed attempt at MaxIt) + 14 (pass one) + 7 (pass two). The 20 is inferred from the arithmetic and the restart
count; I did not dump Fortran per-iteration values. The full constant-dt = 0.1 run completes (1 step of dt/3 plus 2399 steps of 0.1 reach t = 240).
The residual cause of the first-attempt failure (what stops 20 iterations converging at dt = 0.1 against the 45/5 C step) is common to both codes and was not further analysed.

## Why step 2 differs

Fortran step 2 starts from `dtOpt = 0.0333` (set at `WATFLOW.FOR:145`), and `TmCont` keeps it. The harness gives Rust dt = 0.1 again; Rust converges in 2+2 iterations.
Fortran is never asked whether it would converge at 0.1 after step 1, so this is not evidence of a convergence-test difference.

## Proposed fix

None for the existing contract. The parity result is unaffected and the test already drives the Fortran dt/3 schedule on purpose. If controller parity is wanted later,
it is a new feature: port `TmCont` (state `dtOpt`, `lMinStep`, `DMul`, `DMul2`, `ItMin`, `ItMax`, `iStep` split) as a pure function in the driver layer,
feed it `max(IterW, IterC)`, and have the retry set `dtOpt`; add a unit test that after one retry the next dt stays dt/3 when Iter >= 2.
Risk: `vapor_heat_boundary_parity`, `vapor_fortran_parity` and `vapor_nonuniform_parity` pass explicit dt, so a new opt-in function does not change them;
wiring it into the default loop would change `step_off` and other cases (they would grow dt when Iter <= ItMin) and must not be done without a plan.

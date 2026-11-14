# M6 vapor G22.4 acceptance (orchestration 27, task 27.5.1)

Status: **rows decided 2026-11-13; goal G22.4 remains Circled.** The nonuniform
temperature row is accepted with exceptions. The scalar `ConVh` helper row is
resolved by removal. The sink and root row is rejected with triggers (Gate S).
Two items stay open (below), so this document does not close the goal. The
closure decision is the user's.

Basis: [M6_VAPOR_NONUNIFORM_T_AUDIT.md](M6_VAPOR_NONUNIFORM_T_AUDIT.md),
[M6_VAPOR_CONVH_AUDIT.md](M6_VAPOR_CONVH_AUDIT.md),
[M6_VAPOR_SINK_ROOT_AUDIT.md](M6_VAPOR_SINK_ROOT_AUDIT.md),
[M6_VAPOR_NONUNIFORM_NAN_DIAGNOSIS.md](M6_VAPOR_NONUNIFORM_NAN_DIAGNOSIS.md),
[M6_VAPOR_NONUNIFORM_PARITY_MISS_DIAGNOSIS.md](M6_VAPOR_NONUNIFORM_PARITY_MISS_DIAGNOSIS.md),
[STATE_27.md](../STATE_27.md), [coverage](M6_VAPOR_COVERAGE.md), criteria v3
([criteria.json](../../reference/vapor_fortran/criteria.json), unchanged; no
GATE N2 was needed). Commits: audits `ddc8ef6`; fixtures `ecf401d`; parity
`47b36a1`, `cc4fce4`; comparator `a0ab6c1`; balance `5a41592`; ConVh removal
`cbc52c0`; vapor head-clamp fix `6830b16`.

## Per-row decision

| Coverage row | Decision |
|---|---|
| Default thermal conductivity, nonuniform temperature | Accepted with exceptions |
| Scalar isothermal `ConVh` helper | Decided: reachable, output-neutral, removed |
| Nonzero prescribed sink or root uptake with heat/vapor | Rejected with triggers (all of (a)-(d)) |

## Evidence per case

Both cases: dry, h = -1e4 cm, step initial profile (nodes 1-20 at 45 C, nodes
21-41 at 5 C), Dirichlet 45/5 C, fixed heads, `iCampbell=0`, default
conductivity. Times `[0, 6, 24, 72, 240]`.

| Case | Fixture SHA-256 | Parity (Rust vs fixture) | Python comparator | Balance |
|---|---|---|---|---|
| `vapor_nonuniform_step_on` (lVapor on) | `787889b6...` (`ecf401d`) | 0 of 2460 rows over criteria v3 bounds, run at the Fortran dt/3 schedule on all 7200 steps (`47b36a1`, `cc4fce4`) | 0 failing rows (`a0ab6c1`); max err/bound: temperature_c 0.100, liquid_moisture 0.047, v_liquid 0.075 | All 7200 steps checked; worst water ratio 9.3e-7, heat 5.1e-7 (`5a41592`; bounds rel 1e-9 / abs 1e-15 from `criteria.json` `checks`) |
| `vapor_nonuniform_step_off` (lVapor off) | `af3c155e...` (`ecf401d`) | Passes at constant dt 0.1 (615 points) | 0 failing rows | **Not covered** (see exceptions) |

The Python comparator and the Rust test agree: 0 verdict mismatches on either
case; largest error difference 5.0e-8 (print precision). Reference outputs
reproduced exactly on rerun (STATE_27, 27.2.2).

Vapor-path defect found and fixed on the way: the Rust vapor water step mixed
head iterates without clamping to `hSat`, giving a NaN in `ConVh` on the first
nonuniform run. Fixed in `crates/hydrus-core/src/water_solver.rs` (`6830b16`,
new `clamped_mixed_head`), matching `WATFLOW.FOR:455-462`. Diagnosis:
`M6_VAPOR_NONUNIFORM_NAN_DIAGNOSIS.md`. This is a vapor-path change inside
`water_solver.rs`, a production file; existing vapor parity, heat-boundary,
water-row, driver-order and balance tests were unchanged by it (STATE_27, 27.3.3).

## Exceptions and disclosures

1. **Rust time-step controller (partly closed; decision open).** Fortran fails its
   first dt = 0.1 attempt, retries at dt/3 and keeps dt/3 for 7200 steps. An
   opt-in controller now ports the Fortran `TmCont` rule (`4804a0d`, in
   `heat/vapor/driver.rs`). Driven through it, step_on reproduces the Fortran
   step sequence on 7190 of 7200 steps, and all four print-time landings match
   (`0fdc1f9`). The last 10 steps are the final landing on t = 240: Fortran
   computes that split in single precision and prints `0.33332E-01`, while Rust
   uses f64 (about 4e-5 relative). That check is `#[ignore]`d with the reason
   recorded; the tolerance was not widened. The default loop is still
   schedule-driven, so the parity result is unchanged by this work. Retries
   restart both passes rather than retrying inside each WatFlow call (documented).
   Diagnosis: `M6_VAPOR_TIMESTEP_CONTROLLER_DIAGNOSIS.md`. Not a criteria change.
2. **step_off balance (heat side checked; water diagnostic only).** A test-local
   heat-only interior row check (`34e987d`) passes on every accepted step of
   `vapor_nonuniform_step_off` at dt 0.1, worst ratio 4.0e-7 against bound 1.0,
   with bounds from `criteria.json` `heat_balance_interior`. The water balance
   for this case is not asserted: the `water_balance_interior` criteria form does
   not reproduce for it, so only the continuum residual is printed (finite).
   Asserting it needs a user-approved bound. Assessment:
   `M6_VAPOR_STEP_OFF_BALANCE_ASSESSMENT.md`.
3. **Precision (H6).** No new precision-limited rows were observed; none are
   claimed. Criteria v3 were not widened.
4. **Dead parameters.** `_x_conv` and `_t_conv` of `gen_mat` remain after the
   helper removal (signature kept); removing them needs the caller edits at
   `compat_runner.rs:112` and the `water_compat.rs` tests.
5. **Missing `LEVEL_01.DIR`.** `vapor_thermal_gradient`, `vapor_off_control` and
   `vapor_isothermal_head_gradient` list it in `fixture.json` but have none on
   disk; they cannot be regenerated by the producer. Left untouched; needs an
   approved decision.
6. **No nonuniform `discrimination` entries** in criteria v3. The new cases are
   covered by the generic field and time rules only.
7. Test and fixture evidence class is `rust-candidate`; `step_on` schedule
   disclosure is in the parity test header and candidate JSON.

## ConVh helper

`con_vh` (`water_compat.rs`, hardcoded 20 C) was reached by every
`run_compat_deck` call through `gen_mat`, with the result discarded. The
Fortran scalar `ConVh` is `WATFLOW.FOR:968-995`; the array routine is
`ConVapor` (`:849-929`). The vapor path uses its own inline value
(`kernels.rs:52-53`). The user decided on removal (2026-11-13) because a guard
would fire on every compat deck. Removed in `cbc52c0`; 7 `water_compat` tests
pass.

## Sink and root uptake: Gate S disposition

Labels (a)-(d) are the audit's own, not the plan's row letters. Gate S
(user, 2026-11-13): reject all with triggers; no implementation in this
orchestration; no orchestration 28 opened. Guards at `vapor/mod.rs:180` and
`:185` are unchanged.

| Form | Disposition | Trigger |
|---|---|---|
| (a) Prescribed sink, `SinkF` on, `lRoot=0` | Rejected with trigger (the audit had called it a candidate) | A sink-only vapor-on case with `SinkF=1, lRoot=0`, after the `Beta` source is confirmed (audit open item 1) and an explicit balance criterion is approved |
| (b) Root uptake | Rejected with trigger | Root uptake requested with vapor enabled and an approved root-uptake vapor design (new gate) |
| (c) Heat sink coupling and `TempAdj` vapor terms | Rejected with trigger | A sink-bearing vapor case is requested; a balance check must show the sink heat term closed in the vapor energy budget |
| (d) Water step with vapor under a sink | Rejected with trigger | A sink-plus-vapor water balance criterion is written and approved |

Audit open items stay open: `Beta` source when `lRoot=0`; `driver.rs` passes
`input.sink` into the heat passes, so the guard is the only barrier and a future
relaxation must touch both; no balance-closure claim for sink.

## Goal status

G22.4 stays **Circled**. All three rows are decided. Both open items now have
partial closure, but each still needs your decision.

Closure triggers:
- Controller: decide whether the opt-in controller (`4804a0d`) closes item 1, or
  whether the default loop must use it before closure. Either way the final
  landing precision difference stays disclosed.
- step_off balance: decide whether the heat-side check closes the gap, and
  whether the water side needs a user-approved bound or stays a diagnostic.

## Not claimed

- That the default vapor loop uses the opt-in controller; step_off water balance closure.
- Bit-level agreement at the final landing on step_on (f32 versus f64).
- Any nonzero-sink or root-uptake behaviour with vapor.
- Other nonuniform profile shapes, nonzero-flux boundaries, Campbell,
  enhancement off, G22.2, G22.3, G20, G23.
- Agreement with a double-precision reference (the pinned reference is
  single precision, H7).
- Any change to ADR 0021 or the G22.1 acceptance decision.

# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-22
Source State: [STATE_31.md](STATE_31.md) (plan [EXECUTION_PLAN_31.md](EXECUTION_PLAN_31.md))

Magic Wand refresh (2026-11-22, user-approved): extra ranking criteria are retire Fortran risk early, land on main soon, and minimize gate load. G22.3 is split into M1 to M7, which map to Plan 31 tasks. M2 (the `Meteo.in` reader) is new: the G22.3 contract names `meteo::config::MeteoConfig` and `meteo::series::MeteoRecord`, but no Plan 31 task owned them.

## Circled Goal (Active)

- **[G22.3-M1]** A pinned-Fortran met fixture `reference/vapor_fortran/fixtures/vapor_met_bare_soil_on.json` exists. It is produced through `tools/run_vapor_reference.py` from the deck `reference/vapor_fortran/cases/vapor_met_bare_soil_on/`: `lMeteo=t`, `lVapor=t`, `AtmBC=t`, `KodTop=-4`, fixed-head lower at -100 cm, `iCrop=0`, 10 daily records over 240 h. (Plan 31 task 31.2.1.)
  - Why now: the only unstarted input to the parity gate (M5), and the largest Fortran risk in Plan 31. The pinned reference has already diverged with vapor in the dry regime (G22.2-C3). Met evaporation can dry the surface node. Running it first tests the deck design and D-10 before M3 and M4 build on them. No user gate beyond the answers already given.
  - Exit criteria: Fortran reaches 240 h without abort, and the non-convergence count is recorded. The fixture records the number of upper-boundary flux/head switches. Zero switches is a design finding, because M5 could not then test D-10. `python3 -m unittest tools.test_run_vapor_reference` passes. `git diff --quiet -- reference/vapor_fortran/criteria.json`. Provenance (source hash, deck hash) is in the fixture. On divergence: write a record in the G22.2-C3 style and stop at a gate. Do not tune the deck to force convergence without approval.
  - Files: `tools/run_vapor_reference.py` (allowlist ~line 318; the vapor flag check ~line 264 must also accept `f f t t f f`), `tools/test_run_vapor_reference.py`, the case directory, `reference/vapor_fortran/fixtures/`, `planning/STATE_31.md`.
  - Decisions (2026-11-22, user): case name `vapor_met_bare_soil_on`; 240 h with 10 daily records. These answer the open questions in the worker's `planning/tasks/31.2.1.todo.md`.
  - First action: write those answers into the 31.2.1 worker todo and approve its plan. Rebase the worker worktree from `e4fd0ae` onto `49ebb43` and resume it.
  - Depends on: G22.3 contract (31.1.3, done), Gate A (approved 2026-11-15).

## Sequenced Goals (Dependency DAG)

```
M1 fixture --+-------------> M5 parity --+
             +-(deck)-> M2 reader -> M5   +-> M7 record G22.2 close + G22.3 -> G20 (deferred)
M3 evaporation ---------> M5              |
M4 vapor switching -+---> M5              |
                    +---> M6 balance -----+
```

G22.3 (atmospheric water switching with vapor, meteorology contracted in [M6_VAPOR_G22_3_CONTRACT.md](master/M6_VAPOR_G22_3_CONTRACT.md)) is the parent of M1 to M7. It is accepted at M7.

- **[G22.3-M2]** Rust reads `Meteo.in` and its met records into `meteo::config::MeteoConfig` and `meteo::series::MeteoRecord` (`INPUT.FOR:934-1066`, `TIME.FOR:484-497`). Every rejection in contract section 7 (`InvalidMeteoDeck`, temperature range, RH, wind, radiation, radiation mode, growth table) is a typed error raised before state mutation. (New Plan 31 task 31.3.3, wave 3.)
  - Exit criteria: `cargo test -p hydrus-core meteo::config meteo::series` passes, parsing the M1 deck and one test per named rejection.
  - Files: `crates/hydrus-core/src/meteo/config.rs`, `meteo/series.rs` (create), plus module declarations in `meteo/mod.rs`. `meteo/mod.rs` is shared with 31.3.1: one-line `mod` additions only, merged at wave end.
  - Depends on: M1 (deck format; soft), contract (done).

- **[G22.3-M3]** `meteo::evaporation` computes potential evaporation from the 31.2.2 radiation terms in `f32` with source operation order (ADR 0010/0015). (Task 31.3.1.)
  - Exit criteria: `cargo test -p hydrus-core meteo::evaporation` passes against the source formulas.
  - Depends on: 31.2.2 (done).

- **[G22.3-M4]** `advance_atmospheric_step` drives the vapor path by decomposing each step into existing Flux/Head pairs, in Fortran switching order (D-10). The water-only atmospheric tests stay unchanged and green. (Task 31.3.2.)
  - Exit criteria: `cargo test -p hydrus-core atmospheric` passes. `water_solver.rs` is not edited; if it must be, re-scope (escalation 4).
  - Depends on: contract (done), 31.2.2 (done).

- **[G22.3-M5]** `vapor_met_parity` matches the pinned met trajectory from M1. Source-precision rows are listed, not widened. A failure re-opens D-10. (Task 31.4.1.)
  - Depends on: M1, M2, M3, M4.

- **[G22.3-M6]** `vapor_met_balance` reports a water-balance residual within tolerance over the met-switched run. (Task 31.4.2.)
  - Depends on: M4.

- **[G22.3-M7]** G22.2 is recorded closed and G22.3 accepted in `M6_VAPOR_COVERAGE.md`, `M6_VAPOR_G22_3_ACCEPTANCE.md` (create) and this file. (Task 31.5.1.)
  - Depends on: M5, M6, 31.2.3 (done).

- **[G22.2]** Water-flux and free-drainage boundary pairs (and supported mixed pairs) are implemented with vapor, including vapor boundary/storage accounting and balance evidence.
  - Status (2026-11-22): all slices done; only the closure record remains, owned by G22.3-M7. C1, C3, C4 and C6 done (STATE_29/30). C2 confirmed by the user (`M6_VAPOR_G22_2_ACCEPTANCE.md`). C5 done: Gate A Option A (no closed-pair head entries) and M1 (`flux_known_on_off_liquid_moisture`, factor 0.75, `d2d1f32`). Flux upper with fixed-head lower: wet pair accepted (`880c6c2`), G22.2-F1 landing-step artifact fixed, parity 19/19; matrix corrected (`ffa1af1`). Dry (-10000 cm) divergence recorded, not converged. Full history: [STATE_30.md](STATE_30.md), [M6_VAPOR_G22_2_ACCEPTANCE.md](master/M6_VAPOR_G22_2_ACCEPTANCE.md), and git history of this file.

## Independent Backlog

- **[G24]** `feat/vapor-flux-fixedhead` is merged into `main` with the full AGENTS.md suites green. At 2026-11-22 it is 21 commits ahead of `main` and 0 behind: G22.2 work, the C5 criteria entry, the radiation port.
  - Why: scores on "land on main soon"; one approval, no dependency. Can run in parallel with M1.
  - Gate: user approval before the merge.

- **[G7]** Superseded by [G22.2-C6], which carries the same free-drainage end-row assessment inside G22.2.
  - Status: merged into G22.2-C6. Kept for history.

- **[G22.2-C7]** Cleanup: the `planning/STATE_29.md` wave-board rows for 29.3.3 and 29.4.x are updated from pending; the untracked failed-trial output in `.development/29.4.1/runs/B` (about 1.5 GB) and the quarantined partial fixture at `.development/29.3.3/quarantine/` are removed with user approval. Added 2026-11-22: about 100 stale worktrees from plans 1 to 31 (`git worktree list`), pruned after checking each for unmerged work.
  - Why now: hygiene only; low leverage.
  - Gate: deletion needs explicit user approval before any `rm`.
  - Depends on: none.

- **[G22.5]** Enhancement off (`iEnhanc=0`) evidence strategy decided and implemented. The kernel harness can select it; the full executable cannot via input.
  - Trigger to promote: a user decision on the evidence strategy.

- **[G22.6]** Campbell (`iCampbell=1`) vapor path: resolve SD-006 policy, add discriminating fixtures, no silent source correction.
  - Trigger to promote: SD-006 policy decision by the user.

- **[G10]** Investigate the Robin refinement anomaly only if a real supported-use case shows it could change a decision; the current finding is nonblocking.

## Deferred

- **[G20]** Energy balance (`lEnBal`, `UpdateEnergy`, surface energy terms `Evap`, `Rn`, `Rns`, `Rnl`) is added to the coupled capability with its own contract, fixtures and acceptance gate.
  - Status: radiation (`Rn`, `Rns`, `Rnl`) ported in Plan 31 (31.2.2, `b7acedf`) per D-4. The energy update is split out.
  - Trigger: G22.3-M7 records G22.3 accepted; then contract and reprioritize the remainder as its own orchestration.

- **[G23]** Temperature-dependent liquid-water coupling (`lWDep` / `lWTDep`, including `ConLT` in water flux and liquid material temperature adjustments) is separately contracted, implemented and validated when selected.
  - Trigger: separate user scope approval. Remains rejected in orchestration 25; not conflated with vapor's thermal-gradient terms or G20 energy balance.

- **[G9]** Time-axis convergence is re-validated with three time-axis study levels or dual-axis centre reuse.
  - Trigger: only if a real supported-use case needs a stronger temporal claim or an observed timestep effect could change a decision.

## Completed

- **[G22.4]** Remaining G22 rows: nonuniform-temperature coverage for default thermal conductivity, scalar `ConVh` reachability audit, and nonzero prescribed sink or root uptake with heat/vapor.
  - Result (2026-11-13): **complete with exceptions** (user decision). Nonuniform T accepted with exceptions (`vapor_nonuniform_step_on`/`_off`); scalar `ConVh` helper removed (`cbc52c0`); sink and root rejected with triggers at Gate S (no implementation). Record: [M6_VAPOR_G22_4_ACCEPTANCE.md](master/M6_VAPOR_G22_4_ACCEPTANCE.md).
  - Exceptions: (1) the Rust time-step controller is an opt-in port (`4804a0d`) that reproduces the Fortran step sequence on 7190 of 7200 steps; the final-landing single-precision difference is disclosed, and the default loop stays schedule-driven. (2) step_off water balance is checked against criteria v4 `water_balance_interior_step_off_reference` for static-water consistency only. (3) `_x_conv`/`_t_conv` dead parameters remain in `gen_mat`.
- **[G22.1]** Heat top/bottom boundary forms with vapor audited, contracted and accepted or rejected with triggers, recorded in [M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md](master/M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md).
  - Result (2026-11-13): **complete with exceptions** (user decision). DD (existing), DF, DZ, FF, FZ and FD accepted; `kTopT<-1` rejected with trigger; `kTopT=0` invalid; sinusoidal top rejected.
  - Exceptions: FD has one `v_liquid` row failing its frozen bound (1.50x at t=240, node 37). FD and FF both carry a vapor-on head drift (5.70 and 4.41 cm absolute at t=240). Cause identified 2026-11-13: single-precision (REAL*4) arithmetic in the pinned reference's vapor-coupled solve, recorded as a precision exception (H7 class). Rust agrees with the same source built in double precision to 5e-9 cm. Criteria v3 are unchanged. See [D-26.2_DRIFT_MECHANISM.md](master/D-26.2_DRIFT_MECHANISM.md).
- **[G19]** Vapor flow is added to the coupled heat capability: `lVapor` terms (`TempAdj`, `ConVapor`, `VaporContent`, `ConVh`, vapor output `NOD_INF_V.OUT`) are implemented with explicit coupling contracts, rejected combinations preserved until supported, provenance-backed fixtures, and capability-specific validation.
  - Result (2026-11-12): **complete with exception** for the bounded first increment. Recorded in [M6_VAPOR_ACCEPTANCE.md](master/M6_VAPOR_ACCEPTANCE.md) (contract [M6_VAPOR_CONTRACT.md](master/M6_VAPOR_CONTRACT.md), [coverage](master/M6_VAPOR_COVERAGE.md)).
  - Exception: 18 isothermal `v_vapor`/`v_vap_term` rows fail criteria v2 against the pinned fixture; explained by source single-precision `TempN` ([diagnosis](master/M6_VAPOR_PRECISION_DIAGNOSIS.md)) and not claimed as matched. Vapor content evidence is pinned-kernel only; wet fixtures do not discriminate head/moisture/temperature. Restricted to the first-increment selection.
- **[G12]** Heat, temperature and related coupled capability is delivered for M6 (heat only; vapor and energy balance moved to G19 and G20).
  - Result: recorded in [M6_HEAT_ACCEPTANCE.md](master/M6_HEAT_ACCEPTANCE.md). Strict 2e-4 °C parity fails at 20–36 nodes (output rounding); rounded parity passes on the table water model. Campbell fixtures are flat at 25 °C (weak evidence). The zero-gradient bottom boundary is supported in code but not validated against the Fortran.
- **[G13]** to **[G18]** are completed for the heat scope, recorded in [STATE_24.md](STATE_24.md) and the acceptance document.
- **[G21]** The Rust van Genuchten conductivity uses the Mualem pore-connectivity exponent `l` (`Par(6)`, default 0.5) instead of a hardcoded `Se^1.5`, matching `MATERIAL.FOR:29` and `:76`.
  - Result: commit `1d4131a` (2026-11-11). Dated entry in [STATE_24.md](STATE_24.md).
- **[G1]** The M5-V control and evidence blockers are resolved and the final run campaign is ready to start.
  - Result: D12/D13 decided; five control rows and the `23-final` root committed; tools frozen; combined bounds re-signed at Gate B on 2026-11-09.
- **[G2]** Six final batches execute and verify with the frozen tools and combined bounds.
  - Result: all six verified 2026-11-10; batch IDs and counts are in `M5V_FINAL_23.md`.
- **[G3]** The study findings have verified classifications and documented error budgets.
  - Result: 12 achieved, 40 failed, 12,888 inconclusive across six groups; the temporal gap and floors are recorded.
- **[G6]** M5-V findings and limitations are recorded without overstating the evidence.
  - Result: M5-V results accepted with limitations for project progression; see the dated [M5-V/M6 decision](master/M5V_M6_HANDOFF_DECISION.md).

## Historical Goals

- **[G12 original]** Coupled heat, vapor and energy capability, as first stated in the M5-V handoff. Reframed by D1 (2026-11-11): M6 is heat only; vapor and energy are G19 and G20.
- **[G11]** Define and implement the initial M6 heat/temperature scope. Reframed by G12. Retained for planning history.

## Blocked / Pruned

- **[G4]** Holdout-level thresholds are proposed and approved at Gate C.
  - Status: Not requested
  - Reason: No holdout-level threshold is defensible from this campaign.
- **[G5]** The three reserved holdout rows are run against approved thresholds.
  - Status: Blocked
  - Reason: Gate C was not requested and `THRESHOLDS_V2.json` still has `holdout_run_permitted: false`.
- **[G8]** Run holdout rows before Gate C approval.
  - Status: Pruned
  - Reason: Explicitly prohibited; no holdout row ran.
- **[G22 umbrella]** Retained as the parent of G22.1–G22.6 in the coverage matrix; not a single executable goal.
  - Status: Parent
  - Reason: Completion rule and matrix live in [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md).

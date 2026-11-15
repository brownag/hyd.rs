# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-14
Source State: [STATE_30.md](STATE_30.md)

## Circled Goal (Active)

- **[G22.2]** Water-flux and free-drainage boundary pairs (and supported mixed pairs) are implemented with vapor, including vapor boundary/storage accounting and balance evidence.
  - Why now: root of the remaining chain. G22.3 depends on it, and G20 depends on G22.3. Unblocks the atmospheric path.
  - Exit criteria: each water-flux and free-drainage form with vapor on is accepted or rejected with a named trigger; at least one accepted form has a provenance-backed fixture and a balance check; recorded in the coverage matrix. A full G22.2 will not finish in 24 hours; the first slice is the audit.
  - Active slice (2026-11-15): C5 done. Written as criteria v6: closed-pair temperature and moisture discrimination (moisture at factor 0.85, passes with about 1% margin), nonzero entries. Closed head dropped; no dry discrimination entries. Remaining for the G22.2 exit: C2 decision and the single-flux and flux/FixedHead forms. Earlier: C5, the closed-pair criteria, was unblocked. C1, C3, C4 and C6 are done (C3 record in `M6_VAPOR_COVERAGE.md`). C3 result: the thermal gradient diverges at -10000 cm (NaN by 72 h) and the isothermal control converges; hypothesis supported. Free drainage with vapor has fixture parity (30.5.1, `340fec7`). Pending user decisions: C2, and the closed-pair head and moisture criteria (dropped at G30.6). The single-flux and flux/FixedHead forms stay out of this slice.
  - First action: [G22.2-C5] decide the closed-pair head and moisture criteria with the user, and the dry entries (now that C3 is recorded). No edit to `criteria.json` before approval.
  - Depends on: G22.1 (done), G19 (done).
  - Result (2026-11-14, GATE 29.2, user): closed flux/flux pair accepted and implemented with vapor on (fixtures, balance test, typed guard); exit criterion met with caveats. G22.2 stays Circled: free drainage, single flux forms and flux/FixedHead remain accepted for orchestration 29 but not implemented.
  - Result (2026-11-13, GATE W1, user): stays Circled. Accepted for orchestration 29: free drainage (lower), prescribed flux upper and lower, and their pairs (closed flux-flux pair first). Rejected with triggers: seepage face, water table, and pairs with them. `qGWLF` and `TopInF`/`BotInF` (+-3) carry the audit's reject-with-trigger disposition but were not explicitly answered at GATE W1; atmospheric is G22.3. Record: [M6_VAPOR_G22_2_ACCEPTANCE.md](master/M6_VAPOR_G22_2_ACCEPTANCE.md).

## Sequenced Goals (Dependency DAG)

Order: C1, C2, C6, C3, C4, C5. C1, C2 and C6 need no reference run. C3 and C4 each need a gate before any deck or runner change. C5 needs C1 and C3.

Status (2026-11-14): C1, C3, C4 and C6 are done (C4 pending confirmation). C2 is a pending user decision. C5 is the active slice and blocks the G22.2 exit.

- **[G22.2-C1]** Rust trajectory producer for the closed flux/flux pair (vapor on, and off where fields allow) matches the four committed Fortran fixtures (`vapor_closed_flux_{on,off}`, `vapor_flux_known_{on,off}`) within stated tolerances. The comparison runs through `tools/compare_vapor_reference.py` without editing `criteria.json`.
  - Why now: no gate, no reference run. Closes the gap that the closed pair has balance evidence but no trajectory evidence.
  - Exit criteria: `outcome()` produces closed-pair trajectories; the comparison reports tolerance and max differences per field (head, liquid_moisture, temperature_c, and vapor fields where the Fortran output has them); results recorded in `M6_VAPOR_COVERAGE.md`.
  - Files: `crates/hydrus-core/tests/vapor_fortran_parity.rs`, `tools/compare_vapor_reference.py`, `reference/vapor_fortran/`, `planning/master/M6_VAPOR_COVERAGE.md`.
  - Depends on: none (wet-start fixtures already committed in `839ce78`).

- **[G22.2-C2]** The `qGWLF` and time-variable ±3 input rejections (`TopInF`/`BotInF`) are explicitly confirmed or changed.
  - Why now: decision only; closes the accounting gap left at GATE W1.
  - Exit criteria: a dated user decision recorded in `planning/master/M6_VAPOR_G22_2_ACCEPTANCE.md`, with triggers for any rejection kept.
  - Depends on: none.

- **[G22.2-C6]** The free-drainage lower end-row behavior (formerly G7) is assessed against the source-defect register rules, recorded as a defect or modeling choice, and used to set the free-drainage scope for G22.2.
  - Why now: small and controllable; the Fortran `ConB` vapor-conductance question (`WATFLOW.FOR:202-213`) decides whether free drainage with vapor can be accepted.
  - Exit criteria: a source-defect entry (SD-NNN) or a recorded modeling choice; the free-drainage form is marked accepted with its pinning test, or rejected with a trigger.
  - Depends on: none.

- **[G22.2-C3]** Dry-regime closed flux/flux pair with vapor on is either converged in the pinned Fortran reference, or recorded as impossible with a reason. Chosen strategy: isothermal dry closed deck (tests whether the thermal vapor term drives the divergence seen at h = -10000 cm).
  - Why now: the committed fixtures start at -100 cm, so they do not test dry-regime vapor behaviour. Working hypothesis (unproven): a free top head with a dry node stores little water, so the thermal vapor flux moves hTop by thousands of cm in one step.
  - Exit criteria: either a converged dry reference run with provenance, or a documented divergence record (non-convergence counts, hTop trajectory) in `M6_VAPOR_COVERAGE.md`.
  - Gate: required before any deck or input change and before any reference run. Prior failed attempts (for the record): initial head -1000 cm (died 19.5 h); smaller time steps (1350 non-convergence messages); gradient 25→15 C (NaN by 72 h).
  - Depends on: none for the gate request; the design is written before the gate.

- **[G22.2-C4]** Fortran `BALANCE.OUT` is added to the runner's output list, and a full-trajectory balance check is run on the closed pair. Today's balance check is one uniform-temperature, no-sink step only.
  - Why now: needed for the balance claim to cover the whole trajectory.
  - Exit criteria: full-trajectory balance residual reported for both closed-pair decks, with the runner change in `reference/vapor_fortran/` and `tools/test_run_vapor_reference.py` tests.
  - Gate: required before the runner change and the rerun.
  - Depends on: none.

- **[G22.2-C5]** Criteria entries (D5) for the closed pair are approved and written: discrimination entries for head, liquid_moisture and temperature_c per closed pair, and nonzero entries for the `_on` ids. Ad hoc on/off differences (`.development/29.4.2/onoff.txt`): closed pair head 0.897 cm, moisture 9.0e-4, temperature 0.27 C; flux-known pair head 0.877 cm, moisture 8.0e-4, temperature 0.27 C.
  - Why now: entries must bind to the C1 producer output and cover the dry regime from C3 if that succeeds.
  - Exit criteria: tolerances proposed with evidence, approved by the user, then written. No edit to `criteria.json` without explicit approval (D5).
  - Gate: required before any edit to `criteria.json`.
  - Depends on: G22.2-C1, G22.2-C3.

- **[G22.3]** Atmospheric water switching is implemented with vapor, with its meteorological and evaporation prerequisites contracted.
  - Depends on: G22.2 (and G22.1). Coordinated with G20.

- **[G20]** Energy balance (`lEnBal`, `UpdateEnergy`, surface energy terms `Evap`, `Rn`, `Rns`, `Rnl`) is added to the coupled capability with its own contract, fixtures and acceptance gate.
  - Depends on: G22.3 (atmospheric path with meteorological input, `lMeteo`, `MeteoIn`, `INPUT.FOR:966`, `HYDRUS.FOR:243-244`, which the Rust crate does not yet have), G19 (done with exception), G22.1 (top energy boundary).

## Independent Backlog

- **[G7]** Superseded by [G22.2-C6], which carries the same free-drainage end-row assessment inside G22.2.
  - Status: merged into G22.2-C6. Kept for history.

- **[G22.2-C7]** Cleanup: the `planning/STATE_29.md` wave-board rows for 29.3.3 and 29.4.x are updated from pending; the untracked failed-trial output in `.development/29.4.1/runs/B` (about 1.5 GB) and the quarantined partial fixture at `.development/29.3.3/quarantine/` are removed with user approval.
  - Why now: hygiene only; low leverage.
  - Gate: deletion needs explicit user approval before any `rm`.
  - Depends on: none.

- **[G22.5]** Enhancement off (`iEnhanc=0`) evidence strategy decided and implemented. The kernel harness can select it; the full executable cannot via input.
  - Trigger to promote: a user decision on the evidence strategy.

- **[G22.6]** Campbell (`iCampbell=1`) vapor path: resolve SD-006 policy, add discriminating fixtures, no silent source correction.
  - Trigger to promote: SD-006 policy decision by the user.

- **[G10]** Investigate the Robin refinement anomaly only if a real supported-use case shows it could change a decision; the current finding is nonblocking.

## Deferred

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

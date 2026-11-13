# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-13
Source State: [STATE_25.md](STATE_25.md)

## Circled Goal (Active)

_No active circled goal. G22.1 completed 2026-11-13 with exceptions._

## Sequenced Goals (Dependency DAG)

- **[G22.2]** Water-flux and free-drainage boundary pairs (and supported mixed pairs) are implemented with vapor, including vapor boundary/storage accounting and balance evidence.
  - Depends on: G22.1 (mixed heat/water pairs need the heat-boundary contract), G19.

- **[G22.3]** Atmospheric water switching is implemented with vapor, with its meteorological and evaporation prerequisites contracted.
  - Depends on: G22.1, G22.2. Coordinated with G20.

- **[G20]** Energy balance (`lEnBal`, `UpdateEnergy`, surface energy terms `Evap`, `Rn`, `Rns`, `Rnl`) is added to the coupled capability with its own contract, fixtures and acceptance gate.
  - Depends on: G19 (done with exception), G22.1 (top energy boundary), and the atmospheric boundary path with meteorological input (`lMeteo`, `MeteoIn`, `INPUT.FOR:966`, `HYDRUS.FOR:243-244`), which the Rust crate does not yet have. G22.3 is the atmospheric prerequisite.

- **[G22.4]** Remaining G22 rows: nonuniform-temperature coverage for default thermal conductivity, scalar `ConVh` reachability audit, and nonzero prescribed sink or root uptake with heat/vapor (separate source/coupling audit).
  - Depends on: G22.1 for the heat rows; the sink audit is independent.

## Independent Backlog

- **[G22.5]** Enhancement off (`iEnhanc=0`) evidence strategy decided and implemented. The kernel harness can select it; the full executable cannot via input.
  - Trigger to promote: a user decision on the evidence strategy.

- **[G22.6]** Campbell (`iCampbell=1`) vapor path: resolve SD-006 policy, add discriminating fixtures, no silent source correction.
  - Trigger to promote: SD-006 policy decision by the user.

- **[G23]** Temperature-dependent liquid-water coupling (`lWDep` / `lWTDep`, including `ConLT` in water flux and liquid material temperature adjustments) is separately contracted, implemented and validated when selected.
  - Trigger: separate user scope approval; remains deferred. Remains rejected in orchestration 25; not conflated with vapor's thermal-gradient terms or G20 energy balance.

- **[G7]** The free-drainage end-row behavior is assessed against the source-defect register rules and recorded as a defect or modeling choice.

- **[G10]** Investigate the Robin refinement anomaly only if a real supported-use case shows it could change a decision; the current finding is nonblocking.

## Deferred

- **[G9]** Time-axis convergence is re-validated with three time-axis study levels or dual-axis centre reuse.
  - Trigger: only if a real supported-use case needs a stronger temporal claim or an observed timestep effect could change a decision.

## Completed

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

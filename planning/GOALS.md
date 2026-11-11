# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-11
Source State: [STATE_23.md](STATE_23.md)

## Circled Goal (Active)

- **[G12]** Deliver a coupled heat, temperature, vapor-flow, and energy-balance capability.
  - Entry: see [M5V_M6_HANDOFF_DECISION.md](master/M5V_M6_HANDOFF_DECISION.md).
  - Why now: M5-V is accepted with limitations; this is the next desired coupled-physics capability.
  - Boundary: preserve M5 campaign findings and do not expand into unrelated optional physics. Vapor flow and energy balance are desired scope; determine the correct milestone placement during planning rather than assuming they belong to M6.
  - Exit criteria: selected M6 behavior is implemented with explicit coupling contracts, relevant fixtures, and capability-specific validation evidence; unsupported combinations are rejected rather than silently approximated.
  - First action: trace the existing temperature, vapor, and energy paths in the pinned source and current Rust state/API, then propose a bounded implementation sequence and milestone placement.

## Sequenced Goals (Dependency DAG)

- **[G13]** Source-supported heat, temperature, vapor-flow, and energy-balance behavior and coupling order are mapped to explicit contracts.
  - Depends on: G12
- **[G14]** State, parameters, and configuration represent the selected coupled capability and reject unsupported combinations.
  - Depends on: G13
- **[G15]** A bounded coupled heat/temperature/vapor/energy vertical slice is implemented.
  - Depends on: G14
- **[G16]** Provenance-backed fixtures exercise the selected thermal behavior, coupling, and balance properties.
  - Depends on: G13, G15
- **[G17]** Outputs are compared with analytical references where available and pinned Fortran only for source-matched cases; results and limitations are recorded.
  - Depends on: G16
- **[G18]** Acceptance is documented for the implemented capability and intended use, without implying universal accuracy or broader feature support.
  - Depends on: G17

## Completed

- **[G1]** The M5-V control and evidence blockers are resolved and the final run campaign is ready to start.
  - Result: D12/D13 decided; five control rows and the `23-final` root committed; tools frozen; combined bounds `3ac9fd8a...a6ee4` re-signed at Gate B on 2026-11-09.
- **[G2]** Six final batches execute and verify with the frozen tools and combined bounds.
  - Result: all six verified 2026-11-10; batch IDs and counts are in `M5V_FINAL_23.md`.
- **[G3]** The study findings have verified classifications and documented error budgets.
  - Result: 12 achieved, 40 failed, 12,888 inconclusive across six groups; the temporal gap and floors are recorded.
- **[G6]** M5-V findings and limitations are recorded without overstating the evidence.
  - Result: M5-V results accepted with limitations for project progression; no holdout ran, and the original classifications remain. See the dated [M5-V/M6 decision](master/M5V_M6_HANDOFF_DECISION.md).

## Deferred

- **[G9]** Time-axis convergence is re-validated with three time-axis study levels or dual-axis centre reuse.
  - Trigger: only if a real supported-use case needs a stronger temporal claim or an observed timestep effect could change a decision.

## Independent Backlog

- **[G7]** The free-drainage end-row behavior is assessed against the source-defect register rules and recorded as a defect or modeling choice.
- **[G10]** Investigate the Robin refinement anomaly only if a real supported-use case shows it could change a decision; the current finding is nonblocking.

## Historical Goals

- **[G11]** Define and implement the initial M6 heat/temperature scope. Reframed by G12: vapor and energy are desired, while milestone placement remains to be determined. Retained here for planning history.

## Blocked / Pruned

- **[G4]** Holdout-level thresholds are proposed and approved at Gate C.
  - Status: Not requested
  - Reason: No holdout-level threshold is defensible from this campaign; the achieved fields do not establish the holdout level, Robin space failed, and the rest are inconclusive.
- **[G5]** The three reserved holdout rows are run against approved thresholds.
  - Status: Blocked
  - Reason: Gate C was not requested and `THRESHOLDS_V2.json` still has `holdout_run_permitted: false`.
- **[G8]** Run holdout rows before Gate C approval.
  - Status: Pruned
  - Reason: Explicitly prohibited; no holdout row ran.

# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-11
Source State: [STATE_24.md](STATE_24.md)

## Circled Goal (Active)

- **[G19]** Vapor flow is added to the coupled heat capability: `lVapor` terms (`TempAdj`, `ConVapor`, `VaporContent`, `ConVh`, vapor output `NOD_INF_V.OUT`) are implemented with explicit coupling contracts, rejected combinations preserved until supported, provenance-backed fixtures, and capability-specific validation.
  - Why now: G12's M6 heat scope is delivered; vapor is the first deferred physics, and energy balance (G20) depends on the vapor path for evaporation terms.
  - Boundary: vapor only. Energy balance (`lEnBal`, `UpdateEnergy`, `Evap`, `Rn`, `Rns`, `Rnl`) stays out. Heat-to-water feedback (`lWDep`) stays rejected unless separately approved. Hysteresis, dual porosity and root uptake with heat stay out.
  - Exit criteria: a vapor source contract (`TEMPER.FOR:48` `TempAdj`, `TEMPER.FOR:180-268`, vapor output `HYDRUS.FOR:174`) is recorded; the heat-only config accepts `lVapor=true` only for the supported vapor selection; source-matched fixtures exist for each supported vapor case; analytical or balance checks pass at tolerances approved for vapor; the acceptance document states what is not claimed.
  - First action: write the vapor source contract (`planning/master/M6_VAPOR_CONTRACT.md`) by reading `TEMPER.FOR:180-268` (`TempAdj`), `TEMPER.FOR:48`, the vapor conductivity and content code in `WATFLOW.FOR` (`:919-922`, `:995`), and the vapor output in `HYDRUS.FOR:174`; list the vapor variables and their precision. Hand the contract to plan-wave for the execution plan.

## Sequenced Goals (Dependency DAG)

- **[G20]** Energy balance (`lEnBal`, `UpdateEnergy`, surface energy terms `Evap`, `Rn`, `Rns`, `Rnl`) is added to the coupled capability with its own contract, fixtures and acceptance gate.
  - Depends on: G19 (evaporation uses the vapor path), and on the atmospheric boundary path with meteorological input (`lMeteo`, `MeteoIn`, `INPUT.FOR:966`, `HYDRUS.FOR:243-244`), which the Rust crate does not yet have.
  - Trigger to start: G19 accepted.

## Completed

- **[G12]** Heat, temperature and related coupled capability is delivered for M6 (heat only; vapor and energy balance moved to G19 and G20).
  - Result: M6 heat capability with contract, configuration, boundary, transport, state, driver, balance, analytical, source-matched parity and balance property evidence is recorded in [M6_HEAT_ACCEPTANCE.md](master/M6_HEAT_ACCEPTANCE.md). Strict 2e-4 °C parity fails at 20–36 nodes (output rounding); rounded parity passes on the table water model. Campbell fixtures are flat at 25 °C (weak evidence). The zero-gradient bottom boundary is supported in code but not validated against the Fortran.
- **[G13]** to **[G18]** are completed for the heat scope (contract, configuration, vertical slice, fixtures, comparisons, acceptance), recorded in [STATE_24.md](STATE_24.md) and the acceptance document.
- **[G21]** The Rust van Genuchten conductivity uses the Mualem pore-connectivity exponent `l` (`Par(6)`, default 0.5) instead of a hardcoded `Se^1.5`, matching `MATERIAL.FOR:29` and `:76`.
  - Result: commit `1d4131a` (2026-11-11). Workspace 525 tests pass with no existing expectation changed. Water record checked: the Phase 4 water acceptance evidence (`PHASE_4_WATER_ACCEPTANCE_EVIDENCE.md`) records contract tests with inputs, not pinned conductivity outputs, and all five acceptance tests pass. The Fortran-compatible solute path has its own conductivity and is unaffected. No pinned water result needed re-pinning. Dated entry in [STATE_24.md](STATE_24.md).

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

- **[G12 original]** Coupled heat, vapor and energy capability, as first stated in the M5-V handoff. Reframed by D1 (2026-11-11): M6 is heat only; vapor and energy are G19 and G20.
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

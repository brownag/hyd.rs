# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-12
Source State: [STATE_24.md](STATE_24.md)

## Circled Goal (Active)

- None. G19 is complete with exception; G22 (next dependency-ready coverage row, not yet selected) is the next candidate.


## Sequenced Goals (Dependency DAG)

- **[G22]** All valid vapor capabilities and supported interactions are implemented and capability-specifically validated, rather than stopping at G19's restricted first increment.
  - Depends on: G19 initial acceptance; individual rows may also depend on G20, G23 or separately accepted optional physics.
  - Scope and exit: every active vapor branch and valid interaction has a contract, implementation, discriminating source evidence, independent/balance checks and acceptance status in [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md). Invalid combinations are distinguished from deferred capabilities. Campbell, enhancement-off, remaining heat/water boundaries and related interactions remain explicit work, not claims inherited from G19.
  - Trigger to plan the next increment: G19 accepted with exception (2026-11-12); select the next dependency-ready coverage row, not yet selected. Remains open; no execution authorized by the current plan.

- **[G20]** Energy balance (`lEnBal`, `UpdateEnergy`, surface energy terms `Evap`, `Rn`, `Rns`, `Rnl`) is added to the coupled capability with its own contract, fixtures and acceptance gate.
  - Depends on: G19 (evaporation uses the vapor path), and on the atmospheric boundary path with meteorological input (`lMeteo`, `MeteoIn`, `INPUT.FOR:966`, `HYDRUS.FOR:243-244`), which the Rust crate does not yet have.
  - Trigger to start: G19 accepted with exception (2026-11-12), conditional on the vapor interactions G20 requires (atmospheric and flux boundaries are not accepted by G19; they are G22 rows to be resolved or scoped here).

## Completed

- **[G19]** Vapor flow is added to the coupled heat capability: `lVapor` terms (`TempAdj`, `ConVapor`, `VaporContent`, `ConVh`, vapor output `NOD_INF_V.OUT`) are implemented with explicit coupling contracts, rejected combinations preserved until supported, provenance-backed fixtures, and capability-specific validation.
  - Result (2026-11-12): **complete with exception** for the bounded first increment. Contract, configuration, kernels, water rows, heat transport, state, atomic driver, output, analytical, balance and Rust-vs-Fortran parity evidence are recorded in [M6_VAPOR_ACCEPTANCE.md](master/M6_VAPOR_ACCEPTANCE.md) (contract [M6_VAPOR_CONTRACT.md](master/M6_VAPOR_CONTRACT.md), [coverage](master/M6_VAPOR_COVERAGE.md)).
  - Exception: 18 isothermal `v_vapor`/`v_vap_term` rows fail criteria v2 against the pinned fixture; explained by source single-precision `TempN` ([diagnosis](master/M6_VAPOR_PRECISION_DIAGNOSIS.md)) and not claimed as matched. Criteria v2 was a post-failure owner-approved revision of v1. Vapor content evidence is pinned-kernel only; wet fixtures do not discriminate head/moisture/temperature. Restricted to the first-increment selection.
  - Original statement: Vapor flow is added to the coupled heat capability (`lVapor`, `TempAdj`, `ConVapor`, `VaporContent`, `ConVh`, `NOD_INF_V.OUT`).
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

- **[G23]** Temperature-dependent liquid-water coupling (`lWDep` / `lWTDep`, including `ConLT` in water flux and liquid material temperature adjustments) is separately contracted, implemented and validated when selected.
  - Trigger: separate user scope approval after G19 (complete with exception); remains deferred. Remains rejected in orchestration 25; not conflated with vapor's thermal-gradient terms or G20 energy balance.

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

# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-24
Source State: [STATE_32.md](STATE_32.md) (plan [EXECUTION_PLAN_32.md](EXECUTION_PLAN_32.md))

Magic Wand refresh (2026-11-24, user-approved): ranking criteria unchanged (retire Fortran risk early, land on main soon, minimize gate load). Circle is one root goal plus an orthogonal no-code cleanup track. G24 is complete (PR #1, `3f328fc`). New candidates: G28 (Picard switching deviation, folded into G26), G29 (hysteresis, roadmap M7) and G30 (input deck and CLI, roadmap M10).

Magic Wand refresh (2026-11-24, second, user-approved): ranking criteria unchanged. G26 and G28 are complete (orchestration 32). G25 is circled. G32 (merge `feat/met-run-path` to `main`) is new and is G25's step 0. G25 -> G20 is now hard: `UpdateEnergy` reads `vTop` directly (`TIME.FOR:1166-1195`), and Rust's head-mode top flux is the 0.0 fallback. G30 moves to the Independent Backlog, because nothing on the list waits on it.

Dependency graph: `G32 -> [G25, G20, G30]` (soft; clean base on `main`), `G25 -> [G20]` (hard). G22.2-C7, G27, G29, G30 and G31 have no open dependencies.

## Circled Goal (Active)

- **[G25]** At head-type ends (top in head mode, fixed-head bottom), the water solver reports the solved boundary flux, not the 0.0 fallback. The Rust vTop/vBot match the pinned Fortran `T_LEVEL.OUT` vTop/vBot for `vapor_met_bare_soil_on`.
  - Why now: it is the only code goal that unblocks another. G20's `UpdateEnergy` reads `vTop` (`TIME.FOR:1166-1195`), so G20 cannot be right on head-mode steps without it. It retires the last Fortran-facing G22.3 exception (head-end flux checked as interior conservation only), and it has the smallest scope of the code goals.
  - Found in 31.4.2 (2026-11-23): `upper_for` (`atmospheric.rs:484`) and the FixedHead lower pass flux 0.0, so `total_face_fluxes` at those ends is a fallback (`water_solver.rs:584`, `measured_flux().unwrap_or(0.0)`).
  - Exit criteria:
    - The pinned-Fortran vTop/vBot series is produced through `tools/run_vapor_reference.py`, with provenance.
    - A parity test compares the Rust vTop/vBot against it within user-approved bounds.
    - `vapor_met_balance` checks the head ends against the solved flux.
    - Heads and all other fields stay bit-identical. Every existing parity suite passes unchanged.
    - The full AGENTS.md suites are green.
  - Scope:
    - `src/water_solver.rs`, `src/atmospheric.rs` (`upper_for`).
    - `tools/run_vapor_reference.py` (outputs list near `:239`; `runner_sha256` is in the fixture manifest, so a runner change moves the hash pins) and `tools/test_run_vapor_reference.py`.
    - The met fixture or a new vTop/vBot sidecar, and `tests/vapor_met_balance.rs`.
    - `criteria.json` only with approved bounds.
    - Out of scope: G20, G31.
  - Gate: edits `water_solver.rs`, and a runner/fixture change (same gate class as G22.2-C4). The user decides between extending the fixture and adding a sidecar, the hash-pin consequences, and the vTop/vBot bounds.
  - Critical path:
    - (0) G32 merge (user action).
    - (1) Read-only audit: the Fortran head-end vTop/vBot (the vTop form at `WATFLOW.FOR:395-407` and the `T_LEVEL.OUT` write site) against the Rust end-row residual.
    - (2) Gate decision.
    - (3) Solved head-end flux in `water_solver.rs`, report-only, with heads bit-identical.
    - (4) The runner emits `T_LEVEL.OUT`; regenerate the fixture or sidecar.
    - (5) vTop/vBot parity test; balance test upgraded.
    - (6) Full suites.
  - First action: run plan-wave on G25, starting with the step (1) audit.
  - Depends on: G26 (done); G32 (soft).

- **[G22.2-C7]** (parallel, no code) Cleanup. The `planning/STATE_29.md` rows for 29.3.3 and 29.4.x are corrected from pending. About 90 stale worktrees (`git worktree list`, 91 lines on 2026-11-24) and unmerged `docs/master-plan-*` and other task branches are resolved after each is checked for unmerged work. The failed-trial output in `.development/29.4.1/runs/B` (1.5 GB) and `.development/29.3.3/quarantine/` (16 KB) are removed.
  - Why now: touches no code, so it is orthogonal to G25. It cuts noise for future orchestrations.
  - Gate: a read-only report comes first. Any `rm`, `git worktree remove` or branch deletion needs explicit user approval.

## Sequenced Goals (Dependency DAG)

- **[G32]** `feat/met-run-path` (orchestration 32) is merged into `main` through a PR.
  - Why: keeps `main` current under the "land on main soon" criterion, and gives G25 a clean base. No code.
  - Gate: user action (push, PR and merge). The agent prepares nothing remote without explicit instruction.
  - Depends on: none.

- **[G20]** Energy balance (`lEnBal`, `UpdateEnergy`, surface energy terms `Evap`, `Rn`, `Rns`, `Rnl`) is added to the coupled capability with its own contract, fixtures and acceptance gate.
  - Status: radiation (`Rn`, `Rns`, `Rnl`) ported in Plan 31 (31.2.2, `b7acedf`) per D-4. Trigger met 2026-11-23 (G22.3 accepted).
  - Depends on: G25 (hard: `UpdateEnergy` reads `vTop`, `TIME.FOR:1166-1195`). G26 is done.

## Independent Backlog

- **[G30]** A legacy input-deck reader and CLI run a supported HYDRUS case end to end from its input files (roadmap M10 groundwork). Start with the met vapor case and the M4 water cases.
  - Why backlog: unblocked since G26, but no other goal waits on it and its scope is large. Reconsider if user-facing capability becomes a ranking criterion.

- **[G27]** A met discrimination entry (vapor fields nonzero for `vapor_met_bare_soil_on`) is approved and added to `criteria.json`.
  - Gate: D-9. A user-approved criteria edit and the hash-pin updates (as in 31.2.3).

- **[G29]** Hysteresis (roadmap M7, Lenhard) is contracted and implemented with rollback-safe reversal state and drainage/imbibition fixtures.
  - Why backlog: large new capability, orthogonal to the met and energy chain. Its entry evidence (M4 rollback and history contract) already exists.

- **[G10]** Investigate the Robin refinement anomaly only if a real supported-use case shows it could change a decision; the current finding is nonblocking.

## Deferred

- **[G22.5]** Enhancement off (`iEnhanc=0`) evidence strategy decided and implemented. The kernel harness can select it; the full executable cannot via input.
  - Trigger: a user decision on the evidence strategy.

- **[G22.6]** Campbell (`iCampbell=1`) vapor path: resolve SD-006 policy, add discriminating fixtures, no silent source correction.
  - Trigger: SD-006 policy decision by the user.

- **[G23]** Temperature-dependent liquid-water coupling (`lWDep` / `lWTDep`, including `ConLT` in water flux and liquid material temperature adjustments) is separately contracted, implemented and validated when selected.
  - Trigger: separate user scope approval. Remains rejected in orchestration 25; not conflated with vapor's thermal-gradient terms or G20 energy balance.

- **[G9]** Time-axis convergence is re-validated with three time-axis study levels or dual-axis centre reuse.
  - Trigger: only if a real supported-use case needs a stronger temporal claim or an observed timestep effect could change a decision.

- **[G31]** In-loop Picard switching on the met path, so that adaptive dt becomes possible. Fortran switches the top mode inside the Picard loop; Rust does not yet.
  - Trigger (ADR 0022): a request for adaptive dt on the met path, or a met case that fails parity.

## Completed

- **[G26]** A production met run path exists in `src/`: `MetForcing` (`meteo::forcing`) and `VaporWaterHeatDriver::step_atmospheric`, with the `src/` entry points `hydrus_core::MetForcing`, `VaporWaterHeatDriver::step_atmospheric` and `VaporAtmosphericStepReport`.
  - Result (2026-11-24): **complete**. Delivered by `fc16698`, `5beb4ad`, `a6a2211`, `123ac63` and `a6c2ee4`. `vapor_met_parity` 5/5 and `vapor_met_balance` 3/3 pass with unchanged tolerances and criteria. Workspace 704 passed, 0 failed, 4 ignored. Plan [EXECUTION_PLAN_32.md](EXECUTION_PLAN_32.md).
- **[G28]** Picard switching deviation (D-10) decided and recorded.
  - Result (2026-11-24): **resolved**. Step-level re-solve kept, limited to fixed-schedule runs. Record: [ADR 0022](../docs/decisions/0022-re-solve-the-step-on-atmospheric-switches-with-vapor.md) (`e8bbb3f`).

- **[G24]** `feat/vapor-flux-fixedhead` is merged into `main`.
  - Result (2026-11-24 check): merged as PR #1 (`3f328fc`); `main` contains G22.2 closure and G22.3.

- **[G22.3]** Atmospheric water switching is implemented with vapor, with its meteorological and evaporation prerequisites contracted.
  - Result (2026-11-23): **accepted with exceptions** (user decision). Bare soil, daily `Meteo.in`, fixed-head lower, vapor on. Parity `vapor_met_parity` (D-10 gate: 12 fields within the unchanged bounds, switches on the Fortran steps) and balance `vapor_met_balance` (precedent tolerance on all 2400 steps). Exceptions: head-end boundary flux checked as interior conservation only (G25), met run path only in the test driver (G26), no met discrimination entry (G27). Record: [M6_VAPOR_G22_3_ACCEPTANCE.md](master/M6_VAPOR_G22_3_ACCEPTANCE.md). Sub-goals M1 to M7 were Plan 31 tasks 31.2.1, 31.3.1 to 31.3.3, 31.4.1, 31.4.2 and 31.5.1.
- **[G22.2]** Water-flux and free-drainage boundary pairs (and supported mixed pairs) are implemented with vapor, including vapor boundary/storage accounting and balance evidence.
  - Result (2026-11-23): **closed with exceptions** (user decision). C1 to C6 done; C5 entry `d2d1f32`; flux upper with fixed-head lower implemented (`880c6c2`) with exact parity. Forms accepted but not implemented stay listed in the coverage matrix; the dry (-10000 cm) divergence is recorded. Record: [M6_VAPOR_G22_2_ACCEPTANCE.md](master/M6_VAPOR_G22_2_ACCEPTANCE.md), history in [STATE_30.md](STATE_30.md) and [STATE_31.md](STATE_31.md).

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
- **[G7]** Free-drainage end-row assessment.
  - Status: Pruned
  - Reason: superseded by G22.2-C6, which carried the same assessment inside G22.2 (closed 2026-11-23).

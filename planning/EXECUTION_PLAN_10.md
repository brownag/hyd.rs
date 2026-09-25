# Orchestration 10 — M5 single-domain solute transport

Status: **approved by the user on 2026-09-24; planning only.**
Authority: [Master roadmap](master/ROADMAP_AND_RISKS.md), M5 / Phase 5a; [Phase 4 water acceptance evidence](master/PHASE_4_WATER_ACCEPTANCE_EVIDENCE.md).

## Goal and scope

Plan and implement the next master-plan milestone: single-domain solute transport coupled to already accepted Phase 4 water steps. M5 covers aqueous advection and dispersion, species and sequential reaction, sorption and transfer, solute mass accounting, explicit validation of unsupported combinations, and bounded acceptance fixtures. Water transport state is supplied to solute stepping as an accepted face-flux snapshot; solute stepping must not alter the accepted water state.

In scope: an explicit validated solute configuration and state; advection/dispersion over the existing 1D mesh; species and source-anchored sequential first-order decay; linear and nonlinear equilibrium sorption plus kinetic nonequilibrium sorption/transfer for one water domain; concentration and prescribed-total-solute-flux boundaries; species and sorbed-pool mass balance; trial/accepted transaction and rollback; typed rejection of unsupported combinations; in-memory contract fixtures with provenance and limitations; M5 evidence and a later-milestone handoff.

Out of scope: heat, vapor/energy, temperature-dependent transport, dual-domain water flow or dual-porosity hydraulics, hysteresis, density-dependent flow/transport, root solute uptake, bacteria/colloid/filtration, special chemistry, broad legacy input/output, CLI/host adapters, release compatibility, performance claims, and Fortran parity claims. The solute nonequilibrium pool in M5 is distinct from M8 dual-porosity water.

### Verbatim constraints carried forward

> “Do not widen the first water acceptance to include solute, heat, hysteresis, dual porosity, or optional physics.”

> “The first accepted product is single-domain water flow with atmospheric boundaries and root water uptake.”

These are respected as historical Phase 4 constraints; Orchestration 10 starts the separately gated M5 solute milestone and does not reopen or expand the first-water acceptance.

## Reconnaissance

- Repository: `/home/andrew/AI/hyd.rs`; clean tree at plan start; no applicable root `AGENTS.md` or `CONTRIBUTING.md` found.
- Highest prior orchestration is 9. `STATE_9.md` records Waves 1–7 complete. Phase 4 evidence allows Phase 5 **planning** only; it explicitly requires a solute scope, fixture/provenance plan, concentration and balance observables, and typed unsupported behavior before implementation.
- Current baseline re-run passed: `cargo test --workspace` (88 integration tests, 0 failures); `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- The Rust crate is `crates/hydrus-core`, currently with no solute module. Relevant existing references: `hydraulics/linear.rs` and `tests/tridiagonal.rs` for a reusable solve; `water_solver.rs` for matrix assembly and accepted-step reports; `water_state.rs` for fixed-size workspaces, snapshots and rollback; `water_acceptance.rs` for provenance-labeled in-memory scenarios.
- Source anchors: `src_reference/SOLUTE.FOR:6-230` (`Solute`, species chain, solve/commit order); `:1025-1076` (`Disper`); `:1078-1200` (`NEquil`); `:1319-1450` (`MassTran`); `:945-957` (`BanSol`). Use source as algorithm context, not as an executable oracle. Related design anchors are `master/NUMERICAL_DESIGN.md` (transport order, memory and rollback), `master/FEATURE_MATRIX.md` (supported and deferred branches), and `master/VALIDATION_STRATEGY.md` (missing manual decks and fixture requirements).
- Manual Chapters 11.3–11.5 are candidate provenance only; machine-readable decks and expected outputs are absent. Plan uses deterministic in-memory contract cases and makes no calibrated or parity claim.
- Workspace gates are serialized at each wave boundary. All tasks are serial because they share the new solute module, state/workspace APIs, and water-coupling contract. Workers run only their task's targeted test command.

## Decisions

| ID | Decision |
|---|---|
| D01 | (2026-09-24; carried) M5 solute only; M6 heat and M7/M8 later physics remain separate milestones. |
| D02 | (2026-09-24; user-selected) Include M5 sorption and transfer along with advection/dispersion, species/reaction, and solute balance: linear/nonlinear equilibrium sorption and kinetic nonequilibrium sorption/transfer. Keep water single-domain; solute nonequilibrium pools do not imply dual-porosity water. |
| D03 | (2026-09-24; user-selected) Build bounded deterministic in-memory fixtures; document inputs as independent contract cases and do not invent manual expected outputs. |
| D04 | (2026-09-24; user-selected) Pass an explicit accepted face-flux snapshot from the water step into solute transport. Solute stepping does not reconstruct water fluxes or mutate water state. |
| D05 | (2026-09-24; user-selected) Support concentration and prescribed total-solute-flux boundary modes; reject other boundary codes/combinations unless independently established by the scoped evidence. |
| D06 | (2026-09-24; carried) Preserve typed unsupported errors, preallocated fixed-dimension workspaces, accepted/trial state semantics, and serial workspace gates. |
| D07 | (2026-09-24; carried) Do not claim source-release identity, physical calibration, Fortran parity, universal tolerances, or performance improvement without the required evidence. |

## Entry gate, hazards, and wave table

Before Wave 1, record `git status --short`; verify the baseline commands above. Stop before implementation if any gate fails for an unrelated reason or the Phase 4 accepted-state contract/evidence is unavailable. Preserve the baseline record in `STATE_10.md`.

Hazards: concentration and mass units need explicit labels; face flux orientation must agree with positive-into-domain water signs; boundary advection and dispersion terms must not be double-counted; species decay-chain ordering must be explicit; sorbed and dissolved pools must be included exactly once in storage and reaction accounting; rejected trials must restore all species and sorbed pools; M5 immobile solute storage must not become dual-domain water; source branches with uncertain compatibility must fail explicitly; the Fortran source and manual cases do not provide an executable numerical oracle.

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 1 | 10.1.1 | Entry gate | Define validated single-domain solute types and unsupported-combination matrix |
| 2 | 10.2.1 | 10.1.1 | Add solute accepted/trial state, workspace and transactional lifecycle |
| 3 | 10.3.1 | 10.2.1 | Implement advection/dispersion solve and selected boundary rows |
| 4 | 10.4.1 | 10.3.1 | Add species chain, sequential first-order reaction and sorption/transfer |
| 5 | 10.5.1 | 10.4.1 | Add accepted solute balance and integrated water/solute contract fixtures |
| 6 | 10.6.1 | 10.5.1 | Record M5 evidence and later-milestone handoff |

Waves are serial, one task per wave. Each worker task is limited to 12 micro-plan steps and listed files. After each implementation wave, run `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` serially. Commit each accepted task separately using one conventional commit sentence without orchestration/wave numbers. Use the escalation ladder and plan gate in `PROMPT_TEMPLATE_10.md`.

## Task specifications

### 10.1.1 — Define the validated solute capability contract

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** M5 entry gate and accepted Orchestration 9 / Phase 4 water contract.
- **Must edit:** `crates/hydrus-core/src/error.rs`, `crates/hydrus-core/src/lib.rs`; **can create:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/tests/solute_contract.rs`, `planning/tasks/10.1.1.todo.md`; scratch `.development/10.1.1/`.
- **Anchors:** `rg -n 'WaterStepError|pub use|WaterProductConfig|LowerBoundaryMode' crates/hydrus-core/src/error.rs crates/hydrus-core/src/lib.rs crates/hydrus-core/src/water_solver.rs`; `rg -n 'lLinear|lEquil|lNEquil|lDualNEq|ChemIn' src_reference/SOLUTE.FOR src_reference/INPUT.FOR`.
- **Reference:** `WaterProductConfig` and typed selector errors; `hydraulics::Mesh1D` validation; `SOLUTE.FOR:6-45` species ordering/configuration as source context.
- **Spec:** Define public, validated solute species/transport configuration with explicit units, species ordering, sequential first-order reaction, linear and nonlinear equilibrium sorption, kinetic nonequilibrium sorption/transfer, and the selected boundary variants (concentration and prescribed total solute flux). Define typed invalid and unsupported errors. Require single-domain accepted water input; explicitly reject heat/vapor, density, bacteria/filtration, special chemistry, and dual-porosity water coupling. Anchor nonlinear isotherm and transfer equations to named `SOLUTE.FOR` routines; list source options outside this selected M5 subset as unsupported rather than silently substituting behavior. This task defines contracts only; do not implement stepping.
- **Do not touch:** Water solver/state behavior, hydraulics, forcing, dependencies, I/O, historical plans/states.
- **Acceptance:** `cargo test -p hydrus-core --test solute_contract`; tests exercise valid config, units and species order, malformed/nonfinite parameters, supported boundary selection, and typed rejection for each deferred combination. No numerical output or parity expectation.

### 10.2.1 — Add transactional solute state and workspace

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 10.1.1.
- **Must edit:** `crates/hydrus-core/src/solute.rs`; **can edit:** `crates/hydrus-core/src/lib.rs`; **can create:** `crates/hydrus-core/tests/solute_transaction.rs`, `planning/tasks/10.2.1.todo.md`; scratch `.development/10.2.1/`.
- **Anchors:** `rg -n 'struct WaterState|struct WaterWorkspace|snapshot\(|restore\(' crates/hydrus-core/src/water_state.rs`; `rg -n 'accepted|trial|SorbN|SorbN2|Conc\(' src_reference/SOLUTE.FOR`.
- **Reference:** `WaterState`/`WaterWorkspace` snapshot and restore; `tests/water_transaction.rs`; solute accepted/trial promotion in `SOLUTE.FOR:79-90, 159-185`.
- **Spec:** Add solute state and fixed-dimension workspace for dissolved concentrations and selected sorbed/transfer pools. Snapshot all mutable solute pools before a trial, promote only after successful acceptance, and restore exactly on rejected/error trials. Validate node/species dimensions against the mesh/config before mutation. Allocate during setup only; no inner-step allocation for fixed dimensions. Keep water state externally owned and unchanged.
- **Do not touch:** Transport matrix assembly, water APIs/solver, hydraulic modules, optional physics, dependencies, unrelated tests.
- **Acceptance:** `cargo test -p hydrus-core --test solute_transaction`; cover dimension mismatch without mutation, successful promotion, forced rejection rollback across every pool and accounting field, and workspace reuse after restore.

### 10.3.1 — Implement advection/dispersion and selected solute boundaries

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 10.2.1.
- **Must edit:** `crates/hydrus-core/src/solute.rs`; **can create:** `crates/hydrus-core/tests/solute_transport.rs`, `planning/tasks/10.3.1.todo.md`; scratch `.development/10.3.1/`.
- **Anchors:** `rg -n 'solve_tridiagonal|PivotDiagnostics' crates/hydrus-core/src/hydraulics/linear.rs`; `rg -n 'ResetRow|assemble|transform_boundary_row' crates/hydrus-core/src/water_solver.rs`; `rg -n 'subroutine Solute|subroutine Disper|subroutine MatSet|subroutine BanSol|kTopCh|kBotCh' src_reference/SOLUTE.FOR`.
- **Reference:** the reusable tridiagonal kernel and `tests/tridiagonal.rs`; water row assembly as a structural example only; source `Disper`, `MatSet`, and `BanSol` for algebra context.
- **Spec:** Assemble and solve a single-domain, conservative 1D dissolved-solute advection/dispersion step on the existing mesh using the explicit accepted face-flux snapshot and accepted water content. Implement configured concentration and prescribed total-solute-flux boundary rows with documented signs; reject unsupported/ill-posed boundary combinations. Validate all input lengths/finite values before mutation. Use preallocated coefficient/RHS buffers and the shared solver kernel. Do not add decay, sorption, transfer, water-flux reconstruction, artificial dispersion, or tolerance relaxation in this task.
- **Do not touch:** Water solver/state, hydraulics kernel, config types from 10.1.1 except necessary private helpers in `solute.rs`, dependencies, I/O.
- **Acceptance:** `cargo test -p hydrus-core --test solute_transport`; include zero-flow diffusion, signed advection direction, closed-system conservation, concentration boundary, prescribed-flux boundary, invalid input atomicity, and singular/pivot error cases. Use hand-computed small systems or invariant checks, not invented Fortran outputs.

### 10.4.1 — Add sequential reaction, sorption and transfer

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 10.3.1.
- **Must edit:** `crates/hydrus-core/src/solute.rs`; **can edit:** `crates/hydrus-core/src/error.rs`; **can create:** `crates/hydrus-core/tests/solute_reactions.rs`, `planning/tasks/10.4.1.todo.md`; scratch `.development/10.4.1/`.
- **Anchors:** `rg -n 'subroutine Coeff|subroutine NEquil|subroutine MassTran|decay|Omega|GamL|GamS|xKs|xNu|fExp' src_reference/SOLUTE.FOR`; `rg -n 'Sorption|solute|reaction' planning/master/FEATURE_MATRIX.md planning/master/NUMERICAL_DESIGN.md`.
- **Reference:** source `SOLUTE.FOR` sequential species loop, `NEquil`, and `MassTran` as algebra context; Rust validation/error conventions from `root_uptake.rs` and `tests/root_uptake.rs`.
- **Spec:** Implement the user-approved M5 sorption and transfer scope for one water domain: linear and nonlinear equilibrium isotherms, and kinetic nonequilibrium sorption/transfer as represented by the source routines and parameter contract from 10.1.1. Include explicit species-chain ordering and source-anchored first-order decay. For accepted nonequilibrium solute pools, conserve dissolved-plus-sorbed mass absent reaction/boundary exchange. Keep dual-domain water, bacteria/filtration, density and special chemistry rejected. Couple reaction and sorption terms consistently into the transport solve and diagnostics; never silently clip negative or nonfinite results into success.
- **Do not touch:** Water physics, dual-porosity water, heat/vapor, optional chemistry, dependencies, adapters, unrelated fixtures.
- **Acceptance:** `cargo test -p hydrus-core --test solute_reactions`; cover sequential decay-chain mass accounting, linear and nonlinear equilibrium sorption conservation/retardation, nonequilibrium transfer conservation and direction, limiting parameter cases, invalid rates/parameters, and typed rejection of deferred branches. Record the algebra/units for each equation in test comments or module docs.

### 10.5.1 — Integrate solute balance and M5 acceptance fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 10.4.1.
- **Must edit:** `crates/hydrus-core/src/solute.rs`; **can create:** `crates/hydrus-core/tests/solute_acceptance.rs`, `planning/tasks/10.5.1.todo.md`; scratch `.development/10.5.1/`.
- **Anchors:** `rg -n 'struct WaterBalance|balance|accepted_time|boundary_flux' crates/hydrus-core/src/water_solver.rs`; `rg -n 'Solute\(|cvTop|cvBot|cvCh0|cvCh1|cvChR|cvChIm|NodOut|SubReg' src_reference/SOLUTE.FOR src_reference/OUTPUT.FOR`; `rg -n 'Solute reaction/sorption|tolerance|provenance' planning/master/VALIDATION_STRATEGY.md`.
- **Reference:** `WaterBalance` for sign/diagnostic shape, without reusing water-only formulas; provenance conventions in `tests/water_acceptance.rs`.
- **Spec:** Add accepted-step solute accounting by species, including dissolved storage, every supported sorbed/transfer pool, boundary inflow/outflow, and reaction mass conversion/production. State units, sign convention, residual and zero-denominator behavior. Couple only against an accepted water state/time and a correctly sized face-flux snapshot; reject mismatched times or shapes before any mutation. Add bounded in-memory fixtures for conservative transport and the selected nonequilibrium reaction/sorption behavior, with inputs, provenance class, observables, and limitations documented per case. Do not assert calibrated trajectories, manual expected outputs, or Fortran parity.
- **Do not touch:** Master evidence/history files (reserved for 10.6.1), water solver behavior, fixture reference data, I/O, dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test solute_acceptance`; every fixture checks accepted times, concentration/pool observables, species mass closure, rejected-step identity, units/provenance and unsupported-mode errors. Wave gate also passes.

### 10.6.1 — Record M5 evidence and handoff

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 10.5.1 and all previous waves.
- **Must edit:** `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`; **can create:** `planning/tasks/10.6.1.todo.md`.
- **Anchors:** `rg -n 'M5|solute|nonequilibrium|sorption|transfer' planning/master/ROADMAP_AND_RISKS.md planning/master/FEATURE_MATRIX.md planning/master/VALIDATION_STRATEGY.md`; `rg -n 'test result|Status:|cargo test' planning/STATE_10.md`.
- **Reference:** Phase 4 record structure in `master/PHASE_4_WATER_ACCEPTANCE_EVIDENCE.md`; accepted reports in `STATE_10.md`.
- **Spec:** Record delivered M5 contract, exact supported and rejected chemistry/boundary combinations, equations/signs/units, water-flux coupling, transactional behavior, fixture provenance and exact tests, balance evidence, and limitations. Assess M5 exit criteria without claiming physical validation, calibration, parity, release compatibility, or performance. Identify remaining M6/M7/M8 gates as separate future work; do not authorize them.
- **Do not touch:** Implementation, prior evidence or historical plan/state files, roadmap scope, fixture expectations.
- **Acceptance:** Structural review traces every M5 requirement to accepted tests/reports or an explicit evidence limitation; supported/unsupported behavior, source anchors, exact commands, fixture provenance, balance terms, and next-milestone boundaries are recorded. Orchestrator runs all workspace gates serially after this final wave.

## Review notes

- **Spec pass:** M5 master requirements map to Waves 3–6; user decisions D02–D05 are represented in the contracts and fixtures. M6 heat and later optional physics are explicit deferrals. No implementation begins from this planning artifact.
- **Standards pass:** One task per serial wave avoids shared-file parallel edits. Every task names files, anchors, references, a targeted command, and a ≤12-step worker micro-plan ceiling. Large `water_solver.rs` is navigated by anchors. Workspace gates are serial; no unsupported numerical oracle is assumed.
- **Judgment call surfaced for approval:** M5 includes linear and nonlinear equilibrium sorption plus kinetic nonequilibrium transfer in a single water domain. Bacteria/filtration, density, special chemistry, and dual-domain water remain rejected; this follows the M5/M8 boundary in the master roadmap.

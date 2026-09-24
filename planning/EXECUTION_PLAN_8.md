# Orchestration 8 — Phase 4 first water acceptance

Status: **approved by the user on 2026-09-24; implementation has not started.**
Master authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 4.
Phase 3 entry evidence: [Phase 3 water solver evidence](master/PHASE_3_WATER_SOLVER_EVIDENCE.md), updated by Orchestration 7 and verified at commit `52c2cc7`.
This is a new orchestration set. Orchestration 6 remains a historical, unstarted draft and is not overwritten.

## Goal and scope

Implement and accept the first single-domain water product over the completed Phase 3 solver. Add a minimal forcing/input contract, event-aligned atmospheric precipitation and evaporation, ponding and upper head/flux switching, supported lower fixed-head/prescribed-flux/free-drainage boundaries, and one documented root stress model with compensation. Produce provenance-labeled dry-column infiltration, capillary-rise, cyclic atmospheric ponding/switching, and root-stress/compensation fixtures with mass-balance sign-off and typed errors for unsupported combinations. Record Phase 4 evidence and the Phase 5 handoff.

In scope: `iModel=0`; validated water-product capability/configuration; event scheduling and rollback-safe event state; atmospheric forcing and upper switching; the supported lower boundary modes; one root stress/compensation formulation; deterministic in-memory fixtures for dry-column infiltration, capillary rise, cyclic atmospheric ponding/switching, and root stress/compensation; balance diagnostics; unsupported-combination validation; evidence and Phase 5 handoff.

Out of scope: solute, heat, vapor/energy, hysteresis, dual porosity, density, snow/interception, tile drainage, inverse methods, special chemistry, broad legacy I/O, CLI/host bindings, release compatibility, calibrated universal tolerances, Fortran parity without an executable and reviewed fixture outputs, and performance claims.

### Verbatim constraints carried forward

> “Do not widen the first water acceptance to include solute, heat, hysteresis, dual porosity, or optional physics.”

> “The first accepted product is single-domain water flow with atmospheric boundaries and root water uptake.”

> “Phase 4 execution remains pending and has not started.”

## Reconnaissance

- Repo: `/home/andrew/AI/hyd.rs`, one Cargo workspace member: `crates/hydrus-core`.
- No root `AGENTS.md` or `CONTRIBUTING.md` was found. Follow the Rust conventions and worker gates in `PROMPT_TEMPLATE_7.md`, `PROMPT_TEMPLATE_6.md`, and adjacent implementation/tests.
- Relevant source sizes: `water_solver.rs` 374 lines, `water_state.rs` 128, `error.rs` 144, `lib.rs` 16. Large source entry points must be navigated by grep anchors, not read wholesale.
- Test infrastructure: Cargo unit/integration/doc tests; no servers or shared ports. Workers run only their named targeted tests. The orchestrator runs workspace tests serially at each wave boundary.
- Latest recorded baseline: Orchestration 7 reports `cargo test --workspace` 59 passed, 0 failed at `26ae9bb` and `2a3b422`; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed. This is the recorded baseline, not a fresh run at plan authoring time. Record current git status and re-establish the baseline before Wave 1.
- Scientific/Fortran executable baseline, reviewed numerical fixture outputs, and calibrated tolerances remain unavailable. New fixture provenance must say whether values are independently designed or source/manual-derived; do not label these as parity.
- Current implementation references: `advance_water_step` and `WaterStepInput` in `water_solver.rs`; accepted/trial ownership and workspace setup in `water_state.rs`; generic endpoint-row validation in `hydraulics/boundary.rs`; target-time clipping and transaction tests in `tests/water_transaction.rs`; mesh-weighted water balance in `water_solver.rs`; typed errors in `error.rs`.
- Fortran anchors live under `reference/fortran_reference/` and the planning source audit, not `src_reference/`. Useful anchors: `rg -n 'TmCont|Meteo|Evapor|Snow|SetBC' reference/fortran_reference planning`; `rg -n 'Shift|qGWLF|FreeD|SeepF|FAlfa|FSAlfa|SetSnk' reference/fortran_reference planning`; Rust anchors: `rg -n 'WaterStepInput|WaterWorkspace|advance_water_step|target_time|Balance|Boundary' crates/hydrus-core/src crates/hydrus-core/tests`.

## Decisions

| ID | Decision |
|---|---|
| D01 | (2026-09-24; carried from Orchestration 6) Keep `iModel=0` as the only hydraulic family; unsupported selectors fail explicitly. |
| D02 | (2026-09-24; carried from Orchestration 6) Support lower fixed-head, prescribed-flux, and free-drainage modes; reject un-evidenced seepage/water-table combinations. |
| D03 | (2026-09-24; carried from Orchestration 6) Align forcing at events; accounting changes only on accepted steps; rejected trials restore all mutable state, including event cursors and boundary mode. |
| D04 | (2026-09-24; carried from Orchestration 6) Keep fixture provenance explicit and do not manufacture Fortran expected values. |
| D05 | (2026-09-24; approved by the user) Use the source-anchored Feddes stress curve with compensation for the first root model; keep other root formulations out of this acceptance. |
| D06 | (2026-09-24; carried forward) Execute serially because work extends shared water API/state and boundary behavior. Run each full workspace gate serially. |
| D07 | (2026-09-24; carried forward) Do not start Phase 4 implementation until this plan is approved and the baseline/Phase 3 entry gate passes. |

## Entry gate, hazards, and wave table

Before Wave 1, record `git status --short` and run `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`. If Phase 3 APIs/evidence are absent or these gates fail, stop and report; do not backfill Phase 3 in this orchestration.

Hazards: boundary switching must participate in nonlinear iteration and transaction restore; target-time clipping already exists and must be composed with forcing events; endpoint flux signs and balance terms are caller-sensitive; root sink compensation must respect total potential demand and per-node stress; no current reference executable or golden fixture exists. Do not claim physical calibration, parity, or release compatibility.

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 1 | 8.1.1 | Phase 3 entry gate | Validated first-water configuration and public capability contract |
| 2 | 8.2.1 | 8.1.1 | Forcing schedule, event alignment, atmospheric upper switching and ponding |
| 3 | 8.3.1 | 8.2.1 | Supported lower boundaries |
| 4 | 8.4.1 | 8.3.1 | Feddes root stress and compensation |
| 5 | 8.5.1 | 8.4.1 | Required in-memory first-water fixtures and acceptance checks |
| 6 | 8.6.1 | 8.5.1 | Phase 4 evidence and Phase 5 handoff |

All waves are serial. Each task may use at most 12 micro-plan steps, edits only listed files, and runs its targeted test. At each wave boundary the orchestrator runs full workspace tests, formatting, warning-denied Clippy, and diff checks serially before accepting/committing the task. Use the escalation ladder in the worker prompt and one conventional commit per accepted task.

## Task specifications

### 8.1.1 — Define validated first-water product configuration

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/error.rs`, `crates/hydrus-core/src/lib.rs`, `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/water_state.rs`; **can create:** `crates/hydrus-core/tests/water_contract.rs`, `planning/tasks/8.1.1.todo.md`; scratch `.development/8.1.1/`.
- **Depends on:** verified Phase 3 handoff.
- **Anchors:** `rg -n 'WaterStepInput|WaterWorkspace|advance_water_step|WaterStepReport|WaterBalance|HydraulicModelSelector' crates/hydrus-core/src crates/hydrus-core/tests`.
- **Reference:** existing validated inputs and typed errors in Phase 3; `HydraulicModelSelector` rejection in `hydraulics/types.rs`.
- **Spec:** Establish the minimal public configuration/capability boundary needed to express accepted single-domain water behavior and reject unsupported combinations before stepping. Preserve the current solver's filesystem-free API, typed error style, transaction ownership, and caller-supplied hydraulic model. Keep event and physical boundary implementation for dependent tasks. Avoid adding hypothetical general application config that the Phase 4 contract does not need.
- **Do not touch:** hydraulic equations, mesh/stencil algebra, unrelated modules, other planning artifacts, dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test water_contract`; tests cover valid `iModel=0` config, typed unsupported selector/combination rejection, and invalid dimensions/domains without state mutation. Wave gate also passes.

### 8.2.1 — Add event-aligned atmospheric forcing and upper switching

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/water_state.rs`, `crates/hydrus-core/src/error.rs`, `crates/hydrus-core/src/lib.rs`; **can create:** `crates/hydrus-core/src/forcing.rs`, `crates/hydrus-core/src/atmospheric.rs`, `crates/hydrus-core/tests/atmospheric.rs`, `planning/tasks/8.2.1.todo.md`; scratch `.development/8.2.1/`.
- **Depends on:** 8.1.1.
- **Anchors:** `rg -n 'target_time|advance_water_step|snapshot|restore|BoundaryCondition|Natural|Dirichlet|Neumann' crates/hydrus-core/src crates/hydrus-core/tests`; source anchors `rg -n 'TmCont|Meteo|Evapor|SetBC|Shift' reference/fortran_reference planning`.
- **Reference:** Phase 3 target clipping/restore in `water_solver.rs` and `water_transaction.rs`; generic row policy in `hydraulics/boundary.rs`.
- **Spec:** Add a validated in-memory forcing schedule with explicit event times and caller-declared units/meaning; clip accepted/retried solver targets at the next event. Implement precipitation/evaporation demand, ponding storage, and upper fixed-head/flux switching as a mode reevaluated during the nonlinear trial. Snapshot and restore every mutable atmospheric mode, ponded amount, and schedule cursor on rejection. No file parsing, meteo, snow, or interception.
- **Do not touch:** lower-boundary policy, root uptake, hydraulic model equations, host/I/O, dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test atmospheric`; tests cover event landing, rain/infiltration, evaporation limitation, ponding and switch transitions, retries restoring cursors/modes/storage, and typed invalid/unsupported inputs. Wave gate also passes.

### 8.3.1 — Add supported lower boundary modes

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/error.rs`, `crates/hydrus-core/src/lib.rs`; **can create:** `crates/hydrus-core/src/lower_boundary.rs`, `crates/hydrus-core/tests/lower_boundary.rs`, `planning/tasks/8.3.1.todo.md`; scratch `.development/8.3.1/`.
- **Depends on:** 8.2.1.
- **Anchors:** `rg -n 'BoundaryCondition|apply_boundary|BoundaryError|WaterStepInput' crates/hydrus-core/src crates/hydrus-core/tests`; source `rg -n 'qGWLF|FreeD|SeepF|Shift' reference/fortran_reference planning`.
- **Reference:** `hydraulics/boundary.rs` generic row transforms and existing boundary-row tests.
- **Spec:** Integrate lower fixed-head, prescribed-flux, and free-drainage variants with explicit sign conventions and validation. Reuse generic row plumbing where algebraically appropriate. Reject seepage-face and water-table modes/combos not established by this phase's evidence. Include the selected lower mode in accepted diagnostics if needed to make fixtures reviewable.
- **Do not touch:** forcing schedule semantics, root model, hydraulic formulas, I/O, dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test lower_boundary`; tests cover each supported mode, lower signed flux, invalid values, unsupported selector errors, and preservation of rollback behavior. Wave gate also passes.

### 8.4.1 — Add Feddes root stress and compensation

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/error.rs`, `crates/hydrus-core/src/lib.rs`; **can create:** `crates/hydrus-core/src/root_uptake.rs`, `crates/hydrus-core/tests/root_uptake.rs`, `planning/tasks/8.4.1.todo.md`; scratch `.development/8.4.1/`.
- **Depends on:** 8.3.1.
- **Anchors:** `rg -n 'sink|WaterStepInput|water_balance|accepted' crates/hydrus-core/src/water_solver.rs crates/hydrus-core/src/water_state.rs`; source `rg -n 'FAlfa|FSAlfa|SetSnk|root' reference/fortran_reference planning`.
- **Reference:** Phase 3 mesh-weighted sink balance in `water_solver.rs`; source-derived Feddes anchors summarized in `master/FEATURE_MATRIX.md` and `HYDRUS_RESEARCH.md`.
- **Spec:** Implement one validated Feddes reduction curve and a documented compensation rule. Compute node stress from current trial heads and root distribution; constrain uptake by supplied potential demand and available water, with units and signs explicit. Integrate sink effects with existing mesh-weighted water balance and accepted/rejected state. Reject invalid breakpoints, negative demands/weights, and unsupported root options.
- **Do not touch:** alternate S-shaped formulation, salinity, root growth, hydraulic laws, broad config redesign, dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test root_uptake`; tests cover stress breakpoints, no-stress and full-stress limits, compensation, integrated uptake cap, nonuniform mesh weighting, and invalid inputs. Wave gate also passes.

### 8.5.1 — Build first-water acceptance fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** **can create:** `crates/hydrus-core/tests/water_acceptance.rs`, test fixture files under `crates/hydrus-core/tests/fixtures/phase4/`, `planning/tasks/8.5.1.todo.md`; scratch `.development/8.5.1/`.
- **Depends on:** 8.4.1.
- **Anchors:** `rg -n 'fixture|tolerance|dry.column|cyclic|root|balance|provenance' planning/master/VALIDATION_STRATEGY.md planning/master/PHASE_3_WATER_SOLVER_EVIDENCE.md crates/hydrus-core/tests`.
- **Reference:** existing integration tests for water solve, transaction/retry, and mesh geometry; Phase 0–3 evidence files.
- **Spec:** Create deterministic in-memory dry-column infiltration, capillary rise, cyclic atmospheric ponding/switching, and root stress/compensation acceptance scenarios. Each scenario documents provenance (manual/source-derived, independently designed, or pending oracle), units, inputs, expected invariant/observable, mass-balance sign and normalization, accepted times, and unsupported behavior. Use analytic invariants and bounded contract assertions where expected reference outputs do not exist; do not invent golden numbers or claim parity. Test typed rejection for at least the unimplemented seepage/water-table combination.
- **Do not touch:** production source unless a specific acceptance gap is reported and the orchestrator amends this plan; no external reference data or dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test water_acceptance -- --nocapture`; all four scenarios pass bounded, provenance-labeled checks for event times, state, active modes, balance, and retry/rollback as relevant. Wave gate also passes.

### 8.6.1 — Record Phase 4 evidence and Phase 5 handoff

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** **can create:** `planning/master/PHASE_4_WATER_ACCEPTANCE_EVIDENCE.md`, `planning/tasks/8.6.1.todo.md`; scratch `.development/8.6.1/`.
- **Depends on:** 8.5.1.
- **Anchors:** `rg -n 'Phase 4|first water|fixture|balance|unsupported|Phase 5|parity|provenance' planning/HYDRUS1D_RUST_PORT_PLAN.md planning/master planning/STATE_8.md`.
- **Reference:** Phase 0–3 evidence and `STATE_8.md` task reports.
- **Spec:** Record delivered API and supported behavior, exact targeted/full test provenance, all fixture provenance and observables, balance policy, event/switch/retry transaction coverage, unsupported combinations, and limitations. Assess Phase 4 exit criteria from master plan and report any unmet evidence plainly. State Phase 5 readiness only if the actual exit evidence supports it; do not edit historical plans/states.
- **Do not touch:** implementation, master sequence, prior orchestration records, or test expectations.
- **Acceptance:** structural review confirms the evidence includes scope, supported/unsupported behavior, all four fixtures, provenance, mass balance, tests, parity/calibration limits, exit checklist, and Phase 5 handoff; run `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` serially.

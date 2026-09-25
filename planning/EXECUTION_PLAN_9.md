# Orchestration 9 — Phase 4 atmospheric remediation and completion

Status: **original Waves 1–5 complete; review follow-up Waves 6–7 approved by the user on 2026-09-24 and pending execution.**
Authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 4; this plan continues the approved Orchestration 8 scope and addresses its blocked atmospheric task.
Diagnosis: [Atmospheric non-convergence findings](ATMOSPHERIC_NONCONVERGENCE_FINDINGS.md).

## Goal and scope

Resolve the active-set non-convergence found in Orchestration 8.2.1, establish a stable and conservatively accounted atmospheric boundary contract, then complete the remaining approved first-water Phase 4 work: supported lower boundaries, Feddes root stress with compensation, provenance-labeled acceptance fixtures, and Phase 4 evidence. Orchestration 8 remains a historical record; its blocked task is superseded only after this plan is approved and executed.

In scope: atmospheric flux/head iteration and accepted/trial surface storage; event schedule clipping and rollback; upper boundary row/flux and mass-balance consistency; lower fixed-head, prescribed-flux, and free-drainage modes; Feddes root stress/compensation; dry-column infiltration, capillary rise, cyclic atmosphere and root fixtures; unsupported-combination validation; Phase 4 evidence and Phase 5 handoff.

Out of scope: solute, heat, vapor/energy, hysteresis, dual porosity, density, snow/interception, tile drainage, inverse methods, special chemistry, broad legacy I/O, CLI/host bindings, release compatibility, universal tolerance calibration, Fortran parity without an executable and reviewed output, and performance claims.

### Verbatim constraints carried forward

> “Do not widen the first water acceptance to include solute, heat, hysteresis, dual porosity, or optional physics.”

> “The first accepted product is single-domain water flow with atmospheric boundaries and root water uptake.”

> “Use the source-anchored Feddes stress curve with compensation for the first root model; keep other root formulations out of this acceptance.”

> “Do not start Phase 4 implementation until this plan is approved and the baseline/Phase 3 entry gate passes.”

## Reconnaissance and current state

- Repository: `/home/andrew/AI/hyd.rs`; branch `phase4-water-acceptance`; current head `7dbc60c` (`docs(planning): record atmospheric checkpoint`).
- No root `AGENTS.md` or `CONTRIBUTING.md` applies. Follow adjacent Rust source/tests and `PROMPT_TEMPLATE_8.md` conventions; commit discipline is in `git-workflow-standards`.
- Orchestration 8 Wave 1 (water capability) is complete. Wave 2 is blocked: the accepted/trial active upper mode cycles on the rain and evaporation cases; full atmospheric test is 7 passed / 2 failed at checkpoint reproduction. No production changes are made by this plan authoring.
- The atmospheric tests are in `crates/hydrus-core/tests/atmospheric.rs` (392 lines); the solver is `water_solver.rs` (567 lines). Navigate by anchors below.
- Targeted reproduction commands and iteration traces are recorded in `ATMOSPHERIC_NONCONVERGENCE_FINDINGS.md`. The agent's trace used a temporary isolated copy; it did not modify this checkout.
- Baseline is not re-established for this new plan. Before Wave 1, record `git status --short` and run workspace tests, fmt, warning-denied Clippy, and `git diff --check`. Existing untracked research prompt/findings and this plan set must be accounted for explicitly.
- The archived Fortran source is algorithm context only. There is no executable or calibrated numerical oracle. Preserve independent/source-derived fixture provenance; do not claim parity.

## Decisions

| ID | Decision |
|---|---|
| D01 | (2026-09-24; carried) Keep `iModel=0` as the only hydraulic family; unsupported selectors fail explicitly. |
| D02 | (2026-09-24; carried) Support lower fixed-head, prescribed-flux, and free-drainage modes; reject un-evidenced seepage/water-table combinations. |
| D03 | (2026-09-24; carried) Align forcing at events; accounting changes only on accepted steps; rejected trials restore every mutable field, including atmospheric cursor, candidate/accepted ponding, and boundary mode. |
| D04 | (2026-09-24; carried) Keep fixture provenance explicit and do not manufacture Fortran expected values. |
| D05 | (2026-09-24; carried) Use the source-anchored Feddes stress curve with compensation; other root formulations remain out of scope. |
| D06 | (2026-09-24; carried) Execute serially because tasks share solver/state/boundary APIs; run workspace gates serially at wave boundaries. |
| D07 | (2026-09-24; diagnosis-derived) Resolve the active-set transition and row/flux/balance contract before relaxing tolerances, increasing iteration limits, or proceeding to dependent Phase 4 tasks. |
| D08 | (2026-09-24; approved by the user) After atmospheric remediation, continue the remaining approved Phase 4 scope in this orchestration. |

## Entry gate, hazards, and wave table

Before Wave 1, record `git status --short`; run `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`. The two atmospheric exact-test reproductions must be retained as initial failing evidence; do not treat the existing 7/9 result as a green baseline. If Phase 3 APIs/evidence are absent or gates fail for unrelated reasons, stop and report before implementation.

Hazards: active mode and candidate ponding must evolve within a Picard trial without mutating accepted state; the selected mode must correspond to the final assembled solve; forcing events and retries must compose with caller target clipping; upper soil flux, external atmospheric exchange, surface storage, and balance signs must reconcile; critical-head semantics for evaporation need explicit reasoning; no golden numeric oracle is available. A stable iteration cannot be manufactured by weakening tolerances or increasing iterations while the active set is cycling.

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 1 | 9.1.1 | Entry gate | Repair atmospheric active-set iteration and prove row/flux/balance accounting |
| 2 | 9.2.1 | 9.1.1 | Complete supported lower boundary modes |
| 3 | 9.3.1 | 9.2.1 | Feddes root stress and compensation |
| 4 | 9.4.1 | 9.3.1 | First-water acceptance fixtures |
| 5 | 9.5.1 | 9.4.1 | Phase 4 evidence and Phase 5 handoff |

Waves are serial. Each worker task is limited to 12 micro-plan steps and only its listed files. At each wave boundary the orchestrator runs workspace tests, formatting, warning-denied Clippy, and diff checks serially before accepting and committing. Workers run only their targeted test command. Use one conventional commit per accepted task and the escalation ladder in `PROMPT_TEMPLATE_9.md`.

## Task specifications

### 9.1.1 — Repair atmospheric active-set iteration and accounting

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** Phase 3 entry gate and accepted 8.1.1 capability contract.
- **Must edit:** `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/water_state.rs`, `crates/hydrus-core/src/atmospheric.rs`, `crates/hydrus-core/src/error.rs`; **can edit/create:** `crates/hydrus-core/tests/atmospheric.rs`, `planning/tasks/9.1.1.todo.md`; scratch `.development/9.1.1/`.
- **Read:** `ATMOSPHERIC_NONCONVERGENCE_FINDINGS.md`; Orchestration 8 task/report; Phase 3 water solver evidence; archived `WATFLOW.FOR` `Reset`/`Shift` excerpts as source context.
- **Anchors:** `rg -n 'solve_trial|assemble|atmospheric_top_flux|atmospheric_top_row|balance|NonConverged' crates/hydrus-core/src/water_solver.rs`; `rg -n 'snapshot|restore|AtmosphericState|upper_mode|ponded_depth' crates/hydrus-core/src/water_state.rs crates/hydrus-core/src/atmospheric.rs`; `rg -n 'accepted_rain|accepted_evaporation|ponded|rollback' crates/hydrus-core/tests/atmospheric.rs`.
- **Reference:** the findings' isolated traces, Phase 3 accepted/trial snapshot/restore, and `hydraulics/boundary.rs` row transforms.
- **Spec:** Replace the observed accepted-state-only mode fallback with a documented trial-local active-set/storage contract. Derive and document Rust signs for demand, soil flux, upper row and surface exchange against the source algebra without claiming parity; state what the available evidence does and does not establish about critical-head semantics for evaporation. Ensure convergence is checked against the mode used by the final solve. Keep candidate atmospheric state trial-local, promote only on acceptance, and discard it on retry/error. Preserve strict configured tolerances unless the evidence independently shows a separate tolerance defect. Address the prior rain balance residual with separate soil storage, surface storage, soil flux and external exchange accounting.
- **Do not touch:** lower boundary selection, roots, hydraulic equations, forcing file formats, dependencies, host/I/O, or unrelated tests/plans.
- **Acceptance:** `cargo test -p hydrus-core --test atmospheric`; both supplied exact rain/evaporation tests and the full suite pass. Add per-transition assertions for demand, mode, top head, soil flux and candidate surface storage; stable final active mode and converged head/water residuals; trial-vs-accepted ponding rollback; isolated fixed-flux and fixed-head row/flux consistency cases; and separate surface-storage and boundary-exchange balance terms. Tests must retain near-zero balance residual for ponded rain and supply-limited evaporation. Do not loosen tolerances or raise iteration limits to mask the active-set cycle. Wave gate also passes.

### 9.2.1 — Complete supported lower boundary modes

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 9.1.1.
- **Must edit:** `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/error.rs`, `crates/hydrus-core/src/lib.rs`, and existing `WaterStepInput` struct-literal call sites in `crates/hydrus-core/tests/{allocation_contract,atmospheric,water_contract,water_solver,water_transaction}.rs`; **can create:** `crates/hydrus-core/src/lower_boundary.rs`, `crates/hydrus-core/tests/lower_boundary.rs`, `planning/tasks/9.2.1.todo.md`; scratch `.development/9.2.1/`.
- **Anchors:** `rg -n 'BoundaryCondition|apply_boundary|BoundaryError|WaterStepInput' crates/hydrus-core/src`; `rg -n 'qGWLF|FreeD|SeepF|Shift' src_reference planning/master planning/HYDRUS_RESEARCH.md`.
- **Reference:** `hydraulics/boundary.rs` generic row transforms and accepted atmospheric integration.
- **Spec:** Add an explicit lower-boundary mode to `WaterStepInput`, with fixed-head, signed prescribed-flux, and free-drainage variants. Integrate each into lower-row assembly and accepted balance diagnostics; define the public sign convention and derive free-drainage signs from the named source and Rust row algebra. Preserve current behavior at every existing `WaterStepInput` construction site while updating those in-scope call sites for the new field. Reject seepage-face and water-table modes/combinations not established by evidence with typed errors. Keep selection and diagnostics transactional.
- **Do not touch:** atmospheric schedule or upper-switching behavior except call-site adaptation, root model, hydraulic laws, I/O, dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test lower_boundary`; tests cover each supported mode, signed flux, invalid values, unsupported modes, and rollback. Wave gate also passes.

### 9.3.1 — Add Feddes root stress and compensation

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 9.2.1.
- **Must edit:** `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/water_state.rs`, `crates/hydrus-core/src/error.rs`, `crates/hydrus-core/src/lib.rs`, and existing `WaterStepInput` call sites in `crates/hydrus-core/tests/{allocation_contract,atmospheric,lower_boundary,water_contract,water_solver,water_transaction}.rs`; **can create:** `crates/hydrus-core/src/root_uptake.rs`, `crates/hydrus-core/tests/root_uptake.rs`, `planning/tasks/9.3.1.todo.md`; scratch `.development/9.3.1/`.
- **Anchors:** `rg -n 'sink|WaterStepInput|water_balance|accepted' crates/hydrus-core/src/water_solver.rs crates/hydrus-core/src/water_state.rs`; `rg -n 'FAlfa|FSAlfa|SetSnk|root' src_reference/SINK.FOR planning/HYDRUS_RESEARCH.md planning/master/FEATURE_MATRIX.md`.
- **Reference:** Phase 3 mesh-weighted sink balance and source-derived Feddes anchors in `master/FEATURE_MATRIX.md` and `HYDRUS_RESEARCH.md`.
- **Spec:** Add an optional explicit root-uptake input to `WaterStepInput`, preserving all current behavior when absent. Implement the source-anchored piecewise Feddes reduction curve and compensation using `Omega = sum(alpha_i * beta_i * dx_i)`: use `Compen=OmegaC` for `0<Omega<OmegaC`, `Compen=Omega` for `Omega>=OmegaC`, and zero uptake when `Omega=0`; calculate node demand as `alpha_i * beta_i * TPot / Compen`. Treat beta as nonnegative root density per unit length, `TPot` as positive potential uptake depth/time, and sink as positive volumetric extraction rate. Cap node extraction at the source's active available-water limit `0.5 * max(theta_trial - theta(h_wilt), 0) / dt`, then derive/document the integrated actual uptake. Evaluate stress and realized uptake from current trial heads/water contents on every Picard iteration using a workspace-owned trial-sink buffer. Use realized sinks consistently in assembly, atmospheric top flux, weighted balance, and accepted accounting. Update every in-scope existing input fixture with no root uptake. Reject invalid breakpoints, negative demand/weights, and unsupported root options.
- **Do not touch:** alternate S-shaped formulation, salinity, root growth, hydraulic laws, broad config redesign, dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test root_uptake`; cover stress breakpoints, no/full stress, compensation, uptake cap, nonuniform mesh weighting, and invalid inputs. Wave gate also passes.

### 9.4.1 — Build first-water acceptance fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 9.3.1.
- **Can create:** `crates/hydrus-core/tests/water_acceptance.rs`, test fixtures under `crates/hydrus-core/tests/fixtures/phase4/`, `planning/tasks/9.4.1.todo.md`; scratch `.development/9.4.1/`.
- **Anchors:** `rg -n 'fixture|tolerance|dry.column|cyclic|root|balance|provenance' planning/master/VALIDATION_STRATEGY.md planning/master/PHASE_3_WATER_SOLVER_EVIDENCE.md crates/hydrus-core/tests`.
- **Reference:** existing integration tests for water solve, atmospheric switching, lower boundary, root uptake, and transaction/retry; Phase 0–3 evidence.
- **Spec:** Create deterministic in-memory dry-column infiltration, capillary rise, cyclic atmospheric ponding/switching, and root stress/compensation scenarios, including a combined cyclic-atmosphere/root-sink case that exercises demand, stress/compensation and water balance together. Each records provenance, units, observables, balance convention, accepted/event times, and limitations. Use analytic invariants and bounded assertions where no expected outputs exist; do not invent golden numbers or parity. Include typed rejection for unsupported seepage/water-table combinations.
- **Do not touch:** production source unless a concrete acceptance gap is reported and the orchestrator amends the plan; no external reference data or dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test water_acceptance -- --nocapture`; all four provenance-labeled scenarios pass bounded checks for event times, state, modes, balance and rollback as relevant. Wave gate also passes.

### 9.5.1 — Record Phase 4 evidence and Phase 5 handoff

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 9.4.1.
- **Can create:** `planning/master/PHASE_4_WATER_ACCEPTANCE_EVIDENCE.md`, `planning/tasks/9.5.1.todo.md`; scratch `.development/9.5.1/`.
- **Anchors:** `rg -n 'Phase 4|first water|fixture|balance|unsupported|Phase 5|parity|provenance' planning/HYDRUS1D_RUST_PORT_PLAN.md planning/master planning/STATE_9.md`.
- **Reference:** Phase 0–3 evidence files and `STATE_9.md` task reports.
- **Spec:** Record delivered API and supported behavior, exact test provenance, fixture provenance/observables, atmospheric transition and rollback evidence, surface/storage/external balance terms, root uptake, unsupported combinations, and limitations. Assess the Phase 4 exit criteria; state Phase 5 readiness only if evidence supports it. Do not edit historical plans/states.
- **Do not touch:** implementation, master sequence, prior orchestration records, or test expectations.
- **Acceptance:** structural review confirms the evidence covers scope, supported/unsupported behavior, the four fixtures (with root sink/stress exercised in the cyclic atmosphere scenario), provenance, mass balance, exact tests, parity/calibration limits, exit checklist, and Phase 5 handoff. The orchestrator runs `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` serially as the final wave gate.

## Execution policy

Workers write an at-most-12-step micro-plan and stop. The orchestrator checks scope, dependency reports, acceptance mapping, and source claims before sending `APPROVED — execute the plan as written.` One failed targeted verification receives one correction attempt; a second failure escalates to a mid-tier model, with no third small-worker attempt. Out-of-scope requirements require a plan amendment. Full workspace gates run serially at wave boundaries. Commit one conventional commit per accepted task.

## Code-review follow-up amendment — proposed 2026-09-24

The review of `7dbc60c...HEAD` reported one Spec-axis gap and one Standards-axis judgment call:

- Spec gap: 9.1.1 acceptance asks for “isolated fixed-flux and fixed-head row/flux consistency cases”; existing atmospheric tests exercise both modes in broader rain/evaporation scenarios rather than dedicated cases.
- Judgment call: repeated variant dispatch on `LowerBoundaryMode` appears in `validate`, `condition`, `flux_into_domain`, and lower-row selection. The user chose to consolidate this dispatch behind a shared private preparation/helper seam while preserving distinct physical calculations and behavior.
- No documented-standard violation or implementation-wrong finding was reported. The transition trace remains workspace-preallocated under the existing numerical memory contract.

Current baseline at `a7377c9` (`docs(water): record Phase 4 acceptance evidence`): final `cargo test --workspace` passed (88 integration tests); `cargo fmt --check`, warning-denied workspace Clippy, and `git diff --check` passed. The four untracked planning/research files recorded in the original state remain outside implementation scope.

### Review follow-up decisions

| ID | Decision |
|---|---|
| D09 | (2026-09-24; user-selected) Consolidate repeated `LowerBoundaryMode` dispatch behind shared private preparation/helpers; preserve public behavior and separate mode-specific row and exchange semantics. |
| D10 | (2026-09-24; carried from D06) Keep follow-up tasks serial and run workspace gates at each wave boundary. The lower-boundary refactor precedes the independent atmospheric regression tests. |

### Follow-up wave table

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 6 | 9.6.1 | Original Waves 1–5 complete | Consolidate lower-boundary mode dispatch |
| 7 | 9.7.1 | 9.6.1 (serial ordering) | Add isolated atmospheric upper flux/head consistency tests |

These waves address only the code-review findings. They do not reopen Phase 4 scientific validation or authorize Phase 5 implementation.

### 9.6.1 — Consolidate lower-boundary dispatch

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** Original Waves 1–5 complete.
- **Must edit:** `crates/hydrus-core/src/lower_boundary.rs`, `crates/hydrus-core/src/water_solver.rs`; **can edit:** `crates/hydrus-core/tests/lower_boundary.rs`, `planning/tasks/9.6.1.todo.md`; scratch `.development/9.6.1/`.
- **Anchors:** `rg -n 'impl LowerBoundaryMode|fn validate|fn condition|fn flux_into_domain|lower_boundary_mode' crates/hydrus-core/src/lower_boundary.rs crates/hydrus-core/src/water_solver.rs`; `rg -n 'fixed_head|prescribed_flux|free_drainage|unsupported|rollback' crates/hydrus-core/tests/lower_boundary.rs`.
- **Reference:** Existing `LowerBoundaryMode` behavior and typed transactional tests in `lower_boundary.rs` and `tests/lower_boundary.rs`.
- **Spec:** Centralize variant-specific preparation/dispatch for validation, boundary condition, lower-row strategy, and exchange flux behind one private preparation seam so the same mode distinctions are not independently re-dispatched across solver and mode methods. Keep fixed-head caller row/exchange, prescribed-flux signs, source-derived free-drainage signs, invalid-value errors, and unsupported-mode errors unchanged. Avoid public API changes or polymorphic indirection that obscures these physical differences.
- **Do not touch:** Atmospheric scheduling/upper switching, root uptake, hydraulic laws, public APIs, dependencies, or unrelated tests/plans.
- **Acceptance:** `cargo test -p hydrus-core --test lower_boundary` and `cargo test -p hydrus-core --test water_solver`; existing supported-mode, signed-flux, invalid/unsupported, and rollback assertions pass unchanged. Structural review confirms mode-specific validation, condition, row strategy, and exchange are prepared at one private seam and the solver consumes that result without repeating variant dispatch. Wave gate also passes.

### 9.7.1 — Add isolated atmospheric row/flux consistency cases

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 9.6.1 for serial task and gate ordering only; this task makes no production-source changes.
- **Must edit:** `crates/hydrus-core/tests/atmospheric.rs`; **can edit:** `planning/tasks/9.7.1.todo.md`; scratch `.development/9.7.1/`.
- **Anchors:** `rg -n 'accepted_rain|accepted_evaporation|atmospheric_top_row|atmospheric_top_flux|UpperBoundaryMode|soil_boundary_flux' crates/hydrus-core/tests/atmospheric.rs crates/hydrus-core/src/water_solver.rs`.
- **Reference:** Existing rain/head-limited flux assertions in `accepted_rain_infiltrates_then_excess_builds_ponded_storage` and the documented atmospheric signs in the Phase 4 evidence record.
- **Spec:** Add focused, independently named atmospheric tests for the upper prescribed-flux (`AtmosphericFlux`) mode and upper fixed-head (`PondedHead`) mode. For flux mode, configure demand and critical head so the demand-limited row is selected, then assert selected mode, signed soil/external flux, accepted state, and balance. For head mode, configure conditions that select the critical-head row without coupling assertions to transition history; assert top head equals the declared critical head and measured soil flux agrees with the discrete top-row flux expression and balance. Use deterministic fixture values and tolerances consistent with adjacent tests; do not add golden reference values or claim parity.
- **Do not touch:** Production source, lower-boundary/root APIs, schedules, dependencies, existing expected values except where a concrete regression is demonstrated, or other planning/history files.
- **Acceptance:** `cargo test -p hydrus-core --test atmospheric`; the two isolated cases pass and fail if their respective upper row/mode/flux contract is reversed or inconsistent. Wave gate also passes.

# Orchestration 7 — Phase 3 solver correction and closure

Status: **approved by the user on 2026-09-23; Wave 1 in progress.**
Master authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 3.
Prior Phase 3 implementation/evidence: [Orchestration 5](EXECUTION_PLAN_5.md), [Phase 3 evidence](master/PHASE_3_WATER_SOLVER_EVIDENCE.md).
Phase 4 remains gated by the corrected Phase 3 exit evidence; [Orchestration 6](EXECUTION_PLAN_6.md) is planned but unstarted.

## Goal and scope

Resolve the remaining Phase 3 review findings and establish a trustworthy exit gate before first-water acceptance. Correct the validated hydraulic and table contracts, reevaluate material-specific hydraulics during Picard iteration, calculate mesh-weighted water balances, honor caller-supplied target times, remove duplicated stencil assembly, remove public table constructors that always fail, and make fixed-size allocation evidence complete and stable under the workspace test run.

In scope: these Phase 3 correctness and evidence gaps, including both non-blocking cleanup findings accepted by the user; updates to Phase 3 evidence and the Phase 4 entry checklist.

Out of scope: atmospheric/forcing schedules, physical boundary switching or policy, root uptake, first-water fixtures, solute, heat, vapor, hysteresis, dual porosity, legacy I/O, CLI/host bindings, new dependencies, Fortran parity, tolerance calibration, release compatibility, and performance claims. Phase 4 owns event schedules and physical switching; Phase 3 only honors a caller-supplied next target time.

### Verbatim user constraints

> “create a new wave orchestration plan to resolve remaining issues with plan 3/phase 3”
>
> “during code review be sure to assess whether we are ready to proceed to next phase of planning”
>
> “use planning/ for spec”

## Reconnaissance and entry gate

- Repository: `/home/andrew/AI/hyd.rs`; one Rust workspace and one member crate, `crates/hydrus-core`.
- No root `AGENTS.md`, `CONTRIBUTING`, or Rust coding-standards file was found. Follow the completed Phase 3 plan/prompt conventions and existing Rust style.
- Relevant entry points: `water_solver.rs` (337 lines), `water_state.rs` (122), `hydraulics/types.rs` (248), `hydraulics/table.rs` (130), `hydraulics/stencil.rs` (124); integration tests are separate files under `crates/hydrus-core/tests/`.
- Test infrastructure: Cargo unit, integration, and doc tests; no server, external service, or shared port. Run full workspace checks serially at each wave boundary. Targeted tests belong to the task worker.
- The earlier review-session `cargo test --workspace` passed, and its targeted allocation assertion passed. The fresh pre-wave run at `5066dad` failed only `fixed_size_steps_allocate_nothing_after_setup`, counting one allocation (another workspace run counted two); isolating the process-wide counter in a one-test integration binary is part of this plan. Fresh pre-wave formatting, workspace Clippy, and diff checks passed.
- Before Wave 1, record `git status --short` and run `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`. A scientific/Fortran baseline remains unavailable.
- Reference implementations: validated construction in `VanGenuchtenParameters::new`; analytic evaluation in `VanGenuchtenModel::evaluate`; table generation/interpolation in `HydraulicTables`; shared mesh metrics in `hydraulics/geometry.rs`; the source-algebra row contract in `hydraulics/stencil.rs`; transaction/retry behavior in `advance_water_step`; and bounded Rust-contract evidence in `PHASE_3_WATER_SOLVER_EVIDENCE.md`.
- Use grep anchors for code navigation: `rg -n 'VanGenuchtenParameters|HydraulicTables|derivatives|HydraulicModel' crates/hydrus-core/src/hydraulics crates/hydrus-core/tests`; `rg -n 'solve_trial|assemble\(|balance\(|WaterStepInput|advance_water_step|snapshot|restore' crates/hydrus-core/src/water_solver.rs crates/hydrus-core/src/water_state.rs crates/hydrus-core/tests`; `rg -n 'assemble_reset_stencil|ResetRow|ResetStencilInput' crates/hydrus-core/src/hydraulics/stencil.rs crates/hydrus-core/tests/reset_stencil.rs`.

### Review finding traceability

These findings come from the review of `git diff 14d3545...HEAD` at `5066dad` on 2026-09-23. The review reports are preserved in the conversation; the authoritative requirement sources and affected implementation anchors are:

| Review finding | Requirement source | Affected code / task |
|---|---|---|
| Picard reuses fixed conductivity/capacity instead of evaluating at trial heads | [NUMERICAL_DESIGN.md#time-and-nonlinear-solve](master/NUMERICAL_DESIGN.md), “evaluate hydraulics” each Picard pass | `water_solver.rs:121-169`, 7.2.1 |
| Storage and sink sums omit nodal widths | [NUMERICAL_DESIGN.md#balances-and-coupling](master/NUMERICAL_DESIGN.md), mass-balance record | `water_solver.rs:238-276`, 7.3.1 |
| Public parameter fields bypass validation | [EXECUTION_PLAN_3.md](EXECUTION_PLAN_3.md), validated-construction acceptance rule | `hydraulics/types.rs:4-9`, 7.1.1 |
| Table slopes are wrong/zero and tables lack unit metadata | [NUMERICAL_DESIGN.md#hydraulics-and-tables](master/NUMERICAL_DESIGN.md), value/derivative and unit metadata contract | `hydraulics/table.rs:68-128`, 7.1.2 |
| Phase 3 event alignment was deferred despite the master sequence | [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 3 row; [PHASE_3_WATER_SOLVER_EVIDENCE.md](master/PHASE_3_WATER_SOLVER_EVIDENCE.md), unsupported-behavior/handoff sections | `water_solver.rs:22-28,63-116`, 7.4.1 and 7.6.1 |
| Allocation assertion has incomplete path coverage and a process-wide counter was observed to count unrelated allocations | [tasks/5.3.1.todo.md](tasks/5.3.1.todo.md), step 11; [NUMERICAL_DESIGN.md#memory-and-precision](master/NUMERICAL_DESIGN.md) | `tests/water_transaction.rs:10-18,128-149`, 7.1.3 isolates the counter and 7.5.1 completes path coverage |
| Interior row assembly is duplicated | Phase 2 shared stencil contract in [PHASE_2_MESH_STENCIL_EVIDENCE.md](master/PHASE_2_MESH_STENCIL_EVIDENCE.md) | `hydraulics/stencil.rs:43-124`, `water_solver.rs:170-236`, 7.3.1 |
| Public table constructors always fail | No planned use in the Phase 1 table contract; unsupported selectors belong at `HydraulicModelSelector` | `hydraulics/table.rs:94-103`, 7.1.2 |

## Decisions

| ID | Decision |
|---|---|
| D01 | (2026-09-23) Keep generic caller-supplied target-time alignment in Phase 3; forcing schedules and physical switching remain Phase 4. |
| D02 | (2026-09-23) Table unit symbols are caller-declared metadata, validated as non-empty labels; do not convert values or introduce a preferred unit system. Water content is dimensionless. |
| D03 | (2026-09-23) At an interior table knot, derivatives use the interval to its right; at the final endpoint use the interval to its left. Finite-difference checks use points away from knots. |
| D04 | (2026-09-23) Include both cleanup findings: share the interior row calculation between the stencil and solver, and remove public table constructors that always return errors. |
| D05 | (2026-09-23) Allow at most two concurrent workers in Wave 1, in isolated worktrees; all later waves are serial because they share solver/evidence files. |
| D06 | (2026-09-23) Use `gpt-6-luna` workers; after the established one-retry limit, escalate once to `gpt-6-sol`. |
| D07 | (2026-09-23) Each wave requires targeted tests and serial workspace tests, formatting, Clippy with warnings denied, and diff checks before completion. |
| D08 | (2026-09-23) Amend 7.2.1 to include migrating `tests/water_transaction.rs` fixtures to the hydraulic-model input API and rerunning its targeted test. The public `WaterStepInput` migration otherwise leaves that integration test uncompilable; retain its transaction and no-mutation assertions. |

## Wave table

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 1 | 7.1.1, 7.1.2, 7.1.3 | Phase 3 implementation | Validated hydraulic contracts and isolated allocation harness |
| 2 | 7.2.1 | 7.1.1, 7.1.2, 7.1.3 | Material-specific hydraulic evaluation inside Picard iteration |
| 3 | 7.3.1 | 7.2.1 | Mesh-weighted balance and shared stencil-row calculation |
| 4 | 7.4.1 | 7.3.1 | Caller-supplied target-time alignment |
| 5 | 7.5.1 | 7.4.1 | Stable allocation evidence across accepted and retried paths |
| 6 | 7.6.1 | 7.5.1 | Corrected Phase 3 evidence and Phase 4 entry gate |

Wave 1 tasks have pairwise-disjoint file lists. Task 7.1.1 exclusively owns `api_contract.rs`, including removal of assertions against the table constructors that 7.1.2 removes; 7.1.2 updates the dedicated hydraulic-table tests and must not edit `api_contract.rs`. Run at most two workers concurrently in isolated worktrees; start the third task when a worker slot opens. Isolate the allocation harness before the first full-suite gate, then expand its accepted/retry coverage in Wave 5 after the solver API stabilizes. Each remaining wave has one task; it edits a shared solver/test/evidence boundary and must run serially. A dependent task may start only after every prerequisite is marked `done` and its report is recorded in `STATE_7.md`. Run full workspace checks at each wave end before committing one conventional commit per accepted task.

## Shared constraints

- Preserve the filesystem-free core, `f64` arithmetic, typed failures, strict no-fallback selectors, Phase 0–2 evidence, and generic (not physical) boundary-row contract.
- Keep immutable models caller-owned and shareable; mutable state/workspace remain simulation-owned. Hydraulic models are indexed by each mesh node's validated material index. Evaluate each node's model at its current trial head on every Picard iteration; store/reuse evaluation buffers allocated at workspace setup.
- Define balance quantities per unit cross-sectional area. Use nodal control-volume widths for storage changes and nodal sink rates; integrate caller-signed endpoint fluxes over the accepted timestep. Do not claim fixture calibration or source parity.
- A step must not advance beyond its caller-supplied target time. The caller owns event selection and repeats calls as needed; do not add schedules, callbacks, clocks, forcing interpolation, or boundary switching.
- Table labels document caller-declared units only; no unit conversion. Interpolation is piecewise linear with the D03 knot convention.
- Keep fixed-size accepted steps, retries, restore, assembly, elimination, and diagnostics allocation-free after setup; only setup/resizing and error construction are excluded. Allocation counting must not be contaminated by unrelated concurrently running tests.
- Do not rewrite completed Orchestration 5 task reports or historical review reports. Record corrections in this orchestration and refreshed Phase 3 evidence.
- Workers edit only the task's listed files and create their own `planning/tasks/7.*.todo.md`; temporary files go only in `.development/7.*`.

## Task specifications

### 7.1.1 — Make van Genuchten parameters validation-only

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** Phase 3 implementation; parallel with 7.1.2.
- **Must edit:** `crates/hydrus-core/src/hydraulics/types.rs`, `crates/hydrus-core/src/hydraulics/van_genuchten.rs`, `crates/hydrus-core/tests/api_contract.rs`.
- **Can create:** `planning/tasks/7.1.1.todo.md`.
- **Scratch only:** `.development/7.1.1/`.
- **Anchors:** `rg -n 'pub struct VanGenuchtenParameters|impl VanGenuchtenParameters|parameters\(\)|parameters\.(residual_water_content|saturated_water_content|alpha|n|saturated_conductivity)' crates/hydrus-core/src/hydraulics/types.rs crates/hydrus-core/src/hydraulics/van_genuchten.rs crates/hydrus-core/src/hydraulics/mod.rs crates/hydrus-core/tests/api_contract.rs`.
- **Reference:** current `VanGenuchtenParameters::new` validation and `VanGenuchtenModel::new`.
- **Spec:** Make parameter fields private so external struct literals cannot bypass validation. Add read-only accessors for all parameters; update internal analytic evaluation and tests to use them. Remove the API-contract assertions that call the table constructors scheduled for removal in 7.1.2; retain model-selector rejection checks. Preserve the existing parameter-constructor validation rules and `Copy` behavior if still valid.
- **Do not touch:** `table.rs`, exports, solver, stencil, unrelated hydraulic equations, dependencies, or historical planning records.
- **Acceptance:** `cargo test -p hydrus-core --test api_contract`; tests demonstrate valid parameter construction/accessors, typed invalid/non-finite failures, and unsupported model-selector rejection. Full wave checks also pass.

### 7.1.2 — Correct hydraulic table metadata, derivatives, and constructors

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** Phase 3 implementation; parallel with 7.1.1.
- **Must edit:** `crates/hydrus-core/src/hydraulics/table.rs`, `crates/hydrus-core/src/hydraulics/mod.rs`, `crates/hydrus-core/src/lib.rs`, `crates/hydrus-core/tests/hydraulic_tables.rs`.
- **Can create:** `planning/tasks/7.1.2.todo.md`.
- **Scratch only:** `.development/7.1.2/`.
- **Anchors:** `rg -n 'pub struct HydraulicTables|pub fn new|pub fn evaluate|from_selector|pub fn empty|fn derivatives' crates/hydrus-core/src/hydraulics/table.rs crates/hydrus-core/tests/hydraulic_tables.rs`.
- **Reference:** `VanGenuchtenModel::derivatives` in `hydraulics/mod.rs` for the derivative result contract; `NUMERICAL_DESIGN.md#hydraulics-and-tables` and `VALIDATION_STRATEGY.md#component-and-property-tests` for linear interpolation and knot-validation requirements. No existing table-derivative implementation is treated as a correct reference.
- **Spec:** Add caller-declared unit-symbol metadata to generated tables. Unit symbols have private contents, reject empty labels at construction, and are exposed read-only; attach no conversions or preferred units. Return segment slopes for theta, conductivity, and capacity from `HydraulicModel::derivatives`; use the right-hand segment at internal knots and the left segment at the final endpoint. Remove `from_selector` and `empty`, which always fail; keep unsupported-selector rejection at `HydraulicModelSelector::from_i32`. Preserve generated values, interpolation domain, and endpoint behavior.
- **Do not touch:** `types.rs`, analytic van Genuchten equations, solver, stencil, `tests/api_contract.rs` (owned by 7.1.1), or dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test hydraulic_tables`; assert labels survive construction, empty labels reject, slopes agree with segment arithmetic and off-knot finite differences, unsupported selectors still fail at `HydraulicModelSelector`, and endpoint/knot convention is deterministic. Full wave checks also pass.

### 7.1.3 — Isolate the allocation counter before workspace gates

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** Phase 3 implementation; parallel with 7.1.1 and 7.1.2, respecting the two-worker limit.
- **Must edit:** `crates/hydrus-core/tests/water_transaction.rs`.
- **Can create:** `crates/hydrus-core/tests/allocation_contract.rs`, `planning/tasks/7.1.3.todo.md`.
- **Scratch only:** `.development/7.1.3/`.
- **Anchors:** `rg -n 'GlobalAlloc|ALLOCATIONS|COUNTING|fixed_size_steps_allocate_nothing' crates/hydrus-core/tests/water_transaction.rs`.
- **Reference:** current allocation counter and the fixed-dimension contract in `NUMERICAL_DESIGN.md#memory-and-precision`.
- **Spec:** Move the existing accepted-step allocation assertion and its counting allocator into a dedicated integration-test binary containing exactly one test. Measure only the solver call after state/workspace setup; perform assertions after counting stops. Remove the counter from `water_transaction.rs` while retaining all transaction tests. This first slice isolates the current accepted-step proof; 7.5.1 completes accepted/retry coverage after API changes.
- **Do not touch:** production source, other test files, dependencies, or Phase 3 evidence.
- **Acceptance:** `cargo test -p hydrus-core --test allocation_contract -- --test-threads=1` and `cargo test -p hydrus-core --test water_transaction`; accepted-step assertion remains zero-allocation. Full wave checks also pass.

### 7.2.1 — Evaluate each material's hydraulics during Picard iteration

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 7.1.1, 7.1.2, and 7.1.3.
- **Must edit:** `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/water_state.rs`, `crates/hydrus-core/src/lib.rs`, `crates/hydrus-core/tests/water_solver.rs`, `crates/hydrus-core/tests/allocation_contract.rs`, `crates/hydrus-core/tests/water_transaction.rs`.
- **Can create:** `planning/tasks/7.2.1.todo.md`.
- **Scratch only:** `.development/7.2.1/`.
- **Anchors:** `rg -n 'WaterStepInput|WaterWorkspace|solve_trial|assemble\(|conductivities|capacities|HydraulicModel|material_indices' crates/hydrus-core/src/water_solver.rs crates/hydrus-core/src/water_state.rs crates/hydrus-core/src/hydraulics crates/hydrus-core/tests/water_solver.rs`.
- **Reference:** `VanGenuchtenModel::evaluate` in `hydraulics/mod.rs` as the concrete `HydraulicModel` implementation; `Mesh1D::material_indices()` in `hydraulics/types.rs`; and `WaterWorkspace::new` in `water_state.rs` for setup-time buffers.
- **Spec:** Replace caller-frozen conductivity/capacity arrays with a caller-owned model slice indexed by mesh material indices. Validate model count against `material_count`; on every Picard iteration evaluate each node's selected model at that node's current trial head and assemble from the resulting theta, conductivity, and capacity. Use model-evaluated theta for trial water contents and their convergence diagnostic; do not treat capacity as a frozen external input or derive final trial water solely from `C × Δh`. Allocate/reuse all per-node evaluation buffers in `WaterWorkspace::new`, not inside the solve loop. Preserve sink input, transaction ownership, typed hydraulic error propagation, and filesystem-free boundary. Export the trait at the crate root if required by the public input type.
- **Do not touch:** balance formula, time-target API, boundary policy, stencil implementation, tests other than `water_solver.rs`, `allocation_contract.rs`, and the amended `water_transaction.rs`, or dependencies.
- **Acceptance:** `cargo test -p hydrus-core --test water_solver`, `cargo test -p hydrus-core --test allocation_contract -- --test-threads=1`, and `cargo test -p hydrus-core --test water_transaction`; tests prove trial-head changes affect subsequent hydraulic values and theta, material mapping is honored, invalid model/material counts reject, transaction and no-mutation assertions remain covered, and the isolated accepted-step allocation test remains green. Full wave checks also pass.

### 7.3.1 — Weight water balance by mesh control volumes and share row assembly

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 7.2.1.
- **Must edit:** `crates/hydrus-core/src/hydraulics/stencil.rs`, `crates/hydrus-core/src/hydraulics/geometry.rs`, `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/tests/reset_stencil.rs`, `crates/hydrus-core/tests/mesh_geometry.rs`, `crates/hydrus-core/tests/water_solver.rs`.
- **Can create:** `planning/tasks/7.3.1.todo.md`.
- **Scratch only:** `.development/7.3.1/`.
- **Anchors:** `rg -n 'assemble_reset_stencil|ResetRow|dx_a|dx_b|let storage|let sink|fn balance|storage_change|boundary_flux' crates/hydrus-core/src/hydraulics/stencil.rs crates/hydrus-core/src/hydraulics/geometry.rs crates/hydrus-core/src/water_solver.rs crates/hydrus-core/tests`.
- **Reference:** Phase 2 `assemble_reset_stencil`, `adjacent_distance`, and `boundary_half_cell_width`.
- **Spec:** Extract one shared, allocation-free interior-row calculation used by both `assemble_reset_stencil` and the water solver; preserve the existing row algebra and output. Add a checked nodal control-volume width (half adjacent spacing at endpoints, mean adjacent spacing internally). Compute storage change as the sum of nodal water-content change times width and sink as the sum of nodal sink rate times width and accepted `dt`; integrate signed endpoint fluxes over accepted `dt`. Document all balance terms per unit cross-sectional area and retain the existing absolute/normalized residual and zero-denominator rule.
- **Do not touch:** hydraulic model API, table contracts, event target API, physical boundary policy, or allocation test infrastructure.
- **Acceptance:** `cargo test -p hydrus-core --test reset_stencil`, `cargo test -p hydrus-core --test mesh_geometry`, and `cargo test -p hydrus-core --test water_solver`; add a nonuniform-mesh hand calculation that would fail under unweighted sums, and assert both stencil callers produce the same interior rows. Full wave checks also pass.

### 7.4.1 — Align accepted steps to a caller-supplied target time

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 7.3.1.
- **Must edit:** `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/error.rs`, `crates/hydrus-core/tests/water_solver.rs`, `crates/hydrus-core/tests/water_transaction.rs`, `crates/hydrus-core/tests/allocation_contract.rs`.
- **Can create:** `planning/tasks/7.4.1.todo.md`.
- **Scratch only:** `.development/7.4.1/`.
- **Anchors:** `rg -n 'pub fn advance_water_step|WaterStepInput|timestep|minimum_timestep|state\.time|NonConverged' crates/hydrus-core/src/water_solver.rs crates/hydrus-core/src/error.rs crates/hydrus-core/tests/water_solver.rs crates/hydrus-core/tests/water_transaction.rs`.
- **Reference:** existing adaptive retry and accepted-time promotion in `advance_water_step`; `API_AND_IO.md#lifecycle-and-forcing` and `NUMERICAL_DESIGN.md#time-and-nonlinear-solve`.
- **Spec:** Add a caller-supplied finite target time to the generic step request. Clip the requested timestep and every retry so no accepted trial crosses the target; reach the target exactly when it is within the requested step. Reject a target at or before current accepted time with a typed error. The caller selects the next target and repeats calls; this task adds no schedule, clock, callback, event queue, forcing interpolation, or physical switching.
- **Do not touch:** atmospheric/forcing modules, boundary policy, hydraulic model or balance behavior, or planning evidence.
- **Acceptance:** `cargo test -p hydrus-core --test water_solver`, `cargo test -p hydrus-core --test water_transaction`, and `cargo test -p hydrus-core --test allocation_contract -- --test-threads=1`; cover a target inside the requested step, a target farther than one max step, invalid/backward targets, retry clipping, exact accepted-time landing, and continued allocation-test isolation. Full wave checks also pass.

### 7.5.1 — Make allocation evidence isolated and representative

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 7.4.1.
- **Must edit:** `crates/hydrus-core/tests/allocation_contract.rs`.
- **Can create:** `planning/tasks/7.5.1.todo.md`.
- **Scratch only:** `.development/7.5.1/`.
- **Anchors:** `rg -n 'GlobalAlloc|ALLOCATIONS|COUNTING|fixed_size_steps_allocate_nothing|retry|restore' crates/hydrus-core/tests/allocation_contract.rs`.
- **Reference:** existing test allocator and the documented fixed-dimension boundary in `NUMERICAL_DESIGN.md#memory-and-precision`.
- **Spec:** Extend the isolated one-test integration binary created in 7.1.3. Measure after setup and before assertions; cover a fixed-size accepted step and a successful rejected-trial/retry path, which exercises snapshot restore, assembly, and elimination. Both paths must remain allocation-free after setup. Do not count setup/resizing or error construction. Keep transaction assertions in `water_transaction.rs`; do not reintroduce its process-wide counter.
- **Do not touch:** production source, `water_solver.rs`, other tests, dependencies, or Phase 3 evidence.
- **Acceptance:** `cargo test -p hydrus-core --test allocation_contract -- --test-threads=1`; test asserts zero allocations for both measured paths. Also run `cargo test -p hydrus-core --test water_transaction`. Full workspace run at wave end must include the isolated binary and pass.

### 7.6.1 — Refresh Phase 3 evidence and record Phase 4 readiness

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 7.5.1 and all preceding tasks.
- **Must edit:** `planning/master/PHASE_3_WATER_SOLVER_EVIDENCE.md`, `planning/STATE_6.md`.
- **Can create:** `planning/tasks/7.6.1.todo.md`.
- **Scratch only:** `.development/7.6.1/`.
- **Anchors:** `rg -n 'Picard stepping|Balances and allocation|event alignment|Exit checklist|Phase 4 handoff|Phase 3 solver evidence' planning/master/PHASE_3_WATER_SOLVER_EVIDENCE.md planning/STATE_6.md planning/EXECUTION_PLAN_6.md`.
- **Reference:** completed task reports in `STATE_7.md`, the authoritative Phase 3/4 rows in `HYDRUS1D_RUST_PORT_PLAN.md`, and the test/limitation language in existing Phase 3 evidence.
- **Spec:** Replace claims contradicted by the review: record model evaluation at trial heads, weighted balance units and formulas, correct table derivative/unit contracts, generic target-time behavior, shared assembly, and the isolated allocation evidence. Preserve exact test provenance and explicitly retain the absence of Fortran parity/calibration/performance evidence. Update `STATE_6.md` Phase 3 entry checklist to reflect the corrected evidence and state whether Phase 4 planning may resume; keep Phase 4 execution pending and do not authorize or start it.
- **Do not touch:** implementation, historical Orchestration 5 reports, review reports, master phase sequence, or Orchestration 6 scope.
- **Acceptance:** verify all evidence headings and claims against source and completed `STATE_7.md` reports; links resolve; `git diff --check` passes. Final orchestrator acceptance runs `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` serially.

## Execution and escalation policy

Workers produce a micro-plan with at most 12 steps and stop for approval before editing implementation. Use one isolated worktree per Wave 1 task. No later-wave concurrency. Workers run only the targeted tests listed in their task. The orchestrator reviews diffs and runs the complete acceptance set serially after each wave, recording exact results and revision in `STATE_7.md`. Apply the full escalation ladder: (1) after two off-spec micro-plans, the orchestrator writes the fixed micro-TODO; (2) after one failed acceptance retry, stop the small worker and escalate once to `gpt-6-sol`; (3) for a missing/malformed completion report, give one corrective retry, then escalate once if still invalid; (4) out-of-scope file needs require rescoping this plan. Commit one conventional, one-sentence commit per accepted task, with no orchestration or wave number in the message; follow [git-workflow-standards](../../git-workflow-standards/SKILL.md).

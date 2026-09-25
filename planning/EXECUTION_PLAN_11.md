# Orchestration 11 — Resolve M5 solute review findings

Status: **planning only; implementation has not started.**
Authority: [Orchestration 10 M5 plan](EXECUTION_PLAN_10.md), [M5 evidence](master/PHASE_5_SOLUTE_EVIDENCE.md), and the review findings reported for `d98595d...HEAD`.

## Goal and scope

Correct the accepted-step solute storage diagnostic for intervals where water content changes, and remove the repeated transport-operator/boundary assembly so the transport-only and process paths share one implementation. Preserve the accepted water ownership boundary, M5 scope, selected boundary modes, signs, transactional rollback, and explicit scientific-evidence limitations.

Out of scope: new physics, water-solver changes, new boundary behavior, calibrated tolerances, manual/reference output claims, performance claims, dependency changes, and unrelated cleanup.

## Review findings addressed

1. **Spec correctness limitation:** `update_step_balances` calls `species_storage` for both pre-step and post-step solute values using the current accepted water contents. When theta changes, the storage difference omits the prior-theta contribution. The M5 evidence already discloses the missing prior-water snapshot.
2. **Standards judgment call — possible duplicated code:** `assemble_and_solve_species` contains a face transport stencil and boundary logic parallel to `assemble_flux_operator` and `apply_process_boundary_rows` used in the process path.

## Decisions and constraints

| ID | Decision |
|---|---|
| D01 | Supply both start-of-interval and accepted end-of-interval water contents explicitly to the solute step. Water state remains externally owned; do not copy its history into `SoluteState`. |
| D02 | Compute pre-step dissolved storage with start water content and post-step dissolved storage with accepted end water content; equilibrium and kinetic sorbed inventories retain their existing formulas. |
| D03 | Validate both water-content arrays for dimensions, finiteness, and positive values before `begin_trial` or any mutation. The start water content corresponds to the solute state's current time; the accepted snapshot remains at `state.time + timestep`. |
| D04 | Refactor transport and boundary coefficient assembly into shared private helpers used by both paths. Preserve matrix algebra, sign conventions, error behavior, and preallocated workspace use. |
| D05 | Keep this as a correctness/refactoring change. Do not claim that sharing the operator independently validates the numerical method. |

## Reconnaissance and baseline

- Repo: `/home/andrew/AI/hyd.rs`; working tree was clean when this plan was drafted.
- Highest existing orchestration is 10. `STATE_10.md` records the latest green workspace baseline: `cargo test --workspace` (110 passed, 0 failed), `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`.
- Per session instruction, no tests or validation commands were run while drafting this plan. Re-run the four recorded baseline gates serially before Wave 1 and record results here and in `STATE_11.md`; stop on unrelated failures.
- Main implementation anchors: `solute.rs` `advance_solute_step_with_processes`, `species_storage`, `update_step_balances`, `assemble_and_solve_species`, `assemble_and_solve_species_with_processes`, `assemble_flux_operator`, and `apply_process_boundary_rows`.
- Prior-water content is passed by the caller as start-of-interval input, not retained in `SoluteState`; this avoids changing accepted-state ownership and setup/transaction APIs.

## Wave table

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 1 | 11.1.1 | Baseline gate | Make storage accounting use interval start and accepted end water contents |
| 2 | 11.2.1 | 11.1.1 | Share transport and boundary operator assembly across process paths |

Waves are serial because both tasks edit `crates/hydrus-core/src/solute.rs`. Each worker task is limited to 12 micro-plan steps and its listed files. Workers run only their targeted tests. After each wave, the orchestrator runs the full workspace gates serially and commits the accepted task in one conventional commit. Follow [PROMPT_TEMPLATE_11.md](PROMPT_TEMPLATE_11.md) and the escalation ladder from Orchestration 10.

## Task specifications

### 11.1.1 — Account for changing water content

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** green baseline gate above.
- **Must edit:** `crates/hydrus-core/src/solute.rs`; **can edit:** `crates/hydrus-core/tests/solute_acceptance.rs`, `crates/hydrus-core/tests/solute_transport.rs`, `crates/hydrus-core/tests/solute_reactions.rs`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`; **can create:** `planning/tasks/11.1.1.todo.md`; scratch `.development/11.1.1/`.
- **Anchors:** `rg -n 'SoluteTransportInput|AcceptedWaterSnapshot|species_storage|update_step_balances|storage_before|validate_transport_input' crates/hydrus-core/src/solute.rs`; `rg -n 'storage_change|changing|water_contents|AcceptedWaterSnapshot' crates/hydrus-core/tests/solute_acceptance.rs crates/hydrus-core/tests/solute_transaction.rs`.
- **Reference:** Existing `species_storage`, accepted/trial accounting, and `solute_acceptance.rs` balance fixtures. Keep caller-owned water coupling as specified by M5 D04 in `EXECUTION_PLAN_10.md`.
- **Spec:** Add an explicit start-of-interval water-content input paired with the current accepted end-of-interval snapshot. Validate both arrays before trial mutation: each must match mesh node count, contain finite values, and contain positive values. Require the start contents to correspond to the current solute-state time by API contract/documentation; retain existing end accepted-time validation. Calculate pre-step dissolved inventory using start theta and pre-step concentration; calculate post-step inventory using end theta and accepted concentration. Include sorbed pools once using existing formulas. Preserve balances, cumulative updates, and rollback behavior for constant-theta inputs.
- **Do not touch:** water solver/state or public water APIs, workspace allocation policy beyond necessary fixed-size storage, process equations, dependencies, evidence/history docs, unrelated tests.
- **Acceptance:** Run `cargo test -p hydrus-core --test solute_acceptance`, `cargo test -p hydrus-core --test solute_transport`, and `cargo test -p hydrus-core --test solute_reactions` (the public input struct is constructed in all three targets). Add a varying-theta case with a hand-computed dissolved-storage change and residual, retain a constant-theta regression case, and verify malformed start-water dimensions/nonfinite/nonpositive values reject before accepted-state mutation. Update the M5 evidence limitation to describe the corrected interval accounting while retaining all unrelated provenance/calibration limitations. No physical calibration or parity claim.

### 11.2.1 — Share transport and boundary assembly

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 11.1.1.
- **Must edit:** `crates/hydrus-core/src/solute.rs`; **can edit:** `crates/hydrus-core/tests/solute_transport.rs`, `crates/hydrus-core/tests/solute_reactions.rs`; **can create:** `planning/tasks/11.2.1.todo.md`; scratch `.development/11.2.1/`.
- **Anchors:** `rg -n 'assemble_and_solve_species\(|assemble_and_solve_species_with_processes|assemble_flux_operator|apply_process_boundary_rows|SoluteBoundaryMode' crates/hydrus-core/src/solute.rs`; `rg -n 'Concentration|TotalFlux|advection|dispersion|boundary' crates/hydrus-core/tests/solute_transport.rs crates/hydrus-core/tests/solute_reactions.rs`.
- **Reference:** Existing `assemble_flux_operator` and `apply_process_boundary_rows`; preserve current tests as behavioral contracts. The shared `solve_tridiagonal` kernel remains unchanged.
- **Spec:** Route both transport-only and process-enabled solves through a shared private flux-operator assembly and shared endpoint-boundary assembly (or equivalent single source of coefficient logic). Keep process-specific retardation/reaction/source contributions separate from common transport and endpoint terms. Preserve lower/upper orientation, concentration and prescribed-total-flux semantics, pivots/errors, preallocated buffers, and no-allocation inner-step behavior. Avoid a broad abstraction that obscures the coefficient equations.
- **Do not touch:** `error.rs`, public configuration/API types, water/hydraulics modules, dependencies, unrelated tests/docs.
- **Acceptance:** `cargo test -p hydrus-core --test solute_transport` and `cargo test -p hydrus-core --test solute_reactions`; existing transport direction, conservation, endpoint, process-coupling, and rollback assertions pass unchanged or receive narrowly justified coverage for shared paths. No numerical behavior changes beyond task 11.1.1.

## Exit criteria

- The changing-water-content storage case accounts for both endpoint theta values and validates inputs atomically.
- Both solve paths share transport and endpoint assembly while preserving the existing behavioral tests.
- Workspace gates pass after each wave; M5 evidence limitations remain accurate, with no unsupported scientific claim added.

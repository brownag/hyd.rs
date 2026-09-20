# Orchestration 4 — Phase 2 mesh and source stencil

Status: **approved by the user on 2026-09-19; execution in progress.**
Master sequence authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 2.
Entry evidence: [Phase 1 hydraulic evidence](master/PHASE_1_HYDRAULIC_EVIDENCE.md).

## Goal and scope

Define the checked one-dimensional mesh/profile contract needed by the Rust water solver: validate profile inputs, convert Fortran one-based material/layer references once, retain orientation and inclination metrics, compute layer and face geometry, and port the observed `WATFLOW.FOR:Reset` finite-difference stencil for the implemented hydraulic model. Produce focused source-algebra tests and a reviewed Phase 2 handoff. This phase establishes mesh and assembly contracts; it does not implement time stepping, boundary switching, Picard iteration, linear solving, balances, or a water-product acceptance fixture.

In scope: `Mesh1D` validation and one-based conversion; layer/material index checks; signed coordinates and inclination/cosine metrics; boundary/interior half-cell and layer metrics; arithmetic face conductivity with an explicit geometric option; source-anchored storage/stencil coefficients; typed failures and focused tests; Phase 2 evidence and Phase 3 solver handoff.

Out of scope: atmospheric/root forcing, boundary-condition policy and `Shift`, `Gauss`/linear solve, timestep alignment/retries, accepted/trial state, rollback, mass balance, I/O/CLI/host bindings, solute/heat/hysteresis/dual porosity, alternate hydraulic families, general input tables, Fortran parity, fixtures, tolerance calibration, and performance/allocation claims.

### Verbatim constraints

> “Plan only the next incomplete phase in the sequence below.”

> “Do not widen the first water acceptance to include solute, heat, hysteresis, dual porosity, or optional physics.”

> “The source's trial iteration and accepted-state promotion are observed; the full accepted/trial snapshot and rollback transaction is a port design obligation to be validated in Phase 3.”

## Reconnaissance

- Phase 0 and Phase 1 are complete. Phase 0 smoke remains provenance-only; Phase 1 provides source-anchored hydraulic behavior, not numerical parity.
- Current green baseline: `cargo test --workspace` (13 passed), `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`.
- Existing entry points: `crates/hydrus-core/src/hydraulics/mod.rs` (shared API registry), `types.rs`, `van_genuchten.rs`, and `table.rs`; focused tests live under `crates/hydrus-core/tests/`.
- Source anchors: `WATFLOW.FOR:173-303` (`Reset`), especially `:190-228` for gravity, half-cell widths, arithmetic/geometric face means, and storage; `:78-95` for the caller contract; `HYDRUS.FOR:48-95` for one-based arrays, `x`, `MatNum`, `LayNum`, and `CosAlf`.
- `master/STATE_AND_CALLS.md` assigns mesh/profile ownership to coordinates, material/layer arrays, and inclination; `master/NUMERICAL_DESIGN.md` requires increasing coordinates, finite metrics, valid material indices, one-time zero-based conversion, explicit face averaging, and a reusable tridiagonal row contract.
- No project `AGENTS.md` or `CONTRIBUTING.md` applies to this repository. Rust formatting and clippy are the established conventions.

## Decision log

| ID | Decision |
|---|---|
| D01 | Phase 2 implements only the single-domain mesh and source `Reset` stencil required by the next solver phase; boundary policy remains a Phase 3 concern. |
| D02 | Public Rust mesh indices are zero-based after validated one-based adapter conversion; source numbering is not carried into solver internals. |
| D03 | Coordinates must be finite and strictly increasing; material/layer references are validated against declared counts; invalid input is a typed error. |
| D04 | Arithmetic face conductivity is the default; geometric averaging is an explicit option matching the source flag, with no silent fallback. |
| D05 | Preserve source finite-difference algebra and document the manual finite-element terminology reconciliation as an open validation item; no parity claim is made. |
| D06 | Retain the established worker policy: gpt-5.6-luna at low effort; one failed verification retry, then gpt-5.6-terra at medium effort. |

## Wave table

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 1 | 4.1.1 | Pre-wave gate | Checked mesh/profile API and source-index conversion |
| 2 | 4.2.1 | 4.1.1 | Layer, inclination, and face geometry metrics |
| 3 | 4.3.1 | 4.2.1 | `Reset` source stencil and coefficient tests |
| 4 | 4.4.1 | 4.3.1 | Phase 2 evidence and Phase 3 handoff |

All tasks are serial because `hydraulics/mod.rs` is the shared public module boundary and each task consumes the preceding contract. The orchestrator updates `STATE_4.md`, verifies targeted tests, and commits one conventional commit per accepted task.

## Pre-wave gate

Record these results in `STATE_4.md` before Wave 1:

```sh
git status --short
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Any pre-existing diff, failed baseline, or changed immutable source hashes blocks execution pending review.

## Shared constraints

- Preserve `src_reference/`, Phase 0 artifacts, Phase 1 hydraulic behavior/tests, and generated `.development/` artifacts.
- `hydrus-core` remains filesystem-free and owns no process-global or foreign-host state.
- Do not modify Fortran or planning master references in implementation tasks.
- Use `f64` at the current API boundary; do not claim compatibility precision, parity, fixture agreement, allocation freedom, or performance.
- Keep boundary-condition transformation, solver iteration, and transaction/rollback out of this phase.

## Task specifications

### 4.1.1 — Add validated Mesh1D and profile conversion

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** pre-wave gate.
- **Must edit:** `crates/hydrus-core/src/lib.rs`, `crates/hydrus-core/src/hydraulics/mod.rs`, `crates/hydrus-core/src/hydraulics/types.rs`, `crates/hydrus-core/src/error.rs` only as needed for typed mesh errors and public exports.
- **Can create:** `crates/hydrus-core/tests/mesh_contract.rs`, `planning/tasks/4.1.1.todo.md`.
- **Scratch only:** `.development/4.1.1/`.
- **Read:** `master/NUMERICAL_DESIGN.md#discretization`; `master/STATE_AND_CALLS.md#state-inventory`; `src_reference/HYDRUS.FOR:48-95`; Phase 1 task 3.1.1 report.
- **Reference implementation:** existing validated hydraulic types in `crates/hydrus-core/src/hydraulics/types.rs`; source `HYDRUS.FOR` one-based arrays.
- **Anchors:** `rg -n 'struct .*Model|enum HydraulicError|pub mod hydraulics' crates/hydrus-core/src`; `rg -n 'NumNP|MatNum|LayNum|x\(' src_reference/HYDRUS.FOR`.

Define a validated `Mesh1D` carrying finite strictly increasing coordinates, zero-based material indices, optional zero-based layer indices, declared material/layer counts, and a finite inclination cosine/metric. Provide an explicitly named adapter constructor that accepts source-style one-based indices and rejects zero, out-of-range, length mismatch, non-finite, and non-increasing inputs with typed errors. Expose only the mesh data needed by later geometry/stencil tasks; do not calculate coefficients or boundary rows yet.

**Acceptance:** `cargo test -p hydrus-core --test mesh_contract`; `cargo fmt --check`; `cargo clippy -p hydrus-core --all-targets -- -D warnings`. Tests must prove successful one-based conversion, zero-based internal indices, all listed validation failures, and no filesystem or solver dependency.

### 4.2.1 — Add mesh geometry, layer metrics, and face averaging

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 4.1.1.
- **Must edit:** `crates/hydrus-core/src/hydraulics/mod.rs` only for exports/registration.
- **Can create:** `crates/hydrus-core/src/hydraulics/geometry.rs`, `crates/hydrus-core/tests/mesh_geometry.rs`, `planning/tasks/4.2.1.todo.md`.
- **Scratch only:** `.development/4.2.1/`.
- **Read:** task 4.1.1 report; `master/NUMERICAL_DESIGN.md#discretization`; `src_reference/WATFLOW.FOR:173-228`.
- **Reference implementation:** `WATFLOW.FOR:197-204` boundary half-cell and arithmetic/geometric face calculations.
- **Anchors:** `rg -n 'dxB|dxT|ConB|ConT|lGeom|Grav|CosAlf' src_reference/WATFLOW.FOR`; `rg -n 'Mesh1D' crates/hydrus-core/src crates/hydrus-core/tests`.

Implement pure geometry helpers over `Mesh1D`: adjacent distances, boundary half-cell widths, layer spans/metrics, inclination/gravity metric access, and explicit arithmetic or geometric face conductivity averaging. Validate finite conductivity inputs and reject invalid geometric means without changing to arithmetic silently. Keep sign/orientation conventions explicit in names/docs and preserve signed coordinates.

**Acceptance:** `cargo test -p hydrus-core --test mesh_geometry`; `cargo fmt --check`; `cargo clippy -p hydrus-core --all-targets -- -D warnings`. Tests must cover boundary/interior distances, layer grouping, inclination, arithmetic versus geometric means, non-finite values, negative geometric inputs, and exact source-style half-cell results.

### 4.3.1 — Port the observed Reset stencil

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 4.2.1.
- **Must edit:** `crates/hydrus-core/src/hydraulics/mod.rs` only for exports/registration.
- **Can create:** `crates/hydrus-core/src/hydraulics/stencil.rs`, `crates/hydrus-core/tests/reset_stencil.rs`, `planning/tasks/4.3.1.todo.md`.
- **Scratch only:** `.development/4.3.1/`.
- **Read:** task 4.2.1 report; `src_reference/WATFLOW.FOR:173-303`; `master/NUMERICAL_DESIGN.md#discretization`; `master/VALIDATION_STRATEGY.md#component-and-property-tests`.
- **Reference implementation:** `WATFLOW.FOR:Reset`, especially `:197-228`, ported algebraically into a pure coefficient assembly function.
- **Anchors:** `rg -n 'subroutine Reset|dxB=|ConB=|Cap\(|dt|P\(|R\(|S\(' src_reference/WATFLOW.FOR`; `rg -n 'face|Mesh1D|HydraulicModel' crates/hydrus-core/src/hydraulics`.

Implement a pure, filesystem-free source-style interior finite-difference stencil that consumes validated mesh geometry, node conductivity/capacity, water contents or storage inputs required by the observed formula, timestep, gravity metric, and sink/storage terms. Return typed finite-value/domain failures and a documented lower/diagonal/upper/RHS row representation. Preserve arithmetic default and explicit geometric averaging. Do not implement `Shift`, boundary code semantics, tridiagonal elimination, convergence, or retries; boundary rows remain a Phase 3 integration concern.

**Acceptance:** `cargo test -p hydrus-core --test reset_stencil`; `cargo fmt --check`; `cargo clippy -p hydrus-core --all-targets -- -D warnings`. Tests must include hand-calculated two-cell/three-cell rows, gravity/inclination, storage/timestep contribution, sink sign, arithmetic/geometric face choices, non-finite rejection, and a source-algebra note distinguishing stencil evidence from parity.

### 4.4.1 — Record Phase 2 evidence and Phase 3 handoff

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 4.3.1.
- **Must edit:** none.
- **Can create:** `planning/master/PHASE_2_MESH_STENCIL_EVIDENCE.md`, `planning/tasks/4.4.1.todo.md`.
- **Scratch only:** `.development/4.4.1/`.
- **Read:** task reports 4.1.1–4.3.1; committed source/tests; `HYDRUS1D_RUST_PORT_PLAN.md` Phase 2/3 rows; `master/STATE_AND_CALLS.md`; `master/NUMERICAL_DESIGN.md`; Phase 1 evidence format.
- **Reference implementation:** `master/PHASE_1_HYDRAULIC_EVIDENCE.md` structure and the Phase 2 exit requirement in the master plan.
- **Anchors:** `rg -n 'Mesh1D|one-based|layer|inclination|face|stencil|Reset|parity' crates/hydrus-core/src crates/hydrus-core/tests`; `rg -n 'Mesh and source stencil|Water solver transaction' HYDRUS1D_RUST_PORT_PLAN.md`.

Review implementation and tests against this plan. Record the checked mesh contract, one-based conversion, geometry/layer metrics, inclination/sign convention, face averaging policy, source `Reset` algebra, typed failure behavior, explicit boundary/solver deferrals, and the reconciliation item between source finite-difference terminology and manual finite-element terminology. Recommend Phase 3 proceed with boundary-aware tridiagonal assembly and transactional Picard stepping. Do not call source-algebra tests Fortran parity or claim numerical baseline/tolerance/performance evidence.

**Acceptance:** run `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`; and a structure assertion requiring headings `## Mesh contract`, `## Source-index conversion`, `## Geometry and averaging`, `## Reset stencil`, `## Typed failures`, `## Exit checklist`, and `## Phase 3 handoff`, plus the phrases `not a Fortran parity result` and `finite-element terminology`.

## Execution policy

Workers write an at-most-12-step micro-plan and stop. The orchestrator verifies scope, dependency reports, acceptance mapping, and source claims before sending the literal `APPROVED — execute the plan as written.` message. Full workspace tests run serially after Wave 4. One conventional no-phase/wave commit is made per accepted task. A micro-plan off-spec twice is replaced by an orchestrator-written TODO; one failed targeted verification receives one retry, then escalation to gpt-5.6-terra. Out-of-scope requirements respecify this plan rather than widening worker permissions.

# State — Orchestration 4: Phase 2 mesh and source stencil

Plan status: **approved by the user on 2026-09-19.**
Execution status: **complete; Phase 2 evidence handoff recorded.**
Master authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 2.
Resume from [EXECUTION_PLAN_4.md](EXECUTION_PLAN_4.md); worker prompt: [PROMPT_TEMPLATE_4.md](PROMPT_TEMPLATE_4.md).

## Baseline checklist

- [x] Phase 0 handoff complete in `STATE_2.md` and `master/PHASE_0_REFERENCE_EVIDENCE.md`.
- [x] Phase 1 handoff complete in `STATE_3.md` and `master/PHASE_1_HYDRAULIC_EVIDENCE.md`.
- [x] Rust workspace and hydraulic foundation exist; no project `AGENTS.md` or `CONTRIBUTING.md` applies.
- [x] `cargo test --workspace`: 13 passed, 0 failed.
- [x] `cargo fmt --check` passed.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passed.
- [x] `git diff --check` passed; pre-wave worktree was clean.
- [x] Source anchors reviewed: `HYDRUS.FOR:48-95`; `WATFLOW.FOR:173-228`.

Scientific baseline: **unavailable.** Phase 2 source-algebra and property tests will not establish Fortran parity, universal tolerances, release compatibility, allocation behavior, or performance.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 4.1.1 | Validated Mesh1D and one-based conversion | done | c1180fc |
| 2 | 4.2.1 | Geometry, layer metrics, and face averaging | done | 6d2337c |
| 3 | 4.3.1 | Source `Reset` stencil | done | b60b6ef |
| 4 | 4.4.1 | Phase 2 evidence and Phase 3 handoff | done | 5132d37 |

## Task reports

<!-- Orchestrator appends concise task reports here. -->

## 4.2.1

- Status: done; geometry helpers and explicit face averaging implemented.
- Files: `crates/hydrus-core/src/hydraulics/geometry.rs`, `mod.rs`, `mesh_geometry.rs`, and task TODO.
- Layer metrics use the approved contiguous first-to-last coordinate span convention.
- Verification: targeted geometry tests passed (7); fmt, clippy, and `git diff --check` passed.
- No boundary, stencil, solver, filesystem, parity, or performance behavior added.

## 4.3.1

- Status: done; pure interior `Reset` stencil assembly implemented with typed finite/domain validation.
- Files: `crates/hydrus-core/src/hydraulics/stencil.rs`, `mod.rs`, `reset_stencil.rs`, and task TODO.
- Preserves arithmetic default and explicit geometric averaging; boundary rows, `Shift`, solves, retries, and parity remain deferred.
- Verification: targeted stencil tests passed (5); fmt, clippy, and `git diff --check` passed.

## 4.4.1

- Status: done; recorded reviewed Phase 2 mesh/stencil evidence and the bounded Phase 3 handoff in `master/PHASE_2_MESH_STENCIL_EVIDENCE.md`.
- Evidence covers the validated `Mesh1D` contract, explicit one-based source conversion, geometry/layer metrics, inclination/sign convention, arithmetic/geometric face averaging, typed failures, and the interior source `Reset` algebra.
- Explicitly preserves the finite-difference versus finite-element terminology reconciliation as an open validation item and states that source-algebra tests are not a Fortran parity result.
- Phase 3 handoff recommends boundary-aware tridiagonal assembly and transactional Picard stepping; boundary transformation, elimination, convergence/retry, rollback, and coupled-process evidence remain deferred.
- Verification: `cargo test --workspace` passed; `cargo fmt --check` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed; `git diff --check` passed; required evidence headings/phrases assertion passed.
- Scope remained limited to the Phase 2 evidence file, this state report, and the task TODO; implementation source/tests were not modified.

## 4.1.1

- Added filesystem-free validated `Mesh1D` with finite strictly increasing coordinates, zero-based material/layer indices, declared counts, and finite inclination cosine.
- Added explicitly named one-based source adapter with typed rejection for zero, out-of-range, length mismatch, non-finite, non-increasing, and invalid-count inputs.
- Added `mesh_contract` tests covering conversion, optional layers, accessors, and all specified validation failures.
- Verification: `cargo test -p hydrus-core --test mesh_contract` passed; `cargo fmt --check` passed; `cargo clippy -p hydrus-core --all-targets -- -D warnings` passed.
- Scope remained limited to the declared mesh/error/export files, the mesh contract test, and this report; no geometry, coefficient, boundary, solver, or filesystem behavior was added.

## Escalations and amendments

- 4.1.1 scope amended during review to include `crates/hydrus-core/src/lib.rs`: the root re-export is required to expose the new public `Mesh1D` contract; no additional behavior was added.

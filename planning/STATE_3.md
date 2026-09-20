# State — Orchestration 3: Phase 1 hydraulic foundation

Plan status: **approved by the user on 2026-09-19.**
Execution status: **not started.**
Master authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 1.
Resume from [EXECUTION_PLAN_3.md](EXECUTION_PLAN_3.md); worker prompt: [PROMPT_TEMPLATE_3.md](PROMPT_TEMPLATE_3.md).

## Baseline checklist

- [x] Confirm Phase 0 handoff is complete in `STATE_2.md` and `master/PHASE_0_REFERENCE_EVIDENCE.md`.
- [x] Confirm this is the next incomplete master phase: hydraulic foundation.
- [x] Record absent Rust workspace, Rust source/test suite, and project convention file at planning time.
- [x] Record hydraulic source anchors: `MATERIAL.FOR:FK/FC/FQ/FH`; `INPUT.FOR:MatIn/GenMat`; `WATFLOW.FOR:SetMat`.
- [x] Before Wave 1, record clean worktree result with `git status --short` (empty output).
- [x] Before Wave 1, record matching archive/source hashes with `sha256sum planning/H1D_Src.zip src_reference/*.FOR` (recorded in execution log; no changes).
- [x] Before Wave 1, re-run and record Phase 0 provenance smoke command: `python3 tools/run_fortran_reference.py --smoke --timeout-seconds 60` (pass; provenance record completed).
- [x] Before Wave 1, confirm `test ! -e Cargo.toml` (pass).
- [x] Before Wave 1, record green baseline after task 3.1.1 creates the workspace: API contract 4 passed, `cargo fmt --check` passed, and clippy passed.

Scientific baseline: **unavailable.** Phase 0 smoke output is provenance only; Phase 1 source-anchored formula/property tests do not establish fixture parity, a universal tolerance, release compatibility, or performance.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 3.1.1 | Hydraulic-core workspace and typed contract | done | — |
| 2 | 3.2.1 | Analytic `iModel=0` curves and tests | done | — |
| 3 | 3.3.1 | Generated analytic table and tests | pending | — |
| 4 | 3.4.1 | Phase 1 evidence and Phase 2 handoff | pending | — |

## Task reports

<!-- Orchestrator appends concise task reports here. -->

## 3.1.1

- Status: done; hydraulic-core workspace and typed contract implemented.
- Added only the workspace/package, public API, typed errors, hydraulic types/module, and API contract tests.
- `cargo test -p hydrus-core --test api_contract`: passed (4 tests).
- `cargo fmt --check`: passed.
- `cargo clippy -p hydrus-core --all-targets -- -D warnings`: passed.
- `git diff --check`: passed; no source-reference, Fortran, solver, mesh, I/O, or host files changed.
- Formula and generated-table evaluation remain explicitly deferred; selectors do not fall back to model 0.

## 3.2.1

- Status: done; analytic `iModel=0` van Genuchten curves and focused tests implemented.
- Implemented source-anchored FK/FC/FQ/FH behavior with explicit saturated, dry-floor, finite-domain, and inverse-head policy.
- Internal/public calculations remain `f64`; legacy `sngl` locations and source `1.e-37` floors are documented, without parity claims.
- `cargo test -p hydrus-core --test hydraulic_curves`: passed (5 tests).
- `cargo fmt --check`: passed.
- `cargo clippy -p hydrus-core --all-targets -- -D warnings`: passed.
- `git diff --check`: passed; only approved hydraulic source/tests and task todo changed.
- Alternate selectors, generated tables, Fortran comparators, solver, mesh, I/O, and host work remain out of scope.

## Escalations and amendments

<!-- Record retries, rescoping, and model substitutions here. -->

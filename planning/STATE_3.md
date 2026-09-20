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
- [ ] Before Wave 1, record clean worktree result with `git status --short`.
- [ ] Before Wave 1, record matching archive/source hashes with `sha256sum planning/H1D_Src.zip src_reference/*.FOR`.
- [ ] Before Wave 1, re-run and record Phase 0 provenance smoke command: `python3 tools/run_fortran_reference.py --smoke --timeout-seconds 60`.
- [ ] Before Wave 1, confirm `test ! -e Cargo.toml`.
- [ ] Before Wave 1, record green baseline after task 3.1.1 creates the workspace: its targeted API contract, format, and clippy commands.

Scientific baseline: **unavailable.** Phase 0 smoke output is provenance only; Phase 1 source-anchored formula/property tests do not establish fixture parity, a universal tolerance, release compatibility, or performance.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 3.1.1 | Hydraulic-core workspace and typed contract | pending | — |
| 2 | 3.2.1 | Analytic `iModel=0` curves and tests | pending | — |
| 3 | 3.3.1 | Generated analytic table and tests | pending | — |
| 4 | 3.4.1 | Phase 1 evidence and Phase 2 handoff | pending | — |

## Task reports

<!-- Orchestrator appends concise task reports here. -->

## Escalations and amendments

<!-- Record retries, rescoping, and model substitutions here. -->

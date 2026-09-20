# State — Orchestration 2: Phase 0 Fortran reference executable

Plan status: **approved by the user on 2026-09-19.**
Execution status: **not started.**
Master authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 0.

## Baseline checklist

- [x] Confirm immutable source inventory: ten files, 11,053 lines, archive hash recorded in `master/SOURCE_MANIFEST.json`.
- [x] Confirm available tools: GNU Fortran, Python 3, and pdftotext.
- [x] Record unmodified GNU Fortran failure: `MSFLIB` module unavailable and `TIME.FOR:551` requires a logical `IF` condition.
- [x] Confirm absent Cargo workspace, test suite, input deck, executable, and green numerical baseline.
- [x] Record user decisions: GNU Fortran/Linux only; committed patch series/harness; noncanonical smoke input allowed and required to run successfully; generated artifacts untracked.
- [ ] Recheck repository state and source hashes before Wave 1 with `git status --short` and `sha256sum planning/H1D_Src.zip src_reference/*.FOR`.
- [ ] Record the expected-red GNU Fortran baseline with `gfortran -fsyntax-only -std=legacy -ffixed-line-length-none src_reference/*.FOR`; accept only the known MSFLIB and TIME.FOR:551 failures.
- [ ] Create an isolated task worktree and local scratch exclusion before Wave 1.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 2.1.1 | Portability patch series | pending | — |
| 2 | 2.2.1 | Stage/build harness | pending | — |
| 3 | 2.3.1 | Noncanonical smoke run | pending | — |
| 4 | 2.4.1 | Phase 0 evidence handoff | pending | — |

## Task reports

<!-- Orchestrator appends concise task reports here. -->

## Escalations and amendments

<!-- Record retries, rescoping, and model substitutions here. -->

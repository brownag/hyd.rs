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
- [x] Recheck repository state and source hashes before Wave 1 with `git status --short` and `sha256sum planning/H1D_Src.zip src_reference/*.FOR`: worktree clean; archive and all ten source hashes match `master/SOURCE_MANIFEST.json`.
- [x] Record the expected-red GNU Fortran baseline with `gfortran -fsyntax-only -std=legacy -ffixed-line-length-none src_reference/*.FOR`: exited 1 only for unavailable `MSFLIB` at `HYDRUS.FOR:29` and nonlogical `if(iMetHour)` at `TIME.FOR:551`.
- [x] Create an isolated task worktree and local scratch exclusion before Wave 1: `.development/` is locally excluded and worktree `.development/worktrees/2.1.1` is on `docs/master-plan-2.1.1`.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 2.1.1 | Portability patch series | done | committed |
| 2 | 2.2.1 | Stage/build harness | done | committed |
| 3 | 2.3.1 | Noncanonical smoke run | pending | — |
| 4 | 2.4.1 | Phase 0 evidence handoff | pending | — |

## Task reports

<!-- Orchestrator appends concise task reports here. -->

## 2.1.1

- Status: done; portability patch and notes reviewed by the orchestrator.
- Files: `reference/fortran_reference/0001-gfortran-portability.patch`, `PATCH_NOTES.md`, `planning/tasks/2.1.1.todo.md`.
- Patch changes copied `HYDRUS.FOR` signal/MSFLIB use, GNU command-argument compatibility, and `CloseOutput` import; copied `TIME.FOR:551` becomes `iMetHour.eq.1`.
- `iMetHour` proof: `TIME.FOR:502` assigns short-interval `1`, `:504` daily `0`, and `:988` already compares `.eq.1`.
- Orchestrator pass: fresh-copy `git apply --check`, apply, and `gfortran -fsyntax-only -std=legacy -ffixed-line-length-none <copy>/*.FOR` (0 errors).
- Originals remain clean; patch touches only `HYDRUS.FOR` and `TIME.FOR`; `git diff --check` passed.
- Deviation: `GETARG` became standard `GET_COMMAND_ARGUMENT` with default-integer status; documented in notes.
- Gotchas: path backslashes and legacy `NARGS`, `getdat`, and `gettim` remain deferred; no GNU signal-handler replacement is supplied.

## 2.2.1

- Status: done; deterministic stage/build harness reviewed by the orchestrator.
- Files: `tools/stage_fortran_reference.py`, `tools/build_fortran_reference.sh`, `reference/fortran_reference/README.md`, `planning/tasks/2.2.1.todo.md`.
- Stage verifies ten source/archive hashes, archive-member byte equality, and applies the patch only under `.development/reference-build/src_reference`.
- Build uses explicit GNU Fortran flags and writes executable, JSON manifests, and an untracked compatibility shim for retained legacy entry points.
- Orchestrator pass: `python3 tools/stage_fortran_reference.py --output .development/reference-build`; `sh tools/build_fortran_reference.sh --build-dir .development/reference-build`.
- Source/archive hashes were independently rechecked against `master/SOURCE_MANIFEST.json`; `git diff --check` passed and immutable sources remain clean.
- Artifact-notice status remains explicitly unreviewed; build success makes no fixture, parity, or scientific-validation claim.

## Escalations and amendments

<!-- Record retries, rescoping, and model substitutions here. -->

# State — Orchestration 17

Plan status: **Approved by user (“run plan 17”).**
Execution status: **Evidence and decision memo complete; user confirmed pinned-Fortran emulation as the target (2026-10-08).** This orchestration is decision-focused. No implementation is authorized.
Plan: [EXECUTION_PLAN_17.md](EXECUTION_PLAN_17.md). Worker template: [PROMPT_TEMPLATE_17.md](PROMPT_TEMPLATE_17.md).

## Baseline and guardrails

- [x] Read-only reconnaissance completed 2026-10-07; no tests/builds/solvers run.
- [x] Orchestration 16 made no production changes or task commits; its recovered planning/evidence records are preserved in a separate documentation commit.
- [x] Historical baseline only: STATE_15 records 145 Rust, 13 runner/outlet Python, 17 benchmark, 86 convergence tests, formatter, Clippy, and diff check. Not rerun for this plan.
- [x] Independent spec and standards reviews agree that existing evidence shows formulation differences but does not select Rust's target.
- [x] User approves this plan by requesting its execution.
- [x] User confirms the project target is emulation of pinned Fortran; see `planning/master/SOLUTE_FORMULATION_INDEPENDENT_REVIEW_18.md`.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 1 | 17.1.1 | Requirements and verification target report | done |
| 1 | 17.1.2 | Current Rust formulation report | done |
| 1 | 17.1.3 | Pinned Fortran formulation report | done |
| 2 | 17.2.1 | Comparison/options decision memo for user | done |

## Task reports

### 17.1.1 — Requirements and verification target

- **Status:** done; created `planning/master/SOLUTE_FORMULATION_REQUIREMENTS_17.md` and `planning/tasks/17.1.1.todo.md`.
- Distinguished normative continuum requirements, historical implementation/user decisions, open discrete choices, and evidence-class limits; no target selected.
- `git diff --check` passed. No tests/builds run; no deviations.

### 17.1.2 — Current Rust formulation

- **Status:** done; created `planning/master/SOLUTE_RUST_FORMULATION_17.md` and `planning/tasks/17.1.2.todo.md`.
- Mapped the implemented FV/BE scheme and retained four family observations, labeling source facts, measurements, inferences, and unknowns.
- `git diff --check` passed. No tests/builds run; no deviations.

### 17.1.3 — Pinned Fortran formulation

- **Status:** done; created `planning/master/SOLUTE_FORTRAN_FORMULATION_17.md` and `planning/tasks/17.1.3.todo.md`.
- Described source-level assembly and sampled trace comparisons with physical orientation; no Fortran/solver run.
- `git diff --check` passed. No deviations.

### 17.2.1 — Formulation options decision memo

- **Status:** done; created `planning/master/SOLUTE_FORMULATION_DECISION_17.md` and `planning/tasks/17.2.1.todo.md`.
- Memo explains measured and structural differences, presents three options and tradeoffs, and asks the user to choose. Existing evidence sufficed; no diagnostic follow-up or implementation work was needed.
- `git diff --check` passed. No tests/builds run; no deviations.

## Handoff

Orchestration 17's comparison is complete. The user clarified that the target is pinned-Fortran emulation; no further target choice is pending. Before numerical implementation, define the bounded compatibility contract in a separate plan. The independent review is in `planning/master/SOLUTE_FORMULATION_INDEPENDENT_REVIEW_18.md`.

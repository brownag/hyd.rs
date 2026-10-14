---
id: "0001"
title: "Treat the pinned Fortran snapshot as the solute reference"
status: "Accepted"
date: 2026-10-08
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/INITIAL_PROMPT.md; planning/EXECUTION_PLAN_1.md D01; planning/EXECUTION_PLAN_17.md; planning/master/SOLUTE_FORMULATION_DECISION_17.md; planning/EXECUTION_PLAN_18.md; commits 3901211, 10ba967, bce44da"
---

# 1. Treat the pinned Fortran snapshot as the solute reference

## Context and Problem Statement

D01 (2026-09-19) made the supplied, hash-pinned Fortran snapshot the project's numerical oracle. The Rust solute solver was later built on a different formulation: conservative finite volume with backward Euler (FV/BE). Comparisons found large differences, but differences alone don't show a defect or say which method is better.

Orchestration 17 compared the two formulations and put three choices to the user. On 2026-10-08 the user chose pinned-Fortran emulation. This record dates from that choice, which applies D01 to the solute solver.

## Decision Drivers

- Keep the project's goal of matching the pinned source.
- Don't treat a formulation difference as an error or as proof that one method is better.
- Limit parity claims to the audited snapshot, the supported modes, and the observed fields.

## Considered Options

These are the three choices in `planning/master/SOLUTE_FORMULATION_DECISION_17.md`:

1. Keep Rust's FV/BE contract as the target and treat Fortran comparisons as diagnostics.
2. Target pinned Fortran behavior for compatibility.
3. Defer the choice until specific independent or manual evidence exists.

## Decision Outcome

Chosen option: "Target pinned Fortran behavior for compatibility", because the user's stated goal is 1:1 parity with the Fortran features.

The target is the solute subsystem only. [0008](0008-gate-solute-parity-on-audited-active-modes.md) defines which modes count.

Other HYDRUS components supply their state as normalized inputs. This decision doesn't port those components or support whole-program parity claims.

"Exact" means source-faithful behavior with field-specific, precision-aware comparison ([0006](0006-set-parity-tolerances-per-field-from-source-precision.md)). It doesn't mean byte-identical output files. Parity is a separate claim from accuracy or physical validity ([0005](0005-report-each-evidence-class-separately.md)).

### Consequences

- **Positive:** Gives compatibility code, fixtures, and acceptance one stable reference.
- **Negative:** Source operation order, mixed precision, and legacy branches limit implementation choices and add upkeep.
- **Negative:** We can't claim parity until every supported mode and observable has evidence.

## Validation & Compliance

`planning/INITIAL_PROMPT.md` and D01 (`planning/EXECUTION_PLAN_1.md:42`) set the project oracle. `planning/EXECUTION_PLAN_17.md` records the user's choice. `planning/EXECUTION_PLAN_18.md` (approved 2026-10-08, amended through 2026-10-12) records the scope and the meaning of "exact".

`planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` holds the source and observation contract (`3901211`, `10ba967`). The compatibility backend foundation is `bce44da`.

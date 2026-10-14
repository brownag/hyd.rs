---
id: "0007"
title: "Audit Fortran reachability by fixed-form columns and callers"
status: "Accepted"
date: 2026-10-10
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_18.md decisions (2026-10-10); planning/STATE_18.md; planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md; commits 8c7e720, 10ba967, 716a7ea"
---

# 7. Audit Fortran reachability by fixed-form columns and callers

## Context and Problem Statement

The pinned HYDRUS sources are fixed-form Fortran. A `!` in physical column 6 marks a continuation line, not a comment. Earlier reviews read such lines as comments, so mode coverage, fixtures, and task scopes left out active behavior.

The 2026-10-10 audit found two cases. `WATFLOW.FOR`'s `if(iDualPor.gt.0)` continues into an active `call DualPor`, and `SOLUTE.FOR`'s `if(iConcType.eq.2)` continues into an active `call FluxConc`. Parity scope follows reachable caller paths, so we must trace both the syntax and the callers.

## Decision Drivers

- Prevent false exclusions and incomplete parity claims from misread source.
- Base every active, inactive, unselected, unsupported, or untested label on evidence.
- Give every newly found active path an owner for its implementation, fixture, and acceptance.

## Considered Options

1. Find comments and reachability by looking at punctuation and keywords. Waves 1–5 did this, and it produced the errors above.
2. Read physical columns under the pinned compiler's fixed-form rules and trace switches through the active readers and callers.

The sources record no other alternative.

## Decision Outcome

Chosen option: "Read physical columns under the pinned compiler's fixed-form rules", because the compiler's reading is the only one that decides what runs.

We decide comment, continuation, and statement status from fixed-form columns. Column-1 `*` or `C` lines are comments, and so is a real inline `!`. A `!` alone proves nothing.

We trace each full continued statement through reader assignments and callers. Each newly found active behavior gets an implementation, fixture, and acceptance owner before anyone claims all-mode coverage.

A branch that can be called isn't covered until a fixture selects it. Compiling a line proves it is syntax, not that it runs.

### Consequences

- **Positive:** We don't drop real behavior, and we don't implement commented-out code as if it ran.
- **Negative:** Source review and fixture planning take more detail. A new reachability finding can reopen accepted work, as it did for Wave 5.
- **Negative:** We still need runtime discriminators to show a fixture took a branch.

## Validation & Compliance

The Orchestration 18 decision table records the 2026-10-10 correction and the physical-column audit rule. `planning/STATE_18.md` records how the `DualPor` and `FluxConc` paths moved from inactive to active. `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` lists the pinned build flags and source anchors.

Commits `8c7e720` and `10ba967` record parity-source audit fixes. `716a7ea` ports the dynamic dual-porosity exchange that the audit brought into scope.

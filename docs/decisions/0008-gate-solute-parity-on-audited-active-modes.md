---
id: "0008"
title: "Gate solute parity on audited active modes"
status: "Accepted"
date: 2026-10-08
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_18.md decisions (2026-10-08); planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md; planning/STATE_18.md"
---

# 8. Gate solute parity on audited active modes

## Context and Problem Statement

One passing example can't show parity for a source with many selectors, coupled state paths, endpoint variants, species interactions, and helper routines. Some helper code in the source can't be reached from the audited caller. Leaving out active modes, or counting dormant code as tested, would make an "all modes" claim worthless.

Invalid selector combinations also need different handling from valid but untested behavior.

## Decision Drivers

- Match the user's "all solute modes now" scope without turning it into whole-program parity.
- Name the active scope and the unsupported combinations.
- Require discriminating evidence, not routine counts or test counts.
- Keep "can be called" apart from "a fixture ran it".

## Considered Options

1. Do one vertical slice, such as the existing Example 3 case. The user selected all modes "rather than one vertical slice" on 2026-10-08.
2. Audit the active `HYDRUS` → `ChemIn` → `Solute` path, build discriminating fixtures for every reachable supported interaction, and list dormant or unsupported paths.

The sources record no other alternative.

## Decision Outcome

Chosen option: "Audit the active caller path and gate on it", because that is the meaning of "all solute modes" the user approved.

Scope is every active and callable mode and supported interaction reachable through the audited `HYDRUS` → `ChemIn` → `Solute` path. Authenticated fixture evidence must exercise each one, with inputs and mapped observations validated. We list helper code that is present but unreachable and don't count it as runtime parity.

Invalid, unsupported, untested, and active-but-unmapped states stay distinct. An unsupported request fails or stays a recorded blocker. [0007](0007-audit-fortran-reachability-by-fixed-form-columns-and-callers.md) sets how we decide reachability.

A unit test on source alone or a raw-output authentication isn't a paired parity result.

### Consequences

- **Positive:** Each coverage claim traces to selectors, input paths, fixture discriminators, and outputs.
- **Negative:** Missing fixtures, missing dependencies, or source ambiguity can delay the all-mode gate.
- **Negative:** Fixture combinations and observation maps need upkeep whenever the source audit changes.

## Validation & Compliance

The Orchestration 18 decision table (2026-10-08) records the user's choice and the active-path scope. `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` holds the active, inactive, and unsupported matrix and the default gate. `planning/STATE_18.md` tracks run status and blockers.

The plan's final acceptance task requires the full `all` fixture group. Any failing or unmapped active mode blocks the default switch in [0002](0002-keep-fvbe-and-source-compatible-backends-separate.md).

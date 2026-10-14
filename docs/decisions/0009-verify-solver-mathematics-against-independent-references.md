---
id: "0009"
title: "Verify solver mathematics against independent references"
status: "Accepted"
date: 2026-09-26
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_13.md:25-26; planning/master/SOLUTE_VERIFICATION_SPEC.md; commits 490a41b, 0023eb4, 15b21b5, 6285bca, 88b2656"
---

# 9. Verify solver mathematics against independent references

## Context and Problem Statement

Source parity can faithfully reproduce a source defect or a legacy approximation. An implementation can also agree with itself while its equations are wrong. The solute solver needs a check that doesn't depend on either the implementation or the Fortran oracle.

## Decision Drivers

- Catch mathematical errors that source translation and internal conservation checks would miss.
- Keep what the source does apart from what the equations intend.
- Record each reference's provenance, uncertainty, observables, and convergence limits.
- Don't let a passing analytical harness stand in for approved accuracy while error bounds are incomplete.

## Considered Options

1. Start with exact analytical cases that the current APIs support. Orchestration 13 approved this on 2026-09-26.
2. Use manufactured solutions, which need a new forcing hook. Orchestration 13 deferred these rather than approximate them with existing reaction terms.

## Decision Outcome

Chosen option: "Start with exact analytical cases the current APIs support", because they need no new solver API and can't be quietly approximated.

We define references and observables apart from the solver. We authenticate fixture and request provenance and compare on shared space and time points, with units, uncertainty, and error metrics. Expected values never come from solver output, and reference outputs never become model inputs ([0004](0004-use-normalized-request-and-observation-contracts.md)).

Scientific thresholds stay unassigned until independent error budgets and convergence evidence support them. We never widen a limit to turn a diagnostic result into a pass. Conservation closure alone doesn't count as independent verification.

### Consequences

- **Positive:** Can find errors shared by the source and the port, or hidden in the comparison path.
- **Negative:** Deriving references and running convergence studies is costly, and acceptance may stay open.
- **Negative:** A good result at one resolution says nothing general about accuracy.

## Validation & Compliance

`planning/master/SOLUTE_VERIFICATION_SPEC.md` (`490a41b`, 2026-09-26) holds the contract. The configurable harness (`0023eb4`), strict metrics and normalization (`15b21b5`), and analytical references (`6285bca`, `88b2656`) implement it.

`planning/master/SCIENTIFIC_COMPARISON_RESEARCH.md` and `planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md` record current status, with thresholds unassigned and M5-V open.

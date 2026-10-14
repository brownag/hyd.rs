---
id: "0006"
title: "Set parity tolerances per field from source precision"
status: "Accepted"
date: 2026-10-08
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_18.md decisions (2026-10-08); planning/master/SCIENTIFIC_COMPARISON_RESEARCH.md; planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md; planning/master/VALIDATION_STRATEGY.md:44; commit f86a028"
---

# 6. Set parity tolerances per field from source precision

## Context and Problem Statement

The legacy executable mixes precisions, converts between them, and prints rounded output. Repeated baseline runs agree at printed precision, but a promoted-real build changes profiles and balances. One generic tolerance, such as the historical `1e-6` candidate, can't cover fields with different units, scales, print intervals, and zero behavior.

This record covers how parity comparisons are judged. [0010](0010-preserve-source-operation-order-and-precision.md) covers how the compatibility kernels do their arithmetic.

## Decision Drivers

- Respect source arithmetic and print rounding instead of assuming every field is binary64.
- Stop broad tolerances from hiding systematic row or branch errors.
- Check identities and discrete states exactly, apart from floating-point criteria.
- Treat a field without reviewed criteria as a blocker.

## Considered Options

1. Use the prompt's `1e-6` as one universal threshold. `planning/master/VALIDATION_STRATEGY.md:44` calls it "a calibration candidate, not a universal or achieved threshold".
2. Require byte-for-byte text identity. The user rejected this on 2026-10-08.
3. Use field-specific tolerances: exact where the representation permits, calibrated limits otherwise. The user selected this on 2026-10-08.

## Decision Outcome

Chosen option: "Use field-specific tolerances", because each field's precision and meaning differ.

Each criterion states its units, scale or normalization, absolute and relative parts, and zero handling. Rounded outputs use the source's actual print interval, and criteria account for the pinned build's precision and repeatability. Until we review a field's criteria, its tolerance stays unassigned and its acceptance stays blocked.

We never widen a threshold to make a failure pass.

### Consequences

- **Positive:** Tolerances match each field and are less likely to hide structural errors.
- **Positive:** Numeric tolerance can't mask a mismatch in flags, identities, schedules, or availability.
- **Negative:** Every observable needs calibration and review, so full parity takes longer.
- **Negative:** Some fields may stay unaccepted until repeatability or error-budget evidence exists.

## Validation & Compliance

The Orchestration 18 decision table (2026-10-08) records the user's choice and the meaning of "exact". `planning/master/SCIENTIFIC_COMPARISON_RESEARCH.md` and the precision review (`f86a028`) record baseline print repeatability, sensitivity to promoted precision, and no frozen universal threshold.

`planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` defines the paired comparison and print-interval policy.

---
id: "0010"
title: "Preserve source operation order and precision"
status: "Accepted"
date: 2026-10-08
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md; planning/EXECUTION_PLAN_18.md; commits 3901211, fda4f38, 6394d52, 49192d7"
---

# 10. Preserve source operation order and precision

## Context and Problem Statement

The source's solute code mixes default `REAL`, `DOUBLE PRECISION`, and conversions such as `sngl`. Regrouping a formula, even an algebraically equal one, or converting everything to `f64` changes intermediate rounding and the final values. The compatibility backend must reproduce source behavior, and any modernization is a separate choice.

This record covers the kernels' arithmetic. [0006](0006-set-parity-tolerances-per-field-from-source-precision.md) covers how we judge the results.

## Decision Drivers

- Make parity sensitive to the source's real arithmetic and conversions.
- Don't "clean up" formulas in ways that change results.
- Keep modernization out of compatibility code.

## Considered Options

1. Preserve source expression grouping, call order, and precision conversions in the compatibility backend.
2. Change the pinned source or build flags to make parity easier. Orchestration 18 puts changes to `src_reference/` and the portability patches out of scope.
3. Change the scientific meaning of Fortran behavior to improve continuum accuracy. Orchestration 18 also puts this out of scope.

## Decision Outcome

Chosen option: "Preserve source expression grouping, call order, and precision conversions", because parity depends on the source's rounding as well as its formulas.

We follow the pinned build configuration and the source's declarations and conversions. We keep expression order where it affects rounding, and we keep the state-update and call sequence. We test branch-specific formulas against expected values derived independently from the source.

We don't silently promote precision. A different precision or formulation can exist only as a separate backend with its own contract, as [0002](0002-keep-fvbe-and-source-compatible-backends-separate.md) allows.

### Consequences

- **Positive:** Cuts parity drift from regrouping, casts, or reordered state updates.
- **Negative:** Source-order code is less idiomatic and harder to refactor or optimize.
- **Negative:** Compiler and platform behavior stay part of the evidence and may need later portability studies.

## Validation & Compliance

`planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` (`3901211`) records the mixed-precision baseline and known provenance gaps. We record precision at the actual conversion points and the pinned build, not from a few declared arrays. The Orchestration 18 shared rules and kernel tasks require order preservation and fixed-form source review.

The source-compatible transport (`fda4f38`) and endpoint rows (`49192d7`) follow this rule. `6394d52` keeps a literal `BanSol` expression that Clippy would otherwise rewrite.

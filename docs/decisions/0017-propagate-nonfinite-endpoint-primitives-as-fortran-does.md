---
id: "0017"
title: "Propagate non-finite endpoint primitives as the pinned Fortran does"
status: "Accepted"
date: 2026-10-19
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_20.md task 20.6.1 (round 2, design change); refines ADR 0014"
---

# 17. Propagate non-finite endpoint primitives as the pinned Fortran does

## Context and Problem Statement

The compatibility kernels reject non-finite endpoint primitives (`endpoint primitives must be finite`). ADR 0014 places shape checks in each kernel's `validate`. A finiteness check is a value check, and it went beyond shape. In the 44-fixture parity run, `biology/blocking_type_3_noop` and `biology/filtration_deposition` produce NaN in the pinned Fortran reference. The source gives `psi1 = NaN` when `iPsi=3` and `SMax=0` (`SOLUTE.FOR:1561-1594`). The reference outputs contain NaN, and the parity comparison accepts NaN against NaN.

## Decision Drivers

- Match the pinned Fortran output bit-for-bit where it is defined, including NaN.
- Keep shape checks in the kernel, as ADR 0014 requires.
- Do not reject a request the pinned Fortran runs.

## Considered Options

1. Keep rejecting non-finite primitives. The two fixtures cannot run, and the 44/44 gate fails.
2. Remove the finiteness checks. Shape and range checks stay. IEEE non-finite values propagate, as in the Fortran.
3. Replace NaN with a sentinel or zero in the kernel. This changes the output the pinned Fortran writes.

## Decision Outcome

Chosen option: 2. The endpoint kernel validates shape, index bounds, and the documented ranges (`epsi`, `alf`, `dt`, `rMin`). It does not validate that primitive values are finite. Non-finite values flow through the same arithmetic the pinned source uses.

Coordinates stay validated as finite and increasing. They are geometry, not solver state.

### Consequences

- **Positive:** Requests that the pinned Fortran runs now run. The NaN-producing biology fixtures match the reference.
- **Negative:** A NaN from a bad input is not caught at the kernel boundary. The output carries it, as the Fortran does.
- **Scope:** The step-size behavior under NaN follows from the first-step water solve (`compat_runner.rs`), not from a separate NaN rule.

## Validation & Compliance

The regression tests `endpoint_nonfinite_primitives_propagate_like_fortran` (in `boundary.rs`) and the two biology fixture runs in the 20.6.1 parity gate cover this decision.

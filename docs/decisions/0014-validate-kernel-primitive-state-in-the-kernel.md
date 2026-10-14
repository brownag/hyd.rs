---
id: "0014"
title: "Validate kernel primitive state in the kernel"
status: "Accepted"
date: 2026-10-14
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_19.md decision D6; task 19.1.1"
---

# 14. Validate kernel primitive state in the kernel

## Context and Problem Statement

The compatibility request carries primitive state for several kernels, such as `endpoint_primitives` and `dual_porosity_exchange_state`. Per-state shape checks risk living in both the adapter and the kernels, and two copies of a check drift apart.

## Decision Drivers

- Keep one owner for each per-state shape check.
- Keep the request schema of [0004](0004-use-normalized-request-and-observation-contracts.md) stable.
- Reject bad state before any mutation.

## Considered Options

1. Keep per-state checks in the adapter and again in each kernel.
2. Keep `endpoint_primitives` and `dual_porosity_exchange_state` as request fields, put the per-state shape checks only in each kernel's `validate`, and have the adapter call them. Checks that span fields stay in the adapter.
3. Remove the primitive-state fields from the request. This changes the schema 0004 fixes.

## Decision Outcome

Chosen option: "Per-state shape checks live only in the kernel's `validate`", because the kernel owns the meaning of its state.

The adapter calls the kernel validators at parse time, before any mutation (task 19.1.1). Cross-field checks that the kernel can't see, such as `validate_cross_field_dimensions` in the adapter, stay in the adapter.

The request field set and JSON shape are unchanged (0004). The adapter may now reject, before mutation, requests the kernel would reject at run time anyway, for example out-of-range boundary codes or invalid `ParD` columns. No request that could have run successfully is newly rejected.

### Consequences

- **Positive:** One check per rule. Errors surface before mutation.
- **Negative:** Some bad requests now fail at parse time and not at run time, so their error text can change.

## Validation & Compliance

`planning/EXECUTION_PLAN_19.md` records the decision as D6 and assigns it to task 19.1.1. The kernel validators are `validate` in `boundary.rs` and `dual_domain.rs`, and `validate_uptake` and `validate_exclusion` in `source_coupling.rs`, all under `crates/hydrus-core/src/solute_fortran_compat/`.

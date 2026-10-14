---
id: "0015"
title: "Keep precision-faithful hydraulic ports in compat kernels"
status: "Accepted"
date: 2026-10-14
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_19.md task 19.1.3; crates/hydrus-core/src/solute_fortran_compat/dual_domain.rs"
---

# 15. Keep precision-faithful hydraulic ports in compat kernels

## Context and Problem Statement

`retention_head` and `conductivity` in `solute_fortran_compat/dual_domain.rs` port `FH` and `FK` of `MATERIAL.FOR` for `iModel=0`. The crate also has `hydraulics/van_genuchten.rs`, which evaluates the same van Genuchten model. The two look like duplicates, and a cleanup could merge them.

## Decision Drivers

- Follow [0010](0010-preserve-source-operation-order-and-precision.md): keep source operation order and precision.
- Keep the compatibility kernels free of modernization.
- Avoid a merge that changes either module's results.

## Considered Options

1. Make the dual-domain kernel call `hydraulics::van_genuchten`.
2. Keep both, and keep the compatibility ports at source precision.

## Decision Outcome

Chosen option: "Keep both, and keep the compatibility ports at source precision", because the two modules follow different precision rules.

`retention_head` and `conductivity` compute in `f64`, as `implicit double precision (A-H,O-Z)` does in `MATERIAL.FOR`, take default-REAL inputs, and narrow the result to `f32` as `sngl` does (`MATERIAL.FOR:369` for `FH`). `hydraulics/van_genuchten.rs` keeps `f64` throughout and notes that it doesn't apply `sngl` at the `FK`, `FC`, `FQ` and `FH` returns.

We don't unify the two. The duplication is on purpose.

### Consequences

- **Positive:** Compatibility results stay bitwise stable.
- **Negative:** Two van Genuchten implementations exist. A fix to one must be checked against the other.

## Validation & Compliance

Unit tests in `dual_domain.rs` (`retention_head_matches_the_fh_zero_model_path`, `conductivity_matches_the_fk_zero_model_path`) pin the source behavior. The decision follows from 0010.

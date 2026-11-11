---
id: "0019"
title: "Validate scientific accuracy on the default backend"
status: "Accepted; M5-V accepted with limitations"
date: 2026-10-24
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_22.md D2, D3, D8, D10 and task 22.8.1; planning/master/SOLUTE_CONVERGENCE_EVIDENCE_22.md; planning/EXECUTION_PLAN_23.md task 23R.3; planning/master/M5V_FINAL_23.md"
---

# 19. Validate scientific accuracy on the default backend

## Context and Problem Statement

M5-V (the scientific-comparison gate for solute transport) stayed open after Orchestration 13's FV/BE convergence matrix (O13) stopped at its storage ceiling ([SOLUTE_CONVERGENCE_EVIDENCE.md](../../planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md)). Since then [0018](0018-default-to-the-fortran-compatible-solute-backend.md) made `FortranCompatible` the default backend, after the `21.4.1` gate matched the pinned Fortran on 44 of 44 fixtures. Orchestration 22 decided:

- **D2.** Re-scope M5-V. Source parity is met by the 44/44 gate. Physical and analytical validation runs mesh and time refinement of the default `FortranCompatible` backend against analytical references, Rust only.
- **D3.** Cover four analytical families (closed reactions, `diffusion_cosine`, `scalar_robin`, the Example 3 chain). FV/BE is not re-studied.
- **D8.** Produce error budgets first, then a user threshold gate, then holdout rows unseen during threshold selection. M5-V closes only if the holdouts pass.

## Decision Drivers

- The evidence should describe the backend users get by default.
- The pinned Fortran stays the solute reference (0001); parity must tie the evidence to it.
- Thresholds come from observed error against an independent reference, never from solver output or after-the-fact widening (0004, 0009).

## Considered Options

1. **Re-scope (chosen).** Validate accuracy on the default backend; rely on the 44/44 parity to carry the result to the pinned Fortran.
2. **Split the gate.** Keep a source-parity gate and a separate physical-accuracy gate, each with its own owner and closure. This was not chosen because parity already closed and the split adds no new evidence.
3. **Resume the O13 matrix.** Finish the FV/BE convergence rows. This was not chosen because it validates a backend that is no longer the default, and the matrix hit its storage ceiling.

## Decision Outcome

Chosen option: 1. Physical and analytical validation runs on the default `FortranCompatible` backend, and source parity (`21.4.1`) ties that evidence to the pinned Fortran. We don't change FV/BE equations (0002).

Partial outcome (D10, 2026-10-24): M5-V is partial / open: study-level thresholds (`reference/solute_verification/THRESHOLDS_V2.json`, `cases_sha256` `c236eb7c…`, `holdout_run_permitted: false`) are approved under D10, holdouts were not run (no holdout-level thresholds assigned; holdout mesh/dt not isolated), and M6 planning is not unblocked by this record.

Orchestration 23 outcome (2026-11-10): the final campaign ran six verified batches with a signed combined bounds source. It produced 12 `achieved` diffusion-space concentration fields and 40 `failed` Robin-space fields (non-monotone between the fine pair); every time group and every Example 3 group is inconclusive. The time groups have only two study levels because the design's centre rows are space-axis only, so they report `fewer_than_three_levels` (decision D14 accepts the partial outcome). No holdout-level threshold was defensible, Gate C was not requested, and no holdout row ran. M5-V remains open and M6 stays blocked.

### Dated policy clarification — 2026-11-11

The preceding Orchestration 23 statement records the status at campaign close.
By explicit user decision, [M5V_M6_HANDOFF_DECISION.md](../../planning/master/M5V_M6_HANDOFF_DECISION.md)
accepts the M5-V results as they stand, with limitations, and unblocks M6
planning and implementation. This supersedes the holdout-dependent M5-V closure
and unconditional M6-blocking consequences in this ADR. The campaign's original
failed/inconclusive classifications and caveats remain unchanged; no holdout or
new threshold is authorized. M6 acceptance remains separate, and this decision
does not claim universal solute accuracy or M6 acceptance.

### Consequences

- **Positive:** The accuracy evidence now covers the default backend: closed reactions (time order about 1), diffusion and Robin space error, and Example 3 NH4/NO2 norms, with study-level ceilings approved.
- **Negative:** The campaign did not resolve every field: diffusion inventory (known kernel end-row mass-loss defect candidate, no SD registered), Robin `l1`/`l2`/inventory/balance, all temporal error, and Example 3 inventory, balance, node-1 closure, NO3, outlet tail and reaction rates remain caveats to the accepted result.
- **Negative:** The classifier has no bounds source, so every finding is `inconclusive`. The bridge gate was waived for v2 and the v2 bridge row failed. The FV/BE study was not re-run, manufactured forcing is still blocked, and claims hold for one compiler and platform.
- **Negative (23.x):** The final campaign found 40 failed Robin-space fields and could not classify any time or Example 3 field. A future time study needs three time-axis levels or a tested dual-axis centre-reuse rule.
- **Historical follow-up:** Under the original closure policy, closing M5-V would have required holdout-level mesh/dt isolation, thresholds for unassigned fields, a bounds source, and a user decision before holdouts. The 2026-11-11 decision accepts the existing result with limitations; this further campaign work is not required for M6 progression.

## Validation

- [SOLUTE_CONVERGENCE_EVIDENCE_22.md](../../planning/master/SOLUTE_CONVERGENCE_EVIDENCE_22.md): error budgets, observed orders, proposed thresholds, M5-V claims.
- Study reports: [reactions](../../planning/master/M5V_STUDY_22_REACTIONS.md), [diffusion](../../planning/master/M5V_STUDY_22_DIFFUSION.md), [Robin](../../planning/master/M5V_STUDY_22_ROBIN.md), [Example 3](../../planning/master/M5V_STUDY_22_EXAMPLE3.md), [Example 3 space](../../planning/master/M5V_STUDY_22_EXAMPLE3_SPACE.md).
- [SOLUTE_FORTRAN_PARITY_18.md](../../planning/master/SOLUTE_FORTRAN_PARITY_18.md) section 21.4.1: 44/44 source parity.
- [THRESHOLDS_V2.json](../../reference/solute_verification/THRESHOLDS_V2.json): approved study-level ceilings.
- Closure record: [PHASE_5_SOLUTE_EVIDENCE.md](../../planning/master/PHASE_5_SOLUTE_EVIDENCE.md#m5-v-closure-22x--partial--open).

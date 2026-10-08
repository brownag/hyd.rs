# Solute formulation requirements and verification target

Task 17.1.1 records the requirements and evidence standards relevant to choosing a Rust solute formulation. It does not select a discrete scheme or implementation target.

## Classification

- **Normative** means specified by the mathematical verification contract, not necessarily mandated as Rust’s discrete implementation.
- **Historical** means a prior M5 contract or user decision; it describes the selected scope and evidence history, not an automatic target for the present formulation choice.
- **Unresolved** means the cited evidence does not establish a unique choice or claim.

## Specified model and boundary requirements

### Normative continuum equations

The verification contract specifies a one-dimensional coordinate `s` increasing from inlet `0` to outlet `L`, nonnegative flow `q`, constant water content `theta`, and linear equilibrium storage. For species `j`, stored mass density is `m_j = a_j C_j`, where `a_j = theta + rho Kd_j` and `R_j = a_j/theta`. The flux and transport/reaction equation are:

```text
J_j = q C_j - theta D dC_j/ds
a_j dC_j/dt = theta D d²C_j/ds² - q dC_j/ds
               - lambda_j a_j C_j + lambda_(j-1) a_(j-1) C_(j-1)
```

The parent term is absent for the first species. The chain yield is one; terminal decay removes mass from the configured chain. Consequently, the daughter concentration-production coefficient is storage-weighted, `lambda_(j-1) a_(j-1)/a_j`. These are continuum/model requirements, not instructions for a particular matrix, time integrator, or spatial stencil. ( `SOLUTE_VERIFICATION_SPEC.md`, “Equation, dimensions and signs,” lines 58–81.)

The common endpoint laws are inward inlet flux `J_j(0,t) = q c_in,j` and zero-gradient outlet `C_j,s(L,t) = 0`, which implies `J_j(L,t) = q C_j(L,t)`. Signed inward endpoint fluxes are `[q c_in,j, -q C_j(L,t)]`; for `q=0`, the conditions become zero flux. The contract also specifies nonnegative initial data and calls out the incompatible initial/inlet corner for the Example 3 step input; the right-time solution is evaluated for `t>0`, without claiming smoothness at the corner. (“Equation, dimensions and signs,” lines 83–90.)

For closed, spatially uniform reaction benchmarks (`q=D=0`), stored species masses follow the ordered first-order chain `m'=Bm`, with parent transfer and terminal loss as specified. The finite diffusion benchmark is a finite-domain Neumann cosine solution with zero flow and loss. These cases define independent mathematical checks for their stated configurations; they do not prescribe a general transport discretization. (“Closed reaction benchmarks,” lines 105–131; “Finite diffusion benchmark,” lines 133–144.)

### Normative evidence-contract details

The verification contract defines independent continuum quantities for dissolved/sorbed inventory, parent production, species loss, and signed boundary cumulative terms; production and loss are to be integrated separately rather than inferred by closing a balance residual. It defines the finite-domain scalar inlet/outlet benchmark and a finite Example 3 chain, including their parameters and analytical/reference evaluation requirements. (“Equation, dimensions and signs,” lines 92–103; “Scalar finite-domain inlet/outlet benchmark,” lines 146–167; “Actual finite Example 3 chain and a stable Laplace evaluation,” lines 169–220.)

For spatial and temporal verification, the contract specifies norms, coordinate/time sampling, refinement controls, uncertainty treatment, and cautions that observed order requires resolved decreasing errors with other-axis error controlled. It notes backward Euler is first order in time and upwind advection at most first order in space for the described Rust scheme; these are verification expectations, not a target selection. (“Norms, refinement and budgets,” lines 353–381.)

## Open discrete choices

**Unresolved:** The verification spec defines continuum equations and benchmark cases, but does not mandate one discrete Rust scheme. In particular, it does not choose among spatial row/stencil and endpoint-row constructions, time weighting/integration schemes, or compatibility with the pinned executable. The contract’s source mapping describes a particular verification setup and explicitly notes differing endpoint discretizations; it is not a decision that Rust must adopt either endpoint treatment. (`SOLUTE_VERIFICATION_SPEC.md`, “Executable source mapping,” lines 301–351; “Acceptance and honest gaps,” lines 584–594.)

**Unresolved:** No single evidence class presently closes all validation questions. The contract says no solver acceptance threshold is assigned, and reference-budget failure blocks dependent accuracy claims. (`SOLUTE_VERIFICATION_SPEC.md`, “Norms, refinement and budgets,” lines 365–381; “Acceptance and honest gaps,” lines 584–594.)

## What the evidence classes establish

| Evidence class | Can establish | Cannot establish by itself |
|---|---|---|
| **Pinned-source parity** | Agreement with the pinned executable for the mapped case, observables, coordinates/times, and stated precision/provenance. The Phase 5 record documents one derived Example 3 case, repeatability, and diagnostic paired observations. | That the pinned source is physically correct, that the continuum solution is accurate, that untested cases/branches agree, or that calibrated acceptance thresholds have been met. The current comparator is diagnostic and thresholds remain unassigned. |
| **Independent mathematical verification** | Agreement with independently derived analytical/reference solutions and observables for covered equations/cases; with adequate uncertainty and refinement evidence, it can assess numerical error for that coverage. | Physical validation, correctness outside the verified model/cases, or accuracy claims beyond the reference uncertainty and demonstrated refinement. A contract checker or invariant test alone is not an analytical evaluator. |
| **Physical validation** | Agreement with appropriately sourced and measured physical observations for a defined intended use, including stated uncertainty and validation domain. | It is not established by source parity or analytical solution agreement alone; the cited artifacts do not provide calibrated physical acceptance evidence. |

Anchors: `PHASE_5_SOLUTE_EVIDENCE.md`, “Purpose and evidence standard” (lines 3–7), “M5 exit assessment” (lines 56–68), and “Derived comparison and M5-V handoff” (lines 91–104); `SOLUTE_VERIFICATION_SPEC.md`, “Acceptance and honest gaps” (lines 584–594); `VALIDATION_STRATEGY.md`, “M5 solute scientific-comparison gate” and “Tolerance policy” (lines 28–48).

## Prior decisions and historical contract

- **Historical — bounded in-memory contract cases, not invented manual outputs.** Orchestration 10 D03 selected deterministic in-memory fixtures and explicitly rejected invented manual expected outputs. (`EXECUTION_PLAN_10.md`, Decisions, D03, lines 34–42; `tasks/10.3.1.todo.md`, Notes, lines 17–22.)
- **Historical — no unsupported claims.** D07 bars claims of source-release identity, physical calibration, Fortran parity, universal tolerances, or performance improvement without required evidence. (`EXECUTION_PLAN_10.md`, D07, lines 34–42.)
- **Historical — selected M5 API contract.** Phase 5 records a single-water-domain API, caller-declared compatible units, ordered first-order species chain, supported selected sorption/boundary features, and backward-Euler/conservative finite-volume implementation behavior. It also states that this contract and its invariant tests do not establish fixture parity or scientific tolerances. (`PHASE_5_SOLUTE_EVIDENCE.md`, “Delivered M5 contract,” lines 9–36.) This is evidence of what was implemented, not a decision to retain that formulation as the future target.
- **Historical — user’s current decision workflow.** Orchestration 17 records the user’s request to explain Rust’s deviation from pinned Fortran, compare requirements and both formulations, and present options for the user’s choice before code changes. It also records authorization for needed follow-ups without arbitrary retry/time/artifact caps. (`EXECUTION_PLAN_17.md`, “Verbatim constraints carried forward” and “Decisions recorded,” lines 14–33.)

## Unresolved evidence gaps

- Canonical manual decks and expected outputs are absent; the derived Example 3 fixture is not a recovered manual deck. (`PHASE_5_SOLUTE_EVIDENCE.md`, lines 5–7 and 91–104.)
- The available source comparison is diagnostic: no justified field-specific parity tolerances, balance threshold, or near-zero cutoff has been assigned. (`PHASE_5_SOLUTE_EVIDENCE.md`, lines 93–104.)
- Paired mesh/time convergence and a reviewed independent analytical solution for the finite-domain boundary problem remain absent in the Phase 5 handoff; physical validation is also not established. (`PHASE_5_SOLUTE_EVIDENCE.md`, lines 65–74 and 95–104; `VALIDATION_STRATEGY.md`, lines 28–32.)
- Thus existing requirements and evidence do not uniquely select Rust’s discrete target. This report makes no selection; the formulation decision remains with the user after the comparison memo.

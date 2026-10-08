# Solute formulation choices for Rust

## Purpose and evidence labels

This memo explains the retained evidence for why the current Rust solute solver differs from the pinned Fortran implementation and presents formulation choices for the user. It does not select a target or recommend a numerical change.

- **Source/history fact:** directly recorded in implementation source, pinned source, or task/commit history.
- **Measured fact:** recorded in a retained Orchestration 16 trace or normalized report; limited to its case, rows, and precision.
- **Inference:** an explanation consistent with the source and measurements, but not uniquely isolated by them.
- **Unknown:** not established by the retained evidence.

## Why the implementations differ

**Source/history facts — distinct contracts and choices.** Rust’s solute step was introduced under task 10.3.1 as a conservative, single-water-domain, in-memory solve using caller-supplied water state and face fluxes. Its current implementation uses nodal finite-volume control volumes, conservative face fluxes with upwinded advection and dispersion, and implicit backward-Euler time treatment. It supports selected endpoint modes, ordered first-order species reactions, equilibrium sorption, and one kinetic pool. The implementation history records subsequent extensions for reactions/sorption, balance reporting, changing water content, per-species boundaries, and advective outflow (`SOLUTE_RUST_FORMULATION_17.md`, “Implemented formulation” and “Implementation history”). This history documents what Rust implements; it does not establish a decision that Rust must preserve that scheme or match Fortran.

The pinned source describes its method as mass-lumping finite elements. Its `MatSet` builds element-based tridiagonal rows, runs two assembly levels, uses `epsi`/`alf` temporal-spatial weighting, and branches on boundary selections. Reaction and transport contributions depend on species and switches (`SOLUTE_FORTRAN_FORMULATION_17.md`, “Assembly and temporal treatment,” “Transport, species, and reaction contributions,” and “Boundary closures”).

**Measured facts — retained cases.** The four normalized reports identify differing rows at matched physical locations:

| Family | Retained observation | Evidence limit |
|---|---|---|
| Diffusion | All 63 interior Fortran rows differ from Rust by `(+1.37599995327, -2.75199926758, +1.37599995327)` in physical-order lower/diagonal/upper coefficients. The outlet rows also differ; the first-step maximum state difference is `0.0003047828`, at the outlet. | One 65-node case and first step; the traces establish operator divergence, not a required target or defect. (`SOLUTE_DIFFUSION_NORMALIZED_TRACE.md`) |
| Scalar Robin | Sampled inlet, midpoint, and outlet rows differ. At the midpoint Fortran is `(-13.86759472, 30.10119629, -13.65259457; 1.96497154)` versus Rust `(-27.95, 58.05201562, -27.52; 1.96529857)`. The Fortran outlet is a two-node zero-gradient row; Rust retains a half-cell balance. | Selected rows only; no weighting-only or boundary-only counterfactual isolates a cause. (`SOLUTE_ROBIN_NORMALIZED_ROWS.md`) |
| Reaction | In the active case, the retained Rust uniform S1 value is `0.9937888199` and Fortran prints `0.993769467`; the inactive zero-reaction control agrees for the tested nodes/species. The sampled Fortran row has a nonzero neighbor term while Rust’s reaction-only row is diagonal. | The full Fortran element-to-node mass/reaction ledger and mapped phase-specific parameters are missing. Printed values are rounded. (`SOLUTE_REACTION_NORMALIZED_EQUATIONS.md`) |
| Example 3 | At sampled inlet, midpoint, and outlet locations for three species, assembled rows differ; Fortran reports `epsi=0.5` in this run, and the outlet closure differs. | Derived fixture, sampled first interval—not a recovered manual deck, whole-domain comparison, or independent analytical oracle. (`SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md`) |

The comparison reports use physical inlet-to-outlet orientation. In the Fortran tridiagonal row, reversal maps `(B,D,E;F)` to physical `(E,D,B;F)`. Coefficients from FE rows and FV rows share this display convention but not necessarily the same support or quadrature.

**Inference — structural explanation, not a unique causal decomposition.** The clearest recorded causes of structural divergence are the documented FV versus FE spatial assembly, Rust’s implicit endpoint-time rows versus Fortran’s weighted two-level assembly in the measured setups, and the different endpoint closures (Rust half-cell balance versus measured Fortran two-node zero-gradient outlet). Upstream weighting and reaction/storage distribution can also contribute in the sampled comparisons. These are formulation differences; the available comparisons entangle effects and do not assign every observed state/profile difference to one term.

**Unknowns.** The retained evidence does not establish a unique Rust defect, whether Fortran behavior is the required Rust target, or the contribution of each term across all configurations. Fortran precision is not uniformly established: selected matrix arrays are declared double precision, while other calculations follow defaults/build flags; some retained printed values are rounded single precision. No field-specific parity tolerances are calibrated. No parity result establishes physical correctness or continuum accuracy.

## Choices

### 1. Retain Rust’s declared FV/BE contract and assess it independently

- **Meaning:** Keep the current discrete formulation as the Rust target; treat pinned Fortran comparisons as compatibility diagnostics rather than a numerical specification.
- **Compatibility consequence:** Rust is not expected to reproduce the measured Fortran rows/profiles where the formulations differ. Existing Rust behavior remains the baseline unless a separate, scoped change is approved.
- **Evidence needed:** Independent analytical solutions for covered cases, spatial/time refinement with controlled error contributions, independently checked inventories/boundary terms, and reviewed field-specific acceptance criteria. Physical claims additionally require appropriately sourced observations and uncertainty.
- **Likely future-plan scope:** Specify and execute an independent verification program for the selected Rust contract, including analytical/refinement fixtures and calibrated observables. Keep source parity as a separate claim.

### 2. Target pinned Fortran behavior for compatibility

- **Meaning:** Make agreement with the pinned source’s mapped behavior an explicit Rust compatibility objective for defined cases and configurations.
- **Compatibility consequence:** Current Rust rows differ in retained cases, so numerical changes and possible changes to endpoint/time/spatial behavior may be needed. Compatibility would be limited to the pinned source, mapped switches/inputs, observables, and declared precision; it would not imply physical or continuum correctness.
- **Evidence needed:** Define the exact source snapshot/build semantics and supported configuration; recover or construct reviewed inputs; map source switches and outputs; specify observable-level and precision-aware parity criteria; expand comparisons beyond sampled rows; keep independent mathematical/physical validation separate.
- **Likely future-plan scope:** First specify the compatibility contract and fixture/tolerance gate, then plan incremental formulation changes and paired parity validation. Do not infer implementation tasks from this decision memo.

### 3. Defer target selection pending specific independent/manual evidence

- **Meaning:** Leave the numerical target open until additional evidence or domain-owner input resolves the intended use and acceptable evidence basis.
- **Compatibility consequence:** Neither future parity with Fortran nor preservation of the current Rust discretization is promised as the selected target; current implementation behavior remains unchanged in the meantime.
- **Evidence needed:** Name the evidence that would resolve the decision—for example, recovered/manual case provenance and expected quantities, independent analytical/refinement evidence, physical validation data, or an explicit compatibility requirement—and identify its owner and acceptance criteria.
- **Likely future-plan scope:** A bounded evidence/requirements plan with explicit decision criteria and stopping conditions, followed by a separate formulation plan only after the target is selected.

## Evidence boundary and decision

`VALIDATION_STRATEGY.md` distinguishes source-snapshot parity, independent mathematical verification, and physical validation. The current comparison is diagnostic; tolerances remain unassigned, and required analytical/refinement evidence is incomplete. Historical M5 implementation choices and task 10.3.1 explain Rust’s current contract but do not decide today’s target (`SOLUTE_FORMULATION_REQUIREMENTS_17.md`; `STATE_16.md`).

No retained-evidence gap requires a new diagnostic to present these choices: further row counterfactuals could allocate causes more narrowly, but they would not decide the intended target or establish scientific accuracy. No diagnostic was run. No implementation recommendation is selected here.

**Which target do you want for Rust’s solute formulation: retain the declared FV/BE contract and assess it independently, target pinned Fortran behavior for compatibility, or defer the decision pending specified independent/manual evidence?**

## Evidence reviewed

- `planning/master/SOLUTE_FORMULATION_REQUIREMENTS_17.md`
- `planning/master/SOLUTE_RUST_FORMULATION_17.md`
- `planning/master/SOLUTE_FORTRAN_FORMULATION_17.md`
- `planning/STATE_16.md`
- `planning/master/SOLUTE_DIFFUSION_NORMALIZED_TRACE.md`
- `planning/master/SOLUTE_ROBIN_NORMALIZED_ROWS.md`
- `planning/master/SOLUTE_REACTION_NORMALIZED_EQUATIONS.md`
- `planning/master/SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md`
- `planning/master/VALIDATION_STRATEGY.md`

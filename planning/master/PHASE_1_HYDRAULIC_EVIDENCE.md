# Phase 1 hydraulic evidence

Phase 1 records the first filesystem-free Rust hydraulic path. It is contract and source-derived evidence, not a calibrated reference comparison.

## Hydraulic contract

`hydrus-core` exposes validated `VanGenuchtenParameters`, `VanGenuchtenModel`, `HydraulicModel`, `Evaluation`, `HydraulicDerivatives`, `HydraulicTables`, and typed `HydraulicError` values. The implemented selector is only `iModel=0` (van Genuchten). The model provides theta, conductivity, capacity, combined evaluation, derivatives, and inverse-head evaluation. Saturated heads use the explicit saturated limit; finite negative heads use the analytic model; non-finite evaluation inputs fail as typed domain errors.

The component is filesystem-free and has no solver, mesh, boundary, timestep, root, atmospheric, I/O, host, fixture, or benchmark behavior. Parameter validation rejects non-finite values, invalid water-content ordering, non-positive alpha, `n <= 1`, and negative saturated conductivity.

## Source anchors and precision

The source anchors are `src_reference/MATERIAL.FOR:FK`, `FC`, `FQ`, and `FH`, with the model-0 branches and `m=1.d0-1.d0/n`. The Rust implementation follows those formula contracts for theta, conductivity, capacity, and inverse head. `INPUT.FOR:GenMat` supplies the generated-table shape (`alog10(-h)`); `WATFLOW.FOR:SetMat` supplies the analytic-table interval interpolation branch and its `dh` calculation.

The Rust implementation intentionally calculates publicly and internally in `f64`. The legacy routines return through `sngl` and use source floors such as `1.e-37`; the Rust code documents and retains the relevant floor semantics but does not reproduce the legacy conversion points. No mixed-precision compatibility mode has been calibrated, and no Fortran parity result, release-compatibility result, universal tolerance, or fixture result is claimed. The test tolerances are formula/property tolerances only.

## Pointwise and property evidence

The committed curve tests cover saturated and hand-calculated finite negative-head points, finite dry heads, inverse-head round trips away from endpoints, the finite dry endpoint policy, theta bounds, non-negative conductivity/capacity, and monotonicity over a finite sampled domain. They also cover invalid parameters, non-finite heads and inverse inputs, and typed selector rejection.

The committed table tests cover exact requested endpoints, logarithmic head spacing, analytic node reproduction, finite interpolated values, applicable monotonicity, inclusive endpoint evaluation, out-of-range/non-finite rejection, invalid construction, and unsupported selectors. Together these are pointwise and mathematical property checks against the source-shaped contract. They are not a Fortran parity result.

## Invalid selectors and domains

`HydraulicModelSelector::from_i32(0)` is the only accepted selector. Nonzero selectors, including the rejected legacy selector 8, return `UnsupportedSelector`; they never fall back to model 0. `HydraulicTables::from_selector` likewise rejects selectors rather than treating a generated cache as a general table.

Non-finite heads and inverse-theta values return `InvalidEvaluationDomain`. Inverse theta outside the validated residual-to-saturated interval is rejected. Generated-table construction rejects non-finite, nonnegative, unordered, or equal endpoints and fewer than two points. Generated-table evaluation rejects non-finite and out-of-range heads.

## Generated-table boundary

`HydraulicTables` is an immutable cache generated only from a validated model-0 analytic model. Its domain is finite, strictly negative heads with a source-shaped logarithmic grid, exact requested endpoints, and analytic theta/conductivity/capacity values at each node. Evaluation accepts both endpoints and uses linear interpolation in head between nodes, matching the selected `SetMat` interpolation branch.

This cache is not a general or user-supplied input-table implementation. It performs no extrapolation and no implicit saturation extension; heads outside its closed generated interval fail. General tables (`iModel=nTabMod`), alternate analytic families, and their source-specific extrapolation/scaling behavior remain explicitly unsupported and deferred.

## Exit checklist

- [x] Validated model-0 hydraulic API and typed error boundary are present.
- [x] Formula, pointwise, property, domain, selector, and generated-table tests are committed.
- [x] `f64` versus legacy `sngl` limitations and source floors are recorded.
- [x] Generated-table and general-table boundaries are explicit.
- [x] No Fortran parity, release compatibility, universal tolerance, fixture, solver, mesh, or performance claim is made.
- [ ] Canonical fixture comparison and tolerance calibration remain future work.

## Phase 2 handoff

Proceed to Phase 2 mesh and source-stencil work. The next scope should define checked mesh/profile inputs, one-based conversion, layer metrics, inclination, face averaging, and the observed `Reset` stencil, while preserving the Phase 1 hydraulic contract. Reconcile the source finite-difference-style half-cell stencil with the manual finite-element terminology through dedicated algebraic and fixture evidence.

Defer alternate analytic hydraulic families and general/user-supplied tables to a later explicit scope decision. Do not widen this handoff to solver, atmospheric, root, balance, solute, heat, hysteresis, dual-porosity, or optional-physics acceptance.

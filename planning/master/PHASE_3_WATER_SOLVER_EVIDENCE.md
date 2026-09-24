# Phase 3 water solver evidence

## Provenance

This evidence records the corrected Phase 3 Rust contracts and their tests. Its source provenance is `WATFLOW.FOR:3-168,173-353,1079-1125` and `TIME.FOR:3-30`, interpreted through `NUMERICAL_DESIGN.md`, `API_AND_IO.md`, `STATE_AND_CALLS.md`, `VALIDATION_STRATEGY.md`, and the Phase 2 mesh/stencil evidence. It incorporates the completed 5.1.1–5.3.1 and 7.1.1–7.5.1 task reports. The source anchors motivate the assembly, solve, trial, retry, and update shapes; they are not an executable reference result.

## Boundary contract

`hydraulics::transform_boundary_row` uses the zero-based tridiagonal equation `lower * h[i - 1] + diagonal * h[i] + upper * h[i + 1] = rhs`. It accepts only endpoint locations and generic `Dirichlet`, caller-signed `Neumann`, or `Natural` requests. Dirichlet replaces the row with an identity equation, Neumann adds the supplied signed flux to RHS, and Natural preserves the caller-prepared row. The contract validates dimensions, endpoint shape, finite values, a positive finite minimum pivot, and pivot-relevant rows; unsupported selectors return a typed error without fallback.

`boundary_rows` has 6 tests for lower/upper algebra, signed flux, natural preservation, two-node shapes, finite/domain failures, pivot rejection, and typed unsupported behavior. This is generic row plumbing: it does not select atmospheric, ponding, seepage, root, or physical lower-boundary policy.

## Linear solver

`hydraulics::solve_tridiagonal_in_place` is an in-place Thomas kernel over caller-owned `f64` lower, diagonal, upper, and RHS buffers. On success RHS holds the solution, diagonal holds forward-eliminated pivots, and `PivotDiagnostics` reports the minimum accepted absolute pivot. It rejects empty/mismatched/non-finite buffers, invalid endpoint coefficients or threshold, and singular/near-zero pivots through `TridiagonalError`; it does not clamp or substitute pivots.

`tridiagonal` has 5 tests for hand-solved and larger systems, minimum-pivot diagnostics, typed forward/terminal pivot failures, unchanged buffers on failed preflight, and caller buffer reuse. Boundary assembly remains in the caller; this is neither a general-band nor a transport solver.

## Picard stepping

`advance_water_step` combines Phase 2 interior `Reset`-style rows with caller-provided generic endpoint rows and the tridiagonal kernel. Each Picard pass evaluates the model selected by each node's validated material index at its current trial head, then assembles with those evaluated conductivity and capacity values. Trial water content and water convergence use model-evaluated theta. Evaluation buffers are allocated in `WaterWorkspace` setup and reused. The filesystem-free public surface uses `f64` and caller-provided sink and signed endpoint fluxes. Its accepted-step report records accepted time, timestep, iterations, retries, free-node head residual, water residual, minimum pivot, and water balance; successful diagnostics are finite.

The caller supplies a finite target time later than accepted state time. The initial step and every retry are clipped so they cannot cross the target; acceptance landing on it promotes the exact target time. Callers select farther targets and repeat calls. This is generic target-time behavior only: Phase 3 adds no forcing schedule, event selection, or physical boundary switching. Prescribed Dirichlet endpoints are excluded from the reported head residual; endpoint water residuals remain part of convergence.

`hydraulics::ResetRow::calculate` is the shared interior-row calculation used by both `assemble_reset_stencil` and water-solver assembly. This keeps the Phase 2 row algebra in one allocation-free path.

`HydraulicTables` stores caller-declared, non-empty unit symbols for head, conductivity, and capacity; water content is dimensionless, and the labels cause no unit conversion. Its derivatives are the piecewise-linear segment slopes for theta, conductivity, and capacity. At an internal knot the right segment is selected; at the final endpoint the left segment is selected.

## Transactions and retries

`WaterState` holds accepted heads, water contents, time, and accepted-only accounting. `WaterWorkspace` preallocates coefficient, RHS, trial, and snapshot buffers. Each trial snapshots accepted state; only convergence promotes trial fields, time, counters, and cumulative accounting. A rejected trial restores heads, water contents, time, accounting, and trial buffers before dividing timestep by the configured retry factor. If the next retry would fall below `minimum_timestep`, the solver returns typed `NonConverged` without partial acceptance.

`water_transaction` has 4 tests for reduced-timestep retry with accepted-only promotion, target-clipped retry, minimum-step restoration, and invalid-input non-mutation. This transaction covers Phase 3 single-domain water state only; later coupled histories require their own rollback evidence.

## Balances and allocation

The accepted-step `WaterBalance` terms are per unit cross-sectional area. Storage change is `sum((trial_theta - accepted_theta) * nodal_control_volume_width)`. Sink is `accepted_timestep * sum(sink_rate * nodal_control_volume_width)`. Boundary flux is the sum of caller-signed endpoint fluxes times the accepted timestep. The residual is `abs(storage_change - boundary_flux + sink)`; normalization divides by the sum of absolute balance terms, or equals the absolute residual when that denominator is zero. This is a numerical diagnostic over supplied terms, not validation of a physical boundary mass balance.

The single-test `allocation_contract` integration binary uses its own counting allocator. It measures one fixed-size accepted step and one successful rejected-trial/retry call, with setup and warm-up outside the counting windows; both calls report zero allocations. The retry path exercises restore, assembly, elimination, and diagnostics as part of the measured solver call. This is bounded allocation-contract evidence, not a performance result. Construction, setup/resizing, error construction, and output are outside the assertion.

Exact Orchestration 7 targeted provenance is recorded in [STATE_7.md](../STATE_7.md): `api_contract` 4/4; `hydraulic_tables` 6/6; `allocation_contract` 1/1; `water_transaction` 3/3 after Wave 1 and 4/4 in later waves; `water_solver` 5/5 after Wave 2, 7/7 after Wave 3, and 10/10 after Wave 4; `reset_stencil` 6/6; and `mesh_geometry` 8/8. Final serial workspace gates passed at revisions `26ae9bb` and `2a3b422`, each with 59 tests passed, 0 failed; formatting, warning-denied workspace Clippy, and diff checks also passed. These are distinct recorded runs, not one combined targeted invocation.

## Unsupported behavior

Unsupported boundary selectors fail validation rather than falling back. Phase 3 implements only caller-supplied target-time clipping; it excludes atmospheric forcing schedules/event selection, ponding or seepage switching, physical fixed-head/prescribed-flux/free-drainage policy, root uptake, legacy input/output, solute, heat, hysteresis, dual porosity, and host behavior. Strict typed pivot and minimum-step failures deliberately differ from the source's small-pivot substitution and `sngl` conversion.

## Exit checklist

- [x] Boundary rows: 6 targeted tests.
- [x] Tridiagonal kernel: 5 targeted tests.
- [x] Water solver: Orchestration 7 targeted runs grew from 5 to 7 to 10 tests as contracts were added; final targeted result 10/10.
- [x] Water transactions: 4 targeted tests.
- [x] Hydraulic tables: 6 targeted tests for caller-declared unit labels, empty-label rejection, piecewise-linear derivatives, and knot/endpoint side selection.
- [x] Reset stencil and mesh geometry: 6 and 8 targeted tests, including shared-row equivalence and nonuniform control-volume coverage.
- [x] Generic boundary rows, strict pivot diagnostics, per-material trial-head Picard evaluation, accepted-only transactions, retry, caller target-time clipping, weighted balance formulas, and shared row assembly have contract evidence.
- [x] Isolated allocation contract: one test proves zero counted allocations for fixed-size accepted and successful retry paths after setup.
- [x] Final recorded workspace suite: 59 tests passed, 0 failed at both `26ae9bb` and `2a3b422`; formatting, warning-denied workspace Clippy, and diff checks passed.
- [x] The core contract remains `f64` and filesystem-free.
- [ ] A Fortran executable baseline, calibrated tolerances, release compatibility, fixture parity, and benchmark/performance evidence remain unavailable.

This is source-algebra and Rust-contract evidence, not a Fortran parity result. It makes no claim of universal tolerance, physical calibration, reference agreement, release compatibility, or speedup.

## Phase 4 handoff

The Phase 3 solver, public stepping inputs, transaction/retry contract, generic boundary rows, target-time behavior, balances, and updated evidence satisfy the technical Phase 3 entry checklist for resuming Phase 4 planning. This is planning readiness only: it does not accept the Phase 4 water product or authorize implementation. Phase 4 execution remains pending and has not started.

Phase 4 starts from accepted Phase 3 water state and must add the minimal forcing/input adapter; atmospheric event alignment; upper precipitation/evaporation, ponding, and fixed-head/flux switching; supported lower fixed-head, prescribed-flux, and free-drainage variants; and Feddes/S-shaped root stress with compensation. It must produce dry-column infiltration, cyclic atmospheric ponding/switching, and root-stress/compensation fixtures with mass-balance sign-off, independent/reference comparison when the oracle is available, and typed unsupported-combination errors.

Phase 4 is the first water-product acceptance gate. It still excludes solute, heat, hysteresis, dual porosity, optional chemistry, broad I/O/CLI polish, performance claims, and release claims.

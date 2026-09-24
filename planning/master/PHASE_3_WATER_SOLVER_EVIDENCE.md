# Phase 3 water solver evidence

## Provenance

This evidence records the Phase 3 Rust contracts and their tests. Its source provenance is `WATFLOW.FOR:3-168,173-353,1079-1125` and `TIME.FOR:3-30`, interpreted through `NUMERICAL_DESIGN.md`, `API_AND_IO.md`, `STATE_AND_CALLS.md`, `VALIDATION_STRATEGY.md`, and the Phase 2 mesh/stencil evidence. It also incorporates the completed 5.1.1–5.3.1 task reports. The source anchors motivate the assembly, solve, trial, retry, and update shapes; they are not an executable reference result.

## Boundary contract

`hydraulics::transform_boundary_row` uses the zero-based tridiagonal equation `lower * h[i - 1] + diagonal * h[i] + upper * h[i + 1] = rhs`. It accepts only endpoint locations and generic `Dirichlet`, caller-signed `Neumann`, or `Natural` requests. Dirichlet replaces the row with an identity equation, Neumann adds the supplied signed flux to RHS, and Natural preserves the caller-prepared row. The contract validates dimensions, endpoint shape, finite values, a positive finite minimum pivot, and pivot-relevant rows; unsupported selectors return a typed error without fallback.

`boundary_rows` has 6 tests for lower/upper algebra, signed flux, natural preservation, two-node shapes, finite/domain failures, pivot rejection, and typed unsupported behavior. This is generic row plumbing: it does not select atmospheric, ponding, seepage, root, or physical lower-boundary policy.

## Linear solver

`hydraulics::solve_tridiagonal_in_place` is an in-place Thomas kernel over caller-owned `f64` lower, diagonal, upper, and RHS buffers. On success RHS holds the solution, diagonal holds forward-eliminated pivots, and `PivotDiagnostics` reports the minimum accepted absolute pivot. It rejects empty/mismatched/non-finite buffers, invalid endpoint coefficients or threshold, and singular/near-zero pivots through `TridiagonalError`; it does not clamp or substitute pivots.

`tridiagonal` has 5 tests for hand-solved and larger systems, minimum-pivot diagnostics, typed forward/terminal pivot failures, unchanged buffers on failed preflight, and caller buffer reuse. Boundary assembly remains in the caller; this is neither a general-band nor a transport solver.

## Picard stepping

`advance_water_step` combines Phase 2 interior `Reset`-style rows with caller-provided generic endpoint rows and the tridiagonal kernel. It operates on deterministic caller-provided conductivity, capacity, sink, and signed endpoint-flux arrays using a filesystem-free, `f64` public surface. Its accepted-step report records accepted time, timestep, iterations, retries, free-node head residual, water residual, minimum pivot, and water balance; successful diagnostics are finite.

`water_solver` has 3 tests for deterministic stencil/Picard convergence, boundary/kernel integration and balance reporting, the documented zero-denominator balance behavior, and the source-level absence of a filesystem API. Prescribed Dirichlet endpoints are excluded from the reported head residual; endpoint water residuals remain part of convergence.

## Transactions and retries

`WaterState` holds accepted heads, water contents, time, and accepted-only accounting. `WaterWorkspace` preallocates coefficient, RHS, trial, and snapshot buffers. Each trial snapshots accepted state; only convergence promotes trial fields, time, counters, and cumulative accounting. A rejected trial restores heads, water contents, time, accounting, and trial buffers before dividing timestep by the configured retry factor. If the next retry would fall below `minimum_timestep`, the solver returns typed `NonConverged` without partial acceptance.

`water_transaction` has 4 tests for reduced-timestep retry with accepted-only promotion, minimum-step restoration, invalid-input non-mutation, and fixed-size allocation behavior. This transaction covers Phase 3 single-domain water state only; later coupled histories require their own rollback evidence.

## Balances and allocation

The accepted-step `WaterBalance` records storage change, caller-supplied signed boundary flux, sink, absolute residual, and normalized residual. The residual is `abs(storage_change - boundary_flux + sink)`; normalization divides by the sum of absolute balance terms, or equals the absolute residual when that denominator is zero. It is a numerical diagnostic over supplied terms, not validation of a physical boundary mass balance.

The scoped allocation test establishes zero allocations after setup for fixed-size accepted steps, retries, restoration, assembly, elimination, and diagnostics. Construction, setup/resizing, error construction, output, benchmarking, and performance comparison are outside that assertion.

## Unsupported behavior

Unsupported boundary selectors fail validation rather than falling back. Phase 3 intentionally excludes atmospheric forcing/event alignment, ponding or seepage switching, physical fixed-head/prescribed-flux/free-drainage policy, root uptake, legacy input/output, solute, heat, hysteresis, dual porosity, and host behavior. Strict typed pivot and minimum-step failures deliberately differ from the source's small-pivot substitution and `sngl` conversion.

## Exit checklist

- [x] Boundary rows: 6 targeted tests.
- [x] Tridiagonal kernel: 5 targeted tests.
- [x] Water solver: 3 targeted tests.
- [x] Water transactions: 4 targeted tests.
- [x] Workspace evidence: 47 tests when the recorded Phase 3 workspace suite ran.
- [x] Generic boundary rows, strict pivot diagnostics, Picard stepping, accepted-only transactions, retry, balance denominator policy, and scoped allocation behavior have contract evidence.
- [x] The core contract remains `f64` and filesystem-free.
- [ ] A Fortran executable baseline, calibrated tolerances, release compatibility, fixture parity, and benchmark/performance evidence remain unavailable.

This is source-algebra and Rust-contract evidence, not a Fortran parity result. It makes no claim of universal tolerance, physical calibration, reference agreement, release compatibility, or speedup.

## Phase 4 handoff

Phase 4 starts from accepted Phase 3 water state and must add the minimal forcing/input adapter; atmospheric event alignment; upper precipitation/evaporation, ponding, and fixed-head/flux switching; supported lower fixed-head, prescribed-flux, and free-drainage variants; and Feddes/S-shaped root stress with compensation. It must produce dry-column infiltration, cyclic atmospheric ponding/switching, and root-stress/compensation fixtures with mass-balance sign-off, independent/reference comparison when the oracle is available, and typed unsupported-combination errors.

Phase 4 is the first water-product acceptance gate. It still excludes solute, heat, hysteresis, dual porosity, optional chemistry, broad I/O/CLI polish, performance claims, and release claims.

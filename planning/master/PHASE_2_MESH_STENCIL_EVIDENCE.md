# Phase 2 mesh and source-stencil evidence

Phase 2 records a checked, filesystem-free mesh/profile contract, pure geometry helpers, and the observed interior `Reset` stencil. It is source-derived contract and algebra evidence, not a calibrated reference comparison.

## Mesh contract

The committed `hydrus-core` implementation exposes `Mesh1D` through the public hydraulic module and crate root. Construction validates finite, strictly increasing coordinates; coordinate/material/layer length relationships; declared material and layer counts; zero-based material and optional layer indices; and a finite inclination cosine/metric. The representation exposes only the mesh data needed by the later geometry and stencil tasks and has no filesystem or solver dependency.

The committed `mesh_contract` integration test has four tests covering successful construction, optional layers, accessors, and typed rejection of invalid mesh values. The implementation is in `crates/hydrus-core/src/hydraulics/types.rs`, with the public registration in `crates/hydrus-core/src/hydraulics/mod.rs` and the root export in `crates/hydrus-core/src/lib.rs`.

## Source-index conversion

`Mesh1D::from_source_one_based` is the explicit adapter for source-style material and layer indices. It rejects zero indices, out-of-range indices, length mismatches, invalid declared counts, non-finite coordinates or inclination, and non-increasing coordinates before storing successful inputs as zero-based Rust indices. The committed test verifies one-based conversion to the expected zero-based material/layer data and covers the listed zero, range, and length failures.

This boundary reflects the source inventory: profile coordinates and `MatNum`/`LayNum` are source-owned mesh data, and the supplied Fortran arrays are one-based (`HYDRUS.FOR:48-95`; `WATFLOW.FOR:173-220`). No claim is made that the adapter alone establishes compatibility with every legacy input path.

## Geometry and averaging

The pure geometry helpers in `crates/hydrus-core/src/hydraulics/geometry.rs` provide signed adjacent coordinate distances, source-style lower and upper boundary half-cell widths, contiguous layer first-to-last coordinate spans, and explicit inclination/gravity access using the documented convention `Grav = CosAlf`. The layer convention requires declared layers to be present, populated, and contiguous; it is a Phase 2 contract choice, not a Fortran-parity result.

Face conductivity is explicit: arithmetic averaging is the default policy used by the stencil, while geometric averaging is selected only through `FaceAveraging::Geometric`. Conductivity inputs must be finite; geometric averaging rejects negative inputs and never silently falls back to arithmetic. The `mesh_geometry` integration test has seven tests covering nonuniform/signed distances, exact boundary half-cells, layer spans and failures, inclination, both averaging policies, and invalid values.

The source anchors for these calculations are `WATFLOW.FOR:197-228` and the proposed spatial contract in `master/NUMERICAL_DESIGN.md#discretization`. Boundary-condition transformation remains outside this evidence.

## Reset stencil

`assemble_reset_stencil` in `crates/hydrus-core/src/hydraulics/stencil.rs` assembles only interior rows for mesh nodes `1..n-1`. Each returned row contains lower, diagonal, upper, and RHS values. The implementation preserves the observed `WATFLOW.FOR:232-256` algebra: adjacent distances and half-cell width, arithmetic or explicitly geometric face conductivities, `B = (ConA - ConB) * Grav`, conductivity terms, capacity/timestep storage contribution, water-content change, and sink sign.

The committed `reset_stencil` integration test has five tests. It checks hand-calculated multi-row values, inclination and geometric faces, storage/timestep and sink signs, typed invalid-input rejection, and an explicit documentation note distinguishing source algebra from parity. This is stencil evidence, not a Fortran parity result; no boundary rows are represented.

## Typed failures

Mesh, geometry, averaging, and stencil domain failures are returned through typed `HydraulicError::InvalidMesh` values. Tests cover empty or undersized meshes, non-finite values, non-increasing coordinates, mismatched arrays, invalid counts and indices, absent or noncontiguous layers, invalid inclination, invalid timestep, invalid conductivities, negative geometric inputs, and non-finite derived row values. These checks establish the implemented contract only; they do not establish a universal tolerance or a validated legacy error-message/API match.

## Exit checklist

- [x] Checked `Mesh1D` contract and public exports are present.
- [x] Explicit one-based source conversion and zero-based internal representation are tested.
- [x] Geometry, layer metrics, inclination/sign convention, and boundary half-cell behavior are tested.
- [x] Arithmetic default and explicit geometric face averaging are tested without silent fallback.
- [x] Interior source `Reset` row algebra, storage/timestep terms, gravity, and sink signs are tested.
- [x] Typed finite/domain failure behavior is tested across the Phase 2 boundary.
- [x] Boundary transformation, solver iteration, retries, transaction/rollback, and coupled process behavior remain deferred.
- [x] The source finite-difference versus manual finite-element terminology reconciliation remains an open validation item.
- [x] Full workspace tests, formatting, clippy with warnings denied, and `git diff --check` pass.
- [ ] Reviewed Fortran fixture comparison, parity, numerical baseline/tolerance calibration, compatibility precision, allocation, and performance evidence remain future work.

The implementation is filesystem-free and does not add boundary, solver, transaction, fixture, parity, allocation, or performance behavior. The Phase 2 evidence therefore does not claim release compatibility, numerical baseline agreement, or Fortran parity.

## Phase 3 handoff

Phase 3 may proceed from this Phase 2 mesh/stencil evidence to boundary-aware tridiagonal assembly and transactional Picard stepping. The next implementation must integrate boundary-row transformations and `Shift` semantics around the interior rows, define elimination and convergence/retry behavior, and preserve accepted versus trial state explicitly. The state design must address rollback of water contents and heads, boundary mode/ponding/seepage state, accounting, and any coupled histories before claiming a complete transaction; `STATE_AND_CALLS.md` identifies `WatFlow` trial state, `Update`, and retry behavior as the relevant source boundaries.

Phase 3 must add dedicated evidence for boundary conditions, pivot/elimination behavior, convergence and timestep retry, accepted/trial rollback, balances, and any selected forcing/root coupling. It must preserve typed failures and the filesystem-free core boundary. It must also reconcile the source finite-difference terminology with the manual finite-element terminology through dedicated algebraic and fixture validation before making any parity or fixture claim.

This handoff does not accept atmospheric, root, solute, heat, hysteresis, dual-porosity, optional-physics, release-compatibility, allocation, or performance behavior. Those remain later or explicitly gated scopes under the master plan.

# M6 vapor kernel: flux upper with fixed-head lower (DRAFT for review)

Status: DRAFT. Nothing in `crates/` is changed by this document. Applying it needs the gate.

## Current behavior
- `VaporBoundaryPair::classify` (`crates/hydrus-core/src/heat/vapor/mod.rs:~250-305`) accepts four pairs: head/head, closed flux/flux, free-drainage lower with flux upper, and (implicitly) none else. `(FixedHead lower, Flux upper)` returns `UnsupportedUpperBoundary` (line ~301, comment "today's upper rejection").
- `water_solver.rs` (~line 360-420) builds the vapor rows per pair: `ClosedFlux` and `FreeDrainage` add a Neumann lower row and a Neumann upper row through `vapor_upper_flux_row`. `HeadHead` uses the standard assembly.
- Fortran: for `KodTop=-1` the top row is the flux row `PT = ... - ConTA*(T_N - T_{N-1})/dxA - (ThVNew - ThVOld)*dx/dt` (`WATFLOW.FOR:288-291`), which `vapor_upper_flux_row` already implements. For `KodBot >= 0` the bottom node is Dirichlet with `hBot` (`WATFLOW.FOR` `Gauss`, `TIME.FOR` `SetBC`), so no bottom vapor row is needed.

## Proposed change
1. New variant `VaporBoundaryPair::FixedHeadLowerFluxUpper { upper_flux: f64 }`.
2. `classify`: replace the `(FixedHead, Flux)` rejection with `Ok(Self::FixedHeadLowerFluxUpper { upper_flux })` after `validate_finite("upper flux", upper_flux)?`. Keep `(FixedHead, Atmospheric)` rejected.
3. `water_solver.rs`: add an arm that keeps the standard lower Dirichlet assembly and applies the Neumann upper row:
   ```rust
   VaporBoundaryPair::FixedHeadLowerFluxUpper { upper_flux: upper_flux_in } => Some((
       lower_flux_from_assembly,            // Dirichlet at node 0: unchanged
       upper_flux_in,
       /* lower row: not replaced (Dirichlet) */ ...,
       vapor_upper_flux_row(&rows_input, input.face_averaging)?,
   ))
   ```
   The exact shape depends on how the Dirichlet lower row is carried through `closed_rows`; the closed-pair path replaces both rows, so the new arm must skip the lower replacement.
4. Heat coupling: check whether `heat/vapor` or the heat solver branches on `VaporBoundaryPair`. If so, the new variant needs the same arms.
5. Producer (`crates/hydrus-core/tests/vapor_fortran_parity.rs`): the guard and lower-boundary changes from the reverted attempt (`k_top == -1 && k_bot == 1`, lower chosen by `k_bot`), plus `FIXEDHEAD_CASES` and `flux_fixedhead_wet_on_matches_fixture`. The existing flux/flux and free-drainage paths must not change.

## Tests to add
- `classifies_pairs_and_rejects_all_others`: update the table: `(0,1)` is `FixedHeadLowerFluxUpper { upper_flux: -0.25 }`; `(0,2)` stays rejected.
- A unit test that the new row carries the upper vapor terms (`ConTA`, storage) and leaves the lower Dirichlet row unchanged.
- Parity: `flux_fixedhead_wet_on_matches_fixture` (must pass with the approved tolerances, no criteria change).
- Regression: the full parity suite and `vapor_balance` unchanged.

## Risks and open checks
- The Dirichlet lower row in the vapor path has not been traced in this draft. Confirm in `water_solver.rs` that `assemble` already gives the head row at node 0 for `FixedHead` lower.
- Mass balance: a flux upper with fixed-head lower changes the water balance (the bottom absorbs or supplies water). A balance check should be run on the wet fixture, as for the closed pair (`BALANCE.OUT`).
- The dry deck (-10000 cm, graded temperature) diverges in Fortran. Do not add it as a fixture; keep it as the recorded divergence.

## Gate
Approval needed for: (a) the kernel change in `classify` and `water_solver.rs`, (b) the producer change in the test file, (c) the wet fixture check. No change to `criteria.json`.

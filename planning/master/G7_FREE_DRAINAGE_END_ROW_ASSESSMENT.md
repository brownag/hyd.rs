# G7 free-drainage end-row assessment

Task 28.1.4 (orchestration 28). Read-only; no register entry added, no run.

## Question
Is the free-drainage end row at `src_reference/WATFLOW.FOR:213` a source defect, a modeling choice, or not established?

## Register rules (`docs/FORTRAN_SOURCE_DEFECTS.md`)
- `:8-12` the port keeps defects as the source has them; fixes go in a separate non-compat mode.
- `:13-14` each entry has an ID `SD-NNN` and code carries a comment naming it.
- `:15-17` each entry needs a pinning test `source_defect_sdNNN_*` that fails if the defect is "fixed".
- `:18-20` add an entry only after checking every other site computing the same quantity; a consistent pattern is a modeling choice. `:186` "Not defects" holds such cases.
- Next free ID is SD-007 (plan Recon facts; entries after SD-002 not read in scope).

## Fortran path (`WATFLOW.FOR`, subroutine `Reset`, bottom row)
- `:202-203` `ConB` is the arithmetic (geometric if `lGeom`) mean of `Con(1)`, `Con(2)` (liquid conductivity).
- `:205` `B=ConB*Grav`; the `lDensity` variant `:207-209` also uses liquid `ConB` and sets `fRE`.
- `:211` `if(lVapor) ConB=ConB+(ConVh(1)+ConVh(2))/2.` adds vapor hydraulic conductance.
- `:213` `if(FreeD) rBot=-ConB*Grav*fRE` uses the post-vapor `ConB`.
- `:221-222` `PB=B-...+rBot`. Vapor off: `B+rBot=0` (unit gradient, no net gravity drive at the end node). Vapor on: `B` carries liquid K, `rBot` carries liquid+vapor K, so they differ by `ConVh_face*Grav*fRE`.

## Rust path (`crates/hydrus-core/src/lower_boundary.rs`)
- `:161-171` free drainage validates face conductivities `conductivity_a/b` (finite, positive) and gravity.
- `:172-177` arithmetic or geometric face average; `:178-181` comment cites WATFLOW `Reset`, `flux = -conductivity * trial.gravity`, Neumann.
- Whether `conductivity_a/b` include vapor conductance: not read in scope. Rust rejects non-FixedHead lower modes with vapor on (`heat/vapor/mod.rs:213-217`, from plan Recon facts; not read in scope), so the question cannot arise in Rust today.

## Cross-site comparison
Sites computing the gravity-driven liquid flux or its vapor counterpart in `WATFLOW.FOR`:
- Interior rows: `B=(ConA-ConB)*Grav` `:239` is formed before vapor is added at `:246-249`; liquid only.
- Top row: `B=ConA*Grav` `:276` then vapor added `:277`. Exception: with `lDensity`, `B` is recomputed at `:279` from the post-vapor `ConA`, so vapor enters a gravity term there too (`:267-269` in the coordinator's note is `:278-280` in this file as read).
- Bottom row with `lDensity` `:207-209`: `B` stays liquid (vapor added later at `:211`).
- Flux diagnostics: `Shift` `:377` and `:400` use liquid `(Con+Con)/2*(dh/dx+Grav*fRE)`; vapor is a separate term without `Grav` (`:405-407`). `Veloc` `:732` liquid with `Grav`; vapor `vV` `:736` and `:770` have no `Grav`.
- Result: vapor conductance multiplies `Grav` at `:213` and in the top-row `lDensity` branch `:279`; everywhere else it does not. The pattern is not consistent across sites, so it cannot be filed under "Not defects", but the source gives no statement of intent either.

## Evidence limits
- No run is allowed, so no reproduction of the effect on output.
- The top-row `lDensity` case shows the source itself is inconsistent about this pattern, which weakens a simple "`:213` is a typo" reading.
- Vapor-off behavior (the only case the Rust port supports) is the standard unit-gradient free drainage and is unaffected.

Verdict: not established

## Upgrade path (not done here)
To claim a defect: reproduce at kernel level that `:213` with `lVapor` gives `rBot` differing from `-B` by `ConVh_face*Grav*fRE` and decide whether that is intended. Then assign SD-007, add a port comment naming SD-007 and the register, and add a pinning test such as `source_defect_sd007_free_drainage_vapor_conductance_scales_gravity_flux` with a doc comment naming SD-007. Register entry not added.

## Hook for GATE W1 (D5)
Only vapor-on differs from water-only; reconcile with the 28.1.1 free-drainage audit. The Rust vapor rejection stays (H1).

## Questions
None open.

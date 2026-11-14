# M6 vapor closed flux/flux pair: Rust row and guard design (task 29.1.2)

Status: design only (read-only task). No rejection is relaxed here (D6). Heat terms stay separate (D7).
Fortran cites are `src_reference/WATFLOW.FOR`, re-read in this task (scope amendment: lines 202-229, 272-294, 315-334 only). Rust cites are file:line as read.
`heat/vapor/water.rs` was read for lines 1-120 only. `transform_boundary_row`, `heat/vapor/mod.rs` outside :205-230, and the vapor property structs were not read in scope.

## 1. Anchor corrections to the plan
- Rejection sites: lower `heat/vapor/mod.rs:211-218` (catch-all arm `mode =>` at :217), upper `:219-226` (arm at :225). The plan said :213-217 and :219-225. Use the corrected ones.
- Upper flux has no Rust row builder. `UpperBoundary::Flux { flux, row }` takes a caller-supplied row (`water_solver.rs:90`, `:114`). The lower flux row is built in Rust (`lower_boundary.rs:43-96`, via `LowerTrial::flux_row` :219-235) but is liquid-only.
- Boundary rows come from `assemble` (`water_solver.rs:830-898`): lower via `prepare_step` (:830), upper via `caller_row` (:855), then `transform_boundary_row` (:859). The vapor trial calls `assemble_water_rows` (:340) and overwrites nodes `1..n-1` only (:361-367). Boundary rows are therefore liquid-only today.
- `assemble_water_rows` (`heat/vapor/water.rs:38`) is documented as interior-only: "Boundary rows are owned by the water boundary path" (:23).

## 2. Lower row terms (bottom node 1; Rust node 0)
Fortran, WATFLOW.FOR:
| Term | Fortran | Rust source of value |
|---|---|---|
| `ConB` face average (arith/geom) | :202-203 | `LowerTrial::flux_row` :220-223 (existing) |
| Gravity `B=ConB*Grav`, computed before vapor is added, so gravity stays liquid-only | :205 | existing `face_conductivity*gravity` (`lower_boundary.rs:84`); must use liquid `ConB` |
| `ConVh` into face conductance: `ConB=ConB+(ConVh(1)+ConVh(2))/2` | :211 | new field `vapor_head_conductivity_a/b` (nodes 0,1 of `vapor_head_conductivities`) |
| `S(1)=-ConB/dxB`, `SB=-ConB/dxB`, `RB=ConB/dxB+F2` | :212, :216-218 (`RB`, `SB`) | `upper=-conductance`, `diagonal=conductance+storage` (`lower_boundary.rs:77-82`), now with vapor `ConB` |
| `F2=Cap(1)*dx/dt` (effective capacity) | :214 | new input: `effective_hydraulic_capacities[0]`, replacing liquid `capacity` |
| `rBot` in `PB` | :221-222 | NOT in the row. Rust passes it as `BoundaryCondition::Neumann{flux}` (`lower_boundary.rs:160`) |
| `PB=B-Sink*dx+F2*h-(ThNew-ThOld)*dx/dt` | :221 | existing rhs (`lower_boundary.rs:83-85`) |
| `ConTB=(ConVT(1)+ConVT(2))/2` | :225-227 | new fields from `vapor_thermal_conductivities[0,1]` |
| `PB=PB+ConTB*(Temp(2)-Temp(1))/dxB` | :228 | new rhs term; needs temperatures at nodes 0,1 |
| `-(ThVNew(1)-ThVOld(1))*dx/dt` | :228 | new rhs term from `vapor_water_new/old[0]` |
| Neumann elimination `P(2)-=PB*S(1)/RB`, `R(2)-=SB*S(1)/RB`, `RB` floored at `rMin` | :315-322 | reuse `transform_boundary_row` with `Neumann` unchanged (not read in scope) |

Sign: Fortran `rBot` enters `PB` with `+` and free drainage is `rBot=-ConB*Grav*fRE` (:213), so positive `rBot` is into the domain. This matches Rust "positive into the domain" (`lower_boundary.rs:17`), so lower `flux = rBot` with no sign change. Note `ConB` in :213 is the vapor-augmented one, which is the G7 free-drainage inconsistency; that mode stays out of scope.
Dropped by design: `fRE`/`lDensity` (:204-208), `Rate`, `iDualPor`, `lWTDep`, `lCentrif`. The Rust trial does not carry them. Not verified against vapor-on config; wave 2 must reject or assert them off.

## 3. Upper row terms (top node N; Rust node n-1)
| Term | Fortran | Rust |
|---|---|---|
| `ConA` face average | :273-274 | arithmetic as in `water.rs:110`; geometric per `face_averaging` |
| `B=ConA*Grav` before vapor, so gravity is liquid-only | :276 | liquid `ConA * gravity` |
| `ConA=ConA+(ConVh(N)+ConVh(N-1))/2` | :277 | new `ConVh` average, nodes n-1, n-2 |
| `F2=Cap(N)*dx/dt`; `RT=ConA/dxA+F2`; `ST=-ConA/dxA` | :282-284 | `diagonal`, `lower` of the top row, effective capacity |
| `PT=F2*h-(ThNew-ThOld)*dx/dt-Sink*dx-B` | :285 | rhs base |
| `ConTA=(ConVT(N)+ConVT(N-1))/2` | :288-290 | new thermal conductivity average |
| `PT=PT-ConTA*(Temp(N)-Temp(N-1))/dxA` | :291 | rhs term (sign opposite to the lower row) |
| `-(ThVNew(N)-ThVOld(N))*dx/dt` | :291 | rhs term from vapor contents, node n-1 |
| `vTop=...` (diagnostic flux, before rTop) | :293 | measured-flux check only |
| `PT=PT-rTop` | :294 | Fortran `rTop` is positive OUT of the domain (it is subtracted from the source `PT`), so the Rust upper `flux` (positive in) equals `-rTop`. Confirm sign in wave 2 against the Fortran fixture. |
| Neumann elimination `P(N)=PT-P(N-1)*ST/R(N-1)`, `R(N)=RT-S(N-1)*ST/R(N-1)` | :327-334 (:331-332 shown) | reuse `transform_boundary_row` with `Neumann` |

Line cites verified by grep: `ConA` vapor at :277, `PT` vapor block at :291, `PT=PT-rTop` at :294. The plan cited :276 and :287-294; the first is the `B=ConA*Grav` line (before vapor is added), the rest are shifted by 1-3 lines.

## 4. Builders
Place pure builders next to the existing interior one in `heat/vapor/water.rs` (it already owns `VaporProperties` access):
```rust
pub struct VaporBoundaryRowsInput<'a> { /* same fields as VaporWaterRowsInput */ }
pub fn vapor_lower_flux_row(&VaporWaterRowsInput) -> Result<VaporWaterRow, VaporError>   // node 0, uses nodes 0,1
pub fn vapor_upper_flux_row(&VaporWaterRowsInput) -> Result<VaporWaterRow, VaporError>   // node n-1, uses nodes n-2,n-1
```
Reusing `VaporWaterRowsInput` avoids a second input struct: every needed field is already present (`water.rs:8-20`: coordinates, gravity, conductivities, effective capacities, heads, liquid and vapor contents old/new, temperatures, sink, timestep, vapor). Arithmetic face averaging only is what `water.rs:110` does; geometric is a wave-2 decision (Question A below).
`water_solver.rs` converts each row to `BoundaryRow` and passes it through the existing `transform_boundary_row` with `Neumann { flux }`. `lower_boundary.rs` is NOT changed (keeps its liquid-only row), which keeps ADR 0021's no-silent-approximation rule easy to audit.

## 5. Call site
In the vapor trial after `assemble(...)` and before/with the interior overwrite loop (`water_solver.rs:355-368`): when the typed guard (section 6) is `ClosedFlux`, build both rows, transform them with Neumann using the guard's two fluxes, and write `workspace[0]` and `workspace[n-1]` (the same four arrays the loop writes at :362-365). Head/head keeps the current path unchanged.

## 6. Typed guard
```rust
pub(crate) enum VaporBoundaryPair {
    HeadHead,                                   // today's accepted form
    ClosedFlux { lower_flux: f64, upper_flux: f64 },
}
impl VaporBoundaryPair {
    pub(crate) fn classify(lower: LowerBoundaryMode, upper: UpperBoundary)
        -> Result<Self, VaporError>;
}
```
Rules: `(FixedHead, Head)` -> `HeadHead` with the existing finite and row checks (`vapor/mod.rs:212-216`, `:220-224`); `(PrescribedFlux{flux}, Flux{flux,..})` -> `ClosedFlux` after finite checks; every other combination returns `UnsupportedLowerBoundary(lower)` or `UnsupportedUpperBoundary(upper)` exactly as today (`:217`, `:225`). Mixed pairs (flux lower with head upper, head lower with flux upper) are rejected; `FreeDrainage`, `Unsupported(..)` and `Atmospheric` stay rejected. The caller-supplied `Flux.row` is ignored for the vapor path (the builder supplies it); ignore must be tested.
Because the match is on both modes at once, an exhaustive match with no wildcard arm makes new enum variants a compile error.

## 7. Wave 2 change list
29.2.1 (builders, behind the guard):
- `crates/hydrus-core/src/heat/vapor/water.rs`: add `vapor_lower_flux_row`, `vapor_upper_flux_row`, unit tests per term.
- `crates/hydrus-core/src/water_solver.rs`: vapor trial near :340-368 calls the builders and overwrites rows 0 and n-1 for `ClosedFlux`.
29.2.2 (guard and lift):
- `crates/hydrus-core/src/heat/vapor/mod.rs`: add the enum and `classify`; replace the two `match` blocks at :211-226 (enclosing fn name not read in scope).
- `crates/hydrus-core/src/water_solver.rs`: consume the classification (same file as 29.2.1, so sequence them).
- Tests (paths to be chosen in wave 2, not read here): each non-pair mode rejected; liquid-only and head/head paths bit-identical.
No change to `lower_boundary.rs`, `criteria.json`, fixtures or `src_reference/`.

## 8. Open items for the user (GATE 29.1)
A. Geometric face averaging with vapor: `water.rs:110` is arithmetic-only; Fortran :203 and :274 allow `lGeom`. Reject `lGeom` with vapor, or implement it.
B. Upper `rTop` sign convention (section 3): confirm against fixture, since `Flux.flux` is positive-in.
C. `lDensity`, `iDualPor`, `lWTDep` must be asserted off for the vapor path.
D. `transform_boundary_row` Neumann semantics (whether `flux` is added to the rhs) were not read; wave 2 must confirm that `rBot`/`rTop` are not applied twice (once in the row, once as Neumann flux).

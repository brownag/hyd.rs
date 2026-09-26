# Solute advective-outlet numerical contract

Status: **Accepted for task 12.5.1 implementation after orchestrator derivation and exact-example review. This is not scientific parity evidence.**

## Decision

Represent the new mode as the valueless variant
`SoluteBoundaryMode::AdvectiveOutflow`. Identify ends with
`SoluteEndpoint::{Lower, Upper}`. A step with inward water flow returns
`SoluteStepError::InwardFlowAtOutlet { species, endpoint, flux }`, where the
fields are `usize`, `SoluteEndpoint`, and `f64`. The mode means zero normal
dispersive flux and advective export at the endpoint. It is available at
either geometric endpoint, with the sign of the accepted water face flux
deciding whether the mode is admissible. Reject inward water flow with the
typed error before beginning the workspace trial. Exactly zero flow is valid
and contributes no advective exchange.

Use the existing conservative control-volume equations. For node `i`, define
the width as half the adjacent interval at an endpoint and half the distance
between neighboring nodes at an interior node. Define `m_i` as this width
times dissolved plus configured sorbed mass per soil volume, and `r_i` as
the volume-integrated reaction/source rate. Do not replace an endpoint
transport row with the Fortran lower-endpoint adjacent-node equality. The new
endpoint term closes the same finite-volume balance already used by Rust's
endpoint half-cell storage and interior face operator:

\[
\frac{m_i^{n+1}-m_i^n}{\Delta t}
  + F_{i+1/2}^{n+1}-F_{i-1/2}^{n+1}=r_i^{n+1},
\qquad
F=qC-g_f(C_R-C_L),\quad g_f=\theta_f D_f/\Delta x_f,
\]

where interior advection uses the existing upwind split, `D_f` is effective
dispersion and `g_f` is its theta- and spacing-weighted face conductance, and
`m` includes dissolved and configured sorbed storage. At an outlet the boundary
flux contains only its advective part: there is no dispersive endpoint term.
Thus, for increasing
mesh coordinates and endpoint fluxes `qL = face_fluxes[0]` and
`qU = face_fluxes[node_count]`:

| Endpoint | Outflow domain | Matrix term added to endpoint row | Signed inward exchange over the step |
|---|---:|---:|---:|
| Lower | `qL <= 0` | `-qL * C[0]` on the diagonal | `dt * qL * Cnew[0]` |
| Upper | `qU >= 0` | `qU * C[N-1]` on the diagonal | `-dt * qU * Cnew[N-1]` |

The matrix term is nonnegative and stays on the left-hand side; there is no
outlet contribution to the right-hand side. It is independent of `theta` and
dispersion. A zero endpoint flux adds zero to both assembly and accounting.
Nonzero interior dispersion remains active, including in systems using this
mode.

## Physical signs, units, and source mapping

The continuous one-dimensional solute flux in the increasing-coordinate
direction is `J = q C - theta D dC/dx`. A zero-dispersive-gradient outlet
requires the outward dispersive contribution to vanish; with inward flow
excluded, the outward advective flux is `n q C`, where the outward normal is
`n=-1` at the lower endpoint and `n=+1` at the upper endpoint. The Rust signed
inward mass is therefore `-n q C dt`, matching the table. `q` is water volume
per area per time, `C` is the configured dissolved concentration, and the
product integrated over `dt` uses the configured column mass basis.

The fixture's `INPUT.FOR:253` reversal maps internal node 1 to native `x=-250`
(the lower outlet) and internal node N to native `x=0` (the inlet). Pinned
`src_reference/SOLUTE.FOR` uses species-indexed `cBot(jS)`/`cTop(jS)`.
For lower `kBotCh=-1` with outward flow `vN(1)<=0`, its active endpoint row is
`D(1)=-1`, `E(1)=1`, `F(1)=0`, i.e. `-C(1)+C(2)=0`. Inflow takes a different
branch with transport coefficients and the boundary concentration. This
lower source row is an adjacent-node equality in the discrete system. It is
not algebraically the same as a zero dispersive boundary flux in a
control-volume system, and the two must not be described as identical
discretizations.

The source's upper `kTopCh<0` row instead uses its `BN`, `DN`, and `FN`
transport coefficients (the equality row shown above is not used there); its
prescribed `cTop` terms are conditional on inflow. The Rust mode proposed here
is coordinate-mirrored at either endpoint because its finite-volume flux law
supports that contract. This does not claim parity with every Fortran upper
third-type branch.

For lower-boundary accounting the source forms old and new advective fluxes
using `alf=1-epsi`: on outflow, the old term is `alf*ConcOld(1)*vO(1)` and
the new term is `epsi*ConcNew(1)*vN(1)`; on inflow it uses `cBot(jS)` instead
of solved concentration. It accumulates `cvBot*dt` as lower mass. Upper
accounting has its own sign and branch logic (`CumCh(1)` subtracts `cvTop*dt`),
so it is not inferred by copying the lower equation. Fixture `SELECTOR.IN`
sets `epsi=0.5`, yielding old/new weighted endpoint fluxes in this reference
path.

Rust's accepted-water snapshot contains endpoint face fluxes at the accepted
end time but no start flux history or interval-integrated flux. The proposed
contract therefore uses backward-Euler one-point quadrature:

\[
E_{out}=\Delta t\,[q_L C_L^{n+1} - q_U C_U^{n+1}],
\]

including only endpoints configured as outlets. It uses the accepted snapshot
flux and solved endpoint concentration, matching the implicit endpoint
operator and the snapshot's `accepted_time = state.time + dt` convention. This
is an explicit integration approximation; it is not the Fortran old/new
`alf`/`epsi` quadrature, nor evidence that an end-time water flux represents
the full interval. Comparison must still check complete water history and
coefficient/sign mappings. No reference-measured solute flux is used as Rust
forcing.

## Validation, accounting, and state behavior

Keep existing finite/shape checks for water fluxes and validate outlet
directions for every species/end point during input validation, before
`begin_trial`. Inward means `qL>0` at a lower outlet or `qU<0` at an upper
outlet; return a typed `InwardFlowAtOutlet { species, endpoint, flux }`-style
error. Do not clamp a reversal, switch boundary type, or use external
concentration to manufacture an inflow rule. Existing trial rejection remains
the rollback guarantee for errors discovered during later species solves.

Add each known outlet term to species boundary exchange independently of
storage residuals. For a species with both existing prescribed total-flux
and outlet endpoints, the known signed term is
`P = dt * sum(total_flux_inward) + E_out`. With any concentration endpoint,
retain the current explicitly limited remainder inference only after
subtracting all known terms:
`R = storage_change - reaction_change - P`, and report boundary exchange as
`P + R`. Without a concentration endpoint, do not infer a remainder. This
keeps outlet mass independently checkable and prevents it from being hidden
inside the concentration-boundary residual. Existing prescribed total-flux
signs remain positive inward at either endpoint.

Preserve endpoint dissolved and sorbed storage and reaction terms. For
constant water content, the finite-volume example equations close as
`storage_change = known_boundary_exchange + reaction_change`, up to floating
point roundoff. The existing process scheme uses accepted/end water content
for old concentration mass in assembly while storage diagnostics use start
water content; the existing changing-water-content regression reports a
residual of `0.4`. This task does not repair that behavior or claim general
variable-theta closure. Its exact closure examples use constant theta. Preserve
the current nonnegative solved-state expectation where it applies, but qualify
it by nonnegative initial/source data and admissible existing prescribed-flux
modes; signed extraction can otherwise overdraw a pool. Do not add clipping or
broaden validation here.

## Independent rational examples

`tools/test_solute_outlet_contract.py` checks the explicit systems and
hand-entered solutions below using standard-library `Fraction`. The checker
evaluates each displayed matrix against its expected vector and checks storage,
exchange, and reaction sums independently; it has no numerical solver and
does not obtain expected values from Rust or Fortran.

These rational systems isolate solute algebra and do not represent
hydrologically conservative water histories; each full face-flux vector is
given here so the boundary and interior terms are reproducible. For three
nodes at coordinates `(0,2,4)`, endpoint half-cell widths are `(1,2,1)`.
Use constant `theta=(1/2,1/2,1/2)`, `dt=1`, interior face fluxes
`(-1/2,-1/2)`, and input `face_dispersion=(1,1)`. Since each spacing is 2,
the conductance at either interior face is `g=theta_face*D/dx=1/4`. For the
lower case use full face fluxes `(-1/2,-1/2,-1/2,0)`, lower
`AdvectiveOutflow`, and upper `TotalFlux { flux: 0 }`. The explicit row matrix
and expected new concentrations are

\[
A_L=\begin{bmatrix}5/4&-3/4&0\\-1/4&2&-3/4\\0&-1/4&5/4\end{bmatrix},
\quad C_L^{n+1}=\begin{bmatrix}1\\1/2\\1/4\end{bmatrix},
\quad b_L=\begin{bmatrix}7/8\\9/16\\3/16\end{bmatrix}.
\]

The independently reconstructed old vector is `(7/4, 9/16, 3/8)`. Weighted
inventories are `13/8` old and `9/8` new. Lower signed exchange is
`(-1/2)*1=-1/2`, so the inventory change is exactly the outlet exchange.
For the mirrored upper case use full face fluxes `(0,1/2,1/2,1/2)`, lower
`TotalFlux { flux: 0 }`, and upper `AdvectiveOutflow`. Reflecting the equations
gives upper outflow `qU=+1/2`,
`Cnew=(1/4,1/2,1)`, old `(3/8,9/16,7/4)`, and signed upper exchange
`-(1/2)*1=-1/2`, with the same inventories. Both systems retain nonzero
interior dispersion while the outlet has no dispersive term.

For the zero-flow case use all four face fluxes zero, both endpoints
`AdvectiveOutflow`, `theta=(1/2,1/2,1/2)`, `dt=1`, and `face_dispersion=(0,0)`.
The diagonal storage equations leave `(2,1/2,3/4)` unchanged and both
endpoint exchanges are exactly zero.

For a two-node process case, use coordinates `(0,2)` and unit control-volume
widths, `theta=(1/2,1/2)`, `dt=1`, full face fluxes `(-1/2,0,0)`, input
`face_dispersion=(0)`, lower `AdvectiveOutflow`, and upper
`TotalFlux { flux: 0 }`. Set bulk density `(1,1)`, linear equilibrium
coefficient `Kd=1`, and decay rate `1/2`. There is no interior advection or
dispersion. Local stored
mass is `3/2 C`. The explicit equations are `(11/4) C0new = 11/2` and
`(9/4) C1new = 9/4`, giving new `(2,1)` from old `(11/3,3/2)`. Total storage
changes from `31/4` to `9/2`; lower exchange is `-1`, and implicit reaction
change is `-(1/2)*(9/2)=-9/4`. Thus `-13/4 = -1 + -9/4` independently.

To expose the source-row discretization difference, replace only row 0 of the
lower finite-volume matrix above with `-C0+C1=0`, holding the other two
candidate rows and RHS fixed. The explicit replacement system gives
`(27/64,27/64,15/64)`, whose inventory is `3/4`; candidate finite-volume
inventory was `9/8`. The replacement system changes storage by `-7/8`, while
endpoint advective exchange evaluated from its solved concentration is only
`(-1/2)*(27/64)=-27/128`. This deliberately shows that the equality row cannot
be assumed to preserve the Rust half-cell inventory/accounting identity. It
isolates row replacement algebraically; it is not a recreation of the entire
Fortran stencil or a parity claim.

The exact examples have zero rational residual. For implementation checks
using `f64`, allow only accumulated floating-point roundoff, with a bound
scaled by the sum of absolute storage, exchange, reaction, and matrix terms
(for example `64 * f64::EPSILON * max(1, scale)`). This is an arithmetic
check, not a scientific comparison tolerance. Reversal tests require lower
positive flux and upper negative flux to return the typed error; exact zero is
accepted at both endpoints.

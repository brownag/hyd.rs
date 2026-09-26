# Derived Example 3 comparison case

This directory contains a **derived executable case motivated by** HYDRUS 1D
4.08 manual §11.3 (printed pp. 157–159). It is not a recovered manual input
deck and must not be reported as a manual reproduction. The original deck and
numeric CHAIN arrays are absent from the workspace.

`CASE.json` is the normalized record and provenance authority. It distinguishes
manual-transcribed facts from the derived domain, mesh, hydraulic material,
initial conditions, solver settings, sorption mapping, and input-format choices.
`SELECTOR.IN`, `PROFILE.DAT`, and `LEVEL_01.DIR` are the newly authored legacy
input bundle. `PROFILE.DAT` lists the inlet/top first, then nodes toward the
outlet; normalize its reversed native coordinate to flow distance as described
in `CASE.json`.

The second descriptive record after `Pcp_File_Version=4` in `SELECTOR.IN` is
authored format text. The pinned `BasInf` reader skips two records after the
version before reading a heading and the unit records; this record keeps those
fields aligned. It changes no model setting or scientific value.

`PROFILE.DAT` places node rows immediately after its `NumNP/ii/NS` header.
The pinned `NodInf` reader consumes no column-label line between that header
and the node records; the descriptive labels used while authoring this fixture
are therefore omitted from the executable file. Node rows and values are
unchanged.

The initial `lEquil` flag is true (the tenth basic-information flag in
`SELECTOR.IN`). `NodInf` runs before `ChemIn` and uses this initial flag to
select 13 numeric fields per node: ten node/temperature fields and three
dissolved concentrations, with no explicit sorbed columns. The initial pools
remain zero. `ChemIn` later derives its own equilibrium mode from the material
coefficients (`Fraction=1`, immobile water `0`). This is a source-compatible
format choice for this derived fixture, not a recovered manual-deck setting.
See `INPUT.FOR:31–32,248–252,1332–1339` and
`HYDRUS.FOR:196–199,271–277` in the pinned source.

The manual's retardation factors map to linear equilibrium sorption using
`R = 1 + rho_b Kd / theta`. With derived `theta=0.43` and `rho_b=0.8 g/cm3`,
`Kd=0.5375 cm3/g` gives `R=2` for NH4 and zero Kd gives `R=1` for NO2/NO3.
First-order rates are entered in both dissolved and sorbed-phase reaction
fields so the source chain acts on the same total species mass as the selected
Rust M5 contract. This coefficient mapping still requires review against the
run output and source behavior before parity can be claimed.

The derived saturated hydraulic setup targets `|q|=0.43 cm/hour`,
`theta=0.43`, and pore velocity `|q|/theta=1 cm/hour`. Dispersivity
`0.18 cm` with zero molecular diffusion targets effective `D=0.18 cm2/hour`.
These are targets inferred from the setup, not verified run results. The
reference run must check the realized water trajectory, boundary signs,
transport coefficients, successful solute output, and spatial mapping before
this fixture is used for paired comparison. The finite domain and discretization
are derived and differ from the infinite-domain analytical formulation.

## Historical shared-boundary blocker and remaining comparison gates

The historical 12.4.1 comparison diagnosis identified that the Rust M5 API then
stored one endpoint pair shared by every species, while the frozen input
specifies top concentrations NH4=1, NO2=0, NO3=0 (`SELECTOR.IN`, records 56–57;
`src_reference/SOLUTE.FOR` uses per-species `cTop(jS)`). Task 12.3.1 resolved
that representation blocker: `SoluteConfig::new_with_species_boundaries`
accepts a validated endpoint pair for each configured species, while
`SoluteConfig::new` retains shared-pair broadcast behavior. The contract tests
cover species-specific concentration values and coupled-chain accounting.

That API repair does not implement the frozen third-type outflow law. In pinned
`SOLUTE.FOR`, `MatSet` handles `kBotCh=-1` at the first endpoint (lines
810–819): when `vN(1)<=0`, it writes `D(1)=-1`, `E(1)=1`, `F(1)=0`, the
zero-gradient matrix row. Boundary accounting at lines 64–67 and 214–217
computes advective solute flux from the solved endpoint concentration and water
velocity when that boundary is outflow. These are parts of one direction-
dependent outflow condition, not a prescribed zero solute flux. Rust's
unchanged `SoluteBoundaryMode` in `solute.rs:60–68` and
`apply_endpoint_boundary_rows` at `solute.rs:1268–1292` provide prescribed
concentration or scalar total flux; neither expresses this zero-gradient row
with concentration-dependent outflow accounting. Feeding a measured reference
flux into `TotalFlux` would use the oracle as forcing, not establish
independent boundary parity.

Pinned `INPUT.FOR:253` reverses profile records into the solver arrays with
`n=NumNP-n+1`; thus internal endpoint index 1 maps to native `x=-250`, the
fixture's lower outlet, and index N maps to native `x=0`, its inlet. The
endpoint orientation is consistent with `kBotCh` at the outlet and `kTopCh`
at the inlet. Read-only `NOD_INF.OUT` records show water flux
`-0.4300 cm/hour` at printed nodes 1 (`x=-0`) and 1001 (`x=-250`) at 0, 50,
100, and 200 h. `solute3.out` reports final `cvBot=-0.24770E-09` at 200 h
(its header defines solute flux positive into the region). These sparse
observations do not independently establish the complete water history or
provide a Rust boundary input. The 0.43 cm/hour value remains a setup target,
not a verified species boundary-mass-flux input.

Task 12.4.1 stops at this gate. Do not replace the outflow law with zero total
solute flux, use oracle-fed forcing, change the fixture or public solver API,
or describe the result as paired parity. No Rust fixture run, parser, paired
residual report, numeric tolerance, or scientific parity claim is established.
Wave 4 and dependent Wave 5 remain incomplete. Any future comparison requires
a separately scoped representation of the outlet law.
Diagnostic numeric extraction, if later authorized, belongs in
`.development/12.4.1/`; do not commit source-derived golden arrays or infer
tolerances from Rust output.

The input deck was executed under task 12.2.1; its provenance and observed output
schema are recorded in `RUN.md`, with raw files kept under `.development/12.2.1/`.
No normalized paired result or comparison tolerances are established here.

## Outlet plan amendment (approved, 2026-09-26)

The audit above was recorded under comparison task 12.4.1. The approved
amendment moves comparison to **12.6.1** and tolerance review to **12.7.1**,
inserting **12.4.1** for outlet numerical design and **12.5.1** for implementation.
The earlier audit remains historical evidence; no outlet repair or comparison
has executed under this amendment. See [the current plan](../../../../planning/EXECUTION_PLAN_12.md).

## Wave 6 mapping audit (2026-09-26)

The accepted Rust outlet mode is physically mapped to the frozen lower outlet:
with the source-reconstructed water flux `q=-0.43 cm/hour` and endpoint 1 at
the bottom, it applies zero dispersive boundary flux and concentration-dependent
advective export. The Fortran lower equality row and its `epsi=0.5` old/new
boundary quadrature differ from Rust's finite-volume row and backward-Euler
accounting. The accepted outlet contract deliberately retains these methods;
record their effect in residuals and do not treat discrete identity as a gate.

The water forcing is reconstructible as a steady state from the frozen inputs
and pinned source. The 1001-node initial profile has `h=0` throughout, the
homogeneous material has `theta_s=Ks=0.43`, gravity cosine is 1, both water
boundaries are fixed at `h=0`, and sinks are disabled. In `WATFLOW.FOR`,
`Reset` uses equal neighboring conductivities (zero interior gravity RHS) and
zero storage change; `Veloc` then gives the same `q=-Ks=-0.43` at every node
and face. This is an exact discrete stationary solution for every interval,
not a claim that the interior state was printed at every step. Recorded output
corroborates it: `RUN_INF.OUT` has 2000 steps at `dt=0.1` through 200 h;
`T_LEVEL.OUT` has 2000 rows with both endpoint fluxes `-0.43` and constant
volume 107.5; `NOD_INF.OUT` has all 1001 nodes at 0, 50, 100, and 200 h, with
`h=0`, `theta=K=0.43`, and flux `-0.43`. The `RUN_INF.OUT`, `T_LEVEL.OUT`,
`NOD_INF.OUT`, `BALANCE.OUT`, and three `soluteN.out` files match their hashes
in `RUN.md`. Spatial output precision is limited to the printed fields; the
intermediate interior history is established from the input/source invariant
and checked by the full-profile snapshots, not directly logged per step.

The species order and inlet forcing also map without oracle-fed values. The
source input sets `cTop=[1,0,0]` for NH4/NO2/NO3 and applies `kTopCh=-1`.
At the upper endpoint the source retains its transport row and adds the
incoming advective term `-q*cTop`; with `q=-0.43`, the inward species fluxes
are `[0.43,0,0]`. Configure Rust's per-species `TotalFlux` with those values,
derived only from the water invariant and input concentrations. This is not a
reference-measured solute flux. The source and Rust spatial endpoint equations
remain different discrete forms and should be reported with the residuals.

Reaction and dispersion coefficients map to the selected Rust M5 processes.
With `Fraction=1`, `ImmobileWaterContent=0`, linear `Beta=1`, and no nonlinear
or optional process switches, `R=1+rho*Kd/theta`: `rho=0.8`, `Kd=0.5375`,
`theta=0.43` gives NH4 `R=2`, while zero Kd gives NO2/NO3 `R=1`. For each
species, source `GamL1` and `GamS1` both equal its chain rate (`0.005`, `0.1`,
`0` per hour), so the source term transfers the same rate of total dissolved
plus equilibrium-sorbed mass to the next species, as Rust's ordered chain
does. Source dispersion is `alpha*abs(q)/theta + Dw*TauW`; with alpha `0.18`,
`abs(q)/theta=1`, `Dw=0`, tortuosity off, and upstream weighting on, it maps
to effective `D=0.18 cm2/hour` without the disabled correction terms.

The mapping audit established sufficient independent physical inputs for
measurement; expected outlet/spatial/time-quadrature differences remain
residual sources, not equivalence claims. Raw and normalized outputs remain in
`.development/12.6.1/` unless artifact review authorizes committing them.

## Wave 6 scratch paired comparison

The approved diagnostic pairing is implemented by
`crates/hydrus-core/tests/solute_fixture_parity.rs` and
`tools/compare_solute_reference.py`. The ordinary targeted test emits Rust
profiles and independently integrated accounting to
`.development/12.6.1/rust-observations.json`; the comparator verifies the
current fixture inputs and every consumed raw output hash against `RUN.md`,
then writes `normalized-reference.json` and `residual-report.json` in the same
scratch directory. The parser validates `RUN_INF.OUT` and `T_LEVEL.OUT` at all
2000 steps and checks the four complete water/solute profiles.
For each Rust species, the comparator re-integrates the concentration profile,
checks the emitted inventory and cumulative balance residual within a bound
derived only from floating-point summation and JSON serialization precision,
and independently recomputes the cumulative balance from boundary and process
terms.

`OUTPUT.FOR` prints `Conc` and source `Sorb` separately. In this fixture the
printed `Sorb` fields are inactive nonequilibrium pools, all zero; they are
retained as such in normalized data. Equilibrium sorption is independently
derived as `Kd*C`, and total profile inventory is integrated as
`theta*C + rho*Kd*C`, matching the case's total-water-plus-sorbed storage.
`soluteN.out` cumulative `cvCh0`/`cvCh1` columns are `CumCh(3/4)`, accumulated
from the trapezoidal `q0/q1` source integrals in `SOLUTE.FOR`; the comparator
reports them separately from profile-derived inventory and `BALANCE.OUT`.

The exact commands passed are
`cargo test -p hydrus-core --test solute_fixture_parity`,
`python3 tools/compare_solute_reference.py reference/fortran_reference/solute/example3`,
`python3 tools/compare_solute_reference.py --self-test` (11 parser negatives plus a per-species balance regression),
`python3 -m py_compile tools/compare_solute_reference.py`, and `git diff --check`.
The report assigns no tolerance. Across the 50/100/200 h samples, maximum
absolute dissolved-profile residuals range from `0.00164` (NO2) to `0.0432`
(NH4); endpoint and row/quadrature differences remain measurable. Rust-minus-
reference profile-integrated total-inventory differences range from about
`-0.00506` to `0.0117` across species and times. These are diagnostic residuals,
not pass/fail limits or a claim of manual/analytical validation. Review of the
scratch artifacts and selection of any numerical acceptance tolerance remain
outside this task.

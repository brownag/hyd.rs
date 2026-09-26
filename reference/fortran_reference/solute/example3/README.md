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

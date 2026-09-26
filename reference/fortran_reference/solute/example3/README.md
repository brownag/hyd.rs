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

## Paired Rust comparison blocker

The verified Fortran run is available for read-only extraction, but the frozen
fixture cannot be paired faithfully through the current Rust M5 boundary API.
The input prescribes a third-type top boundary (`kTopCh=-1`) with a separate
concentration for each species: NH4=1, NO2=0, NO3=0 (`SELECTOR.IN`, records
56–57; the per-species `cTop(jS)` use is in `src_reference/SOLUTE.FOR`,
`MatSet`). Rust `SoluteBoundaries` contains only one upper and one lower mode;
each `Concentration` or `TotalFlux` mode carries one scalar shared by every
species (`crates/hydrus-core/src/solute.rs`, `SoluteConfig` and
`apply_endpoint_boundary_rows`). A shared concentration would inject every
species, while a shared flux cannot express the fixture's species vector.
Separate Rust configurations would lose the coupled ordered-chain reaction.

The derived record's 0.43 cm/hour Darcy flux is a setup target, not a verified
species boundary-mass-flux observation. It does not resolve this boundary
mapping mismatch. No paired Rust run, residual report, or scientific parity
claim is available for this fixture. Numeric reference extraction, if needed
for diagnosis, belongs in `.development/12.4.1/`; do not commit source-derived
golden arrays or infer tolerances from Rust output. A future scope decision
must address per-species endpoint boundary values before this comparison can
meet its acceptance criteria. The reference run remains evidence of successful
Fortran execution only.

The input deck was executed under task 12.2.1; its provenance and observed output
schema are recorded in `RUN.md`, with raw files kept under `.development/12.2.1/`.
No normalized paired result or comparison tolerances are established here.

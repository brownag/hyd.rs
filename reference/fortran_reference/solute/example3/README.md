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

The input deck has not been executed as part of task 12.1.1. No output files,
reference results, or tolerances are included here. Keep any generated run data
in the scratch location assigned by Orchestration 12 task 12.2.1.

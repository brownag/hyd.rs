# Feature and physics coverage matrix

## Capability matrix

Status uses **observed** for behavior found in the supplied source, **documented-only** for manual capability without confirmed source support, **rejected** for an explicit source refusal, and **uncertain** where the snapshot needs further audit. Inclusion language is a roadmap recommendation, not an implementation claim.

| Capability / selector or branch | Source and manual anchor | Observed status | Milestone / prerequisite | Candidate fixture | Scope disposition |
|---|---|---|---|---|---|
| van Genuchten hydraulic functions (`iModel=0`, `FK/FC/FQ/FH`) | `INPUT.FOR:551-625`; `MATERIAL.FOR:FK,FC,FQ,FH`; manual §2.3 | implemented | first water; hydraulic unit tests | dry-column infiltration | include first acceptance |
| Brooks-Corey / modified hydraulic families (`iModel=1..4`) | `MatIn`; `FK/FC/FQ`; manual §§2.3-2.7 | implemented | after baseline family, same curve tests | family pointwise curves | recommend later variant; review release scope |
| tabulated hydraulic properties (`iModel=nTabMod`, lookup/scaling) | `MatIn`, `GenMat`; `WATFLOW: SetMat`; manual §2.7 | implemented | hydraulic cache and parser | table interpolation/refinement | recommend after analytic family; explicit unsupported-selector error |
| Durner bimodal retention (`iModel=5`) | `MatIn:557`, `GenMat` | implemented | single-domain hydraulic validation | bimodal curve fixture | defer after first acceptance |
| dual-porosity water modes (`iModel=6,7` → `iDualPor=1,2`) | `MatIn:557-625`; `InitDualPor`; `WATFLOW:DualPor`; manual dual-domain chapters | implemented branch, coupling needs audit | validated single-domain water first | mobile/immobile exchange mass balance | required later |
| dual permeability (`iModel=8`) | `MatIn:560-563` | rejected by this program | none | rejection/diagnostic case | unsupported; never silently map to dual porosity |
| hysteresis (`iHyst`, `HYSTER:Hyst/PATH/DRAIN/WET/DRY`) | `MatIn`; `HYSTER.FOR`; manual §§2.6, 11.10 | implemented branch | first water baseline | drainage/imbibition loop | defer later, retained as required planned work |
| mesh, layers, inclined flow (`CosAlf`, `WLayer`) | `NodInf`; `Reset`; manual §§2.1, 7.1 | implemented | mesh/state contract | layered and inclined hydrostatic columns | include core water path |
| upper fixed head/flux, atmospheric switching, ponding | `INPUT:BasInf`; `WATFLOW:Shift`; `TIME:SetBC`; manual §§2.8, 7.4.3 | implemented | event-aligned water stepping | rain/ponding switch | include first acceptance |
| lower fixed head/flux, free drainage, water-table and seepage | `BasInf`; `Shift`; `qGWLF/FreeD/SeepF`; manual §7.4.3 | implemented branches | boundary matrix and signed-flux tests | lower-boundary variants | include supported variants; enumerate unsupported combinations |
| tile/drainage (`qDrain`, `FqDrain`, Hooghoudt/Ernst positions) | `WATFLOW:FqDrain`; manual drainage sections | implemented | drainage geometry and balance fixtures | five `iPosDr` cases | recommend after first acceptance |
| root uptake stress (`FAlfa`), compensation, growth, salinity | `SINK:SetSnk/FAlfa/FSAlfa/SetRG`; manual §2.2 | implemented branches | atmospheric/root contract | seasonal grass profile; salinity stress | include first acceptance for water stress/compensation; defer salinity coupling |
| solute species, advection/dispersion and reactions | `SOLUTE:Solute/Coeff/Disper`; `ChemIn`; manual Chapters 3, 8 | implemented branches | water state and transport matrix | sequential decay chain | later; explicit species/reaction matrix |
| equilibrium/nonlinear sorption | `SOLUTE:NEquil`; `ChemIn`; manual §3 / 11.4 | implemented branch | solute transport | nonlinear sorption case | later |
| nonequilibrium / mobile-immobile transfer | `ChemIn:lDualNEq`; `SOLUTE:MassTran`; manual §3 / 11.5 | implemented branch | solute domains and dual-porosity state | kinetic/nonequilibrium case | later; unsupported combinations rejected |
| colloid/bacteria filtration and deposition | `ChemIn:lBact/lFiltr`; `SOLUTE` deposition routines; manual chemistry chapters | implemented or source-present branch, extent uncertain | solute validation and parameter contract | bacteria/filtration fixture | optional recommendation; defer pending review |
| heat transport and temperature-dependent properties | `TEMPER:Temper/TempAdj`; `TempIn`; manual Chapter 4 / 11.11 | implemented branch | water baseline, heat coefficients | coupled water/heat case | later |
| vapor flow / energy balance | `lVapor`, `WATFLOW` vapor branches; `TEMPER`; `TIME:Evapor`; manual §§2.8, 11.11 | implemented branch | temperature/atmospheric event model | coupled water/vapor/heat | optional later recommendation; do not imply heat-only support |
| density-dependent flow/transport | `lDensity`; `WATFLOW:Reset/SetMat`; `INPUT:Init`; manual density discussion | implemented branch | fluid property and solute coupling | density-gradient test | optional recommendation; defer pending scope gate |
| meteorology, variable forcing, evaporation | `TIME:Meteo/MeteoInt/Evapor`; `lMeteo`; manual atmospheric chapter | implemented branch | atmospheric forcing adapter | cyclic atmospheric forcing | include atmospheric subset first; full meteo later |
| snow and interception | `TIME:Snow`, interception routines; `lSnow`; manual atmospheric sections | implemented branch | temperature/meteo state | snow/interception event deck | optional recommendation; defer pending review |
| inverse estimation | manual Chapters 5–6; no confirmed corresponding source workflow in named snapshot | documented-only / uncertain | parameter API and oracle evidence | inverse benchmark to be designed | defer; explicit user gate |
| special chemistry | manual Chapter 9; source chemical flags require detailed audit | documented-only / uncertain | solute schema and provenance | chemistry-specific fixture | defer or unsupported per feature audit; user gate |

## First acceptance

The confirmed first gate is single-domain water flow with atmospheric boundary behavior and root water uptake, including stress and compensation behavior needed by the water path. It requires at least one validated analytic hydraulic family, mesh/layer state, event-aligned upper forcing, supported lower boundary behavior, mass balance diagnostics, and candidate infiltration/atmospheric-root fixtures. Solute, heat, hysteresis, dual porosity, and optional chemistry are outside this gate. Unsupported selector or feature combinations must fail validation rather than fall back to a nearby model.

## Later required physics

After the simpler water milestone, add solute transport and heat/temperature coupling with their reaction, sorption, transfer, and coupling order explicit. Add hysteresis as a later water milestone with reversal history and drainage/imbibition validation. Dual porosity is required after validated single-domain physics; its mobile/immobile water and transfer state must be distinct from Durner bimodal retention and from rejected dual permeability. Vapor/energy may follow the coupled heat design where the scope gate accepts it.

## Optional scope recommendations

The source contains density, snow, interception, colloid/bacteria, inverse, and special-chemistry branches, but their presence does not commit the Rust product to release support. Recommend deferring each until a parameter schema, independent fixture, and compatibility decision exist. Treat full tabulated and alternate hydraulic families, tile drainage, vapor/energy, density, bacteria/filtration, inverse methods, and special chemistry as separately reviewable rows. Any unsupported combination should produce a typed capability error; no optional feature may silently use single-domain water behavior.

## Source and manual discrepancies

The 4.08 manual describes a finite-element formulation, while `WATFLOW.FOR:Reset` labels and implements a finite-difference-style half-cell stencil with arithmetic conductivity averaging. This is an audit finding requiring algebraic reconciliation and a targeted fixture; it does not authorize substituting a generic finite-element discretization. Manual capability descriptions also exceed what the snapshot proves, especially inverse and special chemistry. The snapshot lineage is 4.08 or a derivative and remains unverified; manual claims are not release parity.

## Evidence

- `HYDRUS_RESEARCH.md` §§2–4 and §6 distinguish manual scope from source evidence, identify the mixed-precision snapshot, and document the finite-difference/finite-element discrepancy.
- `INPUT.FOR:MatIn` maps models 0–7 and explicitly rejects model 8 dual permeability; `InitDualPor` and `WATFLOW` carry the separate dual-porosity branch.
- `WATFLOW:Reset/Shift/FqDrain`, `SINK:SetSnk/FAlfa/FSAlfa`, `SOLUTE:Coeff/NEquil/MassTran`, `TEMPER:Temper/TempAdj`, and `TIME:Snow/Meteo/Evapor` provide the branch anchors above.
- Manual validation candidates include 4.08 §§11.1–11.5, 11.10, and 11.11; capillary-rise and hydrostatic checks remain proposed independent fixtures. No numerical baseline or universal tolerance is claimed.

## Open decisions

1. Which alternate hydraulic families, tabulated models, drainage, vapor/energy, density, snow/interception, colloid/bacteria, inverse, and special-chemistry branches become supported release scope after the first gate?
2. Which exact source/manual version profile and fixture provenance will define compatibility claims?
3. What source combinations are valid for coupled solute, heat, hysteresis, and dual-porosity modes, and which must be rejected at configuration validation?
4. How should the Reset stencil be reconciled with manual FE terminology while preserving observed source behavior?

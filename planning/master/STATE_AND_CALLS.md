# HYDRUS lifecycle, calls, and owned state

This audit describes the supplied ten-file Fortran snapshot. Source behavior is evidence; proposed Rust domains are design obligations, not implemented behavior. The source snapshot's exact 4.08 identity and a numerical baseline remain unverified.

## Lifecycle

| Stage | Observed path and state transition | Evidence |
| --- | --- | --- |
| Process setup | Windows signal/FPE handlers, command-line or `LEVEL_01.DIR` path, options and input units opened. | `HYDRUS.FOR:99-119`, `:889-932` |
| Initialization | `Init` establishes flags and counters; `Selector.in`, `Profile.dat`, atmospheric and optional module inputs populate mesh, materials, time, forcing, sinks, heat and chemistry. | `HYDRUS.FOR:122-140`, `:157-295` |
| Lookup/precompute | `GenMat` creates hydraulic tables; `SetMat` evaluates nodal coefficients; optional `Hyst`, dual-porosity and transport initialization establish coupled arrays. | `HYDRUS.FOR:215-236`; `INPUT.FOR:732-752`; `MATERIAL.FOR` `GenMat` |
| Accepted-step loop | Atmospheric/snow/interception/root forcing is updated, then `WatFlow`; optional energy, velocity, solute, output and accounting work follows. | `HYDRUS.FOR:303-390`, `:399-548` |
| Trial water solve | `WatFlow` repeats `SetMat`/hysteresis, `Reset`, `Shift`, `Gauss`, convergence tests and timestep retry. | `WATFLOW.FOR:48-168` |
| Commit | `Update` copies converged values into old/history arrays, extrapolates next heads, and changes saturated atmospheric boundary mode. | `WATFLOW.FOR:1079-1125` |
| Output/termination | `NodOut`, `SubReg`, balance/output routines flush files; normal completion calls `CloseOutput`; nonconvergence and input/output errors call `ErrorOut`/stop. | `HYDRUS.FOR:439-777`; `:784-882` |

Termination includes user interrupt, abort, FPE, explicit stop after ten failed water convergence attempts, input/open/read failures, dimension and output errors. Signal handlers call `CloseFiles`; `CloseOutput` writes `end` markers conditionally (`HYDRUS.FOR:438-446`, `:772-777`, `:889-957`).

## Call hierarchy

| Caller/phase | Direct calls or references | State relationship / unresolved edge owner |
| --- | --- | --- |
| `HYDRUS` setup | `Init`, `BasInf`, `NodInf`, `MatIn`, `InitW`, `GenMat`, `SetMat`, `Hyst`, `InitDualPor`, `TmIn`, `MeteoIn`, `RootIn`, `TempIn`, `ChemIn`, `OpenSoluteFiles`, `SetBC` | Input adapter owns parsing; simulation context owns resulting state. `GETARG`, `NARGS`, `SIGNALQQ`, `MSFLIB` are platform edges; future portability owner: host/runtime adapter. |
| `HYDRUS` forcing | `Meteo`, `Snow`, `IntercepS` (commented), `DailyVar`, `SinPrec`, `Evapor`, root/sink routines, `TempAdj`, chemistry BCs (some commented) | Forcing and process scheduler own external forcing; commented calls are documented optional edges, investigation owner: feature audit/roadmap. |
| `HYDRUS` solve | `WatFlow`; optional `UpdateEnergy`, `Veloc`, `Solute`, `Temper`, `Update` | Coupling coordinator owns ordering and accepted/trial boundary. `Veloc`, `Solute`, and some energy calls are commented or conditional; owner: numerical/API design task. |
| `WatFlow` | `DualPor` (commented), `SetMat`, `Hyst`/`Hyster`, `Reset`, `Shift`, `Gauss`, hydraulic functions `FH`, `Fqh`, `FqDrain`, `fRo` | Solver workspace owns trial coefficients and matrix; process modules own their coupled arrays. Dynamic model selector `iModel` dispatches hydraulic functions; owner: hydraulic design task. |
| `HYSTER` | `HysterIni`, `Path`, `UpdateHyst`, `HAWPATH`, `DRAIN`, `WET`, `DRY` and related helpers | Hysteresis history must be per simulation, not process-global. COMMON access is an observed global edge; owner: state migration task. |
| Transport/heat | `SOLUTE.FOR` routines (`BanSol`, mass/transfer routines), `TEMPER.FOR` (`TempCap`, `SetUpHeat`, `TempAdj`, `BanSol`) | Shared names and flags require explicit coupling contract; owner: later transport/heat design. |
| Output | `NodOut`, `Profil`, `SubReg`, `CloseOutput`, `CloseFiles` | Output sink owns file handles and serialization; no solver routine may own filesystem state. |

All ten source files are in scope: `HYDRUS.FOR`, `INPUT.FOR`, `MATERIAL.FOR`, `WATFLOW.FOR`, `HYSTER.FOR`, `SINK.FOR`, `TIME.FOR`, `SOLUTE.FOR`, `TEMPER.FOR`, and `OUTPUT.FOR`. Function/library references (`RTime`, date/time APIs, `MSFLIB`, `GETARG`, `NARGS`, `SIGNALQQ`) are unresolved external edges and each is assigned to the host/runtime adapter owner above. Commented calls remain unresolved feature edges and are assigned to the feature audit or later transport design owners rather than omitted.

## State inventory

| Proposed ownership domain | Observed arrays/scalars and assumptions | Evidence |
| --- | --- | --- |
| Mesh/profile | `x`, `MatNum`, `LayNum`, `NumNP`, layer/material counts, `CosAlf`, geometry/drainage coordinates | `HYDRUS.FOR:48-95`; `WATFLOW.FOR:173-220` |
| Hydraulic model/tables | `ParD`, `ParW`, `thr`, `ths`, `hSat`, `hTab`, `ConTab`, `CapTab`, `TheTab`, `Con`, `Cap`, `ConSat`, `Ah`, `AK`, `ATh` | `HYDRUS.FOR:72-95`; `MATERIAL.FOR` hydraulic routines |
| Boundary/forcing | `KodTop/KodBot`, `rTop/rBot`, `hTop/hBot`, `hCritA/hCritS`, `WLayer`, `TopInf`, seepage/drainage/GWL parameters, meteorological and root arrays | `HYDRUS.FOR:303-365`; `WATFLOW.FOR:359-430` |
| Water solver trial/accepted | `hNew`, `hOld`, `hTemp`, `ThNew`, `ThOld`, `ThEq`, `P/R/S`, boundary pivots, `Iter`, `ItCum`, `ConvgF`, `dt` | `HYDRUS.FOR:72-95`; `WATFLOW.FOR:48-168`, `:308-354` |
| Sink/root | `Sink`, `SinkIm`, `Beta`, `xRoot`, root growth and reduction flags | `HYDRUS.FOR:72-95`, `:346-359`; `SINK.FOR` |
| Hysteresis | `/properties/`, `/glob/` (`RHSW`, `RASW`, `SARW`, `RSWAW`, `PHSW`, `MPSW`, `JJH`, `IPSW`), `/local/` scalars and `Kappa`; seven reversal slots and 1001-node bounds | `HYSTER.FOR:6-20`, `:124-145`, `:652-675` |
| Solute/heat/vapor | `Conc`, sorbed pools, `vOld/vNew`, `TempO/TempN`, vapor contents/velocities, transfer and retardation arrays | `HYDRUS.FOR:78-95`; `WATFLOW.FOR:22-33`; `TEMPER.FOR`; `SOLUTE.FOR` |
| Accounting/output | `CumQ`, `SubVol`, `Area`, `CumCh`, `cCumA/cCumT`, `ItCumO`, print levels and units | `HYDRUS.FOR:72-95`, `:418-465`; `OUTPUT.FOR` |

The three COMMON blocks are all in `HYSTER.FOR` and are mutable shared state. `DATA lStopConv,iNonConv` and `DATA iUnit` initialize process control and file-unit state (`HYDRUS.FOR:97`, `:134`); `MATERIAL.FOR` and `WATFLOW.FOR` also contain DATA constants. No explicit `SAVE` was found in the audit set, so compiler/static lifetime assumptions require an isolated implementation investigation owner: state migration task.

## Precision and limits

The main program declares many field arrays as default `REAL`, while time/matrix quantities are double precision (`HYDRUS.FOR:65-95`). `Gauss` explicitly converts solved values with `sngl` (`WATFLOW.FOR:308-353`); hysteresis uses implicit `real*8` internals but REAL argument arrays (`HYSTER.FOR:9-20`). An all-`f64` port is therefore a deliberate behavior change requiring calibration.

Declared storage bounds include 1,001 nodes, 20 materials, 100 tables/observations, 11 solute slots, 1,000 print/growth slots, ten balance regions, and hysteresis arrays of 1,001 nodes (`HYDRUS.FOR:48-95`; `HYSTER.FOR:17-18`). These are storage limits, not proof every count is supported. Fortran indices are one-based; coordinates use signed `x` and `CosAlf`, and flux signs are altered by gravity, density, centrifuge, seepage and drainage branches (`WATFLOW.FOR:190-220`, `:372-430`). Units are converted through input configuration and `xConv/tConv`; the ownership contract must retain units and sign conventions at boundaries.

## Rollback

`WatFlow` keeps `hOld`, `ThOld`, and related histories while iterating on `hNew`; `hTemp` captures the prior trial vector before `Gauss` (`WATFLOW.FOR:86-103`). `Reset` mutates trial coefficients and water contents; `Shift` mutates boundary-related flux/state. On convergence, `Update` promotes `hNew`/`ThNew`, velocities, vapor, immobile water, and temperature into old state (`WATFLOW.FOR:1095-1116`). On failed convergence, the caller can retry with adjusted timestep through the surrounding `HYDRUS` loop, but the exact retry control and all side effects must be isolated and tested before commit.

Rollback must include coupled histories (`TempO`, `vOld`, vapor, immobile water, concentrations/sorbed pools where enabled), hysteresis reversal arrays and `Kappa`, boundary mode/ponding/seepage state, and accounting counters (`ItCum`, cumulative fluxes). `UpdateHyst` mutates reversal state after convergence (`HYSTER.FOR:666-675`), so it cannot run on a rejected trial. Output and balance writes must occur only at accepted-step boundaries. This is a design obligation inferred from mutation order; source behavior for every retry path remains an open investigation.

## Evidence

- `HYDRUS_RESEARCH.md §6` records the partial lifecycle, three COMMON blocks, mixed precision, and fixed bounds; this document expands those findings into source-anchored relationships.
- `HYDRUS.FOR:122-236` anchors initialization and table setup; `:399-548` anchors the main coupled loop; `:784-957` anchors errors, normal close, signals, and file teardown.
- `WATFLOW.FOR:3-17` exposes the solver state contract; `:48-168`, `:173-220`, `:308-354`, `:359-430`, and `:1079-1125` anchor trial solve, assembly, boundary shifts, and commit.
- `HYSTER.FOR:6-20`, `:124-145`, and `:652-675` anchor COMMON declarations, initialization, and post-convergence history mutation.
- The ten-file inventory and unresolved-edge labels above are an audit result; no numerical baseline or verified release claim is made.

## Open decisions

1. The host/runtime owner must replace `MSFLIB`, signal handlers, date/time, command-line, and file-unit behavior with a portable error and cancellation contract.
2. The state migration owner must determine whether any compiler static lifetime is relied upon despite no explicit `SAVE`, and define per-simulation ownership for COMMON and DATA state.
3. The numerical design owner must specify accepted/trial snapshots for every optional process and prove accounting/output writes are rollback-safe.
4. The hydraulic/feature owners must resolve `iModel` dispatch and commented `DualPor`, `Veloc`, chemistry, and meteorological edges before implementation scope is frozen.
5. Unit/sign and coordinate conventions, including `xConv`, `tConv`, gravity orientation, drainage and seepage signs, require fixture-backed confirmation.

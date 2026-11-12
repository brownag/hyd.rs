# M6 heat source contract (heat-only)

Status: source contract for Orchestration 24, task 24.1.1. Heat/temperature only. Vapor and energy balance are out of scope (D1, D7). Every claim cites the pinned Fortran in `src_reference/` (read directly on 2026-11-11). Line numbers are file lines.

## Source map

| Routine or site | Location | Heat state carried | Scope |
|---|---|---|---|
| `Temper` | `TEMPER.FOR:5-76`; arguments `:5-10` | `x`, `dt`, `t`, `MatNum`, `TempO`, `TempN`, `TPar`, `Ampl`, `B,D,E,F`, `vOld`, `vNew`, `ThOld`, `ThNew`, `Cap`, `Cond`, `Sink`, `tPeriod`, `kTopT`, `tTop`, `kBotT`, `tBot`, `HeatFl`, `xConv`, `tConv`, `iCampbell` | In scope |
| `Temper` vapor and energy arguments | `TEMPER.FOR:5-10` | `lVapor`, `ThVOld`, `ThVNew`, `vVOld`, `vVNew`, `g0`, `lEnBal`, `dtMaxT`, `iTemp` | OUT OF SCOPE (vapor, energy) |
| `TempCap` | `TEMPER.FOR:269-309`; called at `:37` (Level 1, `ThOld`, `vOld`) and `:40` (Level 2, `ThNew`, `vNew`) | `Cap`, `Cond` from `TPar`, `Theta`, `Veloc`; `iCampbell` branch `:279-300` | In scope |
| `SetUpHeat` | `TEMPER.FOR:77-179`; called at `:43` per level | Boundary rows of `B,D,E,F`; `kBotT`, `kTopT`, `tBot`, `tTopA`, `HeatFl` | In scope, limited to `## Supported` kinds |
| `BanSol` | `TEMPER.FOR:53` | Solves `B,D,E,F`; result to `TempN` at `:54-56` | In scope (existing tridiagonal path) |
| `TempAdj` | `TEMPER.FOR:180-268`; called at `:48` under `lVapor` | Vapor terms in `B,D,E,F` | OUT OF SCOPE |
| `xLatent` | `TEMPER.FOR:310-323` | Volumetric latent heat (vapor only) | OUT OF SCOPE |
| `WatFlow` call | `HYDRUS.FOR:419` | Passes `TempN`, `TempO`, `iTemp`, `lVapor`, `lWTDep`; water step | Water step. Heat effect in `## Rejected` (`lWDep`) |
| `UpdateEnergy` call | `HYDRUS.FOR:462` (under `lEnBal`, `:451-462`); defined `TIME.FOR:1166` | Evaporation and surface energy terms | OUT OF SCOPE (energy) |
| `Veloc` call | `HYDRUS.FOR:473-476` | Passes `TempN` and `vNew` | Water step |
| `Temper` call | `HYDRUS.FOR:490-491` (under `lTemp`) | Heat state from accepted water result | In scope |
| `Update` call | `HYDRUS.FOR:695` | `TempO=TempN` (`WATFLOW.FOR:1115`); `vOld=vNew` under `lTemp` (`WATFLOW.FOR:1109`) | In scope |
| `TempIn` | `INPUT.FOR:1205-1245` | Reads `TPar`, `Ampl`, `tPeriod`, `iCampbell`, `kTopT`, `tTop`, `kBotT`, `tBot` (`:1236`) | In scope (input) |

## Coupling order

Per-run setup, in this order (`HYDRUS.FOR`):

1. `Init` at `:123` sets defaults, including `lVapor=.false.` and `lEnBal=.false.` (`INPUT.FOR:1665`, `:1673`).
2. `BasInf` at `:159` reads `lTemp`, `lWDep` (`INPUT.FOR:31`) and, for `iVer>=4`, the header `lVapor` (`INPUT.FOR:38`). The header value overrides the default.
3. `MeteoIn` at `:244` (only when `lMeteo`, `:243`) can set `lEnBal` from unit 33 (`INPUT.FOR:966`). The effective `lEnBal` is known only after this call.
4. `TempIn` at `:265` reads `kTopT`, `tTop`, `kBotT`, `tBot` (`INPUT.FOR:1236`). If `lEnBal`, it forces `kTopT=-1` (`HYDRUS.FOR:268`).

Per time step:

1. `iTemp=0` at `:402`.
2. Water step: `WatFlow` at `:419`. Its Picard retry resets `TempN=TempO` (`WATFLOW.FOR:139-140`).
3. Water velocity: `Veloc` at `:473-476` with the water result `hNew` and `TempN`.
4. Heat step: `Temper` at `:491`, on the accepted water result (`vNew`, `ThNew`). `TempCap` at `:37` and `:40`, `SetUpHeat` at `:43`, `BanSol` at `:53`.
5. Accepted state: `Update` at `:695` sets `TempO=TempN` (`WATFLOW.FOR:1115`) and `vOld=vNew` (`WATFLOW.FOR:1109`).

Heat reads water and water never reads heat in M6 (D9). This holds only when `lWDep=false` and `lVapor=false` (see `## Rejected`). `Update` at `:695` is the handoff that makes heat state the previous step for the next water step.

## Variables and precision

Typing basis: `HYDRUS.FOR` and `TEMPER.FOR` have no `implicit` statement. Default Fortran rules apply: names starting A-H or O-Z are `REAL` (single precision), names starting I-N are `INTEGER`. Explicit double precision: `t` (`HYDRUS.FOR:65-67`, `TEMPER.FOR:12`), `P,R,S,Q` (`HYDRUS.FOR:65`), `B,D,E,F` (`TEMPER.FOR:12`, `SetUpHeat`). The Rust column is the D8 choice: `f64` for every real or double value, `i32` for integer codes, `bool` for logicals, `usize` for indices.

| Variable | Fortran type | Unit as the source uses it | Rust type |
|---|---|---|---|
| `t` | double precision | time, source time unit | `f64` |
| `dt` | REAL (implicit) | time step, source time unit | `f64` |
| `x(N)` | REAL (implicit) | node coordinate, source length unit (`xConv`) | `f64` |
| `TempO`, `TempN` | REAL (implicit) | temperature; Celsius-based (`OUTPUT.FOR:449` adds 273.15) | `f64` |
| `TPar(10,NMat)` | REAL (implicit) | per-material heat parameters (`TempIn`, `INPUT.FOR:1216`) | `f64` |
| `Cw` = `TPar(9,1)` | REAL (implicit) | volumetric heat capacity of water | `f64` |
| `Cap(N)` | REAL (implicit) | volumetric heat capacity | `f64` |
| `Cond(N)` | REAL (implicit) | thermal conductivity, scaled by `xConv/tConv^3` (`:300`) | `f64` |
| `Sink(N)` | REAL (implicit) | sink term, water-side | `f64` |
| `vOld(N)`, `vNew(N)` | REAL (implicit) | Darcy flux, source flux unit | `f64` |
| `ThOld(N)`, `ThNew(N)` | REAL (implicit) | volumetric water content | `f64` |
| `B,D,E,F(N)` | double precision | matrix rows and RHS | `f64` |
| `HeatFl` | REAL (implicit) | top heat flux, energy per area per time | `f64` |
| `tTop`, `tBot` | REAL (implicit) | boundary temperature, source temperature unit | `f64` |
| `tTopA` | REAL (implicit) | boundary temperature after sinusoidal modulation | `f64` (only under `tPeriod<=0`, see `## Rejected`) |
| `Ampl`, `tPeriod` | REAL (implicit) | sinusoidal amplitude and period (`TempIn`, `INPUT.FOR:1220-1225`) | `f64` |
| `kTopT`, `kBotT` | INTEGER (implicit, K) | boundary kind codes (sign class, see `## Supported`) | `i32` |
| `iCampbell` | INTEGER (implicit) | conductivity model switch (`TempIn`) | `i32` |
| `iTemp` | INTEGER (implicit) | energy loop counter (out of scope) | `i32` |
| `lEnBal`, `lVapor` | LOGICAL | flags | `bool` |
| `xConv`, `tConv` | REAL (implicit) | unit conversion factors | `f64` |

Single-precision conversion points. Each one is recorded here and must be tested, not silently changed (D8).

| Location | Operation | Effect |
|---|---|---|
| `TEMPER.FOR:19` | `PI=3.141592654` stored as REAL | Pi is single precision |
| `TEMPER.FOR:21` | `sngl(t)` inside the `sin` argument, `sin` of REAL | Double `t` is rounded to single before the sinusoid (rejected path, see `## Rejected`) |
| `TEMPER.FOR:26-27` | `HeatFl` built from REAL `Cw`, `tTopA`, `vNew`, `vOld` | Flux is single precision |
| `TEMPER.FOR:55` | `TempN(i)=sngl(F(i))` | Double solution is rounded to single on store |
| `TEMPER.FOR:279` | `Cap` sum of REAL terms | Capacity is single precision |
| `TEMPER.FOR:286`, `:295`, `:300` | `amax1` on REAL conductivity terms | Conductivity is single precision |
| `SetUpHeat` coefficient expressions | All operands REAL (`dx`, `dt`, `Cap`, `Cond`, `Cw`, `vNew`, `vOld`, `Sink`), e.g. `dx/2./dt*Cap(1)` at `SetUpHeat` bottom branch | Coefficients are computed in single precision, then stored in double `B,D,E,F` |
| `TEMPER.FOR:281-284` | `Cap(i)==0` writes a message, reads from stdin, then `stop` | Must map to a typed runtime error, not a stop, in the Rust port |

Input values (`TPar`, `Ampl`, `tTop`, `tBot`, `tPeriod`) are read into REAL and are single precision as the source has them.

## Supported

Heat-only selection. Every item must hold; each is checked on the effective value (see Coupling order for `lVapor` and `lEnBal`):

| Flag or setting | Required value | Source |
|---|---|---|
| `lTemp` | `true` (heat on) | `INPUT.FOR:31`, `HYDRUS.FOR:264`, `:490` |
| `lVapor` | `false` | `INPUT.FOR:38`, `:1665`; `TEMPER.FOR:48` |
| `lEnBal` | `false` (after `MeteoIn`) | `INPUT.FOR:966`, `:1673`; `HYDRUS.FOR:268` |
| `lWDep` | `false` | `INPUT.FOR:31`; `WATFLOW.FOR:404-405`, `:458-464` (see `## Rejected`) |
| `iDualPor` | `0` | `HYDRUS.FOR:278` |
| `iCampbell` | `0` (default conductivity) or `1` (Campbell conductivity) | `TEMPER.FOR:279-300`; both branches are in scope for M6 (user decision 2026-11-11) |
| `iModel` | `0` (van Genuchten) | `INPUT.FOR:552`; reuse `WaterProductConfig::new` |
| Domains | one | `WaterProductConfig::new` (`water_solver.rs:24`) |

Top boundary kinds, `kTopT`. The source tests only the sign. Integer codes are manual-pending (the input manual is not in the repo).

| Kind | Sign class | Code value | Source | Heat-only behavior |
|---|---|---|---|---|
| Dirichlet (constant temperature) | `kTopT > 0` | manual-pending | `SetUpHeat` `:144-147` (`B(N)=0`, `D(N)=1`, `F(N)=tTopA`) | Fixes `TempN(N)=tTop` |
| Flux (heat flux from water) | `kTopT < 0` | manual-pending | `SetUpHeat` `:148-162`; `Temper` `:25-27` (`HeatFl=Cw*tTopA*(vNew+vOld)/2`) | Flux row with `HeatFl`; only `kTopT=-1` is named in source, and only inside `TempAdj` (vapor, `TEMPER.FOR:255`) |

Bottom boundary kinds, `kBotT`:

| Kind | Sign class | Code value | Source | Heat-only behavior |
|---|---|---|---|---|
| Dirichlet (constant temperature) | `kBotT > 0` | manual-pending | `SetUpHeat` `:87-90` (`D(1)=1`, `E(1)=0`, `F(1)=tBot`) | Fixes `TempN(1)=tBot` |
| Flux (heat flux from water) | `kBotT < 0` | manual-pending | `SetUpHeat` `:91-107` (uses `tBot*Cw*(vNew(1)+vOld(1))/2` in the Level 1 RHS) | Flux row with `tBot` |
| Zero-gradient | `kBotT == 0` | manual-pending | `SetUpHeat` `:108-111` (`else` branch: `D(1)=-1`, `E(1)=1`, `F(1)=0`, so `T(1)=T(2)`) | Zero temperature gradient at the bottom. Reading confirmed on 2026-11-11. Physical semantics: see Question 1 |

`kTopT == 0` is not supported; see `## Rejected`.

## Rejected

Every combination not listed in `## Supported` is rejected. The rule is a catch-all. Each row names the proposed typed error for 24.2.1; the names are proposals and are not implemented in this task.

| Case | Reason (Fortran) | Proposed typed error (24.2.1) | Basis |
|---|---|---|---|
| `lVapor = true` | Vapor terms (`TempAdj` `TEMPER.FOR:48`, `:180-268`; vapor output `HYDRUS.FOR:174`) are out of scope | `VaporNotSupported` | D1, D7 |
| `lEnBal = true` (including the value set from `MeteoIn`) | Forces `kTopT=-1` (`HYDRUS.FOR:268`); energy path (`UpdateEnergy`, `HYDRUS.FOR:462`, `TIME.FOR:1166`); `dtMaxT` and `iTemp` loop (`TEMPER.FOR:57-75`, `HYDRUS.FOR:496`) | `EnergyBalanceNotSupported` | D1, D7 |
| `iDualPor != 0` | Dual porosity: `HYDRUS.FOR:278` (`lEquil=.false.`), `WATFLOW.FOR:1109` and `ThOldIm` path | `DualPorosityNotSupported` | D7 |
| `iModel != 0` | Only van Genuchten in M6 | Existing `HydraulicModelSelector` error | `WaterProductConfig::new` |
| Domain count `!= 1` | Single-domain product | Existing `UnsupportedConfiguration` | `WaterProductConfig::new` |
| `lWDep = true` | Temperature dependence of water flow: `ConLT` and `TempN` enter water flux (`WATFLOW.FOR:224-227`, `:257-263`, `:287-290`, `:404-405`) and surface tension and density (`:458-464`). This is a heat-to-water feedback that D9 forbids. Beyond D7's list. Added at the coordinator's direction | `TemperatureDependentWaterNotSupported` | D9 (addition beyond D7) |
| `kTopT == 0` | `SetUpHeat` (`TEMPER.FOR:144-162`) has `if(kTopT.gt.0)` and `else if(kTopT.lt.0)` and no `else`. Node N's row is never assigned, so the source uses a stale row. Not reproduced | `TopBoundaryKindNotSupported` | Coordinator decision, Question 2 answer |
| `tPeriod > 0` (sinusoidal top boundary) | `tTopA = tTop + Ampl*sin(...)` (`TEMPER.FOR:19-22`) with `sngl(t)` (`:21`). Not in the M6 selection. Coordinator decision | `SinusoidalTopNotSupported` | Coordinator decision, Question 3 answer |
| `kTopT` or `kBotT` outside the Supported sign classes | Not listed in `## Supported` | `BoundaryKindNotSupported` | Catch-all rule |
| Runtime: `Cap(i) == 0` | `TEMPER.FOR:281-284` prints, reads stdin and calls `stop`. This is a runtime error, not a config rejection | `HeatCapacityZero` | Port must return an error, not stop |

Not a rejection: `lTemp = false` means heat is off, so no heat code runs.

## Tolerances

Method: ADR 0006 (per-field tolerances from source precision). Values approved by the user on 2026-11-11 (option: "Derived from REAL precision"). Each value is derived from the single-precision source basis, not chosen from observed errors.

| Field | Units (source) | Precision basis | Tolerance |
|---|---|---|---|
| `TempN` (nodal temperature, accepted step) | degC (`OUTPUT.FOR:449`) | REAL store (`TEMPER.FOR:55`); about 2.4e-6 degC ulp at 25 degC | Absolute 2e-4 degC (about 100 x the ulp) |
| `HeatFl` (top flux) | energy per area per time, scaled by `xConv`, `tConv` | REAL (`TEMPER.FOR:26-27`) | Relative 1e-5, absolute floor 1e-12 |
| `Cap`, `Cond` (material capacity and conductivity) | volumetric heat capacity; conductivity scaled by `xConv/tConv^3` (`:300`) | REAL (`:279`, `:286`, `:300`) | Relative 1e-5, absolute floor 1e-12 |
| Matrix coefficients `B,D,E,F` before solve | units follow `Cap`, `Cond`, `dt` | Computed in REAL inside `SetUpHeat`, stored double | Relative 1e-5, absolute floor 1e-12 |
| Balance residual per step | energy per area | Derived from the storage integral and boundary terms | Absolute, at most 1e-6 of the total storage scale of the step |
| Accepted-step snapshot (`TempO`, `vOld`) | copies (`WATFLOW.FOR:1109`, `:1115`) | Exact copy, no arithmetic | Exact equality (not a tolerance) |
| Boundary kind and flags | integers and logicals | Not floating point | Exact equality (ADR 0006: discrete states exact) |

Zero handling: where a reference value is zero, the absolute floor applies (1e-12). Tolerance values are for comparison only; no code or fixture may tune them to pass.

**Gate closed 2026-11-11:** user approved the values above. Wave 5 may use them.

## Not claimed

- Vapor flow: `lVapor`, `TempAdj`, `xLatent`, `ConVh`, `ConVT`, `ThVOld`, `ThVNew`, `vVOld`, `vVNew`, vapor output (`NOD_INF_V.OUT`). Goal G19.
- Energy balance: `lEnBal`, `UpdateEnergy`, `Evap`, `Rn`, `Rns`, `Rnl`, `dtMaxT` under `lEnBal`, and the `iTemp` loop (`HYDRUS.FOR:496`). Goal G20.
- Heat-to-water feedback (`lWDep`, `lWTDep`, `ConLT` in water flux). Rejected in M6.
- Hysteresis (M7), dual porosity (M8), root uptake with heat coupling, chemistry.
- Universal accuracy or any universal tolerance. Acceptance is capability-scoped (M5V handoff).
- Any numeric tolerance (see `## Tolerances`).
- Correctness of boundary kinds not in `## Supported`, or of integer boundary codes (manual-pending).

## Questions for the user

1. `kBotT == 0` zero-gradient (`TEMPER.FOR:108-111`) is in Supported on the basis of the source reading. Confirm the physical intent (zero temperature gradient at the bottom) before 24.2.1 implements it. Also provide the input-manual integer codes for every boundary kind, or confirm that "manual-pending" stays until a manual is supplied.
2. Numeric tolerances (`## Tolerances`) must be reviewed by the user before 24.5.2 can run. Confirm the review gate.

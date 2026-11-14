# Vapor capability coverage and follow-on goal

Status: **G19 first increment complete with exception, 2026-11-12 ([acceptance](M6_VAPOR_ACCEPTANCE.md)). G22 open.** Planning matrix of 2026-11-11 retained; Status column added. G22.4 rows decided 2026-11-13 (goal stays Circled with open items; see [G22.4 acceptance](M6_VAPOR_G22_4_ACCEPTANCE.md)). G22.2 water-flux rows decided at GATE W1, 2026-11-13 (see [G22.2 acceptance](M6_VAPOR_G22_2_ACCEPTANCE.md)); atmospheric (row below) is G22.3, not decided here.
Authority: [orchestration 25 decisions D2/D3/D5/D11](../EXECUTION_PLAN_25.md).
Goals: G19 is the bounded first increment; **G22 is complete vapor coverage**.
Accepting G19 does not complete G22.

| Capability / interaction | First increment (25) | Remaining coverage / gate | Status (2026-11-12) |
|---|---|---|---|
| Vapor conductivity, content/capacity, velocities, latent/sensible heat | Planned | Every selected path requires kernel, coupled and independent evidence | Accepted with exception (first increment); content is pinned-kernel evidence only; 18 isothermal flux rows explained, not matched |
| Two water→heat passes and atomic acceptance/retry | Planned | Remains required in follow-ons | Accepted (first increment); `vapor_driver_order`, inline driver tests |
| Default thermal conductivity (`iCampbell=0`) | Planned | Nonuniform-temperature source fixtures required | Accepted for the first-increment fixtures. Nonuniform temperature: **Accepted with exceptions (2026-11-13, G22.4)** for `vapor_nonuniform_step_on` / `_off`. Open exceptions: no step_off balance check; Rust time-step controller on step_on untested. See [M6_VAPOR_G22_4_ACCEPTANCE.md](M6_VAPOR_G22_4_ACCEPTANCE.md) |
| Campbell (`iCampbell=1`) | Rejected for vapor | Resolve SD-006 policy and add discriminating fixtures; no silent source correction | Rejected; deferred to G22 (SD-006 policy) |
| Enhancement on (`iEnhanc=1`) | Planned | Unchanged full-executable default | Accepted (first increment) |
| Enhancement off (`iEnhanc=0`) | Rejected | Kernel harness can select it; full executable cannot via input. Decide evidence strategy before implementation | Rejected; deferred to G22 |
| Constant Dirichlet heat at both ends | Planned | Initial supported boundary pair | Accepted (first increment) |
| Heat flux top / bottom and supported pairs | Accepted (DF, DZ, FF, FZ, FD) with exceptions | Audit `TempAdj:255` integer-code distinction; fixtures; parity; balance (zero flux only) | See [M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md](M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md). FD accepted with a failing v_liquid row and a disclosed vapor-on head drift (user decision 2026-11-13). `kTopT<-1` rejected with trigger |
| Zero-gradient heat bottom | Accepted (DZ, FZ) | Source has no vapor term at `kBotT==0` (H2); fixtures, parity, balance | Accepted; see acceptance doc |
| Sinusoidal top temperature | Rejected | Contract periodic forcing, source precision and fixtures | Rejected; deferred to G22 |
| `kTopT=0` | Invalid/unsupported (kept rejected, H3) | Source has no assigned heat row; not a promised valid capability | Invalid/unsupported; not a deferred capability |
| Fixed water heads at both ends | Planned | Initial supported boundary pair | Accepted (first increment) |
| Water flux / free drainage and supported mixed pairs | Rejected | Vapor boundary/storage accounting, source and balance evidence | **Decided at GATE W1 (user, 2026-11-13), split per form below.** Record: [M6_VAPOR_G22_2_ACCEPTANCE.md](M6_VAPOR_G22_2_ACCEPTANCE.md). G22.2 stays Circled. Forms marked "not explicitly answered at GATE W1" carry the audit's disposition; the user has not confirmed them. |
| Water flux: free drainage, lower (`FreeD`, `KodBot=-5`) | Rejected | [Audit](M6_VAPOR_FREE_DRAINAGE_AUDIT.md) | **Accepted for orchestration 29.** Reverts to rejected unless: User selects it for orchestration 29 and approves a case with an explicit bottom-row design matching `WATFLOW.FOR:211-229`. Otherwise stays rejected at `mod.rs:217`. |
| Water flux: upper prescribed flux (`KodTop=-1`, static) | Rejected | [Audit](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Implemented as part of the closed flux/flux pair only (orchestration 29, vapor on).** Single-form use with a fixed-head or free-drainage partner remains accepted for orchestration 29, not implemented. |
| Water flux: lower prescribed flux (`KodBot=-1`) | Rejected | [Audit](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Implemented as part of the closed flux/flux pair only (orchestration 29, vapor on).** Single-form use with a fixed-head or free-drainage partner remains accepted for orchestration 29, not implemented. |
| Water flux: seepage face (`SeepF`, `hSeep`) | Rejected | [Audit](M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md) | **Rejected with trigger.** A nonlinear head condition (zero flux or `hSeep` Dirichlet, switched on the sign of `vBot` and `hNew(1)>=hSeep`) is needed in a vapor case; this needs water-only seepage support first, a design for the `KodBot` switch inside the vapor pass, and a `vBot` consistent with the vapor-augmented `ConB`. |
| Water flux: water table (`qDrain`, drain flux) | Rejected | [Audit](M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md) | **Rejected with trigger.** A case with drain flux depending on `hNew(1)` is requested with vapor on; this needs water-only drain support, an iterated or linearized `rBot(hNew(1))` in the vapor pass, and the drain-parameter model (unread here). |
| Water flux: lower `qGWLF` (`KodBot=-7`) | Rejected | [Audit](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Rejected with trigger** (not explicitly answered at GATE W1; audit disposition recorded). Trigger: no Rust mode for `qGWLF`; needs a new mode and per-iteration `Fqh` update before any vapor work. |
| Water flux: `TopInF` / `BotInF` (+-3) | Rejected | [Audit](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Rejected with trigger** (not explicitly answered at GATE W1; audit disposition recorded). Trigger: time-variable input path not audited here; no Rust mode in the in-scope files. |
| Water flux pair, flux/flux (closed) | Rejected | [Audit pairs table](M6_VAPOR_WATER_FLUX_AUDIT.md); [closed-pair design](M6_VAPOR_CLOSED_PAIR_ROW_DESIGN.md); [fixture design](M6_VAPOR_CLOSED_PAIR_FIXTURE_DESIGN.md) | **Accepted and implemented with vapor on (orchestration 29, GATE 29.2, 2026-11-14).** Exit criterion met with caveats: fixtures `vapor_closed_flux_{on,off}`, `vapor_flux_known_{on,off}` (Fortran reference, 0-240 h, start -100 cm); balance test `vapor_closed_pair_balance`; typed guard `vapor_boundary_guard`. Open follow-ups: Rust trajectory comparison against these fixtures; full-trajectory Fortran-side balance; criteria entries (D5) not yet written. |
| Water flux pair, flux/FixedHead | Rejected | [Audit pairs table](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Accepted for orchestration 29; not implemented** (the guard accepts only the closed pair). |
| Water flux pairs with free drainage (flux/FreeD, head/FreeD) | Rejected | [Audit pairs table](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Accepted for orchestration 29** (follow free drainage). |
| Water flux pairs with `qGWLF`, seepage face or water table | Rejected | [Audit pairs table](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Rejected with trigger** (follow the rejected singles; flux/`qGWLF` is not explicitly answered at GATE W1). Triggers are those of the single forms above. |
| Atmospheric water switching | Rejected | Meteorological/evaporation prerequisites and interactions; coordinate with G20 | Rejected; deferred to G22, coordinated with G20 |
| Nonzero prescribed sink or root uptake with heat/vapor | Rejected | Separate source/coupling audit and acceptance gate | **Rejected with triggers (Gate S, 2026-11-13)**, audit forms (a)-(d); no implementation. Audit: [M6_VAPOR_SINK_ROOT_AUDIT.md](M6_VAPOR_SINK_ROOT_AUDIT.md); triggers in [M6_VAPOR_G22_4_ACCEPTANCE.md](M6_VAPOR_G22_4_ACCEPTANCE.md) |
| Scalar isothermal `ConVh` helper | Audited only | Establish reachable use and distinguish hardcoded 20 °C from array routine | **Decided, resolved by removal (2026-11-13, G22.4)**: audited reachable but output-neutral ([audit](M6_VAPOR_CONVH_AUDIT.md), `ddc8ef6`); helper removed (`cbc52c0`, user decision). Dead `_x_conv`/`_t_conv` params of `gen_mat` remain (see [acceptance](M6_VAPOR_G22_4_ACCEPTANCE.md)) |
| Vapor output fields and solver/output distinction | Planned | Explicit mapping, units, print floors and nonuniform-mesh tests | Accepted (first increment): `vapor_output`, trajectory fields; vapor content is not an output field |
| Other hydraulic models, hysteresis, dual porosity | Separate physics prerequisites | Later vapor interactions remain visible; no promise of valid unsupported combinations | Separate physics prerequisites; deferred to G22 |
| Temperature-dependent liquid water (`lWDep`) | G23, rejected | Separate contract and decision; not conflated with vapor thermal-gradient coupling | Rejected; deferred to G23 |
| Surface energy balance (`lEnBal`) | G20, rejected | Separate meteorological path and acceptance gate | Rejected; deferred to G20 |

## G22 completion rule

Audit all reachable vapor branches and supported interactions, assign each a
contract, implementation owner, discriminating fixture, precision-aware
comparison, independent/balance check, and acceptance status. Expand this matrix
when recon finds another active branch. Do not equate a Cartesian product of
invalid selectors with valid capability. Distinguish invalid, unsupported,
blocked, implemented-unvalidated and accepted. Every deferred valid row needs a
trigger/dependency; no row disappears merely because G19 passes.

Trigger: G19 first-increment acceptance (met 2026-11-12, with the recorded exception). The next dependency-ready row is not yet selected. Sequence the next increment by unresolved
dependencies and user priority; completion is not authorized by orchestration 25.
G20 need not wait for all of G22, but may not assume a vapor boundary interaction
has been accepted merely because the initial fixed-head selection passed.

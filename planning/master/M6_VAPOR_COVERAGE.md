# Vapor capability coverage and follow-on goal

Status: **G19 first increment complete with exception, 2026-11-12 ([acceptance](M6_VAPOR_ACCEPTANCE.md)). G22 open.** Planning matrix of 2026-11-11 retained; Status column added. G22.4 rows decided 2026-11-13 (goal stays Circled with open items; see [G22.4 acceptance](M6_VAPOR_G22_4_ACCEPTANCE.md)).
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
| Water flux / free drainage and supported mixed pairs | Rejected | Vapor boundary/storage accounting, source and balance evidence | Rejected; deferred to G22 |
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

# Vapor capability coverage and follow-on goal

Status: **Planning matrix, 2026-11-11. No vapor capability accepted yet.**
Authority: [orchestration 25 decisions D2/D3/D5/D11](../EXECUTION_PLAN_25.md).
Goals: G19 is the bounded first increment; **G22 is complete vapor coverage**.
Accepting G19 does not complete G22.

| Capability / interaction | First increment (25) | Remaining coverage / gate |
|---|---|---|
| Vapor conductivity, content/capacity, velocities, latent/sensible heat | Planned | Every selected path requires kernel, coupled and independent evidence |
| Two water→heat passes and atomic acceptance/retry | Planned | Remains required in follow-ons |
| Default thermal conductivity (`iCampbell=0`) | Planned | Nonuniform-temperature source fixtures required |
| Campbell (`iCampbell=1`) | Rejected for vapor | Resolve SD-006 policy and add discriminating fixtures; no silent source correction |
| Enhancement on (`iEnhanc=1`) | Planned | Unchanged full-executable default |
| Enhancement off (`iEnhanc=0`) | Rejected | Kernel harness can select it; full executable cannot via input. Decide evidence strategy before implementation |
| Constant Dirichlet heat at both ends | Planned | Initial supported boundary pair |
| Heat flux top / bottom and supported pairs | Rejected | Audit `TempAdj:255` integer-code distinction; fixture each supported interaction |
| Zero-gradient heat bottom | Rejected | Resolve physical/source validation gap; finite fixture and independent checks |
| Sinusoidal top temperature | Rejected | Contract periodic forcing, source precision and fixtures |
| `kTopT=0` | Invalid/unsupported | Source has no assigned heat row; not a promised valid capability |
| Fixed water heads at both ends | Planned | Initial supported boundary pair |
| Water flux / free drainage and supported mixed pairs | Rejected | Vapor boundary/storage accounting, source and balance evidence |
| Atmospheric water switching | Rejected | Meteorological/evaporation prerequisites and interactions; coordinate with G20 |
| Nonzero prescribed sink or root uptake with heat/vapor | Rejected | Separate source/coupling audit and acceptance gate |
| Scalar isothermal `ConVh` helper | Audited only | Establish reachable use and distinguish hardcoded 20 °C from array routine |
| Vapor output fields and solver/output distinction | Planned | Explicit mapping, units, print floors and nonuniform-mesh tests |
| Other hydraulic models, hysteresis, dual porosity | Separate physics prerequisites | Later vapor interactions remain visible; no promise of valid unsupported combinations |
| Temperature-dependent liquid water (`lWDep`) | G23, rejected | Separate contract and decision; not conflated with vapor thermal-gradient coupling |
| Surface energy balance (`lEnBal`) | G20, rejected | Separate meteorological path and acceptance gate |

## G22 completion rule

Audit all reachable vapor branches and supported interactions, assign each a
contract, implementation owner, discriminating fixture, precision-aware
comparison, independent/balance check, and acceptance status. Expand this matrix
when recon finds another active branch. Do not equate a Cartesian product of
invalid selectors with valid capability. Distinguish invalid, unsupported,
blocked, implemented-unvalidated and accepted. Every deferred valid row needs a
trigger/dependency; no row disappears merely because G19 passes.

Trigger: G19 first-increment acceptance. Sequence the next increment by unresolved
dependencies and user priority; completion is not authorized by orchestration 25.
G20 need not wait for all of G22, but may not assume a vapor boundary interaction
has been accepted merely because the initial fixed-head selection passed.

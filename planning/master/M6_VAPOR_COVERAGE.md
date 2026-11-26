# Vapor capability coverage and follow-on goal

Status: **G19 first increment complete with exception, 2026-11-12 ([acceptance](M6_VAPOR_ACCEPTANCE.md)). G22 open.** Planning matrix of 2026-11-11 retained; Status column added. G22.4 rows decided 2026-11-13 (goal stays Circled with open items; see [G22.4 acceptance](M6_VAPOR_G22_4_ACCEPTANCE.md)). G22.2 water-flux rows decided at GATE W1, 2026-11-13 (see [G22.2 acceptance](M6_VAPOR_G22_2_ACCEPTANCE.md)); atmospheric (row below) is G22.3. **G22.2 closed with exceptions and G22.3 accepted with exceptions, 2026-11-23 (user, orchestration 31; see [G22.3 acceptance](M6_VAPOR_G22_3_ACCEPTANCE.md)).**
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
| Water flux: upper prescribed flux (`KodTop=-1`, static) | Rejected | [Audit](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Implemented with the closed flux/flux pair (orchestration 29), with free-drainage lower (orchestration 30, fixtures `vapor_free_drainage_{on,off}`, exact parity 30.5.1), and with fixed-head lower (`880c6c2`, `FixedHeadLowerFluxUpper`, fixture `vapor_flux_fixedhead_wet_on`, exact parity passes).** |
| Water flux: lower prescribed flux (`KodBot=-1`) | Rejected | [Audit](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Implemented as part of the closed flux/flux pair only (orchestration 29, vapor on, `ClosedFlux`).** Single-form use with a fixed-head upper partner or free-drainage partner remains accepted, not implemented in code (`heat/vapor/mod.rs`). |
| Water flux: seepage face (`SeepF`, `hSeep`) | Rejected | [Audit](M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md) | **Rejected with trigger.** A nonlinear head condition (zero flux or `hSeep` Dirichlet, switched on the sign of `vBot` and `hNew(1)>=hSeep`) is needed in a vapor case; this needs water-only seepage support first, a design for the `KodBot` switch inside the vapor pass, and a `vBot` consistent with the vapor-augmented `ConB`. |
| Water table (`qDrain`, drain flux) | Rejected | [Audit](M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md) | **Rejected with trigger.** A case with drain flux depending on `hNew(1)` is requested with vapor on; this needs water-only drain support, an iterated or linearized `rBot(hNew(1))` in the vapor pass, and the drain-parameter model (unread here). |
| Water flux: lower `qGWLF` (`KodBot=-7`) | Rejected | [Audit](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Rejected with trigger** (not explicitly answered at GATE W1; audit disposition recorded). Trigger: no Rust mode for `qGWLF`; needs a new mode and per-iteration `Fqh` update before any vapor work. |
| Water flux: `TopInF` / `BotInF` (+-3) | Rejected | [Audit](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Rejected with trigger** (not explicitly answered at GATE W1; audit disposition recorded). Trigger: time-variable input path not audited here; no Rust mode in the in-scope files. |
| Water flux pair, flux/flux (closed) | Rejected | [Audit pairs table](M6_VAPOR_WATER_FLUX_AUDIT.md); [closed-pair design](M6_VAPOR_CLOSED_PAIR_ROW_DESIGN.md); [fixture design](M6_VAPOR_CLOSED_PAIR_FIXTURE_DESIGN.md) | **Accepted and implemented with vapor on (orchestration 29, GATE 29.2, 2026-11-14).** Exit criterion met with caveats: fixtures `vapor_closed_flux_{on,off}`, `vapor_flux_known_{on,off}` (Fortran reference, 0-240 h, start -100 cm); balance test `vapor_closed_pair_balance`; typed guard `vapor_boundary_guard`. Open follow-ups: Rust trajectory comparison against these fixtures; full-trajectory Fortran-side balance; criteria entries (D5) not yet written. |
| Water flux pair, flux/FixedHead | Rejected | [Audit pairs table](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Accepted and implemented (`880c6c2`, `FixedHeadLowerFluxUpper`).** Wet fixture staged and exact parity passes (`vapor_flux_fixedhead_wet_on`, test `flux_fixedhead_wet_on_matches_fixture`). Dry (-10000 cm) divergence recorded per `M6_VAPOR_FLUX_FIXEDHEAD_DESIGN.md` (NaN by 72 h under graded temperature). Reverse pair (prescribed flux lower with fixed-head upper) remains accepted, not implemented. |
| Water flux pairs with free drainage (flux/FreeD, head/FreeD) | Rejected | [Audit pairs table](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Accepted for orchestration 29** (follow free drainage). |
| Water flux pairs with `qGWLF`, seepage face or water table | Rejected | [Audit pairs table](M6_VAPOR_WATER_FLUX_AUDIT.md) | **Rejected with trigger** (follow the rejected singles; flux/`qGWLF` is not explicitly answered at GATE W1). Triggers are those of the single forms above. |
| Atmospheric water switching | Rejected | Meteorological/evaporation prerequisites and interactions; coordinate with G20 | **Accepted with exceptions (2026-11-23, G22.3, user).** Bare soil, daily `Meteo.in`, `KodTop=-4` with `TopInF=t`, fixed-head lower, vapor on. Fixture `vapor_met_bare_soil_on`; parity `vapor_met_parity` (switches on the Fortran steps); balance `vapor_met_balance`. Exception: head-end boundary flux originally checked as interior conservation only (G25; resolved 2026-11-24: met path reports `TLInf` `vTop`/`vBot` via `t_level_fluxes`/`t_level_fluxes_for_step` in `heat::vapor::output`, verified by `vapor_met_tlevel_parity` against compressed sidecar `vapor_met_bare_soil_on_tlevel.json.gz` under `criteria.json` v8 `t_level_boundary_flux` and `t_level_cumulative_flux`). The independent water-solve balance remains in `vapor_met_balance`; see [M6_VAPOR_G22_3_ACCEPTANCE.md](M6_VAPOR_G22_3_ACCEPTANCE.md). Met run path only in the test driver (G26; resolved 2026-11-24: now in `src/` as `MetForcing` in `meteo::forcing` and `VaporWaterHeatDriver::step_atmospheric` in `heat::vapor::driver`); no met discrimination entry (G27). Other met selections and ponding with vapor rejected with triggers. See [M6_VAPOR_G22_3_ACCEPTANCE.md](M6_VAPOR_G22_3_ACCEPTANCE.md) |
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


## C3 dry-regime closed pair record (2026-11-14)

Gate: G22.2-C3 approved as designed (`planning/master/M6_VAPOR_C3_DRY_DESIGN.md`). Runs under `.development/30.7.3/` (scratch, untracked). Executable: pinned reference build, sha256 `15c24e35e739d992fb6f1a0d92619103baf57223cbdfd12968b0ed33dc246edb`. Records: `.development/30.7.3/c3_record.json`.

Both runs: closed flux/flux pair (`KodTop=-1`, `KodBot=-1`), vapor on, initial head -10000 cm, the same LEVEL_01.DIR. The only difference is the temperature profile and the bottom heat BC.

### Thermal gradient (attempt C deck, 25 to 15 C)
Exit code 0; 22153 steps; RUN_INF non-converged flags 0; final dt 0.10000E-01 h.

| t (h) | top head (cm) | bottom head (cm) | NaN rows |
|---|---|---|---|
| 0.0000 | -10000.000 | -10000.000 | 0/41 |
| 6.0000 | -239533.344 | -5302.966 | 0/41 |
| 24.0000 | -614025.312 | -2300.055 | 0/41 |
| 72.0000 | NaN | NaN | 41/41 |
| 240.0000 | NaN | NaN | 41/41 |

### Isothermal control (25 C, vapor on)
Exit code 0; 2400 steps; non-converged flags 0; final dt 0.10000E+00 h.

| t (h) | top head (cm) | bottom head (cm) | NaN rows |
|---|---|---|---|
| 0.0000 | -10000.000 | -10000.000 | 0/41 |
| 6.0000 | -10000.005 | -10000.005 | 0/41 |
| 24.0000 | -10000.002 | -10000.002 | 0/41 |
| 72.0000 | -9999.998 | -9999.998 | 0/41 |
| 240.0000 | -9999.994 | -9999.994 | 0/41 |

### Finding
Hypothesis supported. At the same head and flux settings, the thermal gradient diverges (head blow-up by 72 h, NaN) and the isothermal control converges to 240 h with head held near -10000 cm.

Two points for the record:
- The Fortran run exits with code 0 while its output is NaN. The divergence is silent to the exit code and to RUN_INF's convergence flags. Any runner must check NOD_INF for NaN, not only the exit code.
- The thermal run takes 22153 steps and its dt shrinks to 0.01 h, against 2400 steps at dt 0.1 h for the control.

Not tested here: whether the divergence goes away with the thermal vapor terms removed from the water equation. That counterfactual (scratch build `.development/30.7.1/cf2`) is outside the approved C3 design and is not run.

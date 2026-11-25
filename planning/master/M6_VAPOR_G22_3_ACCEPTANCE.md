# M6 vapor G22.3 acceptance (orchestration 31, task 31.5.1)

Status: **accepted with exceptions, 2026-11-23 (user decision). Goal G22.3 is Completed.**
Atmospheric water switching with vapor on, driven by meteorological input (`lMeteo=t`),
matches the pinned Fortran trajectory for the Gate A selection. The exceptions below
are limits of this acceptance, not open defects in the accepted path.

Basis: [M6_VAPOR_G22_3_CONTRACT.md](M6_VAPOR_G22_3_CONTRACT.md) (Gate A resolutions, section 9),
[EXECUTION_PLAN_31.md](../EXECUTION_PLAN_31.md) decisions D-10, D-11 and D-12,
[STATE_31.md](../STATE_31.md) task reports and wave-end verifications,
criteria v6 ([criteria.json](../../reference/vapor_fortran/criteria.json), SHA-256 `b03047a6e0091b22...`, unchanged in waves 3 and 4).

## Accepted selection

| Item | Accepted value |
|---|---|
| Meteorological input | `Meteo.in`, daily records (`lMetDaily=f`), `iRadiation=1`, `iSunSh=0`, `iRelHum=0` |
| Crop | Bare soil (`iCrop=0`; SCF=0, rc=0, crop height 0.1 cm) |
| Evaporation | Penman-Monteith combination ET (`SetMeteo`/`Aero`, `TIME.FOR:443-667`), f32 in source order |
| Radiation | Rn, Rns, Rnl (`b7acedf`) |
| Upper boundary | Atmospheric (`KodTop=-4`, `TopInF=t`), switching between flux and head at `hCritA` (`WATFLOW.FOR:392-426`) |
| Decomposition | Existing Flux/Head `VaporBoundaryPair`s, no new variant (D-10) |
| Lower boundary | Fixed head |
| Vapor | On, enhancement on, `iCampbell=0` |

Everything outside this selection stays rejected with the triggers in contract section 7
and the typed errors `UnsupportedMeteoSelection`, `SurfaceEnergyBalanceUnsupported`,
`SurfacePondingUnsupported` and `UnsupportedVaporLowerPair`.

## Evidence

Case `vapor_met_bare_soil_on`: 240 h, dt 0.1 h, 10 daily records, rain on day 6.
Fixture SHA-256 `75b4207387dff483...`, pinned build executable `87969711...0519638`.

| Check | Result | Commit |
|---|---|---|
| Pinned Fortran run (31.2.1) | 2400 steps to 240 h, 0 non-convergence, 3 upper switches (`RUN_INF.OUT`, checked by the orchestrator) | `bcefbc5` |
| Evaporation vs Fortran `Meteo.out` (31.3.1) | All 10 records within 5e-4 on ET, Rns, Rnl, RadTerm and AeroTerm | `ea3c7a9` |
| `Meteo.in` reader (31.3.3) | Parses the committed deck; one test per named rejection | `c98dca8` |
| Atmospheric switching with vapor (31.3.2) | Fortran `Shift` order, rTop held per record (`HYDRUS.FOR:628`); water-only tests unchanged | `d96fcb7`, `5c286f4` |
| Trajectory parity, the D-10 gate (31.4.1) | 12 fields x 11 times x 41 nodes within the unchanged per-field bounds; no source-precision rows. Max differences: head 7.0e-4 cm (bound 4.5), moisture 5.0e-5 (1.05e-3), temperature 5.0e-3 C (0.05) | `87fae80` |
| Switch timing (31.4.1) | -4 to 4 at 42.7 h, 4 to -4 at 120.1 h, -4 to 4 at 200.3 h, on the same 0.1 h step as Fortran | `87fae80` |
| Water balance (31.4.2) | Precedent tolerance `max(1e-9*abs(netIn), 1e-13)` holds on all 2400 steps; max per-step residual about 1e-15 cm; cumulative under 4.3e-14 cm. Fortran `BALANCE.OUT` WatBalT is about 1e-4 cm (evidence only) | `87defcf` |

Workspace at wave 4 end: `cargo test --workspace` 694 passed, 0 failed; clippy `-D warnings` and fmt clean; Python tools 504 OK.

## Exceptions

1. **Boundary flux at head-type ends is not independently checked.** The solver passes flux 0.0 for a head boundary (`atmospheric.rs:486`, the driver's fixed-head bottom), so `total_face_fluxes` there is not a solved flux. In head mode and at the bottom, the balance uses the Fortran vTop form (`WATFLOW.FOR:395-407`), which closes by construction and tests interior conservation only. In flux mode the top is checked independently against the prescribed total flux. User decision 2026-11-23. Follow-up: [G25](../GOALS.md).
2. **The met run path exists only in the test driver.** The atmospheric entry around the vapor water-heat driver and the per-record top temperature `(TMax+TMin)/2` (`TIME.FOR:519`) live in `crates/hydrus-core/tests/support/vapor_met.rs`, not in `src/`. Follow-up: [G26](../GOALS.md).
3. **No met discrimination entry.** `criteria.json` has no vapor-fields-nonzero entry for `vapor_met_bare_soil_on`. Adding one needs approval (D-9). Follow-up: [G27](../GOALS.md).
4. **One fixture, one selection.** Evidence covers a single bare-soil daily case at a fixed-head bottom. Print times are daily, so the switches are checked from the driver trace, not from the fixture.
5. **Iteration history differs.** Fortran switches inside its Picard loop (at most 2 iterations per step). Rust re-solves the step under the new pair (D-10) and needs up to 6 or 7 iterations per pass at the rain record. Trajectories still match within bounds.
6. **Not ported.** Interception, crop modes, Hargreaves, sub-daily records, surface ponding with vapor, and the Sink<=0 90%-depth head clamp (`WATFLOW.FOR:99-100`; bare soil has no sink).

## Update (2026-11-24)

Exception 2 ("the met run path exists only in the test driver") is resolved by plan 32 (G26). The met run path now lives in `src/` (`MetForcing`, `VaporWaterHeatDriver::step_atmospheric`), and both met tests run through it with unchanged tolerances. Plan: [EXECUTION_PLAN_32.md](../EXECUTION_PLAN_32.md).

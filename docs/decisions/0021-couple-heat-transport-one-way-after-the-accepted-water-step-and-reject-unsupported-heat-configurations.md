---
id: "0021"
title: "Couple heat transport one-way after the accepted water step and reject unsupported heat configurations"
status: "Proposed"
date: 2026-11-11
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_24.md task 24.6.2; planning/master/M6_HEAT_CONTRACT.md (## Source map, ## Coupling order, ## Supported, ## Rejected); decisions D7, D9, D13 (2026-11-11)"
---

# 21. Couple heat transport one-way after the accepted water step and reject unsupported heat configurations

## Context and Problem Statement

M6 adds heat-only transport (conduction and advection of temperature) on top of the existing water product. Today the Rust crate has no temperature state and no heat solver. The pinned Fortran couples heat to water in both directions. Water flow reads temperature when `lWDep` is set (temperature-dependent surface tension, viscosity and density, and the `ConLT` conductivity term). Vapor and energy-balance paths also feed back into the water and boundary solves. Implementing those paths now would widen the M6 scope past what the heat-only capability can verify.

The coupling order and the set of accepted configurations must be fixed before the heat code is written. Otherwise an unsupported Fortran mode could be run silently with an approximation, and the result would look like a valid heat solution.

## Decision Drivers

- The pinned Fortran snapshot is the reference for heat behavior (0001).
- No silent approximation. An unsupported configuration must fail with a typed error.
- The accepted water product is not changed by the heat capability. The water-side precision and operation-order rules (0010) still apply.
- No new dependency is needed for the heat step (0011).

## Considered Options

1. **Two-way coupling.** Heat feeds temperature back into the water step (`lWDep`) and into the vapor and energy paths. This matches the full Fortran but needs vapor and energy scope that M6 does not have.
2. **One-way coupling with typed rejection (chosen).** Heat reads the accepted water result and never writes to water. Configurations that need heat-to-water feedback or out-of-scope physics are rejected with a typed error.
3. **Catch-all approximation.** Run heat-only with the feedback terms dropped and report the result. Rejected because the output would not match the Fortran and would not be marked as approximate.

## Decision Outcome

Chosen option: 2.

- **D9 (one-way coupling).** Each time step runs the water step first, `advance_water_step` (`crates/hydrus-core/src/water_solver.rs:183`). The heat step then reads the accepted water snapshot (`AcceptedWaterSnapshot`, `water_state.rs:68`). Heat never writes to water. In the Fortran, the water step is `WatFlow` at `HYDRUS.FOR:419` and the heat step is `Temper` at `HYDRUS.FOR:491`, under `lTemp`. The accepted state handoff is `Update` at `HYDRUS.FOR:695` (`WATFLOW.FOR:1115`, `TempO=TempN`; `WATFLOW.FOR:1109`, `vOld=vNew`). Water never reads heat only when `lWDep` is false and `lVapor` is false. `lWDep=true` is rejected because it is heat-to-water feedback. The input flag is `lWDep` (`INPUT.FOR:31`). The WATFLOW variable is `lWTDep`. Its temperature terms are at `WATFLOW.FOR:403-404` (surface and conductivity terms) and `WATFLOW.FOR:458-464` (surface tension and viscosity).
- **D7 (typed rejection).** Every configuration not listed in the contract's `## Supported` section is rejected with a typed error. The rule is a catch-all. The named rejections are:
  - `lVapor=true`: vapor terms in `TempAdj` (`TEMPER.FOR:48`).
  - `lEnBal=true`: forces `kTopT=-1` (`HYDRUS.FOR:268`) and enters the energy path.
  - `lWDep=true`: heat-to-water feedback (above).
  - `iDualPor != 0`: `HYDRUS.FOR:278` (`lEquil=.false.`).
  - `kTopT == 0`: `SetUpHeat` has no branch for it, so the source reuses a stale row. Not reproduced.
  - `tPeriod > 0`: sinusoidal top boundary, which uses `sngl(t)`. Not in the M6 selection.
  
  The rejection set is the contract's `## Rejected` section in `planning/master/M6_HEAT_CONTRACT.md`. This ADR summarizes the rule and does not repeat the table.
- **D13 (Campbell conductivity).** The heat configuration accepts `iCampbell` 0 and 1. Both branches (`TEMPER.FOR:279-300`) are in M6 scope. The default stays the primary focus.
- **Water-side constraint.** The accepted water product is unchanged, except for the dated van Genuchten exponent correction in commit `1d4131a`. That correction is cited here and is not re-decided.
- **Supersession.** None. ADRs 0013, 0015 and 0019 do not govern heat.

## Consequences

- **Positive:** The coupling order is fixed and testable. Unsupported modes fail with a named error rather than returning a plausible result.
- **Positive:** The heat step adds no change to the water solver's operation order or precision.
- **Negative:** `lVapor`, `lEnBal`, `lWDep`, dual porosity and the sinusoidal top boundary stay out of M6. Users with those flags get an error, not a result, until a later goal adds them.
- **Negative:** Parity evidence for heat comes from fixtures, not from a live run of the pinned Fortran (see the M6 contract).
- **Open item (not decided here):** `kBotT==0` (zero-gradient bottom boundary, `SetUpHeat` `TEMPER.FOR:108-111`) is supported in code. It is not validated against the Fortran. Its physical semantics are an open question in the M6 contract. This ADR records the item as a consequence only.

## Validation & Compliance

- Rejection set and supported set: [M6_HEAT_CONTRACT.md](../../planning/master/M6_HEAT_CONTRACT.md) sections `## Supported` and `## Rejected`.
- Coupling order and source map: [M6_HEAT_CONTRACT.md](../../planning/master/M6_HEAT_CONTRACT.md) sections `## Source map` and `## Coupling order`.
- Fortran lines were checked by grep or read against `src_reference/` on 2026-11-11.
- This ADR stays **Proposed** until the user accepts it. It is not accepted by the M6 acceptance gate.

## Dated note (2026-11-12): scope and navigation

This ADR describes heat-only behavior and is not generalized. The vapor-aware
selection (`lVapor=true`) is governed by `planning/master/M6_VAPOR_CONTRACT.md`;
its acceptance record is `planning/master/M6_VAPOR_ACCEPTANCE.md` and its coverage
map is `planning/master/M6_VAPOR_COVERAGE.md`. The heat-only rejection of `lVapor`
remains in force for heat-only entry points. This ADR stays **Proposed**; it is
neither accepted nor superseded by the vapor acceptance.

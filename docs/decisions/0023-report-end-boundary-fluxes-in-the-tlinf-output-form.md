---
id: "0023"
title: "Report end boundary fluxes in the TLInf output form"
status: "Accepted"
date: 2026-11-24
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_33.md D-1, D-2, and D-4 (task 33.1.1); planning/GOALS.md G25"
---

# 23. Report end boundary fluxes in the TLInf output form

## Context and Problem Statement

The pinned Fortran defines two distinct forms of the top boundary flux `vTop`:

1. **Switching form (`Shift`).** `src_reference/WATFLOW.FOR:400-407` computes `vTop` inside the atmospheric switching check using the liquid face flux, top half-cell liquid storage, sink, and (under `lVapor`) the `ConVh` and `ConVT` vapor face terms, omitting the vapor half-cell storage term. Rust ports this form as `trial_v_top` (`crates/hydrus-core/src/atmospheric.rs:501-510`).
2. **Reported form (`TLInf`).** `src_reference/OUTPUT.FOR:25-54` computes `vTop` (`vT`, `:31-32`, `:43-53`) and `vBot` (`vB`, `:36-37`, `:44-51`, `:54`) for `T_LEVEL.OUT` (`write(71,170)` at `src_reference/OUTPUT.FOR:193`). Unlike `Shift`, `TLInf` adds the half-cell vapor storage term `(ThVNew - ThVOld) * dx / 2 / dt` at both ends (`OUTPUT.FOR:47-50`).

In Rust, head-type boundary ends do not compute a solved flux on the water solver output: `upper_for` (`crates/hydrus-core/src/atmospheric.rs:484-497`) sets `flux: 0.0` in `UpperBoundary::Head`, and `total_face_fluxes` (`crates/hydrus-core/src/water_solver.rs:584-589,611-613`) uses `input.upper.measured_flux().unwrap_or(0.0)` at `:585`. The question is whether to modify `water_solver.rs` and `atmospheric.rs` to return head-end boundary fluxes directly, or to compute reported end fluxes in `heat/vapor/output.rs` using the `TLInf` output formula.

## Decision Drivers

- Solver heads and existing parity trajectories must remain bit-identical.
- `trial_v_top` in `atmospheric.rs` represents the `Shift` switching form and must not be altered to include vapor storage.
- `TLInf` already has a pure post-step observation precedent in `crates/hydrus-core/src/heat/vapor/output.rs` (`observe_vapor` / `observe_step`), using stored conductivities and accepted water/vapor state.
- Reported `vTop` and `vBot` must be backed by pinned-Fortran `T_LEVEL.OUT` evidence and user-approved bounds after measurement.

## Considered Options

1. **Report `TLInf` end fluxes as a pure output function in `heat/vapor/output.rs` (chosen).** Compute `vTop` and `vBot` from the accepted step without modifying `water_solver.rs` or `atmospheric.rs`.
2. **Compute and return head-end boundary fluxes inside `water_solver.rs` and `atmospheric.rs`.** Rejected for G25 because changing solver outputs risks altering accepted state or conflating solver face arrays (`total_face_fluxes`) with the `TLInf` half-cell output form.

## Decision Outcome

Chosen option: 1.

- **Pure `TLInf` output port (D-1, plan 33).** The met path reports `vTop` and `vBot` through a pure port of `TLInf` (`src_reference/OUTPUT.FOR:25-54`) in `crates/hydrus-core/src/heat/vapor/output.rs`, computed from the accepted step.
- **Solver and switching form unchanged (D-1, plan 33).** `crates/hydrus-core/src/water_solver.rs` and `crates/hydrus-core/src/atmospheric.rs` (`trial_v_top`, `upper_for`) are unchanged. `total_face_fluxes` end entries are not a reported boundary flux.
- **Evidence and bounds (D-2, D-4, plan 33).** Evidence is a runner-produced `T_LEVEL.OUT` sidecar fixture (`reference/vapor_fortran/fixtures/vapor_met_bare_soil_on_tlevel.json`) and a `reference/vapor_fortran/criteria.json` v8 entry approved after measurement.

## Consequences

- **Positive:** Heads and all pre-existing solver fields stay bit-identical by construction.
- **Positive:** The `TLInf` output formula stays decoupled from `trial_v_top` (`Shift`) and `total_face_fluxes`, matching Fortran's separation between `WATFLOW.FOR` and `OUTPUT.FOR`.
- **Negative:** The water-only (non-vapor) path has no reported head-end flux yet.
- **Follow-up (G20):** G20 must decide which `vTop` `UpdateEnergy` reads: in Fortran, `call UpdateEnergy` at `src_reference/HYDRUS.FOR:462` runs before `Temper` (`:490`) and before `call TLInf` at `:530` (where `:538` passes `dummy1, dummy2` for `vTop, vBot`).
- **Revisit when:** a caller needs solver-level head-end fluxes (for example on the water-only path or when implementing `UpdateEnergy` in G20).

## Validation & Compliance

- Plan records: `planning/EXECUTION_PLAN_33.md` D-1, D-2, D-4 (and D-8/D-9 for source order and cumulative checks).
- Verification artifacts (produced by downstream tasks in Plan 33): `T_LEVEL.OUT` sidecar fixture and `criteria.json` v8 entry approved after measurement.
- Fortran and Rust line anchors (`src_reference/OUTPUT.FOR:25-54,:193`, `src_reference/WATFLOW.FOR:400-407`, `src_reference/HYDRUS.FOR:462,:530`, `crates/hydrus-core/src/atmospheric.rs:484,:501`, `crates/hydrus-core/src/water_solver.rs:584-589`) were verified by read on 2026-11-24.
- No supersession. ADR 0021 remains Proposed and ADR 0022 remains Accepted; this ADR does not change their status.

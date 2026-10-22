---
id: "0018"
title: "Default to the Fortran-compatible solute backend"
status: "Accepted"
date: 2026-10-22
authors:
  - "Andrew G. Brown"
supersedes: "0013 (default-backend clause only)"
superseded_by: null
source_context: "planning/EXECUTION_PLAN_22.md task 22.1.3; planning/EXECUTION_PLAN_20.md D6; planning/EXECUTION_PLAN_21.md D3; tasks 21.4.1, 21.5.1"
---

# 18. Default to the Fortran-compatible solute backend

## Context and Problem Statement

[0013](0013-select-solute-backend-with-a-closed-enum.md) made `FiniteVolumeBackwardEuler` the default solute backend and left the default switch for later. Orchestration 20 (D6) and Orchestration 21 (D3) deferred that switch until the Rust `FortranCompatible` backend matched the pinned Fortran on every fixture. The `21.4.1` gate passed 44 of 44 fixtures (`17d9d9c`, `planning/master/SOLUTE_FORTRAN_PARITY_18.md` section 21.4.1). Task `21.5.1` then changed the default (`7741a04`).

## Decision Drivers

- The pinned Fortran snapshot is the solute reference (0001).
- Change the default only with parity evidence in hand.
- Keep the old backend available and the enum closed (0013).

## Considered Options

1. Keep `FiniteVolumeBackwardEuler` as the default. Callers who want pinned-source behavior must opt in, and the default stays unproven against the reference.
2. Switch the default to `FortranCompatible` after the 44 of 44 gate.

## Decision Outcome

Chosen option: 2, because the parity gate now supports it. `SoluteConfig` defaults to `SoluteBackend::FortranCompatible`. This supersedes only the default-backend clause of 0013. Its closed-enum decision stays in force. We don't change FV/BE equations (0002).

### Consequences

- **Positive:** The default behaves as the pinned Fortran does, with parity proven on 44 fixtures.
- **Positive:** `FiniteVolumeBackwardEuler` stays selectable through `SoluteConfig::with_backend`.
- **Negative:** Three output differences from Fortran are unobserved by the parity comparison and not yet traced or fixed: `format_fortran_e` prints ` 0.…E+00` for `-0.0` where gfortran prints `-0.…E+00`; the `NOD_INF` depth at t=1, node 2 prints `0.2500` where Fortran prints `-0.2500`; and Rust `make_nod_inf_v_block` prints the solver's `v`/`vv` where Fortran `NodOut` recomputes `vi`/`vVi`.
- **Negative:** Callers that relied on the FV/BE default must now select it explicitly.

## Validation & Compliance

`crates/hydrus-core/tests/solute_solver_selection.rs` (default backend, explicit FV/BE selection, default-step bit identity) and `crates/hydrus-core/tests/solute_backend_contract.rs:199` (default is `FortranCompatible`) cover this decision.

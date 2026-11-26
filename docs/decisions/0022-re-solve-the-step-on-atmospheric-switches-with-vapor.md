---
id: "0022"
title: "Re-solve the step on atmospheric switches with vapor"
status: "Accepted"
date: 2026-11-24
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_32.md D-13 and D-15 (task 32.1.1); planning/EXECUTION_PLAN_31.md D-10; planning/master/M6_VAPOR_G22_3_ACCEPTANCE.md rows 41-42 and exception 5"
---

# 22. Re-solve the step on atmospheric switches with vapor

## Context and Problem Statement

The pinned Fortran switches the top boundary between flux and head (`KodTop`) inside its Picard loop (`src_reference/WATFLOW.FOR:392-426`). The Rust vapor-coupled atmospheric step (`advance_atmospheric_step_vapor`, `crates/hydrus-core/src/atmospheric.rs:517-523`) has no in-loop switch. It solves the step under the start-of-step Flux/Head pair. If the switch test changes the mode, it re-solves the whole step once from the old state under the new pair.

The two codes therefore differ in iteration history. The pinned Fortran needs at most 2 iterations per step. The Rust re-solve needs 6 to 7 iterations per pass at the rain record. The M6 vapor acceptance (`planning/master/M6_VAPOR_G22_3_ACCEPTANCE.md`) records this as exception 5. Trajectories still match the Fortran within the unchanged per-field bounds. The D-10 parity gate (rows 41-42) passes, including switch timing on the same 0.1 h step.

The question is whether to port in-loop switching now, or keep the re-solve and bound its use.

## Decision Drivers

- The Fortran dt control reads iteration counts. Any change in iteration history can change the dt the Fortran would choose.
- The met path currently runs a fixed daily schedule, so dt control is not exercised there.
- In-loop switching needs a change to the water solver's Picard structure, which is out of scope for the met-path work in plan 32.
- Parity evidence must stay unchanged. No bound may be widened to accommodate the difference (plan 31 D-10).

## Considered Options

1. **Port in-loop switching now.** Matches the Fortran iteration history exactly. Needs a change to the Picard loop in the water solver, which plan 32 does not allow.
2. **Keep the step-level re-solve and bound its scope (chosen).** Parity holds on the gated case. The met path is fixed-schedule only. Adaptive dt is excluded until in-loop switching exists.
3. **Keep the re-solve and enable adaptive dt on the met path.** Rejected. Adaptive dt would act on iteration counts that differ from Fortran, so the dt sequence would diverge from the reference without being detected.

## Decision Outcome

Chosen option: 2.

- **Re-solve kept (D-13, plan 32).** The atmospheric step keeps the step-level re-solve from plan 31 D-10. A changed mode re-solves the step once from the old state.
- **No internal dt cut (D-15, plan 32).** `step_atmospheric` forces no internal dt cut. `minimum_timestep` equals the step's dt, as the test driver does today. On non-convergence in either pass, the step rejects staged state and returns a typed error. It never calls the restart path in `step()`.
- **Scope.** The met path is fixed-schedule only.

## Consequences

- **Positive:** The parity gate holds on the accepted bare-soil case with the unchanged bounds, and no source-precision rows are needed.
- **Positive:** A non-converged pass fails with a typed error. It does not silently restart with a smaller dt.
- **Negative:** Iteration history differs from the Fortran (6 to 7 against at most 2 per step). Any feature that reads iteration counts inherits the difference.
- **Negative:** Adaptive dt on the met path is excluded, because Fortran dt control reads iteration counts.
- **Revisit when:** adaptive dt is requested on the met path, or a met case fails parity. The in-loop switching port is tracked as follow-up goal G31.

## Validation & Compliance

- Parity: `vapor_met_parity` (D-10 gate, M6_VAPOR_G22_3_ACCEPTANCE.md rows 41-42). Balance: `vapor_met_balance`.
- Iteration difference: M6_VAPOR_G22_3_ACCEPTANCE.md exception 5 (line 53).
- Plan records: `planning/EXECUTION_PLAN_32.md` D-13 and D-15; `planning/EXECUTION_PLAN_31.md` D-10.
- Fortran lines were checked by read against `src_reference/` on 2026-11-24.
- No supersession. ADR 0021 remains Proposed, and this ADR does not change its status.

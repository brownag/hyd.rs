# Atmospheric boundary non-convergence findings

## Scope and provenance

This note diagnoses the two failing atmospheric acceptance tests in Orchestration 8.2.1. The checkpoint in the prompt identifies `75d7df5`; the inspected checkout is at `7dbc60c` (`docs(planning): record atmospheric checkpoint`), with the prompt as the only untracked path before this note was created. Production code and execution plans were not changed. Reproduction used the checked-out Rust source. Iteration traces came from an isolated `/tmp/hyd-atmos-trace` copy with temporary `eprintln!` diagnostics; instrumentation did not touch this repository.

The archived source `planning/H1D_Src.zip` contains `WATFLOW.FOR`. Source excerpts below are primary-source context for algorithm shape only. There is no Fortran executable or calibrated oracle, so this diagnosis makes no parity claim.

## Reproduction

Commands run:

```sh
cargo test -p hydrus-core --test atmospheric accepted_rain_infiltrates_then_excess_builds_ponded_storage -- --exact --nocapture
cargo test -p hydrus-core --test atmospheric accepted_evaporation_is_limited_by_soil_supply_and_balanced -- --exact --nocapture
cargo test -p hydrus-core --test atmospheric -- --nocapture
```

Both focused tests fail with `NonConverged { timestep: 0.015625, minimum_timestep: 0.01, retries: 6 }`. The full atmospheric integration test has 7 passed and the same 2 failed. The rain test's first light-rain step succeeds; failure is its heavy-rain second step at test line 161. The evaporation test fails at its only `unwrap`, line 190. This matches the final failure type in [STATE_8.md](STATE_8.md#82.1--blocked-implementation-unaccepted), but this run does not reproduce the earlier recorded rain balance residual `0.003225806451612934`: current control flow never accepts that rain step and computes no accepted-step balance.

The failing scenarios are already small and deterministic: a 3-node uniform mesh `[0,1,2]`, constant-conductivity linear material (`theta=0.2+0.1h`, `K=1`, `C=0.1`), zero sink, no-flow lower row, one-hour timestep, and strict `1e-10` head/water tolerances with 8 Picard iterations. For rain, the accepted first step is 0.01 then the second forcing is 1.0 with critical head 0.1. For evaporation, demand is 0.2 with critical head -0.01. The nonzero forcing and a critical head that activates switching are load-bearing by construction; no further input minimization was attempted because the trace exposes a deterministic active-mode cycle in both original cases.

## Ranked hypotheses and evidence

The pre-probe hypotheses were:

1. **Active-mode oscillation:** mode selection alternates between flux and fixed head because it reads accepted pond storage instead of trial pond storage. **Confirmed, high confidence.** At every trial iteration `assemble` receives the last selected mode, but its ponded-storage fallback checks only `state.atmospheric.ponded_depth` ([water_solver.rs:319–345](../crates/hydrus-core/src/water_solver.rs#L319)). The accepted state is correctly unchanged until acceptance; consequently trial ponding cannot hold the active set during the nonlinear solve.
2. **Evaporation sign/regime mapping:** the negative net-demand branch may choose the wrong physical mode. **Not supported as the immediate cause; unresolved as scientific semantics.** Evaporation shows the same alternating flux/head pattern as rain: it enters head-limited mode when trial head falls below -0.01, then returns to flux mode at exactly -0.01 because calculated supply is less than demand in magnitude and accepted ponding is zero. This points to the accepted-vs-trial state gap rather than a rain-only sign defect. The source's `vTop` sign and `rTop` sign convention need explicit reconciliation before changing inequalities.
3. **Top-row, flux, and balance algebra disagree.** **Not established as the current failure cause.** The current failures occur before acceptance, so no balance is available. At the final minimum timestep, assembled modes imply incompatible heads; flux/balance algebra should be audited after active-set convergence is repaired. The historical balance residual remains an independent acceptance gap, not evidence that this is the cause of current non-convergence.
4. **Strict tolerances or too few Picard iterations.** **Rejected as primary cause, high confidence.** At every attempted timestep, including `0.015625`, iterations 2–8 repeat the same two solved head vectors and the same mode alternation. The residual remains macroscopic (`2.6908e-2` head, `2.0040e-2` theta at the rain minimum step; `5.2623e-3` head, `3.9192e-3` theta for evaporation), not a small residual just above tolerance. More identical Picard iterations cannot converge this 2-cycle.
5. **Ponded balance double counts storage.** **Not supported by the current formula, but prior balance issue merits separate regression coverage.** The accepted upper exchange is soil flux plus pond-storage change divided by `dt`, and the total storage change includes that same surface-storage delta. In the balance residual these terms cancel, leaving the soil-storage vs soil-flux relationship. Balance still cannot be validated until an accepted ponding trial exists.

## Per-iteration evidence

The isolated trace instrumented selected mode, previous trial top head, net forcing demand, computed soil/top flux, accepted pond depth, solved heads, and head/water residuals. Values below are at the initial `dt=1`; the minimum-step results confirm the same pattern.

| Case / iteration | Mode selected from current trial | Demand | Prior top head | Soil flux estimate | Solved heads `[bottom, middle, top]` | Max `dH` / `dTheta` |
|---|---|---:|---:|---:|---|---:|
| Rain / 1 | AtmosphericFlux | +1.0 | 0.0559814 | +0.0072009 | `[4.6955637, 4.9280190, 5.6483982]` | 4.87924 / 0.559242 |
| Rain / 2 | PondedHead | +1.0 | 5.6483982 | +1.0 | `[0.0910839, 0.0933152, 0.1]` | 4.83470 / 0.554840 |
| Rain / 3 | AtmosphericFlux | +1.0 | 0.1 | +0.0088857 | `[4.6955637, 4.9280190, 5.6483982]` | 4.83470 / 0.554840 |
| Rain / 4–8 | alternates each pass | +1.0 | 5.6483982 / 0.1 | +1.0 / +0.0088857 | repeats rows 2 / 3 | unchanged |
| Evaporation / 1 | AtmosphericFlux | -0.2 | 0.0 | 0.0 | `[-0.2583979, -0.2713178, -0.3927649]` | 0.271318 / 0.0382765 |
| Evaporation / 2 | PondedHead | -0.2 | -0.3927649 | -0.2 | `[-0.00657895, -0.00690789, -0.01]` | 0.264410 / 0.0382765 |
| Evaporation / 3 | AtmosphericFlux | -0.2 | -0.01 | -0.0085876 | `[-0.2583979, -0.2713178, -0.3927649]` | 0.264410 / 0.0382765 |
| Evaporation / 4–8 | alternates each pass | -0.2 | -0.3927649 / -0.01 | -0.2 / -0.0085876 | repeats rows 2 / 3 | unchanged |

For rain at the retry timestep `0.015625`, the two solutions are `[0.0808325, 0.0825512, 0.3004028]` in atmospheric-flux mode and `[0.0552055, 0.0556429, 0.1]` in ponded-head mode. Iterations 2–8 alternate; final residuals remain `dH=2.69083e-2`, `dTheta=2.00403e-2`. For evaporation at the same timestep, the solutions are `[-0.00629050, -0.00660502, -0.0491917]` in flux mode and `[-0.00127877, -0.00134271, -0.01]` in head mode; iterations 2–8 alternate; final residuals remain `dH=5.26231e-3`, `dTheta=3.91917e-3`. Accepted ponded storage is zero in both traces because acceptance is not reached.

## Root cause and source context

The proximate bug is an inconsistent active-set update. `solve_trial` correctly carries the latest mode into the next Picard assembly ([water_solver.rs:231–240](../crates/hydrus-core/src/water_solver.rs#L231)). However, in the positive-rain case, a flux solve pushes top head above the critical head and selects PondedHead; the head-constrained solve yields soil flux far below rainfall, which should imply excess water and retain ponding. Instead, because accepted ponded depth remains zero, selection falls through to AtmosphericFlux. Evaporation has the symmetric behavior: the supply-limited head-constrained solution cannot supply the full negative demand, but its trial storage is not available to retain that regime. Thus each switch returns the next linear solve to the same incompatible state.

`atmospheric_top_flux` includes a hydraulic-gradient term, a top half-cell storage term, and sink contribution ([water_solver.rs](../crates/hydrus-core/src/water_solver.rs#L405)); `atmospheric_top_row` builds the corresponding top half-cell row ([water_solver.rs](../crates/hydrus-core/src/water_solver.rs#L421)). Their detailed physical/sign parity remains unresolved. In the archived Fortran, `WATFLOW.FOR`'s `Reset` constructs the top row and computes `vTop` (archive lines 270–303); `Shift` re-evaluates the atmospheric boundary inside the nonlinear loop and changes `KodTop` using `vTop`, `rTop`, and `hCritA` (lines 392–427). `Shift` therefore supports the need for an iteration-level transition rule, but does not establish Rust's sign translation or switch correctness. Its logic must be interpreted alongside the row's `vTop=-ST*hNew(N-1)-RT*hNew(N)+PT` expression (line 293). No parity claim is possible without an executable and approved outputs.

Confidence is **high** that the accepted/trial mode-state mismatch causes the observed non-convergence. Confidence is **moderate** that both tests share that single proximate cause: both have the exact two-state active-set cycle, but their source boundary sign behavior still requires review. Unresolved items are source-to-Rust sign convention, the exact physical meaning of critical head for evaporation, the earlier reported rain balance residual, and whether existing top-row/storage algebra matches the intended HYDRUS water-only `Reset` contract.

## Recommended amendment to Orchestration 8

Amend task 8.2.1 before another implementation attempt; keep Waves 3–6 blocked until its acceptance passes. Split the current switching requirement (which currently says to reevaluate mode during nonlinear iteration, [EXECUTION_PLAN_8.md:84–86](EXECUTION_PLAN_8.md#L84)) into explicit subdeliverables:

1. **Boundary active-set contract:** document signed forcing and computed soil-flux conventions; define when flux changes to head and when head may return to flux for both precipitation and evaporation; identify which values are trial-local and which are accepted state. Calculate trial ponded/surface storage from the current trial, and use that candidate state in switching. Keep the candidate isolated from accepted state and promote it only with the rest of an accepted step; rejection must discard it.
2. **Top row and flux derivation:** derive Rust's row RHS, soil-flux expression, half-cell storage, and surface-storage/exchange balance against `WATFLOW.FOR` `Reset`/`Shift`; show the sign mapping algebraically. Mark undecidable source semantics for review rather than adjusting inequalities by intuition.
3. **Focused transition and trace tests:** retain current rain and evaporation scenarios; assert each transition's mode, trial top head, demand, soil flux, candidate surface storage, and convergence outcome. Add fixed-flux and fixed-head algebra cases that isolate top-row/flux consistency, plus a rollback assertion proving rejected candidate ponding/mode never mutates accepted history. Ensure iteration termination cannot accept a result whose active mode changed after the last assembled solve.
4. **Independent balance acceptance:** once the solver converges, assert soil storage, surface storage, signed soil flux, external atmospheric exchange, and residual as separate quantities. Preserve the near-zero conservation invariant but avoid treating total atmospheric demand as the soil boundary flux when ponding storage changes.

Suggested acceptance gate: both supplied exact tests pass, full `cargo test -p hydrus-core --test atmospheric` passes, and trace assertions show a stable mode plus both residuals under the configured criteria before acceptance; ponded rain and supply-limited evaporation separately satisfy balance with surface storage reported. Record that this remains contract-level, independently designed evidence rather than Fortran parity until a reference executable/output is available. Do not loosen tolerances or increase iteration limits as a substitute for resolving the observed active-set cycle.

## References

- Rust cases: [tests/atmospheric.rs](../crates/hydrus-core/tests/atmospheric.rs#L134)
- Rust active-set and trial loop: [water_solver.rs](../crates/hydrus-core/src/water_solver.rs#L223)
- Rust event/cursor promotion: [atmospheric.rs](../crates/hydrus-core/src/atmospheric.rs#L38)
- Task requirements: [EXECUTION_PLAN_8.md](EXECUTION_PLAN_8.md#L77)
- State and prior reports: [STATE_8.md](STATE_8.md#L43)
- Primary source archive: `planning/H1D_Src.zip`, `WATFLOW.FOR` `Reset` lines 270–303 and `Shift` lines 392–427 (archive line numbers).

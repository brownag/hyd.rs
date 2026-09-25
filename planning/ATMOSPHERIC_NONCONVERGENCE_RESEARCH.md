# Research prompt: atmospheric boundary non-convergence

Investigate the non-convergence of the two atmospheric boundary cases in Orchestration 8.2.1. This is a research task: diagnose the cause and recommend a plan amendment; do not implement a fix.

## Repository and checkpoint

- Repo: `/home/andrew/AI/hyd.rs`
- Branch: `phase4-water-acceptance`
- Checkpoint: `75d7df5` (`feat(core): checkpoint atmospheric boundary work`)
- The working tree was clean when this investigation was requested.

## Reproduction cases

Run each case separately, then run the whole atmospheric integration test:

```bash
cargo test -p hydrus-core --test atmospheric accepted_rain_infiltrates_then_excess_builds_ponded_storage -- --exact --nocapture
cargo test -p hydrus-core --test atmospheric accepted_evaporation_is_limited_by_soil_supply_and_balanced -- --exact --nocapture
cargo test -p hydrus-core --test atmospheric -- --nocapture
```

The tests are in `crates/hydrus-core/tests/atmospheric.rs` around lines 134–198.

The rain test has two steps:

- Light rain at `0.01` depth/time should infiltrate in atmospheric-flux mode, with no ponded storage and near-zero balance residual.
- Heavy rain at `1.0` should switch to ponded-head mode, hold the top head at `0.1`, accumulate ponded storage, and retain near-zero balance residual.

The evaporation test applies demand `0.2` from a dry surface with critical head `-0.01`. It expects head-limited evaporation: realized boundary flux between `-0.2` and `0`, no ponded storage, and near-zero balance residual.

Prior attempts reported:

- An earlier rain/evaporation run had 7 passing tests and 2 failures. The rain balance residual was `0.003225806451612934`; evaporation returned `NonConverged` after six retries at timestep `0.015625`.
- After a tentative switching/balance change, both new cases returned `NonConverged` after six retries at timestep `0.015625`.

Reproduce these results on the current checkpoint; do not assume they are unchanged.

## Relevant code and evidence

Use grep anchors and inspect only relevant sections:

- `crates/hydrus-core/src/water_solver.rs`: `solve_trial`, `assemble`, `atmospheric_top_flux`, `atmospheric_top_row`, `balance`, and retry handling.
- `crates/hydrus-core/src/atmospheric.rs`: event clipping, accepted cursor advancement, `net_flux`.
- `crates/hydrus-core/src/forcing.rs`: schedule/rate validation.
- `crates/hydrus-core/src/water_state.rs`: accepted atmospheric state and snapshot/restore.
- `crates/hydrus-core/src/hydraulics/boundary.rs`: generic endpoint row transformations.
- `crates/hydrus-core/src/hydraulics/stencil.rs`: source-shaped row algebra.
- `planning/EXECUTION_PLAN_8.md`, `planning/STATE_8.md`, and `planning/master/PHASE_3_WATER_SOLVER_EVIDENCE.md`.

The archived Fortran snapshot is `planning/H1D_Src.zip`. Inspect, for source-derived evidence only:

- `WATFLOW.FOR`, `Reset`, top row: approximately lines 270–303.
- `WATFLOW.FOR`, `Shift`, atmospheric mode switching: approximately lines 359–429.
- The main Picard loop calls `Reset`, then `Shift`, then solves the system each iteration.

There is no Fortran executable or calibrated numerical oracle. Do not claim parity.

## Investigation method

1. Establish a tight, deterministic reproduction for each test and capture the exact failure, including retries and final timestep.
2. Minimize each failing case by changing one input at a time. Preserve the original test scenarios.
3. Before probing hypotheses, write 3–5 ranked, falsifiable hypotheses. For each, state what measurement or one-variable experiment would support or reject it. Show that list to the user before testing; do not wait indefinitely for a reply.
4. Instrument only what distinguishes the hypotheses. Track, per Picard iteration:
   - active upper mode before and after assembly;
   - trial top head and critical head;
   - atmospheric demand and computed soil/top flux, with sign convention;
   - top-row coefficients and RHS;
   - head/water residuals and convergence decision;
   - ponded storage before/after acceptance;
   - retry timestep and reason.
5. Check source-to-Rust sign translation and whether the atmospheric top-row/storage terms match the water-only `Reset` contract. Examine whether mode changes, convergence residual selection, forcing demand, or balance accounting can cause oscillation or false convergence.
6. Do not change implementation code or acceptance expectations. Temporary, tagged instrumentation or a disposable harness may live only under `.development/8.2.1-diagnosis/`; remove it before finishing.

## Deliverable

Write findings to `planning/ATMOSPHERIC_NONCONVERGENCE_FINDINGS.md` with:

- exact commands and reproduction results for both cases;
- minimized inputs and the variables that proved load-bearing;
- ranked hypotheses, probes, and evidence for/against each;
- a per-iteration trace sufficient to explain the non-convergence;
- the most likely root cause(s), confidence, and any unresolved scientific ambiguity;
- a concrete recommendation for amending `EXECUTION_PLAN_8.md` and its acceptance criteria.

Stop after diagnosis and the plan-amendment recommendation. Do not edit the execution plan or production code.

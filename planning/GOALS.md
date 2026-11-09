# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-09
Source State: [STATE_23.md](STATE_23.md)

## Circled Goal (Active)

- **[G2]** The six final batches (one per fixed-axis group) execute and verify with the frozen tools and combined bounds, and the report records the accepted temporal gap.
  - Why now: G1 is complete; the campaign is the last source of study evidence before classifications and Gate C.
  - Exit criteria: All six batches have an execute result and `--verify-only` exit 0, or a recorded failure. `planning/master/M5V_FINAL_23.md` lists every batch ID, counts, and the `fewer_than_three_levels` gap for every time group. No holdout rows run.
  - First action: Finish the two Example 3 batches and verify them, then write the six-batch report.

## Completed

- **[G1]** The M5-V control and evidence blockers are resolved and the final run campaign is ready to start.
  - Result: D12/D13 decided; five control rows and the `23-final` root committed; tools frozen; combined bounds `3ac9fd8a...a6ee4` re-signed at Gate B on 2026-11-09.

## Sequenced Goals (Dependency DAG)

- **[G3]** The study findings have complete, verified classifications and documented error budgets, including unresolved fields and limits.
  - Depends on: G2
- **[G4]** Holdout-level thresholds are proposed from the accepted study evidence and approved by the user at Gate C.
  - Depends on: G3
- **[G5]** The three reserved holdout rows are run and compared with the approved thresholds.
  - Depends on: G4
- **[G6]** M5-V has an accurate closure record stating whether it closed or remains partial, with known limits and the M6 planning gate correctly stated.
  - Depends on: G5

## Independent Backlog

- **[G7]** The free-drainage end-row behavior is assessed against the source-defect register rules and recorded as a defect or modeling choice.

## Deferred

- **[G9]** Time-axis convergence is re-validated with three time-axis study levels or dual-axis centre reuse.
  - Trigger: after the M5-V closure record, or if the user prioritises closing the temporal claims.

## Blocked / Pruned

- **[G8]** Run holdout rows before Gate C approval.
  - Status: Pruned
  - Reason: Explicitly prohibited by Orchestration 23; `THRESHOLDS_V2.json` keeps `holdout_run_permitted: false`.

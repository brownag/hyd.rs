# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-07
Source State: [STATE_23.md](STATE_23.md)

## Circled Goal (Active)

- **[G1]** The M5-V control and evidence blockers are resolved and the final run campaign is ready to start.
  - Why now: The 23 study batches are stale under current tools, five holdout-level groups have no valid control, and the study bounds review is stale. No threshold or holdout work can be trusted until these are fixed.
  - Exit criteria: D12 and D13 are decided; any approved control rows and v2 checker/tests are committed; tools are frozen with recorded hashes; the combined study-plus-control bounds are regenerated, durable outside `/tmp`, and re-signed at Gate B. No holdout rows run.
  - First action: Obtain the D12/D13 decisions, then add the approved control rows and update the v2 checker/tests in one CASES_V2 change.

## Sequenced Goals (Dependency DAG)

- **[G2]** Six final batches (one per fixed-axis group) execute and verify with the frozen tools and combined bounds.
  - Depends on: G1
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

- None.

## Blocked / Pruned

- **[G8]** Run holdout rows before Gate C approval.
  - Status: Pruned
  - Reason: Explicitly prohibited by Orchestration 23; `THRESHOLDS_V2.json` keeps `holdout_run_permitted: false`.

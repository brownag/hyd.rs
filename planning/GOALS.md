# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-06
Source State: [STATE_23.md](STATE_23.md)

## Circled Goal (Active)

- **[G1]** The Orchestration 23 controls campaign has reconciled approval, bounds coverage, batch lifecycle receipts, verification results, and classification evidence in a tracked report.
  - Why now: The first classification was inconclusive, and the proposed controls/fixed-axis follow-up is the bottleneck for meaningful study classifications and any threshold proposal.
  - Exit criteria: A tracked report identifies the authorized campaign and its exact batches, confirms all required rows and controls have bounds or documents approved exclusions, records execute and verify exit statuses, and gives an evidence-backed classification or a precise blocker. No holdout rows run.
  - First action: Inventory the `23-controls-*` campaign artifacts and match each executed batch to its activation, launch ledger, classification, and verify-only receipt; check that the applicable approval covers the bounds and any analyzer changes.

## Sequenced Goals (Dependency DAG)

- **[G2]** The study findings have complete, verified classifications and documented error budgets, including unresolved fields and limits.
  - Depends on: G1
- **[G3]** Holdout-level thresholds are proposed from the accepted study evidence and approved by the user at Gate C.
  - Depends on: G2
- **[G4]** The three reserved holdout rows are run and compared with the approved thresholds.
  - Depends on: G3
- **[G5]** M5-V has an accurate closure record stating whether it closed or remains partial, with known limits and the M6 planning gate correctly stated.
  - Depends on: G4

## Independent Backlog

- **[G6]** The free-drainage end-row behavior is assessed against the source-defect register rules and recorded as a defect or modeling choice.

## Deferred

- None.

## Blocked / Pruned

- **[G7]** Run holdout rows before Gate C approval.
  - Status: Pruned
  - Reason: Explicitly prohibited by Orchestration 23; `THRESHOLDS_V2.json` keeps `holdout_run_permitted: false`.

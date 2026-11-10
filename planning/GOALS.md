# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-10
Source State: [STATE_23.md](STATE_23.md)

## Circled Goal (Active)

- **[G6]** M5-V has an accurate partial-closure record listing the failed, inconclusive, and unassigned fields, with the holdout decision and the M6 gate stated correctly.
  - Why now: The final campaign is complete; it produced 12 achieved, 40 failed, and the rest inconclusive findings, so a closure claim is not supportable.
  - Exit criteria: `PHASE_5_SOLUTE_EVIDENCE.md`, `VALIDATION_STRATEGY.md`, `ROADMAP_AND_RISKS.md`, `SOLUTE_CONVERGENCE_EVIDENCE.md`, and ADR 0019 state M5-V partial/open; no holdout was run; the Robin-space failures and temporal design gap are named; M6 is not unblocked.
  - First action: Write the partial-closure section and update the roadmap/ADR status lines without claiming a pass.

## Completed

- **[G1]** The M5-V control and evidence blockers are resolved and the final run campaign is ready to start.
  - Result: D12/D13 decided; five control rows and the `23-final` root committed; tools frozen; combined bounds `3ac9fd8a...a6ee4` re-signed at Gate B on 2026-11-09.
- **[G2]** Six final batches execute and verify with the frozen tools and combined bounds.
  - Result: all six verified 2026-11-10; batch IDs and counts are in `M5V_FINAL_23.md`.
- **[G3]** The study findings have verified classifications and documented error budgets.
  - Result: 12 achieved, 40 failed, 13,568 inconclusive across six groups; the temporal gap and floors are recorded.

## Deferred

- **[G9]** Time-axis convergence is re-validated with three time-axis study levels or dual-axis centre reuse.
  - Trigger: after the partial-closure record, or if the user prioritises the temporal claims.

## Independent Backlog

- **[G7]** The free-drainage end-row behavior is assessed against the source-defect register rules and recorded as a defect or modeling choice.

## Blocked / Pruned

- **[G4]** Holdout-level thresholds are proposed and approved at Gate C.
  - Status: Not requested
  - Reason: No holdout-level threshold is defensible from this campaign; the achieved fields do not establish the holdout level, Robin space failed, and the rest are inconclusive.
- **[G5]** The three reserved holdout rows are run against approved thresholds.
  - Status: Blocked
  - Reason: Gate C was not requested and `THRESHOLDS_V2.json` still has `holdout_run_permitted: false`.
- **[G8]** Run holdout rows before Gate C approval.
  - Status: Pruned
  - Reason: Explicitly prohibited; no holdout row ran.

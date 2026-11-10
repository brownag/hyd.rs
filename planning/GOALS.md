# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-10
Source State: [STATE_23.md](STATE_23.md)

## Circled Goal (Active)

- None. M5-V's partial closure is recorded; the next choice is a bounded temporal follow-up
  or M6 planning. M6 stays blocked until the user accepts the M5-V status.

## Completed

- **[G1]** The M5-V control and evidence blockers are resolved and the final run campaign is ready to start.
  - Result: D12/D13 decided; five control rows and the `23-final` root committed; tools frozen; combined bounds `3ac9fd8a...a6ee4` re-signed at Gate B on 2026-11-09.
- **[G2]** Six final batches execute and verify with the frozen tools and combined bounds.
  - Result: all six verified 2026-11-10; batch IDs and counts are in `M5V_FINAL_23.md`.
- **[G3]** The study findings have verified classifications and documented error budgets.
  - Result: 12 achieved, 40 failed, 12,888 inconclusive across six groups; the temporal gap and floors are recorded.
- **[G6]** M5-V has an accurate partial-closure record listing the failed, inconclusive, and unassigned fields.
  - Result: `PHASE_5_SOLUTE_EVIDENCE.md`, `VALIDATION_STRATEGY.md`, `ROADMAP_AND_RISKS.md`, `SOLUTE_CONVERGENCE_EVIDENCE.md`, ADR 0019, and the decisions index record partial/open; no holdout ran; M6 is not unblocked.

## Deferred

- **[G9]** Time-axis convergence is re-validated with three time-axis study levels or dual-axis centre reuse.
  - Trigger: after the M5-V partial closure, or if the user prioritises the temporal claims.

## Independent Backlog

- **[G7]** The free-drainage end-row behavior is assessed against the source-defect register rules and recorded as a defect or modeling choice.
- **[G10]** The 40 failed Robin-space fields are investigated as a solver or formulation issue.

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

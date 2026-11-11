# M5-V acceptance with limitations and M6 progression

Decision date: 2026-11-11  
Decision owner: Andrew G. Brown  
Status: **Accepted by the user**

## Decision

Accept the M5-V results **as they stand, with the limitations below**, and
unblock M6 planning and implementation. No additional M5 campaign, holdout,
threshold exercise, or Robin diagnostic is required by this decision.

The user judged that no actionable difference had been demonstrated. The
observed precision and timestep effects remain numerical caveats, not
blockers by themselves.

This accepts the M5-V gate for project progression; it does not convert
individual failed or inconclusive classifications into passes, nor assert that
every possible solute case or external use has been validated. Keep the original
classifications and evidence intact. M6 has its own capability-specific
implementation and acceptance work; M5-V acceptance does not imply M6 has
passed.

## Basis and caveats

- The final M5-V campaign recorded 12 achieved, 40 failed, and 12,888
  inconclusive findings. The post-fix Robin rerun recorded 39 nonmonotonicity
  failures and 151 inconclusive findings. These are retained as observations,
  not treated as blanket evidence of practical unfitness.
- In the exploratory 257-node Robin comparison at t=1, reducing `dt` from
  1/128 to 1/256 to 1/512 reduced the analytical-reference relative errors:
  concentration L-infinity `0.109% → 0.0382% → 0.0206%`, outlet flux
  `0.138% → 0.0484% → 0.0261%`, and inventory `0.0853% → 0.0126% → 0.00519%`.
  The last run is exploratory, not campaign evidence; inventory has no new
  rigorous quadrature bound.
- The Robin “failed” labels identify nonmonotone mesh refinement, not an
  exceeded, use-calibrated error limit. Most inconclusive labels reflect the
  campaign's uncertainty/bounds and design limitations. They do not establish
  that the differences are actionable for M6.
- The 44/44 Rust/Fortran parity result applies to its fixture suite. The Robin
  campaign's listed errors are against an independent analytical reference;
  there is no direct Fortran trajectory comparison for those exact Robin runs.
- Precision effects and timestep interactions remain caveats. They are recorded
  for future use-specific follow-up if evidence shows they matter; they do not
  block M6 by themselves.

## Follow-up rule

Do not expand or repeat M5-V work for campaign completeness. Reopen a specific
question only when a real supported-use case identifies a consequential output
or decision that current evidence cannot address. Any follow-up should state
what decision its result could change and use the smallest informative check.

M6 is unblocked for planning and implementation. M6 acceptance must still be
based on its selected capability and intended use; no universal tolerance or
M5-derived M6 threshold is introduced here. Broader claims about solute use,
experimental truth, release compatibility, and whole-algorithm fitness remain
outside this acceptance.

## References

- [ADR 0019](../../docs/decisions/0019-validate-scientific-accuracy-on-the-default-backend.md)
- [M5-V final campaign](M5V_FINAL_23.md)
- [Post-fix Robin rerun](M5V_ROBIN_RERUN_23.md)
- [M5-V next-step review](M5V_REVIEW_23_NEXT_STEPS.md)
- [Exploratory Robin timestep provenance](../../.development/23-classify-3/exploratory-robin-dt1-512/exploratory-provenance.json)

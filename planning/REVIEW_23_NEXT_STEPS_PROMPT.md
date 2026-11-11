# Frontier review prompt — M5-V next steps after Orchestration 23

You are a frontier review agent with fresh context. Review the current state of the
M5-V solute-validation work in `/home/andrew/AI/hyd.rs` and recommend the next bounded
work. Do not implement anything during the review. Produce a decision memo.

## Repository state

- Branch: `phase4-water-acceptance`
- HEAD: `1123993` (`docs(solute): correct the inconclusive finding count`)
- Working tree: clean
- Current casebook: `reference/solute_verification/CASES_V2.json`, SHA-256
  `420af7c0085f3ad5cf3335ca036ca5360d7abd370807451d8d479e653a215df7`
- Signed combined bounds: `.development/23-bounds/combined_bounds_23.json`, SHA-256
  `3ac9fd8a45018d850da20fefc2877093f8fcf73910b4a7133a54ec8a039a6ee4`
- Signed Gate B review: `.development/23-bounds/review-combined.json`, SHA-256
  `93db9cfceadb55eedd33b54771bc78d60d79bed553763d09e72fab8c3f2e15b1`
- No holdout row has run. `THRESHOLDS_V2.json` still has `holdout_run_permitted: false`
  and the stale `cases_sha256` `c236eb7c...`. M6 is not unblocked.

## Mandatory reading

Read these before forming an opinion:

1. `planning/master/M5V_FINAL_23.md` — the final campaign report, batch IDs, counts,
   achieved/failed fields, the temporal gap, and the no-threshold handoff.
2. `planning/STATE_23.md` — the continuation board and the 23R.1–23R.5 gate records.
3. `planning/master/M5V_BLOCKERS_23.md` — the control/evidence blockers and the temporal
   design gap.
4. `planning/GOALS.md` — current goal graph and the deferred temporal goal G9.
5. `planning/master/PHASE_5_SOLUTE_EVIDENCE.md` and
   `planning/master/VALIDATION_STRATEGY.md` — the partial-closure sections.
6. `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_23.md` and
   `planning/master/SOLUTE_CONVERGENCE_PLAN_23_AMENDMENT.md` — the earlier classification
   outcome and the proposed controls/grouping change.
7. `planning/master/M5V_ISOLATION_DESIGN_23.md` — the design whose “centre rows serve both
   axes” assumption failed.
8. `docs/decisions/0009-verify-solver-mathematics-against-independent-references.md`,
   `docs/decisions/0019-validate-scientific-accuracy-on-the-default-backend.md`.
9. The relevant tools only if needed: `tools/solute_convergence_analysis.py`
   (grouping, control, floor/rate logic), `tools/run_solute_convergence.py`,
   `tools/solute_convergence_evidence.py`.

## What happened

- The final campaign ran six verified batches with the signed combined bounds:
  diffusion space, diffusion time, Robin space, Robin time, Example 3 space at `dt .04`,
  and Example 3 time.
- Results: 12 `achieved` (diffusion-space S1 concentration norms), 40 `failed`
  (Robin-space S1 fields with non-monotone rates between the fine pair), and 12,888
  `inconclusive`.
- Every time group has only two study levels (`fewer_than_three_levels`) because the
  design's centre rows are `axis: space`, while the classifier groups by the row's axis.
  Decision D14 accepts this partial outcome.
- Example 3 is dominated by control contamination and arithmetic floors; it does not
  establish the claimed norms.
- No holdout-level threshold was defensible, so Gate C was not requested and no holdout
  ran.

## Hard constraints

- Do not run any `holdout` row, edit `THRESHOLDS_V2.json`, sign a gate, or change any
  thresholds/tolerances.
- Do not weaken assertions or declare success without evidence.
- Treat `.development/` as untracked evidence. Read it; do not delete or rewrite it.
- Any change to `reference/solute_verification/CASES_V2.json` or any of the 11 producer
  tools invalidates the producer/cases pins on **all** existing final batches, so a full
  re-run is part of the cost of any such change. Account for that explicitly.
- Keep the review read-only. If you propose an implementation, describe its files,
  gates, and acceptance checks; do not execute it.

## Questions the memo must answer

1. **Temporal path.** Compare:
   - adding three time-axis centre rows (97 at `2^-7` for diffusion, 193 at `2^-7` for
     Robin, 751 at `0.04` for Example 3), deriving their bounds from the existing
     space-centre runs, re-signing Gate B, and re-running everything; versus
   - changing the classifier to let a space-centre run serve as a time level, with tests
     and a full re-run; versus
   - leaving temporal convergence as a documented deferred gap.
   For each: expected scientific gain, cost, risk, whether control contamination would
   still block `achieved`, and the exact files/rows/tools that change.
2. **Robin failures.** Are the 40 failed Robin-space fields a solver/formulation defect, a
   boundary/metric artifact, or an expected non-monotone regime? Trace the failure using
   the batch classifications and the relevant `solute_fortran_compat`/`solute.rs` code
   and analytical reference. If it is a defect, name the likely site and the smallest
   diagnostic study. If it is not, state how the failure should be recorded so it does
   not look like a threshold miss.
3. **Diffusion-space achieved fields.** Do the 12 achieved fields support any defensible
   holdout-level threshold at the reserved 97-node row? If yes, give the exact fields,
   the observed errors/rates, the margin-rule proposal, and what a holdout would test.
   If no, say why and whether a narrower Gate C is still worth running.
4. **Example 3.** Is the control contamination (6,076 of 6,090 findings) a control-design
   problem, a real spatial error, or both? Is a different control or a finer reference
   the only way to get a meaningful Example 3 result? Estimate the cost.
5. **Next orchestration shape.** Recommend one of:
   - a bounded temporal fix + rerun;
   - a Robin root-cause/diagnostic orchestration;
   - an acceptance-and-M6 decision with the partial record;
   - a narrower combined plan.
   Give a wave/task DAG, file scopes, gates, escalation points, and exit criteria.
6. **Anything missing.** Identify any blocker, stale artifact, invalid gate, or missing
   evidence not already listed in `M5V_BLOCKERS_23.md` or `M5V_FINAL_23.md`.

## Deliverable

Write a decision memo with:

- **Verdict:** one paragraph on whether M5-V can close, and what the next orchestration
  should be.
- **Evidence check:** the facts you independently verified, with exact commands and
  file/SHA citations. Flag any claim in the planning files that you could not reproduce.
- **Options:** at least the temporal, Robin, Example 3, and accept-partial options, each
  with scientific gain, cost/time, risk, reversibility, and gate dependencies.
- **Recommendation:** a ranked choice with a concrete first task and exit criteria.
- **Proposed plan:** waves, task IDs, file scopes, targeted checks, and hard user gates.
- **Residual uncertainty:** what the recommendation depends on.

If you write the memo into the repository, place it at
`planning/master/M5V_REVIEW_23_NEXT_STEPS.md` and keep the rest of the tree untouched.
End with a short chat summary (10 lines or fewer) and the single next action you recommend.

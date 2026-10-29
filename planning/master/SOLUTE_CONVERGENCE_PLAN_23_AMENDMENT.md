# Orchestration 23, Wave 4: plan amendment (controls and grouping)

Status: proposed, not started. Supersedes the row lists in the 23.6.1 handoff where they differ.
Builds on: `planning/master/M5V_CLASSIFY_APPROVAL_23.md` (commit 9f3ca9f) and
`planning/master/SOLUTE_CONVERGENCE_EVIDENCE_23.md` (commit c86130c).

## Why

- The 23.6.1 campaign ran 18 study rows without control rows. Every finding is therefore
  `inconclusive` (`control_missing`), so nothing can be `achieved` or `failed`.
- 23.4.3 mixes two dt series (dt 0.025 and dt 0.04) in one space group. The analyzer
  refuses that (`co-refined study axes`). Its group key is `(case, solver, axis, precision)`,
  which does not include the fixed axis value.

## Blocker to resolve first

- `.development/23-bounds/bounds.json` has `bounds_by_launch` entries for the 18 study rows only.
  The six control rows have no bounds. The analyzer needs bounds per row, so the controls
  cannot be classified until bounds exist for them.
- Adding bounds is a change to the signed bounds derivation (`M5V_BOUNDS_23.md`, `build_bounds.py`).
  The approval does not cover that. Decision needed: derive and sign bounds for the control rows,
  or drop the controls and accept that the study stays `inconclusive`.

## Analyzer change (if approved)

- Group key becomes `(case, solver, axis, precision, fixed_axis_value)`, where the fixed value is
  `dt` for space rows and `nodes` for time rows. Each dt series (space) or nodes series (time)
  becomes its own group, and its own control applies.
- Add tests: mixed dt series in one batch produce two groups; a series without a control reports
  `control_missing`; the existing groups are unchanged.

## Rows (CASES_V2 order, controls included, new campaign ID)

Campaign ID: `23-classify-4` (fresh ledger; 23-classify, -2, -3 rows are consumed).

- 23.4.1: space 65, 97, 129 at dt 2e-7; time 97 at dt 2e-6 and 2e-8;
  controls `convergence-diffusion_cosine-space_control` and `convergence-diffusion_cosine-time_control`.
- 23.4.2: space 129, 193, 257 at dt 2e-7; time 193 at dt 2e-6 and 2e-8;
  controls `convergence-scalar_robin-space_control` and `convergence-scalar_robin-time_control`.
- 23.4.3: example3 space 251, 501, 1001 at dt 0.025 (control `convergence-example3-space_control`);
  space 501, 751, 1001 at dt 0.04 (no control exists; stays `inconclusive` by design).
- 23.4.4: example3 time 751 at dt 0.05 and 0.025; control `convergence-example3-time_control`.

## Steps

1. Decide the control-bounds question above. Stop if the answer is "drop the controls".
2. Implement the grouping change and its tests in `tools/solute_convergence_analysis.py`.
3. Change the campaign ID in all five lists: `PHASE_TASKS`, `CONVERGENCE_TASKS`, `ALLOWED_TASKS`,
   the Rust harness allowlist and test, and the test pin. Also `book_tasks` and `scope_task` already cover `23-*`.
4. Refresh producer hashes, bounds SHA, and reviews. Commit on each `orchestration/23.4.N` branch.
5. Pilot one small batch (23.4.4 time pair with its control) before the full run.
6. Execute and verify 23.4.1 to 23.4.4 one at a time. Save exit codes. Stop on the first failure.
7. Rewrite `SOLUTE_CONVERGENCE_EVIDENCE_23.md` for the new campaign. Keep the 23-classify-3 results as a record.

## Known costs and risks

- Every row reruns, including 23.4.1 and 23.4.2, because the analyzer changes.
- Example3 cold caches take hours. Expect a long run.
- The dt 0.04 series has no control, so its findings stay `inconclusive` by design.
- Fixed-axis grouping changes how existing evidence is interpreted. The old classifications
  remain valid only as the bounds-free record.

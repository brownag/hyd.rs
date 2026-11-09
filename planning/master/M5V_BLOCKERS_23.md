# M5-V blockers and continuation plan (Orchestration 23)

Date: 2026-11-07. Repository: `phase4-water-acceptance` at `2165eb9`.
Scope: reconciliation of the post-Wave-4 control campaign and the work still required
before M5-V can close. This report changes no code, cases, bounds, or evidence.

## Status in one line

The Wave 4 isolation runs exist, but they cannot be classified with the current tools and
most of them have no valid other-axis control. The remaining work is one controlled
CASES_V2 change, one tool freeze, a re-signed combined bounds source, and one final run
campaign before Gate C.

## Verified evidence

- `23-controls-8` ran the original 22 study levels plus their controls, without bounds.
- All six executable batches were re-verified at `2165eb9` with
  `run_solute_convergence.py --verify-only --batch <batch>` and returned exit 0
  (`evidence_verified`):
  - `batch-cc25a97ee38a433aac882fdc2bf52fb3` diffusion space
  - `batch-bc96093ecef04d10b2a141ae8927ac07` scalar_robin space
  - `batch-980de4772e034c2baac25f34a9465818` example3 space
  - `batch-96aec7b0f4ac402fa24c8b7396a8490e` diffusion time
  - `batch-6ff354ef8279445d9914a99c89906e4b` scalar_robin time
  - `batch-8effc6b84cf8418ea690e04d667b90b6` example3 time
- Control bounds are signed in
  `.development/23-worktrees/23.4.4/.development/23-bounds/review-controls-8.json`
  (SHA-256 `4289b507...46d2cf`), and its producer hashes match `2165eb9`.
- The control bounds file currently lives only at
  `/tmp/claude-1000/-home-andrew-AI-hyd-rs/2e5bc7ad-16b7-4ffc-8ad8-eca59a60be45/scratchpad/control_bounds_controls8.json`.
  It must be copied to a durable path and re-signed before the final campaign.
- Disk: 505 GB free. `gfortran` and Python 3.12 are available.

## Blockers

### B1. The 23 study batches are stale under the current tools

The six study batches from `23-classify-3` were produced before `5372004`. Their recipes
record producer hashes that differ from `2165eb9` in six files:

| File | Batch recipe | Current |
|---|---|---|
| `tools/check_solute_verification_spec.py` | `04299ff4...` | `77ec248c...` |
| `tools/run_solute_convergence.py` | `4c2cbded...` | `a0871cda...` |
| `tools/run_solute_verification_reference.py` | `e896a0e2...` | `032a2f1a...` |
| `tools/solute_benchmark_lifecycle.py` | `2a2a482b...` | `325935bf...` |
| `tools/solute_convergence_evidence.py` | `7d936a26...` | `3b5d2991...` |
| `tools/solute_reference_cache.py` | `99ec4e86...` | `96a6020c...` |

`ConvergenceBatch.load()` requires the recipe and current producer hashes to be equal, so
these batches cannot be verified or replayed at HEAD. The study rows must run again in a
new campaign after the final tool freeze.

### B2. The signed study bounds review is stale

`.development/23-worktrees/23.4.4/.development/23-bounds/review.json` binds to the old
producer hashes (`04299ff4...`, `4c2cbded...`, `2a2a482b...`). The bounds values are
unchanged, but the review cannot authorize classification at HEAD. Regenerate the bounds
`producer_sha256`, merge the control bounds, and re-sign at Gate B.

The committed `THRESHOLDS_V2.json` is also stale: it pins `cases_sha256` `c236eb7c...`,
while the current `CASES_V2.json` is `ceddd59d...`. The holdout guard therefore refuses
holdouts, which is the intended state until Gate C.

### B3. Five of six holdout-level groups have no valid control

The classifier accepts a finding only when the finest study row has a matching
other-axis control:

- space: same finest mesh, strictly finer `dt`;
- time: same finest `dt`, different (finer) mesh;
- all rows: common analysis times.

The existing controls were built for the original 22 levels, not the 23 holdout-bracket
levels. The gap is:

| Claimed group | Study levels | Required control | Existing control |
|---|---|---|---|
| diffusion space | 65/97/129 at `2^-7` | 129 at `2^-8` | none |
| diffusion time | 97 at `2^-6`, `2^-8` | 193 at `2^-8` | none |
| scalar_robin space | 129/193/257 at `2^-7` | 257 at `2^-8` | none |
| scalar_robin time | 193 at `2^-6`, `2^-8` | 257 at `2^-8` | none |
| example3 space `dt .04` | 501/751/1001 | 1001 at `.0125` | `convergence-example3-space_control-rfc-baseline` |
| example3 space `dt .025` | 251/501/1001 | 1001 at `.0125` | same control row, but a separate batch/root or a duplicate row is needed |
| example3 time | 751 at `.05`, `.025` | 1001 at `.025` | none |

Consequence: without new controls, every new-level finding stays `control_missing` and
`inconclusive`. Thresholds may still be proposed from observed errors, but no claimed
field can earn an `achieved` verdict, and closure would rest on weak evidence.

The amendment's proposed pairing (`23.4.x` rows plus the existing `space_control` /
`time_control`) does not satisfy these conditions for five groups.

### B4. A control-row change forces one more CASES_V2 SHA change

Adding the controls changes `CASES_V2.json`, so `THRESHOLDS_V2.json`, reference caches,
and the bounds review must be regenerated. The study re-run is already forced by B1, so
the incremental reference cost is limited to the new control configurations.

### B5. Fixed-axis grouping is not implemented, and the proposed change is incomplete

The analyzer still groups by `(case, solver, axis, precision)`, not by the fixed axis.
The `23.4.3` mixed-dt batch failed for that reason. The amendment's proposed group key
would put a control row (fixed-axis value `.0125`) in its own group, away from the
`.025`/`.04` study groups, so it does not by itself solve control pairing.

Simplest safe route: run one batch per fixed-axis value, each with its own control row or
its own campaign root. Do not change the analyzer unless a tested design also assigns
controls to the correct fixed-axis group.

### B6. The tool freeze is not in force

Every commit since the Wave 3 freeze changed producer hashes. The final campaign must
run after the last tool/case edit, with no later edits. Any edit after the final freeze
restarts B1 and B2.

## Recommended resolution (Option A)

1. Add five control rows to `CASES_V2.json` in one change:

| Proposed ID | Role | Axis | Nodes | dt | Analysis | Sampling |
|---|---|---|---|---|---|---|
| `convergence-diffusion_cosine-space_control-129-dt2e-8-rfc-baseline` | control | space | 129 | `2^-8` | 0.25, 0.5, 1 | 0.0625 |
| `convergence-diffusion_cosine-time_control-193-dt2e-8-rfc-baseline` | control | time | 193 | `2^-8` | 0.25, 0.5, 1 | 0.0625 |
| `convergence-scalar_robin-space_control-257-dt2e-8-rfc-baseline` | control | space | 257 | `2^-8` | 0.25, 0.5, 1 | 0.0625 |
| `convergence-scalar_robin-time_control-257-dt2e-8-rfc-baseline` | control | time | 257 | `2^-8` | 0.25, 0.5, 1 | 0.0625 |
| `convergence-example3-time_control-1001-dt0.025-rfc-baseline` | control | time | 1001 | 0.025 | 50, 100, 200 | 1 |

Example 3 space at `dt .04` uses the existing `convergence-example3-space_control-rfc-baseline`
(1001 at `.0125`). The `dt .025` re-run is optional and needs its own batch/root or a
duplicate control row.

2. Update the v2 checker/tests and the row manifests. No ceilings.
3. Freeze tools and record SHA-256 values.
4. Regenerate study bounds with current producer hashes; merge the control bounds into one
   source; copy the bounds out of `/tmp`; Gate B re-sign.
5. Run six final batches, one per group, each with its control; verify each.
6. Classify with the combined bounds; write error budgets and threshold proposals; Gate C.
7. Run the three held-back holdout rows only after Gate C; write the closure record.

Estimated incremental work for the new controls: about 24.3M work units, dominated by the
Example 3 time control (1001 nodes, 8000 steps, 3 species). The Example 3 reference
configuration is shared with the space study row, so the new control adds no new reference
family. All references are cold anyway because the checker hash changed.

## Option B (not recommended)

Do not add controls. Re-run the study rows against current tools and the signed study
bounds. Every new-level finding stays `control_missing`/`inconclusive`. Thresholds may be
proposed, but M5-V can only be recorded as partial with an explicit control gap.

## Decisions required

- **D12:** Add the five control rows and make the CASES_V2 change (recommended), or accept
  the control gap and partial closure.
- **D13:** Drop the Example 3 `dt .025` authenticated re-run from the critical path
  (recommended; the holdout threshold uses `dt .04`), or retain it in a separate root.
- **Gate B re-sign:** approve the combined study-plus-control bounds at the new producer
  hashes.
- **Gate C:** approve holdout thresholds only after classification. No holdout row runs
  before this.

## Readiness checklist

- [x] Current tree clean at `2165eb9`.
- [x] Latest control batches verified under current tools.
- [x] Control bounds signed against current producer hashes.
- [ ] Control bounds copied out of `/tmp` and included in the combined bounds.
- [ ] Five control rows added and v2 checker/tests updated.
- [ ] Tools frozen; producer hashes recorded.
- [ ] Combined bounds regenerated and re-signed at Gate B.
- [ ] Six final batches executed and verified.
- [ ] Classifications, error budgets, and threshold proposals written.
- [ ] Gate C approved; `THRESHOLDS_V2.json` regenerated with the new CASES SHA.
- [ ] Holdouts run once; closure record written.

## Temporal design gap discovered during the final campaign (2026-11-09)

The final campaign exposed a design/classifier mismatch:

- The 23.2.1 design has two time-axis study rows per case, bracketing the holdout `dt`
  (`2^-6` and `2^-8` for diffusion/Robin; `.05` and `.025` for Example 3).
- It intended the space-centre row to serve as the middle time level. The centre rows are
  `axis: space`, and the classifier groups levels by the row's `axis`, so the centre row
  never enters the time group.
- Every time group therefore has only two study levels and reports
  `fewer_than_three_levels`; temporal claims cannot be `achieved` in this campaign.

Decision **D14** (user, 2026-11-09): finish the current campaign and record the temporal
fields as inconclusive. Do not restart with new time-centre rows or a classifier change.
M5-V will be recorded partial if the remaining evidence cannot support those claims.

A future time-axis study needs either three time-axis study rows (including a centre row)
or a tested dual-axis centre-reuse rule. Either change alters the casebook or tools and
requires a fresh campaign.

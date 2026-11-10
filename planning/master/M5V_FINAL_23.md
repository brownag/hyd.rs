# M5-V final campaign report (Orchestration 23, task 23R.3)

Date: 2026-11-09. Repository: `phase4-water-acceptance` at `7f3744e` (campaign tools
frozen at `2381fde`). Scope: the six final convergence batches run with the signed
combined bounds. No holdout row was run. No threshold is proposed here.

## Run identity

- Cases: `reference/solute_verification/CASES_V2.json`, SHA-256
  `420af7c0085f3ad5cf3335ca036ca5360d7abd370807451d8d479e653a215df7`.
- Bounds: `.development/23-bounds/combined_bounds_23.json`, SHA-256
  `3ac9fd8a45018d850da20fefc2877093f8fcf73910b4a7133a54ec8a039a6ee4`.
- Signed review: `.development/23-bounds/review-combined.json`, SHA-256
  `93db9cfceadb55eedd33b54771bc78d60d79bed553763d09e72fab8c3f2e15b1`
  (user Gate B sign-off, 2026-11-09).
- Campaign root: `.development/23-final/`. Producer hashes are the 11 values recorded in
  `STATE_23.md` "23R.1 gate".
- Every batch used `--bounds-source` and `--bounds-review` as above and
  `--allow-reference-work` for its cold reference cache.

## Lifecycle and verify status

| Group | Batch | Execute | Verify | Launches |
|---|---|---|---|---|
| diffusion space | `batch-0793968d793e45ddb498c7c6d884bd9c` | 0 | 0 | 4 |
| diffusion time | `batch-293141b4667e42ed8c6686e79abf713c` | 0 | 0 | 3 |
| scalar_robin space | `batch-ad3ac50b1a8243b9836b99ffcd6e2f9f` | 0 | 0 | 4 |
| scalar_robin time | `batch-0bc52085e3c84ab0a1bf416c881ca950` | 0 | 0 | 3 |
| example3 space `dt .04` | `batch-e22e8c92d1564f81b357396d6768a9bf` | 0 | 0 | 4 |
| example3 time | `batch-652a978c087a4039bc388604df5fed59` | 0 | 0 | 3 |

Each `--verify-only` result is `evidence_verified`. No solver was retried and no
`--resume-batch` was used.

## Classification summary

| Group | Findings | achieved | failed | inconclusive | control bounded | floor bounded |
|---|---:|---:|---:|---:|---:|---:|
| diffusion space | 190 | 12 | 0 | 178 | 16 | 63 |
| diffusion time | 190 | 0 | 0 | 190 | 56 | 63 |
| scalar_robin space | 190 | 0 | 40 | 150 | 40 | 99 |
| scalar_robin time | 190 | 0 | 0 | 190 | 1 | 96 |
| example3 space `dt .04` | 6,090 | 0 | 0 | 6,090 | 0 | 374 |
| example3 time | 6,090 | 0 | 0 | 6,090 | 68 | 207 |

The dominant reasons are `control_contamination`, `plateau_or_uncertainty_overlap`,
`arithmetic_floor`, `quadrature_bound_missing`, and `nearzero_error`. The two time groups
are also blocked by `fewer_than_three_levels` (see below).

## Achieved fields

Twelve diffusion-space findings are `achieved`, all for species S1 and all with an
observed decrease consistent with about first order:

- `concentration.linf`, `concentration.l2`, `concentration.endpoint_linf`, and
  `concentration.interior_linf` at times 0.25, 0.5, and 1.0.
- Finest-level errors range from `1.683e-4` (`l2`, t=0.25) to `1.462e-3`
  (`linf`/`endpoint_linf`, t=1.0); observed rates are about 0.93 to 1.10.
- Control assessment is bounded for these fields. The other 178 diffusion-space findings
  remain inconclusive, mostly because the control difference exceeds the finest error
  (`control_contamination`) or the error sits inside the arithmetic bound.

## Failed fields

Forty scalar_robin space findings are `failed` with bounded controls. They are all S1:

- `boundary.instantaneous.1` at every sampled time from 0.1875 to 1.0.
- `concentration.linf`, `concentration.endpoint_linf`, and `concentration.interior_linf`
  at t = 0.25, 0.5, and 1.0.
- `concentration.l1` at t = 1.0; `concentration.l2` at t = 0.5 and 1.0.
- `inventory.total` from t = 0.375 to 1.0; `inventory.dissolved` from t = 0.875 to 1.0.

The failure reason is `nonmonotonicity`: the observed rate is positive between the coarse
pair and strongly negative between the fine pair (for example `1.289` then `-1.649` for
`concentration.linf` at t=0.25). This is a real failed convergence result for those
fields, not an inconclusive one.

## Inconclusive reasons and floors

- **Example 3 space:** every finding is inconclusive. The control difference exceeds the
  finest error for 6,076 of 6,090 findings, and 5,091 sit at an arithmetic floor. Many are
  also `nearzero_error`. The combined holdout-dt space study does not establish convergence.
- **Example 3 time:** every finding is inconclusive and also blocked by the temporal gap.
- **Diffusion/Robin time:** every finding is inconclusive; the temporal gap is the first
  reason for all 190 findings in each group.
- **Arithmetic floors:** the conservative arithmetic scale makes many small-error keys
  unclassifiable. This is a property of the signed bound, not a solver failure.
- **Quadrature:** every control row has `quadrature: null` by design, so keys that need a
  quadrature bound remain unclassifiable.

## Temporal design gap

The 23.2.1 design intended the space-centre row to serve as the middle time level. The
centre rows are `axis: space`, and the classifier groups levels by the row's axis. Each
time group therefore has only two study levels:

- diffusion time: `2^-6`, `2^-8`;
- scalar_robin time: `2^-6`, `2^-8`;
- example3 time: `.05`, `.025`.

Every time finding reports `fewer_than_three_levels`. Temporal claims cannot be
`achieved` or `failed` in this campaign. Decision D14 (user, 2026-11-09) records the
accepted partial outcome; a future temporal study needs three time-axis levels or a
tested dual-axis centre-reuse rule.

## Observations against the spec's expected behaviour

- Diffusion space shows the expected monotone decrease for the 12 achieved concentration
  fields, with rates near one.
- Robin space is non-monotone at the finest pair for 40 fields; the spec treats this as a
  failed refinement, not a plateau.
- The Example 3 combined space study is dominated by control contamination and arithmetic
  floors at the holdout `dt .04`; it does not establish the claimed norms.
- The time axis is unclassifiable by design, not by data quality alone.
- No holdout was run, so no claim is validated at the reserved holdout configuration.

## Inputs for threshold proposal

No holdout-level threshold proposal is made from this campaign:

- Only 12 diffusion-space fields are `achieved`, and they are achieved at a finest mesh
  of 129 nodes while the reserved holdout is 97 nodes. The study does not establish the
  holdout-level field.
- The 40 Robin-space fields are `failed`; a threshold cannot turn a failed refinement into
  a pass.
- All temporal fields are unassigned by the design gap.
- All Example 3 fields are inconclusive.
- No threshold is widened, fitted, or inferred from holdout data.

Gate C is therefore not requested. `THRESHOLDS_V2.json` remains the stale study-level file
(`cases_sha256` `c236eb7c...`, `holdout_run_permitted: false`) and is not used for a
holdout run.

## Limits and handoff

- One compiler/platform; f32 study precision; the 50 null study quadrature keys and all
  control quadrature keys remain unclassifiable.
- Control contamination dominates Example 3 and much of diffusion/Robin; the signed
  arithmetic scale is conservative and produces arithmetic floors on small errors.
- The O13 `--verify-only` limit and the dropped Fortran-only `sensitivity` row remain as
  recorded.
- M5-V remains partial/open. M6 is not unblocked. The closure record belongs to 23R.5 and
  must not claim a pass.

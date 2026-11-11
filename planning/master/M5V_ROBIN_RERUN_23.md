# Post-fix Robin spatial-refinement rerun

Date: 2026-11-11. Status: **evidence verified; refinement remains failed/inconclusive**.
This report records only the authorized Robin spatial-refinement rerun. It does
not accept scientific accuracy, assign thresholds, close M5-V, or supersede the
pre-fix campaign record.

## Scope and authorization

Executed only the three `scalar_robin` space study rows (129/193/257 nodes at
`dt=0.0078125`) and their 257-node `dt=0.00390625` space control. The casebook
SHA-256 is `420af7c0085f3ad5cf3335ca036ca5360d7abd370807451d8d479e653a215df7`;
`chemistry.upstream_weighting` is `true` and the new V2 public request carries
that selector through `FortranCompatible`.

The user authorized the rerun on 2026-11-11, approved the plan, and signed the
scoped Gate B review. Bounds SHA-256:
`d29475495cbc147675857155feaef9d1db2b2d64a97c03d1385164c001490ae3`.
Signed review SHA-256:
`d476498ec4cdf58bfbf053922cac2db45f269d8b205c31a8f3514953060970f1`.
The review requires the fixed schedules and every measured magnitude at most 1.
Observed step counts matched: 128 steps for each study row and 256 for the
control; the largest measured magnitude was `0.9964261302794535`. All
quadrature bounds were null by the signed proposal, so affected findings remain
unclassifiable.

## Commands and lifecycle result

Analytical reference regeneration was limited to the three Robin mesh grids in
`.development/23-classify-3/reference-cache`; all three references were
`admitted`. No solver was started during this preparation. The campaign batch
then ran with the signed bounds/review and without `--allow-reference-work`:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_convergence.py \
  --cases reference/solute_verification/CASES_V2.json \
  --output-root .development/23-classify-3 --execute \
  --activation-file .development/23-robin-rerun/activation-robin-space.json \
  --bounds-source .development/23-robin-rerun/bounds-robin.json \
  --bounds-review .development/23-robin-rerun/review-robin-draft.json
```

Exit status: 0; batch status: `evidence_complete`. Batch:
`.development/23-classify-3/batch-25a12c50ed6a48b7a80d896512de505e`.
Activation SHA-256:
`efcf723272e511a61883a168276ba715f00d44dc48647f54ebf4c52b0c82efbf`.
Recipe SHA-256:
`7159ecefeb124039eaa2d61f0b7e085be0a8277465e741a4194e4e0b6a29fe41`.

| Launch | Run ID | Status | Observations SHA-256 |
|---|---|---|---|
| `convergence-scalar_robin-space_129-dt2e-7-rfc-baseline` | `convergence-scalar_robin-space_129-dt2e-7-rfc-baseline-e1eb55fcefb6` | complete | `4df1b02701ed93003333839b8b3cb39786735f758c838d4209c4fcc36ebe3e06` |
| `convergence-scalar_robin-space_193-dt2e-7-rfc-baseline` | `convergence-scalar_robin-space_193-dt2e-7-rfc-baseline-eaf181ed4cb6` | complete | `5e7896303f6cdfac4807031e83187533e0ce429f08927bfb0cd0943c1288de88` |
| `convergence-scalar_robin-space_257-dt2e-7-rfc-baseline` | `convergence-scalar_robin-space_257-dt2e-7-rfc-baseline-772c36f6fdbe` | complete | `83c71f7b914c7ba0b811ecd0a92ad5069d5b952eb19b8a34a9bd596586afe7be` |
| `convergence-scalar_robin-space_control-257-dt2e-8-rfc-baseline` | `convergence-scalar_robin-space_control-257-dt2e-8-rfc-baseline-3faf2b337671` | complete | `4ba53a45148525c0949fb1b9267e3433ba8d04b0c7d7d594f8161eea388ff258` |

The batch verifier was run on that exact batch:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_convergence.py \
  --cases reference/solute_verification/CASES_V2.json \
  --output-root .development/23-classify-3 --verify-only \
  --batch .development/23-classify-3/batch-25a12c50ed6a48b7a80d896512de505e
```

Exit status: 0; verifier status: `evidence_verified`; filesystem receipts were
authenticated. The verification output preserves the exact current producer
map in the batch recipe and summary.

## Findings

| Status | Count |
|---|---:|
| Achieved | 0 |
| Failed | 39 |
| Inconclusive | 151 |

All 39 failures are `nonmonotonicity`. They include `boundary.instantaneous.1`
at 14 sampled times, concentration endpoint/interior/infinity norms, and
inventory totals. For `concentration.linf` at t=0.25, errors at 129/193/257
nodes are approximately `3.51924e-4`, `2.16383e-4`, and `3.35712e-4`.

The 151 inconclusive findings reflect the intentionally null quadrature terms,
control contamination, arithmetic/reference floors, and uncertainty overlap.
No finding is achieved. The result is not evidence that the fixed mapping
resolved the Robin refinement failure; the remaining nonmonotonicity is direct
post-fix evidence that the mapping fix alone did not remove the failures.

Classification SHA-256:
`0f0ccf30d6b173959b958fc7ddf1bca32854e1075c2a18404ab5bfeb0686e21a`.
Summary SHA-256:
`691474f27ae7b1b8ecc554b34a0dfcf86e693d717ae33806202a3365861f1e20`.
Summary ledger SHA-256:
`fec7ac00803e6b98bf61f9935a7063e551457e0581f4b2eec662c9760f75c958`.
M5-V remains partial/open; no threshold or holdout was used.

## Historical evidence and limits

The original batch `batch-ad3ac50b1a8243b9836b99ffcd6e2f9f`, its signed
review, outputs, and 40-failed/150-inconclusive classification were not
modified. Those remain findings about the pre-fix mapping. This report records
the new 39-failed/151-inconclusive result separately. No Robin-time, other
case, holdout, or threshold run occurred.

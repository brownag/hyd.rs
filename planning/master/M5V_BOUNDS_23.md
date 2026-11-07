# M5-V bounds draft (Orchestration 23, task 23.5.1)

Status: draft for gate B. No review file has been written. No finding is classified. The user signs the review at gate B, not this document.

## Scope

- 18 study launches from the verified Wave 4 batches. These are the rows that 23.4.1 to 23.4.4 executed and verified.
- Batches, with verify exit 0 (`evidence_verified`):
  - `batch-160463d79070463db5d2788339450de9`: diffusion space 65, 97, 129 (dt 2e-7)
  - `batch-b9356e1aac134e4993266b5e18d70338`: diffusion time 97 (dt 2e-6, 2e-8)
  - `batch-771f357ed2d54b5787170d0c267c82e6`: scalar_robin space 129, 193, 257 (dt 2e-7)
  - `batch-a5475a714c964eb7a283c191ae5c883a`: scalar_robin time 193 (dt 2e-6, 2e-8)
  - `batch-17ae1e5216e645ea8e135b4689d0dbfd`: example3 space 251
  - `batch-36f7b3c2b0774b69a7b0409b99dc9d32`: example3 space 501, 1001
  - `batch-9b4698e478424952bb485e9927fdbe54`: example3 space 501, 751, 1001 (dt 0.04)
  - `batch-0b82b0506b7f4b9bb7ae66bd23dcb841`: example3 time 751 (dt 0.05, 0.025)
- The `space_control` row is not a study row and gets no bounds entry. Its control assessment is handled by the classifier.
- Holdout rows are not in scope and were not read.

## Inputs bound to

- `cases_sha256` = `ceddd59db73e1f54abbfd4198b0b5923ef9838b365bf8c96b691f5ad9718cab7` (frozen CASES_V2).
- `producer_sha256`: 11 tool files, copied from the classifier's own output and checked against the current file hashes. No mismatch.
- Draft file: `/home/andrew/AI/hyd.rs/.development/23-bounds/bounds.json`, SHA-256 `234333512ffee95b620c97b34f365419f7307dd15348d6c0af62a69421fa32cf`. Generator: `/home/andrew/AI/hyd.rs/.development/23-bounds/build_bounds.py`.

## Method, per term

Each bound adds a half-width to the error interval. Each entry covers one `(species, observable, time)` per launch.

**Arithmetic (every entry).** `arithmetic = gamma_N * S`, where:
- `gamma_N = N u / (1 - N u)`, with `u = 2^-24` (f32 unit roundoff).
- `N = (solver steps with time <= t) + (mesh nodes)`. Steps come from the run's `observations.json`.
- `S = max(1, max |value|)` over that key's levels. This is the scale of the quantity, not a tight rounding bound.

This is a worst-case accumulation bound, not an empirical fit. The scale choice is conservative, and it is what makes the inventory and boundary bounds large (see Observations).

**Coordinate (every entry).** `0.0`. Every study row is `rust` with an exact request mesh (`profile_coordinate_uncertainty.status = exact_request_mesh`), so no coordinate interpolation enters the metric. The classifier does not need this term for these rows.

**Quadrature (only `boundary.cumulative.1`, `reaction.loss`, `reaction.production`).** Empirical. The value is the `quadrature_sensitivity` the batch classification already records for that key, from the coarsening estimate in the spec (`SOLUTE_VERIFICATION_SPEC.md`, around line 403). This is not a rigorous mesh-weight bound. The spec allows empirical sensitivities, and the classifier's own uncertainty interpretation says so. Gate B may accept or decline this term per launch.

Where no sensitivity exists, the entry is `null` (missing). That field then stays unclassifiable for that key.

## Counts

| Launch group | Entries per launch | Keys with null quadrature |
|---|---|---|
| diffusion space and time (5 launches) | 190 | 1 each (`S1 reaction.loss t=0.0625`) |
| scalar_robin space and time (5 launches) | 190 | 1 each (`S1 reaction.loss t=0.0625`) |
| example3 space and time (8 launches) | 6,090 | 5 each (`NH4` and `NO2` `reaction.loss` at t = 1.0, and others) |

Total: 50,620 entries, 50 null quadrature keys.

## Observations

- Arithmetic bounds are below 5e-5 for the diffusion and scalar_robin rows. For example3 they reach 0.046 at the largest keys, which is larger than most of the measured errors (around 1e-3 at these levels). Those fields are likely to show `arithmetic_floor`. That is a result of the bound, not a bug.
- The largest arithmetic bounds are on `boundary.cumulative.0` (784 entries above 0.01), `inventory.total` (754), `inventory.dissolved` (269), `reaction.loss` and `reaction.production` (267 each), and `inventory.equilibrium_sorbed` (158). These come from the scale `S`, since inventories are in mass units.
- The 50 null quadrature keys block closure for those keys. A decline at gate B is recorded as blocking closure (D6).

## Verification

- JSON parses. Schema is `solute-convergence-bounds-v1`.
- `cases_sha256` equals the frozen value.
- `producer_sha256` equals the current tool hashes.
- All 18 Wave 4 study launch IDs appear in `bounds_by_launch`.
- No review file has been written.

## For gate B

The user reviews the derivations above by launch and observable, and then decides:

1. Accept the arithmetic method, including the scale `S` choice. Or ask for a tighter scale.
2. Accept the empirical quadrature sensitivities. Or decline them for named fields (D6).
3. Accept the 50 null quadrature keys as unclassifiable, or ask for them to be filled.

Once the user signs, the review file is written against this bounds SHA. Classification (23.6.1) happens only after that.

## Gate B: signed (2026-10-27)

- User accepted all three points: the arithmetic method with scale S, the empirical quadrature sensitivities, and the 50 null quadrature keys as unclassifiable.
- Review file: `.development/23-bounds/review.json`, SHA-256 `b33508c633a1bc938fe39ae1f5575cf9265589ae502ee8c789758fc1e19c83a1`. It binds to the bounds SHA `234333512ffee95b620c97b34f365419f7307dd15348d6c0af62a69421fa32cf` and to the frozen CASES_V2 and producer hashes.
- Classification (23.6.1) has not run. The runner writes a classification only during an execute, and a fresh execute makes a new batch. See the TODO status.

## Gate B re-draft (23R.2, 2026-11-07)

Status: **unsigned draft for Gate B**. This section supersedes the input identities in the
2026-10-27 body, but not its arithmetic method or its signed study values. No finding has
been classified against this draft.

### Inputs bound to

- `cases_sha256` = `420af7c0085f3ad5cf3335ca036ca5360d7abd370807451d8d479e653a215df7`
  (`reference/solute_verification/CASES_V2.json`, 58 launches / 7 cases).
- Combined bounds: `.development/23-bounds/combined_bounds_23.json`,
  SHA-256 `3ac9fd8a45018d850da20fefc2877093f8fcf73910b4a7133a54ec8a039a6ee4`.
  Generator: `.development/23-bounds/build_combined_bounds.py`.
- Durable copy of the signed six-control bounds:
  `.development/23-bounds/control_bounds_controls8.json`,
  SHA-256 `4289b507bc2be32544be03043a92d5885147c876a464244a6d48dd14c346d2cf`
  (byte-identical to the `/tmp` original).
- New bounds-free pre-run batch: `batch-222c1bdaa1c244acbdeb9c42ccd5df0b`
  (`.development/23-controls-8/`), verify exit 0, `evidence_verified`.
- Example 3 time-control source: launch `convergence-example3-space_1001-rfc-baseline`
  in the authenticated `batch-980de4772e034c2baac25f34a9465818`
  (`.development/23-worktrees/23.4.4/.development/23-controls-8/`). Same configuration:
  1001 nodes, `dt = 0.025`, sampling 1, analysis `[50,100,200]`, f32,
  `fortran-compatible`. That configuration was not re-run.

### Inventory

- 24 launches: 18 study rows, the five new control rows, and the existing
  `convergence-example3-space_control-rfc-baseline`.
- 63,560 bound entries (50,620 study + 12,940 control).
- The five other old controls in the durable copy are retained as evidence but are not
  part of the combined source; the final campaign does not use them.

### Method

- Study entries: values and `arithmetic`/`coordinate`/`quadrature`/`evidence` are
  unchanged from the 2026-10-27 bounds. Only `cases_sha256` and `producer_sha256` changed.
- Four cheap controls (diffusion space 129 at `2^-8`, diffusion time 193 at `2^-8`,
  Robin space 257 at `2^-8`, Robin time 257 at `2^-8`): `arithmetic = gamma_N * S`,
  `gamma_N = N u / (1 - N u)`, `u = 2^-24`, `N = steps<=t + nodes`,
  `S = max(1, |value|)`, from the verified pre-run above.
- Example 3 time control: the same formula applied to the authenticated
  space 1001/0.025 values and its `observations.json` step count, because the row
  configuration is identical.
- Existing Example 3 space control: signed `arithmetic` values preserved exactly;
  the signed provenance fields (`value`, `N`, `scale`, `batch`, `launch_id`) are folded
  into the canonical `evidence` string.
- `coordinate = 0.0` for every entry (rust rows use the exact request mesh).
- `quadrature = null` for every control entry. The 50 null study quadrature keys accepted
  at the 2026-10-27 gate are unchanged.

### Producer hashes (HEAD `2381fde`, match the 23R.1 freeze)

```
tools/check_solute_benchmarks.py            6e69b5adcdea925fbecb7e9c4741a64d1d354f326ba6d475087e79f3f580f68f
tools/check_solute_verification_spec.py     fa8adcf78dee760f84df510781b3cba098dec0eb0bfa3bb3d824fd397aea6c33
tools/run_solute_convergence.py             a0871cda8001af19ff8b4586a7f86b43b4ac5c889a029766409688552d4b6170
tools/run_solute_verification_reference.py  27cbafd86d11ac9e37a6c4cea8034385bcafeef181204ddb88b6be8be9cf5b56
tools/solute_analytical_reference.py        873d0df6ae69d40c432dd0b39c7cf401f9ff04f3069fb9187e321bc36e219b06
tools/solute_benchmark_bridge.py            caa9cad3661286047a3d2a5f6bbd5f3c1b11044489273e1c38d1007ee5f4d0fa
tools/solute_benchmark_lifecycle.py         325935bf31add8a5e7edf40d18f0aa25d4b7525871368d87d099bb1f80123d73
tools/solute_convergence_analysis.py        bd905699036d826e4b04f713a2a4a553f3562257a872888413d648abc3a348d4
tools/solute_convergence_evidence.py        87bc4ba4e474ea2372b89f4e77d6575cdd02250258ab4d96765a68dbac8de86e
tools/solute_reference_cache.py             c5acd63dd44390f76d8152c448ed2bdfc5aa8f8f739dcdde4ee77ed0627bff46
tools/solute_verification_metrics.py        609a18e04b07cbaeff1b4c75fabcae4710b995bf742dceca0fcfaa94c451dac0
```

### Verification performed

- Combined JSON parses; top-level keys are exactly
  `schema_version cases_sha256 producer_sha256 bounds_by_launch`.
- `cases_sha256` equals the new CASES_V2 SHA; `producer_sha256` equals
  `solute_convergence_evidence.provenance(include_cli=True)`.
- Exactly the expected 24 launch identities appear; every entry has the canonical seven
  keys and a non-empty `evidence`; every species/time/observable is valid for its row.
- All 18 study entries are byte-identical to the 2026-10-27 values.
- Every control entry has `quadrature: null`.

### For Gate B

The combined review is a draft at
`.development/23-bounds/review-combined-draft.json` with a clearly unsigned
`authorization` string. It binds to the combined bounds SHA above, the new CASES SHA, and
the HEAD producer hashes. It must not be used for classification until the user signs it.

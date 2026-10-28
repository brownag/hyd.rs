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

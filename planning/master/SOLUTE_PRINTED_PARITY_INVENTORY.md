# Solute printed-output parity inventory

Read-only provenance inventory for six canonical retained Fortran/Rust pairs. Hash and schedule authentication does not itself establish value parity, continuum accuracy, or physical validity.

- CASES: `reference/solute_verification/CASES.json` (SHA-256 `d3b06531c06545ca89956b95c487bb1616ace1470ba1a8c02a3f1209ff4e8acf`)
- Canonical pairs: 6 / 6 executable cases.
- Supplemental convergence run records: 36 (noncanonical).

## Canonical pairs

| Case | Pair source | Fortran status | Rust status | Nodes | Species | Common profiles | Common endpoints | Steps |
|---|---|---|---|---:|---:|---:|---:|---:|
| `reaction_distinct` | `batch-043cf43e84c04448a38e452670278cba` | `raw_complete` | `complete` | 9 | 3 | 33 | 129 | 128 |
| `reaction_equal` | `batch-043cf43e84c04448a38e452670278cba` | `raw_complete` | `complete` | 9 | 3 | 33 | 129 | 128 |
| `reaction_zero` | `batch-043cf43e84c04448a38e452670278cba` | `raw_complete` | `complete` | 9 | 3 | 33 | 129 | 128 |
| `diffusion_cosine` | `batch-043cf43e84c04448a38e452670278cba` | `raw_complete` | `complete` | 65 | 1 | 17 | 4097 | 4096 |
| `scalar_robin` | `batch-043cf43e84c04448a38e452670278cba` | `raw_complete` | `complete` | 129 | 1 | 17 | 257 | 256 |
| `example3` | `batch-c96c2d089400481a913a4accb6f1650f` | `raw_complete` | `complete` | 1001 | 3 | 4 | 0 | 2000 |

Each canonical record in the JSON contains exact manifest/request paths and hashes, all declared artifact hashes and byte counts, launch-row identity, source field formats, and available schedules. Component batch path `...278cba` is authenticated by its recipe and Orchestration 13 evidence; the `...278c4` spelling in Plan 14 is a typo.

## Validation failures

Failed rows are noncanonical and their raw outputs are excluded from comparison inputs.

| Case | Failure class | Status | Failed manifest records (path, status, hash) | Disposition |
|---|---|---|---|---|
| `example3` | `endpoint observation ids/order mismatch` | `failed` | `.development/13.4.1/study/batch-96dad43bcd3f43c7b87cfc16953f065f/runs/convergence-example3-space_1001-fortran-baseline-7be24ae04ca6/manifest.json` (`failed`, SHA-256 `cabdf389c2114f708649e9cfd2340a5f47c40fe87c8141df433b5928b6c503e9`); `.development/13.3.1/benchmarks/batch-252f19ab8f56442a808e16e3bbad714b/runs/benchmark-example3-component-fortran-baseline-b40c1111dbf8/manifest.json` (`failed`, SHA-256 `175c7b265a71796ef84232f6bd0e861ecc2210af523ae6423c903fd755d6727b`) | Example 3 full/component and 1001-node convergence failed rows are excluded; canonical Example 3 bridge pair is retained, so this failure does not justify a replacement run. |
| `scalar_robin` | `incomplete step inventory` | `failed` | `.development/13.4.1/study/batch-8cc54a32b4274287a8df86950a8c93f6/runs/convergence-scalar_robin-time_0.0625-fortran-baseline-971d11df459d/manifest.json` (`failed`, SHA-256 `0372804104fc8a0b0b811a7fc34268d094bb3aac6e78d3c90efa510e25bbe4aa`) | Supplemental failed row excluded from canonical comparisons; retained canonical pair remains the comparison source. |
| `diffusion_cosine` | `water inventory mismatch` | `failed` | `.development/13.4.1/study/batch-0a120fb95b2549128d683305726b86c8/runs/convergence-diffusion_cosine-time_control-fortran-baseline-cc0c50689bd1/manifest.json` (`failed`, SHA-256 `eddf4573b60d754060d4a532cd6578c9d6ea0be809557d81760003a47d21bc8a`) | Supplemental failed row excluded from canonical comparisons; retained canonical pair remains the comparison source. |

## Supplemental convergence records

Every retained convergence manifest is listed in `SOLUTE_PRINTED_PARITY_INVENTORY.json` under `supplemental.runs`, including exact launch/run identity, batch, schedule, status, manifest path, and manifest hash. These records remain supplemental; failed rows are excluded. None is promoted to a canonical pair by this inventory.

## Scope notes

- Canonical artifacts are authenticated against CASES launch rows, exact request bytes, manifest identity, and every declared artifact SHA-256 and byte count.
- Pair schedules are matched by validated node count, species count, common profile/endpoint times, and step count.
- Fortran inactive nonequilibrium `Sorb` columns are not Rust equilibrium `Kd*C` fields.
- No solver, build, benchmark, or analytical-reference process was started by this inventory.

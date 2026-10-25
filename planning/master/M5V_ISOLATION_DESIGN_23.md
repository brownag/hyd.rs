# M5-V isolation study design (23.2.1)

Report only. No rows are created, run or edited here. This design is the input to 23.3.1, which adds the rows to `reference/solute_verification/CASES_V2.json` in the single SHA change (D4). Holdout rows are not run (A6).

## Holdout levels

| Case | Holdout nodes | Holdout dt | Holdout row ID (existing, reserved) |
|---|---|---|---|
| diffusion_cosine | 97 | 2^-7 (0.0078125) | `holdout-diffusion_cosine-holdout-rfc-baseline` |
| scalar_robin | 193 | 2^-7 (0.0078125) | `holdout-scalar_robin-holdout-rfc-baseline` |
| example3 | 751 | 0.04 | `holdout-example3-holdout-rfc-baseline` |

## Existing study levels (from `CASES_V2.json`, read-only)

- diffusion_cosine space: 17/33/65 at dt 2^-12 (controls 65 at 2^-13). Time: 257 nodes at dt 2^-4, 2^-5, 2^-6 (control 513 at 2^-6). time_end 1.
- scalar_robin space: 33/65/129 at dt 2^-8 (control 129 at 2^-9). Time: 257 nodes at dt 2^-4, 2^-5, 2^-6 (control 513 at 2^-6).
- example3 space: 251/501/1001 at dt 0.025 (control 1001 at 0.0125). Time: 1001 nodes at dt 0.2/0.1/0.05 (control 501 at 0.05). time_end 200.
- Bridge `benchmark-example3-bridge-rfc-baseline` (1001 nodes, dt 0.1): not a claimed field; no isolating row.

None of the existing levels sits at a holdout mesh or dt. The holdout values lie between existing levels, so the design adds bracket rows on both sides and a centre row.

## Design rule

- Space rows hold the holdout dt and vary the mesh, bracketing the holdout mesh.
- Time rows hold the holdout mesh and vary the dt, bracketing the holdout dt.
- Each bracket is isolated only when its centre row is present. The centre row is the holdout mesh at the study dt (space), or the holdout dt at the study mesh (time). Centre rows are role `study`, not `holdout`.

## Proposed rows

All rows: `backend: fortran-compatible`, `precision: "f32"` (D4), `time_controls` policy `fixed`, `timeout_seconds` 60. Analysis times and sampling interval are copied from study siblings: diffusion and Robin use analysis [0.25, 0.5, 1] and sampling 0.0625; Example 3 uses analysis [50, 100, 200] and sampling 1.

Cost is `estimated_work_units` = species × nodes × steps (diffusion and Robin species 1, Example 3 species 3). Steps = time_end / dt.
Bytes use the spec formula, `2,000,000 + (print_count+1)·nodes·(160+36·species) + steps·(700+300·species)`, with print_count = time_end / sampling_interval. The formula reproduces the existing sibling estimates.

| Proposed ID | Role | Axis | Nodes | dt | Steps | Cost (work units) | Bytes est. | Duplicate status |
|---|---|---|---|---|---|---|---|---|
| `convergence-diffusion_cosine-space_65-dt2e-7-rfc-baseline` | study | space | 65 | 2^-7 | 128 | 8,320 | 2,344,580 | new |
| `convergence-diffusion_cosine-space_97-dt2e-7-rfc-baseline` (centre) | study | space | 97 | 2^-7 | 128 | 12,416 | 2,451,204 | matches holdout triple; study role, not reused |
| `convergence-diffusion_cosine-space_129-dt2e-7-rfc-baseline` | study | space | 129 | 2^-7 | 128 | 16,512 | 2,557,828 | new |
| `convergence-diffusion_cosine-time_97-dt2e-6-rfc-baseline` | study | time | 97 | 2^-6 | 64 | 6,208 | 2,387,204 | new |
| `convergence-diffusion_cosine-time_97-dt2e-8-rfc-baseline` | study | time | 97 | 2^-8 | 256 | 24,832 | 2,579,204 | new |
| `convergence-scalar_robin-space_129-dt2e-7-rfc-baseline` | study | space | 129 | 2^-7 | 128 | 16,512 | 2,557,828 | new |
| `convergence-scalar_robin-space_193-dt2e-7-rfc-baseline` (centre) | study | space | 193 | 2^-7 | 128 | 24,704 | 2,771,076 | matches holdout triple; study role, not reused |
| `convergence-scalar_robin-space_257-dt2e-7-rfc-baseline` | study | space | 257 | 2^-7 | 128 | 32,896 | 2,984,324 | new |
| `convergence-scalar_robin-time_193-dt2e-6-rfc-baseline` | study | time | 193 | 2^-6 | 64 | 12,352 | 2,707,076 | new |
| `convergence-scalar_robin-time_193-dt2e-8-rfc-baseline` | study | time | 193 | 2^-8 | 256 | 49,408 | 2,899,076 | new |
| `convergence-example3-space_501-dt0.04-rfc-baseline` | study | space | 501 | 0.04 | 5000 | 7,515,000 | 36,987,868 | new |
| `convergence-example3-space_751-dt0.04-rfc-baseline` (centre) | study | space | 751 | 0.04 | 5000 | 11,265,000 | 50,454,868 | matches holdout triple; study role, not reused |
| `convergence-example3-space_1001-dt0.04-rfc-baseline` | study | space | 1001 | 0.04 | 5000 | 15,015,000 | 63,921,868 | new |
| `convergence-example3-time_751-dt0.05-rfc-baseline` | study | time | 751 | 0.05 | 4000 | 9,012,000 | 48,854,868 | new |
| `convergence-example3-time_751-dt0.025-rfc-baseline` | study | time | 751 | 0.025 | 8000 | 18,024,000 | 55,254,868 | new |

Totals: 15 proposed rows (12 bracket rows plus 3 centre rows). Work units 61,035,160. Output bytes 281,713,740 (about 282 MB). The largest row (18,024,000 units) is below the 24,024,000-unit existing 1001-node space row that runs under the same 60 s timeout, so no row needs a timeout change.

Duplicate check: no proposed triple matches an existing non-holdout row. The three centre triples match the reserved holdout triples only. Those holdout rows are not reused, and the centre rows are not holdout rows. Study outputs at a coincident holdout configuration do not count as the reserved independent experiment (`SOLUTE_VERIFICATION_SPEC.md`, "Norms, refinement and budgets").

## Claims mapping

Field list matches the "M5-V claims" table in `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_22.md`.

| M5-V claim field | Axis | Isolating rows | Status |
|---|---|---|---|
| Diffusion cosine space accuracy (`linf`, `l2`) | space | diffusion space 65, 97 (centre), 129 at dt 2^-7 | isolated |
| Diffusion cosine inventory / mass conservation | space | same space rows (inventory metric) | isolated; known defect SD-005 (`docs/FORTRAN_SOURCE_DEFECTS.md`, `SOLUTE.FOR:803-827`, commit 40b6593), recorded as a known defect and not blocking |
| Scalar Robin space accuracy (`linf`, boundary outflow) | space | Robin space 129, 193 (centre), 257 at dt 2^-7 | isolated |
| Scalar Robin `l1`, `l2`, inventory (ceilings unassigned) | space | same Robin space rows | isolated |
| Temporal convergence, diffusion | time | diffusion time 97 at 2^-6, 2^-7 (centre), 2^-8 | isolated |
| Temporal convergence, Robin | time | Robin time 193 at 2^-6, 2^-7 (centre), 2^-8 | isolated |
| Example 3 full-chain NH4/NO2 norms | space and time | Example 3 space 501, 751 (centre), 1001 at dt 0.04; time 751 at 0.05, 0.04 (centre), 0.025 | isolated |
| Example 3 NO3, inventory, balance, node-1 closure, outlet tail, reaction rates (ceilings unassigned) | space and time | same Example 3 rows | isolated |
| Example 3 inventory, balance, node-1 closure, temporal error | space and time | same Example 3 rows | isolated |
| Classifier verdict (achieved/failed) | n/a | not row-isolated | depends on the Wave 5 bounds source |

Reaction closed-form fields have no holdout row and are outside this design.

## Blocking fields (D6)

None. Every claimed holdout field has bracket rows and a centre row on each axis it depends on. The classifier verdict is blocked by the Wave 5 bounds source, not by this design.

## Dependencies for 23.3.1

- The 15 rows above are added to `CASES_V2.json` in the single SHA change, with precision relabelled `f64` to `f32` (D4).
- Bridge row (D11): the print-step rule in `crates/hydrus-core/tests/solute_verification_harness.rs:221-231` lands in the Wave 3 tool batch before the freeze. The bridge row is not a claimed field and gets no isolating row; `CASES_V2.json` bridge rows are not edited.
- `THRESHOLDS_V2.json` and `POLICY_HASHES` are handled in later waves; they are not part of this design.

## Known defect reference

- SD-005: diffusion end-row mass loss, `SOLUTE.FOR:803-827`. Registered in commit 40b6593.

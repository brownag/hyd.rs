# Solute printed concentration failures: source review

## Scope and conclusion

This is a read-only diagnosis of the authenticated pairs summarized by [`SOLUTE_PRINTED_PARITY_EVIDENCE.md`](SOLUTE_PRINTED_PARITY_EVIDENCE.md). It uses only retained repository artifacts and source; no solver, build, benchmark, web source, or analytical reference was run. The evidence note’s prose is internally consistent with its table: concentration has 9,092 fails in **five** cases, while `reaction_zero` has none. Any “four cases” wording is an error.

The comparison path provides strong evidence that these are real disagreements between the retained Rust binary64 profile values and Fortran’s printed intervals, rather than an obvious species/node/time/coordinate/format alignment defect. It does not identify a unique solver bug. Candidate differences include spatial transport discretization (particularly for diffusion and the upstream-weighted Robin case), reaction/sorption discretization, and Example 3 process/parameter translation. The smallest useful next step is a narrowly scoped one-step coefficient-and-state trace on a tiny retained reaction case, starting from identical state, recording the Fortran and Rust assembled tridiagonal coefficients/RHS and accepted concentration after one step. That can separate translation/algebra differences from accumulated stepping differences without broad changes.

## Reconciled counts

Counts below were independently obtained by counting `field == concentration` records with `status == fail` in `.development/14.3.1/printed_parity.json`; denominator is all concentration records. These are interval failures, not unavailable records.

| Case | Failures | Total | Notes |
|---|---:|---:|---|
| `reaction_distinct` | 855 | 891 | 33 profiles × 9 nodes × 3 species |
| `reaction_equal` | 864 | 891 | 33 × 9 × 3 |
| `diffusion_cosine` | 440 | 1,105 | 17 × 65 × 1 |
| `scalar_robin` | 1,386 | 2,193 | 17 × 129 × 1 |
| `example3` | 5,547 | 12,012 | 4 × 1001 × 3 |
| `reaction_zero` | 0 | 891 | 33 × 9 × 3 |
| **Total** | **9,092** | **17,983** | five cases fail |

The per-case counts agree with the evidence table. `example3` uses four profiles, including the initial profile, but has no common endpoint schedule; its direct profile comparison is still valid.

## Comparison trace and defect checks

1. `tools/solute_printed_parity.py:compare_pair` loads the authenticated Fortran manifest/context, calls the full `metrics._parse_fortran` validation, loads the validated Rust observations, and obtains the retained `NOD_INF.OUT` token blocks. It requires the number of blocks to equal Rust profile count and each block to contain the declared number of nodes.
2. `tools/solute_verification_metrics.py:_node_profiles` validates `NOD_INF` profile times in canonical schedule order using `f14.4`; validates the row count and node identifiers in ascending order; matches each printed negative depth against the expected negative coordinate using `f10.4`; checks known hydraulic fields; and verifies each row has exactly `11 + 2 * species_count` columns. It extracts concentration as columns `11 + j` in declared species order and validates all concentration/sorption tokens as `e12.4`. Fortran concentration columns are emitted as `(Conc(j,Node(i)), j=1,NS)` in `src_reference/OUTPUT.FOR` profile write statements (e.g. lines 1059–1070); format labels 110–221 use the corresponding species count and `e12.4` (lines 1080–1106).
3. `compare_pair` zips Fortran blocks with Rust profiles after schedule-count validation and records the Rust profile time. For each node, `_validate_node_mapping` requires 1-based Fortran node ID equal to the expected index and requires the Rust positive coordinate to lie in the rounding interval around the negated Fortran depth token. This establishes same physical orientation and rejects reordered nodes.
4. Species are enumerated in `rust["species"]`; that list was validated against the request and casebook order in `solute_verification_metrics.py` (`_validate_request`, species check around lines 212–224). The same index selects Fortran `fields[11 + species_index]` and Rust `profile["concentration"][species_index][node_index]`.
5. `_record` parses the original token with `Decimal`, derives the half-unit interval from `e12.4`, converts Rust binary64 exactly using `Decimal.from_float`, and fails only when that value lies outside `[printed-half_width, printed+half_width]` (`tools/solute_printed_parity.py:49–74, 111–139`). E/D exponent spelling is accepted; retained concentration format is authenticated/validated as `e12.4`. The comparator’s unit interval is the correct print-resolution interval and does not impose a looser tolerance.

These checks make a simple species order, profile-time order, node order, coordinate orientation, concentration-column offset, or Fortran exponent parsing defect unlikely. They cannot prove the Fortran and Rust input-to-solver semantic translations identical in every numerical detail; manifests authenticate the retained run inputs and source identities, while `CASES.json` records the reviewed case mapping.

## Residual structure in retained JSON

The following describes signed difference `Rust - Fortran-token`, aggregated only over failed concentration records. It is descriptive, not a causal test.

| Case | Species | Mean signed difference | Range | Distribution |
|---|---|---:|---:|---|
| reaction distinct | S1 | +7.66e-4 | +8.58e-5..+1.149e-3 | 288 fails |
|  | S2 | -2.23e-3 | -2.852e-3..-4.37e-4 | 288 |
|  | S3 | +1.53e-4 | -9.43e-4..+7.47e-4 | 279 |
| reaction equal | S1 | +7.66e-4 | +8.58e-5..+1.149e-3 | 288 fails |
|  | S2 | -2.29e-3 | -2.917e-3..-3.04e-4 | 288 |
|  | S3 | +5.05e-4 | +9.84e-5..+6.69e-4 | 288 |
| diffusion cosine | S1 | -7.15e-4 | -3.147e-3..-5.01e-5 | 440 fails |
| scalar Robin | S1 | -1.89e-4 | -6.189e-4..+9.19e-5 | 1,386 fails |
| Example 3 | NH4 | -4.06e-5 | -4.322e-2..+4.286e-2 | 1,383 fails |
|  | NO2 | -1.98e-6 | -2.980e-3..+1.988e-3 | 2,071 |
|  | NO3 | +1.04e-4 | -5.320e-3..+9.471e-3 | 2,093 |

Reaction-case failures occur across all 33 positive-time profiles, all nine nodes, and all three species (except nine S3 records in the distinct case pass); their species-specific means are strongly different. The shared S1 residuals are numerically identical between `reaction_distinct` and `reaction_equal`, consistent with their identical S1 settings. The failures are not isolated to a boundary node. `diffusion_cosine` fails first at profile 1 (10 nodes), then spans 53 of 65 nodes over the 16 positive-time profiles; all failed differences are negative. `scalar_robin` fails over all 129 nodes in every positive-time profile, with later profiles having more failed intervals; its mean is negative. Example 3 has no failures at initial time; counts by profile and species are NH4 261/420/702, NO2 403/667/1001, NO3 412/691/990 at 50/100/200 hours. Example 3 residual signs are mixed and extreme local differences reach about ±0.043, unlike simple rounding.

## Source-supported implementation context

### Cases with no spatial transport

`reaction_distinct` and `reaction_equal` have `q=0`, `dispersion=0`, uniform initial `[1,0,0]`, and zero inlet concentrations (`CASES.json:54–133`). They differ in downstream rates: distinct uses 0.2, 0.4, 0.1; equal uses 0.2, 0.2, 0. Their configured S1 input is otherwise the same. `reaction_zero` has the same transport setup but all reaction rates zero; it passes all 891 concentration comparisons. This localizes the observed disagreement to active reaction behavior for this test family, but does not by itself establish whether the cause is time integration, reaction mass conversion, or reaction-chain coupling.

Rust solves species sequentially in configured order (`crates/hydrus-core/src/solute.rs:747–771`); in process mode it builds the next species source from the preceding species’ `previous_species_mass` and preceding decay rate (`:1162–1170`), adds decay and source to a single-step or nonlinear solve (`:1173–1235`), and updates reaction accounting (`:1283–1290`). The Fortran source documents sequential first-order decay and sets `lNEquil=.false.` (`src_reference/SOLUTE.FOR:44–51`); species coupling enters the coefficient/source assembly at `:657–685`. The distinct/equal/zero comparison pattern supports investigating reaction discretization and chain coupling first. It does **not** establish an algebraic sign error from source inspection alone.

### Diffusion and scalar Robin

`diffusion_cosine` has zero flow, molecular diffusion 0.2, cosine initial state, zero reaction, and zero-flux/zero-gradient boundaries (`CASES.json:300–366`). Rust assembles face transport using centered neighboring water content and face dispersion, with advective upwind coefficients and a conservative tridiagonal flux operator (`solute.rs:1299–1325`). The retained monotone negative residual is evidence of a systematic temporal/spatial operator difference, not a token parsing issue. Fortran calculates dispersion in `SOLUTE.FOR:1023–1062`, and its upstream-weighting path computes `wc` from local Peclet data at `:1001–1014`; for the pure diffusion case that weighting is not the obvious mechanism because flow is zero.

`scalar_robin` has flow 0.43, dispersion 1, a nonuniform steady-plus-mode initial state, nonzero decay, upstream weighting enabled, and zero-gradient outlet (`CASES.json:368–437`). Rust face assembly uses first-order upwind advection (`solute.rs:1318–1325`); Fortran has a dedicated `lUpW` weighting-factor path (`SOLUTE.FOR:1001–1014`). Differences in the transport stencil are plausible, but the retained JSON cannot show whether the Rust setup’s supplied `face_dispersion` and Fortran’s computed effective `Disp` and weighting coefficients were pointwise equivalent.

### Example 3

The canonical Example 3 pair covers 1001 nodes and 0/50/100/200-hour profiles, with three sequential species (NH4, NO2, NO3). The JSON’s mixed-sign residuals and late-time growth indicate substantial profile disagreement, but there are no common time endpoints in this pair and no endpoint parity evidence. The repository explicitly states that earlier same-solver bridge claims are separate evidence; therefore these 5,547 failures must not be explained by endpoint schedules or treated as endpoint disagreement. Example 3’s retained source/configuration mapping includes inherited physical parameters and authored finite-domain/boundary derivation in `CASES.json:439–532`. More detailed source-to-field checks of its process model, material/water mapping and effective dispersion are required before selecting a cause.

## What remains unproven and next step

Verified: the five-case count; exact interval-based nature of failure; coverage and matching invariants; the case-level residual patterns above; and source-level differences in the broad numerical mechanisms available to each implementation.

Not proven: that any one Rust coefficient, boundary condition, reaction rate, or time discretization is wrong; that the two retained runs began from bitwise-identical effective state after input adaptation; or that a selected algorithmic difference explains all five families. Agreement of `reaction_zero` only rules out a broad baseline mismatch for that case’s inactive reaction setup.

Smallest discriminating next step: use one canonical reaction case with the smallest mesh (`reaction_equal`, nine nodes) and its retained first positive-time profile. Without running either solver in this research task, specify a follow-up instrumented reproduction that fixes the authenticated source and request, compares the time-zero state, then dumps per-species first-step cell mass/derivative, assembled lower/diagonal/upper/RHS before solve, solved concentration, and the next-species source. First compare coefficients at identical state; then compare the one-step result. If coefficients/RHS already differ, trace the input mapping/stencil; if they agree but result differs, inspect linear solve/order; if one-step agrees but later retained profiles diverge, inspect time-step advancement and accumulation. This is smaller and more informative than running all six cases or changing tolerances.

## Sources

- Retained result: `.development/14.3.1/printed_parity.json` (all per-record tokens, intervals, differences, species, time, node and status).
- Existing parity summary: `planning/master/SOLUTE_PRINTED_PARITY_EVIDENCE.md` (canonical identities, schedules, interpretation limits).
- Comparator: `tools/solute_printed_parity.py` (`_half_width`, `_record`, `_validate_node_mapping`, `compare_pair`); validation/parser: `tools/solute_verification_metrics.py` (`_validate_request`, `_node_profiles`, observation validation).
- Case configurations: `reference/solute_verification/CASES.json`.
- Retained Fortran source: `src_reference/OUTPUT.FOR` (profile columns and formats); `src_reference/SOLUTE.FOR` (`Solute`, sequential species coupling, dispersion and weighting).
- Rust implementation: `crates/hydrus-core/src/solute.rs` (`SoluteSpecies`, species-major state, `solve_solute_step`, process and nonprocess assembly, flux operator, boundary rows).

# Robin upstream-weighting mapping investigation

Investigation date: 2026-11-11. This report records the source investigation and a dated resolution below. The original investigation ran no solver, parity suite, campaign, holdout, or Example 3.

## Verdict

**H1 — translation/capture defect, confidence: high.** The initial source review found that the V2 case declared `upstream_weighting: true`, the Fortran reference deck received it, and the public Rust path dropped it. The user resolved the remaining intent question: `FortranCompatible` is expected to honor Fortran selectors. The omission is therefore a bug, not an intentional API limitation. The saved Robin campaign's Rust observations were made with this selector off; fixing the mapping changes what future runs calculate but does not retroactively change or validate those saved results.

## Source semantics (verified facts)

- `INPUT.FOR`, `ChemIn` (around lines 1289–1324): the chemistry input record reads `epsi,lUpW,lArtD,lTDep,...` (lines 1308, 1311); Fortran logical input uses `t`/`.true.` for on and `f`/`.false.` for off in the authored decks. If `lUpW` is true, `ChemIn` forces `lArtD=.false.` (`INPUT.FOR:1324` onward; inspect the complete block for its associated source comments). This coupling matters: the switch is not simply an isolated coefficient toggle.
- `SOLUTE.FOR`, `Solute` calls `Coeff` for each level/iteration; `Coeff` calls `PeCour` (`SOLUTE.FOR:746`). The call is a fixed-form continued statement: column-6 `!` line at `SOLUTE.FOR:747` continues the call from the preceding line. The pinned build accepts fixed form (`tools/build_fortran_reference.sh`, compiler flags); do not read the continuation as an independent statement.
- `PeCour` (`SOLUTE.FOR:965–1019`) declares `logical lUpW,lArtD` and defines `TanH(z)=(exp(z)-exp(-z))/(exp(z)+exp(-z))` at line 972. In the `Level.eq.NLevel` branch it computes Peclet/Courant limits and, when `.not.lUpW.and..not.lArtD`, applies the non-artificial-dispersion Courant correction (`SOLUTE.FOR:986–1003`). The `else if(lUpW.and.Iter.eq.1)` branch (`:1004–1015`) calculates `Pe2`, sets `wc=0` for negligible velocity, uses signed ±1 for `abs(Pe2)>10`, otherwise evaluates `1./TanH(Pe2)-1./Pe2` and clamps to [-1,1].
- `MatSet` uses `wc` in its element-row transport terms (`SOLUTE.FOR:784–899`, many long/commented lines; executable blocks should be distinguished by fixed-form column 1). `Disper` also branches on `lUpW` for the non-artificial dispersion correction (`SOLUTE.FOR:1025–1071`). Thus `lUpW` controls the PeCour weight generation and a separate coefficient/courant/dispersion behavior; setting native `cfg.upwind` is not “only enable wc.”
- The current Rust `transport.rs::pecour` mirrors the relevant weight branch only when `upstream_weighting && iteration == 1` (`:263`); `driver.rs` additionally passes the flag to `coefficients(...)` (`:1245`), whose `if !artificial_dispersion && !upstream_weighting` branch alters the correction (`transport.rs:134`).

## Original mapping trace (pre-fix)

| Boundary | Evidence | Preservation/disposition |
|---|---|---|
| `CASES_V2.json` | `scalar_robin` chemistry declares `upstream_weighting: true` (`reference/solute_verification/CASES_V2.json`, around line 424). | Metadata value exists. |
| Reference deck/request production | Before the fix, `tools/run_solute_verification_reference.py` formatted the value into the Fortran deck. The Rust request did not carry it. | Fortran reference received the value; Rust request omitted it. |
| Public Rust harness | Before the fix, the request/harness fields did not include `upstream_weighting`. | Case-to-public-Rust mapping dropped the selector. |
| `SoluteConfig` → workspace | Before the fix, `SoluteConfig` lacked this option and the public step used the default `compat_cfg.upwind: false`. | Public path ran with weighting off. See resolution below for current behavior. |
| `SoluteStepConfig` → coefficients and Peclet inputs | `driver.rs::coeff_level_into` calls `coefficients(..., cfg.upwind, ...)` around line 1245 and constructs `PecletInputs { upstream_weighting: cfg.upwind, ... }` around 1440–1451. | Native lower-level path preserves the flag if set. |
| `pecour` | `transport.rs::pecour` gates source `wc` on flag and first iteration; the audit instrumentation reports zero such calls for saved Robin runs. | No `wc` branch in historical runs; current mapping is enabled by the V2 request as described above. |
| Deck compatibility path | `run_compat_deck` constructs `SoluteStepConfig` from parsed `chem.l_up_w` (`compat_runner.rs` around 1070). `deck_reader.rs` reads/validates optional upstream metadata (`:1140`); adapter DTO includes `lUpW` (`adapter/transport.rs:167`) and maps the field (`adapter/mod.rs:143,197–198,452`). | Native deck adapter DOES preserve switch, including checks against the deck value. |

The public API route is distinct from `run_compat_deck`: shared kernel/config type does not imply shared selector mapping.

## Resolution and implementation status (2026-11-11)

The mapping defect is fixed in the current working tree:

1. `SoluteConfig::with_upstream_weighting(bool)` exposes the source selector,
   defaulting to `false` for existing callers. The public
   `advance_fortran_compatible_step` copies it into `SoluteStepConfig.upwind`.
   FV/BE continues to use its own face-flux upwinding and ignores this option.
2. New V2 Rust requests include `upstream_weighting=true|false` from the case's
   chemistry metadata. The Rust verification harness parses the field and passes
   it through `SoluteConfig` to the compatibility kernel. V1 requests and earlier
   V2 requests without the field retain the historical false/default behavior.
3. Request/metrics validation checks a supplied selector against case metadata.
   Lifecycle request matching accepts both the new V2 request and the prior V2
   byte form. Historical adapter-pin checks and saved campaign records are not
   rewritten.

The request serializer and its lifecycle/metrics consumers have new source
identities after this fix. Current SHA-256 values are:

| File | SHA-256 |
|---|---|
| `tools/run_solute_verification_reference.py` | `f4b8ead0b506c40784d9107636a63f03adc440c27ddf25f5d1faeffa6b9fb221` |
| `tools/solute_benchmark_lifecycle.py` | `232a62ac61a8e5259e0f9ad257b6db57190c62ce92dcc8bcfa615caff8e338a5` |
| `tools/solute_verification_metrics.py` | `adf317c251dc582b4965ee91daa03c56333da28f85f1f750917fd4214846e847` |
| `crates/hydrus-core/tests/solute_verification_harness.rs` | `11ed5e425e1750b71722d06b1443f958e18e3b3c6af93dd326f5c169900ebcc8` |

These hashes identify current sources; the six historical campaign batch recipes
and their producer pins remain unchanged and identify the pre-fix run. No batch,
holdout, Example 3, or solver verification campaign was rerun. Existing Robin
failure classifications remain facts about the saved settings-off Rust runs;
they are not results for the corrected mapping. Any replacement official
evidence should come from an authorized rerun of the affected Robin
spatial-refinement campaign, following its existing approval, producer-pin,
bounds, and evidence gates. Preserve the pre-fix artifacts as historical
evidence; do not overwrite or reclassify them.

Focused validation completed: Rust tests
`solute_verification_harness::{upstream_weighting_request_reaches_public_fortran_path,legacy_request_defaults_upstream_weighting_off}`
and `pecour_updates_weight_only_on_first_iteration_and_computes_limit` passed;
Python V2 request/lifecycle tests passed (5 tests); all 25
`tools.test_solute_verification_metrics` tests passed; `cargo fmt --all -- --check`
and `git diff --check` passed. One initial Python test command named a nonexistent
test class; it was rerun with the actual `V2LifecycleTests` class and passed.

## Case metadata and original intent question (resolved below)

`CASES_V2.json::chemistry.upstream_weighting` is validated as a boolean by `tools/check_solute_verification_spec.py` and is consumed by Fortran deck production. Before the fix, the Rust request/harness did not pass the chemistry flag into `SoluteConfig`; the loss point was between case metadata and public config construction.

At investigation time, the spec did not explicitly state the public mapping requirement. The implementation decision was subsequently resolved in favor of carrying this Fortran selector through `FortranCompatible`; the fix and request contract are documented above and in `SOLUTE_VERIFICATION_SPEC.md`.

## Parity and verification consequences

- The retained 44-fixture gate is `planning/master/SOLUTE_FORTRAN_PARITY_18.md`, section 21.4.1; it reports 44/44. Its fixtures are independent deck fixtures parsed by `run_compat_deck` / compatibility adapter, not the `CASES_V2` Robin harness. Their `SELECTOR.IN` records and manifests contain upstream selector values; examples include explicit `lUpW` metadata in `reference/fortran_reference/solute/parity/transport_properties/tortuosity_millington/CASE.json` and upstream-on/off transport cases. The current deck path maps these into `cfg.upwind`; the parity fixture path therefore already exercises the selector. The audit does not establish a full coverage count of on/off parity fixtures, but `transport-upstream-off` exists and source manifests explicitly include enabled switch cases.
- The existing 44-fixture suite exercises the deck path, which already maps the selector. The public verification mapping now has direct enabled/disabled regression coverage.
- The V2 campaign recipe pins the 11 producer tools and casebook. Since no producer or casebook would be edited by the adapter-only change, existing recipe hashes are not inherently broken. But changed numerical output means saved campaign evidence is not evidence for the changed mapping. Whether it mandates a **six-batch rerun and new Gate B** depends on campaign governance/approval: the current investigation cannot authorize that. It would at minimum require a separately approved rerun/reclassification and new evidence admission; do not silently reuse existing results.
- `FiniteVolumeBackwardEuler` uses its own conservative face-flux upwind split (`solute.rs::assemble_flux_operator`; `SOLUTE_RUST_FORMULATION_17.md`, “Transport and storage rows”). It always selects advective upwind direction from signed face flux, independent of this Fortran `lUpW` switch. The case flag is not a separate FV/BE user-facing control. The question is specifically relevant to `FortranCompatible`/source-parity path.

## Hypothesis assessment

| Hypothesis | Evidence for | Evidence against / qualification | Assessment |
|---|---|---|---|
| H1 Translation/capture defect | Case says true, reference deck emits true, and pre-fix public Rust workspace remained false despite an active source switch. | No longer unresolved: the public mapping was fixed and tested. | **Confirmed and fixed.** |
| H2 Intentional scope limit | Public `SoluteConfig` exposes selected fields; FV/BE has unrelated internal upwinding. | V2 `FortranCompatible` now carries the declared selector. | Not the chosen behavior. |
| H3 Specification ambiguity | The spec had not previously named this public mapping. | The verification request contract now specifies the mapping. | Resolved. |
| H4 Case-metadata defect | The field was previously only used for reference-deck execution. | New V2 Rust requests validate and propagate the same value. | Fixed at the request/config boundary. |
| H5 Other/insufficient | Campaign causation remains undetermined. | Mapping facts are established; historical outcomes were not rerun. | Remains relevant only to campaign interpretation. |

## Historical resolution options considered (superseded)

The investigation considered specification clarification, mapping the flag, or declaring it reference-only. The mapping option was implemented: new V2 requests pass the selector into public `SoluteConfig`, with enabled/disabled regression coverage. Historical requests retain their false default. No campaign rerun was authorized or performed.

The option-selection discussion above records the pre-fix investigation and is superseded by the resolution. Enabling `cfg.upwind` changes coefficient/Courant behavior beyond `wc`; the implementation carries the declared selector rather than treating it as a `wc`-only toggle.

## Documentation intent and temporal status

- `docs/decisions/0001` and `0002`: compatibility to pinned source and separation of source-compatible versus FV/BE formulations; do not define each public selector's exposure.
- `docs/decisions/0008`: parity scope is active/callable behavior reachable via audited `HYDRUS → ChemIn → Solute`; requires fixture evidence for covered modes, but its phrase “supported interaction” does not alone make every input field public.
- `docs/decisions/0013` and `0018`: backend selection/default history; current ADR README says `FortranCompatible` is default after 44/44, FV/BE remains selectable. Not a mapping decision.
- `SOLUTE_VERIFICATION_SPEC.md`: current verification design, including the upstream selector in new V2 Rust requests and legacy omission behavior.
- `SOLUTE_FORMULATION_DECISION_17.md`: historical formulation-choice memo, explicitly did not choose target. Its open choices are superseded in part by later compatibility/backend ADRs, but not by a selector-level mapping decision.
- `SOLUTE_FORTRAN_FORMULATION_17.md` / `SOLUTE_RUST_FORMULATION_17.md`: descriptive source and implementation reports; the former notes upstream source behavior, the latter describes FV/BE upwinding independent of Fortran `lUpW`.
- `SOLUTE_FORTRAN_PARITY_18.md`: historical gate results culminating in 21.4.1 44/44; these validate the deck parity path, not the later public V2 case-to-API map.
- `SOLUTE_OUTLET_CONTRACT.md`: accepted FV/BE outlet contract, explicitly not parity evidence; says existing conservative equations/upwind split are retained and must not be replaced by Fortran endpoint equality.
- `M5V_ROBIN_AUDIT_23.md`: newest audit confirms no public-path `PeCour` weight calls in saved Robin runs and diagnostic forced-weight values. Those forced variants are diagnostic only and are not official evidence; they do not answer the specification decision.

## Explicit unresolved items and limitations

1. Campaign governance was not re-read in full to decide formal rerun/Gate B requirements; this report does not authorize campaign activity.
2. The 44-fixture suite's exact count of fixtures exercising `lUpW=true` versus false was not computed. The selector path and representative fixtures establish that the compatibility deck route can carry it.
3. No hashes were computed for this report.

## Appendix: inspected files

`src_reference/INPUT.FOR`, `src_reference/SOLUTE.FOR`, `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/solute_fortran_compat/{driver.rs,transport.rs,compat_runner.rs,deck_reader.rs,types.rs,adapter/mod.rs,adapter/transport.rs}`, `tools/{run_solute_verification_reference.py,solute_benchmark_bridge.py,check_solute_verification_spec.py}`, `reference/solute_verification/CASES_V2.json`, representative parity manifests and decks, `planning/master/{SOLUTE_VERIFICATION_SPEC.md,SOLUTE_FORMULATION_DECISION_17.md,SOLUTE_FORTRAN_FORMULATION_17.md,SOLUTE_RUST_FORMULATION_17.md,SOLUTE_FORTRAN_PARITY_18.md,SOLUTE_OUTLET_CONTRACT.md,M5V_ROBIN_AUDIT_23.md}`, and `docs/decisions/{0001,0002,0008,0013,0018}`.

# Robin upstream-weighting mapping investigation

Date: 2026-11-11. Read-only investigation of the public verification-to-native adapter. No solver, parity suite, campaign, holdout, or Example 3 run was performed. Existing working-tree changes were preserved.

## Verdict

**H1 — translation/capture defect, confidence: high.** The initial source review found that the V2 case declared `upstream_weighting: true`, the Fortran reference deck received it, and the public Rust path dropped it. The user resolved the remaining intent question: `FortranCompatible` is expected to honor Fortran selectors. The omission is therefore a bug, not an intentional API limitation. The saved Robin campaign's Rust observations were made with this selector off; fixing the mapping changes what future runs calculate but does not retroactively change or validate those saved results.

## Source semantics (verified facts)

- `INPUT.FOR`, `ChemIn` (around lines 1289–1324): the chemistry input record reads `epsi,lUpW,lArtD,lTDep,...` (lines 1308, 1311); Fortran logical input uses `t`/`.true.` for on and `f`/`.false.` for off in the authored decks. If `lUpW` is true, `ChemIn` forces `lArtD=.false.` (`INPUT.FOR:1324` onward; inspect the complete block for its associated source comments). This coupling matters: the switch is not simply an isolated coefficient toggle.
- `SOLUTE.FOR`, `Solute` calls `Coeff` for each level/iteration; `Coeff` calls `PeCour` (`SOLUTE.FOR:746`). The call is a fixed-form continued statement: column-6 `!` line at `SOLUTE.FOR:747` continues the call from the preceding line. The pinned build accepts fixed form (`tools/build_fortran_reference.sh`, compiler flags); do not read the continuation as an independent statement.
- `PeCour` (`SOLUTE.FOR:965–1019`) declares `logical lUpW,lArtD` and defines `TanH(z)=(exp(z)-exp(-z))/(exp(z)+exp(-z))` at line 972. In the `Level.eq.NLevel` branch it computes Peclet/Courant limits and, when `.not.lUpW.and..not.lArtD`, applies the non-artificial-dispersion Courant correction (`SOLUTE.FOR:986–1003`). The `else if(lUpW.and.Iter.eq.1)` branch (`:1004–1015`) calculates `Pe2`, sets `wc=0` for negligible velocity, uses signed ±1 for `abs(Pe2)>10`, otherwise evaluates `1./TanH(Pe2)-1./Pe2` and clamps to [-1,1].
- `MatSet` uses `wc` in its element-row transport terms (`SOLUTE.FOR:784–899`, many long/commented lines; executable blocks should be distinguished by fixed-form column 1). `Disper` also branches on `lUpW` for the non-artificial dispersion correction (`SOLUTE.FOR:1025–1071`). Thus `lUpW` controls the PeCour weight generation and a separate coefficient/courant/dispersion behavior; setting native `cfg.upwind` is not “only enable wc.”
- The current Rust `transport.rs::pecour` mirrors the relevant weight branch only when `upstream_weighting && iteration == 1` (`:263`); `driver.rs` additionally passes the flag to `coefficients(...)` (`:1245`), whose `if !artificial_dispersion && !upstream_weighting` branch alters the correction (`transport.rs:134`).

## Mapping trace

| Boundary | Evidence | Preservation/disposition |
|---|---|---|
| `CASES_V2.json` | `scalar_robin` chemistry declares `upstream_weighting: true` (`reference/solute_verification/CASES_V2.json`, around line 424). | Metadata value exists. |
| Reference deck/request production | `tools/run_solute_verification_reference.py`, `build_deck` around line 365 formats the value as `t` or `f` into the `epsi lUpW ...` deck record. This is the Fortran/reference side. The Rust benchmark request lifecycle serializes the case's benchmark row/request; request keys are validated by `tools/check_solute_verification_spec.py`. | Preserved for Fortran deck generation; not necessarily present in Rust request. |
| Public Rust harness | `tools/solute_benchmark_bridge.py` is the bridge; its comments describe dispatch to the public `SoluteConfig` API. The request/harness’s mapped solute fields include mesh, state, species, boundaries and backend; no `upstream_weighting` appears in the public config construction/call interface. | Dropped at the case-to-public-Rust mapping boundary (case metadata is not made a `SoluteConfig` member). |
| `SoluteConfig` → workspace | `crates/hydrus-core/src/solute.rs::SoluteWorkspace::new` initializes `compat_cfg.upwind: false` (around line 492). `SoluteConfig` does not declare an upstream-weighting option. `advance_fortran_compatible_step` populates coordinates, material, linearity, `equil`, timestep, boundaries and tolerances (around 1176–1209), but never assigns `compat_cfg.upwind`. | Dropped/constant false. |
| `SoluteStepConfig` → coefficients and Peclet inputs | `driver.rs::coeff_level_into` calls `coefficients(..., cfg.upwind, ...)` around line 1245 and constructs `PecletInputs { upstream_weighting: cfg.upwind, ... }` around 1440–1451. | Native lower-level path preserves the flag if set. |
| `pecour` | `transport.rs::pecour` gates source `wc` on flag and first iteration (`:263`); audit instrumentation reports zero such calls for saved Robin runs. | No `wc` branch on official public Robin path. |
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
evidence requires a separate decision and fresh approval of the applicable
campaign/pin process.

Focused validation completed: Rust tests
`solute_verification_harness::{upstream_weighting_request_reaches_public_fortran_path,legacy_request_defaults_upstream_weighting_off}`
and `pecour_updates_weight_only_on_first_iteration_and_computes_limit` passed;
Python V2 request/lifecycle tests passed (5 tests); all 25
`tools.test_solute_verification_metrics` tests passed; `cargo fmt --all -- --check`
and `git diff --check` passed. One initial Python test command named a nonexistent
test class; it was rerun with the actual `V2LifecycleTests` class and passed.

## Case metadata consumption and intent

`CASES_V2.json::chemistry.upstream_weighting` is validated as a boolean by `tools/check_solute_verification_spec.py` around lines 245–250. It is consumed when producing the Fortran reference deck (`run_solute_verification_reference.py:365`). The benchmark/verification runner's request carries selected row/backend fields, not a corresponding public `SoluteConfig` switch. The public harness does not pass the chemistry flag into Rust. Exact loss point: between normalized case metadata / bridge case-to-API construction and `SoluteConfig`; the Rust config has no such field, and the compatibility workspace defaults false.

The spec's canonical source mapping and its requirement that case values be declared do not explicitly state “map `upstream_weighting` into public Rust.” It does define the reference equation and selected Rust solver/backend, and a casebook boolean that only affects the reference executable is a potential asymmetry. No explicit unsupported-field declaration or ignored-field rationale was found. Therefore current documents leave this unresolved rather than requiring a specific code mapping.

## Parity and verification consequences

- The retained 44-fixture gate is `planning/master/SOLUTE_FORTRAN_PARITY_18.md`, section 21.4.1; it reports 44/44. Its fixtures are independent deck fixtures parsed by `run_compat_deck` / compatibility adapter, not the `CASES_V2` Robin harness. Their `SELECTOR.IN` records and manifests contain upstream selector values; examples include explicit `lUpW` metadata in `reference/fortran_reference/solute/parity/transport_properties/tortuosity_millington/CASE.json` and upstream-on/off transport cases. The current deck path maps these into `cfg.upwind`; the parity fixture path therefore already exercises the selector. The audit does not establish a full coverage count of on/off parity fixtures, but `transport-upstream-off` exists and source manifests explicitly include enabled switch cases.
- Changing the public mapping would not mechanically alter existing 44-fixture assertions because that path already maps the selector; it may alter Robin M5 verification results and any public-contract expectations. A direct test/fixture should be added if the approved decision is to expose/map the field, so it proves true and false case propagation and catches accidental defaulting.
- The V2 campaign recipe pins the 11 producer tools and casebook. Since no producer or casebook would be edited by the adapter-only change, existing recipe hashes are not inherently broken. But changed numerical output means saved campaign evidence is not evidence for the changed mapping. Whether it mandates a **six-batch rerun and new Gate B** depends on campaign governance/approval: the current investigation cannot authorize that. It would at minimum require a separately approved rerun/reclassification and new evidence admission; do not silently reuse existing results.
- `FiniteVolumeBackwardEuler` uses its own conservative face-flux upwind split (`solute.rs::assemble_flux_operator`; `SOLUTE_RUST_FORMULATION_17.md`, “Transport and storage rows”). It always selects advective upwind direction from signed face flux, independent of this Fortran `lUpW` switch. The case flag is not a separate FV/BE user-facing control. The question is specifically relevant to `FortranCompatible`/source-parity path.

## Hypothesis assessment

| Hypothesis | Evidence for | Evidence against / qualification | Assessment |
|---|---|---|---|
| H1 Translation/capture defect | Case says true, reference deck emits true, public Rust workspace remains false, and source switch is active. | No clear spec clause says this case field must reach the public API; FortranCompatible public API may deliberately be a narrower projection. | Plausible but unproven. |
| H2 Intentional scope limit | Public `SoluteConfig` intentionally exposes selected fields only; native deck route preserves broader selectors; FV/BE has unrelated internal upwinding. | No explicit decision says upstream weighting is unsupported/ignored; parity objective says reproduce mapped source behavior. | Not demonstrated. |
| H3 Specification ambiguity | Spec/casebook declares the flag and source backend contract, but does not resolve whether this specific switch is part of public API mapping. | None material. | **Best supported.** |
| H4 Case-metadata defect | The field is only used for reference-deck source execution in this verification path; public Rust path ignores it. | Case-derived provenance and source mapping may intend identical selector semantics; no explicit “reference-only” label. | Possible, not established. |
| H5 Other/insufficient | Current diagnostics cannot decide intended API requirement or campaign treatment. | Direct mapping facts are established. | Residual limitation, but H3 captures the central uncertainty better. |

## Smallest justified resolution options (ranked)

1. **Resolve specification first (recommended).** State whether V2 upstream weighting is a parity input required on `FortranCompatible`, or reference-only metadata intentionally not mapped. Code change: no immediate change. Casebook change: only if decision is reference-only and schema should say so; otherwise none. Test/fixture: add a propagation contract test if requiring mapping. Six-batch rerun/new Gate B: not automatically; if mapping changes candidate observations, assess/approve campaign rerun and fresh evidence gate before using results. Documentation correction: yes, record decision and boundary.
2. **Map the flag to `compat_cfg.upwind`.** Code change: yes, plus a public API path to carry the value (likely `SoluteConfig` or step config); not just flip native workspace default, since cases may be false and caller-specific. Casebook change: no. Test/fixture: yes, public regression for both states; likely a scalar Robin fixture plus false control. Six-batch rerun/new Gate B: changed solver outputs invalidate comparability of existing campaign artifacts; governance must determine rerun scope, with a new Gate B required for replacement official evidence. Documentation correction: update API/verification mapping and note coupled native effects.
3. **Declare it intentionally unsupported/reference-only.** Code change: no. Casebook change: possibly rename/scope metadata or validate its reference-only use; not required absent schema redesign. Test/fixture: add a test asserting the public request deliberately excludes/ignores it, and reference-side generation retains it. Six-batch rerun/new Gate B: no solver behavior changes; existing Rust evidence remains representative of current public path, but any reclassification of approved campaign semantics is an owner decision. Documentation correction: yes, required.

No option should be implemented as a solver fix without a separate approval decision. Enabling `cfg.upwind` also changes coefficient/courant behavior beyond `wc`.

## Documentation intent and temporal status

- `docs/decisions/0001` and `0002`: compatibility to pinned source and separation of source-compatible versus FV/BE formulations; do not define each public selector's exposure.
- `docs/decisions/0008`: parity scope is active/callable behavior reachable via audited `HYDRUS → ChemIn → Solute`; requires fixture evidence for covered modes, but its phrase “supported interaction” does not alone make every input field public.
- `docs/decisions/0013` and `0018`: backend selection/default history; current ADR README says `FortranCompatible` is default after 44/44, FV/BE remains selectable. Not a mapping decision.
- `SOLUTE_VERIFICATION_SPEC.md`: current verification design, with casebook/source mapping and backend request contract; no explicit upstream public-API mapping clause found.
- `SOLUTE_FORMULATION_DECISION_17.md`: historical formulation-choice memo, explicitly did not choose target. Its open choices are superseded in part by later compatibility/backend ADRs, but not by a selector-level mapping decision.
- `SOLUTE_FORTRAN_FORMULATION_17.md` / `SOLUTE_RUST_FORMULATION_17.md`: descriptive source and implementation reports; the former notes upstream source behavior, the latter describes FV/BE upwinding independent of Fortran `lUpW`.
- `SOLUTE_FORTRAN_PARITY_18.md`: historical gate results culminating in 21.4.1 44/44; these validate the deck parity path, not the later public V2 case-to-API map.
- `SOLUTE_OUTLET_CONTRACT.md`: accepted FV/BE outlet contract, explicitly not parity evidence; says existing conservative equations/upwind split are retained and must not be replaced by Fortran endpoint equality.
- `M5V_ROBIN_AUDIT_23.md`: newest audit confirms no public-path `PeCour` weight calls in saved Robin runs and diagnostic forced-weight values. Those forced variants are diagnostic only and are not official evidence; they do not answer the specification decision.

## Explicit unresolved items and limitations

1. No authoritative requirement explicitly states whether `CASES_V2.chemistry.upstream_weighting` is a required public Rust input or a reference-only selector.
2. Campaign governance was not re-read in full to decide formal rerun/Gate B requirements; this report does not authorize campaign activity.
3. The 44-fixture suite's exact count of fixtures exercising `lUpW=true` versus false was not computed. The selector path and representative fixtures establish that the compatibility deck route can carry it.
4. Current working tree contains unrelated audit changes (including `transport.rs`, `mod.rs`, and diagnostic artifacts). This report treats their state as present and does not edit/revert them.
5. No hashes were computed for this report.

## Appendix: inspected files

`src_reference/INPUT.FOR`, `src_reference/SOLUTE.FOR`, `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/solute_fortran_compat/{driver.rs,transport.rs,compat_runner.rs,deck_reader.rs,types.rs,adapter/mod.rs,adapter/transport.rs}`, `tools/{run_solute_verification_reference.py,solute_benchmark_bridge.py,check_solute_verification_spec.py}`, `reference/solute_verification/CASES_V2.json`, representative parity manifests and decks, `planning/master/{SOLUTE_VERIFICATION_SPEC.md,SOLUTE_FORMULATION_DECISION_17.md,SOLUTE_FORTRAN_FORMULATION_17.md,SOLUTE_RUST_FORMULATION_17.md,SOLUTE_FORTRAN_PARITY_18.md,SOLUTE_OUTLET_CONTRACT.md,M5V_ROBIN_AUDIT_23.md}`, and `docs/decisions/{0001,0002,0008,0013,0018}`.

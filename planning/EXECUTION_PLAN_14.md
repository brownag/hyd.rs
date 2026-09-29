# Orchestration 14 — Solute printed-output parity

Status: **Draft for user approval. Planning only. No solver or analytical reference runs are authorized.**
Companion files: [worker prompt](PROMPT_TEMPLATE_14.md), [state](STATE_14.md).

## Goal and scope

**Goal.** Determine, for each executable example in `reference/solute_verification/CASES.json` and each directly comparable output field, whether Rust reproduces the value the retained Fortran program prints. Compare the Rust value with the rounding interval implied by that specific Fortran field format and token, including scientific-notation values near zero. Report results by example, field, species, and available time/record. Preserve the signed raw difference and mark fields unavailable when their semantics, provenance, or records cannot be matched. This establishes printed-output parity only; it does not establish continuum accuracy or physical validity.

**In scope:**

- Inventory all six executable examples and the exact canonical and supplemental Fortran/Rust artifacts already retained; authenticate identities, requests, configurations, schedules, and file hashes.
- Audit the recorded Fortran validation failures and decide which retained fields can safely be compared or must remain unavailable.
- Reuse the Example 3 parser, source-format metadata, token rounding helpers, and bridge comparison code where applicable; add only the cross-language comparison/reporting gap.
- Compare one canonical retained pair per example: the five component pairs and Example 3 bridge pair. Use supplemental convergence artifacts only to resolve a concrete field or schedule gap.
- Produce a field-level coverage table with `pass`, `fail`, or `unavailable`, raw differences, interval widths, evidence references, and a separate print-resolution indicator.
- Reassess resources from retained sizes and actual free space. The approved future-work ceiling is 1 GB total new artifacts/build/scratch; it is a ceiling, not a target.
- Propose no solver runs in the base DAG: a retained pair exists for all six examples. If comparison discovers a genuine coverage gap that cannot be resolved from retained artifacts, stop and seek a case-specific user-approved amendment before proposing any run.

**Out of scope:** new analytical references; solver/core/source changes; Fortran precision or output-format changes; new physics; global/scientific tolerance thresholds; M5-V closure; repeating the Orchestration 13 57-start matrix; interpreting printed-output parity as analytical accuracy, physical validation, or general HYDRUS release compatibility. More Fortran digits may be proposed only as a separate optional user decision with its cost and consequences.

**Verbatim constraints from the request:**

> “Do not pursue accuracy beyond what Fortran’s printed output can establish unless a separate user-approved output-format change is proposed.”

> “No solver run or new analytical reference calculation is authorized by this planning request.”

> “Do not apply one global tolerance to every output.”

> “Do not repeat or silently inherit the old oversized 57-start matrix.”

> “Success [is] agreement within each Fortran field’s rounding interval (or a specifically justified comparator rule), not 12-digit continuum accuracy.”

> “Preserve raw differences and identify cases where a printed interval is too wide to be useful.”

> “Keep solver changes, Fortran output precision changes, new physics, threshold setting, and M5-V closure out of scope unless a concrete evidence gap makes one necessary.”

> “The core question is simple: for each executable example and each comparable output field, does Rust reproduce what the retained Fortran program prints?”

## Decisions — 2026-09-28

- User confirmed: retain Orchestration 13 and blocked 13.4.1 as historical work; mark 13.5.1 and 13.6.1 superseded for this parity goal. Do not rewrite their historical status or evidence.
- User set a maximum of **three parallel workers**.
- User agreed that a **1 GB (1,000,000,000-byte) ceiling** is reasonable for new artifacts, builds, and scratch if later work is separately approved; expected usage should be far below it when things work properly. Treat it as a safety ceiling, not a target, and do not inherit the earlier lifecycle accounting ceiling.
- User selected **one canonical retained pair per executable example**. Inventory supplemental pairs; use them only for a specific field/schedule gap.
- The plan is not authorization to launch solvers or calculate analytical references. Any such work needs a separate, case-specific approval after read-only results identify the need.
- Preserve the existing worktree changes to `planning/EXECUTION_PLAN_13.md`, `planning/STATE_13.md`, and untracked `planning/prompt.md`; this orchestration creates only its own numbered files plus named future task outputs.

## Reconnaissance log

Repository: `/home/andrew/AI/hyd.rs`, branch `phase4-water-acceptance`. No repository-local `AGENTS.md` or `CONTRIBUTING.md` was found. The working tree already contains user changes listed above; do not overwrite them.

`CASES.json` has six executable cases: `reaction_distinct`, `reaction_equal`, `reaction_zero`, `diffusion_cosine`, `scalar_robin`, and `example3`. The five component pairs are in `.development/13.3.1/benchmarks/batch-043cf43e84c04448a38e452670278c4`; their manifests show Fortran `raw_complete` and Rust `complete`. Example 3’s canonical pair is in `.development/13.3.1/benchmarks/batch-c96c2d089400481a913a4accb6f1650f` (`benchmark-example3-bridge-fortran-baseline` and `...-rust-baseline`, profiles 0/50/100/200 h). The bridge’s Fortran report has zero retained-field failures; its Rust report records a passing same-solver bridge against the retained Rust artifact. The earlier `compare_solute_reference.py` report is direct Example 3 Fortran/Rust residual evidence, but has no printed-interval pass/fail result.

Supplemental pairs exist in `.development/13.4.1/study/`; do not use these as canonical evidence. `planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md` records three failed Fortran validations: `endpoint observation ids/order mismatch` (Example 3), `incomplete step inventory` (scalar Robin time run), and `water inventory mismatch` (diffusion time control). Both Example 3 full/component and 1001-node convergence failures share the endpoint-order warning. Failed-run manifests retain raw artifacts, but their fields are not accepted merely because files exist. The canonical bridge pair avoids requiring a replacement run for the Example 3 case.

Comparator anchors and limits:

- `tools/compare_solute_reference.py` (770 lines): `verified_paths`, `parse_node_profiles`, `parse_solute_output`, `parse_balance`, `parse_rust_observations`, `compare` (~440), and `run` (~684). This is Example 3-specific, computes residual summaries, and does not gate each value on its source print interval.
- `tools/solute_benchmark_bridge.py` (648 lines): `_token_interval` (~395), `compare_fortran` (~405), and `compare_rust` (~453). It already uses field-specific print intervals for retained-versus-new same-solver bridges; do not replace it or mislabel it as the general cross-language comparator.
- `tools/solute_verification_metrics.py` (1,547 lines): `FORMATS`, `rounding_half_width` (~157), `_node_profiles` (~556), `_solute_rows` (~532), `_balance` (~629), `_endpoints` (~682). Reuse these parsers/format rules when their validation contract applies.
- `tools/run_solute_verification_reference.py` (1,217 lines): `FIELD_FORMATS`, `validate_outputs` (~486–630), and `verify_run`; `src_reference/OUTPUT.FOR` contains native formats around lines 170–200, 693–715, and 891–926. Do not infer one format for all records.
- Concentration and nodal concentration tokens use `e12.4`; solute flux/cumulative and many balance values use `e13.5`; coordinates/times and ancillary fields use several fixed and exponential formats. Use the exact field descriptor plus token exponent in `rounding_half_width`, including small scientific values. Fortran inactive nonequilibrium `Sorb` columns are not Rust equilibrium `Kd*C` and must not be paired as if they were the same field.

Recent recorded regression baseline (not rerun for this planning request): 145 Rust tests, 13 Python runner/outlet tests, 12 Example 3 comparator self-checks, and 85 convergence classifier/evidence/CLI tests passed in the Orchestration 13 wave-end record; formatting, Clippy with warnings denied, and `git diff --check` passed. Use targeted task tests and serialize any workspace-wide suites. Do not use the legacy comparator’s default paths or overwrite `.development/12.6.1` artifacts.

Resource observations: `.development/13.3.1/benchmarks` is 192,541,487 bytes and `.development/13.4.1/study` is 646,228,104 bytes on disk; the filesystem had 228,376,735,744 bytes available at reconnaissance. A retained 1001-node Example 3 Fortran/Rust run pair is about 76.8 MB; retained Rust build trees are about 21.5 MB. These figures replace the prior lifecycle-only stop as the resource basis. Recheck actual free space before any separately approved run; stop before 1,000,000,000 bytes of new task artifacts/build/scratch.

## Task DAG and wave table

| Wave | Task | Depends on | Deliverable |
|---|---|---|---|
| 1 | 14.1.1 Inventory canonical pairs and validation failures | — | Authenticated six-case artifact inventory, per-case config/schedule pairing, supplemental index, failure dispositions |
| 2 | 14.2.1 Add field-aware printed-parity comparison | 14.1.1 | Narrow cross-language comparison command and tests, reusing existing readers/format logic |
| 2b | 14.2.2 Correct coordinate interval matching and CLI invocation | 14.2.1 | Source-interval coordinate matching and direct-script import bootstrap with tests |
| 3 | 14.3.1 Compare canonical pairs and publish coverage | 14.2.2 | Immutable raw comparison report and final example/field coverage table |
| 4* | 14.4.1 Prepare exact run proposal only for a remediable gap | 14.3.1 | Case-specific proposal, or `skipped_no_remediable_gap`; no run starts |

*Wave 4 is conditional. If Wave 3 finds no remediable gap that retained data cannot resolve, record `skipped_no_remediable_gap` and start no worker. No task in this DAG launches a solver or calculates an analytical reference. Any later run needs separate explicit user approval of the concrete proposal.

### 14.2.2 — Correct coordinate interval matching and CLI invocation

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `tools/solute_printed_parity.py`; `tools/test_solute_printed_parity.py`.
- **Can create:** `planning/tasks/14.2.2.todo.md`; `.development/14.2.2/`.
- **Depends on:** accepted 14.2.1; discovered during 14.3.1 before any report was emitted.
- **Spec:** Make the prescribed direct CLI invocation work from the repository root without `PYTHONPATH` overrides. Replace the binary-float coordinate mapping check with a source-format interval comparison: invert the sign of the Fortran coordinate token, derive its exact `f10.4` print interval, and accept the Rust coordinate when it lies on either inclusive edge or within that interval. Reject a value outside the interval and retain fail-closed node order handling. Keep the rest of the accepted comparator semantics unchanged.
- **Do not touch:** inventory, evidence/report, retained artifacts, source, casebook, existing plans/state other than this documented decomposition amendment, or task 14.3.1 outputs.
- **Anchors:** `tools/solute_printed_parity.py`: `_validate_node_mapping`, `main`, imports; `tools/test_solute_printed_parity.py`: coordinate mapping tests.
- **Acceptance:** tests cover the reported `-0.0312`/`0.03125` boundary, both inclusive interval edges, an outside-interval value, and node-order failure. The prescribed command must start without `PYTHONPATH` and may only fail later due to data behavior, if any. `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity`; `git diff --check`. No solver/build/reference work.

## Task specifications

### 14.1.1 — Inventory canonical pairs and validation failures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `tools/solute_printed_parity_inventory.py`; `tools/test_solute_printed_parity_inventory.py`; `planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json`; `planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.md`; `.development/14.1.1/`.
- **Depends on:** none.
- **Spec:** Read-only inventory of the six executable case IDs. Authenticate exact canonical Fortran/Rust manifests, requests, case/config fields, common nodes/species/times/steps, artifact hashes and bytes. Pin the five component pairs to `batch-043cf43e84c04448a38e452670278c4` and Example 3 to `batch-c96c2d089400481a913a4accb6f1650f`. List supplemental retained convergence pairs separately. Include the three validation failure classes and exact manifest paths/statuses; identify the Example 3 failed full rows as noncanonical and do not use their status as a reason to launch a replacement. Do not claim raw files from a failed run are valid comparison inputs.
- **Do not touch:** prior plans/state/evidence; `CASES.json`; source, solver, old comparison artifacts; any raw run output.
- **Grep anchors:** `tools/solute_benchmark_lifecycle.py`: `verify_run`, `source_hashes`; `tools/run_solute_verification_reference.py`: `OUTPUTS`, `FIELD_FORMATS`, `verify_run`; `planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md`: “Batch outcomes”; manifests and run paths listed above.
- **Acceptance:** the inventory contains exactly six executable case IDs, six canonical pairs, authenticated manifest/artifact hashes, and explicit supplemental/failure classifications. `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity_inventory`; `git diff --check`. No solver, build, or reference process.

### 14.2.1 — Add field-aware printed-parity comparison

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `tools/solute_printed_parity.py`; `tools/test_solute_printed_parity.py`; `.development/14.2.1/`.
- **Depends on:** 14.1.1 accepted inventory.
- **Spec:** Add a narrow comparator/CLI over authenticated retained Fortran manifests and Rust observation artifacts. Reuse `metrics.rounding_half_width`, the applicable raw readers/field descriptors, and bridge interval patterns. Match only equivalent fields at common coordinates/times/species with validated orientation/sign mapping. For each Fortran numeric token, derive its half-unit interval from that field’s actual format and exponent; compare the Rust binary64 value to that interval without a global tolerance. Use exact/robust arithmetic at interval edges. Emit `pass`, `fail`, or `unavailable`, the raw Fortran token/value, Rust value, signed difference, interval, source format, coordinate/time/species, and reason. Do not compare Fortran nonequilibrium `Sorb` to Rust equilibrium storage. Mark `low_information` separately when the print interval half-width is at least the absolute Rust value (or Rust value is zero); this is a print-resolution diagnostic, not a pass/fail tolerance. Fail closed on hash/config/schedule/schema mismatches. Never normalize by overwriting source artifacts.
- **Do not touch:** existing comparator/bridge semantics unless an exact shared helper extraction is necessary and separately scoped; solver/core/source, `CASES.json`, Fortran output formats, prior runs, tolerances, analytical-reference code.
- **Grep anchors:** `compare_solute_reference.py`: `compare`, `run`; `solute_benchmark_bridge.py`: `_token_interval`, `_Comparison.number`, `compare_fortran`; `solute_verification_metrics.py`: `FORMATS`, `rounding_half_width`, `_node_profiles`, `_solute_rows`, `_balance`, `_endpoints`; `run_solute_verification_reference.py`: `FIELD_FORMATS`.
- **Acceptance:** tests cover each native format family in the selected output fields, exact interval-edge behavior, small/underflow-scale scientific tokens, D/E exponents, wrong mappings/order, missing counterpart (`unavailable`), output resolution diagnostics, and preservation of raw signed differences. `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity`; `git diff --check`. No solver or analytical work.

### 14.3.1 — Compare canonical pairs and publish coverage

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/SOLUTE_PRINTED_PARITY_EVIDENCE.md`; `.development/14.3.1/` (immutable JSON result and logs only).
- **Depends on:** 14.2.1 accepted comparator and 14.1.1 inventory.
- **Spec:** Run the comparator read-only on the six canonical pairs only. Confirm Example 3’s printed-output parity result is stated with its exact available fields and bounds; keep the earlier same-solver bridge claims separate from direct Fortran/Rust comparisons. Include one final table with one row per example/field/species and status; preserve detailed values and differences in JSON. Include exact artifact/report hashes and commands. For each `fail`, retain the mismatching token/value and interval. For each `unavailable`, state whether the cause is missing Rust output, incompatible semantics, failed retained validation, missing common record, or unauthenticated artifact. Identify wide print intervals via the declared `low_information` rule. Supplemental convergence artifacts may be read only if the inventory proves they fill a named field/schedule gap; label them supplemental and retain their validation caveats. Do not start runs; if a genuine, remediable gap remains after retained-artifact review, pass it to conditional task 14.4.1 for a proposal only.
- **Do not touch:** all input artifacts, prior summaries, Fortran/Rust source, casebook, 13.x evidence files, or any analytical cache. Never write under 12.x/13.x roots.
- **Grep anchors:** `planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.md`; `tools/solute_printed_parity.py`; canonical batch directories and Example 3 report path in this plan.
- **Acceptance:** all six case IDs appear; every comparable field has pass/fail and every noncomparable field has unavailable with reason; all raw differences and source intervals are retained; inventory/report hashes verify; new output root is under `.development/14.3.1/`. `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity`; `PYTHONDONTWRITEBYTECODE=1 python3 tools/solute_printed_parity.py --cases reference/solute_verification/CASES.json --inventory planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json --output-root .development/14.3.1`; `git diff --check`. This command reads retained files only.

### 14.4.1 — Prepare exact run proposal only for a remediable gap (conditional)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/SOLUTE_PRINTED_PARITY_RUN_PROPOSAL.md`; `planning/master/SOLUTE_PRINTED_PARITY_RUN_PROPOSAL.json`; `.development/14.4.1/`.
- **Depends on:** 14.3.1 complete and an explicit field/schedule gap recorded in its report.
- **Spec:** Determine from the accepted inventory and comparison report whether any unavailable field is remediable by a new retained pair. If none, write `status: skipped_no_remediable_gap`, cite the evidence, and do not create a solver command or start a worker. If a gap is remediable, prepare a concrete proposal only: exact case ID; output field/species/time/record; absent solver side and evidence of the gap; exact eligible unique unconsumed `CASES.json` launch IDs; complete proposed command; applicable row timeout (60 seconds only where the selected row says so); analogous observed artifact sizes; build/temp estimate and margin; remaining amount under the 1 GB ceiling; stop conditions; and explicit no-retry policy. Exclude launch IDs or evidence associated with failed or consumed runs. If lifecycle tooling cannot perform the proposed run without broader changes, stop and state that a new plan is required. State clearly that this proposal confers no execution authority and requires separate explicit user approval before any solver start. No analytical reference work.
- **Do not touch:** `CASES.json`, source/solver, existing manifests or raw outputs, any prior evidence, `.development/12.*` and `.development/13.*`.
- **Grep anchors:** accepted `planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.md`; `planning/master/SOLUTE_PRINTED_PARITY_EVIDENCE.md`; `reference/solute_verification/CASES.json`; `tools/solute_benchmark_lifecycle.py` launch/authorization validation.
- **Acceptance:** either a cited `skipped_no_remediable_gap` record and zero starts, or a complete case-specific proposal with every required field above and zero starts. `git diff --check`. Do not run a solver, build, or reference calculation.

## Resource and launch accounting

Base plan: **zero solver starts, zero analytical reference calculations, zero replacement runs**. The six canonical pairs already exist. Read-only parsing and report output belong under `.development/14.*`; do not change or copy raw run files. Conditional task 14.4.1 may prepare a detailed proposal only. Any actual run requires separate explicit user approval after that proposal is reviewable. The 1 GB total is a safety ceiling; expected usage is far below it when things work properly. Estimate from observed analog sizes and include build/temp margin. No automatic retries; failed starts count. If the task cannot use an existing uniquely authorized launch row without changing `CASES.json` or broad lifecycle policy, stop and require a new execution plan. Never trigger analytical work through `check_solute_benchmarks.py`; only cached references may be read if needed, and analytical references are not part of parity acceptance.

## Risks and gates

- A field’s matching name does not prove matching meaning. Require source-backed field mapping and keep undefined semantics unavailable.
- Fortran concentration’s `e12.4` interval changes with token exponent; fixed absolute tolerances would hide small-value discrepancies.
- Printed intervals can make a numerical pass uninformative. Report interval width and `low_information`; do not widen the interval or call it higher accuracy.
- The Example 3 endpoint-order validation warning also appears in the failed full component and 1001-node convergence artifacts. Diagnose it, keep it out of canonical inputs, and do not rerun to work around it.
- Supplemental 13.4.1 artifacts have incomplete groups and other validation failures; they cannot silently become canonical.
- Current disk availability and prior lifecycle accounting are snapshots. Recheck before any separate run approval; the 1 GB ceiling governs new task artifacts/build/scratch.
- Passing printed parity says Rust reproduces the retained Fortran program’s printed values at measured coordinates/times. It does not establish agreement beyond print precision.

## Independent plan review

Independent reviews found two gaps, now addressed: (1) require a concrete proposal if the read-only comparison finds a remediable coverage gap, while keeping run execution separately gated; (2) restore the worker prompt’s exact approval token and completion report contract. Approval applies to this plan only; it does not authorize solver or analytical runs. On approval, hand off to **wave-orchestration** with “run wave 1”.

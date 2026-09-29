# Orchestration 15 — Investigate solute concentration parity failures

Status: **User approved on 2026-09-29. Waves 1–3 have produced bounded reports with documented trace gaps; Wave 4 synthesis is pending. No production solver changes are authorized or made.**
Companion files: [worker prompt](PROMPT_TEMPLATE_15.md), [state](STATE_15.md).

## Goal and scope

**Goal.** Investigate the five concentration failure families documented in `planning/master/SOLUTE_CONCENTRATION_FAILURES_RESEARCH.md` and identify where each implementation first diverges. Produce evidence-backed, narrow pathways for later orchestration plans to implement and validate targeted fixes. Use identical authenticated starting state and compare inputs, first-step operators, solved states, and subsequent advancement in that order. Do not infer a cause from end-profile residuals alone.

**In scope:**

- Establish a reproducible, bounded first-step diagnostic trace for the nine-node `reaction_equal` case, with `reaction_zero` as the control and `reaction_distinct` as a check on chain-rate effects.
- Diagnose `diffusion_cosine` independently, including effective dispersion, face/cell coefficients, boundary rows, and first-step state.
- Diagnose `scalar_robin` independently, including Fortran upstream weighting, Rust face transport, effective dispersion, and outlet/boundary assembly.
- Audit Example 3 input/process translation and compare coefficients/state at explicitly aligned checkpoints; explain the lack of common internal step endpoints and use matched profile states only where valid.
- Record evidence, reject unsupported hypotheses, and produce a synthesis mapping each confirmed divergence to a bounded candidate fix scope, regression evidence needed, and unresolved questions for subsequent targeted plans.

**Out of scope:** production solver fixes; changing output precision or parity tolerances; changing case definitions or scientific acceptance thresholds; analytical-reference calculations; broad convergence studies; changing the Fortran reference source; treating any single source-level difference as causal without discriminating evidence.

**Verbatim user constraints:**

> “this first plan should primarily focus on investigation of each class of failure, producing proven pathways for subsequent plans to write targeted fixes for”

The input research reports **9,092 failures in five cases** (`reaction_distinct`, `reaction_equal`, `diffusion_cosine`, `scalar_robin`, `example3`), with `reaction_zero` passing. This plan preserves the research’s caveat that the failure count does not itself identify a solver defect.

## Decisions — 2026-09-29

- Investigation begins with the nine-node reaction family, the smallest diagnostic system, then proceeds to diffusion, scalar Robin, and Example 3 before synthesis.
- Diagnostic reproduction is limited to the exact retained source/configuration and minimal checkpoints needed to discriminate hypotheses. No production changes are included.
- No solver launches, builds, or new analytical references have been performed in preparing this plan. Task execution is gated on approval of this plan and its bounded diagnostic run protocol. Any expansion beyond the named cases/checkpoints requires a plan amendment.
- Example 3 profile comparisons remain distinct from internal endpoint comparisons: its retained pair has no common endpoints.
- User approved this orchestration and its bounded diagnostic run protocol on 2026-09-29. This authorizes only the named, task-gated diagnostics; it does not authorize production fixes or scope expansion.

## Reconnaissance log

Repository: `/home/andrew/AI/hyd.rs`; branch and working-tree context follow Orchestration 14. Existing user changes include `planning/EXECUTION_PLAN_13.md`, `planning/STATE_13.md`, `planning/PROMPT_TEMPLATE_14.md`, `planning/master/SOLUTE_CONCENTRATION_FAILURES_RESEARCH.md`, `planning/prompt.md`, and `planning/prompt2.md`; preserve them.

No repository-local `AGENTS.md` or `CONTRIBUTING.md` was found. Rust workspace is `crates/hydrus-core`; recorded Orchestration 14 baseline: 145 Rust tests, 13 runner/outlet Python tests, 17 benchmark tests, 86 convergence tests, formatter, Clippy with warnings denied, and diff check passed. Baseline was recorded, not rerun for this planning task. Targeted tests are only required if a task creates or changes diagnostic tooling; do not run broad suites concurrently.

Relevant file sizes and anchors: `crates/hydrus-core/src/solute.rs` (1,474 lines; `solve_solute_step`, process assembly around lines 1162–1290, transport assembly around 1299–1325, boundary rows); `src_reference/SOLUTE.FOR` (2,105 lines; sequential coupling around 657–685, upstream weighting around 1001–1014, dispersion around 1023–1062); `reference/solute_verification/CASES.json` (case definitions); `tools/solute_printed_parity.py` (263 lines; authenticated retained comparison); `planning/master/SOLUTE_CONCENTRATION_FAILURES_RESEARCH.md` (findings and limitations); `planning/master/SOLUTE_PRINTED_PARITY_EVIDENCE.md` (record-level parity evidence).

The authenticated retained pairs and hashes are recorded in printed parity evidence. New diagnostic artifacts must be isolated under `.development/15.<task>/`, never written into retained Orchestration 13/14 artifacts. Use immutable copies or explicit run roots. Trace output must state units, array indexing, species order, boundary convention, and the exact pre/post-solve point represented. Do not conflate Fortran `Sorb` with Rust equilibrium storage. Do not use a global tolerance to decide coefficient/state equality; compare exact inputs where possible and report field-specific representation/roundoff where not.

## Task DAG and wave table

| Wave | Task | Depends on | Deliverable |
|---|---|---|---|
| 1 | 15.1.1 Reaction-only first-step trace | — | Aligned reaction coefficient/RHS/state trace and causal branch classification |
| 2 | 15.2.1 Pure-diffusion trace | 15.1.1 | Coefficient, boundary, and first-step divergence diagnosis for `diffusion_cosine` |
| 2 | 15.2.2 Scalar Robin trace | 15.1.1 | Effective transport/weighting/boundary and first-step diagnosis for `scalar_robin` |
| 3 | 15.3.1 Example 3 mapping and checkpoint diagnosis | 15.1.1, 15.2.1, 15.2.2 | Source-to-field audit and aligned checkpoint divergence report |
| 4 | 15.4.1 Evidence synthesis and targeted-fix pathways | all prior tasks | Per-family proven pathway, candidate fix boundary, required regression and open evidence gaps |

Wave 2 tasks may run concurrently only after task 15.1.1 establishes the instrumentation and comparison contract. They have disjoint task-local output roots. Orchestrator serializes builds/runs if they share resources.

## Shared diagnostic protocol and gates

Every executable diagnostic must use immutable, authenticated source identities and case inputs. Before execution, each task provides a micro-plan, exact command, expected output files, upper bound on run count and artifact bytes, timeout, cleanup policy, and a check that no retained input will be overwritten. The orchestrator reviews this protocol before sending the worker gate. The exact prescribed diagnostic runs are authorized only within the task scope after this plan is approved; no retries or expanded case matrices without amendment. A failed/ambiguous diagnostic stops with evidence rather than silently retrying.

At each comparison level record Fortran and Rust values side by side: adapted physical/input fields; time-zero concentration and storage; per-species local mass/derivative and reaction source; lower/diagonal/upper/RHS before solve; boundary-row coefficients; solved concentration; and the state passed to the next species/step. Align source conventions and units before comparison. Identify the earliest unequal layer. If the first-step coefficients match but solutions differ, inspect linear solve/order; if one-step outputs match but later profiles diverge, inspect advancement and accumulation. All new outputs stay under `.development/15.*`; retained runs are read-only.

No production source edits are authorized. Temporary diagnostic instrumentation may be added only in task-local copies/worktrees and must not be merged into production files. If a necessary trace cannot be obtained without production changes, stop and report the smallest instrumentation-only follow-up plan needed.

## Task specifications

### 15.1.1 — Reaction-only first-step trace

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/SOLUTE_REACTION_FAILURE_DIAGNOSIS.md`; `planning/tasks/15.1.1.todo.md`; `.development/15.1.1/` task-local copies, scripts, logs, and trace output.
- **Depends on:** none.
- **Spec:** Start from the authenticated `reaction_equal` pair and select its first positive-time interval. Compare effective time-zero species state, storage/mass factors, reaction rates and source conventions. Obtain a one-step trace from both implementations for the same nine-node input, reporting each species’ cell mass/derivative, assembled tridiagonal lower/diagonal/upper/RHS, solve result, and next-species source. Use `reaction_zero` as the inactive-process control and check `reaction_distinct` only to isolate rate/chain coupling after the base trace. Determine the first layer that differs and classify it as input adaptation, reaction algebra/assembly, solve, or later stepping. Do not declare a defect merely because source implementations differ.
- **Do not touch:** production solver/core files, `src_reference/SOLUTE.FOR`, `CASES.json`, retained pair directories, parity tolerances, or prior orchestration records.
- **Anchors:** research doc sections “Cases with no spatial transport” and “What remains unproven”; `solute.rs` species sequencing/process assembly anchors; `SOLUTE.FOR` sequential coupling anchors; canonical pair identity in parity evidence.
- **Acceptance:** report records authenticated source/request hashes, selected first positive-time interval, every compared quantity with units/index/species, first divergent layer, control result, run count/commands, and evidence for or against reaction conversion/coupling hypotheses. If exact trace reproduction is blocked, report blocker and smallest required trace seam. `git diff --check`; task-local trace validator or an explicitly documented manual checksum/reconciliation. No broad suite.

### 15.2.1 — Pure-diffusion trace

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/SOLUTE_DIFFUSION_FAILURE_DIAGNOSIS.md`; `planning/tasks/15.2.1.todo.md`; `.development/15.2.1/`.
- **Depends on:** accepted 15.1.1 trace protocol.
- **Spec:** Reproduce the first failing positive-time profile for `diffusion_cosine` from the authenticated 65-node input. Compare effective face/node dispersion, water content, timestep, cell storage, each interior transport coefficient, endpoint boundary rows, RHS, and solved state. Because q=0, explicitly verify whether upstream weighting is inactive and do not presume it explains the residual. Relate the first-step evidence to the retained all-negative residual pattern and identify whether divergence begins in input/effective-D mapping, spatial stencil, boundary treatment, solve, or later advancement. Keep investigation confined to this case.
- **Do not touch:** production files, reference source, casebook, retained artifacts, reaction diagnosis, or global tolerances.
- **Anchors:** research “Diffusion and scalar Robin”; `solute.rs` transport assembly; `SOLUTE.FOR` dispersion/weighting paths; `CASES.json` `diffusion_cosine`.
- **Acceptance:** diagnosis includes hashes, first failing profile/step, effective D and face coefficient comparisons, boundary rows, RHS/solution, earliest divergence, and a falsifiable follow-up for any unresolved cause. Run and artifact bounds appear in report. `git diff --check`; task-local trace consistency check if created.

### 15.2.2 — Scalar Robin trace

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/SOLUTE_ROBIN_FAILURE_DIAGNOSIS.md`; `planning/tasks/15.2.2.todo.md`; `.development/15.2.2/`.
- **Depends on:** accepted 15.1.1 trace protocol.
- **Spec:** Reproduce the first failing positive-time profile for `scalar_robin` from the authenticated 129-node input. Compare effective dispersion, local Peclet values, Fortran upstream weighting factors, Rust face flux coefficients, storage/reaction terms, inlet/outlet boundary rows, RHS and solved concentration. Verify mapping of inlet/outlet orientation and Robin/zero-gradient laws from source and case inputs. Identify the first divergent layer and test whether the observed negative residual trend is explained by weighting, boundary assembly, effective parameter translation, or later stepping. Do not infer from residual sign alone.
- **Do not touch:** production files, reference source, casebook, retained artifacts, other diagnosis outputs, or global tolerances.
- **Anchors:** research “Diffusion and scalar Robin”; `solute.rs` transport/boundary assembly; `SOLUTE.FOR` `lUpW` and dispersion; `CASES.json` `scalar_robin`.
- **Acceptance:** source-backed orientation and boundary mapping plus aligned coefficient/RHS/solution trace; report earliest divergence, falsified hypotheses, run/artifact bounds and a concrete unresolved discriminator if needed. `git diff --check`; task-local trace consistency check if created.

### 15.3.1 — Example 3 mapping and checkpoint diagnosis

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/SOLUTE_EXAMPLE3_FAILURE_DIAGNOSIS.md`; `planning/tasks/15.3.1.todo.md`; `.development/15.3.1/`.
- **Depends on:** 15.1.1, 15.2.1, 15.2.2.
- **Spec:** Audit the Example 3 Fortran-to-Rust effective mapping for physical/material/water parameters, dispersion, initial conditions, species order, reaction/process rates, boundaries, units, and orientation, using canonical manifests, `CASES.json`, and relevant source. Since the retained pair has no common internal endpoints, do not align its internal steps by index or call them endpoint parity. Use a minimal bounded diagnostic reproduction with matching initial state and selected profile/checkpoint times, then compare per-species effective inputs, representative assembled rows/RHS and state. Separate input translation mismatches from operator/process differences, and report which large mixed-sign residual features are explained versus unresolved. Avoid rerunning the 1001-node full profile matrix if a smaller source-faithful discriminator suffices; justify any required run.
- **Do not touch:** production sources, reference source, casebook, retained results, and prior diagnosis reports.
- **Anchors:** research “Example 3”; `CASES.json` Example 3 mapping; `solute.rs` process/transport assembly; `SOLUTE.FOR` input/process and dispersion paths; parity evidence schedule note.
- **Acceptance:** complete source-to-field mapping table; matched-state/checkpoint definition; explicit treatment of no common endpoints; first proven divergence or precise remaining evidence gap; bounded-run accounting; no claims unsupported by aligned records. `git diff --check`; task-local trace consistency check if created.

### 15.4.1 — Synthesis and targeted-fix pathways

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/SOLUTE_CONCENTRATION_FAILURES_DIAGNOSIS.md`; `planning/tasks/15.4.1.todo.md`.
- **Depends on:** 15.1.1, 15.2.1, 15.2.2, 15.3.1.
- **Spec:** Synthesize the five failing families and the passing `reaction_zero` control. For each family state: observed signature; earliest proven divergence; evidence and confidence; hypotheses falsified; smallest candidate code boundary for a later fix plan; regression cases/records needed; and open questions. Group families only where traces prove a shared cause. Write follow-on plan recommendations as bounded investigation-to-fix pathways, not code changes or a predetermined algorithm choice. Explicitly retain the 9,092 record count as descriptive parity evidence and keep printed parity separate from continuum/physical accuracy.
- **Do not touch:** implementation, tests, casebook, retained reports, and previous orchestration artifacts.
- **Anchors:** this orchestration’s four diagnosis reports; research document and printed parity evidence.
- **Acceptance:** all five failure cases and `reaction_zero` control covered; each causal claim points to a trace row/source location; each later fix pathway names scope, regression and uncertainty; no unsupported shared-cause grouping; `git diff --check`.

## Risks and review gates

- A trace from separately initialized runs is not a same-state comparison. Verify initial arrays and effective input values before comparing operators.
- Fortran and Rust may have different step schedules. Compare a genuinely common first interval or explicitly construct a controlled diagnostic; never zip unrelated internal steps.
- Instrumentation can perturb or obscure behavior. Keep it task-local, minimal, and hash the uninstrumented sources and patch.
- The retained comparison is profile-level evidence. It establishes printed interval disagreement, not which solver is correct.
- If a numerical trace requires changing production behavior or broadening the launch matrix, stop and return a concrete amendment request.

## Plan self-review

**Spec axis:** covers reaction, pure diffusion, scalar Robin, and Example 3 individually; uses the passing reaction-zero control; culminates in pathways for targeted later fixes. No numerical fix or analytical reference work is assigned.

**Standards axis:** each task has explicit file scope, dependencies, anchors, output, and check. Parallel work is limited to Wave 2 with task-local files and orchestrator-managed shared resources. The sequence advances from smallest discriminating case to synthesis. Diagnostic command/count/storage details are mandatory worker micro-plan facts and an orchestrator gate; no command is run during planning.

Plan approval is for this bounded investigation orchestration only. Execution starts with wave 1 after explicit approval.

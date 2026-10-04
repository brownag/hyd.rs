# Orchestration 16 — Targeted solute concentration parity fixes

Status: **Approved by user on 2026-09-30.** No production code, tests, builds, or solvers were run while preparing this plan. User authorized the bounded diagnostic runs and conditional implementation of every Rust correction supported by evidence; a no-fix outcome is acceptable when no Rust defect is established, with closer investigation deferred to a later session.
Companion files: [worker prompt](PROMPT_TEMPLATE_16.md), [state](STATE_16.md).

## Goal & scope

**Goal.** Use Orchestration 15's evidence to justify only targeted Rust changes that have a demonstrated cause for a printed concentration parity discrepancy. Where the cause is still ambiguous, establish the minimal normalized equation/operator discriminator first. Keep printed-output parity as the accepted comparison target for the retained cases, without treating it as proof of physical or continuum accuracy.

**In scope:**

- Normalize and compare the reaction equations, diffusion operator, scalar Robin rows, and Example 3 first-interval rows at the earliest observed divergence.
- Run new, narrowly scoped traces only after their launch protocol is explicitly authorized; Orchestration 15's failed/missing traces do not authorize retries.
- Implement a Rust correction only where the normalized evidence identifies a specific Rust discrepancy; otherwise record a no-fix disposition with the remaining cause gap.
- For each implementation, add targeted Rust regressions and run the existing authenticated printed-parity checks for affected retained cases plus passing `reaction_zero`.

**Out of scope:** changing Fortran reference sources; changing authenticated inputs, manifests, print intervals, case definitions, or parity tolerances; broad solver/discretization redesign; selecting the physically correct solver from parity evidence; claims of physical or continuum accuracy; broad convergence work unless a proposed change has separate physical/continuum evidence and a separately approved scope.

**Verbatim constraints:**

> “Separate evidence-backed code fixes from additional diagnostic work needed to justify a fix. Do not prescribe numerical changes for unresolved causes.”
>
> “Preserve printed-output parity as the established comparison target; do not imply it establishes physical or continuum accuracy. Keep any diagnostic gaps explicit.”
>
> “Make the normalized equation comparison or other minimal discriminator a prerequisite wherever Orchestration 15 did not establish a cause.”
>
> “Preserve authenticated inputs, manifests, print intervals, case definitions, and tolerances. Do not change Fortran reference sources as a corrective fix.”
>
> “Do not modify production code, run builds/tests/solvers, or commit while creating the plan.”

## Baseline and reconnaissance

- Repository: `/home/andrew/AI/hyd.rs`; branch `phase4-water-acceptance`; current committed baseline `ec90ba6dcff06de1326670d6a0af46c60a0cbde2`.
- Working tree was clean at plan start (`git status --porcelain=v1` empty).
- No `AGENTS.md` or `CONTRIBUTING*` was found by `rg --files`.
- No build, test, or solver was run. Prior recorded baseline in `planning/STATE_15.md` is 145 Rust tests, 13 runner/outlet Python tests, 17 benchmark tests, 86 convergence tests, formatter, Clippy with warnings denied, and diff check. This is historical only, not a newly verified baseline.
- Main anchors: `crates/hydrus-core/src/solute.rs` (1,474 lines; `assemble_and_solve_species_with_processes`, `assemble_flux_operator`, `apply_endpoint_boundary_rows`); `src_reference/SOLUTE.FOR` (2,105 lines; `Coeff`, `MatSet`, upstream weighting, `Disper`); `reference/solute_verification/CASES.json` (2,849 lines; six retained cases); `tools/solute_printed_parity.py` (263 lines) and `tools/test_solute_printed_parity.py` (108 lines).
- Printed parity check: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity`; retained comparison command from Orchestration 14: `PYTHONDONTWRITEBYTECODE=1 python3 tools/solute_printed_parity.py --cases reference/solute_verification/CASES.json --inventory planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json --output-root .development/14.3.1`. Implementation acceptance must invoke the established comparison read-only against authenticated retained pairs; it must not overwrite `.development/14.3.1` or modify inputs. Use a task-local output root or the comparator's read-only mode if supported; verify CLI behavior before accepting a task command.
- Rust targeted tests should be scoped to `crates/hydrus-core` and named in each accepted micro-plan. No test command is claimed as run or green in this planning phase.

## Evidence and plan decisions

Evidence sources: `planning/master/SOLUTE_CONCENTRATION_FAILURES_RESEARCH.md`, `planning/master/SOLUTE_CONCENTRATION_FAILURES_DIAGNOSIS.md`, the four linked family diagnosis reports, `planning/master/SOLUTE_PRINTED_PARITY_EVIDENCE.md`, and the authenticated inventory/comparison outputs they identify.

- There is no evidence-backed numerical correction ready to implement at plan start. A printed parity failure identifies disagreement with a Fortran print interval, not which solver is physically/continuum accurate.
- Reaction: `reaction_zero` passes. Active cases first differ in S1 assembly/state; S2/S3 differences grow. Competing storage weighting, sink conversion, and chain mapping remain unresolved because row formulations are not normalized.
- Diffusion: Rust trace exists; Fortran stopped before assembly because `'.\\Selector.in'` was missing. No retry was authorized in Orchestration 15.
- Scalar Robin: first solve's interior and endpoint rows differ; upstream weighting, FE/FV assembly, and boundary treatment are not isolated.
- Example 3: source-to-field audit and matched 50 h profiles exist; retained pair has no common internal endpoints, and Fortran first-step rows are missing because trace instrumentation missed `Solute` call time. No retry was authorized.
- The unintended seventh `reaction_distinct/Fortran` launch from task 15.1.1 remains documented and excluded from approved comparison evidence.
- User decision — 2026-09-30: authorized the diffusion and Example 3 diagnostic runs within the per-task bounds below; authorized implementation of every correction supported by evidence; accepted a documented no-fix outcome where no Rust defect is established, with closer investigation deferred to a later session. This authorizes only the scoped runs in 16.1.3 and 16.1.4 after the required task-local pre-launch ledger is prepared and reviewed; it does not authorize extra attempts or scope expansion.
- Implementation tasks remain conditional children of the applicable discriminator. If the discriminator does not establish a Rust defect, close that branch with a no-fix report; do not modify numerical code to force parity.

## Task DAG and wave table

| Wave | Task | Depends on | Deliverable |
|---|---|---|---|
| 1 | 16.1.1 Reaction equation normalization | — | Common-unit/common-basis S1 equation comparison and explicit branch decision |
| 1 | 16.1.2 Scalar Robin row normalization | — | Common-basis inlet/interior/outlet row comparison and isolated or unresolved mechanism |
| 1 | 16.1.3 Diffusion Fortran first-step discriminator | — | One bounded Fortran trace compared with preserved Rust trace, or explicit stopped disposition |
| 1 | 16.1.4 Example 3 first-step discriminator | — | Bounded Fortran trace at actual `Solute` entry and matched physical points, or explicit stopped disposition |
| 2 | 16.2.1 Reaction correction (conditional) | 16.1.1 identifies a specific Rust defect | Narrow Rust change or no-fix disposition; tests/parity for equal, distinct, zero |
| 2 | 16.2.2 Scalar Robin correction (conditional) | 16.1.2 identifies a specific Rust defect | Narrow Rust change or no-fix disposition; tests/parity for scalar_robin and reaction_zero control |
| 2 | 16.2.3 Diffusion correction (conditional) | 16.1.3 identifies a specific Rust defect | Narrow Rust change or no-fix disposition; tests/parity for diffusion_cosine and reaction_zero control |
| 2 | 16.2.4 Example 3 correction (conditional) | 16.1.4 identifies a specific Rust defect | Narrow Rust change or no-fix disposition; tests/parity for example3 and reaction_zero control |

Wave 1 tasks have disjoint output scopes. Solver runs, if authorized, must be serialized because they share compiler/runtime resources. Wave 2 tasks have disjoint Rust source/test scopes in the proposed split; do not run concurrently if code inspection finds a shared file is needed. Any shared production/test file dependency requires serializing or recombining the tasks before execution. A no-fix finding ends that family branch and does not block other branches.

## Shared run and acceptance gates

Before any task starts a new solver:

1. This orchestration authorizes the one bounded Fortran diagnostic run in tasks 16.1.3 and 16.1.4. Before launch, the worker must prepare the concrete task-local ledger and exact command and the orchestrator must verify it against the stated limits. The authorization does not extend to retries or other cases.
2. Re-authenticate canonical manifests, requests, sources, and casebook against `planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json`; write a pre-launch hash/byte-count ledger before copying or launching.
3. Use task-local copies/artifacts only under `.development/16.<task>/`; preserve retained inputs and task 15 artifacts.
4. Limit each solver task to **one total solver launch**, with **no retry**, **60 second timeout per launch**, and **104,857,600 byte task artifact cap**. Preflight/build are not solver launches but must have their own bounded commands and be included in the cap. Stop on any failed launch, hash mismatch, cap breach, missing trace fields, or inability to map state/units. Do not compensate with another run.
5. Record attempt outcome and final integrity hashes, including a failed attempt. Keep each family separate and compare only the checkpoints/species/coordinates named by the task.

These ceilings are the authorized task limits. The exact command, source/request hashes, and artifacts must appear in a task's reviewed pre-launch ledger before launch. If ledger review finds the run cannot meet the bounds, stop and request a new plan decision; do not launch.

For every implementation task that proceeds, acceptance requires both (a) named targeted Rust tests and (b) the existing printed-parity comparator on that family's retained case(s) plus `reaction_zero`. Preserve original print intervals and show per-field/species outcomes, not only aggregate counts. Record whether the correction changes a physically meaningful equation; parity success alone cannot support a physical/continuum claim. Such a claim needs separate analytical/convergence/experimental evidence and a separately scoped plan.

## Task specifications

### 16.1.1 — Normalize active-reaction equations

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/16.1.1.todo.md`; `planning/master/SOLUTE_REACTION_NORMALIZED_EQUATIONS.md`; `.development/16.1.1/` for scratch derivations only.
- **Depends on:** none.
- **Spec:** Derive the S1 first-step equation on both sides in common units and a declared common row basis, including aqueous/equilibrium storage, Fortran `Sorb` semantics, local mass/element weighting, sink conversion, boundary contribution, and time-level conventions. Use authenticated `reaction_equal` trace values and source anchors; do not use the unauthorized distinct/Fortran launch as accepted evidence. Then test S2 and S3 source mapping only after S1 is normalized. Conclude with either a specific, falsifiable Rust defect and proposed smallest change boundary, or `no_fix_cause_unisolated` and the exact missing discriminator. Do not execute solvers, builds, or numerical code.
- **Do not touch:** production files, Fortran source, casebook, retained manifests/inputs, tolerances, task 15 reports/artifacts.
- **Anchors:** `solute.rs` `assemble_and_solve_species_with_processes` around 1145–1290; `SOLUTE.FOR` `Coeff` around 241–270 and chain coupling around 648–685; reaction diagnosis “Trace comparison”; cases `reaction_equal`, `reaction_zero`, `reaction_distinct`.
- **Acceptance:** written dimensional derivation, node/element normalization, boundary/time-level mapping, line-by-line trace mapping, and explicit disposition. `git diff --check`. No build/test/solver command.

### 16.1.2 — Normalize scalar Robin rows

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/16.1.2.todo.md`; `planning/master/SOLUTE_ROBIN_NORMALIZED_ROWS.md`; `.development/16.1.2/` scratch only.
- **Depends on:** none.
- **Spec:** Normalize the existing `scalar_robin` first-solve inlet, representative interior, and outlet equations into a common physical orientation and common row basis. Separate upstream weight, FE/FV spatial weighting, time weighting, and endpoint closure algebraically; use the existing two-run trace and no counterfactual solver run. State which terms can be falsified from retained data and whether a unique Rust defect follows. If not, return `no_fix_cause_unisolated`; do not prescribe an endpoint or weighting change.
- **Do not touch:** production, Fortran source, casebook, retained artifacts, parity comparator/tolerances.
- **Anchors:** `solute.rs` 1299–1358; `SOLUTE.FOR` `MatSet` 755–925, upstream weights 1001–1014; Robin report “Aligned first-step evidence”.
- **Acceptance:** normalized row equations and predicted coefficients for inlet/interior/outlet, sign/index map, separate effect attribution, explicit disposition. `git diff --check`. No builds/tests/solvers.

### 16.1.3 — Obtain missing diffusion Fortran discriminator (run gated)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/16.1.3.todo.md`; `.development/16.1.3/` task-local run/provenance/trace only; `planning/master/SOLUTE_DIFFUSION_NORMALIZED_TRACE.md`.
- **Depends on:** none.
- **Spec:** Prepare one Fortran-only `diffusion_cosine` first-step trace using the exact authenticated 65-node case, preserving physical inputs and the previously captured Rust trace. Resolve the literal Linux alias only inside task-local input copies; log and authenticate that alias. Capture accepted timestep, all row/RHS coefficients or enough indexed rows to reconstruct all 65 rows, endpoint closure, and solved state. Normalize to Rust indexing before comparison. Do not reuse the 15.2.1 launch or claim its failure grants a retry. This task is authorized for one run within the shared bounds after the task-local pre-launch ledger and exact command are prepared and verified by the orchestrator; it does not authorize retries or scope expansion.
- **Do not touch:** production/Fortran source, cases, retained inputs/manifests, task 15 artifacts, parity settings.
- **Anchors:** diffusion report “Rust trace and retained profile evidence”; `solute.rs` `assemble_flux_operator`/`apply_endpoint_boundary_rows`; `SOLUTE.FOR` `Disper`, `Coeff`, `MatSet`; case `diffusion_cosine`.
- **Run protocol:** exactly one solver launch total; 60 s timeout; 100 MiB artifact cap; pre-launch authenticated ledger; no retry. Stop on alias/preflight/hash mismatch, nonzero exit, missing rows/state, or cap breach. Preflight and build commands must be bounded and recorded; solver run command must be exact in the approved ledger.
- **Acceptance:** either (a) authorized trace yields normalized row/state comparison and a falsifiable cause, or (b) no authorization/failure yields a clearly marked blocked/no-fix disposition with no second attempt. `git diff --check`. No automatic rerun.

### 16.1.4 — Obtain missing Example 3 Fortran discriminator (run gated)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/16.1.4.todo.md`; `.development/16.1.4/`; `planning/master/SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md`.
- **Depends on:** none.
- **Spec:** Prepare one Fortran-only first-interval trace for the authenticated Example 3-derived fixture. Instrumentation must be triggered by a measured `Solute` entry-time value rather than an assumed zero-time guard. Capture effective dispersion, weighting, storage/reaction terms, rows/RHS, and solved values at inlet/midpoint/outlet for all three species; normalize the reversed Fortran coordinate/index convention. Preserve authored input fields and task 15's matched 50 h profiles. Do not pair internal endpoints or steps by index. This task is authorized for one run within the shared bounds after the task-local pre-launch ledger and exact command are prepared and verified by the orchestrator; it does not authorize retries or scope expansion.
- **Do not touch:** production/Fortran source, case definitions, retained input/manifests, task 15 artifacts, print intervals/tolerances.
- **Anchors:** Example 3 diagnosis “Source-to-field mapping” and “Finding and evidence gap”; `HYDRUS.FOR` `Solute` call around 506; `SOLUTE.FOR` `Solute`/`Coeff`/`Disper`/`MatSet`; Rust assembly 1162–1358; case `example3`.
- **Run protocol:** exactly one solver launch total; 60 s timeout; 100 MiB artifact cap; pre-launch hashes/bytes ledger; no retry. Stop on failed preflight/run, unobserved call entry time, missing requested species/rows, cap breach, or mapping ambiguity. No 15.3.1 authorization carries forward.
- **Acceptance:** a normalized three-location trace tied to actual call time and source identity, or explicit no-run/failed-discriminator disposition; retain profile-only 50 h evidence without endpoint claims. `git diff --check`. No automatic rerun.

### 16.2.1 — Reaction correction (conditional)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit if defect established:** `crates/hydrus-core/src/solute.rs` and a new or existing narrowly scoped test module under `crates/hydrus-core/src/` (name confirmed during micro-plan).
- **Can create:** task micro-plan; task-local comparison outputs under `.development/16.2.1/`.
- **Depends on:** 16.1.1 identifies a specific Rust error; otherwise record no-fix and stop.
- **Spec:** Implement only the coefficient/source correction predicted by normalized equations. Preserve sequential species semantics and boundary behavior outside demonstrated cause. Add regressions covering active `reaction_equal`, `reaction_distinct`, and passing `reaction_zero`. Do not edit Fortran, requests, cases, print intervals, manifests, or tolerances. If evidence implies a physically meaningful model change rather than a parity implementation defect, stop and require separate physical/continuum evidence and plan.
- **Anchors/reference:** reaction-normalization report; Rust process assembly 1145–1290; tests of `advance_solute_step_with_processes` and harness test patterns; retained parity records.
- **Acceptance:** exact named focused test command, e.g. `cargo test -p hydrus-core <targeted_filter>` as specified in approved micro-plan, plus `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity` and read-only parity comparison for `reaction_equal`, `reaction_distinct`, `reaction_zero` against authenticated retained data. Do not overwrite canonical comparison output. Report per-species intervals and zero-control status. `git diff --check`.
- **Rollback:** revert only this task's Rust source/test diff; discard only `.development/16.2.1/` outputs. Preserve evidence and retained data.

### 16.2.2 — Scalar Robin correction (conditional)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit if defect established:** `crates/hydrus-core/src/solute.rs` plus a task-local Rust test file under `crates/hydrus-core/src/`.
- **Can create:** task micro-plan; `.development/16.2.2/` parity output.
- **Depends on:** 16.1.2 identifies a specific Rust error; otherwise record no-fix and stop.
- **Spec:** Make only the isolated Robin term correction supported by the normalized derivation. Keep other discretization choices unchanged. Add a `scalar_robin` first-solve/operator regression and retain a no-advection or passing `reaction_zero` boundary control if relevant. Do not alter case setup or compare different print schedules.
- **Anchors:** normalized row report; Rust transport/boundary functions 1299–1358; existing solute transport/boundary tests.
- **Acceptance:** targeted Rust tests; Python printed-parity tests; authenticated printed comparison for `scalar_robin` and `reaction_zero`, with original intervals and per-node records. If implementation requires changing a shared file touched by another wave task, serialize before execution. `git diff --check`.
- **Rollback:** revert only this task's Rust source/test diff and its task-local outputs.

### 16.2.3 — Diffusion correction (conditional)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit if defect established:** `crates/hydrus-core/src/solute.rs` plus a task-local Rust test file under `crates/hydrus-core/src/`.
- **Can create:** task micro-plan; `.development/16.2.3/` parity output.
- **Depends on:** 16.1.3 yields a normalized comparison that identifies a specific Rust error; otherwise no-fix disposition.
- **Spec:** Correct only the diffusion coefficient/stencil/storage/boundary term proven inconsistent with the accepted comparison target. Preserve zero flow and inactive upstream weighting. Do not introduce an accuracy claim based on retained Fortran parity.
- **Anchors:** normalized diffusion trace; `assemble_flux_operator`, `apply_endpoint_boundary_rows`; existing diffusion tests and case `diffusion_cosine`.
- **Acceptance:** targeted Rust diffusion test(s), Python printed-parity tests, read-only retained comparison for `diffusion_cosine` and `reaction_zero`; preserve node-level print interval detail and negative-tail signature. `git diff --check`.
- **Rollback:** revert only task Rust source/test edits and discard task-local output.

### 16.2.4 — Example 3 correction (conditional)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit if defect established:** `crates/hydrus-core/src/solute.rs` plus a task-local Rust test file under `crates/hydrus-core/src/`.
- **Can create:** task micro-plan; `.development/16.2.4/` parity output.
- **Depends on:** 16.1.4 yields normalized rows/state and identifies a specific Rust error; otherwise no-fix disposition.
- **Spec:** Correct only a demonstrated translation/assembly/state defect. Regress the actual identified process and preserve the full 1001-node domain, species order, authenticated fields, and retained 0/50/100/200 h profile schedule. Compare profiles only at common printed times; do not infer internal endpoint parity. Any adjustment that changes physical equations needs independent physical/continuum evidence under separate scope.
- **Anchors:** normalized Example 3 report; Rust assembly 1162–1358; existing verification harness/case tests; `CASES.json` Example 3 mapping.
- **Acceptance:** targeted Rust test(s), Python printed-parity tests, retained comparison for `example3` and `reaction_zero`; per-species/per-profile interval reports and matched-coordinate orientation checks. `git diff --check`.
- **Rollback:** revert only task Rust source/test changes and discard task-local parity output.

## Risks and explicit gaps

- Discretization forms cannot be compared by raw lower/diagonal/upper values; normalization must precede attribution.
- A matching printed interval is a source-output replication result only. It does not establish continuum accuracy, physical validity, or preference over Fortran.
- Retained Example 3 output has no common internal endpoints; no task may claim endpoint parity.
- The task 15.1.1 extra Fortran run is excluded from approved comparisons and remains documented.
- Diffusion and Example 3 traces remain absent on the Fortran side; their new runs require separate user authorization and strict no-retry handling.
- The existing parity CLI's `--output-root` writes result files. Every implementation task must use a task-local output root and must not overwrite `.development/14.3.1`.
- Physical/continuum impact of any proposed change is unknown until assessed independently. If a candidate fix changes the modeled equation, parity acceptance is insufficient.

## Approval and remaining judgment calls

User approved this plan's decision framework on 2026-09-30: bounded diagnostics are authorized; implement all corrections justified by their evidence; record no-fix and defer closer investigation when no Rust defect is established. Wave 1 diagnostic work may proceed after the orchestrator reviews each task-local ledger. Conditional implementation tasks proceed only when their dependency identifies a specific Rust defect. No additional user decision is required to begin within these limits.

No numerical source change is authorized merely to reduce interval failures. Execute through the wave-orchestration workflow.

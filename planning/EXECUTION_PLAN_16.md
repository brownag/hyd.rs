# Orchestration 16 — Targeted solute concentration parity fixes

Status: **Complete.** Original scope approved 2026-09-30; amendment approved by the user 2026-10-05. Waves 1–6 are closed. Reaction, Robin, diffusion, and Example 3 have `no_fix_cause_unisolated` dispositions. No production Rust correction was justified by the captured evidence; no physical or continuum accuracy claim is made.
Companion files: [worker prompt](PROMPT_TEMPLATE_16.md), [state](STATE_16.md).

## Goal & scope

**Goal.** Use Orchestration 15's evidence to justify only targeted Rust changes that have a demonstrated cause for a printed concentration parity discrepancy. Where the cause is still ambiguous, establish the minimal normalized equation/operator discriminator first. Keep printed-output parity as the accepted comparison target for the retained cases, without treating it as proof of physical or continuum accuracy.

**In scope:**

- Normalize and compare the reaction equations, diffusion operator, scalar Robin rows, and Example 3 first-interval rows at the earliest observed divergence.
- Build a task-local, reusable Fortran diagnostic runner and use it to capture missing diffusion and Example 3 evidence. Run preparation, builds, and solvers as often as needed to produce valid evidence; record every attempt. Follow-up runs may adjust task-local instrumentation or use narrowly designed diagnostic variants needed to distinguish candidate causes.
- Implement a Rust correction only where the normalized evidence identifies a specific Rust discrepancy; otherwise record a no-fix disposition with the remaining cause gap.
- For each implementation, add targeted Rust regressions and run the existing authenticated printed-parity checks for affected retained cases plus passing `reaction_zero`.

**Out of scope:** changing Fortran reference sources as a corrective implementation; mutating canonical authenticated inputs, manifests, print intervals, case definitions, or parity tolerances; broad solver/discretization redesign; selecting the physically correct solver from parity evidence; claims of physical or continuum accuracy; broad convergence work unless a proposed change has separate physical/continuum evidence and a separately scoped plan. Task-local derived diagnostic inputs, instrumentation, and repeated diagnostic runs are in scope when needed to isolate the named causes.

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

## Amendment decision log

- 2026-10-04 — User directed that timeouts and artifact caps must not block needed work or trigger repeated user input, and approved retries and follow-up runs within the diffusion and Example 3 diagnostic scope. The amendment removes fixed timeout/artifact ceilings and one-launch/no-retry restrictions as gates. Each attempt must still be logged; routine in-scope retries do not require another user authorization.
- 2026-10-04 — Add a setup wave that creates and validates a task-local Fortran-only driver, followed by serialized diffusion and Example 3 trace tasks. Conditional correction tasks follow those traces.

## Baseline and reconnaissance

- Repository: `/home/andrew/AI/hyd.rs`; branch `phase4-water-acceptance`; current committed baseline `ec90ba6dcff06de1326670d6a0af46c60a0cbde2`.
- Working tree was clean at plan start (`git status --porcelain=v1` empty).
- No `AGENTS.md` or `CONTRIBUTING*` was found by `rg --files`.
- No build, test, or solver was run. Prior recorded baseline in `planning/STATE_15.md` is 145 Rust tests, 13 runner/outlet Python tests, 17 benchmark tests, 86 convergence tests, formatter, Clippy with warnings denied, and diff check. This is historical only, not a newly verified baseline.
- Main anchors: `crates/hydrus-core/src/solute.rs` (1,474 lines; `assemble_and_solve_species_with_processes`, `assemble_flux_operator`, `apply_endpoint_boundary_rows`); `src_reference/SOLUTE.FOR` (2,105 lines; `Coeff`, `MatSet`, upstream weighting, `Disper`); `reference/solute_verification/CASES.json` (2,849 lines; six retained cases); `tools/solute_printed_parity.py` (263 lines) and `tools/test_solute_printed_parity.py` (108 lines).
- Diagnostic runner anchors: `.development/15.2.1/trace_runner.py` (172 lines; hard-coded task root and two-launch interface), `.development/15.3.1/trace_runner.py` (292 lines; preparation/instrumentation patterns but assumes two runs and a 300 s solver timeout), and `tools/run_solute_verification_reference.py` (1,217 lines; `generate_decks`, `build_reference`, launch adapter). Reuse suitable helpers without inheriting old task roots or fixed resource ceilings.
- Printed parity check: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity`; retained comparison command from Orchestration 14: `PYTHONDONTWRITEBYTECODE=1 python3 tools/solute_printed_parity.py --cases reference/solute_verification/CASES.json --inventory planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json --output-root .development/14.3.1`. Implementation acceptance must invoke the established comparison read-only against authenticated retained pairs; it must not overwrite `.development/14.3.1` or modify inputs. Use a task-local output root or the comparator's read-only mode if supported; verify CLI behavior before accepting a task command.
- Rust targeted tests should be scoped to `crates/hydrus-core` and named in each accepted micro-plan. No test command is claimed as run or green in this planning phase.

## Evidence and plan decisions

Evidence sources: `planning/master/SOLUTE_CONCENTRATION_FAILURES_RESEARCH.md`, `planning/master/SOLUTE_CONCENTRATION_FAILURES_DIAGNOSIS.md`, the four linked family diagnosis reports, `planning/master/SOLUTE_PRINTED_PARITY_EVIDENCE.md`, and the authenticated inventory/comparison outputs they identify.

- There is no evidence-backed numerical correction ready to implement at plan start. A printed parity failure identifies disagreement with a Fortran print interval, not which solver is physically/continuum accurate.
- Reaction: `reaction_zero` passes. Active cases first differ in S1 assembly/state; S2/S3 differences grow. Competing storage weighting, sink conversion, and chain mapping remain unresolved because row formulations are not normalized.
- Diffusion: Rust trace exists; Fortran stopped before assembly because `'.\\Selector.in'` was missing. The amendment allows corrected task-local reruns and follow-ups.
- Scalar Robin: first solve's interior and endpoint rows differ; upstream weighting, FE/FV assembly, and boundary treatment are not isolated.
- Example 3: source-to-field audit and matched 50 h profiles exist; retained pair has no common internal endpoints, and Fortran first-step rows are missing because trace instrumentation missed `Solute` call time. The amendment allows corrected task-local reruns and follow-ups.
- The unintended seventh `reaction_distinct/Fortran` launch from task 15.1.1 remains documented and excluded from approved comparison evidence.
- User decision — 2026-09-30: authorized diagnostic work for diffusion and Example 3 and implementation of supported Rust corrections; no-fix is acceptable where evidence does not establish a Rust defect.
- User decision — 2026-10-04: approved retries and follow-up runs as needed in those diagnostic scopes, and directed that fixed timeouts/artifact caps not block work or trigger excessive user-input loops. Each attempt remains logged; task-local instrumentation corrections and targeted diagnostic variants are in scope when needed to isolate the cause.
- Implementation tasks remain conditional children of the applicable discriminator. If the discriminator does not establish a Rust defect, close that branch with a no-fix report; do not modify numerical code to force parity.

## Task DAG and wave table

| Wave | Task | Depends on | Deliverable |
|---|---|---|---|
| 1 | 16.1.1 Reaction equation normalization | — | Common-unit/common-basis S1 equation comparison and explicit branch decision |
| 1 | 16.1.2 Scalar Robin row normalization | — | Common-basis inlet/interior/outlet row comparison and isolated or unresolved mechanism |
| 2 | 16.2.1 Fortran diagnostic runner setup | Existing task-15 drivers and shared adapter | Reusable task-local Fortran-only driver, validation procedure, attempt ledger |
| 3 | 16.3.1 Diffusion Fortran first-step discriminator | 16.2.1 | Normalized Fortran/Rust trace; follow-up runs as needed |
| 4 | 16.4.1 Example 3 Fortran first-interval discriminator | 16.2.1, 16.3.1 | Actual-call-time normalized trace at matched physical locations; follow-ups as needed |
| 5 | 16.5.1 Diffusion correction (conditional) | 16.3.1 identifies a specific Rust defect | Narrow Rust change or no-fix disposition; tests/parity for diffusion_cosine and reaction_zero control |
| 6 | 16.6.1 Example 3 correction (conditional) | 16.4.1 identifies a specific Rust defect | Narrow Rust change or no-fix disposition; tests/parity for example3 and reaction_zero control |

Wave 2 has one setup task. Diffusion and Example 3 diagnostics are in sequential waves because they share compiler/runtime resources. Diffusion and Example 3 correction tasks also edit the shared solute module, so they are in separate sequential waves. Reaction and Robin already have recorded no-fix decisions and require no additional implementation tasks. A no-fix finding ends its family branch and does not block unrelated branches.

## Shared run and acceptance gates

Before any task starts a new solver:

1. The amendment authorizes repeated preparation, builds, and diagnostic solver launches needed to obtain a valid discriminator for diffusion and Example 3. Log every attempt; a failed attempt informs correction or follow-up rather than automatically ending the task.
2. Re-authenticate canonical manifests, requests, sources, and casebook against `planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json`; write provenance before copying or launching and verify retained files afterward.
3. Use task-local copies and outputs under `.development/16.<task>/`; preserve canonical inputs and task 15 artifacts. Derived diagnostic variants may be created task-locally when their purpose and differences are recorded.
4. Fixed timeouts and artifact-size ceilings are not acceptance gates. Use process monitoring and workspace resources sensibly; report elapsed time and output size and keep outputs organized. If a process hangs, consumes resources unexpectedly, or output growth needs management, stop/adjust and continue with a corrected attempt without routine user reauthorization.
5. Before the first launch in each diagnostic task, the orchestrator reviews the approved worker micro-plan, task-local inputs, instrumentation, and run procedure. Follow-up launches within the approved family scope do not require repeated user input. Record command, identity, outcome, artifacts, and reason for each retry.
6. Keep families separate and compare only named species, physical coordinates, checkpoints, and state variables. For Example 3, do not pair internal endpoints or steps by index; retain 0/50/100/200 h profile comparisons.

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

### 16.2.1 — Build reusable Fortran diagnostic runner

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `planning/STATE_16.md` (append the task report); task-local runner only.
- **Can create:** `planning/tasks/16.2.1.todo.md`; `.development/16.2.1/` runner, usage notes, and driver validation outputs.
- **Depends on:** completed task-15 diagnosis and existing runner/adapter reconnaissance in this plan.
- **Spec:** Create a task-local Python driver for Fortran-only diagnostics that can authenticate inputs against the inventory; stage isolated case inputs and aliases; apply task-local Fortran trace instrumentation; build the reference solver; launch selected runs; preserve outputs and append each attempt to a durable ledger; verify source/input/output identities; and support reruns without overwriting prior evidence. Reuse appropriate helper logic from `.development/15.2.1/trace_runner.py`, `.development/15.3.1/trace_runner.py`, and `tools/run_solute_verification_reference.py`. Correct their hard-coded roots, coupled Rust builds/launches, and fixed launch/timeout/artifact ceilings. Do not put this runner into production tools or use it to change canonical data. Support task-specific instrumentation hooks or clearly defined separate instrumentation files so diffusion and Example 3 do not collide.
- **Do not touch:** production Rust source, canonical Fortran source, canonical requests/cases/manifests/inventory, task-15 artifacts, parity tolerances.
- **Anchors:** `trace_runner.py` `prepare/build/run/verify` functions in tasks 15.2.1 and 15.3.1; adapter `generate_decks` and `build_reference`; task-15 diagnoses for path aliases and actual Example 3 `Solute` entry time.
- **Acceptance:** documented exact task-local usage; dry-run/preparation checks for both target fixtures; automated checks that prove every solver attempt is logged before launch, retries use unique output paths, and retained files are hash-verified; task-local driver tests pass with `PYTHONDONTWRITEBYTECODE=1 python3 .development/16.2.1/test_trace_runner.py`. Preparation commands: `python3 .development/16.2.1/trace_runner.py prepare --case diffusion_cosine --task-root .development/16.3.1 --dry-run` and the corresponding `example3` / `.development/16.4.1` command. No fixed timeout or output-size cap may reject an otherwise useful attempt. `git diff --check`. Do not launch solvers in setup task.

### 16.3.1 — Obtain diffusion Fortran discriminator

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `planning/STATE_16.md` (append the task report).
- **Can create:** `planning/tasks/16.3.1.todo.md`; `.development/16.3.1/` task-local run/provenance/trace only; `planning/master/SOLUTE_DIFFUSION_NORMALIZED_TRACE.md`.
- **Depends on:** 16.2.1.
- **Spec:** Use the task-local runner to obtain a Fortran `diffusion_cosine` first-step trace for the authenticated 65-node case, preserving physical inputs and the Rust trace. Resolve the literal Linux alias only inside task-local copies. Capture accepted timestep, all row/RHS coefficients or enough indexed rows to reconstruct all 65 rows, endpoint closure, and solved state. Normalize to Rust indexing before comparison. Make additional runs or diagnostic variants as needed when instrumentation or setup fails or evidence does not distinguish the candidate terms. Every attempt is recorded; failures do not by themselves close the task. Continue until a useful discriminator is obtained or the orchestrator documents why further work cannot answer the question.
- **Do not touch:** production/Fortran source, cases, retained inputs/manifests, task 15 artifacts, parity settings.
- **Anchors:** diffusion report “Rust trace and retained profile evidence”; `solute.rs` `assemble_flux_operator`/`apply_endpoint_boundary_rows`; `SOLUTE.FOR` `Disper`, `Coeff`, `MatSet`; case `diffusion_cosine`.
- **Run protocol:** authenticate and record every source/request/input identity; use task-local aliases and outputs; record every prepare/build/launch command, outcome, and artifact. No fixed timeout or artifact-size cap is an execution gate. Do not overwrite prior attempts.
- **Acceptance:** normalized 65-row/state comparison against the preserved Rust trace. Use the shared interface: `python3 .development/16.2.1/trace_runner.py prepare --case diffusion_cosine --task-root .development/16.3.1`; `build --task-root .development/16.3.1`; `run --task-root .development/16.3.1 --attempt-label <unique-label>`; repeat `run` with a new label when follow-up is needed; then `verify --task-root .development/16.3.1`. If the current approach fails, devise and execute a different relevant task-local instrumentation or diagnostic variant; do not stop because of elapsed time, output size, or an arbitrary retry count. Conclude with a limitation report only after documenting why available in-scope approaches cannot identify the operator cause. `git diff --check`.

### 16.4.1 — Obtain Example 3 Fortran discriminator

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `planning/STATE_16.md` (append the task report).
- **Can create:** `planning/tasks/16.4.1.todo.md`; `.development/16.4.1/`; `planning/master/SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md`.
- **Depends on:** 16.2.1 and 16.3.1; the diffusion task completes before Example 3 begins.
- **Spec:** Use the task-local runner for a Fortran first-interval trace of the authenticated Example 3-derived fixture. Instrumentation must be triggered by a measured `Solute` entry-time value, not an assumed zero-time guard. Capture effective dispersion, weighting, storage/reaction terms, rows/RHS, and solved values at inlet/midpoint/outlet for all three species; normalize the reversed Fortran coordinate/index convention. Preserve authored physical fields and task-15 matched 50 h profiles. Do not pair internal endpoints or steps by index. Make additional runs or diagnostic variants as needed to get the actual call time and requested trace fields; log every attempt and continue through corrected instrumentation/setup as needed.
- **Do not touch:** production/Fortran source, case definitions, retained input/manifests, task 15 artifacts, print intervals/tolerances.
- **Anchors:** Example 3 diagnosis “Source-to-field mapping” and “Finding and evidence gap”; `HYDRUS.FOR` `Solute` call around 506; `SOLUTE.FOR` `Solute`/`Coeff`/`Disper`/`MatSet`; Rust assembly 1162–1358; case `example3`.
- **Run protocol:** authenticate and record every source/request/input identity; use task-local aliases and outputs; record every preparation/build/launch and its artifacts. No fixed timeout or artifact-size cap is an execution gate. Preserve prior attempts.
- **Acceptance:** a normalized three-location trace tied to actual call time and source identity; retain profile-only evidence without endpoint claims. Commands use the shared interface with `--case example3 --task-root .development/16.4.1` for `prepare`, `build`, `run --attempt-label <unique-label>`, and `verify`; repeat launches under new labels for corrected instrumentation or focused variants. If the current approach fails, devise another relevant in-scope instrumentation or diagnostic method; do not stop due to elapsed time, output size, or an arbitrary retry count. Conclude with a limitation report only after documenting why available in-scope approaches cannot obtain or interpret the requested evidence. `git diff --check`.

### 16.5.1 — Diffusion correction (conditional)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit if defect established:** `crates/hydrus-core/src/solute.rs` plus a task-local Rust test file under `crates/hydrus-core/src/`, and `planning/STATE_16.md` for the task report.
- **Can create:** task micro-plan; `.development/16.5.1/` parity output.
- **Depends on:** 16.3.1 yields a normalized comparison that identifies a specific Rust error; otherwise no-fix disposition.
- **Spec:** Correct only the diffusion coefficient/stencil/storage/boundary term proven inconsistent with the accepted comparison target. Preserve zero flow and inactive upstream weighting. Do not introduce an accuracy claim based on retained Fortran parity.
- **Anchors:** normalized diffusion trace; `assemble_flux_operator`, `apply_endpoint_boundary_rows`; existing diffusion tests and case `diffusion_cosine`.
- **Acceptance:** the approved micro-plan must name the exact new regression test and `cargo test -p hydrus-core <test-filter>` command before execution. Also run `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity` and `PYTHONDONTWRITEBYTECODE=1 python3 tools/solute_printed_parity.py --cases reference/solute_verification/CASES.json --inventory planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json --output-root .development/16.5.1/parity`; preserve node-level print interval detail and negative-tail signature. `git diff --check`.
- **Rollback:** revert only task Rust source/test edits and discard task-local output.

### 16.6.1 — Example 3 correction (conditional)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit if defect established:** `crates/hydrus-core/src/solute.rs` plus a task-local Rust test file under `crates/hydrus-core/src/`, and `planning/STATE_16.md` for the task report.
- **Can create:** task micro-plan; `.development/16.6.1/` parity output.
- **Depends on:** 16.4.1 yields normalized rows/state and identifies a specific Rust error; otherwise no-fix disposition.
- **Spec:** Correct only a demonstrated translation/assembly/state defect. Regress the actual identified process and preserve the full 1001-node domain, species order, authenticated fields, and retained 0/50/100/200 h profile schedule. Compare profiles only at common printed times; do not infer internal endpoint parity. Any adjustment that changes physical equations needs independent physical/continuum evidence under separate scope.
- **Anchors:** normalized Example 3 report; Rust assembly 1162–1358; existing verification harness/case tests; `CASES.json` Example 3 mapping.
- **Acceptance:** the approved micro-plan must name the exact new regression test and `cargo test -p hydrus-core <test-filter>` command before execution. Also run `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity` and `PYTHONDONTWRITEBYTECODE=1 python3 tools/solute_printed_parity.py --cases reference/solute_verification/CASES.json --inventory planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json --output-root .development/16.6.1/parity`; report species/profile intervals and matched-coordinate orientation checks. `git diff --check`.
- **Rollback:** revert only task Rust source/test changes and discard task-local parity output.

## Risks and explicit gaps

- Discretization forms cannot be compared by raw lower/diagonal/upper values; normalization must precede attribution.
- A matching printed interval is a source-output replication result only. It does not establish continuum accuracy, physical validity, or preference over Fortran.
- Retained Example 3 output has no common internal endpoints; no task may claim endpoint parity.
- The task 15.1.1 extra Fortran run is excluded from approved comparisons and remains documented.
- The diffusion and Example 3 Fortran traces are now captured and normalized. They identify operator and state differences but do not establish which formulation Rust must implement.
- The existing parity CLI's `--output-root` writes result files. Every implementation task must use a task-local output root and must not overwrite `.development/14.3.1`.
- Physical/continuum impact of any proposed change is unknown until assessed independently. If a candidate fix changes the modeled equation, parity acceptance is insufficient.

## Approval and final disposition

The original framework was approved on 2026-09-30. The user approved this amendment on 2026-10-05. The setup wave and serialized diagnostics are complete; both conditional correction branches closed as no-fix because neither trace identified a specific Rust defect. In-scope follow-up runs were made without additional user approval.

No numerical source change was justified merely to reduce interval failures. The plan was executed through the wave-orchestration workflow; no production source changes or task commits were made. Planning/evidence artifacts are being preserved in a separate documentation recovery commit.

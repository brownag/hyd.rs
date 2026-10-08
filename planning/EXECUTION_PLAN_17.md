# Orchestration 17 — Explain and choose the Rust solute formulation

Status: **Approved and executed; user confirmed pinned-Fortran emulation as the target on 2026-10-08.** This plan compared the documented solute requirements, Rust's current formulation, and the pinned Fortran formulation. It authorizes no numerical implementation.
Companion files: [worker prompt](PROMPT_TEMPLATE_17.md), [state](STATE_17.md).

## Goal & scope

**Goal.** Explain why Rust's solute results differ from the pinned Fortran behavior by comparing the documented requirements with the equations and numerical choices in both implementations. Present the evidence, uncertainties, and consequences of the available formulation choices so the user can choose Rust's target before any numerical code changes are planned.

**In scope:** Read-only source/history/evidence review; separate reports for requirements, Rust, and Fortran; a synthesis memo that distinguishes measured differences from inferred causes; task-local diagnostic follow-ups if needed to resolve a specific causal question; a user decision gate.

**Out of scope:** Production or test changes; unnecessary builds, tests, solver launches, or numerical experiments; changes to pinned Fortran or canonical inputs/manifests/cases/tolerances; selecting a formulation on the user's behalf; implementing the eventual choice; claims that parity establishes physical or continuum accuracy. If the existing traces and source/history review leave a specific causal question unanswered that is needed to explain the options, a task-local diagnostic follow-up is in scope. A later implementation plan is separate and requires the user's formulation choice.

**Verbatim constraints carried forward:**

> “write a prompt for the next wave orchestration that will resolve which formulation Rust should use with my input”
>
> “im not sure why the Rust deviates from pinned fortran behavior”
>
> “yes, that is literally what i asked for” — confirming the comparison-first, options-to-user workflow before code changes.
>
> “allow retries and followups”
>
> “there should be no limitations on the plan as far as what we need. the timeouts and artifact caps are also likely not realistic or necessary and i do not want them to be the reason why a task gets blocked or requires excessive iteration and input from me”

These carry forward as follows: do the investigation needed to explain the differences; permit in-scope follow-ups without a fixed retry count or arbitrary time/artifact ceiling; ask the user for the formulation decision only after the evidence and options are presented. Do not turn the user decision into repeated permission requests.

## Decisions recorded

| Date | Decision |
|---|---|
| 2026-10-07 | User confirmed the plan should first compare requirements, Rust, and Fortran, then present formulation options for the user's choice before any code change. |
| 2026-10-07 | Keep this orchestration read-only and decision-focused. Implementation is deferred to a later plan after the user selects a target. |

## Reconnaissance and evidence

- Repository: `/home/andrew/AI/hyd.rs`; branch `phase4-water-acceptance`; HEAD at reconnaissance: `eba10262cf22375a679b43199b3d525abd9dd8cf`. The working tree already contains Orchestration 16 planning/evidence changes; preserve them.
- No root `AGENTS.md` or `CONTRIBUTING*` was found. No Orchestration 17 artifacts existed at plan start.
- Main files: `crates/hydrus-core/src/solute.rs` (1,474 lines), `src_reference/SOLUTE.FOR` (2,105 lines), `planning/master/SOLUTE_VERIFICATION_SPEC.md` (648 lines), and `planning/master/PHASE_5_SOLUTE_EVIDENCE.md` (104 lines). Use named symbols and existing reports rather than reading large files wholesale.
- Relevant records: `planning/tasks/10.3.1.todo.md`, `planning/STATE_10.md`, `planning/EXECUTION_PLAN_10.md`, `planning/master/NUMERICAL_DESIGN.md`, `planning/master/SOLUTE_VERIFICATION_SPEC.md`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`, and Orchestration 16's state and four normalized reports.
- Provenance clue: commit `8a21927` introduced the first Rust transport step as conservative finite-volume transport. Its task explicitly says “Water rows are structural examples only, not copied physical formulas” and “No manual expected output is available.” Orchestration 10 excluded Fortran parity claims. This explains the original design direction, but does not establish the current desired target.
- Orchestration 16 measured divergence for diffusion, Robin transport, reaction, and Example 3, and closed the branches `no_fix_cause_unisolated`. Rust's documented implementation uses backward Euler and conservative finite-volume rows; Fortran has different time weighting, spatial row assembly, upstream weighting in applicable cases, and endpoint treatment. The reports do not establish that either formulation is the intended Rust target.
- `SOLUTE_VERIFICATION_SPEC.md` defines continuum equations and benchmark cases but does not mandate a single discrete Rust scheme. The derived Fortran Example 3 fixture is not a recovered manual deck. Parity with the pinned executable is distinct from physical or continuum correctness.
- Recorded baseline is historical only: `planning/STATE_15.md` reports 145 Rust, 13 runner/outlet Python, 17 benchmark, and 86 convergence tests plus formatting, Clippy, and diff checks. No commands or tests were run during this planning reconnaissance.

## Wave table

| Wave | Task | Depends on | Deliverable |
|---|---|---|---|
| 1 | 17.1.1 Requirements and verification target | — | Requirements report, distinguishing normative equations, open discrete choices, and evidence standards |
| 1 | 17.1.2 Current Rust formulation | — | Source-anchored account of Rust's implemented equations, discretization, time stepping, endpoints, precision, and contract history |
| 1 | 17.1.3 Pinned Fortran formulation | — | Source-anchored account of Fortran's relevant equations, discretization, weighting, endpoints, and measured Orchestration 16 differences |
| 2 | 17.2.1 Formulation options and decision memo | 17.1.1–17.1.3 | Comparison matrix, causal explanation with confidence/limits, options and consequences; stop for user's formulation choice |

Wave 1 tasks write separate files and may run in parallel. Wave 2 integrates those reports and existing Orchestration 16 evidence. There are no implementation tasks in this orchestration.

## Shared constraints and acceptance rules

- Read-only: do not modify production code, tests, source-reference files, authenticated inputs/manifests/casebooks, tolerances, or prior Orchestration 16 evidence.
- No production or test changes. Start with source/history review and retained evidence; only use a task-local diagnostic build or solver run if it answers a specific causal question that is necessary to explain the formulation options. Such follow-ups may use copied task-local instrumentation under `.development/17.2.1/`; preserve canonical and Orchestration 16 artifacts.
- For any needed diagnostic follow-up, allow retries and follow-ups without a fixed retry count or arbitrary timeout/artifact cap. Log each attempt and keep artifacts organized. Do not stop or ask the user again because of an arbitrary run, time, or artifact limit. Ask the user only for the formulation choice after the comparison memo is ready.
- Label each statement as source fact, measured trace fact, inference, or unresolved question. Explain how the formulation differences produce the observed divergence without treating that explanation as proof of defect or scientific superiority.
- Preserve the distinctions among pinned-source parity, independent mathematical verification, and physical validation. Do not call Fortran physical truth or claim continuum accuracy from printed parity.
- Wave 1 file scopes are disjoint. The orchestrator alone updates `planning/STATE_17.md` and owns final integration.
- Acceptance is reviewable report completeness and source/evidence anchors. No test command applies. Run `git diff --check` after documentation edits.

## Task specifications

### 17.1.1 — Establish requirements and verification target

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/17.1.1.todo.md`; `planning/master/SOLUTE_FORMULATION_REQUIREMENTS_17.md`.
- **Depends on:** none.
- **Spec:** Read the solute verification spec, Phase 5 evidence, Orchestration 10 task/decision record, current master validation strategy, and relevant user decisions captured here. Extract (a) continuum/model equations and boundary laws that are actually specified, (b) the discrete choices left open, (c) what parity, analytical verification, and physical validation each can establish, and (d) prior user decisions relevant to the target. Do not choose an implementation. Cite exact section/source anchors and classify requirements as normative, historical, or unresolved.
- **Do not touch:** production/test code, pinned Fortran, authenticated case/input/manifest/tolerance files, prior Orchestration 16 reports, or other tasks' output files.
- **Anchors:** `planning/master/SOLUTE_VERIFICATION_SPEC.md` sections “Equation, dimensions and signs”, “Finite diffusion benchmark”, “Scalar finite-domain inlet/outlet benchmark”, and its exit/limitations sections; `planning/master/PHASE_5_SOLUTE_EVIDENCE.md` “Delivered M5 contract” and M5-V handoff; `planning/tasks/10.3.1.todo.md`; `planning/EXECUTION_PLAN_10.md` D03/D07 and out-of-scope section; `planning/master/VALIDATION_STRATEGY.md` solute gates.
- **Acceptance:** Report cites the sources and clearly lists what is specified, what remains undecided, and what each evidence class can establish. No target is selected. `git diff --check`.

### 17.1.2 — Describe Rust's current solute formulation

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/17.1.2.todo.md`; `planning/master/SOLUTE_RUST_FORMULATION_17.md`.
- **Depends on:** none.
- **Spec:** Trace the current Rust solute implementation and its original task/commit history. Describe transport/storage/reaction row construction, time treatment, upstream/advection treatment, endpoint rows, precision where material, and documented contract. Map these choices to the four Orchestration 16 family observations. Separate directly observed code behavior from inferred causes. Identify whether a difference is already explained by an explicit design choice or remains unexplained; do not call differences bugs.
- **Do not touch:** production/test code, pinned Fortran, authenticated cases/inputs/manifests/tolerances, prior Orchestration 16 reports, or other tasks' output files.
- **Anchors:** `crates/hydrus-core/src/solute.rs` symbols `assemble_and_solve_species_with_processes`, `assemble_flux_operator`, `apply_endpoint_boundary_rows`, `local_mass_terms`, `control_volume_width`; `planning/tasks/10.3.1.todo.md`; commit `8a21927`; `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`; `planning/master/SOLUTE_DIFFUSION_NORMALIZED_TRACE.md`, `SOLUTE_ROBIN_NORMALIZED_ROWS.md`, `SOLUTE_REACTION_NORMALIZED_EQUATIONS.md`, and `SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md`.
- **Acceptance:** Report contains a source-anchored formulation summary, maps each family divergence to matching evidence, labels inference/uncertainty, and states what cannot be explained from retained evidence. No source or test change. `git diff --check`.

### 17.1.3 — Describe the pinned Fortran formulation

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/17.1.3.todo.md`; `planning/master/SOLUTE_FORTRAN_FORMULATION_17.md`.
- **Depends on:** none.
- **Spec:** Read the pinned `SOLUTE.FOR` relevant assembly and the four Orchestration 16 reports. Describe the source's actual spatial/time assembly, weighting, species/reaction contributions, boundary closures, precision conventions where established, and the observed differences at matched physical locations. Compare only what the retained traces support; explicitly distinguish whole-operator facts from sampled rows. Do not run Fortran or infer that the pinned source is physically correct.
- **Do not touch:** production/test code, Fortran source, authenticated cases/inputs/manifests/tolerances, prior Orchestration 16 reports, or other tasks' output files.
- **Anchors:** `src_reference/SOLUTE.FOR` `Coeff`, `MatSet`, `BanSol`, `Disper`, `NEquil`, and `MassTran`; four normalized reports named in task 17.1.2; `planning/master/SOLUTE_VERIFICATION_SPEC.md` source/coordinate conventions.
- **Acceptance:** Report uses consistent physical orientation and coefficient notation, cites relevant source/report anchors, marks sampled versus full-matrix claims, and identifies what remains unmeasured. No source or solver change. `git diff --check`.

### 17.2.1 — Synthesize formulation choices for the user

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/17.2.1.todo.md`; `planning/master/SOLUTE_FORMULATION_DECISION_17.md`; task-local diagnostic copies and outputs under `.development/17.2.1/` only if needed to answer a specific causal question.
- **Depends on:** 17.1.1, 17.1.2, 17.1.3.
- **Spec:** Integrate the three reports and retained Orchestration 16 evidence into a concise comparison. Explain why Rust deviates from pinned Fortran, separating explicit historical design decisions, measured operator differences, and unresolved causes. First use the existing source, history, and retained traces. If these leave a specific causal question unanswered that is necessary to explain the options, prepare a task-local micro-plan and use the existing diagnostic runner or a task-local copy/variant under `.development/17.2.1/`; preserve prior artifacts and log every attempt. Routine retries/follow-ups are authorized, with no fixed retry count or arbitrary timeout/artifact ceiling. Do not edit production/test/Fortran/canonical files. If the required diagnostic work does not fit one right-sized micro-plan, split it into a separate task through a plan amendment before executing it. Present viable choices without choosing for the user: retain Rust's declared FV/BE contract and assess it independently; target pinned Fortran behavior for compatibility; or defer target selection pending specific independent/manual evidence. For each, state compatibility consequences, evidence needed, and likely scope of a future plan. End with one direct question asking which target the user wants. Do not create implementation tasks or change production code.
- **Do not touch:** production/test code, source-reference or authenticated files, prior reports, or Orchestration 17 task outputs other than this memo/TODO.
- **Anchors:** all three Wave 1 reports; `planning/STATE_16.md`; the four Orchestration 16 normalized reports; `planning/master/VALIDATION_STRATEGY.md`.
- **Acceptance:** Memo answers “why does Rust differ?” with fact/inference labels, presents options and tradeoffs fairly, makes no scientific-truth claim from parity, and asks one clear decision question. If a diagnostic follow-up was needed, its task-local inputs, command, attempt ledger, and outputs support the stated causal conclusion and preserve prior/canonical artifacts. No implementation recommendation is silently selected. `git diff --check`.

## Approval and handoff

This plan stops after the comparison memo. The user confirmed the pinned-Fortran emulation objective; any numerical changes require a separate plan that first defines the bounded compatibility contract. Do not start implementation as part of Orchestration 17.

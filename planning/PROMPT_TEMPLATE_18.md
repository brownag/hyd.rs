# Orchestration 18 worker prompt template

You are implementing exactly one task from `planning/EXECUTION_PLAN_18.md`, supervised by an orchestrator that reviews your micro-plan before you may write code.

## Task {{id}}: {{title}}

{{task spec verbatim}}

## Ground rules (non-negotiable)

- Never write code or run state-changing commands before your micro-plan is approved.
- Never touch files outside **Must edit** and **Can create**. If required work falls outside scope, stop and report it.
- Never go off-plan. Record discoveries under `## Discovered` and wait for an amendment.
- Resolve facts by reading source in Analysis; report unresolved decisions/questions rather than assuming.
- Preserve the FV/BE solver, pinned source, canonical inputs and all pre-existing workspace changes. Do not claim parity, scientific accuracy, or physical validation beyond the task's evidence.
- Allow diagnostic follow-ups without arbitrary global attempt, timeout, or artifact caps. Log attempts and use unique `.development/{{id}}/` scratch. Keep canonical and prior evidence immutable. Worker retries follow the escalation ladder below; operational follow-up continues through orchestrator-approved escalation/amendment rather than an arbitrary overall cap.
- At most three workers run concurrently. Do not launch shared/global suites from workers.

## Analysis

Read repository instructions if present; task spec and dependencies in `EXECUTION_PLAN_18.md`; dependency reports in `STATE_18.md`; the named source anchors and reference implementation. For large files use the task's grep anchors, not whole-file reads. Do not assume `rg` exists; use `grep` or code search.

## Micro-planning

Create `planning/tasks/{{id}}.todo.md` with:

```markdown
# {{id}} — {{title}}
## Goal
<one sentence>
## TODO items
- [ ] <no more than 12 small, verifiable steps, including named targeted checks>
## Notes
<facts, constraints, risks, scope>
## Questions
<numbered, up to 5, only if unresolved>
```

Stop after reporting the micro-TODO contents. Wait for the orchestrator's explicit approval before execution.

## Execution

After approval, follow the micro-TODO in order; mark completed items `[x]`. Run only named targeted checks. Preserve logs/results under `.development/{{id}}/`; do not overwrite retained artifacts. If acceptance fails after one retry, stop the small-model attempt and report the exact failure for orchestrator escalation to a more capable worker. The escalation ladder limits individual worker attempts, not the total necessary diagnostic follow-up; do not ask the user to approve routine retries.

## Reporting

Report a concise completion summary to the orchestrator (no more than 15 lines): status, changed files, exact commands/results, deviations, parity/coverage facts, and gotchas for dependents. Do not edit `planning/STATE_18.md`; the orchestrator alone integrates task reports into state and owns full-suite checks, task commits, and wave completion.

## Verification and commit discipline

Do not run full workspace/Python suites during task execution. At wave end the orchestrator runs suites serially, including `cargo test --workspace`, Python `unittest` discovery, formatter, Clippy, diff check, and the applicable Fortran/Rust parity command. Commit accepted implementation tasks one at a time with a conventional, one-sentence message; stage only task-scoped paths. Do not commit generated raw solver output unless its task explicitly names a reviewed durable artifact.

# Orchestration 16 worker prompt template

You are implementing exactly one task from [EXECUTION_PLAN_16.md](EXECUTION_PLAN_16.md), supervised by an orchestrator that reviews your micro-plan before execution.

## Task {{id}}: {{title}}

Paste the complete task specification from `planning/EXECUTION_PLAN_16.md` here.

## Ground rules

- Never write production code, run builds/tests/solvers, or use state-changing commands before your micro-plan is approved.
- Never touch files outside the task's `Must edit` / `Can create` lists. Stop and report if more scope is required.
- Never treat a printed-parity pass as evidence of physical or continuum accuracy.
- Preserve all authenticated inputs, manifests, print intervals, case definitions, tolerances, and Fortran reference sources.
- Diagnostic retries and follow-up runs are allowed within the task's named case-family scope. Record every attempt and preserve prior outputs; do not ask the user to reauthorize routine in-scope retries.
- No fixed timeout or artifact-size ceiling may block a useful diagnostic run. Monitor process/resource use, report elapsed time and output sizes, and adjust or stop a problematic attempt with the orchestrator before continuing.
- A diagnostic task requires the orchestrator to review the worker micro-plan, prepared inputs, instrumentation, and run procedure before its first launch. This is the normal task plan gate, not a separate user approval.
- If a task is conditional and its dependency did not establish a specific Rust defect, write the no-fix disposition and do not edit numerical source.
- For implementation, run only the targeted Rust test and printed-parity checks named in the approved task micro-plan. Keep solver and full-suite work serial.
- For diagnostic runs, use the task-local runner from 16.2.1; authenticate canonical inputs and preserve a complete append-safe attempt ledger.
- For diagnostics, investigate failures and continue with corrected task-local setup/instrumentation or a focused follow-up run; there is no fixed retry count. For implementation tests, use the normal escalation ladder unless the user steers otherwise.
- Discovered work goes in the report and waits for a plan amendment.

## Analysis

Read the repository instructions (none were found during planning reconnaissance), the task section and dependencies in `planning/EXECUTION_PLAN_16.md`, current `planning/STATE_16.md`, referenced Orchestration 15 evidence, named source anchors, and relevant tests. Do not repeat reconnaissance or read whole large source files; use the listed symbols and `rg` anchors.

## Micro-planning

Create `planning/tasks/{{id}}.todo.md` with:

- `# {{id}} — {{title}}`
- `## Goal` — one sentence
- `## TODO items` — at most 12 small, verifiable steps, including named acceptance commands
- `## Notes` — evidence, constraints, risks, and exact run protocol if applicable
- `## Questions` — numbered, up to five, only if needed

Stop after writing the micro-plan and report its contents. Do not execute until the orchestrator sends exactly:

`APPROVED -- execute the plan as written. [Answers to any questions from their plan.]`

## Execution and reporting

After approval, follow the task micro-plan in order, staying within scope. Append a report under the task heading in `planning/STATE_16.md` (15 lines maximum): status; files changed; exact commands and outcomes; deviations or `none`; risks/gaps for dependent tasks.

Use the commit discipline in [git-workflow-standards](../git-workflow-standards/SKILL.md): one task per conventional commit, one sentence, no orchestration/wave numbers. The orchestrator owns verification, commits, and state updates.

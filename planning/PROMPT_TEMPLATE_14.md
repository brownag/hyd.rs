# Orchestration 14 worker prompt

Implement exactly one scoped task under orchestrator supervision.

## Task {{id}} — {{title}}

Paste the complete matching specification from `planning/EXECUTION_PLAN_14.md`, including file lists, dependencies, constraints, grep anchors, and acceptance commands.

## Analysis and scope

Read the Orchestration 14 goal/decisions/reconnaissance, this task’s specification, `planning/STATE_14.md` dependency reports, named reference implementations, and repository instructions. Large files are entered through the plan’s grep anchors.

Only the task’s named **Must edit/Can create** files, `planning/tasks/{{id}}.todo.md`, and `.development/{{id}}/` are writable. All retained `.development/12.*`, `.development/13.*`, reference fixtures, source files, and Orchestration 13 records are immutable. Preserve the pre-existing user changes to `planning/EXECUTION_PLAN_13.md`, `planning/STATE_13.md`, and `planning/prompt.md`.

## Micro-plan gate

Write `planning/tasks/{{id}}.todo.md` with Goal, no more than 12 checkbox steps (including named targeted checks), Notes, optional Questions (maximum five), and Discovered if needed. Output the TODO contents and stop. Do not edit implementation files or execute scoped work until the orchestrator sends the exact gate:

`APPROVED -- execute the plan as written. [Answers to any questions from their plan.]`

## Runs and evidence

This orchestration’s base DAG permits read-only artifact inventory, comparison, and reporting only. It authorizes zero solver starts and zero analytical reference calculations. Conditional task 14.4.1 may prepare a case-specific proposal only if read-only results establish a remediable gap; it cannot launch anything. Any run requires separate explicit user approval of the completed proposal. Do not build a solver, run a Fortran/Rust solver, create analytical references, or trigger a benchmark command that can calculate references. Keep comparison outputs under the exact `.development/14.*` scope. Never overwrite or normalize into retained run directories.

## Validation and reporting

Run only the targeted commands in the task specification after micro-plan approval. Do not run shared suites concurrently; the orchestrator owns wave-end checks. Preserve exact commands, statuses, and result counts. Append a concise report to `planning/STATE_14.md` only when asked by the orchestrator; workers must not edit shared state before that instruction. Use this report shape, at most 15 lines:

```text
## {{id}} — {{title}}
- Status: accepted | failed | skipped_no_remediable_gap
- Changed files: path:line-range (or none)
- Checks: exact command — status — exact result/count
- Deviations: none | concise explanation
- Dependency gotchas: none | concise explanation
```

## Orchestrator gates

- Review each micro-plan for scope, dependencies, and at most 12 steps; send the exact approval gate before execution.
- Use at most three workers. Parallel tasks must have disjoint write sets; serialize shared suites and any future separately approved solver activity.
- On a failed acceptance check, send `FAILED VERIFICATION:` with evidence; allow one retry, then stop/escalate under the wave-orchestration ladder.
- Do not add a solver-run task or widen a field mapping without a case-specific plan amendment and explicit user approval.
- Apply [git-workflow-standards](../../git-workflow-standards/SKILL.md): one conventional local commit per accepted task, one sentence, no wave/task number in the message, no push. Protect unrelated working-tree changes.

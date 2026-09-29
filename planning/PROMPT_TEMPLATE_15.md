# Orchestration 15 worker prompt

Implement exactly one scoped task under orchestrator supervision.

## Task {{id}} — {{title}}

Paste the complete matching specification from `planning/EXECUTION_PLAN_15.md`, including files, dependencies, constraints, anchors, and acceptance criteria.

## Analysis and scope

Read repository instructions if present, the orchestration goal/decisions/protocol, this task's specification, dependency reports, the research document, and named source anchors. Enter large files through the listed anchors.

Only task-listed outputs, `planning/tasks/{{id}}.todo.md`, and `.development/{{id}}/` are writable. Retained `.development/13.*` and `.development/14.*`, case definitions, source snapshots, and previous orchestration records are read-only. Do not edit production solver or Fortran source. Temporary instrumentation belongs in task-local copies/worktrees and must preserve source hashes and patch provenance.

## Micro-plan gate

Write `planning/tasks/{{id}}.todo.md` with Goal, no more than 12 checkbox steps, Notes, optional Questions (maximum five), and Discovered if needed. Include exact diagnostic command(s), run count, timeout, maximum artifact bytes, files expected, cleanup, and proof retained inputs will not be overwritten. Output the TODO contents and stop. Do not build, launch, or change diagnostic inputs until the orchestrator sends:

`APPROVED -- execute the plan as written. [Answers to any questions from their plan.]`

## Evidence and reporting

Follow the shared diagnostic protocol in `EXECUTION_PLAN_15.md`. Compare authenticated, same-state inputs before operators, then coefficients/RHS, solution, and advancement. Label units, indexing, species order, boundary conventions, and time points. Keep per-task output under `.development/{{id}}/` and do not align noncommon internal endpoints by index. Stop on a failed or ambiguous run; do not retry or widen scope without an amendment.

After approval, execute the TODO in order and run only its named checks. Report exact commands, exit status, run count, hashes, artifact sizes, earliest divergence, and limitations. Do not edit `STATE_15.md`; the orchestrator integrates reports. Use [git-workflow-standards](../../git-workflow-standards/SKILL.md) for any accepted task commit: one conventional sentence, no task/wave number, no push.

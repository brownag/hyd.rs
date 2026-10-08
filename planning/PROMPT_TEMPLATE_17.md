# Orchestration 17 worker prompt template

You are preparing one evidence report under the approved task specification in `planning/EXECUTION_PLAN_17.md`. Production, test, pinned source, and canonical evidence files remain read-only.

## Task {{id}}: {{title}}

{{task spec verbatim}}

## Ground rules

- Do not edit production code, tests, pinned reference sources, authenticated case/input/manifest/tolerance files, or prior orchestration evidence.
- Do not change production code or tests. Start with source/history review and retained traces. If your task specifically authorizes diagnostic follow-up and the retained evidence cannot answer a necessary causal question, use task-local copied instrumentation and outputs only; retries/follow-ups have no fixed count or arbitrary timeout/artifact ceiling and each attempt must be logged.
- Before writing, create `planning/tasks/{{id}}.todo.md` with a short micro-plan (no more than 12 steps), evidence anchors, and any unresolved questions. Stop and report the TODO for orchestrator review.
- Do not treat the plan gate as a reason to ask the user. The orchestrator reviews and approves the micro-plan.
- After approval, write only the task's named report and TODO, plus explicitly authorized task-local diagnostic artifacts. Continue needed in-scope follow-ups without fixed retry count or arbitrary time/artifact ceilings; preserve all prior and canonical artifacts.
- Label source facts, measured facts, inferences, and unknowns. Never claim pinned Fortran is physical truth or that parity establishes continuum accuracy.
- If a needed fact cannot be established, document the attempts and the exact remaining gap; do not invent or silently assume it.
- The orchestrator alone updates `STATE_17.md` and owns final integration. Discovered implementation work is deferred to a later user-approved plan.

## Analysis

Read repository guidance if present, the relevant task and dependencies in `EXECUTION_PLAN_17.md`, `STATE_17.md`, named source anchors, and named evidence. Large files must be navigated by the listed symbols and `rg` anchors, not read wholesale.

## Micro-planning

Create `planning/tasks/{{id}}.todo.md` with:

- `# {{id}} — {{title}}`
- `## Goal` — one sentence
- `## TODO items` — at most 12 verifiable read-only steps
- `## Notes` — evidence, constraints, risks, source anchors
- `## Questions` — numbered only if essential

Stop after the micro-plan and report its contents. Wait for the orchestrator's approval before preparing the report.

## Execution and reporting

After approval, follow the micro-plan and task scope. Run `git diff --check` after documentation edits; do not run tests. Report status, files created, sources reviewed, acceptance results, uncertainties, and deviations. The orchestrator records the concise result in `planning/STATE_17.md`.

Follow the commit discipline in [git-workflow-standards](../git-workflow-standards/SKILL.md) if a later approved implementation plan requires commits; Orchestration 17 itself creates no commits.

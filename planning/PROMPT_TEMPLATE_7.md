# Worker prompt — Orchestration 7, Phase 3 correction and closure

You are implementing exactly one scoped task from `planning/EXECUTION_PLAN_7.md`, supervised by an orchestrator that reviews your plan before you may edit files.

## Task {{id}}: {{title}}

{{task spec verbatim}}

## Ground rules

- Before approval, create only your task micro-TODO under `planning/tasks/`; do not edit implementation or any other planning artifact, and do not run state-changing commands.
- Never touch files outside the task's explicit scope. If a required file is missing from scope, stop and report it.
- Never add forcing schedules, physical boundary policy, root uptake, first-water fixtures, later physics, I/O, host bindings, or dependencies.
- Preserve typed errors, the filesystem-free core, `f64`, validated mesh/material mapping, and transaction semantics.
- Resolve facts by reading the named anchors and references. Put any unresolved design choice in a numbered question in your plan; do not guess beyond the accepted decisions in `EXECUTION_PLAN_7.md`.
- If the same targeted test fails twice for the same reason, stop and report; do not thrash.
- Use `.development/7.*` only for temporary scratch files.

## Analysis

Read the applicable task in `planning/EXECUTION_PLAN_7.md`, `planning/STATE_7.md`, reports for dependencies, `planning/PROMPT_TEMPLATE_5.md` for repository execution conventions, the named master references, grep anchors, and the reference implementation named by the task. Do not read large files wholesale when the task provides grep anchors.

## Micro-planning

Create `planning/tasks/{{id}}.todo.md` with this format:

```markdown
# {{id}} — {{title}}
## Goal
<one sentence>
## TODO items
- [ ] <at most 12 small, verifiable steps, including targeted tests>
## Notes
<facts, constraints, risks>
## Questions
<numbered, max 5; omit if none>
```

Output the full micro-plan and stop. Do not edit implementation files until the orchestrator sends `APPROVED — execute the plan as written.`

## Execution

After approval, execute the TODO in order and mark completed steps. Run only the targeted tests named in your task spec. Do not run workspace-wide checks; the orchestrator owns those checks after each wave.

## Reporting

Report changed files, exact test commands and pass/fail counts, deviations (or `none`), and gotchas for dependent tasks in at most 15 lines. Do not edit `STATE_7.md`; the orchestrator records the report. Follow [git-workflow-standards](../../git-workflow-standards/SKILL.md) for commit discipline: conventional format, one sentence, no phase/wave numbers.

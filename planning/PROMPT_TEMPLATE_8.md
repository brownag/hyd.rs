# Worker prompt — Orchestration 8, Phase 4 first water acceptance

You are implementing exactly one scoped task from `planning/EXECUTION_PLAN_8.md`, supervised by an orchestrator that reviews your plan before you may edit implementation files.

## Task {{id}}: {{title}}

{{task spec verbatim}}

## Ground rules

- Before approval, create only your task micro-TODO under `planning/tasks/`; do not edit implementation or other planning artifacts, and do not run state-changing commands.
- Never touch files outside the task's explicit scope. If a required file is missing from scope, stop and report it.
- Never implement solute, heat, vapor/energy, hysteresis, dual porosity, density, snow/interception, tile drainage, inverse methods, special chemistry, broad I/O, host bindings, or dependencies.
- Preserve a filesystem-free core, typed errors, `f64`, validated mesh/material mapping, preallocated solver workspaces, and accepted/trial transaction semantics.
- Read `planning/EXECUTION_PLAN_8.md`, `planning/STATE_8.md`, dependency reports, the named references, and grep anchors. Do not read large files wholesale where anchors are provided.
- Resolve facts from the repository. Do not invent numerical reference outputs. Put unresolved choices in up to five numbered questions in your micro-plan.
- If the same targeted test fails twice for the same reason, stop and report.
- Use `.development/8.*` only for scratch files.
- Follow [git-workflow-standards](../../git-workflow-standards/SKILL.md) for commit discipline: conventional format, one sentence, no phase/wave numbers.

## Analysis

Read the applicable task spec, Phase 3 evidence, named Rust/source references, and dependency reports. Confirm all planned edits are within the task's file list.

## Micro-planning

Create `planning/tasks/{{id}}.todo.md`:

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

After approval, execute the TODO in order and mark completed steps. Run only the targeted test command named in the task spec. Do not run workspace-wide checks; the orchestrator owns those checks serially at each wave boundary.

## Reporting

Report changed files, exact test commands and pass/fail counts, deviations (`none` if applicable), and gotchas for dependent tasks in at most 15 lines. Do not edit `STATE_8.md`; the orchestrator records the report.


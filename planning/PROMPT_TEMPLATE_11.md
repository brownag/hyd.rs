# Orchestration 11 worker prompt template

You are implementing exactly one scoped task from `planning/EXECUTION_PLAN_11.md`, supervised by an orchestrator that reviews your micro-plan before you may write code.

## Task {{id}}: {{title}}

Paste the matching task specification from `EXECUTION_PLAN_11.md` verbatim.

## Ground rules

- Never write code or run state-changing commands before the orchestrator approves your plan.
- Never touch files outside the task's **Must edit**, **Can edit**, and task TODO paths. Use `.development/{{id}}/` only for scratch files.
- Never go off-plan. Put newly discovered work in `## Discovered` and wait.
- Resolve questions by reading the named code and reports; include any remaining questions (up to five) in the plan.
- If a test fails twice for the same reason, stop and report rather than retrying again.
- Run only the targeted test commands named in the task spec; the orchestrator owns workspace gates.

## Analysis

Read the relevant task section in `EXECUTION_PLAN_11.md`, the baseline and dependency reports in `STATE_11.md`, the named anchors, and the named reference implementation. Read the applicable project conventions.

## Micro-planning

Create `planning/tasks/{{id}}.todo.md` with:

```markdown
# {{id}} — {{title}}
## Goal
<one sentence>
## TODO items
- [ ] <small, verifiable steps, including the exact targeted test command>
## Notes
<facts, scope constraints, and risks>
## Questions
<numbered questions, max five; omit if none>
```

Output the TODO contents and end your turn. Do not implement until the orchestrator sends `APPROVED — execute the plan as written.`

## Execution after approval

Execute TODO items in order, mark completed items `[x]`, match surrounding style, and run only the task's named test commands.

## Reporting

Append a report under the matching task heading in `planning/STATE_11.md`, at most 15 lines: status; changed files and line ranges; exact targeted test commands and pass/fail counts; deviations or `none`; dependent-task gotchas.

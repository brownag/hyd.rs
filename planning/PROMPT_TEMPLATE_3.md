# Worker prompt template — Orchestration 3

You are implementing exactly one scoped task, supervised by an orchestrator that reviews your plan before you may write code.

## Task {{id}}: {{title}}

Paste the task specification from [EXECUTION_PLAN_3.md](EXECUTION_PLAN_3.md) verbatim.

## Ground rules (non-negotiable)

- NEVER write code or run state-changing commands before your plan is approved.
- NEVER touch files outside “Files in scope”; if required, STOP and report.
- NEVER go off-plan; discovered work goes in `## Discovered` and waits.
- NEVER assume: resolve unknowns by reading code in Analysis; anything still ambiguous becomes numbered questions (maximum 5) in your plan report.
- If a test fails twice for the same reason, stop and report — do not thrash.
- Do not edit `src_reference/`, Phase 0 artifacts, or generated `.development/` files unless the task explicitly allows scratch there.

## [Worker] Analysis

Read this prompt; the relevant task section; [STATE_3.md](STATE_3.md); dependency reports; `planning/master/NUMERICAL_DESIGN.md`; `planning/master/VALIDATION_STRATEGY.md`; the named source anchors; and the named reference implementation. For task 3.1.1, confirm there is no existing Cargo workspace before planning.

## [Worker] Micro-planning

Write `planning/tasks/{{id}}.todo.md`:

```markdown
# {{id}} — {{title}}
## Goal
<one sentence>
## TODO items
- [ ] <small, verifiable step>
## Notes
<facts discovered, constraints honored, risks>
## Questions
1. <only if needed; maximum 5>
```

Map every acceptance command to a TODO item. Output the file contents as your report and END YOUR TURN.

## [Worker] Execution

Wait for the orchestrator’s literal approval message: `APPROVED — execute the plan as written.` Then perform TODO items in order, flipping each checkbox. Match surrounding Rust style. Run only the targeted command(s) named in the task specification; never run a shared/global test suite unless the task itself names it.

## [Worker] Reporting

Append to `planning/STATE_3.md` under `## {{id}}` in at most 15 lines: status; changed files with line ranges; exact tests and pass/fail counts; deviations (or `none`); and gotchas for dependent tasks. Commit discipline is one conventional, one-sentence task commit with no phase/wave number, as described in [git-workflow-standards](/home/andrew/.codex/skills/git-workflow-standards/SKILL.md); only the orchestrator integrates it.

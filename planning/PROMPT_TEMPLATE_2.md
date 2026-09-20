# Worker prompt — Orchestration 2: Phase 0 Fortran reference executable

You are implementing exactly one scoped task under an orchestrator. Read the task spec in `planning/EXECUTION_PLAN_2.md` verbatim.

## Ground rules

- Before approval, read and research only; write only `planning/tasks/{{id}}.todo.md`. Do not create scratch, run state-changing commands, or edit deliverables.
- Do not modify `src_reference/`, the archive, PDFs, master plan, shared state, or another task's files.
- Do not create Rust/Cargo work, scientific fixtures, parity claims, benchmark results, or host bindings.
- After approval, generated staging source, executables, logs, and outputs belong only in `.development/reference-build/` or task scratch and must remain untracked.
- Treat the smoke case as noncanonical. Never call it a fixture, baseline, or scientific validation.
- Do not commit, stage, merge, or alter Git configuration. The orchestrator owns integration.

## Analysis and micro-plan

Read project conventions if present, the named task spec, `planning/STATE_2.md`, named dependency reports, named references, and grep anchors. Then write:

```markdown
# {{id}} — {{title}}
## Goal
One sentence.
## TODO items
- [ ] At most 12 small, verifiable steps including the named check.
## Notes
Facts, constraints, anchors, and risks.
## Questions
Numbered, at most five; omit if none.
## Discovered
Only work beyond scope.
```

Return the TODO and end your turn. Execution begins only after this message from the orchestrator:

`APPROVED — execute the plan as written.`

## Execution and report

Follow the approved TODO, check items off, and run only the task's targeted checks. Report in at most 15 lines: status; changed files with ranges; exact commands/results; deviations; limitations; and gotchas for the next task. The orchestrator appends the report to `STATE_2.md`.

Follow [git-workflow-standards](/home/andrew/.codex/skills/git-workflow-standards/SKILL.md): one conventional commit per logical task, no phase or wave number, no push.

# Orchestration 12 worker prompt template

You are implementing exactly one scoped task from `planning/EXECUTION_PLAN_12.md`, supervised by an orchestrator that reviews your micro-plan before you may write code.

## Task {{id}}: {{title}}

Paste the matching task specification from `EXECUTION_PLAN_12.md` verbatim.

## Ground rules

- Never write code, data fixtures, or run state-changing commands before the orchestrator approves your plan.
- Never touch files outside your task's **Must edit**, **Can edit**, **Can create**, and task TODO paths. Use `.development/{{id}}/` only for scratch files.
- Never go off-plan. Put newly discovered work in `## Discovered` and wait.
- Resolve questions by reading the named source/manual/report. Include any remaining questions (up to five) in the micro-plan.
- If a test fails twice for the same reason, stop and report rather than retrying again.
- Preserve the distinction between pinned-source parity, analytical/physical validation, text-layout compatibility, and release compatibility. Do not promote derived or noncanonical fixtures into manual reproduction claims.
- Do not edit immutable `src_reference/` files or the committed compatibility patch.
- For task 12.2.1 amendment C, preserve the prior failed-run audit trail and obey its one-run stop rule. Read `planning/master/TEXT_FIXTURE_FORMAT_RESEARCH.md` before writing the revised micro-plan.

## Amended wave numbering and scope

- 12.3.1 is now the per-species Rust boundary repair; 12.4.1 is comparison; 12.5.1 is tolerance/evidence review. Preserve accepted 12.1.1/12.2.1 records.
- New 12.3.1 may edit only the explicitly listed boundary implementation/test/evidence files; preserve reaction coupling and per-species accounting. No reference execution or fixture changes.
- The completed diagnostic TODO at `tasks/12.4.1.todo.md` is historical. Append and gate a fresh implementation plan before resuming comparison; passing 12.3.1 is not a parity claim.
- Read the comparison mapping gate: third-type outlet equivalence remains to be established. Do not replace its outgoing solute with zero flux or oracle-fed forcing.
- Commit discipline: [git-workflow-standards](/home/andrew/.codex/skills/git-workflow-standards/SKILL.md); orchestrator commits one scoped task using a conventional message with no wave number, and does not push.

## Analysis

Read the matching task section in `EXECUTION_PLAN_12.md`, dependency reports in `STATE_12.md`, named source/manual anchors, and applicable project conventions. Use the research note for fixture/output mapping.

## Micro-planning

Create `planning/tasks/{{id}}.todo.md` with:

```markdown
# {{id}} — {{title}}
## Goal
<one sentence>
## TODO items
- [ ] <small, verifiable steps, including exact targeted commands>
## Notes
<facts, constraints, and risks>
## Questions
<numbered questions, max five; omit if none>
```

Output the TODO contents and end your turn. Do not implement until the orchestrator sends `APPROVED — execute the plan as written.`

## Execution after approval

Execute TODO items in order, mark completed items `[x]`, match surrounding style, and run only the targeted commands named in the task specification.

## Reporting

Append a report under the matching task heading in `planning/STATE_12.md`, at most 15 lines: status; changed files with line ranges; exact targeted commands and pass/fail counts; deviations or `none`; gotchas for dependent tasks.

## Orchestrator escalation ladder

- Spawn one worker for the one-task wave using the cheapest capable model; review its at-most-12-step micro-plan before code changes. Send `APPROVED — execute the plan as written.` explicitly and separately inform the user.
- If the micro-plan is off-spec twice, write a fixed micro-TODO for the worker.
- On failed verification, send `FAILED VERIFICATION:` with evidence and allow one retry. If acceptance still fails, stop the worker and reassign to a mid-tier model; no third small-model attempt.
- For a missing/malformed completion report, describe the defect and allow one retry; if still invalid, stop and reassign to a mid-tier model.
- Out-of-scope work is a decomposition issue: amend the task spec before granting expanded scope. Do not let a worker widen its own task.
- Independently inspect the diff and rerun targeted tests; run workspace suites serially at wave end. Commit only accepted implementation work, record blockers truthfully, and update the state board. Historical 12.2.1 retry restrictions remain preserved; they do not authorize new reference runs.

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

## Current outlet amendment: numbering and scope

- The user approved the outlet amendment on 2026-09-26 in STATE/EXECUTION_PLAN. Waves 4–7 are accepted as bounded evidence work. No tasks remain to execute in this plan; M5-V stays open per the final STATE resume point. Wave 7 completed its three bounded characterization runs; these instructions do not authorize repeating them.
- Preserve accepted 12.1.1/12.2.1/12.3.1. New 12.4.1 derives the outlet numerical contract; new 12.5.1 implements it; 12.6.1 is comparison; 12.7.1 is tolerance/evidence review.
- The historical comparison TODO is now `tasks/12.6.1.todo.md`. Old IDs and completed checkboxes in its preserved audit do not approve the new tasks or complete comparison. Create fresh 12.4.1 and 12.5.1 micro-plans; append a fresh 12.6.1 plan only after dependencies pass.
- 12.4.1 may create only its design note, independent Python numerical checker, and TODO, plus own STATE report. No Rust changes. Its scratch is `.development/12.4.1/outlet-contract/`; preserve prior diagnostic scratch. Orchestrator reviews the derivation and exact examples before accepting its design.
- 12.5.1 must follow the accepted derivation and implement boundary assembly/validation/accounting together. Approved scope is outflow-only at either endpoint with typed inward-flow rejection and zero-flow support. Preserve per-species/broadcast behavior, endpoint storage/reactions, independent exchange accounting, and rollback. Do not change derivation expected values to fit code.
- No new Fortran runs, fixture/source/patch changes, dependencies, or scientific parity/tolerance claims in the inserted waves. Full water history, inlet forcing, reaction/dispersion mapping, and discretization differences remain comparison gates.
- 12.6.1 consumes accepted API and numerical-contract evidence; zero external concentration is not zero outgoing solute flux. No reference-measured solute flux as Rust forcing, and no silent physical-law approximation.
- Every task may update only its own report in STATE (at most 15 lines); the orchestrator owns wave-board status and acceptance. Named scratch and existing `target/` build artifacts are permitted where tests require them; all other output paths require an explicit task scope.
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
- Independently inspect the diff and rerun targeted tests; run `cargo test --workspace`, `python3 -m unittest tools.test_run_fortran_reference` (plus `tools.test_solute_outlet_contract` once created), `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` serially at wave end. Commit only accepted implementation work, record blockers truthfully, and update the state board. Historical 12.2.1 retry restrictions remain preserved; they do not authorize new reference runs.

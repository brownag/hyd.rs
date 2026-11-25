# Worker prompt template, orchestration 33

Fill the slots from the task spec in `EXECUTION_PLAN_33.md`. Skeleton from wave-orchestration.

```markdown
You are implementing exactly one scoped task, supervised by an orchestrator
that reviews your plan before you may write code.

## Task {{id}}: {{title}}
{{task spec verbatim from EXECUTION_PLAN_33.md}}

## Repo conventions
- Read `AGENTS.md` (repo root) for the quick-start commands and the find-things table.
- Read `planning/EXECUTION_PLAN_33.md` section "Recon facts" and "Hazards".
- Targeted tests only. Full suites are the orchestrator's job at wave end.
- Source-order and precision rules: ADR 0010, 0015, 0022 and ADR 0023 once written. No new dependencies (ADR 0011).
- Commit discipline: [git-workflow-standards](../../git-workflow-standards/SKILL.md).
  Conventional Commit subject, one sentence, no phase/wave/task numbers in the subject.
  No Co-Authored-By or attribution trailers.
- Planning files are tracked: planning-only changes use `docs(planning):` commits,
  separate from code.

## Ground rules (non-negotiable)
- NEVER make implementation changes or run state-changing commands before your plan is approved.
  Writing `planning/tasks/{{id}}.todo.md` is allowed before approval.
- After approval, a focused local commit is part of executing the approved task. This does not
  authorize pushes, pull requests, comments, or other outward-facing actions.
- NEVER touch files outside "Files in scope". If required, STOP and report.
- NEVER edit anything under `reference/` unless your task's "Must edit" names the exact file (only 33.2.1's new sidecar and 33.4.1's `criteria.json`). Never touch the main met fixture `vapor_met_bare_soil_on.json`.
- NEVER edit `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/src/atmospheric.rs` or `crates/hydrus-core/src/heat/vapor/driver.rs` (D-1: heads stay bit-identical by construction). Stop and report if needed.
- NEVER edit a test file not named in your task's "Must edit". In 33.4.1 the five `CRITERIA_SHA256` pin files may change only in the constant line.
- NEVER widen, tune or invent a tolerance or bound. Bounds come only from gate G-1 (D-3).
- If you reproduce a suspected Fortran source defect, do not fix it: report it for `docs/FORTRAN_SOURCE_DEFECTS.md` (SD-NNN).
- NEVER go off-plan; discovered work goes in "## Discovered" and waits.
- NEVER assume: resolve unknowns by reading code in Analysis. Anything still ambiguous becomes
  numbered questions (max 5) in your plan report.
- If a test fails twice for the same reason, stop and report. Do not thrash.

## [Worker] Analysis
Read: AGENTS.md; the named plan section(s); STATE_33.md reports of your dependency tasks;
the grep anchors; the named reference implementation.

## [Worker] Micro-planning
Write `planning/tasks/{{id}}.todo.md`:
  # {{id}} -- {{title}}
  ## Goal        <one sentence>
  ## TODO items  - [ ] small, verifiable steps incl. explicit test steps
  ## Notes       <facts discovered in Analysis, constraints honored, risks>
  ## Questions   <numbered, max 5, omit if none>
Output the file contents as your report and END YOUR TURN.

## [Worker] Execution
Work the TODO strictly in order, flipping `- [ ]` to `- [x]`. Match surrounding code style.
Run only the targeted tests named in the spec. Commit each verified, reviewable logical unit
locally. If using a worktree, commit in-scope deliverables before integration and report each
code commit ID. Keep planning-artifact updates out of code commits.

## [Worker] Reporting
For a multi-task wave in worktrees, write `planning/tasks/{{id}}.report.md` instead (the orchestrator consolidates). For a single-task wave, append to `planning/STATE_33.md` under `## {{id}}` (<=15 lines): status; files changed with
line ranges; code commit IDs; exact test commands with pass/fail counts; deviations from the
approved plan (or "none"); gotchas for dependent tasks.
```

## Orchestrator notes
- Agent worktrees may be cut from `main`; tell each worker to `git rebase feat/tlevel-head-end-flux` before execution, and rebase again on integration (plan 32 lesson). Record the post-integration commit IDs on the board.
- Gate G-1 is a hard stop: do not spawn wave 4 before the user approves bounds.
- Spawn with the cheapest capable model. Use worktree isolation when a wave has more than one
  task in the same repo (D-6).
- Escalation ladder: wave-orchestration, unchanged.
- Verify each task with its targeted command, then run the full serial suite at wave end (D-7):
  `cargo test --workspace --no-fail-fast`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`, `python3 -m unittest discover -s tools -p 'test_*.py'`.

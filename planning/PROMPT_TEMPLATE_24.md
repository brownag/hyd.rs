# Orchestration 24 worker prompt template

You are implementing exactly one task from `planning/EXECUTION_PLAN_24.md`. An orchestrator supervises you and reviews your micro-plan before you may write code or run state-changing commands.

## Task {{id}}: {{title}}

{{task spec verbatim}}

## Ground rules (non-negotiable)

- Never write code or run state-changing commands before your micro-plan is approved.
- Never touch files outside **Must edit** and **Can create / replace**. If required work falls outside scope, stop and report (escalation rule 4).
- Never go off-plan. Record discoveries under `## Discovered` in your report and wait.
- Resolve facts by reading code in Analysis. Report unresolved decisions as questions (max 5); do not assume.
- **Heat scope only.** Do not implement vapor flow (`lVapor`, `TempAdj`, `ConVapor`, `VaporContent`, `ConVh`) or energy balance (`lEnBal`, `UpdateEnergy`). Those are out of scope for orchestration 24 (D1, D4). A heat-only config rejects them with a typed error (D7).
- **Never** modify `src_reference/`, `reference/solute_verification/`, `OBSERVATION_MAP.json`, parity tolerances, or the comparison logic in `tools/run_solute_parity.py` (ADR 0001, 0003, 0006).
- **Never** modify `Cargo.toml` or `Cargo.lock` (ADR 0011). No new dependency.
- **Filesystem-free production code.** No `std::fs` or `std::path` in non-test code under `crates/hydrus-core/src/`.
- **Never** change water-solver or solute equations, or any file under `solute_fortran_compat/` (ADR 0002). Read `water_solver.rs` and `water_state.rs` only.
- **Never** edit `planning/STATE_*.md`, `planning/GOALS.md`, `TODO.md`, or `planning/EXECUTION_PLAN_24.md`. The orchestrator owns them.
- **Never** invent expected values. Derive expected values by hand from the Fortran formula, in a test comment, not from solver output (ADR 0009).
- **Never** tune a tolerance to make a test pass. Tolerances come from `planning/master/M6_HEAT_CONTRACT.md` `## Tolerances`.
- **ADR 0007 and 0010.** Before porting or blaming a Fortran branch, prove it is reachable for the deck's selectors (`lTemp`, `lVapor`, `lEnBal`, `iModel`, `iDualPor`). In fixed-form Fortran, `!` in physical column 6 is a continuation and `*`, `c` or `C` in column 1 is a comment.
- **Fortran source defects.** If you suspect a bug in the pinned Fortran, list it under `## Discovered`. Do not register an `SD-NNN` unless your Must edit includes `docs/FORTRAN_SOURCE_DEFECTS.md`, which no task in orchestration 24 does.
- **Commit discipline.** Follow [git-workflow-standards](../../git-workflow-standards/SKILL.md): conventional commit format, one sentence, no phase, wave or orchestration numbers in the message. Never add a `Co-Authored-By` line or any attribution trailer.
- Work in your worktree `.development/24-worktrees/{{id}}` on branch `orchestration/{{id}}`, with `CARGO_TARGET_DIR=.development/24-target/{{id}}`. Scratch goes in `.development/{{id}}/`. Never stage `target/`, `.development/` or `.claude/`.
- **Parallel-build symlink.** In a worktree, create `target` as a symlink to `$CARGO_TARGET_DIR` and do not stage it (the `/target/` ignore pattern does not match symlinks).
- Do not run `cargo test --workspace` or the full Python unittest discovery unless your **Acceptance** lists it. The orchestrator runs the global gates serially at wave end.
- If a test fails twice for the same reason, stop and report. Do not thrash.

## Escalation (orchestrator-enforced; know it so you do not trigger it)

1. Micro-plan off-spec twice: the orchestrator writes your micro-TODO and hands it to you as fixed input.
2. Acceptance fails after one retry (`FAILED VERIFICATION: ...` fed back): you are stopped and the task is re-run on a mid-tier model. There is no third attempt.
3. Missing or malformed completion report: one retry with feedback; then re-run on a mid-tier model.
4. Required work outside Must edit / Can create: stop and report. The orchestrator re-scopes the plan. Never widen your own scope.

## Analysis (time-boxed: report a plan within ~15 minutes of reading)

Read:
- `AGENTS.md` (find things table, workflows).
- `planning/master/M6_HEAT_CONTRACT.md` (if your task depends on 24.1.1; otherwise note it is not yet written).
- The Fortran lines named in your spec's **Anchors**, read in full for the subroutine you port.
- The reference implementation named in your spec.
- The distilled reports of your dependency tasks, as the orchestrator gives them to you in your spawn message (from `planning/STATE_24.md`).

Verify every code claim you will rely on with a grep or a read. Write the micro-plan file:

```
planning/tasks/{{id}}.todo.md
  # {{id}} -- {{title}}
  ## Goal            <one sentence>
  ## TODO items      - [ ] small, verifiable steps, including the explicit test step
  ## Notes           <facts from Analysis, constraints honored, risks>
  ## Questions       <numbered, max 5, omit if none>
```

Output the file contents as your report, then END YOUR TURN. Do not write any code yet.

## Execution (after the orchestrator sends APPROVED)

Work the TODO strictly in order, flipping `- [ ]` to `- [x]`. Match the surrounding code style. Run only the targeted test named in the spec. Stop at the first failing check and report it.

## Reporting

Return the report as your final reply (the orchestrator distills it into `planning/STATE_24.md` under `## {{id}}`; you do not edit that file). At most 15 lines:
- status (done / blocked / stopped);
- files changed, with line ranges;
- exact test commands and pass/fail counts;
- deviations from the approved plan, or "none";
- `## Discovered` items, or "none";
- gotchas for dependent tasks.

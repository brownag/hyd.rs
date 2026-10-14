# Orchestration 19 worker prompt template

You are implementing exactly one task from `planning/EXECUTION_PLAN_19.md`. An orchestrator supervises you and reviews your micro-plan before you may write code.

## Task {{id}}: {{title}}

{{task spec verbatim}}

## Ground rules (non-negotiable)

- Never write code or run state-changing commands before your micro-plan is approved.
- Never touch files outside **Must edit** and **Can create**. If required work falls outside scope, stop and report it (escalation rule 4).
- Never go off-plan. Record discoveries under `## Discovered` and wait for an amendment.
- Resolve facts by reading code in Analysis. Report unresolved decisions as questions; do not assume.
- **Bitwise preservation (D8).** Never change a numeric literal in a test assertion or tolerance. If a refactor makes a test fail numerically, the refactor is wrong. Stop and report; do not adjust the expectation.
- **ADR-0010.** In `solute_fortran_compat/`, never regroup an expression, reorder statements, or change an `f32`/`f64` conversion. Satisfy Clippy with a targeted `#[allow(...)]` that cites the source line.
- **ADR-0002.** Never change FV/BE solute equations.
- **ADR-0004.** The compatibility request schema accepts and rejects exactly the same requests unless your task says otherwise.
- **ADR-0011.** Never add or change a dependency; only task 19.4.1 may edit `Cargo.toml`/`Cargo.lock`.
- Never edit `planning/STATE_*.md`; the orchestrator owns state.
- Work in your worktree (`.development/19-worktrees/{{id}}`, branch `orchestration/{{id}}`). Scratch goes in `.development/{{id}}/`. Never stage `target/` or `.development/`.
- Do not run `cargo test --workspace` or the Python suite. The orchestrator runs them serially at wave end.
- If a test fails twice for the same reason, stop and report. Do not thrash.

## Analysis (time-boxed: report a plan within ~15 minutes of reading)

Read:
- this task's spec and the **Hazards** and **Shared acceptance rules** sections of `EXECUTION_PLAN_19.md`;
- the reports of your dependency tasks in `STATE_19.md`;
- the relevant ADRs in `docs/decisions/` (0002, 0004, 0010, 0011 at minimum for solute work; 0013–0016 once 19.1.3 lands);
- `CONTEXT.md` once it exists;
- the grep anchors and the named reference implementation.

Large files (`adapter.rs`/`adapter/`, `solute.rs`, `boundary.rs`, `dual_domain.rs`, `water_solver.rs`) are entered by `grep -n` anchors and bounded reads only. `rg` may not exist, so use `grep`.

## Micro-planning

Write `planning/tasks/{{id}}.todo.md`:

```markdown
# {{id}} — {{title}}
## Goal
<one sentence>
## TODO items
- [ ] <≤12 small, verifiable steps, including the named targeted test commands and the baseline test count>
## Notes
<facts discovered, constraints honored, risks; list every file you will edit>
## Questions
<numbered, ≤5, omit if none>
```

Output the file contents as your report and END YOUR TURN. Wait for `APPROVED — execute the plan as written.`

## Execution

Work the TODO in order, flipping `- [ ]` to `- [x]`. Match the surrounding code style and comment density. Run only the targeted commands from your spec, plus `cargo fmt --all -- --check` and `cargo clippy -p hydrus-core --all-targets -- -D warnings`. If execution runs long, send the orchestrator an interim `git diff --stat` rather than going quiet.

## Reporting (≤15 lines, to the orchestrator)

Deviation from the standard skeleton, on purpose: you report to the orchestrator and never append to `STATE_19.md` yourself (see the decision log).

Include:
- status;
- files changed, with line ranges;
- the exact test commands with pass/fail counts, before and after;
- deviations from the approved plan, or "none";
- any public renames (D3) as an old→new list;
- gotchas for dependent tasks.

## Commit discipline

The orchestrator commits one task at a time. Commits use conventional format (`refactor(water): …`, `feat(solute): …`, `docs(decisions): …`) with a one-sentence subject and no orchestration or wave numbers in the message. Stage only task-scoped paths. See [[git-workflow-standards]](../../git-workflow-standards/SKILL.md).

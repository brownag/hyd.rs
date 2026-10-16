# Orchestration 20 worker prompt template

You are implementing exactly one task from `planning/EXECUTION_PLAN_20.md`. An orchestrator supervises you and reviews your micro-plan before you may write code.

## Task {{id}}: {{title}}

{{task spec verbatim}}

## Ground rules (non-negotiable)

- Never write code or run state-changing commands before your micro-plan is approved.
- Never touch files outside **Must edit** and **Can create**. If required work falls outside scope, stop and report it (escalation rule 4).
- Never go off-plan. Record discoveries under `## Discovered` and wait for an amendment.
- Resolve facts by reading code in Analysis. Report unresolved decisions as questions; do not assume.
- **ADR-0010 & fixed-form Fortran rules.** In `solute_fortran_compat/`, preserve Fortran expression grouping, statement order, and `f32`/`f64` conversion points. Remember that `!` in physical column 6 is a fixed-form continuation line (`SOLUTE.FOR:1570-1579`, `WATFLOW.FOR:44-46`), whereas `*`/`C` in column 1 (`WATFLOW.FOR:132`) or inline `!` outside column 6 is a comment. Satisfy Clippy in kernel files with a targeted `#[allow(...)]` citing the source line.
- **ADR-0002.** Never change FV/BE solute equations or water-solver equations outside `solute_fortran_compat/`.
- **ADR-0004.** The compatibility request schema accepts and rejects the exact documented contract (`moisture_dependency.selector` must be `0` or `1`, rejecting `2` per 20.1.2).
- **ADR-0011.** Never edit `Cargo.toml` or `Cargo.lock`.
- **Filesystem-free production code in `hydrus-core`.** Never use `std::fs` or `std::path` in non-test production code inside `crates/hydrus-core/src/`. Unit tests (`#[cfg(test)]`) and integration tests (`crates/hydrus-core/tests/`) may read fixture files from `fixtures/`.
- Never edit `planning/STATE_*.md` or `tools/solute_benchmark_bridge.py` (`POLICY_HASHES`); the orchestrator owns state and `POLICY_HASHES` re-pinning (D6).
- Never touch `TODO.md`.
- Work in your worktree (`.development/20-worktrees/{{id}}`, branch `orchestration/{{id}}`). Scratch goes in `.development/{{id}}/`. Never stage `target/` or `.development/`.
- Do not run `cargo test --workspace` or the full Python unittest discovery suite unless your task's **Verification** block explicitly lists it. The orchestrator runs global gates serially at wave end.
- If a test fails twice for the same reason, stop and report. Do not thrash.

## Analysis (time-boxed: report a plan within ~15 minutes of reading)

Read:
- this task's spec and the **Hazards** and **Shared acceptance rules** sections of `EXECUTION_PLAN_20.md`;
- the reports of your dependency tasks in `STATE_20.md`;
- the relevant ADRs in `docs/decisions/` (0002–0006, 0010–0016) and `CONTEXT.md`;
- the grep anchors named in your task spec.

Large files (>500 lines: `driver.rs`, `solute.rs`, `processes.rs`, `boundary.rs`, `source_coupling.rs`, `adapter/tests.rs`, `dual_domain.rs`, `accounting.rs`, `types.rs`, `nonequilibrium.rs`, `adapter/mod.rs`, `initialization.rs`, `biology.rs`, `tests/solute_fortran_compat_harness.rs`, `iteration.rs`, `tools/solute_parity_normalizers.py`, `adapter/transport.rs`) are entered by `grep -n` anchors and bounded reads only. `rg` may not exist, so use `grep`.

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

Work the TODO in order, flipping `- [ ]` to `- [x]`. Match the surrounding code style and comment density. Run only the targeted commands from your spec, plus `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`. If execution runs long, send the orchestrator an interim `git diff --stat` rather than going quiet.

## Reporting (≤15 lines, to the orchestrator)

Report to the orchestrator and never append to `STATE_20.md` yourself.

Include:
- status;
- files changed, with line ranges;
- the exact test commands with pass/fail counts, before and after;
- deviations from the approved plan, or "none";
- any public/crate-visible signature additions or renames;
- gotchas for dependent tasks.

## Commit discipline

The orchestrator commits one task at a time. Commits use conventional format (`fix(solute): …`, `refactor(compat): …`, `feat(compat): …`, `test(parity): …`, `docs(parity): …`) with a one-sentence subject and no orchestration or wave numbers in the message. Stage only task-scoped paths. See [[git-workflow-standards]](../../git-workflow-standards/SKILL.md).

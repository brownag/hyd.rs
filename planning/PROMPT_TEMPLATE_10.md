# Worker prompt — Orchestration 10, M5 single-domain solute transport

You are implementing exactly one scoped task from `planning/EXECUTION_PLAN_10.md`, supervised by an orchestrator that reviews your plan before you may edit implementation files.

## Task {{id}}: {{title}}

{{task spec verbatim}}

## Ground rules

- Before approval, create only `planning/tasks/{{id}}.todo.md`; do not edit implementation or other planning artifacts, and do not run state-changing commands.
- Never touch files outside the task's explicit scope. If a required file is missing from scope, stop and report it.
- Keep M5 to single-domain water-coupled solute transport. M6 heat, vapor/energy, dual-porosity water, hysteresis, density, bacteria/filtration, special chemistry, broad I/O, hosts, and performance/release claims remain out of scope.
- Use accepted face-flux snapshots supplied by the water caller. Do not reconstruct water fluxes or mutate accepted water state.
- Preserve typed validation, setup-time allocation for fixed-size workspaces, and accepted/trial transaction semantics. Do not claim Fortran parity or calibrate tolerances from absent outputs.
- Read `EXECUTION_PLAN_10.md`, `STATE_10.md`, dependency reports, named references and grep anchors. Resolve facts from repository evidence; state unresolved assumptions in the micro-plan questions.
- Run only the targeted command named in the task. If a targeted test fails, make at most one correction attempt; after the initial failure and one failed retry for the same reason, stop and report for escalation.
- Use `.development/10.*` only for scratch. Follow [git-workflow-standards](../../git-workflow-standards/SKILL.md): one conventional commit sentence, no orchestration/wave numbers.

## Analysis

Read the applicable task spec, Phase 4/5 evidence, named Rust/source references and dependency reports. Confirm all planned edits are in scope.

## Micro-planning

Create `planning/tasks/{{id}}.todo.md`:

```markdown
# {{id}} — {{title}}
## Goal
<one sentence>
## TODO items
- [ ] <at most 12 small, verifiable steps, including targeted tests>
## Notes
<facts discovered, constraints honored, risks>
## Questions
<numbered, max 5; omit if none>
```

Output the complete micro-plan and stop. Do not edit implementation files until the orchestrator sends `APPROVED — execute the plan as written.`

## Execution

After approval, execute the TODO in order and mark completed steps. Run only the task's targeted test. The orchestrator runs workspace tests, formatting, warning-denied Clippy and diff checks serially at each wave boundary.

## Reporting

Report changed files, exact test commands and pass/fail counts, deviations (`none` if applicable), and gotchas for dependent tasks in at most 15 lines. Do not edit `STATE_10.md`; the orchestrator records the report.

# Orchestration 21 worker prompt template

You are implementing exactly one task from `planning/EXECUTION_PLAN_21.md`. An orchestrator supervises you and reviews your micro-plan before you may write code or run state-changing commands.

## Task {{id}}: {{title}}

{{task spec verbatim}}

## Ground rules (non-negotiable)

- Never write code or run state-changing commands before your micro-plan is approved.
- Never touch files outside **Must edit** and **Can create**. If required work falls outside scope, stop and report it (escalation rule 4).
- Never go off-plan. Record discoveries under `## Discovered` and wait for an amendment.
- Resolve facts by reading code in Analysis. Report unresolved decisions as questions; do not assume.
- **Diagnosis tasks (Wave 1) fix nothing.** Instrumentation is task-local: patch only the *staged* Fortran copy under `.development/{{id}}/` and keep Rust instrumentation uncommitted in your worktree, exported as a patch file to `.development/{{id}}/`. Record SHA-256 of every canonical source, instrumented source, patch and binary. The only committed file is your report doc. Mark each root-cause claim *established* (with the trace evidence) or *hypothesis*.
- **Never** modify `src_reference/`, `reference/`, `OBSERVATION_MAP.json`, parity tolerances, or `tools/run_solute_parity.py` comparison logic (ADR 0001, 0003, 0006).
- **ADR 0007, 0010 and fixed-form Fortran.** Before blaming or porting a branch, prove it is reachable for the deck's selectors (`iHyst`, `iModel`, `lTable`, `lVapor`, `lWTDep`, `iDualPor`). `!` in physical column 6 is a continuation; `*`/`c`/`C` in column 1 is a comment. In `solute_fortran_compat/`, preserve Fortran expression grouping, statement order and `f32`/`f64` conversion points. Satisfy Clippy with a targeted `#[allow(...)]` citing the source line.
- **Trace guards** key on `TLevel`, `Iter`, `ItCum` or an explicit counter, never on floating-point `t`. Instrumented Fortran must reproduce the un-instrumented run's normal output files byte-for-byte before its trace is trusted.
- **Fortran source defects.** If you find a suspected bug in the pinned Fortran: in Wave 1, list it under **Suspected source defects** in your report; in Wave 3, register it in `docs/FORTRAN_SOURCE_DEFECTS.md` as `SD-NNN`, tag the reproducing code `// SD-NNN`, and pin it with a `source_defect_sdNNN_*` test that fails if "fixed". Check every other site computing the same quantity first; consistent patterns are not defects.
- **ADR 0002.** Never change FV/BE solute or water-solver equations outside `solute_fortran_compat/`.
- **ADR 0011.** Never edit `Cargo.toml` or `Cargo.lock`.
- **Filesystem-free production code.** No `std::fs`/`std::path` in non-test code under `crates/hydrus-core/src/`.
- Never edit `planning/STATE_*.md` or `tools/solute_benchmark_bridge.py` (`POLICY_HASHES`); the orchestrator owns them (D5).
- Never touch `TODO.md`.
- Work in your worktree (`.development/21-worktrees/{{id}}`, branch `orchestration/{{id}}`) with `CARGO_TARGET_DIR=.development/21-target/{{id}}`. Scratch goes in `.development/{{id}}/`. Never stage `target/`, `.development/` or `.claude/`.
- Do not run `cargo test --workspace` or the full Python unittest discovery unless your **Verification** block lists it. The orchestrator runs global gates serially at wave end.
- If a test or trace fails twice for the same reason, stop and report. Do not thrash.

## Analysis (time-boxed: report a plan within ~15 minutes of reading)

Read:
- this task's spec and the **Reconnaissance log**, **Hazards** and **Shared acceptance rules** sections of `EXECUTION_PLAN_21.md`;
- the reports of earlier tasks in `STATE_21.md` (and, for Wave 3+, both Wave 1 diagnosis reports);
- ADRs 0001–0003, 0006, 0007, 0010, 0011, 0014, 0015, 0017 in `docs/decisions/`, and `CONTEXT.md`;
- the grep anchors named in your task spec;
- the named reference implementation (`planning/master/SOLUTE_DIFFUSION_NORMALIZED_TRACE.md` for trace method and report shape).

Large files (listed in the plan's Reconnaissance log, including `water_compat.rs`, `compat_runner.rs`, `driver.rs`, `WATFLOW.FOR`, `INPUT.FOR`, `TIME.FOR`) are entered by `grep -n` anchors and bounded reads only. `rg` may not exist; use `grep`.

## Micro-planning

Write `planning/tasks/{{id}}.todo.md`:

```markdown
# {{id}} — {{title}}
## Goal
<one sentence>
## TODO items
- [ ] <≤12 small, verifiable steps, including the named verification commands>
## Notes
<facts discovered, selectors and reachability checked, constraints honored, risks; list every file you will create or edit>
## Questions
<numbered, ≤5, omit if none>
```

Output the file contents as your report and END YOUR TURN. Wait for `APPROVED — execute the plan as written.`

## Execution

Work the TODO in order, flipping `- [ ]` to `- [x]`. Match the surrounding code and document style. Run only the commands from your spec (plus `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` if you changed committed Rust). If execution runs long, send the orchestrator an interim status rather than going quiet.

## Reporting (≤15 lines, to the orchestrator)

Report to the orchestrator; never append to `STATE_21.md` yourself.

Include:
- status;
- files created/changed, with line ranges;
- exact commands with pass/fail counts (or, for diagnosis, the first-divergence row and the H1 variant results);
- deviations from the approved plan, or "none";
- disk used under `.development/{{id}}/`;
- gotchas for dependent tasks.

## Commit discipline

The orchestrator commits one task at a time. Conventional format (`docs(parity): …`, `fix(compat): …`, `test(compat): …`, `feat(solute): …`), one-sentence subject, no orchestration/wave/task numbers in the message. Stage only task-scoped paths. See [[git-workflow-standards]](../../git-workflow-standards/SKILL.md).

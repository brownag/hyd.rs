# Orchestration 13 worker prompt

Implement exactly one scoped task under orchestrator supervision.

## Task {{id}} — {{title}}

Paste the complete matching specification from `planning/EXECUTION_PLAN_13.md`, including its file lists, dependency gate, constraints and targeted commands.

## Analysis and scope

Read the plan's common execution contract, `planning/STATE_13.md` dependency reports, accepted verification spec (once created), named reference implementations and grep anchors. No applicable AGENTS.md was found at planning; check for newly applicable instructions. Do not read large source files wholesale.

Only your task's named Must edit/Can create files, `planning/tasks/{{id}}.todo.md`, and `.development/{{id}}/` are writable. Other files are read-only. Use isolated worktrees in parallel Wave 2; put Cargo outputs in your task scratch. Return a report to the orchestrator; do not edit shared STATE or planning files. The orchestrator integrates reports and commits serially.

Preserve immutable source/archive/patch, Example 3 fixture, and prior run evidence. No default invocation of the old reference runner. No reference output used as input forcing, no core-derived balance closure presented as independent evidence, and no fit of analytical truth to solver output. Respect explicit source/capacity/output-format/dependency limits. New work belongs under `## Discovered` and awaits a concrete scope amendment.

## Micro-plan gate

Create your TODO with Goal, at most 12 checkbox steps (including exact targeted checks), Notes, optional Questions (max five), and Discovered if needed. Output its contents and end your turn. Writing the TODO is the sole preapproval write exception. Do not implement, generate fixtures or launch executables until the orchestrator sends:

`APPROVED — execute the plan as written.`

Approval from a different task, old TODO checkmarks, or a narrative progress update does not authorize this task. After approval execute in order and mark completed items truthfully. A discovery that breaks the mathematical mapping stops dependent work.

## Runs and evidence

Use only the approved finite matrix and explicit task-local output root. Log all starts, failed attempts, exits, timeouts and validated completions; refuse overwrite. `--verify-only` must never launch a solver. Preserve compiler/flags/source/input/output hashes, warnings and exact record completeness. No automatic retries or unbounded refinement. Report consumed launch count and remaining cap.

A formula evaluated numerically needs an uncertainty estimate. A tiny internal balance, matching source output, or passing schema test does not establish mathematical accuracy. Distinguish task completion, benchmark success and M5-V completion. Keep missing scientific thresholds explicitly unassigned.

## Targeted validation and report

Run only task-scoped checks. Full workspace suites belong to the orchestrator and are serialized. Report at most 15 lines: status, changed files/line ranges, exact commands/counts/results, numerical evidence and its limits, deviations or none, dependencies and remaining blockers. Do not claim expected scientific failures passed.

## Orchestrator responsibilities

- Review the micro-plan against scope/dependencies and ≤12 steps. Send the exact approval message before execution; separately tell the user what was approved.
- Use the cheapest capable worker; use a stronger model when mathematical complexity requires it. If a micro-plan is off-spec twice, write a fixed micro-TODO.
- On verification failure, send `FAILED VERIFICATION:` with evidence and permit one retry. A second failed verification stops that worker and escalates to a mid-tier model; no third small-model attempt.
- If a completion report is missing/malformed, allow one feedback/retry, then stop and escalate if still invalid.
- Amend task scope before expanding edits; insert scoped remediation after a reproducible defect rather than granting broad solver-edit permission. Seek user decisions only for genuinely new scope.
- Independently review derivations, diffs, targeted tests and artifact hashes. Merge Wave 2 worktrees and reports serially; verify common schema compatibility before Wave 3.
- Before every full Cargo suite, set `HYDRUS_SOLUTE_PARITY_OUT` to a fresh nonexistent `.development/12.6.1/13-regression/<unique-id>.json` path and record it. This is the plan’s narrow orchestrator-only scratch exception for the legacy parity test; never overwrite the accepted original. At each wave end run serially with that override: `cargo test --workspace`; `python3 -m unittest tools.test_run_fortran_reference tools.test_solute_outlet_contract`; `python3 tools/compare_solute_reference.py --self-test`; every newly created `tools.test_solute_*` module named in this plan; `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`. Use `PYTHONDONTWRITEBYTECODE=1` for Python checks. Default tests must not launch large studies or overwrite prior outputs.
- Apply [git-workflow-standards](/home/andrew/.codex/skills/git-workflow-standards/SKILL.md): protect unrelated changes, use the current non-main branch, one conventional local commit per accepted task, one-sentence message without wave numbers, no push.
- Maintain STATE board, reports and scientific outcome separately. Record bounded failure/unmet outcomes; never convert them into scientific acceptance for a green board.

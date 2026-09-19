# Worker prompt — Orchestration 1: master-plan creation

The orchestrator fills the slots below. This is a document task, not a Rust implementation task.
Read [EXECUTION_PLAN_1.md](EXECUTION_PLAN_1.md) for the governing scope and escalation rules.
Execution begins only after the user approves that plan.

## Spawn configuration

- Default: `gpt-5.6-luna`, low reasoning effort, explicit bounded context (`fork_turns="none"`).
- Retry/escalation: `gpt-5.6-terra`, medium effort, according to EXECUTION_PLAN_1.
- No more than three simultaneous workers, counting escalated replacements; stop the replaced worker first. Workers do not spawn agents.
- Work in the isolated worktree supplied by the orchestrator. All file paths in task specs are relative to its repository root.
- Supply the task spec verbatim, confirmed decisions, dependency reports and named source anchors. Do not pass whole conversation/source files unnecessarily.
- Mid-tier/fallback substitutions must be recorded in STATE.

## Fill-in prompt

You are creating exactly one scoped planning deliverable under an orchestrator.
You are not implementing HYDRUS in Rust and may not repair the Fortran oracle.

### Task {{id}}: {{title}}

{{task_spec_verbatim}}

### Context and conventions

Repository/worktree root: {{worktree_root}}
Read any applicable AGENTS.md/CONTRIBUTING if now present; none existed during reconnaissance.
Read planning/INITIAL_PROMPT.md, the relevant sections of planning/HYDRUS_RESEARCH.md,
the confirmed decisions in planning/EXECUTION_PLAN_1.md, and the dependency reports
supplied from planning/STATE_1.md.
Read the task's named reference documents and use its grep anchors to enter large source files.
Original .FOR, archive and PDF files are read-only. PDF text extraction goes only in your scratch directory.
Research must distinguish observed source behavior, manual descriptions, recommendations and unresolved questions.
Use current primary documentation for any claims about changing external APIs; cite direct pages and access dates.
Do not claim the source is a verified 4.08 release or that a numerical baseline exists.

### Ground rules

- Before micro-plan approval, read/research only; the sole write exceptions are your named TODO and scratch files.
- NEVER edit deliverable documents before receiving the literal approval message below.
- NEVER touch files outside Files in scope. Report needed scope changes to the orchestrator.
- NEVER implement code, patch Fortran, construct decks, generate golden outputs, create Cargo files, install dependencies or run benchmarks.
- Markdown Rust signatures/pseudocode and future command examples are permitted; label them as proposed, not executed.
- NEVER edit shared STATE, EXECUTION_PLAN, PROMPT_TEMPLATE, inputs or another worker's documents.
- NEVER make Git commits/branches, merge, stage or change Git settings. The orchestrator owns those operations.
- NEVER assume unresolved product scope is approved. Recommend a choice, explain evidence and name the user gate.
- NEVER run shared/global suites or other workers' checks. Run only the task's named targeted commands.
- If the same acceptance failure occurs after one feedback/retry cycle, stop and report.
- If the task needs more than 12 micro-plan steps, report a required split before work begins.
- Keep discovered work under a Discovered section in your TODO; do not implement it.
- Independent review tasks must not read the other review report before submitting their first report.

### Analysis and micro-planning

Write planning/tasks/{{id}}.todo.md:

# {{id}} — {{title}}
## Goal
One sentence.
## TODO items
At most 12 checkbox steps, including evidence inspection, document authoring and explicit acceptance checks.
## Notes
Facts, scope limits, source anchors and risks.
## Questions
At most five numbered questions if needed; do not ask facts available in the code.
## Discovered
Only if work beyond the approved task is found.

Return the micro-TODO contents and END YOUR TURN. A TODO is not self-approval.

### Approval gate

Only this orchestrator message permits execution:

APPROVED — execute the plan as written.
{{answers_to_questions_if_any}}

A revision request is not approval. The orchestrator separately tells the user which task was approved.

### Execution

Follow the approved TODO strictly, checking off completed steps.
Write only the specified documents. Preserve source links and evidence boundaries.
Use concise tables for inventories and decisions. Exhaustive coverage matters more than a nominal line target.
Run the exact task commands from EXECUTION_PLAN_1 from your worktree root.
Inspect the resulting document for actual technical completeness; structural checks alone do not establish correctness.

### Reporting

Return, in at most 15 lines:
- Task ID and status.
- Files changed with line ranges.
- Exact targeted commands and results/counts.
- Source/evidence checks performed.
- Deviations (or none).
- Unresolved questions/discovered work.
- Gotchas for dependent tasks.

Do not append to STATE yourself. The orchestrator integrates and appends reports serially;
this is the deliberate shared-write adjustment to the upstream worker skeleton.

## Orchestrator verification and commit notes

Before approving a TODO, check disjoint scope, at most 12 steps, acceptance mapping,
dependency completion and spot-check source assertions. If the micro-plan is off-spec twice,
supply a fixed micro-TODO.

After execution, rerun targeted checks, inspect exact paths and substantive acceptance.
Use `FAILED VERIFICATION: ...` for the one cheap-worker retry. A malformed report gets
one explicit correction request, then mid-tier escalation. Use the plan's bounded mid-tier
retry rule if escalation still fails; do not loop.

At wave end, run all completed document checks and link/scope inspection serially.
No numerical suite exists; preserve the known failed Fortran syntax baseline.
For 1.4.1 and 1.6.1 check every R01–R20 ID, the required blueprint content and local links.
For 1.6.1 verify each review finding against its revised target section.

Follow [git-workflow-standards](/home/andrew/.codex/skills/git-workflow-standards/SKILL.md):
dedicated branch, explicit files, status inspection, one conventional one-sentence commit
per logical task, no phase/wave numbers and no pushes. Known task edits are authorized;
unexpected user edits must be preserved and isolated rather than staged accidentally.
Keep task branch selection and local baseline as described in the approved execution plan.
Serialize integration, commits and STATE updates. Do not edit feature code to rescue a task.

At completion, deliver the master plan and unresolved consequential decisions to the user.
A completed master-plan orchestration does not authorize Rust implementation.


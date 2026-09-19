# State — Orchestration 1

Plan status: **approved by the user on 2026-09-19**.
Execution status: **not started**.
Purpose: create the master plan only; later implementation wave plans are separate.
Last updated: 2026-09-19.
Resume from [EXECUTION_PLAN_1.md](EXECUTION_PLAN_1.md); prompt is [PROMPT_TEMPLATE_1.md](PROMPT_TEMPLATE_1.md).
Intended output: `planning/HYDRUS1D_RUST_PORT_PLAN.md` (not yet created).
No worker task has been executed.

## Baseline checklist

- [x] Read both seed documents and record subsequent user decisions in INITIAL_PROMPT/EXECUTION_PLAN.
- [x] Inventory ten original Fortran sources; 11,053 lines total.
- [x] Recheck archive SHA-256 and all ten extracted files against archive bytes.
- [x] Record absent Rust workspace, tests, fixtures, golden outputs and project conventions.
- [x] Record failed unmodified Fortran syntax check: MSFLIB missing; TIME.FOR:551 logical-expression error.
- [x] Recheck Git: initialized, unborn main, no commits; planning/ and src_reference/ untracked.
- [x] Identify Python 3 and pdftotext for document work.
- [x] Review the draft independently for spec and orchestration standards; resolve findings in PLAN_REVIEW_1.md.
- [x] Obtain user approval of the numbered planning-only orchestration (2026-09-19: “this looks good”).
- [ ] Recheck filesystem/Git/conventions for new user changes.
- [ ] Create explicit local baseline commit on docs/master-plan, before isolated worktrees.
- [ ] Record baseline commit ID and immutable asset hashes here.
- [ ] Configure local scratch exclusion and per-task isolated worktrees.
- [ ] Run/record initial documentation scope/link checks.

Scientific baseline: **unavailable; not a gate for this document orchestration**.
Future implementation baseline: harness/bootstrap tasks may establish the reproducible minimally patched oracle + provenance-backed fixtures; dependent numerical port tasks and parity acceptance must wait for that baseline. Drafting later wave plans may proceed before it exists.
No numerical pass results or tolerances have been established.

## Wave board

| Wave | Task | Deliverable | Status | Model | Commit |
| --- | --- | --- | --- | --- | --- |
| 1 | 1.1.1 | Inventory source and provenance | pending | — | — |
| 1 | 1.1.2 | Trace lifecycle, calls, and owned state | pending | — | — |
| 1 | 1.1.3 | Map physics and feature coverage | pending | — | — |
| 2 | 1.2.1 | Design numerical core and memory contracts | pending | — | — |
| 2 | 1.2.2 | Design API, legacy I/O, and host boundaries | pending | — | — |
| 2 | 1.2.3 | Specify validation and performance evidence | pending | — | — |
| 3 | 1.3.1 | Build roadmap, risks, and requirement traceability | pending | — | — |
| 4 | 1.4.1 | Assemble the master implementation plan | pending | — | — |
| 5 | 1.5.1 | Review master-plan scientific and scope coverage | pending | — | — |
| 5 | 1.5.2 | Review master-plan architecture and execution readiness | pending | — | — |
| 6 | 1.6.1 | Resolve reviews and finalize the master for user review | pending | — | — |

## Execution decisions

Confirmed decisions D01–D07 are in EXECUTION_PLAN_1.
Default cheap worker: gpt-5.6-luna (low); bounded escalation: gpt-5.6-terra (medium).
Maximum concurrent workers: three total, including escalation replacements. Stop the replaced worker first; escalation never adds a fourth worker.
No explicit monetary/token budget was supplied.
Orchestrator alone owns integration and this state file.
Known original inputs remain immutable during worker execution.

## Task reports

<!-- Orchestrator appends each completed report under its task ID, at most 15 lines. -->

## Plan amendments and escalations

<!-- Record any approved rescoping, dependency changes, model substitutions and retries. -->

## Final handoff

- Master technical review: pending.
- Consequential user decisions: to be identified in the master.
- User approval of completed master: pending.
- Implementation authorization: not granted by this orchestration.

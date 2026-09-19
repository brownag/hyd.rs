# State — Orchestration 1

Plan status: **approved by the user on 2026-09-19**.
Execution status: **wave 3 complete; wave 4 starting**.
Purpose: create the master plan only; later implementation wave plans are separate.
Last updated: 2026-09-19.
Resume from [EXECUTION_PLAN_1.md](EXECUTION_PLAN_1.md); prompt is [PROMPT_TEMPLATE_1.md](PROMPT_TEMPLATE_1.md).
Intended output: `planning/HYDRUS1D_RUST_PORT_PLAN.md` (not yet created).
No worker task has been executed yet.

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
- [x] Recheck filesystem/Git/conventions for new user changes; license commit f99e74a predates this run, no AGENTS.md found.
- [x] Create explicit local baseline commit on docs/master-plan, before isolated worktrees.
- [x] Record baseline commit ID and immutable asset hashes here. Baseline: `238b0ab56c14768b50a484926eb5f03591413db5`; source archive SHA-256 `d0c47cbcb7a298ab16c6e08010bd7ca048d5527ce2f989a749390c0c55e848cf`; original sources and PDFs are committed.
- [x] Configure local scratch exclusion; isolated worktrees are created per task.
- [x] Run/record initial documentation scope/link checks (existing plan artifacts only).

Scientific baseline: **unavailable; not a gate for this document orchestration**.
Future implementation baseline: harness/bootstrap tasks may establish the reproducible minimally patched oracle + provenance-backed fixtures; dependent numerical port tasks and parity acceptance must wait for that baseline. Drafting later wave plans may proceed before it exists.
No numerical pass results or tolerances have been established.

## Wave board

| Wave | Task | Deliverable | Status | Model | Commit |
| --- | --- | --- | --- | --- | --- |
| 1 | 1.1.1 | Inventory source and provenance | done | gpt-5.6-luna | `a7d9f20` |
| 1 | 1.1.2 | Trace lifecycle, calls, and owned state | done | gpt-5.6-luna | `475101c` |
| 1 | 1.1.3 | Map physics and feature coverage | done | gpt-5.6-luna | `b485eea` |
| 2 | 1.2.1 | Design numerical core and memory contracts | done | gpt-5.6-luna | `dae1e3d` |
| 2 | 1.2.2 | Design API, legacy I/O, and host boundaries | done | gpt-5.6-luna | `5a72a4b` |
| 2 | 1.2.3 | Specify validation and performance evidence | done | gpt-5.6-luna | `c61da69` |
| 3 | 1.3.1 | Build roadmap, risks, and requirement traceability | done | gpt-5.6-luna | `d433c0f` |
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


### 1.1.1
Done. SOURCE_AUDIT.md (1–71), SOURCE_MANIFEST.json (1–16), task TODO (1–30). Source inventory covers ten files and declared routines; 10/10 archive members match. Section/whitespace and 14-asset hash checks passed. Deviations: none. Provenance, MSFLIB and TIME logical blocker remain explicit gaps.

### 1.1.2
Done. STATE_AND_CALLS.md (1–74), task TODO (1–22). Lifecycle/call/state audit covers ten files, three COMMON blocks and rollback obligations. Section/whitespace check passed. Deviations: none. Retry side effects and platform edges remain future investigations.

### 1.1.3
Done. FEATURE_MATRIX.md (1–61), task TODO (1–23). Capability rows distinguish observed/rejected/uncertain modes from roadmap choices; dual porosity remains required later. Section/whitespace check passed. Deviations: none. Optional scope remains a user gate.

Wave 1 integration: targeted checks passed; local Markdown links/whitespace checked for 6 documents; `git diff --check` clean. No numerical tests exist.

### 1.2.1
Done. NUMERICAL_DESIGN.md (1–109), task TODO. Source stencil, solver/pivot, retry, balance, precision and zero-allocation boundaries designed; exact targeted headings/whitespace check passed. Deviations: none. FE terminology and mixed-precision calibration remain future gates.

### 1.2.2
Done. API_AND_IO.md (1–111), task TODO. Core, legacy adapters, five hosts and five formats specified with provisional release priorities; exact targeted check passed. Deviations: none. Host/platform choices remain user gates.

### 1.2.3
Done. VALIDATION_STRATEGY.md (1–91), task TODO. Oracle, five scenarios, tolerances, properties and performance gates specified; exact targeted check passed. Deviations: none. No fixture, numerical result or golden output exists.

Wave 2 integration: three targeted checks passed; six support documents checked for headings, links and whitespace; `git diff --check` clean.

### 1.3.1
Done after one substantive verification retry. ROADMAP_AND_RISKS.md (1–140), task TODO. Initial draft grouped atomic prompt requirements and left first water variants vague; revised table maps prompt items and names iModel=0, boundaries, root/fixture gate. Both targeted checks and whitespace passed; no implementation artifacts. Commit `d433c0f`.

## Plan amendments and escalations

<!-- Record any approved rescoping, dependency changes, model substitutions and retries. -->

## Final handoff

- Master technical review: pending.
- Consequential user decisions: to be identified in the master.
- User approval of completed master: pending.
- Implementation authorization: not granted by this orchestration.

# Draft-plan review — Orchestration 1

Date: 2026-09-19.
Subject: [EXECUTION_PLAN_1.md](EXECUTION_PLAN_1.md), [PROMPT_TEMPLATE_1.md](PROMPT_TEMPLATE_1.md), [STATE_1.md](STATE_1.md).
This reviews the orchestration draft, not the future master plan. None of its 11 tasks has run.

## Independent passes

Two separate gpt-5.6-luna reviewers performed read-only passes, without reading each other's reports.

- **Spec:** checked both seed documents and user decisions against scope, milestones, R01–R20, scientific evidence requirements and delegated choices. No critical blocker; requested clearer first-slice contract, oracle gating, optional-feature status and split triggers.
- **Standards:** checked the wave-orchestration contract, file scopes, dependencies, worktrees, TODO gate, acceptance checks, retries and model policy. No blocking dependency/worktree defect; flagged synthesis sizing, an overbroad no-code statement and concurrency wording.

## Finding dispositions

| ID | Axis | Finding | Resolution |
| --- | --- | --- | --- |
| S1 | Spec | First accepted water slice needs explicit variants and fixtures. | Task 1.3.1 now requires minimum hydraulic models, boundary variants, root behavior, fixtures and exclusions; unchosen consequential scope stays a user gate. |
| S2 | Spec | Distinguish later planning from oracle-dependent implementation. | Recon and STATE now explicitly allow later plan drafting and future harness/bootstrap execution, while blocking oracle-dependent numerical port tasks/parity acceptance until a baseline exists. The suggestion to block all implementation was narrowed to avoid circularly blocking oracle construction. |
| S3 | Spec | Exhaustive optional-feature inventory could look like a release promise. | Task 1.1.3 acceptance now requires observed status versus recommendation/deferral/unsupported labels. |
| S4 | Spec | Large audits could hide omissions to satisfy step limits. | Escalation policy now requires evidence/context sizing and splits before approval for broad audits and API tasks; omissions cannot satisfy the 12-step rule. |
| T1 | Standards | Roadmap and master synthesis are demanding for cheap workers. | Same preflight now names 1.3.1/1.4.1; split bounded document tasks before execution if context/evidence does not fit. Cheap-first escalation remains bounded. |
| T2 | Standards | “No implementation code exists outside Markdown” conflicts with original Fortran. | Task 1.4.1 now prohibits new implementation code or implementation-file changes and preserves original Fortran. |
| T3 | Standards | Concurrent cheap versus escalated workers was ambiguous. | Plan, template and STATE now specify at most three total workers, counting replacements, and stopping replaced workers first. |

No task split or dependency change was necessary at draft review. Worker micro-plans remain subject to substantive sizing checks.

## Verification

The parent checked the revised draft for:
- Eleven unique task IDs and eleven pending STATE rows.
- Dependencies only on existing earlier-wave tasks.
- Pairwise-disjoint must-edit/can-create/TODO/scratch paths within each wave.
- Python syntax and shell argument structure of every targeted check.
- Existing local file-link targets and whitespace.
- Presence of the corrections listed above.

These checks validate the plan structure, not HYDRUS numerical behavior. Heading checks cannot prove an exhaustive audit; source/evidence and independent master review remain required. Heading anchors in links require manual review.

## Remaining judgment calls for plan approval

- Isolated worktrees and local baseline/task commits on docs/master-plan are proposed execution defaults.
- Start with gpt-5.6-luna at low effort; bounded reruns use gpt-5.6-terra at medium effort. No currency/token ceiling was requested.
- Broad tasks may need a documented split at micro-plan review; the orchestrator must preserve scope and rerun dependency/disjointness checks.
- Exact future feature variants, bindings/platforms, precision and tolerance values are master-plan recommendations or explicit user gates, not decisions silently finalized here.

Status: draft reviewed and corrected; user approved this orchestration on 2026-09-19 (“this looks good”). Execution has not started.


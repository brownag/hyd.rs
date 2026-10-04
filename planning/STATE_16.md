# State — Orchestration 16

Plan status: **Approved by user on 2026-09-30.**
Execution status: **Not started.** No production code, builds, tests, solvers, or commits were performed while drafting.
Plan: [EXECUTION_PLAN_16.md](EXECUTION_PLAN_16.md). Worker template: [PROMPT_TEMPLATE_16.md](PROMPT_TEMPLATE_16.md).

## Baseline and guardrails

- [x] Read-only reconnaissance completed 2026-09-30.
- [x] Committed baseline: `ec90ba6dcff06de1326670d6a0af46c60a0cbde2` on `phase4-water-acceptance`.
- [x] Working tree clean at planning start (`git status --porcelain=v1` empty).
- [x] Prior recorded baseline only (not rerun): 145 Rust; 13 runner/outlet Python; 17 benchmark; 86 convergence tests; fmt, Clippy with warnings denied, diff check passed (STATE_15).
- [x] Read Orchestration 15 plan/state, synthesis, four family reports, printed-parity evidence, Rust/Fortran anchors, casebook, parity tests and repository instructions search.
- [x] Historical limits carried forward: no normalized reaction equations; diffusion Fortran failed before assembly/no retry; Robin row causes unresolved; Example 3 Fortran rows absent/no retry and no common endpoints; unintended seventh reaction-distinct Fortran launch excluded.
- [x] User authorized bounded diffusion and Example 3 diagnostic runs (one launch each, task-local reviewed ledger, 60 s timeout, 100 MiB cap, no retry).
- [x] User approved conditional implementation where a Rust defect is supported; no-fix is acceptable, with closer investigation deferred to a later session.
- [ ] No diagnostic task may launch without its own approved pre-launch ledger, one-launch cap, 60 s timeout, 100 MiB cap, and no-retry rule.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 1 | 16.1.1 | Reaction equation normalization | pending |
| 1 | 16.1.2 | Scalar Robin row normalization | pending |
| 1 | 16.1.3 | Diffusion Fortran discriminator (separately run-gated) | pending |
| 1 | 16.1.4 | Example 3 Fortran discriminator (separately run-gated) | pending |
| 2 | 16.2.1 | Conditional reaction correction/no-fix disposition | pending |
| 2 | 16.2.2 | Conditional Robin correction/no-fix disposition | pending |
| 2 | 16.2.3 | Conditional diffusion correction/no-fix disposition | pending |
| 2 | 16.2.4 | Conditional Example 3 correction/no-fix disposition | pending |

## Task reports

None. Planning artifacts are drafts; no worker tasks have started.

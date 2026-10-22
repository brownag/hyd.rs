# State — Orchestration 22

Plan status: **Approved by user on 2026-10-22 (“YES”).**
Execution status: **Not started.**
Plan: [EXECUTION_PLAN_22.md](EXECUTION_PLAN_22.md). Worker template: [PROMPT_TEMPLATE_22.md](PROMPT_TEMPLATE_22.md).
Follows: Orchestration 21 (complete; [STATE_21.md](STATE_21.md) § Follow-ups).

## Baseline and guardrails

- [ ] Baseline at HEAD (the O21 wave-end A4 at `b22c5fa` covers the same code; re-confirm in Wave 0):
  - `cargo test --workspace --no-fail-fast`: 429 passed, 0 failed, 1 ignored;
  - Python `unittest discover -s tools`: 317 OK;
  - fmt, clippy and `git diff --check`: clean;
  - parity `--fixture-group all`: 44/44.
- [ ] Disk: at least 10 GB free before each wave.
- [x] User decisions D1–D9 recorded in [EXECUTION_PLAN_22.md](EXECUTION_PLAN_22.md) (2026-10-22).
- [x] Plan review (Spec and Standards) completed and applied (2026-10-22): Spec 0 blockers/4 should-fix/4 nits; Standards 3 blockers/4 should-fix/2 nits; all applied (Wave 3 restructured into 3A/3B/3C with phase roots and `backend` field; 22.1.2 split into 22.1.2/22.1.4).
- [x] User approves this execution plan (2026-10-22).
- [ ] Wave 6 user gate: thresholds approved (D10).
- [ ] Holdouts pass (`22.7.1`) before M5-V closure (`22.8.1`).

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 0 | — | Baseline, commit plan | pending |
| 1A | 22.1.1 | Serialize cached parity-runner build (flake) | pending |
| 1A | 22.1.2 | Negative zero E-format sign | pending |
| 1A | 22.1.3 | ADR 0018 default backend | pending |
| 1B | 22.1.4 | NOD_INF depth sign; NodOut flux recomputation | pending |
| 2 | — | Safe worktree cleanup (D5) | pending |
| 3A | 22.3.1 | `CASES_V2.json` and spec checker | pending |
| 3B | 22.3.2 | v2 rows through reference and lifecycle tools | pending |
| 3B | 22.3.3 | Harness backend selection; public-API path check | pending |
| 3C | 22.3.4 | Benchmark/convergence/holdout CLIs on v2; bug fixes | pending |
| 4A | 22.4.1 | Study: closed reactions | pending |
| 4A | 22.4.2 | Study: `diffusion_cosine` | pending |
| 4A | 22.4.3 | Study: `scalar_robin` | pending |
| 4B | 22.4.4 | Study: Example 3 chain | pending |
| 5 | 22.5.1 | Error budgets, orders, proposed thresholds | pending |
| 6 | — | User gate D10: thresholds | pending |
| 7 | 22.7.1 | Holdouts | pending |
| 8 | 22.8.1 | M5-V closure docs, ADR 0019 | pending |

## Verification log

## Task reports

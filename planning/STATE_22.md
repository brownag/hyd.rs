# State — Orchestration 22

Plan status: **Approved by user on 2026-10-22 (“YES”).**
Execution status: **Wave 1A done; Wave 1B next.**
Plan: [EXECUTION_PLAN_22.md](EXECUTION_PLAN_22.md). Worker template: [PROMPT_TEMPLATE_22.md](PROMPT_TEMPLATE_22.md).
Follows: Orchestration 21 (complete; [STATE_21.md](STATE_21.md) § Follow-ups).

## Baseline and guardrails

- [x] Baseline at HEAD (Rust 429/0/1 confirmed 2026-10-22) (the O21 wave-end A4 at `b22c5fa` covers the same code; re-confirm in Wave 0):
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
| 1A | 22.1.1 | Serialize cached parity-runner build (flake) | done |
| 1A | 22.1.2 | Negative zero E-format sign | done |
| 1A | 22.1.3 | ADR 0018 default backend | done |
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

- Wave 1A (merged 7124aa0): cargo workspace 430/0/1; Python 320 OK; fmt, clippy, diff --check clean; parity 44/44 (worker run on 22.1.2; paired gate in cargo suite passed). No POLICY_HASHES change needed.

## Task reports

### 22.1.1 — done (a9583ee)
fcntl lock around check-and-build in `tools/solute_rust_candidate_adapter.py`; temp file + os.replace; 3 new tests; harness flake check 3/3 at 12/12. Tests load the adapter via importlib (normal import breaks fake-module tests in test_solute_fortran_parity). Gotcha: a real `target` dir in a worktree makes `ln -s target` land inside it.

### 22.1.2 — done (2a485dd)
`format_fortran_e` prints `-0.…E+00` for negative zero. Evidence: probe in `.development/22.1.2/probe/`, `sd002_defect_a/T_LEVEL.OUT:9` (parity reference outputs hold no such line). `format_fortran_f` already matches gfortran for -0.0. 38 of 44 candidate digests changed (observations only), 44/44 held.

### 22.1.3 — done (e6fa676)
ADR 0018 written, ADR 0013 status line, README row and note.

# State — Orchestration 23

Plan status: **Approved by user on 2026-10-25 ("yes").**
Execution status: **Not started.**
Plan: [EXECUTION_PLAN_23.md](EXECUTION_PLAN_23.md). Worker template: [PROMPT_TEMPLATE_23.md](PROMPT_TEMPLATE_23.md).
Follows: Orchestration 22 (complete; M5-V partial/open; [STATE_22.md](STATE_22.md)).

## Baseline and guardrails

- [ ] Baseline at HEAD (re-confirm in Wave 0; figures from the 22.8.1 final gates, 2026-10-24):
  - `cargo test --workspace --no-fail-fast`: 448 passed, 0 failed, 1 ignored;
  - Python `unittest discover -s tools`: 354 OK;
  - fmt, clippy and `git diff --check`: clean;
  - parity `--fixture-group all`: 44/44.
- [ ] Disk: at least 10 GB free before each wave.
- [ ] Holdout guard: no holdout row has run (A6). Confirm before Wave 7 with `git log` and the `.development/23-holdout/` listing.
- [ ] `CASES_V2.json` SHA-256 frozen at Wave 3 (record below). `THRESHOLDS_V2.json` (SHA `c236eb7c…` before this orchestration) is regenerated at gate C against the frozen value.
- [x] User approves this execution plan (2026-10-25).
- [ ] Gate B (bounds review and signature) given.
- [ ] Gate C (holdout-level thresholds) given.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 0 | — | Baseline, commit plan | pending |
| 1A | 23.1.1 | Holdout guard reads status and SHA | pending |
| 1A | 23.1.2 | Bridge row inventory mismatch: explain and fix | pending |
| 1A | 23.1.3 | Diffusion end-row loss: SD-005 or "Not defects" | pending |
| 2 | 23.2.1 | Isolation study design | pending |
| 2 | — | Orchestrator gate; `POLICY_HASHES` check | pending |
| 3 | 23.3.1 | CASES_V2 isolating rows, f32 relabel, checker | pending |
| 3 | — | Tool freeze; record SHA-256 values | pending |
| 4A | 23.4.1 | Isolating study: `diffusion_cosine` | pending |
| 4A | 23.4.2 | Isolating study: `scalar_robin` | pending |
| 4A | 23.4.3 | Example 3 space re-run (authenticated); space isolation | pending |
| 4B | 23.4.4 | Example 3 time isolation; bridge re-run | pending |
| 5 | 23.5.1 | Bounds drafting | pending |
| 5 | — | **Gate B**: user signs bounds review | pending |
| 6 | 23.6.1 | Classify; error budgets; threshold proposal | pending |
| 6 | — | **Gate C**: user approves thresholds; regenerate `THRESHOLDS_V2.json` | pending |
| 7 | 23.7.1 | Holdout runs (three rows) | pending |
| 8 | 23.8.1 | M5-V closure record (closed or partial); known limits | pending |

## Frozen values (filled at the named gate)

- CASES_V2.json SHA-256 (Wave 3 freeze): _pending_
- Tool hashes at freeze (checker, metrics, lifecycle, convergence CLI): _pending_
- `POLICY_HASHES` re-pins (Wave 1A and Wave 3): _pending_

## Verification log

- **Wave 0 baseline at HEAD `25baab4`** (2026-10-25, branch `phase4-water-acceptance`, clean tree apart from planning files):
  - `cargo test --workspace --no-fail-fast`: 448 passed, 0 failed, 1 ignored (matches 22.8.1).
  - `python3 -m unittest discover -s tools -p 'test_*.py'`: Ran 354 tests, OK.
  - `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean. `git diff --check`: clean.
  - Parity `--fixture-group all --backend fortran-compatible`: _running (log `.development/23-gate-0/parity.log`)_.
  - Disk: 211 GB free on `/` (≥10 GB ✓).
  - Holdout guard: no holdout row run (no `.development/23-holdout/` exists).

## Task reports

_Empty. Workers report to the orchestrator; the orchestrator distils each report here (≤15 lines)._

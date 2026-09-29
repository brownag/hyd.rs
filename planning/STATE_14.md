# State — Orchestration 14

Plan status: **Complete.**
Evidence status: **Six canonical pairs compared read-only; field-level coverage report accepted. No solver starts or analytical reference calculations authorized.**
Plan: [EXECUTION_PLAN_14.md](EXECUTION_PLAN_14.md). Worker template: [PROMPT_TEMPLATE_14.md](PROMPT_TEMPLATE_14.md).

## Baseline and guardrails

- [x] Read-only reconnaissance completed on 2026-09-28; no tests, builds, solver runs, or reference calculations run for this plan.
- [x] Last recorded baseline: 145 Rust tests; 13 runner/outlet Python tests; 12 comparator self-checks; 85 convergence classifier/evidence/CLI tests; fmt, Clippy with warnings denied, and diff checks passed (Orchestration 13 wave-end record).
- [x] Six executable examples have retained canonical Fortran/Rust pairs: five component pairs and Example 3 bridge pair.
- [x] Preserve Orchestration 13 and blocked 13.4.1 as history. 13.5.1/13.6.1 are superseded only for this parity goal.
- [x] No solver/analytical run is authorized. Any future gap run needs a case-specific user-approved amendment. User-set new artifact/build/scratch ceiling: 1,000,000,000 bytes.
- [x] Maximum worker concurrency: 3. Canonical comparison scope: one retained pair per example; use supplemental pairs only for a named gap.
- [x] User approved Orchestration 14 by explicit command `run plan 14` (read-only parity scope only).

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 1 | 14.1.1 | Authenticated canonical/supplemental artifact inventory and validation-failure classification | done |
| 2 | 14.2.1 | Field-aware, print-interval cross-language comparator and tests | done |
| 2b | 14.2.2 | Correct coordinate interval matching and direct CLI invocation | done |
| 3 | 14.3.1 | Read-only six-example comparison and per-field coverage report | done |
| 4* | 14.4.1 | Case-specific run proposal only, or `skipped_no_remediable_gap`; zero starts | skipped_no_remediable_gap |

*Wave 4 starts only if Wave 3 records a remediable gap that retained artifacts cannot resolve. Proposal preparation confers no run authority.

## Task reports

## 14.1.1 — Inventory canonical pairs and validation failures
- Status: accepted; six canonical pairs authenticated, 36 supplemental records indexed, four failed manifests excluded across three failure classes.
- Changed files: `tools/solute_printed_parity_inventory.py`; `tools/test_solute_printed_parity_inventory.py`; `planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.{json,md}`; `planning/tasks/14.1.1.todo.md`.
- Checks: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity_inventory` — pass — 6 tests; orchestrator rerun also passed.
- Checks: `git diff --check` — pass.
- Deviations: Plan's component batch spelling `...278c4` corrected by evidence to authenticated `...278cba`.
- Dependency gotchas: none; no solver/build/reference process started.

## 14.2.1 — Add field-aware printed-parity comparison
- Status: accepted; compares concentration and theta against source-backed Fortran print intervals, with unmapped raw fields explicitly unavailable.
- Changed files: `tools/solute_printed_parity.py`; `tools/test_solute_printed_parity.py`; `planning/tasks/14.2.1.todo.md`.
- Checks: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity` — pass — 7 tests; orchestrator rerun also passed.
- Checks: `git diff --check` — pass.
- Deviations: one small-model acceptance retry followed by mid-tier correction set unverified capacity/sink formats to unknown.
- Dependency gotchas: per-field coverage is concentration/theta comparable; other candidate nodal fields remain unavailable with reasons.

## 14.3.1 — Compare canonical pairs and publish coverage (attempt 1)
- Status: stopped before report generation; no partial evidence emitted.
- Checks: named comparator unit test — pass — 7 tests; prescribed CLI — failed — import `tools` unavailable; `PYTHONPATH=.` retry — failed — coordinate boundary comparison at diffusion_cosine node 2.
- Evidence: Fortran `-0.0312` (`f10.4`) implies an inclusive orientation-correct interval containing Rust `0.03125`; float tolerance rejected the exact upper edge.
- Deviations: none; task stayed read-only and stopped as required.
- Dependency gotchas: Wave 3 resumes after 14.2.2 fixes coordinate interval comparison and direct CLI import bootstrap.

## 14.2.2 — Correct coordinate interval matching and CLI invocation
- Status: accepted; sign-inverted coordinates are matched against exact inclusive `f10.4` intervals; direct script bootstraps repository imports.
- Changed files: `tools/solute_printed_parity.py`; `tools/test_solute_printed_parity.py`; `planning/tasks/14.2.2.todo.md`.
- Checks: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity` — pass — 10 tests; orchestrator rerun also passed.
- Checks: `env -u PYTHONPATH PYTHONDONTWRITEBYTECODE=1 python3 tools/solute_printed_parity.py --help` — exit 0; `git diff --check` — pass.
- Deviations: none.
- Dependency gotchas: Wave 3 may now rerun the canonical retained-pair comparison.

## 14.3.1 — Compare canonical pairs and publish coverage
- Status: accepted; all six authenticated canonical pairs compared, with per-record pass/fail/unavailable results.
- Changed files: `planning/master/SOLUTE_PRINTED_PARITY_EVIDENCE.md`; `.development/14.3.1/printed_parity.json` (immutable output; SHA-256 `d3a2f97e45536e14b3323305c032ca67e51acd5479882fcb232a20960add29ad`); `planning/tasks/14.3.1.todo.md`.
- Checks: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity` — pass — 10 tests; exact canonical CLI completed for six cases; `git diff --check` — pass.
- Findings: theta passes for all records; concentrations have both passes and failures (9,092 failed records across four cases); other candidate fields are unavailable with reasons. Example 3 has 4 profiles, 1,001 nodes, 3 species, and zero common endpoints.
- Deviations: initial attempt stopped before report due to coordinate-boundary/import defects; fixed by 14.2.2, then comparison completed.
- Dependency gotchas: none; the comparison has no remediable retained-data coverage gap.

## 14.4.1 — Conditional run proposal
- Status: skipped_no_remediable_gap; no proposal or solver run created.
- Basis: authenticated canonical pairs cover all six cases and their matched profile schedules; concentration failures are measured parity discrepancies, while unavailable fields have semantic/output mapping limits a new retained pair cannot resolve.
- No solver/build/reference process was started.

## Wave-end regression checks
- `cargo test --workspace --quiet` — pass — 145 Rust tests.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_fortran_reference tools.test_solute_outlet_contract` — pass — 13 tests.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_benchmarks` — pass — 17 tests.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_convergence tools.test_solute_convergence_evidence tools.test_solute_convergence_cli` — pass — 86 tests.
- `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check` — all passed.

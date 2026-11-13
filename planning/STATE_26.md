# Orchestration 26 state — G22.1 heat top/bottom boundary rows with vapor

Created: 2026-11-12. Status: **PLAN DRAFT — awaiting user approval. Nothing executed.**
Plan: [EXECUTION_PLAN_26.md](EXECUTION_PLAN_26.md).
Worker: [PROMPT_TEMPLATE_26.md](PROMPT_TEMPLATE_26.md).
Goal: [GOALS.md](GOALS.md) circled goal G22.1. Coverage: [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md).

## Baseline and pre-wave-1 checklist

- [ ] Working tree clean before wave 1 (`git status --short`). Tree was clean at `237642f` when planned.
- [ ] Repeat the five Quick Start suites on `237642f` (orchestration 25 baseline was on `a8fc200`):

| Suite | Command | Result |
|---|---|---|
| Build | `cargo build --workspace --locked` | pass (exit 0) |
| Rust | `cargo test --workspace --no-fail-fast` | pass: 584 passed, 0 failed, 2 ignored (matches STATE_25) |
| Format | `cargo fmt --all -- --check` | pass (exit 0) |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pass (exit 0) |
| Python | `python3 -m unittest discover -s tools -p 'test_*.py'` | pass: 447 tests, OK |

- [ ] Reference executable located by SHA-256 `824179ab…` under `.development/25.2.2/`, or rebuild recorded (decision D8). Path: pending.
- [ ] `planning/tasks/` directory present for worker micro-TODOs.
- [ ] User approved this plan (Phase 6). Approval recorded: pending.

## Numeric approval gate (GATE N1)

- [ ] Wave 2 reference-only evidence complete (26.2.2).
- [ ] Orchestrator proposes criteria v3: per-field tolerances, print rules, zero floor, discrimination (vapor on/off) per new form.
- [ ] User approves the numbers before any candidate comparison (decision D6).
- [ ] Criteria v3 written to `reference/vapor_fortran/criteria.json` and frozen with a contract revision. Hash: pending.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 1 | 26.1.1 | Heat-boundary code audit (source and Rust) | done (audit committed) |
| 2 | 26.2.1 | Vapor heat-boundary case specs and producer | done (07cce6e) |
| 2 | 26.2.2 | Reference-only fixture run | done (fixtures committed; reference-only) |
| — | GATE N1 | User approves numeric criteria v3 | pending |
| 3 | 26.3.1 | Vapor config accepts audited forms, rejects others | pending |
| 3 | 26.3.2 | TempAdj and top/bottom vapor heat terms | pending |
| 4 | 26.4.1 | Rust candidate runs and parity test | pending |
| 4 | 26.4.2 | Python comparator against criteria v3 | pending |
| 5 | 26.5.1 | Independent balance checks for accepted forms | pending |
| 6 | 26.6.1 | Acceptance, coverage and goal handoff | pending |

Max two workers. Separate worktrees with absolute target and run roots. The
orchestrator owns shared state and serial full-suite gates, one task commit at a time.

## Wave gates

_Filled in as each wave completes (commits, serial gate results, deviations)._

## Task reports

## 26.1.1
- Status: done. Audit: `planning/master/M6_VAPOR_HEAT_BOUNDARY_AUDIT.md`. Micro-TODO: `planning/tasks/26.1.1.todo.md`.
- Files changed: the audit file only (created). No code, test or fixture changed.
- Tests: none run (doc task). Orchestrator spot-checked TEMPER.FOR:249-255, :220, INPUT.FOR:1236, config.rs:162-172. All match.
- Deviations: worker model was Sonnet for this task, not Haiku as chosen by the user. Later workers use Haiku.
- Decisions taken: Q1 yes (kTopT=-1 only top code proposed for acceptance). Q2 lEnBal out of scope for vapor.
- Findings: H1 confirmed with nuance (kTopT < -1 omits the TEMPER.FOR:255 term; reject-with-trigger). H2, H3, H4 confirmed. Candidates: DF, DZ, FD, FF, FZ; kBotT==0 and kBotT<0 are candidates; kTopT<-1 is reject-with-trigger.
- Gotchas for 26.2.1: the candidate set includes kBotT==0 (DZ, FZ) and kBotT<0 variants.

## 26.2.1
- Status: done (commit 07cce6e). Worker: Haiku.
- Files: `tools/run_vapor_reference.py` (expected set +10), `tools/test_run_vapor_reference.py` (+3 tests), 10 new case dirs `reference/vapor_fortran/cases/vapor_heatbc_{df,dz,fd,ff,fz}_{on,off}/`.
- Flux values set to 0.0 for every imposed flux (approved). IDs use underscores (approved).
- Tests: `tools.test_run_vapor_reference` 15 tests, OK. Existing cases and fixtures untouched.

## 26.2.2
- Status: done, reference-only evidence class `pinned-fortran-trajectory`-equivalent (no Rust comparison, no tolerances). Worker: Haiku.
- Executable: rebuilt via harness into `.development/26.2.2/build/` (GNU Fortran 13.3.0). SHA-256 `e6d69dd352d7dc99eba98ca1c5a14391eee8dd20a98bee6e5736ed5035ada8fa`. **Does NOT match pinned `824179ab…`.** Approved to proceed with the new hash as provenance; mismatch is reported, not hidden.
- Fixtures: 10 new under `reference/vapor_fortran/fixtures/vapor_heatbc_*.json`. Existing 6 byte-identical (checked against pre-run baseline).
- Output times `[0, 6, 24, 72, 240]`, 41 nodes. No failed IDs; no retry.
- Reproducibility: `NOD_INF.OUT` header contains a run-time stamp, so its file SHA is not reproducible. `NOD_INF_V.OUT` matched on rerun. Any reproducibility check must compare parsed values, not file hashes.
- Discrimination (max |on−off|): head df 4959.4 / dz 4927.2 / fd 20254.3 / ff 8455.9 / fz 8306.5 cm; moisture ≈0.0018–0.0035; temperature 1.0–5.4 C. Records in `.development/26.2.2/record.json`.
- **Open question for GATE N1:** the vapor-field maxima (`con_vh`, `con_vt`, `v_vapor`, `v_total`) are identical across all five pairs. Likely set at the initial time, so they do not discriminate boundary forms. Must check the time index of the maxima before proposing criteria v3.

## Open execution blockers / discoveries

- Provisional accepted-form set in 26.3.1 depends on the Wave 1 audit and the Wave 2 evidence; confirm at Phase 6.
- Hazard H1: if a negative `kTopT` code other than `-1` is reachable, Rust's all-negative-is-flux mapping is unfaithful for vapor. Must be settled in 26.1.1.

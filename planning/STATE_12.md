# State — Orchestration 12: M5 scientific-comparison catch-up

Plan status: **approved; execution in progress.**
Plan: [EXECUTION_PLAN_12.md](EXECUTION_PLAN_12.md); worker prompt: [PROMPT_TEMPLATE_12.md](PROMPT_TEMPLATE_12.md).

## Baseline checklist

- [x] Orchestration 11 complete; M5 implementation/contract evidence is recorded.
- [x] Working tree was clean after Orchestration 11 commit `73712ef` before planning this catch-up.
- [x] Latest recorded baseline: `cargo test --workspace` — 112 passed, 0 failed; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- [x] Before Wave 1, `cargo test --workspace` passed (112 tests, 0 failures); `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed serially.
- [x] Scientific evidence remains unavailable: no approved M5 fixture, paired solute oracle output, or calibrated per-field tolerance.

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 12.1.1 | Complete, provenance-labeled M5 comparison fixture | done | this commit |
| 2 | 12.2.1 | Isolated Fortran solute run and verified output manifest | planned | — |
| 3 | 12.3.1 | Normalized observables and paired Rust/reference comparison | planned | — |
| 4 | 12.4.1 | Reviewed tolerance decision and M5 evidence handoff | planned | — |

## Task reports

<!-- Append each accepted worker report here, at most 15 lines per task. -->

## 12.1.1

- **Status:** done; derived Example 3 motivated fixture definition/input bundle created.
- **Files:** new `reference/fortran_reference/solute/example3/` fixture bundle (`CASE.json` 1–128, `README.md` 1–35, `SELECTOR.IN` 1–59, `PROFILE.DAT` 1–1007, `LEVEL_01.DIR`); research note (31–43, 68–76); task TODO.
- **Targeted command:** `python3 -m json.tool reference/fortran_reference/solute/example3/CASE.json` — passed (valid JSON).
- **Gates:** `cargo test --workspace` — 112 passed, 0 failed; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- **Deviations:** none; no Fortran execution run in this task.
- **Gotchas:** No original manual deck or CHAIN arrays exist in the workspace. The authored finite-domain case is explicitly derived; task 12.2.1 must verify deck execution, realized water/dispersion, boundary signs, and solute outputs before comparison.

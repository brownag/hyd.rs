# State — Orchestration 12: M5 scientific-comparison catch-up

Plan status: **approved; Wave 2 accepted; Wave 3 blocked by the Rust boundary contract.**
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
| 1 | 12.1.1 | Complete, provenance-labeled M5 comparison fixture | done | b8dd5f3 |
| 2 | 12.2.1 | Isolated Fortran solute run and verified output manifest | done | 05bd911 |
| 3 | 12.3.1 | Normalized observables and paired Rust/reference comparison | blocked | blocker recorded in this commit |
| 4 | 12.4.1 | Reviewed tolerance decision and M5 evidence handoff | planned | — |

## Task reports

<!-- Append each accepted worker report here, at most 15 lines per task. -->

## 12.3.1

- **Status:** blocked; the frozen fixture cannot map to the current Rust M5 endpoint-boundary configuration, so paired acceptance is unmet.
- **Files:** fixture `README.md` 54–80 documents the blocker; task TODO 1–14 and execution-plan finding record the scoped stop; no parser, Rust test, normalized oracle, or residual artifact was added.
- **Evidence:** `SELECTOR.IN` prescribes `kTopCh=-1` and top concentrations `[1, 0, 0]`; Fortran `MatSet` uses species-indexed `cTop(jS)`. Rust `SoluteBoundaries` has one scalar per endpoint mode applied across the species loop (`crates/hydrus-core/src/solute.rs:68, 707, 1236–1258`).
- **Consequence:** one shared concentration or total-flux scalar cannot express the frozen species-specific inlet; separate runs would remove chain coupling. The fixture's 0.43 cm/hour Darcy value is a target, not a verified species mass-flux observation.
- **Checks:** `git diff --check` — passed. Fortran was not rerun; Rust parity test and comparator were not run because no matching Rust boundary input is representable.
- **Orchestrator gates:** `cargo test --workspace` — 112 passed, 0 failed; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- **Deviations:** acceptance is blocked by the existing API and task scope forbids public solver API changes. No numeric tolerance or source-derived golden arrays were created; dependent 12.4.1 remains unmet.

## 12.2.1

- **Status:** accepted by orchestrator after amendment C. Exactly one fresh run exited 0 and produced complete per-species outputs through 200 h; Wave 3 may proceed.
- **Files:** amendment C changes `SELECTOR.IN` 10; `CASE.json` 88; fixture `README.md` 27–36; `RUN.md` 1–124; focused fixture-layout test 11–41; task TODO 46–77. Existing runner and earlier amendment edits are preserved.
- **Preflight:** selector differs from amendment B by only tenth flag `lEquil=f` to `t`; profile/path bytes and all prior scientific JSON fields unchanged; 1001 numeric 13-field rows, NS=3, NObs=0; conditional inputs source-checked; staged source matches pinned snapshot plus unchanged patch.
- **Targeted checks:** `python3 -m json.tool reference/fortran_reference/solute/example3/CASE.json` passed; `python3 -m unittest tools.test_run_fortran_reference` passed 7/7; `git diff --check` passed.
- **Orchestrator verification:** independently checked frozen fixture/source/patch/runner bytes, all input/output/log hashes, finite solute rows and terminal markers without another execution; focused tests 7/7, `cargo test --workspace` (112 passed), `cargo fmt --check`, and `cargo clippy --workspace --all-targets -- -D warnings` passed serially.
- **Fixture command:** `python3 tools/run_fortran_reference.py --case reference/fortran_reference/solute/example3 --timeout-seconds 60` — one execution, wrapper/executable exit 0; no further executable invocation.
- **Observed outputs:** each `solute1.out`–`solute3.out` has 2000 finite 17-field records at 0.1–200 h and exact terminal `end`; `NOD_INF.OUT` has 1001 finite rows at 0/50/100/200 h. Other output-specific completion records inspected read-only.
- **Provenance:** source/archive, patch, compiler/platform/flags, executable, input/output/log hashes and observed schema recorded in `RUN.md`; old complete manifest retained verbatim. Prior fixture/run/build and verification evidence archived in `.development/12.2.1/amendment-c-audit-2p2srnlb/`.
- **Warnings:** IEEE underflow/denormal flags reported; printed header date is malformed. Scientific impact is unreviewed. Existing `NodInf` final-argument mismatch is inactive with both flags false.
- **Runner caveat:** inherited marker check accepts substrings; actual exact terminal records were separately verified. `I_CHECK.OUT` correctly ends in the input echo `tPulse=200`, not `end`; generated manifest prose corrected without changing runner.
- **Audit:** original attempts failed at selector parsing; amendment A fixed selector alignment, then failed at profile parsing; B removed the profile label but hit EOF at `INPUT.FOR:251`; C resolves the initial record-width mismatch. Earlier audit remains in TODO and preserved manifest.
- **Deviations:** none. Raw output stays in scratch; derived status retained. Rust pairing, scientific coefficient/balance review, analytical evidence, tolerances, and M5-V remain outstanding.

## 12.1.1

- **Status:** done; derived Example 3 motivated fixture definition/input bundle created.
- **Files:** new `reference/fortran_reference/solute/example3/` fixture bundle (`CASE.json` 1–128, `README.md` 1–35, `SELECTOR.IN` 1–59, `PROFILE.DAT` 1–1007, `LEVEL_01.DIR`); research note (31–43, 68–76); task TODO.
- **Targeted command:** `python3 -m json.tool reference/fortran_reference/solute/example3/CASE.json` — passed (valid JSON).
- **Gates:** `cargo test --workspace` — 112 passed, 0 failed; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- **Deviations:** none; no Fortran execution run in this task.
- **Gotchas:** No original manual deck or CHAIN arrays exist in the workspace. The authored finite-domain case is explicitly derived; task 12.2.1 must verify deck execution, realized water/dispersion, boundary signs, and solute outputs before comparison.

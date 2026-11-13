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
| 3 | 26.3.1 | Vapor config accepts audited forms, rejects others | done (a07cd92) |
| 3 | 26.3.2 | TempAdj and top/bottom vapor heat terms | done (573f816) |
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

## 26.3.1
- Status: done (a07cd92). Worker: Haiku, in worktree; base reset to cee7943 (approved).
- Files: `heat/config.rs` (constant `VAPOR_ACCEPTED_NEGATIVE_TOP_CODES = [-1]`; accepts kTopT>0 or -1; accepts every nonzero/zero kBotT), `tests/vapor_config_contract.rs` (kTopT=-1 and kBotT=0 moved out of rejected list; kTopT=-2 added), new `tests/vapor_heat_boundary_config.rs`.
- Reuses existing typed `VaporSelectionNotSupported{name:"kTopT"}` (approved; no public variant added).
- Targeted: vapor_config_contract 6/6, vapor_heat_boundary_config 5/5, heat_config_contract 12/12 unchanged.

## 26.3.2
- Status: done (573f816). Worker: Haiku, in worktree; base reset to cee7943 (approved). One retry used for a clippy `type_complexity` in a test helper (fixed by type aliases, values unchanged).
- Files: `heat/vapor/adjust.rs` (new `adjust_boundary_rows`, `top_heat_flux_vapor_terms`; `VaporHeatAdjustmentInput` unchanged), `heat/transport.rs` (vapor branch: flux-top heat_flux adds vapor parts; endpoint adjustments after SetUpHeat rows), `tests/vapor_heat_transport.rs` (+7 tests).
- Targeted: vapor_heat_transport 11/11; heat::transport 4/4; heat_driver_order and vapor_driver_order pass.
- **Deviation (recorded, not widened):** endpoint and interior sums are grouped before adding, not accumulated in source order. Expected ULP-level difference under H7; to be checked by the Wave 4 comparison, not by widening criteria.
- Gotcha for 26.4: FD (flux top + vapor) is covered at adjust/helper level; the end-to-end FD path depends on 26.3.1 config, which is now merged.

## Wave 3 serial gate (orchestrator, final: after clippy retry, HEAD 573f816)
- `cargo build --workspace --locked`: exit 0.
- `cargo test --workspace --no-fail-fast`: 596 passed, 0 failed, 2 ignored (baseline 584 + 12 new).
- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0 (first attempt failed on test-helper type_complexity; fixed by worker retry).
- `python3 -m unittest discover -s tools`: 450 tests OK (baseline 447 + 3 producer tests from 26.2.1).
- Note: the first clippy attempt ran on commit 9e3fbbb (pre-fix), which was replaced by 573f816 (same tree plus the clippy fix); the re-run above is on 573f816.

## 26.4.1 (Rust candidate parity, in progress)
- Worker: Haiku, worktree `agent-a4b02a8af1f3f57a6`, base `1f38cb1`. **Not committed.** Test file `crates/hydrus-core/tests/vapor_heat_boundary_parity.rs`.
- Result: 10 of 11 pass. Failing row: `vapor_heatbc_fd_on`, `v_liquid`, t=240, node 37 (cand 4.156e-9, ref 3.630e-9, bound 3.50e-10, ratio 1.50). Other 204 rows of that field and all other fields/cases pass.
- Ruled out: constant-dt schedule. Source runs constant 0.1 h (RUN_INF.OUT, 2400 steps, all Iter=2); schedule-matched run fails the same row (ratio 1.5018).
- Ruled out: output formula. `observe_vapor` (Rust, output.rs:111-155) matches `NodOut` (OUTPUT.FOR:776-816) term by term; Fortran formula applied to Rust inputs reproduces the Rust value.
- **Found: vapor-on head drift.** Rust head minus reference head: fd_on up to 5.70 cm at t=240; ff_on up to 4.41 cm. Vapor-off controls match to ~4e-9 cm. Temperature drift ~0.005 C. Head criterion (2% relative) still passes, so v_liquid is the field that exposes it. The drift is not in criteria v2/v3 and is not recorded in the G19 record.
- Decision (user, 2026-11-13): **hold FD open; disclose the vapor-on head drift.** FD is not accepted and not rejected. Start a src-level investigation of the vapor-coupled water step (`heat/vapor/water.rs`, `heat/vapor/adjust.rs`) as a separate task. No tolerance widened. Criteria v3 unchanged.
- Test file stays uncommitted: the default suite currently fails on `heatbc_fd_on` and must not be committed as a red gate.

## Waves 4–6 and final gate (2026-11-13)
- GATE N1: approved by user as proposed (2026-11-12). Criteria v3 frozen at `1f38cb1`; contract revision `0c4c743`; `contract_sha256` `33bdc31e…`; criteria file sha `f198d0c1…`.
- 26.4.2 comparator: committed `c509fd4`. 38 heat-boundary discrimination checks PASS on real fixtures. Existing 25.7.2 tests pass after the helper fix.
- 26.4.1 Rust parity: committed `072431e` for accepted forms only (DF, DZ, FF, FZ; on and off). 9 pass, 1 investigation test ignored. FD is held (not asserted), reported as a diagnostic.
- 26.5.1 balance: committed `df57561`. DF, DZ, FF, FZ pass with worst ratios ~7e-7 water and ~3e-7 heat, against a bound of 1.0. Zero flux only (synthetic setup carries no flux value).
- FD held open: vapor-on head drift (~5.7 cm at t=240; FF ~4.4 cm). Schedule ruled out. Src-level investigation is a separate task.
- 26.6.1 acceptance: committed `e20fe6a`. `planning/master/M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md`; coverage rows updated; GOALS G22.1 stays Circled (partially complete).
- Deviation: 25.7.1 test hardcoded criteria SHA of v2; bumped to v3 hash in `94967be` (values of existing fields unchanged; approved change). All 10 parity tests pass again.
- **Final serial gate (HEAD `94967be`):** build exit 0; Rust 607 passed / 0 failed / 3 ignored; fmt exit 0; clippy exit 0; Python 459 OK. Baseline was 584 / 447.

## Discovered

### D-26.1 Vapor-coupled head drift (approved by user 2026-11-13)
- Goal: find the cause of the vapor-on head drift against the Fortran reference (FD 5.7 cm, FF 4.4 cm at t=240; vapor-off controls match to ~4e-9 cm; temperature ~0.005 C).
- Scope: diagnosis first. Read `crates/hydrus-core/src/heat/vapor/water.rs`, `heat/vapor/adjust.rs`, `heat/vapor/driver.rs`, and the Fortran water/vapor coupling (`src_reference/` routines that update head with vapor).
- Output: a diagnosis with a verified cause and evidence. Any src fix is a separate, later decision by the user.
- Gate: FD stays held and FF's acceptance stays conditional until this is recorded.

## Open execution blockers / discoveries

- **FD blocker (open):** vapor-on head drift up to 5.7 cm at t=240 (FD), 4.4 cm (FF). Causes v_liquid miss in FD. FD held open pending a src investigation of the vapor-coupled water step. FF also carries the drift; its v_liquid passes by margin, so FF's acceptance needs the same disclosure.
- Provisional accepted-form set in 26.3.1 depends on the Wave 1 audit and the Wave 2 evidence; confirm at Phase 6.
- Hazard H1: if a negative `kTopT` code other than `-1` is reachable, Rust's all-negative-is-flux mapping is unfaithful for vapor. Must be settled in 26.1.1.

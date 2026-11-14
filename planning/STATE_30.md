# Orchestration 30 state — G22.2 follow-ups

Created: 2026-11-14. Status: **Drafted; awaiting user approval (Phase 6).** Wave 0 not started.
Plan: [EXECUTION_PLAN_30.md](EXECUTION_PLAN_30.md). Prompt: [PROMPT_TEMPLATE_30.md](PROMPT_TEMPLATE_30.md). Goal: [GOALS.md](GOALS.md) G22.2 (Circled).
Predecessor: [STATE_29.md](STATE_29.md) (closed).

## Baseline checklist (wave 0; not yet run)

- [ ] `git status --short` recorded. Known at drafting: `M  planning/GOALS.md` (staged; not staged by this plan). Confirm with the user before any commit (D11).
- [ ] `cargo build --workspace --locked`
- [ ] `cargo test --workspace --no-fail-fast` (expected baseline: 633 passed, 0 failed, 4 ignored; re-verify)
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `python3 -m unittest discover -s tools -p 'test_*.py'` (expected: 479 OK; re-verify)
- [ ] Reference build hash matches `.development/29.3.3/reference-build` (record sha256 of the executable).

## Gates

- [ ] Wave 0 baseline green (record HEAD and counts)
- [ ] G30.2 (user): free-drainage option from `planning/master/M6_VAPOR_FREE_DRAINAGE_ASSESSMENT.md`. Not yet written.
- [ ] G30.3 (user): deck designs (30.3.2), runner `BALANCE.OUT` change (30.3.1). Not yet written.
- [ ] G30.4 (user): each reference run design before its run. Not yet written.
- [ ] G30.5 (user): fixture staging and new trajectory comparisons. Not yet written.
- [ ] G30.6 (user): criteria D5 entries. Not yet written.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 0 | — | Baseline; reference-build hash; staged-file check | pending |
| 1 | 30.1.1 | Rust closed-pair flux producer (C1) | pending |
| 1 | 30.1.2 | Confirm qGWLF and ±3 rejections (C2) | pending |
| 1 | 30.1.3 | Free-drainage ConB/rBot assessment (C6) | pending |
| 2 | 30.2.1 | Free-drainage lower row with vapor and guard | pending (after G30.2) |
| 3 | 30.3.1 | Runner `BALANCE.OUT` output and parser (C4 part 1) | pending (after G30.3) |
| 3 | 30.3.2 | Deck design: dry isothermal and free-drainage pairs | pending (after G30.3) |
| 4 | 30.4.1 | Dry isothermal closed-pair run (C3) | pending (after G30.4) |
| 4 | 30.4.2 | Free-drainage vapor-on and vapor-off runs (C6 evidence) | pending (after G30.4) |
| 4 | 30.4.3 | Full-trajectory balance of the closed pair (C4 part 2) | pending (after G30.4) |
| 5 | 30.5.1 | Stage approved fixtures; extend producer; trajectory tests | pending (after G30.5) |
| 5 | 30.5.2 | Criteria proposal (D5, no write) | pending |
| 6 | 30.6.1 | Write approved criteria entries (D5) | pending (after G30.6) |

## Wave gates

_Filled by the orchestrator at each wave end: five suites and the path check (plan D8)._

## Task reports

_Appended by the orchestrator from each worker's `planning/tasks/<id>.report.md`._

## Discovered

- Recon (2026-11-14): `run_case` in `vapor_fortran_parity.rs` asserts fixed heads at both ends; closed-pair decks need a flux branch (task 30.1.1).
- Recon: the closed-pair `SELECTOR.IN` has `rTop=0 rBot=0`; the flux-known pair has nonzero rTop and rBot. Verify in each deck.
- Recon: `BALANCE.OUT` is not in the runner's output list (task 30.3.1).
- Scope change (2026-11-14, user): free drainage implemented in this orchestration (D1). The orchestration input's out-of-scope line for free drainage is superseded.

# Orchestration 30 state — G22.2 follow-ups

Created: 2026-11-14. Status: **Waves 0-5 done; waiting on G30.6 before Wave 6.**
Plan: [EXECUTION_PLAN_30.md](EXECUTION_PLAN_30.md). Prompt: [PROMPT_TEMPLATE_30.md](PROMPT_TEMPLATE_30.md). Goal: [GOALS.md](GOALS.md) G22.2 (Circled).
Predecessor: [STATE_29.md](STATE_29.md) (closed).

## Baseline checklist (wave 0; HEAD `0ed71fe`, pre-GOALS commit `16ba824`)

- [x] `git status --short` recorded. Pre-staged `M  planning/GOALS.md` committed as `0ed71fe` (`docs(planning): update GOALS.md for G22.2`) per user confirmation (D11).
- [x] `cargo build --workspace --locked` (passed)
- [x] `cargo test --workspace --no-fail-fast` (passed)
- [x] `cargo fmt --all -- --check` (failed on `crates/hydrus-core/tests/vapor_closed_pair_balance.rs:59`; noted)
- [x] `cargo clippy --workspace --all-targets -- -D warnings` (passed)
- [x] `python3 -m unittest discover -s tools -p 'test_*.py'` (483 tests OK)
- [x] Reference build hash: `15c24e35e739d992fb6f1a0d92619103baf57223cbdfd12968b0ed33dc246edb  .development/29.3.3/reference-build/hydrus-reference`

## Gates

- [x] Wave 0 baseline recorded (HEAD `0ed71fe`)
- [x] G30.2 (user, 2026-11-14): Option (a) — match `WATFLOW.FOR` as written (`rBot = -(K_L + K_vh) * Grav`); log proposed SD-008 in a subsequent docs task; accept `FreeDrainage` exclusively with `UpperBoundary::Flux`.
- [x] G30.3 (user, 2026-11-14): Deck A & B designs in `M6_VAPOR_ORCH30_DECK_DESIGN.md` and `BALANCE.OUT` runner change (`d645e42`) approved.
- [x] G30.4 (user, 2026-11-14): Wave 4 reference runs (`30.4.1`, `30.4.2`, `30.4.3`) under `.development/` approved.
- [x] G30.5 (user, 2026-11-14): approved as written: stage the four candidate fixtures (vapor_dry_isothermal_on/off, vapor_free_drainage_on/off) with cases and manifests into `reference/vapor_fortran/`; extend `vapor_fortran_parity.rs` with trajectory comparisons.
- [ ] G30.6 (user): criteria D5 entries. Proposal in `planning/master/M6_VAPOR_ORCH30_CRITERIA_PROPOSAL.md` (`b8f070a`); 7 entries are below 2.0x tolerance and need a decision. Not yet written.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 0 | — | Baseline; reference-build hash; staged-file check | done (`0ed71fe`) |
| 1 | 30.1.1 | Rust closed-pair flux producer (C1) | done (`9fc945a`) |
| 1 | 30.1.2 | Confirm qGWLF and ±3 rejections (C2) | done (`d965934`) |
| 1 | 30.1.3 | Free-drainage ConB/rBot assessment (C6) | done (`0dda1be`) |
| 2 | 30.2.1 | Free-drainage lower row with vapor and guard | done (`a0a24b2`, `4e1ebd3`) |
| 3 | 30.3.1 | Runner `BALANCE.OUT` output and parser (C4 part 1) | done (`d645e42`) |
| 3 | 30.3.2 | Deck design: dry isothermal and free-drainage pairs | done (`868e2bc`) |
| 4 | 30.4.1 | Dry isothermal closed-pair run (C3) | done (converged, 240 h) |
| 4 | 30.4.2 | Free-drainage vapor-on and vapor-off runs (C6 evidence) | done (converged, 240 h) |
| 4 | 30.4.3 | Full-trajectory balance of the closed pair (C4 part 2) | done (`WatBalR` max 0.156%) |
| 5 | 30.5.1 | Stage approved fixtures; extend producer; trajectory tests | done (`340fec7`); 4 new parity tests pass |
| 5 | 30.5.2 | Criteria proposal (D5, no write) | done (`b8f070a`); 7 entries flagged below 2.0x tolerance for G30.6 |
| 6 | 30.6.1 | Write approved criteria entries (D5) | pending (after G30.6) |

## Wave gates

_Filled by the orchestrator at each wave end: five suites and the path check (plan D8)._

- Wave 5 (30.5.1), 2026-11-14: build OK; clippy `-D warnings` OK; `cargo fmt --check` fails only on `vapor_closed_pair_balance.rs:59` (pre-existing, not touched); `cargo test --workspace` 648 passed / 0 failed; Python 499 OK. Path check: commit `340fec7` covers only the approved paths.

## Task reports

_Appended by the orchestrator from each worker's `planning/tasks/<id>.report.md`._

## Discovered

- Recon (2026-11-14): `run_case` in `vapor_fortran_parity.rs` asserts fixed heads at both ends; closed-pair decks need a flux branch (task 30.1.1).
- Recon: the closed-pair `SELECTOR.IN` has `rTop=0 rBot=0`; the flux-known pair has nonzero rTop and rBot. Verify in each deck.
- Recon: `BALANCE.OUT` is not in the runner's output list (task 30.3.1).
- Scope change (2026-11-14, user): free drainage implemented in this orchestration (D1). The orchestration input's out-of-scope line for free drainage is superseded.
- Scope change (2026-11-14, user, task 30.5.1): `tools/run_vapor_reference.py` added to 30.5.1's must-edit list to add the four approved case names to the `--all`/`expected` whitelist (plan escalation rule 4: re-scoped, not ad hoc). Its test file `tools/test_run_vapor_reference.py` may be read but not changed.

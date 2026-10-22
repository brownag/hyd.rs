# State — Orchestration 21

Plan status: **Approved by user on 2026-10-20 (“yes”).**
Execution status: **Wave 3 complete and verified (39/44, 2026-10-21); Wave 3R (21.3.4–21.3.6) dispatched.**
Plan: [EXECUTION_PLAN_21.md](EXECUTION_PLAN_21.md). Worker template: [PROMPT_TEMPLATE_21.md](PROMPT_TEMPLATE_21.md).
Supersedes: Orchestration 20 tasks `20.6.1` (gate re-run → `21.4.1`) and `20.7.1` (default switch → `21.5.1`).

## Baseline and guardrails

- [x] Reconnaissance 2026-10-20 at `470ae05` on branch `phase4-water-acceptance` (working tree clean):
  - `cargo test --workspace`: **413 passed**, 0 failed, **1 ignored**.
  - `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean. `git diff --check`: clean.
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: **317 passed**, 0 failed (`OK`, 140.7 s). A first attempt aborted with `No space left on device` (root filesystem 100%) and truncated `planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json` to 0 bytes; restored from HEAD (SHA-256 `616662ba…219c`) and re-run after space recovered (221 GB free).
  - Parity gate: 35/44 (`22d3ef1`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md:124`).
- [x] User decisions D1–D5 recorded in [EXECUTION_PLAN_21.md](EXECUTION_PLAN_21.md).
- [x] Plan review (Spec + Standards) completed and applied.
- [x] User approves this execution plan.
- [x] `STATE_20.md` rows `20.6.1`/`20.7.1` marked superseded (Wave 0).
- [x] Wave 2 user gate: Wave 3 fix specs approved (D6, 2026-10-20).
- [ ] Wave 3S user gate: fix specs from the 3R reports approved (D7).
- [ ] Genuine 44/44 all-mode Rust-vs-Fortran parity (`21.4.1`) before the default switch (`21.5.1`).

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 0 | — | Baseline, close Orchestration 20, commit plan | done |
| 1 | 21.1.1 | Long-run single-porosity trace report (`tortuosity_millington`) | done |
| 1 | 21.1.2 | Wetting-front dual-porosity trace report (`model6_no_exchange`) | done |
| 1 | 21.1.3 | Model 0 isolating-deck trace report (WatFlow vs DualPor) | done |
| 2 | — | User gate: Wave 3 fix specs (D6) | done |
| 3 | 21.3.1 | Deck-derived lTable, lExtrap, lMinStep | done |
| 3 | 21.3.2 | Single blended set_mat in wat_flow | done |
| 3 | 21.3.3 | hTemp/vCorr aliasing (SD-003), table overrun (SD-004) | done |
| 3R | 21.3.4 | Diagnose model6_exchange | in progress |
| 3R | 21.3.5 | Diagnose model7_exchange, model7_no_exchange | in progress |
| 3R | 21.3.6 | Diagnose temperature_coefficients, vapor_enabled_external_state | in progress |
| 3S/3F | — | User gate D7, then fixes from 3R | pending |
| 4 | 21.4.1 | 44-fixture gate re-run and evidence docs | pending |
| 5 | 21.5.1 | Default backend → `FortranCompatible` | pending |

## Verification log

- 2026-10-21, Wave 3 end, HEAD `a6a51b0`, orchestrator (resumed session; the earlier session merged 21.3.1–21.3.3 without recording them here):
  - `cargo test --workspace --no-fail-fast`: **419 passed**, 0 failed, 1 ignored (+6 vs baseline). A first `cargo test --workspace` run saw `test_parity_biology_filtration_deposition` fail once (paired gate rebuilding the stale cached runner during a concurrent harness run); it passed alone and in two later full runs. Flake, not regression.
  - Parity `--fixture-group all --backend fortran-compatible` (`.development/21.3V/all.json`): **39/44**. Failing: model6_exchange, model7_exchange, model7_no_exchange, temperature_coefficients, vapor_enabled_external_state (the Wave 3R set). None of the 35 baseline passes regressed.
  - fmt, clippy `-D warnings`, `git diff --check`: clean. `python3 -m unittest tools.test_solute_fortran_parity`: 20 OK. Envelope check `git diff 470ae05 HEAD -- tools Cargo.* solute.rs reference`: empty.
  - SD-003 (`compat_runner.rs:1540`, `source_defect_sd003_htemp_aliases_vcorr_on_dirichlet_nodes`) and SD-004 (`water_compat.rs:488,580,596`, `source_defect_sd004_set_mat_reads_past_table`) registered in `docs/FORTRAN_SOURCE_DEFECTS.md`.

## Task reports

### Wave 1 (2026-10-20) — docs only, A4 suites not re-run (no code changed; diffs to crates/tools/src_reference verified clean per worktree)
- 21.1.1 `8aa43df`-lineage merge `883470f`: tortuosity_millington first divergence = hTemp zeroed via Solute vCorr (HYDRUS.FOR:510-511). Combined prototype (zeroing + l_table + l_extrap + OOB table read + single set_mat) 38/44; +l_min_step 39/44; no regressions.
- 21.1.2 (`b32d2bb`): model6_no_exchange needs same 5 changes; iteration trace matches 482/482.
- 21.1.3 (`8aa43df`): Model 0 deck also diverges => WatFlow-level cause, not DualPor.
- Still failing after prototype: model6_exchange, model7_exchange, model7_no_exchange, temperature_coefficients, vapor_enabled_external_state.
- Suspected source defects: hTemp/vCorr aliasing (HYDRUS.FOR:510-511); unbounded table index (WATFLOW.FOR:482-484,515-518).
- Scratch: .development/21.1.1 (3.4 GB) etc.; disk 191 GB free.

### Wave 3 (2026-10-20/21) — fixes; verified by orchestrator at `a6a51b0`
- 21.3.1 (`f5363d9`, merge `61f16f3`): `deck_reader.rs` applies `INPUT.FOR:560-580` hTab negation and derives `l_table`; runner passes it; `L_EXTRAP = true` (`INPUT.FOR:1712`), `L_MIN_STEP_INIT = true` (`:1584`). 3 new tests.
- 21.3.2 (`fc9a87d`): `wat_flow` makes one `set_mat` call with `th_eq` output (`WATFLOW.FOR:65`); `72d2544`'s second call removed. 72d2544 protected `equilibrium_water_content_follows_the_new_head` (model6_exchange, not reactions); the reactions group stays green.
- 21.3.3 (`a6a51b0`): SD-003 copies the solver's final `v_corr` into `h_temp` after Solute (`HYDRUS.FOR:510-511`); SD-004 emulates column-major `tab(NTab+1, M)` reads in `set_mat` (`WATFLOW.FOR:482-484,515-518`). Its todo checkboxes were never flipped; the orchestrator re-ran every 21.3.3 verification (above) instead.
- Result: 39/44 (+4: tortuosity_millington, tortuosity_moldrup_artificial, gas_henry_diffusion, model6_no_exchange).


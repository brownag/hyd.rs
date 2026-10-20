# State — Orchestration 21

Plan status: **Approved by user on 2026-10-20 (“yes”).**
Execution status: **Wave 1 complete (reports merged); at Wave 2 user gate.**
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
- [ ] Wave 2 user gate: Wave 3 fix specs approved (D6).
- [ ] Genuine 44/44 all-mode Rust-vs-Fortran parity (`21.4.1`) before the default switch (`21.5.1`).

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 0 | — | Baseline, close Orchestration 20, commit plan | done |
| 1 | 21.1.1 | Long-run single-porosity trace report (`tortuosity_millington`) | done |
| 1 | 21.1.2 | Wetting-front dual-porosity trace report (`model6_no_exchange`) | done |
| 1 | 21.1.3 | Model 0 isolating-deck trace report (WatFlow vs DualPor) | done |
| 2 | — | User gate: Wave 3 fix specs | in progress |
| 3 | 21.3.x | Fixes (specified at gate) | pending |
| 4 | 21.4.1 | 44-fixture gate re-run and evidence docs | pending |
| 5 | 21.5.1 | Default backend → `FortranCompatible` | pending |

## Verification log

## Task reports

### Wave 1 (2026-10-20) — docs only, A4 suites not re-run (no code changed; diffs to crates/tools/src_reference verified clean per worktree)
- 21.1.1 `8aa43df`-lineage merge `883470f`: tortuosity_millington first divergence = hTemp zeroed via Solute vCorr (HYDRUS.FOR:510-511). Combined prototype (zeroing + l_table + l_extrap + OOB table read + single set_mat) 38/44; +l_min_step 39/44; no regressions.
- 21.1.2 (`b32d2bb`): model6_no_exchange needs same 5 changes; iteration trace matches 482/482.
- 21.1.3 (`8aa43df`): Model 0 deck also diverges => WatFlow-level cause, not DualPor.
- Still failing after prototype: model6_exchange, model7_exchange, model7_no_exchange, temperature_coefficients, vapor_enabled_external_state.
- Suspected source defects: hTemp/vCorr aliasing (HYDRUS.FOR:510-511); unbounded table index (WATFLOW.FOR:482-484,515-518).
- Scratch: .development/21.1.1 (3.4 GB) etc.; disk 191 GB free.

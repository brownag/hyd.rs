# State — Orchestration 21

Plan status: **Approved by user on 2026-10-20 (“yes”).**
Execution status: **Wave 3 complete and verified (39/44, 2026-10-21); Wave 3S approved (D7); Wave 3 (incl. 3R/3F) complete: 44/44 at `99fe2a6`; Wave 4 done (44/44 documented); Wave 5 (21.5.1) dispatched.**
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
- [x] Wave 3S user gate: fix specs from the 3R reports approved (D7, 2026-10-21).
- [x] Genuine 44/44 all-mode Rust-vs-Fortran parity (`21.4.1`, 2026-10-22, `17d9d9c`) before the default switch (`21.5.1`).

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
| 3R | 21.3.4 | Diagnose model6_exchange | done |
| 3R | 21.3.5 | Diagnose model7_exchange, model7_no_exchange | done |
| 3R | 21.3.6 | Diagnose temperature_coefficients, vapor_enabled_external_state | done |
| 3S | — | User gate D7: Wave 3F specs (21.3.7–21.3.9) | done |
| 3F-A | 21.3.7 | Dual-porosity lEquil, Model 7 immobile params, exchange errors | done |
| 3F-A | 21.3.8 | format_fortran_e ties-to-even | done |
| 3F-B | 21.3.9 | Vapor enhancement and NOD_INF_V fluxes | done |
| 4 | 21.4.1 | 44-fixture gate re-run and evidence docs | done |
| 5 | 21.5.1 | Default backend → `FortranCompatible` | in progress |

## Verification log

- 2026-10-21, Wave 3 end, HEAD `a6a51b0`, orchestrator (resumed session; the earlier session merged 21.3.1–21.3.3 without recording them here):
  - `cargo test --workspace --no-fail-fast`: **419 passed**, 0 failed, 1 ignored (+6 vs baseline). A first `cargo test --workspace` run saw `test_parity_biology_filtration_deposition` fail once (paired gate rebuilding the stale cached runner during a concurrent harness run); it passed alone and in two later full runs. Flake, not regression.
  - Parity `--fixture-group all --backend fortran-compatible` (`.development/21.3V/all.json`): **39/44**. Failing: model6_exchange, model7_exchange, model7_no_exchange, temperature_coefficients, vapor_enabled_external_state (the Wave 3R set). None of the 35 baseline passes regressed.
  - fmt, clippy `-D warnings`, `git diff --check`: clean. `python3 -m unittest tools.test_solute_fortran_parity`: 20 OK. Envelope check `git diff 470ae05 HEAD -- tools Cargo.* solute.rs reference`: empty.
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: exit 0 (OK); `planning/master` clean afterwards.
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

### Wave 3R (2026-10-21) — diagnosis only; A5 clean per worktree, verified by orchestrator. Workers hit an API rate limit mid-wave and were resumed.
- 21.3.4 (`36a3741`, merge `8a06634`), `..._21R_EXCH6.md`: model6_exchange first divergence = `lEquil` stays true in Rust; `HYDRUS.FOR:278` (`if(iDualPor.gt.0) lEquil=.false.`) not ported, so `NEquil`/`SorbConc` never run. Water matches in all 2,340 records. Prototype (`deck_reader.rs:~1056`, chem `l_equil=false` when `i_dual_por>0`) → 40/44, no regressions. Also: `STrans` never copied back to the `NOD_INF` array (unobserved); stale harness comment at `solute_fortran_compat_harness.rs:817-818`. model6_no_exchange 1-ulp residual gone. No SD.
- 21.3.5 (`67df1a5`, merge `b72dae9`), `..._21R_MODEL7.md`: two causes. (1) `compat_runner.rs:920,925` test `mat.i_model == 7` after `deck_reader.rs:549-551` remapped it to 0, so `immobile_n`/`immobile_saturated_conductivity` = None → `dual_porosity_exchange` Err on every call, swallowed by `if let Ok` at `water_compat.rs:1334`; Ks floor 1e-37 (`MATERIAL.FOR:28`) explains model7_no_exchange's 1e-34 Sorb. (2) Same `HYDRUS.FOR:278` omission as 21.3.4. Both prototyped in `compat_runner.rs` → 42/44 (model6_exchange, model7_* pass), no regressions. No SD.
- Fix-spec note: 21.3.4 and 21.3.5 share the `lEquil` fix; draft one task for it. Scratch lives inside each worktree's `.development/` (main-repo `.development/<id>` symlinks there): do not remove those worktrees before Wave 4.
- 21.3.6 (merge after `e481b6d`), `..._21R_TEMPVAPOR.md`: temperature_coefficients = output-only: `CumCh` bit-identical; `format_fortran_e` rounds the exact tie 49.0625 away from zero, gfortran to even. vapor_enabled_external_state: first divergence `ConVT` at call 1 — `i_enhanc` 0 in Rust (`compat_runner.rs:180,1015`) vs `iEnhanc=1` (`INPUT.FOR:1718`); then `vTotal` f64 sum and `vVapIso`/`vVapTerm` = 0 in `NOD_INF_V`. SD-003 alias ruled out (`vCorr` = 0). Prototypes of all four → 41/44, no regressions. No SD.

### Wave 3F-A (2026-10-21) — fixes; merged tree verified by orchestrator at `dd95e58`
- 21.3.8 (`7fc72f5`, merge `d14337e`): `format_fortran_e` uses exact `{:.Ne}` conversion (ties to even), re-based to `0.ddddE±ee`. gfortran 13.3.0 probe: 39,295 field writes, 0 mismatches (HEAD function: 23, all ties). Test `format_fortran_e_rounds_exact_ties_to_even`. Worker parity 40/44. Known unobserved difference: gfortran prints `-0.0…E+00` (all 44 reference outputs), Rust ` 0.0…E+00`; parity parses numerically, so no effect (follow-up candidate, out of scope).
- 21.3.7 (`0b6c1ff`, merge `dd95e58`): chem `l_equil=false` when `l_chem && i_dual_por>0` (`HYDRUS.FOR:271,278`, `deck_reader.rs:1065-1070`); `dual_por_materials` keys Model 7 immobile n/Ks on `i_dual_por==2`; `wat_flow` returns `Result<(), KernelError>` (error propagated by runner); `s_trans` copied back for NOD_INF; stale harness comment replaced. 5 tests (red by assertion against a stub). Worker parity: dual_exchange 4/4, nonequilibrium 5/5, all 42/44. Out-of-scope observation: Rust NOD_INF node-2 depth `0.2500` vs Fortran `-0.2500` at t=1 (unobserved).
- Orchestrator on merged tree: lib `solute_fortran_compat` 244 passed/1 ignored; harness 12/12; parity all **43/44** (`.development/21.3FA/all.json`, runner `29596d66…`), only vapor_enabled_external_state failing.

### Wave 3F-B (2026-10-21) — verified by orchestrator at `99fe2a6` (A4, serial)
- 21.3.9 (merge `99fe2a6`): `I_ENHANC = 1` (`INPUT.FOR:1718`) at initial `set_mat` and `WatFlowInputs`; `make_nod_inf_v_block` extracted to a module fn: f32 `vTotal` (`OUTPUT.FOR:816`), `vVapIso`/`vVapTerm` per `OUTPUT.FOR:769-770,783-784,804-809`. 3 tests (red vs stub); deck-level test ~15 s debug. Worker parity 44/44.
- A4 at `99fe2a6`: `cargo test --workspace --no-fail-fast` **428 passed**, 0 failed, 1 ignored; fmt, clippy `-D warnings`, `git diff --check` clean; Python discover **317 OK** (133.9 s); `planning/master` clean.
- Parity `--fixture-group all`: **44/44**, top-level `"status": "pass"` (`.development/21.3F/all.json`).
- Unobserved output differences (follow-up candidates, not parity-gated): `-0.0` E-format sign; NOD_INF node depth sign at t=1; `NodOut` recomputes `vi`/`vVi` where Rust prints solver `v`/`vv`.

### Wave 4 (2026-10-22) — docs only
- 21.4.1 (merge after `17d9d9c`): all-mode + 7 group runs at `17d9d9c`, all exit 0, **44/44** (transport 18, reactions 4, transport_properties 5, nonequilibrium 5, dual_exchange 4, biology 6, flux_concentration 2). 349 reference vs 122 candidate digests, disjoint; candidate digests identical to the orchestrator's 21.3F run in all 44; reference digests differ only in wall-clock `Date/Time` and `Calculation time` lines (not reproducible run to run: compare candidate digests). Coverage: iPsi=3 ×1; W-volumeI ×9 (6 coupled DualPor); iHyst=0 ×44. Same section appended to `SOLUTE_FORTRAN_PARITY_18.md`, `PHASE_5_SOLUTE_EVIDENCE.md`, `VALIDATION_STRATEGY.md`; 20.6.1 sections marked superseded.


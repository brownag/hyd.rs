# Water transient diagnosis (round 2): `dual_exchange/model7_exchange` and `model7_no_exchange`

Task 21.3.5, HEAD `a3c5d60` (code identical to `a6a51b0`, the Wave 3 result, 39/44), branch `orchestration/21.3.5`. Diagnosis only: no fix is committed. Claims are marked **established** (a paired trace or a run shows it) or **hypothesis**. Node numbers are internal Fortran numbers (node 1 = bottom, node 9 = top). Scratch lives under `.development/21.3.5/`.

## Run identity

- Fixtures (SHA-256 of `SELECTOR.IN`, `PROFILE.DAT`, `CASE.json`):
  - `processes/dual_exchange/model7_no_exchange`: `9b3c471f…`, `2e3cb7c5…`, `0b07d40f…`.
  - `processes/dual_exchange/model7_exchange`: `ef273457…`, `2e3cb7c5…`, `34ea4d58…`.
  - The two decks differ only in `ParD(11)` (immobile `Ks`): `0.0` vs `0.01`. Full hashes: `.development/21.3.5/canonical.sha256`.
- Canonical sources, unchanged at hand-off: `src_reference/WATFLOW.FOR` `e5591ceb…`, `HYDRUS.FOR` `fca01114…`, `SOLUTE.FOR` `81fed6ad…`, `MATERIAL.FOR` `a2a2beb4…`.
- Instrumented staged sources (both fixtures identical): `WATFLOW.FOR` `b896ae68…` (same instrumentation as 21.1.2), `HYDRUS.FOR` `4d5aed51…` (staged pre-instrumentation `d78f15aa…`, the portability-patched copy), `SOLUTE.FOR` `bde84e91…`. Fortran patches: `instr_model7_no_exchange/fortran-instrumentation.patch` `85be64c3…`, `instr_model7_exchange/…` `5cc16902…`.
- Fortran binaries: baseline `model7_no_exchange` `0295e8ed…`, `model7_exchange` `a41bd473…`; instrumented `4c35b0fd…`, `a8ab3f90…`.
- Rust instrumentation patch (trace only, HEAD behaviour): `.development/21.3.5/rust-instrumentation.patch` `723ac0ae…`. Prototype patches: `proto_dualpor2.patch` `a5e48f47…` (fix candidates A+B, no trace code), `proto_equil_only.patch` `b8e3cc44…` (B only), `proto_slots_with_instr.patch` `32136da4…` (A + traces), `proto_dualpor2_with_instr.patch` `3355d4f2…` (A+B + traces).
- Candidate runner SHA-256 (cached runner deleted before every build; each differs from the others): HEAD plain `351a96fe…`; HEAD instrumented `eafbdeee…`; A instrumented `b4acec2e…`; A+B instrumented `a71720de…`; A+B plain (all-mode run) `cb963682…`; B plain `c0e4e2f6…`.
- Method: the 21.1.2 tooling, copied and re-pathed (`run_instr.py` stages via `tools.run_fortran_reference.run_fixture`, patches the staged copy with `make_instr.py` between staging and build). Additions for this task: a `SOLUTE.FOR` `SorbConc` probe (unit 92, `sol_trace.txt`, record `M`: call counter, node, `SinkIm`, `ThImob`, `ThImobO`, `Conc`, `Sorb`, `STrans`, written after `STrans` at `:1274`, guarded by a saved call counter), and Rust records `E` (text of any `Err` from `dual_porosity_exchange`), `P` (the `DualPorMaterial` handed to `wat_flow`), `M` (same fields in `processes::sorb_conc`). Water record types `D/S/H/C/T` as in 21.1.2. Guards are `ItCum`/`Iter`/call counters only.
- Commands (worktree root, `CARGO_TARGET_DIR=.development/21-target/21.3.5`, temporary `target` symlink to it for `tools/solute_rust_candidate_adapter.py`, removed at hand-off):
  - `python3 -B -I .development/21.3.5/run_instr.py <fixture>/CASE.json .development/21.3.5/instr_<fixture>`; baseline via `tools.run_fortran_reference.run_fixture(<CASE.json>, .development/21.3.5/base_<fixture>)`.
  - `sh .development/21.3.5/build_runner.sh`, then `HYD_WAT_TRACE=<dir>/rust_trace.txt target/debug/solute_compat_deck_runner <CASE.json> <dir>/out`.
  - `python3 -I .development/21.3.5/cmp_trace.py <fortran trace> <rust trace> 1e-6` (relative tolerance; absolute floor lowered from 1e-30 to 1e-45 so ~1e-35 rates are compared).
  - `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group {all,dual_exchange} --backend fortran-compatible --task-root .development/21.3.5/<variant>`, run serially.

## Instrumentation validation

- Fortran: for both fixtures, instrumented vs baseline run and instrumented vs the 21.3V reference run (`.development/21.3V/all/00{10,11}-…/reference-run`): all eight normal outputs identical except the wall-clock `Date/Time` lines (`cmp_outputs.py`: 0 files with non-timestamp differences); `solute1.out` byte-identical.
- Rust: instrumented HEAD runner vs un-instrumented HEAD runner, both fixtures, with and without `HYD_WAT_TRACE`: `diff -r` of all output files empty.
- Probes are on separate units at statement/loop boundaries; the `SOLUTE.FOR` probe follows the `STrans` assignment and reads only already-computed values.

## First divergence

Both fixtures diverge at the same point, the first `DualPor` call after step 1 (`TLevel` 2, `ItCum` 8; step 1 took 8 iterations in both codes). Every water record before it matches (`ItCum` 0 `DualPor` is all-zero in both: uniform `ThOldIm`, no gradient yet).

| fixture | TLevel | ItCum | node | variable | Fortran | Rust (HEAD) | Fortran `file:line` | Rust `file:line` |
|---|---|---|---|---|---|---|---|---|
| `model7_no_exchange` | 2 | 8 | 2 | `SinkIm` | 5.82130290e-36 | 0.0 | `WATFLOW.FOR:1043-1053` (`iDualPor.eq.2`; `FK` `Ks=amax1(Par(5),1.e-37)` at `MATERIAL.FOR:28`) | `water_compat.rs:1334` (`if let Ok` drops the `Err`) ← `compat_runner.rs:920,925` |
| `model7_exchange` | 2 | 8 | 2 | `SinkIm` | 2.07488480e-4 | 0.0 | same | same |
| `model7_exchange` | 2 | 8 | 2 | `ThNewIm` | 7.87081870e-2 | 7.87011310e-2 | `WATFLOW.FOR:1068` | same |
| both | every step | – | – | `SorbConc` calls | 117 calls (1053 `M` records) | 0 calls | `HYDRUS.FOR:278` (`if(iDualPor.gt.0) lEquil=.false.`) → `SOLUTE.FOR:184-188` | `compat_runner.rs:62` (`chem.l_equil` kept `true`) → `driver.rs:1877` `should_call_sorb_conc` |

- The Rust `E` record shows `dual_porosity_exchange` returning `Err("iDualPor=2 needs ParD(10,M), the immobile n")` on **every** `wat_flow` call (117 of 117, both fixtures). The `P` record shows why: `DualPorMaterial { …, immobile_n: None, immobile_saturated_conductivity: None } i_model=0 i_dual_por=2`. The deck reader has already remapped `i_model` 7 → 0 (`deck_reader.rs:549-551`, as `INPUT.FOR:622-624`), but `compat_runner.rs:920,925` fill those two slots only when `mat.i_model == 7`. **Established.**
- The `Err` is discarded by `if let Ok(exchange)` at `water_compat.rs:1334`, so `SinkIm` stays 0, `ThNewIm` keeps its initial value and `WTransf` is never set for the whole run. **Established.**
- Fortran's `model7_no_exchange` `SinkIm` is not zero: with `ParD(11)=0`, `FK` floors `Ks` to `1.e-37` (`MATERIAL.FOR:28`), giving rates of 5.8e-36 to 1.3e-35. These flow into the immobile concentration (`Sorb`, printed as `Sorb` in `NOD_INF.OUT`), which is the 1e-34 `Sorb` record that fails parity. Rust's `conductivity` already reproduces the floor (`dual_domain.rs:280`); it is never reached. **Established** (the `M` records match after the fix, below).
- The second, independent divergence is solute-side: Fortran forces `lEquil=.false.` for any dual-porosity deck right after `ChemIn` (`HYDRUS.FOR:278`, a live statement in the `if(lChem)` block, after `NodInf` at `:196`), so `SorbConc` runs every step (`SOLUTE.FOR:184`, `lLinear` is true for `Nu=0`, `Beta=1`). The Rust runner keeps the deck's `lEquil=t` (`chem.l_equil`, derived in `deck_reader.rs:697-710` like `INPUT.FOR:1332-1336`), so `sorb_conc` is never called and every other `l_equil` branch in the runner/driver takes the equilibrium path. **Established** (Fortran 117 `SorbConc` calls vs Rust 0; ablation below).
- Reachability (ADR 0007), selectors `Model 7` → `iModel=0`, `iDualPor=2`, `iHyst=0`, `lTable=.true.`, `lChem=t`, `lTemp=f`, `lVapor=f`, `lBact=f`, `lDualNEq=f`, `Omega=0`, `Nu=0`, `Beta=1`: `DualPor` call `WATFLOW.FOR:43-46` (`iDualPor.gt.0`) live; `iDualPor.eq.2` branch `:1043-1056` live; `SinkIm>0` clamp `:1057-1061` fires for `model7_exchange` (e.g. node 5 rate 0.535); `HYDRUS.FOR:278` column 1-6 blank (statement, not comment/continuation), inside `if(lChem)` live; `SOLUTE.FOR:184` guard live (trace proves 117 calls); `SorbConc` dual branch `:1240-1275` live (`iDualPor.gt.0`, `.not.lBact`). No hysteresis, vapor or temperature branch is involved.

## Hypothesis tests

`A` = `immobile_n`/`immobile_saturated_conductivity` keyed on `mat.i_dual_por == 2` (`compat_runner.rs:920,925`). `B` = `chem.l_equil = false` when `l_chem && i_dual_por > 0` (`HYDRUS.FOR:278`), applied at `compat_runner.rs:62`. HEAD = 39/44 (`.development/21.3V/all.json`).

| variant | water trace vs Fortran (`D/S/H/C/T`, rtol 1e-6) | `SorbConc` trace | `dual_exchange` group | `all` group | status changes vs HEAD |
|---|---|---|---|---|---|
| HEAD | first diff `ItCum` 8 node 2 `SinkIm` (both fixtures) | F 1053 / R 0 records | 1/4 | 39/44 | – |
| A | **0 differences** (10327 / 10175 records) | F 1053 / R 0 | 1/4: `model7_exchange` fails `Conc` (0.0005), `model7_no_exchange` fails `Sorb` (3.5e-34), `model6_exchange` fails `Conc` | not run | none |
| B | not traced | – | 2/4: `model6_exchange` passes; both Model 7 fail as at HEAD | not run | `model6_exchange` fail→pass |
| **A+B** | **0 differences** (both fixtures) | **0 differences** (1053 / 1053 records, both fixtures) | **4/4** | **42/44** | `model6_exchange`, `model7_exchange`, `model7_no_exchange` fail→pass; no fixture regresses |

- A+B all-mode run: `.development/21.3.5/proto_dualpor2.json` (runner `cb963682…`, 2 min 32 s). Remaining failures are the 21.3.6 set only: `temperature_coefficients` (`sum_cvTop`, 0.001) and `vapor_enabled_external_state` (`cvTop`, 1.4e-6).
- Both changes are needed for the Model 7 decks; `B` alone also fixes `model6_exchange` (the 21.3.4 fixture). That deck has `SinkIm ≠ 0` with `iDualPor=1`, so the missing `SorbConc`/`FlMacro` path is visible there too.
- Pre-remap model tests elsewhere (`grep -rnE "i_model *== *[67]|model *== *[67]" crates/hydrus-core/src/solute_fortran_compat/`): only `compat_runner.rs:920,925` test the pre-remap value against a post-remap `i_model`. `water_compat.rs:19` `normalize_model` maps 6/7 → 0 (harmless after the remap). `adapter/nonequilibrium.rs:175,200` test `dual.water_model`, a separate request field that keeps the pre-remap model by design, so it is correct. `deck_reader.rs:544,549` is the remap itself.

## Root-cause statement

*Established* (paired traces, `E`/`P` records, and the A/B ablation): the two Model 7 fixtures fail for two independent Rust omissions, neither in the water-flow equations.

1. **Immobile `ParD(10)`/`ParD(11)` dropped for Model 7.** `compat_runner.rs:920,925` test `mat.i_model == 7` after `deck_reader.rs:549-551` has remapped `i_model` to 0 (mirroring `INPUT.FOR:622-624`). The `iDualPor=2` material therefore has no immobile `n`/`Ks`; `dual_porosity_exchange` returns `Err` on every call; `water_compat.rs:1334` swallows it; `SinkIm` stays 0 for the whole run. In `model7_exchange` this is a visible water-head difference (≤ 0.002 cm). In `model7_no_exchange` Fortran's exchange is non-zero only through the `1.e-37` `Ks` floor (`MATERIAL.FOR:28`), and the difference shows up only as the 1e-34 immobile `Sorb`.
2. **`lEquil` is not forced false for dual-porosity decks.** `HYDRUS.FOR:278` sets `lEquil=.false.` whenever `iDualPor>0`. Rust keeps the deck value. So `SorbConc` (`SOLUTE.FOR:184-188`), which updates the immobile concentration and `STrans` including `FlMacro` (`:1263-1274`), never runs. Other `lEquil`-dependent branches also take the equilibrium path.

With both corrected, water and `SorbConc` traces match Fortran record for record, and parity rises from 39/44 to 42/44 with no regression.

*Hypothesis:* `model6_exchange` (21.3.4) shares cause 2 alone. This is shown only by the `B` parity run; its trace is 21.3.4's scope.

## Suspected source defects

None. Both causes are Rust porting omissions. The pinned source is self-consistent: the `1.e-37` `Ks` floor in `FK` is used at every `FK` call and is already reproduced in Rust, and the `lEquil` override is deliberate (dual-porosity always needs the immobile update). No `SD-NNN` is proposed.

## Fix outline

Outline for the Wave 3S gate; nothing here is applied.

1. `crates/hydrus-core/src/solute_fortran_compat/compat_runner.rs:920,925` (`run_parsed_deck`, `DualPorMaterial` construction): key `immobile_n` and `immobile_saturated_conductivity` on `mat.i_dual_por == 2` (`INPUT.FOR:622-624`, `NPar=11`). Regression test: `dual_por_materials_carry_model7_immobile_slots_after_imodel_remap`, which parses the `model7_exchange` deck and asserts both slots are `Some(1.2)` / `Some(0.01)`.
2. `compat_runner.rs:62` (`run_parsed_deck`): after the deck is parsed, set an owned copy's `l_equil = false` when `bas.l_chem && mat.i_dual_por > 0`, citing `HYDRUS.FOR:278`. Do not change `deck_reader.rs:1061`: `NodInf` (`HYDRUS.FOR:196`) reads the profile before the override, with the selector's `lEquil`. Regression test: `dual_porosity_forces_noneq_sorb_conc_as_hydrus_for_278`, which asserts `sorb_conc` runs or immobile `Sorb` is non-zero for `model7_no_exchange` after step 2.
3. `water_compat.rs:1334` (`wat_flow`): stop discarding the `Err`. Either make `wat_flow` fallible, or validate the `DualPorPrimitiveState` once before the time loop and `expect` afterwards (as the test at `:2125` does), so a malformed dual-porosity material can no longer silently zero the exchange. Regression test: `wat_flow_surfaces_dual_porosity_exchange_errors`.
4. Verification for the fix task: `--fixture-group all` ≥ 42/44 with `model6_exchange`, `model7_exchange` and `model7_no_exchange` passing and no regression, plus `--fixture-group nonequilibrium` 5/5 (`dual_model6`/`dual_model7` also have `iDualPor>0` and now take the `lEquil=.false.` path; they stayed green in the A+B all-mode run). Coordinate with 21.3.4, since item 2 also closes `model6_exchange`.
5. Cannot-be-matched verdict: none. Every difference found is reproducible.

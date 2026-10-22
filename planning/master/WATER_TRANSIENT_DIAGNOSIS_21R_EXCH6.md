# Water transient diagnosis, second round: `dual_exchange/model6_exchange`

Task 21.3.4, HEAD `a3c5d60` (code identical to `a6a51b0`, parity 39/44), branch `orchestration/21.3.4`. Diagnosis only: no fix is committed. Claims are marked **established** (a paired trace or a run shows it) or **hypothesis**. Node numbers are internal Fortran numbers (node 1 = bottom, node 9 = top). Scratch lives under `.development/21.3.4/`. The worktree-local `.development/21.3.4/` is a real directory because `tools/run_fortran_reference.py:126-137` requires the task root to sit under `<worktree>/.development`. The main-repo path `.development/21.3.4` is a symlink to it.

## Run identity

- Fixture `reference/fortran_reference/solute/parity/processes/dual_exchange/model6_exchange/`: `SELECTOR.IN` `3c53d8c3…`, `PROFILE.DAT` `2e3cb7c5…`, `CASE.json` `cbfebfe6…`. It differs from the passing `model6_no_exchange` only in the 9th material value, the Model 6 transfer coefficient `ParD(9,1)` (0.1 instead of 0.0). `PROFILE.DAT` is identical.
- Canonical sources: `src_reference/WATFLOW.FOR` `e5591cebae4cbb44e8fe07520e08fc98c1522531c67753c0cc03fe2e164b5ef9`, `SOLUTE.FOR` `81fed6ad…`, `HYDRUS.FOR` `fca01114…`. All are unchanged at hand-off. The full hashes are in `.development/21.3.4/canonical.sha256` and `hashes.txt`.
- Instrumented staged sources: `WATFLOW.FOR` `b896ae68…`, which is the 21.1.2 water trace and the same file hash as that task, and `SOLUTE.FOR` `b34d3b54…`. Fortran patch `instr_f/fortran-instrumentation.patch` `a21b91b1…`.
- Fortran binaries: baseline `base_f/build/hydrus-reference` `34a19030…`; instrumented `instr_f/build/hydrus-reference` `525b5998…`.
- Rust:
  - Trace-only instrumentation patch `.development/21.3.4/rust-instrumentation.patch` `995d107e…`. It adds `HYD_SOL_TRACE` `Q`/`E`/`X`/`Y` records around the solute step in `compat_runner.rs`.
  - Prototype patch `proto_lequil_dualpor.patch` `30a531c1…` (deck_reader only).
  - Candidate runner SHA-256: HEAD (uninstrumented) `913c2f66…`; HEAD plus trace `8e006424…`; prototype plus trace `1ad463e8…`. Each was rebuilt after deleting the cached `solute_compat_deck_runner`, and every hash differs, so no stale binary was used.
- Commands, run from the worktree with `CARGO_TARGET_DIR=.development/21-target/21.3.4` and an untracked `target` symlink to it:
  - `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group dual_exchange --backend fortran-compatible --task-root .development/21.3.4/head_dual_exchange`
  - `python3 .development/21.3.4/tools/instr_run.py <fixture>/CASE.json .development/21.3.4/{base_f,instr_f} {base,instr}`
  - `HYD_SOL_TRACE=<file> target/debug/solute_compat_deck_runner <fixture>/CASE.json <out>`
  - `python3 -I .development/21.3.4/tools/cmp_sol.py instr_f/run/sol_trace.txt rust_<v>_trace/sol_trace.txt 1e-6`
  - `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group all --backend fortran-compatible --task-root .development/21.3.4/proto_lequil_dualpor`
- Trace records:
  - Fortran unit 92 and Rust use one per-call counter `kSol`, never `t`.
  - `Q`: `kSol`, `TLevel`, `lEquil` flag as seen by `Solute`, `dt`.
  - `E`: `Solute` entry, per node: `thO thN ThOIm ThNIm SinkIm Conc Sorb vO vN`.
  - `X`: `Solute` exit, per node: `Conc Sorb STrans`.
  - `Y`: `cvTop cvBot cvChIm cvCh1`.

## Instrumentation validation

- **Fortran.** All eight normal outputs (`I_CHECK`, `RUN_INF`, `T_LEVEL`, `NOD_INF`, `BALANCE`, `OBS_NODE`, `PROFILE`, `solute1`) were compared with `cmp_outputs.py`:
  - instrumented vs baseline: identical except the wall-clock date/time lines;
  - baseline vs the HEAD parity reference run: identical in the same way;
  - `solute1.out` is byte-identical.
- The `SOLUTE.FOR` writes sit before label `10` and after the `FluxConc` call, outside any arithmetic statement sequence. The `WATFLOW.FOR` writes are the validated 21.1.2 set.
- **Rust.** The instrumented runner's outputs, with and without `HYD_SOL_TRACE`, match the uninstrumented HEAD candidate run (`head_dual_exchange/0000-model6_exchange/candidate-run/rust_candidate_run/run`) under `diff -r`.
- **HEAD parity reproduced.** The `model6_exchange` `first_divergence` block equals the one in `.development/21.3V/all.json` (`Conc` S1).
- **Not just the first failing record.** The comparator stops at the first failure, so the raw outputs were also compared:
  - `NOD_INF.OUT` agrees in head, moisture, `WTrans` (`SinkIm`), `Im.Moist.` (`ThNewIm`) and flux.
  - Rust prints `STrans = 0` and `Sorb = 0` (immobile concentration) at every node and time, against 0.016-0.27 in Fortran.
  - In `solute1.out`, `Sum(cvNEql)` stays at 0 in Rust (Fortran reaches 0.13752 at 4 h).

## First divergence

| TLevel (`kSol`) | Level | node | variable | Fortran | Rust | Fortran `file:line` | Rust `file:line` |
|---|---|---|---|---|---|---|---|
| 1 (1) | – | – | `lEquil` seen by `Solute` | `.false.` | `true` | `HYDRUS.FOR:278` `if(iDualPor.gt.0) lEquil=.false.` (after `ChemIn`, `:272-277`) | no counterpart; `deck_reader.rs:697-710,829` (ChemIn rule only) → `compat_runner.rs:1112` `equil: chem.l_equil` |
| 2 (2) | final | 1 | `Conc` (exit) | 1.27031230e+00 | 1.27152940e+00 | `SOLUTE.FOR:631-641` (`NEquil` called only if `.not.lEquil`), `:644-` (`g0`/`g1` terms), `:1095-1125` (immobile update with `FlMacro`) | `driver.rs:1251` (`should_call_nequil(cfg.equil)`, `nonequilibrium.rs:112`) |
| 2 (2) | final | 2 | `Sorb` (immobile conc.) / `STrans` | 4.08859270e-03 / 2.36976110e-02 | 0 / 0 | as above; `SOLUTE.FOR:177` (`Sorb=SorbN`), `:184-188` (`SorbConc`), `:191-194` (`MassTran`) | as above |

- **The lEquil flag diverges from the first call** (established, `Q` records). In all 117 Fortran `Solute` calls `lEquil = .false.`; in all 117 Rust calls `equil = true`.
- **Step 1 still matches, because no water moves into the immobile domain yet** (established). At `kSol = 1` every `E` record has `SinkIm = 0`: the first `DualPor` call sees equilibrium `ThOldIm`. So `X`/`Y` at `kSol = 1` match.
- **The first value divergence is `kSol = 2`.** There `SinkIm(2) = 1.8654948e-2` is non-zero and identical in both traces.
- **Water matches at the solute interface** (established). Across all 2340 records, none of `thO`, `thN`, `ThOIm`, `ThNIm`, `SinkIm`, `vO`, `vN` differs beyond 1e-6 relative. The divergence is purely solute-side.
- **Reachability** (ADR 0007):
  - Model 6 sets `iModel = 0`, `iDualPor = 1` (`INPUT.FOR:617-620`).
  - `lChem = t`, so `HYDRUS.FOR:278` runs.
  - The deck's own ChemIn rule leaves `lEquil = .true.` (`INPUT.FOR:1332-1336`: `Frac = 1`, `ImmobileWaterContent = 0`, `lBact = f`).
  - With `lEquil = .false.`, the following become live:
    - the `NEquil` call (`SOLUTE.FOR:631`) and its mobile-immobile branch, since `iDualPor > 0` and `.not.lBact` (`:1104-1125`);
    - `lLinear(1) = .true.` (`INPUT.FOR:1348-1371`; `Nu = 0`, `Beta = 1`);
    - `SorbConc` (`SOLUTE.FOR:184-188`);
    - the `g1 -= SinkIm` term (`:696`) and the `g0 -= FlMacro` term (`:649`).
  - Solute `Omega = ChPar(jjj+20) = 0`, so all exchange is the water-driven `FlMacro`/`SinkIm` path.
  - The branches not taken: `NonEqInit` (`HYDRUS.FOR:285-288`; `lEqInit = lMassIni = f`), `lDualNEq`, `lBact`.
- **`NodInf` is unaffected.** It runs at `HYDRUS.FOR:196`, before `ChemIn` and line 278, so it reads `PROFILE.DAT` with the Block-A `lEquil = t` (`INPUT.FOR:247-266`): there is no `Sorb` column and the initial immobile concentration is 0. Rust's `parse_profile_dat(…, bas_inf.l_equil, …)` (`deck_reader.rs:1057-1064`) already matches this.

## Hypothesis tests

| variant | Rust change (uncommitted) | trace result vs Fortran (2340 records, rtol 1e-6) | parity `--fixture-group all` | status changes vs HEAD 39/44 |
|---|---|---|---|---|
| HEAD | none (trace only) | first divergence `kSol 2`, node 1 `Conc`; 1037 `Conc`, 812 `Sorb`, 806 `STrans`, 116×3 `Y` records differ | 39/44 (`.development/21.3V/all.json`; `dual_exchange` re-run here reproduces it) | – |
| `proto_lequil_dualpor` | `deck_reader.rs:1056`: after `parse_selector_in`, `if mat_in.i_dual_por > 0 { chem_in.l_equil = false; }` (`HYDRUS.FOR:278`) | **no differing record** | **40/44** | `model6_exchange` fail (`Conc`) → **pass**; no other change |

- With the prototype, Rust `solute1.out` matches Fortran in every line (0 differing numeric lines), including `Sum(cvNEql)`.
- `NOD_INF.OUT` matches in head, moisture, `WTrans`, `Im.Moist.`, `Conc` and `Sorb`. The `STrans` column still prints 0 in Rust (see Fix outline item 2). The parity comparator does not check that column, so the fixture passes.
- The four fixtures still failing are the other Wave 3R targets, with first divergences unchanged from 21.3V: `model7_exchange` (`head`), `model7_no_exchange` (`Sorb`, Fortran ~1.3e-34 vs Rust 0, values unchanged by the prototype), `temperature_coefficients` (`sum_cvTop`) and `vapor_enabled_external_state` (`cvTop`).

## Root-cause statement

*Established* (paired trace, prototype, parity): Rust omits the `HYDRUS.FOR:278` rule `if(iDualPor.gt.0) lEquil=.false.`, which the pinned main program applies right after `ChemIn`.

- **Why lEquil stays wrong in Rust.** Rust derives `l_equil` only from the `ChemIn` material rule (`INPUT.FOR:1332-1336`, mirrored at `deck_reader.rs:697-710`). That rule leaves it `true` for this deck (`Frac = 1`, no immobile water content).
- **What that switches off.** The compat solute step (`compat_runner.rs:1112` → `driver.rs:1251`) never calls `NEquil`, never updates the immobile-domain concentration, and drops the dual-porosity transfer terms. That covers `FlMacro` in `g0`, `SinkIm` in `g1`, `SorbConc` and the `MassTran` immobile flux.
- **Effect on outputs.** Rust's mobile concentration is too high wherever water moves into the immobile domain (`SinkIm > 0`). The printed immobile concentration, `STrans` and `Sum(cvNEql)` stay 0.
- **What it does not touch.** The water solution is unaffected: all water fields at the solute interface agree in all 117 steps.
- **Proof the rule is sufficient.** Applying the rule alone makes all 2340 trace records and `solute1.out` match, and the fixture pass (40/44, no regressions).
- **Why other dual-porosity decks pass at HEAD.** `model6_no_exchange` and `nonequilibrium/dual_model6` have the same flaw but pass. Their `SinkIm` is 0 throughout (transfer coefficient 0, or uniform head), so the immobile path is inert. This is consistent with 21.1.2's water traces.

*Hypothesis:* the same omission affects `iDualPor = 2` decks (`model7_*`), but it is not their first divergence (their status did not change under the prototype). 21.3.5 owns them.

## Suspected source defects

None. `HYDRUS.FOR:278` is deliberate pinned behavior: dual-porosity water forces the nonequilibrium solute path, and nothing in the Fortran contradicts it. The Rust gap is a porting omission, not a reproduced Fortran bug.

## Fix outline

1. **`l_equil` for dual porosity** (fixes `model6_exchange`). In `crates/hydrus-core/src/solute_fortran_compat/deck_reader.rs` (`parse_and_validate_case`, `:1049-`, the `parse_selector_in` destructure at `:1056`), set `chem_in.l_equil = false` when `mat_in.i_dual_por > 0`, citing `HYDRUS.FOR:278`.
   - Do not touch `bas_inf.l_equil`: `NodInf` (`HYDRUS.FOR:196`, `INPUT.FOR:247-266`) must keep reading `PROFILE.DAT` with the Block-A value.
   - Every runner use of `chem.l_equil` then sees the Fortran value: `compat_runner.rs:286` (`NonEqInit` gate, matches `HYDRUS.FOR:285`), `:311`, `:645-804` (mass-balance terms) and `:1112`.
   - Proposed tests:
     - `deck_reader_forces_nonequilibrium_solute_for_dual_porosity_as_hydrus_for_278` (unit test; `model6_exchange` deck gives `l_equil == false`, `bas_inf.l_equil == true`);
     - harness `model6_exchange_immobile_concentration_follows_fortran` (NOD_INF `Sorb` at 1 h, node 2, equals 0.1754 within print precision; it fails at HEAD with 0).
   - Re-run parity `--fixture-group all`: expected 40/44 with no regression. The prototype showed exactly this.
2. **`STrans` output plumbing** (cosmetic for parity, established by trace).
   - Fortran prints `STrans` from `MassTran` (`SOLUTE.FOR:191-194` → `OUTPUT.FOR:833-865`).
   - Rust computes the right value: `solute_st.s_trans` matches the Fortran `X` records under the prototype.
   - But `compat_runner.rs` never copies it back to the outer `s_trans` (declared `:86`, passed into the step at `:1166`, printed at `:860`/`:1412`). The copy-backs at `:1213-1215` cover `conc`/`sorb`/`sorb2` only.
   - Add `s_trans.copy_from_slice(&solute_st.s_trans)` after the step.
   - Proposed test: `nod_inf_reports_solute_strans_after_solute_step`. The parity comparator does not check this column, so it does not change the pass count.
3. **Note for the 3F fix task** (this task does not edit the harness): the comment at `crates/hydrus-core/tests/solute_fortran_compat_harness.rs:817-818` in `equilibrium_water_content_follows_the_new_head` ("The head still differs by about 0.02 cm, which is an open dual-porosity item") is stale. Head and moisture for this fixture now match Fortran exactly at print precision (21.3V `head`/`water_content` difference 0.0).
4. **Verdict:** this fixture can be matched. No "cannot be matched" exclusion is needed.

**`model6_no_exchange` 1-ulp residual (read-only check).** The single-record `top_flux` print difference that 21.1.2 reported (difference 9.99999999999767e-05) no longer appears. In `.development/21.3V/all.json`, in this task's HEAD `dual_exchange` re-run and in the prototype `all` run, `model6_no_exchange` passes with all 30 compared records at a maximum absolute difference of exactly 0.0, `top_flux` included. Whether the 1-ulp `Con` interpolation differences inside the water iteration still exist was not re-traced. If they do, they no longer reach any printed quantity after the Wave 3 fixes.

# Water transient diagnosis: dual-porosity wetting front (`dual_exchange/model6_no_exchange`)

Task 21.1.2, HEAD `e777ea2` (code identical to `470ae05`), branch `orchestration/21.1.2`. Diagnosis only: no fix is committed. Claims are marked **established** (a paired trace or a run shows it) or **hypothesis**. Node numbers are internal Fortran numbers (node 1 = bottom, node 9 = top of the 9-node profile). All scratch lives under `.development/21.1.2/`.

## Run identity

- Fixtures (SHA-256 of `SELECTOR.IN`, `PROFILE.DAT`, `CASE.json`):
  - primary `processes/dual_exchange/model6_no_exchange`: `2c3c7736…`, `2e3cb7c5…`, `5265766e…`.
  - control `processes/nonequilibrium/dual_model6`: `5a8bae0f…`, `891ade5c…`, `fdbb2186…`.
  - Full hashes: `.development/21.1.2/canonical.sha256`, `.development/21.1.2/hashes.txt`.
- Canonical `src_reference/WATFLOW.FOR` `e5591ceb…` and `HYDRUS.FOR` `fca01114…`, both unchanged at hand-off.
- Instrumented staged `WATFLOW.FOR` `b896ae68…`, staged `HYDRUS.FOR` `4d5aed51…`; Fortran patches `instr_primary/fortran-instrumentation.patch` `c7e87a65…`, `instr_control/fortran-instrumentation.patch` `aaecbc7c…`.
- Fortran binaries: baseline primary `29acce4a…`, control `4435719b…`; instrumented primary `1149c3f7…`, control `64c91331…`.
- Rust instrumentation patch (trace only, HEAD behaviour) `.development/21.1.2/rust-instrumentation.patch` `17a2d674…`. Per-variant patches `rust-variant-*.patch` (hashes in `hashes.txt`). Candidate runner SHA-256 per variant: HEAD (uninstrumented) `50cfbc2a…`; HEAD (instrumented) `e9129081…`; no 2nd SetMat `960fa8e5…`; + hTemp alias `98f73625…`; + lTable `a2e98d9e…`; + lExtrap `8c40c076…`; + lMinStep `1c60be20…`. Each differs from HEAD, so no stale binary was used.
- Method: `tools/stage_fortran_reference.py` + `tools/build_fortran_reference.sh` through a thin driver (`instr_run.py`) that patches the staged copy between staging and building and then calls `tools.run_fortran_reference.run_fixture`. Traces: `.development/21.1.2/instr_*/run/wat_trace.txt` (Fortran, unit 91) and `rust_<variant>_<fixture>/rust_trace.txt`. Record types: `D` (DualPor result at `WatFlow` entry), `S` (`hTemp`, `hNew` at `SetMat` entry), `H` (per-iteration header: `dt`, `rTop`, `vTop`, `EpsTh`, `EpsH`, failing node, `t`), `C` (per node after the convergence test: `hTemp hNew Con ConO Cap ThNew ThEq ThNewIm SinkIm`), `T` (after `TmCont`: `dtMaxA dtOpt dt`). Guards are `ItCum`/`Iter` counters only.
- Commands: `python3 -I .development/21.1.2/instr_run.py <fixture>/CASE.json .development/21.1.2/instr_<name>`; `.development/21.1.2/run_variant.sh <variant>` (trace compare via `cmp_trace.py` + both parity groups); `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group {dual_exchange,nonequilibrium,all} --backend fortran-compatible --task-root .development/21.1.2/h1_<variant>_<group>`.
- Local note: `tools/solute_rust_candidate_adapter.py` hard-codes `<repo>/target/debug`; a temporary `target` symlink to `CARGO_TARGET_DIR` was used and removed.

## Instrumentation validation

- Fortran: instrumented vs baseline run, all eight normal outputs (`I_CHECK`, `RUN_INF`, `T_LEVEL`, `NOD_INF`, `BALANCE`, `OBS_NODE`, `PROFILE`, `solute1`), both fixtures: identical except the wall-clock `Date/Time` line 3 (a baseline-vs-baseline rerun differs the same way). `solute1.out` is byte-identical. Checked with `cmp_outputs.py`.
- Rust: instrumented HEAD runner vs the un-instrumented HEAD candidate (`h1_head_*/…/candidate-run/rust_candidate_run/run`), both fixtures: `diff -r` empty (byte-identical). Variant builds only add the same `trc` calls; the parity runs are made without `HYD_WAT_TRACE`, so they do not write traces.
- The trace was written at loop boundaries to a separate unit, so no arithmetic statement is reordered.

## First divergence

HEAD vs pinned Fortran, primary fixture, step 1 (`ItCum` 1 matches in every field; `S` inputs for the second Picard iteration are identical):

| TLevel | Iter | node | variable | Fortran | Rust | Fortran `file:line` | Rust `file:line` |
|---|---|---|---|---|---|---|---|
| 1 (`ItCum` 2) | 2 | 2 | `Con` | 1.13238020e-2 | 1.13139490e-2 | `WATFLOW.FOR:470-476` (blend `hiM=0.1*hi1+0.9*hi2`), `:496-503` (table interpolation) | `water_compat.rs:1375-1411` (second `set_mat`, `hTemp=hNew`); `:559-565` (`&& l_table`) |
| 1 (`ItCum` 2) | 2 | – | `vTop` | -1.94473510 | -2.31600190 | `WATFLOW.FOR:293` | `water_compat.rs:787` |

- The Fortran value equals the 100-point table interpolation between h = -86.975 and -100 evaluated at the blended head -93.1013 (0.0113238025 from a stand-alone recomputation). Rust HEAD returns the analytic `K(-92.3348)=0.0113139`.
- Step-1 iteration count: Fortran **8**, Rust HEAD **9** (20.6.1 claim reproduced). Total Picard iterations over the run: Fortran 482, Rust HEAD 199 (different time stepping).
- The control (`dual_model6`, uniform -100 head) first diverges at HEAD in `S` (`ItCum` 2, `Iter` 0, nodes 1 and 9): `hTemp` Fortran `0.0`, Rust `-100`. It still passes parity (see below) because the boundary-node `Con` difference does not reach any printed quantity.

Five independent causes, found in this order by repeated trace comparison (each row: first divergence left after adding that change, cumulative):

| step | change (cumulative) | primary first divergence beyond 1e-5 | control |
|---|---|---|---|
| A | none (HEAD) | `ItCum` 2 / `Iter` 2 / node 2 `Con` (above) | `ItCum` 2 / `Iter` 0 / nodes 1,9 `hTemp` |
| B | remove 2nd `set_mat`, route 1st call's `theta` to `ThEq` | same record, `Con` 1.1034e-2 (analytic, no table) | same `hTemp` |
| C | + `hTemp := vCorr` after each solute step | same record | `ItCum` 3 `Con` node 1 (table) |
| D | + `l_table = true` | `ItCum` 8 / `Iter` 0 / node 2 `hNew` -41.787 vs -56.559 (extrapolation) | **none (1880 records)** |
| E | + `lExtrap = true` | `ItCum` 15 / `dt` 0.034004 vs 0.034170 (`dtOpt` not clamped at first `TmCont`) | none |
| F | + `lMinStep = true` | **none: 10327 records, 482 `H` records, differences only at the 1e-4 relative level in `EpsTh`** | none |

Ablation (variant F with one change reverted) reintroduces a divergence in every case: no `l_table` -> `ItCum` 2 `Con`; no `lExtrap` -> `ItCum` 8 `hNew`; no `lMinStep` -> `ItCum` 8 `dtOpt`; no alias -> `ItCum` 8 `hTemp` node 1. The task-local zero-exchange uniform-head control (`.development/21.1.2/ctrl_zero_exchange/`, `dual_model6` with the Model 6 transfer coefficient set to 0.0; never placed under `reference/`) matches variant F in all 1919 records, and at HEAD diverges at the same `hTemp` record as the control.

Remaining differences once variant F is in place:
- 1-ulp `Con` differences in table interpolation begin at `ItCum` 2, node 6, `Con` 4.90934140e-2 vs 4.90934180e-2 (inputs identical). These produce a one-digit `top_flux` print difference in one record of `model6_no_exchange` (parity still fails on that single record, `difference` 9.9999999999767e-05). *Hypothesis:* f32 rounding in the `SetMat` table path; not located.
- `model7_exchange`: water trace diverges at the first `DualPor` call after step 1 (`ItCum` 8, node 2, `SinkIm` Fortran 2.07488480e-4, Rust 0.0; `ThNewIm` 7.8708187e-2 vs 7.8701131e-2).
- `model6_exchange`: water trace matches (no difference beyond 1e-4 relative); its `Conc` divergence is solute-side.

## H1 result

H1 asked whether the second `set_mat` call (`water_compat.rs:1370-1411`) and/or the `hTemp` given to the first `set_mat` is the first divergence. Result: **established that both are real divergences from the pinned source, but neither alone, nor together, is sufficient.** The first divergence at HEAD is indeed the blended `Con` at node 2, but the blend is not the only difference there: the table (`l_table`) is also off.

Parity (pass counts out of 4 `dual_exchange` / 5 `nonequilibrium`; all statuses identical to HEAD for every variant):

| variant | Rust change (patch) | `dual_exchange` | `nonequilibrium` | status changes vs HEAD |
|---|---|---|---|---|
| A (HEAD) | none | 0/4 | 5/5 | – |
| B | second `set_mat` removed, `theta` routed to `th_eq` (`rust-variant-no2ndsetmat`) | 0/4 | 5/5 | none |
| C | B + `h_temp := solute v_corr` (`rust-variant-hTempAlias`) | 0/4 | 5/5 | none |
| D | C + `l_table = true` (`…B_alias_lTable`) | 0/4 | 5/5 | none |
| E | D + `lExtrap = true` | 0/4 | 5/5 | none |
| F | E + `lMinStep = true` | 0/4 | 5/5 | none |

- Variant B note: literally deleting the second call leaves `ThEq` unset (the first call writes `theta` into `ThNew`), so B also passes `th_eq` to the first call, which is what `WATFLOW.FOR:65` does. Variant C is the conditional "corrected `hTemp`" variant: the Fortran `hTemp` differs from Rust's (boundary nodes hold `vCorr`, see Root cause).
- Whole-set run for variant F (`--fixture-group all`): **35/44, the same 9 fixtures fail as at HEAD, no fixture changes status** (so reverting the `72d2544` second call together with the other four changes does not regress the fixture it fixed). First divergences move: `model6_no_exchange` `head` -> `top_flux` (1 record); `tortuosity_millington`, `tortuosity_moldrup_artificial`, `gas_henry_diffusion` `head` -> `cvTop` (1e-7 level); `temperature_coefficients` -> `sum_cvTop`; `vapor_enabled_external_state` -> `cvTop`; `model7_exchange` stays `head` (0.001-0.002); `model7_no_exchange` `head` -> `Sorb` (Fortran ~1e-34, Rust 0); `model6_exchange` `head` -> `Conc`.
- Variant B alone, the `all` run: not run (that is `21.1.1`'s scope).

## Root-cause statement

*Established* (trace + ablation): the Rust water solver differs from the pinned Fortran for these decks in five places, all outside the solver equations. (1) `l_table` is hard-coded `false` (`compat_runner.rs:166`, `:1001`) although these decks give `lTable=.true.` (`INPUT.FOR:565-580`, `hTab1 hTabN = 0.0001 100`), so Rust evaluates analytic `FK/FC/FQ` where Fortran linearly interpolates a 100-point table; the interpolation error is largest at the wetting front (heads between table nodes) and is invisible for uniform -100 cm profiles (-100 is a table node). (2) `Update` is called with `lExtrap=false` (`compat_runner.rs:1463`) but Fortran sets `lExtrap=.true.` in `Init` (`INPUT.FOR:1712`), which extrapolates the initial `hNew` of the next step (`WATFLOW.FOR:1097-1099`); it vanishes for uniform heads. (3) `lMinStep` starts `false` (`compat_runner.rs:896`) but `Init` sets `.true.` (`INPUT.FOR:1584`), so the first `TmCont` clamps `dtOpt` to `dtMaxC` (`TIME.FOR:8-12`); Rust keeps `dtOpt` at 0.1, giving a different step sequence once solute `dtMaxC` is finite (wetting front). (4) The extra second `set_mat` call (`water_compat.rs:1370-1411`, commit `72d2544`) removes the `0.1*hTemp+0.9*hNew` blend of `Con`, `Cap` and `ThEq`; the `iHyst.eq.3` branch it cites is unreachable (`iHyst=0`, `WATFLOW.FOR:63`). (5) `HYDRUS.FOR:508-511` passes `hTemp` as the `vCorr` dummy of `Solute` (`SOLUTE.FOR:11`); `SOLUTE.FOR:741` overwrites every element each solute step (0.0 here, `Dg=0`), and `Update` (`WATFLOW.FOR:1092-1105`) only restores non-Dirichlet nodes, so with `KodBot=KodTop=1` `hTemp(1)` and `hTemp(NumNP)` enter the next `SetMat` as 0 (treated as saturated in the blend). Rust keeps `v_corr` in a separate array. Evidence: `H`/`S`/`C`/`T` records above, plus the Fortran probe that printed `hTemp(1)=hTemp(NumNP)=0` at `WatFlow` entry (`E` probe, since removed) and the ablation set. With all five Rust changes, the wetting-front water trace reproduces Fortran (482 of 482 iteration headers) and the `model6` exchange trace for zero and non-zero coefficients, so **`HYSTER.FOR` is not involved** (unreachable, all 44 decks `iHyst=0`).

*Hypothesis:* the residual 1-ulp `Con` differences (table path rounding) cause the single remaining `top_flux` print difference in `model6_no_exchange`; `model7_exchange` fails on the `iDualPor=2` branch of `DualPor` (`WATFLOW.FOR:1043-1056`; Rust `dual_domain.rs:107`, whose `if let Ok(exchange)` at `water_compat.rs:1303` silently returns zero on error); `model6_exchange`/`model7_no_exchange` residuals are solute-side (not water). The uniform-head `transport_properties` decks (21.1.1) likely share causes (1), (4) and (5) at the boundary nodes; the `cvTop` first divergences after variant F suggest they are close, but this is untraced here.

## Suspected source defects

- **SD candidate 1 (established as behaviour, intent hypothesis):** `HYDRUS.FOR:508-511` passes the water-flow Picard array `hTemp` to `Solute` in the `vCorr` position (`SOLUTE.FOR:11`, assigned at `:741` for all nodes). Solute therefore overwrites `hTemp`; `Update` restores only the interior nodes when `KodBot>0` and `KodTop>0`, so boundary nodes keep `vCorr` into the next time step's first `SetMat` blend. Rust must emulate this (diagnostic variant: `h_temp.copy_from_slice(&solute_st.v_corr)` after the solute step, `compat_runner.rs:~1200`). Before registering as `SD-NNN`, Wave 3 must check every other site that passes the same actual argument (`grep -n hTemp src_reference/HYDRUS.FOR`: lines 215, 225, 420, 511, 696 only) and any `Dg>0` deck, where `vCorr` is non-zero.
- No other candidate. `lExtrap`, `lMinStep` and `lTable` are Rust omissions (consistent pattern, not source defects).

## Fix outline

Outline for the Wave 2 gate only; nothing here is applied.

1. `compat_runner.rs:166`, `:1001` and `deck_reader.rs:515-530`: derive `l_table` as `INPUT.FOR:565-580` does (take `abs`, `hTab1=-min`, `hTabN=-max`, then the `> -0.00001`/equal fallback) and pass it to `set_mat` and the `ConVapor`/`init` call; test `compat_ltable_follows_deck_htab_selectors` plus a table-vs-analytic `set_mat` unit test at a non-node head.
2. `compat_runner.rs:1463`: `l_extrap` = `true`; test `update_extrapolates_head_like_pinned_lextrap` (`water_compat.rs` tests, expect -41.787 for the node-2 case).
3. `compat_runner.rs:896`: `l_min_step` starts `true`; test `tm_cont_first_call_clamps_dt_opt_to_dt_max_c`.
4. `water_compat.rs:1370-1411`: delete the second `set_mat`, pass `state.th_eq` as `theta` in the first call (`WATFLOW.FOR:65`); update the doc comment and the unit tests that relied on `hTemp=hNew`; regression check `nonequilibrium` 5/5 and the `all` group stays at least 35/44.
5. `compat_runner.rs:~1200`: after the solute step, set `h_temp` from the solver's final `v_corr` (SD candidate 1), tagged `// SD-NNN`, with `source_defect_sdNNN_htemp_aliased_as_vcorr` pinning test.
6. After 1-5: `model6_no_exchange` is one `top_flux` record from parity; trace the table-path 1-ulp difference (`Con`, `ItCum` 2, node 6) next, then `DualPor` `iDualPor=2` for `model7_exchange` and solute-side residuals for `model6_exchange` and `model7_no_exchange`.

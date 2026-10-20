# Water transient diagnosis: Model 0 isolating deck (task 21.1.3)

Diagnosis only. No fix is committed. The isolating deck lives under `.development/21.1.3/isolating_model0/` and is not part of the 44-fixture parity set.

## Run identity

- Base commit `e777ea2` (branch `orchestration/21.1.3`). Compiler: GNU Fortran 13.3.0, flags `-std=legacy -ffixed-line-length-none`.
- Canonical SHA-256 (unchanged): `src_reference/WATFLOW.FOR` `e5591cebae4cbb44e8fe07520e08fc98c1522531c67753c0cc03fe2e164b5ef9`; `src_reference/HYDRUS.FOR` `fca01114a773b0ec9d614d17ed493a008bac82f23d22a187a10a4d4f48fbf3a9`.
- Instrumented staged sources: `WATFLOW.FOR` `1221af71925b68d0a48c2ef3ea60bcd59d823685bd542fa25a5430d9be148ebe`; `HYDRUS.FOR` `c355a5d61f4390d3a631c6038fe2ae5aaf709a8cd3b00c1668d0c548fa63f32d`.
- Patches: `fortran-instrumentation.patch` `0d44127d1346898d339fe5ac24b50dd1b61c610c3fefbd5cb33c597868ffd8cb`; `rust-instrumentation.patch` `d4b31344bca2d9f8964b86d8004d01b2d3464d0b06351f1f6425b17b275b1707`; cumulative variant patches `variant_B1..B6_total.patch`, `variant_E_total.patch` (SHA-256 in the scratch hash list).
- Binaries: canonical-staged reference `bf6e3bce2a0d922cc02eee87212b84a9d74fda6f9c483ae79d6d1df191b8751f`; instrumented Fortran `22333f4283b016a1e2523e5f2fd2d8e79cbf3c6440d8b770858e8d8445c1201e`. Rust candidate runner: HEAD `9a161efa...16b2`, HEAD+trace `814be8a8...ad87`, B1 `7ad67a90...817a`, B2 `6b343e17...f2df`, B3 `96377de6...6508`, B4 `f62ca333...ead9`, B5 `5c50db14...61b1`, B6 `b923c01c...7e9b`, E `15e8c716...e43b` (each differs from HEAD).
- Deck SHA-256: `SELECTOR.IN` `e9057e3d3a91a6838c567998d2dbb7359bf039eda03ee38ec09b4a9a0597760d`; `PROFILE.DAT` `2e3cb7c5...0588` and `LEVEL_01.DIR` `eb4bd64f...b786` unchanged from `model6_no_exchange`.
- Commands: `python3 tools/run_solute_parity.py --backend fortran-compatible --fixture-manifest .development/21.1.3/isolating_model0/CASE.json --task-root .development/21.1.3/cmp_<variant>` (with `HYD_TRACE_PATH` for Rust traces); `python3 tools/stage_fortran_reference.py --output .development/21.1.3/staged`; `sh tools/build_fortran_reference.sh --build-dir .development/21.1.3/staged`. Env note: the tools need `.development/` inside the worktree (REPO_ROOT), so scratch lives at `<worktree>/.development/21.1.3` with `/home/andrew/AI/hyd.rs/.development/21.1.3` as a symlink to it, and `<worktree>/target` was a symlink to `CARGO_TARGET_DIR` (adapter looks in `<root>/target/debug`; removed afterwards).

## Instrumentation validation

- Instrumented Fortran (trace unit 99, writes only at loop boundaries; unit 77 is OBS_NODE and must not be reused): `T_LEVEL.OUT`, `BALANCE.OUT`, `NOD_INF.OUT`, `solute1.out`, `PROFILE.OUT`, `OBS_NODE.OUT`, `RUN_INF.OUT`, `I_CHECK.OUT` and stdout are identical to the un-instrumented reference run except physical header line 3 (wall-clock `Date/Time`).
- Instrumented Rust (env-gated append to `HYD_TRACE_PATH`): all outputs under `rust_candidate_run/run/` and `candidate_observations.json` are byte-identical to the un-instrumented candidate (`cmp`).
- Trace guards key on `ItCum` and `Iter`, never on `t`. Rust `T_LEVEL.OUT` is a one-line stub, so water comparison needs the trace.

## First divergence

Isolating deck: `model6_no_exchange` with `Model Hysteresis` `6 0` -> `0 0` and the material row trimmed to `0.08 0.43 0.015 1.875 0.43 0.5`. Model 0 gives `iDualPor = 0` (`INPUT.FOR:610-625`), so `DualPor`/`InitDualPor` are not reachable. `iHyst = 0`, `lTable = .true.` (`hTab1 hTabN = 0.0001 100`, `INPUT.FOR:567-580`), `lVapor = lWDep = f`.

**Result: the divergence reproduces in plain Model 0. `DualPor` is not required.** HEAD parity: `fail` on `head` (differences up to 0.23 cm at t = 1, 2, 4 h) and the Rust run takes 199 water iterations against Fortran's 482. Step-1 iterations: Fortran 8, Rust HEAD 9.

Node numbers are internal (node 1 = bottom, node 9 = top, held at head 0).

| Layer | TLevel/ItCum, Iter | Node | Variable | Fortran | Rust | Fortran `file:line` | Rust `file:line` |
|---|---|---|---|---|---|---|---|
| 1 (HEAD) | ItCum 2, Iter 2 (step 1) | 2 | `Con` | 1.1323802e-2 | 1.1313949e-2 | `WATFLOW.FOR:65` / `:470-476`, `:480-520` (`SetMat`, blended `hiM`, table branch) | `water_compat.rs:1370-1411` (second `set_mat`, no blend), `compat_runner.rs:166,1001` (`l_table: false`) |
| 2 | ItCum 9, Iter 1 (step 2 start) | 2..8 | `hTemp` (extrapolated start of step) | -4.1786972e+1 | -5.6558727e+1 | `WATFLOW.FOR:1097` (`Update`, `lExtrap`) | `compat_runner.rs:1463` (`false, // l_extrap`) |
| 3 | ItCum 9, Iter 1 | 1 | `Con`, `Cap`, `ThEq` | 1.2437832e-2 (= table value at hiM = -90) | 8.8456981e-3 (value at -100) | `HYDRUS.FOR:507-511` + `SOLUTE.FOR:30,741` (`vCorr` aliases `hTemp`, zeroes it) | `compat_runner.rs:1155` (separate `v_corr`, `h_temp` kept) |
| 4 | ItCum 16 (step 3 start) | n/a | `dt` | 3.4003951e-2 | 3.4170479e-2 | `TIME.FOR:8-11`, `HYDRUS.FOR:660` (`lMinStep` true at first `TmCont`) | `compat_runner.rs:896` (`l_min_step = false`) |

Layers 2 to 4 are masked by the layer before them and were found by applying cumulative variants (next section). Layer 1 is the first divergence. In HEAD, iteration 1 of step 1 matches to print precision in `hTemp`, `hNew`, `Con`, `ConO`, `Cap`, `ThNew`, `ThEq`, `rTop`/`vTop`, `EpsTh`; iteration 2 does not.

Sub-findings of layer 1 (all at ItCum 2):
- 1a. `wat_flow` calls `set_mat` twice. The second call (commit `72d2544`) passes `hNew` as `hTemp` and overwrites `Con`/`Cap`/`ThEq` without the 0.1/0.9 blend. Fortran evaluates `Con`/`Cap`/`ThEq` at the blended head. Its stated source (`call Hyst`, `WATFLOW.FOR:73-76`) is in the `iHyst = 3` branch (unreachable).
- 1b. The first `set_mat` call passes `state.th_new` where Fortran passes `ThEq` (`WATFLOW.FOR:65`, `SetMat` argument `theta`). `Reset` overwrites `ThNew` from `ThEq` (`WATFLOW.FOR:194`), so only `ThEq` is the real output.
- 1c. `l_table` is hard-coded `false` in both Rust `set_mat` call sites. Fortran uses interpolated tables (`lTable = .true.`): Fortran `Con` 1.13238e-2 matches table interpolation at hiM = -93.10 (1.13243e-2), while Rust returns the analytic value (1.10341e-2). The 35 passing fixtures were insensitive to this because their heads are uniform at a table node.

## H1 result

Deck-local only (per approval, no `--fixture-group all` sweep). Cumulative variants; "first divergence" is against the instrumented Fortran trace at relative tolerance 1e-5.

| Variant | Change (cumulative) | Water iterations (F = 482) | Step-1 iters (F = 8) | First divergence | Parity |
|---|---|---|---|---|---|
| HEAD | none | 199 | 9 | ItCum 2 Iter 2, node 2 `Con`/`hNew` | fail (`head`) |
| B1 | remove second `set_mat` (literal) | 197 | 9 | ItCum 2 Iter 2 (`ThEq` never written) | fail |
| B2 | B1 + first-call `theta` -> `th_eq` | 198 | 8 | ItCum 2 Iter 2 node 2 `Con` (no table) | fail |
| B3 | B2 + `l_table = true` | 198 | 8 | ItCum 9 Iter 1 (`hTemp`, extrapolation) | fail |
| B4 | B3 + `l_extrap = true` | 203 | 8 | ItCum 9 Iter 1 node 1 `Con` (`hTemp(1)`) | fail |
| B5 | B4 + `h_temp := v_corr` after Solute | 418 | 8 | ItCum 16 Iter 1 (`dt`) | fail |
| B6 | B5 + `l_min_step = true` | 482 | 8 | none above 1e-5 (f32 rounding noise from ItCum 6) | fail on `top_flux` only: one record differs by 1e-4 in the last printed digit; `head` passes |
| E | B6 but second `set_mat` restored | 248 | 9 | ItCum 2 Iter 2 node 2 `Con` | fail (`head`) |

E shows that, with every other fix in place, the second call alone still breaks parity, so removing it is necessary. The "`hTemp` passed to the first `set_mat` differs" variant is layer 3 (B5): Fortran `hTemp(1)` is 0 after `Solute`, so `hi1 = hSat`.

Regression risk: commit `72d2544` reportedly fixed one `reactions` fixture's equilibrium water content with the second call. Not measured here (no sweep). B3 to B6 also change step selection, so every fixture with a Dirichlet boundary node, an extrapolating step or a first `TmCont` call needs the Wave 4 sweep.

## Root-cause statement

*Established* (trace evidence above): the Rust compat water path diverges from the pinned Fortran in Model 0 with no dual porosity, through four independent, stacked differences, each shown by the first differing trace row once the previous one is corrected: (1) the extra unblended `set_mat` call and the wrong output array (`th_new` instead of `th_eq`) plus `l_table = false`; (2) `l_extrap = false` where Fortran defaults `lExtrap = .true.` (`INPUT.FOR:1712`); (3) Fortran's `Solute` call zeroes `hTemp` through the `vCorr` argument alias, which changes `SetMat`'s blend only at nodes `Update` does not rewrite (Dirichlet boundary nodes); (4) `lMinStep` is true at the first `TmCont` call in the pinned build (`TMPRE` trace), which sets `dtOpt` to the Courant limit, while Rust starts it false. With all four corrected, the Rust water trace reproduces all 482 Fortran iterations. *Hypothesis*: that `lMinStep` is uninitialised storage (it is declared at `HYDRUS.FOR:60`, never assigned when `TopInf`, `BotInf` and `AtmBC` are false and `SetBC` is not called, and reads true on both runs); proving the cause needs an uninitialised-read check not done here. *Hypothesis*: the remaining `top_flux` last-digit difference is f32 operation-order noise in `set_mat` or `gauss` (first visible at ItCum 6 as 2e-7 relative). The model-0 result implies the dual-porosity (`model6_no_exchange`) gap is shared with `WatFlow`; whether `DualPor` adds anything is left to 21.1.2's evidence.

## Suspected source defects

- SD candidate A: `HYDRUS.FOR:511` passes `hTemp` as `Solute`'s `vCorr` argument (`SOLUTE.FOR:11,30,741`), so `Solute` overwrites the water iterate `hTemp` with the gas-diffusion correction (0 here). Matters whenever a boundary node keeps a stale `hTemp`.
- SD candidate B: `lMinStep` is read at `TIME.FOR:8` without a reachable assignment when `SetBC` is not called. Reads true in the pinned build. Needs the Wave 3 check for a consistent pattern before registering.

## Fix outline

For the gate (not a change). Candidates:
- `water_compat.rs` `wat_flow` (about `:1344`, `:1370-1411`): first `set_mat` writes `th_eq`; delete the second `set_mat` call and its misattributed comment.
- `compat_runner.rs:166,1001`: pass the `lTable` value computed per `INPUT.FOR:567-580` (note `deck_reader.rs:517-527` applies the reset test to unsigned values; Fortran negates first, `INPUT.FOR:560-561`).
- `compat_runner.rs:1463`: `l_extrap = true` (default).
- `compat_runner.rs` after the solute step (about `:1205`): copy the solute `vCorr` into `h_temp`, tagged `// SD-NNN`; `compat_runner.rs:896`: initial `l_min_step = true` tagged `// SD-NNN`, each with a `source_defect_sdnnn_*` pinning test.
- Proposed regression test: `isolating_model0_water_trace_matches_fortran` using the 482-iteration trace values (task-local deck would need to be promoted by the gate).
- Expect Wave 4 re-measurement for all 44 fixtures; the `72d2544` reactions fixture is the named risk.

## Isolating deck

Diff (`isolating_deck.diff`, SHA-256 `a2da48e01721856726dc39b59c7072ae76c7a213c6291af2c55a63e079cc63bb`):
- `SELECTOR.IN`: `6 0` -> `0 0`; `0.08 0.43 0.015 1.875 0.43 0.5 0.05 0.25 0.0` -> `0.08 0.43 0.015 1.875 0.43 0.5` (Model 0 reads `NPar = 6`, `INPUT.FOR:609-635`).
- `ChemIn` (`INPUT.FOR:1318-1335`) needs no edit: it does not depend on `iModel`; the deck already has `iNonEqul = 0`, `lDualNEq = f`, `Fraction = 1`, `Immobile = 0`.
- `CASE.json` (task-local): `input_files.SELECTOR.IN` hash, `dual_domain` `water_model 0`/`iDualPor 0`, `branch_coverage` `water_model 0`/`mode single`, `request_sha256` recomputed with `canonical_hash`; the file is re-serialised with sorted keys.
- `OBSERVATION_MAP.json` (task-local): the `dual_immobile_water_volume` observation is removed because `BALANCE.OUT` has no `W-volumeI[L]` under Model 0.

Fortran run identity: the fresh un-instrumented reference run is `cmp_head/reference-run` (executable `bf6e3bce...751e`). Comparison method: `tools/run_solute_parity.py --backend fortran-compatible --fixture-manifest` accepted the task-local manifest unchanged, with the same normalizers. `reference/` is untouched (`git status --short reference` is empty).

# Water transient diagnosis: `transport_properties/tortuosity_millington` (single porosity, long run)

Diagnosis only. No production code was changed. Scratch, traces, patches and variant logs are under `.development/21.1.1/` (untracked).

## Run identity

- Commit: `e777ea289570cbfb8171a2a52a3a3e1dfae5c313` (branch `orchestration/21.1.1`). Fixture group baseline: 35 of 44 pass.
- Fixture `reference/fortran_reference/solute/parity/transport_properties/tortuosity_millington/` SHA-256: `CASE.json` `35e959d397f72300af40b0a2260898ad44a0161dc9db3f4324aa476fb98f235d`; `SELECTOR.IN` `635858e34fe63453957846c058e4038dc521388a7fc4e9178c84b5f543871ee6`; `PROFILE.DAT` `7e1adeb6646ce7448dea24d60e7d0b5a6c938ecd20d53e53c9847d6742fbc228`.
- Deck selectors: `iModel 0`, `iHyst 0`, `hTab1 hTabN = 0.0001 100`, `lVapor f`, `lTemp f`, `lWDep f`, `KodTop = KodBot = 1`, `dt/dtMin/dtMax = 0.1/0.01/0.5`, `tMax = 200`, uniform head -100, 1001 nodes, 2000 time steps.
- Canonical sources SHA-256: `src_reference/WATFLOW.FOR` `e5591cebae4cbb44e8fe07520e08fc98c1522531c67753c0cc03fe2e164b5ef9`; `HYDRUS.FOR` `fca01114a773b0ec9d614d17ed493a008bac82f23d22a187a10a4d4f48fbf3a9`; `TIME.FOR` `8b6af595a1ccb3952cfa7b1eb56c0e37a1cfc6b8d55e26d8907288c41f08b92f`.
- Instrumented sources (staged copy only): `WATFLOW.FOR` `5122ef2bed351a2e4e574b1c9728187fa463fab43cbb6f527e9c83de116ef55c`; `HYDRUS.FOR` `54cdbb7216e83e700288512f9757ab581fdaacdc54e89a7ab2deffbbe36e8039`. Patch `fortran-instrumentation.patch` `dd65904c08bf5f45a7c9bdaea115cf54f9de86b3be12da667d988e47b9a972fc`.
- Binaries: canonical Fortran `70f582c37f2f1628bfba2b6787d74a1dca611373d2fdb4ef96515ba011ad9abc` (GNU Fortran 13.3.0); instrumented Fortran `e6a379ec7391a77bed032998bf1b48dea0a59840c007f8b86623b94f0f2282c2`.
- Rust patch `rust-instrumentation.patch` `6d771337cc7eb687de89ee573a68037ed039dc88df9042f9207423b81dcee0c1`. It contains the trace code plus the fix prototype of the last variant (see H1 result). Variant generator `mkvariant.py` `18e9849a342336e192c5d7c654913da5e163c50bdda17320f05b87553c35570f`; comparator `cmp_trace.py` `230c65e1a2ed09509d75dc157d03dcd0d4eaffda5a5179e2e7355291fbbac339`.
- Rust candidate binary SHA-256 prefixes (full values in `runner_<variant>.sha`): plain HEAD `441bc409ec21`; head+trace `9650ebf109ec`; per variant see the H1 table (all distinct).
- Commands:
  - `python3 tools/run_fortran_reference.py --fixture-manifest <CASE.json> --task-root .development/21.1.1/canon`
  - `python3 tools/stage_fortran_reference.py --output .development/21.1.1/fi`, patch the staged `WATFLOW.FOR`/`HYDRUS.FOR`, then `sh tools/build_fortran_reference.sh --build-dir .development/21.1.1/fi`, run in `fi_run/` as `../fi/hydrus-reference solute-input`.
  - Rust: `HYD_TRACE=1 target/debug/solute_compat_deck_runner <CASE.json> <outdir> 2> trace`.
  - Per variant: `python3 .development/21.1.1/mkvariant.py <v>`, `cargo test -p hydrus-core --test solute_fortran_compat_harness --no-run`, then `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group all --backend fortran-compatible --task-root .development/21.1.1/h1_<v>`. The adapter caches its runner and does not rebuild the library, so the cached binary was deleted and re-linked per variant.
- Trace guard: counter of `WatFlow` entries (`call`), `Iter`, `ItCum`, node. Full node rows for calls <= 200, nodes 1, 2, 501, 1000, 1001 afterwards (1,052,000 rows per side). Node numbers are internal (1 = bottom).

## Instrumentation validation

- Instrumented Fortran vs canonical run: `T_LEVEL.OUT`, `BALANCE.OUT`, `NOD_INF.OUT`, `solute1.out` (and `solute2/3.out`, `PROFILE.OUT`, `OBS_NODE.OUT`, `RUN_INF.OUT`, `I_CHECK.OUT`) are byte-identical except the wall-clock `Date/Time` header (line 3) and `Calculation time` in `BALANCE.OUT`, which differ between any two runs. Units 91 and 92 were used for traces (units 77 and 78 are taken by the program).
- Instrumented Rust vs un-instrumented Rust (HEAD): `diff -r` of all output files is empty.
- Reference hashes: `T_LEVEL.OUT` `b3624fff56ae...`, `BALANCE.OUT` `ea81206dd812...`, `NOD_INF.OUT` `188835f31899...`, `solute1.out` `55ecbefb08c1...` (canonical run; prefixes).

## First divergence

At HEAD the first row to differ beyond print precision is the second `WatFlow` call (time step 2), `Iter 0`, `ItCum 2`. Call 1 matches in every row.

| TLevel (call) | Iter | node | variable | Fortran | Rust | Fortran file:line | Rust file:line |
|---|---|---|---|---|---|---|---|
| 2 | 0 | 1 and 1001 (Dirichlet nodes) | `hTemp` | 0.0 | -100.0 | `SOLUTE.FOR:741` writes `vCorr`, aliased to `hTemp` by `HYDRUS.FOR:510-511`; `WATFLOW.FOR:1091-1100` (`Update`) skips Dirichlet nodes | `compat_runner.rs:1456` (`update`, no aliasing); `water_compat.rs:1113` |
| 2 | 0 | 1 and 1001 | `Con` / `Cap` / `ThEq` | 0.012437832 / 0.0013521235 / 0.29829249 | 0.0088456981 / 0.0012236567 / 0.28523183 | `WATFLOW.FOR:470-476` (blend `hiM`), `:482-484`, `:515-518` | `water_compat.rs:~540-580` (`set_mat`) |

Inside call 1 there is no difference. The zeroing happens between calls (`HYTRACE.OUT`: `hTemp(1)`, `hTemp(2)`, `hTemp(NumNP)` are -100 before `WatFlow` of step 1 and 0.0 after `Solute`; `Update` restores only interior nodes). Fortran `Con(1) = K(0.1*0 + 0.9*(-100) = -90)`; Rust uses -100.

After correcting that cause, the next divergences found, in order, were:

| Order | Where | Evidence | Fortran | Rust |
|---|---|---|---|---|
| 2 | Call 2, iter 1, node 2, `Con`/`Cap`/`ThEq` (inputs equal, `hiM = -100.0296`) | Rust table range is -0.01..-1e4, Fortran -1e-4..-100 (`T` rows) | `Con 0.0088374745` (direct `FK`) | `0.0088393455` (interpolated) |
| 3 | Call 2 end to call 3 start, node 2, `hNew` | Fortran `Update` extrapolates: -100.0005 becomes -100.00101 | -100.00101 | -100.000504 |
| 4 | Call 2, iter 1, nodes 975-979, `Con` rel 1e-6 | `hiM` in the last table interval; `iT = NTab(1) = 100` makes Fortran read `hTab(101,1) = 0`, `ConTab(101,1) = 0` (`X` rows: `dh = 7.6e-8`, `Coni = 0.88456972e-2`) | `0.0088456962` | `0.0088457055` |

## H1 result

Rust variants (parity group `all`, `fortran-compatible`, 44 fixtures). "alias" zeroes `h_temp` after the solute step (emulates `vCorr` aliasing, valid when `vCorr = 0`, which holds here: `Dg = 0`). "table" is `l_table = true` (`compat_runner.rs:166,1001`). "tfix" adds the `INPUT.FOR:570-571` sign/abs transform to `deck_reader.rs:517-518` (hTab range). "faith" removes the second `set_mat` and passes `th_eq` to the first. "ext" is `l_extrap = true` (`compat_runner.rs:1469`). "oob" emulates the Fortran read of table row `NTab+1` as zeros. "minstep" is `l_min_step = true` (`compat_runner.rs:896`).

| Variant | Candidate binary | First divergence (trace) | Parity pass | Differs from HEAD |
|---|---|---|---|---|
| HEAD | `8c285710fc79` | call 2, iter 0, node 1/1001 `hTemp` (0.0 vs -100) | 35/44 | none |
| second `set_mat` removed (literal; `ThEq` then stale) | `540e3b3a45c9` | same point | 35/44 | none |
| alias (hTemp corrected) | `c0e38f98e930` | call 2, iter 0, `Con` (0.012437832 vs 0.0122205773) | 35/44 | none |
| table only | `562b4d0cf60d` | same as HEAD | 35/44 | none |
| alias + table | `861a7792d1d0` | call 2 iter 0 `ThEq` (stale; two-call structure) | 35/44 | none |
| faith (alias, no second call, `th_eq`) + table | `deeaf1669b02` | call 2, iter 1, node 2, `Con` rel 2e-4 (table range) | 35/44 | none |
| faith + tfix | `c6185fb9d2f0` | call 2, iter 1, nodes 975+, `Con` rel 1e-6; `vTop` rel 2.6e-4 at call 3 | 35/44 | none |
| ext only | `5450348f52d2` | same as HEAD | 35/44 | none |
| faith + tfix + ext | `b95e05a56594` | call 2, iter 1, nodes 975+ rel 1e-6; `vTop` rel 1.2e-4 at call 40 | 35/44 | none |
| alias + tfix + ext + oob, second `set_mat` kept | `1067bf2ed8b1` | call 2, iter 1, node 1 `Con` (0.012437832 vs 0.00884569809) | 35/44 | none |
| faith + tfix + ext + oob | `17284ad0ae0d` | none: all 1,052,000 rows within rel 5e-8 (print precision) | 38/44 | +`tortuosity_millington`, +`tortuosity_moldrup_artificial`, +`gas_henry_diffusion` |
| faith + tfix + ext + oob + minstep | `8185600c6b2e` | none | 39/44 | the three above, +`model6_no_exchange`; still failing: `model6_exchange`, `model7_exchange`, `model7_no_exchange`, `temperature_coefficients`, `vapor_enabled_external_state` |

- No variant regresses any of the 35 passing fixtures. The `72d2544` "equilibrium water content" fixture was not among the changes in any variant, so removing the second `set_mat` (together with the other corrections) put no fixture at risk in this experiment.
- H1 (the second `set_mat` is the first divergence) is refuted for this deck: it is not the first divergence and removing it alone changes nothing. It is, however, structurally wrong: with all other causes fixed and the second call kept, `Con`/`Cap`/`ThNew` after the call differ from Fortran at the very first iteration (`G` rows call 2 iter 1 node 1, `Con` 0.012437832 vs 0.00884569809) and the fixture still fails.
- `l_min_step` has no effect on this deck (trace identical, parity for `tortuosity_millington` unchanged); it fixes `model6_no_exchange` (outside this task's scope, noted for the other diagnosis).

## Root-cause statement

*Established* (trace evidence above): `tortuosity_millington` fails because of four stacked differences, not one. (1) **First divergence:** in Fortran `hTemp` is passed as the `vCorr` workspace to `Solute` (`HYDRUS.FOR:510-511`, `SOLUTE.FOR:6-12,741`); `Solute` overwrites it with zeros (here `Dg = 0`), and `Update` (`WATFLOW.FOR:1091-1100`) restores only non-Dirichlet nodes, so `hTemp(1)` and `hTemp(NumNP)` stay 0 from step 2 onward and the `hiM = 0.1*hi1 + 0.9*hi2` blend at `WATFLOW.FOR:470-476` evaluates `Con`, `Cap`, `ThEq` at -90 for the two boundary nodes; Rust keeps -100. (2) Rust never uses the conductivity/capacity tables (`l_table` hard-coded false at `compat_runner.rs:166,1001`), and the deck reader does not apply `INPUT.FOR:570-571`, so it also builds the table over -0.01..-1e4 instead of -1e-4..-100 (`deck_reader.rs:517-527`); at the pinned `hiM` values this changes `Con` by up to 2e-4 relative. (3) `l_extrap` is hard-coded false (`compat_runner.rs:1469`); Fortran sets `lExtrap = .true.` (`INPUT.FOR:1712`), so `Update` extrapolates `hNew = 2*hNew - hOld` at interior nodes (changes `hNew(2)` by 5e-4 per step). (4) Fortran reads one row past the table (`iT = NTab(1)`) when `hiM` lies within ~1e-6 of `hTab(NTab)` (zeros: `hTab(101,1) = ConTab(101,1) = 0`), shifting `Con` by ~2e-9; Rust clamps `iT` to `NTab-2`. Correcting all four, plus removing the second `set_mat` (which overwrites `Con`/`Cap`/`ThEq` with unblended values at `hNew`, not in Fortran) and passing `th_eq` to the first call, makes the whole 1,052,000-row trace agree to print precision and `tortuosity_millington` passes. Each cause was established by an individual variant or trace row; the attribution to individual fixture outcomes (only the combination changes parity) is established by the table above.

*Hypothesis:* for the other failures in the group, the same four causes apply wherever `vCorr` is zero. Where gas diffusion is active (`Dg > 0`) the alias writes nonzero `vCorr` into `hTemp`; the zero emulation used here is not exact for those decks (it still passes `gas_henry_diffusion`, which suggests `Dg*derK` is negligible, but that was not verified).

**Reachability (ADR 0007/0010):** `iHyst = 0` so `WATFLOW.FOR:63-69` (SetMat) runs; the `Hyst` call at `:73-76` and `Hyster` (`iHyst > 0`) are unreachable. `iModel 0 < nTabMod`, `lTable` true so the table branches at `:480-486` and `:512-519` are reachable. `lWTDep f`, `lVapor f` skip the `AT/BT` and vapor branches. `KodTop = KodBot = 1` makes the `Update` skip (`:1092,1094`) reachable. `lExtrap = .true.` is set unconditionally at `INPUT.FOR:1712`. `WATFLOW.FOR:116` (`c  if(TauW.gt.0) ...`) is a comment (column 1) and not a branch.

## Suspected source defects

- **SD candidate A (aliasing):** `HYDRUS.FOR:510-511` passes `hTemp` where `Solute` declares `vCorr` (`SOLUTE.FOR:30,741`). The solver then clobbers its own iterate for Dirichlet nodes. Reproduced (`HYTRACE.OUT` rows `P`/`Q`/`U`, `hTemp(1) = hTemp(NumNP) = 0`). Other sites computing `vCorr` (`SOLUTE.FOR:130-131,742-743`) use it consistently as a velocity correction, so the defect is the actual-argument choice. Pinning test would assert `hTemp` boundary nodes equal `vCorr` after a solute step.
- **SD candidate B (table overrun):** `WATFLOW.FOR:482-484` and `:515-518` (and `:497`, `:533`) have no bound on `iT`, so `iT = NTab(1)` reads `hTab/ConTab/CapTab/TheTab(NTab+1, 1)` (zero here). Reproduced (`X` rows). Consistent with the table branch in the `iModel.eq.nTabMod` path, which loops to `NTab-1`.

## Fix outline

An outline for the gate; nothing here is applied.

1. `compat_runner.rs:~1456-1472`: after the solute step each time step, copy the solute state's `v_corr` (`driver.rs:1431`) into `h_temp` (`SOLUTE.FOR:741`), before `update(...)`. Test: `source_defect_sdNNN_htemp_aliases_vcorr_on_dirichlet_nodes`.
2. `deck_reader.rs:517-527`: apply `hTab1 = -min(|hTab1|,|hTabN|)`, `hTabN = -max(...)` before the reset test, and carry `l_table` (INPUT.FOR:569-580) into the deck struct; pass it at `compat_runner.rs:166,1001` instead of `false`. Test: `deck_reader_derives_table_range_and_ltable_from_input_for_570_580`.
3. `compat_runner.rs:1469`: `l_extrap = true` (`INPUT.FOR:1712`). Test: `update_extrapolates_head_as_fortran_lextrap`.
4. `water_compat.rs:~1370-1411`: delete the second `set_mat` call and pass `state.th_eq` as `theta` at `:1359`; revisit the comment citing `WATFLOW.FOR:73-76` (unreachable). Test: `wat_flow_uses_single_blended_set_mat_for_ihyst0`.
5. `water_compat.rs:~540-580` (`set_mat` table branches): reproduce the `iT = NTab(1)` out-of-range read as zeros for `hTab/ConTab/CapTab/TheTab(NTab+1)`, tagged `// SD-NNN`, pinned by `source_defect_sdNNN_set_mat_reads_past_table`.
6. Separately (other diagnosis, noted): `compat_runner.rs:896` `l_min_step = true` (`INPUT.FOR:1584`) is needed for `model6_no_exchange`.
7. Remaining failures after 1-6 in this experiment (`model6_exchange`, `model7_*`, `temperature_coefficients`, `vapor_enabled_external_state`) are outside this fixture and were not traced.

# M6 vapor: scalar isothermal ConVh helper reachability audit (27.1.2)

Read-only audit. No code, fixture, or Fortran change. Search log: `.development/27.1.2/grep_log.txt`.

## Disposition

reachable

The scalar helper is reached by every `run_compat_deck` call, but its return value is discarded at `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs:281`. The call is output-neutral. Remove-versus-guard is a user decision and is not made here.

## Anchor correction

The spec labels `src_reference/WATFLOW.FOR:968-995` as the array routine. It is the scalar function:

- `WATFLOW.FOR:968` `real function ConVh(hNew,Theta,ths,xConv,tConv)`, with `Temp=20.` at `:985`.
- The array routine is `subroutine ConVapor` at `WATFLOW.FOR:849-929`. Its `ConVh(i)` loop is at about `:896-919`.

The orchestrator accepted this correction.

## Locations

| Item | Location | Notes |
|---|---|---|
| Fortran scalar `ConVh` | `src_reference/WATFLOW.FOR:968-995` | `Temp=20.` at `:985` |
| Fortran array routine `ConVapor` | `src_reference/WATFLOW.FOR:849-929` | Not the helper |
| Rust scalar helper `con_vh` | `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs:196-217` | `temp = 20.0f32` at `:202`; `pub(crate)` |
| Rust sole call site | `water_compat.rs:281` | `let _conv = con_vh(h_tab[i], the_tab[m][i], ths[m], x_conv, t_conv);` inside `gen_mat` (`:238-293`) |
| Fortran scalar call site | `src_reference/INPUT.FOR:804` | `ConV=ConVh(hTab(i,1),TheTab(i,M),ths(M),xConv,tConv)` inside `GenMat` for `iModel.lt.nTabMod` |

## Callers

### Fortran scalar `ConVh`

- The only scalar call is `INPUT.FOR:804`.
- `ConV` is written only to unit 50 at `INPUT.FOR:806-808` (`write(50,130,...)`). It feeds no table.
- The listed anchors `WATFLOW.FOR:14,31,68,83,88,178,187,211` are argument lists, declarations, or comments that name the array `ConVh(NumNP)`. They are not calls to the scalar. The real array uses are at `:211, 247-248, 277, 406, 736, 756-770`.

### Rust `con_vh`

| Caller | Path | Reached by supported input? |
|---|---|---|
| `gen_mat` (`water_compat.rs:238-293`), line `:281` | `lib.rs:34` (`pub use ... run_compat_deck`) → `compat_runner.rs:112` (`gen_mat(...)`) | Yes. `gen_mat` is not gated on `l_vapor` or any vapor flag, so every compat deck reaches it. The value is discarded. |
| Unit tests calling `gen_mat` | `water_compat.rs:1612, 1704, 1834` | Test only |

`con_vh` is a pure function with no side effects. Its result is bound to `_conv` and never used. No output field depends on it.

## Output neutrality

- The vapor path uses its own inline `con_vh` in `crates/hydrus-core/src/heat/vapor/kernels.rs:52-53`. It does not call the helper.
- `compat_runner.rs:1651-1704` reads `st.con_vh`, which is the array buffer from `water_compat.rs` `con_vh_out` (the array routine path), not the scalar helper.
- Conclusion: removing the `:281` call would not change any output. The `:281` call is dead computation.

## Search log

Commands (full output in `.development/27.1.2/grep_log.txt`):

1. `grep -rniE '20\.0|293\.15|isothermal|con_vh|ConVh|con_vh_scalar' crates/hydrus-core/src/ tools/`
   - `con_vh` hits: `water_compat.rs:196, 215, 216, 281`, plus `con_vh_out` and `con_vh_in` array names.
   - `isothermal` hits: `water_compat.rs:195` (doc comment), `compat_runner.rs:1913` (test name), `tools/run_vapor_reference.py:184`, `tools/test_run_vapor_reference.py:113`, `tools/test_compare_vapor_reference.py:349`. None is the scalar helper.
   - `20.0` hits: many unrelated test and temperature literals in `solute*` modules. The only helper-related literal is `water_compat.rs:202`.
   - `293.15` hits: `TR`/reference-temperature constants in `solute_fortran_compat/` (`initialization.rs:13`, `biology.rs:49`, `driver.rs:60`, `processes.rs:27`, `boundary.rs:11`). Unrelated to `ConVh`.
   - `con_vh_scalar`: no hits.
   - `tools/`: no hit names the helper.
2. `grep -n "ConVh(" src_reference/*.FOR` → `INPUT.FOR:804` (scalar call). The rest are declarations or comments.
3. `grep -rn "gen_mat(" crates/hydrus-core/src` → `compat_runner.rs:112`, `water_compat.rs:1612, 1704, 1834`.
4. `grep -rn "run_compat_deck" crates/ tools/` → `lib.rs:34`, test uses in `compat_runner.rs` and `tests/solute_fortran_compat_harness.rs`.

## 27.3.1 candidate file set (requires user approval before any edit)

- `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs`: the `:281` call and, for removal, the fn at `:195-217`. Tests at `:1612, 1704, 1834` do not call `con_vh` directly.

No other file is required. Whether to remove or guard the call, and whether to keep the function, is a user decision.

## Constraints honored

- Read-only audit. No Rust, Fortran, fixture, or tool edit.
- No cargo or test run. No commit.
- Writes limited to this doc and `.development/27.1.2/`.

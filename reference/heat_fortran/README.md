# Pinned-Fortran heat decks and fixtures (24.4.3)

These decks and fixtures are **source-matched comparison targets** made from the
pinned GNU Fortran build (`src_reference/`, patch `0001-gfortran-portability.patch`).
They make **no physical claim**. The input parameter values are chosen inputs, the
input manual is not in the repository, and no numeric tolerance is written here or
into any fixture or manifest. Tolerances belong to the contract (`M6_HEAT_CONTRACT.md`,
`## Tolerances`).

## Build provenance

| Item | Value |
|---|---|
| Executable `hydrus-reference` sha256 | `5a3734beefe82bf2f0d6a3e4f94a0aa2f8ce0e3edc6c298cf22f448cb234ab25` |
| `build-manifest.json` sha256 | `94bf5b96c9ffd9d0d92740a33a48309dafb4a84fa6ff5a1f6ecf6c7dadf27cb` |
| Portability patch sha256 | `622d1ea68285257e98a169f94dec83f3df1c671a9366f651e845806cb8c00d7e` |
| Archive `planning/H1D_Src.zip` sha256 | `d0c47cbcb7a298ab16c6e08010bd7ca048d5527ce2f989a749390c0c55e848cf` |
| Compiler | GNU Fortran 13.3.0 (Ubuntu 13.3.0-6ubuntu2~24.04.1) |
| Build command | `gfortran -std=legacy -ffixed-line-length-none src_reference/*.FOR gnu_compatibility.FOR -o hydrus-reference` |

Staged and built with `tools/stage_fortran_reference.py` and
`tools/build_fortran_reference.sh --build-dir .development/24.4.3/reference-build`
(untracked; `.development` is a worktree symlink, see Deviations).

## Input layout (SELECTOR.IN, `Pcp_File_Version=4`)

Record order as read by the pinned source:

1. `BasInf` flags, line 10: `lWat lChem lTemp SinkF lRoot ShortO lWDep lScreen AtmBC lEquil`
   (`INPUT.FOR:31`). Every deck: `t f t f f f f f f f`.
2. `BasInf` iVer 4 flags, line 12: `lSnow lDummy lMeteo lVapor lActRSU lFlux`
   (`INPUT.FOR:38`). Every deck: `f f f f f f`.
3. Water block (`TopInf`, `KodTop`, `BotInf`, `KodBot`, model, materials), copied from
   `reference/fortran_reference/solute/example3/` (already accepted by the pinned build).
4. Time block `TmIn` (`HYDRUS.FOR:236`): `tInit tMax` (`0 24`), `TPrint` (`6 12 24`).
5. Heat block `TempIn` (`INPUT.FOR:1205`, read at `HYDRUS.FOR:264-265` under `if(lTemp)`):
   - two skip records (`INPUT.FOR:1213-1214`), then one NMat row of nine values
     `Beta Qn Qo B1 B2 B3 Cn Co Cw` (`:1215-1216`);
   - one skip record (`:1218`), then `Ampl tPeriod iCampbell SnowMF lSnowInput`
     (iVer 4 branch, `:1225`). `lSnowInput` is `f`, so no extra skip records;
   - one skip record (`:1235`), then `kTopT tT kBotT tB` (`:1236`).

The `NObs` count line (`INPUT.FOR:355`) ends the profile, not the selector.

## Boundary codes (source-pinned; integer values not from a manual)

`kTopT`, `kBotT` are read at `INPUT.FOR:1236` and tested in `TEMPER.FOR`:

| Code | Behaviour | Test line (TEMPER.FOR) | Branch |
|---|---|---|---|
| `kTopT > 0` | Dirichlet: `F(N)=tTopA` | `:144` | `B(N)=0, D(N)=1` |
| `kTopT < 0` | Flux: `F(N)=F(N)-HeatFl` | `:148`; `HeatFl` set at `:25-27` | flux row |
| `kTopT == 0` | no branch: row N unassigned | `:144`, `:148` | rejected (contract) |
| `kBotT > 0` | Dirichlet: `F(1)=tBot` | `:87` | `D(1)=1, E(1)=0` |
| `kBotT < 0` | Flux: `tBot*Cw*(vNew+vOld)/2` in RHS | `:91` | flux row |
| `kBotT == 0` | zero gradient: `D(1)=-1, E(1)=1, F(1)=0` | `:108-111` (`else`) | `T(1)=T(2)` |

`tTop=tT` only when `TopInf` is false (`INPUT.FOR:1237`); the decks use `f`.
`tBot=tB` only when `BotInf` is false (`:1238`); the decks use `f`.
`tPeriod` is `0` in every deck (`tPeriod>0` is rejected by the contract; `TEMPER.FOR:21`).

## Reachability (ADR 0007)

- `Temper` is called at `HYDRUS.FOR:491` under `if(lTemp)` (`:490`), after the water
  step gated by `lWat` (`:473`).
- Upper flux branch: `TEMPER.FOR:25` (`kTopT.lt.0.and..not.lEnBal`), with
  `lEnBal` false.
- `SetUpHeat` is called at `TEMPER.FOR:43` for both levels (`:35-45` loop).
- Campbell branch: `TEMPER.FOR:287` (`iCampbell.eq.1`), reached through `TempCap`
  at `:37` and `:40`.
- Fixed-form check: `TEMPER.FOR` comments are `*` in column 1 (`:1,3,24,47,52,58`).
  Continuations use `!` in column 6 (for example `:22`, `:31`); no branch is hidden
  in a comment.

## Campbell TPar naming overlap

The default and Campbell branches read the same TPar rows with different meanings:

- The Fortran header (`INPUT.FOR:1249`) names rows 4-6 `B1 B2 B3`. The default branch
  (`TEMPER.FOR:286`) uses them as `B1 + B2*th + B3*sqrt(th)`.
- The Campbell branch (`TEMPER.FOR:288-291` comments, `:292-299`) reads the same rows
  as the volume fractions of quartz, other minerals and clay (`TPar(4)`, `TPar(5)`,
  `TPar(6)`), with `TPar(1)` the solids fraction.
- The Rust port names the same nine rows in `heat/capacity.rs` `TempMaterial::from_tpar`
  (lines 32-44): `solid_fraction`, `organic_fraction`, `dispersivity`, `quartz`,
  `other_minerals`, `clay`, `solid_heat_capacity`, `organic_heat_capacity`,
  `water_heat_capacity`. The row positions match the Fortran order, but the Fortran
  header names rows 2-3 `Qn Qo` (the Rust names are inferred from usage).
- Each deck uses one row, so both branches read identical numbers. The row is
  `0.5 0.2 0.0 0.3 0.5 0.1 1.0 1.0 1.0`.

## Decks and fixtures

| Case id | `kTopT` | `tT` | `kBotT` | `tB` | `iCampbell` | Kinds covered | Fixture |
|---|---|---|---|---|---|---|---|
| `heat_dir_top_dir_bot_default` | 1 | 25 | 1 | 5 | 0 | top>0, bottom>0, default | `fixtures/heat_dir_top_dir_bot_default.json` |
| `heat_dir_top_flux_bot_default` | 1 | 25 | -1 | 5 | 0 | bottom<0 | `fixtures/heat_dir_top_flux_bot_default.json` |
| `heat_flux_top_dir_bot_campbell` | -1 | 25 | 1 | 5 | 1 | top<0, bottom>0, Campbell | `fixtures/heat_flux_top_dir_bot_campbell.json` |
| `heat_flux_top_zero_bot_campbell` | -1 | 25 | 0 | 0 | 1 | top<0, bottom==0, Campbell | **no fixture** (refused; see below) |

All decks: 41 nodes, depth 0 to -40 cm (dx = 1 cm), `LUnit=cm`, `TUnit=hours`,
`MUnit=mmol`, `dt=0.1 dtMin=0.01 dtMax=0.5 DMul=1.2 DMul2=0.5 ItMin=1 ItMax=20 MPL=3`.
`dtMin=0.01` is the example3 value (the plan said 0.001; accepted by the coordinator).
Initial head is `-10` at every node, initial temperature `15.0`.

Coverage: all five supported kinds (`kTopT>0`, `kTopT<0`, `kBotT>0`, `kBotT<0`,
`kBotT==0`) and both conductivity modes appear in the fixtures above.

Each fixture JSON holds `source_matched: true`, `evidence_class: pinned-fortran`,
`temperature_unit: C`, `time_points`, `node_numbers`, `node_depths`, `temperatures`
(one row per time point, top node first), and a `manifest` with the source/archive
hashes, patch hash, executable hash, build-manifest hash, compiler, host, flags,
command, input and output sha256, and the SELECTOR/PROFILE sha256. No tolerance field.

### The refused case (SD-006)

`heat_flux_top_zero_bot_campbell` has a valid deck and runs to completion, but the
pinned build produces NaN temperatures from the first print at `t=6` (node 1). The
producer refuses it: `refusal.json` (kept in the case directory) records the reason, and
no fixture is written. The cause and the isolation (single-factor runs, and a scratch
build with the Campbell unit factor removed) are recorded in
`docs/FORTRAN_SOURCE_DEFECTS.md` as **SD-006**. The contract's Supported entry for
`kBotT==0` is not changed by this; the decision on the Campbell scaling is the user's.

## Isolation and reproduction

- `tools/run_heat_reference.py --all --task-base .development/24.4.3/heat-runs`
  runs each case in a fresh run directory: inputs are copied and byte-checked, aliases
  `Selector.in` and `Profile.dat` are created with the Fortran `prefix\name` names, the
  executable hash is checked against the build manifest, and outputs are validated
  (non-empty, `end` marker) before parsing.
- Parsing reads `NOD_INF.OUT` (unit 75, `HYDRUS.FOR:147-148`) at fixed columns: node
  `1-4`, depth `6-15`, Temp `104-111` (formats 120 and 130, `OUTPUT.FOR:820-821`, `:853`).
  A `*` overflow is refused.
- A non-finite Temp refuses the fixture (`guard_finite`), and `refusal.json` is written
  into the run root.

### Deviations

- `tools/run_fortran_reference.run_fixture` is not used. It resolves task roots
  physically and calls `relative_to(REPO_ROOT)`, which fails under the approved
  worktree `.development` symlink. The producer mirrors its isolation steps and reuses
  the verified build. `run_fortran_reference.py` is not edited.
- `dtMin=0.01` (see above).

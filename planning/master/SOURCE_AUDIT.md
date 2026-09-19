# HYDRUS source and provenance audit

## Inventory

The supplied snapshot contains ten fixed-form Fortran files (11,053 lines total). Sizes below are byte counts from the immutable working tree; the archive contains the same ten basenames.

| File | Lines | Bytes | Role / anchors |
|---|---:|---:|---|
| `src_reference/HYDRUS.FOR` | 999 | 42,822 | Main program at 20; initialization 123; time loop 399; shutdown 784–998 |
| `src_reference/HYSTER.FOR` | 756 | 23,206 | Lenhard hysteresis routines |
| `src_reference/INPUT.FOR` | 1,723 | 61,696 | Input, initialization, tables, profile and chemistry readers |
| `src_reference/MATERIAL.FOR` | 946 | 26,108 | Retention/conductivity functions and quadrature |
| `src_reference/OUTPUT.FOR` | 1,114 | 49,156 | Time-level, balance, profile and observation output |
| `src_reference/SINK.FOR` | 366 | 11,576 | Root uptake, salinity stress and root growth |
| `src_reference/SOLUTE.FOR` | 2,105 | 80,186 | Solute transport, reactions, sorption and exclusion |
| `src_reference/TEMPER.FOR` | 323 | 11,722 | Heat transport and latent heat |
| `src_reference/TIME.FOR` | 1,594 | 59,518 | Time control, meteorology, snow, interception and evaporation |
| `src_reference/WATFLOW.FOR` | 1,127 | 45,187 | Richards flow, boundary switching, vapor and dual porosity |

`WATFLOW.FOR` is the actual water-flow filename; the illustrative `WATER.FOR` name in the initial prompt is absent. The archive is `planning/H1D_Src.zip`; companion artifacts are the 4.08 manual, 4.17 manual, and the 2008 VZJ Hydrus/Stanmod paper listed in the manifest.

## Routine inventory

Every declared program/routine, grouped by source and with declaration line, is recorded here. Functions without an explicit type are retained as declared Fortran functions.

| Source | Declared program/routines |
|---|---|
| `HYDRUS.FOR` | `HYDRUS` program (20); internal `h_sig` (32), `hand_fpe` (40), `ErrorOut` (784), `CloseOutput` (853), `h_sig` (889), `hand_fpe` (910), `CloseFiles` (936) |
| `HYSTER.FOR` | `Hyst` (6), `HysterIni` (124), `PATH` (240), `HAWPATH` (292), `DRAIN` (444), `DRY` (535), `WET` (594), `UPDATEHYST` (652) |
| `INPUT.FOR` | `BasInf` (3), `Conversion` (182), `NodInf` (208), `InitW` (472), `InitDualPor` (508), `MatIn` (536), `HysterIn` (732), `GenMat` (762), `TmIn` (866), `MeteoIn` (934), `SinkIn` (1070), `RootIn` (1139), `TempIn` (1205), `Profil` (1256), `ChemIn` (1289), `OpenSoluteFiles` (1464), `iGetFileVersion` (1498), `EmptyStr` (1545), `Init` (1560) |
| `MATERIAL.FOR` | `FK` (16), `FC` (201), `FQ` (279), `FH` (350), `FS` (421), `FKQ` (493), `FKS` (557), `qnorm` (623), `xMualem` (642), `DoublePor` (663), `ZBRAK` (694), `ZBRENT` (729), `Gamma` (804), `BInc` (831), `qromb` (857), `trapzd` (879), `polint` (904) |
| `OUTPUT.FOR` | `TLInf` (3), `ALInf` (296), `SubReg` (325), `NodOut` (720), `ObsNod` (931) |
| `SINK.FOR` | `SetSnk` (3), `SetSSnk` (138), `FSAlfa` (226), `FAlfa` (245), `SetRG` (268), `RootInN` (326) |
| `SOLUTE.FOR` | `Solute` (6), `Coeff` (241), `MatSet` (755), `BanSol` (945), `PeCour` (965), `Disper` (1025), `NEquil` (1078), `SorbConc` (1212), `MassTran` (1319), `FluxConc` (1501), `Blocking` (1561), `Deposit` (1595), `NonEqInit` (1661), `Exclusion` (1713), `MoistDepIn` (1786), `rMD` (1821), `rMD1` (1869), `MassInit` (1912), `cInit` (1962), `SolMass` (1976), `ZBRAK1` (2002), `ZBRENT1` (2035) |
| `TEMPER.FOR` | `Temper` (5), `SetUpHeat` (77), `TempAdj` (180), `TempCap` (269), `xLatent` (310) |
| `TIME.FOR` | `TmCont` (3), `RTime` (36), `SetBC` (59), `SetChemBC` (186), `DailyVar` (213), `SinPrec` (238), `Snow` (257), `Meteo` (317), `SetMeteo` (443), `Aero` (632), `RadGlobal` (672), `Cloudiness` (698), `CropRes` (733), `RadLongNet` (771), `Intercept` (787), `IntercepS` (823), `MeteoInt` (864), `Evapor` (913), `RadLongNet1` (1068), `AeroRes` (1088), `UpdateEnergy` (1166), `DailyMet` (1225), `DailyTemp` (1334), `Humidity` (1384), `SetDayMet` (1420), `DayMeteoOut` (1545), `Table` (1562) |
| `WATFLOW.FOR` | `WatFlow` (3), `Reset` (173), `Gauss` (308), `Shift` (359), `SetMat` (433), `Fqh` (572), `FqDrain` (579), `Veloc` (717), `Hyster` (778), `ConVapor` (849), `VaporContent` (930), `ConVh` (968), `fRo` (1001), `DualPor` (1027), `Update` (1079) |

Intrinsic and compiler/library calls include date/time (`getdat`, `gettim`), argument (`GETARG`), formatted/unformatted I/O, `STOP`, and math operations. Project calls are the routines in this table; calls such as `Table`, `Evapor`, `Snow`, `MeteoInt`, `CloseOutput`, `CloseFiles`, and `Exclusion` are source-level dependencies even where branches are optional. `MSFLIB` signal handlers (`SIGNALQQ`, `SIGFPE`, `SIGTERM` paths) and any compiler-provided date/time/argument extensions are unresolved portability dependencies. No call edge was silently treated as a project routine when only a compiler/library implementation was observed.

Observed external or unresolved symbols include `MSFLIB`/signal APIs, platform date/time and argument extensions, and branch-referenced routines absent from this ten-file set (for example commented or optional `DayMeteoOut`-adjacent integrations). They require investigation in a future oracle-portability task.

## Provenance

SHA-256 values for all required assets are in `SOURCE_MANIFEST.json`. The archive lists exactly the ten source basenames. A byte-for-byte comparison of each extracted archive member against the corresponding `src_reference/*.FOR` file is required evidence and was consistent with the research record: all ten matched. Archive timestamps span 2009–2022 and therefore do not establish a release date.

The main header identifies an internal “version 7.0” and an October 2009 modification. The local 4.08 manual is the planned documentation companion; the 4.17 manual is comparative evidence. These artifacts support a working lineage described as “4.08 or derivative,” but do not verify a product release identity or 4.17 parity. Artifact notices and redistribution terms remain to be reviewed per artifact; this audit assigns no license and does not infer public-domain status from the prompt.

## Build blockers

The unmodified syntax check recorded in the research evidence (`gfortran -fsyntax-only -std=legacy -ffixed-line-length-none src_reference/*.FOR`) exits 1 at `HYDRUS.FOR:29` because `msflib.mod` is unavailable and at `TIME.FOR:551` because `if(iMetHour)` supplies a non-scalar logical expression. This is not a link, runtime, or numerical baseline. Additional unresolved symbols may appear after those blockers are addressed.

The minimal future oracle-portability plan is a separately identified copy: replace or isolate MSFLIB signal handling; verify `iMetHour` semantics before changing it; audit `GETARG`, `getdat`, `gettim`, `CloseOutput`, Windows path separators, command arguments, and compiler-specific logical behavior; then record patch hash, compiler/flags, executable hash, inputs, and outputs. Do not alter these originals, silently promote REAL precision, rewrite algorithms, or claim a working oracle here.

Observed file/platform coupling includes input units 10, 30–36 and 35, output units 43–45, 50, 70–72, 75–78, 80+i, 99, and inquiry of 81–83 and 41–42; filenames are assembled from `cDataPath`/`cFileName`, with legacy path and status/error conventions. The main program opens input/profile/atmospheric/material/solute and output/error files, while `OpenSoluteFiles` allocates units 80+i. Unit ownership and canonical names must be confirmed during future adapter design.

## Evidence

- `HYDRUS.FOR:20`, header lines 1–18, `use MSFLIB` at 29, and shutdown handlers establish the entry point and platform coupling.
- `INPUT.FOR` anchors 3, 182, 208, 536, 762, 866, 1070, 1205, 1289, 1498, and 1560 establish input and initialization responsibilities.
- `MATERIAL.FOR` anchors 16, 201, 279, 350, 493, 557, and 623–904 establish hydraulic functions and numerical helpers.
- `WATFLOW.FOR` anchors 3, 173, 308, 359, 433, 717, 778, 849, 1027, and 1079 establish flow, boundary, vapor, hysteresis, and dual-porosity branches.
- `TIME.FOR:551` is the recorded integer/logical blocker; `HYDRUS.FOR:29` is the MSFLIB blocker.
- The archive listing, SHA-256 manifest, source sizes, and PDF title-page identities are the reproducible artifact evidence. No numerical outputs or verified release baseline exist.

## Open decisions

1. Confirm the intended compatibility release label and whether the working lineage should remain “4.08 or derivative”; ownership: user/release decision before oracle fixtures.
2. Decide the supported host/compiler and exact MSFLIB replacement policy after semantic review of signal and date/time behavior; ownership: oracle-portability task.
3. Confirm canonical input/output filename and file-unit mapping for a future compatibility adapter; ownership: I/O design task.
4. Determine artifact-specific notices and redistribution terms from the source/archive/manual custodians; ownership: provenance review, with no license inferred here.

# Derived Example 3 Fortran reference run

This run uses the authored derived fixture described in `CASE.json`; it is not a recovered manual deck or manual reproduction. Raw output is retained only under `.development/12.2.1/`.

## Command and status

`python3 tools/run_fortran_reference.py --case reference/fortran_reference/solute/example3 --timeout-seconds 60`

Reference invocation: `/home/andrew/AI/hyd.rs/.development/12.2.1/reference-build/hydrus-reference solute-input` in `.development/12.2.1/run`; exit status `0` within 60 seconds.

## Provenance

- source/archive hashes: `{"planning/H1D_Src.zip": "d0c47cbcb7a298ab16c6e08010bd7ca048d5527ce2f989a749390c0c55e848cf", "src_reference/HYDRUS.FOR": "fca01114a773b0ec9d614d17ed493a008bac82f23d22a187a10a4d4f48fbf3a9", "src_reference/HYSTER.FOR": "29e4818a3b8c58a6698257a4ac4fa9df439ed43e983271423c7877a9cd9d8a67", "src_reference/INPUT.FOR": "d29d073ac40611b9991f77416edcdbcf8d0affe01330675e5a08f66db1f52411", "src_reference/MATERIAL.FOR": "a2a2beb4cd986946c6a0b61e31d92de9e776d9d4d9e3ee9bf9ccee1cbcb698ca", "src_reference/OUTPUT.FOR": "66bed77a0375f2561c39e2d93d6e9765980dae8abdb48cd0a68c163e6a031f80", "src_reference/SINK.FOR": "95afb5f6ae99e03478a08641e9bea77451f7beda7f0c3460fd12af3a36e7901f", "src_reference/SOLUTE.FOR": "81fed6adbbd18aabe53385151166ab36d0df504a7939b54e10460c2254488910", "src_reference/TEMPER.FOR": "ac7ff3fb20283b50265af65445413da0dd7d3dad814679c147c9acd0a8fc2ad3", "src_reference/TIME.FOR": "8b6af595a1ccb3952cfa7b1eb56c0e37a1cfc6b8d55e26d8907288c41f08b92f", "src_reference/WATFLOW.FOR": "e5591cebae4cbb44e8fe07520e08fc98c1522531c67753c0cc03fe2e164b5ef9"}`
- patch: `622d1ea68285257e98a169f94dec83f3df1c671a9366f651e845806cb8c00d7e`
- executable: `b37f27bc69b6182cb798a1537527f2bdd6fe6d73d2a55978e45878a8228c8f3a`
- compiler: `GNU Fortran (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0`
- platform: `Linux-7.0.0-31-generic-x86_64-with-glibc2.39`; `x86_64`
- flags: `-std=legacy -ffixed-line-length-none`
- Python: `3.12.3`

### Input hashes

- `LEVEL_01.DIR`: `eb4bd64f7014f7d42e9d358035802242741b974e8dfcd37c59f9c21ce29d781e`
- `PROFILE.DAT`: `81bc1fa627743fa2a6794d3b34a68e409aa00c190c871188b32596b61903a60e`
- `SELECTOR.IN`: `5295301f190125560c5574a0c1ca63c50f9818b13dd611ec99040b5e7e949d69`

### Output hashes

- `BALANCE.OUT`: `0c0b30b17941f0671b6d5ced0631067cd29f33982bc709a4dff5226e673a60fb`
- `I_CHECK.OUT`: `44cf5bc435fa3e05ba4382f9a67c6efc004522c0f132a147c202f7687d4f935d`
- `NOD_INF.OUT`: `063536244e52fe76684b677b27b1a0b576f20033867e742ca092cad654159aae`
- `OBS_NODE.OUT`: `329bf0fa7064364b3c792b10846b4850e5fe53f47b26563edc1a80d2c3471877`
- `PROFILE.OUT`: `2f1bb35cfa7f0355117213519aedbfff0a70a66854fc354853ad8e349cb5f947`
- `RUN_INF.OUT`: `df645a96813ef12c93c5d86e5870a6123e7a3ac6c8e8964f7d29ea4a2d7879d8`
- `T_LEVEL.OUT`: `4b504db6e53a9c340a6f1a1bd06f5f32e82ee7520e8596981d2b59b40819d2dc`
- `solute1.out`: `deaceec2adb21f8d55888c156b30b20c713c9adf81c233af08eac07e4c11384e`
- `solute2.out`: `d10020b1cf68adb3b8db21760c4eb08a07b68af82b56f56428acfc651f50fcbd`
- `solute3.out`: `ee5da17a20d64e94c8d2d0988a06faf0e5665f7766b5c235753edb642d7efc51`

### Captured log hashes

- `stderr.txt`: `cde5e7d17e64813704a51a756095eaf585d28601c664fa68a1f27038b6399fcc`
- `stdout.txt`: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`

## Completion evidence

The single amendment C execution returned zero; all seven standard output files and all three per-species outputs were present and nonempty. Read-only inspection confirmed exact terminal `end` records in `RUN_INF.OUT`, `T_LEVEL.OUT`, `NOD_INF.OUT`, `OBS_NODE.OUT`, `PROFILE.OUT`, and `solute1.out` through `solute3.out`; `BALANCE.OUT` ends with `Calculation time [sec]`. `I_CHECK.OUT` is an input echo, ending with `tPulse = 200.000`, as written by `ChemIn` (`INPUT.FOR:1420`) before unit 50 closes (`HYDRUS.FOR:359`); it has no terminal `end` record. No `Error.msg` was produced.

## Observed output schema and completion depth

Each per-species file contains 2000 finite numeric rows with 17 columns, strictly increasing times from 0.1 through 200 hours, and consecutive `TLevel` values 1–2000. The printed columns are `Time`, `cvTop`, `cvBot`, `Sum(cvTop)`, `Sum(cvBot)`, `cvCh0`, `cvCh1`, `cTop`, `cRoot`, `cBot`, `cvRoot`, `Sum(cvRoot)`, `Sum(cvNEql)`, `TLevel`, `cGWL`, `cRunOff`, `Sum(cRunOff)`; `NObs=0` adds no observation-node columns. The file header states that solute fluxes are positive into the region.

`NOD_INF.OUT` contains 1001 finite numeric node rows at each of 0, 50, 100, and 200 hours. Each row has 17 columns: node, native coordinate, head, moisture, conductivity, capacity, flux, sink, kappa, velocity/conductivity ratio, temperature, three dissolved concentrations, and three sorbed values. These are observed printed layouts, not yet a normalized comparison parser or a validated interpretation of every scientific field. `RUN_INF.OUT` reaches step 2000 at 200 hours.

## Amendment C preflight and preservation

- The only new selector input change is the tenth basic-information flag, `lEquil=f` to `t`. All other selector bytes, all `PROFILE.DAT` bytes, and `LEVEL_01.DIR` bytes match the archived pre-amendment fixture. Every pre-existing `CASE.json` field is identical; only the initial-profile format/mode provenance entry was added.
- The profile header declares `NumNP=1001`, `NS=3`; exactly 1001 numeric rows contain 13 fields each and the final observation-node count is `NObs=0`. Source `NodInf` reads this width for initial `lEquil=t`; later `ChemIn` independently derives equilibrium mode from fraction 1 and immobile water 0.
- `TopInF=BotInF=AtmBC=lMeteo=false`, so no atmospheric or meteorological input is required. `iModel=0`, while `nTabMod=10`, so `Mater.in` is unnecessary. The supplied `solute-input` argument bypasses `LEVEL_01.DIR`; `Options.in` is optional on the source open-error continuation path.
- Independently applying the unchanged portability patch to a scratch copy of the pinned source yielded bytes identical to the existing staged source before execution and the freshly staged source after execution. Source/archive, patch, and runner hashes remained identical to the archived baseline. Staged input copies match the committed fixture bytes.
- The pinned `NodInf` call ends `lDualNEq,lFlux`, while its formal arguments end `lFlux,lDualNEq`; both initial flags are false in this case. This existing source-interface caveat remains unresolved.
- JSON validation passed; `python3 -m unittest tools.test_run_fortran_reference` passed 7/7; `git diff --check` passed. Exactly one fresh fixture execution was performed, with no additional executable invocation for verification.

## Warnings

- `Note: The following floating-point exceptions are signalling: IEEE_UNDERFLOW_FLAG IEEE_DENORMAL`

The inherited runner searches for marker substrings rather than exact terminal records. Its generated prose incorrectly claimed an `end` marker for `I_CHECK.OUT`; this manifest corrects that claim using actual files and the source reader. The runner remains unchanged under amendment C. The output headers also print a malformed date (`26.**.`); build/staging manifests retain the UTC run date. The reported IEEE underflow/denormal flags require review during scientific comparison; this task does not classify their numeric impact.

## Limitations

This verifies reference execution and output completeness only. Realized water/solute coefficients, boundary signs, coordinate mapping, paired Rust parity, analytical agreement, calibrated tolerances, artifact redistribution permission, and release compatibility remain unverified.

## Previous failed-run audit

Before this run, the full previous fixture, raw failed run, and reference build were copied to `.development/12.2.1/amendment-c-audit-2p2srnlb/` (`fixture-before/`, `prior-run/`, and `prior-build/`). Its `baseline-hashes.json`, `preflight.json`, `read-only-verification.json`, and `wrapper-result.json` preserve amendment C checks and execution count. Earlier selector-alignment and profile-label attempts remain documented in `planning/tasks/12.2.1.todo.md`; amendment B's complete previous `RUN.md` follows verbatim. Old paths inside that preserved manifest refer to the then-current failed run, now retained under `.development/12.2.1/amendment-c-audit-2p2srnlb/prior-run/`.

<details>
<summary>Preserved amendment B manifest (EOF at INPUT.FOR:251)</summary>

# Derived Example 3 Fortran reference run

This attempt did not produce a completed scientific reference run. The fixture is derived, not a recovered manual deck or manual reproduction. Raw files remain only under `.development/12.2.1/`.

## Command and status

`python3 tools/run_fortran_reference.py --case reference/fortran_reference/solute/example3 --timeout-seconds 60`

Reference invocation: `/home/andrew/AI/hyd.rs/.development/12.2.1/reference-build/hydrus-reference solute-input`; exit status `2` within 60 seconds.

## Provenance

- source/archive hashes: `{"planning/H1D_Src.zip": "d0c47cbcb7a298ab16c6e08010bd7ca048d5527ce2f989a749390c0c55e848cf", "src_reference/HYDRUS.FOR": "fca01114a773b0ec9d614d17ed493a008bac82f23d22a187a10a4d4f48fbf3a9", "src_reference/HYSTER.FOR": "29e4818a3b8c58a6698257a4ac4fa9df439ed43e983271423c7877a9cd9d8a67", "src_reference/INPUT.FOR": "d29d073ac40611b9991f77416edcdbcf8d0affe01330675e5a08f66db1f52411", "src_reference/MATERIAL.FOR": "a2a2beb4cd986946c6a0b61e31d92de9e776d9d4d9e3ee9bf9ccee1cbcb698ca", "src_reference/OUTPUT.FOR": "66bed77a0375f2561c39e2d93d6e9765980dae8abdb48cd0a68c163e6a031f80", "src_reference/SINK.FOR": "95afb5f6ae99e03478a08641e9bea77451f7beda7f0c3460fd12af3a36e7901f", "src_reference/SOLUTE.FOR": "81fed6adbbd18aabe53385151166ab36d0df504a7939b54e10460c2254488910", "src_reference/TEMPER.FOR": "ac7ff3fb20283b50265af65445413da0dd7d3dad814679c147c9acd0a8fc2ad3", "src_reference/TIME.FOR": "8b6af595a1ccb3952cfa7b1eb56c0e37a1cfc6b8d55e26d8907288c41f08b92f", "src_reference/WATFLOW.FOR": "e5591cebae4cbb44e8fe07520e08fc98c1522531c67753c0cc03fe2e164b5ef9"}`
- patch: `622d1ea68285257e98a169f94dec83f3df1c671a9366f651e845806cb8c00d7e`
- executable: `b37f27bc69b6182cb798a1537527f2bdd6fe6d73d2a55978e45878a8228c8f3a`
- compiler: `GNU Fortran (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0`; platform: `Linux-7.0.0-31-generic-x86_64-with-glibc2.39` / `x86_64`
- flags: `-std=legacy -ffixed-line-length-none`

### Input hashes

- `LEVEL_01.DIR`: `eb4bd64f7014f7d42e9d358035802242741b974e8dfcd37c59f9c21ce29d781e`
- `PROFILE.DAT`: `81bc1fa627743fa2a6794d3b34a68e409aa00c190c871188b32596b61903a60e`
- `SELECTOR.IN`: `dd455952f005344092813fceda1caf816aae497d9e2d1bce0ce6edeb1cf33727`

### Captured log hashes

- `stderr.txt`: `dd57b76a2f4b553f6cd51a327d8b84176bac4b68db416e54a6bbb238ba619e85`
- `stdout.txt`: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`

### Observed output hashes (incomplete)

- `solute-input\BALANCE.OUT`: `d0cde678c8c066ab0e18665370800dd6ffbec280fc2e67dd4b1af268935bb84f`
- `solute-input\I_CHECK.OUT`: `f7b66452df94741a79b0fac5b0510a5ebe33c11d8ec347fcd8cfb0f4061c4863`
- `solute-input\NOD_INF.OUT`: `d0cde678c8c066ab0e18665370800dd6ffbec280fc2e67dd4b1af268935bb84f`
- `solute-input\OBS_NODE.OUT`: `d0cde678c8c066ab0e18665370800dd6ffbec280fc2e67dd4b1af268935bb84f`
- `solute-input\PROFILE.OUT`: `d0cde678c8c066ab0e18665370800dd6ffbec280fc2e67dd4b1af268935bb84f`
- `solute-input\RUN_INF.OUT`: `d0cde678c8c066ab0e18665370800dd6ffbec280fc2e67dd4b1af268935bb84f`
- `solute-input\T_LEVEL.OUT`: `d0cde678c8c066ab0e18665370800dd6ffbec280fc2e67dd4b1af268935bb84f`

## Blocker

This is the one fixture run authorized by scope amendment B. The reference exited 2 with a gfortran end-of-file runtime error at `src_reference/INPUT.FOR:251` while reading `Profile.dat`. The stdout/stderr and staged inputs are retained in `.development/12.2.1/run/`. The error occurs while reading node records; the profile column-label line was removed, and the count, header, and all 1001 node rows remain. No solute outputs or completion markers were produced. No scientific result is available, and the task's stop rule is satisfied. Further diagnosis requires a new orchestration plan.

</details>

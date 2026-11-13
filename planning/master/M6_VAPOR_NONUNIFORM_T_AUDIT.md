# M6 vapor: nonuniform-temperature coverage audit (27.1.3)

Status: read-only audit, 2026-11-12. No fixture written, no producer edit, no executable run, not committed.
Scope: row "Default thermal conductivity (`iCampbell=0`)" in `planning/master/M6_VAPOR_COVERAGE.md:12`, whose requirement is "Nonuniform-temperature source fixtures required" and whose status is "nonuniform-temperature coverage deferred to G22".
Orchestrator answers applied: dry regime (h = -1e4 cm) for the proposed pair; one pair only; LEVEL_01.DIR content identical to the dry case; no producer change proposed here.

## 1. Existing thermal cases

All cases use `reference/vapor_fortran/cases/<id>/`. Every case has Pcp_File_Version=4, so `TempIn` reads `Ampl,tPeriod,iCampbell,SnowMF,lSnowInput`. The Campbell record is `0.0 0.0 0 0.0 f` in every SELECTOR.IN, so iCampbell=0 in all of them. Water BC: `TopInf=f, BotInf=f, KodTop=1, KodBot=1` in every case, so fixed head at both ends. The dry-case head value is -1e4 cm at both ends (commit `b8e0544`).

| Case | lVapor (SELECTOR line 12) | Boundary record (kTop, tTop, kBot, tBot) | Initial T profile (PROFILE.DAT, 41 nodes, node 1 = top) | Initial head | tMax (h) | Role |
|---|---|---|---|---|---|---|
| `vapor_thermal_gradient` | t | 1, 25.0, 1, 5.0 (both Dirichlet) | uniform 15.00 C | hydrostatic, -5.0 cm at top to -17.5 cm at node 26 | 24 | wet, vapor on |
| `vapor_off_control` | f | 1, 25.0, 1, 5.0 | uniform 15.00 C | same as above | 24 | wet control for the row above (differs only in line 12) |
| `vapor_isothermal_head_gradient` | t | 1, 25.0, 1, 25.0 | uniform 25.00 C | wet, hydrostatic | 24 | isothermal; no off control |
| `vapor_dry_thermal_gradient` | t | 1, 45, 1, 5 (both Dirichlet) | linear ramp, 45.00 (node 1) to 5.00 (node 41) | -1e4 cm uniform | 240 | dry, vapor on |
| `vapor_dry_off_control` | f | 1, 45, 1, 5 | same ramp | -1e4 cm | 240 | dry control |
| `vapor_heatbc_df_on` / `_off` | t / f | 1, 45, -1, 0.0 (top Dirichlet, bottom flux 0) | same ramp as dry case | -1e4 cm | 240 | BC-variant pair |
| `vapor_heatbc_dz_on` / `_off` | t / f | 1, 45, 0, 5 (top Dirichlet, bottom zero flux, see 4) | same ramp | -1e4 cm | 240 | BC-variant pair |
| `vapor_heatbc_fd_on` / `_off` | t / f | -1, 0.0, 1, 5 (top flux 0, bottom Dirichlet) | same ramp | -1e4 cm | 240 | BC-variant pair |
| `vapor_heatbc_ff_on` / `_off` | t / f | -1, 0.0, -1, 0.0 (both flux 0) | same ramp | -1e4 cm | 240 | BC-variant pair |
| `vapor_heatbc_fz_on` / `_off` | t / f | -1, 0.0, 0, 5 (top flux 0, bottom zero flux) | same ramp | -1e4 cm | 240 | BC-variant pair |

Sources: `SELECTOR.IN` and `PROFILE.DAT` for each case; `fixture.json` `data_prefix` (e.g. `vapor_thermal_gradient/fixture.json:23`).

Boundary-code meanings, from `src_reference/TEMPER.FOR`:
- `kTopT > 0` or `kBotT > 0` is a Dirichlet temperature (`TEMPER.FOR:87-90`, `:144-147`).
- `kTopT < 0` or `kBotT < 0` is a flux (`:91`, `:148`).
- `kBotT = 0` takes the `else` branch at `TEMPER.FOR:109-111` (`D(1)=-1, E(1)=1, F(1)=0`), which is a zero-gradient (zero conductive flux) bottom node. `tBot` is not used. This resolves the "z" code in the heatbc names.

## 2. What the M6 row requires, and what existing cases do not provide

Existing coverage with vapor on:
- Nonuniform T as a steady ramp: `vapor_dry_thermal_gradient` and the ten `vapor_heatbc_*` cases use the dry 45 to 5 C ramp. Its ends match the boundary temperatures, so the profile is close to a conductive linear steady state.
- Nonuniform T that develops in time: `vapor_thermal_gradient` starts uniform at 15 C and relaxes toward a 25 to 5 C boundary-driven gradient over 24 h. This is wet, and the commit message for `b8e0544` says wet fixtures "cannot show vapor feedback on head, moisture or temperature at print precision".

Gaps:
1. No case has a non-linear initial temperature profile. Every initial profile is either uniform (15 or 25 C) or a linear ramp (45 to 5 C).
2. No wet case has a nonuniform initial profile. The wet nonuniform cases are only nonuniform in time, and they do not discriminate at print precision.
3. No case has an inverted boundary gradient (top colder than bottom). The M6 row does not name this as required, so it is recorded as not required here and not proposed (orchestrator answer 2).

So the row's required source fixtures are missing for a non-linear initial profile under the dry regime, which is the only regime shown to discriminate.

## 3. Proposed case table

One shape pair (orchestrator answer 2). Shape: `step`, a non-linear initial profile the existing cases lack.

| Case ID | Vapor | Regime | Initial T profile (proposed) | Boundary record | Water BC | Conductivity | Base deck | Producer schema check |
|---|---|---|---|---|---|---|---|---|
| `vapor_nonuniform_step_on` | t (line 12 `f f f t f f`) | dry, h = -1e4 cm uniform | nodes 1 to 20 at 45.00 C, nodes 21 to 41 at 5.00 C | 1, 45, 1, 5 (both Dirichlet, matching the dry case) | fixed head both ends (`KodTop=KodBot=1`) | iCampbell=0; default TPar record 0.60/0.02/0.5/... | copy of `vapor_dry_thermal_gradient` SELECTOR.IN, PROFILE.DAT replaced in its last column only | expressible; see 4 |
| `vapor_nonuniform_step_off` | f (line 12 `f f f f f f`) | same as on | same | same | same | same | copy of `vapor_dry_off_control` SELECTOR.IN, PROFILE.DAT as in on | expressible; see 4 |

Notes on the proposed case:
- The step profile is a design choice, not a run. The exact node index of the step (node 20 to 21) is a proposal for 27.2.1 to confirm.
- The SELECTOR and PROFILE layout is inherited from the dry case, so the time schedule, print times, and TPar are unchanged. Only the step profile changes, which keeps the comparison with the existing dry pair clean.

## 4. Producer schema check (`tools/run_vapor_reference.py`)

No new schema field is needed for either proposed row.
- Each case directory needs `fixture.json` (`case_id`, `runner.inputs`, `runner.outputs`, `runner.aliases`, `runner.data_prefix`), plus `SELECTOR.IN`, `PROFILE.DAT`, and `LEVEL_01.DIR`. The inputs and aliases follow the dry pattern exactly (`run_case` at about `:100-160`; inputs copied at `:114-118`; `data_prefix` and aliases at `:120-122`).
- `LEVEL_01.DIR` is copied unconditionally at `:123`. Per orchestrator answer 3, the new cases carry the same content as the dry case, which is a single `.` line.
- `run_case` refuses to overwrite fixtures and run roots (`:97-107` area); the new cases are new IDs, so that is not triggered.
- Vapor reachability is checked by the string `"f f f t f f"` at `:146-150`. The `_on` case matches it. The `_off` case has `f f f f f f`, so `enabled` is false and `vapor_blocks` must be absent, which matches the control cases.
- The `expected` set at `:20-21` does not yet contain the new IDs. Extending it is required for 27.2.1. It is not part of this audit.
- The spec's anchor `tools/run_vapor_reference.py:15-35` covers constants, the expected set and the parsers. The `run_case` body and the `main` argument handling are at about `:100-160` and `:171-194`. The spec cited the right file, but the line anchors are approximate.

Case-directory gap to be aware of: `vapor_thermal_gradient`, `vapor_off_control` and `vapor_isothermal_head_gradient` list `LEVEL_01.DIR` in `fixture.json` inputs but have no such file on disk. It is not in git history either. Because `run_case` copies it unconditionally, these three wet cases cannot be regenerated from the current producer as the repo stands. This does not block the proposed dry pair, but 27.2.1 must not reuse those wet directories as templates without adding the file.

## 5. Fortran input limits (`src_reference/INPUT.FOR`)

- `TempIn` is at `:1205-1245`. Heat transport parameters are read at `:1216-1217` (nine values per material, loop at `:1215-1218`).
- For `iVer >= 3`, `Ampl,tPeriod,iCampbell,SnowMF` are read at `:1223`. For `iVer > 3`, `lSnowInput` is read at `:1225`. For `iVer <= 2`, `iCampbell = 0` is set at `:1221`.
- The boundary record `kTopT,tT,kBotT,tB` is read at `:1236`. `tTop` and `tBot` take those values only when `TopInf` or `BotInf` is false (`:1237-1238`). All cases use `f`, so the boundary temperatures take the values in the table above.
- No temperature-range limit is enforced in `TempIn` itself. The initial profile is read from `PROFILE.DAT`, not here.

## 6. Criteria

`reference/vapor_fortran/criteria.json` keys (no values read): `approved_at`, `approved_by`, `checks`, `contract_revision`, `contract_sha256`, `discrimination`, `fields`, `revision_note`, `schema_version`. The proposed case uses the same fields as existing cases. No new criteria key is needed.

## 7. Risks

- Wet regime does not discriminate at print precision (commit `b8e0544`). The proposed pair is therefore dry (orchestrator answer 1).
- `LEVEL_01.DIR` gap in three wet directories (section 4).
- The step profile is a design choice that 27.2.1 must confirm before running.

## 8. Questions for the user

None open. Orchestrator answers 1 to 3 resolve the regime, the number of pairs, and the LEVEL_01.DIR content.

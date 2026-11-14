# M6 vapor closed flux/flux pair: Fortran fixture design (task 29.1.1)

Status: design only. No deck, fixture, criteria or runner change is made by this document. Anchors re-verified 2026-11-14 against `src_reference/`.

## Decks (four, in two on/off pairs)

All cloned from `reference/vapor_fortran/cases/vapor_dry_thermal_gradient` and `vapor_dry_off_control` (SELECTOR/PROFILE/LEVEL_01.DIR; they differ from each other only in `lVapor`, `SELECTOR.IN` line 12). Shared base: one loamy material, 41 nodes, uniform h=-100 cm, 45 C top to 5 C bottom Dirichlet heat (`1 45 1 5`), 240 h, prints 6/24/72/240 h, `lWDep=false`.

| Case id | lVapor | rTop | rBot | Purpose |
|---|---|---|---|---|
| `vapor_closed_flux_on` | t | 0 | 0 | Zero-flux closed pair; vapor boundary rows and storage with no imposed flux |
| `vapor_closed_flux_off` | f | 0 | 0 | Control for the above |
| `vapor_flux_known_on` | t | -1.0e-4 cm/h | -5.0e-5 cm/h | Nonzero known flux; exercises the rTop and rBot terms |
| `vapor_flux_known_off` | f | -1.0e-4 | -5.0e-5 | Control for the above |

SELECTOR edits relative to the base deck (everything else identical):
- Top record `TopInf WLayer KodTop InitCond` values: `f f -1 f` (was `f f 1 f`).
- Bottom record values: `f f f f -1 f 0` (was `... 1 f 0`).
- New two lines after the bottom record: a header line, then `rTop rBot rRoot` (`0.0 0.0 0.0` or `-1.0e-4 -5.0e-5 0.0`). These are read only for this combination: `INPUT.FOR:63-67` (condition `.not.TopInF.and.KodTop.eq.-1` or the bottom analogue, read at :66-67). Without them the deck mis-parses.
- PROFILE.DAT unchanged. `TopInF/BotInF=f`, so `INPUT.FOR:104-105` do not rewrite the codes to +-3; `WLayer=f` so `:110` does not apply; `AtmBC=f` so `:106-108` do not apply.

Flux choice and sign: assembly adds `rBot` to `PB` (`WATFLOW.FOR:221-222`) and subtracts `rTop` from `PT` (`:294`), so rTop<0 is inflow at the top and rBot<0 is outflow at the bottom (same sign pattern as the free-drain definition `rBot=-ConB*Grav*fRE`, `:215`). This is derived from the row algebra and must be confirmed from the 29.3.2 output (end-node fluxes). Net inflow 5e-5 cm/h times 240 h is 0.012 cm, so the column stays dry (h<0 everywhere); magnitude is deliberately small to avoid Fortran time-step cuts (`WATFLOW.FOR:129-146`). Terms tested: zero deck tests `ConB`/`ConA` with `ConVh` (:211, :277), `ConTB`/`ConTA` thermal vapor (:224-229, :287-291), `ThVNew-ThVOld` storage (:229, :291), with `rBot`/`rTop` = 0. Known-flux deck additionally tests the `+rBot` term in `PB` (:221-222) and the `-rTop` term in `PT` (:294), including their interaction with the vapor-added conductance through the Neumann elimination (`Gauss`: bottom :315-322, top :327-334, back-substitution :337-352). Whether the on/off difference is visible at print precision is unknown until the run (task 29.3.2); the dry pair is discriminating for fixed heads (README).

## Fortran lines the fixtures depend on

- Input: `INPUT.FOR:51-52` (top code, `lVarBC`), `:54-61` (bottom record), `:63-67` (rTop/rBot read), `:104-115` (code rewrite, `kTOld/kBOld`).
- Bottom row: `WATFLOW.FOR:202-229` (:211 ConVh, :221-222 rBot, :224-229 ConTB and ThV).
- Top row: `:272-294` (:277 ConA+ConVh, :287-291 ConTA and ThV, :293 vTop, :294 `PT=PT-rTop`).
- Elimination and back-substitution: `:315-334`, `:337-352`.
- Inert here, verified by reading: `Shift` seepage/top-infiltration branches `:379`, `:393-394` (KodBot<0, TopInf=f); the saturated shortcut `:1117-1121` (needs `lSat`, false because h<0).

## Fields

NOD_INF.OUT: head, liquid_moisture, temperature_c. NOD_INF_V.OUT (vapor on only): con, con_lt, con_vh, con_vt, v_liquid, v_vapor, v_total, v_vap_iso, v_vap_term (format 140, `reference/vapor_fortran/README.md`). Control decks have vapor output marked unavailable, never zero. Times: 6, 24, 72, 240 h.

## Reused criteria (criteria.json, read only)

- Reusable as is: `fields.*` (case-independent tolerances: con, con_lt, v_vapor, ... relative 0.02, evidence class `pinned-fortran-trajectory`) and the `checks.*` forms. The discrimination rule form (`factor_of_tolerance` 2.0, `any_print_time_after_initial`) is reused as a pattern.
- Not reusable by name: every `discrimination.*` entry and both `*_vapor_fields_nonzero` entries are bound to existing case ids. No existing entry covers the new cases.

## New criteria entries needing user approval (D5, none written)

For each of the two on/off pairs: `closed_flux_head`, `closed_flux_liquid_moisture`, `closed_flux_temperature_c` and `flux_known_*` equivalents (discrimination, factor 2.0, rule `any_print_time_after_initial`), and the two new case ids added to a `vapor_fields_nonzero`-style list (fields con_vh, con_vt, v_vapor, v_vap_iso, v_vap_term). Alternative: report on/off differences without a gate. Adding decks themselves is not a criteria change.

## Fortran-side balance

For a closed column, integrated liquid plus vapor content should be conserved (zero-flux deck) or change by the net imposed flux times elapsed time (known-flux deck). Inputs available from NOD_INF/NOD_INF_V: theta and fluxes per node. The vapor content is not printed (README: no direct vapor-content observations). `BALANCE.OUT` is not in the runner's output list (`fixture.json` outputs); using it would be a runner change. Print precision (theta f6.4) limits this check; the Rust balance check (29.3.1) is the primary balance evidence.

## Scope to be amended for Wave 3 (not part of 29.1.1)

- `tools/run_vapor_reference.py:184-186` hard-codes the allowed case set; the four new ids must be added (producer refuses unknown ids, :187-188; refuses to overwrite fixtures, :103-104). Task 29.3.2 scope must include this file.
- `tools/compare_vapor_reference.py` and `tools/test_*` for the runner/comparator: validation and discrimination bind to case names (`_dry_check`, `_nonzero_checks`); scope amendment needed if new entries are approved. Not read in scope.
- Four new case directories under `reference/vapor_fortran/cases/` and four fixtures under `reference/vapor_fortran/fixtures/` (29.3.2).
- `reference/vapor_fortran/README.md` (describes the decks) may need an update.

2026-11-14 amendment: the initial head is -100 cm, not -1e4 cm. The -1e4 cm start diverged with vapor on; the -100 cm start (candidate D) converged on both closed-pair decks (`planning/tasks/29.4.1.report.md`). The start is no longer dry, so the fixture does not cover dry-regime vapor behaviour.

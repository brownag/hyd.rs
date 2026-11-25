# Execution Plan 33: reported head-end boundary fluxes for the met path (G25)

Status: APPROVED 2026-11-24.
Repo: hyd.rs. Branch `feat/tlevel-head-end-flux`, stacked on `feat/met-run-path` at `c7c294c` (D-10). Tracked planning (D-5).
Source goal: [GOALS.md](GOALS.md) Circled G25 (`c7c294c`).

## Goal & Scope

**Goal.** The met vapor path reports the end boundary fluxes vTop and vBot the way the pinned Fortran prints them in `T_LEVEL.OUT`. These are the `TLInf` values (`OUTPUT.FOR:25-54`): the interior face flux plus the half-cell storage and sink terms, with vapor terms. They are computed from the accepted state every step, in both top modes. The Rust values match a pinned-Fortran `T_LEVEL.OUT` sidecar for `vapor_met_bare_soil_on`, per step and cumulatively, within bounds the user approves at a gate after measurement. The head-type ends no longer rely on the solver's 0.0 fallback for reported flux.

**In scope.**
- ADR 0023: reported end fluxes follow the `TLInf` output form. The solver's `total_face_fluxes` end fallback is kept and documented.
- Runner: a `--tlevel` mode in `tools/run_vapor_reference.py` that parses `T_LEVEL.OUT` (format 170) into `fixtures/<case>_tlevel.json`.
- The pinned `vapor_met_bare_soil_on_tlevel.json` sidecar, produced through the runner with the fixture's build.
- Rust: a pure `TLInf` vT/vB function in `heat/vapor/output.rs`.
- The test driver records the reported fluxes. `vapor_met_balance` uses the `src/` function in place of its local `net_vtop`.
- A measurement report, then a user bounds gate.
- `criteria.json` v8 entry; criteria hash rebound at all pin sites; asserting per-step and cumulative parity tests.
- Record G25 as done.

**Out of scope (explicit).**
- Any edit to `water_solver.rs` or `atmospheric.rs`. The `total_face_fluxes` 0.0 end fallback and `trial_v_top` (the `Shift` switching form) stay as they are (D-1).
- G20 (energy balance). The choice of which vTop `UpdateEnergy` reads (`HYDRUS.FOR:462`, before `TLInf` at `:530`) is recorded for G20, not decided here.
- The water-only (non-vapor) path and `T_LEVEL.OUT` for any other case.
- G31, G27 and the main met fixture (`vapor_met_bare_soil_on.json`, its pinned `FIXTURE_SHA256`).
- New dependencies (ADR 0011).

**Verbatim user decisions.**
- Flux locus: "Port TLInf as output fn".
- Fixture: "New sidecar fixture".
- Bounds: "Measure first, then gate".
- Bound home: "criteria.json entry".
- Balance: "Reuse src fn + cumulative check".
- Branch: "New branch, stacked now".
- Settings: "Carry over unchanged".

## Decision log

- **D-1 (2026-11-24, user):** vTop/vBot are a port of the `TLInf` formula (`OUTPUT.FOR:25-54`) as a pure post-step function in `heat/vapor/output.rs`. `water_solver.rs` and `atmospheric.rs` are not edited, so heads are bit-identical by construction. G25's wording changes from "the water solver reports" to "the met path reports". ADR 0023 records this.
- **D-2 (2026-11-24, user):** Fortran `T_LEVEL.OUT` data goes into a new sidecar, `reference/vapor_fortran/fixtures/vapor_met_bare_soil_on_tlevel.json`, following the `<case>_balance.json` precedent. The main fixture, its `FIXTURE_SHA256` and its evidence are untouched.
- **D-3 (2026-11-24, user):** Measure first, then gate. Wave 3 measures and reports. The user approves bounds at gate G-1. Wave 4 adds the asserting tests. Bounds are never tuned after a failure.
- **D-4 (2026-11-24, user):** The approved bounds become a `criteria.json` v8 `checks` entry under the D-9 process: a revision note, and `CRITERIA_SHA256` rebound at every pin site in one task (precedent `d2d1f32`).
- **D-5 (carried):** Planning artifacts are tracked. `docs(planning):` commits are separate from code.
- **D-6 (carried):** Up to 3 workers per wave, worktrees when a wave has more than one task, full suites serial at wave end.
- **D-7 (carried, AGENTS.md):** Wave-end verification: `cargo test --workspace --no-fail-fast`, clippy `-D warnings`, fmt check, Python `tools` unittest.
- **D-8 (ADR constraints):**
  - ADR 0010 (source order) applies to the `TLInf` port: terms are added in the source sequence.
  - ADR 0015 applies: the port follows the f64 precedent of `observe_vapor`, and single-precision differences are measured, not emulated.
  - ADR 0011 applies: no new dependencies.
  - ADR 0022 is untouched.
  - New ADR 0023 (32-style, Accepted).
- **D-9 (2026-11-24, user):** `vapor_met_balance` replaces the test-local `net_vtop` with the `src/` function. Its doc comment still says interior conservation. The new cumulative evidence (Rust ΣvTop·dt and ΣvBot·dt against Fortran `sum(vTop)` = CumQ(3) and `sum(vBot)` = CumQ(5)) lives in the new parity test under the same gated entry.
- **D-10 (2026-11-24, user):** Branch `feat/tlevel-head-end-flux` from `feat/met-run-path` at `c7c294c`. After the user merges G32 into `main`, the orchestrator rebases onto `main` between waves, only with the tree clean and no worker running. The G25 PR targets `main` and is a user action.
- **D-11 (carried):** Haiku for docs tasks and Sonnet for code tasks. Escalation to Opus per the ladder.
- **D-12 (orchestrator, acceptance):** Pre-existing outputs are unchanged. 33.2.2 dumps every pre-existing `MetRun` and `StepTrace` field before and after (plan 32 D-17 protocol) and requires byte equality. Only the new fields may differ.

## Recon facts (anchors)

- **Fortran report form:**
  - `TLInf` at `src_reference/OUTPUT.FOR:3`.
  - vT at `:31-32`: liquid face, top half-cell storage and sink.
  - vB at `:36-37`: liquid face, plus bottom half-cell storage and sink.
  - Vapor terms at `:44-51`: ConVh and ConVT faces, plus the vapor half-cell storage `(ThVNew-ThVOld)*dx/2/dt` at both ends.
  - `vTopW` (liquid part) at `:43` and `vTopV` (vapor part) at `:52`.
  - The ConLT terms (`:39-42`) need `lWTDep`, which is rejected in Rust (G23), so they are omitted.
  - `fRE=1`, no dual porosity, no centrifuge.
- **Write and cadence:**
  - `write(71,170)` at `OUTPUT.FOR:193`: `t, rTop, rRoot, vTop, vRoot, vBot, CumQ(1..5), hNew(N), hRoot, hNew(1), vRunOff, CumQ(6), Volume, CumQ(7), CumQ(8), TLevel, CumQ(11), SnowLayer`.
  - Format 170 at `:286` is `f13.4,11e13.5,2e13.5,5e13.5,i7,e13.5,f11.3`, which gives about 5 significant digits. The header is format 130 at `:262-271`.
  - The met deck has `ShortO=f` and `nPrintSteps=1` (`SELECTOR.IN:10`, `:34`), so there is one row per step: 2400 rows.
- **Switching form (not changed):**
  - `WATFLOW.FOR:400-407` (`Shift`), ported as `trial_v_top` (`atmospheric.rs:501`) and carried in `VaporAtmosphericState.v_top`.
  - It has the same terms as vT except the vapor storage term.
- **Solver fallback (not changed):** `water_solver.rs:584`, `measured_flux().unwrap_or(0.0)`. `upper_for` at `atmospheric.rs:484` passes flux 0.0 in head mode.
- **Rust inputs available after a step:** `VaporStepReport` (`heat/vapor/driver.rs:105`) has these fields:
  - `liquid_conductivities`, `vapor_head_conductivities`, `vapor_thermal_conductivities`;
  - `liquid_contents_old/new`, `vapor_contents_old/new`;
  - `timestep`, `temperatures`.

  `VaporState::snapshot()` has `heads` and `temperatures`.
- **Reference implementation for the port:** `observe_vapor` / `observe_step` (`heat/vapor/output.rs:87`, `:194`, 234 lines). It matched `NOD_INF_V.OUT` with the same face formulas and conventions (bottom-first index, `gravity`).
- **Test driver:**
  - `tests/support/vapor_met.rs` (`StepTrace`, the `step_atmospheric` call at about `:505`).
  - `tests/vapor_met_balance.rs`: `net_vtop` at `:58`, its use at `:128`, and `net_flux_mode` at `:72` (a negative control).
- **Runner:**
  - `tools/run_vapor_reference.py` (377 lines). `run_case` at `:251` has the sidecar precedent `--balance`, which writes `<case>_balance.json`.
  - The overwrite guards at `:259-262` refuse to write if the main fixture exists and the sidecar mode is off.
  - `main` is at `:348`, the case allowlist at `:363-367`, `parse_balance` at `:104`.
  - Tests are in `tools/test_run_vapor_reference.py` (473 lines). The balance parser tests from `:216` are the pattern.
- **Fixture build:** the met fixture's `executable_sha256` is `87969711f9241b836ad04e24c1987c46e58de2c035d09270ef5c0a33b0519638`. That build exists only at `/home/andrew/.gemini/antigravity-cli/worktrees/82d82ee1-e414-4952-996d-921e46861bbf/subagent-Worker-31-2-1-met-harness-self-7c19db32/.development/31.2.1/reference-build/` (checked 2026-11-24). Its manifest `output_sha256` binds `NOD_INF.OUT` and `NOD_INF_V.OUT`. The build procedure is in `reference/fortran_reference/README.md`.
- **Criteria:**
  - `reference/vapor_fortran/criteria.json` (schema `vapor-criteria-v2`). The `checks` entries are `{absolute, assumptions, relative, scale, unit}`; see `water_balance_interior`.
  - The validator is `tools/compare_vapor_reference.py:123` (`validate_criteria`, `CHECK_KEYS`).
  - `CRITERIA_SHA256 = "b03047a6…"` is pinned in `tests/vapor_met_parity.rs:31`, `vapor_fortran_parity.rs:51`, `vapor_heat_boundary_parity.rs:51`, `vapor_nonuniform_parity.rs:56` and `vapor_nonuniform_timestep_controller.rs:65`. It is also cited in `planning/master/M6_VAPOR_G22_3_ACCEPTANCE.md`.
  - Rebind precedent: `d2d1f32`.
- **Baseline (2026-11-24, `c7c294c`):** workspace tests 704 passed, 0 failed, 4 ignored. Clippy and fmt clean. Python 504 OK. Met parity 5 and balance 3 pass in release.
- **Next ADR number:** 0023.

## Hazards

- **Sign conventions.** Fortran vT is positive upward (out of the soil at the top) on an upward `x`. Rust indexes bottom first. Follow `observe_vapor`'s conventions and report any systematic sign or offset difference. Never flip a sign to make a measurement fit.
- **Which Con.** `TLInf` uses the `Con` array from the last Picard iteration. Use the report's stored conductivities, as `observe_step` does (its `NOD_INF_V` parity proves the pairing). Do not re-evaluate conductivities from the accepted heads.
- **Runner overwrite guards.** `run_case` refuses if the main fixture exists unless a sidecar mode is set. The `--tlevel` mode must generalise the guard, not delete it, and must still refuse to overwrite an existing sidecar.
- **Build identity.** The sidecar must come from the fixture's executable (`87969711…`), and its `NOD_INF.OUT`/`NOD_INF_V.OUT` hashes must equal the main fixture manifest's. A rebuild with a different hash, or a hash mismatch, is a STOP.
- **Criteria rebind** touches 5 test files outside the met chain. Only the hash constant line changes in each. The Python validator must accept the new entry; run the tools suite.
- **Bit-identity.** 33.2.2 adds fields to `StepTrace`. All pre-existing fields stay byte-identical (D-12).
- **Forbidden:** `water_solver.rs`, `atmospheric.rs`, `heat/vapor/driver.rs`, the main met fixture, and `reference/` except the one new sidecar (33.2.1) and `criteria.json` (33.4.1).

## Waves

| Wave | Task | Depends on | Files (must edit / create) | Goal |
| --- | --- | --- | --- | --- |
| 1 | 33.1.1 | none | `docs/decisions/0023-*.md` (create), `docs/decisions/README.md` | ADR 0023: reported end fluxes follow `TLInf` |
| 1 | 33.1.2 | none | `tools/run_vapor_reference.py`, `tools/test_run_vapor_reference.py` | Runner `--tlevel` mode and `T_LEVEL.OUT` parser |
| 1 | 33.1.3 | none | `crates/hydrus-core/src/heat/vapor/output.rs` | `TLInf` vT/vB port as a pure function |
| 2 | 33.2.1 | 33.1.2 | `reference/vapor_fortran/fixtures/vapor_met_bare_soil_on_tlevel.json` (create) | Produce the pinned sidecar through the runner |
| 2 | 33.2.2 | 33.1.3 | `crates/hydrus-core/tests/support/vapor_met.rs`, `crates/hydrus-core/tests/vapor_met_balance.rs` | Record reported fluxes; balance uses the `src/` function |
| 3 | 33.3.1 | 33.2.1, 33.2.2 | `crates/hydrus-core/tests/vapor_met_tlevel_parity.rs` (create), `planning/master/M6_VAPOR_G25_TLEVEL_MEASUREMENT.md` (create) | Structural checks and measurement report |
| gate | G-1 | 33.3.1 | none | **User approves bounds** (values, scale, unit) |
| 4 | 33.4.1 | G-1 | `reference/vapor_fortran/criteria.json`, `tests/vapor_met_tlevel_parity.rs`, the 5 `CRITERIA_SHA256` pin files | v8 entry, hash rebind, asserting tests |
| 5 | 33.5.1 | 33.1.1, 33.4.1 | `planning/GOALS.md`, `planning/master/M6_VAPOR_G22_3_ACCEPTANCE.md`, `planning/master/M6_VAPOR_COVERAGE.md` | Record G25 done |

Wave 1 and wave 2 file sets are pairwise disjoint. 33.1.3 does not touch `lib.rs`: `heat::vapor::output` is already a public path, so the worker verifies that and reports if it is not.

## Task specs

### 33.1.1 ADR 0023: reported end fluxes follow the `TLInf` output form
- Depends on: none. Docs only (Haiku).
- Spec: write ADR 0023 in the format of `docs/decisions/0022-*.md`. Status Accepted, date 2026-11-24, source Plan 33 D-1, D-2 and D-4.
  - Context: Fortran has two vTop forms. The switching form is `WATFLOW.FOR:400-407`, ported as `trial_v_top` (`atmospheric.rs:501`). The reported form is `TLInf` (`OUTPUT.FOR:25-54`), which adds the vapor storage term and also defines vB. Rust head-type ends report a 0.0 fallback in `total_face_fluxes` (`water_solver.rs:584`; `upper_for` `atmospheric.rs:484`).
  - Decision: the met path reports vTop/vBot through a pure port of `TLInf` in `heat/vapor/output.rs`, computed from the accepted step. The solver and the switching form are unchanged, and `total_face_fluxes` end entries are not a reported boundary flux. Evidence is a runner-produced `T_LEVEL.OUT` sidecar and a `criteria.json` v8 entry approved after measurement.
  - Consequences:
    - Heads stay bit-identical.
    - The water-only path has no reported head-end flux yet.
    - G20 must decide which vTop `UpdateEnergy` reads: `HYDRUS.FOR:462` runs before `TLInf` at `:530`.
    - Revisit if a caller needs solver-level head-end fluxes.
  - Add the README table row.
- Must edit: `docs/decisions/0023-report-end-boundary-fluxes-in-the-tlinf-output-form.md` (create) and `docs/decisions/README.md`. No code.
- Anchors: the README table (row 0022), `OUTPUT.FOR:25-54` and `:193`, `WATFLOW.FOR:400-407`, `atmospheric.rs:484` and `:501`, `water_solver.rs:584`, `HYDRUS.FOR:462` and `:530`.
- Acceptance: `test -f docs/decisions/0023-*.md && grep -q "0023" docs/decisions/README.md`. `git diff --quiet -- crates/ reference/ tools/`.

### 33.1.2 Runner `--tlevel` mode
- Depends on: none (Sonnet).
- Spec:
  - Add `parse_t_level(text)` to `tools/run_vapor_reference.py`. It parses `T_LEVEL.OUT` format 170 rows after the format-130 header into dicts with named columns: `time, r_top, r_root, v_top, v_root, v_bot, sum_r_top, sum_r_root, sum_v_top, sum_v_root, sum_v_bot, h_top, h_root, h_bot, run_off, sum_run_off, volume, sum_infil, sum_evap, t_level, cum_wtrans, snow_layer`. Read the real header and any end marker from `OUTPUT.FOR` and confirm the column order against `:193`.
  - Reject truncated or short rows, non-finite values, duplicate or non-increasing times, a `t_level` sequence not increasing by 1, a missing header, and an empty table. Raise `ValueError` with a message, as `parse_balance` does.
  - Add `--tlevel` (help: "parse T_LEVEL.OUT and write <case>_tlevel.json"). Put `T_LEVEL.OUT` in the checked outputs when the flag is set.
  - Write `fixtures/<case>_tlevel.json`:
    - `schema_version 1`, `case_id`, `source_matched`;
    - `evidence_class "pinned-fortran-t-level"`;
    - `output_format` (the format 170 string);
    - `time_points` and `rows`;
    - a manifest like the balance one, whose `output_sha256` covers `T_LEVEL.OUT`, `NOD_INF.OUT` and `NOD_INF_V.OUT`, so the sidecar binds to the main fixture's run.
  - Generalise the overwrite guards so a sidecar mode with an existing main fixture is allowed, and an existing `_tlevel.json` is refused. `--balance` behaviour is unchanged.
- Must edit: `tools/run_vapor_reference.py`, `tools/test_run_vapor_reference.py`. Do not run the Fortran executable. Do not touch `reference/`. Scratch goes in `.development/33.1.2/`.
- Anchors: `parse_balance` (`:104`), `run_case` (`:251-347`, guards `:259-262`), `main` (`:348`). Test pattern at `test_run_vapor_reference.py:216` onward. Format at `OUTPUT.FOR:130`/`:262-271`, `:170`/`:286`, `:193`.
- Acceptance:
  - New tests: the parser on a synthetic format-170 sample (values round-trip), each rejection, and the guard logic (use a temp directory or monkeypatch `FIXTURES`; never write to the real `reference/`).
  - `python3 -m unittest tools.test_run_vapor_reference` passes, and `python3 -m unittest discover -s tools -p 'test_*.py'` passes.
  - `git diff --quiet -- reference/ crates/`.

### 33.1.3 `TLInf` vT/vB port
- Depends on: none (Sonnet).
- Spec:
  - Add `pub struct TLevelFluxInput<'a>` to `heat/vapor/output.rs` with: `coordinates`, `heads`, `temperatures`, `liquid_conductivities`, `vapor_head_conductivities`, `vapor_thermal_conductivities`, `liquid_contents_old/new`, `vapor_contents_old/new`, `sink`, `timestep`, `gravity`.
  - Add `pub struct TLevelFluxes { v_top, v_bot, v_top_liquid, v_top_vapor }` (`vTop`, `vBot`, `vTopW`, `vTopV`).
  - Add `pub fn t_level_fluxes(&TLevelFluxInput) -> Result<TLevelFluxes, VaporError>`. Port `OUTPUT.FOR:25-54` term by term in source order (D-8) with `fRE=1`. ConLT, dual-porosity and centrifuge terms are omitted, with a doc comment citing G23 and the rejected selectors. Validate the lengths and `n >= 3`, as `observe_vapor` does.
  - Add `pub fn t_level_fluxes_for_step(state: &VaporState, report: &VaporStepReport, coordinates, sink, gravity)`, mirroring `observe_step`'s consistency checks.
  - Use the report's stored conductivities (see Hazards), with signs and conventions as in `observe_vapor`.
- Must edit: `crates/hydrus-core/src/heat/vapor/output.rs` only. Do not touch `water_solver.rs`, `atmospheric.rs`, `driver.rs`, `lib.rs` or `tests/`. Scratch goes in `.development/33.1.3/`.
- Anchors: `observe_vapor` (`output.rs:87`), `observe_step` (`:194`), `VaporStepReport` (`driver.rs:105`), `OUTPUT.FOR:25-54`. Reference implementation: the face formulas inside `observe_vapor`.
- Acceptance: unit tests in `output.rs`:
  - A 3-node and a 5-node profile with hand-written expected values in the same operation order (exact `assert_eq!`).
  - With zero vapor conductivities and vapor storage, `v_top == v_top_liquid` and `v_top_vapor == 0`.
  - `v_top == v_top_liquid + v_top_vapor` to within one rounding step, documented.
  - Length-mismatch and too-few-node errors.
  - Commands: `cargo test -p hydrus-core --lib heat::vapor::output` passes. Clippy `-D warnings` clean.

### 33.2.1 Produce the `T_LEVEL.OUT` sidecar
- Depends on: 33.1.2 (Sonnet).
- Spec:
  - Get a reference executable with sha256 `87969711…` (the full hash is in Recon). Copy the 31.2.1 build directory (path in Recon) into `.development/33.2.1/reference-build/` and verify the hash. If the source path is gone, stage and build per `reference/fortran_reference/README.md` and compare the hash. Either mismatch is a STOP.
  - Run `python3 tools/run_vapor_reference.py --case vapor_met_bare_soil_on --tlevel --build-dir .development/33.2.1/reference-build --task-base .development/33.2.1/runs`.
  - Verify all of the following with a short script in `.development/33.2.1/` (its output goes in the report):
    - the sidecar `input_sha256` equals the main fixture's;
    - `output_sha256` for `NOD_INF.OUT` and `NOD_INF_V.OUT` equals the main fixture manifest's;
    - `executable_sha256` matches;
    - there are 2400 rows, `t_level` runs 1..2400, and the last time is 240.
  - Commit only the sidecar.
- Must edit: `reference/vapor_fortran/fixtures/vapor_met_bare_soil_on_tlevel.json` (create). Nothing else under `reference/`. No runner edits: a runner bug is a STOP and a 33.1.2 defect. Scratch and runs go in `.development/33.2.1/`.
- Anchors: the runner `--tlevel` from 33.1.2 (read its `STATE_33.md` report), and `reference/vapor_fortran/README.md:33-38`.
- Acceptance: the verification script passes. `git status` shows only the sidecar (plus planning files). `git diff --quiet -- reference/vapor_fortran/criteria.json reference/vapor_fortran/fixtures/vapor_met_bare_soil_on.json`.

### 33.2.2 Record reported fluxes in the met trace
- Depends on: 33.1.3 (Sonnet).
- Spec:
  - Before editing, run the plan 32 D-17 dump protocol over every pre-existing `MetRun` and `StepTrace` field: a scratch integration test, deleted before commit, writing `.development/33.2.2/before.json`.
  - Add `StepTrace` fields `v_top_reported`, `v_bot_reported`, `v_top_liquid_reported` and `v_top_vapor_reported`. Fill them from `t_level_fluxes_for_step` after each `step_atmospheric`, with zero sink and the run's gravity.
  - In `vapor_met_balance.rs`, replace the local `net_vtop(s, w, false)` construction with the reported fields: bottom inflow minus top outflow, with signs checked against `net_vtop`. Every existing assertion, tolerance and doc claim stays the same. Negative-control helpers (`drop_vapor_storage`, `net_flux_mode`) may keep their test-local forms.
  - Dump `after.json` with the new fields excluded, and `cmp`.
- Must edit: `crates/hydrus-core/tests/support/vapor_met.rs`, `crates/hydrus-core/tests/vapor_met_balance.rs`. Do not touch `src/`, `vapor_met_parity.rs` or `reference/`. Scratch goes in `.development/33.2.2/`.
- Anchors: `StepTrace` and the `step_atmospheric` call in `vapor_met.rs` (about `:505-560`), `vapor_met_balance.rs:58-135`.
- Acceptance:
  - `cmp` of `before.json` and `after.json` succeeds.
  - `cargo test -p hydrus-core --release --test vapor_met_parity --test vapor_met_balance` gives 5 + 3 passing.
  - `grep -n "fn net_vtop" crates/hydrus-core/tests/vapor_met_balance.rs` either finds nothing or only a negative-control variant documented as such.
  - Clippy clean.

### 33.3.1 Structural checks and measurement report
- Depends on: 33.2.1, 33.2.2 (Sonnet).
- Spec:
  - Create `tests/vapor_met_tlevel_parity.rs` (`#[path = "support/vapor_met.rs"] mod vapor_met;`). It:
    - pins the sidecar SHA-256 as a constant;
    - checks that its `input_sha256` matches the deck files, as `vapor_met_parity.rs:365-373` does;
    - asserts structure only: 2400 rows, `time_points` equal the Rust step times within 1e-9, `t_level` 1..2400.
  - Add a measurement test that prints and does not assert. Report:
    - per step, `|ΔvTop|` and `|ΔvBot|`, absolute and relative, split by top mode;
    - the worst 10 steps;
    - the values at the rain record and at the 3 switch steps (42.7, 120.1, 200.3 h);
    - cumulative Rust ΣvTop·dt against `sum_v_top` and ΣvBot·dt against `sum_v_bot`;
    - the `e13.5` print floor (half a unit in the 5th significant digit);
    - for reference, the Rust switching-form `v_top` against the Fortran vTop.
  - Write `planning/master/M6_VAPOR_G25_TLEVEL_MEASUREMENT.md` with the tables, the observed sign convention, and 2-3 bound options (absolute, relative, scale) with their pass margins and a recommendation.
  - A systematic sign, offset or factor error is reported as a finding with evidence and no bound proposed (STOP).
- Must edit: `crates/hydrus-core/tests/vapor_met_tlevel_parity.rs` (create), `planning/master/M6_VAPOR_G25_TLEVEL_MEASUREMENT.md` (create). Do not touch `criteria.json`, `src/` or other tests.
- Anchors: `vapor_met_parity.rs:340-375` (fixture authentication), the 33.2.1 and 33.2.2 reports in `STATE_33.md`, and the `kod_changes()` switch times in `vapor_met_balance.rs` `run_geometry_and_switch_guards`.
- Acceptance: `cargo test -p hydrus-core --release --test vapor_met_tlevel_parity -- --nocapture` passes, and its output appears in the report. The measurement doc exists. Clippy clean.

### Gate G-1 (user): approve vTop/vBot bounds
- The orchestrator presents the measurement doc and stops. The user approves the values, scale and unit of the v8 entry (or several entries), or sends the plan back. No wave 4 worker is spawned before approval. The decision is recorded as D-13 in this plan.

### 33.4.1 Criteria v8 entry, hash rebind and asserting tests
- Depends on: G-1 (Sonnet).
- Spec:
  - Follow `d2d1f32` exactly (`git show d2d1f32`). Add the approved `checks` entry or entries (for example `met_t_level_end_flux`) to `criteria.json`, with `assumptions` naming the `TLInf` form, the sidecar and the `e13.5` print floor.
  - Extend `revision_note` with v8 and update the approval fields the way earlier revisions did.
  - Rebind `CRITERIA_SHA256` in the 5 pin files (only the constant line).
  - In `vapor_met_tlevel_parity.rs`, add a `CRITERIA_SHA256` pin and asserting tests for per-step vTop/vBot and the cumulative sums under the entry. Keep the measurement test.
  - Update the hash cited in `M6_VAPOR_G22_3_ACCEPTANCE.md` only if `d2d1f32`'s precedent did the same for its docs; otherwise list it under Discovered.
- Must edit: `reference/vapor_fortran/criteria.json`, `crates/hydrus-core/tests/vapor_met_tlevel_parity.rs`, and the 5 pin files `tests/vapor_met_parity.rs`, `vapor_fortran_parity.rs`, `vapor_heat_boundary_parity.rs`, `vapor_nonuniform_parity.rs`, `vapor_nonuniform_timestep_controller.rs` (constant line only). No `src/`.
- Anchors: `criteria.json` `checks.water_balance_interior`, `revision_note` at about `:495`, `tools/compare_vapor_reference.py:123` (`validate_criteria`, `CHECK_KEYS`), `d2d1f32`.
- Acceptance:
  - `cargo test -p hydrus-core --release --test vapor_met_tlevel_parity --test vapor_met_parity --test vapor_fortran_parity --test vapor_heat_boundary_parity --test vapor_nonuniform_parity --test vapor_nonuniform_timestep_controller` passes.
  - `python3 -m unittest discover -s tools -p 'test_*.py'` passes.
  - `grep -rn "b03047a6" crates/` is empty.
  - Clippy clean.
  - A failing assertion under the approved bounds is a STOP; bounds are never adjusted.

### 33.5.1 Record G25 as done
- Depends on: 33.1.1, 33.4.1 (Haiku). Docs only.
- Spec:
  - `GOALS.md`: move G25 to Completed, reworded per D-1 ("the met path reports ..."), with commit IDs from the `STATE_33.md` board and the v8 entry name. Clear the Circled Goal, following the `30a96ed` precedent, with G20 named as the next candidate. Add to G20 the note that `UpdateEnergy`'s vTop source must be decided (ADR 0023). Update the dependency graph and the Source State header.
  - `M6_VAPOR_G22_3_ACCEPTANCE.md`: add a dated update saying that exception 1 (head-end boundary flux checked as interior conservation only) is resolved by plan 33.
  - `M6_VAPOR_COVERAGE.md`: mark that exception in the met row as resolved, keeping the original wording visible.
- Must edit: the three listed files. No code.
- Acceptance: `grep -n "G25" planning/GOALS.md` shows it under Completed. `git diff --quiet -- crates/ reference/ tools/`.

## Escalation and gates

- The wave-orchestration escalation ladder applies unchanged.
- **Stop conditions (report to the user, never work around):**
  - a reference executable hash mismatch, or sidecar output hashes that disagree with the main fixture (33.2.1);
  - a byte-identity failure (33.2.2);
  - a systematic sign, offset or factor mismatch in the measurement (33.3.1);
  - a failing assertion under the approved bounds (33.4.1);
  - any task needing `water_solver.rs`, `atmospheric.rs`, `driver.rs` or an unlisted `reference/` file.
- **Gate G-1** is a hard user stop between waves 3 and 4.
- Wave-end verification is serial (D-7). A wave is marked done only after the full suites are green.

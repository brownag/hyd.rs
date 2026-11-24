# Execution Plan 31: close G22.2 and deliver G22.3 with meteorology

Status: DRAFT, pending user approval (plan-wave Phase 6).
Repo: hyd.rs (branch `feat/vapor-flux-fixedhead`). Tracked planning (decision D-T).
Source goal: [GOALS.md](GOALS.md) Circled G22.2, plus Sequenced G22.3 and the radiation part of G20.

## Goal & Scope

**Goal.** Close the G22.2 exit (flux/FixedHead matrix, C5 closed-pair criteria proposal), then deliver G22.3 (atmospheric water switching with vapor) including meteorological forcing, with the net radiation terms (Rn, Rns, Rnl) computed in Rust. Each piece has pinned-Fortran provenance.

**In scope.**
- G22.2 closure: correct the stale coverage row (`M6_VAPOR_COVERAGE.md:31` says the kernel rejects flux/FixedHead; commit `880c6c2` accepts it); record the dry-regime divergence as a documented record.
- G22.2-C5: draft closed-pair criteria as a proposal. No `criteria.json` edit without user approval.
- G22.3: atmospheric switching with vapor on, `lMeteo=true`, evaporation from met input.
- Radiation: Rn, Rns, Rnl computed from meteorology (the part of G20 that evaporation needs).

**Out of scope (explicit).**
- The rest of G20: the energy update (`lEnBal`, `UpdateEnergy`). It becomes its own orchestration after G20 is split and reprioritized.
- G23 (temperature-dependent liquid coupling, `lWDep`/`lWTDep`). Deferred.
- G22.5 and G22.6 (enhancement off, Campbell path).
- Single-flux forms beyond the pairs already accepted. Matrix recorded, not implemented here.
- Any dependency addition (ADR 0011 needs an approved plan amendment).

**Verbatim user constraints.**
- "all" (scope answer), read as: close G22.2, draft C5 for approval, and include G22.3.
- "Full G22.3 with meteorology".
- "Pinned Fortran runs" for met evidence.
- "accept it as input and ideally we should get that fully implemented so we can include both" and "Split: radiation in 31".
- "Track" for planning artifacts.
- "Up to 3, worktrees for same-repo tasks".

## Decision log

- **D-1 (2026-11-15, user):** Scope = close G22.2, draft C5 as a proposal, include G22.3.
- **D-2 (2026-11-15, user):** G22.3 is full, with meteorology. Not contract-only.
- **D-3 (2026-11-15, user):** Met evidence comes from pinned Fortran runs with `lMeteo=true`, following the vapor harness.
- **D-4 (2026-11-15, user):** Radiation (Rn, Rns, Rnl) is computed in 31. The rest of G20 is split out.
- **D-5 (2026-11-15, user):** Planning artifacts are tracked, with `docs(planning):` commits separate from code.
- **D-6 (2026-11-15, user):** Up to 3 workers per wave; worktrees for same-repo tasks; full suites serial at wave end.
- **D-7 (default, AGENTS.md):** Full verification is serial at wave end: `cargo test --workspace`, clippy, fmt, Python tools.
- **D-8 (ADR constraints):** ADR 0010 (preserve source operation order and precision) and ADR 0015 (precision-faithful ports in compat kernels) apply to all Rust ports of Fortran met/radiation code. ADR 0011 (no new dependencies without a plan amendment) applies. No ADR is superseded.
- **D-10 (2026-11-15, user, Gate A Q3/Q4):** `hCritA` with vapor follows the Fortran source. The atmospheric step decomposes into the existing Flux/Head pairs; no new `VaporBoundaryPair` variant. **Fortran parity constraint:** the decomposition must reproduce the Fortran switching order, switch test, and flux/head selection per step (ADR 0010). It is accepted only if `vapor_met_parity` (31.4.1) matches the pinned Fortran trajectory. If parity cannot hold, stop and re-open this decision; do not adjust bounds. Tasks 31.3.2 and 31.4.1 must cite this.
- **D-11 (2026-11-22, user, Magic Wand refresh):** (a) Add task 31.3.3, the `Meteo.in` reader (`meteo::config`, `meteo::series`). The contract names these modules, but no task owned them, and 31.4.1 needs them. (b) 31.2.1 case name is `vapor_met_bare_soil_on`, with 240 h and 10 daily records. (c) 31.2.1 is the critical-path task: it records the Fortran non-convergence count and the upper-boundary flux/head switch count. Fortran divergence, or zero switches, stops at a gate. Do not tune the deck without approval.
- **D-12 (2026-11-23, orchestrator, user approved running wave 4):** Wave 4 runs in sequence: 31.4.1, then 31.4.2. 31.4.1 is the D-10 gate and builds the shared Rust met-run driver in a test-support module, and 31.4.2 reuses it. 31.4.1 adds no `criteria.json` entries (D-9). If no existing criterion applies to the met case, it reports the measured differences and the bounds go to the user. First execution step: a dry-head probe at -1000 cm, since the 31.3.2 fixture hit singular pivots at dry heads.
- **D-9 (gate):** C5 criteria tolerances require explicit user approval before any `criteria.json` edit (GOALS.md D5).

## Recon facts (anchors)

- Rust atmospheric module (water-only): `crates/hydrus-core/src/atmospheric.rs` (507 lines), `forcing.rs` (94 lines), `water_state.rs`. The vapor path does not feed it yet.
- Kernel boundary acceptance: `crates/hydrus-core/src/heat/vapor/mod.rs` (`VaporBoundaryPair`, ~line 229-320). Flux/FixedHead now accepted (`880c6c2`).
- Water solver: `crates/hydrus-core/src/water_solver.rs` (1621 lines). Entry: `solve_vapor_water_trial`, ~line 382-460.
- Fortran met source: `src_reference/INPUT.FOR` line 38 (lMeteo read), line 934 (`MeteoIn`: Latitude, Altitude, ShortWaveRadA/B, ...). `src_reference/TIME.FOR`, `src_reference/HYDRUS.FOR` (lMeteo references).
- Vapor reference harness: `tools/run_vapor_reference.py` (case allowlist ~line 318), `reference/vapor_fortran/cases/`, `reference/vapor_fortran/fixtures/`, `tools/compare_vapor_reference.py`.
- Parity suite baseline: `crates/hydrus-core/tests/vapor_fortran_parity.rs` (19 pass, after `880c6c2` and `5ed1df2`).
- Workspace baseline (2026-11-15): `cargo test --workspace` green, clippy `-D warnings` green, fmt clean, Python `tools` 499 OK.

## Hazards

- The Rust atmospheric module is water-only. Extending it with vapor changes the boundary-switch accounting. Reference: `M6_VAPOR_COVERAGE.md`.
- The landing-step artifact (`880c6c2`, 9.5e-12 h step) affects any observation divided by dt. Met runs add more print times; keep the landing loop at 1e-9 h.
- Fortran `REAL*4` precision in the vapor-coupled solve (H7 class). Expect source-precision rows; record them, do not widen bounds.
- Shared file: `water_solver.rs`. Two tasks in one wave must not both edit it.

## Waves

| Wave | Task | Repo | Depends on | Files (must edit) | Goal |
| --- | --- | --- | --- | --- | --- |
| 1 | 31.1.1 | hyd.rs | none | `planning/master/M6_VAPOR_COVERAGE.md`, `planning/master/M6_VAPOR_G22_2_ACCEPTANCE.md` | Fix stale flux/FixedHead row; record dry divergence |
| 1 | 31.1.2 | hyd.rs | none | `planning/master/M6_VAPOR_C5_PROPOSAL.md` (create) | Draft C5 criteria proposal (no criteria.json edit) |
| 1 | 31.1.3 | hyd.rs | none | `planning/master/M6_VAPOR_G22_3_CONTRACT.md` (create) | G22.3 + met contract: inputs, Fortran mapping, acceptance |
| GATE A | user | | 31.1.2, 31.1.3 | | Approve C5 tolerances and G22.3 contract |
| 2 | 31.2.1 | hyd.rs | 31.1.3 | `tools/run_vapor_reference.py`, `reference/vapor_fortran/cases/vapor_met_*` (create) | Met deck generator and pinned Fortran `lMeteo=true` runs |
| 2 | 31.2.2 | hyd.rs | 31.1.3 | `crates/hydrus-core/src/meteo/` (create), `crates/hydrus-core/src/lib.rs` | Port Rn, Rns, Rnl from `MeteoIn` (ADR 0010/0015) |
| 2 | 31.2.3 | hyd.rs | GATE A | `reference/vapor_fortran/criteria.json` | Add `flux_known_on_off_liquid_moisture` (factor 0.75, C5 Option M1) |
| 3 | 31.3.1 | hyd.rs | 31.2.2 | `crates/hydrus-core/src/meteo/evaporation.rs` (create), `crates/hydrus-core/src/meteo/mod.rs` | Port evaporation/ET from met input |
| 3 | 31.3.2 | hyd.rs | 31.1.3, 31.2.2 | `crates/hydrus-core/src/atmospheric.rs`, `crates/hydrus-core/src/forcing.rs` | Atmospheric switching accepts vapor (water-only behavior unchanged) |
| 3 | 31.3.3 | hyd.rs | 31.1.3, 31.2.1 (deck) | `crates/hydrus-core/src/meteo/config.rs`, `crates/hydrus-core/src/meteo/series.rs` (create) | `Meteo.in` reader with contract section 7 typed rejections (D-11) |
| 4 | 31.4.1 | hyd.rs | 31.2.1, 31.3.1, 31.3.2, 31.3.3 | `crates/hydrus-core/tests/vapor_met_parity.rs` (create), `reference/vapor_fortran/fixtures/` | Parity against pinned Fortran met trajectory |
| 4 | 31.4.2 | hyd.rs | 31.3.2 | `crates/hydrus-core/tests/vapor_met_balance.rs` (create) | Water balance check with met switching |
| 5 | 31.5.1 | hyd.rs | 31.4.1, 31.4.2, 31.2.3 | `planning/master/M6_VAPOR_COVERAGE.md`, `planning/master/M6_VAPOR_G22_3_ACCEPTANCE.md` (create), `GOALS.md` | Record G22.2 close and G22.3 acceptance |

Wave disjointness: wave 1 touches docs only. Wave 2 `31.2.1` (tools, cases), `31.2.2` (meteo module) and `31.2.3` (criteria.json) are disjoint. Wave 3 `31.3.1` (meteo/evaporation.rs), `31.3.2` (atmospheric.rs, forcing.rs) and `31.3.3` (meteo/config.rs, meteo/series.rs) are disjoint, except that 31.3.1 and 31.3.3 each add one `mod` line to `meteo/mod.rs`. The orchestrator merges those lines at wave end. Wave 4 `31.4.1` and `31.4.2` are disjoint test files. `water_solver.rs` is not edited by any task; if 31.3.2 needs it, it becomes a re-scope (escalation 4).

## Task specs

### 31.1.1 Fix G22.2 coverage matrix
- Depends on: none. Repo: hyd.rs.
- Spec: `M6_VAPOR_COVERAGE.md:31` says "Rust kernel still rejects it (`heat/vapor/mod.rs:301`)". That is stale. Update the row to: accepted and implemented (`880c6c2`), wet fixture parity passes (`vapor_flux_fixedhead_wet_on`), dry (-10000 cm) divergence recorded per `M6_VAPOR_FLUX_FIXEDHEAD_DESIGN.md`. Mirror the change in `M6_VAPOR_G22_2_ACCEPTANCE.md`.
- Must edit: the two listed docs. Do not touch code.
- Anchors: `M6_VAPOR_COVERAGE.md` rows 24, 25, 31.
- Acceptance: `grep -n "still rejects" planning/master/M6_VAPOR_COVERAGE.md` returns nothing; `cargo test -p hydrus-core --test vapor_fortran_parity flux_fixedhead_wet_on_matches_fixture` passes.

### 31.1.2 Draft C5 closed-pair criteria proposal
- Depends on: none. Repo: hyd.rs.
- Spec: write a proposal with the evidence for tolerances on head, `liquid_moisture`, `temperature_c` for the closed pairs, using the on/off differences in GOALS C5 (head 0.897 cm, moisture 9.0e-4, temperature 0.27 C; flux-known head 0.877 cm, moisture 8.0e-4, temperature 0.27 C). Do not edit `criteria.json`.
- Must edit: `planning/master/M6_VAPOR_C5_PROPOSAL.md` (create).
- Anchors: `reference/vapor_fortran/criteria.json` (read only), `tools/compare_vapor_reference.py`.
- Acceptance: file exists; `git diff --quiet -- reference/` (no criteria change).

### 31.1.3 G22.3 and met contract
- Depends on: none. Repo: hyd.rs.
- Spec: write the contract. Inputs: meteorological series (Tmax/Tmin, radiation, wind, humidity), latitude, altitude, atmospheric switching modes (`UpperBoundaryMode`, `atmospheric.rs:10`). Map each to Fortran: `INPUT.FOR:38` (lMeteo), `INPUT.FOR:934` (`MeteoIn`), `TIME.FOR`/`HYDRUS.FOR` lMeteo uses. Acceptance and rejections with triggers.
- Must edit: `planning/master/M6_VAPOR_G22_3_CONTRACT.md` (create).
- Acceptance: file exists, contains an input-to-Fortran mapping table and at least one named rejection with trigger.

### 31.2.1 Met reference harness
- Depends on: 31.1.3. Repo: hyd.rs.
- Case (D-11): `vapor_met_bare_soil_on`, 240 h, 10 daily records. Record the non-convergence count and the upper-boundary switch count. Fortran divergence, or zero switches, stops at a gate.
- Spec: add a met case generator and `lMeteo=true` cases using the pinned Fortran reference, following the vapor harness (case dir layout, `SELECTOR.IN` with `lMeteo=t`, `LEVEL_01.DIR`). Extend the case allowlist in `run_vapor_reference.py` (~line 318) for the new cases only. Run through the existing producer. Do not edit `criteria.json`.
- Must edit: `tools/run_vapor_reference.py`, `reference/vapor_fortran/cases/vapor_met_*` (create). Must not touch `criteria.json` (owned by 31.2.3).
- Anchors: `tools/run_vapor_reference.py:318`, `reference/vapor_fortran/cases/vapor_flux_fixedhead_wet_on/SELECTOR.IN` (template).
- Acceptance: `python3 -m unittest tools.test_run_vapor_reference` passes; a `vapor_met_*` fixture is produced under `reference/vapor_fortran/fixtures/`.

### 31.2.2 Port radiation Rn, Rns, Rnl
- Depends on: 31.1.3. Repo: hyd.rs.
- Spec: port the net radiation terms from `MeteoIn` (`INPUT.FOR:934`) into a new `meteo/` module. Preserve operation order and precision (ADR 0010, 0015). Export through `lib.rs`.
- Must edit: `crates/hydrus-core/src/meteo/mod.rs` (create), `crates/hydrus-core/src/meteo/radiation.rs` (create), `crates/hydrus-core/src/lib.rs`.
- Anchors: `src_reference/INPUT.FOR:934` (`MeteoIn`), `ShortWaveRadA`/`ShortWaveRadB`.
- Acceptance: `cargo test -p hydrus-core meteo::radiation` passes, with unit tests against the source formulas.

### 31.2.3 C5 criteria entry (Option M1)
- Depends on: GATE A (approved 2026-11-15: head Option A, moisture M1). Repo: hyd.rs.
- Spec: add exactly one entry to `reference/vapor_fortran/criteria.json`: `flux_known_on_off_liquid_moisture` (case `vapor_flux_known_on`, control `vapor_flux_known_off`, `factor_of_tolerance` 0.75, field `liquid_moisture`, rule `any_print_time_after_initial`, unit `cm3/cm3`). Copy from `planning/master/M6_VAPOR_C5_PROPOSAL.md` section 5. No head entries (Option A). Bump the criteria version/changelog fields the way v6 did (see commit `231aa46`).
- Must edit: `reference/vapor_fortran/criteria.json`, plus the `CRITERIA_SHA256` constant in the four hash-pinning tests (`vapor_fortran_parity.rs`, `vapor_heat_boundary_parity.rs`, `vapor_nonuniform_parity.rs`, `vapor_nonuniform_timestep_controller.rs`), as in `231aa46`. Re-scoped 2026-11-15: the original spec missed the hash pins. This is the only task allowed to edit `criteria.json`; 31.2.1 must not.
- Anchors: `tools/compare_vapor_reference.py` (`validate_criteria`), commit `231aa46`.
- Acceptance: `python3 -m unittest tools.test_compare_vapor_reference` passes; the comparator over existing fixtures passes the new entry (r_max 0.7619); `cargo test -p hydrus-core --test vapor_fortran_parity` still 19 pass.

### 31.3.1 Port evaporation from met input
- Depends on: 31.2.2. Repo: hyd.rs.
- Spec: port the potential evaporation/transpiration calculation consumed by the atmospheric path, using the radiation terms from 31.2.2. Same precision rules.
- Must edit: `crates/hydrus-core/src/meteo/evaporation.rs` (create), `crates/hydrus-core/src/meteo/mod.rs`.
- Anchors: `src_reference/INPUT.FOR:934` onward (`MeteoIn` ET calls; locate by grep `ET`/`Evap` after 934).
- Acceptance: `cargo test -p hydrus-core meteo::evaporation` passes.

### 31.3.2 Vapor-aware atmospheric switching
- Depends on: 31.1.3, 31.2.2. Repo: hyd.rs.
- Spec (D-10: decompose into existing Flux/Head pairs, preserving Fortran switching order and parity): let `advance_atmospheric_step` (`atmospheric.rs:55`) accept the vapor coupling per the contract. Water-only behavior must not change: the existing atmospheric tests stay green.
- Must edit: `crates/hydrus-core/src/atmospheric.rs`, `crates/hydrus-core/src/forcing.rs`.
- Anchors: `atmospheric.rs:10` (`UpperBoundaryMode`), `:55` (`advance_atmospheric_step`), `forcing.rs:10` (`AtmosphericRates`).
- Acceptance: `cargo test -p hydrus-core atmospheric` passes, including unchanged water-only tests.

### 31.3.3 Meteo.in reader (added by D-11)
- Depends on: 31.1.3; 31.2.1 for the committed deck used as test input. Repo: hyd.rs.
- Spec: parse `Meteo.in` site and configuration data (`MeteoIn`, `INPUT.FOR:934-1066`) into `meteo::config::MeteoConfig`, and the met records (`SetMeteo`, `TIME.FOR:484-497`: `tAtm, Rad, TMax, TMin, RHMean, Wind_kmd, SunHours`) into `meteo::series::MeteoRecord`. Follow the contract's mapping table. Raise each section 7 rejection as a typed `MeteoError` before any state changes. Scope is the Gate A selection (bare soil, daily records). Other modes are rejected with a trigger, not implemented. No new dependencies (ADR 0011).
- Must edit: `crates/hydrus-core/src/meteo/config.rs` (create), `crates/hydrus-core/src/meteo/series.rs` (create), and one-line `mod`/`pub use` additions in `meteo/mod.rs`.
- Anchors: `planning/master/M6_VAPOR_G22_3_CONTRACT.md` sections 3 and 7, `reference/vapor_fortran/cases/vapor_met_bare_soil_on/Meteo.in`.
- Acceptance: `cargo test -p hydrus-core meteo::config` and `meteo::series` pass. They parse the committed 31.2.1 deck and include one test per named rejection.

### 31.4.1 Met parity
- Depends on: 31.2.1, 31.3.1, 31.3.2, 31.3.3. Repo: hyd.rs.
- Spec (D-10: this test is the parity gate for the Flux/Head decomposition): new parity test comparing the Rust met path with the pinned Fortran trajectory from 31.2.1, in the style of `vapor_fortran_parity.rs`. Source-precision rows are listed, not widened.
- Must edit: `crates/hydrus-core/tests/vapor_met_parity.rs` (create), fixtures under `reference/vapor_fortran/fixtures/`.
- Acceptance: `cargo test -p hydrus-core --test vapor_met_parity` passes.

### 31.4.2 Met water balance
- Depends on: 31.3.2. Repo: hyd.rs.
- Spec: water balance over the met-switched run, per the closed-pair balance pattern in `vapor_closed_pair_balance.rs`.
- Must edit: `crates/hydrus-core/tests/vapor_met_balance.rs` (create).
- Acceptance: `cargo test -p hydrus-core --test vapor_met_balance` passes.

### 31.5.1 Record G22.2 close and G22.3 acceptance
- Depends on: 31.4.1, 31.4.2, 31.2.3 (criteria entry landed). Repo: hyd.rs.
- Spec: record closure in the coverage matrix and GOALS. The acceptance doc is the record for G22.3. If C5 is not approved, do not claim G22.2 closed; record the open item.
- Must edit: `planning/master/M6_VAPOR_COVERAGE.md`, `planning/master/M6_VAPOR_G22_3_ACCEPTANCE.md` (create), `planning/GOALS.md`.
- Acceptance: files updated; `grep -n G22.3 planning/GOALS.md` shows the new result.

## Escalation and gates

- GATE A (after wave 1): user approves C5 tolerances (31.1.2) and the G22.3 contract (31.1.3). No waves 2+ before this gate.
- Worker escalation follows wave-orchestration. A task that needs `water_solver.rs` is a re-scope (escalation 4), not an ad hoc widening.
- Full serial verification at each wave end (D-7).

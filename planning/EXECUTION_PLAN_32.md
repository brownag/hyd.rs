# Execution Plan 32: production met run path in `src/` (G26, with G28)

Status: APPROVED 2026-11-24 (user). COMPLETE 2026-11-24 (see [STATE_32.md](STATE_32.md)).
Repo: hyd.rs (branch `feat/met-run-path`, cut from `main` at `3f328fc`). Tracked planning (D-5).
Source goal: [GOALS.md](GOALS.md) Circled G26, with G28 folded in as step 1.

## Goal & Scope

**Goal.** A met-driven vapor water-heat run has a supported path in `src/`. The forcing built from `Meteo.in` (bare-soil ET rates and the per-record top temperature `(TMax+TMin)/2` in f32, `TIME.FOR:519`) and the two-pass coupled step around `advance_atmospheric_step_vapor` move out of `tests/support/vapor_met.rs` into `hydrus-core`. The test driver keeps only case wiring: deck parsing, the fixed-grid loop and print sampling. The D-10 switching deviation (G28) is decided and recorded as ADR 0022.

**In scope.**
- G28: ADR 0022 records the step-level re-solve and its limit to fixed-schedule runs.
- `meteo::forcing`: per-record rates, forcing schedule and top temperatures from `MeteoConfig`, `MeteoRecord`s and `ATMOSPH.IN` record data.
- `VaporWaterHeatDriver::step_atmospheric`: the two-pass coupled step with atmospheric switching. It shares commit and predictor code with `step`. It never cuts dt internally and returns a typed non-convergence error.
- Rewire `tests/support/vapor_met.rs` onto both, so its outputs are bit-identical.
- Record G26 and G28 as done, and add a follow-up goal for in-loop switching with adaptive dt.

**Out of scope (explicit).**
- G25 (solved head-end boundary fluxes). `upper_for` (`atmospheric.rs:484`) and the FixedHead flux 0.0 stay as they are.
- G20 (energy balance), G27 (met discrimination criteria), G30 (input-deck reader, CLI, and the run loop in `src/`).
- In-loop Picard switching and adaptive timestepping on the met path (new follow-up goal, D-13).
- Any edit to `water_solver.rs`, `atmospheric.rs`, `reference/`, `criteria.json` or the two met test files.
- Any new dependency (ADR 0011).

**Verbatim user constraints.**
- D-10/G28: "Keep + ADR, fixed grid".
- API: "Driver method + forcing builder".
- Non-convergence: "Typed error, no retry".
- Operations: "Reuse, new branch".
- Exit (Magic Wand handoff): "`vapor_met_parity` and `vapor_met_balance` pass via src/ with unchanged tolerances and criteria hashes; ADR for D-10; full AGENTS.md suites green."

## Decision log

- **D-5 (2026-11-24, user, carried from plan 31):** Planning artifacts are tracked. `docs(planning):` commits are kept separate from code.
- **D-6 (2026-11-24, user, carried):** Up to 3 workers per wave, worktrees for same-repo tasks, full suites serial at wave end.
- **D-7 (default, AGENTS.md):** Full verification at wave end: `cargo test --workspace --no-fail-fast`, clippy `-D warnings`, fmt check, Python `tools` unittest.
- **D-8 (ADR constraints):** ADR 0010 (source order and precision) and ADR 0015 (precision-faithful ports) apply. Moved code keeps its operation order exactly, including the f32 `(t_max + t_min) / 2.0_f32` and the `rSoil = EvapP*rConv/TTConv` conversion. ADR 0011 applies: no new dependencies. ADR 0021 (Proposed, one-way heat coupling) is not touched, because the vapor driver already couples two-pass. No ADR is superseded.
- **D-13 (2026-11-24, user):** G28 resolution: keep the step-level re-solve from plan 31 D-10. ADR 0022 records it, along with the iteration-history difference (Rust 6-7 iterations per pass at the rain record, Fortran 2 or fewer) and its scope: fixed-schedule runs only. Because Fortran dt control reads iteration counts, adaptive dt is excluded until in-loop switching exists. The trigger to revisit is any request for adaptive dt on the met path, or a met case that fails parity. A new follow-up goal (G31) owns the in-loop port.
- **D-14 (2026-11-24, user):** API shape. Forcing becomes `meteo::forcing` (`MetForcing`). The coupled step becomes `VaporWaterHeatDriver::step_atmospheric`. The fixed grid loop and print sampling stay in test wiring (G30 adds a `src/` loop later).
- **D-15 (2026-11-24, user):** `step_atmospheric` forces no internal dt cut (`minimum_timestep` = the step's dt, as the test driver does today). On non-convergence in either pass it rejects staged state and returns a typed error. It never calls `step()`'s restart path.
- **D-16 (2026-11-24, user):** Branch `feat/met-run-path` from `main` (`3f328fc`). The 2026-11-24 GOALS.md refresh is the first `docs(planning):` commit on it.
- **D-17 (orchestrator, acceptance):** "Unchanged" means bit-identical. 32.2.1 dumps the `MetRun` series (head, moisture, temperature, 9 vapor fields, per-step `StepTrace` scalars and face fluxes) before and after the rewire and requires exact equality. Any difference stops the task. Bounds are never adjusted.

## Recon facts (anchors)

- Test driver: `crates/hydrus-core/tests/support/vapor_met.rs` (785 lines). `run_met_case` at :262. Deck parsing at :264-360. Forcing (rates and schedule) at :361-430. Per-record `VaporConfig` with tTop at :448-475. Water, heat and coupling setup at :476-560. Step loop at :584-783: pass 1 (:614), the heat closure (:625-664), pass 2 (:670-688), commit (:690-708), `StepTrace` (:712-731), `VaporStepReport` (:732-777).
- Helpers to move or share: `next_predictor` (:208), `commit_predictor` (:217). The driver already has its own `commit_predictor` (`heat/vapor/driver.rs`, used near :213) and `fixed_ends` (:593).
- Driver: `crates/hydrus-core/src/heat/vapor/driver.rs` (1030 lines). `VaporWaterHeatDriver` :121, `new` :138, `step` :175-300, `preflight` :310, `run_passes` :373, pass bodies :435-550 (`solve_vapor_water_trial` at :445 and :483), `VaporDriverError` :44, tests module from about :860.
- Atmospheric vapor step: `src/atmospheric.rs:523` `advance_atmospheric_step_vapor(state, workspace, input, policy, schedule, VaporAtmosphericState, VaporAtmosphericCoupling, VaporWaterContext)`. It returns a step with `trial`, `state`, `switches` and `below_critical_head`. Read only in this plan.
- `VaporConfig` is `Copy` (`heat/vapor/mod.rs:24`). `new` :51, `top_temperature` :119. The test builds one config per record because tTop lives in the config.
- Meteo: `src/meteo/mod.rs` (mods `config`, `evaporation`, `radiation`, `series`; re-exports :8-21). `calculate_bare_soil_et`, `MeteoSite`, `EvaporationInput` in `evaporation.rs`. `parse_meteo_deck` and `MeteoRecord` in `series.rs`.
- Exports: `src/lib.rs` (47 lines), with `pub mod meteo` at :13 and the meteo re-exports at :28.
- Tests that use the driver: `tests/vapor_met_parity.rs` (616 lines, 5 tests; it pins deck sha256 at :371) and `tests/vapor_met_balance.rs` (211 lines, 3 tests). Both call `vapor_met::run_met_case(RunOptions::default())`.
- Baseline (2026-11-24, `main` `3f328fc`, release): `cargo test -p hydrus-core --release --test vapor_met_parity --test vapor_met_balance` gives 5 + 3 pass in about 5 s. The full suites are checked before wave 1.
- Next ADR number: 0022 (`docs/decisions/README.md` table).

## Hazards

- `driver.rs` is 1030 lines and holds hot code. Enter it by the anchors only. `step()` behaviour and its tests must not change: the commit and predictor refactor is a pure extraction.
- Bit-identity: the test driver interleaves heat and water in a specific order (heat closure reads `snap` and the old temperatures). Reordering a float operation breaks D-17. Port it line for line.
- `next_predictor` uses `previous_timestep.unwrap_or(1.0)` and the pass-2 `initial_material_heads` is pass 1's `pre_solve_heads`. Both are easy to "simplify" wrongly.
- The pass-2 `VaporAtmosphericState` keeps pass 1's boundary memory but the *step-start* `event_cursor` (vapor_met.rs:670-673). This is the subtle part of the D-10 decomposition.
- `water_solver.rs` and `atmospheric.rs` must not be edited (G25 owns them next).
- The test driver bypasses `preflight` today. Turning it on for the met path may reject inputs, such as `Atmospheric` in `validate_water_input`. Any rejection is a question for the 32.1.3 plan, not something to silence.
- `step()` must stay bit-identical after the `attempt`/`heat_pass` generalisation. The existing driver tests and every vapor parity suite guard it, so run `cargo test -p hydrus-core --release --test vapor_fortran_parity` in 32.1.3 too.

## Waves

| Wave | Task | Repo | Depends on | Files (must edit) | Goal |
| --- | --- | --- | --- | --- | --- |
| 1 | 32.1.1 | hyd.rs | none | `docs/decisions/0022-*.md` (create), `docs/decisions/README.md` | ADR 0022: step-level switching re-solve, fixed-schedule limit (G28) |
| 1 | 32.1.2 | hyd.rs | none | `src/meteo/forcing.rs` (create), `src/meteo/mod.rs`, `src/lib.rs` | `MetForcing` builder: rates, schedule, per-record tTop |
| 1 | 32.1.3 | hyd.rs | none | `src/heat/vapor/driver.rs`, `src/heat/vapor/mod.rs` | `VaporWaterHeatDriver::step_atmospheric` with typed non-convergence |
| 2 | 32.2.1 | hyd.rs | 32.1.2, 32.1.3 | `tests/support/vapor_met.rs` | Rewire the test driver onto `src/`, with bit-identical outputs |
| 3 | 32.3.1 | hyd.rs | 32.1.1, 32.2.1 | `planning/GOALS.md`, `planning/master/M6_VAPOR_G22_3_ACCEPTANCE.md`, `planning/master/M6_VAPOR_COVERAGE.md` | Record G26 and G28 as done; add G31 |

Wave 1 file sets are pairwise disjoint. `lib.rs` belongs only to 32.1.2. 32.1.3 does not re-export through `lib.rs`, because `heat::vapor::driver` is already a public path.

## Task specs

### 32.1.1 ADR 0022: step-level switching re-solve (G28)
- Depends on: none. Repo: hyd.rs. Docs only.
- Spec: write ADR 0022 in the format of `docs/decisions/0021-*.md`, with status Accepted, date 2026-11-24, and source Plan 32 D-13 (plan 31 D-10).
  - Context: Fortran switches KodTop inside the Picard loop (`WATFLOW.FOR:392-426`). Rust re-solves the step once under the new Flux/Head pair (`atmospheric.rs:523` doc comment). Iterations: Rust needs 6-7 per pass at the rain record, Fortran 2 or fewer. Parity evidence is from `M6_VAPOR_G22_3_ACCEPTANCE.md` rows 41-42 and exception 5.
  - Decision: keep the re-solve. The met path is fixed-schedule only. `step_atmospheric` forces no internal dt cut (D-15).
  - Consequences: adaptive dt on the met path is excluded, because Fortran dt control reads iteration counts. Revisit when adaptive dt is requested on the met path, or when a met case fails parity. Follow-up goal G31.
  - Add the row to the README table.
- Must edit: `docs/decisions/0022-re-solve-the-step-on-atmospheric-switches-with-vapor.md` (create), `docs/decisions/README.md`. Do not touch code.
- Anchors: `docs/decisions/README.md` table (row 0021), `planning/master/M6_VAPOR_G22_3_ACCEPTANCE.md:41-53`, `crates/hydrus-core/src/atmospheric.rs:511-522`.
- Acceptance: `test -f docs/decisions/0022-*.md && grep -q "0022" docs/decisions/README.md`. `git diff --quiet -- crates/ reference/`.

### 32.1.2 `MetForcing` builder
- Depends on: none. Repo: hyd.rs.
- Spec: create `meteo::forcing` with:
  - `AtmosphericRecord { t_atm: f64, precipitation: f64 }`: the `ATMOSPH.IN` columns the met path reads.
  - `MetForcing::new(config: &MeteoConfig, meteo: &[MeteoRecord], atmosphere: &[AtmosphericRecord], x_conv, t_conv) -> Result<MetForcing, MetForcingError>`. It returns per-record `(t_atm, AtmosphericRates)` (cm/h, as `vapor_met.rs:361-415`) and per-record `top_temperatures: Vec<f64>`, set to `f64::from((t_max + t_min) / 2.0_f32)` as in `vapor_met.rs:448-452`.
  - `MetForcing::schedule(&self, units) -> Result<ForcingSchedule, _>`. Each record applies to the interval ending at its tAtm (`vapor_met.rs:417-430`).
  - Typed errors for record-count mismatch, tAtm mismatch between decks, and evaporation failure. These replace the test driver's `assert!`s.
  - Port the arithmetic in source order (D-8). Keep the fixed modes the test uses (`SolarRadiationInput`, `SunshineHours`, `RelativeHumidityPercent`) and add a doc comment saying they are the only modes evidenced.
  - Add a `pub mod forcing;` and re-export in `meteo/mod.rs`, plus `MetForcing` in `lib.rs`.
- Must edit: `crates/hydrus-core/src/meteo/forcing.rs` (create), `crates/hydrus-core/src/meteo/mod.rs`, `crates/hydrus-core/src/lib.rs`. Scratch: `.development/32.1.2/`.
- Anchors: `tests/support/vapor_met.rs:361-430`, `:448-452`. `meteo/evaporation.rs` (`calculate_bare_soil_et`, `MeteoSite`). `forcing.rs` (`ForcingSchedule::new`, `ForcingEvent`). Reference implementation: the test driver code itself.
- Acceptance:
  - Unit tests in `forcing.rs`: on the committed `vapor_met_bare_soil_on` `Meteo.in` and `ATMOSPH.IN` (read via `CARGO_MANIFEST_DIR`), the rates and top temperatures equal the values the test driver computes (copy that arithmetic as an in-test oracle, or assert against values captured from `run_met_case`). Mismatch errors are typed.
  - `cargo test -p hydrus-core --lib meteo::forcing` passes.
  - `cargo clippy -p hydrus-core --all-targets -- -D warnings` is clean.

### 32.1.3 `VaporWaterHeatDriver::step_atmospheric`
- Depends on: none. Repo: hyd.rs.
- Spec: add `step_atmospheric(&mut self, water_state, water_workspace, water_input /* upper = Atmospheric */, policy, schedule: &ForcingSchedule, atmo: VaporAtmosphericState, coupling: VaporAtmosphericCoupling, top_temperature: f64, source_materials, heat_input) -> Result<VaporAtmosphericStepReport, VaporDriverError>`.
  - **Reuse, do not duplicate.** `attempt` (:427-528) and `heat_pass` (:531-590) already have the test driver's two-pass structure:
    - the same predictor;
    - pass 2 from pass 1's heads, with `initial_material_heads` = pass 1's `pre_solve_heads`;
    - the same heat adjustment inputs.

    Generalise `attempt` so the water-pass call is a parameter (a closure or a small enum). `step()` passes `solve_vapor_water_trial` and gets identical behaviour. `step_atmospheric` passes `advance_atmospheric_step_vapor`, threading `VaporAtmosphericState`: pass 2 gets pass 1's boundary memory with the *step-start* `event_cursor` (`vapor_met.rs:670-673`). It also collects switches and `below_critical_head`.
  - **Per-step config.** Add `VaporConfig::with_top_temperature(self, f64) -> Result<Self, VaporError>` in `mod.rs`. `attempt`, `heat_pass` and `preflight` take `config: &VaporConfig` instead of reading `self.config`. `step()` passes `&self.config`; `step_atmospheric` passes the step config, and the caller's `heat_input.top_temperature` must equal it (existing preflight check).
  - **Preflight.** `step_atmospheric` runs `preflight` against the step config. If `validate_water_input` rejects `UpperBoundary::Atmospheric`, put it as a plan question. The fix may only go in `mod.rs`, never in `water_solver.rs`.
  - **No retry (D-15).** Call `attempt` once, with `policy.minimum_timestep = water_input.timestep`. Never use `run_passes`' restart loop. On any error, call `self.state.reject()` and return. Add `VaporDriverError::Atmospheric(VaporAtmosphericError)`. A non-converged pass surfaces as the existing `Water(NonConverged)` or `Atmospheric(..)` variant, with the pass number in the report or error.
  - **Commit.** Reuse `step()`'s commit block (:205-300, extracted into a private helper with no behaviour change). `top_fixed` comes from the final pass-2 `kod_top > 0`, not from `fixed_ends(upper)` (`vapor_met.rs:693-695`).
  - **Return.** `VaporAtmosphericStepReport { step: VaporStepReport, state: VaporAtmosphericState, switches: Vec<(u8, VaporSwitchRecord)>, below_critical_head: [bool; 2] }`. The doc comment cites ADR 0022.
- Must edit: `crates/hydrus-core/src/heat/vapor/driver.rs`, `crates/hydrus-core/src/heat/vapor/mod.rs`. Must not touch `atmospheric.rs`, `water_solver.rs`, `lib.rs`, `tests/`. Scratch: `.development/32.1.3/`.
- Anchors:
  - `driver.rs`: `step` :175, `preflight` :310, `run_passes` :373, `attempt` :427, `heat_pass` :531, `fixed_ends` :593, `commit_predictor` :600, tests module about :860 (with `Inject` hooks).
  - `mod.rs`: `VaporConfig::new` :51, `top_temperature` :119, `validate_water_input`.
  - `atmospheric.rs:523`, read only.
  - Reference implementation: `tests/support/vapor_met.rs:584-708`.
- Acceptance: new unit tests in `driver.rs`.
  - (a) A wet profile with a schedule whose evaporation keeps the top in flux mode (no switch recorded). One `step_atmospheric` gives the same heads, temperatures and vapor history as `step()` with `UpperBoundary::Flux { flux: net rate, row: zero row }` (exact).
  - (b) A forced non-convergence (`max_iterations = 1` with a tight tolerance) returns an error and leaves `state()` unchanged.
  - (c) The existing driver tests are unchanged and pass.
  - Commands: `cargo test -p hydrus-core --lib heat::vapor::driver` passes; `cargo test -p hydrus-core --release --test vapor_met_parity --test vapor_met_balance --test vapor_fortran_parity` passes; clippy clean.

### 32.2.1 Rewire the met test driver onto `src/`
- Depends on: 32.1.2, 32.1.3. Repo: hyd.rs.
- Spec:
  - Before any edit, add a temporary dump (env-gated, scratch only, not committed) or write a scratch test that serialises the full `MetRun` from `run_met_case(Default)` to `.development/32.2.1/before.json`. Use exact `{:?}`/bits for floats.
  - Then replace the forcing block with `MetForcing`, and the per-step pass, heat and commit block with `VaporWaterHeatDriver::step_atmospheric`, built once from the deck setup.
  - Keep deck parsing, `RunOptions` (including `top_head_override`), the k/10 grid loop, print sampling and the `StepTrace` construction, now filled from `VaporAtmosphericStepReport`. Remove `next_predictor`, `commit_predictor` and the heat closure.
  - Dump `after.json` and require byte equality (D-17).
  - Update the module doc comment: the run path is `src/`, and this file is wiring.
- Must edit: `crates/hydrus-core/tests/support/vapor_met.rs`. Must not touch `tests/vapor_met_parity.rs`, `tests/vapor_met_balance.rs`, `reference/` or `src/`. A needed `src/` change means STOP and report: that is a 32.1.x defect. Scratch: `.development/32.2.1/`.
- Anchors: `vapor_met.rs:262`, `:361-475`, `:584-783`.
- Acceptance:
  - `cmp .development/32.2.1/before.json .development/32.2.1/after.json` succeeds.
  - `cargo test -p hydrus-core --release --test vapor_met_parity --test vapor_met_balance` gives 5 + 3 pass.
  - `grep -nE "advance_atmospheric_step_vapor|solve_vapor_heat_transport|calculate_bare_soil_et|fn commit_predictor|fn next_predictor" crates/hydrus-core/tests/support/vapor_met.rs` returns nothing.
  - `git diff --quiet -- reference/ crates/hydrus-core/tests/vapor_met_parity.rs crates/hydrus-core/tests/vapor_met_balance.rs`.
  - Clippy clean.

### 32.3.1 Record G26 and G28 as done
- Depends on: 32.1.1, 32.2.1. Repo: hyd.rs. Docs only.
- Spec:
  - GOALS.md: move G26 and G28 to Completed with commit IDs. Add G31 (in-loop Picard switching on the met path, enabling adaptive dt; trigger per ADR 0022) to Deferred. Clear the Circled Goal, listing G25 and G20 as the next candidates.
  - `M6_VAPOR_G22_3_ACCEPTANCE.md`: add a dated note that exception "met run path only in the test driver" is resolved by plan 32.
  - `M6_VAPOR_COVERAGE.md`: update the met row to name the `src/` entry points.
- Must edit: the three listed files. No code.
- Acceptance: `grep -n "G31" planning/GOALS.md` matches. `git diff --quiet -- crates/ reference/`.

## Escalation and gates

- wave-orchestration escalation ladder, unchanged.
- **Stop conditions (report to user, never work around):**
  - 32.2.1 bit-identity fails;
  - any task needs `water_solver.rs`, `atmospheric.rs`, `reference/` or a met test file;
  - parity or balance fails at any wave end.
- Wave-end verification is serial (D-7). Mark a wave done only after the full suites are green.

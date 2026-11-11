# Orchestration 24 — G12: M6 heat/temperature capability (through G18)

Status: **DRAFT — awaiting user approval.** Not executed. Written by plan-wave from the codebase recon of 2026-11-11.

## Goal and scope

**Goal.** Deliver a bounded, coupled heat/temperature capability on top of the accepted M4 water product, with explicit coupling contracts, rejected unsupported combinations, provenance-backed fixtures, analytical and source-matched comparisons, and documented acceptance (GOALS.md G12 → G18, scoped to heat/temperature). Vapor flow and energy balance are **not** in this orchestration; they are logged as new sequenced goals with triggers (see decision D1, D4).

**In scope (heat/temperature only):**
1. Source contract for `Temper`, `SetUpHeat`, `TempCap` and the water→heat coupling order (G13).
2. Heat configuration that accepts the heat-only selection and rejects unsupported combinations (G14).
3. A bounded vertical slice: capacity/conductivity kernel, boundary setup, transport solve, heat state with accepted-step snapshot, coupled driver (G15).
4. Provenance-backed fixtures: analytical references, plus pinned-Fortran decks for source-matched cases (G16).
5. Comparisons with analytical references and source-matched Fortran; balance properties (G17).
6. Acceptance documentation, capability-scoped, plus a draft ADR (G18).

**Out of scope:**
- Vapor flow: `lVapor`, `ConVapor`, `VaporContent`, `TempAdj` vapor terms, `ConVh`, `iEnhanc`, vapor output (`NOD_INF_V.OUT`). Logged as a new goal, G19 (see Sequenced Goals).
- Energy balance: `lEnBal`, `UpdateEnergy`, `Evap`/`Rn`/`Rns`/`Rnl` surface energy terms. Logged as a new goal, G20.
- Hysteresis (M7), dual porosity (M8), root uptake with heat coupling, chemistry, `iModel` other than the already-accepted `iModel=0` path.
- Any change to the water solver's equations, to the solute path, or to `solute_fortran_compat/`.
- Changing `src_reference/`, `reference/solute_verification/`, parity tolerances or comparator logic of the solute tools (ADR 0001, 0003, 0006).
- Adding a heat deck reader to the Fortran-compatible deck runner. Parity uses a fixture JSON (see D6).
- `TODO.md`, `Cargo.toml`, `Cargo.lock` (ADR 0011).

**Verbatim user constraints:**
- GOALS.md G12 (2026-11-11): "Boundary: preserve M5 campaign findings and do not expand into unrelated optional physics. Vapor flow and energy balance are desired scope; determine the correct milestone placement during planning rather than assuming they belong to M6."
- G12 exit criteria: "selected M6 behavior is implemented with explicit coupling contracts, relevant fixtures, and capability-specific validation evidence; unsupported combinations are rejected rather than silently approximated."
- M5-V handoff (2026-11-11): "M6 acceptance must still be based on its selected capability and intended use; no universal tolerance or M5-derived M6 threshold is introduced here."
- M5-V handoff: "Do not expand or repeat M5-V work for campaign completeness."

## Decision log

| Date | Decision |
|---|---|
| 2026-11-11 | D1. (User.) Milestone placement: **M6 = heat/temperature now.** Vapor and energy balance become separate follow-on goals (G19, G20), each with its own acceptance gate. |
| 2026-11-11 | D2. (User.) Fixtures: **analytical references plus pinned-Fortran decks** for source-matched cases. Fortran cases are labeled source-matched; analytical-only cases are labeled analytical-only. |
| 2026-11-11 | D3. (User.) Parallelism: **≤3 concurrent workers per wave**, each in its own worktree. |
| 2026-11-11 | D4. (User.) Chain depth: this orchestration runs **G13 through G18 scoped to heat**. G12 exit criteria are met for M6 heat when Wave 6 passes. |
| 2026-11-11 | D5. (User.) Acceptance: each task runs its targeted tests; **each wave ends with the full AGENTS.md Quick Start suite run serially**. Fortran rebuilds only when a task needs new source-matched output. |
| 2026-11-11 | D6. (Default, not asked.) Heat parity uses a committed fixture JSON produced by `tools/run_heat_reference.py` from the pinned Fortran build, not a new deck reader. Adding a heat deck reader would widen `solute_fortran_compat/` and is out of scope. |
| 2026-11-11 | D7. (Default, not asked.) Heat-only configuration **rejects** `lVapor=true`, `lEnBal=true`, `iDualPor≠0`, and any top/bottom heat boundary kind not in the contract's Supported list. Rejection is a typed error, never a silent approximation (G14). |
| 2026-11-11 | D8. (Default, not asked.) Precision: production heat code uses `f64`. Any Fortran `real` (single) conversion point is recorded in the contract (`planning/master/M6_HEAT_CONTRACT.md`, task 24.1.1) and tested, not silently changed. ADR 0010 governs `solute_fortran_compat/` only; heat code outside it is not bound to f32 operation order unless the contract says so. |
| 2026-11-11 | D9. (Default, not asked.) Water coupling is **one-way per step**: heat reads the accepted water snapshot (face fluxes and contents) after `advance_water_step`; heat never feeds back into water in M6. This follows `WatFlow` before `Temper` at `HYDRUS.FOR:419` and `:491`. |
| 2026-11-11 | D10. (Default, not asked.) An ADR is drafted at Wave 6 (task 24.6.2) for the heat coupling order and the heat-only rejection rule, status **Proposed** until the user accepts it. No existing ADR is superseded (ADR 0013, 0015, 0019 checked: none governs heat). |
| 2026-11-11 | D11. (Default, not asked.) Execution setup as Orchestration 23 D10: worktrees `.development/24-worktrees/<id>` on branch `orchestration/<id>`; `CARGO_TARGET_DIR=.development/24-target/<id>`; one escalation retry; serial wave-end gates; orchestrator owns `planning/STATE_24.md`. Worker model tier: `inherit` (judgment call, flagged for approval). |
| 2026-11-11 | D12. (Gate, not a design choice.) **Pre-wave-1 gate:** at recon time 16 planning and doc files were uncommitted (`git status`). Worktrees branch from `HEAD` and would not see them. The user must commit or stash them before Wave 1 starts. See STATE_24 baseline. |

## Reconnaissance log (2026-11-11, branch `fix/robin-upstream-mapping1`)

- **Fortran thermal path.** `src_reference/TEMPER.FOR` (323 lines): `Temper` at `:5`, `SetUpHeat` at `:77`, `TempAdj` at `:180` (vapor term, out of scope), `TempCap` at `:269`. Call sites in `src_reference/HYDRUS.FOR`: `WatFlow` at `:419`, `UpdateEnergy` at `:462` (energy, out of scope), `Temper` at `:491`, `Update` at `:695`. Inside `Temper`, `TempCap` is called at `TEMPER.FOR:37` and `:40`, and `TempAdj` at `:48` under `lVapor`.
- **Heat variables.** `kTopT`, `tTop`, `kBotT`, `tBot`, `lEnBal`, `HeatFl`, `xConv`, `tConv`, `dtMaxT`, `iCampbell`, `iTemp` are arguments of `Temper` (`TEMPER.FOR:5-10`).
- **Input flags.** `INPUT.FOR:38` reads `lSnow,lDummy,lMeteo,lVapor,lActRSU,lFlux`; `INPUT.FOR:1586` sets `lVaporOut`; `INPUT.FOR:1643` documents `lEnBal`. Output temperature: `output_compat.rs:162` (`Temperature Temp(i)`).
- **Rust has no temperature state.** `grep -i 'temperature|heat|thermal'` over `crates/hydrus-core/src` hits only solute-compat deposition inputs, output-compat formatting and the `HeatVol` line in `output_compat.rs:590-697`. No heat solver, no thermal state.
- **Water side to couple to.** `crates/hydrus-core/src/water_solver.rs:183` `advance_water_step(state, workspace, input, policy)`; `WaterStepInput` at `:57`; `WaterProductConfig` at `:18` (accepts only `iModel=0` and one domain). `water_state.rs:68` `AcceptedWaterSnapshot` exposes `water_contents` and `face_fluxes` (`:75`). `water_state.rs:8` `WaterState`.
- **Patterns to match.** `crates/hydrus-core/src/lower_boundary.rs` (`UnsupportedLowerBoundaryMode`, typed rejection); `water_solver.rs:18-40` (`WaterProductConfig::new` validation); `hydraulics/types.rs` (parameter validation style); `tests/water_transaction.rs` (accept/reject transaction shape, for `heat/state.rs`).
- **Linear solve.** `crates/hydrus-core/src/hydraulics/linear.rs` and `tests/tridiagonal.rs` exist; the transport solve reuses the existing tridiagonal path, no new dependency (ADR 0011).
- **Fixtures.** `reference/` has no heat deck. `planning/master/VALIDATION_STRATEGY.md:30` records "Coupled heat/vapor | manual Chapter 11.11; deck absent". Fortran reference harness: `reference/fortran_reference/README.md` (stage with `tools/stage_fortran_reference.py`, build with `tools/build_fortran_reference.sh` under `.development/reference-build/`).
- **Existing crate layout.** `crates/hydrus-core/src/lib.rs` declares modules and re-exports; no `heat` module exists. Integration tests live in `crates/hydrus-core/tests/`.
- **Validation roadmap.** `planning/master/ROADMAP_AND_RISKS.md:36`: "M6 is unblocked for planning and implementation. Define heat/temperature scope; vapor/energy only if separately selected."
- **Uncommitted work.** `git status --short`: 16 entries, mostly `planning/master/*` and `planning/GOALS.md`, `planning/STATE_23.md`, `README.md`, `TODO.md`, `docs/decisions/*`. See D12.

## Module layout (decided here so tasks stay disjoint)

Wave 1 task 24.1.2 creates `crates/hydrus-core/src/heat/mod.rs` declaring all planned submodules, and one-line placeholder files for the ones not yet implemented. Each later task owns exactly one of these files and replaces its placeholder. No task edits another task's file.

```
crates/hydrus-core/src/heat/
  mod.rs        (24.1.2: declares modules; re-exports nothing into lib.rs in M6)
  capacity.rs   (24.1.2: TempCap)
  config.rs     (placeholder until 24.2.1)
  boundary.rs   (placeholder until 24.2.2: SetUpHeat)
  transport.rs  (placeholder until 24.3.1: Temper core)
  state.rs      (placeholder until 24.3.2)
  driver.rs     (placeholder until 24.4.1: coupled step)
  balance.rs    (placeholder until 24.4.2)
```

`crates/hydrus-core/src/lib.rs` is edited only by 24.1.2 (adds `pub mod heat;`). Downstream tests import `hydrus_core::heat::...`. No `pub use` into the crate root in M6.

## Wave table

| Wave | Task | Title | Depends on | Repo |
|---|---|---|---|---|
| 1 | 24.1.1 | Heat source contract | — | hyd.rs |
| 1 | 24.1.2 | Heat module skeleton and TempCap kernel | — | hyd.rs |
| 2 | 24.2.1 | Heat-only configuration and rejection | 24.1.1 | hyd.rs |
| 2 | 24.2.2 | SetUpHeat boundary kernel | 24.1.1, 24.1.2 | hyd.rs |
| 3 | 24.3.1 | Temper transport solve (heat-only) | 24.1.2, 24.2.2 | hyd.rs |
| 3 | 24.3.2 | Heat state and accepted-step snapshot | 24.2.1 | hyd.rs |
| 4 | 24.4.1 | Coupled water→heat step driver | 24.2.1, 24.3.1, 24.3.2 | hyd.rs |
| 4 | 24.4.2 | Heat-energy balance accounting | 24.3.1 | hyd.rs |
| 4 | 24.4.3 | Pinned-Fortran heat decks and fixture producer | 24.1.1 | hyd.rs |
| 5 | 24.5.1 | Analytical comparisons | 24.4.1 | hyd.rs |
| 5 | 24.5.2 | Source-matched Fortran parity | 24.4.1, 24.4.3 | hyd.rs |
| 5 | 24.5.3 | Balance and invariant properties | 24.4.1, 24.4.2 | hyd.rs |
| 6 | 24.6.1 | Capability acceptance document | 24.5.1, 24.5.2, 24.5.3 | hyd.rs |
| 6 | 24.6.2 | Draft ADR for heat coupling and rejection | 24.6.1 | hyd.rs |

**Disjointness check (by wave).**
- Wave 1: `planning/master/M6_HEAT_CONTRACT.md` (new) vs `heat/mod.rs`, `heat/capacity.rs`, placeholders, `lib.rs`, `tests/heat_capacity.rs`. Disjoint.
- Wave 2: `heat/config.rs` + `tests/heat_config_contract.rs` vs `heat/boundary.rs` + `tests/heat_boundary.rs`. Disjoint. Neither edits `mod.rs`.
- Wave 3: `heat/transport.rs` vs `heat/state.rs` + `tests/heat_state.rs`. Disjoint.
- Wave 4: `heat/driver.rs` + `tests/heat_driver_order.rs` vs `heat/balance.rs` vs `reference/heat_fortran/**`, `tools/run_heat_reference.py`, `tools/test_run_heat_reference.py`. Disjoint.
- Wave 5: `tests/heat_analytical.rs` vs `tests/heat_fortran_parity.rs` + `tools/compare_heat_reference.py` + `tools/test_compare_heat_reference.py` vs `tests/heat_balance.rs`. Disjoint.
- Wave 6: `planning/master/M6_HEAT_ACCEPTANCE.md` vs `docs/decisions/0021-*.md` + `docs/decisions/README.md`. Disjoint.

**Hidden-coupling check.** `lib.rs` is single-owner (24.1.2). `Cargo.toml` is untouched. Tests use only the public `heat` module. Tools use only their own new files and read `tools/run_fortran_reference.py` / `tools/stage_fortran_reference.py` by import only (no edits).

## Wave-end gate (every wave, serial, orchestrator-run)

Per AGENTS.md Quick Start, in the main checkout after merging the wave:
```
cargo build --workspace --locked
cargo test --workspace --no-fail-fast
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
python3 -m unittest discover -s tools -p 'test_*.py'
```
Record counts in `planning/STATE_24.md`. Any failure stops the wave (escalation ladder).

---

## Task specs

### 24.1.1 — Heat source contract

- **Repo:** hyd.rs
- **Depends on:** —
- **Must edit:** none.
- **Can create:** `planning/master/M6_HEAT_CONTRACT.md` only.
- **Temp / scratch:** `.development/24.1.1/`.
- **Spec.** Write the source contract for the heat-only capability. Sections, in this order:
  1. `## Source map`: every subroutine and call site in the Anchors list, with the arguments that carry heat state (`TEMPER.FOR:5-10`, `:77`, `:269`; `HYDRUS.FOR:419`, `:491`, `:695`). Mark `TempAdj` and `UpdateEnergy` as out of scope.
  2. `## Coupling order`: the per-step sequence (water step, then heat step on the accepted water result), with the Fortran lines that establish it. Note `Update` at `HYDRUS.FOR:695`.
  3. `## Variables and precision`: each heat variable with its Fortran type (`double precision`, `real`, or unset), its unit as the source uses it, and the Rust type chosen (D8). Flag every single-precision conversion point.
  4. `## Supported`: the heat-only selection (`lVapor=false`, `lEnBal=false`, `iDualPor=0`, `iModel=0`, one domain) and the top/bottom heat boundary kinds from `SetUpHeat` that M6 supports. Each kind names its Fortran code value.
  5. `## Rejected`: every other combination, with the reason and the typed rejection that 24.2.1 must return.
  6. `## Tolerances`: per-field parity tolerances derived from source precision (ADR 0006 method). Declare them here before any comparison runs.
  7. `## Not claimed`: vapor, energy balance, universal accuracy, hysteresis.
  Read the Fortran directly. Do not trust the summary in this plan; the recon lines above were located by grep, not reviewed in full.
- **Anchors.** `src_reference/TEMPER.FOR` (`Temper` `:5`, `SetUpHeat` `:77`, `TempCap` `:269`); `src_reference/HYDRUS.FOR` (`:419`, `:462`, `:491`, `:695`); `src_reference/INPUT.FOR` (`:38`, `:1586`, `:1643`); `planning/master/VALIDATION_STRATEGY.md:30`.
- **Reference implementation.** ADR 0019 and `planning/master/M5V_M6_HANDOFF_DECISION.md` for scope wording. `docs/decisions/0006-*.md` for tolerance method.
- **Constraints.** Documentation only. Do not edit `src_reference/`, `reference/`, `docs/decisions/`, or `TODO.md`.
- **Acceptance.** `test -f planning/master/M6_HEAT_CONTRACT.md`; `grep -c -E 'Temper|SetUpHeat|TempCap|TempAdj' planning/master/M6_HEAT_CONTRACT.md` ≥ 4; `grep -n -E '^## (Source map|Coupling order|Variables and precision|Supported|Rejected|Tolerances|Not claimed)' planning/master/M6_HEAT_CONTRACT.md` matches all seven headings; `git status --short` shows only that file as new.
- **Targeted test:** none (doc task). Verification by the orchestrator reading the file.

### 24.1.2 — Heat module skeleton and TempCap kernel

- **Repo:** hyd.rs
- **Depends on:** — (may run in Wave 1 alongside 24.1.1; it does not read the contract).
- **Must edit:** `crates/hydrus-core/src/lib.rs` (add `pub mod heat;` next to `pub mod forcing;`, keep the module list alphabetical).
- **Can create:** `crates/hydrus-core/src/heat/mod.rs`, `heat/capacity.rs`, placeholders `heat/config.rs`, `heat/boundary.rs`, `heat/transport.rs`, `heat/state.rs`, `heat/driver.rs`, `heat/balance.rs`, and `crates/hydrus-core/tests/heat_capacity.rs`.
- **Temp / scratch:** `.development/24.1.2/`.
- **Spec.**
  1. Create `heat/mod.rs` with `pub mod capacity; pub mod config; pub mod boundary; pub mod transport; pub mod state; pub mod driver; pub mod balance;` and doc comment `//! Heat-only M6 capability. See planning/master/M6_HEAT_CONTRACT.md.` (The contract file may not exist yet when this task starts; reference it by path only.)
  2. Each placeholder file contains exactly one line: `//! Placeholder; implemented by a later task in orchestration 24.`
  3. Port `TempCap` (`TEMPER.FOR:269-323`) to `heat/capacity.rs`: volumetric heat capacity and thermal conductivity as functions of water content, water flux and the `iCampbell` option, using the `TPar` parameter layout the source uses. Expose a function over slices of the same length, returning a typed error for mismatched lengths or non-finite inputs. Keep the source's order of operations.
  4. Tests in `tests/heat_capacity.rs`: at least three hand-computed points for each branch (with and without Campbell), plus length-mismatch and non-finite rejection. Expected values come from the Fortran formula worked by hand in the test comments, not from running the code (ADR 0009).
- **Anchors.** `src_reference/TEMPER.FOR:269-323` (`TempCap`); `TEMPER.FOR:37` and `:40` (call sites and argument order); `crates/hydrus-core/src/lib.rs` (module list).
- **Reference implementation.** `crates/hydrus-core/src/hydraulics/types.rs` for parameter-validation style; `crates/hydrus-core/src/lower_boundary.rs` for typed errors.
- **Constraints.** Filesystem-free production code (no `std::fs`). No new dependency. Do not edit `error.rs` (the heat module owns its own errors per 24.2.1). Do not implement any other heat subroutine.
- **Acceptance.** `cargo test -p hydrus-core --test heat_capacity` passes; `cargo build -p hydrus-core --locked` passes; `cargo clippy -p hydrus-core --all-targets -- -D warnings` passes for the new files.
- **Targeted test:** `cargo test -p hydrus-core --test heat_capacity`.

### 24.2.1 — Heat-only configuration and rejection

- **Repo:** hyd.rs
- **Depends on:** 24.1.1 (uses the Supported/Rejected lists).
- **Must edit:** none.
- **Can create / replace:** `crates/hydrus-core/src/heat/config.rs` (replace placeholder), `crates/hydrus-core/tests/heat_config_contract.rs`.
- **Temp / scratch:** `.development/24.2.1/`.
- **Spec.**
  1. Define `HeatConfig` in `heat/config.rs`: holds the heat-only selection and validated boundary kind. Constructor `HeatConfig::new(...)` returns `Result<HeatConfig, HeatConfigError>`.
  2. Define `HeatConfigError` in `heat/config.rs` (not in `error.rs`). Variants: one per rejected category in the contract's `## Rejected` list, each carrying the offending value.
  3. Reject, with a typed error and never a silent change: `lVapor=true`, `lEnBal=true`, `iDualPor≠0`, `iModel≠0` (the `WaterProductConfig` already enforces this on the water side; heat must also reject), a boundary kind not in `## Supported`, and multi-domain configurations.
  4. Tests in `tests/heat_config_contract.rs`: one accepted case (heat-only, supported kind) and one rejected case per rejection rule. Each rejected case asserts the variant and the carried value.
- **Anchors.** `planning/master/M6_HEAT_CONTRACT.md` `## Supported` and `## Rejected` (written by 24.1.1); `crates/hydrus-core/src/water_solver.rs:18-40` (`WaterProductConfig::new`); `crates/hydrus-core/src/lower_boundary.rs` (`UnsupportedLowerBoundaryMode`).
- **Reference implementation.** `water_solver.rs:18-40` for the constructor pattern.
- **Constraints.** Do not edit `error.rs`. Do not edit `water_solver.rs` or `solute*`. Do not accept any combination the contract marks Rejected.
- **Acceptance.** `cargo test -p hydrus-core --test heat_config_contract` passes with one accept case and one reject case per Rejected rule.
- **Targeted test:** `cargo test -p hydrus-core --test heat_config_contract`.

### 24.2.2 — SetUpHeat boundary kernel

- **Repo:** hyd.rs
- **Depends on:** 24.1.1, 24.1.2.
- **Must edit:** none.
- **Can create / replace:** `crates/hydrus-core/src/heat/boundary.rs` (replace placeholder), `crates/hydrus-core/tests/heat_boundary.rs`.
- **Temp / scratch:** `.development/24.2.2/`.
- **Spec.** Port `SetUpHeat` (`TEMPER.FOR:77-179`) for the boundary kinds listed in the contract's `## Supported` list only. Produce the top and bottom heat boundary terms the transport solve consumes (`kTopT`, `tTop`, `kBotT`, `tBot` → the boundary rows of the tridiagonal system). Unsupported kinds return a typed error (the rejection itself is in 24.2.1; this task only refuses kinds it does not implement). Keep the source's branch order.
- **Anchors.** `src_reference/TEMPER.FOR:77-179`; `TEMPER.FOR:5-10` (argument list, for `kTopT`, `tTop`, `kBotT`, `tBot`); `planning/master/M6_HEAT_CONTRACT.md` `## Supported`.
- **Reference implementation.** `crates/hydrus-core/src/lower_boundary.rs` for boundary-kind dispatch.
- **Constraints.** Do not edit `mod.rs`, `config.rs`, `capacity.rs` or any file owned by another task. Do not change the water boundary code.
- **Acceptance.** `cargo test -p hydrus-core --test heat_boundary` passes: each supported kind produces the expected boundary row values (worked by hand in test comments); each unsupported kind returns the typed error.
- **Targeted test:** `cargo test -p hydrus-core --test heat_boundary`.

### 24.3.1 — Temper transport solve (heat-only)

- **Repo:** hyd.rs
- **Depends on:** 24.1.2 (capacity), 24.2.2 (boundary).
- **Must edit:** none.
- **Can create / replace:** `crates/hydrus-core/src/heat/transport.rs` (replace placeholder). Unit tests inline under `#[cfg(test)]` in that file.
- **Temp / scratch:** `.development/24.3.1/`.
- **Spec.** Port the heat-only path of `Temper` (`TEMPER.FOR:5-76`): assemble the tridiagonal system for the temperature update from the capacity and conductivity (24.1.2), the advective term from the water flux, and the boundary rows (24.2.2); solve with the existing tridiagonal path in `hydraulics/linear.rs` (no new dependency). Skip every `lVapor` branch (`TempAdj` is out of scope). Respect `dtMaxT` as a substep limit if the source uses it in the heat-only path; record in Notes if it does not.
- **Anchors.** `src_reference/TEMPER.FOR:5-76`; `TEMPER.FOR:37-48` (capacity calls and the `lVapor` guard); `crates/hydrus-core/src/hydraulics/linear.rs`; `crates/hydrus-core/tests/tridiagonal.rs` (solver test pattern).
- **Reference implementation.** `crates/hydrus-core/src/water_solver.rs` for the step-and-report shape (read only; ADR 0002 forbids changing the water solver).
- **Constraints.** Filesystem-free production code. No new dependency (ADR 0011). Do not edit the water solver, `solute*`, or `error.rs`. Do not add vapor terms.
- **Acceptance.** Inline tests: a uniform-temperature state with no flux is unchanged; a mass-conservative step with a known profile matches a hand-computed value; a singular-pivot input returns an error and does not produce NaN.
- **Targeted test:** `cargo test -p hydrus-core --lib heat::transport`.

### 24.3.2 — Heat state and accepted-step snapshot

- **Repo:** hyd.rs
- **Depends on:** 24.2.1 (validated configuration).
- **Must edit:** none.
- **Can create / replace:** `crates/hydrus-core/src/heat/state.rs` (replace placeholder), `crates/hydrus-core/tests/heat_state.rs`.
- **Temp / scratch:** `.development/24.3.2/`.
- **Spec.** `HeatState` holds nodal temperatures and time, mirrors `WaterState` (`water_state.rs:8`), and provides an `AcceptedHeatSnapshot` analogous to `AcceptedWaterSnapshot` (`water_state.rs:68`). Accept/reject is transactional: a rejected coupled step leaves the state unchanged. Validate the node count against the mesh and reject non-finite temperatures.
- **Anchors.** `crates/hydrus-core/src/water_state.rs:8`, `:68`; `crates/hydrus-core/tests/water_transaction.rs` (transaction test pattern); `planning/master/M6_HEAT_CONTRACT.md` `## Variables and precision`.
- **Reference implementation.** `water_state.rs` and `tests/water_transaction.rs`.
- **Constraints.** Do not edit `water_state.rs` (owned by the accepted water path). Do not add history rollback beyond the single accepted snapshot (M7 scope).
- **Acceptance.** `cargo test -p hydrus-core --test heat_state` passes: construction validation, accept, reject-leaves-unchanged, non-finite rejection.
- **Targeted test:** `cargo test -p hydrus-core --test heat_state`.

### 24.4.1 — Coupled water→heat step driver

- **Repo:** hyd.rs
- **Depends on:** 24.2.1, 24.3.1, 24.3.2.
- **Must edit:** none.
- **Can create / replace:** `crates/hydrus-core/src/heat/driver.rs` (replace placeholder), `crates/hydrus-core/tests/heat_driver_order.rs`.
- **Temp / scratch:** `.development/24.4.1/`.
- **Spec.** One coupled step: call `advance_water_step` (`water_solver.rs:183`) first; only on success read the accepted water snapshot (`water_state.rs:68-75`) and advance heat with `transport` (24.3.1) and `state` (24.3.2). Heat never writes to water (D9). A heat failure after a successful water step must roll back or report explicitly; the chosen behavior is recorded in Notes and tested. Reject unsupported configurations first, through `HeatConfig` (24.2.1).
- **Anchors.** `src_reference/HYDRUS.FOR:419` (`WatFlow`), `:491` (`Temper`); `crates/hydrus-core/src/water_solver.rs:183-189`; `crates/hydrus-core/src/water_state.rs:68-75`.
- **Reference implementation.** `crates/hydrus-core/tests/water_solute_coupling.rs` for coupling-test shape.
- **Constraints.** Do not edit `water_solver.rs`, `water_state.rs`, or any solute file. Do not add a second water step.
- **Acceptance.** `cargo test -p hydrus-core --test heat_driver_order` passes: (a) water is advanced before heat (asserted by the snapshot used); (b) an unsupported config is rejected before any water step runs; (c) a uniform-temperature column with zero flux stays uniform across coupled steps; (d) a heat failure after a water success is handled as recorded in Notes.
- **Targeted test:** `cargo test -p hydrus-core --test heat_driver_order`.

### 24.4.2 — Heat-energy balance accounting

- **Repo:** hyd.rs
- **Depends on:** 24.3.1.
- **Must edit:** none.
- **Can create / replace:** `crates/hydrus-core/src/heat/balance.rs` (replace placeholder). Unit tests inline.
- **Temp / scratch:** `.development/24.4.2/`.
- **Spec.** Accumulate the heat-storage change and the boundary heat flux and sink contributions over steps, and report the residual per step. This is heat-only accounting; no surface-energy terms (out of scope). Expose a `HeatBalanceReport` with storage change, boundary inflow/outflow, and residual, all finite.
- **Anchors.** `src_reference/TEMPER.FOR:5-10` (`Sink`, `HeatFl` arguments); `crates/hydrus-core/src/water_state.rs:18` (`WaterAccounting` pattern for accumulated quantities).
- **Reference implementation.** `water_state.rs:18-24` (`WaterAccounting`).
- **Constraints.** Do not edit any other heat file. Do not add energy balance (`lEnBal`).
- **Acceptance.** Inline tests: zero-flux closed column has zero residual; a known-flux step's storage change matches the hand-computed value; non-finite input is rejected.
- **Targeted test:** `cargo test -p hydrus-core --lib heat::balance`.

### 24.4.3 — Pinned-Fortran heat decks and fixture producer

- **Repo:** hyd.rs
- **Depends on:** 24.1.1 (the contract names the cases and the output fields).
- **Must edit:** none.
- **Can create:** `reference/heat_fortran/README.md`; `reference/heat_fortran/cases/<case-id>/` (SELECTOR.IN and any other inputs the deck needs; heat-only: `lTemp` on, `lVapor` off, `lEnBal` off); `reference/heat_fortran/fixtures/<case-id>.json` (produced by the producer, committed); `tools/run_heat_reference.py`; `tools/test_run_heat_reference.py`.
- **Temp / scratch:** `.development/24.4.3/`; the Fortran build goes in `.development/24.4.3/reference-build/` (inside the worktree, untracked).
- **Spec.**
  1. Stage and build the pinned reference per `reference/fortran_reference/README.md` (`tools/stage_fortran_reference.py`, `tools/build_fortran_reference.sh --build-dir <path>`). Do not edit `src_reference/` or the patch.
  2. Author 3 to 5 heat-only decks, each covering one supported boundary kind from the contract. Record the Fortran input fields in each deck's README entry.
  3. `tools/run_heat_reference.py`: runs the built executable on each case in an isolated task root, records the manifest (source hash, patch hash, executable hash, command), extracts the per-node temperature from the output named in `output_compat.rs:162`, and writes `fixtures/<case-id>.json` with `source_matched: true`, the time points, and the node coordinates. Follow ADR 0003 (isolate and authenticate runs).
  4. `tools/test_run_heat_reference.py`: unit tests on the parser and manifest writer with small synthetic text inputs (no Fortran run required).
- **Anchors.** `reference/fortran_reference/README.md`; `tools/run_fortran_reference.py` (read only, for its isolation pattern); `crates/hydrus-core/src/solute_fortran_compat/output_compat.rs:162` (temperature output).
- **Reference implementation.** `tools/run_fortran_reference.py` (read only).
- **Constraints.** Do not edit `tools/run_fortran_reference.py`, `tools/stage_fortran_reference.py`, `src_reference/`, `reference/solute_verification/`, or any solute tool. Do not run holdout rows (none exist here). Each deck must be reachable under ADR 0007: prove the heat branch is active for the deck's selectors (cite the line in the README).
- **Acceptance.** `python3 -m unittest tools.test_run_heat_reference` passes; at least 3 `fixtures/*.json` files exist, each with `source_matched: true` and a recorded executable hash; the manifest for each fixture records the build hash.
- **Targeted test:** `python3 -m unittest tools.test_run_heat_reference`.

### 24.5.1 — Analytical comparisons

- **Repo:** hyd.rs
- **Depends on:** 24.4.1.
- **Must edit:** none.
- **Can create:** `crates/hydrus-core/tests/heat_analytical.rs`.
- **Temp / scratch:** `.development/24.5.1/`.
- **Spec.** Compare the coupled heat driver against closed-form solutions: (a) steady conduction with Dirichlet top and bottom temperatures gives a linear profile; (b) steady conduction with a constant top flux gives the expected gradient; (c) a transient pure-conduction case with Dirichlet ends matches a truncated Fourier-series value at fixed times. Each case states its tolerance (from the contract's `## Tolerances`, not chosen after seeing results). Label each case `analytical-only` in the test name.
- **Anchors.** `planning/master/M6_HEAT_CONTRACT.md` `## Tolerances`; `crates/hydrus-core/src/heat/driver.rs` (24.4.1).
- **Reference implementation.** `crates/hydrus-core/tests/solute_analytical`-style tests if present (check `tests/solute_outlet.rs` for the pattern); otherwise `tests/solute_transport.rs`.
- **Constraints.** Do not tune any tolerance to pass. If a case fails, report it under `## Discovered`. Do not edit production heat files; a fix is a re-scope.
- **Acceptance.** `cargo test -p hydrus-core --test heat_analytical` passes at the contract tolerances, or the failing case is reported with its value and the tolerance.
- **Targeted test:** `cargo test -p hydrus-core --test heat_analytical`.

### 24.5.2 — Source-matched Fortran parity

- **Repo:** hyd.rs
- **Depends on:** 24.4.1, 24.4.3.
- **Must edit:** none.
- **Can create:** `crates/hydrus-core/tests/heat_fortran_parity.rs`, `tools/compare_heat_reference.py`, `tools/test_compare_heat_reference.py`.
- **Temp / scratch:** `.development/24.5.2/`.
- **Spec.** For each fixture from 24.4.3, rebuild the same inputs (mesh, initial temperature, boundary, time points) from the case's recorded fields, run the coupled driver, and compare the nodal temperature at each recorded time against the fixture using the contract's per-field tolerances. The comparison runs on the **fixture values, not on a live Fortran run**. `tools/compare_heat_reference.py` reads the same fixture JSON and produces the same verdict table, so the two sides can be checked against each other.
- **Anchors.** `reference/heat_fortran/fixtures/*.json` (24.4.3); `planning/master/M6_HEAT_CONTRACT.md` `## Tolerances`; `tools/run_solute_parity.py` (read only, for the comparator style).
- **Reference implementation.** `crates/hydrus-core/tests/solute_fixture_parity.rs` for fixture-driven parity shape.
- **Constraints.** Do not edit `tools/run_solute_parity.py` comparison logic or `OBSERVATION_MAP.json` (ADR 0006). Do not widen any tolerance. Report f32 rounding floors; never fit an order through one.
- **Acceptance.** `cargo test -p hydrus-core --test heat_fortran_parity` and `python3 -m unittest tools.test_compare_heat_reference` pass; the verdict table lists each fixture, field, error and tolerance.
- **Targeted tests:** `cargo test -p hydrus-core --test heat_fortran_parity`; `python3 -m unittest tools.test_compare_heat_reference`.

### 24.5.3 — Balance and invariant properties

- **Repo:** hyd.rs
- **Depends on:** 24.4.1, 24.4.2.
- **Must edit:** none.
- **Can create:** `crates/hydrus-core/tests/heat_balance.rs`.
- **Temp / scratch:** `.development/24.5.3/`.
- **Spec.** Property checks on the coupled driver over a fixed set of seeded inputs: heat storage change plus boundary flux equals the residual within tolerance over many steps; zero-flux uniform state does not drift; temperatures stay finite; the balance residual does not grow with step count beyond the contract tolerance. Use fixed seeds and list them in the test.
- **Anchors.** `crates/hydrus-core/src/heat/balance.rs` (24.4.2); `planning/master/M6_HEAT_CONTRACT.md` `## Tolerances`.
- **Reference implementation.** `crates/hydrus-core/tests/water_acceptance.rs` for invariant-style tests.
- **Constraints.** No new dependency for random inputs; use a hand-rolled deterministic generator. Do not edit production heat files.
- **Acceptance.** `cargo test -p hydrus-core --test heat_balance` passes.
- **Targeted test:** `cargo test -p hydrus-core --test heat_balance`.

### 24.6.1 — Capability acceptance document

- **Repo:** hyd.rs
- **Depends on:** 24.5.1, 24.5.2, 24.5.3.
- **Must edit:** none.
- **Can create:** `planning/master/M6_HEAT_ACCEPTANCE.md`.
- **Temp / scratch:** `.development/24.6.1/`.
- **Spec.** Document the implemented heat/temperature capability: supported configuration, rejected combinations (from the contract), evidence per exit criterion (analytical, source-matched, balance), the exact commands that produced the evidence, and a `## Not claimed` section stating that vapor, energy balance, hysteresis, dual porosity and universal accuracy are not claimed. Each evidence line cites the test name and the fixture or case ID. Do not claim any tolerance the evidence did not meet.
- **Anchors.** `planning/master/M6_HEAT_CONTRACT.md`; the test files from 24.5.x; `planning/master/M5V_M6_HANDOFF_DECISION.md` for the "no universal tolerance" wording.
- **Reference implementation.** `planning/master/M5V_M6_HANDOFF_DECISION.md` structure.
- **Constraints.** Documentation only. Do not update `planning/GOALS.md` or `planning/STATE_24.md` (the orchestrator owns them).
- **Acceptance.** `grep -n '^## Not claimed' planning/master/M6_HEAT_ACCEPTANCE.md` matches; every evidence line names a test or fixture that exists (`grep` check by the orchestrator).
- **Targeted test:** none (doc task); orchestrator verifies the cited names exist.

### 24.6.2 — Draft ADR for heat coupling and rejection

- **Repo:** hyd.rs
- **Depends on:** 24.6.1.
- **Must edit:** `docs/decisions/README.md` (add one row).
- **Can create:** `docs/decisions/0021-<slug>.md` (0020 is taken by "Keep architecture decisions distinct from project policy", accepted 2026-11-11) (slug chosen by the worker from the title "Couple heat transport one-way after the accepted water step and reject unsupported heat configurations").
- **Temp / scratch:** `.development/24.6.2/`.
- **Spec.** Use adr-distillation to write one ADR recording D9 (one-way coupling after the accepted water step) and D7 (typed rejection of unsupported heat configurations), with the Fortran lines that establish each. Status **Proposed**. State that no existing ADR is superseded. The README row gets the same status.
- **Anchors.** `planning/master/M6_HEAT_CONTRACT.md`; `docs/decisions/0019-*.md` (format); `docs/decisions/README.md`.
- **Reference implementation.** `docs/decisions/0018-default-to-the-fortran-compatible-solute-backend.md` (format and supersession wording).
- **Constraints.** Do not edit other ADRs. Do not mark the ADR Accepted.
- **Acceptance.** `ls docs/decisions/0021-*.md` exists; `grep -n '0021' docs/decisions/README.md` matches; `grep -n 'Proposed' docs/decisions/0021-*.md` matches.
- **Targeted test:** none (doc task).

---

## Discovered

(Workers record off-plan findings here. Stop and report. No entries yet.)

## Judgment calls for user approval

1. **Worker model tier** (D11): `inherit`, one escalation retry, following Orchestration 23. Alternative: a named cheap tier.
2. **Parity by fixture JSON, not a heat deck reader** (D6). Live parity would widen `solute_fortran_compat/`.
3. **`f64` in heat production code** (D8). Fortran single-precision points are recorded and tested in the contract, not replicated by default.
4. **Heat-only rejection of `lVapor`/`lEnBal`** (D7). This is the only way G14's "reject unsupported combinations" holds until G19/G20 land.
5. **Pre-wave-1 gate** (D12): commit or stash the 16 uncommitted planning files before Wave 1.
6. **Two new goals to log** (not executed here): G19 vapor flow and G20 energy balance, each with a trigger in GOALS.md at wave close.

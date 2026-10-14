# Orchestration 19 — Architecture deepening interlude

## Goal and scope

**Goal.** Resolve the findings of the 2026-10-14 architectural review of `phase4-water-acceptance` (`/reviewer-architecture`) and its deepening report (`/improve-codebase-architecture`, `/tmp/architecture-review-20261014-002835.html`). The goal is to deepen the water step and the solute compatibility modules before Orchestration 18 writes the solute driver (18.10.1) and exposes the backend selector (18.12.1). All work follows the ADRs in `docs/decisions/` (0001–0012 at `1747e0c`).

**In scope** (report candidate numbers in brackets):
- [1] The compat request adapter calls each kernel's own `validate` instead of a parallel copy of the same checks. The primitive request fields stay.
- Review hygiene in `solute_fortran_compat`:
  - a blanket `#![allow(dead_code)]` becomes per-module allowances naming the wiring task;
  - one kernel error alias;
  - consistent item visibility in `types.rs`;
  - stub docs reconciled with their owning 18 tasks;
  - a "do not unify" pointer on the precision-faithful `FH`/`FK` ports;
  - `adapter.rs` (2590 lines) split into submodules.
- [2] The water step publishes the accepted water snapshot (water contents plus `node_count + 1` face fluxes) that the solute step consumes.
- [3] The atmospheric module owns upper-boundary mode switching, row building, top flux and ponding transitions.
- [4] `WaterStepInput` takes one boundary value per endpoint. Rows are derived inside the step, and `hCritA` is no longer encoded in a Dirichlet value. Shared water-test fixtures go in `tests/common/`.
- [5] Closed `SoluteBackend` enum dispatch with one `FiniteVolumeBackwardEuler` variant. Public `SoluteWorkspace` mutators are removed. Contract tests run over every variant.
- [6] `HydraulicModel` provides `theta`/`conductivity`/`capacity` from `evaluate`.
- [7] serde-derived request-record decoding in the adapter. The duplicate-key pre-pass and canonical hash are kept.
- ADRs 0013–0016, a root `CONTEXT.md`, and amendments to every Orchestration 18 task whose files, names or conventions this plan changes.

**Out of scope:**
- Any compat kernel formula, operation-order or precision change (ADR-0010).
- Any FV/BE equation change (ADR-0002).
- The `FortranCompatible` variant (18.12.1).
- The solute driver and the primitive-state builders it will contain (D6, 18.10.1).
- The duplicated finiteness helpers and face-conductivity averaging copies, beyond what [3]/[4] move.
- The `advance_solute_step` / `_with_processes` option-flag pass-through.
- `tools/**`, fixtures, pinned Fortran source.
- Changing the default solute backend.

**Constraints stated by the user** (verbatim answers, 2026-10-14):
- Sequencing: "Pause 18, run 19 now."
- Scope: "Compat refactor + review fixes, Water-step deepening, HydraulicModel trait shrink, Solute seam + serde (#5, #7)".
- API: "Break freely, update tests."
- Solute seam: "Enum-dispatched step, FV/BE only."
- serde: "Add serde derive."
- Workers: "Sonnet, ≤3 parallel."
- Acceptance: "Same + restore Python first", i.e. bitwise-preserving results plus Rust gates, plus the Python suite restored and re-baselined before wave 1.
- [1] depth: "Keep fields, dedupe validation."
- ADRs: "docs/decisions/ + CONTEXT.md."

## Decision log

| Date | Decision |
|---|---|
| 2026-10-14 | D1. Orchestration 18 pauses before 18.7.8. It resumes after Orchestration 19's wave 7. Pending 18 work at resume: 18.7.2, 18.7.5, 18.7.6 and 18.7.7 (fixtures and root uptake) and 18.7.8, then Waves 8–16, all under the 19.7.2 amendments. |
| 2026-10-14 | D2. All seven report candidates and the review hygiene items are in scope. |
| 2026-10-14 | D3. Public `hydrus_core` interface changes are allowed without deprecation. Every caller and test migrates in the same task. The orchestrator collects old→new renames in `STATE_19.md`, and 19.7.2 applies them to Orchestration 18 specs. |
| 2026-10-14 | D4. The solute seam is a closed `#[non_exhaustive]` `SoluteBackend` enum selected on `SoluteConfig` (default `FiniteVolumeBackwardEuler`), not a trait. 18.12.1 adds `FortranCompatible`. 18.16.1's default switch becomes a change of `SoluteConfig`'s default backend. |
| 2026-10-14 | D5. Add `serde` with `derive`. This is the dependency amendment ADR-0011 requires. Owner: task 19.4.1 (the only task that may edit `Cargo.toml`/`Cargo.lock`). Reason: replace the shallow hand-written field decoders. Lock: `serde_derive` 1.0.229 and its proc-macro deps are already locked, so no new packages. Recorded as ADR-0016. Unknown-field rejection uses `deny_unknown_fields`. Rejection tests assert the offending field name, not exact message text. |
| 2026-10-14 | D6. `endpoint_primitives` and `dual_porosity_exchange_state` stay as request fields. Per-state shape checks live only in the kernel's `validate`, which the adapter calls before mutation. Cross-field checks that the kernel cannot see stay in the adapter. |
| 2026-10-14 | D7. Workers are Sonnet 5.5, at most 3 concurrent, each in its own git worktree. The escalation tier is Opus 5.5. |
| 2026-10-14 | D8. Acceptance is bitwise-preserving: no existing numeric expected value or tolerance in any test may change, and refactors reproduce results bit for bit. Test counts are unchanged except for deletions a task names explicitly. The Python suite gates every wave. |
| 2026-10-14 | D9. New ADRs go in `docs/decisions/` as 0013–0016, in the format of `0002-keep-fvbe-and-source-compatible-backends-separate.md`. `CONTEXT.md` at the repo root holds the domain glossary. |
| 2026-10-14 | Fact (recon). The `STATE_18.md` Python blocker is stale. System `/usr/bin/python3` 3.12.3 runs `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` with 291 tests OK (147 s) and leaves the tree clean. |
| 2026-10-14 | Template deviation (standards review). Workers report to the orchestrator instead of appending to `STATE_19.md`, because a worker-era replacement once corrupted `STATE_18.md`. The orchestrator distills each report into state. |

## Wave table

| Wave | Task | Title | Must-edit / create (disjoint within wave) |
|---|---|---|---|
| 0 | — | Orchestrator baseline (no worker) | `planning/STATE_19.md`, `planning/STATE_18.md` |
| 1 | 19.1.1 | Compat adapter delegates shape checks to kernels | `compat/{adapter,boundary,dual_domain}.rs` |
| 1 | 19.1.2 | `HydraulicModel` provided projections | `hydraulics/{mod,table}.rs`, list W |
| 1 | 19.1.3 | ADRs 0013–0016 and `CONTEXT.md` | `docs/decisions/*`, `CONTEXT.md` |
| 2 | 19.2.1 | Compat hygiene: dead code, error alias, visibility, stub docs | list C |
| 2 | 19.2.2 | Deepen the atmospheric upper-boundary module | `atmospheric.rs`, `water_solver.rs` |
| 3 | 19.3.1 | Split the request adapter into submodules | `compat/adapter.rs` → `compat/adapter/` |
| 3 | 19.3.2 | Shared water test fixtures | `tests/common/mod.rs`, list W |
| 3 | 19.3.3 | Remove public solute workspace mutators | `solute.rs`, `tests/solute_transaction.rs` |
| 4 | 19.4.1 | serde decoding: dependency + transport group | `Cargo.toml`, `Cargo.lock`, `compat/adapter/{fields,transport}.rs` |
| 4 | 19.4.2 | One lower-boundary value in `WaterStepInput` | `water_solver.rs`, `lower_boundary.rs`, `lib.rs`, `tests/common/mod.rs`, list W |
| 5 | 19.5.1 | serde decoding: reactions + transport-property groups | `compat/adapter/{reactions,transport_properties}.rs` |
| 5 | 19.5.2 | serde decoding: nonequilibrium + biology groups | `compat/adapter/{nonequilibrium,biology}.rs` |
| 5 | 19.5.3 | One upper-boundary value in `WaterStepInput` | `water_solver.rs`, `atmospheric.rs`, `error.rs`, `lib.rs`, `tests/common/mod.rs`, list W |
| 6 | 19.6.1 | Remove orphaned adapter decode helpers | `compat/adapter/{fields,mod}.rs` |
| 6 | 19.6.2 | Water step publishes the accepted water snapshot | `water_solver.rs`, `water_state.rs`, `solute.rs`, `lib.rs`, `tests/water_solute_coupling.rs` (new) |
| 7 | 19.7.1 | Closed `SoluteBackend` dispatch + contract tests | `solute.rs`, `lib.rs`, `tests/solute_backend_contract.rs` (new) |
| 7 | 19.7.2 | Amend Orchestration 18 for 19's changes | `planning/EXECUTION_PLAN_18.md` |

**Path abbreviations.** Paths are relative to `crates/hydrus-core/src/` unless they start with `tests/` (`crates/hydrus-core/tests/`), `docs/`, `planning/`, `Cargo` or `CONTEXT.md`. `compat/` means `solute_fortran_compat/`.

**List W** (the eight water test files, the only callers of `WaterStepInput` and the only test implementations of `HydraulicModel`): `tests/{lower_boundary,water_solver,allocation_contract,atmospheric,water_transaction,water_contract,root_uptake,water_acceptance}.rs`.

**List C** (19.2.1's 14 files): `compat/{mod,types,adapter,accounting,assembly,boundary,dual_domain,transport,biology,deposition,driver,iteration,nonequilibrium,processes}.rs`.

**Dependency chains:**
- compat: 19.1.1 → 19.2.1 → 19.3.1 → 19.4.1 → {19.5.1, 19.5.2} → 19.6.1
- water: 19.1.2 → 19.2.2 → 19.3.2 → 19.4.2 → 19.5.3 → 19.6.2
- solute: 19.3.3 → 19.6.2 (moves `AcceptedWaterSnapshot` out of `solute.rs`) → 19.7.1
- docs: 19.1.3, then 19.7.2 (after every rename exists)

Waves are sequential, so every task also depends on all earlier waves.

## Hazards (from reconnaissance and plan review)

- **Large files.** Enter them through `grep -n` anchors only, never a whole-file read:
  - `adapter.rs` (2590 lines; tests from `:1867`)
  - `solute.rs` (1474)
  - `boundary.rs` (1279)
  - `dual_domain.rs` (898)
  - `water_solver.rs` (792)
- **ADR-0010.** Compat kernels keep source expression grouping, call order and `f32`/`f64` conversion points. Clippy fixes in kernel files use a targeted `#[allow(...)]` that cites the source line, never a rewrite (precedent `6394d52`).
- **ADR-0004.** The request schema is a reviewed contract. The serde work must accept and reject exactly the same requests. serde differs from the hand decoders on:
  - explicit `null` (`Option<T>` accepts it);
  - integer versus float coercion;
  - missing `units` in a `{units, value}` wrapper.

  Each serde task adds rejection tests for these. The canonical SHA-256 is still computed over the parsed `serde_json::Value`, after `reject_duplicate_keys`. serde_json keeps the last of two duplicate keys, so the pre-pass is load-bearing.
- **ADR-0002.** No FV/BE equation changes. `SoluteBackend` adds selection only.
- **ADR-0011.** Only 19.4.1 may edit `Cargo.toml`/`Cargo.lock`.
- **Mapper signatures.** `map_fields` (`adapter.rs:~211-430`, later `adapter/mod.rs`) calls each `maybe_*` mapper by name (e.g. `:368-369`). The serde tasks must keep every mapper's name and signature so `mod.rs` needs no edit until 19.6.1.
- **Dead code.** Nothing outside `solute_fortran_compat` calls into it (`lib.rs:10`), so about 155 items are dead in the non-test build. Allowances are per module in `compat/mod.rs` (`#[allow(dead_code)] // wired by 18.10.1` above each `mod` line), not per item.
- **Allocation contract.** `tests/allocation_contract.rs` requires steps not to allocate. Any new workspace buffer (19.6.2 face fluxes) is sized at `WaterWorkspace` construction.
- **Atmospheric capacity.** Atmospheric steps need `WaterWorkspace::with_transition_capacity(n, max_iterations)`, or they fail with `InsufficientAtmosphericTraceCapacity` (`water_solver.rs:147-154`).
- **Ignored or overloaded inputs.**
  - `lower_row` is ignored for `PrescribedFlux`/`FreeDrainage` (`water_solver.rs:442`).
  - `upper_row` is ignored in atmospheric mode but still validated (`:784-790`).
  - `hCritA` is extracted from `upper_condition: Dirichlet{value}` at `:341-347`/`:479-486`. Once moved by 19.2.2, these are in `atmospheric.rs`.
- **State-file integrity.** Workers never edit `STATE_*.md`.
- **Worker stalls.** Orchestration 18 workers hit a 30-minute async wall while idle at the plan gate. Keep Analysis time-boxed. The orchestrator asks for an interim `git diff --stat` during long executions.
- **Worktrees.** Use worktrees under `.development/19-worktrees/<id>` on branch `orchestration/<id>`. Never stage `target/` or `.development/`.

## Shared acceptance rules (every task)

- **A1.** No numeric literal in any existing test assertion or tolerance changes (D8). The orchestrator reviews `git diff <base> -- crates/hydrus-core/tests` and any moved `#[cfg(test)]` code. A move must preserve the literal set.
- **A2.** The task's targeted commands pass. The worker reports test counts before and after. Counts are unchanged except for deletions the spec names. Moved tests are counted at their new location.
- **A3.** `cargo fmt --all -- --check` and `cargo clippy -p hydrus-core --all-targets -- -D warnings` are clean in the worktree.
- **A4.** At wave end the orchestrator runs serially:
  - `cargo test --workspace`
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`
  - fmt
  - clippy `-D warnings`
  - `git diff --check`

  It records the results against the baseline in `STATE_19.md`, along with any D3 renames from the wave's reports.

## Wave 0 — Orchestrator baseline (no worker)

1. Record the baseline at the approved HEAD in `STATE_19.md` (A4 suite list; 202 Rust and 291 Python at recon).
2. In `STATE_18.md`:
   - set Execution status to **paused for Orchestration 19 before 18.7.8** and list the pending 18 tasks (D1);
   - correct the Python blocker, so that 18.7.6 is blocked only on fixtures.
3. Commit the plan files and state with `docs(planning): pause solute compatibility work for architecture deepening`.

---

## Wave 1 — Validation, projections, decisions

### 19.1.1 — Compat adapter delegates shape checks to kernels

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/{adapter,boundary,dual_domain}.rs`.
- **Can create:** `planning/tasks/19.1.1.todo.md`; scratch `.development/19.1.1/`.
- **Depends on:** Wave 0.
- **Spec:**
  - Expose the kernel validators as `pub(super)`:
    - `boundary::validate(state, lower_code, upper_code)` (`boundary.rs:116`);
    - `dual_domain::validate(state)` (`dual_domain.rs:308`).
  - In `validate_cross_field_dimensions` (`adapter.rs:494`), replace the per-state shape and range checks with calls to these validators:
    - the `endpoint_state` block, `~:666-800`;
    - the `dual_porosity_exchange_state` block, `~:590-635`.

    Map the error to `String` with the field name as prefix (`format!("endpoint_primitives: {e}")`).
  - Keep in the adapter every check the kernel cannot perform:
    - both boundary codes present (`~:802`);
    - `dual_porosity` agreeing with `dual_domain.iDualPor` (`~:618-622`);
    - models 6/7 reaching `DualPor` as `iModel 0` (`~:631`).
  - An adapter-only shape check that the kernel lacks moves into the kernel validator.
  - Every request currently rejected is still rejected before mutation.
- **Do not touch:** kernel formulas, request field names and JSON shape, `types.rs`.
- **Anchors:**
  - `fn validate_cross_field_dimensions`
  - `if let Some(state) = &fields.transport.endpoint_state`
  - `fields.nonequilibrium.dual_porosity_exchange_state`
  - tests `endpoint_primitives_reject_derived_and_inconsistent_state` (`~:2064`) and `dual_porosity_exchange_state_rejects_unknown_missing_and_ragged_state` (`~:2358`)
- **Reference:** `assemble_endpoints` → `validate` (`boundary.rs:101-106`).
- **Acceptance:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat` passes with 57 tests.
  - Each rejection test still rejects its case. Only the asserted substring may change, and it must still name the offending field.
  - The report lists the removed adapter line ranges.
  - A1–A3.

### 19.1.2 — `HydraulicModel` provided projections

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/hydraulics/{mod,table}.rs`, list W.
- **Can create:** `planning/tasks/19.1.2.todo.md`.
- **Depends on:** Wave 0.
- **Spec:**
  - Give `theta`, `conductivity` and `capacity` default bodies on `HydraulicModel` (`hydraulics/mod.rs:39-45`), each `Ok(self.evaluate(head)?.<field>)`.
  - Delete the overrides in `VanGenuchtenModel` (`:64`) and `HydraulicTables` (`table.rs:161`). Review confirmed both are exact projections.
  - In each list-W file, delete test-model overrides that are exact projections of that model's `evaluate`. Keep and report any that differ.
- **Do not touch:** `evaluate`/`derivatives` bodies, `van_genuchten.rs`, `water_solver.rs`, non-W tests.
- **Anchors:** `pub trait HydraulicModel`; `impl HydraulicModel for` in src and tests.
- **Reference:** `hydraulics/mod.rs:65-75`.
- **Acceptance:**
  - `cargo test -p hydrus-core --test hydraulic_curves --test hydraulic_tables` passes.
  - The list-W command passes with unchanged per-file counts: `cargo test -p hydrus-core --test lower_boundary --test water_solver --test allocation_contract --test atmospheric --test water_transaction --test water_contract --test root_uptake --test water_acceptance`.
  - A1–A3.

### 19.1.3 — ADRs 0013–0016 and `CONTEXT.md`

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `docs/decisions/README.md`.
- **Can create:**
  - `docs/decisions/0013-select-solute-backend-with-a-closed-enum.md`
  - `docs/decisions/0014-validate-kernel-primitive-state-in-the-kernel.md`
  - `docs/decisions/0015-keep-precision-faithful-hydraulic-ports-in-compat-kernels.md`
  - `docs/decisions/0016-add-serde-derive-for-request-decoding.md`
  - `CONTEXT.md`
  - `planning/tasks/19.1.3.todo.md`
- **Depends on:** Wave 0.
- **Spec:**
  - Write four ADRs in the front-matter and section format of `0002-keep-fvbe-and-source-compatible-backends-separate.md`, with `status: "Accepted"` and `date: 2026-10-14`:
    - **0013:** D4; cites 0002.
    - **0014:** D6; cites 0004.
    - **0015:** `dual_domain.rs` `retention_head`/`conductivity` (`FH`/`FK`) duplicate `hydraulics::van_genuchten` at source precision on purpose and must not be unified; cites 0010.
    - **0016:** D5, the ADR-0011 amendment that names task 19.4.1, its reason and the zero-new-package lock fact. It states that `hydrus-core`'s dependency list becomes `serde`, `serde_json`, `sha2`, and cites 0011 (not superseding it).
  - Add four README index rows in the existing column format.
  - Create `CONTEXT.md` with one bold-term entry each, naming the Rust type or module and the Fortran anchor where one exists:
    - water step
    - upper boundary (flux/head/atmospheric, `hCritA`)
    - lower boundary mode
    - accepted water snapshot
    - solute step
    - solute backend
    - compatibility request
    - kernel primitive state
    - source switches
    - fixed-form continuation
- **Do not touch:** ADRs 0001–0012, any source or planning file.
- **Anchors:** the `0002-…` file (format); the README table header.
- **Acceptance:**
  - `ls docs/decisions/001[3456]-*.md | wc -l` = 4.
  - `grep -c '^| \[001[3456]\]' docs/decisions/README.md` = 4.
  - `git diff --stat -- 'docs/decisions/000*' 'docs/decisions/001[012]*'` is empty.
  - `grep -c '^\*\*' CONTEXT.md` ≥ 10.
  - `git diff --check` is clean.

---

## Wave 2 — Ownership moves (no behavior change)

### 19.2.1 — Compat hygiene: dead code, error alias, visibility, stub docs

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** list C.
- **Can create:** `planning/tasks/19.2.1.todo.md`.
- **Depends on:** 19.1.1, 19.1.3.
- **Spec:**
  1. **Dead code.** Replace `#![allow(dead_code)]` (`mod.rs:2`) with a `#[allow(dead_code)] // wired by 18.10.1` attribute line above each `mod` declaration whose items are still unreached, using `18.12.1` for `adapter`/`types`. No item-level allowances.
  2. **Error alias.** Add `pub(crate) type KernelError = &'static str;` to `types.rs`, then replace `Result<_, &'static str>` with `Result<_, KernelError>` in kernel files. This is a type-alias change only. The adapter keeps `String`.
  3. **Visibility.** Change top-level `pub struct`/`pub enum`/`pub fn`/`impl` methods in `types.rs` to `pub(crate)`. Fields stay `pub`; the module is private, so fields are crate-visible already, matching `transport.rs:7-47`.
  4. **Stub docs.** Correct each placeholder's doc comment to its current owning task in `EXECUTION_PLAN_18.md` (e.g. `driver.rs` → 18.10.1, `iteration.rs` → 18.11.1). Grep the plan for each file name.
  5. **Unify pointer.** Add a one-line doc note above `retention_head` and `conductivity` in `dual_domain.rs` (`:227`, `:255`): "Intentionally not shared with `hydraulics`; see ADR-0015."
- **Do not touch:** any expression, statement order or numeric literal (ADR-0010), and any name other than the alias.
- **Anchors:** `mod.rs:2-16`; `grep -n "&'static str>" compat/*.rs`; `grep -n '^pub ' compat/types.rs`; `grep -n 'solute_fortran_compat/<file>' planning/EXECUTION_PLAN_18.md`.
- **Acceptance:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat` passes with unchanged count.
  - `grep -n '#!\[allow(dead_code)\]' -r crates/hydrus-core/src/solute_fortran_compat` returns nothing.
  - `grep 'allow(dead_code)' crates/hydrus-core/src/solute_fortran_compat/mod.rs | grep -vc 'wired by'` = 0.
  - `grep -c '^pub \(struct\|enum\|fn\)' crates/hydrus-core/src/solute_fortran_compat/types.rs` = 0.
  - A1–A3.

### 19.2.2 — Deepen the atmospheric upper-boundary module

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/{atmospheric,water_solver}.rs`.
- **Can create:** `planning/tasks/19.2.2.todo.md`.
- **Depends on:** 19.1.2.
- **Spec:**
  - Move into `atmospheric.rs`, behind a small `pub(crate)` interface that `water_solver` calls each iteration:
    - `atmospheric_mode` (`water_solver.rs:600`)
    - `atmospheric_top_row` (`:624`)
    - `atmospheric_top_flux` (`:555`)
    - `atmospheric_top_flux_for_heads` (`:573`)
    - the mode/ponding transition block in `solve_trial` (`:330-374`)
    - the two `hCritA` matches (`:341-347`, `:479-486`)
  - The interface is a row for trial heads, the soil-surface flux, and a transition decision returning `AtmosphericTransition`. The worker may refine it in the micro-plan.
  - Merge the two top-flux functions only if the merged expression evaluates in identical order for both callers. Otherwise move both.
  - The moved code may keep writing `pub(crate)` `WaterWorkspace` fields; `water_state.rs` needs no edit.
  - Add ≥3 unit tests in `atmospheric.rs` that drive flux↔head switching and ponding onset/drain through the new interface without a full step.
- **Do not touch:** `WaterStepInput` and all public types (19.4.2/19.5.3), `lower_boundary.rs`, `water_state.rs`, tests, arithmetic order.
- **Anchors:** `fn atmospheric_mode`, `fn atmospheric_top_row`, `fn atmospheric_top_flux`, `fn solve_trial`, `BoundaryCondition::Dirichlet` in `water_solver.rs`; `pub fn advance_atmospheric_step` in `atmospheric.rs`.
- **Reference:** `LowerBoundaryMode::prepare_step` (`lower_boundary.rs:182`).
- **Acceptance:**
  - `cargo test -p hydrus-core --test atmospheric --test water_acceptance --test water_solver --test allocation_contract` passes with unchanged counts.
  - `cargo test -p hydrus-core --lib atmospheric` passes with ≥3 tests.
  - `grep -c 'fn atmospheric_' crates/hydrus-core/src/water_solver.rs` = 0.
  - A1–A3.

---

## Wave 3 — Structure and fixtures

### 19.3.1 — Split the request adapter into submodules

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/adapter.rs`, which is deleted.
- **Can create:** these files under `crates/hydrus-core/src/solute_fortran_compat/adapter/`:
  - `mod.rs`: `parse_case_manifest`, `parse_request`, `canonical_hash`, `parse_root_value`, `map_fields`, the `derive_*` functions, `validate_cross_field_dimensions` dispatch
  - `json_guard.rs`: `reject_duplicate_keys`, `finite_tree`, `validate_field`, `validate_object_keys`
  - `fields.rs`: the `as_*`/`maybe_<scalar>`/`unit_value`/`explicit` helpers
  - group files, each holding that group's record mappers and its block of cross-field checks:
    - `transport.rs`
    - `reactions.rs`
    - `transport_properties.rs`
    - `nonequilibrium.rs`
    - `biology.rs`
  - `tests.rs`: the former `mod tests`
  - `planning/tasks/19.3.1.todo.md`
- **Depends on:** 19.2.1.
- **Spec:**
  - A pure move. No logic, message, name or signature changes beyond `pub(super)` visibility.
  - The one addition is an `#[ignore]`d test `dump_parsed_fixtures` in `tests.rs`. It writes, to the path in env var `HYDRUS_ADAPTER_SNAPSHOT`:
    - the `{:?}` of `parse_case_manifest` for every fixture `CASE.json` (sorted path order, reusing `walkdir`);
    - the `Err` text of every request literal in the existing rejection tests, collected into a `const` list.

    Later serde tasks diff this snapshot before and after.
  - The `mod adapter;` line in the parent `mod.rs` is unchanged; its 19.2.1 attribute is kept.
- **Do not touch:** any other file.
- **Anchors:** `grep -n '^fn \|^pub(crate) fn ' adapter.rs`; `pub(crate) struct RequestFields` groups in `types.rs`; `fn walkdir` (`~:1894`).
- **Acceptance:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat` passes with unchanged count (the ignored test is not counted).
  - `HYDRUS_ADAPTER_SNAPSHOT=$PWD/.development/19.3.1/snap.txt cargo test -p hydrus-core --lib dump_parsed_fixtures -- --ignored` writes a non-empty file.
  - No adapter file exceeds 900 lines.
  - Insertions ≈ deletions ±5% in `git diff --stat`.
  - A1–A3.

### 19.3.2 — Shared water test fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** list W.
- **Can create:** `crates/hydrus-core/tests/common/mod.rs`, `planning/tasks/19.3.2.todo.md`.
- **Depends on:** 19.1.2.
- **Spec:**
  - Extract helpers that are byte-identical across list-W files into `tests/common/mod.rs` (`mod common;`). Candidates: `mesh`, `close`/`assert_close`, `policy`, test hydraulic models, `no_flow_lower`, `input`.
  - Helpers that differ stay local, unless one parameterized helper keeps every call site's numbers identical.
  - `#![allow(dead_code)]` at the top of `common/mod.rs` is permitted. This is the standard integration-test idiom.
- **Do not touch:** `src/`, non-W tests, any assertion.
- **Anchors:** `grep -n '^fn ' tests/<W files>`; the scaffold at `tests/atmospheric.rs:10-127`.
- **Acceptance:**
  - The list-W command passes with unchanged per-file counts.
  - The total `wc -l` of list W plus `common` drops (report before/after).
  - A1–A3.

### 19.3.3 — Remove public solute workspace mutators

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/tests/solute_transaction.rs`.
- **Can create:** `planning/tasks/19.3.3.todo.md`.
- **Depends on:** Wave 2.
- **Spec:**
  - In `impl SoluteWorkspace` (`solute.rs:411-549`), change `trial_reaction_mass_mut` to `pub(crate)`; it is used at `:1289`.
  - The other mutators have no `src` callers: `trial_concentrations_mut`, `trial_kinetic_sorbed_mut`, `trial_boundary_mass_mut`, `set_trial_accepted_steps`. Make them `#[cfg(test)] pub(crate)`.
  - Move the three tests in `tests/solute_transaction.rs` that write through mutators (`~:81-116`) verbatim into a `#[cfg(test)] mod transaction_tests` in `solute.rs`, adapting only imports.
  - Read-only `trial_*` getters stay public.
- **Do not touch:** FV/BE equations, `lib.rs`, other files.
- **Anchors:** `impl SoluteWorkspace`; `grep -rn '_mut(\|set_trial' crates/hydrus-core/tests`.
- **Acceptance:**
  - `cargo test -p hydrus-core --lib transaction_tests` passes with 3 tests.
  - `cargo test -p hydrus-core --test solute_transaction --test solute_contract --test solute_transport --test solute_acceptance --test solute_outlet --test solute_reactions --test solute_species_boundaries --test solute_fixture_parity --test solute_verification_harness` passes. The integration count drops by exactly the 3 moved tests.
  - `grep -n 'pub fn .*_mut\|pub fn set_trial' crates/hydrus-core/src/solute.rs` returns nothing.
  - A1–A3.

---

## Wave 4 — Decoding reference and lower boundary

### 19.4.1 — serde decoding: dependency + transport group (reference implementation)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/Cargo.toml`, `Cargo.lock`, `crates/hydrus-core/src/solute_fortran_compat/adapter/{fields,transport}.rs`.
- **Can create:** `planning/tasks/19.4.1.todo.md`; `.development/19.4.1/`.
- **Depends on:** 19.3.1, 19.1.3 (ADR-0016).
- **Spec:**
  1. Add `serde = { version = "1", features = ["derive"] }`. `git diff Cargo.lock` adds no `[[package]]`.
  2. Snapshot before: `HYDRUS_ADAPTER_SNAPSHOT=$PWD/.development/19.4.1/before.txt cargo test -p hydrus-core --lib dump_parsed_fixtures -- --ignored`.
  3. In `fields.rs`, add generic `Deserialize` support for the `{units, value}` wrapper and the `Explicit` availability form. Existing helpers stay; 19.6.1 removes the orphans.
  4. In `transport.rs`, replace the bodies of the transport-group `maybe_*` mappers (including `maybe_endpoint_state`) with private `#[derive(Deserialize)] #[serde(deny_unknown_fields)]` DTOs converted to the existing `types.rs` structs. Keep each mapper's name and signature.
  5. The flow stays text → `reject_duplicate_keys` → `Value` → hash verify → per-group `serde_json::from_value`. `expect_units` and finiteness checks are preserved.
  6. Add `#[cfg(test)] mod tests` in `transport.rs` with rejection cases for explicit `null`, a wrong JSON type, a float where an integer is required, a missing `units`, and an unknown nested key. Each asserts the error names the field (D5) and is a rejection both before and after this change. To confirm "before", run them first against the old bodies.
  7. Snapshot after into `after.txt` and diff.
- **Do not touch:** `types.rs`, `adapter/mod.rs`, other group files, `json_guard.rs`, `tests.rs`, the canonical hash.
- **Anchors:** `fn maybe_endpoint_state`, `fn maybe_transport_switches`, `fn maybe_coordinates`, `fn maybe_orientation`, `fn unit_value`, `fn explicit`.
- **Acceptance:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat` passes; the count is the old count plus the new `transport::tests`.
  - The parsed-input sections of `before.txt`/`after.txt` are identical, and the rejection section lists the same rejected requests (texts may differ). Attach `diff` output.
  - `git diff Cargo.lock | grep -c '^+\[\[package\]\]'` = 0.
  - A1–A3.

### 19.4.2 — One lower-boundary value in `WaterStepInput`

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/{water_solver,lower_boundary,lib}.rs`, `crates/hydrus-core/tests/common/mod.rs`, list W.
- **Can create:** `planning/tasks/19.4.2.todo.md`.
- **Depends on:** 19.3.2.
- **Spec:**
  - Replace `lower_row` + `lower_boundary_mode` with one `lower: LowerBoundaryMode` whose variants carry exactly the data each mode reads; the fixed-head variant carries what `lower_row` held.
  - Collapse the private mirror (`struct LowerBoundaryPlan` `lower_boundary.rs:134`, `enum PreparedLowerBoundaryMode` `:139`) into direct preparation of the mode where that preserves arithmetic.
  - Derive the lower row inside the step.
  - Update `lib.rs` re-exports.
  - Migrate `tests/common` and every list-W literal and field reference.
  - Report the old→new mapping (D3).
- **Do not touch:** row arithmetic, `upper_row`/`upper_condition` (19.5.3), `atmospheric.rs`, `solute.rs`, non-W tests.
- **Anchors:** `pub struct WaterStepInput` (`water_solver.rs:57`), `fn validate` (`:727`), `fn assemble` lower row (`:433-452`), `LowerBoundaryMode` (`lower_boundary.rs:8`), `fn prepare` (`:94`), `fn prepare_step` (`:182`).
- **Reference:** the 19.2.2 atmospheric interface (an endpoint module builds its own row).
- **Acceptance:**
  - The list-W command passes with unchanged per-file counts.
  - `cargo test -p hydrus-core --test api_contract --test boundary_rows` passes.
  - `grep -n 'lower_row\|lower_boundary_mode' crates/hydrus-core/src/water_solver.rs` lists no `WaterStepInput` field.
  - A1–A3.

---

## Wave 5 — Decoding completion and upper boundary

### 19.5.1 — serde decoding: reactions + transport-property groups

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/adapter/{reactions,transport_properties}.rs`.
- **Can create:** `planning/tasks/19.5.1.todo.md`; `.development/19.5.1/`.
- **Depends on:** 19.4.1.
- **Spec:**
  - Apply 19.4.1 steps 2–7 to `maybe_chpar`, `chpar_records`, `maybe_species`, `maybe_moisture_dependency` and the group's other mappers. Keep their names and signatures.
  - Use `fields.rs` helpers without editing that file. If one is missing, stop and report (escalation rule 4).
- **Do not touch:** `fields.rs`, `mod.rs`, `tests.rs`, other group files, `types.rs`, `Cargo.*`.
- **Anchors:** the functions named above.
- **Reference:** `adapter/transport.rs` after 19.4.1.
- **Acceptance:** as in 19.4.1, minus the `Cargo.lock` check.

### 19.5.2 — serde decoding: nonequilibrium + biology groups

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/adapter/{nonequilibrium,biology}.rs`.
- **Can create:** `planning/tasks/19.5.2.todo.md`; `.development/19.5.2/`.
- **Depends on:** 19.4.1.
- **Spec:**
  - Apply 19.4.1 steps 2–7 to `maybe_dual_domain`, `dual_porosity_material`, `maybe_dual_porosity_exchange_state`, `maybe_biology_mode` and `maybe_biology_parameters`. Keep names and signatures.
  - The 19.1.1 kernel-validate delegation and the `iDualPor` cross-field checks stay intact.
  - `pinned_requests_cannot_supply_the_dual_porosity_exchange_state` passes unchanged.
- **Do not touch:** `fields.rs`, `mod.rs`, `tests.rs`, other group files, `types.rs`, `Cargo.*`.
- **Anchors:** the functions named above; `dual_porosity_exchange_state_*` tests in `adapter/tests.rs`.
- **Reference:** `adapter/transport.rs` after 19.4.1.
- **Acceptance:** as in 19.4.1, minus the `Cargo.lock` check.

### 19.5.3 — One upper-boundary value in `WaterStepInput`

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/{water_solver,atmospheric,error,lib}.rs`, `crates/hydrus-core/tests/common/mod.rs`, list W.
- **Can create:** `planning/tasks/19.5.3.todo.md`.
- **Depends on:** 19.4.2, 19.2.2.
- **Spec:**
  - Replace `upper_row` + `upper_condition` with one `upper: UpperBoundary { Head{..} | Flux{..} | Atmospheric { critical_head } }`.
  - The atmospheric interface from 19.2.2 reads `critical_head` directly. This removes the Dirichlet-as-`hCritA` extraction and the validation of ignored rows (`water_solver.rs:784-790`).
  - Remove `WaterStepError::UnsupportedAtmosphericInput` (`error.rs:110`, `:184`) if it has no remaining producer.
  - Delete exactly the two tests that become unrepresentable:
    - the `UnsupportedAtmosphericInput` assertion test ending at `tests/atmospheric.rs:~474`;
    - the one at `tests/water_acceptance.rs:~253`.

    Name them in the report.
  - Update `lib.rs`. Migrate `tests/common` and list W. Report old→new (D3).
- **Do not touch:** row and flux arithmetic, `lower_boundary.rs`, `solute.rs`, non-W tests.
- **Anchors:** `upper_condition`, `upper_row`, `UnsupportedAtmosphericInput`; the 19.2.2 interface in `atmospheric.rs`.
- **Reference:** 19.4.2's `lower` field.
- **Acceptance:**
  - The list-W command passes. Per-file counts are unchanged except −1 in `atmospheric` and −1 in `water_acceptance`.
  - `cargo test -p hydrus-core --lib atmospheric --test api_contract --test boundary_rows` passes.
  - `grep -n 'upper_row\|upper_condition' crates/hydrus-core/src/water_solver.rs` returns nothing.
  - A1–A3.

---

## Wave 6 — Decoder cleanup and water→solute coupling

### 19.6.1 — Remove orphaned adapter decode helpers

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/adapter/{fields,mod}.rs`.
- **Can create:** `planning/tasks/19.6.1.todo.md`.
- **Depends on:** 19.5.1, 19.5.2.
- **Spec:**
  - Delete `as_*`/`maybe_*` helpers in `fields.rs` with no remaining callers.
  - In `mod.rs`, inline group mappers into `map_fields` only where a mapper is now a one-line `serde_json::from_value` call.
  - No behavior change.
  - The `dump_parsed_fixtures` snapshot is identical before and after.
- **Do not touch:** group files, `json_guard.rs`, `tests.rs`.
- **Anchors:** `grep -rn 'fn as_\|fn maybe_' adapter/`, then grep each name for callers.
- **Acceptance:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat` passes with unchanged count.
  - The snapshot diff is empty.
  - The report lists the deleted helpers.
  - A1–A3.

### 19.6.2 — Water step publishes the accepted water snapshot

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/{water_solver,water_state,solute,lib}.rs` (`solute.rs` only for the `AcceptedWaterSnapshot` move and a `pub use`).
- **Can create:** `crates/hydrus-core/tests/water_solute_coupling.rs`, `planning/tasks/19.6.2.todo.md`.
- **Depends on:** 19.5.3, 19.3.3.
- **Spec:**
  - Move `AcceptedWaterSnapshot` (`solute.rs:637-645`) to `water_state.rs`. Re-export it from `lib.rs` and `pub use` it in `solute.rs`, so both public paths keep working.
  - Add a `face_fluxes` buffer of `node_count + 1` to `WaterWorkspace`, allocated at construction.
  - On acceptance, fill it from the accepted solve's face-averaging rule and boundary rows, with sign positive toward increasing coordinates.
  - Expose `WaterStepReport::accepted_water(&self) -> AcceptedWaterSnapshot<'_>`.
- **Do not touch:** solve arithmetic, FV/BE equations, any other `solute.rs` line.
- **Anchors:** `pub struct AcceptedWaterSnapshot`; `pub struct WaterStepReport` (`water_solver.rs:104`); `fn balance` (`:663`); `WaterWorkspace` (`water_state.rs:69-86`).
- **Reference:** the boundary-flux terms that `fn balance` already computes.
- **Acceptance:**
  - `cargo test -p hydrus-core --test water_solute_coupling` passes with ≥3 tests:
    - endpoint fluxes equal the balance's boundary fluxes bitwise;
    - Σ boundary flux·dt matches the storage change within the existing water-balance tolerance;
    - one water step followed by `advance_solute_step` on the published snapshot succeeds.
  - `--test allocation_contract` and the list-W command pass.
  - A1–A3.

---

## Wave 7 — Seam closure and Orchestration 18 handoff

### 19.7.1 — Closed `SoluteBackend` dispatch + contract tests

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/{solute,lib}.rs`.
- **Can create:** `crates/hydrus-core/tests/solute_backend_contract.rs`, `planning/tasks/19.7.1.todo.md`.
- **Depends on:** 19.6.2.
- **Spec:**
  - Add `#[non_exhaustive] pub enum SoluteBackend { FiniteVolumeBackwardEuler }` with `pub const ALL: &'static [SoluteBackend]`.
  - `SoluteConfig` stores a backend, set by `with_backend(self, SoluteBackend) -> Self` and defaulting to FV/BE. Existing constructors are unchanged. Add the accessor `backend()`.
  - `advance_solute_step` and `advance_solute_step_with_processes` dispatch with an exhaustive `match config.backend()`. The FV/BE arm calls the current code unchanged.
  - Contract tests loop over `SoluteBackend::ALL` and assert backend-agnostic invariants, without FV/BE numbers:
    - a rejected trial leaves `SoluteState` unchanged;
    - accept advances time to `accepted_water.accepted_time`;
    - malformed input is rejected before mutation;
    - species and node counts are preserved.
  - Re-export `SoluteBackend`.
- **Do not touch:** FV/BE equations, `solute_fortran_compat/`, existing tests.
- **Anchors:** `pub fn advance_solute_step` (`solute.rs:722`), `pub fn advance_solute_step_with_processes` (`:734`), `pub struct SoluteConfig` (`:117`), the `lib.rs` solute re-export block.
- **Reference:** the closed-enum `match` on `LowerBoundaryMode` in `water_solver.rs`.
- **Acceptance:**
  - `cargo test -p hydrus-core --test solute_backend_contract` passes with ≥4 tests.
  - All `tests/solute_*.rs` targets pass with the counts recorded after 19.3.3.
  - A1–A3.

### 19.7.2 — Amend Orchestration 18 for 19's changes

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `planning/EXECUTION_PLAN_18.md`.
- **Can create:** `planning/tasks/19.7.2.todo.md`.
- **Depends on:** all earlier 19 tasks. Input: the D3 rename list and task reports in `STATE_19.md`.
- **Spec:**
  - Append a dated "Amendment 2026-10-14 (Orchestration 19)" block to each affected task. Do not rewrite existing text.
  - **Every remaining compat kernel task** (18.7.2, 18.7.5, 18.7.7, 18.7.8, 18.8.1, 18.8.2, 18.9.1–18.9.5, 18.11.1): new kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1`. Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and are called from the adapter (ADR-0014). New request records are serde DTOs in the matching `adapter/<group>.rs`; `adapter.rs` no longer exists. `Cargo.*` stays frozen (ADR-0011).
  - **18.7.8:** its must-edit `types.rs`/`adapter.rs` becomes `types.rs` + `adapter/<group>.rs` (name the group from its mapped state).
  - **18.10.1:** the driver builds `EndpointPrimitiveState`/`DualPorPrimitiveState` from step state in source order, does not re-validate what kernel `validate` covers, and removes each `wired by 18.10.1` allowance it satisfies. It consumes `WaterStepReport::accepted_water` where it needs accepted water state.
  - **18.12.1:** add `SoluteBackend::FortranCompatible`, remove the `wired by 18.12.1` allowances, and run `tests/solute_backend_contract.rs` over both variants. Its `FortranCompatible`/`FiniteVolumeBackwardEuler` "selectors" are the enum variants.
  - **18.16.1:** "switch the default" means changing `SoluteConfig`'s default backend.
  - Apply every D3 rename from `STATE_19.md` wherever 18 text names an old public item.
- **Do not touch:** any other file, including `STATE_18.md`, which the orchestrator updates.
- **Anchors:** `^### 18\.` headings; `grep -n 'adapter.rs\|types.rs\|solute.rs\|advance_solute_step\|SoluteWorkspace' planning/EXECUTION_PLAN_18.md`.
- **Acceptance:**
  - `grep -c 'Amendment 2026-10-14 (Orchestration 19)' planning/EXECUTION_PLAN_18.md` ≥ 15.
  - Every D3 rename's old name either no longer appears outside amendment blocks, or appears beside its new name (the report lists the grep).
  - `git diff --check` is clean.
  - `git diff --stat` shows only `EXECUTION_PLAN_18.md` changed.

## Escalation ladder (hard limits)

1. A micro-plan is off-spec twice → the orchestrator writes the micro-TODO and hands it over as fixed input.
2. Acceptance fails after one `FAILED VERIFICATION:` retry → stop the Sonnet worker and re-run the task on Opus 5.5. There is never a third Sonnet attempt.
3. A report is missing or malformed → one feedback retry, then re-run on Opus 5.5.
4. Out-of-scope edits are needed → that is a decomposition bug. Amend this plan's spec with a dated note; never widen permissions ad hoc.
5. An A1 violation (a numeric expectation changed) is an automatic rejection, never resolved by updating the expected value.

## Exit

After wave 7:
- The orchestrator records the final suite results in `STATE_19.md`.
- In `STATE_18.md`, it sets Orchestration 18 to resumable with the D1 pending list and the 19.7.2 amendments.
- It closes Orchestration 19.

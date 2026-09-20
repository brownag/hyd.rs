# Orchestration 3 — Phase 1 hydraulic foundation

Status: **approved by the user on 2026-09-19; ready for wave execution.**
Master sequence authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 1.
Entry evidence: [Phase 0 reference evidence](master/PHASE_0_REFERENCE_EVIDENCE.md).
Execution contract: [wave-orchestration](/home/andrew/workspace/aigb-skills/skills/wave-orchestration/SKILL.md).

## Goal and scope

Implement and validate the first Rust hydraulic path: single-domain `iModel=0` van Genuchten retention, conductivity, capacity/derivative behavior, parameter and evaluation domains, and the source-style generated lookup table used for that analytic family. The result is a filesystem-free `hydrus-core` library with source-anchored pointwise and property tests. It establishes neither a full solver nor a numerical parity claim: source-derived checks are contract evidence until canonical fixture comparison and tolerance calibration occur later.

In scope: a minimal Cargo workspace; `hydrus-core` public hydraulic types and typed errors; validated `iModel=0` parameters; analytic theta, conductivity, capacity, and inverse-head evaluation; generated logarithmic analytic tables with explicitly documented interpolation/endpoints; targeted curve and table tests; and a Phase 1 evidence handoff that recommends the next hydraulic scope.

Out of scope: mesh/stencil, timestep, solver, boundary, root, atmospheric, balance, I/O, CLI, hosts, fixtures/parity, benchmarks, allocation claims, all other analytic hydraulic families, hysteresis, dual porosity, and general input-table (`iModel=nTabMod`) support. General input tables are deliberately not silently represented by the generated table cache in this phase.

### Verbatim constraints

> “Plan only the next incomplete phase in the sequence below.”

> “Do not widen the first water acceptance to include solute, heat, hysteresis, dual porosity, or optional physics.”

> “The default recommendation is source-compatible mixed precision first, with strict typed failures for nonconvergence.”

The first two are from the master plan. The third is retained as a future precision policy: this phase records source conversion points but does not claim a calibrated compatibility mode.

## Reconnaissance

- Phase 0 is complete in `STATE_2.md`; its GNU Fortran/Linux smoke run is provenance only, not a hydraulic fixture or numerical baseline.
- No `Cargo.toml`, Rust source, Rust test suite, project convention file, or prior Rust formatting/lint configuration exists. The bootstrap task establishes the workspace and its focused test commands.
- `MATERIAL.FOR` is 923 lines. `FK`, `FC`, `FQ`, and `FH` begin at lines 16, 201, 279, and 350; the `iModel=0` path derives `m=1-1/n`, returns saturated values for nonnegative heads, floors selected dry values, and converts returned values with `sngl`.
- `INPUT.FOR:536-625` maps model selectors, rejects selector 8 in the legacy program, and normalizes generated-table endpoints. `INPUT.FOR:762-843` generates a log-spaced analytic table. `WATFLOW.FOR:451-545` selects direct values or linearly interpolates the generated table and contains separate general-table extrapolations.
- `NUMERICAL_DESIGN.md#hydraulics-and-tables` requires explicit monotonicity, endpoint, interpolation, units, finite-domain checks, and no silent selector fallback. `VALIDATION_STRATEGY.md#component-and-property-tests` names pointwise curves, derivatives, saturation/dry limits, selector rejection, and NaN/overflow boundaries.

## Decision log

| ID | Decision |
|---|---|
| D01 | Phase 1 implements only `iModel=0`; every other selector, including the rejected legacy selector 8, returns a typed unsupported-model error. |
| D02 | Implement the source-generated analytic lookup table and its documented interpolation behavior, but defer user-supplied general tables (`iModel=nTabMod`) to a later explicit scope decision. |
| D03 | Use `f64` internal/public calculations for this isolated component; record legacy `sngl` conversion points in evidence and do not claim mixed-precision or Fortran parity until calibration. |
| D04 | Use source-anchored formula/vector and mathematical property tests, not Phase 0 smoke output, as Phase 1 evidence. |
| D05 | Preserve the established worker policy: gpt-5.6-luna at low effort; one failed verification retry, then gpt-5.6-terra at medium effort. |

## Wave table

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 1 | 3.1.1 | Pre-wave gate | Bootstrap the minimal hydraulic-core contract |
| 2 | 3.2.1 | 3.1.1 | Implement and test analytic van Genuchten behavior |
| 3 | 3.3.1 | 3.2.1 | Generate and evaluate the analytic lookup table |
| 4 | 3.4.1 | 3.3.1 | Review Phase 1 evidence and record the next-scope decision |

All tasks are serial: each task defines an interface or evidence consumed by the next. The orchestrator alone updates `STATE_3.md`, verifies task diffs/tests, and commits one conventional commit per accepted task.

### Pre-wave gate

Record these commands and results in `STATE_3.md` before Wave 1. They must pass except that no Cargo manifest is expected before task 3.1.1.

```sh
git status --short
sha256sum planning/H1D_Src.zip src_reference/*.FOR
python3 tools/run_fortran_reference.py --smoke --timeout-seconds 60
test ! -e Cargo.toml
```

Changed immutable hashes, a failed Phase 0 smoke command, or pre-existing Rust workspace blocks execution pending review. The smoke command remains a provenance regression check, never an expected hydraulic result.

## Shared constraints

- Preserve `src_reference/`, `planning/H1D_Src.zip`, Phase 0 artifacts, and generated `.development/` artifacts byte-for-byte/untracked as applicable.
- `hydrus-core` owns no filesystem, process-global, or foreign host state. It may not introduce solver, mesh, I/O, or optional-physics abstractions.
- Inputs and outputs are explicitly `f64`; source `sngl` locations must be documented, not imitated or erased by an unvalidated precision claim.
- A table cache is generated only from the implemented analytic model. It must reject invalid construction and never stand in for general input tables.
- Test tolerances must distinguish formula/property checks from unestablished Fortran parity tolerances.

## Task specifications

### 3.1.1 — Bootstrap the hydraulic-core contract

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** pre-wave gate.
- **Must edit:** none.
- **Can create:** `Cargo.toml`; `crates/hydrus-core/Cargo.toml`; `crates/hydrus-core/src/lib.rs`; `crates/hydrus-core/src/error.rs`; `crates/hydrus-core/src/hydraulics/mod.rs`; `crates/hydrus-core/src/hydraulics/types.rs`; `crates/hydrus-core/tests/api_contract.rs`; `planning/tasks/3.1.1.todo.md`.
- **Scratch only:** `.development/3.1.1/`.
- **Read:** `master/NUMERICAL_DESIGN.md#hydraulics-and-tables`; `master/VALIDATION_STRATEGY.md#component-and-property-tests`; task reports in `STATE_2.md`.
- **Reference implementation:** proposed `HydraulicModel` and error boundary in `master/NUMERICAL_DESIGN.md#hydraulics-and-tables`.
- **Anchors:** `rg -n 'trait HydraulicModel|HydraulicTables|non-finite|unsupported selectors' planning/master/NUMERICAL_DESIGN.md`; `rg -n 'iModel = 0|iModel.eq.8|nTabMod' src_reference/INPUT.FOR`.

Create the smallest library workspace that exposes a stable, filesystem-free hydraulic API, validated van Genuchten parameter type, evaluation result/derivative type, and a typed error taxonomy capable of distinguishing non-finite/invalid parameters, invalid evaluation domain, invalid table configuration, and unsupported selector. Declare model-selector handling centrally; no other model may fall back to model 0. Do not implement the formula or table yet.

**Acceptance:** `cargo test -p hydrus-core --test api_contract` passes, proving public types are constructible only through validation and nonzero/unsupported selector paths are typed failures. `cargo fmt --check` and `cargo clippy -p hydrus-core --all-targets -- -D warnings` pass. No source-reference, Fortran, solver, mesh, I/O, or host file appears in the diff.

### 3.2.1 — Implement analytic `iModel=0` van Genuchten curves

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 3.1.1.
- **Must edit:** `crates/hydrus-core/src/hydraulics/mod.rs`.
- **Can create:** `crates/hydrus-core/src/hydraulics/van_genuchten.rs`; `crates/hydrus-core/tests/hydraulic_curves.rs`; `planning/tasks/3.2.1.todo.md`.
- **Scratch only:** `.development/3.2.1/`.
- **Read:** task 3.1.1 report; `src_reference/MATERIAL.FOR`; `master/FEATURE_MATRIX.md`; `master/VALIDATION_STRATEGY.md#component-and-property-tests`.
- **Reference implementation:** `MATERIAL.FOR:FK`, `FC`, `FQ`, and `FH` for the `iModel=0` branch; the formulas are the port pattern, not the smoke output.
- **Anchors:** `rg -n '^      real function (FK|FC|FQ|FH)|iModel.eq.0|m=1.d0-1.d0/n|sngl\(' src_reference/MATERIAL.FOR`.

Implement only validated `iModel=0` theta, conductivity, capacity/derivative, and inverse-head behavior with explicit saturated and dry-domain policy. Translate formula semantics deliberately into `f64`; retain an evidence note in code comments/tests for source floors and `sngl` conversion points. Add independent hand-calculated point tests plus properties for finite outputs, bounds, saturation behavior, monotonicity where differentiable, inverse consistency away from endpoints, and invalid/non-finite domain failures. Do not add other model formulas or a Fortran-output comparator.

**Acceptance:** `cargo test -p hydrus-core --test hydraulic_curves` passes; it must include positive and saturated head cases, dry finite-head cases, invalid `n`, alpha, water-content ordering, NaN/infinite inputs, typed selector rejection, and a documented non-parity formula tolerance. `cargo fmt --check` and `cargo clippy -p hydrus-core --all-targets -- -D warnings` pass.

### 3.3.1 — Add source-style generated analytic hydraulic tables

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 3.2.1.
- **Must edit:** `crates/hydrus-core/src/hydraulics/mod.rs`.
- **Can create:** `crates/hydrus-core/src/hydraulics/table.rs`; `crates/hydrus-core/tests/hydraulic_tables.rs`; `planning/tasks/3.3.1.todo.md`.
- **Scratch only:** `.development/3.3.1/`.
- **Read:** task 3.2.1 report; `src_reference/INPUT.FOR`; `src_reference/WATFLOW.FOR`; `master/NUMERICAL_DESIGN.md#hydraulics-and-tables`.
- **Reference implementation:** `INPUT.FOR:GenMat` generated log-head grid and `WATFLOW.FOR:SetMat` analytic-table interpolation branch.
- **Anchors:** `rg -n 'hTab1|hTabN|alog10|ConTab|CapTab|TheTab|iModel.lt.nTabMod' src_reference/INPUT.FOR`; `rg -n 'if\(iModel.lt.nTabMod\)|dh=|Coni=|Capi=|Thei=' src_reference/WATFLOW.FOR`.

Build an immutable table cache from a validated `iModel=0` model and explicit negative finite head bounds/count. Generate source-style logarithmically spaced heads and values; define and test endpoint behavior and interpolation of theta, conductivity, and capacity. Reject unordered/nonnegative/non-finite endpoints, too-few points, and out-of-range evaluation according to a documented typed policy. Keep user-supplied tables and the source's general-table extrapolation out of scope and unmistakably unsupported.

**Acceptance:** `cargo test -p hydrus-core --test hydraulic_tables` passes, including exact grid endpoints, log-spacing, node reproduction, interval interpolation, applicable monotonicity and finite generated data, construction failures, evaluation endpoint/out-of-range behavior, and proof that general-table/unsupported selectors do not route through this cache. `cargo fmt --check` and `cargo clippy -p hydrus-core --all-targets -- -D warnings` pass.

### 3.4.1 — Record Phase 1 hydraulic evidence and scope decision

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 3.3.1.
- **Must edit:** none.
- **Can create:** `planning/master/PHASE_1_HYDRAULIC_EVIDENCE.md`; `planning/tasks/3.4.1.todo.md`.
- **Scratch only:** `.development/3.4.1/`.
- **Read:** task reports for 3.1.1–3.3.1; their committed source/tests; `HYDRUS1D_RUST_PORT_PLAN.md` Phase 1/2 rows; phase references named by the master plan.
- **Reference implementation:** Phase 0 handoff format in `master/PHASE_0_REFERENCE_EVIDENCE.md` and Phase 1 exit requirements in the master plan.
- **Anchors:** `rg -n 'iModel=0|unsupported|table|sngl|tolerance|parity' crates/hydrus-core/src crates/hydrus-core/tests`; `rg -n 'Hydraulic foundation|Mesh and source stencil' planning/HYDRUS1D_RUST_PORT_PLAN.md`.

Review the code and task evidence against this plan. Record the implemented API, formula/source anchors, pointwise/property evidence, invalid-selector behavior, generated-table boundary, precision limitation, and a concrete recommendation that Phase 2 proceed with mesh/stencil while alternate analytic families and general tables remain deferred. Do not call the source-derived tests a Fortran parity result.

**Acceptance:** run `cargo test -p hydrus-core`; `cargo fmt --check`; `cargo clippy -p hydrus-core --all-targets -- -D warnings`; and `python3 -c 'from pathlib import Path; s=Path("planning/master/PHASE_1_HYDRAULIC_EVIDENCE.md").read_text(); required=["## Hydraulic contract","## Source anchors and precision","## Pointwise and property evidence","## Invalid selectors and domains","## Generated-table boundary","## Exit checklist","## Phase 2 handoff"]; assert all(x in s for x in required); assert "not a Fortran parity result" in s; assert not any(line.rstrip()!=line for line in s.splitlines()); print("Phase 1 evidence structure clean")'`. All must pass.

## Execution policy

Workers write an at-most-12-step micro-plan and stop. The orchestrator verifies file scope, dependencies, acceptance mapping, and source claims before sending the literal `APPROVED — execute the plan as written.` message. The orchestrator runs the full Phase 1 suite serially only after Wave 4, integrates one conventional no-phase/wave commit per task, and records concise reports in `STATE_3.md`. A micro-plan off-spec twice is replaced by an orchestrator-written TODO. One failed targeted verification receives feedback; the repeated failure escalates to gpt-5.6-terra. An out-of-scope requirement respecifies this plan rather than widening a task.

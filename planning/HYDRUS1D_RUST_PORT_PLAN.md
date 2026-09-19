# HYDRUS-1D Rust port: master implementation plan

## Goal and scope

This document is the technical blueprint for a future Rust port of the selected scientific subset of the supplied HYDRUS source snapshot. It seeds later implementation wave plans and authorizes no implementation by itself. The compatibility reference is the hash-pinned supplied snapshot, described as 4.08 or a derivative; the exact release identity is unverified. The 4.08 manual is the primary companion, while 4.17 is comparative only. No numerical baseline or verified parity result exists.

The confirmed first acceptance is single-domain water flow with atmospheric boundaries and root water uptake. Solute and heat follow later; hysteresis follows first water acceptance; dual porosity remains required after simpler physics. Optional density, vapor/energy, snow/interception, colloid/bacteria, inverse, special chemistry, alternate hydraulic families, and host/format priorities remain decision gates.

## Decisions and open gates

Confirmed decisions are the hash-pinned snapshot, water-first milestone, later solute/heat, later hysteresis, and required later dual porosity. Technical recommendations are: preserve the observed `Reset` half-cell stencil while reconciling its finite-difference label with manual finite-element terminology; begin with `iModel=0` van Genuchten; use a small preallocated tridiagonal kernel; retain a mixed-precision compatibility mode before considering all-`f64`; and make nonconvergence strict by default.

Open gates are the verified release label, oracle portability/compiler, exact supported boundary combinations, alternate hydraulic and optional branch scope, precision modernization, linear algebra dependency, artifact terms, and host/serialization priorities. Each gate has an owner and milestone in [the roadmap and risk register](master/ROADMAP_AND_RISKS.md).

## Source audit

The snapshot has ten fixed-form Fortran files and 11,053 lines. `HYDRUS.FOR` owns process setup, initialization, the coupled time loop, output and teardown. `INPUT.FOR` reads configuration and generates tables; `MATERIAL.FOR` evaluates hydraulic curves; `WATFLOW.FOR` assembles and solves water flow; `TIME.FOR` drives events and meteorology; `SINK.FOR` handles roots; `SOLUTE.FOR`, `TEMPER.FOR`, and `OUTPUT.FOR` provide later transport, heat and accounting. `HYSTER.FOR` stores hysteresis history.

The lifecycle is initialization → input/profile/material/table setup → forcing and sink update → `WatFlow` trial/Picard solve (`SetMat`, `Reset`, `Shift`, `Gauss`) → optional velocity/heat/solute → balances/output → `Update` commit → event-aligned next step. Failed steps restore trial state and reduce `dt`; termination includes input/output errors, user/signal paths, and repeated nonconvergence. Main arrays and three COMMON blocks mean state is broader than COMMON alone; DATA/static lifetime assumptions require explicit ownership in the port.

The source audit records every routine, file unit, platform dependency, hash and provenance gap in [SOURCE_AUDIT.md](master/SOURCE_AUDIT.md) and [SOURCE_MANIFEST.json](master/SOURCE_MANIFEST.json). Known unmodified syntax blockers are missing `msflib.mod` and `TIME.FOR:551` scalar-logical semantics. A future isolated portability patch must record patch/compiler/executable hashes; it must not alter the originals or claim a working oracle.

## Architecture

```text
legacy files / host forcing
          |
   hydrus-compat adapters ---> validated SimulationConfig + Forcing
          |                                  |
          +--------------------------> hydrus-core
                                         |  Mesh + models
                                         |  accepted/trial state
                                         |  preallocated workspace
                                         v
                                  diagnostics + output events
```

The core owns no filesystem, process globals, or host handles. Immutable hydraulic models and configuration may be shared; each simulation owns mutable state and workspace. Independent simulations can run concurrently. The API design details boundaries and alternatives in [API_AND_IO.md](master/API_AND_IO.md), while [STATE_AND_CALLS.md](master/STATE_AND_CALLS.md) supplies the ownership evidence.

Proposed workspace/module layout (design only):

```text
hydrus-rs/
  Cargo.toml                 # future workspace manifest
  crates/
    hydrus-core/             # config, mesh, hydraulics, solver, state, diagnostics
    hydrus-compat/           # SELECTOR/ATMOSPH/PROFILE parsers and legacy writers
    hydrus-cli/               # future command-line runner; depends on compat/core
    hydrus-host/              # optional C/Python/R/Wasm boundary adapters
    hydrus-validation/       # future fixture/oracle harness, never core runtime state
```

`hydrus-core` must remain filesystem-free and depend only on numerical primitives selected after benchmarking. `hydrus-compat` owns fixed-format text and file-unit translation. `hydrus-cli`, host bindings and validation are optional downstream crates; no Cargo files are created by this planning orchestration.

## Rust interfaces

Proposed signatures are illustrative Markdown, not implementation:

```rust
struct SimulationConfig { tolerances: Tolerances, capabilities: Capabilities, time: TimePolicy }
struct Mesh1D { x: Vec<f64>, material: Vec<usize>, inclination: f64 }
trait HydraulicModel: Send + Sync { /* theta, conductivity, capacity, derivatives */ }
struct BoundaryConditions { upper: UpperBoundary, lower: LowerBoundary }
struct SimulationState { accepted: Fields, trial: Fields, time: f64, history: History }
struct MassBalanceTracker { input: f64, output: f64, storage: f64 }
struct SimulationWorkspace { matrix: Tridiagonal, hydraulic: HydraulicScratch, snapshot: Snapshot }
fn advance(state: &mut SimulationState, ws: &mut SimulationWorkspace,
           request: StepRequest) -> Result<StepDiagnostics, StepError>;
```

Validation converts one-based legacy arrays to checked zero-based slices once. Units, coordinate orientation and flux signs are explicit in constructors. `Send + Sync` applies to immutable models and independent simulation values; one mutable simulation is exclusively borrowed during `advance`. Cancellation and host errors are reported at the boundary, never through global callbacks.

## Numerical design

`WATFLOW.FOR:Reset` uses half-cell boundary widths, arithmetic face conductivity by default, optional geometric averaging, gravity/inclination and storage terms. `Gauss` applies water-specific boundary rows and tiny-pivot replacement; `SOLUTE.FOR:BanSol` is a distinct tridiagonal elimination. The port should preserve these caller-specific transformations and use a reusable Thomas kernel only after assembly. The finite-difference/finite-element terminology discrepancy is an M2 algebraic reconciliation gate.

Hydraulic values, derivatives, tables, interpolation endpoints and invalid domains are explicit model contracts. Start with `iModel=0` van Genuchten, then add alternate families and tables behind typed capability validation. A fixed-capacity workspace holds matrix diagonals, RHS, fields, residuals and all later module scratch. Allocation-free guarantees apply to hydraulic evaluation, assembly, Picard iterations, solve, retry and snapshot restore at fixed dimensions; setup/resizing and buffered output are outside that guarantee.

The step transaction is: align `dt` to forcing/print/final events; snapshot accepted coupled state; iterate hydraulics and boundary assembly; solve and test convergence; on success calculate balance, output, and commit all histories; on failure restore every coupled history and reduce `dt`, returning strict typed failure at `dtMin`. Future hysteresis reversal arrays, vapor/heat, solute/sorbed pools and dual-porosity mobile/immobile fields are part of the snapshot. Full numerical contracts and alternatives are in [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md).

## I/O and host adapters

`hydrus-compat` should parse `SELECTOR.IN`, `ATMOSPH.IN`, and `PROFILE.DAT`, and write canonical `NOD_INF.OUT`, `T_LEVEL.OUT`, and `BALANCE.OUT` records for comparison. The core receives validated in-memory forcing and emits typed diagnostics/events. JSON/YAML/TOML are proposed versioned schemas; GeoPackage and Arrow require dedicated adapters and are not automatic serde formats. Python, R, native Node, browser Wasm and C ABI boundaries remain provisional, with ownership/copying/error/thread rules documented in [API_AND_IO.md](master/API_AND_IO.md). No host or format is a release commitment.

## Validation strategy

M0 must create an isolated oracle harness with immutable inputs, timeout/completion checks, compiler/patch/fixture hashes, and no stale outputs. Missing input decks and expected results remain explicit gaps. The five requested scenarios are dry infiltration, capillary rise, cyclic atmosphere/root, nonequilibrium solute/degradation, and temperature-coupled heat. M4 requires dry-column, cyclic ponding/switching, and root-stress/compensation fixtures; M5/M6 add the later transport cases.

Compare fields with per-variable absolute/relative criteria, L-infinity/L2/RMS diagnostics and a defined zero-denominator mass-balance rule. `1e-6` is only a candidate requiring calibration. Add curve derivative/property tests, conservation, analytical limits, mesh refinement, event/retry, interleaved/threaded state, allocation counting, and comparable Criterion/oracle performance experiments only after their producer milestones. Details and proposed future commands are in [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md).

## Risk registry

| Risk | Owner | Mitigation / evidence | Gate and residual condition |
|---|---|---|---|
| Release identity and artifact terms are unverified | Provenance owner | Pin source/archive/PDF hashes; preserve notices; review each artifact | M0; no release parity or license claim until resolved |
| No numerical baseline; source syntax blockers | Oracle owner | Isolated MSFLIB/logical portability patch, manifest compiler/flags/outputs | M0; tolerance work waits for reviewed baseline |
| Manual FE terminology conflicts with `Reset` stencil | Numerical owner | Algebraic derivation and half-cell fixture; preserve observed source behavior | M2; unresolved equivalence remains user gate |
| Mixed REAL/REAL*8 and `sngl` drift | Numerical/validation owner | Compatibility precision mode, f64 comparison and calibrated tolerances | M1/M3; no universal 1e-6 claim |
| Boundary rows or tiny pivots fail | Solver owner | Caller-specific assembly, typed pivot diagnostics, singular fixtures | M3; strict failure behavior reviewed |
| Rejected retries mutate coupled state | State owner | Accepted/trial twin buffers and transactional snapshot of histories, balances and modes | M3/M4; event/retry tests pass |
| One-based indexing, signs, units and inclination are mishandled | Mesh/API owner | One-time checked conversion, dimensional/sign fixtures, explicit units | M2/M4; all supported boundaries covered |
| State exists outside COMMON or relies on compiler lifetime | State migration owner | Audit main arrays/DATA, explicit per-simulation ownership, no globals | M3; static-lifetime assumptions resolved |
| Inner-loop allocations reduce determinism/performance | Workspace owner | Fixed-capacity scratch, allocation counting, output outside solve | M3/M10; measured boundaries documented |
| Optional branch combinations silently fall back | Feature owner | Capability matrix, typed unsupported errors, independent fixtures | M4 onward; every selected combination declared |
| Platform APIs and file units are not portable | Host/I/O owner | Isolate MSFLIB, file-unit map, host cancellation/error contract | M0/M10; supported platforms explicitly gated |
| Required later physics are omitted by subset selection | Roadmap owner | Dedicated solute/heat/hysteresis/dual-porosity milestones and traceability | Roadmap review; no silent deferral |
| Host/format scope is overcommitted | API owner | Versioned schemas and provisional adapters; demand-based gates | M9/M10; user chooses release priorities |

No risk is closed by a structural document check; each gate requires the evidence named above.

## Implementation roadmap

| Phase | Exact checklist and dependencies |
|---|---|
| 0 Harness/oracle | [ ] preserve/hash inputs; [ ] isolate minimally patched oracle; [ ] record compiler/flags/patch/executable; [ ] run timeout/completion checks; [ ] review fixture provenance and baseline before parity claims. |
| 1 Hydraulic curves | [ ] implement/test `iModel=0` values, derivatives and domains; [ ] generate/interpolate tables; [ ] test saturation/dry limits and invalid selectors; [ ] gate alternate families and tables with independent evidence. |
| 2 Mesh/stencil | [ ] validate coordinates/layers/material indices; [ ] convert one-based inputs once; [ ] implement half-cell metrics, inclination and face averaging; [ ] compare algebra against manual/source fixture; [ ] review matrix coefficients. |
| 3 Water solver | [ ] assemble boundary-aware rows; [ ] implement reusable tridiagonal kernel and pivot errors; [ ] implement Picard diagnostics; [ ] align events and adaptive retries; [ ] snapshot/restore all coupled state; [ ] calculate balances; [ ] validate dry infiltration/capillary cases. |
| 4 Atmosphere/root | [ ] support `iModel=0`; [ ] support atmospheric rain/evaporation, ponding and switching; [ ] support fixed-head/flux upper modes; [ ] support lower fixed-head, prescribed-flux and free-drainage variants; [ ] test Feddes/S-shaped stress and compensation; [ ] pass dry-column, cyclic ponding/switching and root-stress fixtures; [ ] reject unsupported combinations. |
| 5 Solute/heat | [ ] add solute advection/dispersion/reaction; [ ] add sorption and nonequilibrium transfer; [ ] add heat/temperature coupling; [ ] add vapor/energy only behind its scope gate; [ ] validate each balance and coupling order; [ ] keep all outside first acceptance. |
| 6 I/O/polish | [ ] parse/write legacy formats; [ ] compare text layouts separately from science; [ ] add CLI and selected host adapters; [ ] count allocations; [ ] benchmark comparable work; [ ] document supported platforms and release evidence. |
| Later required | [ ] add hysteresis reversal/rollback after M4; [ ] add mobile/immobile dual porosity after validated single-domain physics; [ ] keep dual permeability rejected; [ ] gate density, snow/interception, colloid/bacteria, inverse and special chemistry individually. |

The detailed DAG, entry/exit evidence, dependency reasons, and exact checklist are in [ROADMAP_AND_RISKS.md](master/ROADMAP_AND_RISKS.md). The first acceptance excludes solute, heat, hysteresis, dual porosity, and optional chemistry even though those remain planned work.

## Requirement traceability

| ID | Master destination |
|---|---|
| R01 | Goal/scope, Source audit, M0 provenance |
| R02 | Source audit and lifecycle overview |
| R03 | Architecture, Rust interfaces, M3 ownership |
| R04 | Rust interfaces and numerical ownership |
| R05 | Numerical design, Phase 1 |
| R06 | Numerical design, Phase 2/3 |
| R07 | Numerical design transaction and Phase 3/4 |
| R08 | Numerical design memory contract and validation |
| R09 | I/O and host adapters, Phase 4/6 |
| R10 | I/O serialization gate |
| R11 | Host adapters and validation platform tests |
| R12 | Validation strategy and Phases 4/5 |
| R13 | Numerical precision and validation tolerance policy |
| R14 | Validation component/property tests |
| R15 | Decisions, roadmap first/later ordering |
| R16 | Risk registry and optional roadmap gates |
| R17 | Implementation roadmap and future waves |
| R18 | Architecture, Rust interfaces, roadmap |
| R19 | Goal/scope planning-only statement |
| R20 | Decisions/open gates and roadmap decision register |

Atomic prompt traceability:

| Prompt item | Destination / disposition |
|---|---|
| Discovery: source files and modules | Source audit inventory; Phase 0. |
| Discovery: initialization/input lifecycle | Source audit and lifecycle; Phases 0–3. |
| Discovery: lookup tables and time loop | Numerical design; Phases 1/3. |
| Discovery: output flushing and teardown | Source audit; I/O Phase 6. |
| Discovery: COMMON/global/static state domains | Architecture and numerical ownership; Phase 3. |
| Architecture: domain structs | Rust interfaces; Phase 3. |
| Architecture: re-entrant `Send + Sync` core | Rust interfaces/host contracts; Phase 6. |
| Architecture: tridiagonal versus crates | Numerical design alternatives; Phase 3 benchmark gate. |
| Architecture: zero inner-loop allocation | Numerical memory contract; Phases 3/6. |
| Architecture: legacy parsers/writers | I/O and host adapters; Phases 4/6. |
| Architecture: serde/schema/dynamic forcing | I/O and host adapters; later format gate. |
| Verification: five named scenarios | Validation strategy; Phases 4/5. |
| Verification: norms/tolerances/precision drift | Validation and numerical design; Phases 0/3. |
| Verification: derivative/property/conservation tests | Validation; Phases 1/4/5. |
| Roadmap Phases 0–6 | Implementation roadmap above. |
| Output 1: diagrams | Architecture diagram above. |
| Output 2: Cargo/module layout | Proposed workspace layout above. |
| Output 3: core signatures | Rust interfaces above. |
| Output 4: risk registry | Detailed table above. |
| Output 5: exact phase checklist | Checklist table above. |
| Global: evidence/recommendation/decision separation | Goal, decisions, evidence links and future gates; applies throughout. |

## Future wave plans

Future plans should use vertical slices: M0 provenance/oracle, M1 curves, M2 mesh/stencil, M3 water solver, M4 first water acceptance, M5 solute, M6 heat, M7 hysteresis, M8 dual porosity, M9 optional/host adapters, and M10 integration. Each plan must name immutable inputs, fixture producers, entry/exit evidence, allocation and rollback boundaries, and user decisions. A later plan may refine a milestone but may not silently broaden M4 or convert recommendations into release commitments.

The supporting reports remain the detailed evidence appendices: [SOURCE_AUDIT.md](master/SOURCE_AUDIT.md), [STATE_AND_CALLS.md](master/STATE_AND_CALLS.md), [FEATURE_MATRIX.md](master/FEATURE_MATRIX.md), [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md), [API_AND_IO.md](master/API_AND_IO.md), [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md), and [ROADMAP_AND_RISKS.md](master/ROADMAP_AND_RISKS.md).

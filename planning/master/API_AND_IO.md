# API, legacy I/O, and host boundaries

This is a proposed design based on the supplied source snapshot and prior audits. It does not commit release scope, crate names, host priorities, or numerical behavior beyond the confirmed first water/atmosphere/root milestone.

## Architecture

```text
legacy files / JSON DTO / host bindings
              | validate + convert units/indexes
       hydrus-compat / hydrus-schema / adapters
              v
 filesystem-free hydrus-core
  config + mesh + models + state + workspace + stepping
              | reports / owned copies / typed errors
       CLI, Python, R, Node, Wasm, C ABI
```

The core owns no paths, streams, foreign objects, callbacks, clocks, or global mutable state. One `Simulation` has one mutable stepping owner; immutable validated configuration may be shared. Independent simulations are the proposed first parallelism unit.

## Crates and modules

| Proposed package | Responsibility | Status |
| --- | --- | --- |
| `hydrus-core` | units, mesh, hydraulic models, boundaries, state, workspace, stepping, diagnostics, cancellation, typed errors | proposed core boundary |
| `hydrus-compat` | `Selector.in`, `Profile.dat`, `Atmosph.in` and legacy output adapters; fixed-format conversion | proposed; source layout confirms inputs/outputs |
| `hydrus-schema` | versioned DTO validation and JSON first adapter | proposed; format priority is a user gate |
| `hydrus-cli` | filesystem, project directories, logging and command exit policy | proposed |
| `hydrus-python`, `hydrus-r`, `hydrus-wasm`, `hydrus-node`, `hydrus-ffi` | optional host adapters | provisional; no release commitment |

## Core signatures

Illustrative Markdown signatures only:

```rust
pub struct SimulationConfig { pub units: UnitSystem, pub tolerances: Tolerances,
    pub physics: PhysicsFlags, pub time: TimePolicy }
pub struct Mesh1D { pub z: Vec<f64>, pub material: Vec<u32> }
pub enum HydraulicModel { VanGenuchten, BrooksCorey, Tabulated { id: String } }
pub struct BoundaryConditions { pub top: TopBoundary, pub bottom: BottomBoundary }
pub struct SimulationState { pub time: f64, pub head: Vec<f64>, pub water: Vec<f64> }
pub struct MassBalanceTracker { pub input: f64, pub output: f64, pub storage: f64 }
pub struct Simulation { /* owns State + Workspace + immutable validated config */ }
pub struct Workspace { /* preallocated trial/coupled arrays */ }
pub enum SimulationError { InvalidConfig, NonConverged, Cancelled, Io(String), Unsupported(String) }
impl Simulation {
  pub fn new(config: ValidatedConfig, mesh: Mesh1D) -> Result<Self, SimulationError>;
  pub fn step(&mut self, forcing: &Forcing) -> Result<StepReport, SimulationError>;
  pub fn advance_to(&mut self, t_stop: f64, forcing: &ForcingSchedule)
      -> Result<Vec<StepReport>, SimulationError>;
  pub fn snapshot(&self) -> Snapshot; // accepted state only
}
```

`Send + Sync` is appropriate for immutable configuration, mesh, model parameters, and detached reports when their contained types qualify. Mutable `Simulation` is `&mut self` and must not be stepped concurrently; wrappers may add a mutex but that is host policy. A checkpoint is versioned, unit-tagged, and contains accepted state/history/config hashes, never a partially converged trial.

## Lifecycle and forcing

`validate` checks dimensions, finite values, supported feature combinations, unit declarations, monotonic coordinates and boundary compatibility before allocation. `step` may perform adaptive internal trials but returns only an accepted report. `advance_to` repeatedly steps to an event-aligned target and rejects backward time. Forcing schedules carry explicit units and interpolation (`Hold`, `Linear`, or event-defined); discontinuities become solver events, with no interpolation across a declared switch. Cancellation is checked between iterations, before retries, and at accepted-step boundaries; no foreign callback runs inside a Picard loop.

Diagnostics include accepted time, internal iterations, retries, active boundary modes, convergence residuals, balances, warnings, and termination reason. Trial mutations and accounting are discarded on rejection according to the rollback contract in `STATE_AND_CALLS.md`.

## Legacy I/O and CLI

`BasInf` reads selector flags, units, tolerances, boundary modes and hydraulic settings (`INPUT.FOR:3-93`); `NodInf` reads profile nodes, heads, materials, observations and concentrations (`INPUT.FOR:208-373`); `TmIn`, `ChemIn`, and `TempIn` read time, chemistry, and heat options (`INPUT.FOR:866`, `:1289`, `:1205`). The adapter owns one-based-to-zero-based conversion, unit conversion, fixed names, comments, and source-compatible validation errors.

`TLInf` writes time-level diagnostics, `NodOut` writes node profiles, and `SubReg` writes layer balances (`OUTPUT.FOR:3`, `:325`, `:720`). Legacy writers preserve field ordering and formatting as a compatibility mode. Numeric comparison uses parsed physical fields and tolerances; byte comparison is a separate layout test because timestamps, widths, ordering, and rounding are not scientific parity. CLI owns directories, file units, exit codes and logs; core receives readers/writers as completed data or explicit sink traits only at the adapter boundary.

## Serialization

Use a versioned DTO with schema version, units, coordinate convention, array ordering, feature flags, and explicit optional fields. JSON is the proposed first interchange format because it is broadly available, but this remains a user decision gate. YAML and TOML are proposed human-authored alternatives after schema parity tests. Serde can support these syntax formats; it does not define scientific validation.

GeoPackage is a geospatial container requiring a domain adapter and CRS/geometry policy. Arrow is a columnar interchange requiring an explicit schema and ownership/copy policy. Neither is an automatic Serde format. Their inclusion, schemas, and release timing remain provisional user decisions.

## Host contracts

| Host | Boundary proposal | Ownership/thread/error rule |
| --- | --- | --- |
| Python | PyO3 wrapper around owned Rust values | validate/convert before releasing host GIL; return copied arrays and structured errors; wheel matrix is provisional |
| R | extendr adapter | host API and NA conversion on R thread; owned Rust computation; interrupt/lifetime rules require tests |
| Browser Wasm | wasm-bindgen coarse calls and typed-array copies | no ordinary filesystem/thread assumption; cancellation via bounded calls; zero-copy is deferred |
| Native Node | napi-rs addon or Wasm package | choose one after packaging trial; do not conflate Node native with browser Wasm |
| C ABI | optional opaque handle with fixed-width status codes and buffers | catch expected errors, define destroy functions and allocator ownership; never expose Rust layout types |

All wrappers validate at entry, copy or borrow only under documented lifetimes, translate typed errors, and prevent concurrent mutable stepping. Host cancellation must map to a core cancellation token checked at bounded solver boundaries. Independent instances may run in separate threads after host/runtime thread rules are verified.

## Platform recommendations

Keep `hydrus-core` `no_std`-adjacent where practical but do not promise `no_std`; test a pure core on native targets first. Browser Wasm requires a separate filesystem-free dependency graph. Candidate initial development target is a supported desktop 64-bit Rust toolchain; exact OS/architecture, Python/R versions, Node ABI, browser matrix, and C ABI stability are all provisional user gates. Primary adapter references recorded in research include PyO3 parallelism, extendr thread-safety guidance, wasm-bindgen typed arrays, napi-rs, Rust FFI guidance, Rust Wasm platform documentation, and Serde overview (accessed 2026-09-19 via `HYDRUS_RESEARCH.md §7`).

## Alternatives

1. A single `hydrus-core` plus adapter crates is the recommended starting shape because it keeps the numerical boundary deep and limits dependency leakage.
2. One monolithic CLI is simpler for legacy parity but prevents clean host reuse and Wasm packaging; it remains an alternative for a reference-executable wrapper only.
3. Direct host callbacks during stepping could reduce copies but would make timing, cancellation, reentrancy, and thread guarantees fragile; defer unless a measured integration need appears.
4. Zero-copy foreign views may improve throughput, but copied batched arrays provide clearer lifetimes and are the provisional binding baseline.

## Evidence

- `STATE_AND_CALLS.md` establishes mutable state, rollback, mixed precision, unit/sign/index hazards, and the filesystem boundary.
- `INPUT.FOR:BasInf/NodInf/TmIn/ChemIn/TempIn` and `OUTPUT.FOR:TLInf/NodOut/SubReg` anchor legacy contracts, file formats, and error-directed I/O.
- `HYDRUS_RESEARCH.md §7` supplies the adapter architecture, host capability references, ownership/cancellation recommendations, and format cautions.
- `SOURCE_AUDIT.md` records file units, Windows path coupling, and the unverified source lineage. `FEATURE_MATRIX.md` records first milestone scope and later optional feature gates.
- Proposed APIs and priorities are not executed, benchmarked, or user-approved release commitments.

## Open decisions

1. User gate: choose initial shipped hosts among Python, R, browser Wasm, native Node, and C ABI; retain others as future adapters.
2. User gate: select JSON-only first interchange or add YAML/TOML at the first release; defer GeoPackage and Arrow until schemas and use cases are approved.
3. Fortran-reference portability owner: confirm canonical legacy filenames, file-unit mapping, path handling, and source-compatible error/exit behavior.
4. Numerical/API design owner: reconcile exact `advance_to` report shape, forcing interpolation, retry visibility, and checkpoint contents with numerical contracts.
5. User gate: approve supported OS/architecture and binding packaging matrix after a minimal core and adapter prototype exists.

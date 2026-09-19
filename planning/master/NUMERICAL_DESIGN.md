# Numerical core and memory contracts

This is a proposed design grounded in the supplied snapshot. It specifies future Rust behavior; it reports source observations and does not claim an implementation, numerical baseline, or verified 4.08 release.

## Discretization

`WATFLOW.FOR:Reset` labels its assembly “Finite differences.” For adjacent nodes it uses half-cell widths (`dx=(x(2)-x(1))/2`), arithmetic conductivity averages by default, optional geometric averages, gravity/inclination factors, and storage terms from capacity and `dt` (`:197-228`). Interior rows therefore have lower/diagonal/upper coefficients assembled from neighboring conductivities and capacity; `Shift` then modifies boundary rows for flux, head, seepage and atmospheric switching. The manual Chapters 2, 7 and 8 describe a finite-element formulation. This terminology is unresolved until the stencil is algebraically reconciled against the manual; preserving the observed stencil is the compatibility recommendation.

Proposed spatial contract:

```text
Mesh1D: x[0..n), material[0..n), layer metrics, cos_inclination
edge i: dx = x[i+1]-x[i], face metric = dx/2 at boundaries
face K: arithmetic mean by default; geometric mean only when source flag requests it
row i: lower[i] * h[i-1] + diag[i] * h[i] + upper[i] * h[i+1] = rhs[i]
boundary rows: transformed by boundary code before elimination
```

Require strictly increasing coordinates, finite metrics, valid material indices and a documented orientation/sign convention. Preserve one-based source indexing at the adapter boundary, then validate and convert once to zero-based slices.

## Hydraulics and tables

`MATERIAL.FOR` supplies `FK`, `FC`, `FQ`, `FH`, `FS`, `FKQ`, `FKS` and numerical helpers. `INPUT.FOR:GenMat` generates `hTab`, `ConTab`, `CapTab`, `TheTab`, saturation values and conductivity limits; `WATFLOW.FOR:SetMat` selects analytic or table values at each trial head and applies temperature/density/vapor options. The design must expose value and derivative contracts for each selected hydraulic family, with domain checks at saturation, dry limits, non-finite parameters and unsupported selectors. Tables need explicit monotonicity, endpoint, interpolation and unit metadata; no selector may silently fall back.

Proposed interface (Markdown only):

```rust
trait HydraulicModel: Send + Sync {
    fn theta(&self, h: f64, material: usize) -> Result<f64, HydraulicError>;
    fn conductivity(&self, h: f64, material: usize) -> Result<f64, HydraulicError>;
    fn capacity(&self, h: f64, material: usize) -> Result<f64, HydraulicError>;
    fn derivatives(&self, h: f64, material: usize) -> Result<HydraulicDerivatives, HydraulicError>;
}
struct HydraulicTables { head: Vec<f64>, theta: Vec<f64>, conductivity: Vec<f64>, capacity: Vec<f64> }
```

The first acceptance should calibrate a compatible analytic family and later add alternate/tabulated families. Source `REAL`, `REAL*8`, and `sngl` behavior remains evidence for a mixed-precision compatibility mode; an all-`f64` mode is a modernization requiring separate validation.

## Linear solver

`Gauss` (`WATFLOW.FOR:308-353`) is a boundary-aware forward/back substitution over water rows. It handles fixed and non-fixed bottom/top codes, clamps small pivots to `rMin`, and converts solved values with `sngl`. `SOLUTE.FOR:BanSol` is a separate generic tridiagonal elimination (`:945-957`) without the water boundary transformations. Keep assembly and boundary policy in the caller and share only a tested kernel.

Recommended kernel: a reusable in-place Thomas solver over preallocated lower/diagonal/upper/RHS buffers, with explicit finite-value and pivot checks and a typed `SingularPivot` error. Preserve caller-specific boundary row construction and the source's minimum-pivot policy in compatibility mode. A `faer` or `ndarray` dependency is justified only if later coupled systems become wider, batched, or demonstrably faster; future benchmark experiments must compare identical assembled systems, allocations and failure cases. No implementation experiment is run here.

## Time and nonlinear solve

The proposed step lifecycle is:

```text
align dt to min(print_time, forcing_time, final_time, process limits)
snapshot accepted water/coupled histories and accounting
repeat Picard: evaluate hydraulics -> assemble -> boundary Shift -> solve -> convergence test
if converged: compute balances, emit accepted output, Update all histories, advance time
if failed: restore snapshot, reduce dt, retry; at dtMin return typed nonconvergence
```

`TmCont` aligns `dt` to events and uses single-precision conversions in the legacy path (`TIME.FOR:3-30`). `WatFlow` performs `SetMat`, `Reset`, `Shift`, `Gauss`, convergence and retry work (`WATFLOW.FOR:3-168`). Proposed defaults are strict errors with diagnostics; a legacy continuation policy remains an explicit option. Convergence must report iteration count, max head/theta residuals, timestep, pivot minima and boundary mode. Output and irreversible counters occur only after acceptance.

## Balances and coupling

Water storage, boundary fluxes, sinks and drainage must produce a mass-balance record with absolute and normalized residuals and a defined zero-denominator rule. `OUTPUT.FOR:SubReg` consumes water, chemistry, heat, vapor, density and dual-porosity arrays; it is evidence that accounting is coupled, not a separate postprocessor. Proposed accepted-step ordering is forcing and sink update, water solve, velocity/transport coefficients, optional heat/solute modules, balance calculation, output, then history commit. Future hysteresis reversal arrays, immobile-domain water, vapor, temperature, concentrations and sorbed pools belong in the same transaction snapshot.

## Memory and precision

`SimulationWorkspace` owns fixed-capacity vectors for coefficients, RHS, hydraulic values, trial fields, residuals, and module scratch. Allocate or resize only during validated setup. The zero-allocation guarantee applies to hydraulic evaluation, matrix assembly, Picard iterations, tridiagonal elimination, convergence diagnostics, accepted/rejected retries, and snapshot restore for fixed dimensions. It excludes initial setup/resizing and explicitly permits buffered output serialization and error construction outside the inner solve; output must never trigger hidden solver allocations.

Use separate accepted and trial state, with copy-on-write or preallocated twin buffers. A rejected step restores all coupled histories, boundary modes, hysteresis state, balances and event cursors. Future dual porosity needs mobile/immobile arrays and exchange workspaces; it cannot alias single-domain water state. Thread-safe immutable models and independently owned mutable simulations can be `Send + Sync`; one simulation's workspace is exclusively mutable during stepping.

Compatibility mode should retain source-like `f32` field conversion points and `f64` accumulators where observed; modernization may use `f64` throughout only with calibrated tolerances and independent checks. Do not claim either mode passes without a future oracle baseline.

## Interfaces

Proposed numerical boundary:

```rust
struct StepRequest { target_time: f64, forcing: ForcingView<'_> }
struct StepDiagnostics { accepted: bool, iterations: u32, dt: f64, balance: BalanceResidual, pivots: PivotStats }
fn advance(state: &mut SimulationState, ws: &mut SimulationWorkspace,
           req: StepRequest) -> Result<StepDiagnostics, StepError>;
```

The core accepts validated arrays and forcing views, owns no files or global state, and exposes diagnostics sufficient to reproduce retry decisions. I/O, adapters and host cancellation belong outside this numerical contract.

## Alternatives

| Decision | Recommendation | Alternative / gate |
|---|---|---|
| Stencil | Preserve `Reset` half-cell stencil and document FE terminology reconciliation | Derive an algebraically equivalent FE form only after manual/source fixture agreement |
| Solver | Small reusable Thomas kernel with caller boundary assembly | `faer`/`ndarray` if wider systems or benchmark evidence justify dependency |
| Precision | Mixed compatibility mode first; optional f64 modernization | f64-only after oracle and independent convergence/refinement evidence |
| Failure | Strict typed nonconvergence at minimum step | Legacy continuation behind explicit policy and diagnostics |
| Memory | Preallocated twin accepted/trial workspaces | COW snapshots only if allocation counts prove equivalent |

## Evidence

- `WATFLOW.FOR:173-303` anchors source stencil, half-cell metrics, averaging, gravity, storage and coupled corrections.
- `WATFLOW.FOR:308-353` anchors water boundary rows, tiny-pivot substitution, back substitution and `sngl` conversion.
- `WATFLOW.FOR:359-430` anchors boundary switching; `:433-571` anchors material evaluation; `:1079-1125` anchors commit/update.
- `INPUT.FOR:762-803` anchors hydraulic table generation; `SOLUTE.FOR:945-957` anchors the distinct transport tridiagonal kernel.
- `TIME.FOR:3-30` anchors event alignment and timestep policy; `OUTPUT.FOR:325-356` anchors coupled balance inputs.
- Dependency reports document mixed precision, rollback state, future hysteresis/dual-porosity requirements, and the absence of a numerical baseline.

## Open decisions

1. Confirm the algebraic compatibility interpretation of the source finite-difference stencil versus manual finite-element language; owner: numerical validation, gate: independent stencil fixture.
2. Choose compatibility precision and exact f64 modernization policy; owner: roadmap/user, gate: oracle baseline and drift study.
3. Set minimum-pivot and nonconvergence behavior for strict and legacy modes; owner: solver design, gate: singular/near-singular fixtures.
4. Decide whether a third-party linear algebra crate is justified after comparable benchmark evidence; owner: performance task, gate: future implementation prototype.
5. Confirm the complete coupled snapshot boundary for later hysteresis, heat, solute, vapor and dual porosity; owner: transport/state design, gate: feature-specific rollback tests.

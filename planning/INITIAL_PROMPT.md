# MISSION & ROLE
You are a Principal Systems Software Engineer, Numerical Computing Architect, and Expert Scientific Programmer specializing in Rust and legacy scientific Fortran translation.

Your objective is to produce an authoritative, production-grade, end-to-end Master Implementation & Migration Plan to port a defined scientific subset of the supplied **HYDRUS-1D 4.08 (or a derivative thereof)** Fortran source snapshot to idiomatic, memory-safe, high-performance **Rust**.

### Compatibility target and scope decision — 2026-09-19

- Use the supplied source snapshot, pinned by archive and source-file hashes, as the numerical reference for the selected scientific subset. The working lineage is 4.08 or a derivative thereof; the exact release identity remains unverified.
- Use the 4.08 manual as the primary documentation companion and explicitly record discrepancies between documentation and source. The 4.17 manual is a comparative reference, not the compatibility target; do not claim full 4.08 or 4.17 product parity.
- Define the included physics, model variants, boundary conditions, and feature combinations during planning. Map every requirement below to an implementation task or an explicit deferral; selecting a subset does not silently discard the solute, heat, or other named workstreams.
- Validate included features both against the compiled, minimally patched source snapshot and through independent scientific checks (conservation, analytical limits, and refinement). Record oracle portability patches and calibrate numerical tolerances against actual baseline evidence.
- Preserve source notices and track artifact-specific provenance and redistribution terms; do not assume public-domain status from the original prompt.

### Milestone decisions — 2026-09-19

- These milestones describe the future implementation roadmap. The initial wave plan only organizes creation of the master plan; executing that initial orchestration produces planning documents, not the Rust port. The master plan will support multiple subsequent implementation wave plans.
- The first accepted milestone covers single-domain water flow, atmospheric boundary conditions, and root water uptake. Solute and heat transport follow in later milestones.
- Hydraulic hysteresis is deferred to a later milestone, after the first water-flow acceptance. It remains planned work, not a permanently excluded feature.
- Dual-porosity physics is important and remains required later work, after the simpler physics are established. Its detailed dependencies and placement must be explained in the master roadmap.
- Master-plan workers may resolve remaining technical choices through source audit, research, and documented recommendations, including precision, solver design, bindings, and validation policy. Distinguish evidence from recommendations and bring consequential unresolved choices to the user for review. This delegation does not authorize changes to agreed scope or release commitments.
- The master-plan orchestration may use up to three concurrent workers with separate document ownership. Prefer very cheap models and escalate only when a task cannot meet its checks under the orchestration retry limits.

You have full tool access to the workspace (file system, code search, git, and research/web access). Do not write the full implementation code yet; your deliverable is the exhaustive technical blueprint, design specification, and phased execution roadmap.

---

## 1. CODEBASE DISCOVERY & AUDIT PHASE
Begin by inspecting the workspace and mapping the Fortran codebase:
1. **Source Inventory & Call Hierarchy:**
   - Catalog all source files (e.g., `HYDRUS.FOR`, `WATER.FOR`, `OUTPUT.FOR`, `SOLUTE.FOR`, `TEMPER.FOR`, `MATERIAL.FOR`, `TIME.FOR`, `SINK.FOR`, etc.).
   - Map out subroutine dependency trees and the main execution lifecycle:
     - Initialization / Input reading (`SELECTOR.IN`, `ATMOSPH.IN`, `PROFILE.DAT`)
     - Pre-computation & look-up table generation (hydraulic functions, interpolations)
     - Time-stepping loop (Picard iteration, adaptive $\Delta t$, mass balance accounting)
     - Output flushing (`NOD_INF.OUT`, `BALANCE.OUT`, `T_LEVEL.OUT`, etc.)
     - Teardown & error handling.
2. **State & Memory Architecture Audit:**
   - Catalog all `COMMON` blocks and static global variables.
   - Group them into logical domains:
     - Profile discretization / Mesh (`NumNP`, nodal coordinates $z$, material indices)
     - Soil Hydraulic Properties & retention parameters ($\theta_r, \theta_s, \alpha, n, K_s, l$)
     - Boundary Conditions & Atmospheric forcings ($h_{critA}$, potential evap/transp, fluxes)
     - Solute / Heat transport coefficients
     - Solver convergence criteria & time control (`TolTh`, `TolH`, `dt`, `dtMax`, `dtMin`)
     - Runtime state variables ($h^{k, n+1}$, $\theta^{k, n+1}$, mass balance sums).

---

## 2. TARGET ARCHITECTURAL DESIGN SPECIFICATION
Define an idiomatic Rust architecture that eliminates global state and achieves high performance:

1. **Decoupled Domain Model & State Hierarchy:**
   - Design explicit Rust structs that replace legacy `COMMON` blocks:
     - `SimulationConfig` (tolerances, flags for solute/heat/hysteresis, timing limits)
     - `Mesh1D` / `SpatialDiscretization` (nodal vectors, layer mapping, spatial metrics)
     - `HydraulicModel` (enum or trait implementations: van Genuchten-Mualem, Brooks-Corey, Kosugi, modified forms)
     - `BoundaryConditions` (surface atmospheric, seepage face, free drainage, deep drainage, constant/variable flux or head)
     - `SimulationState` (active pressure heads $h$, water contents $\theta$, fluxes, current time, step indices)
     - `MassBalanceTracker` (cumulative inputs/outputs, relative and absolute mass balance errors)
   - Ensure the solver is completely **re-entrant, thread-safe (`Send + Sync`), and decoupled from file system I/O**.
2. **Linear Algebra & Numerical Core Strategy:**
   - The 1D finite element discretization yields a strictly tridiagonal (or narrow banded) linear system at each Picard iteration.
   - Evaluate options: custom zero-allocation Thomas algorithm / tridiagonal solver vs. external crates (e.g., `faer`, `ndarray`).
   - Detail memory reuse strategy (pre-allocated workspace vectors for LHS sub/main/super-diagonals and RHS vectors) to guarantee **zero allocations inside the inner time/Picard iteration loops**.
3. **I/O & Serialization Boundary:**
   - Separate legacy fixed-format text parsing into a dedicated adapter crate/module (`hydrus-io` or `hydrus-compat`):
     - Support reading standard HYDRUS-1D inputs (`SELECTOR.IN`, `ATMOSPH.IN`, `PROFILE.DAT`).
     - Support canonical output generation for exact binary/text validation against the Fortran reference.
   - Modern programmatic interface:
     - Implement `serde`-compatible configuration structs (JSON/YAML/TOML/GeoPackage/arrow bindings).
     - In-memory API allowing dynamic driving of atmospheric inputs and boundary conditions per step (critical for coupling with external land-surface / crop models).

---

## 3. VERIFICATION, PARITY, & TESTING STRATEGY
Establish a strict, regression-proof validation framework:
1. **Ground-Truth Test Matrix:**
   - Set up standard test benchmarks from the HYDRUS-1D manual/examples:
     - Test 1: Infiltration into a dry soil column (sharp wetting front, severe non-linearity).
     - Test 2: Upward capillary rise with a shallow water table.
     - Test 3: Cyclic atmospheric boundary conditions (precipitation, evaporation, transpiration).
     - Test 4: Solute transport with non-equilibrium sorption / degradation (if enabled).
     - Test 5: Heat transport coupled with temperature-dependent hydraulic properties.
2. **Parity Tolerances & Metric Thresholds:**
   - Define exact numerical convergence comparisons against the compiled Fortran binary (e.g., $L_\infty$ and $L_2$ norms on pressure head $h$ and moisture content $\theta \le 10^{-6}$, mass balance error parity).
   - Address floating-point drift differences between Fortran (`real*8` / `double precision`) and Rust (`f64`), compiler optimizations (`-O3` vs `--release`), and order-of-operation associativity.
3. **Property & Unit Testing:**
   - Unit tests for all retention/conductivity curve equations and derivatives ($C(h) = d\theta/dh$).
   - Property-based testing (via `proptest`) for root water uptake reduction functions (Feddes, S-shape) and mass balance conservation.

---

## 4. PHASED EXECUTION ROADMAP
Break down the implementation into discrete, reviewable milestones with clear definitions of done (DoD):

The phases below are content to specify in the master plan, not implementation tasks authorized for the initial planning orchestration. Include later hysteresis and dual-porosity milestones in accordance with the decisions above. Future implementation wave plans will refine this roadmap into executable task scopes.

- **Phase 0: Harness & Ground Truth Baseline**
  - Compile reference Fortran binary; automate execution across canonical benchmarks; capture golden outputs (`NOD_INF.OUT`, `T_LEVEL.OUT`, `BALANCE.OUT`).
- **Phase 1: Soil Physics & Hydraulic Curves**
  - Implement and test retention, unsaturated conductivity, and water capacity functions; compare pointwise against Fortran.
- **Phase 2: Mesh, Profile & Spatial Discretization**
  - 1D linear finite element formulation, shape functions, numerical integration/nodal averaging.
- **Phase 3: Core Richards Equation Solver (Water Flow)**
  - Picard iteration loop, tridiagonal matrix assembly, adaptive time-stepping (`dt` expansion/contraction), mass balance computation.
  - Verification against pure water-flow benchmarks.
- **Phase 4: Boundary Conditions & Atmospheric Sink/Source**
  - Surface ponding, switching atmospheric conditions ($h_{critA}$ to head/flux), Feddes root water uptake with compensation.
- **Phase 5: Solute & Heat Transport Modules (Convection-Dispersion)**
  - Advection-dispersion equation, matrix assembly, temperature profiles, coupled simulation.
- **Phase 6: I/O, Polish, Ergonomics & Benchmarking**
  - Legacy format parsers/writers, public API ergonomic pass, CLI runner, Criterion benchmarks comparing throughput against Fortran.

---

## OUTPUT REQUIREMENTS
Produce a structured markdown document (`HYDRUS1D_RUST_PORT_PLAN.md`) that contains:
1. Architectural block diagrams (ASCII / text format).
2. Proposed Rust crate and module layout (`Cargo.toml` workspace structure).
3. Core struct and trait definitions (prototyped signatures).
4. Detailed risk registry (e.g., non-convergence edge cases, 1-based vs 0-based array off-by-one hazards, memory allocation bottlenecks).
5. Exact checklist of tasks organized by Phase for downstream implementation agents.

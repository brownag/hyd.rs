# Current Rust solute formulation

## Scope and evidence labels

This report describes the current implementation in `crates/hydrus-core/src/solute.rs` and relates it to the retained Orchestration 16 comparisons. It is not a prescription for a new target or a claim of parity, physical correctness, or continuum accuracy.

- **Source fact** means behavior directly visible in the current Rust source or its recorded task contract.
- **Measured fact** means a value or comparison retained in an Orchestration 16 normalized report.
- **Inference** means an explanation supported, but not uniquely established, by source and measurements.
- **Unknown** means the retained material does not establish the answer.

## Implemented formulation

### State, coordinates, and water coupling

**Source facts.** `advance_solute_step` delegates to `advance_solute_step_with_processes`; the latter validates inputs, starts a trial, solves species in configured order, updates step balances, and accepts the trial only after success. Errors reject the trial. The workspace and accepted state hold dissolved concentrations, one kinetic sorbed pool per species/node, time, and accounting fields. The mesh coordinates define increasing spatial order.

Water is caller-owned. Each step borrows accepted water contents and a face-flux snapshot of length `node_count + 1`, ordered lower boundary, interior faces, upper boundary, with flux positive toward increasing mesh coordinates. It does not reconstruct or mutate water values. The accepted water time must match solute state time plus the step within `8 * f64::EPSILON * max(abs(start), abs(end))`; this is explicitly a floating-point roundoff check, not a physical tolerance. The current and accepted water contents are separately supplied for storage/accounting.

### Transport and storage rows

**Source facts.** For each interior face, the implementation averages adjacent water contents, computes a dispersive face conductance `theta_face * D_face / distance`, then combines it with signed water flux using upwind selection: `q.max(0) + conductance` on the left coefficient and `q.min(0) - conductance` on the right. The resulting conservative face contributions are added with opposite signs to the neighboring nodal rows. This is a finite-volume face-flux operator; it is not a finite-element element matrix.

Nodal control-volume width is half the adjacent interval at either endpoint and half the distance between neighboring nodes for an interior node. In the transport-only path, storage adds `theta_accepted * width / dt` to the diagonal and multiplies the old concentration for the RHS. The tridiagonal system is solved implicitly with the shared solver. The reaction/process path adds local mass and reaction terms to these same face-flux rows. Thus the transport solve uses an implicit (backward-Euler) step, not an explicit transport update.

### Reactions and sorption

**Source facts.** Configured species order defines a sequential first-order chain. Each species' decay rate removes mass from that species; the next species receives a source based on the previous species' solved mass, while terminal decay leaves the configured chain. The assembled local mass is dissolved plus equilibrium-sorbed plus kinetic-sorbed mass, with bulk density converting sorbed storage to the concentration-volume basis.

Equilibrium sorption supports none, linear `S_eq = Kd*C`, and nonlinear `S_eq = f*K*C^n/(1 + nu*C^n)`. Linear and kinetic terms are represented through local mass, its concentration derivative, and an affine intercept. Nonlinear equilibrium sorption is iterated using a caller-supplied maximum iteration count and concentration-change tolerance; the tolerance is a solver setting, not a calibrated physical threshold. A nonfinite derivative (including exponent below one at zero concentration) is rejected. The one kinetic pool is advanced implicitly from its prior value and current solved concentration. A second source kinetic pool is not represented.

### Time and endpoint treatment

**Source facts.** The transport-only storage term uses accepted end-of-interval water content; the process/accounting path also records pre-step storage using the supplied start-of-interval water contents. Process rows use `gamma = 1/dt + decay_rate`, the local mass derivative on the diagonal, and the old mass/source/affine-intercept terms on the RHS. These are implicit endpoint-time rows; the implementation does not use the Fortran `epsi` temporal weighting in the retained comparisons.

Endpoint selections are independent per species. A concentration endpoint replaces the endpoint row with an identity row and prescribed concentration. A total-solute-flux endpoint adds the configured flux to the RHS, with that flux defined positive inward at either end. `AdvectiveOutflow` adjusts the endpoint diagonal using the signed accepted water flux, imposes zero dispersive endpoint flux, and accounts for solution-dependent advective export. Inward water flow at a configured outlet is rejected before trial mutation; zero flow is allowed. Concentration-boundary exchange is inferred from the discrete storage/reaction balance after known flux contributions, so it is not an independent flux measurement.

### Precision and contract

**Source facts.** Solute numerical values and solver inputs are `f64`; time comparison explicitly uses `f64::EPSILON`. The contract requires nonempty caller-declared concentration and sorbed-mass unit labels, but performs no unit conversion and does not define one universal physical basis. Callers are responsible for mutually compatible values. The selected contract supports one water domain, the above process and boundary choices, and typed validation/rejection for malformed or unsupported selections. Heat/vapor and temperature dependence, density-dependent transport, bacteria, filtration, special chemistry, dual-porosity water, mobile-immobile transport, artificial dispersion, and moisture-dependent transport remain deferred.

**Measured precision context.** The Orchestration 16 Example 3 report notes Fortran printed values are rounded single-precision and records a Rust `dt=0.1 h` against Fortran's measured `dt=0.100000001 h`; the Robin report records initial states differing by at most `2.9911e-8` after coordinate reversal, consistent with Fortran single precision. Phase 5 reports a promoted-default-real Fortran run with bounded precision sensitivity only on GNU Fortran 13.3.0/Linux x86_64. This does not broaden the platform claim or alter the fact that Rust source uses `f64`.

## Orchestration 16 family observations

| Family | Measured observation | Rust source correspondence | Assessment and limit |
|---|---|---|---|
| Diffusion | The retained first-step trace reports all 63 interior Fortran off-diagonals at half the Rust magnitude; it also records different endpoint rows and a maximum first-step state difference of `0.0003047828`, concentrated near the outlet. | Rust's observed row is a conservative FV face-flux row with nodal control-volume storage. Rust keeps half-cell endpoint balances; its endpoint row is not replaced by a two-node zero-gradient relation. | **Inference:** source time/spatial weighting and endpoint closure are plausible contributors to the measured difference. The retained rows show formulation divergence, but do not decide which formulation is required or prove a Rust defect. |
| Scalar Robin | At matched physical coordinates, the report records different Rust and Fortran inlet, midpoint, and outlet rows. It identifies Rust face terms as diffusion `27.52` plus advection `0.43`; the midpoint RHS values are close while matrix coefficients differ. | Rust's interior face assembly selects advective direction by the sign of water flux and adds dispersion conductance. Its nodal FV endpoint balance remains in place. | **Inference:** upstream treatment, FV versus FE spatial support, time/storage assembly, and endpoint closure may contribute. The report explicitly lacks a weighting-only counterfactual; it cannot assign the difference to `wc` alone or isolate a time-weighting defect. |
| Reaction | For `reaction_equal`, the retained Rust S1 first-step value is `0.9937888199`; Fortran prints `0.993769467`. Rust's retained zero-reaction control agrees across the tested nodes/species, while active reaction diverges before chain-source accumulation. | Rust forms a nodal control-volume mass balance with `gamma = 1/dt + decay`, mass derivative on the diagonal, and sequential solved-mass sources for later species. | **Inference:** the nonzero-neighbor Fortran row is consistent with its element/consistent-mass assembly differing from Rust's nodal FV basis. **Unknown:** without full Fortran element coefficients and a mapped aqueous/sorbed reaction ledger, the active-reaction difference cannot be uniquely attributed to a rate or Rust error. |
| Example 3 | Matched-location traces show different assembled rows and inlet states; Fortran reports `epsi=0.5` time weighting and upstream weighting in FE rows, while Rust uses FV face-flux rows. The endpoint closures differ. Tiny Rust midpoint values are subnormal. | Rust uses implicit face-flux rows, its own nodal storage, and its selected endpoint modes; the retained row comparison shows the Rust outlet row differs from the Fortran zero-gradient row. | **Inference:** the observed divergence follows the different assembled formulations and outlet closures, but the available trace does not identify a unique causal term. The case is derived, not a recovered manual deck or independent analytical oracle. |

## Implementation history

**Source/history facts.** Task `10.3.1` specified one conservative single-domain dissolved-solute step driven by borrowed accepted-water contents and caller-supplied face fluxes, with selected concentration/total-flux endpoints, typed failures, and atomic accepted-state behavior. Commit `8a21927` (`feat: solve conservative solute transport`, 2026-09-24) introduced the transport implementation and its focused tests. Subsequent recorded changes extended the formulation: `e23fb6b` added reaction/sorption processes; `da4070e` added balance reporting; `6d67ae7` accounted for changing water content; `73712ef` shared transport assembly; `b8f6b4d` added per-species endpoint boundaries; and `80f9d81` added conservative advective-outflow boundaries. This is implementation history, not evidence that any one version matches the pinned source.

## Conclusions and unresolved evidence

- The current Rust source directly supports a single-domain, implicit, conservative nodal finite-volume face-flux formulation with upwinded advection, dispersion, selected endpoint modes, ordered reactions, equilibrium sorption, and one kinetic pool.
- Some differences are already explained by explicit Rust design choices: FV nodal control volumes rather than the FE rows shown in retained Fortran comparisons; implicit endpoint-time treatment rather than the reported Fortran `epsi=0.5` weighting; and Rust's half-cell flux balance rather than the sampled Fortran zero-gradient endpoint row. These explain structural differences, not their full quantitative effect.
- Upstream weighting and time/spatial assembly remain entangled in the retained comparisons. The retained evidence does not supply the counterfactuals or complete normalized coefficient ledgers needed to isolate their individual contributions.
- No observed family divergence is classified here as a bug. The evidence cannot select Rust's intended target, establish pinned Fortran as physical truth, establish parity tolerance, or establish physical/continuum accuracy. Independent analytical validation, calibrated tolerances, and convergence evidence remain unavailable or unmet in the Phase 5 record.

## Sources

- [`solute.rs`](../../crates/hydrus-core/src/solute.rs): `advance_solute_step_with_processes`, `assemble_and_solve_species_with_processes`, `assemble_flux_operator`, `apply_endpoint_boundary_rows`, `local_mass_terms`, `equilibrium_sorption_value`, and `control_volume_width`.
- [Task 10.3.1](../tasks/10.3.1.todo.md) and commit `8a21927`; subsequent history commits `e23fb6b`, `da4070e`, `6d67ae7`, `73712ef`, `b8f6b4d`, and `80f9d81`.
- [Phase 5 solute evidence](PHASE_5_SOLUTE_EVIDENCE.md), including delivered M5 contract and derived comparison/precision limitations.
- [Diffusion normalized trace](SOLUTE_DIFFUSION_NORMALIZED_TRACE.md), [Robin normalized rows](SOLUTE_ROBIN_NORMALIZED_ROWS.md), [reaction normalized equations](SOLUTE_REACTION_NORMALIZED_EQUATIONS.md), and [Example 3 normalized trace](SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md).

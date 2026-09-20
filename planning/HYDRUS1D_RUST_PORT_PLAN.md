# HYDRUS-1D Rust port: sequential master plan

## Purpose and use

This is the authoritative sequence for the Rust port. It is not an implementation plan itself. Build one subsequent plan at a time, beginning with Phase 0. A later phase is not planned in executable detail until the preceding phase has produced and reviewed its exit evidence.

The documents in `planning/master/` are reference documents for individual phases. They record audit evidence and design decisions; they do not create competing execution sequences. A subsequent plan should read this document first, then only the phase references named below.

The compatibility reference is the supplied, hash-pinned Fortran snapshot, described as HYDRUS-1D 4.08 or a derivative. Its exact release identity is unverified. The 4.08 manual is the primary documentation companion and the 4.17 manual is comparative only. No working oracle, input deck, golden output, numerical baseline, parity result, tolerance, or performance result exists yet.

## Rules for every subsequent plan

1. Plan only the next incomplete phase in the sequence below.
2. State its entry evidence, concrete outputs, checks, and exit evidence before implementation begins.
3. Use the named reference documents for that phase; do not reopen settled scope without evidence.
4. Record any changed technical recommendation or new user decision in the new plan and its state record.
5. Do not claim a numerical result, fixture, release compatibility, or performance result before its producer phase has created and reviewed the evidence.
6. Do not widen the first water acceptance to include solute, heat, hysteresis, dual porosity, or optional physics.

## Confirmed scope and decision gates

The first accepted product is single-domain water flow with atmospheric boundaries and root water uptake. Solute and heat come later. Hysteresis comes after the first water acceptance. Dual porosity remains required after the simpler physics is validated; it is distinct from the source snapshot's rejected dual-permeability selector.

Before the relevant phase starts, the user must decide or review these gates: the compatibility release label after provenance work; the initial supported host and legacy/schema format subset before adapter work; selected optional physics; and any all-`f64` modernization after a precision-drift study. The default recommendation is source-compatible mixed precision first, with strict typed failures for nonconvergence.

## Sequential phase plan

| Phase | The next plan must accomplish | Entry evidence | Exit evidence before planning the next phase | Reference documents |
|---|---|---|---|---|
| 0. Oracle and provenance | Create an isolated, minimally patched copy of the reference source; establish reproducible build/run provenance and fixture acquisition rules. | Pinned archive/source hashes; known `MSFLIB` and `TIME.FOR:551` blockers. | Patch, compiler, flags, executable, input/output and environment recorded; a reviewed run policy; no parity claim merely because the harness builds. | [SOURCE_AUDIT.md](master/SOURCE_AUDIT.md), [STATE_AND_CALLS.md](master/STATE_AND_CALLS.md), [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md) |
| 1. Hydraulic foundation | Implement and validate the first hydraulic path: `iModel=0` van Genuchten values, derivatives, domains and table behavior. | Phase 0 provenance process and curve source anchors. | Pointwise/property evidence, invalid-selector behavior, and a decision on the next hydraulic family or table scope. | [FEATURE_MATRIX.md](master/FEATURE_MATRIX.md), [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md), [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md) |
| 2. Mesh and source stencil | Define checked mesh/profile inputs, one-based conversion, layer metrics, inclination, face averaging and the observed `Reset` stencil. | Phase 1 hydraulic contract. | A source-algebra versus manual terminology reconciliation and a reviewed mesh/stencil fixture. | [STATE_AND_CALLS.md](master/STATE_AND_CALLS.md), [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md) |
| 3. Water solver transaction | Build the Picard solver, boundary-aware tridiagonal assembly, event-aligned stepping, diagnostics, balances, and accepted/trial state handling. | Phase 2 mesh/stencil evidence. | Pivot, retry, rollback, allocation, and water-balance checks. Complete rollback is a port contract to prove; it is not assumed legacy behavior. | [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md), [STATE_AND_CALLS.md](master/STATE_AND_CALLS.md), [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md) |
| 4. First water acceptance | Add the minimum atmospheric/root product and accept it only against the specified water fixtures. | Phase 3 accepted water solver and Phase 0 provenance process. | `iModel=0`; atmospheric precipitation/evaporation, ponding and switching; supported fixed-head/flux and lower fixed-head/prescribed-flux/free-drainage boundaries; Feddes or S-shaped stress with compensation; dry-column, capillary-rise, and cyclic atmosphere/root fixture evidence; explicit unsupported-combination errors. | [FEATURE_MATRIX.md](master/FEATURE_MATRIX.md), [API_AND_IO.md](master/API_AND_IO.md), [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md) |
| 5. Solute transport | Add advection/dispersion, species/reactions, sorption, and nonequilibrium transfer after water behavior is accepted. | Phase 4 water state and a solute fixture/provenance plan. | Solute balance and concentration evidence for each selected reaction/transfer combination. | [FEATURE_MATRIX.md](master/FEATURE_MATRIX.md), [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md), [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md) |
| 6. Heat and temperature coupling | Add heat transport and temperature-dependent behavior; add vapor/energy only if its scope gate is accepted. | Phase 4 water behavior and heat fixture/provenance plan. | Heat/coupling balance evidence and a decision on vapor/energy scope. | [FEATURE_MATRIX.md](master/FEATURE_MATRIX.md), [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md), [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md) |
| 7. Hysteresis | Add reversal history and drainage/imbibition behavior after the first water product. | Phase 4 transactional state contract and hysteresis fixture plan. | Reversal-history rollback evidence and drainage/imbibition validation. | [FEATURE_MATRIX.md](master/FEATURE_MATRIX.md), [STATE_AND_CALLS.md](master/STATE_AND_CALLS.md), [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md) |
| 8. Dual porosity | Add mobile/immobile water and exchange after validated single-domain behavior. | Phase 4 state contract plus an exchange/balance fixture plan. | Mobile/immobile balance, rollback, and transfer evidence. Never substitute rejected dual permeability. | [FEATURE_MATRIX.md](master/FEATURE_MATRIX.md), [STATE_AND_CALLS.md](master/STATE_AND_CALLS.md), [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md) |
| 9. Selected optional capabilities and adapters | Add only user-selected optional physics, serialization formats, or host bindings. | Explicit user scope decision and a capability-specific fixture or adapter contract. | The selected capability's scientific/host evidence, typed unsupported behavior, and supported-platform record. | [FEATURE_MATRIX.md](master/FEATURE_MATRIX.md), [API_AND_IO.md](master/API_AND_IO.md), [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md) |
| 10. Integration and release evidence | Complete legacy I/O, CLI, selected adapters, allocation/performance work, and release documentation for the supported set. | Evidence from each selected preceding phase. | Reproducible validation report, separate text-layout and scientific comparison results, allocation measurements, comparable benchmark results, and supported-platform statement. | [API_AND_IO.md](master/API_AND_IO.md), [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md), [ROADMAP_AND_RISKS.md](master/ROADMAP_AND_RISKS.md) |

## Phase-specific reference map

The references above are deliberately narrow. Use this guide only when a phase needs a question answered outside its own scope.

| Question | Reference document |
|---|---|
| What is actually in the supplied source, and what is its provenance? | [SOURCE_AUDIT.md](master/SOURCE_AUDIT.md) and [SOURCE_MANIFEST.json](master/SOURCE_MANIFEST.json) |
| Where does state live, what calls what, and what must be transactional? | [STATE_AND_CALLS.md](master/STATE_AND_CALLS.md) |
| Which physics is observed, required later, optional, rejected, or uncertain? | [FEATURE_MATRIX.md](master/FEATURE_MATRIX.md) |
| What are the numerical, precision, memory, solver, and rollback contracts? | [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md) |
| What is the core/adapter boundary, legacy I/O approach, and host policy? | [API_AND_IO.md](master/API_AND_IO.md) |
| What evidence is needed for oracle, fixtures, tolerances, allocations, and performance? | [VALIDATION_STRATEGY.md](master/VALIDATION_STRATEGY.md) |
| What risks, alternatives, and deferred decisions remain? | [ROADMAP_AND_RISKS.md](master/ROADMAP_AND_RISKS.md) |

## Cross-phase architecture constraints

The Rust core is filesystem-free and owns no process-global or foreign host state. Immutable configuration and hydraulic models may be shared; every simulation owns its mutable state and workspace. The core boundary includes `SimulationConfig`, `Mesh1D`, `HydraulicModel`, `BoundaryConditions`, `SimulationState`, `MassBalanceTracker`, diagnostics, typed errors, and a preallocated workspace. Legacy fixed-format input/output, CLI behavior, and host bindings stay outside the core.

At fixed dimensions, the allocation-free boundary covers hydraulic evaluation, assembly, Picard iterations, linear solve, accepted/rejected retries, and snapshot restore. Setup, resize, buffered output, and error construction are outside that boundary. The source's trial iteration and accepted-state promotion are observed; the full accepted/trial snapshot and rollback transaction is a port design obligation to be validated in Phase 3.

## Reference scenario sequence

These fixtures are future artifacts, not existing examples or baselines. Their producer plan must record input provenance, expected observables, hashes, and review evidence.

| Scenario | Producer phase | Required observables |
|---|---|---|
| Dry-column infiltration | 4 | Pressure head, water content, surface flux, cumulative water balance, refinement response |
| Capillary rise / shallow water table | 4 | Head/water-content profile, water-table position, steady flux, balance |
| Cyclic atmosphere/root | 4 | Event times, ponding/switching, evaporation/transpiration/root sink, head/water content, balance |
| Nonequilibrium solute/degradation | 5 | Concentration profiles, mobile/immobile transfer, reaction mass, solute balance |
| Temperature-coupled heat | 6 | Temperature/vapor fields, heat flux, water/energy coupling, balance |

## Requirement traceability

| Requirement | Sequence location |
|---|---|
| R01 | Phase 0 source identity and provenance |
| R02 | Phase 0 source/call evidence |
| R03 | Phases 0 and 3 state ownership evidence |
| R04 | Phases 1–4 core architecture and interfaces |
| R05 | Phase 1 hydraulic foundation |
| R06 | Phases 2–3 mesh, stencil, and solver |
| R07 | Phases 3–4 stepping, retries, and boundaries |
| R08 | Phases 3 and 10 allocation boundary and measurements |
| R09 | Phases 4 and 10 legacy I/O |
| R10 | Phase 9 schema and serialization choices |
| R11 | Phases 9–10 host boundary and platform choices |
| R12 | Phases 0 and 4–6 scenario evidence |
| R13 | Phases 0, 1, and 3 precision and tolerance calibration |
| R14 | Phases 1 and 4–6 property, derivative, and conservation tests |
| R15 | Phases 4–8 water-first ordering and later required physics |
| R16 | Phase 9 optional capability decisions |
| R17 | The full phase sequence and Phase 10 integration |
| R18 | This master plus the API, numerical, and risk references |
| R19 | Planning-only scope of this master and all subsequent plans |
| R20 | Explicit decision gates in this master and future plans |

The five requested blueprint outputs are retained as references: architecture/module layout and signatures in [API_AND_IO.md](master/API_AND_IO.md), numerical contracts in [NUMERICAL_DESIGN.md](master/NUMERICAL_DESIGN.md), risks in [ROADMAP_AND_RISKS.md](master/ROADMAP_AND_RISKS.md), and the actionable checklist in the sequential phase table above.

## Immediate next step

Create the Phase 0 plan only. Its scope is the isolated oracle and provenance process. It must not begin the Rust solver, Cargo workspace, fixtures, or parity claims. Once Phase 0 exit evidence is reviewed, return to this document and create the Phase 1 plan.

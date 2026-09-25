# Implementation roadmap, risks, and traceability

This roadmap organizes future implementation waves. It authorizes no Rust implementation, oracle repair, release commitment, or future `EXECUTION_PLAN_N` file. Evidence comes from the six supporting documents and the supplied prompt; recommendations remain provisional unless marked as confirmed.

## Milestone DAG

```text
M0 provenance + oracle bootstrap ──┐
                                  v
M1 hydraulics -> M2 mesh/stencil -> M3 water solver -> M4 water acceptance
                                  |
                                  ├──> M5 solute -> M5-V scientific comparison -> M6 heat
                                  ├──> M7 hysteresis
                                  └──> M8 dual porosity
                                               \       |       /
                                                v      v      v
                                         M9 optional branches/hosts
                                                    |
                                                    v
                                         M10 integration/release gate
```

Phase 0's pinned-source staging/build process and noncanonical smoke are complete; the initial syntax blockers are resolved in the isolated compatibility patch. That smoke proves runner/provenance mechanics only. No canonical scientific solute fixture, expected solute outputs, or calibrated tolerance has been reviewed. M1 depends on source/feature audit and produces curve contracts. M2 depends on M1 and reconciles `Reset`'s observed stencil with manual terminology. M3 depends on M2 and provides Picard, tridiagonal solve, retry, and balance behavior. M4 depends on M3 plus minimal legacy forcing/input adapters and is the first accepted milestone. M5 implementation and contract evidence exist, but its scientific-comparison gate (M5-V) remains open and is the next planning priority; plan it before Phase 6 heat/temperature work and before relying on M10 for scientific validation. M6 requires accepted water state and heat coupling contracts; M7 requires accepted water plus history rollback. M8 requires validated single-domain state and explicit mobile/immobile exchange. M9 is gated independently per optional capability. M10 depends on the selected supported set and reviewed validation evidence.

## Phase checklists

| Milestone / original phase mapping | Entry evidence | Checklist and exit evidence |
|---|---|---|
| M0 / Phase 0 | Source hashes, audit, portability owner | Isolated oracle copy; patch/hash/toolchain manifest; reviewed noncanonical smoke and run policy. Case-specific scientific fixture/output provenance is tracked at each physics gate, including M5-V; smoke is not a numerical pass. |
| M1 / Phase 1 | `MATERIAL` and `GenMat` anchors | Implement candidate hydraulic families, derivatives, domains and tables; pointwise/unit checks; explicit unsupported selectors; curve evidence. |
| M2 / Phase 2 | Mesh and stencil reconciliation decision | Mesh/layer/inclination metrics, one-to-zero index conversion, half-cell boundary algebra, layered/refinement fixtures; reviewed matrix coefficients. |
| M3 / Phase 3 | M1/M2 contracts and solver diagnostics | Picard loop, reusable tridiagonal kernel, pivot policy, adaptive `dt`, rollback, water balances; dry infiltration/capillary checks and oracle comparison when available. |
| M4 / Phase 4 | M3 accepted water behavior | Atmospheric event alignment; minimum hydraulic model `iModel=0` van Genuchten; upper atmospheric precipitation/evaporation with ponding and fixed-head/flux switching; lower fixed-head, prescribed-flux and free-drainage variants, with seepage/water-table combinations gated if fixture evidence is absent; Feddes/S-shaped root stress and compensation behavior. Required fixtures are dry-column infiltration, cyclic atmospheric forcing with ponding/switching, and a root-stress/compensation profile with mass balance. First acceptance sign-off requires these fixtures and explicit unsupported-combination errors. Excludes solute, heat, hysteresis, dual porosity and optional chemistry. |
| M5 / Phase 5a | M4 water state and I/O contracts | Solute advection/dispersion, species/reaction, sorption, transfer and mass balance; nonequilibrium fixture; explicit combination validation. |
| M5-V / Phase 5a follow-up | O11 M5 implementation/contract evidence and [scientific-comparison research](SCIENTIFIC_COMPARISON_RESEARCH.md) | A source-matched, provenance-reviewed solute fixture and paired reference results for the selected M5 subset; independent analytical/balance checks where applicable; field-specific tolerances reviewed. This evidence gate remains open. |
| M6 / Phase 5b | M4, reviewed M5-V comparison handoff, and thermal evidence | Heat, temperature dependence, vapor/energy only where separately accepted; coupled fixture and energy/balance evidence. |
| M7 / later water | M4 rollback and history contract | Lenhard hysteresis paths, reversal state, drainage/imbibition fixtures; no implication that first acceptance includes it. |
| M8 / later coupled physics | M3/M4 state plus exchange design | Mobile/immobile water and transfer, dual-porosity balance and rollback; keep distinct from Durner bimodal retention and rejected dual permeability. |
| M9 / optional | Capability-specific schema and fixture | Drainage, density, snow/interception, colloid/bacteria, inverse and special chemistry each require a decision, independent fixture, and typed unsupported combinations. |
| M10 / Phase 6 | Selected capabilities and validation reports | Legacy I/O/CLI polish, host adapters, packaging, allocation/performance measurements and reproducible release evidence. |

## Future wave-plan boundaries

Each future wave should contain one vertical slice with immutable source inputs, a named producer for fixtures, and a reversible acceptance gate. M0 is a provenance/oracle wave; M1–M2 are hydraulic/spatial slices; M3 is the solver slice; M4 is the first water product slice; M5 implementation is followed by M5-V scientific comparison before Phase 6 planning; M6–M8 are separately reviewable physics slices; M9 is split by optional capability; M10 is integration and host packaging. No wave may silently expand M4 scope or treat a documented branch as supported behavior.

## Risk registry

| Risk | Evidence / consequence | Mitigation and owner | Gate |
|---|---|---|---|
| Unverified 4.08/derivative identity | Source header and manuals disagree on release identity | Pin hashes; use “4.08 or derivative”; provenance owner | Before parity claims |
| No scientific solute baseline | Phase 0 smoke is noncanonical and water-only; no reviewed solute deck/output or calibrated tolerance | Dedicated M5-V fixture recovery, reference run, paired observations, and tolerance review; owner: next orchestration | Before Phase 6 planning and M10 scientific/release evidence |
| MSFLIB/platform extensions | `use MSFLIB`, signals, date/time, arguments | Host/runtime portability owner; preserve patch manifest | M0 |
| Integer/logical blocker | `TIME.FOR:551` semantics unclear | Verify semantics before patch | M0 |
| FE/manual vs FD/source stencil | `Reset` labels finite differences | Algebraic reconciliation and stencil fixture | M2 |
| Mixed precision and `sngl` drift | Source uses REAL/REAL*8 conversions | Compatibility mode plus calibrated f64 study | M1/M3 |
| Boundary pivot failure | `Gauss` substitutes tiny pivots and rewrites rows | Typed pivot diagnostics and singular fixtures | M3 |
| Retry side effects | Coupled histories/accounting can mutate on rejected steps | Transactional trial state and rollback tests | M3/M4 |
| Index/sign/unit hazards | One-based arrays, `CosAlf`, conversion factors | Validated adapters and dimensional fixtures | M2/M4 |
| State outside COMMON | Main arrays, DATA and compiler storage assumptions | Explicit ownership audit and no globals | M3 |
| Allocation in inner loops | Tables/workspaces and output paths can allocate | Fixed-size workspace counting | M3/M10 |
| Optional branch combinations | Source flags interact across modules | Capability matrix and typed rejection | M4 onward |
| Provenance/licensing gaps | Notices and redistribution terms unresolved | Artifact-specific review, no license inference | M0/M10 |
| Unsupported later physics dropped | Solute, heat, hysteresis, dual porosity required later | Dedicated milestones and traceability | Roadmap review |
| Host/format overcommitment | JSON/YAML/TOML/GeoPackage/Arrow and hosts differ | Provisional adapter recommendations | M9/M10 |

## Requirement traceability

| ID | Destination / disposition |
|---|---|
| R01 | Source audit/provenance; M0 hash pin and release decision gate. |
| R02 | Source audit and call/state audit; M0–M3 lifecycle evidence. |
| R03 | State/call audit and M3 ownership/index milestone. |
| R04 | API design plus numerical interfaces; M3/M4 core contract. |
| R05 | Feature matrix and numerical hydraulics; M1 with later family gates. |
| R06 | Numerical design; M2 stencil and M3 solver. |
| R07 | State/call rollback plus numerical design; M3/M4. |
| R08 | Numerical memory contract and validation allocation tests; M3/M10. |
| R09 | API/I/O design and validation; M4 then M10 legacy parity. |
| R10 | API serialization section; M9 provisional schema decision. |
| R11 | API host contracts and validation; M9/M10 platform gates. |
| R12 | Validation fixture matrix; M4 cyclic/root, M5 solute, M6 heat. |
| R13 | Numerical precision and validation tolerance policy; M0/M1/M3. |
| R14 | Validation component/property tests; M1/M4/M5. |
| R15 | Confirmed M4 first gate and M5/M6/M7/M8 later sequence. |
| R16 | Feature matrix and M9 optional capability gates; no silent omission. |
| R17 | This roadmap phase mapping and API/validation sections; M0/M4/M5-V/M10. |
| R18 | Master synthesis destination; supporting design documents provide inputs. |
| R19 | This orchestration remains planning-only and seeds later waves. |
| R20 | Decision register below and bounded worker policy in execution plan. |

Atomic prompt traceability:

| Prompt location / atomic requirement | Destination and disposition |
|---|---|
| Discovery 1.1: catalog all source files and named modules | `SOURCE_AUDIT.md` inventory; M0 provenance gate. |
| Discovery 1.1: map initialization/input, lookup, time loop, output and teardown lifecycle | `STATE_AND_CALLS.md` lifecycle; M0–M3. |
| Discovery 1.2: catalog COMMON/static/global state and group mesh, hydraulic, boundary, transport and solver domains | `STATE_AND_CALLS.md` state inventory; M3 ownership gate. |
| Architecture 2.1: define `SimulationConfig`, `Mesh1D`, `HydraulicModel`, `BoundaryConditions`, `SimulationState`, `MassBalanceTracker` | `API_AND_IO.md` signatures; numerical ownership in `NUMERICAL_DESIGN.md`; M3/M4. |
| Architecture 2.1: eliminate global state and provide re-entrant `Send + Sync`, filesystem-free core | API core contract; M3/M9 host gate. |
| Architecture 2.2: evaluate tridiagonal/narrow-band solver and external crates | `NUMERICAL_DESIGN.md` linear solver alternatives; M3 benchmark gate. |
| Architecture 2.2: preallocate workspace and avoid inner-loop allocation | numerical memory contract and validation allocation tests; M3/M10. |
| Architecture 2.3: legacy parsers/writers for `SELECTOR.IN`, `ATMOSPH.IN`, `PROFILE.DAT` and canonical outputs | `API_AND_IO.md` legacy I/O; M4 minimal adapter, M10 parity. |
| Architecture 2.3: serde-compatible schemas and dynamic in-memory forcing | API serialization/lifecycle sections; M9 provisional format gate. |
| Verification 3.1: dry infiltration, capillary rise, cyclic atmosphere, solute, and heat scenarios | `VALIDATION_STRATEGY.md` fixture matrix; M4–M6, with missing decks recorded. |
| Verification 3.2: L-infinity/L2 metrics, tolerances, precision drift and mass balance | validation tolerance policy plus numerical precision section; M0/M3. |
| Verification 3.3: curve derivatives, root uptake properties and conservation tests | validation component/property tests; M1/M4/M5. |
| Roadmap Phase 0: harness and ground-truth baseline | M0 portability patch and provenance review are complete; scientific fixture/output provenance remains per-physics work, with M5 solute comparison assigned to M5-V. |
| Roadmap Phase 1: soil physics and hydraulic curves | M1, minimum `iModel=0`, later families gated. |
| Roadmap Phase 2: mesh/profile/spatial discretization | M2, source stencil reconciliation. |
| Roadmap Phase 3: Richards/Picard/adaptive water solver and balance | M3, strict retry/rollback contract. |
| Roadmap Phase 4: boundary conditions and atmospheric sink/root uptake | M4 first acceptance, exact variants and fixtures above. |
| Roadmap Phase 5: solute and heat transport | M5/M6 later required milestones; no first-gate implication. |
| Roadmap Phase 6: I/O, polish, ergonomics, CLI and benchmarking | M10 after selected capability evidence. |
| Output 1: architecture block diagrams | `API_AND_IO.md` and future master synthesis; M10 documentation gate. |
| Output 2: Cargo workspace/module layout | `API_AND_IO.md`; proposed only, M10 implementation planning. |
| Output 3: core struct/trait signatures | `API_AND_IO.md` and `NUMERICAL_DESIGN.md`; proposed only. |
| Output 4: detailed risk registry | This document's Risk registry; owners and gates assigned. |
| Output 5: exact phase task checklist | This document's Phase checklists and Future wave-plan boundaries; future waves must refine them. |
| Global scope: distinguish evidence, recommendations and unresolved user decisions; preserve notices; do not claim release identity/baseline | All supporting docs; this Decision register; M0 and user gates. |

## Design reconciliation

The numerical design recommends preserving the observed half-cell finite-difference-style `Reset` stencil while documenting the manual finite-element terminology; this roadmap makes algebraic reconciliation an M2 gate. The API design keeps the core filesystem-free while M4 requires only the minimal input/forcing adapter needed for water validation; full legacy parity remains M10. Mixed precision is a compatibility recommendation, while all-f64 remains a measured modernization option. Feature matrix status is evidence of source branches, not release scope: M4 excludes later physics, and unsupported combinations must fail explicitly. Dual porosity follows validated single-domain water and is never substituted by rejected dual permeability.

## Decision register

| Decision gate | Recommendation | Alternatives | Owner / latest gate |
|---|---|---|---|
| Release identity | Say “4.08 or derivative,” pin artifacts | Adopt a verified release label only with evidence | Provenance owner / M0 |
| First hydraulic families | Start with one validated analytic family, then variants/tables | Include all families in M4 | Physics owner / M1 |
| Stencil interpretation | Preserve observed source algebra | FE rewrite after fixture evidence | Numerical owner / M2 |
| Precision | Mixed compatibility mode first | f64 modernization after drift study | Numerical/validation / M3 |
| First boundary set | Atmospheric plus supported fixed head/flux and lower modes needed by fixtures | Broader drainage/seepage in M4 | Water owner / M4 |
| Optional branches | Defer until schema and fixture exist | Commit selected branches earlier | Roadmap/user / M9 |
| Host and formats | Treat adapters as provisional and version schemas | Prioritize a host after demand evidence | API owner / M10 |
| License/provenance terms | Review each artifact and preserve notices | No assumptions from prompt | Provenance owner / M0/M10 |

Unresolved decisions are gates with owners above, not implicit commitments. No future implementation wave may begin until its prerequisite evidence and the relevant user-facing consequential decisions are recorded.

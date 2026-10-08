# Pinned Fortran solute formulation

## Scope and conventions

This report describes the pinned `src_reference/SOLUTE.FOR` and the retained Orchestration 16 comparisons. It does not execute the source or treat its behavior as physical truth.

Use physical coordinate `s` increasing from inlet to outlet and coefficient rows

`a_lower C[k-1] + a_diag C[k] + a_upper C[k+1] = rhs`.

The verification specification records native Fortran coordinates as `x=-s` and notes that input node indexing is reversed (`planning/master/SOLUTE_VERIFICATION_SPEC.md`, “Equation, dimensions and signs”, lines 58–65). The source's tridiagonal row is `B(i) C(i-1) + D(i) C(i) + E(i) C(i+1) = F(i)` (`SOLUTE.FOR`, `MatSet`, lines 830–864). Under coordinate reversal, its physical-order coefficients are `(E,D,B;F)`. This is an orientation conversion, not an assertion that finite-element and finite-volume rows have the same support or quadrature.

Labels below distinguish **Source fact**, **Measured trace fact**, **Inference**, and **Unresolved**.

## Assembly and temporal treatment

- **Source fact — assembly path.** `Solute` sets `NLevel=2` and loops over `Level=1..NLevel`; it calls `Coeff` to calculate transport/reaction-related quantities, `MatSet` to construct the matrix, then calls `BanSol` (`SOLUTE.FOR`, lines 35–37, 106–144). `BanSol` performs tridiagonal elimination/back-substitution (lines 945–960).
- **Source fact — element spatial rows.** In `MatSet`, adjacent interval widths are `a1=x(i)-x(i-1)` and `b1=x(i+1)-x(i)`. Interior rows use neighboring nodal values of `theta*Disp`, velocity, upstream-weight factors `wc`, and `g0/g1` (lines 830–864). The source header identifies the method as “Mass-lumping finite elements” (lines 3–4). The traced equation rows are therefore source-assembled element rows, not Rust control-volume rows.
- **Source fact — levels and weights.** The source defines `alf=1.-epsi` and uses two assembly levels (lines 35–37, 106–128). In the active Level 2 interior matrix expressions, `epsi` multiplies spatial terms, while storage contributes `dx/dt * thN(i) * Retard(i)` to the diagonal (lines 850–863). The Level 1 expressions shown in this block are largely commented; active uses of `alf` include boundary-flux terms and the retained report's observed two-level treatment. Do not infer a general scheme from commented expressions.
- **Measured trace fact — retained parameterization.** For diffusion, Orchestration 16 reports `epsi`-weighted current-level terms and `alf=1-epsi` Level 1 RHS terms, with `NLevel=2`, and gives the measured interior row differences against Rust (`SOLUTE_DIFFUSION_NORMALIZED_TRACE.md`, lines 85–96). Example 3 specifically reports `epsi=0.5` in its measured setup (`SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md`, lines 41–49, 63). Those settings are trace-specific, not proof every run uses those values.
- **Inference.** The combined temporal/spatial row construction differs from Rust's retained FV/BE rows. The retained comparisons do not isolate a pure time-weighting effect from the spatial basis and endpoint contributions.

## Transport, species, and reaction contributions

- **Source fact — coefficient inputs.** `Coeff` computes `Retard`, calls `Disper`, and forms `g0`, `g1`, `q0`, and `q1` from liquid, sorbed, gas, chain, and sink terms (`SOLUTE.FOR`, `Coeff`, around lines 620–727). Exact terms depend on species index and switches such as equilibrium, mobile-immobile, dual-porosity, and bacteria/filtration modes.
- **Source fact — dispersion.** `Disper` forms `Disp(i)` from the parameterized dispersivity contribution `ChPar(2,M)*abs(v)/ThW`, aqueous diffusion `Dw*TauW`, and gas contribution `ThG/ThW*Dg*Henry*TauG` (lines 1034–1050). Optional tortuosity, artificial dispersion, and `lArtD` branches alter the calculation (lines 1034–1068); these are conditional source paths, not universal terms in every configured run.
- **Source fact — upstream weighting.** `MatSet` includes `wc` in the velocity combinations in interior rows (lines 836–849, 851–861). `Coeff` calls `Disper` with `lUpW`; the `Disper` branch shown distinguishes `lUpW` when applying its non-artificial-dispersion correction (lines 1051–1068). **Unresolved:** the effect of a specific weighting choice cannot be generalized without the run's switches and values.
- **Source fact — species/reaction.** `Coeff` builds phase-specific reaction and decay coefficients and adds prior-species contributions to `g0/q0` for `jS>1` (lines 643–728). `NEquil` updates kinetic/mobile-immobile and dual nonequilibrium sorption states in applicable modes (lines 1078 onward); the source also contains a `MassTran` routine for integrated transfer accounting and related mobile-immobile/sorption terms (lines 1319–1426). These branches are mode-dependent.
- **Measured trace fact — reaction case.** The retained active-reaction report identifies a Fortran element row with a nonzero neighbor contribution and notes that `g1` includes phase-weighted loss; it says the available row does not provide the full element-to-node mass matrix or a common-basis reaction ledger (`SOLUTE_REACTION_NORMALIZED_EQUATIONS.md`, lines 40–65). Its reported chain-source quantities are not normalized nodal mass sources (lines 67–79).
- **Inference.** Species and reaction contributions are assembled through the Fortran spatial rows and may be distributed across element basis functions. A sampled row alone cannot establish a global transformation into Rust nodal control-volume weights.

## Boundary closures

- **Source fact — lower boundary.** `MatSet` branches on `kBotCh`: `1` sets a prescribed concentration row; `-1` constructs a type-3 boundary row with flow-direction handling; `0` sets `D(1)=-1`, `E(1)=1`, `F(1)=0` (lines 778–828). The last is a two-node zero-gradient relation in Fortran ordering.
- **Source fact — upper boundary.** For prescribed concentration, `kTopCh>0` sets `D(N)=1` and `F(N)=cTop`; other branches construct a type-3 row, apply flow-direction logic, and optionally add atmospheric transfer when `kTopCh=-2` (lines 867–927).
- **Measured trace fact.** The diffusion task's selected `kBotCh=0` closure maps to the physical outlet and is the normalized Fortran equation `(1,-1,0;0)`; Rust instead has a half-cell balance (`SOLUTE_DIFFUSION_NORMALIZED_TRACE.md`, lines 98–105). The Robin report likewise records a Fortran zero-gradient outlet versus Rust's half-cell balance (`SOLUTE_ROBIN_NORMALIZED_ROWS.md`, lines 17–30). In Example 3, the measured Fortran outlet row is `(1,-1,0;0)` after reversal (`SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md`, lines 25–39).
- **Inference.** Endpoint rows are independent sources of measured operator difference; interior row comparisons cannot stand in for boundary-row equivalence.
- **Unresolved.** These reports measure selected closures/cases only; they do not measure every combination of `kBotCh`, `kTopCh`, vapor, atmospheric, and flow-direction switches.

## Precision conventions

- **Source fact.** `Solute` declares `B,D,E,F,t` as double precision (line 23), as do `MatSet`'s coefficient arrays and `BanSol`'s working arrays (lines 762, 946). Other quantities are not declared double precision in these routines; absent explicit declarations, their precision follows the source/compiler's Fortran defaults and any build flags.
- **Measured trace fact.** The normalized reports describe Fortran printed values as rounded single-precision values where applicable (for example, `SOLUTE_REACTION_NORMALIZED_EQUATIONS.md`, lines 9–13), and the diffusion report records output values at finite decimal precision (`SOLUTE_DIFFUSION_NORMALIZED_TRACE.md`, lines 85–96).
- **Unresolved.** The retained source excerpt and reports do not establish the precision of every expression, input/output conversion, or compiler mode. Do not characterize the entire Fortran calculation as double precision based only on the matrix declarations.

## Measured differences at matched locations

All measured statements here are limited to the sampled cases/rows described by the reports. Mapping to the common convention is Fortran `(E,D,B;F)` to physical `(lower,diagonal,upper;rhs)`.

| Retained comparison | Measured Fortran evidence | Supported interpretation / limit |
|---|---|---|
| Diffusion, 65 nodes, first step | All 63 interior rows differ from Rust by `(+1.37599995327,-2.75199926758,+1.37599995327)` in lower/diagonal/upper coefficients; outlet closure also differs. | Measured rows show an operator difference, not which method Rust is required to implement (`SOLUTE_DIFFUSION_NORMALIZED_TRACE.md`, lines 85–105, 117–125). |
| Scalar Robin, inlet/midpoint/outlet | The sampled midpoint Fortran row is `(-13.86759472,30.10119629,-13.65259457;1.96497154)`; the physical outlet is `(1,-1,0;0)`. | The report attributes row differences jointly to FE/FV spatial-time assembly, upstream weighting, and endpoint closure; no weighting-only counterfactual was measured (`SOLUTE_ROBIN_NORMALIZED_ROWS.md`, lines 17–32). |
| Active reaction, first interval | A sampled interior Fortran row has nonzero neighbor contribution; retained source/reaction quantities are not fully normalized to Rust nodal weights. | No full common-basis Fortran reaction/storage matrix is available; sampled row does not establish whole-operator parity (`SOLUTE_REACTION_NORMALIZED_EQUATIONS.md`, lines 42–65, 73–79). |
| Example 3, first interval | At the measured inlet/midpoint/outlet for three species, Fortran rows differ from Rust; the Fortran outlet is `(1,-1,0;0)`. | This is a derived fixture and sampled first-interval evidence, not a recovered manual deck or independent analytical oracle (`SOLUTE_EXAMPLE3_NORMALIZED_TRACE.md`, lines 23–49, 61–63). |

## Conclusions and limits

- **Source facts:** The pinned source uses two assembly levels, element-based tridiagonal assembly with mode-dependent transport/reaction contributions, explicit upstream-weight factors in row construction, and configurable endpoint branches. The source comments describe the method as mass-lumping finite elements.
- **Measured facts:** Retained Orchestration 16 reports demonstrate differing Fortran and Rust rows at matched physical locations in diffusion, Robin, reaction, and Example 3 evidence. Diffusion and Example 3 directly measure multiple rows; Robin measures selected endpoint/interior rows; the reaction report documents a normalization gap.
- **Inference:** The differences are consistent with distinct spatial/time assembly, upstream weighting, species/reaction terms, and endpoint treatment. Existing traces do not allocate all observed state/profile differences uniquely among those causes or establish a Rust defect.
- **Unresolved:** Whole-operator comparisons are not available for every configuration; the reaction evidence lacks a complete common-basis matrix; weighting-only and boundary-only counterfactuals are absent. Precision is not established for every computation. The reports provide no basis to claim that pinned Fortran is physically correct or that parity proves continuum accuracy.

No Fortran execution, solver run, or production/test change was made for this report.

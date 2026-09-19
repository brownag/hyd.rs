# HYDRUS Rust backend: research for implementation planning

Research date: 2026-09-19. Read this document together with [INITIAL_PROMPT.md](INITIAL_PROMPT.md). This is research input for a later **plan-wave** session and **wave-orchestration** execution, not an approved implementation plan. Recommendations below are proposals; unresolved scope choices remain decisions for that planning session.

## Read first: corrections to the initial assumptions

- **Version identity is unresolved:** the supplied archive matches the extracted files, but its timestamps and header do not establish a v4.17 source release.
- **The oracle does not compile unchanged here:** a GNU Fortran syntax check found missing MSFLIB and an integer/logical extension. No successful simulation baseline exists yet.
- **Precision is mixed:** field arrays and explicit single-precision conversions coexist with double-precision arithmetic. All-f64 Rust changes numerical behavior and requires calibrated acceptance criteria.
- **The local source is narrower than the advertised product:** dual permeability is explicitly rejected; other optional models need an implementation/fixture coverage matrix.
- **Cross-language support needs explicit host contracts:** browser Wasm, native Node, Python, R, and a C ABI are distinct adapters around the same owned Rust state.

Evidence and exact anchors appear in [the local audit](#6-local-source-audit-and-reproducible-evidence). [Architecture options](#7-architecture-recommendations-for-frontend-independence), [validation requirements](#8-validation-evidence-required-before-implementation-waves), and [the planning handoff](#9-handoff-to-the-plan-wave-agent) distinguish recommendations from unresolved decisions.

## 1. Main findings and source authority

The intended product is a modern, platform-independent numerical Rust backend with Python, R, JavaScript, and other callers. The main planning risk is treating a source port, a HYDRUS-1D 4.17 compatibility implementation, and a new scientific API as the same deliverable. They need distinct acceptance criteria.

The local 4.08 and 4.17 manuals are primary scientific documentation. The 4.17 title page is dated June 2013; the 4.08 manual describes a particular earlier feature set. Neither title establishes the version or completeness of the local Fortran source. The manual itself carries older version references in its abstract, so use title-page identity, chapter-level requirements, and source evidence together. Sources: [local 4.08 manual](reference/HYDRUS1D-4.08.pdf), [local 4.17 manual](reference/HYDRUS1D-4.17.pdf), especially title pages, introduction, and input-data chapters.

The two requested PC-Progress URLs were opened, but the browsing service returned the generic homepage for both (including normalization of the query parameter to an empty value). Direct Python retrieval returned HTTP 403. The legacy manual URLs also failed through the browsing service. Therefore this report does **not** claim to have verified the current contents of the requested description page or its `#k8` source-code section. Sources attempted: [requested description/source page](https://www.pc-progress.com/en/Default.aspx?H1D-description#k8), [requested product page](https://www.pc-progress.com/en/Default.aspx?hydrus-1d).

A public HydroShare archive crediting Šimůnek, Šejna, and van Genuchten identifies its executable/manual as 4.16, but its source/manual pair as 4.08. Its README preserves the authors' warning that the source posted in 2009 differs from later computational modules. The archive declares GPLv3. This is useful provenance evidence, not proof that an independently supplied local source tree is identical to that archive or that every constituent has the same license. Source: [HydroShare archive and README](https://www.hydroshare.org/resource/d24921ba081d431e80685bf177a0840f/).

An accessible current official manual identifies itself internally as **Version 5.05, January 2025**, despite older cover metadata. It documents additional capabilities and a broader product. It is a useful scientific cross-reference, not a substitute specification for 4.17 or evidence of source availability. Source: [PC-Progress technical manual, title pages and contents](https://www.pc-progress.com/downloads/Pgm_Hydrus3D5/HYDRUS_Technical_Manual_1D_V5.pdf).

**Planning implication:** establish separate version identifiers for the local source snapshot, each manual, any reference executable, fixture format, and Rust compatibility profile. A claim of “4.17 parity” must name the exact 4.17 binary and tested feature matrix; compiling an older source snapshot cannot establish that claim.

## 2. Scientific scope: existing model versus new product requirements

The historical manuals describe considerably more than a single Richards-equation solver. They include heterogeneous profiles, inclined flow, root uptake, hydraulic hysteresis, dual-domain flow and transport, sequential reactions, heat/vapor coupling, inverse estimation, and optional chemistry systems. A manual-level capability is not evidence that the supplied source implements it. Sources: [4.08 manual](reference/HYDRUS1D-4.08.pdf), introduction and Chapters 2–9; [4.17 manual](reference/HYDRUS1D-4.17.pdf), same chapters.

| Workstream | Evidence and scope distinction | Implication for later planning |
| --- | --- | --- |
| Richards water flow | Existing model; Chapters 2 and 7 of both local manuals | Identify the supported single/dual-domain modes before assigning a matrix structure or solver. |
| Retention, conductivity, capacity | Existing hydraulic-function families and hysteresis; §2.3–2.7 | Match implemented model selectors, parameter bounds, saturated branches, and interpolation behavior, not just textbook formula names. |
| Root water uptake | Existing sink models and compensation; §2.2 | Keep potential transpiration, depth distribution, stress response, and compensation distinct. |
| Atmospheric and lower boundaries | Existing prescribed and system-dependent boundaries; §2.8, §7.4.3, Chapter 10 | Boundary switching is part of the numerical algorithm and must be tested within nonlinear iterations. |
| Solute transport | Existing reactions, nonequilibrium, and multiple-species behavior; Chapters 3 and 8 | “Solute enabled” is too broad a milestone; inventory each reaction/domain mode. |
| Heat and vapor coupling | Existing manual capabilities; Chapters 2 and 4 | Distinguish conduction/convection heat from fully coupled water/vapor/energy transport. |
| Inverse estimation and special chemistry | Documented in Chapters 5, 6, and 9 | INITIAL_PROMPT does not clearly commit to porting these systems; record explicit inclusion/defer decisions. |
| Re-entrancy, language bindings, incremental forcing, serialization | Requirements of the new Rust product in INITIAL_PROMPT and the user's request | These require new lifecycle, ownership, error, and compatibility contracts; they are not legacy numerical parity. |
| GeoPackage/Arrow and JSON/YAML/TOML | Formats named by INITIAL_PROMPT | They are not interchangeable `serde` encodings. Decide which are actual deliverables and which are future adapters. |

Source for the table's scientific descriptions: [4.08 manual](reference/HYDRUS1D-4.08.pdf) and [4.17 manual](reference/HYDRUS1D-4.17.pdf), sections named in each row. Product scope is sourced to [INITIAL_PROMPT.md](INITIAL_PROMPT.md) and the user's current request.

## 3. Numerical behavior that the blueprint must preserve explicitly

### 3.1 Discretization and nonlinear solution

The 4.08 manual §7.1 describes a fully implicit time discretization, Picard iteration, and the mass-conservative water-content treatment associated with Celia et al. The linearized water system is solved by Gaussian elimination exploiting its tridiagonal symmetric structure (§7.4.1). Initial pressure-head estimates are extrapolated from previous time levels. Convergence compares pressure changes in saturated regions and water-content changes in unsaturated regions. Source: [4.08 manual](reference/HYDRUS1D-4.08.pdf), printed pp. 101–105; corresponding 4.17 discussion is Chapter 7.

**Recommendation:** a small reusable tridiagonal kernel is a strong candidate for the audited single-domain water path, but approve it only after inspecting every assembly and elimination branch. Symmetric water assembly does not prove that reactive transport or coupled-domain systems share that contract. Distinguish a faithful mixed-precision reference mode from a deliberate all-`f64` modernization; numerical agreement must determine whether the latter changes convergence or accepted time steps.

### 3.2 Adaptive stepping and events

The manual distinguishes numerical time steps, boundary-condition time points, and requested output times. Numerical steps must align with the latter two. Its documented adaptation increases/decreases steps according to iteration counts; excessive iterations trigger a retry at one third of the failed step. Source: [4.08 manual](reference/HYDRUS1D-4.08.pdf), §7.4.2, printed pp. 105–106. These are manual rules; exact thresholds and corner cases must be verified in the source.

**Recommendation:** the in-memory API should separate a requested target time from an internally accepted time step. Define forcing interpolation, discontinuity handling, duplicate event times, and final-time rounding. A failed attempt must roll back every mutable physical and accounting quantity, including boundary mode, hysteresis state, uptake totals, transport history, and output-event cursors. Otherwise incremental coupling may silently diverge from batch runs.

### 3.3 Boundary transitions

Atmospheric conditions switch between head and flux depending on hydraulic constraints. A seepage face can switch between zero flux when unsaturated and a prescribed limiting head when saturated. These transitions are assessed during solution, not merely when reading input. Source: [4.08 manual](reference/HYDRUS1D-4.08.pdf), §7.4.3, printed p. 106.

**Recommendation:** retain separate configured boundary type and current active boundary mode. Test transitions in both directions, ponding limits, and rejection/retry near thresholds. Keep coordinate orientation and signed flux conventions explicit at every adapter boundary.

### 3.4 Conservation metrics

HYDRUS' reported relative water-balance error is not simply absolute imbalance divided by total stored water. Its denominator uses the larger of accumulated absolute element storage changes and accumulated absolute boundary/uptake contributions. Source: [4.08 manual](reference/HYDRUS1D-4.08.pdf), §7.4.4, equations 7.17–7.20, printed pp. 106–108.

**Recommendation:** reproduce legacy diagnostic definitions for compatibility and additionally expose unambiguous storage change, boundary integrals, sink integrals, absolute residual, and normalization. Specify zero-denominator behavior. Do not apply a universal percentage threshold to a closed, nearly stationary system without an absolute-error criterion.

## 4. Validation research and proposed acceptance evidence

The five benchmark types in INITIAL_PROMPT are useful requirements, but they are not the manual's numbered examples. In particular, manual Example 2 is a field profile under grass, not a standalone capillary-rise test. The prompt's capillary-rise case therefore needs an explicitly constructed fixture and provenance rather than a misleading manual-example label. Sources: [INITIAL_PROMPT.md](INITIAL_PROMPT.md), §3; [4.08 manual](reference/HYDRUS1D-4.08.pdf), Chapter 11; [4.17 manual](reference/HYDRUS1D-4.17.pdf), Chapter 11.

| Candidate fixture | Primary reference | Evidence it can provide |
| --- | --- | --- |
| Column infiltration | 4.08 §11.1, p. 148; 4.17 §11.1, p. 152 | Sharp-front water-flow behavior; initial head −150 cm, saturated conductivity 0.0433 cm/min, and 90-minute run are specified in 4.08. Full input data must still be recovered/constructed. |
| Seasonal field profile under grass | 4.08 §11.2, p. 151; 4.17 §11.2, p. 155 | Layered soil, atmospheric forcing, uptake, and drainage relationship. |
| Sequential decay chain | Both manuals §11.3 | Multi-solute reaction coupling and comparison with an analytical solution. |
| Nonlinear and nonequilibrium sorption | Both manuals §11.4–11.5 | Separate equilibrium/nonlinear and kinetic transport cases. |
| Hysteresis | Both manuals §11.10 | Drainage/imbibition history, if present in the scoped source. |
| Coupled water/vapor/heat | Both manuals §11.11 | Coupled physics; should not be relabeled as a heat-only case. |
| Hydrostatic equilibrium and capillary rise | New fixtures proposed by this report/the prompt | Independent sign, gravity, conservation, and transient-flow checks; not claimed to be distributed manual examples. |

Sources: the two [local](reference/HYDRUS1D-4.08.pdf) [manuals](reference/HYDRUS1D-4.17.pdf), Chapter 11. An illustrated result or published comparison is not a machine-readable golden fixture.

Recommended evidence layers:

1. **Source-snapshot parity:** compiled local Fortran against Rust for identical inputs and feature switches. Record source hashes, compiler/version, flags, runtime platform, exit status, and outputs.
2. **Release compatibility:** separately compare against an identified 4.17 executable where available. Preserve discrepancies between source and release behavior rather than silently changing the baseline.
3. **Independent numerical checks:** equilibrium/conservation identities, analytical limits, mesh/time refinement, and applicable analytical transport solutions. Agreement with a legacy executable alone reproduces its defects too.
4. **Cross-interface equivalence:** the same configuration and forcing through Rust and each implemented binding must yield equivalent accepted state and diagnostics.

`1e-6` in INITIAL_PROMPT is an example target, not an established scientifically justified universal acceptance threshold. Specify pressure units, water-content units, mixed absolute/relative tolerances, time alignment, and norm normalization. Text output rounding bounds observable accuracy; pressure, moisture, cumulative flux, and relative mass error require different criteria. Compare accepted-step trajectories and boundary modes as diagnostics, while allowing a separately justified physical-result tolerance where platform rounding changes adaptation. These are recommendations derived from the manual's nonlinear/event-driven behavior and the prompt's proposed parity requirement.

No baseline run, golden fixture set, or cross-platform numerical tolerance has been established by this research alone.

## 5. Provenance and release-readiness questions

The prompt calls the code public domain. Available evidence does not justify applying that statement indiscriminately to the source snapshot, manuals, example data, GUI, later releases, or embedded third-party routines. The HydroShare deposit uses GPLv3 and distinguishes its source from later binaries; the current official manual carries a copyright notice. Sources: [HydroShare archive](https://www.hydroshare.org/resource/d24921ba081d431e80685bf177a0840f/), [current official manual](https://www.pc-progress.com/downloads/Pgm_Hydrus3D5/HYDRUS_Technical_Manual_1D_V5.pdf), title pages.

**Recommendation:** create an artifact-specific provenance inventory before choosing the Rust distribution license: original acquisition URL, archive hash, contained license/notice, source header, and any third-party numerical routines. Preserve attributions. Treat unresolved rights as a concrete release dependency rather than guessing a license from “free software” or product-page wording. This is an evidence gap, not a legal determination.


## 6. Local source audit and reproducible evidence

This section records direct inspection on 2026-09-19. Paths are relative to this document. Recommendations below are design proposals, not existing implementation or approved scope.

### Snapshot identity and build status

The workspace contains ten Fortran files (11,053 lines), the source ZIP, the initial prompt, and three PDFs. No Cargo workspace, test suite, benchmark input decks, golden outputs, build scripts, or applicable AGENTS.md were found. `git -C .. status --short` reported that the workspace is not a Git repository. Therefore there is no green baseline and no existing worktree/commit workflow to inherit. Recheck these facts when planning begins.

`planning/H1D_Src.zip` has SHA-256 `d0c47cbcb7a298ab16c6e08010bd7ca048d5527ce2f989a749390c0c55e848cf`; all ten extracted files matched their archive entries byte for byte. Archive timestamps span 2009–2022. The main header says internal “version 7.0,” modified October 2009, whereas the local 4.17 manual title page says June 2013. Neither stale comments nor ZIP timestamps prove release identity. Pin this snapshot by hash and resolve the intended v4.17 baseline before calling results v4.17 parity. Sources: [archive](H1D_Src.zip), [main header](../src_reference/HYDRUS.FOR), [4.17 manual](reference/HYDRUS1D-4.17.pdf).

An unmodified syntax check was run in a temporary directory with GNU Fortran 13.3.0:

```sh
# Run from repository root; use a scratch working directory if generating objects later.
gfortran -fsyntax-only -std=legacy -ffixed-line-length-none src_reference/*.FOR
```

Exit status: **1**. Observed blockers: `HYDRUS.FOR:29`, missing `msflib.mod`; `TIME.FOR:551`, `if(iMetHour)` requires a scalar logical expression. This was a syntax check, not a link or simulation run. No numerical baseline was generated. Additional link/runtime issues may emerge after these blockers are addressed. Rust was available as `rustc 1.96.0-nightly (c75612477 2026-04-07)`; this is an environment observation, not a recommended toolchain pin.

Keep original files immutable. Build a separately identified reference copy with a minimal, reviewable portability patch for MSFLIB/signal handling, command arguments, dates/times, Windows path separators, and compiler-specific logical behavior. Verify the intended `iMetHour` semantics before patching. Record patch hash, compiler/flags, executable hash, inputs, and outputs. Do not silently promote all REALs to double precision or rewrite algorithms in the oracle. Sources: [HYDRUS.FOR](../src_reference/HYDRUS.FOR), anchors `use MSFLIB`, `SIGNALQQ`, `NARGS`, `GETARG`, `getdat`, `gettim`, `CloseOutput`; [TIME.FOR](../src_reference/TIME.FOR), anchor `if(iMetHour)`.

### Source inventory and entry anchors

Line numbers refer to the supplied snapshot; use symbol searches if lines move. This is a navigation map of major routines, not an exhaustive mechanically resolved call graph.

| Source | Lines | Responsibilities and useful entry anchors |
| --- | ---: | --- |
| [HYDRUS.FOR](../src_reference/HYDRUS.FOR) | 999 | Main program line 20; initialization 123; time loop 399; `ErrorOut` 784; `CloseOutput` 853; `CloseFiles` 936 |
| [INPUT.FOR](../src_reference/INPUT.FOR) | 1,723 | `BasInf` 3; `Conversion` 182; `NodInf` 208; `MatIn` 536; `GenMat` 762; `TmIn` 866; `SinkIn` 1070; `TempIn` 1205; `ChemIn` 1289; `iGetFileVersion` 1498; `Init` 1560 |
| [MATERIAL.FOR](../src_reference/MATERIAL.FOR) | 946 | Conductivity `FK` 16; capacity `FC` 201; water content `FQ` 279; inverse `FH` 350; `FS` 421; `FKQ` 493; `FKS` 557; auxiliary root finding/quadrature from 623 |
| [WATFLOW.FOR](../src_reference/WATFLOW.FOR) | 1,127 | `WatFlow` 3; assembly `Reset` 173; elimination `Gauss` 308; boundary switching `Shift` 359; properties `SetMat` 433; drains `FqDrain` 579; `Veloc` 717; `Hyster` 778; vapor routines 849–1001; `DualPor` 1027; `Update` 1079 |
| [HYSTER.FOR](../src_reference/HYSTER.FOR) | 756 | Lenhard `Hyst` 6; `HysterIni` 124; `PATH` 240; `HAWPATH` 292; `DRAIN` 444; `DRY` 535; `WET` 594; `UPDATEHYST` 652 |
| [SINK.FOR](../src_reference/SINK.FOR) | 366 | `SetSnk` 3; solute uptake `SetSSnk` 138; salinity `FSAlfa` 226; stress `FAlfa` 245; growth `SetRG` 268; `RootInN` 326 |
| [TIME.FOR](../src_reference/TIME.FOR) | 1,594 | `TmCont` 3; `SetBC` 59; chemistry BC 186; daily forcing 213; precipitation 238; snow 257; `Meteo` 317; interception 787/823; surface evaporation 913; energy update 1166; daily meteo 1225 |
| [SOLUTE.FOR](../src_reference/SOLUTE.FOR) | 2,105 | `Solute` 6; `Coeff` 241; `MatSet` 755; `BanSol` 945; `PeCour` 965; `Disper` 1025; `NEquil` 1078; `MassTran` 1319; deposition 1595; moisture dependence 1786; initial mass 1912 |
| [TEMPER.FOR](../src_reference/TEMPER.FOR) | 323 | `Temper` 5; `SetUpHeat` 77; `TempAdj` 180; `TempCap` 269; `xLatent` 310 |
| [OUTPUT.FOR](../src_reference/OUTPUT.FOR) | 1,114 | `TLInf` 3; `ALInf` 296; balances `SubReg` 325; profiles `NodOut` 720; observations `ObsNod` 931 |

The actual filename is `WATFLOW.FOR`, not the illustrative `WATER.FOR` in INITIAL_PROMPT.

### Execution lifecycle and coupling

The following partial hierarchy is traced from [HYDRUS.FOR](../src_reference/HYDRUS.FOR), with callees in the table above. Conditional branches matter; it is not a universally executed sequence.

```text
HYDRUS
  Init -> open input/output units -> BasInf
  NodInf -> MatIn -> InitW/GenMat/SetMat or Hyst -> InitDualPor
  TmIn -> optional MeteoIn/RootIn/TempIn/ChemIn
  SetBC / meteorology / snow / interception -> SinkIn -> SetSnk
  initial Profil/NodOut/SubReg/Veloc
  time loop (label 12)
    optional energy-balance Evapor
    WatFlow
      DualPor (optional)
      nonlinear iteration: SetMat or Hyst -> Reset -> Shift -> Gauss
      convergence test -> retry with smaller dt / return failure
    Veloc -> root growth / SetSnk
    Temper -> optional return to water/heat coupling loop (label 13)
    Solute -> Coeff / MatSet / BanSol and reaction/transfer branches
    TLInf / ObsNod / NodOut / SubReg / ALInf
    update atmospheric/meteo records
    terminate or TmCont -> forcing update -> Update -> next step
  CloseOutput / error dispatch -> CloseFiles / STOP
```

Important consequences for a port:

- Boundary switching is inside the water nonlinear solve. Phase 3 cannot have meaningful water parity if all boundary work is postponed to Phase 4; at least fixed head/flux and the exact associated assembly must arrive together. [WATFLOW.FOR](../src_reference/WATFLOW.FOR), `WatFlow`, `Shift`.
- `WatFlow` reduces failed-step `dt` by three, restores heads, temperature and boundary codes, and can return non-convergence at minimum step. The main loop counts consecutive failures and can continue before its stop threshold. Decide explicitly whether the modern API offers strict failure by default and an opt-in legacy behavior; preserve enough diagnostics to distinguish them. [WATFLOW.FOR](../src_reference/WATFLOW.FOR), lines 126–154; [HYDRUS.FOR](../src_reference/HYDRUS.FOR), lines 433–445.
- `TmCont` aligns steps to print, forcing, and final times and incorporates other process limits. A coupled API needs a caller-specified stop time in the same event scheduler; output schedules can affect the legacy time trajectory. [TIME.FOR](../src_reference/TIME.FOR), `TmCont`; [HYDRUS.FOR](../src_reference/HYDRUS.FOR), lines 640–699.
- Water `Reset` explicitly labels its stencil “Finite differences,” uses half-cell widths at boundaries and arithmetic conductivity averages by default. Reconcile this with the manual’s finite-element description algebraically. Do not replace the stencil with a generic FE library merely because the prompt says FEM. [WATFLOW.FOR](../src_reference/WATFLOW.FOR), `Reset`, especially lines 189–228.
- Water `Gauss` has special boundary rows and tiny-pivot substitutions; transport `BanSol` is a different general tridiagonal elimination. A reusable Rust solver must retain caller-specific assembly and make pivot/failure policy explicit. [WATFLOW.FOR](../src_reference/WATFLOW.FOR), `Gauss`; [SOLUTE.FOR](../src_reference/SOLUTE.FOR), `BanSol`.

### State, precision, and limits

Most state lives in arrays declared in the main program and passed through long argument lists. The three distinct COMMON blocks found are `/properties/`, `/glob/`, and `/local/`, repeated in HYSTER.FOR. A COMMON-only migration would miss most state and file-unit coupling. No explicit SAVE statement was found in the ten files; DATA initialization and compiler storage behavior still require inspection. Sources: [HYDRUS.FOR](../src_reference/HYDRUS.FOR), declarations 49–97; [HYSTER.FOR](../src_reference/HYSTER.FOR), declarations 9–20; [MATERIAL.FOR](../src_reference/MATERIAL.FOR), `BInc` DATA initialization.

| Ownership domain proposed | Observed legacy state | Porting implication |
| --- | --- | --- |
| Validated configuration | flags, `TolTh`, `TolH`, `MaxIt`, material parameters | Reject unsupported combinations before stepping |
| Mesh/material map | `x`, `MatNum`, `LayNum`, `Ah`, `AK`, `ATh` | Preserve bottom-to-top indexing and conversion from one-based IDs |
| Hydraulic data/cache | `ParD`, `ParW`, `hTab`, `ConTab`, `CapTab`, `TheTab`, `NTab` | Keep direct evaluation versus table interpolation selectable for comparison |
| Accepted and trial state | `hOld/hNew/hTemp`, `ThOld/ThNew`, `TempO/TempN`, velocities | Transactional rejected steps must restore every affected state component |
| Hysteresis history | three COMMON blocks; `Kappa`, reversal arrays | Per-simulation history, including seven reversal slots in the legacy representation |
| Forcing and boundary state | `KodTop/KodBot`, old codes, ponding, meteo/snow/interception variables | Separate specified forcing from active solver boundary mode |
| Solute and heat state | concentrations, sorbed pools, immobile water, temperature, transfer arrays | Model each stored mass compartment and coupling order explicitly |
| Diagnostics and scratch | `P/R/S/Q`, `CumQ`, water/solute accumulators | Separate reusable workspace, balance state, and output formatting |

The main arrays are bounded by parameters including 1,001 nodes, 20 materials, 100 table/observation slots, 11 solute slots, and 1,000 print/growth slots; some arrays also hard-code ten balance regions. These are storage bounds, not all guaranteed supported user counts. Hysteresis hard-codes 1,001 nodes independently. Dynamic Rust storage needs validation and tests at old boundaries; removing one bound does not remove every related bound. Sources: [HYDRUS.FOR](../src_reference/HYDRUS.FOR), lines 49–95; [HYSTER.FOR](../src_reference/HYSTER.FOR), `/glob/`.

The source is **mixed precision**: time and water linear coefficients are double precision; many field arrays are default REAL; `Gauss` explicitly converts results with `sngl`; hydraulic routines often compute double intermediates but accept/return REAL. Hysteresis likewise mixes implicit REAL*8 internals and explicitly REAL arguments. Thus an all-f64 Rust port is a deliberate precision change, not a type-for-type translation. A blanket `1e-6` head/moisture threshold and byte-identical output cannot be promised without experiments. Sources: [HYDRUS.FOR](../src_reference/HYDRUS.FOR), declarations; [WATFLOW.FOR](../src_reference/WATFLOW.FOR), `Gauss`; [MATERIAL.FOR](../src_reference/MATERIAL.FOR), `FK`; [HYSTER.FOR](../src_reference/HYSTER.FOR), `Hyst`.

`MatIn` explicitly rejects `iModel=8` because dual-permeability is implemented in different code. Distinguish Durner’s bimodal retention function, mobile/immobile dual-porosity exchange, and two-flow-domain dual permeability. Do not infer availability of the latter from a hydraulic model name or website feature list. [INPUT.FOR](../src_reference/INPUT.FOR), `MatIn`, lines 552–565.

## 7. Architecture recommendations for frontend independence

These recommendations interpret the user's platform-independent, multi-language goal. They are inputs to the later design interview, not approved crate names or interfaces.

```text
Python adapter   R adapter   Node adapter / browser Wasm   C ABI / other hosts
       \             |              |                         /
                     safe Rust simulation API
                   /            |              \
          validated model   owned state      diagnostics
                   \            |              /
           hydraulic laws + assembly + time/event control
                          reusable workspace

Legacy text adapter / CLI ---> same safe Rust API
Serialization adapters -----> validated configuration conversion
```

A candidate workspace is `hydrus-core` (physics, state, errors, stepping), `hydrus-compat` (legacy parsing/writing), `hydrus-cli`, and optional `hydrus-python`, `hydrus-r`, `hydrus-wasm`, `hydrus-ffi`; add a Node-native crate only if required. Start with modules inside the core rather than one crate per physical process. Keep all runtime, filesystem, GUI, and foreign-object dependencies outside the core. This follows the task's explicit I/O and re-entrancy goals, with the actual coupling documented above.

A design sketch for the planner to refine:

```rust
// Illustrative signatures only; not implemented or a finalized ABI.
Simulation::new(config: ValidatedConfig, initial: InitialState)
    -> Result<Simulation, SimulationError>;
simulation.advance_to(stop_time, forcing) -> Result<StepReport, SimulationError>;
simulation.state() -> StateView<'_>;
simulation.copy_outputs_into(buffers) -> Result<(), SimulationError>;
```

Require monotonically advancing time, explicit forcing units/signs and interpolation semantics, bounded event handling, accepted-step diagnostics, and structured non-convergence errors. Do not allow a failed trial to partially commit histories or balance accumulators. Clarify whether `advance_to` performs many internal adaptive steps, and how a caller changes boundary inputs at synchronization points. Cancellation should be checked at bounded solver boundaries. Avoid callbacks into foreign runtimes from inside Picard loops.

One mutable simulation should have one active owner. `Send + Sync` can be compile-time properties of owned Rust types without promising simultaneous mutable stepping. Independent simulations are the first parallelism target; host wrapper objects need their own synchronization rules. Immutable configuration can be shared; mutable workspaces and hysteresis must be per instance. Validate serial/interleaved/threaded runs for contamination rather than relying only on trait bounds.

### Binding options and costs

| Host | Primary-source capability | Recommended boundary and unresolved choice |
| --- | --- | --- |
| Python | PyO3 exposes Rust and can detach from Python during long computation. [PyO3 parallelism](https://pyo3.rs/main/parallelism) | Convert/validate inputs before detaching; compute on Rust-owned memory; return arrays/structured errors. Decide supported Python builds and wheel platforms later. |
| R | extendr provides Rust-to-R package bindings. [extendr introduction](https://extendr.rs/introduction/); [thread-safety implementation](https://extendr.rs/extendr/src/extendr_api/thread_safety.rs.html) | Keep R allocation/API access on the host thread, with worker computation limited to owned Rust values. Specify NA/NaN, indexing, external-pointer lifetime, interrupts, and R package distribution tests. |
| Browser JavaScript | wasm-bindgen supports numeric typed arrays; boxed numeric slices copy across the boundary. [wasm-bindgen arrays](https://wasm-bindgen.github.io/wasm-bindgen/reference/types/boxed-number-slices.html) | Offer coarse calls and batched outputs; start with copies. Any later zero-copy view needs explicit lifetime and memory-growth rules. Browser workers are a separate packaging/integration choice. |
| Node.js | napi-rs builds native Node addons. [napi-rs guide](https://napi.rs/docs/introduction/getting-started) | Native Node and browser Wasm are different deliverables. Decide whether Node can use the Wasm package or needs a native addon. |
| Other native hosts | Rust supports C calling conventions and `cdylib`/`staticlib`. [Rust FFI guide](https://doc.rust-lang.org/nomicon/ffi.html) | Optional versioned opaque-handle ABI; use fixed-width status codes, explicit pointer/length buffers, matching destroy functions, and documented ownership. Keep unsafe code in this adapter. |

For C ABI use, specify panic containment, invalid-handle behavior, allocation ownership, error-message lifetime, and concurrency. Never expose Rust `Vec`, `String`, trait objects, or Rust-layout enums as a stable C ABI. Catching unwinding panics does not recover an aborting process; expected numerical failures should use typed errors. [Rust FFI guide](https://doc.rust-lang.org/nomicon/ffi.html).

The `wasm32-unknown-unknown` target does not provide ordinary host filesystem or thread functionality; std availability alone is not portability. Build/test the pure core for Wasm early if browser support is selected, and keep CLI/filesystem dependencies out of that dependency graph. [Rust target documentation](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html).

Serde supplies serialization traits and format integrations; it does not define a scientific schema or automatically provide domain-specific GeoPackage/Arrow support. Recommend a versioned configuration DTO with validation, one first format such as JSON, and explicit future adapter decisions for TOML/YAML/Arrow/GeoPackage. Keep physical units and array ordering in the schema. [Serde overview](https://serde.rs/); [INITIAL_PROMPT](INITIAL_PROMPT.md).

### Numerical and memory design choices

Prefer contiguous f64 vectors with precomputed mesh geometry and reusable diagonal/RHS workspaces as the production design candidate. Preserve a small, independently tested tridiagonal kernel. Evaluate `faer`/`ndarray` only against a demonstrated operation requirement; no crate/version/performance superiority is established by this research. Water boundary elimination and transport non-symmetric coefficients must remain distinguishable.

Define “zero allocations” narrowly and measurably: after initialization, accepted and retried numerical steps at fixed mesh/species size should allocate nothing in the core hot path. Setup, requested output copies, wrapper conversion, and errors may allocate under a separately documented policy. Count allocations in a scoped test, including retry and optional-process paths. Avoid growing output history in a step; preallocate requested outputs or stream at accepted-step boundaries. These are recommendations motivated by the prompt and the observed array reuse, not measured Rust performance.

## 8. Validation evidence required before implementation waves

The initial prompt's five scenarios are test requirements, not test decks already present in this workspace. The later planner must acquire or construct complete inputs and establish their expected results. Do not label a reconstructed scenario an official example without provenance.

Recommended test layers:

1. **Reference provenance and execution:** build the pinned Fortran copy, run each deck in its own output directory with timeout, capture stdout/stderr and completion diagnostics, reject partial or non-converged runs even if process exit looks successful. Record all hashes, compiler flags and applicable feature flags.
2. **Component parity:** instrument small drivers around `FK/FC/FQ/FH`, `GenMat/SetMat`, `Reset/Gauss`, `Shift`, `TmCont`, uptake functions and balances. Compare intermediate arrays before relying on end-to-end rounded text.
3. **Scenario parity:** dry infiltration, capillary rise, cyclic rain/evaporation/transpiration, non-equilibrium reactive solute, and heat coupling from INITIAL_PROMPT. Add layered/nonuniform meshes, switches into/out of ponding, seepage activation, forcing discontinuities, final-time alignment, and retries.
4. **Independent physical checks:** hydrostatic equilibrium/zero net flux; closed-domain conservation; suitable analytical steady-flow and simplified transport cases; mesh/time refinement. Distinguish discretization error from porting error.
5. **Host contract tests:** same fixture through Rust/Python/R/selected JavaScript/C interfaces, two concurrent independent simulations, invalid inputs, structured failures, cancellation and resource release. Validate packaging on each selected OS/architecture.

Track `h`, water content, boundary fluxes, root uptake, cumulative storage/inflow/outflow, solute masses in all compartments, temperatures, solver iterations, retries and termination status. Use paired tolerances `abs(a-b) <= atol + rtol*max(abs(a),abs(b))` per variable and unit, plus an explicitly defined RMS or mesh-weighted L2 and L-infinity norm. Distinguish a dimensionless moisture tolerance from a head tolerance in the selected length unit. Handle near-zero mass-balance denominators with an absolute residual and documented normalization. Thresholds require baseline evidence and user acceptance; `1e-6` is a candidate from the prompt, not a verified achievable universal bound.

Compare at shared physical output times. Adaptive trajectories can diverge under precision changes; first diagnose event/branch/iteration traces, then assess state and conservation. Keep separate checks for legacy text layout and numerical values; elapsed time, timestamps, formatting width and rounded digits prevent useful unrestricted byte comparison. Scientific validation should use unrounded instrumentation where needed. This recommendation follows the mixed precision, formatted outputs and event alignment observed in [WATFLOW.FOR](../src_reference/WATFLOW.FOR), [OUTPUT.FOR](../src_reference/OUTPUT.FOR), and [TIME.FOR](../src_reference/TIME.FOR).

For curve properties, test physical parameter domains, branch boundaries, saturation and dry limits, and compare derivatives only where the selected model is differentiable. Test uptake reduction bounds and integrated demand limits under the correct stress/compensation mode; do not apply properties from a single Feddes curve blindly to every uptake model. Sources: [MATERIAL.FOR](../src_reference/MATERIAL.FOR), `FK/FC/FQ`; [SINK.FOR](../src_reference/SINK.FOR), `SetSnk/FAlfa/FSAlfa`.

Benchmark only after parity: separate initialization, steady stepping, output formatting, wrapper copying, and ensembles; report problem size, enabled physics, hardware, compiler, tolerances, steps and iteration counts. An f64 port and mixed-precision reference doing different numerical work cannot establish speedup from wall time alone.

## 9. Handoff to the plan-wave agent

Read this document together with **all of [INITIAL_PROMPT.md](INITIAL_PROMPT.md)**. The deliverable of the present task is research. The future agent should produce the requested `HYDRUS1D_RUST_PORT_PLAN.md` design blueprint and the numbered orchestration artifact set; do not silently substitute one for the other. Read the installed plan-wave and wave-orchestration skills when that task begins. The supplied orchestration contract uses `EXECUTION_PLAN_N.md`, `PROMPT_TEMPLATE_N.md`, `STATE_N.md`, and `tasks/N.W.id.todo.md`; it takes precedence over unnumbered examples in plan-wave.

### Requirement-to-planning map

| Requirement from INITIAL_PROMPT / current user | Evidence or correction | Planning consequence |
| --- | --- | --- |
| v4.17 public-domain Fortran baseline | Snapshot identity and rights need explicit provenance | Resolve source/version/license evidence; preserve originals and record oracle patches |
| Full source/call/state audit | Inventory and partial hierarchy above; COMMON confined to hysteresis | Expand to exact call/dependency and mutable-state inventory before scoped port tasks |
| Re-entrant, I/O-free, high-performance Rust | State ownership and foreign-runtime separation above | Define API, units, errors, ownership and workspace contracts before workers implement modules |
| Python/R/JavaScript/other frontends | Distinct native/Wasm/C options above | Decide actual shipped bindings and platform matrix; keep all on one core |
| Hydraulic models, nonlinear flow and boundary variants | Concrete material/assembly/switching anchors above | Track each model and boundary mode in a coverage matrix; basic boundaries precede water acceptance |
| Hysteresis, solute, heat and optional physics | Legacy branches exceed the six phase headings; dual permeability absent | Explicitly include, defer or reject each feature; no silent feature fallback |
| Legacy inputs and outputs | Parser/output code present, decks absent | Minimal parsing/fixture/output comparison belongs in baseline/early vertical slices, not only final polish |
| serde, JSON/YAML/TOML/GeoPackage/Arrow | Serialization and scientific data adapters are different work | Select minimum formats and record explicit deferrals for the others |
| Exact validation and 1e-6 examples | Mixed precision and event schedules affect parity | Calibrate per-field tolerances; separate text format from scientific equivalence |
| Diagrams, crate layout, API signatures, risk registry, phased tasks | Research includes candidates, not approved contracts | Include finalized versions in the blueprint and link task specs to those sections |
| Six-phase roadmap | Dependencies require early harness/I/O/basic BCs | Retain all requirements while reordering into testable vertical slices |

### Decisions the planner should ask, one at a time

Recommendations are starting points, not decisions made for the user:

1. **Compatibility target:** exact supplied snapshot, verified 4.17 release, or a defined scientific subset? Recommend a hash-pinned supplied snapshot plus documented 4.17 differences unless an authoritative 4.17 archive is obtained.
2. **First release feature coverage:** recommend single-domain water with basic hydraulic laws and required atmospheric/root behavior as the first accepted slice; retain solute and heat as planned work. Explicitly decide hysteresis, vapor/energy, dual porosity, drainage, density, snow, bacteria/colloid branches and inverse/calibration features.
3. **Precision and behavior contract:** recommend f64 production numerics with measured tolerances and strict failure reporting; decide whether mixed-precision/legacy failure compatibility is a separate test or runtime mode.
4. **Platforms and frontends:** recommend desktop Linux/macOS/Windows as native targets; decide architectures, browser versus Node requirements, binding priorities, and whether a public C ABI is required at launch.
5. **Data interchange and coupling:** recommend validated versioned configuration plus batched arrays and caller-controlled time synchronization; decide initial formats, unit policy, checkpoints, forcing interpolation and output contracts.
6. **Acceptance and performance:** choose per-field tolerance policy, independent benchmark bar, required platform tests and explicit allocation guarantees after oracle experiments.
7. **Execution mechanics:** repository initialization, worker/escalation models, concurrency, worktree policy and budget remain future plan-wave decisions. This workspace currently lacks Git metadata.

### Dependency guidance, not an approved wave schedule

A plausible dependency chain is provenance/build/fixtures → domain/API/error contracts and a minimal parser → hydraulic functions plus mesh → basic boundary assembly and linear solve → nonlinear flow, event scheduling, rollback and balance → atmospheric/root switching → optional physics and coupling → binding/package acceptance and performance. Prototype binding/build feasibility early after the API contract so browser or R constraints surface before the entire solver is finished.

Expose separate task scopes for primitive curves, interpolation tables, mesh metrics, linear kernels, boundary switches, time scheduler, rollback, balance accounting, and fixture comparison. “Port SOLUTE.FOR” is too large for one small-model task. Global Cargo manifests, feature registries, shared public types and STATE files are shared-write hazards even when implementation files differ. Let the orchestrator serialize shared-file integration and report appends. Each task needs existing source anchors, explicit allowed output paths, prerequisites, and a named targeted test command that actually exists by execution time. This research does not claim hypothetical cargo test targets already exist.

Before wave 1, establish a real baseline, Git/worktree conventions, and a reproducible oracle or explicitly mark the baseline blocked. Use parallel tasks only with disjoint file scopes; run global suites serially. Preserve the worker micro-plan approval and escalation rules from wave-orchestration. The research itself grants no implementation approval.

### Risk register for the blueprint

| Risk | Evidence | Required treatment |
| --- | --- | --- |
| Wrong source identity or redistribution assumptions | Header/archive/manual mismatch; source copyright header | Source manifest and rights evidence; separate scientific reconstruction from distribution claims |
| Oracle changes numerical behavior | MSFLIB/logical extensions; mixed precision | Minimal reviewed portability patch; no blanket REAL promotion; paired compiler experiments |
| Model advertised but absent locally | `MatIn` rejects dual permeability | Versioned capability matrix; explicit unsupported-feature errors |
| Silent state corruption on retry | `WatFlow` restores selected state and changes dt | Transactional accepted/trial state; forced-rejection tests covering coupled histories |
| Wrong sign/index/unit conventions | Bottom node 1/top node N; signed boundary codes and forcing | Conversion layer, typed boundary variants, nonuniform-mesh balance tests |
| False numerical confidence | Rounded outputs, mixed precision, adaptive branch sensitivity | Intermediate comparisons, per-variable tolerances, independent conservation/refinement checks |
| Oversimplified hysteresis | Three COMMON blocks and reversal histories | Own all history per simulation; cyclic and interleaved-instance tests |
| Unstable elimination or changed pivot policy | `Gauss` tiny-pivot replacement versus `BanSol` direct division | Near-singular tests and explicit solver policy; do not silently substitute an algorithm |
| Lost optional process coupling | Repeated water/heat loop and solute-dependent limits | Feature-combination matrix and staged coupling acceptance |
| Host crash or stale view | Foreign handles/buffers and runtime threading | Boundary validation, defined lifetime, typed errors, destruction and concurrency tests |
| Allocation/output growth defeats throughput | Fixed-array legacy versus dynamic API | Preallocated workspace and separately measured output/copy costs |
| Oversized or conflicting worker tasks | 2,105-line SOLUTE; common manifests and state | Small symbol-based scopes, dependency ordering and serialized shared writes |

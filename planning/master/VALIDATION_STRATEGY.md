# Validation and performance strategy

## Fortran reference harness

The Fortran reference executable is a separately identified copy of the pinned ten-file snapshot. Phase 0 preserves the originals, applies a reviewable GNU Fortran/Linux portability patch for the known `MSFLIB` and `iMetHour` blockers, and records source/archive, patch, compiler, flags, executable, host, and date. It does not silently promote precision or rewrite solute algorithms. The patched executable passed the explicitly noncanonical water smoke, so the runner/build path works; no canonical scientific solute run or expected output exists yet. See [Phase 0 evidence](PHASE_0_REFERENCE_EVIDENCE.md).

Each fixture must run in its own temporary output directory with immutable input copies, an explicit wall-clock timeout, captured stdout/stderr, exit status, expected completion marker (`CloseOutput`/`end` where applicable), file inventory, and no stale outputs. A manifest records fixture ID, input provenance, source and patch hashes, compiler/toolchain, flags, precision mode, platform, command, timeout, output hashes, and observed warnings. A run is incomplete if it times out, exits nonzero, omits required outputs, or writes malformed/truncated records. The current runner supports only the noncanonical smoke; solute-specific fixture execution, output validation, and comparison are planned under M5-V.

## Fixture matrix

| Fixture | Input provenance status | Observables and checks | Producer/prerequisite |
|---|---|---|---|
| Dry-column infiltration | 4.08 Chapter 11.1 parameters are documented; machine-readable deck absent | `h`, water content, surface flux, cumulative balance, accepted times and iterations | construct/recover deck; Fortran-reference bootstrap + water solver |
| Capillary rise / shallow water table | proposed independent fixture, not a claimed manual example | equilibrium gradient, transient heads, gravity/sign convention, balance and refinement | fixture design + mesh/water path |
| Cyclic atmospheric forcing with roots | manual Chapter 11.2 provenance; complete deck absent | atmospheric mode switches, ponding, root uptake/stress/compensation, `SubReg` balance and outputs | forcing/root adapter + water acceptance |
| Solute reaction/sorption | manual Chapters 11.3–11.5; repository decks and golden arrays absent | species concentration profiles, supported sorbed pools, boundary flux histories, species inventories and independent balance | M5 scientific-comparison catch-up; Example 3 is the first candidate, subject to scope/input recovery |
| Coupled heat/vapor | manual Chapter 11.11; deck absent | temperature, vapor/energy terms, water/heat coupling order and balance | later heat/vapor milestone |
| Hydrostatic equilibrium | proposed analytic fixture | residual gradient, flux≈0, conservation under no forcing | mesh/hydraulic core |
| Layered/inclined mesh | proposed fixture; source anchors `CosAlf`, layers | interface flux continuity, orientation/sign, refinement | mesh metrics + boundary contract |
| Event/switch/retry | proposed forcing with discontinuities and forced difficult steps | target-time alignment, boundary mode, iteration count, retry `dt`, rollback equality | scheduler/rollback implementation |
| Conservation/refinement | proposed manufactured or analytical fixture | storage/inflow/outflow/sink closure under space/time refinement | balance implementation |
| Hysteresis | manual Chapter 11.10; deck absent | reversal history, drainage/imbibition path, accepted/rejected trial state | later hysteresis milestone |
| Dual porosity | manual/source branch evidence; deck absent | mobile/immobile water and transfer mass, coupling and rollback | later dual-porosity milestone |
| Host equivalence | same canonical fixture through each selected adapter | accepted state, diagnostics, errors, cancellation/resource release | API and each host binding |

Input recovery or construction must preserve the source of every value: manual page/section, supplied artifact hash, or clearly labeled independent design. A reproduced chart is not a golden output. Expected results remain unknown until the producer milestone creates and reviews them.

## M5 solute scientific-comparison gate

M5's contract and invariant tests do not complete its scientific validation. The next planning session after Orchestration 11 is a dedicated M5 comparison catch-up before Phase 6 heat/temperature planning. Use [SCIENTIFIC_COMPARISON_RESEARCH.md](SCIENTIFIC_COMPARISON_RESEARCH.md) as the evidence and fixture-selection brief. The first candidate is 4.08 manual Example 3 (sequential decay) because its process maps most directly to the selected Rust chain and the manual reports an analytical CHAIN comparison; its incomplete machine input and missing output arrays must be recovered or replaced with a clearly labeled derived case.

The catch-up must establish a complete, provenance-reviewed fixture; solute-specific reference execution with output inventory/completion checks; normalized Rust/reference observables at matching space and time points; independent inventory/balance checks and an analytical cross-check where feasible; and reviewed field-specific tolerances. Preserve source-snapshot parity, independent analytical/physical validation, text-layout compatibility, and release compatibility as separate evidence claims. If any prerequisite remains unavailable, report the blocker and keep the M5 scientific gate open. Do not use the noncanonical Phase 0 smoke as an M5 baseline.

## Component and property tests

Future test files should include `tests/hydraulic_curves.rs`, `tests/root_uptake.rs`, `tests/mesh_equilibrium.rs`, `tests/time_events.rs`, and `tests/fixture_parity.rs`, plus `benches/solver.rs`; names are proposed. Test `FK`, `FC`, `FQ`, `FH`, `FAlfa`, table interpolation and derivatives across finite physical domains, saturated limits, dry limits, selector rejection, monotonicity where mathematically required, and NaN/overflow boundaries. Proptest domains must constrain positive conductivity/length, valid material parameters, ordered nodes and finite heads while including boundary-adjacent values; generated cases must never treat invalid input as a passing physical result.

Property checks should cover hydrostatic equilibrium, conservation, refinement trends, root uptake bounds, and rollback identity. Compare analytical derivatives to finite differences only where the function is differentiable and use a separately recorded kink policy. `Gauss` and `BanSol` require pivot, boundary-row, singular/near-singular, and residual checks against independently assembled small systems.

## Tolerance policy

Report per-variable units before comparison: pressure head and coordinates in the configured length unit, water content dimensionless, flux and cumulative water in configured length/time and length, concentration and sorbed mass in their declared units, temperature in degrees, and relative balance as dimensionless. For each scalar/vector field use paired acceptance `abs(a-b) <= atol + rtol * max(abs(a), abs(b))`, plus normalized L-infinity, RMS, and mesh-weighted L2 where applicable. State vector normalization and node weights explicitly; do not compare rounded text as if it were full precision.

Use absolute residual criteria when a mass-balance denominator is zero or near zero, and record the denominator and normalization branch. Compare accepted times, event times, boundary modes, iteration counts, retries, timestep history, termination status, and balances as diagnostics even when physical-result tolerances permit trajectory differences. The prompt's `1e-6` is a calibration candidate, not a universal or achieved threshold. Calibrate per field after a real Fortran-reference executable and fixture baseline, including mixed-precision versus f64 compiler comparisons.

Text-layout checks are separate: verify filenames, headers, column count, delimiters, units, end markers, line endings, numeric width/precision, and deterministic ordering. A text mismatch does not by itself prove scientific mismatch; a text match does not prove numerical correctness.

## Host and platform tests

Run the same in-memory fixture through the core API and each selected Python/R/JavaScript/Wasm/C host only after those adapters are approved. Compare state and diagnostics at accepted event boundaries, invalid-input error classes, cancellation, resource release, serialization round trips, and output encoding. Interleave two independent simulations and run them concurrently to detect shared COMMON/static state, accidental global caches, data races, and cross-instance contamination. Matrix platform/compiler claims only after a release host decision; record OS, architecture, compiler, optimization, and math/runtime details.

## Allocation and performance

Allocation counting is a future implementation test, not an assertion about the current repository. Instrument the inner time/Picard loop and verify zero allocations for fixed-size stepping, while separately measuring setup, retries, output, error construction, and dynamic resizing. A failing allocation budget must report call site and phase. Criterion experiments should benchmark equivalent numerical work: same mesh, physics, forcing, accepted steps, iteration limits, output disabled/enabled as separate cases, and recorded iteration/retry counts. Compare against the patched Fortran reference executable only after parity is established, separating initialization, stepping, formatting, wrapper copies, and ensemble throughput. Report distributions, hardware, compiler flags, precision, and problem size; claim no speedup until workloads and results are comparable.

## Future commands and prerequisites

The following are proposed commands, not executable checks today:

| Command | Purpose | Prerequisite |
|---|---|---|
| `python3 tools/run_reference.py --fixture <id> --manifest <path>` | isolated Fortran-reference run and provenance capture | Fortran-reference bootstrap, tool script and fixture |
| `cargo test -p hydrus-core --test hydraulic_curves` | curve/property tests | Rust core and test files |
| `cargo test -p hydrus-core --test fixture_parity -- --nocapture` | paired reference comparison | reference outputs, parser and tolerance policy |
| `cargo test -p hydrus-core --test state_isolation` | interleaved/threaded independence | re-entrant core and host decision |
| `cargo bench -p hydrus-core --bench solver` | Criterion performance | benchmark harness and parity-approved case |
| `python3 tools/check_outputs.py --layout ...` | text-layout validation | canonical writer and expected schema |

The current document structural check validates documentation only; it is not numerical validation.

## Acceptance gates

1. Provenance gate: immutable source/archive hashes, artifact notices, patch manifest, and reproducible Fortran-reference command exist.
2. Fixture gate: each required scenario has a reviewed input source, expected observables, and isolated run contract; missing decks remain explicitly blocked.
3. Water gate: single-domain water, atmosphere, root uptake, mesh, events, rollback, and balance pass independent and Fortran-reference comparisons after tolerance calibration.
4. Later-physics gates: solute/heat, hysteresis, and required dual porosity each receive separate fixtures and coupling/rollback evidence; no feature is inferred from inventory alone.
5. Host gate: selected bindings pass equivalence, invalid-input, concurrency, cancellation, and resource tests.
6. Performance gate: allocation and benchmark results use comparable numerical work and are reported with reproducible environment metadata.

Recommendations become implementation commitments only after the corresponding gate, fixture provenance, and user scope decision are recorded. Manual/platform verification remains distinct from source-snapshot parity and does not establish release identity.

## Evidence

- `SOURCE_AUDIT.md` records ten immutable source files, the failed syntax baseline, output/source anchors, and no numerical results.
- `STATE_AND_CALLS.md` records accepted/trial state, rollback obligations, mixed precision, and output lifecycle.
- `FEATURE_MATRIX.md` records first acceptance, later solute/heat/hysteresis/dual-porosity ordering, and optional scope gates.
- `HYDRUS_RESEARCH.md` §§4 and 8 identify manual Chapter 11 candidates, missing machine-readable decks, per-field tolerance recommendations, host tests, and performance separation.
- Direct anchors inspected: `OUTPUT.FOR:TLInf/NodOut/SubReg`, `WATFLOW.FOR:Gauss`, `SOLUTE.FOR:BanSol`, `MATERIAL.FOR:FK/FC/FQ`, and `SINK.FOR:FAlfa`.
- The local manual is evidence for documented cases; no golden output, passing numerical comparison, universal tolerance, or benchmark speedup is claimed.

## Open decisions

1. Who owns construction/recovery and review of missing decks and expected results, and which independent fixtures are acceptable?
2. Which compiler, precision modes, operating systems, and host bindings are release targets?
3. What per-field tolerances and mass-balance zero-denominator policy are accepted after Fortran-reference calibration?
4. Which optional physics and output formats receive performance and host-equivalence commitments?
5. Which exact patched-reference semantics are permitted for MSFLIB and the `iMetHour` blocker?

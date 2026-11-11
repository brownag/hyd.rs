# hyd.rs

**One-dimensional water flow and reactive solute transport in Rust.**

hyd.rs implements selected HYDRUS numerical models against a pinned Fortran
source snapshot. The core library, [`hydrus-core`](crates/hydrus-core), uses owned
state and rolls back rejected time steps. Python tools run reference comparisons
and convergence studies.

The project checks both agreement with the source and accuracy against independent
analytical solutions. Matching Fortran can reproduce its defects, so both checks matter.

**Status:** active development. The core library and verification tooling exist;
scientific acceptance of the default solute backend remains open.

[Getting started](#getting-started) · [Numerical approach](#numerical-approach) ·
[Validation status](#validation-status) · [Contributing](CONTRIBUTING.md)

## What is here

- Hydraulic models, one-dimensional meshes, water flow, atmospheric forcing,
  boundary conditions, and root uptake.
- Solute advection–dispersion, sequential reactions, equilibrium and kinetic
  sorption, and species-level accounting.
- Accepted water snapshots for coupling water flow to solute transport.
- Reference comparisons and mesh/time refinement with error budgets and hashed
  run records.

## Getting started

Install Rust and Cargo. From the repository root:

```sh
cargo build --workspace --locked
cargo test --workspace --no-fail-fast
cargo doc -p hydrus-core --no-deps --open
```

The crate uses Rust edition 2021. Python 3 is used for the verification tools;
GNU Fortran is needed for reference builds and Fortran comparison workflows.
The recorded reference campaigns use GNU Fortran 13.3.0 on Linux x86_64.

### Explore the API

Start with the [public exports](crates/hydrus-core/src/lib.rs) and
[integration tests](crates/hydrus-core/tests). For a small example of backend selection:

```sh
cargo test -p hydrus-core --test solute_solver_selection
```

## Numerical approach

| Backend | Purpose |
|---|---|
| `SoluteBackend::FortranCompatible` **(default)** | Reproduce the pinned source's numerical behavior, including operation order and mixed precision. |
| `SoluteBackend::FiniteVolumeBackwardEuler` | A selectable finite-volume, backward-Euler formulation. |

The default follows the successful Rust-versus-Fortran parity campaign described in
[ADR 0018](docs/decisions/0018-default-to-the-fortran-compatible-solute-backend.md).
The compatibility backend retains source single-precision arithmetic where required;
Rust API values represented as `f64` do not imply an all-f64 solver.

The [source-defect register](docs/FORTRAN_SOURCE_DEFECTS.md) records known defects
and the behavior preserved for compatibility.

## Validation status

The Orchestration 21 campaign passed all 44 Rust-versus-Fortran parity fixtures on
its documented compiler and platform. Independent accuracy checks remain incomplete.

The Orchestration 23 scientific-validation campaign produced six verified batches:

| Study | Recorded outcome |
|---|---|
| Diffusion, spatial refinement | 12 achieved concentration findings; remaining fields inconclusive |
| Robin boundary problem, spatial refinement | 40 failed findings with non-monotone refinement |
| Temporal refinement | Inconclusive: the time groups contain only two study levels |
| Example 3 reaction chain | Inconclusive: control and uncertainty/floor limitations |

The solute scientific-comparison milestone (**M5-V**) remains **partial/open**.
No reserved holdout row was run in that campaign, and the heat milestone (**M6**)
has not been unblocked. See the [campaign report](planning/master/M5V_FINAL_23.md)
for the exact claims and the [next-step review](planning/master/M5V_REVIEW_23_NEXT_STEPS.md)
for interpretation, limitations, and recommended diagnostics.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for checks, code entry points, and the
numerical-change workflow. [AGENTS.md](AGENTS.md) gives agents a workspace map.

## Documentation

- [Current goals](planning/GOALS.md).
- [Roadmap](planning/master/ROADMAP_AND_RISKS.md).
- [Architecture decisions](docs/decisions/README.md).
- [Water acceptance evidence](planning/master/PHASE_4_WATER_ACCEPTANCE_EVIDENCE.md).
- [Solute acceptance evidence](planning/master/PHASE_5_SOLUTE_EVIDENCE.md).
- [Solute verification specification](planning/master/SOLUTE_VERIFICATION_SPEC.md).
- [Validation strategy](planning/master/VALIDATION_STRATEGY.md).

Planning documents retain historical results. Use their dated campaign and
supersession sections when interpreting older claims.

## License and provenance

The repository includes the [GNU General Public License, version 3](LICENSE.md).
Licensing and redistribution of the pinned Fortran snapshot remain separate,
documented questions. See the
[source audit](planning/master/SOURCE_AUDIT.md) and
[reference harness notes](reference/fortran_reference/README.md).

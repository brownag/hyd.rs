# hyd.rs

**One-dimensional water flow and reactive solute transport in Rust, with traceable numerical evidence.**

hyd.rs is a Rust implementation of selected HYDRUS numerical models, developed
against a pinned Fortran source snapshot. It brings explicit state ownership,
transactional time stepping, and testable numerical interfaces to soil-water and
solute simulation.

The project asks two separate questions: **does the implementation reproduce the
reference source, and does it accurately solve the intended equations?** Source
comparisons, independent analytical solutions, convergence studies, and documented
error budgets provide distinct answers.

**Status:** active development. The core library and verification tooling exist;
scientific acceptance of the default solute backend remains open.

[Getting started](#getting-started) · [Numerical approach](#numerical-approach) ·
[Validation status](#validation-status) · [Documentation](#documentation)

## What is here

- **Hydraulics and water flow:** hydraulic constitutive models, one-dimensional
  meshes, water-step state and workspaces, atmospheric forcing, lower-boundary
  conditions, and root-uptake models.
- **Solute transport:** advection–dispersion, sequential species reactions,
  equilibrium and kinetic sorption, configurable endpoint conditions, and
  species-level accounting.
- **Transactional stepping:** accepted and trial states, rollback on rejected
  steps, and explicit coupling through accepted water snapshots.
- **Verification infrastructure:** pinned-source comparisons, independent
  analytical references, mesh/time refinement, field-specific uncertainty budgets,
  and hashed run manifests.

The Cargo workspace currently contains one library, [`hydrus-core`](crates/hydrus-core).
Python tools manage reference execution and evidence. The roadmap tracks broader
physics, host bindings, packaging, and release work separately.

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

Start with the [public exports](crates/hydrus-core/src/lib.rs) and executable
examples in the integration tests:

| Topic | Entry point |
|---|---|
| Hydraulic models and meshes | [`hydraulics`](crates/hydrus-core/src/hydraulics/mod.rs) |
| Water stepping | [`water_solver.rs`](crates/hydrus-core/src/water_solver.rs) |
| Atmospheric forcing and events | [`atmospheric.rs`](crates/hydrus-core/src/atmospheric.rs), [`forcing.rs`](crates/hydrus-core/src/forcing.rs) |
| Solute configuration and transport | [`solute_contract.rs`](crates/hydrus-core/tests/solute_contract.rs), [`solute_transport.rs`](crates/hydrus-core/tests/solute_transport.rs) |
| Selecting a solute backend | [`solute_solver_selection.rs`](crates/hydrus-core/tests/solute_solver_selection.rs) |
| Accepted/trial state behavior | [`solute_transaction.rs`](crates/hydrus-core/tests/solute_transaction.rs) |

Run a focused example suite with:

```sh
cargo test -p hydrus-core --test solute_solver_selection
```

These tests demonstrate API contracts and specific numerical behavior; their
assertions are not a blanket scientific-accuracy guarantee.

## Numerical approach

### Two explicit solute backends

| Backend | Purpose |
|---|---|
| `SoluteBackend::FortranCompatible` **(default)** | Reproduce the pinned source's numerical behavior, including operation order and mixed precision. |
| `SoluteBackend::FiniteVolumeBackwardEuler` | An explicitly selectable finite-volume, backward-Euler formulation with its own numerical contract. |

The default follows the successful Rust-versus-Fortran parity campaign described in
[ADR 0018](docs/decisions/0018-default-to-the-fortran-compatible-solute-backend.md).
The compatibility backend retains source single-precision arithmetic where required;
Rust API values represented as `f64` do not imply an all-f64 solver.

### Evidence is part of the implementation

Reference inputs, tool versions, outputs, and classifications are linked through
hashes and manifests. Independent analytical references are defined separately from
solver output. Known source defects are recorded in the
[defect register](docs/FORTRAN_SOURCE_DEFECTS.md), with compatibility behavior kept
distinct from proposed mathematical improvements.

The guiding decisions are [source fidelity](docs/decisions/0001-treat-pinned-fortran-snapshot-as-solute-reference.md),
[separate numerical backends](docs/decisions/0002-keep-fvbe-and-source-compatible-backends-separate.md),
and [independent mathematical verification](docs/decisions/0009-verify-solver-mathematics-against-independent-references.md).

## Validation status

**Source parity and scientific accuracy are separate milestones.** The recorded
Orchestration 21 campaign passed all 44 Rust-versus-Fortran parity fixtures on its
documented compiler/platform. That establishes agreement with the pinned source for
those fixtures, including behavior inherited from the source.

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

## Repository guide

```text
crates/hydrus-core/   Rust library and integration tests
tools/               Reference runners, metrics, evidence checks, and Python tests
reference/           Casebooks, fixtures, and reference-build configuration
src_reference/       Pinned Fortran source snapshot
docs/                Architecture decisions and source-defect register
planning/            Designs, milestone evidence, execution records, and current goals
.development/        Local, untracked run artifacts and scratch evidence
```

## Development

Routine Rust checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```

The Python tooling has its own test suite:

```sh
python3 -m unittest discover -s tools -p 'test_*.py'
```

For the source staging and compiler workflow, follow the
[Fortran reference harness guide](reference/fortran_reference/README.md).
Full scientific campaigns have separate activation, provenance, and review steps;
they are not part of the build quickstart.

When contributing a numerical change, identify the affected backend and the
evidence needed to support it. Preserve the pinned reference source and existing
run artifacts. Changes to casebooks or evidence-producing tools can invalidate
campaign pins, so account for re-verification and rerun costs in the work plan.

## Documentation

- [Current goals](planning/GOALS.md) — active priorities and deferred work.
- [Roadmap](planning/master/ROADMAP_AND_RISKS.md) — milestone dependencies and scope.
- [Architecture decisions](docs/decisions/README.md) — design choices and rationale.
- [Water acceptance evidence](planning/master/PHASE_4_WATER_ACCEPTANCE_EVIDENCE.md).
- [Solute acceptance evidence](planning/master/PHASE_5_SOLUTE_EVIDENCE.md).
- [Solute verification specification](planning/master/SOLUTE_VERIFICATION_SPEC.md).
- [Validation strategy](planning/master/VALIDATION_STRATEGY.md).

Planning documents retain historical results. Use their dated campaign and
supersession sections when interpreting older claims.

## License and provenance

The repository includes the [GNU General Public License, version 3](LICENSE.md).
Reference artifacts have their own provenance records; the project license does
not resolve the separately documented licensing and redistribution questions for
the pinned Fortran snapshot. See the
[source audit](planning/master/SOURCE_AUDIT.md) and
[reference harness notes](reference/fortran_reference/README.md).

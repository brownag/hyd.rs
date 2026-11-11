# hyd.rs agent guide

## Quick Start

Run from the repository root:

```sh
git status --short
cargo build --workspace --locked
cargo test --workspace --no-fail-fast
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
python3 -m unittest discover -s tools -p 'test_*.py'
```

Focused checks and API docs:

```sh
cargo test -p hydrus-core --test solute_solver_selection
python3 -m unittest tools.test_solute_convergence
cargo doc -p hydrus-core --no-deps --open
```

## Find Things

| Path | Use |
|---|---|
| `Cargo.toml`, `Cargo.lock` | Workspace members and dependency versions |
| `crates/hydrus-core/src/lib.rs` | Public Rust exports |
| `crates/hydrus-core/src/hydraulics/` | Hydraulic models, meshes, and stencils |
| `crates/hydrus-core/src/water_solver.rs`, `water_state.rs` | Water stepping and state |
| `crates/hydrus-core/src/solute.rs` | Solute API and backend dispatch |
| `crates/hydrus-core/src/solute_fortran_compat/` | Source-compatible kernels and adapters |
| `crates/hydrus-core/tests/` | Integration tests and verification harnesses |
| `tools/` | Python runners, metrics, evidence checks, and tests |
| `reference/` | Casebooks, fixtures, and reference-build configuration |
| `src_reference/` | Pinned Fortran source |
| `docs/decisions/` | Architecture decisions |
| `planning/master/` | Specifications, designs, and evidence reports |
| `planning/` | Goals, execution plans, task records, and state boards |
| `.development/` | Untracked run artifacts and scratch evidence |

## Workflows

- Inspect repository changes before editing. Preserve work from other tasks.
- Find the governing plan and matching state record through `planning/GOALS.md`
  and `planning/EXECUTION_PLAN_*.md`. Read dated updates and supersession notes.
- Follow [CONTRIBUTING.md](CONTRIBUTING.md) for numerical changes and checks.
  Record results in the task report and state board, not in this file.
- For source-reference work, follow the
  [reference harness guide](reference/fortran_reference/README.md). Stage a copy;
  preserve the pinned source and existing evidence.
- Before changing cases or producer tools, inspect recipe hash bindings and the
  campaign's rerun requirements. Follow its bounds and threshold gates for holdouts.

## Infrastructure

- Rust edition and dependencies live in `crates/hydrus-core/Cargo.toml`.
- Python tools use Python 3. Reference compilation uses GNU Fortran.
- Cargo writes builds to `target/` and API docs to `target/doc/`.
- Reference workflows write manifests and outputs under `.development/`.
- `reference/solute_verification/` holds verification cases and threshold files.
  Run approvals and bounds are governed by campaign records.
- Solute backend contracts differ. Read `solute.rs` and the backend decisions
  before changing equations, operation order, or precision.

## References

- [Project overview](README.md)
- [Contributor workflow](CONTRIBUTING.md)
- [Architecture decisions](docs/decisions/README.md)
- [Source-defect register](docs/FORTRAN_SOURCE_DEFECTS.md)
- [Solute verification specification](planning/master/SOLUTE_VERIFICATION_SPEC.md)
- [Validation strategy](planning/master/VALIDATION_STRATEGY.md)
- [Roadmap](planning/master/ROADMAP_AND_RISKS.md)

# Contributing

## Build and check

Use Rust and Cargo for the library, Python 3 for verification tools, and GNU
Fortran for source-reference workflows. Run commands from the repository root.

```sh
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
python3 -m unittest discover -s tools -p 'test_*.py'
```

For focused checks:

```sh
cargo test -p hydrus-core --test solute_solver_selection
python3 -m unittest tools.test_solute_convergence
```

Choose checks that exercise the changed behavior. Report the commands and results,
including checks you could not complete. Documentation-only edits need link and
content checks rather than solver runs.

## Find the code

| Area | Entry point |
|---|---|
| Public API | [`lib.rs`](crates/hydrus-core/src/lib.rs) |
| Hydraulic models and meshes | [`hydraulics/`](crates/hydrus-core/src/hydraulics) |
| Water stepping and state | [`water_solver.rs`](crates/hydrus-core/src/water_solver.rs), [`water_state.rs`](crates/hydrus-core/src/water_state.rs) |
| Forcing | [`atmospheric.rs`](crates/hydrus-core/src/atmospheric.rs), [`forcing.rs`](crates/hydrus-core/src/forcing.rs) |
| Solute API and backend selection | [`solute.rs`](crates/hydrus-core/src/solute.rs) |
| Source-compatible kernels | [`solute_fortran_compat/`](crates/hydrus-core/src/solute_fortran_compat) |
| API examples and regressions | [`tests/`](crates/hydrus-core/tests) |
| Reference runners and Python tests | [`tools/`](tools) |

The solute contract, transport, backend-selection, and transaction tests show how
to construct inputs and inspect results. Read neighboring code before adding a
new pattern or dependency.

## Plan a numerical change

1. Read the relevant [architecture decisions](docs/decisions/README.md) and
   [verification specification](planning/master/SOLUTE_VERIFICATION_SPEC.md).
   Identify the backend and the claim the change should support.
2. Record scope, checks, and required decisions in the task plan. For campaign
   work, use the governing `planning/EXECUTION_PLAN_*.md`, task records, and
   matching `planning/STATE_*.md`.
3. Add evidence that distinguishes the proposed cause from competing explanations.
   Keep diagnostic variants separate from production numerical changes.
4. Record results and limits in the task report. Update the state record and
   affected documentation when the work is accepted.

Source parity, analytical accuracy, and conservation checks support different
claims. Preserve source operation order and precision in compatibility code.
Consult the [source-defect register](docs/FORTRAN_SOURCE_DEFECTS.md) before changing
behavior inherited from Fortran. Do not widen a tolerance to turn a failure into a pass.

## Work with reference evidence

Follow the [Fortran reference harness guide](reference/fortran_reference/README.md)
to stage and build a patched copy. Keep `src_reference/` unchanged.

Runs belong in isolated roots under `.development/`. Preserve existing outputs,
including failed attempts, and record inputs, commands, toolchain, and hashes.
The [evidence lifecycle decision](docs/decisions/0003-isolate-and-authenticate-reference-and-verification-runs.md)
defines the full protocol.

Casebook and producer-tool edits can invalidate campaign pins. Check the affected
recipes and budget fresh runs before changing them. Bounds review and threshold
approval are separate gates; follow the governing campaign plan before launching
reserved holdouts. A successful build or unit test does not grant run approval.

## Write documentation

Keep commands and paths concrete. Link to the source of a rule instead of copying
it into several files. Mark historical results and superseded claims by campaign.

Use `AGENTS.md` for stable navigation and commands. Put task status in planning
records, evidence in reports, and enduring design decisions in `docs/decisions/`.

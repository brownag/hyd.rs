# Context

Domain glossary for hyd.rs. Paths are under `crates/hydrus-core/src/` unless noted. Fortran anchors point into the pinned `src_reference/`. A term marked "planned" names a type that does not exist yet and the Orchestration 19 task that creates it. See `docs/decisions/` for the decisions behind these terms.

**water step**: One transactional advance of the water-flow solution over a time step. `advance_water_step` (`water_solver.rs`) takes a `WaterStepInput` and works in a `WaterWorkspace` (`water_state.rs`). The Fortran counterpart is `WatFlow` (`WATFLOW.FOR:3`), called at `HYDRUS.FOR:419`.

**upper boundary**: The top-of-profile condition of a water step: prescribed flux, prescribed head, or atmospheric. `UpperBoundaryMode` and `advance_atmospheric_step` live in `atmospheric.rs`. In atmospheric mode the surface head is bounded by `hCritA`, which `INPUT.FOR:102-103` sets to `-abs(1e10)`. The compatibility request carries it as an explicit primitive (`types.rs`), never a default. Task 19.5.3 folds the upper boundary into one `WaterStepInput` value.

**lower boundary mode**: The bottom-of-profile condition: fixed head, prescribed flux, free drainage, or an unsupported source mode. `LowerBoundaryMode` in `lower_boundary.rs`. The Fortran code selects it with `KodBot`, read at `INPUT.FOR:56` and `:61`. Task 19.4.2 folds it into one `WaterStepInput` value.

**accepted water snapshot**: The water state a solute step may rely on, after a water step is accepted. `AcceptedWaterSnapshot` exists today in `solute.rs`. Planned: the water step publishes it (task 19.6.2), which also moves it out of `solute.rs`. No Fortran anchor.

**solute step**: One advance of the solute solution over a time step. `advance_solute_step` and `advance_solute_step_with_processes` (`solute.rs`) run the FV/BE solver. The Fortran counterpart is `Solute` (`SOLUTE.FOR:6`).

**solute backend**: The numerical contract a solute step follows: FV/BE or source-compatible (ADR 0002, ADR 0013). Planned: a closed `SoluteBackend` enum selected on `SoluteConfig` (task 19.7.1), with `FortranCompatible` added by task 18.12.1. No Fortran anchor.

**compatibility request**: The normalized, hashed JSON request that drives the source-compatible backend (ADR 0004). `parse_request` and `parse_case_manifest` in `solute_fortran_compat/adapter.rs` decode it into `RequestFields` (`solute_fortran_compat/types.rs`). The module is private to the crate (`lib.rs`). Task 19.3.1 splits the adapter into submodules, and task 19.4.1 starts decoding with serde (ADR 0016).

**kernel primitive state**: The raw per-kernel inputs a compatibility kernel needs, carried as request fields and validated by the kernel's own `validate` (ADR 0014). Examples are the endpoint primitives (`boundary.rs`), `DualPorPrimitiveState` (`dual_domain.rs`), and `RootUptakePrimitiveState` and `SizeExclusionPrimitiveState` (`source_coupling.rs`, added in 18.7.8).

**source switches**: The Fortran control flags that gate source behavior, held together as `SourceSwitches` (`solute_fortran_compat/types.rs`). Among them `root_sink` is `SinkF` and `water_flow` is `lWat`, both read at `INPUT.FOR:31`.

**fixed-form continuation**: In the pinned fixed-form Fortran, a character in physical column 6 marks a continuation line, not a comment (ADR 0007). For example `INPUT.FOR:32` continues the control-record read of `INPUT.FOR:31`. Reachability audits read columns, not `!` alone.

# Phase 0 reference evidence

This handoff records the reviewed Phase 0 evidence required by the sequential master plan. It is provenance for a narrowly scoped GNU Fortran/Linux executable, not a scientific validation result.

## Reference executable

The executable is a separately staged copy of the immutable ten-file `src_reference/` snapshot, not a modification of the checked-in source. The authoritative [source manifest](SOURCE_MANIFEST.json) pins every source file and `planning/H1D_Src.zip`; the [source audit](SOURCE_AUDIT.md) records the source inventory and original GNU Fortran blockers. The [harness documentation](../../reference/fortran_reference/README.md) records that staging, generated source, compatibility shim, executable, manifests, logs, and outputs are confined to untracked `.development/reference-build/`.

The reviewed smoke record identifies executable SHA-256 `c012c4d2c76107fd189229547c6583112d95e3535e0abcfe52f8f6edc33f3a1a`, GNU Fortran 13.3.0, Linux/x86_64, and `-std=legacy -ffixed-line-length-none`; see [SMOKE_RUN.md](../../reference/fortran_reference/SMOKE_RUN.md#hashes) and its [environment](../../reference/fortran_reference/SMOKE_RUN.md#environment).

## Patch and build

The reviewed [portability patch](../../reference/fortran_reference/0001-gfortran-portability.patch) is applied only after source and archive verification to the staged copy. Its SHA-256 is `622d1ea68285257e98a169f94dec83f3df1c671a9366f651e845806cb8c00d7e`. The [patch notes](../../reference/fortran_reference/PATCH_NOTES.md#source-identity) identify the pinned archive SHA-256 `d0c47cbcb7a298ab16c6e08010bd7ca048d5527ce2f989a749390c0c55e848cf`, affected sources, intended GNU/Linux compiler flags, and behavioral boundary.

The stage and build commands are [documented](../../reference/fortran_reference/README.md#commands) and reviewed in the [2.2.1 state report](../STATE_2.md#221): staging verifies source/archive hashes and archive-member byte equality, applies the patch to the copy, and writes `staging-manifest.json`; building writes `hydrus-reference` and `build-manifest.json` with compiler, host, UTC, command, flags, hash, and artifact-notice status. The original `src_reference/` remains immutable.

## Smoke run

The reviewed [noncanonical smoke record](../../reference/fortran_reference/SMOKE_RUN.md) records a zero exit within the requested 60-second timeout, required output inventory hashes, hashes for the four copied inputs and stdout/stderr captures, and completion markers: legacy `end` markers in six outputs plus `BALANCE.OUT`'s final `Calculation time` record. The [runner](../../tools/run_fortran_reference.py) stages and builds the copy, creates a fresh isolated run directory, copies inputs byte-for-byte, captures stdout/stderr, enforces the timeout, rejects incomplete evidence, and hashes outputs.

The four [smoke inputs](../../reference/fortran_reference/smoke/) are independently constructed, minimal, and explicitly noncanonical. As their [README](../../reference/fortran_reference/smoke/README.md) and the run record state, this smoke case is not a scientific fixture, baseline, parity result, tolerance source, numerical validation, or release evidence.

## Exit checklist

- [x] **Source and archive identity:** [SOURCE_MANIFEST.json](SOURCE_MANIFEST.json) pins the archive and all ten immutable source hashes; [PATCH_NOTES.md](../../reference/fortran_reference/PATCH_NOTES.md#source-identity) identifies the patch inputs and archive hash.
- [x] **Reviewed patch and build provenance:** [patch notes](../../reference/fortran_reference/PATCH_NOTES.md), [harness commands and manifest boundary](../../reference/fortran_reference/README.md), and the [2.2.1 report](../STATE_2.md#221) record copied-tree patching, byte-match verification, compiler flags, manifests, and executable provenance.
- [x] **Executable evidence:** [SMOKE_RUN.md hashes](../../reference/fortran_reference/SMOKE_RUN.md#hashes) link the reviewed executable SHA-256 to the patch SHA-256.
- [x] **Input and output evidence:** [input hashes](../../reference/fortran_reference/SMOKE_RUN.md#inputs), [output hashes](../../reference/fortran_reference/SMOKE_RUN.md#outputs), and [input labels](../../reference/fortran_reference/smoke/README.md) record immutable copied inputs and isolated generated outputs.
- [x] **Environment evidence:** [SMOKE_RUN.md environment](../../reference/fortran_reference/SMOKE_RUN.md#environment) records GNU Fortran, OS, architecture, flags, and Python version.
- [x] **Reviewed run policy:** [VALIDATION_STRATEGY.md](VALIDATION_STRATEGY.md#fortran-reference-harness) requires isolated output directories, immutable input copies, timeout, captured streams, exit/completion checks, inventory, hashes, warnings, and malformed/truncated-record rejection; the [2.3.1 report](../STATE_2.md#231) records that the implementation passed its command.
- [x] **Completion evidence:** [SMOKE_RUN.md exit status and markers](../../reference/fortran_reference/SMOKE_RUN.md#completion-evidence) records zero exit, timeout bound, and required completion records.

## Limitations

The executable evidence applies only to the stated GNU Fortran/Linux environment. The retained legacy `NARGS`, `getdat`, and `gettim` calls, backslash path semantics, and absent GNU replacement for the legacy Intel signal handlers remain deferred; see [patch limitations](../../reference/fortran_reference/PATCH_NOTES.md#open-limitations). The smoke record retains an `IEEE_OVERFLOW_FLAG` warning.

Artifact notice/licensing and redistribution approval remain unreviewed. The supplied snapshot is described as HYDRUS-1D 4.08 or a derivative, but its exact release identity is unverified. No canonical fixture, expected scientific output, parity comparison, tolerance calibration, physical-correctness result, performance result, or cross-platform/cross-compiler claim follows from this work. The Phase 0 smoke case is specifically not a scientific fixture.

## Phase 1 handoff

At the time of this handoff, Phase 1 was unplanned. Its plan was to use this completed Phase 0 provenance process and the Phase 1 entry evidence named in the [sequential master plan](../HYDRUS1D_RUST_PORT_PLAN.md#sequential-phase-plan): Phase 0 provenance plus the curve source anchors. This historical note does not describe the current next step, which is recorded in the master plan.

Before implementation, that future plan should consult the specified [feature matrix](FEATURE_MATRIX.md), [numerical design](NUMERICAL_DESIGN.md), and [validation strategy](VALIDATION_STRATEGY.md), recover or state the relevant source anchors, and establish its own acceptance checks. This handoff does not create a Phase 1 execution plan or promote the smoke outputs to a Phase 1 fixture.

# Noncanonical smoke input

These four files are independently constructed, minimal inputs for one narrow
purpose: exercising the patched GNU Fortran reference executable through normal
completion. They are not recovered HYDRUS examples and are explicitly
noncanonical. They must not be used as a scientific fixture, baseline, parity
result, tolerance source, numerical validation, or release evidence.

`SELECTOR.IN` selects a small water-only, no-atmospheric-boundary calculation;
`PROFILE.DAT` supplies two nodes and one material; `ATMOSPH.IN` is retained as a
task-required input artifact but is not consumed by this selected path.
`LEVEL_01.DIR` is recorded for legacy input-layout completeness. The runner
copies all four files byte-for-byte into a fresh untracked run location and
adapts their names only to the legacy executable's backslash path convention.

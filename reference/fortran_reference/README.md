# GNU Fortran reference harness

This harness creates a patched, untracked copy of the immutable `src_reference/`
snapshot for the current GNU Fortran/Linux target. It is build provenance only:
it does not establish a scientific fixture, numerical validation, parity result,
tolerance, release identity, or redistribution permission.

## Commands

From the repository root, stage the verified source and apply the committed
portability patch:

```sh
python3 tools/stage_fortran_reference.py --output .development/reference-build
```

Then compile that staged copy with the explicit GNU Fortran flags:

```sh
sh tools/build_fortran_reference.sh --build-dir .development/reference-build
```

## Generated paths

Both commands write only beneath the untracked `.development/reference-build/`
directory. Staging creates `src_reference/` and `staging-manifest.json`; building
creates `hydrus-reference` and `build-manifest.json`. The manifests record pinned
source/archive hashes, archive byte-match evidence, patch hash, command, GNU
Fortran version, host details, UTC timestamps, and executable hash.

The artifact-notice field truthfully remains unreviewed: the source audit does
not assign a license or infer redistribution approval. The staged source is a
copy and receives `0001-gfortran-portability.patch`; the checked-in
`src_reference/` files are never modified.

The build also generates `gnu_compatibility.FOR` beside the executable. It is an
untracked build-local shim for the remaining `NARGS`, `getdat`, and `gettim`
legacy entry points; it does not alter the committed source snapshot or patch.

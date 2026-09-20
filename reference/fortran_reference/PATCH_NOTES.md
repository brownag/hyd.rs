# GNU Fortran portability patch notes

## Scope

`0001-gfortran-portability.patch` applies only to a copied `src_reference/` tree. It changes `HYDRUS.FOR` and `TIME.FOR`; the checked-in source snapshot is immutable. The target is GNU Fortran on the current Linux environment using `-std=legacy -ffixed-line-length-none`.

## Source identity

- Archive: `planning/H1D_Src.zip`, SHA-256 `d0c47cbcb7a298ab16c6e08010bd7ca048d5527ce2f989a749390c0c55e848cf`.
- Full source snapshot: ten `.FOR` files, whose SHA-256 inventory is `planning/master/SOURCE_MANIFEST.json`.
- Patch inputs: `src_reference/HYDRUS.FOR` SHA-256 `fca01114a773b0ec9d614d17ed493a008bac82f23d22a187a10a4d4f48fbf3a9`; `src_reference/TIME.FOR` SHA-256 `8b6af595a1ccb3952cfa7b1eb56c0e37a1cfc6b8d55e26d8907288c41f08b92f`.

## Changes

- `HYDRUS.FOR:29`: removes the main-program `use MSFLIB` dependency.
- `HYDRUS.FOR:69,105-106`: uses default-integer `status` and replaces Intel-style three-argument `GETARG` with standard `GET_COMMAND_ARGUMENT(1, cDataPath, status)`. This retains the existing first-user-argument convention; `NARGS()` remains unchanged.
- `HYDRUS.FOR:100-103`: removes all `SIGNALQQ`/`SIG$...` registrations. The two retained handler entry points no longer import `MSFLIB` or dispatch `SIG$`/`FPE$` constants; they report that the unavailable handler was invoked and retain their existing close/return structure. They are no longer registered by this build.
- `HYDRUS.FOR:856`: removes the `CloseOutput` `MSFLIB` import. Its `getdat`/`gettim`, elapsed-time calculation, output writes, and closes are otherwise unchanged.
- `TIME.FOR:551`: changes `if(iMetHour)` to `if(iMetHour.eq.1)`. The local initialization at `TIME.FOR:501-504` assigns `1` for a short interval and `0` for daily data; `TIME.FOR:988` independently already tests `iMetHour.eq.1`. Thus the replacement makes the same integer-flag branch explicit rather than changing its intended predicate.

## Behavior preserved

- Normal program initialization, input fallback, output-close behavior, and meteorological calculations are unchanged except for the compiler-interface adjustment to command-argument retrieval.
- GNU Fortran accepts `getdat`/`gettim` as legacy compiler calls in the patched syntax diagnostic; the patch deliberately does not claim a cross-compiler date/time abstraction.
- The default input path (`LEVEL_01.DIR`) and all existing Windows-style path construction remain byte-for-byte unchanged.
- GNU Fortran no longer installs the Intel/MSFLIB abort, break, interrupt, or floating-point signal handlers. This intentionally preserves ordinary solver behavior while giving up the source's platform-specific asynchronous signal handling.

## Open limitations

| Item | Anchor | Disposition |
| --- | --- | --- |
| Command arguments | `HYDRUS.FOR:105-106` | Patched: `GETARG` becomes `GET_COMMAND_ARGUMENT`; `NARGS()` is deferred as an accepted GNU legacy extension. |
| Date/time | `HYDRUS.FOR:394-396`, `CloseOutput:861-863` | Deferred: `getdat`/`gettim` remain GNU-accepted legacy calls; no portability contract is asserted. |
| Paths | `HYDRUS.FOR:118,136-190` | Deferred: backslash-separated Windows paths require a later runtime portability decision. |
| `CloseOutput` | `HYDRUS.FOR:853-884` | Patched only to remove `MSFLIB`; its timing and output behavior is preserved. |
| Signals | `HYDRUS.FOR:29,100-103,889-932` | Patched: Intel-specific registration/constants are isolated out; no GNU signal replacement is supplied. |
| `iMetHour` | `TIME.FOR:501-504,551,988` | Patched: explicit `.eq.1` preserves its observed `0`/`1` flag meaning. |

Non-goals: this patch does not alter algorithms, numerical precision, input formats, output formats, path semantics, date/time semantics, signal replacement policy, or scientific validation. It does not make claims for compilers or operating systems other than the stated GNU Fortran/Linux target.

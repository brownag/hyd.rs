# Pinned `VaporContent` kernel harness

`requests.json` uses schema `vapor-content-request-v1`. Each request names a
positive `x_conv` in cm/m, a contiguous 1-based material table of porosities, and
nodes with liquid water content, temperature in °C, head in cm, material index,
and the input hydraulic capacity in 1/cm. Inputs are finite and constrained to
physical ranges by the runner; the sample requests cover dry/moist states,
multiple temperatures, heads, porosities, and positive length conversions.

The standard-library runner extracts the exact pinned `VaporContent` bytes from
`src_reference/WATFLOW.FOR`, checks whole-source and routine SHA-256, and builds
the extracted fixed-form `.FOR` with this separate free-form driver using GNU
Fortran `-std=legacy -ffixed-line-length-none -O0`. REAL remains default
single-precision. Output uses `ES24.16E3`, sufficient to round-trip a default
REAL value; JSON records the parsed decimal observation and retains the raw
stdout hash. The capacity field in each input is retained separately from the
routine-adjusted output capacity.

From the repository root:

```sh
python3 tools/run_vapor_kernel_reference.py --help
python3 tools/run_vapor_kernel_reference.py --requests reference/vapor_fortran/kernel_harness/requests.json --task-root .development/25.1.2/kernel-reference-001 --output reference/vapor_fortran/kernel_fixtures/vapor_content.json --timeout-seconds 60
```

Attempt roots and fixture destinations are never overwritten. Build/run logs,
commands, timestamps, exit codes, hashes and failure records are retained under
the task root. Evidence is labeled `pinned-fortran-kernel`: it is direct output
from the pinned routine, not a full-program trajectory, a Rust comparison, a
scientific tolerance, or numerical acceptance.

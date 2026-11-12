# Full-executable vapor fixtures

The three case decks under `cases/` are source-matched inputs for the unchanged,
staged HYDRUS reference executable. `vapor_isothermal_head_gradient` uses constant
initial/boundary temperature with a fixed water-head gradient; `vapor_thermal_gradient`
uses a temperature gradient and fixed water heads; `vapor_off_control` repeats the
thermal deck with only `lVapor` disabled. Vapor-enabled decks set `lWDep=false`.
All decks use `LUnit=cm`, `TUnit=hours`, `MUnit=kg`, and `iCampbell=0`. The
isothermal case uses the illustrative sandy row; the thermal/off-control pair uses
the identical illustrative loamy row. Exact parameter values, source basis, and
conversion arithmetic are recorded in
[`planning/master/M6_VAPOR_THERMAL_MATERIAL_RESEARCH.md`](../../planning/master/M6_VAPOR_THERMAL_MATERIAL_RESEARCH.md).
These rows are selected for order-of-magnitude and dimensional-analysis behavior
only: fractions/dispersivity include explicit fixture assumptions, the clay curve
has a documented fallback, and no row is calibrated or asserted universal. The
profile rows map node, depth, initial
head, material selectors, and initial temperature as shown in each `PROFILE.DAT`.

The producer records every printed time and top-first source node. `NOD_INF.OUT`
contains head, liquid moisture and temperature; when enabled, `NOD_INF_V.OUT`
contains `Con`, `ConLT`, `ConVh`, `ConVT`, `vLiquid`, `vVapor`, `vTotal`,
`vVapIso`, and `vVapTerm` in source format 140 (`i4,1x,f10.4,1x,9e13.5`).
Head formatting depends on the source's 120/130 branch, moisture is `f6.4`,
temperature is `f8.2`; fixture metadata records the print schedule and output
hashes. Vapor output absent in the off-control is marked unavailable, never zero.

From the repository root, stage and build as documented in
[`../fortran_reference/README.md`](../fortran_reference/README.md), then run:

```sh
python3 tools/run_vapor_reference.py --all --build-dir .development/25.2.2/reference-build --task-base .development/25.2.2/reference-runs
```

This is reference-only evidence labelled `pinned-fortran-trajectory`. It does not
establish Rust parity, numerical acceptance, tolerance, or direct vapor-content
trajectory observations. Fixture provenance includes input, output, source/build,
selector/profile and producer hashes.

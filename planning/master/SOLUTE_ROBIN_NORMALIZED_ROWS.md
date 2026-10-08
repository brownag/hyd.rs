# Scalar Robin first-solve rows in a common orientation

## Convention and retained inputs

Use physical coordinate `s` increasing from inlet to outlet, and write each equation as

`a_lower C[k-1] + a_diag C[k] + a_upper C[k+1] = rhs`.

Rust uses node `k=0..128` in increasing `s`. Fortran uses `x=-s` and reversed node order, so Fortran `i` maps to Rust `k=129-i`. Its matrix is `B(i) C(i-1) + D(i) C(i) + E(i) C(i+1) = F(i)`. Consequently the physical-order mapping is `(a_lower,a_diag,a_upper)=(E,D,B)` for interior rows; endpoint neighbor terms map by the same reversal. No extra sign reversal is required: both stored equations are left-hand matrix times concentration equals RHS. This is a common oriented coefficient basis, not a claim that the FE weak-form row and FV control-volume row have identical quadrature or support.

Both traces use `dt=0.00390625 h`, 129 nodes over 2 cm (`dx=0.015625 cm`), `theta=0.43`, `D=1 cm²/h`, `q=+0.43 cm/h` in physical `+s`, linear mass derivative `theta + rho Kd = 0.645`, and decay `0.2 h⁻¹`. The case sets inlet total flux with `C_in=1` and zero-gradient outlet. Fortran's internal velocity and `wc` signs are reversed: at the midpoint `Pe2=-0.0078125`, `wc=-0.00244140625`; magnitudes correspond to `Pe=0.015625`, `|v|=1 cm/h`. Initial concentration states differ by at most `2.9911e-8` after reversal, consistent with Fortran single precision.

## Normalized measured equations

The table gives rows in the convention above. Fortran's raw midpoint and inlet tuples are reordered from `(B,D,E,F)`; the endpoint rows are converted by node mapping. Values come from `.development/15.2.2/trace-summary.json`.

| Location | Rust `(lower, diag, upper ; rhs)` | Fortran in physical order `(lower, diag, upper ; rhs)` |
| --- | --- | --- |
| Inlet `k=0` / `i=129` | `(0, 29.24100781, -27.52 ; 1.54355686)` | `(0, 15.15809822, -13.65259457 ; 1.32847661)` |
| Interior `k=64` / `i=65` | `(-27.95, 58.05201562, -27.52 ; 1.96529857)` | `(-13.86759472, 30.10119629, -13.65259457 ; 1.96497154)` |
| Outlet `k=128` / `i=1` | `(-27.95, 29.24100781, 0 ; 0.93590662)` | `(1, -1, 0 ; 0)` |

For example, Fortran midpoint is `-13.86759472 C[63] + 30.10119629 C[64] - 13.65259457 C[65] = 1.96497154`; the Fortran outlet equation `-C[128]+C[127]=0` appears in physical ordering as `C[127]-C[128]=0`. The reported Rust outlet row retains the half-cell conservative balance, rather than imposing this two-point zero-gradient closure. The inlet RHS values also differ despite matched physical boundary request and close first-step state.

## What the equations identify

- **Upstream weighting:** Rust face transport at the first face is diffusion coefficient `27.52` and advection `0.43`, yielding the recorded flux coefficients `27.95` and `-27.52`. Fortran's `wc` is small but nonzero and enters its FE advection terms in `MatSet`; its magnitude and sign are known. The trace does not provide a weighting-only counterfactual, so the row differences cannot be assigned to `wc` alone.
- **Spatial form:** Fortran uses FE element contributions assembled across adjacent intervals; Rust uses conservative FV face fluxes and nodal control-volume rows. The interior coefficients and endpoint support therefore differ even if the continuum transport terms agree. The raw FE coefficients cannot be converted to Rust FV coefficients by a universal scalar row factor while preserving the different endpoint supports.
- **Time weighting/storage:** Both rows include storage and decay. Rust's reported linear mass derivative is `0.645`; Fortran's retardation is `1.5` and `g1=-0.129`, giving comparable reported storage/reaction row contribution. The midpoint RHS values are close (`1.96529857` versus `1.96497154`), while diagonal and off-diagonal coefficients differ substantially. Because spatial mass matrices/control volumes and time discretization are assembled together, this comparison does not isolate a time-weighting defect.
- **Endpoint closure:** The zero-gradient outlet is encoded as a distinct Fortran two-node equation, while Rust keeps a half-cell flux balance. At the inlet, the trace maps both cases to total inflow `q C_in`, but the FE endpoint row and FV half-cell balance have different coefficients and RHS. This is a demonstrated assembly difference, not proof that either implementation violates its declared boundary law.

The retained two-run trace falsifies a pure effective-parameter translation explanation and shows divergence in the first assembled solve. It does not separate upstream weighting, FE/FV spatial/time weighting, and endpoint closure; it therefore does not identify a unique Rust defect. No numerical change is justified from this evidence.

## Disposition

`no_fix_cause_unisolated`. The normalized inlet, midpoint, and outlet equations establish where the formulations differ, while the existing evidence cannot allocate the profile residual to a specific Rust term. A weighting-only or boundary-only counterfactual would be the discriminator, but no such solver run is authorized by this task. The conditional follow-up should stop without production changes unless a separately authorized discriminator isolates a Rust error.

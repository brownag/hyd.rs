# M6 vapor `step_off` water-balance bound proposal (task 28.3, awaiting approval)

Date: 2026-11-13. Read-only analysis. No criteria, fixture, test or production code changed.
Criteria stay frozen at v3 (`criteria.json` SHA `f198d0c1...`); this proposes one new key only.

## Question for the user

Approve adding `checks.water_balance_interior_step_off_reference` (JSON below) to
`reference/vapor_fortran/criteria.json`, with every existing key byte-identical? Specific points to
confirm: (1) relative 1e-6, (2) absolute 1e-15 in L/T per node row, (3) scale definition, (4) that it
claims "static-water consistency", not a general water balance.

## Evidence (pinned Fortran only)

Source: `.development/27.2.2/runs/vapor_nonuniform_step_off/run/*NOD_INF.OUT`, `BALANCE.OUT`,
`T_LEVEL.OUT`. Executable `.development/26.2.2/build/hydrus-reference` (SHA `e6d69dd3...`), built
with `-std=legacy` only, so default REAL is single precision. Scratch script:
`.development/28.3-bound/an.py`.

Formula check. Fortran prints node fluxes `v(i)` from `Veloc` (`WATFLOW.FOR:717-775`): interior
`v(i)` is the dx-weighted mean of the two face fluxes (lines 748-751); end nodes add the storage term
(`v(1)` line 771 `+dx1/2*(dTh/dt+Sink)`, `v(N)` line 729 `-dxN/2*(...)`). The Rust diagnostic residual
`dx*((th_new-th_old)/dt + s) + (q[i+1]-q[i])` is the continuum form of the Richards row
`P(i)` in `WATFLOW.FOR:235-267` (storage `(ThNew-ThOld)*dx/dt` plus `Sink*dx` plus flux divergence).
Face fluxes are not printed by the Fortran, so for the reference the face difference is taken from
the printed node flux. With a spatially uniform node flux the two coincide exactly.

Reference data, all 41 nodes, all 5 printed times (0, 6, 24, 72, 240 h):

| Quantity | Value at every node and time |
|---|---|
| head | -10000.000 cm (Dirichlet both ends, uniform) |
| theta | 0.0844 (print `f6.4`) |
| K | 0.7228E-10 cm/h |
| node flux | -0.72283E-10 cm/h, identical at every node |
| sink | 0 |
| `WatBalT` / `WatBalR` | 0.00000E+00 L / 0.000 %, every printed time (`OUTPUT.FOR:712-713`) |
| `W-volume` | 0.33746E+01, unchanged 0 to 240 h |
| `T_LEVEL.OUT` vTop = vBot | -0.72283E-10 for all 2400 steps (flux constant) |
| `RUN_INF.OUT` | dt 0.1, Iter 2 per step, 2400 steps, all converged |

Residual characterisation (printed resolution, 4 intervals):
- max |flux divergence| `|v[i+1]-v[i]|` = 0 (printed quantum 1e-15 L/T, the last digit of `E11.4`/`E12.5`).
- max |storage| from print-to-print theta change = 0 (theta printed to 1e-4).
- Residual = 0 at every interior node, every interval. Relative to the row scale (7.2283e-11) it is
  0, not 1e-16; it is structurally zero (uniform fixed head gives identical theta and uniform flux),
  so it is neither roundoff nor discretisation error in the Fortran output.

## Limits of the evidence

- Only five snapshots, not per-step values. The Fortran writes no per-step theta or face flux.
  Per-step coverage rests on the static-state argument above (and constant `T_LEVEL.OUT` fluxes).
- Theta resolution is 1e-4. From print alone, storage is bounded only by
  `dx*1e-4/dt_print`, about 1.7e-5 L/T per cm over the shortest 6 h interval, much larger than the
  flux (7.2e-11). So the print cannot demonstrate a tight storage bound; `WatBalT` = 0 and constant
  `W-volume` (5 digits) support it only globally. The tight bound is justified by the constant-head
  structure, not by printed digits.
- Single-precision Fortran: roundoff epsilon about 6e-8 relative.
- Dirichlet end nodes (1 and 41) excluded; their flux includes boundary bookkeeping.
- Single case, near-static (h = -10000 cm, K = 7e-11). It says nothing about wetting or draining.
- The Fortran's residual cannot be computed at the `max_abs_row_term` form of
  `water_balance_interior` (no `h_it`, `thL_it`, `C_eff`); see
  `M6_VAPOR_STEP_OFF_BALANCE_ASSESSMENT.md`.

## Proposed bound

Residual at interior node `i` (0-based 1..n-2), per accepted step:
`R_i = dx*((th_new[i]-th_old[i])/dt + s[i]) + (q[i+1]-q[i])`, `dx=(x[i+1]-x[i-1])/2`.
Scale: `max(|dx*((th_new-th_old)/dt + s)|, |q[i+1]|, |q[i]|)` (named `max_abs_row_term`, as in the
existing checks, here over the storage and the two face-flux terms). Pass if
`|R_i| <= max(relative*scale, absolute)`.

- relative 1e-6: about 16 times single-precision epsilon (6e-8). Rationale: the reference is a
  single-precision run, so roundoff-level agreement cannot be demanded tighter than that scale
  without evidence; it is a safety factor over the reference arithmetic, not over the Rust candidate.
- absolute 1e-15 L/T: the printed quantum of the Fortran flux and the existing criteria floor.
  With scale 7.2e-11, `relative*scale` = 7.2e-17, so the absolute floor governs. Effective bound
  1e-15, about 1.4e-5 of the flux magnitude.
- Fortran's own residual: 0 against an effective bound of 1e-15, so it satisfies the proposal with
  zero used (infinite margin at print resolution; the margin is not measurable below the quantum).
- A candidate must keep storage below 1e-15 L/T, i.e. `|dtheta|` below about 1e-16 per step at
  dx = 1 cm, dt = 0.1 h. That is a strong demand on theta, consistent with the Fortran holding
  theta exactly constant. The first run of the asserting test may fail on this if the candidate
  drifts theta; in that case the answer is a diagnosis, not a looser number.

## Exact key and JSON (not written)

`checks.water_balance_interior_step_off_reference`

```json
"water_balance_interior_step_off_reference": {
  "absolute": 1e-15,
  "assumptions": [
    "case vapor_nonuniform_step_off only (lVapor off, constant dt 0.1, fixed heads, sink 0)",
    "interior nodes only; Dirichlet end nodes not asserted",
    "continuum residual dx*((th_new-th_old)/dt + s) + (q[i+1]-q[i]) from accepted theta and face fluxes",
    "bound derived from pinned-Fortran NOD_INF/BALANCE/T_LEVEL output; Fortran residual is 0 at print resolution",
    "reference is single precision; theta printed to 1e-4, so per-step storage is not resolved by print"
  ],
  "relative": 1e-06,
  "scale": "max_abs_row_term",
  "unit": "model"
}
```

Existing keys (`heat_balance_interior`, `water_balance_interior`, and all others) remain
byte-identical; the file's `schema_version` and any recorded hash must be re-frozen by the approver
(adding a key changes the file hash, so this is a v4 act, not an in-place v3 edit).

## What the bound claims and does not claim

Claims: on the static `step_off` case the accepted theta and face fluxes satisfy the continuum
interior water balance to within 1e-15 L/T (relative 1e-6 of the largest row term), the same
level the pinned Fortran output exhibits (zero residual, `WatBalT` 0).

Does not claim: closure of the `water_balance_interior` iterate form; any statement about
non-static water, wetting fronts, or other cases; Dirichlet end-node flux; per-step Fortran
agreement (the Fortran reference is five snapshots plus constant `T_LEVEL.OUT` fluxes); a bound
derived from the Rust candidate; that the Fortran residual is roundoff-limited rather than
structurally zero.

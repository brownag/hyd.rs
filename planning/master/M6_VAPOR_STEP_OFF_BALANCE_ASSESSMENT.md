# M6 vapor `step_off` balance assessment (task 28.2 diagnosis)

Date: 2026-11-13. Read-only diagnosis. No code, test, fixture or criteria changed.

## Verdict: C (new heat-only row-residual helper needed), water side limited

- (A) `vapor_balance` with vapor off: not possible.
- (B) `heat::balance::HeatBalance` is a reusable global-energy helper, but it is not
  the same check as `heat_balance_interior` (see below). Usable only as an extra.
- (C) Chosen: a heat-only interior row residual, about 90 lines, test-local.
- Water: only a continuum interior residual is available; it is not the
  `water_balance_interior` check and needs a user-approved bound.

## Search log

1. `crates/hydrus-core/src/heat/vapor/balance.rs` read in full.
2. `grep -rniE "balance|heat_balance|energy|residual"` over `src/heat`, `src/*.rs`,
   `tests/`. Relevant hits:
   - `src/heat/balance.rs` (276 lines): `HeatBalanceInput`, `HeatBalance::record_step`.
     Global storage-change versus boundary and sink heat, caller supplies all terms.
   - `tests/heat_balance.rs` (688 lines): recomputes end fluxes and sink heat in the
     test, replays `advance_water_step` on clones to get face fluxes, copies
     `node_velocities` from `driver.rs` (private there). Seeded synthetic columns only.
   - `src/water_solver.rs`: `WaterBalance` inside `WaterStepReport` (global, uses
     `measured_flux` of the upper boundary).
   - `tests/vapor_heat_boundary_balance.rs`, `tests/vapor_balance.rs`,
     `tests/vapor_nonuniform_balance.rs`: vapor driver only.
   - No heat-only per-row (interior) residual exists anywhere.
3. `reference/vapor_fortran/cases/vapor_nonuniform_step_off/SELECTOR.IN` line 12
   `f f f f f f` (lVapor off); `vapor_nonuniform_parity.rs` step_off path
   (`run_case`, `!deck.l_vapor` branch).

## Can `vapor_balance` run with vapor off? No

- Signature takes `&VaporStepReport`, produced only by `VaporWaterHeatDriver::step`.
  `VaporConfig::new` rejects `l_vapor = false` (`UnsupportedBoolean`), so no report
  exists for step_off. Fabricating a report by hand is not sound: it would test the
  fabrication.
- Water formulas need fields the legacy path never produces: `pre_solve_heads`
  (`h_it`), `pre_solve_liquid_water_contents` (`thL_it`), `effective_hydraulic_capacities`
  (`C_eff`), `liquid_conductivities` at the iterate, `passes[1].heads`. The legacy
  `WaterStepReport` exposes only `water_contents`, `face_fluxes`, `balance`.
- Heat formulas do reduce to heat-only when vapor is zero: `heat_balance()` is the
  `TEMPER.FOR:112-143` rows plus `adjust_heat_rows`, and with `v_vapor = 0`, `thV = 0`
  the adjustment arrays are zero (all `Cv*v` and latent terms vanish). But the
  function is private and bound to `VaporStepReport`/`passes[1]`. Reuse would need
  editing `balance.rs`; this assessment forbids changing its numerics.

## What the legacy driver exposes for step_off

- `HeatWaterDriver::step` returns `CoupledStepReport { accepted_time, timestep,
  heat_flux, minimum_absolute_pivot, water_iterations, water_retries }`.
- `driver.state().temperatures()` gives accepted `T`; `water.heads()`,
  `water.water_contents()` give accepted state. No face fluxes, no velocities, no
  old contents. The test must hold old `theta`, `T`, `v_old` itself and obtain face
  fluxes by replaying `advance_water_step` on clones (the technique in
  `tests/heat_balance.rs:480-505`), then `node_velocities` as in `driver.rs`
  (Veloc, `WATFLOW.FOR:717-775`).
- step_off schedule: constant dt 0.1, 2400 steps, `sink = 0`, fixed heads at both
  ends, material 0 only (`parity` header and `M6_VAPOR_NONUNIFORM_PARITY_MISS_DIAGNOSIS.md`).
- Initial heads are -10000 cm (very dry, `PROFILE.DAT`); water is near static.

## Proposed heat helper (formulas)

Interior nodes `i = 1..n-2` (zero-based), `dxA = x[i]-x[i-1]`, `dxB = x[i+1]-x[i]`,
`dx = (x[i+1]-x[i-1])/2`, `Cw` = `TempMaterial::water_heat_capacity`, `s` = sink.
Level one uses `Cap_old`, `Cond_old`, `vOld`, `TempO`; level two uses `Cap_new`,
`Cond_new`, `vNew`. `Cap`/`Cond` from public `temp_cap` with `water_old`/`water_new`
and the respective velocity.

Citations: `src_reference/TEMPER.FOR:112-143` (the `do 12 i=2,N-1` loop, level 2
`B`,`D`,`E`; level 1 `F`). These are identical to the vapor-off terms already
transcribed in `balance.rs:473-507` with the `adj.*` terms set to zero:

```text
F_i = TO[i-1]*((Cond_o[i]+Cond_o[i-1])/4/dxA + Cw*(vo[i]+2vo[i-1])/12 - dxA/24*Cw*(s[i-1]+s[i]))
    + TO[i]  *(-Cw*(vo[i+1]-vo[i-1])/12 + dx/dt*Cap_o[i] - (Cond_o[i+1]+Cond_o[i])/4/dxB
               - (Cond_o[i]+Cond_o[i-1])/4/dxA - dxA/24*Cw*(s[i-1]+3s[i]) - dxB/24*Cw*(3s[i]+s[i+1]))
    + TO[i+1]*((Cond_o[i+1]+Cond_o[i])/4/dxB - Cw*(2vo[i+1]+vo[i])/12 - dxB/24*Cw*(s[i+1]+s[i]))
B_i = -(Cond_n[i]+Cond_n[i-1])/4/dxA - Cw*(vn[i]+2vn[i-1])/12 + dxA/24*Cw*(s[i-1]+s[i])
D_i = (Cond_n[i-1]+Cond_n[i])/4/dxA + (Cond_n[i]+Cond_n[i+1])/4/dxB + dx/dt*Cap_n[i]
      + Cw*(vn[i+1]-vn[i-1])/12 + dxA/24*Cw*(s[i-1]+3s[i]) + dxB/24*Cw*(3s[i]+s[i+1])
E_i = -(Cond_n[i]+Cond_n[i+1])/4/dxB + Cw*(2vn[i+1]+vn[i])/12 + dxB/24*Cw*(s[i+1]+s[i])
R_i = B_i*TN[i-1] + D_i*TN[i] + E_i*TN[i+1] - F_i
scale_i = max |each of the 3 lhs products and 3 F_i products|
```

`R_i` is the assembled heat row minus its right side, so it closes to tridiagonal
solve roundoff for the same inputs. Closure proves the accepted `TN` satisfies the
level-one/level-two rows built from the accepted water, not that the rows are right
(that is the parity test's job). It is a discrete-consistency check, same class as
the step_on heat check. It checks `v_new`/`Cap_new` reconstruction independently of
the driver only if the helper builds them itself (it should).

Kernel and bound: reuse criteria `checks.heat_balance_interior` (`relative 1e-9`,
`absolute 1e-15`, scale `max_abs_row_term`), loaded the way
`vapor_nonuniform_balance.rs::load_bound` does. `|R_i| <= max(rel*scale_i, abs)`.

File location: a private `fn heat_only_row_residual(...)` in a new
`crates/hydrus-core/tests/vapor_nonuniform_step_off_balance.rs`. Size estimate: about
90 lines for the helper plus about 120 lines of case setup (deck/profile parse and
water/hydraulics setup copied from `vapor_nonuniform_balance.rs`/parity, which both
already say "copied, not imported"), roughly 250 lines total. Alternative: a public
`heat::balance::heat_row_residual` in `src/heat/` (about 80 lines plus tests); it
adds public API surface (`api_contract.rs`), so test-local is cheaper and safer.

## Water side

Not reproducible in the `water_balance_interior` form (needs `h_it`, `thL_it`,
`C_eff`, iterate conductivities; none exposed). Available alternative, computed from
the replay report: for interior `i`,
`R_i = dx*((th_new-th_old)/dt + s) + (q[i+1]-q[i])`, `q = face_fluxes`
(`WATFLOW.FOR:235-267` with `lVapor` terms dropped). It closes only to the water
convergence tolerance (`TolTh`, `TolH`), the same class as `corrected_content_residual`,
which `balance.rs` documents as reported-only. A strict `1e-9` bound is not valid.
With heads near -10000 cm and static water the residual and scale are both tiny, so
the check has little power. Recommendation: report water residual (maximum
`|R_i|/scale`) as diagnostic, or accept the gap explicitly; any asserted bound needs
a user-approved criteria entry (criteria.json is not edited here). The global
`WaterBalance.absolute_residual` is not usable: ends are fixed head and the test
passes `flux: 0.0`.

Also optional and cheap: feed `HeatBalance::record_step` (B) as a cross-check using
the `heat_balance.rs` end-flux and reservoir convention. It is a different
quantity (global energy with a Dirichlet reservoir convention), so it does not
replace the row residual.

## Test sketch

```text
run step_off via HeatWaterDriver at constant dt 0.1 (as parity run_case, l_vapor false branch)
per step:
  theta_old = water.water_contents(); t_old = driver.state().temperatures(); v_old kept
  replay: clone water+workspace, advance_water_step -> face_fluxes, timestep
  driver.step(...) -> assert report.timestep bits equal replay timestep
  theta_new, t_new from state; v_new = node_velocities(face, theta_old, theta_new, sink, dt)
  (cap_o,cond_o) = temp_cap(theta_old, v_old); (cap_n,cond_n) = temp_cap(theta_new, v_new)
  for i in 1..n-1: R_i and scale_i per formulas; assert |R_i| <= max(rel*scale_i, abs)
  v_old = v_new
optional: report max |R_water_i|/scale (diagnostic only)
```

Run only this test file (about 2400 steps on 41 nodes, cheap). Dependencies:
`HeatConfigInput { l_vapor: false, ..}`, `TempMaterial::from_tpar`, public `temp_cap`,
`advance_water_step`.

## Regression risk

- No change to `src/heat/vapor/balance.rs`, `vapor_balance`, the vapor driver, the
  legacy heat driver, `criteria.json`, or any existing test: new test file only,
  so existing balance tests and numerics are unaffected.
- Duplicated `node_velocities` can drift from `driver.rs`; the precedent
  (`tests/heat_balance.rs`) uses the same approach, and divergence shows as a
  closure failure.
- `step_off` evidence stays class `rust-candidate`. The result does not upgrade
  G22.4 beyond "step_off heat balance covered"; the water gap and step_on
  controller exception remain as stated in `M6_VAPOR_G22_4_ACCEPTANCE.md`.
- The new check reads `checks.heat_balance_interior` bounds; a criteria version
  change must keep that key (schema `vapor-criteria-v2` assertion as in
  `vapor_nonuniform_balance.rs`).

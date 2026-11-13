# M6 vapor nonuniform step: NaN diagnosis (task 27.3.3)

Status: diagnosis only. No production code, test, fixture or criteria file was changed.
Scratch evidence: `.development/27.3.3-diag/` (`copy/` instrumented tree, `run4/` instrumented Fortran run).

## Summary

The NaN is a symptom, not the defect. The Rust vapor water step mixes the iterate heads for the
SetMat material evaluation WITHOUT first clamping each head to `hSat`. The Fortran clamps both
`hTemp` and `hNew` to `hSat` before the 0.1/0.9 mix (`WATFLOW.FOR:455-462`). This is a defect in the
Rust port, not an input the Fortran handles differently.

At dt = 0.1 the Picard iteration for this case overshoots (iteration 1 gives h = +19388 cm at node
21). Fortran then sees an unsaturated mixed head and keeps iterating boundedly until `MaxIt`, fails
the attempt, and retries with dt/3. Rust sees a saturated mixed head, zeroes the vapor terms at that
node, and diverges to h of about 1e9 cm. There `Hr = exp(h*xMol*g/R/T)` overflows to inf and
`ConVh = 0 * inf = NaN`, which the kernel guard reports.

## Reproduction

```sh
cargo test -p hydrus-core --test vapor_nonuniform_parity
# vapor step failed: Water(Vapor(NonFinite { name: "ConVh", index: Some(20), value: NaN }))
```

## Value chain (Rust, first failing attempt, dt = 0.1, node 21 = index 20, T = 5 C)

Heads printed after each linear solve; scratch `eprintln!` in the copy only.

| iter | Rust h[idx 19..22] (cm) | note |
|---|---|---|
| 1 | -9994.78, +19387.5, -31984.9, -10029.6 | matches Fortran iter 1 (FDIAG: -9994.78, 19388.1, -31985.4, -10029.6) |
| 2 | -10102.7, -588080, -588092, -588088 | Fortran iter 2: -9999.4, -6534.2, 18607.5, -55839.6 |
| 3-6 | grows to -1.1e7 | |
| 7 | h(idx 20) = 9.8e8 cm | `h` = 9.8e6 m |

Iteration 2, node 21 (idx 20), the divergence point:
- `trial_heads` = +19387 cm, `h_temp` = -10000 cm, so the mixed head = 0.1*(-1e4) + 0.9*19387 = +16449 cm.
- That is above saturation, so `theta = 0.43 = theta_s`, `theta_a = 0`, `tau = 0`, `diff = 0`.
- `ConVh = ConVT = 0` and `cap = 0` at the node (logged: `con_vh=0`, `cap[20]=0`).
- Fortran value at the same point: `hi1 = min(hSat,-1e4) = -1e4`, `hi2 = min(hSat,19388) = hSat` (about 0),
  `hiM = -1000 cm`. The node stays unsaturated, with finite `ConVh`, `ConVT` and `Cap`.

Iteration 7 (the NaN):
- `h = 9808654 m` gives `hr = exp(9.8e6 * 0.018015 * 9.81 / 8.314 / 278.15) = inf`.
- `diff = 0` because the node is saturated, so `con_vh = diff/rho*rho_vs*...*hr = 0 * inf = NaN`.
- The expression is `crates/hydrus-core/src/heat/vapor/kernels.rs:52-53`, the `ConVh` product, with `hr` from `kernels.rs:41`.

Not the cause (ruled out): negative `powf`/`sqrt` argument (`theta_a > 0` guard, `0.02.sqrt()` constant);
division by zero (`tk`, `rho` positive); the 5 C vs 45 C contrast itself (all node 21 inputs are finite at iterations 1-6);
orientation or reversal of the arrays (harness reversal matches Fortran's own reversal at `INPUT.FOR:252-254`;
Rust idx k equals Fortran internal i = k+1, and iteration 1 matches to 4-5 digits).

## Fortran comparison (line by line)

Fortran `SetMat`, `src_reference/WATFLOW.FOR:455-462`:
```
hi1=amin1(hSat(M),hTemp(i)/Ah(i)/AT)
hi2=amin1(hSat(M), hNew(i)/Ah(i)/AT)
hiM=0.1*hi1+0.9*hi2
```
Then `Coni=ConSat` if `hi1>=hSat .and. hi2>=hSat` (`:466-467`), else `FK(hiM)`; `Capi=0, Thei=ths` if
`hiM>=hSat` (`:490-492`). `ConVapor` and `VaporContent` then use the resulting `theta` (`:564-569`).

Rust, `crates/hydrus-core/src/water_solver.rs:316`:
```
let mixed_head = 0.1 * head_temp + 0.9 * candidate_workspace.trial_heads[node];
```
There is no `min(hSat, .)` on either term. The compat path already does it correctly:
`solute_fortran_compat/water_compat.rs:538-549` (`h_sat.min(...)`, `con_sat` branch). The vapor step in
`water_solver.rs` is the only place that omits it. The other terms of `ConVapor` (`kernels.rs:30-58`) match
`WATFLOW.FOR:891-922` and were not at fault.

The Fortran trajectory (instrumented `diagexe`, `run4/`) is itself chaotic at dt = 0.1: `|h|` reaches about 5e6
cm by iteration 20, but never 1e9. It fails the attempt after `MaxIt=20` and retries at dt/3 (`WATFLOW.FOR:136-146`).
Fortran `RUN_INF.OUT` confirms the first accepted step has dt = 0.0333 and ItCum = 41.

## Verification of the hypothesis (scratch copy only)

Only change in `.development/27.3.3-diag/copy/crates/hydrus-core/src/water_solver.rs`: clamp both terms to
`source_h_sat(parameters)` before mixing. Result:
- Iteration 2 heads: -6534.29, 18606.8, -55838.3 (Fortran: -6534.21, 18607.45, -55839.64). The Fortran difference is float32.
- The dt = 0.1 attempt runs the full 20 iterations, fails (no NaN), and the dt/3 retry converges
  (iter 1: -191.7, -17343.3; Fortran: -191.52, -17343.48), as in Fortran.
- `vapor_nonuniform_parity`: no NaN; 4 of 5 tests pass. `nonuniform_step_on_matches_fixture` then reports
  147 of 2460 criteria rows outside bounds (first: head, t=6, node 17, error 682 vs bound 246). That is a
  real parity result for 27.3.2 to report, probably related to the constant-dt versus adaptive-schedule
  disclosure in the test header. It is outside this diagnosis and must not be tuned.

## Proposed minimal fix (after approval)

File: `crates/hydrus-core/src/water_solver.rs`, loop at about lines 314-322.
1. Compute `h_sat = source_h_sat(parameters)` for the node's material (already available at `:202`; the
   convergence test at `:387` builds it the same way).
2. `hi1 = h_temp.min(h_sat)`, `hi2 = trial.min(h_sat)`, `mixed_head = 0.1*hi1 + 0.9*hi2`
   (`Ah = AT = 1` here, so the divisions are no-ops; keep that comment).
3. For source fidelity, when `hi1 >= h_sat && hi2 >= h_sat` use the saturated conductivity (`ConSat`, as
   `WATFLOW.FOR:466-467` and `water_compat.rs:547-548`) instead of `FK(h_sat)`. Check that the model exposes
   `ConSat`; if it equals `evaluate(h_sat).conductivity` to full precision the clamp alone suffices, otherwise add
   the branch. Capacity and theta at `hiM >= hSat` already come out as `0` and `theta_s` through `evaluate`.
4. Do NOT soften the `NonFinite` guard in `kernels.rs`; it is correct. No change to `kernels.rs`, fixtures,
   criteria, tolerances or the test harness.

## Targeted test

Add to the existing unit tests in `water_solver.rs` (near `:1258`, the D1 mixed-head test), or a new
`crates/hydrus-core/tests/vapor_hsat_mixed_head.rs`:
- Build a single-material 5-node vapor case with a pre-solve iterate above `hSat` at one node and
  `h_temp` strongly negative; call the liquid evaluation path. Assert the mixed evaluation equals
  `evaluate(0.1*h_temp + 0.9*hSat)` (unsaturated), not the saturated state.
- End-to-end: `cargo test -p hydrus-core --test vapor_nonuniform_parity` no longer returns
  `NonFinite`; the first accepted step has `timestep == dt/3` (trace `cand.trace`) and `ItCum`-equivalent
  retry occurred, matching Fortran `RUN_INF.OUT` (`dt = 0.03333`).

## Regression risk

- Heads at or below `hSat` are unchanged (`min` is the identity), so cases that never exceed saturation are
  bit-identical. This should cover the passing dry uniform and heat-boundary vapor cases (`h = -1e4`), but they
  must be re-run: `vapor_heat_boundary_parity`, the 25.x/26.x vapor suites, and `cargo test -p hydrus-core --lib`
  for `water_solver`.
- Cases with a ponded or saturated node and vapor on change behavior toward the Fortran. Any existing
  expectation encoded from the unclamped mix is a latent bug, but would need a fixture re-check, not a tolerance change.
- The test at `water_solver.rs:1258-1262` mixes unclamped heads, but its heads are below `hSat`, so it is unaffected.
- Not run here (per instructions): full workspace suites, clippy.

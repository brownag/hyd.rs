# Bounded Robin coefficient/error audit (R0/R1/R3)

Date: 2026-11-11. Scope: the bounded Robin diagnostic recommended by
`M5V_REVIEW_23_NEXT_STEPS.md`. This is a diagnostic record, not a solver fix,
threshold change, gate signature, holdout, or M5-V closure claim.

## Verdict

The evidence does **not** support small-Peclet f32 upstream-weight cancellation
as the cause of the 40 failing Robin-space findings in the official campaign.
The public verification path never exercises the upstream-weight branch: across
all four saved Robin runs the `PeCour` weight branch is called **0** times. The
public adapter leaves `compat_cfg.upwind = false`, even though
`CASES_V2.json` declares `chemistry.upstream_weighting: true`. Forcing that
branch on at the failing configurations changes the measured errors only at the
`1e-7` to `1e-5` level, far below the `~1.2e-4` non-monotone jump.

The strongest supported attribution is **temporal/spatial error interaction at
the selected dt**, not a fixed spatial coefficient or outlet-only defect. At
dt=2^-7 the 129/193/257 `linf` errors are non-monotone. At the completed
dt=2^-8 triplet the same mesh sequence is monotone at every sampled time. The
outlet remains the maximum-error location, but the interior tracks it closely,
so outlet closure cannot explain the interior/inventory failures by itself.
Accumulated f32 roundoff is not supported as the dominant cause, although it is
not fully excluded.

## R0: reconciliation and pins

All checks are read-only. The existing `.development/23-final` tree was not
modified; new outputs are under `.development/23-robin-audit/`.

- `CASES_V2.json` SHA-256: `420af7c0085f3ad5cf3335ca036ca5360d7abd370807451d8d479e653a215df7`
  (matches the recipe).
- All 11 producer hashes in the batch recipe match the current files.
- Classification SHA-256: `b1dd3b199461b325209a084ef9e32b12c0ee357ececdfdf5da8e5399b12df38c`.
- Result hashes:
  - 129/2^-7: `25a72da926da991a977cfd7490bebc7b3fd91165bbe0538719944ed8328ae24b`
  - 193/2^-7: `1828a1ad241d657e6a78d1707a7d7ade678a05ab3b1b417beedc132375948ff4`
  - 257/2^-7: `eed7d018bd5f7a3d92ad92fcef27bec2f28b9537960f962b5a3b645e905f7b15`
  - 257/2^-8 control: `a04700d5f4cf9baebe67d184aed7c3741c90515b7db263e4361c5d0963503597`
- Every reference receipt artifact hash matches its cached file.
- Independent recomputation from saved raw observations plus admitted reference
  values agrees with every relevant classification level: **264 comparisons,
  maximum absolute difference 0.0 for both absolute and signed errors**.
- Fresh full campaign lifecycle verification was not attempted; this audit
  reproduces the saved receipts and classification mathematics only.

## R1: orientation, actual coefficients, and reproduced errors

Public positive flow is reversed into kernel order: public node `i` maps to
kernel node `N-1-i`, kernel `x[k] = (x_last - x[i]) as f32`, and nodal velocity
changes sign. Thus the physical outlet becomes kernel node 1 and the inlet
becomes kernel node N. The kernel `pe2` is negative.

Actual per-element values reconstructed from the driver's f32 mesh conversion
and actual f32 `q/theta/D` values (not nominal real arithmetic):

| Nodes | `pe2` range | First source `wc` | First stable `wc` | Max `|stable-source|` |
|---:|---:|---:|---:|---:|
| 129 | -0.0078125 | -0.00244140625 | -0.002604156034 | 1.62750e-4 |
| 193 | -0.005208373 to -0.005208313 | -0.00260925293 | -0.00173610135 | 8.73152e-4 |
| 257 | -0.00390625 | -0.001953125 | -0.00130208198 | 6.51043e-4 |

The 193-node mesh is the notable case: f32 coordinate rounding makes `pe2`
slightly non-uniform and the source `wc` varies from -0.002609 to -0.000961
across elements. A nominal uniform-mesh calculation would miss that.

Reproduced public baseline at `t=0.25` (signed errors are actual minus reference;
`inv` is inventory total; `outinst` is `boundary.instantaneous.1`; `outcum` is
`boundary.cumulative.1`):

| Nodes / dt | `linf` | inlet | outlet | interior | `inv` | `outinst` | `outcum` |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 129 / 2^-7 | 3.405995e-4 | 6.203775e-5 | 3.405995e-4 | 3.280814e-4 | 1.169068e-4 | -1.464578e-4 | -2.406088e-5 |
| 193 / 2^-7 | 2.019591e-4 | 1.162184e-4 | 2.019591e-4 | 1.963859e-4 | 1.264000e-4 | -8.684242e-5 | -1.442800e-5 |
| 257 / 2^-7 | 3.245659e-4 | 1.742137e-4 | 3.245659e-4 | 3.214282e-4 | 2.604184e-4 | -1.395633e-4 | -2.040009e-5 |
| 257 / 2^-8 | 1.571960e-4 | 2.108936e-5 | 1.571960e-4 | 1.540584e-4 | 4.184219e-5 | -6.759429e-5 | -1.129938e-5 |

New diagnostic-only dt=2^-8 completions at `t=0.25`:

| Nodes / dt | `linf` | inlet | outlet | interior | `inv` | `outinst` | `outcum` |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 129 / 2^-8 | 3.738589e-4 | 7.598524e-5 | 3.738589e-4 | 3.613407e-4 | 1.604325e-4 | -1.607593e-4 | -2.588900e-5 |
| 193 / 2^-8 | 2.288408e-4 | 7.926349e-5 | 2.288408e-4 | 2.232676e-4 | 1.130346e-4 | -9.840155e-5 | -1.592239e-5 |
| 257 / 2^-8 | 1.571960e-4 | 2.108936e-5 | 1.571960e-4 | 1.540584e-4 | 4.184219e-5 | -6.759429e-5 | -1.129938e-5 |

Observed `linf` rates:

| Time | dt=2^-7, 129->193 / 193->257 | dt=2^-8, 129->193 / 193->257 |
|---:|---:|---:|
| 0.25 | 1.288994 / -1.649123 | 1.210590 / 1.305375 |
| 0.5 | 1.161501 / -1.976446 | 1.202668 / 1.537457 |
| 1.0 | 0.886491 / -2.059940 | 1.140397 / 1.896305 |

The dt=2^-7 sign change disappears at dt=2^-8. This is evidence for a temporal
component interacting with the spatial error, not proof of asymptotic first-order
convergence (the finest dt=2^-8 pair is above 1 at late times).

## R3: isolated baseline/stable-weight comparisons

The Rust harness runs the exact `scalar_robin` case (including the
steady-plus-mode initial condition) through the public API. It writes
`diagnostic_runs.json` with three variants per configuration:

1. `public_baseline_no_upwind`: the actual public path. Recorded weight calls:
   **0** at every configuration.
2. `forced_upstream_source_weight`: diagnostic-only activation of the weight
   branch, preserving the source-ordered f32 expression.
3. `forced_upstream_stable_weight`: same activation, stable f64 expression cast
   to f32.

At the official dt=2^-7 configurations, forced source-weight versus stable-weight
changes any concentration, inventory, or outflow error by at most:

- 129: `1.192e-7`
- 193: `1.115e-5`
- 257: `7.153e-7`

Forced source-weight versus the public baseline changes errors by at most
`1.49e-6`, `1.11e-5`, and `4.17e-7` respectively. These changes are one to three
orders of magnitude below the fine-pair `linf` jump (`2.019591e-4` to
`3.245659e-4`, a change of `1.23e-4`).

One-step transport-operator residuals (interior rows, actual f32 coefficients,
`g0=g1=0`, so dense source terms are excluded):

| Nodes | max coefficient/RHS difference | max row-sum difference |
|---:|---:|---:|
| 129 | 3.814697e-5 | 2.861023e-6 |
| 193 | 1.869202e-4 | 1.907349e-6 |
| 257 | 1.411438e-4 | 0.0 |

The diagnostic also found a genuine conditioning sensitivity when the weight
branch is **forced on**, but not in the official path: at 257 nodes, dt=2^-8,
the forced source-weight run at `t=1` has `linf = 7.861242e-4` while the stable
run has `3.070221e-4`, and inventory-total errors differ by `5.592e-4`. This is
recorded as a reason not to declare the f32 expression harmless everywhere. It
does not explain the official dt=2^-7 failures because those runs never compute
`wc`.

## Causal reading

- **Small-Peclet f32 upstream-weight cancellation:** not supported as the cause
  of the 40 official findings. The weight is inactive in the public path; even
  when forced, its source-versus-stable perturbation at dt=2^-7 is too small to
  explain the non-monotonicity.
- **Outlet discretization:** partially relevant. The outlet is the maximum-error
  location at every dt, but the interior maximum is only `1e-6` to `1e-5` lower.
  The interior and inventory failures cannot be attributed to the outlet closure
  alone, and the outlet error itself becomes monotone when dt is halved.
- **Temporal error:** best supported. The full dt=2^-8 triplet is monotone at all
  sampled times while the dt=2^-7 triplet is not. The 129 and 193 errors also
  rise slightly when dt is halved, which is consistent with competing temporal
  and spatial error terms rather than a simple fixed-order improvement.
- **Accumulated assembly roundoff:** not supported as dominant. Stable-weight
  changes are small at dt=2^-7, and dt halving changes errors by `~1e-4`, much
  larger than the observed arithmetic sensitivity. A broader high-precision
  assembly comparison was not performed, so this is a bounded conclusion.

## Unresolved / limitations

- The public adapter's failure to carry `chemistry.upstream_weighting: true` into
  `compat_cfg.upwind` is a provenance/mapping question that this audit did not
  resolve or fix. It means the official verification runs do not match that
  declared case switch. This needs a separate source-parity review; do not change
  it silently.
- The forced-upstream dt=2^-8 257-node sensitivity shows that the coefficient
  expression can matter in a different numerical regime. That regime is not the
  official failing one.
- The one-step residual excludes dense source/reaction terms and endpoint rows.
  It isolates the interior transport-operator effect of `wc`, not the full
  assembled system.
- No independent f64/high-precision solver trajectory, new reference, extra
  precision mode, Example 3 run, or campaign rerun was performed.
- The classification reproduction covers the saved Robin batch and its reference
  receipts only; it does not re-verify the historical full campaign lifecycle.

## Smallest justified follow-up

1. Resolve the public-to-native upstream-weight mapping as a separate parity
   question: should `CASES_V2.json` `upstream_weighting: true` reach
   `compat_cfg.upwind`? If yes, that is a translation/capture defect, and any
   change requires the six-batch rerun and new Gate B described in the review
   memo. No production change is justified by this audit alone.
2. If temporal attribution is to be confirmed, run a bounded fixed-mesh time
   refinement (for example 193 nodes at dt=2^-7, 2^-8, 2^-9) and compare the
   observed temporal order. Do not add meshes or precision modes without
   approval.
3. Do not pursue a stable-weight production fix. It would not address the
   official failing path and is contradicted as the primary cause by the current
   evidence.

## Reproduction commands

```sh
cd /home/andrew/AI/hyd.rs
cargo test -p hydrus-core --lib robin_coefficient_error_audit -- --ignored --nocapture
PYTHONDONTWRITEBYTECODE=1 python3 tools/robin_diagnostic_audit.py
cargo test -p hydrus-core --lib
cargo fmt --check
```

Focused checks run: 255 library tests passed (2 ignored, including this audit);
the 6 transport tests passed; `cargo fmt --all -- --check` passed; `cargo clippy
--workspace --all-targets -- -D warnings` passed; and the Python tool test suite
completed successfully.

## Artifact hashes

| Artifact | SHA-256 |
|---|---|
| `.development/23-robin-audit/diagnostic_runs.json` | `e3ea0c42ac490ca803f48f657e6953724bcef281657232e234a602b05c048920` |
| `.development/23-robin-audit/reconciliation.json` | `19a1261e85a4ef798e4f142ce42e2b1713bb2e9fa55c8416abce445839b2bc11` |
| `tools/robin_diagnostic_audit.py` | `25f35ace84e1f237201a086654cfcf523829b8fd6004d9d7c31910e41eb3067a` |
| `crates/hydrus-core/src/solute_fortran_compat/robin_diagnostic.rs` | `f8271d5865111926d9f08b908a1e82cdb530e706028c5670a84a61b4e061d07d` |
| `crates/hydrus-core/src/solute_fortran_compat/transport.rs` | `dcdcc09434ab346d4f9c847c58925ff3c65c5510edc6a88a89b7f2118450d959` |
| `crates/hydrus-core/src/solute_fortran_compat/mod.rs` | `d6b37a0c3c10e7bba7cb4bf1f7225bc643bc723e3cd0b2b2ea695b7714cdd627` |

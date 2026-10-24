# Solute convergence evidence (Orchestration 22): error budgets, observed orders and PROPOSED thresholds

Synthesis only (task 22.5.1). No code changed, no solver or holdout row run, no bounds file written, no classifier verdict claimed. Every threshold below is a **proposal** for the D10 user gate; nothing here is approved, and `THRESHOLDS_V2.json` does not exist yet.

Source tags: `R:n`, `D:n`, `B:n`, `E:n`, `S:n` are line numbers in `planning/master/M5V_STUDY_22_{REACTIONS,DIFFUSION,ROBIN,EXAMPLE3,EXAMPLE3_SPACE}.md`. Numbers are copied from those rows (the table generator `.development/22.5.1/gen.py` parses them from the reports, so a quoted figure is the report figure).

## Basis and rules

- **No classifier verdict.** Every report states that the convergence classification is `inconclusive` for every finding (0 achieved, 0 failed) because no `solute-convergence-bounds-v1` source exists (R:156-159, D:108, 111, B:132-138, E:163-164, S:133-137). The classifier has no bounds source, so this document does not use the words achieved or failed for any finding.
- **Derivation source.** Proposals derive from the *observed absolute errors against the independent analytical or Laplace reference* in the study reports. Solver output is never used as an expected value (ADR 0004, 0009). The reference side is the uniformization series (reactions), the cosine/Robin analytic series, or the 400/650-digit Laplace inversion (Example 3).
- **ADR 0009 / 0006.** No threshold is widened to make a study result pass: each ceiling is computed from the rule in the next section applied to the observed error, and it is never revised afterwards. Thresholds are per field, from observed error and precision (ADR 0006). Conservation closure alone is not independent verification (ADR 0009).
- **Absolute or named-scale relative.** All proposals are **absolute** ceilings on the stated error metric, in the units of the field (concentration units, or mass for inventory/boundary). A relative form is admitted only with the scale the spec names (the reference norm of the same field and time), `scale = 0` giving `branch=zero` (absolute only), and `scale <=` reference-plus-rounding uncertainty giving `branch=near_zero` (absolute only), with no universal epsilon (`SOLUTE_VERIFICATION_SPEC.md:353-411`). Because the ceilings are absolute, no near-zero branch is needed for them. The reports give the relative errors and branches alongside (R, D, B, E, S tables).
- **Unassigned stays unmet** (`SOLUTE_VERIFICATION_SPEC.md:409`; plan Wave 8). A subset of approved ceilings cannot close M5-V.
- **CASES_V2 identity.** Studies 22.4.1 to 22.4.4 ran under `CASES_V2.json` SHA-256 `644c4845956d48cc27e0f45387a06317dbbf9aa671667617fc6487e4b3ad8a3d`; 22.4.5 changed it to `c236eb7cd42dee691a36787625865457df62c0bb4a72cb0388a786cbe1e1b43b` (S:8). A `THRESHOLDS_V2.json` must carry the current hash. The 22.4.4 time batch no longer verifies under the new tools (`--verify-only` exit 2, "batch recipe identity changed", S:25-26); its original exit was 0 (E:32).

### Verify status of the evidence batches (from the reports)

| Study | Batch | `--verify-only` exit | Source |
|---|---|---|---|
| 22.4.1 reactions | benchmark `fb343c78…`; convergence `b1961bbd…` | 0; 0 | R:24-25 |
| 22.4.2 diffusion | benchmark `620d5ab3…`; convergence `6ca52a89…` | 0 (`--batch`); 0 | D:30-31 |
| 22.4.3 Robin | benchmark `bd9a83dd…`; convergence `d8607f24…` | 0; 0 | B:21-27 |
| 22.4.4 Example 3 | component `f1ecd5da…`; time `0cf92fa6…`; bridge `9c2c0249…`; space `66f7e9ba…` | 0; 0; 2 (lifecycle `mapping_validation` failure); 2 (incomplete, space_control timed out) | E:27, 32 |
| 22.4.5 Example 3 space control | `df5e1531…` | 0; the copied 22.4.4 batches `66f7e9ba…`, `0cf92fa6…` exit 2 under the new casebook | S:24-28 |

Example 3 space-axis 251/501/1001 numbers come from a scratch replay (`.development/22.4.5/space_old.py` against an unedited archive), **not** from authenticated batch evidence (S:28). Every Example 3 space proposal below carries that caveat.

## Margin rule and holdout-level candidate formula

**Margin rule (proposal, approved by the coordinator as the stated rule).** For an observed absolute error `e` at a study level: `e2 = e` rounded **up** to 2 significant figures; ceiling `= 1.5 * e2`, rounded **up** to 2 significant figures. The 1.5 factor is a margin for run-to-run and platform variation of the f32 solver; it is not derived from a measured variance (the studies ran each row once, apart from the repeatability evidence cited in `PHASE_5_SOLUTE_EVIDENCE.md`). The rule is applied once, uniformly, and is not revised after seeing any result (ADR 0009). A ceiling at a level is a ceiling for the error of *that mesh/dt configuration*, not an accuracy claim about the continuum limit.

**Holdout-level candidate (derived estimate, not measured).** Holdout rows are not run and must not be (D10 gate). For a space axis with mesh width `h = 1/(N-1)` the candidate for the holdout mesh `N_h` is the log-log interpolation/extrapolation from the two finest study levels `(h_c, e_c)` and `(h_f, e_f)`: `p = ln(e_c/e_f) / ln(h_c/h_f)`, `e_h = e_f * (h_h/h_f)^p`. This is an estimate from observed study errors, with no solver output used as an expected value. It is quoted **only** for a single axis at the study value of the other axis. The holdout meshes combine a new mesh *and* a new dt (diffusion_cosine 97 nodes / dt 0.0078125; scalar_robin 193 / 0.0078125; example3 751 / dt 0.04), and in every case the other axis is not isolated by the studies (see each case). A combined holdout candidate is therefore **unassigned** unless stated otherwise; the single-axis estimate is shown only as a labelled partial.

## Reactions (22.4.1): `reaction_distinct`, `reaction_equal`, `reaction_zero`

### Error budget

| Component | Value | Source |
|---|---|---|
| Discretization (backward Euler in dt) | `profile_linf` 5.1e-4 (S1) to 2.8e-3 (S2) at dt 0.03125, t=1..4; halves per dt halving | R:34-36 |
| Reference (uniformization series tail and roundoff) | 1e-16 to 4e-16, far below every error | R:162 |
| Quadrature | nil: 9-node trapezoid is exact for the spatially uniform fields; mesh-weight and exact-continuum inventory coincide | R:30 |
| Spatial sensitivity | 17-node control reproduces the 9-node finest level to the printed digits for every species and metric (q = D = 0) | R:153 |
| f32 floor | about 1e-7 relative; smallest resolved error about 3e-4 (S3), so the floor is not reached and no plateau or sign change in S1/S2 profile errors | R:163 |

### Observed orders and confidence

`profile_linf` local orders `log2(e(dt)/e(dt/2))` for `reaction_distinct`, pair (0.125 to 0.0625, 0.0625 to 0.03125), t=1,2,4, are in the table below (S1 0.990 to 0.994, S2 0.976 to 0.995, S3 0.938 to 0.997). All errors are positive, decreasing and resolved, and the orders rise toward 1 as dt shrinks. **Confidence statement:** consistent with first order in dt (backward Euler), descriptive only; the classifier marked every finding `inconclusive` (1050 findings per group, bounds missing, R:156-159), so no classifier-accepted fit exists. No order is fitted through a floor or plateau because none is present. The S3 coarse-pair orders of 0.938 to 0.978 at t=1,2 are pre-asymptotic (three-term chain), not a plateau (R:153).

### Per-level ceilings (proposed), `reaction_distinct` `profile_linf`

Levels dt = 0.125 / 0.0625 / 0.03125 (9 nodes). Ceilings by the margin rule. Only the dt = 0.03125 ceilings are intended as the acceptance candidates; the coarser rows document the rule.

| case | sp | t | e(0.125) | e(0.0625) | e(0.03125) | ceil(0.125) | ceil(0.0625) | ceil(0.03125) | local orders | source |
|---|---|---|---|---|---|---|---|---|---|---|
| reaction_distinct | S1 | 1 | 2.02e-03 | 1.02e-03 | 5.10e-04 | 3.20e-03 | 1.70e-03 | 7.80e-04 | 0.990, 0.992 | R:109,94,34 |
| reaction_distinct | S1 | 2 | 3.31e-03 | 1.66e-03 | 8.36e-04 | 5.10e-03 | 2.60e-03 | 1.30e-03 | 0.990, 0.993 | R:110,95,35 |
| reaction_distinct | S1 | 4 | 4.44e-03 | 2.23e-03 | 1.12e-03 | 6.80e-03 | 3.50e-03 | 1.80e-03 | 0.992, 0.994 | R:111,96,36 |
| reaction_distinct | S2 | 1 | 9.01e-03 | 4.58e-03 | 2.31e-03 | 1.40e-02 | 6.90e-03 | 3.60e-03 | 0.976, 0.988 | R:114,99,39 |
| reaction_distinct | S2 | 2 | 1.10e-02 | 5.56e-03 | 2.80e-03 | 1.70e-02 | 8.40e-03 | 4.20e-03 | 0.980, 0.990 | R:115,100,40 |
| reaction_distinct | S2 | 4 | 7.06e-03 | 3.56e-03 | 1.78e-03 | 1.10e-02 | 5.40e-03 | 2.70e-03 | 0.989, 0.995 | R:116,101,41 |
| reaction_distinct | S3 | 1 | 2.83e-03 | 1.46e-03 | 7.41e-04 | 4.40e-03 | 2.30e-03 | 1.20e-03 | 0.957, 0.978 | R:119,104,44 |
| reaction_distinct | S3 | 2 | 1.60e-03 | 8.34e-04 | 4.26e-04 | 2.40e-03 | 1.30e-03 | 6.50e-04 | 0.938, 0.969 | R:120,105,45 |
| reaction_distinct | S3 | 4 | 3.60e-03 | 1.81e-03 | 9.09e-04 | 5.60e-03 | 2.90e-03 | 1.40e-03 | 0.991, 0.997 | R:121,106,46 |

### `reaction_equal` `profile_linf` at dt = 0.03125 (benchmark row only; no time-refinement rows were run for this case)

| case | sp | t | e(0.03125) | ceiling | source |
|---|---|---|---|---|---|
| reaction_equal | S1 | 1 | 5.10e-04 | 7.80e-04 | R:49 |
| reaction_equal | S1 | 2 | 8.36e-04 | 1.30e-03 | R:50 |
| reaction_equal | S1 | 4 | 1.12e-03 | 1.80e-03 | R:51 |
| reaction_equal | S2 | 1 | 1.83e-03 | 2.90e-03 | R:54 |
| reaction_equal | S2 | 2 | 2.67e-03 | 4.10e-03 | R:55 |
| reaction_equal | S2 | 4 | 2.68e-03 | 4.10e-03 | R:56 |
| reaction_equal | S3 | 1 | 5.41e-04 | 8.30e-04 | R:59 |
| reaction_equal | S3 | 2 | 6.65e-04 | 1.10e-03 | R:60 |
| reaction_equal | S3 | 4 | 2.97e-04 | 4.50e-04 | R:61 |

S1 is identical in `reaction_equal` and `reaction_distinct` (same parent rate, R:169). No fitted order is available for `reaction_equal`: the order statement for the chain applies only by analogy and is not claimed.

### `reaction_distinct` `inventory.total` absolute error at dt = 0.03125 (proposed)

| case | sp | t | e(0.03125) | ceiling | source |
|---|---|---|---|---|---|
| reaction_distinct | S1 | 1 | 8.78e-04 | 1.40e-03 | R:34 |
| reaction_distinct | S1 | 2 | 1.44e-03 | 2.30e-03 | R:35 |
| reaction_distinct | S1 | 4 | 1.93e-03 | 3.00e-03 | R:36 |
| reaction_distinct | S2 | 1 | 1.98e-03 | 3.00e-03 | R:39 |
| reaction_distinct | S2 | 2 | 2.41e-03 | 3.80e-03 | R:40 |
| reaction_distinct | S2 | 4 | 1.53e-03 | 2.40e-03 | R:41 |
| reaction_distinct | S3 | 1 | 9.56e-04 | 1.50e-03 | R:44 |
| reaction_distinct | S3 | 2 | 5.49e-04 | 8.30e-04 | R:45 |
| reaction_distinct | S3 | 4 | 1.17e-03 | 1.80e-03 | R:46 |

### `reaction_zero`

All errors are exactly 0 for every species and field, with `branch=zero` and `temporal_l2 = 0` (R:62-76, 164). **Proposed: absolute error 0 (exact retention of the initial state)** for `profile_linf` and inventory. The derivation is the analytic identity `m' = 0` (the reference is the initial state), not the solver result; it is a proposal because an exact-zero ceiling has no margin and a single f32 non-identity would fail it.

### Unassigned in this case

- `reaction.production` and `reaction.loss` rates and `balance`: the reports list `near_zero` and `zero` branches for several S2/S3 entries (for example `balance` S2 t=1 `near_zero`, R:39), and no independent reference tolerance for the integrated rates was studied. **Unassigned: near-zero or zero reference scale with signs that change between times (chain transient), no resolved relative error.**
- `reaction_equal` order statements: unassigned (one level).

### Holdout-level candidate

None: the reaction cases have no holdout row in the plan (the three v2 holdouts are diffusion_cosine, scalar_robin, example3).

## Diffusion cosine (22.4.2)

### Error budget

| Component | Value | Source |
|---|---|---|
| Discretization (space, dt 2.44e-4) | `linf` 1.353e-2 / 6.236e-3 / 2.893e-3 at t=1 for 17/33/65 nodes; errors concentrate at the endpoints (`endpoint_linf = linf`) | D:44-52 |
| Time sensitivity | space control (65 nodes, half dt) differs from space_65 by 2.6 percent in `linf` at t=1 (2.817e-3 vs 2.893e-3) | D:102 |
| Reference | 1e-16 to 1e-17; rounding uncertainty 0; no coordinate uncertainty (exact request mesh) | D:109 |
| Quadrature | `inv_mesh_quadrature` exactly 0 at the reported times, so the inventory error is solution mass loss, not a quadrature artefact | D:71-79, 96 |
| f32 floor | about 1e-7 relative, 3 to 4 orders below the smallest errors; not reached on the space axis | D:108 |

### Observed orders and confidence

Space local orders (last two columns) are in the table: about 1.1 to 1.4 per halving in `linf`/`l2` (ratio about 2.2 in `linf` at t=1), which is below the second order that smooth interior centered diffusion may show; the spec states no order for this case and notes endpoint conditions and global inventory can reduce it (D:115). **Confidence statement:** descriptive local orders from three resolved levels with a 2.6 percent control; not classifier-accepted (inconclusive, 175 informative findings, 0 achieved). The decreasing endpoint-dominated error is consistent with the candidate defect section below. No temporal order is claimed (see Unassigned).

### Per-level ceilings (proposed), space axis

| metric | t | e(17) | e(33) | e(65) | ceil(17) | ceil(33) | ceil(65) | local orders | source | space-only e(97), derived |
|---|---|---|---|---|---|---|---|---|---|---|
| linf | 0.25 | 9.06e-03 | 3.87e-03 | 1.76e-03 | 1.40e-02 | 5.90e-03 | 2.70e-03 | 1.23, 1.14 | D:44,47,50 | 1.11e-03 |
| l2 | 0.25 | 2.03e-03 | 8.02e-04 | 3.51e-04 | 3.20e-03 | 1.30e-03 | 5.40e-04 | 1.34, 1.19 | D:44,47,50 | 2.17e-04 |
| linf | 0.5 | 1.12e-02 | 4.99e-03 | 2.31e-03 | 1.80e-02 | 7.50e-03 | 3.60e-03 | 1.16, 1.11 | D:45,48,51 | 1.47e-03 |
| l2 | 0.5 | 2.85e-03 | 1.21e-03 | 5.43e-04 | 4.40e-03 | 2.00e-03 | 8.30e-04 | 1.24, 1.16 | D:45,48,51 | 3.39e-04 |
| linf | 1.0 | 1.35e-02 | 6.24e-03 | 2.89e-03 | 2.10e-02 | 9.50e-03 | 4.40e-03 | 1.12, 1.11 | D:46,49,52 | 1.85e-03 |
| l2 | 1.0 | 4.05e-03 | 1.80e-03 | 8.09e-04 | 6.20e-03 | 2.90e-03 | 1.30e-03 | 1.17, 1.15 | D:46,49,52 | 5.07e-04 |

Proposed acceptance candidates are the 65-node ceilings; the 17/33 rows document the rule. The last column is the derived log-log extrapolation of the space part to the 97-node holdout mesh from the 33 and 65 rows (formula above); it is **not measured and not a threshold**.

### Holdout-level candidate (97 nodes, dt 0.0078125)

**Unassigned.** The holdout dt (0.0078125) is 32 times the study space-axis dt (2.44e-4), the time axis at 257 nodes is non-monotone and spatially contaminated (below), so the temporal part of the holdout error is not estimated. The space-only estimate in the table is not a combined ceiling.

### Unassigned in this case

- **`inventory.total` (candidate known defect): unassigned: known defect candidate.** Errors for 17/33/65 nodes are in the next table; see the candidate-defect section. No ceiling is proposed so that no threshold is set to encode a suspected defect (ADR 0009).
| t | |inv err| 17 | 33 | 65 | local orders | source |
|---|---|---|---|---|---|
| 0.25 | 1.31e-03 | 4.95e-04 | 1.87e-04 | 1.41, 1.41 | D:71,74,77 |
| 0.5 | 2.20e-03 | 8.91e-04 | 3.44e-04 | 1.31, 1.37 | D:72,75,78 |
| 1.0 | 3.76e-03 | 1.59e-03 | 6.15e-04 | 1.24, 1.37 | D:73,76,79 |

- **Time axis:** unassigned. At 257 nodes `linf` at t=1.0 is 3.778e-4 / 5.753e-4 / 6.961e-4 for dt 0.0625 / 0.03125 / 0.015625 (error grows as dt shrinks, `resolved_last_decrease` negative) and the 513-node control at dt 0.015625 gives 3.265e-4, below every 257-node row. The spatial error at 257 nodes is as large as the temporal signal, so the temporal error is not isolated (D:103-104).
- **Boundary cumulative and reaction errors:** exactly 0 with `scale = 0`, `branch=zero`, `relative=null` (D:38). **Proposed: absolute 0**, derived from the analytic zero truth (Neumann, no reaction), not from solver output. This is a proposal with no margin, as for `reaction_zero`.
- **`balance`:** unassigned. Reported 0 (`branch=zero`) while the inventory loses mass, so the balance residual is not an independent check of inventory (ADR 0009).
- `temporal_l2` (space 3.196e-3 / 1.386e-3 / 6.214e-4, time 1.567e-4 / 1.347e-4 / 1.553e-4, D:44-52): unassigned, as the time axis is.

## Scalar Robin (22.4.3)

### Error budget

| Component | Value | Source |
|---|---|---|
| Discretization (space, dt 2^-8) | `concentration.linf` at t=1: 2.427e-3 / 1.196e-3 / 8.407e-4 for 33/65/129 nodes | B:152 |
| Time contamination | space control (129 nodes, half dt) `linf` error 1.41e-4 at t=1, so the finest-level dt error is not small next to the 129-node space error | B:128 |
| Reference | about 5e-16 (analytic), not limiting; t=0 errors 1e-16 to 2e-16 (`reference_floor`) | B:135-136 |
| Quadrature | `inventory.total` t=0 rate about 2 (initial-condition quadrature); no separate quadrature term table in the report | B:130 |
| f32 floor | about 1e-7 relative, not reached (smallest nonzero errors 1e-5 to 1e-4); no plateau explains the non-monotone time axis | B:134 |

### Observed orders and confidence

Space local orders `(33 to 65, 65 to 129)` for `concentration.linf` are 1.04 / 1.03 / 1.02 then 0.72 / 0.61 / 0.51 at t=0.25 / 0.5 / 1.0 (table below). The first pair is about first order (spec: at most first order in space); the second pair drops because the dt = 2^-8 temporal error is no longer small next to the 129-node space error. **Confidence statement:** the first-pair order is descriptive only, the finest level is time-contaminated, and no order is claimed for the finest level; classifier verdict inconclusive (175 informative findings per axis, bounds missing).

### Per-level ceilings (proposed), space axis, resolved and unflagged fields

| metric | t | e(33) | e(65) | e(129) | ceil(33) | ceil(65) | ceil(129) | observed rates | flags | source |
|---|---|---|---|---|---|---|---|---|---|---|
| concentration.linf | 0.25 | 1.26e-03 | 6.14e-04 | 3.74e-04 | 2.00e-03 | 9.30e-04 | 5.70e-04 | 1.038, 0.715 | none | B:60 |
| concentration.linf | 0.5 | 1.74e-03 | 8.56e-04 | 5.61e-04 | 2.70e-03 | 1.30e-03 | 8.60e-04 | 1.026, 0.612 | none | B:61 |
| concentration.linf | 1.0 | 2.43e-03 | 1.20e-03 | 8.41e-04 | 3.80e-03 | 1.80e-03 | 1.30e-03 | 1.021, 0.509 | none | B:62 |
| boundary.cumulative.1 | 0.25 | 9.40e-05 | 4.50e-05 | 2.59e-05 | 1.50e-04 | 6.80e-05 | 3.90e-05 | 1.062, 0.798 | none | B:45 |
| boundary.cumulative.1 | 0.5 | 2.57e-04 | 1.25e-04 | 7.65e-05 | 3.90e-04 | 2.00e-04 | 1.20e-04 | 1.042, 0.705 | none | B:46 |
| boundary.cumulative.1 | 1.0 | 7.10e-04 | 3.48e-04 | 2.29e-04 | 1.10e-03 | 5.30e-04 | 3.50e-04 | 1.030, 0.603 | none | B:47 |

`concentration.linf` equals `endpoint_linf` in these rows (B:60-62 and the endpoint rows), so one ceiling covers both. The finest-level (129-node) ceilings are the proposed acceptance candidates; they inherit the time contamination of that level, which is why the ceilings are stated as ceilings on the total error at (129 nodes, dt 2^-8) and not as discretization-only bounds. Boundary cumulative outflow has `branch=resolved` with 1.03/0.60 observed rates at t=1 (B:45-47).

### Fields with no proposed ceiling (flagged by the tool)

| metric | t | e(33) | e(65) | e(129) | observed rates | flags | source |
|---|---|---|---|---|---|---|---|
| concentration.l2 | 1.0 | 8.986e-04 | 4.435e-04 | 4.204e-04 | 1.019, 0.077 | control_contamination | B:59 |
| concentration.l1 | 1.0 | 6.351e-04 | 3.185e-04 | 3.848e-04 | 0.995, -0.273 | nonmonotonicity | B:56 |
| inventory.total | 1.0 | 8.528e-04 | 4.193e-04 | 4.985e-04 | 1.024, -0.250 | nonmonotonicity | B:71 |

All three: **unassigned**. `concentration.l1` and `inventory.total` are non-monotone in space (`nonmonotonicity`), `concentration.l2` carries `control_contamination` (B:59, 56, 71).

### Time axis (257 nodes, dt 0.0625 / 0.03125 / 0.015625): unassigned

| metric | t | e(dt1) | e(dt2) | e(dt3) | observed rates | flags | source |
|---|---|---|---|---|---|---|---|
| concentration.linf | 1.0 | 7.449e-04 | 1.061e-03 | 2.839e-04 | -0.511, 1.903 | control_contamination,nonmonotonicity | B:106 |
| inventory.total | 1.0 | 7.508e-04 | 1.056e-03 | 1.188e-04 | -0.492, 3.152 | control_contamination,nonmonotonicity | B:115 |

Not monotone in this window; the 513-node control at dt 0.015625 has `linf` error 1.10e-3 against 2.8e-4 at 257 nodes, so the spatial and temporal errors partially cancel and every time-axis field carries `control_contamination` (B:128-129). Temporal error is not isolated: **unassigned**. Observed time-axis rates up to 3.2 on `inventory.total` come from cancellation and are not orders.

### Other Robin fields

- `boundary.cumulative.0`: errors exactly 0, flagged `zero_error` (B:42). The report flags `reference_floor`, `plateau_or_uncertainty_overlap` and `zero_error` and does not state the reference scale. **Unassigned: reference scale for this field is not given in the report.**
- `balance`: `near_zero` branch (errors 3.692e-4 / 1.818e-4 / 1.895e-4 at t=0.25 and nonmonotone, B:39). **Unassigned: near-zero reference scale and non-monotone.**

### Holdout-level candidate (193 nodes, dt 0.0078125)

**Unassigned.** The finest-level `linf` second order (0.51) shows time contamination, so a log-log extrapolation of the space part would not be a valid single-axis estimate, and the time axis is non-monotone. No number is derived.

## Example 3 chain (22.4.4 time axis, 22.4.5 space axis)

### Error budget

| Component | Value | Source |
|---|---|---|
| Reference (Laplace inversion, 400-digit N=192/256/384 plus 650-digit stage at 1001 nodes) | all 4233 (22.4.4) observables `admitted`, largest uncertainty/budget ratio 0.054; uncertainty at most 5.4e-9 (NO3 concentration norms t=200), about 1e-12 or smaller elsewhere; far below every resolved error | E:20, 170; S:129, 135 |
| Discretization (space, dt .025) | NH4 `linf` t=50 4.33e-2 / 1.27e-2 / 4.32e-3 for 251/501/1001 nodes | S:90 |
| Discretization (time, 1001 nodes) | NH4 `linf` t=200 8.89e-3 / 4.69e-3 / 2.49e-3 for dt .2/.1/.05 | E:66 |
| Control sensitivity | space control (1001 nodes, dt .0125) error is a third to a half of the 1001-node error; time control (501 nodes, dt .05) errors are comparable to or larger than the 1001-node dt .05 error for NH4/NO2 at t=50,100 (NH4 `linf` 8.26e-3 vs 6.20e-3) | S:130; E:164 |
| Quadrature | `empirical_quadrature_floor` marks 710 reaction findings; reaction rows are therefore not interpreted as resolved | S:135 |
| f32 floor | about 1e-7 relative of the unit inlet value is not reached by the concentration norms (smallest error 7.6e-6, NO3 endpoint); boundary cumulative outflow rows sit on the reference floor (`reference_floor`, 1577 and 1391 findings) and give no rates | S:135; E:173 |

### Observed orders and confidence

- **Space (dt .025):** NH4 and NO2 concentration norms decrease monotonically with local rates about 1.8 (251 to 501) and 1.1 to 1.55 (501 to 1001); the second rate is lower because the control error (dt .0125) is a third to a half of the 1001-node error, so time error is already comparable. **Confidence statement:** descriptive only; spatial order is inconclusive per spec :390-395, and the spec bound is at most first order for upwind.
- **Time (1001 nodes):** concentration rates 0.75 to 1.0 per halving (approach 1 from below), inventory 1.0 to 1.3; contaminated by the 501-node control, so **temporal error cannot be isolated: inconclusive** (E:163-164).

### Per-level ceilings (proposed), space axis, NH4 and NO2 concentration norms

Absolute ceilings on the error at (N nodes, dt .025) against the Laplace reference. Space-axis numbers are scratch replay, not authenticated batch evidence (S:28). `holdout-level` is the derived 751-node **space-only** partial at dt .025, from the 501 and 1001 rows (formula above), **not measured, not a combined holdout candidate**.

| metric | sp | t | e(251) | e(501) | e(1001) | ceil(251) | ceil(501) | ceil(1001) | local rates | flags | source | 751-node space-only, derived |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| concentration.linf | NH4 | 50 | 4.33e-02 | 1.27e-02 | 4.32e-03 | 6.60e-02 | 2.00e-02 | 6.60e-03 | 1.77, 1.55 | none | S:90 | 6.76e-03 |
| concentration.linf | NH4 | 100 | 2.52e-02 | 7.19e-03 | 2.77e-03 | 3.90e-02 | 1.10e-02 | 4.20e-03 | 1.81, 1.38 | none | S:91 | 4.12e-03 |
| concentration.linf | NH4 | 200 | 1.13e-02 | 3.24e-03 | 1.49e-03 | 1.80e-02 | 5.00e-03 | 2.30e-03 | 1.80, 1.12 | none | S:92 | 2.06e-03 |
| concentration.linf | NO2 | 50 | 1.62e-03 | 4.82e-04 | 1.77e-04 | 2.60e-03 | 7.40e-04 | 2.70e-04 | 1.75, 1.44 | none | S:93 | 2.68e-04 |
| concentration.linf | NO2 | 100 | 1.25e-03 | 3.69e-04 | 1.58e-04 | 2.00e-03 | 5.60e-04 | 2.40e-04 | 1.76, 1.22 | none | S:94 | 2.25e-04 |
| concentration.linf | NO2 | 200 | 7.16e-04 | 2.11e-04 | 1.08e-04 | 1.10e-03 | 3.30e-04 | 1.70e-04 | 1.76, 0.96 | none | S:95 | 1.43e-04 |
| concentration.l2 | NH4 | 50 | 5.81e-03 | 1.61e-03 | 5.97e-04 | 8.90e-03 | 2.60e-03 | 9.00e-04 | 1.85, 1.43 | none | S:81 | 9.01e-04 |
| concentration.l2 | NH4 | 100 | 3.92e-03 | 1.10e-03 | 4.70e-04 | 6.00e-03 | 1.70e-03 | 7.10e-04 | 1.83, 1.23 | none | S:82 | 6.69e-04 |
| concentration.l2 | NH4 | 200 | 2.05e-03 | 5.97e-04 | 3.15e-04 | 3.20e-03 | 9.00e-04 | 4.80e-04 | 1.78, 0.92 | none | S:83 | 4.11e-04 |
| concentration.l2 | NO2 | 50 | 2.41e-04 | 6.54e-05 | 2.48e-05 | 3.80e-04 | 9.90e-05 | 3.80e-05 | 1.88, 1.40 | none | S:84 | 3.71e-05 |
| concentration.l2 | NO2 | 100 | 2.09e-04 | 5.86e-05 | 2.66e-05 | 3.20e-04 | 8.90e-05 | 4.10e-05 | 1.84, 1.14 | none | S:85 | 3.69e-05 |
| concentration.l2 | NO2 | 200 | 1.35e-04 | 3.99e-05 | 2.32e-05 | 2.10e-04 | 6.00e-05 | 3.60e-05 | 1.77, 0.78 | none | S:86 | 2.91e-05 |

Proposed acceptance candidates are the 1001-node ceilings. They are ceilings on total error including the time contribution at dt .025 (the control at dt .0125 is a third to a half of the 1001-node error).

**Holdout-level candidate (751 nodes, dt 0.04): unassigned.** The holdout dt (0.04) differs from the study space-axis dt (.025) and the time axis is not isolated (control contamination), so the sum of a space and a time part is not a justified estimate. The single-axis partials in the table above are shown for transparency only.

### Unassigned in this case

NO3 concentration norms at t=200 (outlet region, large Peclet number): the tool flags contamination or non-monotonicity.

| metric | sp | t | e(251) | e(501) | e(1001) | local rates | flags | source |
|---|---|---|---|---|---|---|---|---|
| concentration.linf | NO3 | 200 | 1.53e-03 | 8.66e-04 | 7.40e-04 | 0.82, 0.23 | control_contamination | S:98 |
| concentration.l2 | NO3 | 200 | 4.05e-04 | 2.06e-04 | 2.38e-04 | 0.97, -0.21 | nonmonotonicity | S:89 |
| concentration.l1 | NO3 | 200 | 2.22e-04 | 9.36e-05 | 1.78e-04 | 1.25, -0.92 | nonmonotonicity | S:80 |

Inventory and balance (dt-dominated, sign changes, non-monotone in space; **unassigned**):

| metric | sp | t | e(251) | e(501) | e(1001) | local rates | flags | source |
|---|---|---|---|---|---|---|---|---|
| inventory.total | NH4 | 200 | 8.36e-03 | 1.59e-03 | 1.10e-02 | 2.40, -2.80 | nonmonotonicity,sign_change | S:101 |
| inventory.total | NO2 | 200 | 4.33e-04 | 1.55e-05 | 5.54e-04 | 4.80, -5.16 | nonmonotonicity,sign_change | S:104 |
| inventory.total | NO3 | 200 | 1.85e-03 | 5.19e-03 | 6.70e-03 | -1.48, -0.37 | control_contamination,nonmonotonicity,sign_change | S:107 |

- **Inventory / balance (all species):** unassigned. `inventory.total` NH4 is 2.7e-5, 2.65e-5, 2.69e-5 at t=1 for 251/501/1001 (flat in dx), with the dt .0125 control about half of that: error dominated by dt. At t=200 NH4 changes sign across levels and the balance residual has the same pattern (NH4 t=200: -1.25e-2, +1.97e-3, -1.57e-2; control 1.41e-3). Also non-monotone in dt (S:140-141; E:179).
- **Node-1 outflow closure:** unassigned. The outlet cumulative mass at t=200 is 5e-31 (NH4), 1.7e-19 to 2.9e-17 (NO2), 7.8e-11 to 1.5e-8 (NO3) against inventory totals 2.6 to 54; the outlet front lies beyond t=200, so closure loss is not observable on this horizon and cannot explain the inventory error (S:140).
- **Temporal-error claims:** unassigned (the time axis at 1001 nodes is contaminated by the 501-node control).
- **Outlet-tail `boundary.*.1`:** at t=50,100 the solver error equals the reference scale (1e-30 to 1e-113) for all dt (plateau, `reference_floor`); at t=200 NO3 outlet error 1.45e-8, 1.19e-9, 2.22e-10 is at or below the reference uncertainty (2.6e-9 cumulative, 1.3e-9 flux), so it is not a resolved error (E:171). **Unassigned; if ever assigned it must be absolute**, since the reference is below f32 resolution.
- **Bridge row (`legacy_bridge`):** lifecycle `mapping_validation` failure ("record inventory mismatch"), verify exit 2 (E:27, 32). No claim.
- **Reaction production/loss:** `empirical_quadrature_floor` flagged: unassigned.

### Time-axis observations (context only, no ceiling proposed)

| metric | sp | t | e(.2) | e(.1) | e(.05) | local rates | ctrl abs (501 nodes) | flags | source |
|---|---|---|---|---|---|---|---|---|---|
| concentration.linf | NH4 | 200 | 8.89e-03 | 4.69e-03 | 2.49e-03 | 0.92, 0.92 | 1.99e-03 | none | E:66 |
| concentration.linf | NO2 | 200 | 6.67e-04 | 3.48e-04 | 1.82e-04 | 0.94, 0.94 | 1.24e-04 | none | E:102 |
| concentration.linf | NO3 | 200 | 5.45e-03 | 2.85e-03 | 1.47e-03 | 0.94, 0.96 | 2.05e-04 | none | E:138 |
| inventory.total | NH4 | 200 | 1.78e-02 | 8.13e-03 | 3.20e-03 | 1.13, 1.34 | 1.02e-03 | none | E:69 |
| inventory.total | NO3 | 200 | 1.51e-02 | 7.31e-03 | 5.12e-03 | 1.05, 0.51 | 3.97e-04 | none | E:141 |


## Candidate known defect, not a pass: `diffusion_cosine` end-row mass loss

The Fortran-compatible path loses solution mass in `diffusion_cosine`: inventory error `|total|` 3.76e-3 / 1.59e-3 / 6.15e-4 at t=1.0 for 17/33/65 nodes (D:73,76,79), first order in dx (local orders 1.24 to 1.41 in the table above), with the mesh-quadrature term exactly 0, so it is not a quadrature artefact. The D report attributes it to the kernel end-row `c1 = c2` treatment (`SOLUTE.FOR:803-827`, "confirmed by the coordinator", D:115). **This document records it as a candidate known defect, not as a pass and not as a confirmed defect.** Reachability of that branch for the deck selectors (ADR 0007) was not re-proved here: only the D report, not a trace, is the source. It is not registered in `docs/FORTRAN_SOURCE_DEFECTS.md` because that file is outside this task's scope; see Discovered. The inventory field of this case stays unassigned.

## Metrics lacking a resolved error

| Case / metric | Why no resolved error | Source |
|---|---|---|
| Example 3 `inventory.*`, `balance` (all species, all times) | dt-dominated at dt .025; sign changes and non-monotone in space and dt; space control comparable | S:140-141; E:179 |
| Example 3 node-1 outflow closure | outlet front beyond t=200: closure loss not observable | S:140 |
| Example 3 `boundary.*.1` (t=50,100; NO3 t=200) | solver error equals or is below reference scale/uncertainty (`reference_floor`) | E:171; S:135 |
| Example 3 NO3 `l1/l2/linf` t=200 | `control_contamination` / `nonmonotonicity` | S:80, 89, 98 |
| Example 3 temporal error (all fields) | 501-node control contaminates; capacity prevents a finer control (spec :384-394) | E:163-164 |
| Example 3 bridge row | lifecycle failure (`mapping_validation`) | E:27, 32 |
| Example 3 space axis (all) | scratch replay of the old batch; not `--verify-only` clean under the new casebook | S:24-28 |
| Example 3 reaction production/loss | `empirical_quadrature_floor` (710 findings) | S:135 |
| Diffusion `inventory.total` | candidate known defect; first order in dx | D:73-79 |
| Diffusion time axis (257 nodes) | non-monotone; 513-node control lower than every 257-node row | D:103-104 |
| Diffusion `balance` | reported 0 while inventory is not conserved: not independent | D:38 |
| Robin `concentration.l1`, `concentration.l2`, `inventory.total` | `nonmonotonicity` / `control_contamination` | B:56, 59, 71 |
| Robin time axis | non-monotone; spatial error at 257 nodes comparable | B:106-115, ~127-128 |
| Robin `balance` | `near_zero`, non-monotone | B:39 |
| Reactions `balance` and rates | `near_zero`/`zero` branches in chain transient | R:39 |
| All cases | no bounds source (`arithmetic_bound_missing`, `quadrature_bound_missing`), so every finding is `inconclusive` | R:156-159 |

## M5-V claims

Style of `PHASE_5_SOLUTE_EVIDENCE.md:101-108`. "Status if approved" is the status each claim would have if D10 approves the proposals above and the holdout rows pass (22.7.1); until then every row is **unmet** because `THRESHOLDS_V2.json` does not exist and no holdout has run. Holdout-level ceilings are unassigned where noted, so those fields cannot pass a holdout.

| M5-V claim | Status if the proposals are approved |
|---|---|
| Independent analytical reference for closed reactions, time axis (distinct/equal/zero) | Would be **met for `profile_linf` at the listed species/times on `reaction_distinct`/`reaction_equal`** (one level for `equal`); no holdout row exists for it; inventory and reaction rates remain **unassigned** |
| Diffusion cosine space accuracy (`linf`, `l2`) | **Conditionally met** at 65 nodes, dt 2.44e-4 only; the 97-node/dt 0.0078125 holdout candidate is **unassigned**, so the holdout cannot confirm it |
| Diffusion cosine inventory / mass conservation | **Unmet: unassigned, known defect candidate** |
| Scalar Robin space accuracy (`linf`, boundary outflow) | **Conditionally met** at 129 nodes, dt 2^-8 only; holdout candidate **unassigned**; `l1`, `l2`, inventory **unassigned** |
| Temporal convergence (diffusion, Robin, Example 3) | **Unmet**: not isolated on any non-reaction case |
| Example 3 full-chain NH4/NO2 concentration norms | **Conditionally met** at 1001 nodes, dt .025 (scratch-replay evidence); NO3, inventory, balance, node-1 closure, outlet tail, reaction rates **unassigned** |
| Example 3 inventory, balance, node-1 closure, temporal error | **Unmet: unassigned** |
| Classifier verdict (achieved/failed) | **Unmet**: no bounds source; every finding `inconclusive` |
| M5-V overall | **Partial; open.** Unassigned fields remain unmet (`SOLUTE_VERIFICATION_SPEC.md:409`) and the roadmap does not unblock M6 (plan Wave 8) |

## Spot-check list

Each figure below is in the cited report row; run `.development/22.5.1/gen.py` to re-derive the tables.

- Reactions S1 `profile_linf` t=1, dt .125/.0625/.03125: 2.016e-3, 1.015e-3, 5.104e-4 (R:109, 94, 34).
- Reactions S2 `profile_linf` t=2, dt .03125: 2.797e-3 (R:40).
- Reactions S3 `reaction_equal` t=4: 2.969e-4 (R:61).
- Diffusion `linf` t=1.0 for 17/33/65: 1.353e-2, 6.236e-3, 2.893e-3 (D:46, 49, 52; table).
- Diffusion inventory error t=1.0: 3.755e-3, 1.587e-3, 6.153e-4 (D:73, 76, 79).
- Diffusion time axis `linf` t=1.0, 257 nodes: 3.778e-4, 5.753e-4, 6.961e-4; 513-node control 3.265e-4 (D time rows; D:104).
- Robin `concentration.linf` t=1.0: 2.427e-3, 1.196e-3, 8.407e-4 (B:62, 152).
- Robin `inventory.total` t=1.0: 8.528e-4, 4.193e-4, 4.985e-4 (B:71).
- Robin time `concentration.linf` t=1.0: 7.449e-4, 1.061e-3, 2.839e-4 (B:106).
- Example 3 time NH4 `linf` t=200: 8.89e-3, 4.69e-3, 2.49e-3 (E:66); NO3 inventory t=100 1.35e-2, 6.76e-3, 3.58e-3 (E, spot-check list).
- Example 3 space NH4 `linf` t=50: 4.33e-2, 1.27e-2, 4.32e-3 (S:90); NO3 `linf` t=200 flagged contamination (S:98).
- Example 3 space NH4 `inventory.total` t=200: 8.36e-3, 1.59e-3, 1.10e-2 (S:101).
- Margin rule check: Robin `linf` t=1.0 e(129)=8.407e-4 gives 8.5e-4, then 1.3e-3 (B:62).

## Discovered

- Candidate Fortran source defect (not registered; outside this task's Must edit): `diffusion_cosine` end-row mass loss, `SOLUTE.FOR:803-827`, evidence D:73-79. Other sites computing the same quantity have not been checked here. Needs an `SD-NNN` entry and a reachability proof for the deck selectors from a task whose scope includes `docs/FORTRAN_SOURCE_DEFECTS.md`.
- The batch-recipe identity binds to the casebook SHA and to the checker file SHAs, so the 22.4.4 time batch (original exit 0) and the old space batch (original exit 2) cannot be re-verified after the 22.4.5 casebook edit (S:24-28). The Example 3 reference cache is likewise keyed to a per-file checker SHA (3 h recompute on any change).
- Holdout dt for diffusion and Robin (0.0078125) is larger than the study space-axis dt (2.44e-4 and 2^-8), and the Example 3 holdout dt (0.04) is larger than the space-axis dt (.025); no study isolates the temporal part, so combined holdout ceilings are unassigned until a time-isolating study exists.

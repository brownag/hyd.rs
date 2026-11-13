# M6 vapor heat-boundary acceptance (orchestration 26, task 26.6.1)

Status: **partial acceptance.** DF, DZ, FF, FZ (and existing DD) accepted for
the heat-boundary increment, with the exceptions below. **FD held open.**
`kTopT < -1` rejected with a trigger. `kTopT == 0` invalid. Sinusoidal top
rejected. G22.1 stays Circled until FD is resolved.

Basis: [M6_VAPOR_HEAT_BOUNDARY_AUDIT.md](M6_VAPOR_HEAT_BOUNDARY_AUDIT.md),
criteria v3 ([criteria.json](../../reference/vapor_fortran/criteria.json),
contract revision `0c4c743`, approved at GATE N1, 2026-11-12),
[STATE_26.md](../STATE_26.md). Commits: audit `d7ddf11`; producer `07cce6e`;
fixtures `cee7943`; config `a07cd92`; transport/adjust `573f816`; comparator
`c509fd4`; balance `df57561`; parity `072431e`.

## Evidence class

- Fixtures: reference-only (`pinned-fortran-trajectory`), built from a
  harness-rebuilt executable with SHA-256 `e6d69dd3…`. This does **not** match
  the earlier pinned `824179ab…`. Disclosed; the new hash is the provenance.
- Parity: Rust candidate against the fixtures, against criteria v3 only.
- Balance: internal consistency on a seeded synthetic mesh, not case inputs.

## Per-form decision

| Form | kTopT, kBotT | Source branch | Parity (Rust vs Fortran) | Balance | Discrimination | Decision |
|---|---|---|---|---|---|---|
| DD | >0, >0 | no vapor boundary term | G19 accepted | G19 | G19 | Accepted (existing) |
| DF | 1, -1 | bottom vapor term `TEMPER.FOR:220-232` | pass on/off | pass (worst 3.0e-7 heat, 2.8e-7 water) | head, moisture, temperature | **Accepted** |
| DZ | 1, 0 | no vapor boundary term; zero-gradient | pass on/off | pass (worst 1.9e-7 heat, 7.1e-7 water) | head, moisture, temperature | **Accepted** |
| FF | -1, -1 | top `HeatFl` + storage `:255` + bottom | pass on/off | pass (worst 1.9e-7 heat, 2.8e-7 water) | head, temperature; **moisture below 2x (exception)** | **Accepted, with disclosure** (see below) |
| FZ | -1, 0 | top `HeatFl` + storage `:255`; zero-gradient bottom | pass on/off | pass (worst 2.6e-7 heat, 3.9e-7 water) | head, temperature; **moisture below 2x (exception)** | **Accepted, with disclosure** |
| FD | -1, 1 | top `HeatFl` + storage `:255`; Dirichlet bottom | **v_liquid row fails (t=240, node 37, ratio 1.50)** | not in balance file | head, temperature, moisture (2x) | **HELD OPEN** |
| `kTopT < -1` | -2 etc. | `:255` omitted | not run | not run | not run | Rejected, trigger: a reference case plus a source decision on the omitted storage term |
| `kTopT == 0` | 0 | unassigned (`SetUpHeat` no branch) | n/a | n/a | n/a | Invalid; kept rejected (H3) |
| Sinusoidal top | `tPeriod` ≠ 0 | excluded | n/a | n/a | n/a | Rejected (excluded from scope) |

## Exceptions and disclosures

1. **FD held open.** One `v_liquid` row at t=240, node 37 exceeds its bound
   by 1.50x (candidate 4.156e-9, reference 3.630e-9, bound 3.50e-10). Other
   204 rows of that field and every other field pass. The constant-dt schedule
   was **ruled out** (source runs constant 0.1 h; schedule-matched run fails the
   same row). Cause traced to a **vapor-on head drift**: up to 5.7 cm at t=240
   for FD, and 4.4 cm for FF, against vapor-off controls that match to ~4e-9 cm.
   The head criterion (2% relative) still passes, so the drift is hidden by
   the head tolerance and exposed only by `v_liquid`. Root cause is not yet
   pinned in `heat/vapor/water.rs` or `heat/vapor/adjust.rs`. Trigger to accept
   FD: the vapor-on head drift is explained and either fixed or accepted by a
   user decision with a criterion change.
2. **FF carries the same head drift** (4.4 cm at t=240). Its `v_liquid` passes
   by margin. FF acceptance is therefore conditional on the same investigation.
   If the drift is found to be a real bug, FF must be re-opened.
3. **FF and FZ moisture discrimination below the 2x rule** (observed 0.0018
   against a threshold of 0.0021). Recorded exception in criteria v3; head and
   temperature discriminate. Tolerances are not widened.
4. **Vapor fields do not discriminate pairs by maxima.** The global maxima of
   `con_vh`, `con_vt`, `v_vapor` sit at the initial-state node and are identical
   across the five pairs. Vapor fields are checked for nonzero per case only;
   pair-level differences were judged on full fields in the parity evidence.
5. **Balance checks cover zero flux only.** The synthetic balance setup uses
   zero imposed flux top and bottom (the balance input carries no flux value).
   Nonzero-flux balance is **not** shown. The parity fixtures use `tT=0.0` and
   `tB=0.0` for flux forms, so nonzero flux magnitudes are also untested.
6. **Single-precision source constants (H7).** Vapor REAL constants in the
   source are single precision. Not separately quantified for these forms.
   The 18 isothermal flux rows from G19 remain explained, not matched.
7. **Operation-order grouping (26.3.2).** Endpoint and interior sums are grouped
   before adding, not accumulated in source order. ULP-level, under H7. Not
   separately measured.
8. **`NOD_INF.OUT` header carries a run timestamp**, so its file SHA-256 is not
   reproducible. Reproducibility checks compare parsed values.

## Not claimed

- G22.2 (water flux, free drainage), G22.3 (atmospheric switching), G20 (energy balance),
  G22.4–G22.6 (sink/root uptake, ConVh, enhancement off), G23 (temperature-dependent
  water). Campbell (SD-006) remains out of scope (H8).
- Nonzero-flux heat-boundary balance and magnitudes.
- Vapor-on head agreement with the reference beyond the head criterion.
- Acceptance of proposed ADR 0021.

## Next step (outside this orchestration)

A src-level investigation of the vapor-coupled water step (`heat/vapor/water.rs`,
`heat/vapor/adjust.rs`) to find the vapor-on head drift. Its outcome decides FD
and must be recorded before FF's conditional acceptance is final.

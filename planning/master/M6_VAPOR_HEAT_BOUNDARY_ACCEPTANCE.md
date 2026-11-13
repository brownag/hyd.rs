# M6 vapor heat-boundary acceptance (orchestration 26, task 26.6.1)

Status: **accepted with exceptions (2026-11-13, user decision).** DF, DZ, FF,
FZ and FD (and existing DD) accepted for the heat-boundary increment, with the
exceptions below. FD and FF carry a vapor-on head drift whose cause is identified:
single-precision (REAL*4) arithmetic in the pinned reference's vapor-coupled solve
(H7 precision class, proven 2026-11-13). It is recorded as a precision exception,
not a Rust defect. Criteria v3 are unchanged. `kTopT < -1` rejected
with a trigger. `kTopT == 0` invalid. Sinusoidal top rejected. G22.1 is
Completed with these exceptions.

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
| FD | -1, 1 | top `HeatFl` + storage `:255`; Dirichlet bottom | **v_liquid row fails (t=240, node 37, ratio 1.50); head drift 5.7 cm** | not in balance file | head, temperature, moisture (2x) | **Accepted with exceptions (user decision 2026-11-13)** |
| `kTopT < -1` | -2 etc. | `:255` omitted | not run | not run | not run | Rejected, trigger: a reference case plus a source decision on the omitted storage term |
| `kTopT == 0` | 0 | unassigned (`SetUpHeat` no branch) | n/a | n/a | n/a | Invalid; kept rejected (H3) |
| Sinusoidal top | `tPeriod` ≠ 0 | excluded | n/a | n/a | n/a | Rejected (excluded from scope) |

## Exceptions and disclosures

1. **FD accepted with a failing criterion row (disclosed).** One `v_liquid`
   row at t=240, node 37 exceeds its frozen bound by 1.50x (candidate
   4.156e-9, reference 3.630e-9, bound 3.50e-10). Other 204 rows of that field
   and every other field pass. The constant-dt schedule was **ruled out**
   (source runs constant 0.1 h; schedule-matched run fails the same row).
   The cause is a **vapor-on head drift** against the reference: 5.70 cm
   maximum absolute error at t=240 (node 4), against vapor-off controls that
   match to ~4e-9 cm. The head criterion (2% relative) still passes, so the
   drift is exposed only by `v_liquid`. Criteria v3 are not widened. Accepting
   FD is a user decision, not a passing criterion. The `v_liquid` row is
   recorded as a known failure.
   - **Mechanism: identified (2026-11-13,
     [D-26.2_DRIFT_MECHANISM.md](D-26.2_DRIFT_MECHANISM.md)).** Single-precision
     (REAL*4) arithmetic inside the pinned reference's vapor-coupled solve. The
     same Fortran source built with `real*8` defaults agrees with the Rust driver
     to 5e-9 cm (head) and 5e-10 °C (temperature) at every step for FD. The
     single-precision fixture differs from that double build by the 5.70 cm drift.
     The FD `v_liquid` row at node 37 agrees to 1e-14 with the double build; the
     fixture differs by 5.3e-10, which is the 1.50× failure. Ruled out earlier
     (with evidence in [STATE_26.md](../STATE_26.md)): driver machinery, constant-dt
     schedule, boundary rows, and Rust-side storage rounding (M2, M3a–c). The
     earlier D-26.1 claim that single precision was ruled out was wrong; that test
     did not change the reference arithmetic.
   - **Not isolated:** which REAL declaration amplifies the error. Not required
     for this acceptance.
   - **Evidence class:** the fixtures stay single-precision reference evidence.
     The Rust candidate is compared against them with the unchanged v3 criteria.
     The double-precision build is diagnostic only, not a new evidence class.
2. **FF carries the same head drift** (4.41 cm at t=240, absolute). Its
   `v_liquid` passes by margin. FF is accepted with the same disclosure. If the
   drift is found to be a real bug, FF and FD must be re-opened.
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

## Follow-up (optional, outside this acceptance)

- Isolate the amplifying REAL declaration (mixed-precision build of the
  vapor-coupled arrays). Diagnostic only.
- Decide whether to add a double-precision reference evidence class with new
  fixtures and a separate criteria decision. Not required for this acceptance.

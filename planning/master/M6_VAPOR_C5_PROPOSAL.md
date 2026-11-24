# M6 Vapor C5 Closed-Pair Criteria Proposal

Status: PROPOSAL pending explicit user approval (GATE A / D-9).
Scope: Closed-pair boundary discrimination criteria for `vapor_closed_flux` and `vapor_flux_known`.
Constraint: Non-modifying proposal. No changes are made to `reference/vapor_fortran/criteria.json`.

---

## 1. Executive Summary

This proposal provides the numerical evidence, headroom analysis, and candidate options for criteria entries governing closed-pair boundary conditions (`vapor_closed_flux` and `vapor_flux_known`) under Milestone 6 (G22.2-C5).

Currently in `reference/vapor_fortran/criteria.json` (criteria v6, commit `231aa46`):
- `closed_flux_on_off_temperature_c`: factor 2.0 (PASS, r_max 5.4000)
- `flux_known_on_off_temperature_c`: factor 2.0 (PASS, r_max 5.4000)
- `closed_flux_on_off_liquid_moisture`: factor 0.85 (PASS, r_max 0.8571)
- `closed_vapor_fields_nonzero`: cases `vapor_closed_flux_on` and `vapor_flux_known_on`, factor 1.0 (PASS)

Open decisions for GATE A:
1. **Head discrimination**: Closed-pair on/off head differences are 0.897 cm (`closed_flux`) and 0.877 cm (`flux_known`). Under the comparator tolerance denominator $0.5 + 0.02 \cdot |h_{\text{on}}| \approx 2.93\text{ cm}$, the maximum ratio $r_{\text{max}}$ is $\approx 0.30$. Three options (A, B, C) are presented neutrally below.
2. **Moisture discrimination for `flux_known`**: `flux_known_on_off_liquid_moisture` exhibits a maximum on/off difference of 0.0008, with $r_{\text{max}} = 0.7619$. Options to add it at factor 0.75, standardize both closed pairs to 0.75, or omit it are evaluated.
3. **Dry-regime status**: Confirmed omitted due to Fortran reference divergence under thermal gradient at -10000 cm (C3 finding).

---

## 2. Evidence and Tolerance Formulas

### 2.1 Comparator Rules

In `tools/compare_vapor_reference.py`, discrimination checks under `any_print_time_after_initial` compare reference trajectories between an active case ($r_{\text{on}}$) and a control ($r_{\text{off}}$) at all print times $t > 0$:

$$\text{tol} = \text{factor\_of\_tolerance} \cdot (\text{abs\_tol} + \text{rel\_tol} \cdot |r_{\text{on}}|)$$

A point $(t, n)$ passes discrimination if:

$$|r_{\text{on}} - r_{\text{off}}| > \text{tol}$$

The maximum passing factor for an entry is:

$$r_{\text{max}} = \max_{t>0, n} \frac{|r_{\text{on}} - r_{\text{off}}|}{\text{abs\_tol} + \text{rel\_tol} \cdot |r_{\text{on}}|}$$

An entry passes if and only if $\text{factor\_of\_tolerance} < r_{\text{max}}$.

### 2.2 Field Tolerance Specifications

From `reference/vapor_fortran/criteria.json` `fields`:
- `head`: absolute = 0.5 cm, relative = 0.02, scale = `abs_reference`
- `liquid_moisture`: absolute = 0.00105 $\text{cm}^3/\text{cm}^3$, relative = 0.0, scale = `none`
- `temperature_c`: absolute = 0.05 °C, relative = 0.0, scale = `none`

### 2.3 Measured On/Off Differences across Closed Pairs

Measurements extracted from reference fixtures `reference/vapor_fortran/fixtures/vapor_{closed_flux,flux_known}_{on,off}.json` across print times $t \in \{6, 24, 72, 240\}\text{ h}$:

| Case Pair | Field | Max On/Off Diff | Time ($t$) | Node | $r_{\text{on}}$ | $r_{\text{off}}$ | Denominator | $r_{\text{max}}$ |
|---|---|---|---|---|---|---|---|---|
| `vapor_closed_flux` | `head` | 0.8970 cm | 240.0 h | 0 | -121.626 cm | -120.729 cm | 2.9325 cm | 0.3059 |
| `vapor_closed_flux` | `liquid_moisture` | 0.000900 | 72.0 h | 0 | 0.2623 | 0.2632 | 0.001050 | 0.8571 |
| `vapor_closed_flux` | `temperature_c` | 0.2700 °C | 72.0 h | 16 | 28.94 °C | 28.67 °C | 0.0500 °C | 5.4000 |
| `vapor_flux_known` | `head` | 0.8770 cm | 240.0 h | 0 | -121.096 cm | -120.219 cm | 2.9219 cm | 0.3001 |
| `vapor_flux_known` | `liquid_moisture` | 0.000800 | 72.0 h | 0 | 0.2627 | 0.2635 | 0.001050 | 0.7619 |
| `vapor_flux_known` | `temperature_c` | 0.2700 °C | 240.0 h | 15 | 29.92 °C | 29.65 °C | 0.0500 °C | 5.4000 |

---

## 3. Options for Head Discrimination

Closed-pair head differences (~0.88–0.90 cm) fail under the conventional factor of 2.0 (threshold ~5.86 cm) and under factor 1.0 (threshold ~2.93 cm). Three options are available:

### Option A: Do not add head discrimination entries (Recommended as Proposal)
- **Description**: Omit head discrimination entries for `closed_flux` and `flux_known`. Head dynamics remain strictly verified by exact candidate-to-reference parity checks (`vapor_closed_flux_on_matches_fixture`, `vapor_closed_flux_off_matches_fixture`, `vapor_flux_known_on_matches_fixture`, and `vapor_flux_known_off_matches_fixture`).
- **Evidence**: On/off differences are 0.897 cm and 0.877 cm on suctions of ~-121 cm, yielding $r_{\text{max}} \approx 0.30$.
- **Precedent**: Follows the accepted precedent of `free_drain_on_off_head` (ORCH30, `M6_VAPOR_ORCH30_HEAD_CRITERIA_DRAFT.md`), which was dropped when $r_{\text{max}} = 0.9121 < 2.0$.
- **Pros**:
  - Maintains a rigorous standard for discrimination factors ($> 0.75$ or $2.0$) without introducing weak, dilute factors ($\le 0.30$).
  - Avoids modifying shared comparator logic or tolerances.
  - Parity against pinned Fortran fixtures guarantees complete head trajectory correctness.
- **Cons**:
  - No automated on/off head divergence metric checked inside `criteria.json`.

### Option B: Add head discrimination entries at a reduced factor ($\le 0.30$)
- **Description**: Add `closed_flux_on_off_head` and `flux_known_on_off_head` at `factor_of_tolerance = 0.30` (or 0.25).
- **Evidence**: At factor 0.30:
  - `closed_flux_on_off_head`: threshold = $0.30 \times 2.9325 = 0.8798\text{ cm} < 0.8970\text{ cm}$ (1 point above tolerance, PASS).
  - `flux_known_on_off_head`: threshold = $0.30 \times 2.9219 = 0.8766\text{ cm} < 0.8770\text{ cm}$ (1 point above tolerance, PASS).
- **Pros**:
  - Provides an active check in `criteria.json` requiring vapor-on and vapor-off heads to differ.
  - Conforms to existing validator schema without modifying `compare_vapor_reference.py`.
- **Cons**:
  - Factor 0.30 has very narrow margin (<2% margin at factor 0.30, or requires 0.25 for ~18% margin).
  - Creates a precedent of very low discrimination factors for physical fields.

### Option C: Use absolute-only head tolerance for discrimination
- **Description**: Change the head discrimination rule or scale to use the absolute tolerance (0.5 cm) alone without the 0.02 relative term.
- **Evidence**:
  - Max difference is 0.897 cm.
  - At factor 1.0 against absolute tolerance 0.5 cm, threshold = 0.5 cm ($r_{\text{max}} = 1.794$, PASS).
  - At factor 2.0 against absolute tolerance 0.5 cm, threshold = 1.0 cm ($0.897 < 1.0$, FAIL).
- **Pros**:
  - Prevents high background suction from inflating the discrimination threshold.
- **Cons**:
  - Requires structural modifications to `compare_vapor_reference.py` (custom discrimination scaling per field or per entry).
  - Still fails under the standard factor of 2.0 unless factor is lowered to $\le 1.75$.

---

## 4. Options for Liquid Moisture Discrimination

`closed_flux_on_off_liquid_moisture` was approved and written in criteria v6 at `factor_of_tolerance = 0.85` ($r_{\text{max}} = 0.8571$). For `flux_known`, the maximum moisture difference is slightly smaller (0.000800 vs 0.000900), giving $r_{\text{max}} = 0.7619$.

### Option M1: Add `flux_known_on_off_liquid_moisture` at factor 0.75 (Recommended as Proposal)
- **Description**: Add the entry at factor 0.75.
- **Evidence**: Threshold = $0.75 \times 0.00105 = 0.0007875 < 0.0008000$ (4 points above tolerance at $t=72\text{ h}$, nodes 0–3; PASS).
- **Pros**: Retains moisture discrimination on both wet closed pairs.
- **Cons**: Factors differ slightly between `closed_flux` (0.85) and `flux_known` (0.75).

### Option M2: Standardize both closed pairs to factor 0.75
- **Description**: Update `closed_flux_on_off_liquid_moisture` from 0.85 to 0.75, and add `flux_known_on_off_liquid_moisture` at 0.75.
- **Evidence**: Both pass with solid margin (factor 0.75 vs $r_{\text{max}} = 0.8571$ and $0.7619$).
- **Pros**: Uniform factor across all closed-pair moisture discrimination checks.
- **Cons**: Requires re-editing existing criteria entry and re-hashing criteria.

### Option M3: Omit `flux_known_on_off_liquid_moisture`
- **Description**: Retain `closed_flux_on_off_liquid_moisture` at 0.85 as the representative moisture check for closed boundaries; rely on exact fixture parity for `flux_known`.
- **Pros**: Zero edits to existing criteria.
- **Cons**: Asymmetric criteria coverage between `closed_flux` and `flux_known`.

---

## 5. Summary of Candidate JSON Entries

If the user approves adding any of the candidate entries at GATE A, the exact JSON entries conforming to `tools/compare_vapor_reference.py` schema are:

```json
{
  "closed_flux_on_off_head": {
    "case": "vapor_closed_flux_on",
    "control": "vapor_closed_flux_off",
    "factor_of_tolerance": 0.30,
    "field": "head",
    "rule": "any_print_time_after_initial",
    "unit": "cm"
  },
  "flux_known_on_off_head": {
    "case": "vapor_flux_known_on",
    "control": "vapor_flux_known_off",
    "factor_of_tolerance": 0.30,
    "field": "head",
    "rule": "any_print_time_after_initial",
    "unit": "cm"
  },
  "flux_known_on_off_liquid_moisture": {
    "case": "vapor_flux_known_on",
    "control": "vapor_flux_known_off",
    "factor_of_tolerance": 0.75,
    "field": "liquid_moisture",
    "rule": "any_print_time_after_initial",
    "unit": "cm3/cm3"
  }
}
```

---

## 6. Dry-Regime Analysis and Rationale

Milestone G22.2-C3 investigated whether dry-regime closed-pair fixtures could be generated at initial head -10000 cm (`planning/master/M6_VAPOR_C3_DRY_DESIGN.md`).

Key findings recorded in `planning/master/M6_VAPOR_COVERAGE.md`:
1. Under a 25 °C to 15 °C thermal gradient, the Fortran reference diverges silently (exits with code 0 while suction explodes to -614,025 cm at 24 h and full mesh NaN at 72 h).
2. The isothermal control at -10000 cm converges stably to 240 h with head held near -10000 cm.
3. Because Fortran cannot complete the thermal-gradient run in the dry regime, no valid Fortran thermal trajectory exists.

**Conclusion for C5**: No dry-regime closed-pair discrimination entries can be defined because discrimination requires comparing a converged thermal vapor-on case against a control. Dry isothermal conditions show negligible thermal vapor flow and produce zero discrimination on moisture and temperature. Therefore, dry closed-pair discrimination entries are formally omitted from the criteria specification.

---

## 7. Recommendation for GATE A

The following recommendation is submitted as a proposal for explicit user approval at GATE A:

1. **Head Discrimination**: Adopt **Option A** (do not add head discrimination entries for `closed_flux` or `flux_known`; rely on exact fixture parity tests).
2. **Moisture Discrimination**: Adopt **Option M1** (add `flux_known_on_off_liquid_moisture` at factor 0.75, or Option M3 if keeping criteria unchanged is preferred).
3. **Temperature Discrimination**: Maintain existing entries `closed_flux_on_off_temperature_c` and `flux_known_on_off_temperature_c` at factor 2.0 (already approved in criteria v6).
4. **Nonzero Vapor Fields**: Maintain existing `closed_vapor_fields_nonzero` at factor 1.0 (already approved in criteria v6).
5. **Dry Regime**: Confirm omission of dry closed-pair discrimination entries based on the C3 Fortran reference divergence record.

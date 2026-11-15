# M6 vapor G22.2 acceptance (orchestration 28, task 28.2.1)

Status: **GATE W1 decided, 2026-11-13 (user). Goal G22.2 stays Circled.** At least one form is
accepted for orchestration 29. No implementation, fixture or run was done in orchestration 28.

Basis: [M6_VAPOR_FREE_DRAINAGE_AUDIT.md](M6_VAPOR_FREE_DRAINAGE_AUDIT.md),
[M6_VAPOR_WATER_FLUX_AUDIT.md](M6_VAPOR_WATER_FLUX_AUDIT.md),
[M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md](M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md),
[G7_FREE_DRAINAGE_END_ROW_ASSESSMENT.md](G7_FREE_DRAINAGE_END_ROW_ASSESSMENT.md),
[STATE_28.md](../STATE_28.md), [coverage](M6_VAPOR_COVERAGE.md).

## Per-form decision

| Form or pair | Status | Answered at GATE W1 |
|---|---|---|
| Free drainage, lower (`FreeD`) | accepted for orchestration 29 | yes |
| Prescribed flux, upper (`KodTop=-1`) | accepted for orchestration 29 | yes |
| Prescribed flux, lower (`KodBot=-1`) | accepted for orchestration 29 | yes |
| Seepage face (`SeepF`) | rejected with trigger | yes |
| Water table (`qDrain`) | rejected with trigger | yes |
| Lower `qGWLF` (`KodBot=-7`) | confirmed rejected with trigger | confirmed by user 2026-11-14 (C2; was audit disposition) |
| `TopInF` / `BotInF` (+-3) | confirmed rejected with trigger | confirmed by user 2026-11-14 (C2; was audit disposition) |
| Atmospheric (`KodTop=-4`) | not decided here (G22.3) | **not explicitly answered** |
| Pair flux/flux (closed pair) | accepted and implemented (orchestration 29, `ClosedFlux`) | yes (follow the singles) |
| Pair flux/FixedHead (`FixedHeadLowerFluxUpper`) | accepted and implemented (`880c6c2`); wet parity passes; dry divergence recorded | yes (follow the singles) |
| Pairs flux/FreeD, head/FreeD | accepted for orchestration 29 | yes (follow free drainage) |
| Pairs with seepage face or water table | rejected with trigger | yes (follow the singles) |
| Pair flux/`qGWLF` | confirmed rejected with trigger | confirmed by user 2026-11-14 (C2; follows `qGWLF`) |

The user confirmed `qGWLF`, `TopInF`/`BotInF` and the flux/`qGWLF` pair on 2026-11-14 (see C2 decision below).
Atmospheric remains G22.3 (not decided here). Trigger text is in the coverage matrix rows, copied from the audits.

## Evidence per form (audit level only)

- `FreeD`: Fortran `WATFLOW.FOR:211` adds `ConVh` to `ConB` before `:213` `rBot`; Rust
  `lower_boundary.rs:161-182` uses liquid K only. Acceptance needs a bottom-row design matching
  `WATFLOW.FOR:211-229` and a balance where bottom exchange equals the vapor-augmented drainage flux.
- Upper and lower flux: `ConVh` and `ThV` enter the end rows (`WATFLOW.FOR:276,211`). Balance named:
  liquid plus vapor column storage change equals imposed flux times dt plus the other exchange.
- Seepage face, water table: nonlinear or head-dependent bottom conditions, rejected in the
  water-only path too (`lower_boundary.rs:108-116`).
- No case, fixture or balance check exists yet for any form. These are specified for orchestration 29.

## Gate disposition

G22.2 stays in Circled Goal: the exit criterion (a provenance-backed fixture and a balance check
for at least one accepted form) is not met and is deferred to orchestration 29. It is not closed.

## G7

Verdict: not established ([assessment](G7_FREE_DRAINAGE_END_ROW_ASSESSMENT.md)). No SD-007 entry is
made. Vapor-off free drainage is unaffected.

## Not claimed

- Any implemented, run or balance-checked water-flux or free-drainage vapor case.
- That Fortran `WATFLOW.FOR:211` is a defect, or that the Rust liquid-only free drainage is wrong.
- Atmospheric switching beyond the audit dispositions (G22.3; decisions on `qGWLF` and ±3 forms are resolved below).
- Anything about G22.3, G20, G23.

## ADR 0021 compliance

All decisions comply with ADR 0021 (heat-only, one-way water then heat; vapor deferred to
`M6_VAPOR_CONTRACT.md`). Its status (Proposed) is unchanged. G22.1 and G22.4 decisions and the
criteria are unchanged.

## C2 decision (user, 2026-11-14)

The user explicitly confirmed the rejection of `qGWLF`, `TopInF`/`BotInF` (±3), and the flux/`qGWLF` pair on 2026-11-14 as part of orchestration 30 planning (decision D2).

### User decision (verbatim)

> "Confirm rejection (Recommended)" for qGWLF and TopInF/BotInF (±3): keep rejected with their triggers.

### Confirmation, audit triggers, and code reject sites

1. **Lower `qGWLF` (`KodBot=-7`, head-dependent flux)**
   - **Status:** Confirmed rejected.
   - **Audit trigger quote (`M6_VAPOR_WATER_FLUX_AUDIT.md:48`):**
     > "Trigger: no Rust mode for `qGWLF`; needs a new mode and per-iteration `Fqh` update before any vapor work. Check if later accepted: exchange equals `Fqh(h-GWL0L)` at the accepted head, in the column balance. ADR 0021 compliant."
   - **Code reject site:**
     - `crates/hydrus-core/src/lower_boundary.rs:11-26`: `LowerBoundaryMode` defines only `FixedHead`, `PrescribedFlux`, `FreeDrainage`, and `Unsupported(SeepageFace | WaterTable)`. There is no enum variant for `qGWLF`.
     - `crates/hydrus-core/src/heat/vapor/mod.rs:210, 235-300`: `validate_water_input` validates boundary pairs via `VaporBoundaryPair::classify(input.lower, input.upper)`. Any unsupported lower boundary mode fails validation with `VaporError::UnsupportedLowerBoundary(lower)`.
     - Grep `qGWLF` across `crates/hydrus-core/src`: only matches a comment header in `crates/hydrus-core/src/solute_fortran_compat/deck_reader.rs:391`.

2. **Time-variable inputs: Upper `TopInF` / Lower `BotInF` (`KodTop=±3`, `KodBot=±3`)**
   - **Status:** Confirmed rejected.
   - **Audit trigger quote (`M6_VAPOR_WATER_FLUX_AUDIT.md:49`):**
     > "Trigger: time-variable input path not audited here; no Rust mode in the in-scope files."
   - **Code reject site:**
     - `crates/hydrus-core/src/water_solver.rs:83-97`: `UpperBoundary` defines only `Head`, `Flux`, and `Atmospheric`. There is no variant for `TopInF` (`±3`).
     - `crates/hydrus-core/src/lower_boundary.rs:11-26`: `LowerBoundaryMode` has no variant for `BotInF` (`±3`).
     - `crates/hydrus-core/src/heat/vapor/mod.rs:210, 235-300`: `validate_water_input` dispatches to `VaporBoundaryPair::classify(input.lower, input.upper)`. Any non-whitelisted upper or lower boundary fails with `VaporError::UnsupportedUpperBoundary` or `VaporError::UnsupportedLowerBoundary`.
     - Grep `TopInF` / `BotInF` across `crates/hydrus-core/src`: only matches the comment header in `crates/hydrus-core/src/solute_fortran_compat/deck_reader.rs:391`.

3. **Pair flux/`qGWLF`**
   - **Status:** Confirmed rejected (follows the confirmed rejection of `qGWLF`).

## C2 decision (2026-11-15, user)
The user confirmed the rejections that GATE W1 left unanswered. Both stay rejected with their existing triggers: `qGWLF` (groundwater-level-dependent bottom flux) and time-variable +-3 `TopInF`/`BotInF` inflow with vapor. No code change. This closes G22.2-C2.

## G22.2 flux/FixedHead acceptance and implementation (2026-11-15)

The flux/FixedHead water boundary pair (`FixedHeadLowerFluxUpper`: lower fixed head with upper prescribed flux) was implemented in `880c6c2`:
- **Kernel & solver acceptance:** `crates/hydrus-core/src/heat/vapor/mod.rs` accepts `(LowerBoundaryMode::FixedHead, UpperBoundary::Flux)` via `VaporBoundaryPair::FixedHeadLowerFluxUpper { upper_flux }`. The lower row retains the standard Dirichlet row from assembly, while the upper row receives the vapor flux row (`WATFLOW.FOR:288-291`) in `crates/hydrus-core/src/water_solver.rs`.
- **Wet fixture & parity:** The wet fixture `reference/vapor_fortran/fixtures/vapor_flux_fixedhead_wet_on.json` (bottom head -100 cm, 0-240 h) passes exact parity in `crates/hydrus-core/tests/vapor_fortran_parity.rs` (`flux_fixedhead_wet_on_matches_fixture`).
- **Dry divergence record:** The dry deck (-10000 cm) diverges with NaN by 72 h under graded temperature (exit code 0 in Fortran, head blow-up); recorded as a documented divergence per `M6_VAPOR_FLUX_FIXEDHEAD_DESIGN.md` and `M6_VAPOR_COVERAGE.md`.
- **Reverse pair status:** Prescribed flux lower with fixed-head upper remains accepted, not implemented in code (`crates/hydrus-core/src/heat/vapor/mod.rs` returns `VaporError::UnsupportedLowerBoundary`).



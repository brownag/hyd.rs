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
| Lower `qGWLF` (`KodBot=-7`) | rejected with trigger | **not explicitly answered; audit disposition recorded** |
| `TopInF` / `BotInF` (+-3) | rejected with trigger | **not explicitly answered; audit disposition recorded** |
| Atmospheric (`KodTop=-4`) | not decided here (G22.3) | **not explicitly answered** |
| Pairs flux/flux (closed pair first), flux/FixedHead | accepted for orchestration 29 | yes (follow the singles) |
| Pairs flux/FreeD, head/FreeD | accepted for orchestration 29 | yes (follow free drainage) |
| Pairs with seepage face or water table | rejected with trigger | yes (follow the singles) |
| Pair flux/`qGWLF` | rejected with trigger | follows `qGWLF`, which was not explicitly answered |

The user should confirm the three flagged items at wave 2 close. Trigger text is in the coverage
matrix rows, copied from the audits.

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
- A decision on `qGWLF`, +-3 forms or atmospheric switching beyond the audit dispositions (G22.3).
- Anything about G22.3, G20, G23.

## ADR 0021 compliance

All decisions comply with ADR 0021 (heat-only, one-way water then heat; vapor deferred to
`M6_VAPOR_CONTRACT.md`). Its status (Proposed) is unchanged. G22.1 and G22.4 decisions and the
criteria are unchanged.

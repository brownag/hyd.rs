# M6 vapor G22.2: flux/FixedHead pair design (DRAFT for gate)

Status: DESIGN for the gate. No deck in `reference/`, no fixture, no run, no change to `criteria.json` before approval.

## Why
The coverage matrix lists "Water flux pair, flux/FixedHead" as accepted for orchestration 29 but not implemented (`M6_VAPOR_COVERAGE.md`). The existing decks pair a top flux (`KodTop=-1`) only with a flux or free-drainage bottom. None has a top flux with a fixed-head bottom (`KodBot=1`). This is a new form.

## Decisions (user, 2026-11-15)
1. Upper flux: `rTop = -1.0e-4 cm/h`, the same as `vapor_flux_known`, so the new deck can be compared with the existing flux/flux pair.
2. Bottom fixed head: two decks, one at -100 cm (stable wet regime) and one at -10000 cm (dry regime).

## Recommended, not yet decided
3. Temperature: isothermal 25 C top and bottom (`1 25 1 25` heat BC line). This isolates the flux/FixedHead coupling. A graded profile repeats the C3 confound.
4. Vapor on, closed in every other respect (`lVapor = t`), matching the flux/flux decks.

## Deck shape
- `KodTop = -1`, `rTop = -1.0e-4`, `TopInf = f`.
- `KodBot = 1` (fixed head at the bottom), `FreeD = f`, `BotInf = f`, `qGWLF = f`.
- Initial head uniform: -100 cm (wet) or -10000 cm (dry).
- Based on `vapor_flux_known_on` and `vapor_dry_isothermal_on` inputs, with only the bottom BC and the flux line changed. Keep the LEVEL_01.DIR and the same time/print settings.

## Must verify before generating decks
- How the fixed bottom head value is set for `KodBot = 1`: `src_reference/TIME.FOR` (`SetBC`) and `INPUT.FOR`. Confirm the value comes from the deck (not from the initial profile), and record which line sets it.

## Procedure (after approval)
1. Generate candidate decks under `.development/30.8.1/` (scratch). Record the input hashes.
2. Run both through the pinned reference executable with the harness-style provenance record. Check for NaN explicitly, since exit code 0 does not rule out divergence (see C3).
3. If the wet deck converges, add its fixture and case to `reference/` under a separate gated step. Do not write the dry deck as a fixture if it diverges. Record it as a divergence instead.
4. Rust: check that `vapor_fortran_parity` can run the flux/FixedHead form. The producer's flux branch was written for flux/flux and free drainage, so this may need a gated change.

## Exit (G22.2 flux/FixedHead)
- Wet deck: provenance-backed fixture and exact parity, or a documented reason.
- Dry deck: converged fixture, or a divergence record in `M6_VAPOR_COVERAGE.md`.
- Coverage matrix row updated.


## Results (scratch runs, 2026-11-15, `.development/30.8.1/`)
Settings: top flux -1.0e-4 cm/h (`KodTop=-1`), bottom fixed head (`KodBot=1`), vapor on, heat BC `1 25 1 15` (graded, as approved). Executable sha256 `15c24e35e739d992fb6f1a0d92619103baf57223cbdfd12968b0ed33dc246edb` (pinned reference build). Record: `.development/30.8.1/record.json`. Built from `vapor_flux_known_on` with only the bottom BC, the flux line and the profile changed.

Source check: `WATFLOW.FOR` `Gauss` uses `hBot` when `KodBot >= 0`, and `NodInf` sets `hBot = hNew(1)`, so the fixed bottom head is the node-1 head of the initial profile.

### Wet deck (bottom -100 cm): exit 0, 2400 steps, non-converged 0
| t (h) | top head (cm) | bottom head (cm) | NaN rows |
|---|---|---|---|
| 0.0000 | -100.000 | -100.000 | 0/41 |
| 6.0000 | -107.291 | -100.000 | 0/41 |
| 24.0000 | -114.506 | -100.000 | 0/41 |
| 72.0000 | -124.608 | -100.000 | 0/41 |
| 240.0000 | -136.811 | -100.000 | 0/41 |

### Dry deck (bottom -10000 cm): exit 0, 21149 steps, non-converged 0
| t (h) | top head (cm) | bottom head (cm) | NaN rows |
|---|---|---|---|
| 0.0000 | -10000.000 | -10000.000 | 0/41 |
| 6.0000 | -107839.070 | -10000.000 | 0/41 |
| 24.0000 | -286130.062 | -10000.000 | 0/41 |
| 72.0000 | NaN | -10000.000 | 41/41 |
| 240.0000 | NaN | -10000.000 | 41/41 |

Reading: the wet deck converges and holds the bottom head at -100 cm. The dry deck diverges at the top, with NaN from 72 h, as C3 did. The graded temperature (approved) confounds the dry result the same way C3 did, so it does not isolate the flux/FixedHead coupling. The isothermal variant was not run.

Not yet done: fixtures (second gate), exact parity against Fortran, and a Rust producer check for this form.

## Staging and Rust check (2026-11-15)
- Staged (approved): `reference/vapor_fortran/cases/vapor_flux_fixedhead_wet_on/` (fixture.json, SELECTOR.IN, PROFILE.DAT, LEVEL_01.DIR) and `reference/vapor_fortran/fixtures/vapor_flux_fixedhead_wet_on.json`, produced by `run_vapor_reference.run_case` with the pinned executable.
- Rust check: BLOCKED. `crates/hydrus-core/src/heat/vapor/mod.rs:301` rejects a fixed-head lower boundary with a flux upper (`UnsupportedUpperBoundary`). It is commented "today's upper rejection". Adding the producer branch alone makes the candidate run fail and breaks the shared `outcome()` for every parity test, so the producer change was reverted. `vapor_fortran_parity.rs` is unchanged from HEAD.
- Needed to pass the Rust check: a kernel change to support flux upper with fixed-head lower, plus the producer branch. That is a new gate.
- Dry deck: recorded as a divergence above (not staged as a fixture).

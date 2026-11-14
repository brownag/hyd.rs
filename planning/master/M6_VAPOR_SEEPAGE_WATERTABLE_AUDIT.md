# M6 vapor seepage-face and water-table source/coupling audit (28.1.3)

Status: read-only audit. No code, fixture, or executable run. Dispositions are proposals for GATE W1.

Source of truth: pinned Fortran under `src_reference/`. Rust citations are to `crates/hydrus-core/src/`.

Read in scope: `WATFLOW.FOR:360-400` and `:208-222`, `INPUT.FOR:54-64`, `lower_boundary.rs:25-29, 105-118`, `heat/vapor/mod.rs:211-226`, `docs/decisions/0021-*.md`. Anything else is "not read in scope".

## Fortran facts

- **Input.** `INPUT.FOR:56-61` reads `BotInF, qGWLF, FreeD, SeepF, KodBot, qDrain` and, for `iVer>3`, `hSeep`. For `iVer<=3`, `hSeep=0.` (`INPUT.FOR:57`, the assignment after the label at `:58-59`).
- **Seepage face.** Branch `WATFLOW.FOR:373` (`if(SeepF)`). `vBot` is computed at `:377-378` from `Con(1:2)`, `hNew(1:2)`, `Grav*fRE`, `ThNew(1)-ThOld(1)` over `dt`, and `Sink(1)`. No `ConVh`, `ConVT` or vapor content appears in this expression, although `lVapor`, `ConVh` and `ConVT` are in the subroutine argument list (`WATFLOW.FOR:360-363`).
- **Switching.** If `KodBot.ge.0` (`:379`) and `vBot.gt.0.` (`:380`): `KodBot=-2`, `rBot=0.` (`:381-382`), i.e. a zero-flux condition. Otherwise, if `hNew(1).ge.hSeep` (`:385`): `KodBot=2`, `hBot=hSeep` (`:386-387`), i.e. a Dirichlet head. The state `KodBot` is mutated by the head solution, so the condition is nonlinear and history-dependent.
- **Water table (`qDrain`).** `INPUT.FOR:56, 60` read the flag only. Bottom flux: `WATFLOW.FOR:218-220` `rBot=FqDrain(x(1)+hNew(1), zBotDr, BaseGW, rSpacing, ...)`. The argument is the bottom pressure head plus elevation, so `rBot` is a nonlinear function of `hNew(1)`. `rBot` enters the bottom-row right-hand side at `WATFLOW.FOR:221` (`PB=...+rBot`). `zBotDr`, `BaseGW`, `rSpacing` and the `FqDrain` body (`WATFLOW.FOR:579-708`): **not read in scope**, so where they are read and how the flux is formed is unverified.
- **Shared vapor term in the bottom row.** `WATFLOW.FOR:211` `if(lVapor) ConB=ConB+(ConVh(1)+ConVh(2))/2.` precedes the `rBot` assignments (`:213, :217, :218`) and enters `S(1)` and `RB/SB` (`:212, :215-216`). So the bottom matrix row carries vapor conductance for any bottom form; the seepage and drain flux expressions themselves do not.
- **Whether `SeepF` and `qDrain` can both act:** not determined here (order of `KodBot` handling relative to `:218` is outside the read range).

## Rust rejection map

- `lower_boundary.rs:27-28`: variants `SeepageFace`, `WaterTable` of `UnsupportedLowerBoundaryMode`.
- `lower_boundary.rs:108-116`: both return `WaterStepError::UnsupportedLowerBoundary` (`"seepage_face"`, `"water_table"`). They are rejected in the water-only path too.
- `heat/vapor/mod.rs:213-217`: with vapor on, only `LowerBoundaryMode::FixedHead` passes; every other mode returns `UnsupportedLowerBoundary(mode)` at `:217`. Not relaxed (H1).
- Water-only and vapor code are separate (H4). Whether the water solver has any hook for a `KodBot` switch is not read in scope.

## Audit table

| Form | Fortran lines | Rust lines | Vapor term affected (y/n) | Disposition | Trigger | Balance check needed |
|---|---|---|---|---|---|---|
| Seepage face (`SeepF`, `hSeep`) | `INPUT.FOR:56-61`; `WATFLOW.FOR:373-390` (`vBot` `:377-378`, switch `:379-387`); shared row `:211` | `lower_boundary.rs:27, 108-112`; `vapor/mod.rs:213-217` | n in `vBot` and the switch; y in the shared bottom row (`ConVh` at `:211`) | Reject with trigger. | A nonlinear head condition (zero flux or `hSeep` Dirichlet, switched on the sign of `vBot` and `hNew(1)>=hSeep`) is needed in a vapor case; this needs water-only seepage support first, a design for the `KodBot` switch inside the vapor pass, and a `vBot` consistent with the vapor-augmented `ConB`. | Cumulative bottom outflow (seepage flux or zero in the no-seepage state) against the change in liquid plus vapor storage, closed per step across switch events, plus a check that `hNew(1)<=hSeep` whenever the head condition is active. |
| Water table (`qDrain`, drain flux) | `INPUT.FOR:56, 60`; `WATFLOW.FOR:218-221`; shared row `:211`; `FqDrain` and drain inputs not read in scope | `lower_boundary.rs:28, 113-116`; `vapor/mod.rs:213-217` | n in the drain flux call (`:218-220` has no vapor argument); y in the shared bottom row (`:211`) | Reject with trigger. | A case with drain flux depending on `hNew(1)` is requested with vapor on; this needs water-only drain support, an iterated or linearized `rBot(hNew(1))` in the vapor pass, and the drain-parameter model (unread here). | Drain-outflow accumulation against liquid plus vapor storage change per step, with the flux evaluated at the converged `hNew(1)`. |

Disposition summary for GATE W1: both forms reject. Neither is a bounded-case candidate while the water-only path also rejects them.

## ADR 0021 compliance

- **Seepage face:** compliant. D7 requires typed rejection of unsupported configurations; `vapor/mod.rs:217` and `lower_boundary.rs:108-112` do so. No silent approximation is introduced. D9 (heat reads the accepted water snapshot, never writes water) is untouched.
- **Water table:** compliant for the same reasons (`lower_boundary.rs:113-116`, `vapor/mod.rs:217`).
- ADR 0021 status is Proposed; no supersession is needed (D6). The ADR does not itself name seepage or water table; the compliance reading rests on its catch-all rule.

## Constraints honored

- No code, fixture, criteria or register change, no executable run, no commit. Vapor rejections not relaxed.

## Open items

1. `FqDrain`, `zBotDr`, `BaseGW`, `rSpacing`: not read in scope (approved by the coordinator).
2. Interaction of `KodBot=-2/2` with the vapor solve (where `KodBot` is consumed): not read in scope.
3. Rust water-solver hooks for a bottom-condition switch: not read in scope.

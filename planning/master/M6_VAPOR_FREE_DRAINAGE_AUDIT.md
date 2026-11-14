# M6 vapor free-drainage source/coupling audit (28.1.1)

Status: read-only audit. No code, fixture, or executable run. Disposition is a proposal for GATE W1.

Source of truth: pinned Fortran under `src_reference/`. Rust citations are to `crates/hydrus-core/src/`. Files outside the task scope (`driver.rs`, `vapor/balance.rs`, `vapor/water.rs`) were not read; claims that would need them are marked "not read in scope".

## Fortran coupling facts

- **Input.** `INPUT.FOR:56-61` reads `BotInF,qGWLF,FreeD,SeepF,KodBot,qDrain[,hSeep]`. `INPUT.FOR:63-65` includes `.not.FreeD` in the read-skip test. `INPUT.FOR:112` `if(FreeD) KodBot=-5` (after `:111` `qGWLF` -> -7; `:113` `SeepF` -> -2 wins over both).
- **Passing.** `WATFLOW.FOR:6, 19-20` declare `FreeD`, `lVapor`; the `Reset` call at `:78-85` passes `FreeD,qGWLF,qDrain,lVapor,ConVT,ConVh,ThVNew,ThVOld`.
- **Free-drainage flux.** `WATFLOW.FOR:206-208` forms `ConB` (arithmetic or geometric average). `:211` `if(lVapor) ConB=ConB+(ConVh(1)+ConVh(2))/2.` runs before `:213` `if(FreeD) rBot=-ConB*Grav*fRE`. The Fortran free-drainage flux therefore contains the vapor head conductivity.
- **Matrix.** `:212` `S(1)=-ConB/dxB`, `:215` `RB`, `:216` `SB` use the same vapor-augmented `ConB`. `:221-222` `PB=B-Sink(1)*dx+F2*hNew(1)-(ThNew(1)-ThOld(1))*dx/dt*fRE+rBot` is where `rBot` enters. `Gauss` (`:315-319`) applies `PB,RB,SB` when `KodBot<0`.
- **Other vapor terms in the same row.** `:224-229`: `ConTB` includes `(ConVT(1)+ConVT(2))/2.`, and `PB` gains `ConTB*(Temp(2)-Temp(1))/dxB-(ThVNew(1)-ThVOld(1))*dx/dt` (vapor thermal term and vapor storage).
- **Later overwrites.** `:217` (`qGWLF`) and `:218` (`qDrain`) overwrite `rBot` after `:213`; relevant only for multi-flag inputs (28.1.2, 28.1.3).

Answer to spec step 3: yes. `ConVh` enters the free-drainage flux and matrix (`:211-216`); `ConVT` and vapor storage `ThVNew-ThVOld` enter the bottom row (`:224-229`). `VaporContent` is not referenced in the bottom assembly.

## Rust rejection and coupling points

- `heat/vapor/mod.rs:213-217`: the lower match accepts only `LowerBoundaryMode::FixedHead`; any other mode returns `UnsupportedLowerBoundary(mode)` at `:217` (variant at `:243`).
- `lower_boundary.rs:19` `FreeDrainage`; `:127-132` requires `count >= 2` and no head row; `:161-171` builds `K_face` from liquid conductivities only (`conductivity_a/b`, arithmetic or geometric); `:182` flux `-K_face*g`; condition is Neumann; row from `flux_row` (`:216-232`), liquid `K` only. Comment `:179-180` cites `WATFLOW.FOR` `Reset` `rBot=-ConB*Grav` and does not mention `ConVh`.
- `water_solver.rs:830` calls `prepare_step` with `evaluated_conductivity[0..1]` and liquid capacity; `water_solver.rs:340,363-368` (grep) show the vapor trial assigning vapor rows by node loop. Whether the lower row is overwritten there: not read in scope.

What a vapor-enabled free drainage would touch: (1) `lower_boundary.rs:161-182` flux and `:216-232` row must add the `ConVh` face average (`WATFLOW.FOR:211`); (2) the bottom row needs the `ConVT` temperature term and `ThV` storage term (`:224-229`); (3) the vapor rejection at `vapor/mod.rs:217`; (4) bottom-boundary exchange and balance accounting: not read in scope. Relaxing `mod.rs:217` alone would run a liquid-only free-drainage row, which differs from `WATFLOW.FOR:211-229`; this orchestration does not relax it (H1).

## Audit table

| Form | Fortran lines | Rust lines | Vapor term affected (y/n) | Disposition | Trigger | Balance check needed |
|---|---|---|---|---|---|---|
| Free drainage lower (`FreeD`, `KodBot=-5`) with vapor on | `INPUT.FOR:56-61,63-65,112`; `WATFLOW.FOR:6,19-20,78-85,206-229,315-319` | `vapor/mod.rs:213-217`; `lower_boundary.rs:19,127-132,161-182,216-232`; `water_solver.rs:830` | y (`ConVh` in flux and row; `ConVT`; `ThV` storage) | Candidate for a bounded case at GATE W1. Not implemented here. | User selects it for orchestration 29 and approves a case with an explicit bottom-row design matching `WATFLOW.FOR:211-229`. Otherwise stays rejected at `mod.rs:217`. | Water balance where bottom exchange equals the vapor-augmented drainage flux and vapor storage is included in the stored-water change. Existing lower-boundary balance support: not read in scope. |

## ADR 0021 compliance

ADR 0021 (Proposed, `docs/decisions/0021-*.md`) requires no silent approximation and typed rejection of unsupported configurations. This audit changes nothing; the rejection at `vapor/mod.rs:217` stays. Any later implementation must not relax it without the full vapor row, otherwise it would be a silent liquid-only approximation. Compliant; no supersession needed (D6).

## Open items and G7 reconciliation

- 28.1.4 (G7 end row) also reads `WATFLOW.FOR:213`; neither task edits it. Reconcile at GATE W1.
- Which `hNew`/temperature iterate evaluates `ConVh(1..2)` in the Rust vapor trial: not read in scope.

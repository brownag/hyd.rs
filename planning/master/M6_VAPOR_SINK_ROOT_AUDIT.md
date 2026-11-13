# M6 vapor sink and root-uptake source/coupling audit (27.1.1)

Status: read-only audit. No code, fixture, or executable run. Dispositions are proposals for Gate S.

Source of truth: pinned Fortran under `src_reference/`. Rust citations are to `crates/hydrus-core/src/`.

Extra read-only sources (orchestrator approval): `src_reference/SINK.FOR` (`SetSnk`, `SetRG`), `src_reference/TIME.FOR` (`SetBC`, `DailyVar`), `src_reference/WATFLOW.FOR` (`Veloc`, `ConVapor`, `VaporContent`).

## Fortran coupling facts

- **Gating.** `INPUT.FOR:31` reads `SinkF` and `lRoot`. `HYDRUS.FOR:345-352` calls `SinkIn` and then `SetSnk` only when `SinkF`. `HYDRUS.FOR:479-480` calls `SetRG` when `lRoot` or `iCrop>1`. `HYDRUS.FOR:482-487` is an `if(SinkF)` block whose `SetSnk` call is commented out (inactive). The only active `SetSnk` call is `HYDRUS.FOR:352`.
- **Root potential.** `INPUT.FOR:67` reads `rRoot`. `INPUT.FOR:71` sets it to 0 in the no-input branch. `INPUT.FOR:101` applies `rRoot=abs(rRoot)`. `TIME.FOR:213-230` (`DailyVar`) sets `rRoot` daily. `SetSnk` receives `rRoot` as its potential `TPot` (`HYDRUS.FOR:352`).
- **Uptake formula.** `SINK.FOR:55` (Hanks branch, inactive via `lHanks=.false.`). Active branch: `SINK.FOR:97` `Sink(i)=Alfa*SAlfa*Beta(i)*TPot/Compen`. Limit `SINK.FOR:98` zeroes uptake when `ThNew-0.00025 < ThR`. Limit `SINK.FOR:103` caps by the water-content margin over `dt`. Accumulation `SINK.FOR:104` `vRoot=vRoot+Sink(i)*dxM`.
- **Negative-Beta branch.** `SINK.FOR:117-121` sets `Sink(i)=Beta(i)*rBot` for `Beta(i)<0`, bounded below by the residual-content limit. Whether any input path sets `Beta<0` is not confirmed (see Open items).
- **Root distribution.** `SetRG` (`SINK.FOR:268-281`) sets `Beta` from `iRootIn` (`1` = table, `2` = analytic, gated on `lRoot`). It is not called unless `lRoot` or `iCrop>1`.
- **Water balance.** `WATFLOW.FOR:221` (`PB=B-Sink(1)*dx+...`), `:254` (`P(i)=...-Sink(i)*dx`), `:285` (`PT=...-Sink(N)*dx-B`). These are the liquid rows. Vapor storage terms sit in the same rows (`:228`, `:266`, `:291`, `ThVNew-ThVOld`).
- **Liquid boundary velocity.** `WATFLOW.FOR:717` `Veloc` takes `Sink`. Its boundary velocities use it at `:17` (`v(N)` term `Sink(N)`) and `:51` (`v(1)` term `Sink(1)`). Vapor boundary velocities (`:738`, `:772`) use vapor storage only.
- **Vapor functions.** `WATFLOW.FOR:849` `ConVapor` and `:930` `VaporContent` take no `Sink` argument.
- **Heat.** `TEMPER.FOR:77` `SetUpHeat` takes `Sink` and uses it in active code: level-2 bottom `:17-21`, `:26-29`; interior `:42-52`, `:54-65`; top `:75-77`, `:83-89`. Terms are `Cw * dx/24 * (Sink weights)`.
- **Vapor enthalpy.** `TEMPER.FOR:48` calls `TempAdj(N,x,dt,TempN,TempO,B,D,E,F,vVOld,vVNew,g0,...,ThVNew,ThVOld,...)`. `TempAdj` takes no `Sink` argument.

## Rust rejection and coupling points

- `heat/vapor/mod.rs:179-181`: `RootUptake` rejects any `input.root_uptake` that is `Some`.
- `heat/vapor/mod.rs:182-188`: `NonzeroSink` rejects any nonzero entry, then checks sink length.
- `heat/vapor/mod.rs:246-247`: typed variants `RootUptake`, `NonzeroSink { index, value }`.
- `heat/vapor/mod.rs:139-150`: `validate_water_input` is the single preflight that enforces the above on all water-pass inputs.
- `heat/vapor/driver.rs:466`, `:507`, `:549`, `:577`: the vapor driver passes `input.sink` into both heat passes. The guard at `mod.rs:182` is the only barrier.
- `heat/transport.rs:42` field `sink`; `:195` length check; `:381` forwarded; `:405` level-1 assembly uses `s`; `:444` level-2 assembly uses `s`.
- `heat/vapor/balance.rs:202`, `:266-288`, `:342`: balance accounts sink (`domain_sink_rate`) on the vapor side.
- `water_solver.rs:65-67`: `sink` and `root_uptake` inputs. `:806-819`: root uptake is added to the sink field via `fill_trial_sink`. `:876`, `:948-962`: sink in water balance.
- `root_uptake.rs:1-12`: Feddes root-uptake model (heads, demand, weights).

## Audit table

| Form | Fortran lines | Rust lines | Vapor term affected (y/n) | Disposition | Trigger |
|---|---|---|---|---|---|
| (a) Prescribed sink, `SinkF` on, no root (`lRoot=0`) | `INPUT.FOR:31`, `HYDRUS.FOR:345-352`, `SINK.FOR:97`, `SINK.FOR:117-121` (Beta<0), `WATFLOW.FOR:221,254,285`, `WATFLOW.FOR:17,51`, `TEMPER.FOR:17-89` | `vapor/mod.rs:182-188`; `heat/transport.rs:405,444`; `vapor/driver.rs:466,507`; `vapor/balance.rs:342` | y (liquid rows, liquid boundary velocity, heat sink weights) | Candidate for a bounded case at Gate S, conditional on confirming the source of `Beta` when `lRoot=0` (Open item 1). Not implemented here. | User selects option (ii) in Gate S, and a case with `SinkF=1, lRoot=0` and vapor on is approved with an explicit balance criterion. |
| (b) Root uptake (`lRoot`/`iRootIn`, `TPot=rRoot`) | `INPUT.FOR:67,71,101,1150-1167`, `HYDRUS.FOR:256-257,345-352,479-482`, `SINK.FOR:268-281,97`, `TIME.FOR:213-230` | `vapor/mod.rs:179-181`; `root_uptake.rs:1-12`; `water_solver.rs:66-67,806-819` | y (same liquid sink path as (a), with root-weighted `Beta`) | Reject with trigger. Vapor rejects `root_uptake` (`mod.rs:180`), and the root path also needs `SinkF` and `SetRG`. | A case requests root uptake with vapor enabled, and an approved root-uptake vapor design exists (new Gate, not this orchestration). |
| (c) Heat-equation sink coupling and `TempAdj` vapor terms | `TEMPER.FOR:48,77-89,180` (`TempAdj`, no `Sink` argument); `SetUpHeat` sink terms `TEMPER.FOR:17-89`; liquid velocity `WATFLOW.FOR:17,51` feeds `vNew/vOld` | `heat/transport.rs:405,444`; `heat/vapor/mod.rs:182-188` | y for the advective/sink heat terms through liquid `v`; n for `TempAdj` vapor terms (they do not reference `Sink`) | Reject with trigger. Heat sink terms are real and exist in heat-only transport; the vapor-specific `TempAdj` path does not depend on `Sink` directly, but a sink-bearing liquid `v` changes the heat advection the vapor path shares. | A sink-bearing vapor case is requested. Then a balance check must show the sink heat term is closed in the vapor energy budget (`balance.rs`). |
| (d) Water step with vapor under a sink | `WATFLOW.FOR:221,254,285` (liquid rows with vapor storage at `:228,266,291`); `WATFLOW.FOR:717` (`Veloc`, sink in liquid boundary `v`, not in `vV` at `:738,772`); `ConVapor` `:849` and `VaporContent` `:930` (no `Sink` argument) | `heat/vapor/mod.rs:139-150,182-188`; `water_solver.rs:806-828` (sink in the liquid water step, not the vapor pass) | y (liquid solve and liquid boundary velocity; vapor conductance and content do not take `Sink`; coupling is indirect through shared `hNew`) | Reject with trigger. The vapor water step is guarded at preflight (`validate_water_input`). | Gate S chooses (ii) and a sink-plus-vapor water balance criterion is written and approved. |

Disposition summary for Gate S:
- Row (a): candidate for a bounded case, subject to Open item 1.
- Rows (b), (c), (d): reject, each with the trigger above.

## Constraints honored

- No code, fixture, or executable run. No commit.
- The guards `vapor/mod.rs:180` and `:185` are not relaxed. Heat-only paths are not changed.

## Open items

1. When `lRoot=0` and `SinkF=1`, `SetRG` is not called, so `Beta` source is unconfirmed. The `Beta<0` branch (`SINK.FOR:117-121`) may be the only prescribed-sink path. Check the `Beta` initialization before classing (a) as a bounded case.
2. `driver.rs` passes `input.sink` into heat passes even though `mod.rs:182` rejects nonzero sink. The guard is therefore the only barrier. A future relaxation must touch both.
3. The Rust `balance.rs` sink accounting was read only for citation. No balance-closure claim is made.
4. `OUTPUT.FOR:70` `CumQ(2)+rRoot*dt` accumulates root flux independently of `SetSnk`. Not used in the dispositions above.

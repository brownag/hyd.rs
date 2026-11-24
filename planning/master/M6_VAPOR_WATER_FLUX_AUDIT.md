# M6 vapor prescribed water-flux and mixed-pair audit (28.1.2)

Status: read-only audit. No code, fixture, or executable run. Dispositions are proposals for GATE W1.

Source of truth: pinned Fortran under `src_reference/`. Rust citations are to `crates/hydrus-core/src/`. Atmospheric codes (`KodTop=-4`, `AtmBC`) are G22.3 and not decided here. Free drainage (`FreeD`), seepage face and water table are owned by 28.1.1 and 28.1.3; they appear here only as cross-references and as pair members. Files under `heat/vapor/` other than `mod.rs:211-225` were not read in scope.

## Fortran codes

- **Top inputs.** `INPUT.FOR:51-52` reads `TopInF, WLayer, KodTop, lInitW`. `:53` `KodTop=0` sets `lVarBC`. `rTop` is read at `:63-69` only when `(.not.TopInF .and. KodTop==-1)`; otherwise it is zeroed.
- **Bottom inputs.** `INPUT.FOR:56-61` reads `BotInF, qGWLF, FreeD, SeepF, KodBot, qDrain[, hSeep]`. `rBot` is read at `:63-69` only when `.not.BotInF .and. KodBot==-1` and no other bottom flag is set. `:77-81` (qGWLF) reads `GWL0L, Aqh, Bqh`.
- **Remaps.** `INPUT.FOR:104-110`: `TopInF` gives `KodTop=isign(3,KodTop)`; `BotInF` gives `KodBot=isign(3,KodBot)`; `AtmBC` with `KodTop<0` gives `KodTop=-4` (G22.3); `WLayer` gives `KodTop=-|KodTop|`; `qGWLF` gives `KodBot=-7`; `FreeD` gives `-5`; `SeepF` gives `-2`.
- **Code list.**

| Side | Code | Meaning | Cite |
|---|---|---|---|
| Top | `KodTop=-1`, `TopInF` false | Prescribed flux `rTop`, static | `INPUT.FOR:51-52,63-69` |
| Top | `KodTop=+1` | Prescribed head | `INPUT.FOR:51-52`; `WATFLOW.FOR:327-334` (Dirichlet when `KodTop>0`) |
| Top | `KodTop=+-3` (`TopInF`) | Time-variable value from input, flux when negative | `INPUT.FOR:106`; `rTop` zeroed at `:70-72` |
| Top | `KodTop=-4` | Atmospheric | `INPUT.FOR:107-110`: G22.3, not decided |
| Bottom | `KodBot=-1` with `rBot` read | Prescribed flux | `INPUT.FOR:63-69` |
| Bottom | `KodBot=+-3` (`BotInF`) | Time-variable value from input | `INPUT.FOR:107` |
| Bottom | `KodBot=-7` | `qGWLF` head-dependent flux | `INPUT.FOR:77-81,109`; `WATFLOW.FOR:217` |
| Bottom | `KodBot=-5` / `-2` / `qDrain` | `FreeD` / `SeepF` / drain | 28.1.1 and 28.1.3 |

## Fortran water step with vapor on

- **Bottom.** `WATFLOW.FOR:206-212` builds `ConB`; with vapor `:211` adds `ConVh`. `:213` sets `rBot` for `FreeD`; `:217` sets `rBot=Fqh(hNew(1)-GWL0L,Aqh,Bqh)` for `qGWLF` (`Fqh` at `:572-574`, `Aqh*exp(Bqh*|GWL|)`). `:221-222` puts `+rBot` in `PB`. Vapor adds `ConTB*(Temp(2)-Temp(1))/dxB-(ThVNew(1)-ThVOld(1))*dx/dt` at `:223-228`. Static `rBot` (`KodBot=-1`) enters `PB` unchanged.
- **Top.** `WATFLOW.FOR:272-276` builds `ConA`; with vapor `:276` adds `ConVh`. `:287-291` adds the vapor `ConTA` and `ThV` terms to `PT`. `:294` `PT=PT-rTop` applies the flux.
- **Solve.** `Gauss` takes the Neumann branch at the bottom when `KodBot<0` (`WATFLOW.FOR:315-322`) and at the top when `KodTop<=0` (`:327-334`).
- **Switching.** `Shift` (`WATFLOW.FOR:393-423`) acts only when `TopInf` is true and `abs(KodTop)=4` or `abs(KodTop)=1` with `rTop>0`. After the `INPUT.FOR:106` remap, `TopInF` forces `abs(KodTop)=3`, so the `abs=1` branch cannot occur with `TopInF`. A static `KodTop=-1` flux is therefore never switched in `Shift`. Bottom switching exists only for `SeepF` (`:373-392`, 28.1.3).
- **Result.** For every non-atmospheric flux code the Fortran water step with vapor on takes the flux as a boundary term in `PB` or `PT`, and the end rows carry vapor terms (`ConVh` in the face conductance, `ConT*dT` and `dThV` in the right side).

## Rust mapping

- **Vapor rejection.** `heat/vapor/mod.rs:211-217`: only `LowerBoundaryMode::FixedHead` is accepted; any other mode returns `UnsupportedLowerBoundary(mode)` at `:217`. `:219-225`: only `UpperBoundary::Head`; others return `UnsupportedUpperBoundary` at `:225`.
- **Water-only lower flux.** `lower_boundary.rs:17` `PrescribedFlux`, `:19` `FreeDrainage`; `:105-107` finite check; `:132` accepted; `:160` flux becomes `BoundaryCondition::Neumann`; `:161-178` free-drainage flux `-K*g`; `:108-116` `SeepageFace` and `WaterTable` are `Unsupported`.
- **Water-only upper flux.** `water_solver.rs:90` `UpperBoundary::Flux { flux, row }`; `:114` maps it to Neumann with a caller row; `:105` `measured_flux`.
- **Row content.** The lower flux row is built by `lower_flux_row` (`lower_boundary.rs:43-86`) from `face_conductivity`, `capacity`, `theta` and gravity. It has no vapor head conductivity, no temperature term and no vapor storage. The upper `Flux` row is caller-supplied (`water_solver.rs:90`). Whether a vapor-aware row builder exists is not read in scope; the vapor row assembly file is outside this audit's read list.
- **Audit finding.** Within the in-scope files, a flux end is built water-only, while the Fortran flux ends carry vapor terms (`WATFLOW.FOR:211,223-228,276,287-291`). Lifting the `mod.rs:217/225` rejection alone would not match the source. Each flux form needs new end-row coupling plus vapor boundary and storage accounting. This is a finding, not a design.
- **ADR 0021.** Status Proposed (`docs/decisions/0021-*.md` frontmatter), heat-only, one-way water then heat; its 2026-11-12 note defers vapor to `M6_VAPOR_CONTRACT.md`. All dispositions below comply with it and do not supersede it.

## Single-form table

| Form | Fortran lines | Rust lines | Vapor term affected | Disposition | Trigger / balance check named (not run) |
|---|---|---|---|---|---|
| Upper prescribed flux (`KodTop=-1`, static) | `INPUT.FOR:51-52,63-69`; `WATFLOW.FOR:276,287-294,327-334` | `water_solver.rs:90,114`; `vapor/mod.rs:225` | Yes: `ConVh` in `ConA`, `ConTA` and `ThV` in `PT` | Candidate for a bounded case | Trigger to reject: no vapor-aware upper flux row source in scope. Check: liquid plus vapor column storage change equals `rTop` times dt plus lower exchange, per step. ADR 0021 compliant (water step first, heat reads result). |
| Lower prescribed flux (`KodBot=-1`, `rBot` read) | `INPUT.FOR:56-61,63-69`; `WATFLOW.FOR:211,221-228,315-322` | `lower_boundary.rs:17,105,132,160`; `vapor/mod.rs:217` | Yes: `ConVh` in `ConB`, `ConTB` and `ThV` in `PB` | Candidate for a bounded case | Trigger to reject: lower flux row at `lower_boundary.rs:43-86` is water-only. Check: same column balance with `rBot` as the lower exchange. ADR 0021 compliant. |
| Lower `qGWLF` (`KodBot=-7`, head-dependent) | `INPUT.FOR:77-81,109`; `WATFLOW.FOR:217,572-574` | Not present in `LowerBoundaryMode` (`lower_boundary.rs:11-21`); `vapor/mod.rs:217` | Yes, as above, plus `rBot` depends on `hNew(1)` | Reject with trigger | Trigger: no Rust mode for `qGWLF`; needs a new mode and per-iteration `Fqh` update before any vapor work. Check if later accepted: exchange equals `Fqh(h-GWL0L)` at the accepted head, in the column balance. ADR 0021 compliant. |
| Lower `BotInF` / upper `TopInF` (`+-3`) | `INPUT.FOR:106-107`; `WATFLOW.FOR:393-394` | Not read in scope | Same flux terms; value comes from time-variable input | Reject with trigger | Trigger: time-variable input path not audited here; no Rust mode in the in-scope files. |
| Free drainage (`FreeD`, `KodBot=-5`) | `WATFLOW.FOR:213` | `lower_boundary.rs:19,127-132,161-178` | Yes | Cross-reference: 28.1.1 owns the verdict | Reconcile at GATE W1. |
| `SeepF`, `qDrain` | `WATFLOW.FOR:373-392,218` | `lower_boundary.rs:108-116` | Yes | Cross-reference: 28.1.3 owns the verdict | Reconcile at GATE W1. |
| Atmospheric (`KodTop=-4`, `AtmBC`) | `INPUT.FOR:107-110` | `water_solver.rs:93` | n/a | G22.3, not decided | n/a |

## Pairs table

Each pair needs new coupling beyond its two single forms only where noted. Free-drainage verdicts are conditional on 28.1.1 and every one is marked reconcile at GATE W1.

| Pair (upper, lower) | New coupling beyond the singles | Cite | Disposition | Note |
|---|---|---|---|---|
| Flux, FixedHead | No. Both ends are independent rows (`PT` at `WATFLOW.FOR:294`, Dirichlet bottom at `Gauss:315`). Needs both ends vapor-aware, and the upper flux alone is the new piece. | `WATFLOW.FOR:294,315-322`; `lower_boundary.rs:11-14` | Follows the upper-flux row | Balance: upper flux plus fixed-head lower exchange. |
| Flux, PrescribedFlux | No new interaction in the Fortran: both are independent Neumann ends. Needs both end rows. `Gauss` handles two Neumann ends (`WATFLOW.FOR:315-334`); a closed column makes flux a sole balance constraint. | `WATFLOW.FOR:221-222,294,315-334` | Follows the two singles | Balance: net imposed flux equals column storage change. Closed pair (both zero) is the cleanest bounded case. |
| Flux, `qGWLF` | Yes, `rBot` depends on `hNew(1)` | `WATFLOW.FOR:217` | Reject (qGWLF rejected) | Follows the `qGWLF` row. |
| Flux, FreeD | No new interaction beyond singles in the Fortran: `rBot=-ConB*Grav*fRE` uses the vapor-augmented `ConB` (`WATFLOW.FOR:211,213`), which the lower end needs anyway. | `WATFLOW.FOR:211,213,294`; `lower_boundary.rs:161-178` | Conditional on 28.1.1; reconcile at GATE W1 | Note that Rust free drainage uses liquid `K` only (`lower_boundary.rs:161-178`) while Fortran adds `ConVh` before `rBot`; this is a coupling point for 28.1.1. |
| Head, FreeD | Only the lower end changes; the upper end is already supported (`vapor/mod.rs:219-225`) | `vapor/mod.rs:219-225`; `WATFLOW.FOR:213` | Conditional on 28.1.1; reconcile at GATE W1 | Same `ConB` vapor-addition point. |
| Flux (upper), FreeD (lower) with drain, seepage | Out of this pair list | n/a | See 28.1.1 and 28.1.3 | Reconcile at GATE W1. |

## Questions and discovered items

- Discovered: the Fortran free-drainage `rBot` multiplies the vapor-augmented `ConB` (`WATFLOW.FOR:211,213`), but the Rust free-drainage flux uses liquid conductivities only (`lower_boundary.rs:161-178`). Relevant to 28.1.1 and to the FreeD pairs; not decided here.
- Discovered: the vapor-on end-row builder was not read in scope, so the claim "no vapor-aware flux row exists" is limited to the in-scope files.
- Open for GATE W1: whether `TopInF`/`BotInF` (`+-3`) time-variable forms should be audited later.

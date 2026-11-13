# M6 vapor heat-boundary code audit (task 26.1.1)

Read-only audit of `kTopT` and `kBotT` in the pinned Fortran and the Rust heat/vapor config path.
No code, test or fixture was changed. The Fortran executable was not run. No numerical acceptance is claimed.

## Reachable codes

- `INPUT.FOR:1236` is the only reader of `kTopT`/`kBotT` (`read(30,*,err=901) kTopT,tT,kBotT,tB`); `INPUT.FOR:1205` is the `TempIn` signature. There is no range check, so any integer is accepted. Only `INPUT.FOR`, `TEMPER.FOR` and `HYDRUS.FOR` mention either name.
- `HYDRUS.FOR:265-268`: after `TempIn`, `if(lEnBal) kTopT=-1`. `lEnBal=true` is out of scope for vapor (orchestrator decision); noted only.
- Consequence: every negative integer (-1, -2, ...) is reachable for both codes, as are 0 and all positives.

## Code table

| Code | Source branch (TEMPER.FOR) | Rust mapping | Vapor term | Disposition |
|---|---|---|---|---|
| kTopT > 0 | `SetUpHeat` Dirichlet `:144-147`; `Temper` upper flux skipped (`:25`); `TempAdj` top block skipped (`:249`) | `heat/config.rs:161` Dirichlet; `heat/boundary.rs:241`; vapor accepted (`config.rs:178` rejects only `<=0`) | n | accepted today (D) |
| kTopT == -1 | `SetUpHeat` flux `:148-`; `Temper` `HeatFl` incl. vapor `:25-32`; `TempAdj` matrix terms `:249` plus explicit term `:255` (level 1) | `config.rs:162` `_ => Flux`; `boundary.rs` else-branch (`:245-`); rejected under vapor by `config.rs:178-180` (`VaporSelectionNotSupported{name:"kTopT"}`) | y (`:25-32`, `:249-255`) | candidate for acceptance |
| kTopT < -1 | same as -1 except `:255` does not fire (`kTopT.eq.-1`); `HeatFl` vapor flux (`:25-32`) and `:249` matrix terms still fire | silently mapped to Flux (`config.rs:162`); rejected under vapor | partial (HeatFl and matrix terms y; `:255` term n) | reject with trigger: needs a reference case showing the `:255` omission, or a decision |
| kTopT == 0 | `SetUpHeat` no branch (`:144-160`): B(N), D(N), F(N) unassigned; `Temper` flux skipped; `TempAdj` skipped | rejected: `config.rs:155-160`, `boundary.rs:183-184` (`TopBoundaryKindNotSupported`) | n/a | reject (invalid/unassigned) |
| kBotT > 0 | `SetUpHeat` Dirichlet `:87-90`; `TempAdj` bottom skipped (`:220`) | `config.rs:171` Dirichlet; `boundary.rs:196` | n | accepted today (D) |
| kBotT == 0 | `SetUpHeat` no branch (natural row); `TempAdj` bottom skipped (`:220` needs `<0`) | `config.rs:170` ZeroGradient; rejected under vapor by `config.rs:181-183` | n | candidate for acceptance |
| kBotT < 0 | `SetUpHeat` flux `:91-`; `TempAdj` bottom `:220-232`, level-1 term `tBot*Cv*(vVNew+vVOld)/2` | `config.rs:172` `_ => Flux`; `boundary.rs:200`; rejected under vapor | y (`:220-232`) | candidate for acceptance |
| kBotT < -1 | identical to -1 in source (only sign tested at bottom) | same as above | y | candidate (same as -1) |

## Form table

| Form | kTopT | kBotT | Source behavior | Rust today | Disposition |
|---|---|---|---|---|---|
| DD | >0 | >0 | no vapor boundary term | accepted | accepted (existing) |
| DF | >0 | <0 | bottom vapor term only (`:220-232`) | rejected (`kBotT`) | candidate for acceptance |
| DZ | >0 | ==0 | no vapor boundary term; zero-gradient bottom | rejected (`kBotT`) | candidate for acceptance |
| FD | -1 | >0 | top `HeatFl` vapor plus `:249-255` | rejected (`kTopT`) | candidate for acceptance |
| FF | -1 | <0 | top plus bottom vapor terms | rejected | candidate for acceptance |
| FZ | -1 | ==0 | top vapor terms only | rejected | candidate for acceptance |
| top <-1 (any bottom) | <-1 | any | `:255` omitted; other terms present | rejected (mapped to Flux in non-vapor) | reject with trigger |
| top == 0 (any bottom) | 0 | any | node N unassigned | rejected | invalid |
| lEnBal forced -1 | -1 | any | `HYDRUS.FOR:268` | n/a | out of scope for vapor |

## Questions answered

- (a) Is `kTopT == -1` the only top code with a vapor storage term? Not strictly. `TEMPER.FOR:25-32` (`HeatFl`) and `:249` matrix terms apply to every negative code; only the explicit `-tTopA*Cv*(...)` term at `:255` is -1 only. Orchestrator decision: propose -1 only for acceptance, other negatives reject-with-trigger.
- (b) Other negative codes in reachable input? Yes: `INPUT.FOR:1236` accepts any integer.
- (c) Does `kBotT == 0` add a vapor term? No (`TEMPER.FOR:220` tests `kBotT.lt.0` only).
- (d) `kTopT == 0`: nothing assigned for node N in `SetUpHeat` (`TEMPER.FOR:144-160`); `Temper` flux and `TempAdj` top blocks are skipped.

## Hazards H1-H4

- H1 confirmed with nuance. The `:255` term is -1 only (`TEMPER.FOR:255`), while Rust maps all negatives to Flux (`config.rs:162`). Faithful for `SetUpHeat`; unfaithful for vapor if a code other than -1 were accepted. No current bug because vapor rejects `kTopT<=0` (`config.rs:178`).
- H2 confirmed. `kBotT==0` adds nothing (`TEMPER.FOR:220`); Rust vapor rejects it today (`config.rs:181`).
- H3 confirmed. No `==0` top branch (`TEMPER.FOR:144-160`); Rust rejects (`config.rs:155-160`, `boundary.rs:183`).
- H4 confirmed. `config.rs:162` and `:172` use `_ => Flux`; any vapor-selection branch must key on `-1` explicitly.

## Notes and line corrections

- Plan cited `config.rs:179-182`; actual vapor rejections are `config.rs:178-183`. Plan cited `boundary.rs:98-110`; those are doc comments, the code rejection is `boundary.rs:183-184`.
- `heat/vapor/mod.rs:52` wraps `HeatConfigError` as `VaporError::Heat`; `:65` requires `vapor_enabled`. `tests/vapor_config_contract.rs` covers `kTopT=0` rejection (`vapor_config_contract.rs:84`) among the excluded selectors.
- The `:255` term is level-1 only; level 2 (`:257-259`) only adjusts B(N), D(N), F(N) with no explicit `tTopA` term.
- Contract reference: `M6_VAPOR_CONTRACT.md` accepts only `kTopT>0`, `kBotT>0` today.

## Questions

None open. Resolved by orchestrator: Q1 (-1 only for acceptance), Q2 (`lEnBal` out of scope).

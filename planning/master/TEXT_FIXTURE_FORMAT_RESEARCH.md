# Legacy text fixture format audit: M5 solute case

**Status:** source audit; no fixture edits or reference execution performed.

**Scope:** diagnose the recorded `PROFILE.DAT` EOF and check the other text inputs used by the derived Example 3 case against the pinned HYDRUS 1D 4.08 reader.

**Execution follow-up:** This audit describes the pre-amendment C fixture. Amendment C subsequently set initial `lEquil=t`; its single authorized run completed with all three solute outputs through 200 hours. See [RUN.md](../../reference/fortran_reference/solute/example3/RUN.md) for verified output hashes, warnings, and the preserved failed-run manifest. Scientific parity and tolerances remain unverified.

## Finding: the EOF is caused by an initial-condition mode mismatch

The captured run reports an EOF at the nodal `READ` in `NodInf` (`INPUT.FOR:251`); the runner retained its inputs and logs in `.development/12.2.1/run/`, and [RUN.md](../../reference/fortran_reference/solute/example3/RUN.md) records the failure and input hashes.

The `SELECTOR.IN` v4 basic-information flags currently select `lChem=t` and `lEquil=f` ([fixture lines 9–10](../../reference/fortran_reference/solute/example3/SELECTOR.IN#L9-L10)). `BasInf` reads this row at [INPUT.FOR:31–32](../../src_reference/INPUT.FOR#L31-L32). HYDRUS calls `NodInf` before it calls `ChemIn` ([HYDRUS.FOR:196–199](../../src_reference/HYDRUS.FOR#L196-L199), then [271–277](../../src_reference/HYDRUS.FOR#L271-L277)), so the `lEquil` value governing the initial profile read is still false.

For `lChem=t`, `NodInf` reads each row as:

`node, x, h, material, layer, beta, Ax, Bx, Dx, temperature, C[1..NS], S[1..NS]`

when `lEquil` is false ([INPUT.FOR:242–252](../../src_reference/INPUT.FOR#L242-L252)). At `NS=3`, that is 16 numeric fields. The current fixture has 13 per row: the nine hydraulic/node fields, temperature, and three dissolved concentrations ([PROFILE.DAT](../../reference/fortran_reference/solute/example3/PROFILE.DAT)). The missing three sorbed initial values explain the EOF at the read statement.

The alternative branch for `lEquil=t` reads the same fields through temperature and dissolved concentrations but omits `S[1..NS]`, so it requires 13 values. Later, `ChemIn` resets `lEquil` to true and derives its value from the equilibrium fraction and immobile-water content ([INPUT.FOR:1331–1339](../../src_reference/INPUT.FOR#L1331-L1339)); in this case `Fraction=1` and `ImmobileWaterContent=0` in [SELECTOR.IN:42–43](../../reference/fortran_reference/solute/example3/SELECTOR.IN#L42-L43)). That later classification cannot change the earlier profile read.

### Fixture-level correction choices

Both layouts satisfy the `NodInf` reader, but they describe different profile records:

1. **Preferred for this case:** set the initial `lEquil` selector flag to true and keep the 13-column profile. This matches the fixture's declared all-equilibrium sorption configuration (`Fraction=1`, immobile water `0`) and zero initial dissolved concentration. Update `CASE.json` to explicitly explain that the selector flag controls `NodInf`'s initial-profile record width, while `ChemIn` subsequently derives the active sorption mode from its coefficients.
2. **Alternative:** keep `lEquil=f` and append three explicit zero sorbed concentrations to every profile row. This is the appropriate record width if the fixture intentionally declares nonequilibrium initial conditions with a zero initial sorbed pool. Document the fields and their zero-value provenance in `CASE.json`.

Do not infer which historical HYDRUS deck layout Example 3 used: no original machine-readable deck is present. The choice above is a source-compatible derived-fixture decision, not evidence of manual-deck reproduction.

## `PROFILE.DAT` record layout checks

The current header/record order is aligned with `NodInf`: version record; integer `n=1`; one skipped descriptive record; `NumNP, ii, NS`; exactly `NumNP` nodal rows; then `NObs` and, only when positive, its node list ([INPUT.FOR:223–228](../../src_reference/INPUT.FOR#L223-L228), [355–363](../../src_reference/INPUT.FOR#L355-L363)). The current derived file has 1001 node rows and `NObs=0`, as does its declared mesh. There is no column-label record between the node header and first node row. Removing that label line was correct: the reader consumes no such record. The repository smoke profile uses the same version/count/skip/header/node/observation sequence ([smoke PROFILE.DAT](../../reference/fortran_reference/smoke/PROFILE.DAT)).

Node-column widths are conditional on `lChem`, `lTemp`, and `lEquil`, not just on `NS` ([INPUT.FOR:242–252](../../src_reference/INPUT.FOR#L242-L252)). In the M5 fixture, chemistry is enabled and three species are declared, so the initial `lEquil` flag must agree with whether the rows include sorbed values. Temperature remains a required column in either chemistry branch.

## Other text inputs and conditional files

| File | Reader contract | Finding for this fixture |
|---|---|---|
| `SELECTOR.IN` | The version marker is read first; v4 `BasInf` skips two records, reads one heading, skips a unit heading, reads three unit records, then reads the control flag rows and values. Later routines consume material, time, and chemistry sections sequentially ([INPUT.FOR:21–42](../../src_reference/INPUT.FOR#L21-L42), [866–893](../../src_reference/INPUT.FOR#L866-L893), [1287–1345](../../src_reference/INPUT.FOR#L1287-L1345)). | The current two descriptive records after the version now keep the basic-information fields aligned. `hours` is a recognized time unit ([INPUT.FOR:182–202](../../src_reference/INPUT.FOR#L182-L202)). Keep the v4 optional-flag record and the chemistry records because `lChem=t`. Labels are skipped as whole records; their presence/order matters, while their wording is not parsed as data. |
| `LEVEL_01.DIR` | With no path argument, HYDRUS reads one path record using `(A)`; when a path argument is supplied, it uses that argument instead ([HYDRUS.FOR:106–116](../../src_reference/HYDRUS.FOR#L106-L116), format at [HYDRUS.FOR:779](../../src_reference/HYDRUS.FOR#L779)). | The current single `.` record is suitable for the no-argument legacy path. The recorded fixture invocation passes `solute-input` as an argument, so this file is not consumed on that invocation. |
| `Options.in` | HYDRUS attempts to open it, but an open error branches to the continuation label and execution proceeds ([HYDRUS.FOR:118–120](../../src_reference/HYDRUS.FOR#L118-L120)). | Not a required text fixture for this run path. |
| `ATMOSPH.IN` | Open/read occurs only when `TopInF`, `BotInF`, or `AtmBC` is true ([HYDRUS.FOR:179–194](../../src_reference/HYDRUS.FOR#L179-L194)). | All three are false in this fixture, so it is not required. The smoke fixture's retained atmospheric file is explicitly unused on its selected water-only path ([smoke README](../../reference/fortran_reference/smoke/README.md)). |
| `Meteo.in` | Opened only inside the atmospheric-boundary path when `lMeteo` is true ([HYDRUS.FOR:179–193](../../src_reference/HYDRUS.FOR#L179-L193)). | Not required; atmospheric boundaries and meteorological forcing are disabled. |
| `Mater.in` | Opened only when `iModel` selects tabulated hydraulic properties ([HYDRUS.FOR:210–213](../../src_reference/HYDRUS.FOR#L210-L213)). | Not required; the fixture selects van Genuchten model 0. |

The smoke `PROFILE.DAT` is intentionally water-only, so its nine-field rows are not an example of a valid three-species profile row. Its successful layout should not be copied for chemistry cases.

## Pinned-source interface note

The portability patch does not change the `NodInf` call or signature: comparison of the staged source under `.development/12.2.1/reference-build/src_reference/` with the tracked call/signature shows the same arguments, and [0001-gfortran-portability.patch](../../reference/fortran_reference/0001-gfortran-portability.patch) contains no `NodInf` interface edit. There is a separate final-flag order mismatch: the call supplies `...,lPrint,lDualNEq,lFlux`, while the formal signature ends `...,lPrint,lFlux,lDualNEq` ([HYDRUS.FOR:198–199](../../src_reference/HYDRUS.FOR#L198-L199); [INPUT.FOR:208–212](../../src_reference/INPUT.FOR#L208-L212)). In this fixture both flags are false, so that mismatch does not explain the present EOF and is not expected to affect this case's selected flags. It should be corrected/reviewed before using cases that enable either flux output or dual nonequilibrium.

## Research conclusion and next check

The evidenced immediate defect is the `lEquil=f` initial selector setting paired with 13-field node records. For the declared all-equilibrium sorption case, set that flag to true and explain its initial-record-layout role in `CASE.json`; this preserves the existing scientific values and matches the 13-field branch. If the orchestration explicitly chooses to preserve `lEquil=f`, every node needs three additional explicit zero sorbed values instead. After a separately authorized fixture update, the first run should check only input acceptance and output creation; a successful parser pass alone does not establish scientific validity, and the source-side final-flag issue should remain in the source audit.

## Primary sources

- Pinned source files in this workspace: [`INPUT.FOR`](../../src_reference/INPUT.FOR), [`HYDRUS.FOR`](../../src_reference/HYDRUS.FOR), [`TIME.FOR`](../../src_reference/TIME.FOR).
- Original source archive retained by the project: [`H1D_Src.zip`](../H1D_Src.zip). The archive includes the upstream Fortran source files; the checked-in `src_reference/` files are the task's pinned source snapshot.
- Repository fixtures used as format comparators: [smoke inputs](../../reference/fortran_reference/smoke/) and [derived Example 3 inputs](../../reference/fortran_reference/solute/example3/).
- Recorded failing run and hashes: [RUN.md](../../reference/fortran_reference/solute/example3/RUN.md).

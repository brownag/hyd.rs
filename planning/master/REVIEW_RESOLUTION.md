# Review resolution

## Resolution table

| Finding ID | Disposition | Changed master section | Verification evidence |
|---|---|---|---|
| F-01 | Resolved. Source-observed trial iteration, accepted promotion and failure counter are separated from proposed transactional rollback and `dt` retry semantics. | Source audit; Numerical design | Master now cites `WATFLOW.FOR:1079-1123` for promotion and labels complete rollback/retry as M3 design obligations; review recheck required. |
| F-02 | Resolved. Added five-row scenario table with provenance status, observables, producer and review owner; missing decks/baselines remain explicit. | Validation strategy | Master table covers all five named scenarios and states proposed artifacts are not official examples or baselines. |
| S-01 | Resolved. First-water fixture production now has named planning labels M0.1 and M4.1, with acquisition/construction source, hashes and review evidence required. | Validation strategy; Future wave plans | M4.1 owns dry-column, capillary-rise and cyclic/root fixture production; M0.1 owns oracle/bootstrap provenance. |
| S-02 | Resolved. Proposed future commands have one planning owner and prerequisite artifact condition. | Future wave plans | Master assigns M0.1, M3.1, M4.1, M5.1, M6.1 and M10.1; labels are explicitly planning-only. |
| S-03 | Remaining user decision. Host and format scope stays provisional; master now requires selection before adapter implementation. | Future wave plans; Decisions and open gates | Master names the user gate and preserves Python/R/Node/Wasm/C and schema alternatives without selecting a release target. |

No finding is treated as a numerical acceptance result. The reviews did not run an oracle, numerical suite, or performance benchmark, and this resolution does not claim one.

## Remaining user decisions

The user must still decide the compatibility release label after provenance review; the supported host and initial legacy/schema format subset before adapter implementation; optional hydraulic, drainage, vapor/energy, density, snow/interception, colloid/bacteria, inverse and special chemistry scope; and whether to adopt mixed compatibility precision only or later approve an all-`f64` modernization. The first water gate remains `iModel=0` with the explicitly named boundary/root fixtures, while solute, heat, hysteresis and dual porosity remain later planned work.

## Verification

The master structural check, R01–R20 check, local-link check and whitespace check are rerun after the revisions. Review rechecks must confirm F-01 against the revised Source audit/Numerical design paragraphs, F-02 against the five-row Validation table, S-01 against M0.1/M4.1 ownership, S-02 against command ownership, and S-03 against the retained user gate. No source, implementation, Cargo, fixture, oracle or shared-state file is changed by this task.

## Handoff

The revised [HYDRUS1D_RUST_PORT_PLAN.md](../HYDRUS1D_RUST_PORT_PLAN.md) is ready for orchestrator recheck and user review. Technical completion does not constitute user approval or permission to implement. Future wave labels in the master are planning identifiers only; they do not create runnable execution plans. Reviewers should verify every disposition above against the changed sections before presenting the blueprint to the user.

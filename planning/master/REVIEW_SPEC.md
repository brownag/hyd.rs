# Scientific and scope review

## Verdict

The master is broadly aligned with the confirmed water-first, planning-only scope and preserves the absent-baseline constraint. No release-blocking contradiction was found in the milestone ordering. Two medium findings should be corrected before user handoff: distinguish observed retry behavior from the proposed transactional design, and make the five-scenario provenance/observable mapping explicit in the master rather than relying only on the linked validation appendix. The review did not execute numerical tests or verify the oracle.

## Coverage checked

| ID | Disposition | Master location |
| --- | --- | --- |
| R01 | covered | Goal/scope; Source audit; M0 |
| R02 | covered | Source audit/lifecycle; linked source audit |
| R03 | covered | Architecture; interfaces; M3 ownership |
| R04 | covered | Rust interfaces; numerical ownership |
| R05 | covered | Numerical design; Phase 1 |
| R06 | covered | Numerical design; Phases 2–3 |
| R07 | covered, with finding F-01 | transaction; Phases 3–4 |
| R08 | covered | memory contract; validation |
| R09 | covered, with finding F-02 | I/O; Phases 4/6 |
| R10 | covered | I/O serialization gate |
| R11 | covered | host adapters; platform tests |
| R12 | partial presentation, with F-02 | validation; Phases 4/5 |
| R13 | covered | precision; tolerance policy |
| R14 | covered | component/property tests |
| R15 | covered | decisions; roadmap |
| R16 | covered | risk registry; optional gates |
| R17 | covered | roadmap; future waves |
| R18 | covered | architecture; interfaces; roadmap |
| R19 | covered | Goal/scope planning-only statement |
| R20 | covered | decisions/open gates; roadmap register |

Checked the five named scenarios: dry infiltration, capillary rise, cyclic atmosphere/root, nonequilibrium solute/degradation, and temperature-coupled heat. Checked first acceptance exclusions, later solute/heat/hysteresis order, required later dual porosity, rejected dual permeability, provisional optional features, and no claimed numerical baseline. The review is limited to document/source inspection; it does not establish scientific correctness or fixture availability.

## Findings

### F-01 — Medium — retry behavior is phrased as observed source behavior

Sections: `Source audit` and `Numerical design`, especially “Failed steps restore trial state and reduce `dt`” and the transaction paragraph. `HYDRUS.FOR:417-445` shows convergence counting and a stop after repeated failure; `WATFLOW.FOR:48-168` shows trial iteration, while `WATFLOW.FOR:1079-1123` shows accepted-state promotion. The exact surrounding timestep-retry and complete coupled rollback semantics are not established by those excerpts.

Required correction: label restoration, timestep reduction, and full coupled snapshot behavior as proposed design obligations unless the supporting numerical document supplies direct source evidence. Preserve the risk and fixture gate. Acceptance implication: reviewers can distinguish compatibility evidence from the future transaction contract, preventing an unsupported claim about legacy retries.

### F-02 — Medium — scenario provenance and observables are only summarized

The `Validation strategy` names all five scenarios and says missing decks/results remain gaps, but the master does not give each scenario an explicit provenance status and observable set. This is required for downstream review because R12 and the prompt require scenario-specific evidence, while `SOURCE_AUDIT.md` confirms no input decks or golden outputs exist.

Required correction: add a compact five-row table in the master naming each scenario’s current status (missing/reconstruct/acquire), proposed observable fields/balances, and fixture producer milestone; retain “not official example” language until provenance is obtained. Acceptance implication: R12 becomes auditable without opening an appendix and no future fixture is mistaken for an existing baseline.

No other prioritized scientific or scope blocker was identified. The master correctly keeps `1e-6` as a candidate, separates text layout from numerical parity, calls out mixed precision and `sngl`, retains the Reset/manual stencil discrepancy as a gate, and requires source-specific pivot/balance and rollback work in later milestones.

## Evidence

- `WATFLOW.FOR:308-353`: `Gauss` uses boundary-specific elimination, `rMin` pivot replacement, and explicit `sngl` conversions.
- `WATFLOW.FOR:1079-1123`: `Update` promotes water, velocity, vapor, immobile, and temperature histories after a step; this supports the proposed accepted/trial ownership but does not by itself prove retry restoration.
- `HYDRUS.FOR:399-465`: main-loop ordering and nonconvergence counter/termination; `:438-445` stops after ten failures.
- `OUTPUT.FOR:3-23`, `:325-347`, `:720-883`: time-level, layer-balance, and node-output families and their coupled fields.
- `HYDRUS_RESEARCH.md §§6–8`, `SOURCE_AUDIT.md`, `STATE_AND_CALLS.md`, `FEATURE_MATRIX.md`, `NUMERICAL_DESIGN.md`, `VALIDATION_STRATEGY.md`, and `ROADMAP_AND_RISKS.md` were used as supporting evidence. No numerical command, oracle build, or fixture run was performed.

## Required corrections

1. Resolve F-01 by marking legacy-observed versus proposed retry/rollback semantics explicitly and retaining the source evidence boundary.
2. Resolve F-02 by adding a five-row scenario table with provenance status, observables, and producer milestone to the master or its authoritative validation section.
3. Re-run the master’s structural, R01–R20, link, and whitespace checks after those edits; then recheck each finding against the changed sections.

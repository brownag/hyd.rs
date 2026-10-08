# State — Orchestration 18

Plan status: **Approved by user on 2026-10-08 (“yes”).**
Execution status: **Wave 1 complete; wave 2 is next.** Compatibility backend must not become default until the full in-scope parity gate passes.
Plan: [EXECUTION_PLAN_18.md](EXECUTION_PLAN_18.md). Worker template: [PROMPT_TEMPLATE_18.md](PROMPT_TEMPLATE_18.md).

## Baseline and guardrails

- [x] Reconnaissance completed 2026-10-08; preserve existing uncommitted planning/review work.
- [x] `cargo test --workspace`: 137 integration tests passed; zero unit/doc tests.
- [x] `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: 274 tests passed.
- [x] `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check`: passed.
- [x] No new Fortran build/run during planning reconnaissance; current runner/comparator remain Example 3-specific.
- [x] User decisions recorded: all modes reachable through the audited solute caller path in this orchestration; inactive source helpers are identified, not treated as executed modes; default source-compatible backend after parity gate; preserve FV/BE as explicit alternative; field-specific parity criteria; up to three workers and uncapped diagnostic follow-ups subject to worker escalation.
- [x] Independent spec and standards reviews completed; plan corrected for verification-before-default ordering, task scopes, and escalation rules.
- [x] User approves this execution plan.
- [ ] Full audited solute compatibility/parity gate passes before the default changes.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 1 | 18.1.1 | Source mode matrix and compatibility contract | done |
| 2 | 18.2.1 | Manifest-driven isolated Fortran runner | pending |
| 2 | 18.2.2 | Source-only parity orchestration/comparator | pending |
| 3 | 18.3.1 | Transport/weighting/boundary fixtures | pending |
| 3 | 18.3.2 | Reaction/sorption fixtures | pending |
| 3 | 18.3.3 | Optional transport-property fixtures | pending |
| 3 | 18.3.4 | Nonequilibrium/dual-domain fixtures | pending |
| 3 | 18.3.5 | Biology/filtration/deposition fixtures | pending |
| 4 | 18.4.1 | Compatibility types/state/precision model | pending |
| 5 | 18.5.1 | Transport and matrix assembly kernel | pending |
| 5 | 18.5.2 | Endpoint rows and flux closure | pending |
| 5 | 18.5.3 | Source accounting and observations | pending |
| 6 | 18.6.1 | Reaction chains and equilibrium sorption | pending |
| 6 | 18.6.2 | Active mobile/immobile coefficients | pending |
| 6 | 18.6.3 | Active dual-domain coefficients | pending |
| 6 | 18.6.4 | Bacteria/filtration | pending |
| 6 | 18.6.5 | Deposition/blocking/exclusion | pending |
| 7 | 18.7.1 | Source-ordered linear-mode driver | pending |
| 8 | 18.8.1 | Guard inactive nonlinear mode from accidental exposure | pending |
| 9 | 18.9.1 | Expose compatibility candidate; keep current default | pending |
| 10 | 18.10.1 | All-mode parity report and acceptance | pending |
| 11 | 18.11.1 | Switch default after accepted parity; retain FV/BE | pending |

## Task reports

### 18.1.1 — Source behavior and parity contract

- **Status:** done; created `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` and `planning/tasks/18.1.1.todo.md`.
- Inventoried active and inactive source paths, caller/input mappings, mode constraints, source precision, normalized inputs, trace checkpoints, output semantics, tolerance policy, and default-switch gate.
- Based on the audit, the execution plan now gates parity on every active/callable solute mode and records commented/unreachable helper algorithms without treating them as executable modes.
- Spot-checked `INPUT.FOR` flag initialization and commented helper calls in `SOLUTE.FOR`/`HYDRUS.FOR`; no Fortran execution or code changes.
- `git diff --check` passed; untracked outputs also checked for trailing whitespace. No deviations; precision/compiler and active-mode gaps are documented.

The orchestrator owns wave verification and state updates. Preserve all pre-existing working-tree changes. A blocker requiring out-of-scope edits requires a plan amendment.

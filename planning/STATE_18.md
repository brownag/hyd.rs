# State — Orchestration 18

Plan status: **Approved by user on 2026-10-08 (“yes”).**
Execution status: **Waves 1–3 complete; Wave 4 is next.** Wave 3 fixture runs authenticate raw outputs only; normalized parity remains pending. Compatibility backend must not become default until the full in-scope parity gate passes.
Plan: [EXECUTION_PLAN_18.md](EXECUTION_PLAN_18.md). Worker template: [PROMPT_TEMPLATE_18.md](PROMPT_TEMPLATE_18.md).

## Baseline and guardrails

- [x] Reconnaissance completed 2026-10-08; preserve existing uncommitted planning/review work.
- [x] `cargo test --workspace`: 137 integration tests passed; zero unit/doc tests.
- [x] `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: 274 tests passed.
- [x] `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check`: passed.
- [x] Wave 2 global checks: `cargo test --workspace` passed (137 integration tests); Python unittest discovery passed (290 tests); fmt, Clippy with warnings denied, and diff check passed.
- [x] No new Fortran build/run during planning reconnaissance; current runner/comparator remain Example 3-specific.
- [x] User decisions recorded: all modes reachable through the audited solute caller path in this orchestration; inactive source helpers are identified, not treated as executed modes; default source-compatible backend after parity gate; preserve FV/BE as explicit alternative; field-specific parity criteria; up to three workers and uncapped diagnostic follow-ups subject to worker escalation.
- [x] Independent spec and standards reviews completed; plan corrected for verification-before-default ordering, task scopes, and escalation rules.
- [x] Wave 2 integration review found the Fortran runner authenticates raw files only; plan amended to add fixture observation maps, a strict output normalizer, and paired Rust/Fortran CLI stages before all-mode acceptance.
- [x] Wave 2 raw-output authentication is not labeled normalized parity; output normalization and candidate comparison are separate gates before all-mode acceptance.
- [x] User selected full pinned HYDRUS runs for dual-domain fixtures: use models 6/7 with independently authored decks and active `InitDualPor` state initialization; no call-boundary injection harness. Source audit found the `WATFLOW` `DualPor` exchange-update call commented out, so dynamic transfer/nonzero `SinkIm` is not claimed.
- [x] User approves this execution plan.
- [ ] Full audited solute compatibility/parity gate passes before the default changes.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 1 | 18.1.1 | Source mode matrix and compatibility contract | done |
| 2 | 18.2.1 | Manifest-driven isolated Fortran runner | done |
| 2 | 18.2.2 | Source-only parity orchestration/comparator | done |
| 3 | 18.3.1 | Transport/weighting/boundary fixtures | done |
| 3 | 18.3.2 | Reaction/sorption fixtures | done |
| 3 | 18.3.3 | Optional transport-property fixtures | done |
| 3 | 18.3.4 | Nonequilibrium/dual-domain fixtures | done |
| 3 | 18.3.5 | Biology/filtration/deposition fixtures | done |
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
| 10 | 18.10.1 | Normalize raw Fortran outputs using fixture observation maps | pending |
| 11 | 18.11.1 | Wire paired Fortran/Rust execution and comparison | pending |
| 12 | 18.12.1 | All-mode parity report and acceptance | pending |
| 13 | 18.13.1 | Switch default after accepted parity; retain FV/BE | pending |

## Task reports

### 18.1.1 — Source behavior and parity contract

- **Status:** done; created `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` and `planning/tasks/18.1.1.todo.md`.
- Inventoried active and inactive source paths, caller/input mappings, mode constraints, source precision, normalized inputs, trace checkpoints, output semantics, tolerance policy, and default-switch gate.
- Based on the audit, the execution plan now gates parity on every active/callable solute mode and records commented/unreachable helper algorithms without treating them as executable modes.
- Spot-checked `INPUT.FOR` flag initialization and commented helper calls in `SOLUTE.FOR`/`HYDRUS.FOR`; no Fortran execution or code changes.
- `git diff --check` passed; untracked outputs also checked for trailing whitespace. No deviations; precision/compiler and active-mode gaps are documented.

### 18.2.1 — Generalized isolated Fortran fixture runner

- **Status:** done; updated `tools/run_fortran_reference.py` and its tests; completed `planning/tasks/18.2.1.todo.md`.
- Manifest-defined fixtures now use unique build/run roots, return logical output paths and hashes, and retain source/build/input/output provenance. Incomplete runs do not produce a complete manifest; optional timeout defaults to unbounded.
- Existing Example 3 `RUN.md` is preserved byte-for-byte; smoke and Example 3 command forms remain available.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_fortran_reference` — 14 passed; `git diff --check` passed. No Fortran launch or deviations.

### 18.2.2 — Raw source-only parity CLI and comparison helpers

- **Status:** done; created `tools/run_solute_parity.py`, its tests, and `planning/tasks/18.2.2.todo.md`.
- CLI supports one manifest or serial fixture groups with unique child roots. Reference-only results authenticate raw files and are explicitly **not normalized parity**; pure comparison helpers require exact field/location identity and report first divergence.
- Targeted parity tests — 10 passed; `tools.test_solute_printed_parity` — 10 passed; `tools/compare_solute_reference.py --self-test` — 12 passed.
- Mocked end-to-end runner/group integration verified the shared result contract, nested roots, output paths and hashes; no real Fortran/Rust solver was launched. `git diff --check` passed.
- Provenance comparison uses the runner's exact staged inventory (archive plus `.FOR` source files); unrelated PDF hashes in `SOURCE_MANIFEST.json` are excluded, with regression tests for missing/changed staged entries.

### 18.3.1–18.3.3 — Reference fixture groups

- **Status:** done; fixture files and task reports committed in `85f8bd6`, `292a70c`, and `3210f5d`.
- 18.3.1: 18 transport fixtures cover all 14 active endpoint-code × native-flow combinations; `kTopCh=-2` positive/negative cases are included. Raw outputs authenticated; no normalized parity claim.
- 18.3.2: four reaction/sorption fixtures authenticated; all maps converted to flat `solute-observation-map-v1`. Per-map counts: 60 observations for each chain fixture, 22 for the single-species case; retained inputs and outputs unchanged.
- 18.3.3: five property fixtures authenticated, including vapor-enabled diagnostics. Density and moisture-dependent paths are documented inactive from the active caller/input path.
- Three workers ran in separate worktrees; all run attempts remain in `.development/18-worktrees/18.3.{1,2,3}/.development/`. Each task's exact hashes and output inventory are in its TODO and retained manifests. Reaction maps use the versioned flat observation schema; their input/output provenance was rechecked after conversion without rerunning.
- Orchestrator check across imported fixtures: all 27 `CASE.json` inputs and observation maps validate. Acceptance is raw output authentication only; normalization and paired parity remain pending.

### 18.3.4 — Nonequilibrium/dual-domain fixtures

- **Status:** done under the 2026-10-09 user-approved full-run amendment; fixtures/task report committed in `b5ef3ee`.
- Added five fixtures: mobile-immobile and matched control, dual models 6 and 7, and single-domain control. Both water models are accepted through the pinned active reader and maps include printed `VolumeIm` evidence.
- Full pinned reference-only run authenticated all five cases under `.development/18.3.4/attempt-20261009-unsaturated/`; orchestrator revalidated all five run manifests, declared output hashes, pinned source/archive/patch/executable provenance, fixture inputs, and observation maps.
- Discriminators: dual model 6 `VolumeIm=0.33455E+00 cm`; model 7 `0.15740E+00 cm`; single-domain control omits the `W-volumeI[L]` row. Mobile-immobile `cvCh1` at 0.1 h differs from its zero-immobile control (`-0.30400E-01` vs `-0.34400E-01`). These are raw printed outputs, not normalized parity.
- Active `InitDualPor` initializes `ThNewIm`/`ThOldIm` and `Solute` consumes dual storage. `WATFLOW.FOR:42–46` comments out `call DualPor`; `SinkIm=0`, so dynamic exchange/nonzero-`SinkIm` behavior is explicitly not claimed. `NEquil`, `SorbConc`, and `NonEqInit` remain inactive.

### 18.3.5 — Biology/filtration/deposition fixtures

- **Status:** done; committed in `cf651ff`.
- Cases discriminate blocking codes 1–5 (including source no-op code 3) and filtration/deposition with valid positive particle/grain diameters. Reader mappings for bacteria/filtration and `ChPar` values are present; `Exclusion` remains inactive.
- Reference-only run authenticated all six raw output inventories and source/archive, patch, executable, request, and input hashes under `.development/18.3.5/attempt-20261009T000000Z/`; no normalized parity claim.
- Orchestrator contract validation after 18.3.4: all 38 manifests and versioned observation maps validate; all five new maps have unique field/location identities. New attempts remain raw authentication only, not normalized parity.

### Wave 3 verification

- `cargo test --workspace` — passed (137 integration tests; zero unit/doc tests).
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` — 291 passed; one expected guarded “solver execution forbidden” diagnostic was emitted inside a passing test.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` — passed.
- The five 18.3.4 retained raw-run manifests, declared output hashes, pinned source/archive/patch/executable provenance, and fixture input hashes were independently revalidated. No normalized parity claim.

### Wave 2 verification

- `cargo test --workspace` — 137 integration tests passed; Python unittest discovery — 291 tests passed; fmt and Clippy with warnings denied passed.
- This wave establishes isolated raw Fortran execution and parity plumbing only. Fixture-specific normalization and paired Rust comparison remain planned in Waves 3 and 10–11; no parity claim is made here.

The orchestrator owns wave verification and state updates. Preserve all pre-existing working-tree changes. A blocker requiring out-of-scope edits requires a plan amendment.

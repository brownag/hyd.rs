# State — Orchestration 18

Plan status: **Approved by user on 2026-10-08 (“yes”).**
Execution status: **Waves 1–6 complete; Wave 7 active.** 18.7.4 selector/state mapping was accepted 2026-10-12. 18.7.1 contract amendment is implemented; endpoint kernels and independent acceptance coverage remain incomplete. Accepted Wave 5 work includes 18.5.1 and 18.5.3; 18.5.2 is incomplete and superseded by 18.7.1. Wave 3 fixtures authenticate raw outputs only; normalized parity remains pending. Default backend must not change before the full parity gate.
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
- [x] User selected full pinned HYDRUS runs for dual-domain fixtures: use models 6/7 with independently authored decks and active `InitDualPor` state initialization; no call-boundary injection harness. **Correction 2026-10-10:** fixed-form reading shows `DualPor` is actively called; dynamic transfer is in scope, and retained fixtures require revalidation for exchange discrimination.
- [x] User approves this execution plan.
- [ ] Full audited solute compatibility/parity gate passes before the default changes.

## Wave 7 sequencing and worker-tier amendment — 2026-10-13

- Wave 7 is now three sequenced sub-waves because `types.rs`/`adapter.rs` are single-owner files that 18.7.1, 18.7.3 and 18.7.8 each must extend: **7A** 18.7.1 ∥ 18.7.6, **7B** 18.7.2 ∥ 18.7.3, **7C** 18.7.5 ∥ 18.7.7 ∥ 18.7.8. 18.7.3 and 18.7.8 specs were re-scoped in `EXECUTION_PLAN_18.md` (escalation rule 4: shared-state additions are a decomposition gap, not ad hoc permission widening).
- Green baseline re-recorded at `726662f`: `cargo test --workspace` — 179 passed, 0 failed; `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` — 291 passed, OK.
- Worker-tier constraint: the `r9700` local server (`llama-cpp.service`, port 8001) is in a crash loop (`invalid device: ROCm0`, restart counter >4000), so the planned cheap/parallel worker tier is unavailable. Wave 7 workers run one at a time on the single reachable agent endpoint (`strata/qwen3.8-flash-next-unsloth-ud-iq4_xs`, the same engine as the orchestrator), which also caps practical concurrency below the approved three.
- Fixed-form re-verification by the orchestrator: `src_reference/WATFLOW.FOR:44–46` (`call DualPor`) and `WATFLOW.FOR:1067–1068` (`WTransf` accumulation) are column-6 continuations, i.e. executable. The 18.6.6 correction stands.

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
| 4 | 18.4.1 | Compatibility types/state/precision model | done |
| 5 | 18.5.1 | Transport and matrix assembly kernel | done; fixed-form corrections accepted |
| 5 | 18.5.2 | Endpoint rows and flux closure | superseded/incomplete; replaced by 18.7.1 |
| 5 | 18.5.3 | Source accounting and observations | done; fixed-form corrections accepted |
| 6 | 18.6.6 | Fixed-form reachability re-audit | done; consistency review accepted |
| 7 | 18.7.4 → 18.7.1–18.7.3, 18.7.5–18.7.8 | Selector mapping first, then endpoint/active kernels and fixtures | 18.7.1 in progress; contract and partial kernel present, acceptance coverage not met |
| 8 | 18.8.1–18.8.2 | Conditional initialization and selector-2 audit resolution | amended; pending |
| 9 | 18.9.1–18.9.5 | Source-faithful process kernels | pending |
| 10 | 18.10.1 | Source-ordered solute step driver | pending |
| 11 | 18.11.1 | Nonlinear iteration and convergence | pending |
| 12 | 18.12.1 | Expose compatibility candidate; keep current default | pending |
| 13 | 18.13.1 | Normalize raw Fortran outputs using fixture observation maps | pending |
| 14 | 18.14.1 | Wire paired Fortran/Rust execution and comparison | pending |
| 15 | 18.15.1 | All-mode parity report and acceptance | pending |
| 16 | 18.16.1 | Switch default after accepted parity; retain FV/BE | pending |

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
- Active `InitDualPor` initializes `ThNewIm`/`ThOldIm`, and `Solute` consumes dual storage. **Correction 2026-10-10:** the earlier statement that `WATFLOW.FOR:42–46` comments out `call DualPor` was wrong; the call is an executable fixed-form continuation under `if(iDualPor.gt.0)`. Dynamic exchange is active-path scope and retained fixtures need revalidation for nonzero `SinkIm` discrimination. The continued `NEquil`, `SorbConc`, and `NonEqInit` calls are also conditionally active, as documented by the 18.6.6 audit.

### 18.3.5 — Biology/filtration/deposition fixtures

- **Status:** done; committed in `cf651ff`.
- Cases discriminate blocking codes 1–5 (including source no-op code 3) and filtration/deposition with valid positive particle/grain diameters. Reader mappings for bacteria/filtration and `ChPar` values are present. `Exclusion` has a conditional continued caller and is untested; external state mapping remains a blocker.
- Reference-only run authenticated all six raw output inventories and source/archive, patch, executable, request, and input hashes under `.development/18.3.5/attempt-20261009T000000Z/`; no normalized parity claim.
- Orchestrator contract validation after 18.3.4: all 38 manifests and versioned observation maps validate; all five new maps have unique field/location identities. New attempts remain raw authentication only, not normalized parity.

### Wave 3 verification

- `cargo test --workspace` — passed (137 integration tests; zero unit/doc tests).
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` — 291 passed; one expected guarded “solver execution forbidden” diagnostic was emitted inside a passing test.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` — passed.
- The five 18.3.4 retained raw-run manifests, declared output hashes, pinned source/archive/patch/executable provenance, and fixture input hashes were independently revalidated. No normalized parity claim.

### 18.4.1 — Compatibility types, strict adapter, and precision model

- **Status:** done; implementation and task report committed in `bce44da`.
- Added private `solute_fortran_compat` types for source precision, units/availability, switches, boundaries, species/process fields, trace data, and source-index mappings. Added compileable placeholder modules for later kernels; the existing FV/BE API and dispatch remain unchanged.
- Added `serde_json` and `sha2` for serialized fixture request parsing and canonical hash verification. Adapter maps both accepted schemas across all 38 Wave 3 manifests, rejects duplicate/unknown keys and invalid units/shapes/modes, and keeps unavailable values explicit.
- Coordinates in current fixtures provide range metadata, not numeric arrays; the adapter preserves that metadata and explicit source permutation but does not claim physical coordinate values. No solver parity claim.
- Targeted tests: `cargo test -p hydrus-core solute_fortran_compat::types::tests` — 4 passed; `cargo test -p hydrus-core solute_fortran_compat::adapter::tests` — 4 passed, including 38-fixture mapping/hash sweep.

### Wave 4 verification

- `cargo test --workspace` — 8 unit and 137 integration tests passed; zero doc tests.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` — 291 passed; one expected guarded solver-execution diagnostic was emitted inside a passing test.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` — passed.

### Wave 5 source-reading amendment — 2026-10-10

- Source review found a fixed-form parsing error in all three uncommitted Wave 5 implementations: `!` in physical column 6 is a continuation marker, not a comment. Relevant executable continuations include `MatSet` Level-1 interior/endpoint RHS, old-level boundary contributions, and `MassTran` `cvChIm` branch equations.
- The plan requires fixed-form interpretation and source-derived tests; 18.5.3 covers reachable `cvChIm`. The later audit found further conditional calls, now assigned explicit follow-on tasks.
- Earlier targeted test passes (18.5.1: 5 transport + 4 assembly; 18.5.2: 3 boundary; 18.5.3: 4 accounting) are superseded as acceptance evidence because tests encoded the mistaken source interpretation.
- Corrected 18.5.1 and 18.5.3 were independently re-reviewed and integrated; 18.5.2 remains preserved in its worktree as incomplete and is superseded by 18.7.1.
- Fixed-form caller audit corrected reachability for `DualPor`, `FluxConc`, helper updates, initialization, accounting, moisture factors, root sink, exclusion, and nonlinear convergence. Source-callability remains distinct from runtime-selected coverage; see the current matrix and task DAG.
- 18.5.2's correction attempt added direct bottom flux evaluation and orientation tests, but its row/RHS/flux API still accepts opaque precomputed aggregates; it is not accepted and has been replaced in the plan by 18.7.1.

### Wave 5 verification — 2026-10-12

- Accepted and committed 18.5.1 (`fda4f38`) and 18.5.3 (`4fe1d3d`) after fixed-form/source-order review; Clippy correction commits are `6394d52` and `ee1c38c`.
- Targeted tests: transport 5, assembly 4, accounting 7 passed. `cargo test --workspace` passed (24 unit, 137 integration; zero doc tests); Python discovery passed (291 tests); `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- Clippy allowances preserve exact `BanSol` subtraction and `Deposit` source literal/argument order. No normalized or cross-language parity claim.
- 18.5.2 remains incomplete and superseded. Wave 6 task 18.6.6 remains under consistency review.

### Wave 6 consistency review — 2026-10-12

- 18.6.6 evidence distinguishes syntax from runtime selection and identifies conditional callers for beginning/end `MassTran`, `NEquil`, `SorbConc`, `MassInit`, `NonEqInit`, nonlinear convergence, moisture factors, `SetSSnk`, and `Exclusion`; selector-2 `MoistDepIn` remains unresolved/unselected by the documented flag mapping.
- Consistency review reconciled the matrix and task DAG: Wave 7 uses unique task IDs 18.7.1–18.7.8, Wave 8 owns initializers and selector-2 resolution, and downstream numbering/dependencies align. Dynamic dual-exchange fixture discrimination remains untested.
- No implementation, runtime selection, normalized parity, or default change is claimed.

### Wave 2 verification

- `cargo test --workspace` — 137 integration tests passed; Python unittest discovery — 291 tests passed; fmt and Clippy with warnings denied passed.
- This wave establishes isolated raw Fortran execution and parity plumbing only. Fixture-specific normalization and paired Rust comparison remain planned in Waves 3 and 10–11; no parity claim is made here.

The orchestrator owns wave verification and state updates. Preserve all pre-existing working-tree changes. A blocker requiring out-of-scope edits requires a plan amendment.

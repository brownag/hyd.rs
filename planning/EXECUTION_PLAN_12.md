# Orchestration 12 — M5 scientific-comparison catch-up

Status: **approved; Wave 2 accepted after amendment C; Wave 3 ready.**
Authority: the updated sequential master plan, [scientific-comparison research](master/SCIENTIFIC_COMPARISON_RESEARCH.md), and [text-fixture format audit](master/TEXT_FIXTURE_FORMAT_RESEARCH.md).

## Goal and scope

Close the gap between the implemented M5 contract and scientific comparison by producing one provenance-reviewed in-scope solute fixture, running it through the pinned Fortran snapshot and Rust, comparing independent observables, and reviewing field-specific tolerances. Start with manual Example 3 (sequential decay), subject to the scope and completeness audit in task 12.1.1. If its missing machine-input details cannot be recovered, construct an explicitly derived source-matched fixture or report the blocker; do not call it a manual reproduction.

This catch-up is placed after Orchestration 11 and before Phase 6 heat/temperature planning. It covers selected M5 only. It does not add solute physics, alter water behavior, broaden M5 feature support, or claim general HYDRUS release compatibility. Keep source-snapshot parity, analytical/physical validation, text layout, and release compatibility as distinct evidence levels.

## Acceptance principles

- Match mesh, units, initial pools, species order, water-content/face-flux history, boundaries, and comparison times between Fortran and Rust. If the run is coupled, compare and report the water trajectory as well.
- Compare dissolved profiles, supported sorbed pools, boundary flux histories, inventories and independently recomputed per-species balances at common coordinates/times. Compare Example 3 against an independently reviewed analytical result if its configuration supports that benchmark.
- Record source/archive/patch/executable/compiler/flags/platform/input/output hashes, run status, completion markers, warnings, and parser schema. Keep raw runs isolated and reproducible.
- Calibrate tolerances per observable using repeatability, precision sensitivity and paired residual evidence. The existing `1e-6` value is only a candidate; it is not an acceptance threshold.
- If fixture provenance, oracle output, a supported equation mapping, or tolerances cannot be established, preserve the blocker and leave M5 scientific validation unmet.

## Reconnaissance and baseline

- Current root: `/home/andrew/AI/hyd.rs`; prior plan: Orchestration 11, all implementation waves complete.
- `planning/master/PHASE_0_REFERENCE_EVIDENCE.md` records a working patched GNU Fortran/Linux build and a noncanonical water smoke. Wave 2 added a fixture-specific `--case` runner mode and focused tests; amendment C resolved profile parsing and produced verified solute outputs. Scientific comparison remains the work of Waves 3–4.
- `planning/master/PHASE_5_SOLUTE_EVIDENCE.md` records constructed contract fixtures, no manual solute deck or expected output, and no calibrated tolerance or parity evidence.
- At plan drafting, the latest serial workspace gates are green: `cargo test --workspace` — 112 tests, 0 failures; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed. Re-run and record the baseline before Wave 1; stop on unrelated failures.
- No `AGENTS.md` applies within this repository. Do not change the immutable `src_reference/` snapshot or the Fortran portability patch.

## Wave table

| Wave | Task | Deliverable | Depends on | Theme |
|---|---|---|---|---|
| 1 | 12.1.1 | Complete, provenance-labeled M5 comparison fixture definition/input | Baseline gate | Fixture selection and scope |
| 2 | 12.2.1 | Isolated Fortran solute run and verified output manifest | 12.1.1 | Reference execution |
| 3 | 12.3.1 | Normalized observables and paired Rust/reference comparison | 12.2.1 | Parsing and comparison |
| 4 | 12.4.1 | Reviewed tolerance decision and M5 evidence handoff | 12.3.1 | Calibration and acceptance |

Waves are serial because each consumes the preceding fixture/output artifact. Worker tasks are limited to 12 micro-plan steps and their listed files. Workers run only their task's targeted commands; the orchestrator runs workspace gates after each wave. Follow [PROMPT_TEMPLATE_12.md](PROMPT_TEMPLATE_12.md) and the escalation ladder in [PROMPT_TEMPLATE_11.md](PROMPT_TEMPLATE_11.md).

## Task specifications

### 12.1.1 — Freeze an in-scope M5 comparison fixture

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** green baseline gate.
- **Must edit:** `planning/master/SCIENTIFIC_COMPARISON_RESEARCH.md`; **can create:** `reference/fortran_reference/solute/example3/CASE.json`, `README.md`, and required input files; **can create:** `planning/tasks/12.1.1.todo.md`; scratch `.development/12.1.1/`.
- **Anchors:** manual §11.3, printed pp. 157–159; `src_reference/INPUT.FOR` `ChemIn` around 1289 and solute-file setup around 1464; `src_reference/SOLUTE.FOR` `Solute` around 6, `Coeff` around 241, `NEquil` around 1078, `MassTran` around 1319; `planning/master/PHASE_5_SOLUTE_EVIDENCE.md` scope; research note's Example 3 inventory.
- **Spec:** Establish which Example 3 equations map to currently supported Rust M5. Produce a complete normalized case record and the Fortran input bundle, with every value attributed to the manual, source, or a clearly labeled derived choice. Include spatial domain/discretization, initial dissolved and sorbed state, water/face-flux trajectory, boundaries, times, units, species order, processes, and expected observables. If manual details are incomplete, retain Example 3 as motivation and specify a self-contained derived source-matched case instead; do not invent a manual deck or claim reproduction. Document unsupported model differences.
- **Do not touch:** solver code/tests, `src_reference/`, portability patch, runner, dependencies, generated oracle outputs.
- **Acceptance:** Review confirms all numeric fixture values have provenance labels; the case exercises only supported M5 behavior; no undeclared inputs remain; `python3 -m json.tool reference/fortran_reference/solute/example3/CASE.json` passes. The TODO names any missing reference-input or artifact-permission facts for the orchestrator.

### 12.2.1 — Add isolated Fortran solute execution

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 12.1.1.
- **Must edit:** `tools/run_fortran_reference.py`; **can edit:** `reference/fortran_reference/README.md`; **can create:** `tools/test_run_fortran_reference.py`, `reference/fortran_reference/solute/example3/RUN.md`, `planning/tasks/12.2.1.todo.md`; scratch `.development/12.2.1/` for the executable and raw output.
- **Anchors:** runner CLI and `run_smoke`; harness guarantees in `reference/fortran_reference/README.md`; exit markers and hashes in `reference/fortran_reference/SMOKE_RUN.md`; output creation in `src_reference/INPUT.FOR` `OpenSoluteFiles`; solute output records in `src_reference/OUTPUT.FOR` and the 4.08 manual Chapter 13.
- **Spec:** Extend the isolated runner with a fixture-specific solute mode, retaining immutable staging, fresh input/output directories, timeout, captured logs, output completeness checks, hashes, and the existing smoke command unchanged. Record required solute output files and semantic completion markers only after verifying actual outputs. Keep raw generated outputs under `.development/12.2.1/`; do not commit them or source-derived golden data unless the repo's artifact review supports retaining them.
- **Do not touch:** `src_reference/`, compatibility patch, Rust solver, non-solute smoke behavior, dependencies, unrelated tools.
- **Acceptance:** `python3 -m unittest tools.test_run_fortran_reference` passes; the fixture-specific command `python3 tools/run_fortran_reference.py --case reference/fortran_reference/solute/example3 --timeout-seconds 60` completes and records a manifest with source, patch, executable, compiler, platform, input/output hashes, status, markers and warnings. If the case does not execute solute outputs, stop and document the blocker without treating water output as a pass.

#### 12.2.1 scope amendment A — selector record alignment

- **Reason:** The first two fixture executions failed in `BasInf` while parsing `SELECTOR.IN`, before solute execution. Source inspection indicates the basic-information reader consumes two records before its heading and units; the derived selector may be missing one descriptive record.
- **Allowed correction:** Edit only `reference/fortran_reference/solute/example3/SELECTOR.IN` record layout and the corresponding format/provenance description in that fixture's `CASE.json` and `README.md`. Keep all model values, flags, units, and scientific choices frozen. No runner or model changes are allowed under this amendment.
- **Retry:** One mid-tier worker retry is allowed after its revised TODO is reviewed and approved. Rerun the selector parse check and focused runner tests, then the fixture command once. If it fails at a new or repeated blocker, stop and document; do not launch another correction attempt without a new orchestrator decision.

#### 12.2.1 scope amendment B — profile record layout

- **Reason:** Amendment A cleared `BasInf`; its single retry then failed in `NodInf` while reading `PROFILE.DAT`. Source inspection shows `NodInf` reads the file version, a record count, skips that many descriptive records, then reads the node header and immediately expects node records. The fixture has an extra column-label record after the node header, which is not consumed by this reader.
- **Orchestrator decision:** Permit one orchestrator-owned format correction and one fixture execution. This does not reopen worker retries.
- **Allowed correction:** Remove only the non-input column-label record after the node header. Do not change the record count, node values, material assignments, species concentrations, selector settings, runner, or scientific choices. Update format provenance in `CASE.json`/`README.md` and the run record.
- **Stop rule:** Run JSON validation and focused runner tests, then execute the fixture command once. If it fails, stop and mark Wave 2 blocked; no further correction attempts under amendment B. Its one run failed at `INPUT.FOR:251` with end-of-file while reading `Profile.dat`. The later user-requested amendment C is a new decision based on a source audit.

#### 12.2.1 scope amendment C — equilibrium initial-profile mode

- **Authority and diagnosis:** The user's new amendment request follows the primary-source audit in `planning/master/TEXT_FIXTURE_FORMAT_RESEARCH.md`. `BasInf` reads `lEquil=f` from the current `SELECTOR.IN`; `NodInf` runs before `ChemIn` and therefore requests 16 values per node (including three initial sorbed values), while every current `PROFILE.DAT` row has 13. This explains the recorded EOF at `INPUT.FOR:251`. The fixture declares all-equilibrium sorption (`Fraction=1`, immobile water `0`) and zero initial dissolved/sorbed pools.
- **Chosen correction:** Set the initial `lEquil` selector flag to `t` so `NodInf` reads the existing 13-value rows. This is the source-compatible derived-case choice; do not add sorbed columns or alter any node, material, concentration, hydraulic, reaction, boundary, time, or unit value. Update the `CASE.json`/fixture `README.md` format and mode provenance to state why the initial selector flag is true and that `ChemIn` later derives its own equilibrium mode. Keep the case labeled derived, not a recovered manual deck.
- **Files in scope:** **Must edit:** `reference/fortran_reference/solute/example3/SELECTOR.IN`, `CASE.json`, `README.md`, `RUN.md`, `planning/tasks/12.2.1.todo.md`, and the task report in `planning/STATE_12.md`. **Can edit:** `tools/test_run_fortran_reference.py` only to add a focused fixture-layout check for the selected `lEquil` branch, node count, per-row width, and required conditional inputs. **Read only:** `PROFILE.DAT`, `LEVEL_01.DIR`, `tools/run_fortran_reference.py`, `src_reference/`, and the portability patch. Scratch remains `.development/12.2.1/`.
- **Preflight:** Before execution, check the exact selector flag position, 1001 profile rows of 13 numeric fields, `NumNP=1001`, `NS=3`, `NObs=0`, and the source conditions that make `ATMOSPH.IN`, `Meteo.in`, and `Mater.in` unnecessary. Note that `LEVEL_01.DIR` is not consumed when the runner passes the data-path argument. Compare staged source/patch and record the known final `lFlux`/`lDualNEq` argument-order mismatch; both flags are false in this case, so it is a separate audit caveat.
- **Execution gate:** Because earlier small-model and mid-tier retries are exhausted, assign this evidence-backed correction to one frontier-capable worker with a fresh micro-plan and orchestrator approval. The worker runs `python3 -m json.tool reference/fortran_reference/solute/example3/CASE.json`, `python3 -m unittest tools.test_run_fortran_reference`, and exactly one fresh `python3 tools/run_fortran_reference.py --case reference/fortran_reference/solute/example3 --timeout-seconds 60` after preflight. The orchestrator reviews the one-flag diff, source mapping, manifest, and actual solute files before accepting Wave 2.
- **Stop and acceptance:** If the run fails or lacks per-species solute output/completion markers, record the exact blocker and stop; no further fixture retry is authorized by this amendment. Wave 3 remains blocked until the run manifest contains verified solute outputs and all provenance/hashes required by the original 12.2.1 acceptance.

### 12.3.1 — Parse and compare Rust/reference observables

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 12.2.1.
- **Must edit:** `crates/hydrus-core/tests/solute_fixture_parity.rs`, `tools/compare_solute_reference.py`; **can edit:** `reference/fortran_reference/solute/example3/README.md`; **can create:** normalized reference observations only as allowed by the evidence/artifact review, `planning/tasks/12.3.1.todo.md`; scratch `.development/12.3.1/`.
- **Anchors:** Fortran `NOD_INF.OUT`, `SOLUTE.OUT`, `BALANCE.OUT`, and `RUN_INF.OUT` field definitions from manual Tables 13.2, 13.3, and 13.5; Rust M5 `SoluteTransportInput`, `SoluteProcessInput`, and `SoluteStepAccounting`; research note's observable/sign/unit mapping.
- **Spec:** Add strict parsing for only the verified fixture output records; normalize species, units, signs, coordinates, and times. Run the matching Rust fixture with the same initial pools and water history. Compare dissolved profiles, applicable supported pools, integrated inventories, boundary terms and independently recomputed species balances at common points/times. Preserve raw values and emit residual summaries; compare manual Example 3 with an independent analytical result only if the frozen fixture maps to that equation. Do not infer or hard-code a tolerance from Rust output itself.
- **Do not touch:** public solver API, water solver, reference source/patch, dependencies, unrelated parsers or tests.
- **Acceptance:** `cargo test -p hydrus-core --test solute_fixture_parity` and `python3 tools/compare_solute_reference.py reference/fortran_reference/solute/example3` pass schema/shape/unit/sign checks and produce a residual report. A numeric mismatch is evidence to report, not permission to weaken checks. If no independent/reference values are available, report the precise blocker and leave this task incomplete.

### 12.4.1 — Review tolerances and close the M5 evidence handoff

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 12.3.1.
- **Must edit:** `planning/master/SCIENTIFIC_COMPARISON_RESEARCH.md`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`, `planning/master/VALIDATION_STRATEGY.md`; **can edit:** the fixture's normalized acceptance thresholds/metadata; **can create:** `planning/tasks/12.4.1.todo.md`.
- **Anchors:** paired fixture residual report; reference manifests/repeatability records; tolerance policy in `planning/master/VALIDATION_STRATEGY.md`; M5 unmet rows in `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`.
- **Spec:** Characterize repeated reference runs and material compiler/precision sensitivity, summarize paired residuals by observable, and propose field-specific absolute/relative thresholds and balance zero-denominator policy with rationale. Review thresholds against analytical/invariant evidence and mesh/time sensitivity where available; freeze only justified values into the comparison regression. Update M5 evidence to state precisely which source-parity/analytical claims passed and which remain unavailable. Do not claim universal HYDRUS compatibility, physical validation from source parity alone, or release readiness.
- **Do not touch:** source/solver equations, broadened feature support, dependencies, release claims, unrelated evidence.
- **Acceptance:** the runner/parser commands from dependencies and `cargo test -p hydrus-core --test solute_fixture_parity` pass. The evidence table links exact fixture/run hashes and records reviewed tolerances or explicitly leaves the gate unmet with its blocker. The orchestrator owns workspace gates.

## Exit criteria

- One complete, provenance-reviewed, currently supported M5 fixture has reproducible solute output from the pinned Fortran snapshot and matching Rust input/water trajectory.
- Verified parsers compare independent observables at common space/time locations; mass balance is independently recomputed.
- Tolerances are per observable, justified from repeatability, precision sensitivity, paired residuals and available analytical evidence, and retained as a regression gate.
- M5 evidence separates source-snapshot parity, analytical/physical evidence, text-layout compatibility, and release compatibility; unavailable claims remain explicit.

If a fixture, runnable solute output, artifact basis, or defensible tolerance cannot be established, leave M5-V open and record the specific blocker. Do not proceed to Phase 6 planning as if the gate passed.

# Orchestration 12 — M5 scientific-comparison catch-up

Status: **draft; implementation has not started.**
Authority: the updated sequential master plan and [scientific-comparison research](master/SCIENTIFIC_COMPARISON_RESEARCH.md).

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
- `planning/master/PHASE_0_REFERENCE_EVIDENCE.md` records a working patched GNU Fortran/Linux build and a noncanonical water smoke only. `tools/run_fortran_reference.py` currently exposes `--smoke` only.
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

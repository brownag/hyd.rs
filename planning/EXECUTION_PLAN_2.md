# Orchestration 2 — Phase 0 Fortran reference executable and provenance

Status: **draft for user approval; do not execute.**
Master sequence authority: [HYDRUS1D_RUST_PORT_PLAN.md](HYDRUS1D_RUST_PORT_PLAN.md), Phase 0.
Execution contract: [wave-orchestration](/home/andrew/workspace/aigb-skills/skills/wave-orchestration/SKILL.md).

## Goal and scope

Create a committed, reviewable portability patch series and harness for an isolated GNU Fortran reference executable from the immutable `src_reference/` snapshot. The phase must finish with a successful run using a committed, explicitly noncanonical smoke input. It records exact source, patch, compiler, command, environment, and output evidence; it establishes no scientific fixture, tolerance, parity, release-identity, or benchmark claim.

In scope: a patch series applied into an untracked staging directory; GNU Fortran on the current Linux environment; source/hash guards; build and run scripts; one minimal noncanonical smoke input; a committed run record; and a concise Phase 0 evidence document.

Out of scope: changes to `src_reference/`; Rust/Cargo work; canonical or scientific fixtures; numerical comparison; golden outputs; tolerance calibration; performance work; host bindings; other compilers or operating systems; and Phase 1 hydraulic work.

### Verbatim user decisions

> “yes” — Phase 0 may create a minimal, explicitly noncanonical smoke input solely to verify the patched Fortran reference executable.

> “yes” — GNU Fortran on the current Linux environment is the Phase 0 target.

> “yes” — a successful smoke run is required before Phase 0 finishes.

> “yes” — the portability patch and harness are committed; generated executables and run outputs stay untracked.

> “committed patch series” — patch the immutable source into an isolated build directory; do not create a second committed source tree.

## Reconnaissance

- Ten immutable fixed-form files in `src_reference/`, 11,053 lines total; their hash manifest is [SOURCE_MANIFEST.json](master/SOURCE_MANIFEST.json).
- `gfortran` is available. The unmodified syntax command fails at `HYDRUS.FOR:29` (`MSFLIB` unavailable) and `TIME.FOR:551` (integer used as an `IF` condition).
- No Cargo manifest, test suite, input deck, executable, or project convention file exists. Python 3 and `pdftotext` are available.
- Relevant source anchors: `HYDRUS.FOR:20,29,99-109,853-957`; `TIME.FOR:443-551`; `INPUT.FOR:3,208,536,866`; `OUTPUT.FOR:3,325,720`.
- Baseline is intentionally red: the unmodified reference source cannot compile. Its exact failure is a Phase 0 input, not a regression.

## Decision log

| ID | Decision |
|---|---|
| D01 | Use GNU Fortran on the current Linux environment only. |
| D02 | Keep `src_reference/` immutable; commit a patch series, not a duplicate source tree. |
| D03 | Commit scripts, smoke input, and run record; leave staged source, executable, and outputs untracked. |
| D04 | Require one successful explicitly noncanonical smoke run; never present it as a scientific fixture or parity result. |
| D05 | Start workers with gpt-5.6-luna at low effort; use gpt-5.6-terra at medium effort only under the escalation rule. |

## Wave table

| Wave | Task | Depends on | Theme |
|---|---|---|---|
| 1 | 2.1.1 | Pre-wave gate | Portability patch series |
| 2 | 2.2.1 | 2.1.1 | Deterministic stage/build harness |
| 3 | 2.3.1 | 2.2.1 | Noncanonical smoke input and successful run |
| 4 | 2.4.1 | 2.3.1 | Evidence review and Phase 0 handoff |

All tasks are serial because each phase artifact is an input to the next. The orchestrator alone updates `STATE_2.md`, integrates commits, and runs phase-end checks.

### Pre-wave gate

The orchestrator must run, record in `STATE_2.md`, and pass these commands before starting 2.1.1:

```sh
git status --short
sha256sum planning/H1D_Src.zip src_reference/*.FOR
gfortran -fsyntax-only -std=legacy -ffixed-line-length-none src_reference/*.FOR
```

The third command is expected to fail only with the existing `MSFLIB` and `TIME.FOR:551` diagnostics. Any changed source hash, unexpected working-tree edit, or different failure blocks Wave 1 pending review.

## Shared constraints

- Preserve `src_reference/`, `planning/H1D_Src.zip`, PDFs, and original notices byte-for-byte.
- Stage under `.development/reference-build/`; generated source, executable, logs, outputs, and temporary directories are untracked.
- A committed patch must apply cleanly to the hash-guarded source snapshot. Do not rewrite algorithms, promote precision wholesale, or silently change `iMetHour` semantics.
- Every worker uses only its named targeted command. No Rust, Cargo, fixture, benchmark, or full-suite work belongs here.
- A smoke input must state that it is noncanonical and may not be used for scientific validation.

## Task specifications

### 2.1.1 — Define the GNU Fortran portability patch

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** preflight.
- **Must edit:** none.
- **Can create:** `reference/fortran_reference/0001-gfortran-portability.patch`; `reference/fortran_reference/PATCH_NOTES.md`; `planning/tasks/2.1.1.todo.md`.
- **Scratch only:** `.development/2.1.1/`.
- **Read:** `src_reference/HYDRUS.FOR`, `src_reference/TIME.FOR`, `master/SOURCE_AUDIT.md`, `master/STATE_AND_CALLS.md`.
- **Reference implementation:** none; preserve the source snapshot and express all changes as one GNU Fortran portability patch.
- **Anchors:** `rg -n 'use MSFLIB|SIGNALQQ|SIG\$|GETARG|iMetHour|if\(iMetHour\)' src_reference/{HYDRUS,TIME}.FOR`.

Create one reviewable patch that permits GNU Fortran compilation by isolating platform-specific MSFLIB signal registration and correcting only the compiler-invalid `iMetHour` condition after proving its integer flag semantics from local assignments. Record every changed line, preserved behavior, uncertainty, source hash, and explicit non-goals. The patch must be applicable with `git apply` to a copied source tree; it must not alter originals.

**Acceptance:** PATCH_NOTES also inventories `GETARG`, date/time, paths, `CloseOutput`, signals, and `iMetHour`, with an anchor and patched/deferred disposition for each. It cites the local assignments proving the exact `iMetHour` replacement. Run `tmp_dir=$(mktemp -d); cp -R src_reference "$tmp_dir/src"; git apply --check --directory="$tmp_dir/src" reference/fortran_reference/0001-gfortran-portability.patch; rm -rf "$tmp_dir"`; it must pass. PATCH_NOTES contains `Scope`, `Source identity`, `Changes`, `Behavior preserved`, `Open limitations`; no original file appears in the diff.

### 2.2.1 — Build the deterministic reference harness

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 2.1.1.
- **Must edit:** none.
- **Can create:** `tools/stage_fortran_reference.py`; `tools/build_fortran_reference.sh`; `reference/fortran_reference/README.md`; `planning/tasks/2.2.1.todo.md`.
- **Scratch only:** `.development/2.2.1/` and `.development/reference-build/`.
- **Read:** task 2.1.1 report, patch and notes; `master/SOURCE_AUDIT.md`; `master/VALIDATION_STRATEGY.md`.
- **Reference implementation:** task 2.1.1 patch and `master/SOURCE_MANIFEST.json` are the provenance pattern.
- **Anchors:** `rg -n 'MSFLIB|TIME.FOR:551|gfortran|patch hash|compiler' reference/fortran_reference/{0001-gfortran-portability.patch,PATCH_NOTES.md}`.

Create a Python staging command that verifies immutable input hashes, copies source into the configured untracked directory, applies the committed patch, and records staging metadata. Create a shell build command that invokes GNU Fortran with explicit flags, writes the executable and build manifest under the staging directory, and fails clearly when a prerequisite is absent. README must document commands, generated paths, and the no-scientific-validation boundary.

**Acceptance:** run `python3 tools/stage_fortran_reference.py --output .development/reference-build` then `sh tools/build_fortran_reference.sh --build-dir .development/reference-build`; both must pass. The manifest records source/archive hashes and their byte-match status, patch hash, GNU Fortran version, host OS/architecture, UTC date, command, flags, executable hash, and artifact-notice review status; `src_reference/` remains unchanged.

### 2.3.1 — Add and run the noncanonical smoke case

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 2.2.1.
- **Must edit:** none.
- **Can create:** `reference/fortran_reference/smoke/LEVEL_01.DIR`; `reference/fortran_reference/smoke/SELECTOR.IN`; `reference/fortran_reference/smoke/PROFILE.DAT`; `reference/fortran_reference/smoke/ATMOSPH.IN`; `reference/fortran_reference/smoke/README.md`; `tools/run_fortran_reference.py`; `reference/fortran_reference/SMOKE_RUN.md`; `planning/tasks/2.3.1.todo.md`.
- **Scratch only:** `.development/2.3.1/` and `.development/reference-build/`.
- **Read:** task 2.2.1 report; `INPUT.FOR:BasInf/NodInf/TmIn`; `HYDRUS.FOR:99-140`; `master/VALIDATION_STRATEGY.md`.
- **Anchors:** `rg -n 'LEVEL_01.DIR|Selector|Profile|Atmosph|open\(|read\(' src_reference/{HYDRUS,INPUT}.FOR`.
- **Reference implementation:** `INPUT.FOR` and `HYDRUS.FOR` define the input layout; `master/VALIDATION_STRATEGY.md` defines the isolated-run record.

Derive the smallest input layout needed to reach normal program completion. Commit only the input files and a README declaring them noncanonical. The run command must execute in an isolated output directory, capture stdout/stderr, verify exit status and completion markers, hash generated outputs, and write a committed run record containing exact toolchain/patch/input hashes and command. Generated outputs remain untracked.

**Acceptance:** `python3 tools/run_fortran_reference.py --smoke --timeout-seconds 60` exits zero. It must copy committed inputs immutably, start from a clean output directory, enforce the timeout, inventory files, capture warnings/stdout/stderr, reject malformed or truncated records, and hash outputs. SMOKE_RUN includes `Input status`, `Command`, `Hashes`, `Environment`, `Exit status`, `Completion evidence`, `Output inventory`, `Warnings`, and `Limitations`; no smoke artifact is described as a fixture, parity result, or baseline.

### 2.4.1 — Review Phase 0 evidence and hand off

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Depends on:** 2.3.1.
- **Must edit:** none.
- **Can create:** `planning/master/PHASE_0_REFERENCE_EVIDENCE.md`; `planning/tasks/2.4.1.todo.md`.
- **Scratch only:** `.development/2.4.1/`.
- **Read:** all Phase 0 committed artifacts and `HYDRUS1D_RUST_PORT_PLAN.md` Phase 0/1 rows.
- **Anchors:** `rg -n 'Source identity|Changes|gfortran|Input status|Exit status|Limitations' reference/fortran_reference/{PATCH_NOTES.md,SMOKE_RUN.md} reference/fortran_reference/README.md`.
- **Reference implementation:** `master/VALIDATION_STRATEGY.md` provenance and acceptance-gate sections.

Review the patch, harness, smoke record, input labels, hashes, and generated-artifact exclusions against this plan. Create an evidence handoff that identifies the exact completed Phase 0 exit evidence, remaining limitations, and Phase 1 inputs. It must say that the smoke case is not a scientific fixture and that Phase 1 is still unplanned.

**Acceptance:** evidence document contains `Reference executable`, `Patch and build`, `Smoke run`, `Exit checklist`, `Limitations`, `Phase 1 handoff`; its exit checklist links source/archive/patch/executable/input/output/environment evidence and the reviewed run policy. Run `python3 -c 'from pathlib import Path; p=Path("planning/master/PHASE_0_REFERENCE_EVIDENCE.md"); s=p.read_text(); required=["## Reference executable","## Patch and build","## Smoke run","## Exit checklist","## Limitations","## Phase 1 handoff"]; assert all(x in s for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print("Phase 0 evidence structure clean")'`; it must pass.

## Execution policy

Workers produce an at-most-12-step micro-plan and stop. The orchestrator checks file scope, acceptance mapping, source claims, and task dependencies before sending the literal approval message. The orchestrator creates one conventional, no-phase/wave commit per task. A micro-plan off-spec twice is replaced with an orchestrator-written fixed TODO. A failed targeted check gets one feedback retry; the same failure escalates to gpt-5.6-terra. A malformed report gets one correction request then escalation. An out-of-scope need requires plan rescoping, never informal permission. Full document/link checks run serially at the end.

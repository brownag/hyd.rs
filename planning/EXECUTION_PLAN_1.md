# Orchestration 1 — Create the HYDRUS Rust master plan

Status: **Approved by the user on 2026-09-19.** Execution has not started.
Independent draft review and finding dispositions: [PLAN_REVIEW_1.md](PLAN_REVIEW_1.md).
Target repository: `/home/andrew/AI/hyd.rs`. Run all commands below from that root.
Contract: [wave-orchestration](/home/andrew/.codex/skills/wave-orchestration/SKILL.md).
Inputs: [INITIAL_PROMPT.md](INITIAL_PROMPT.md), [HYDRUS_RESEARCH.md](HYDRUS_RESEARCH.md).
The dated user decisions in the prompt and this plan supersede research recommendations.

## Goal and scope

Create the exhaustive technical blueprint, design specification and phased execution roadmap for a Rust port of a defined scientific subset of the supplied HYDRUS-1D **4.08 or derivative** snapshot. The deliverable of executing this orchestration is `planning/HYDRUS1D_RUST_PORT_PLAN.md`, supported by source-audit and design documents. It will seed multiple future implementation wave plans. This orchestration does **not** implement the port.

In scope: evidence-based source/call/state/feature audit; architecture and Markdown Rust signatures; numerical, ownership, memory, I/O, serialization and host contracts; oracle/fixture/parity/property/performance **designs**; risk register; requirement traceability; phased checklists and later wave-plan boundaries; independent reviews and revision. Read-only source inspection, primary-source research and scratch PDF text extraction are permitted.

Out of scope for this orchestration: Rust/Fortran implementation changes, Cargo workspace creation, functioning oracle patches/builds, fixture construction, numerical experiments, CI/package/binding implementation, benchmarks, GUI, distribution/publication and later numbered implementation plans. Their designs and prerequisite tasks belong in the master plan. Git baseline/worktrees, task TODOs, review reports and document acceptance commands are execution administration, not feature development.

Future implementation ordering is confirmed only at this level: first accepted single-domain water with atmospheric conditions and root uptake; solute/heat later; hysteresis later; dual porosity required after simpler physics. Exact optional feature, format and host/platform release scope remains a documented recommendation or an explicit unresolved user gate. The supplied source rejects dual permeability; record that fact and do not substitute it for dual porosity.

### Verbatim user constraints

> “1 and 3; update the prompt to reflect that it should be 4.08 (or a derivative thereof);”

> “defer to later”

> “confirm that this initial wave plan will just be creating a master plan (from which we may create multiple wave plans); dual-porosity is important but we need to get everything simpler done first.”

> “sounds good, use very cheap models if possible”

From the supplied prompt:

> “Do not write the full implementation code yet; your deliverable is the exhaustive technical blueprint, design specification, and phased execution roadmap.”

> “zero allocations inside the inner time/Picard iteration loops”

The latter is a design/validation requirement to specify; no allocation measurements are claimed here.

## Decision log

| ID | Date | Decision and rationale |
| --- | --- | --- |
| D01 | 2026-09-19 | User chose supplied snapshot + defined scientific subset. Working lineage is 4.08 or derivative; hashes define the oracle identity, not an unverified release label. |
| D02 | 2026-09-19 | User accepted first water/atmosphere/root milestone, followed by solute and heat. Establish the simpler physical foundation first. |
| D03 | 2026-09-19 | User deferred hysteresis to a later milestone, retaining it as planned work. |
| D04 | 2026-09-19 | User clarified this orchestration creates only the master plan, which may seed several later implementation wave plans. |
| D05 | 2026-09-19 | User emphasized dual porosity is important and belongs after simpler physics. It remains required. |
| D06 | 2026-09-19 | User delegated technical research and documented recommendations to master-plan workers; consequential unresolved choices return for user review. |
| D07 | 2026-09-19 | User accepted up to three concurrent workers with separate document ownership and requested very cheap models where possible. |

Execution defaults proposed for approval with this plan: isolated worktrees; a local baseline commit on `docs/master-plan` followed by one commit per completed task; no remote operations; document/evidence review acceptance rather than numerical-test acceptance. No monetary or token ceiling was supplied, so none is invented.

## Reconnaissance and baseline

- Ten original Fortran sources, 11,053 lines; no Rust source, Cargo manifest, executable tests, input decks or golden outputs.
- Sizes: HYDRUS 999; INPUT 1,723; MATERIAL 946; WATFLOW 1,127; HYSTER 756; SINK 366; TIME 1,594; SOLUTE 2,105; TEMPER 323; OUTPUT 1,114 lines. All are under `src_reference/` with `.FOR` suffixes.
- Reference implementations are legacy routines named per task; there are no existing Rust patterns to copy.
- No AGENTS.md, CONTRIBUTING, earlier numbered plan, TODO/state set or test/CI convention was found in the project/checked ancestors. Recheck at execution start.
- Python 3 and pdftotext are available for document checks and scoped manual extraction.
- Archive SHA-256: `d0c47cbcb7a298ab16c6e08010bd7ca048d5527ce2f989a749390c0c55e848cf`. Rechecked: all ten extracted sources match archive bytes.
- Syntax command `gfortran -fsyntax-only -std=legacy -ffixed-line-length-none src_reference/*.FOR` failed with missing MSFLIB and TIME.FOR:551 integer/logical IF. This is an observed failed syntax check, not a simulation result.
- Git was absent during the earlier research. It is now initialized on an unborn `main`, with `planning/` and `src_reference/` untracked and no commits. Do not repeat the stale claim that Git is absent.
- No scientific green baseline exists. This does not block document work or drafting later wave plans. Future harness/bootstrap tasks may establish the minimally patched oracle and provenance-backed fixtures; numerical port tasks that depend on that oracle, and scientific parity acceptance, must wait for its verified baseline. This avoids making oracle construction depend on an oracle that does not yet exist.
- No e2e servers or shared ports exist. Workers must not create servers or run global suites; future harness runs must use isolated output directories. Orchestrator runs document-wide checks serially.
- Major hazards: oversized legacy files, mixed precision, source/manual mismatch, state outside COMMON, finite-difference/FE interpretation, boundary rows/pivot policy, retry histories, unproven fixture provenance, and optional physics coupling.
- Shared-write hazards: master blueprint (tasks 1.4.1 then 1.6.1), STATE, execution plan, input documents and Git administration. These are serialized as specified below.

## Requirement coverage

This is a coverage index, not permission to omit subrequirements. Task 1.3.1 expands it to every atomic prompt bullet, with source section, disposition and destination. Tasks 1.4.1/1.6.1 carry the complete matrix into the master.

| ID | Requirement | Owner tasks |
| --- | --- | --- |
| R01 | 4.08 or derivative; pinned snapshot plus scientific subset; provenance and immutable originals | 1.1.1, 1.1.3 |
| R02 | All source files/routines, call hierarchy, lifecycle, platform/file-unit dependencies | 1.1.1, 1.1.2 |
| R03 | COMMON, globals/main arrays, DATA/static assumptions, domains, precision, bounds/index/sign/unit hazards | 1.1.2 |
| R04 | SimulationConfig, Mesh1D, HydraulicModel, BoundaryConditions, SimulationState, MassBalanceTracker; Send+Sync and filesystem-free core | 1.2.1, 1.2.2 |
| R05 | Hydraulic families, derivatives, tables, scaling and parameter/branch domains | 1.1.3, 1.2.1, 1.2.3 |
| R06 | Spatial discretization/FE interpretation, matrix structure, Thomas versus crates, pivot policy | 1.2.1 |
| R07 | Picard/adaptive events, rollback, boundary switching, strict errors, coupled histories | 1.1.2, 1.2.1, 1.2.2 |
| R08 | Workspace reuse; zero allocations in accepted/retried inner loops with measurable boundaries | 1.2.1, 1.2.3 |
| R09 | SELECTOR.IN, ATMOSPH.IN, PROFILE.DAT; NOD_INF.OUT, T_LEVEL.OUT, BALANCE.OUT; text versus numeric parity | 1.2.2, 1.2.3 |
| R10 | serde/versioned schema; JSON/YAML/TOML/GeoPackage/Arrow; in-memory dynamic forcing | 1.2.2 |
| R11 | Python/R/JavaScript/other hosts, browser versus Node/C ABI, platforms, lifetime/cancellation/concurrency | 1.2.2, 1.2.3 |
| R12 | Five scenarios: dry infiltration, capillary rise, cyclic atmosphere/root, nonequilibrium solute/degradation, temperature-coupled heat | 1.1.3, 1.2.3 |
| R13 | L-infinity/L2 norms, per-field tolerances, mixed precision/f64, optimization/order drift, legacy mass balance | 1.2.1, 1.2.3 |
| R14 | Curve derivative/unit/property tests; Feddes/S-shape; conservation and independent analytical/refinement checks | 1.2.3 |
| R15 | First acceptance single-domain water/atmosphere/root; later solute/heat and hysteresis; required dual porosity after simpler physics | 1.1.3, 1.3.1 |
| R16 | Drainage, vapor/energy, density, snow/meteo, colloids/bacteria, inverse/special chemistry: explicit evidence/status and proposed scope decisions | 1.1.3, 1.3.1 |
| R17 | Phase 0–6 mapping, exact phased checklists/DoD, early minimal harness/I/O/basic BC, CLI ergonomics and Criterion/Fortran benchmark design | 1.2.2, 1.2.3, 1.3.1 |
| R18 | Master Markdown with ASCII diagrams, Cargo/module layout, Rust signatures, detailed risk registry and phase checklist | 1.4.1, 1.6.1 |
| R19 | Planning-only orchestration; master seeds multiple future wave plans; no implementation, baseline build or claimed experimental results | all |
| R20 | Documented technical recommendations; unresolved consequential decisions for user; maximum three cheap workers with bounded escalation | 1.3.1, 1.4.1, execution policy |

## Wave table and dependencies

Every dependency must be done and integrated before the next task starts. Transitive dependencies supply all prior audit/design artifacts.

| Wave | Theme | Tasks | Dependencies | Max workers |
| --- | --- | --- | --- | ---: |
| 1 | Source evidence | 1.1.1 inventory/provenance; 1.1.2 calls/state; 1.1.3 physics matrix | Preflight | 3 |
| 2 | Design specifications | 1.2.1 numerics; 1.2.2 API/I/O/hosts; 1.2.3 validation | All wave 1 tasks | 3 |
| 3 | Implementation roadmap | 1.3.1 roadmap/risks/traceability | All wave 2 tasks | 1 |
| 4 | Master synthesis | 1.4.1 assemble blueprint | 1.3.1 | 1 |
| 5 | Independent master reviews | 1.5.1 spec; 1.5.2 standards | 1.4.1 | 2 |
| 6 | Review resolution | 1.6.1 revise master and disposition findings | Both wave 5 tasks | 1 |

Pairwise write scopes are disjoint within every wave, including per-task TODO and scratch files. Read overlap is intentional. No worker appends to shared STATE: workers return reports, and the orchestrator appends them serially. The template explicitly overrides the shared-append example to prevent concurrent changes.

## Execution policy and preflight

1. After user approval, inspect Git status, current files, conventions and tool availability. Verify only the expected supplied inputs and reviewed planning artifacts are present; never blanket-stage unrelated files.
2. Establish the local baseline and `docs/master-plan` branch before worktrees. The existing repository has no commit: stage only the explicitly inventoried source, archive/PDF, input and approved planning files; create one conventional baseline commit. Known input/planning changes are intended for this baseline. If unexpected user changes appear, preserve them and obtain direction only if isolation cannot proceed safely.
3. Keep `src_reference/`, original archive/PDFs, INITIAL_PROMPT and HYDRUS_RESEARCH immutable during worker execution. Record hashes and failed syntax result in STATE. Do not repair the oracle in this orchestration.
4. For every task create an isolated worktree from the latest integrated approved state on a dedicated branch. Use task branches `docs/master-plan-<id>`. Worktree-local scratch is `.development/<id>/`, excluded through Git local excludes; do not commit scratch or add a repository-wide .gitignore as incidental work.
5. Start workers with **gpt-5.6-luna**, low effort, using explicit scoped context rather than full-history forks. The user authorized cheap models. If unavailable, select the cheapest available capable worker and record the substitution. No price claims are made. At most three workers in total, including any replacement/escalated workers, plus one orchestrator; stop the replaced worker before escalation. No nested worker delegation.
6. Each worker produces a micro-TODO of at most 12 verifiable steps, including targeted document checks, and ends its turn. Before approval only reading/research and its named TODO/scratch files are permitted; no deliverable edits.
7. Orchestrator verifies scope, evidence anchors, completeness and step count. Send exactly `APPROVED — execute the plan as written.` plus answers; tell the user which task was approved.
8. Workers run only their targeted checks and return reports of at most 15 lines. Orchestrator reruns checks, verifies substantive claims/diffs, stages explicit files, integrates one task at a time and commits each with a short conventional sentence. No phase/wave numbers in messages; no pushes.
9. Run document-wide checks serially at wave end. Update STATE and compact if needed. Later work must read dependency reports; independent review workers must not read one another's reports before first submission.
10. When master revisions are complete, present remaining decisions and the master to the user. Record technical completion separately from user approval; never start a later implementation orchestration automatically.

### Escalation and cost limits

- Micro-plan off-spec twice: orchestrator writes the fixed micro-TODO, then returns it to the cheap worker.
- Targeted acceptance fails: supply `FAILED VERIFICATION: ...` and permit one retry. Same failure after retry: stop the cheap worker and rerun the scoped task with **gpt-5.6-terra** at medium effort. No third cheap-model attempt.
- Missing/malformed completion report: give explicit feedback and one retry, then escalate the scoped task to the same mid-tier model.
- If the mid-tier worker cannot meet acceptance after one feedback/retry cycle, stop and have the orchestrator diagnose/rescope or ask the user about a consequential decision. Do not launch an unbounded expensive loop.
- Out-of-scope work or a micro-plan requiring more than 12 steps is a decomposition issue. Amend this plan and reassess dependencies/file overlap before execution; do not widen worker permissions informally. For broad audits (1.1.2/1.1.3), API work (1.2.2), roadmap (1.3.1) and synthesis (1.4.1), the orchestrator must also check that the evidence inventory and context fit the worker. If not, split into bounded document tasks before approval; never compress away branches or accept superficial coverage merely to stay within 12 steps.
- Default output is concise tables and decision contracts, usually 100–200 lines per supporting report; inventories may be longer. Master synthesis may be longer to meet the exhaustive blueprint requirement. These are cost guidance, not caps that justify missing requirements.
- Reuse local evidence and cited primary docs. Research a current dependency claim only when needed; record URL, access date and limitations. Do not turn inaccessible sources into inferred facts. Prefer source inspection to speculative prose.

## Acceptance for this documentation orchestration

Each task has a concrete Python 3 command below, executable once its document exists. These check nonempty required sections/whitespace and specific artifacts where stated. They are necessary but insufficient: the orchestrator also checks source claims, coverage, design consistency and conclusions against the task's substantive criteria. Do not equate a heading check with scientific correctness.

For each wave, run all completed-task commands serially, `git diff --check` for tracked edits, and inspect the complete file list including untracked outputs. Check local Markdown file links from each document's own directory; proposed paths in code blocks are examples and must be labeled future files, not broken live links. Every numerical command in the master must be either reproducibly observed or labeled proposed with its creation prerequisite.

For master synthesis and revision also require each R01–R20 ID, all five output requirements, all five scenario plans and the confirmed milestone ordering. Review reports are outputs of wave 5, not evidence that the master is approved in advance.

Run this document-wide link/whitespace check serially from the repository root after each wave. It checks existing orchestration outputs only, skips fenced examples and external URLs, and verifies local file targets (heading-anchor accuracy still requires review):

```sh
python3 - <<'PY'
from pathlib import Path
import re
from urllib.parse import unquote
paths = [Path('planning') / n for n in ('EXECUTION_PLAN_1.md', 'PROMPT_TEMPLATE_1.md', 'STATE_1.md', 'HYDRUS1D_RUST_PORT_PLAN.md')]
paths += sorted(Path('planning/master').glob('*.md'))
checked = 0
for p in paths:
    if not p.exists():
        continue
    text = p.read_text()
    assert not any(line.rstrip() != line for line in text.splitlines()), p
    prose = re.sub(r'```[^\n]*\n.*?```', '', text, flags=re.S)
    for target in re.findall(r'\[[^\]]+\]\(([^)]+)\)', prose):
        target = target.strip().strip('<>')
        if re.match(r'[A-Za-z][A-Za-z0-9+.-]*:', target) or target.startswith('#'):
            continue
        target = re.sub(r':\d+$', '', unquote(target.split('#', 1)[0]))
        if target:
            assert (p.parent / target).exists(), (p, target)
    checked += 1
print('Local file links and whitespace checked:', checked, 'documents')
PY
```

## Task specifications

Common scope rule: every path is relative to the repository root. Read access covers the supplied evidence and declared dependencies; write access is only the files below, the named task TODO, and task-local scratch. Future source file paths may occur in Markdown examples only. Do not touch other tasks' documents, shared STATE/plans, source/assets, Git settings or manifests. Git administration/integration belongs to the orchestrator.

### 1.1.1 — Inventory source and provenance

- **ID:** 1.1.1
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** preflight only.
- **Must edit:** none; all deliverables are new.
- **Can create:** `planning/master/SOURCE_AUDIT.md`, `planning/master/SOURCE_MANIFEST.json`; `planning/tasks/1.1.1.todo.md`.
- **Scratch only:** `.development/1.1.1/` (untracked).
- **Reference implementations/evidence:** HYDRUS_RESEARCH.md §6, snapshot identity and source inventory; HYDRUS.FOR:20 (program), header lines 1–18; archive entries and local PDF title pages.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Inventory all ten source files, every declared routine, sizes, archive/file hashes, manuals and external dependencies. Identify the 4.08-or-derivative working lineage without inventing a release identity. Compare archive entries byte-for-byte. Record artifact-specific notices and provenance gaps without assigning a license. Distinguish intrinsic/library calls from project routines and identify unresolved external symbols. Describe the current build blockers and a minimal future oracle-portability patch plan; do not patch or build a working oracle. Record all observed file-unit and platform dependencies by source. SOURCE_MANIFEST.json must be an object mapping all ten src_reference/*.FOR paths, planning/H1D_Src.zip, and the three planning/reference/*.pdf paths to SHA-256 strings.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n -i 'program |subroutine |function |copyright|version|use MSFLIB' src_reference/*.FOR
```

**Acceptance:** The manifest reproduces every observed asset hash; all ten files and their routine declarations are inventoried. Version uncertainty, MSFLIB and integer/logical blockers are explicit. Audit appendix tables may exceed the normal report size. Required sections: `Inventory`, `Routine inventory`, `Provenance`, `Build blockers`, `Evidence`, `Open decisions`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/master/SOURCE_AUDIT.md"); s=p.read_text(); required=["## Inventory","## Routine inventory","## Provenance","## Build blockers","## Evidence","## Open decisions"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'
python3 -c 'import hashlib,json; from pathlib import Path; m=json.loads(Path("planning/master/SOURCE_MANIFEST.json").read_text()); expected=set(map(str,Path("src_reference").glob("*.FOR")))|set(map(str,Path("planning/reference").glob("*.pdf")))|{"planning/H1D_Src.zip"}; assert set(m)==expected; assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in m.items()); print("Asset hashes verified")'
```



### 1.1.2 — Trace lifecycle, calls, and owned state

- **ID:** 1.1.2
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** preflight only.
- **Must edit:** none; all deliverables are new.
- **Can create:** `planning/master/STATE_AND_CALLS.md`; `planning/tasks/1.1.2.todo.md`.
- **Scratch only:** `.development/1.1.2/` (untracked).
- **Reference implementations/evidence:** HYDRUS_RESEARCH.md §6 lifecycle/state tables; HYDRUS.FOR:49–97 declarations, 123 initialization, 399 time loop, 784/853/936 teardown; WATFLOW.FOR:3,173,308,359,1079; HYSTER.FOR:6,124,652.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Expand the partial lifecycle into a source-anchored call hierarchy covering initialization, lookup tables, accepted/trial steps, optional water/heat/solute ordering, output and all termination paths. Catalog project CALL edges and function references with unresolved dynamic/external edges labeled, not silently omitted. Map mutable main-program arrays, COMMON/DATA/static assumptions, precision conversions, limits, units, coordinates, signs, and file units into proposed ownership domains. Trace rollback and boundary/history/accounting mutations across the call chain. Use declarative inventories and scoped routine reads; derive design obligations rather than implementing a solver. Record exact evidence and any unresolved edges. The feature audit in 1.1.3 owns scientific branch coverage; this task owns call/state relationships.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n -i 'call |common |data |save|^ *12 |^ *13 |subroutine (WatFlow|Update|Hyst|Close)' src_reference/*.FOR
```

**Acceptance:** All ten source files occur in the call/state coverage table; all three COMMON blocks, main arrays and DATA initialization are addressed. Every unresolved edge has an investigation owner. Calls and ownership claims receive source-symbol references, and rollback covers coupled histories and accounting. Required sections: `Lifecycle`, `Call hierarchy`, `State inventory`, `Precision and limits`, `Rollback`, `Evidence`, `Open decisions`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/master/STATE_AND_CALLS.md"); s=p.read_text(); required=["## Lifecycle","## Call hierarchy","## State inventory","## Precision and limits","## Rollback","## Evidence","## Open decisions"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'

```



### 1.1.3 — Map physics and feature coverage

- **ID:** 1.1.3
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** preflight only.
- **Must edit:** none; all deliverables are new.
- **Can create:** `planning/master/FEATURE_MATRIX.md`; `planning/tasks/1.1.3.todo.md`.
- **Scratch only:** `.development/1.1.3/` (untracked).
- **Reference implementations/evidence:** HYDRUS_RESEARCH.md §§2–4 and §6; MATERIAL.FOR FK/FC/FQ/FH; INPUT.FOR MatIn; WATFLOW.FOR Reset/Shift/DualPor; SOLUTE.FOR Coeff/NEquil/MassTran; TEMPER.FOR Temper/TempAdj; SINK.FOR SetSnk/FAlfa/FSAlfa; TIME.FOR Snow/Meteo/Evapor.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Create a capability matrix keyed by source selectors and manual sections: hydraulic families and lookup/scaling, mesh/inclined flow, all upper/lower boundaries and drainage, root stress/compensation/growth/salinity, hysteresis, solute species/reaction/sorption/transfer modes, heat/temperature dependence, dual porosity, vapor/energy, density, meteorology/snow/interception, colloids/bacteria, inverse methods and special chemistry. Distinguish implemented, rejected, documented-only and uncertain modes. Give each row a recommended milestone, prerequisite, fixture need and explicit inclusion/defer/unsupported status. Keep first acceptance single-domain water plus atmospheric/root behavior; solute/heat and hysteresis later; dual porosity required after simpler physics. Dual permeability is rejected by this snapshot, not another name for dual porosity. Unagreed optional scope remains a recommendation requiring review. Reconcile manual FE terminology with Reset's finite-difference stencil as an audit finding, not a license to substitute numerics.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n -i 'iModel|iDualPor|lVapor|lChem|lTemp|lHyst|subroutine|function' src_reference/{MATERIAL,INPUT,WATFLOW,HYSTER,SINK,SOLUTE,TEMPER,TIME}.FOR
```

**Acceptance:** Every named physics branch from both seed documents is represented; unsupported features cannot silently fall back. Confirmed milestone decisions are separate from recommendations. Dual porosity is required later; dual permeability is distinguished. Each supported row has a source symbol and candidate validation case. Every optional row must explicitly distinguish observed implementation status from recommended inclusion, proposed deferral or unsupported status; inventory alone is not a release commitment. Required sections: `Capability matrix`, `First acceptance`, `Later required physics`, `Optional scope recommendations`, `Source and manual discrepancies`, `Evidence`, `Open decisions`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/master/FEATURE_MATRIX.md"); s=p.read_text(); required=["## Capability matrix","## First acceptance","## Later required physics","## Optional scope recommendations","## Source and manual discrepancies","## Evidence","## Open decisions"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'

```



### 1.2.1 — Design numerical core and memory contracts

- **ID:** 1.2.1
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** `1.1.1`, `1.1.2`, `1.1.3`.
- **Must edit:** none; all deliverables are new.
- **Can create:** `planning/master/NUMERICAL_DESIGN.md`; `planning/tasks/1.2.1.todo.md`.
- **Scratch only:** `.development/1.2.1/` (untracked).
- **Reference implementations/evidence:** STATE_AND_CALLS.md and FEATURE_MATRIX.md; WATFLOW.FOR Reset/Gauss/Shift/SetMat/Veloc/Update; INPUT.FOR GenMat; SOLUTE.FOR BanSol; TIME.FOR TmCont; OUTPUT.FOR SubReg; local 4.08 manual Chapters 2,7,8.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Specify spatial metrics and the source stencil algebra, hydraulic functions/derivatives/tables, boundary-aware matrix assembly, elimination/pivot policies, Picard convergence, event alignment/adaptive retries, balances and coupled-process order. Compare a small reusable tridiagonal kernel with faer/ndarray only against actual requirements. Recommend precision and failure behavior with evidence, distinguishing mixed-precision oracle from any f64 modernization. Design preallocated workspace and accepted/trial history ownership including future hysteresis and dual porosity. Define exactly where zero-allocation guarantees apply, including retry, fixed dimensions, output and error policies. Provide pseudocode and prototyped numerical interfaces in Markdown, plus alternatives and unresolved decisions. Do not run implementation experiments; specify future experiments when evidence cannot settle a choice.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n -i 'subroutine (Reset|Gauss|Shift|SetMat|GenMat|BanSol|TmCont|SubReg)|sngl|Finite differences' src_reference/*.FOR
```

**Acceptance:** The source stencil and pivot policy are reconciled rather than assumed; rollback and allocation contracts include later physics. Choices are labeled as evidence-based recommendations or unresolved decisions. API ownership is an input to synthesis, not edits to another worker's document. Required sections: `Discretization`, `Hydraulics and tables`, `Linear solver`, `Time and nonlinear solve`, `Balances and coupling`, `Memory and precision`, `Interfaces`, `Alternatives`, `Evidence`, `Open decisions`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/master/NUMERICAL_DESIGN.md"); s=p.read_text(); required=["## Discretization","## Hydraulics and tables","## Linear solver","## Time and nonlinear solve","## Balances and coupling","## Memory and precision","## Interfaces","## Alternatives","## Evidence","## Open decisions"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'

```



### 1.2.2 — Design API, legacy I/O, and host boundaries

- **ID:** 1.2.2
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** `1.1.1`, `1.1.2`, `1.1.3`.
- **Must edit:** none; all deliverables are new.
- **Can create:** `planning/master/API_AND_IO.md`; `planning/tasks/1.2.2.todo.md`.
- **Scratch only:** `.development/1.2.2/` (untracked).
- **Reference implementations/evidence:** STATE_AND_CALLS.md; HYDRUS_RESEARCH.md §7; INPUT.FOR BasInf/NodInf/TmIn/ChemIn/TempIn; OUTPUT.FOR TLInf/NodOut/SubReg; official primary documentation for candidate adapters.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Propose crate/module layout and ASCII architecture; prototype SimulationConfig, Mesh1D, HydraulicModel, BoundaryConditions, SimulationState, MassBalanceTracker and owned simulation/workspace/error types. Define Send+Sync versus single mutable owner, independent ensembles, units/sign/index conversion, validated configuration, step/advance_to semantics, forcing interpolation/events, diagnostics, cancellation and snapshot/checkpoint recommendations. Separate filesystem-free core from legacy parsers/writers and CLI; define legacy layout versus numeric comparison. Compare Python, R, browser Wasm, native Node and C ABI boundaries, ownership/copying/error/thread rules, packaging and OS/architecture recommendations. Explicitly disposition JSON/YAML/TOML/GeoPackage/Arrow with a versioned schema; do not describe GeoPackage/Arrow as automatic serde formats. Research current primary docs where needed; proposed host/platform/format priorities are not user-approved release commitments. Conflicts with numerical-design recommendations are reconciled later.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n -i 'subroutine (BasInf|NodInf|TmIn|ChemIn|TempIn|TLInf|NodOut|SubReg)|read\(|write\(' src_reference/{INPUT,OUTPUT}.FOR
```

**Acceptance:** All named structs, input/output families, formats and hosts have a contract or explicit proposed deferral. Core has no host/filesystem dependencies; foreign ownership and cancellation are addressed. Include Markdown Rust signatures only, no Cargo or implementation files. Required sections: `Architecture`, `Crates and modules`, `Core signatures`, `Lifecycle and forcing`, `Legacy I/O and CLI`, `Serialization`, `Host contracts`, `Platform recommendations`, `Alternatives`, `Evidence`, `Open decisions`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/master/API_AND_IO.md"); s=p.read_text(); required=["## Architecture","## Crates and modules","## Core signatures","## Lifecycle and forcing","## Legacy I/O and CLI","## Serialization","## Host contracts","## Platform recommendations","## Alternatives","## Evidence","## Open decisions"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'

```



### 1.2.3 — Specify validation and performance evidence

- **ID:** 1.2.3
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** `1.1.1`, `1.1.2`, `1.1.3`.
- **Must edit:** none; all deliverables are new.
- **Can create:** `planning/master/VALIDATION_STRATEGY.md`; `planning/tasks/1.2.3.todo.md`.
- **Scratch only:** `.development/1.2.3/` (untracked).
- **Reference implementations/evidence:** SOURCE_AUDIT.md, STATE_AND_CALLS.md, FEATURE_MATRIX.md; HYDRUS_RESEARCH.md §§4,8; OUTPUT.FOR SubReg/NodOut/TLInf; source hydraulic/linear routines; local 4.08 manual Chapter 11.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Design the future oracle build/patch/run harness, provenance records and isolated fixture directories/timeouts/completion checks. Specify all five requested scenarios, plus equilibrium, layered mesh, event/switch/retry, conservation/refinement, later hysteresis/dual-porosity and host-equivalence tests. Record missing input decks honestly and define acquisition/construction and expected-result provenance. Define per-variable units, paired absolute/relative tolerance and L-infinity/L2/RMS normalization, mass-balance zero denominator, event/iteration diagnostics, compiler/precision comparisons and text-layout checks separate from scientific parity. 1e-6 remains a candidate requiring calibration; no invented passing results. Specify derivative and proptest domains, interleaved/threaded state tests, allocation counting, and Criterion/Fortran performance experiments with comparable numerical work. Name proposed future test files/commands and creation prerequisites; distinguish them from checks executable now. Define gates for accepting recommendations, future implementation waves and manual/platform verification.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n -i 'subroutine (SubReg|NodOut|TLInf|Gauss|BanSol)|real function (FK|FC|FQ|FAlfa)' src_reference/*.FOR
```

**Acceptance:** All five scenarios have input provenance status and observables. No golden outputs, universal tolerances or speedups are claimed. Future commands have producer/prerequisite milestones; document validation is not numerical validation. Required sections: `Oracle harness`, `Fixture matrix`, `Component and property tests`, `Tolerance policy`, `Host and platform tests`, `Allocation and performance`, `Future commands and prerequisites`, `Acceptance gates`, `Evidence`, `Open decisions`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/master/VALIDATION_STRATEGY.md"); s=p.read_text(); required=["## Oracle harness","## Fixture matrix","## Component and property tests","## Tolerance policy","## Host and platform tests","## Allocation and performance","## Future commands and prerequisites","## Acceptance gates","## Evidence","## Open decisions"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'

```



### 1.3.1 — Build roadmap, risks, and requirement traceability

- **ID:** 1.3.1
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** `1.2.1`, `1.2.2`, `1.2.3`.
- **Must edit:** none; all deliverables are new.
- **Can create:** `planning/master/ROADMAP_AND_RISKS.md`; `planning/tasks/1.3.1.todo.md`.
- **Scratch only:** `.development/1.3.1/` (untracked).
- **Reference implementations/evidence:** All six master audit/design documents; INITIAL_PROMPT.md full text; EXECUTION_PLAN_1.md requirement map and decisions.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Create the future implementation milestone DAG with detailed phase checklists, entry/exit evidence, dependency reasons, risk owners and candidate boundaries for multiple future wave plans. Preserve original phases 0–6 through explicit mapping while moving minimal I/O/basic boundaries early enough for water validation. Define the first water acceptance contract explicitly: minimum hydraulic models, boundary variants, root stress/compensation behavior, validation fixtures, and exclusions. Recommend any unchosen variants and expose consequential scope decisions for user review. Solute, heat, hysteresis and dual porosity do not enter that first acceptance gate by implication. Sequence first accepted water/atmosphere/root milestone before solute/heat and hysteresis; place required dual porosity after simpler validated physics and explain exact dependencies. Resolve design-document conflicts through reasoned recommendations without editing those inputs. Include a risk registry covering every research risk and a requirement matrix R01–R20 plus every atomic numbered/bulleted requirement in INITIAL_PROMPT.md. Each requirement must point to a document section, future milestone or explicit proposed deferral with reason. List unresolved consequential decisions with recommendation, alternatives, owner and latest decision gate. Keep feature, binding and format exclusions provisional unless the user decided them. Do not create future EXECUTION_PLAN_N files or runnable port tasks.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n '^##|^###|Open decisions|recommend|defer|prerequisite' planning/master/*.md
```

**Acceptance:** Every R01–R20 ID and every prompt requirement is mapped; no required later physics is dropped. Each future milestone has verifiable DoD and prerequisites. Risk mitigations and unresolved decisions identify a milestone owner, not a vague TODO. Required sections: `Milestone DAG`, `Phase checklists`, `Future wave-plan boundaries`, `Risk registry`, `Requirement traceability`, `Design reconciliation`, `Decision register`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/master/ROADMAP_AND_RISKS.md"); s=p.read_text(); required=["## Milestone DAG","## Phase checklists","## Future wave-plan boundaries","## Risk registry","## Requirement traceability","## Design reconciliation","## Decision register"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'

```

Also check all coverage IDs:

```sh
python3 -c 'from pathlib import Path; s=Path("planning/master/ROADMAP_AND_RISKS.md").read_text(); assert all("R%02d"%i in s for i in range(1,21)); print("R01-R20 present")'
```


### 1.4.1 — Assemble the master implementation plan

- **ID:** 1.4.1
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** `1.3.1`.
- **Must edit:** none; all deliverables are new.
- **Can create:** `planning/HYDRUS1D_RUST_PORT_PLAN.md`; `planning/tasks/1.4.1.todo.md`.
- **Scratch only:** `.development/1.4.1/` (untracked).
- **Reference implementations/evidence:** planning/master/SOURCE_AUDIT.md, STATE_AND_CALLS.md, FEATURE_MATRIX.md, NUMERICAL_DESIGN.md, API_AND_IO.md, VALIDATION_STRATEGY.md, ROADMAP_AND_RISKS.md; INITIAL_PROMPT.md.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Synthesize the seven supporting documents into a standalone, readable master blueprint. Include audited call/state overview, architecture ASCII diagrams, proposed Cargo workspace/module layout, core Rust signatures, numerical/ownership contracts, I/O/bindings, validation, risk registry and detailed phased checklists. Incorporate the reconciled recommendations and explicit source/precision/version limitations. Link detailed evidence appendices without replacing requested blueprint content with an index. Separate user decisions, technical recommendations, unresolved gates and future implementation commitments. State that this document can seed multiple later wave plans and authorizes no implementation by itself. Rebase relative links from planning/master to planning correctly. Do not simply concatenate contradictory reports.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n '^##|^###|Decision|reconciliation|R[0-9]{2}' planning/master/*.md
```

**Acceptance:** The master contains all five original output requirements itself, full phase checklists, and R01–R20 mapping. All document links resolve; no new implementation code or implementation-file changes are introduced by this orchestration. The original Fortran remains intact. Contradictory recommendations are reconciled or explicitly raised. Required sections: `Goal and scope`, `Decisions and open gates`, `Source audit`, `Architecture`, `Rust interfaces`, `Numerical design`, `I/O and host adapters`, `Validation strategy`, `Risk registry`, `Implementation roadmap`, `Requirement traceability`, `Future wave plans`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/HYDRUS1D_RUST_PORT_PLAN.md"); s=p.read_text(); required=["## Goal and scope","## Decisions and open gates","## Source audit","## Architecture","## Rust interfaces","## Numerical design","## I/O and host adapters","## Validation strategy","## Risk registry","## Implementation roadmap","## Requirement traceability","## Future wave plans"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'

```

Also check all coverage IDs:

```sh
python3 -c 'from pathlib import Path; s=Path("planning/HYDRUS1D_RUST_PORT_PLAN.md").read_text(); assert all("R%02d"%i in s for i in range(1,21)); print("R01-R20 present")'
```


### 1.5.1 — Review master-plan scientific and scope coverage

- **ID:** 1.5.1
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** `1.4.1`.
- **Must edit:** none; all deliverables are new.
- **Can create:** `planning/master/REVIEW_SPEC.md`; `planning/tasks/1.5.1.todo.md`.
- **Scratch only:** `.development/1.5.1/` (untracked).
- **Reference implementations/evidence:** INITIAL_PROMPT.md; HYDRUS_RESEARCH.md; EXECUTION_PLAN_1.md decisions/R01–R20; master blueprint and source/manual anchors needed to verify claims.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Independently review the master against the original prompt, later user decisions and evidence. Check every requirement, five scenarios, scope status, simple-first order, dual porosity and recommendations versus commitments. Spot-check scientific assertions against source/manual anchors, especially precision, stencil, pivot, mass balance and rejected-step coupling. Report prioritized findings with section, evidence, required correction and acceptance implication; explicitly record checked requirement IDs. Do not read REVIEW_STANDARDS.md before submitting the first report. Do not change the blueprint or supporting design documents.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n '^##|R[0-9]{2}|4\.08|4\.17|dual|hyster|defer|decision' planning/HYDRUS1D_RUST_PORT_PLAN.md
```

**Acceptance:** Report every R01–R20 coverage disposition and any scientific/scope blockers with actionable evidence. A no-findings verdict still documents source spot checks and limitations. Required sections: `Verdict`, `Coverage checked`, `Findings`, `Evidence`, `Required corrections`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/master/REVIEW_SPEC.md"); s=p.read_text(); required=["## Verdict","## Coverage checked","## Findings","## Evidence","## Required corrections"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'

```



### 1.5.2 — Review master-plan architecture and execution readiness

- **ID:** 1.5.2
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** `1.4.1`.
- **Must edit:** none; all deliverables are new.
- **Can create:** `planning/master/REVIEW_STANDARDS.md`; `planning/tasks/1.5.2.todo.md`.
- **Scratch only:** `.development/1.5.2/` (untracked).
- **Reference implementations/evidence:** Master blueprint, roadmap, numerical/API/validation documents; wave-orchestration right-sizing and git-workflow-standards for future handoffs.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Independently review interface coherence, re-entrancy/foreign ownership, error/rollback design, allocation claims and algorithm assumptions. Check milestone dependencies, test creation prerequisites, executable versus proposed commands, risk treatments and enough detail to generate several small-model wave plans later. Detect shared Cargo registries/public types/global tests as future coordination hazards. Verify links and distinguish unknown evidence from passing acceptance. Report concrete findings without enforcing executable task specs on every future master-roadmap checklist item: detailed future wave plans are intentionally later work. Do not read REVIEW_SPEC.md before submitting the first report; do not change the master.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n '^##|prerequisite|allocation|rollback|ownership|command|milestone|gate' planning/HYDRUS1D_RUST_PORT_PLAN.md
```

**Acceptance:** Reports dependency and interface contradictions, dead links, invented green baselines and missing decision gates with exact references. Records review limitations even if no blocking findings. Required sections: `Verdict`, `Checks performed`, `Findings`, `Evidence`, `Required corrections`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/master/REVIEW_STANDARDS.md"); s=p.read_text(); required=["## Verdict","## Checks performed","## Findings","## Evidence","## Required corrections"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'

```



### 1.6.1 — Resolve reviews and finalize the master for user review

- **ID:** 1.6.1
- **Repo:** `/home/andrew/AI/hyd.rs` (worker's isolated worktree).
- **Depends on:** `1.5.1`, `1.5.2`.
- **Must edit:** `planning/HYDRUS1D_RUST_PORT_PLAN.md`.
- **Can create:** `planning/master/REVIEW_RESOLUTION.md`; `planning/tasks/1.6.1.todo.md`.
- **Scratch only:** `.development/1.6.1/` (untracked).
- **Reference implementations/evidence:** Master blueprint and both review reports; supporting evidence documents read-only.
- **Do not touch:** all paths outside the explicit write scope, especially `src_reference/`, shared state, input documents and implementation files.

**Specification:** Revise the master to resolve both reviews. Record each finding's ID, disposition, changed section and verification evidence in REVIEW_RESOLUTION.md. Do not silently broaden agreed scope or choose a consequential unresolved release commitment: retain a specific user decision gate. Explain any superseded appendix recommendation in the authoritative master without rewriting other workers' outputs. If a finding requires additional source research beyond this task's bounded revision, stop and request an explicit plan amendment. Rerun master structural/traceability/link checks and ask the orchestrator to recheck each corrected review item. Submit the master for user review; technical completion does not mean user approval or permission to implement.

**Grep anchors** (use scoped excerpts around hits; do not dump entire large files):

```sh
rg -n '^##|blocking|high|medium|Findings|Required corrections' planning/master/REVIEW_*.md
```

**Acceptance:** Every review finding is resolved or marked as a specific remaining user decision; no unexplained blocker is called complete. The master passes its 1.4.1 checks again and includes the final required roadmap and decision gates. Required sections: `Resolution table`, `Remaining user decisions`, `Verification`, `Handoff`.

**Targeted check** (repository root; after the document is written):

```sh
python3 -c 'from pathlib import Path; p=Path("planning/master/REVIEW_RESOLUTION.md"); s=p.read_text(); required=["## Resolution table","## Remaining user decisions","## Verification","## Handoff"]; assert all(x in s for x in required), [x for x in required if x not in s]; assert all(s.split(x,1)[1].split("\n## ",1)[0].strip() for x in required); assert not any(line.rstrip()!=line for line in s.splitlines()); print(str(p)+": sections nonempty and whitespace clean")'

```


Rerun both master checks from 1.4.1 as well. The orchestrator rechecks review finding dispositions against the actual revised sections.

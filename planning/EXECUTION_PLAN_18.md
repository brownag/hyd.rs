# Orchestration 18 — Source-faithful Fortran solute compatibility backend

Status: **Approved by user on 2026-10-08, with amendments through 2026-10-12.** Waves 1–5 are complete; Wave 6 fixed-form reachability review is in progress, followed by the amended kernel/process waves. Companion files: [worker prompt](PROMPT_TEMPLATE_18.md), [state](STATE_18.md).

## Goal & scope

**Goal.** Build a Rust implementation of pinned `SOLUTE.FOR` behavior that becomes the default solute solver after parity verification across every mode and interaction reachable through the audited `HYDRUS` → `ChemIn` → `Solute` path. Preserve the current FV/BE solver as an explicitly selectable alternative. Enumerate source-present but inactive/unreachable helper code without claiming it as runtime parity. Demonstrate source-snapshot parity with field-specific, precision-aware tolerances, separately from mathematical accuracy and physical validation. This advances the user's broader goal of 1:1 parity for all Fortran features using Rust; it is not the whole-program port.

**In scope:** audit active/callable `SOLUTE.FOR` behavior and inventory inactive/unreachable source paths; define the normalized input/output contract; generalize the isolated reference-run and Rust-run adapters and comparator; create source-derived fixtures for every active mode/supported interaction; implement a source-faithful Rust backend preserving Fortran call order, boundaries, time/spatial weighting, active species/process behavior, solver behavior, and precision; expose it as default only after active-mode parity passes; preserve FV/BE behind explicit selection; record inactive/external prerequisites.

**Out of scope:** modifying pinned `src_reference/` or portability patches; porting the main HYDRUS driver, water-flow solver, heat solver, or unrelated Fortran modules; claiming full release/product parity; replacing or deleting the FV/BE solver; changing the scientific meaning of Fortran behavior to improve continuum accuracy; physical validation; arbitrary retuning of calibrated comparison limits to make a failing mode pass. Fortran solute modes that depend on externally computed water/thermal/chemical state must receive explicit normalized inputs; this orchestration does not port the upstream solver that produces those fields. Source-present solute paths that cannot be exercised or mapped must be reported as blockers, not silently excluded while claiming all-mode parity.

**Verbatim constraints and decisions carried forward:**

> “we need to preserve this solver, I do not want to throw it out, but our default behavior has to have parity with fortran so we can verify correctness 1:1. we can evaluate adding additional solvers.”
>
> “the very next orchestration is to build something that emulates the fortran EXACTLY, compares it, and confirms we have it actually working, so we can proceed with our overall plan to get 1:1 parity with all fortran features using rust”
>
> User selected **all solute modes now** for this orchestration, rather than one vertical slice.
>
> User selected **field-specific tolerances** for parity acceptance (exact agreement where representation permits, calibrated numerical limits otherwise), not byte-for-byte text identity.
>
> Prior instruction: “allow retries and followups”; do not let arbitrary timeout or artifact caps block work or force excessive user input.

## Decisions recorded

| Date | Decision |
|---|---|
| 2026-10-08 | Preserve FV/BE; the source-compatible Fortran-emulation backend becomes the default after its complete in-scope parity gate passes. FV/BE remains explicitly selectable. |
| 2026-10-08 | “All solute modes now” means every mode/interacting branch reachable through the audited active caller path, not merely one existing case. Source-present but unreachable helper algorithms are inventoried but are not claimed as executable modes. The overall Rust port continues feature-by-feature beyond solute. |
| 2026-10-08 | “Exact” means source-faithful behavior and field-specific precision-aware parity, not bit-identical output bytes. Derive tolerances from repeated/pinned build precision evidence; do not import the historical `1e-6` candidate as a universal threshold. |
| 2026-10-08 | Use up to three concurrent workers, consistent with `INITIAL_PROMPT.md`. Permit retries/follow-ups without a fixed attempt count or arbitrary timeout/artifact ceiling; log attempts and keep per-task scratch isolated. |
| 2026-10-08 | Task 18.1.1 source audit distinguishes the active `HYDRUS` → `ChemIn` → `Solute` behavior from helper code whose call sites are commented or whose switches cannot be enabled by the active reader. All active/callable solute behavior is parity scope; dormant helper algorithms are recorded as inactive, not implemented or claimed as parity modes. |
| 2026-10-08 | Wave 2 integration review established that the runner authenticates raw Fortran outputs but does not normalize them. Add a manifest-driven normalizer and paired Rust/Fortran CLI stages before full parity acceptance; fixture manifests must carry observation maps. |
| 2026-10-09 (superseded 2026-10-10) | For task 18.3.4, exercise dual-domain behavior through a full pinned HYDRUS run using independently authored water-model 6/7 deck inputs; let active `InitDualPor` initialize the dual-water state naturally. Do not add a Solute call-boundary injection harness or represent independently derived histories as active-path inputs. The original conclusion that `WATFLOW`'s `DualPor` exchange-update call was commented out was based on a fixed-form parsing error and is superseded: its `!` in column 6 continues the guarded active call. |
| 2026-10-09 | Task 18.4.1 may add the minimal `serde_json` and SHA-256 dependencies to `hydrus-core` (and update `Cargo.lock`) because the crate has no existing JSON/hash support, while the accepted adapter contract requires parsing serialized fixture requests and verifying canonical request hashes. Keep all dependency edits confined to this task. |
| 2026-10-10 | Wave 5 source review found that fixed-form lines with `!` in column 6 are continuation lines, not comments. Every Wave 5 implementation must establish executable status from fixed-form columns (or the pinned compiler's fixed-form interpretation) before excluding a line; column-1 `*`/`C` comment lines and actual inline `!` comments are distinct. Re-review and correct all three Wave 5 tasks, tests, and completion reports against active compiled statements before integration. Preserve the existing worktrees; prior targeted test passes do not establish correctness because tests encoded the mistaken interpretation. |
| 2026-10-10 | Follow-up fixed-form caller audit corrected two earlier reachability findings: `WATFLOW.FOR`'s `if(iDualPor.gt.0)` continues into an active `call DualPor`, and `SOLUTE.FOR`'s `if(iConcType.eq.2)` continues into an active `call FluxConc`; `HYDRUS.FOR` sets `iKod=2` when `lFlux` is enabled. Dynamic dual exchange and flux-concentration output are therefore active-path scope when their selectors are enabled. Reassess affected fixtures, compatibility input/output mapping, and downstream tasks; prior inactive/unreachable statements are not accepted evidence. |

## Reconnaissance and baseline

- Repo `/home/andrew/AI/hyd.rs`, Rust workspace package `hydrus-core`, edition 2021. No root `AGENTS.md`, `CONTRIBUTING*`, or README was found. The root `TODO.md` is unrelated planning-metadata cleanup; do not overwrite it.
- Entry points: `crates/hydrus-core/src/solute.rs` (1,474 lines), `crates/hydrus-core/src/lib.rs` (32 lines), pinned `src_reference/SOLUTE.FOR` (2,105 lines). Solute source routines include `Solute`, `Coeff`, `MatSet`, `BanSol`, `PeCour`, `Disper`, `NEquil`, `SorbConc`, `MassTran`, `FluxConc`, `Blocking`, `Deposit`, `NonEqInit`, `Exclusion`, `MoistDepIn`, `MassInit`, `SolMass`, `ZBRAK1`, `ZBRENT1`; use symbol/branch anchors rather than reading large files wholesale.
- `SOLUTE.FOR` contains source-present paths for linear/nonlinear species, equilibrium and nonequilibrium sorption, chain reactions, upwind/artificial dispersion, tortuosity, mobile/immobile and dual nonequilibrium behavior, bacteria/filtration/deposition, moisture/density/temperature/gas/vapor terms, and multiple boundary codes. Task 18.1.1 distinguishes active/callable behavior from dormant helper code; the latter is explicitly reported, not mislabeled as parity coverage.
- Current Rust API is `crates/hydrus-core/src/solute.rs`; `DeferredSoluteFeature` explicitly rejects multiple source-present modes. Current `advance_solute_step` is FV/BE. Preserve it as a named alternative; add a distinct source-compatible kernel rather than mutating its equations in place.
- The reference build path is `tools/stage_fortran_reference.py` plus `tools/build_fortran_reference.sh`; pinned source stays immutable and build artifacts belong below `.development/`. `tools/run_fortran_reference.py` currently hardcodes one derived Example 3 fixture and fixed output roots. `tools/compare_solute_reference.py` is likewise tied to Example 3. Existing Rust test/adapter `tests/solute_fixture_parity.rs` covers that derived case and does not represent all source state fields.
- Existing comparison material includes six authenticated printed-output pairs and 4 normalized operator reports. It shows real differences and 9,092 concentration interval failures in five cases; `reaction_zero` agrees. Existing parity tolerances are diagnostic/unassigned, and fields such as source Sorb, temperature, nodal hydraulic state, and several balances are unavailable or semantically unmatched. The 1,001-node derived Example 3 case is not a recovered manual deck.
- Validation sources: `planning/INITIAL_PROMPT.md` lines 6–11 establishes pinned snapshot as numerical reference for a selected subset; `planning/master/VALIDATION_STRATEGY.md` §§3, 28–50; `planning/master/SOLUTE_FORMULATION_INDEPENDENT_REVIEW_18.md`; `planning/master/SOLUTE_FORMULATION_DECISION_17.md`; `planning/master/SOLUTE_PRINTED_PARITY_EVIDENCE.md`; `planning/master/SOLUTE_ANALYTICAL_EVIDENCE.md`; `planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md`; `planning/master/FEATURE_MATRIX.md`; and Orchestrations 12–17.
- Baseline run 2026-10-08: `cargo test --workspace` passed (137 integration tests; no unit/doc tests); `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` passed (274 tests); `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed. Python tests emitted a guarded “solver execution forbidden” diagnostic inside a passing test; they did not run the pinned Fortran solver. No new Fortran run/build was performed for this reconnaissance. Existing working-tree changes are user work; preserve them.
- No shared test server/port is used. Python/reference runs can be independent only with unique task scratch/build/run roots. At wave end, execute global checks serially. `rg` is not installed in this environment; use `grep`/code search for source anchors.

## Wave table

| Wave | Task | Depends on | Deliverable |
|---|---|---|---|
| 1 | 18.1.1 | — | Audited source-mode matrix, Rust/Fortran normalized contract, parity field/tolerance plan |
| 2 | 18.2.1, 18.2.2 | 18.1.1 | General Fortran runner and source-only fixture/comparison orchestrator |
| 3 | 18.3.1–18.3.5 | Wave 2 | Reproducible Fortran fixtures/reference manifests spanning transport/boundaries, reactions/sorption, optional properties, nonequilibrium, and biology/deposition modes |
| 4 | 18.4.1 | Wave 3 | Rust compatibility state/types/precision and normalized-input adapter |
| 5 | 18.5.1, 18.5.3 | 18.4.1; 18.5.2 superseded by 18.7.1 | Source-faithful transport/time assembly and accounting; primitive endpoint implementation deferred |
| 6 | 18.6.6 | 18.5.1, 18.5.3; 18.5.2 superseded | Full fixed-form caller/reachability re-audit and corrected compatibility matrix |
| 7 | 18.7.4, then 18.7.1–18.7.3 and 18.7.5–18.7.8 | 18.6.6 | Selector mapping gates numerical endpoints, active updates, accounting, coupling and fixtures. **Amended 2026-10-13:** Wave 7 runs as sequenced sub-waves because 18.7.1, 18.7.3 and 18.7.8 each need shared `types.rs`/`adapter.rs` additions — 7A: 18.7.1 ∥ 18.7.6; 7B: 18.7.2 ∥ 18.7.3; 7C: 18.7.5 ∥ 18.7.7 ∥ 18.7.8. |
| 8 | 18.8.1–18.8.2 | Wave 7 | Conditional source initialization and selector-2 moisture-path resolution |
| 9 | 18.9.1–18.9.5 | Wave 8 | Source-faithful species/process kernels |
| 10 | 18.10.1 | Wave 9 and accepted Wave 7–8 prerequisites | Source-ordered solute driver, including both accounting call sites |
| 11 | 18.11.1 | 18.10.1 | Nonlinear iteration, convergence, and retry behavior from active `ChemIn` inputs |
| 12 | 18.12.1 | 18.11.1 | Expose selectable compatibility candidate; keep current default |
| 13 | 18.13.1 | Wave 3, Wave 7 | Normalize raw Fortran outputs using fixture observation maps |
| 14 | 18.14.1 | 18.12.1, 18.13.1 | Wire paired Fortran/Rust execution and field-wise comparison |
| 15 | 18.15.1 | accepted 18.14.1 | Run/accept all-mode parity and publish evidence |
| 16 | 18.16.1 | accepted 18.15.1 | Switch default to compatibility backend; keep FV/BE selectable |

Tasks within each wave own disjoint files. Waves are sequential because the normalized contract, adapters, fixtures, and shared types gate implementation. Use worktree isolation when parallel tasks in a wave share the repository; all task scratch/build/run roots must be unique.

**Wave 2 runner/CLI interface (orchestrator contract):** `tools.run_fortran_reference.run_fixture(fixture_manifest_path, task_root, timeout_seconds=None)` returns a result mapping with `complete`, `task_root`, `run_root`, `input_sha256`, `output_paths` (logical name → path relative to `task_root`), `output_sha256`, and pinned source/archive, patch, and executable hashes. The parity CLI accepts one `--fixture-manifest` or serial `--fixture-group` values: `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `flux_concentration`, or `all`. Group roots map to their corresponding parity fixture directories; `all` discovers all `CASE.json` manifests under the parity fixture root. Each fixture receives a unique child task root. The CLI validates returned hashes/paths and reports authenticated **raw outputs**, not normalized observations; Wave 13 adds normalization before comparison.

## Shared execution rules

- No production implementation begins before this plan is approved and each worker's micro-TODO is approved by the orchestrator. No worker may broaden scope or touch another task's files.
- Treat the pinned source snapshot plus the recorded portability patch/compiler configuration as the oracle. Do not edit the pinned source or alter its algorithm for convenience. Record any unavoidable build shim separately from source behavior.
- **Fixed-form source reading is mandatory:** determine statement/comment/continuation status by physical columns under the pinned compiler's fixed-form rules, not visual `!` characters. In particular, `     !` has `!` in column 6 and continues the preceding statement. Trace each active executable formula through its full continuation chain; tests must exercise those formulas. Exclusions require evidence that the source line is a fixed-form comment or is unreachable on the audited caller path.
- Match source-level call/iteration order, coefficients, endpoints, branch conditions, truncations/casts, and mixed precision. Track earliest divergence (normalized inputs → source terms → matrix rows/RHS → solve → state/output) so “different outputs” become actionable parity failures, not a reason to skip comparisons.
- Do not turn parity into a physical/continuum correctness claim. Keep independent analytical/convergence evidence separate. Existing FV/BE tests must remain valid under explicit solver selection.
- For each audited mode/valid interaction, retain fixture/input provenance and source/build/output hashes; compare inputs before outputs. Use field-specific `abs(a-b) <= atol + rtol*max(|a|,|b|)` and appropriate discrete/exact checks for flags, schedules, state transitions, iteration status, and output schema. Tolerances must be empirically calibrated to pinned output precision and repeatability; unexplained failures remain failures, not tolerance-expansion invitations.
- **Fixture observation-map schema:** each fixture bundle has a `CASE.json` with `fixture_id`, normalized `request`, `request_sha256`, and `input_files` SHA-256 map whose keys exactly match the runner manifest's legacy input list. `OBSERVATION_MAP.json` uses `schema_version: "solute-observation-map-v1"` and an `observations` array; each record declares a unique normalized field/location identity (field, units, species/time/node as applicable), raw output file, row selector, source column(s), native-to-canonical coordinate/species/time mapping, and source numeric format/precision interval. Unavailable fields have an explicit reason; do not map them to zero. Task 18.13.1 owns the parser implementation, but fixture tasks must emit this contract.
- Allow diagnostic/evidence follow-ups without an arbitrary total attempt, timeout, or artifact ceiling. Log every attempt, preserve failures, use unique `.development/<task-id>/` roots, and resume rather than treating operational resource limits as scientific outcomes. Individual worker retries still follow the wave escalation ladder: after one acceptance retry, stop the small-model worker and escalate; the orchestrator may continue with a capable worker and a reviewed plan amendment, without imposing a global diagnostic cap. Do not overwrite prior authenticated artifacts. Ask the user only if a genuine product-scope decision cannot be resolved from the recorded decisions.
- Run only task-targeted checks during worker execution. Orchestrator reruns targeted checks, then all wave-end suites serially: `cargo test --workspace`; `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`; `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`; plus the task's full source-vs-Rust parity command for the final wave. Preserve the parity run summary and hashes.
- One conventional commit per accepted implementation task; stage only that task's files. Never stage or alter the pre-existing dirty Orchestration 16/17 and independent-review work.

## Task specifications

### 18.1.1 — Freeze source behavior and parity contract

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.1.1.todo.md`; `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md`.
- **Depends on:** none.
- **Spec:** Audit every active source-present branch in `SOLUTE.FOR` and required callers/inputs. Create a traceable matrix for transport/time weighting, endpoints, species/reactions, equilibrium/non-equilibrium, mobility/dual modes, bacteria/filtration/deposition, property terms, and every other discovered branch. Distinguish flags from valid combinations and clearly mark source-present code that is not reachable through the active reader/caller. Define normalized inputs/checkpoints/outputs, units, precision, exact versus tolerance comparisons, fixture provenance, and non-goals outside `SOLUTE.FOR`. Preserve FV/BE and gate the default switch on parity evidence for every active/callable mode; do not imply dormant helper code was executed. Do not decide scientific superiority or edit code.
- **Do not touch:** code, Fortran source, inputs, fixtures, existing parity evidence, `TODO.md`, or other tasks' outputs.
- **Anchors:** `SOLUTE.FOR` routines listed in Reconnaissance; relevant `INPUT.FOR`/`HYDRUS.FOR` call and input paths; `INITIAL_PROMPT.md` compatibility decision; `FEATURE_MATRIX.md`; `SOLUTE_VERIFICATION_SPEC.md`; `VALIDATION_STRATEGY.md`; Orchestrations 15–17 reports; `CASE.json` for existing derived Example 3.
- **Acceptance:** every source-present `SOLUTE.FOR` conditional path is either mapped to active/callable parity scope or explicitly identified with evidence as inactive/unreachable; normalized contract maps input/output semantics and precision; exact checks and field-specific tolerance calibration are stated; default-switch gate and preserved alternative are unambiguous. `git diff --check`.

### 18.2.1 — Generalize isolated Fortran fixture runner

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `tools/run_fortran_reference.py`, `tools/test_run_fortran_reference.py`.
- **Can create:** `planning/tasks/18.2.1.todo.md`.
- **Depends on:** 18.1.1.
- **Spec:** Replace the one-case hardcoding with a manifest-driven runner for the compatibility-contract fixtures. Preserve pinned source staging/hash checks, immutable input copies, isolated output roots, completeness/schema validation, compiler/build provenance, and input/output hashes. Permit task-local build/run roots so concurrent fixture tasks never share mutable outputs. Keep existing smoke and Example 3 commands working. Do not introduce fixed run/artifact caps; expose operational options without allowing an incomplete run to be recorded complete.
- **Do not touch:** pinned source/patch, case inputs, comparator, production Rust code, or retained run artifacts.
- **Anchors:** `run_fortran_reference.py` `run_case`, `validate_case_outputs`, `main`; `test_run_fortran_reference.py`; `stage_fortran_reference.py`; reference README.
- **Acceptance:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_fortran_reference`; runner self-test covers multiple fixture manifests, wrong/missing outputs, output-root isolation and preservation; `run_fixture` returns the fixed Wave 2 result contract including relative output paths/hashes under its unique task root; existing smoke/example invocations remain schema-compatible and do not rewrite retained `RUN.md`. No reference solver launch required for unit tests.

### 18.2.2 — Add source-only parity orchestration and comparator

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.2.2.todo.md`; `tools/run_solute_parity.py`; `tools/test_solute_fortran_parity.py`.
- **Depends on:** 18.1.1.
- **Spec:** Create a general source-only fixture runner/comparator CLI using the normalized compatibility contract and manifest-driven Fortran runner. Accept either one `--fixture-manifest` or `--fixture-group` for the named groups in the Wave 2 interface; execute group manifests serially with unique per-fixture child roots and an aggregate result. `all` discovers every `CASE.json` under the parity fixture root. Initially support `--reference-only` to authenticate raw outputs before normalizers/Rust kernels exist; later support backend selection and paired comparison. Consume the fixed `run_fixture` result; verify complete status, fixture input hashes, pinned source/archive/patch/executable provenance, every output path/hash under the task root, and isolation. Match `source_and_archive_sha256` to exactly the staged inventory (archive plus `src_reference/*.FOR`) from `SOURCE_MANIFEST.json`; ignore unrelated manual PDF entries that are not staged. Report raw authenticated outputs as raw; never fabricate normalized observations or feed Fortran values back as model inputs. Keep the current Example 3 comparator untouched.
- **Do not touch:** production modules, Fortran source, canonical CASES, existing verification schema/fixtures, current Example 3 comparator, or other tasks' files.
- **Anchors:** `run_fortran_reference.py` manifest pattern; `tests/solute_verification_harness.rs` output contract; printed parity evidence schema; `SOLUTE_FORTRAN_COMPATIBILITY_18.md`.
- **Acceptance:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_fortran_parity`; self-tests cover one-manifest and group discovery, serial execution/unique child roots, raw result schema/provenance/input/output hash validation, staged source-inventory matching including exclusion of unstaged PDF entries, path containment, exact/within/outside comparison helper behavior, field identity mismatch, and missing/duplicate observations. Existing Example 3 comparator self-test and read-only command still pass. No Rust or Fortran launch required for unit tests.

### 18.3.1 — Create transport, weighting, and boundary reference fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.3.1.todo.md`; disjoint fixture directories under `reference/fortran_reference/solute/parity/transport/` (including `CASE.json` and `OBSERVATION_MAP.json`) and task artifacts under `.development/18.3.1/`.
- **Depends on:** 18.2.1–18.2.2.
- **Spec:** Create reproducible normalized + legacy input pairs for baseline transport/storage, temporal `epsi`, upstream `wc`, base dispersivity, and every solute endpoint code/flow direction. Include `kTopCh=-2` using explicitly labeled derived `dSurf/cAtm` inputs from the accepted source reader; these are fixture forcing values, not expected outputs. Optional transport-property flags belong to 18.3.3. Add `OBSERVATION_MAP.json` mapping active printed output records to the normalized observation contract (file, row identity, species/time/node, column, units, source precision/interval); do not invent expected results. Run the pinned source through the generalized isolated runner, validate outputs/provenance, and do not call a derived fixture manual reproduction.
- **Do not touch:** solver implementation, pinned source, existing fixture inputs, tolerances, or other fixture-group paths.
- **Anchors:** `SOLUTE.FOR` `Solute`, `Coeff`, `MatSet`, `PeCour`, `Disper`, endpoint cases; current `scalar_robin`, `diffusion_cosine`, and Example 3 inputs/reports.
- **Acceptance:** every baseline transport/time-weighting/endpoint branch, including active `kTopCh=-2`, has a Fortran fixture or source-backed blocker after a documented attempt; normalized and legacy inputs agree; observation maps identify each expected output; completion and provenance hashes validate. Run `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group transport --reference-only --task-root <fresh unique .development/18.3.1 attempt root>`; no Rust expected outputs are authored.

### 18.3.2 — Create reaction and sorption reference fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.3.2.todo.md`; disjoint fixtures under `reference/fortran_reference/solute/parity/reactions/` (including `CASE.json` and `OBSERVATION_MAP.json`) and `.development/18.3.2/`.
- **Depends on:** 18.2.1–18.2.2.
- **Spec:** Cover one/multiple species, zero/distinct/equal chain rates, parent/terminal behavior, and active equilibrium/linear-sorption behavior. Nonlinear `lLinear=false` is reader-reachable and remains active scope, but its fixture is a follow-on to the corrected reader audit and must not be labelled inactive. Kinetic/mobile-immobile/dual-domain terms are owned by 18.3.4. Add `OBSERVATION_MAP.json` for active printed fields, distinguishing `Conc`, `Sorb`, `Sorb2`, species, time and units.
- **Do not touch:** solver code, pinned source, existing case files, parity tolerances, or other fixture-group paths.
- **Anchors:** `SOLUTE.FOR` `Solute`, `Coeff`, `MassInit`; reaction equality/distinct/zero cases; Example 3 provenance limitations.
- **Acceptance:** each assigned chain/equilibrium-sorption branch has a discriminating Fortran fixture; nonlinear selector is assigned to downstream fixture work and is not called inactive; the observation map covers expected active outputs without conflating pools. Inputs, provenance and output hashes validate. `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group reactions --reference-only --task-root <fresh unique .development/18.3.2 attempt root>`.

### 18.3.3 — Create optional transport-property fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.3.3.todo.md`; disjoint fixtures under `reference/fortran_reference/solute/parity/transport_properties/` (including `CASE.json` and `OBSERVATION_MAP.json`) and `.development/18.3.3/`.
- **Depends on:** 18.2.1–18.2.2.
- **Spec:** Exercise active transport-property branches from 18.1.1: tortuosity, artificial dispersion, temperature-dependent coefficients, and vapor/gas contributions with reachable inputs. `lDensity` is initialized false and has no active enabling assignment; record it inactive, like moisture-dependent helper factors. For active modes needing external histories, author explicit independent normalized histories and matching Fortran inputs. Add an observation map for active output values, units, source precision, and unavailable printed fields. Do not include biology or sorption/process fixtures.
- **Do not touch:** solver code, pinned source, existing fixture inputs, tolerances, or other fixture-group paths.
- **Anchors:** `SOLUTE.FOR` `Coeff`, `PeCour`, `Disper`, `MoistDepIn`; `FEATURE_MATRIX.md` solute rows.
- **Acceptance:** every active assigned transport-property branch, including a reachable vapor/gas discriminator, has a complete Fortran fixture; inactive moisture and density paths are evidenced from caller assignments; observation maps cover active printed outputs; normalized inputs are not oracle-fed; outputs/provenance hashes validate. Use `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group transport_properties --reference-only --task-root <fresh unique .development/18.3.3 attempt root>`.

### 18.3.4 — Create nonequilibrium and dual-domain fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.3.4.todo.md`; disjoint fixtures under `reference/fortran_reference/solute/parity/processes/nonequilibrium/` (including `CASE.json` and `OBSERVATION_MAP.json`) and `.development/18.3.4/`.
- **Depends on:** 18.2.1–18.2.2.
- **Spec (amended 2026-10-12):** Exercise active mobile-immobile and dual-domain coefficient/row behavior. For mobile-immobile mode, author a source-supported chemistry deck. For dual-domain mode, run full pinned HYDRUS with water models 6/7; `InitDualPor` initializes dual state and the continued `WATFLOW` call executes `DualPor` for `iDualPor>0`. Record selection and determine whether nonzero exchange is discriminated; never infer zero exchange from unobserved internals. Do not inject call-boundary arrays or misstate `NEquil`, `SorbConc`, or `NonEqInit` as commented: their continuation-completed callers are active conditionally and are follow-on work. Add observation maps and explicit unavailability reasons. Biology/deposition fixtures remain in 18.3.5.
- **Do not touch:** solver implementation, pinned source, existing fixtures, tolerances, other fixture-group paths, or runner/harness code unless the existing full-run interface demonstrably cannot launch the required deck; if so, stop and request a further amendment rather than silently adding a harness.
- **Anchors:** `SOLUTE.FOR` `NEquil`, `SorbConc`, `MassTran`, `NonEqInit`; `FEATURE_MATRIX.md` solute rows.
- **Acceptance (amended):** provide authenticated mobile-immobile and full-run model-6/7 dual fixtures; demonstrate source caller selection and `Solute` use, plus a dynamic-exchange discriminator or precise blocker. Observation maps mark unprinted internals unavailable without implying inactive. Validate hashes and run reference-only. This is raw authentication, not Rust parity.

### 18.3.5 — Create biology, filtration, and deposition fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.3.5.todo.md`; fixtures under `reference/fortran_reference/solute/parity/processes/biology/` (including `CASE.json` and `OBSERVATION_MAP.json`) and `.development/18.3.5/`.
- **Depends on:** 18.2.1–18.2.2.
- **Spec:** Exercise active `lBact`, `lFiltr`, `Blocking`, and `Deposit` paths and valid interactions. `Exclusion` has an active guarded caller; exercise only if required external state is faithfully available, otherwise document the blocker for 18.7.8. Map observed outputs/pools/units/precision and unavailable reasons.
- **Do not touch:** solver implementation, pinned source, existing fixtures, tolerances, or other fixture-group paths.
- **Anchors:** `SOLUTE.FOR` `Coeff`, `Blocking`, `Deposit`, `Exclusion`; `FEATURE_MATRIX.md` solute rows.
- **Acceptance:** each assigned biology/filtration/deposition branch has a Fortran discriminator; `Exclusion` is either exercised from a supported full run or explicitly blocked on mapped external state, never marked inactive based on column-6 `!`. Validate inputs, manifests, maps and hashes with reference-only runner; no parity claim.

### 18.4.1 — Add compatibility types, mode state, and precision model

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/lib.rs` (register a private module only); `crates/hydrus-core/Cargo.toml` and workspace `Cargo.lock` only to add the minimal JSON and SHA-256 dependencies required for the serialized adapter contract.
- **Can create:** `planning/tasks/18.4.1.todo.md`; `crates/hydrus-core/src/solute_fortran_compat/mod.rs`; `crates/hydrus-core/src/solute_fortran_compat/types.rs`; `crates/hydrus-core/src/solute_fortran_compat/adapter.rs`; placeholder module files `transport.rs`, `assembly.rs`, `boundary.rs`, `accounting.rs`, `processes.rs`, `nonequilibrium.rs`, `dual_domain.rs`, `biology.rs`, `deposition.rs`, `driver.rs`, and `iteration.rs`; unit tests in `types.rs` and `adapter.rs`.
- **Depends on:** Wave 3.
- **Spec (amended 2026-10-09):** Define source-mapped compatibility inputs, state/workspace, per-species and per-node process arrays, source switch enums/validated combinations, boundary forms, intermediate precision conversions matching the recorded Fortran build semantics, and trace observations. Implement parsing/mapping for serialized Wave 3 `CASE.json` requests in both accepted request schemas, preserving units, orientation, source indexing, switches, external state histories, explicit unavailable reasons and hashes. Verify canonical request SHA-256 using the same canonical JSON rules as `tools/run_solute_parity.py`; reject missing, duplicate, unmapped, nonfinite or inconsistent fields before mutation. Add/register private module skeletons so later kernels can be tested incrementally. This is the new separate backend foundation; do not alter or route the existing FV/BE API yet.
- **Do not touch:** `solute.rs` dispatch, `lib.rs` public exports beyond the private module registration, Fortran source, fixtures, or numerical implementations in kernel-module files assigned to later tasks. Do not introduce dependencies beyond the minimal JSON/SHA-256 support approved above.
- **Anchors:** compatibility contract and fixtures; `SOLUTE.FOR` declarations at `Solute`/`Coeff`; `INPUT.FOR`/`HYDRUS.FOR` mapping; current `SoluteConfig`, `SoluteState`, `SoluteWorkspace` as API patterns only (not formulas).
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::types::tests` and `cargo test -p hydrus-core solute_fortran_compat::adapter::tests`; every contract input/mode maps to explicit typed fields; invalid flags/hash/schema reject before mutation; precision conversions are tested. No solver parity claim yet.

### 18.5.1 — Port transport coefficients and time/spatial assembly

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/transport.rs`, `crates/hydrus-core/src/solute_fortran_compat/assembly.rs` (unit tests in those modules).
- **Can create:** `planning/tasks/18.5.1.todo.md`.
- **Depends on:** 18.4.1.
- **Spec (amended 2026-10-10):** Port source-equivalent `Coeff` transport terms, `PeCour`, `Disper`, and `MatSet` interior row/time-level formulas for audited non-process transport modes, preserving operation/rounding order, element support, upstream weighting and source conditionals. Implement `BanSol` or exact equivalent source pivot/order semantics. Read fixed-form source by columns and include all executable continuation lines: notably the Level-1 interior `F(i)` expression continued on lines 836–849 is executable, not commented. Exclude endpoint branches (18.5.2) and reaction/sorption process terms (18.8.1+); expose named internal seams for those contributions.
- **Do not touch:** API dispatch, shared types, boundaries/process modules, Fortran source, fixtures, or existing FV/BE implementation.
- **Anchors:** `SOLUTE.FOR` `Coeff` 241–754, `MatSet` 755–944, `BanSol` 945–964, `PeCour` 965–1024, `Disper` 1025–1077; transport fixture group.
- **Acceptance (amended):** source-anchored unit tests cover every assigned transport/time branch at coefficient, RHS/matrix, and solved-state levels, including a Level-1 interior RHS discriminator derived from the complete executable continuation chain. Preserve the Fortran expression tree and default-REAL operation order exactly (e.g., `Conc*alf*term` must not be regrouped as `Conc*(alf*term)`); expected test values must be independently derived, not copied from the implementation. The task report identifies fixed-form column interpretation for every source range and distinguishes actual comments from continuations. Run `cargo test -p hydrus-core solute_fortran_compat::transport::tests` and `cargo test -p hydrus-core solute_fortran_compat::assembly::tests`. Cross-language parity for the completed backend is gated and run in 18.15.1; do not widen tolerances to mask systematic row differences.

### 18.5.2 — Port Fortran solute endpoint rows and flux closure

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/boundary.rs`, `types.rs`, and `adapter.rs` with focused unit tests.
- **Can create:** `planning/tasks/18.5.2.todo.md`.
- **Depends on:** 18.4.1; interface integrates with 18.5.1 after both complete.
- **Spec (amended 2026-10-10):** Port all `kBotCh`/`kTopCh` branches and directional flow behavior into source-compatible endpoint equations, including equality-row replacement, third-type rows, vapor/atmospheric terms where in scope, and source boundary flux accounting. Read fixed-form source by columns and include executable continuation lines, including the Level-1 lower/upper row RHS formulas and their old-level endpoint flux terms; do not treat `     !` as a comment. The boundary module must evaluate endpoint formulas from their primitive source inputs (geometry, concentrations, transport/process coefficients, and atmospheric state), not accept already-aggregated RHS/row/flux answers that bypass this task. Keep only genuine inputs from prior modules as seams. Use physical and native orientation explicitly in tests. Do not approximate a Fortran equality row with FV half-cell rows.
- **Do not touch:** API dispatch, shared types, transport/process kernels, reference source, fixture files, or existing FV/BE boundaries.
- **Anchors:** `SOLUTE.FOR` `Solute` boundary accounting 51–85, 197–225; `MatSet` 755–944; boundary/transport fixtures and orientation contract.
- **Acceptance (amended):** `cargo test -p hydrus-core solute_fortran_compat::boundary::tests`; source-anchored unit tests cover each endpoint code/direction and boundary-flux equation, including Level-1 endpoint RHS evaluation and old/new flux accounting from complete executable continuation chains. Tests must pass primitive inputs and independently calculate expected expressions; they must not supply the completed RHS/flux as opaque input. The report documents fixed-form column interpretation and actual comments versus continuations; no oracle-fed boundary forcing. Cross-language parity is gated to 18.15.1.

### 18.5.3 — Port Fortran mass and source/output accounting

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/accounting.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.5.3.todo.md`.
- **Depends on:** 18.4.1; integrates with transport and boundary modules after Wave 5.
- **Spec (amended 2026-10-10):** Port source cumulative transport/process boundary accounting and observation quantities for `MassTran`/`FluxConc` with Fortran's quadrature/order/precision. In `MassTran`, implement every active `cvCh0`, `cvCh1`, `cvChR`, and `cvChIm` contribution selected by reachable modes, following full fixed-form continuation chains; the `cvChIm` mobile/immobile, dual-domain, two-site, and bacteria/filtration expressions on lines 1366–1488 are executable continuations, not comments. `FluxConc` is called from `Solute` when `iConcType==2`; `HYDRUS` sets that selector from `lFlux`, so preserve and expose this active conditional output in the compatibility contract/harness rather than labeling the call inactive. Preserve `Conc`, `Sorb`, `Sorb2`, flux concentration, and integrated totals as distinct source fields. Continue to exclude genuinely commented-out statements and truly unavailable fields. Compare every mapped output independently; do not infer source `Sorb` from an incompatible Rust equilibrium pool or close a residual by subtraction.
- **Do not touch:** public API, transport/boundary/process files, comparator, pinned source, or retained outputs.
- **Anchors:** `SOLUTE.FOR` `MassTran` 1319–1500, `FluxConc` 1501–1560; output parsers; compatibility output schema.
- **Acceptance (amended):** `cargo test -p hydrus-core solute_fortran_compat::accounting::tests`; source-anchored unit tests cover base trapezoids and each active `cvChIm` branch (including dual-domain and bacteria/filtration where reachable), terminal-node behavior, accumulation order, field mapping, and `FluxConc` helper equations. Report its conditional active caller path (`iConcType==2` from `lFlux`) and fixed-form column interpretation. Cross-language parity is gated to 18.15.1; all non-comparable fields remain explicitly unavailable.

### 18.6.6 — Re-audit fixed-form reachability and reset the active-mode matrix

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md`.
- **Can create:** `planning/tasks/18.6.6.todo.md`; `.development/18.6.6/` source-audit evidence.
- **Depends on:** accepted 18.5.1 and 18.5.3; 18.5.2 is superseded by 18.7.1 and is not a gate. Consistency review was accepted on 2026-10-12.
- **Spec:** Re-audit every previously classified active/inactive `SOLUTE.FOR`, `HYDRUS.FOR`, `WATFLOW.FOR`, and `INPUT.FOR` path using physical fixed-form columns and pinned compiler flags. Trace complete continued statements/calls from reader assignments through callers. Specifically resolve `NEquil`, `SorbConc`, `NonEqInit`, `MassInit`, `DualPor`, `FluxConc`, nonlinear `lLinear`, `lDualNEq`, `lFlux`/`iConcType`, beginning `MassTran`, moisture factors, `SetSSnk`, and `Exclusion`. Record active, unselected/unreachable, unsupported, and untested separately with physical-column evidence. Do not implement kernels or modify fixtures/API. Every discovered continuation caller must have an explicit follow-on task owner before acceptance.
- **Do not touch:** pinned Fortran files, code, fixtures/manifests/maps, existing parity artifacts, or other task TODOs.
- **Anchors:** fixed-form column 1–7 rules and pinned build flags; all former audit rows plus `SOLUTE.FOR` calls, `INPUT.FOR` flag assignments, and `HYDRUS.FOR`/`WATFLOW.FOR` callers.
- **Acceptance:** updated matrix classifies every audited call/branch and traces active call reachability; no source line is called a comment solely because it contains `!`; every exclusion cites fixed-form comment-column or unreachable caller evidence; follow-on task list covers every newly active behavior and its task has an explicit scope/acceptance owner. Distinguish source-level conditional callability from runtime-selected fixture coverage. `git diff --check`.

### 18.7.1 — Implement endpoint equations from primitive source inputs

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/boundary.rs`, `types.rs`, and `adapter.rs` with focused tests.
- **Can create:** `planning/tasks/18.7.1.todo.md`.
- **Depends on:** 18.6.6 and completed selector/state mapping from 18.7.4.
- **Spec (scope amended 2026-10-12):** Supersede incomplete 18.5.2 worktree implementation. Extend the normalized request only with source primitive inputs missing from `EndpointPrimitiveState`: per-node material IDs, source ChPar/TDep, iDualPor/lMobIm/lBact, rBot, cBot/cTop, atmospheric surface/material state, TLevel, accepted/new endpoint concentrations, and old/new endpoint flow. Implement lower/upper Level-1 and Level-2 equations and endpoint flux closure in source expression order for prescribed/type-three/free-drainage/atmospheric cases, vapor exception, flow predicates, and rMin. Never accept precomputed rows/RHS/residuals. Preserve native node 1/N equations and map orientation separately.
- **Do not touch:** assembly/transport/accounting/process modules, public API, fixtures, pinned source, or FV/BE boundaries.
- **Anchors:** fixed-form `SOLUTE.FOR` `MatSet` lines 755–944 and `Solute` flux accounting lines 197–225; corrected audit from 18.6.6.
- **Acceptance:** adapter and boundary tests cover source argument parsing/shape, every supported code/direction and assembly level, atmospheric/vapor/TLevel paths, orientation, and independently computed `D1*C1+E1*C2-F1`; `cargo fmt --check` and `git diff --check`. Do not accept partial path coverage as completion.
### 18.7.2 — Port active non-equilibrium and moisture-dependent updates

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/nonequilibrium.rs`, `crates/hydrus-core/src/solute_fortran_compat/processes.rs` (unit tests in the modules). **Amended 2026-10-14:** also `compat/mod.rs`, limited to adding `#[allow(dead_code)] // wired by 18.10.1` on the `nonequilibrium`/`processes` lines (per the 19.2.1 amendment). `Deposit` stays with 18.9.5: `SorbConc`'s `lFiltr` branch takes caller-supplied `rKa1`/`rKa2`.
- **Can create:** `planning/tasks/18.7.2.todo.md`.
- **Depends on:** 18.6.6 and normalized selector/state mapping from 18.7.4.
- **Spec:** Implement the continuation-completed, caller-reachable `NEquil` and `SorbConc` update calls, including source formula behavior for valid `lEquil`, `lLinear`, `lDualNEq`, bacteria, mobile/immobile, and dual-domain guards. Implement the reachable `iMoistDep=1` `rMD` factor assignments across `Coeff` and `SorbConc` (source ranges in the audit), including the reader's table layout, interpolation/selection, `f1` reset/retention, and default-REAL expression order. Resolve the distinct selector-2 `MoistDepIn` route by tracing all assignments/callers; unless a separate caller enables 2, record it as unselected by the audited reader, not as activated by `lMoistDep`. Preserve species/pool indexing and mutation timing. `NonEqInit`/`MassInit` belong to 18.8.1; do not label calls inactive because their arguments start in column 7 of a continuation record. Do not duplicate unrelated coefficient terms.
- **Do not touch:** shared types/adapter, transport/boundary/accounting, dual-domain, other process modules, public API, fixtures, or pinned source.
- **Anchors:** corrected 18.6.6 call matrix; `SOLUTE.FOR` `NEquil` 631–641 / 1078–1204 and `SorbConc` 184–188 / 1212–1313.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::nonequilibrium::tests` and `cargo test -p hydrus-core solute_fortran_compat::processes::tests`; independent source-derived tests cover each reachable updater/factor family, table/material/species selection, state/pool order, and selector guards. Selector-2 status is evidenced explicitly. No parity claim.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011).

### 18.7.3 — Port active dynamic dual-porosity exchange

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/dual_domain.rs` (unit tests in the module) and, **as amended 2026-10-13**, the shared `types.rs`/`adapter.rs` primitive state that `DualPor` consumes (per-material `ParD(7..11)` retention/immobile bounds and transfer coefficient, `thr`/`ths`, native node coordinates, `hNew`, `hCritA`, `iModel`, `MatNum`, old/new immobile water) with focused adapter tests.
- **Can create:** `planning/tasks/18.7.3.todo.md`.
- **Depends on:** 18.6.6, completed selector/state mapping from 18.7.4, and **merged 18.7.1** (amendment 2026-10-13: `DualPor` takes primitive state that no existing request type carries, and `types.rs`/`adapter.rs` are single-owner files, so this task is sequenced after 18.7.1 rather than run beside it). Where the pinned fixtures supply only coordinate *ranges* rather than numeric node arrays, the task must reject them explicitly or report the input blocker; it must not synthesize values.
- **Spec:** Port the active guarded `DualPor` exchange behavior identified by fixed-form caller audit, including initialization/iteration state, `SinkIm` sign and source-pool selection, material/water-model branches, operation order, and outputs consumed by `Solute`/`MassTran`. Do not preserve the earlier erroneous “commented call” behavior. Respect the active reader's water model mapping and reject only truly unsupported requests.
- **Do not touch:** boundary/transport/accounting/other process files, fixtures, public API, or pinned Fortran.
- **Anchors:** corrected 18.6.6 caller matrix; `WATFLOW.FOR:43–46` guarded `DualPor` call and `WATFLOW.FOR:1027–1071` body (including the column-6 continuation that activates the `WTransf` accumulation under `if(i.ge.2)`), `INPUT.FOR` `InitDualPor`/model mapping; `SOLUTE.FOR` dual-domain coefficient/accounting terms.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::dual_domain::tests`; source-derived tests discriminate zero/positive/negative exchange and both supported water models with correct pool selection and state timing.
### 18.7.4 — Map source selectors and conditional modes in normalized requests

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/types.rs`, `crates/hydrus-core/src/solute_fortran_compat/adapter.rs` and adapter tests.
- **Can create:** `planning/tasks/18.7.4.todo.md`.
- **Depends on:** 18.6.6.
- **Spec (approved expansion):** Map source-derived selectors and controls: `lFlux` → `iConcType=2`; `lMoistDep` → `iMoistDep=1` plus WDep data; `lMassIni`/`lEqInit`; per-material/species `ChPar` and `lBact` inputs needed to derive `lLinear` through `ChemIn`; and `iDualPor`/water-model state. Add a single typed primitive endpoint-state bundle for 18.7.1 containing coordinates, level-specific water/velocity/dispersion/retardation, old concentration, source terms, time weights, and optional atmospheric inputs. Do not accept precomputed rows/residuals. Preserve existing fixture defaults and explicit unavailable values. No kernel formulas or fixture edits.
- **Do not touch:** accounting/kernel formulas, fixtures, public dispatch, Fortran source, or parity tools.
- **Anchors:** fixed-form audit 18.6.6; `HYDRUS.FOR` `lFlux`/`iKod` and `SOLUTE.FOR` `iConcType` call; adapter schema/tests.
- **Acceptance:** adapter tests cover source defaults, enabled/disabled flux, WDep and initialization flags, exact per-species linear predicates across every material, and dual-model selectors. Endpoint bundle parsing/shape checks compile, with full equation-discriminating tests owned by 18.7.1; no runtime parity claim.

### 18.7.5 — Implement beginning-of-step MassTran accounting

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/accounting.rs` and its unit tests.
- **Can create:** `planning/tasks/18.7.5.todo.md`.
- **Depends on:** accepted 18.5.3 accounting API and 18.6.6 and 18.7.4.
- **Spec:** Add the source call at `SOLUTE.FOR:135–140`, guarded by `Level==1 && Iter==1`, using old-level temperature, water, velocity, pools, and `alf`. Preserve separation from the existing end-of-step call and prove call ordering/cumulative state effect. Do not modify the driver or introduce oracle-fed aggregate values. The driver task owns invoking this implementation.
- **Do not touch:** driver, transport/boundary, shared adapter/types, other kernels, fixtures, or pinned source.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::accounting::tests`; source-derived tests distinguish old/new snapshots, guard behavior, weighting (`alf` vs `epsi`), and cumulative order. No parity claim.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011).

### 18.7.6 — Add authenticated dynamic dual-exchange reference fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.7.6.todo.md`; new disjoint full-HYDRUS fixtures under `reference/fortran_reference/solute/parity/processes/dual_exchange/` and unique attempt roots under `.development/18.7.6/`.
- **Depends on:** 18.6.6 and 18.7.4; use the generalized runner and existing fixture schema. This task is independent of Rust exchange implementation and must not claim Rust parity.
- **Spec:** Revalidate existing model-6/7 nonequilibrium runs for actual dynamic `SinkIm` evidence. If they do not discriminate exchange, author independent full-run decks for each supported water model and matched controls; authenticate all raw outputs and provenance. Do not edit or overwrite the already authenticated 18.3.4 artifacts. Maps must capture only printed active evidence and explicitly mark unprinted internal exchange values unavailable; no parity claim from raw runs.
- **Do not touch:** prior fixtures/artifacts, source, runner/harness, kernels, or tolerances.
- **Anchors:** corrected fixed-form `DualPor` call path in 18.6.6; 18.3.4 decks/manifests as reference only.
- **Acceptance:** model 6 and 7 each have authenticated full-run evidence of an exchange-discriminating output or a precise source/input blocker; input/run/output/provenance hashes validate; run the nonequilibrium reference-only parity CLI in a fresh task root. Raw authentication is not normalized parity.
### 18.7.7 — Add authenticated conditional FluxConc reference fixture

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** new disjoint fixture under `reference/fortran_reference/solute/parity/flux_concentration/` and unique attempt roots under `.development/18.7.7/`.
- **Depends on:** 18.6.6 and 18.7.4.
- **Spec:** Create a source-supported HYDRUS fixture with `lFlux` enabled so the active `iConcType=2` call executes; map printed flux-concentration output if present, otherwise record the exact raw output limitation and do not fabricate observations. Authenticate inputs, runner provenance, outputs and hashes; keep expected values out of the normalized request.
- **Do not touch:** prior fixture groups/artifacts, kernels, parity normalizer/comparator, pinned source, or tolerances.
- **Anchors:** `HYDRUS.FOR` lFlux flag setup, `SOLUTE.FOR` FluxConc call/output, Wave 3 observation-map schema.
- **Acceptance:** a complete authenticated reference-only run proves the enabled selector reaches the call; map every printed output or declare explicit unavailability and a blocker. Validate request/input/output/provenance hashes; no parity claim.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011). Its producer state reaches the kernel through `adapter/source_coupling.rs` (D10). 18.7.8 is done and is not amended.

### 18.8.1 — Port conditional source initializers

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/initialization.rs`, `crates/hydrus-core/src/solute_fortran_compat/mod.rs` (module registration), and unit tests in `initialization.rs`.
- **Can create:** `planning/tasks/18.8.1.todo.md`.
- **Depends on:** 18.6.6 and 18.7.4.
- **Spec:** Implement source behavior for the continuation-completed, reader-selectable `MassInit` and `NonEqInit` caller paths. Reconstruct their complete formulas and state/order effects from pinned source (`MassInit:1912–1975`, `NonEqInit:1661–1706`), including `lMassIni`, `lEqInit`, `lEquil`, `lLinear`, material/species ordering, both sorbed pools, dual/mobile water state, and any `SolMass` root-solve path. Map initial concentrations/pools from explicit normalized state; do not assume these calls are disabled because their argument records use column-6 continuation. Do not port unrelated input-reader or water/heat initialization.
- **Do not touch:** shared types/adapter (owned by 18.7.4), transport, boundary, accounting, process kernels, public API, fixtures, or pinned source.
- **Anchors:** 18.6.6 physical-column evidence; `HYDRUS.FOR:282–288`; `INPUT.FOR` version-4 initialization selectors/defaults; `SOLUTE.FOR` `MassInit`, `NonEqInit`, `SolMass`, `ZBRAK1`, and `ZBRENT1`.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::initialization::tests`; tests cover selected and disabled calls, species/material traversal, pool mutation/order, root-solve behavior, and invalid/missing state rejection. No parity claim until wired by the driver.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011).

### 18.7.8 — Implement root and exclusion source-coupling consumers

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/source_coupling.rs`, `crates/hydrus-core/src/solute_fortran_compat/mod.rs` (module registration), and module tests; and, **as amended 2026-10-13**, the shared `types.rs`/`adapter.rs` mapping of the root/exclusion producer state this consumer reads (`lActRSU`, `SinkF`, sink/rate arrays, `lWat`, biology state) with focused adapter tests.
- **Can create:** `planning/tasks/18.7.8.todo.md`.
- **Depends on:** 18.6.6, normalized selector/state contract in 18.7.4, and **merged 18.7.1** (amendment 2026-10-13: the producer state it must map has no home in the shared request types, and `types.rs`/`adapter.rs` are single-owner files).
- **Spec:** Implement the continuation-completed `if(SinkF) call SetSSnk(...)` and guarded `if(lBact.and..not.lWat) call Exclusion(...)` consumers. Preserve source guards/order and arguments. Map required root/water/biology state, including `lActRSU` and sink arrays; do not implement producer algorithms. Missing external state must yield explicit unsupported before mutation, never a zero substitute.
- **Do not touch:** external water/root producer, public API, fixtures, pinned source, or unrelated kernels.
- **Anchors:** `SOLUTE.FOR:94–96`, `HYDRUS.FOR:345–355,390–391,482–487,513–517`, `INPUT.FOR:31,1313–1314`, and 18.6.6 evidence.
- **Acceptance:** module tests discriminate both guards, argument/unit mapping, and unsupported/missing producer state before mutation. No parity claim until driver and fixtures exercise these paths.
### 18.8.2 — Resolve remaining moisture-dependent selector path

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md`.
- **Can create:** `planning/tasks/18.8.2.todo.md`.
- **Depends on:** 18.6.6 and 18.7.4.
- **Spec:** Trace every assignment to `iMoistDep` and all callers of `MoistDepIn`. If no source path outside `lMoistDep → 1` selects 2, update the matrix with exact evidence and close without runtime implementation; if a path exists, implement and test its table loading/order. Do not duplicate mode-1 `rMD` factors owned by 18.7.2.
- **Do not touch:** runtime code, adapter/types, transport/boundary/accounting, dual-domain, biology/deposition, public API, fixtures, or pinned source.
- **Anchors:** 18.6.6 source-column evidence; `INPUT.FOR:1318–1320,1393–1406`; `SOLUTE.FOR:330–368,434–447,464–492,1232–1259`; `rMD` implementation in `SOLUTE.FOR`.
- **Acceptance:** source audit traces every assignment to `iMoistDep` and caller guard. If selector 2 is reachable, focused tests cover selected table loading and source order; otherwise provide exact caller/assignment evidence and classify selector 2 unselected. No parity claim.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011).

### 18.9.1 — Port reaction chains and equilibrium sorption

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/processes.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.9.1.todo.md`.
- **Depends on:** 18.4.1; consumes coefficient/assembly contribution seams from Wave 5.
- **Spec:** Port active species iteration/source semantics, `GamL/GamS` chain terms, equilibrium/linear sorption, `lEquil`, and relevant `Coeff` formulas in original order and precision. Nonlinear mode is conditionally reachable through the corrected reader audit; its iteration loop is owned by 18.11.1. Preserve source-specific mass terms and do not replace them with cleaner chain equations.
- **Do not touch:** types, transport, boundary, accounting modules, public API, or Fortran source.
- **Anchors:** `SOLUTE.FOR` `Solute` 44–230; `Coeff` reaction and sorption regions; `SolMass`, reaction fixture group, and independent operator reports.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::processes::tests`; tests cover zero/equal/distinct chain, terminal loss, linear equilibrium, and the nonlinear coefficient interface assigned from audit 18.6.6. Cross-language parity is gated to 18.15.1.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011).

### 18.9.2 — Port active mobile/immobile coefficients

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/nonequilibrium.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.9.2.todo.md`.
- **Depends on:** 18.4.1; use the stable shared process contribution interface declared by 18.4.1. Do not edit `processes.rs`.
- **Spec:** Port active mobile/immobile coefficient and row contributions identified by corrected audit 18.6.6. Active non-equilibrium update routines are owned by 18.7.2; do not duplicate them here. Preserve source pool fields and explicit availability. Dual-domain contributions are owned by 18.9.3.
- **Do not touch:** shared types, other numerical kernels, API, fixtures, or pinned source.
- **Anchors:** `SOLUTE.FOR` `Coeff` mobile/immobile rows; active update call matrix in 18.6.6; compatibility contract.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::nonequilibrium::tests`; tests verify active coefficient behavior and corrected state-update interfaces. Cross-language parity is gated to 18.15.1.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011).

### 18.9.3 — Port active dual-domain coefficients

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/dual_domain.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.9.3.todo.md`.
- **Depends on:** 18.4.1; use the stable shared process/mode interfaces declared there. Do not edit `processes.rs` or `nonequilibrium.rs`.
- **Spec:** Port active `iDualPor` transport/storage coefficient behavior and material selection, building on dynamic exchange implementation from 18.7.3 and evidence from 18.7.6. `lDualNEq` behavior follows corrected audit 18.6.6 and active update task 18.7.2 as applicable; do not label fixed-form continuations comments. This is separate from active mobile/immobile coefficients in 18.9.2. Report any active external-state mapping gap for amendment.
- **Do not touch:** shared types, transport/boundary/reaction/nonequilibrium modules, public API, fixtures, or Fortran source.
- **Anchors:** `SOLUTE.FOR` `Coeff` dual-domain paths; `INPUT.FOR` caller-derived `iDualPor`; active `NEquil` updater; domain fixtures.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::dual_domain::tests`; source-anchored tests cover active domain coefficient and exchange behavior based on Wave 6 audit. Cross-language parity is gated to 18.15.1.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011).

### 18.9.4 — Port bacteria and filtration branches

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/biology.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.9.4.todo.md`.
- **Depends on:** 18.4.1; use the stable shared process/mode interfaces declared there.
- **Spec:** Port source-present `lBact`, `lFiltr`, and their participating `Coeff`/`MassTran` terms for valid mapped configurations. Include exact source branch and parameter provenance. Deposition/blocking are separate in 18.9.5; `Exclusion` is owned by 18.7.3. If an active branch needs external data/coupling not expressible in the normalized contract, report it for plan amendment.
- **Do not touch:** shared types, transport/boundary/reaction/nonequilibrium/dual-domain/deposition modules, public API, fixtures, or Fortran source.
- **Anchors:** `SOLUTE.FOR` `Coeff` 372–541; process/biology fixture group.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::biology::tests`; source-anchored tests cover assigned bacteria/filtration equations or stop for explicit scope amendment. Cross-language parity is gated to 18.15.1.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011).

### 18.9.5 — Port deposition/blocking

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/deposition.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.9.5.todo.md`.
- **Depends on:** 18.4.1; use shared process/biology interfaces declared there.
- **Spec:** Port active `Blocking`/`Deposit` calls and their `Coeff`/`MassTran` contributions, preserving branch conditions, parameter provenance, operation order, and state updates. `Exclusion` is a separate active guarded HYDRUS path owned by 18.7.8. Do not edit `biology.rs`; any shared interface change requires a plan amendment before execution.
- **Do not touch:** shared types, other numerical modules, public API, fixtures, or Fortran source.
- **Anchors:** active `SOLUTE.FOR` `Blocking` 1561–1594 and `Deposit` 1595–1660; biology fixture group.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::deposition::tests`; tests cover active deposition/blocking; no claim about `Exclusion`, which is owned by 18.7.8. Cross-language parity is gated to 18.15.1.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011).

### 18.10.1 — Port source-ordered solute step driver

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/driver.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.10.1.todo.md`.
- **Depends on:** Waves 5–9; requires accepted 18.7.5 beginning accounting, 18.7.8 caller coupling, and 18.8.1 initialization.
- **Spec:** Implement the source `Solute` driver and exact call order for each reachable species/mode and both assembly levels, including source state inputs/outputs, coefficient → matrix → solve → update → accounting order, source casts and failure/termination diagnostics. Call `MassTran` at the beginning when `Level==1 && Iter==1` using old temperature/water/velocity/pools and `alf`, and at the end with accepted/new state and `epsi`; assert call order and cumulative state effects. Integrate other audited active kernels in source order and hand nonlinear iteration to 18.11.1. Keep source sequence even where Rust's existing transaction/step design differs; map external caller state explicitly.
- **Do not touch:** public `advance_solute_step` dispatch, source kernels owned by prior tasks, runner/comparator, fixtures, or Fortran source.
- **Anchors:** `SOLUTE.FOR` `Solute` 6–240 and calls to `Coeff`, `MatSet`, `BanSol`, `NEquil`, `MassTran`; all prior module tests and fixture matrix.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::driver::tests`; source-anchored tests verify both `MassTran` call sites, call ordering and state transitions for every branch assigned by 18.6.6. Full cross-language parity is gated to 18.15.1.
- **Amendment 2026-10-14 (Orchestration 19):** The driver builds `EndpointPrimitiveState`/`DualPorPrimitiveState` from step state in source order and does not re-validate what kernel `validate` covers (19.1.1). It removes each `wired by 18.10.1` allowance it satisfies (19.2.1). It consumes `WaterStepReport::accepted_water()` for accepted water state; `AcceptedWaterSnapshot` now lives in `water_state.rs` (19.6.2). `WaterStepInput` now has `lower: LowerBoundaryMode` and `upper: UpperBoundary` (19.4.2, 19.5.3). In `Head`/`Flux` modes the caller-owned upper row defines the last cell's equation, so it must be a true half-cell balance (19.6.2). It calls `source_coupling::{root_solute_uptake, size_exclusion}` at their source call sites. Adapter group files reach kernels as `super::super::<module>` (19.3.1).

### 18.11.1 — Implement nonlinear iteration and convergence behavior

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/iteration.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.11.1.todo.md`.
- **Depends on:** 18.6.6, 18.7.4, and 18.10.1.
- **Spec:** Implement the reader-reachable `lLinear=false` predicate and full source nonlinear loop/state sequencing, including the continued `lConv=.false.` assignment, convergence tolerance check, retry limit, timestep reduction, state rollback/commit, and termination diagnostics. Do not preserve the former erroneous guard that rejected nonlinear requests as unreachable. Do not change the existing default.
- **Do not touch:** public `solute.rs` dispatch, other kernel modules, runner/comparator, fixtures, or Fortran source.
- **Anchors:** corrected `INPUT.FOR` `ChemIn` source assignments; `SOLUTE.FOR` `Solute` nonlinear branch; normalized mode contract.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::iteration::tests`; source-derived tests cover linear/nonlinear reader selection, convergence success/failure, retry/timestep behavior, and state sequencing. No reachable branch is silently omitted; no parity claim.
- **Amendment 2026-10-14 (Orchestration 19):** New kernel files are registered in `compat/mod.rs` with `#[allow(dead_code)] // wired by 18.10.1` (STATE_19, 19.2.1). Kernel errors use `KernelError`; shared types are `pub(crate)`. Shape checks live in the kernel's `validate` and the adapter calls it (ADR-0014). `adapter.rs` no longer exists: it is `adapter/{mod,json_guard,fields,tests}.rs` plus group files `transport`, `reactions`, `transport_properties`, `nonequilibrium`, `biology`, `source_coupling` (19.3.1). New request records are serde DTOs in the matching group file, using the `fields.rs` helpers `decode_unit_value`, `present`, `map_available` and `require_object` (at every struct level, D12) with `deny_unknown_fields` (19.4.1); run rejection tests against the old code first. `Cargo.*` stays frozen (ADR-0011).

### 18.12.1 — Expose compatibility candidate without changing default

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/lib.rs`, and `crates/hydrus-core/tests/solute_solver_selection.rs`.
- **Can create:** `planning/tasks/18.12.1.todo.md`; `crates/hydrus-core/tests/solute_fortran_compat_harness.rs`; public `FortranCompatible` and `FiniteVolumeBackwardEuler` selectors. Explicit FV/BE selection updates may be made only in these existing targets: `solute_transport.rs`, `solute_acceptance.rs`, `solute_reactions.rs`, `solute_fixture_parity.rs`, `solute_species_boundaries.rs`, `solute_outlet.rs`, and `solute_verification_harness.rs`.
- **Depends on:** 18.11.1.
- **Spec:** Integrate the new backend with an explicit solver selector/call path and add a strict harness that consumes the normalized request, runs the candidate, and emits all mapped observations in the shared observation schema. Leave the preexisting default unchanged during qualification. Preserve the full FV/BE path and route existing scheme-specific tests explicitly to FV/BE without weakening expectations. Reject unsupported/malformed source-mode inputs before state mutation.
- **Do not touch:** compatibility kernels, Fortran fixtures/source, parity limits, analytical evaluator, or prior evidence artifacts.
- **Anchors:** `solute.rs` public advance entry points, `lib.rs`, existing solute integration tests, normalized contract from 18.1.1.
- **Acceptance:** `cargo test -p hydrus-core --test solute_solver_selection`, `cargo test -p hydrus-core --test solute_fortran_compat_harness`, and `cargo test -p hydrus-core --tests`; candidate emits schema-valid observations, current default remains FV/BE, and explicit FV/BE results/contracts remain unchanged.
- **Amendment 2026-10-14 (Orchestration 19):** Add `SoluteBackend::FortranCompatible` to the enum from 19.7.1 (`SoluteBackend::ALL`, `SoluteConfig::with_backend`, `SoluteConfig::backend()`); the `FortranCompatible`/`FiniteVolumeBackwardEuler` "selectors" are the enum variants. Remove the `wired by 18.12.1` allowances (19.2.1). `tests/solute_backend_contract.rs` extends: adding the variant to `SoluteBackend::ALL` makes its existing loop cover it; add variant-specific assertions only if needed, and add that file to Must edit. D11: this task edits `solute.rs`, so it also re-pins `POLICY_HASHES["crates/hydrus-core/src/solute.rs"]` in `tools/solute_benchmark_bridge.py`, with a bitwise-preservation justification for the FV/BE path. Add `tools/solute_benchmark_bridge.py` to Must edit for that one line.

### 18.13.1 — Normalize raw Fortran outputs from fixture maps

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `tools/solute_parity_normalizers.py`, `tools/test_solute_parity_normalizers.py`.
- **Can create:** `planning/tasks/18.13.1.todo.md`; `tools/solute_parity_normalizers.py`; `tools/test_solute_parity_normalizers.py`.
- **Depends on:** all Wave 3 fixture tasks, Wave 7 active-output fixtures, and the corrected normalized observation contract.
- **Spec:** Implement a strict manifest-driven parser that reads each fixture's `OBSERVATION_MAP.json`, authenticates raw output paths/hashes from `run_fixture`, and emits normalized `solute-observation-v1` records in the field/species/time/location/unit order declared by the fixture. Preserve source print intervals, D exponents/fixed-width formats as declared, coordinate reversal, availability reasons, and distinct `Conc`/`Sorb`/`Sorb2` fields. Reject missing, duplicate, extra, or malformed rows and never impute values. Normalize only outputs present in raw Fortran artifacts; internal checkpoints not printed or instrumented stay explicitly unavailable.
- **Do not touch:** runner, `run_solute_parity.py`, comparator, Rust solvers, fixture inputs/maps, pinned source, or prior evidence.
- **Anchors:** `SOLUTE_FORTRAN_COMPATIBILITY_18.md` observation semantics; Wave 3 `OBSERVATION_MAP.json` files; `OUTPUT.FOR` format statements; `tools/solute_printed_parity.py` parser/interval handling as reference only.
- **Acceptance:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_parity_normalizers`; synthetic tests cover fixed/scientific/D-exponent values, coordinate reversal, identity, print intervals, null availability, and malformed/duplicate/incomplete observations. Every Wave 3 observation map validates against its contract. No solver launch.

### 18.14.1 — Wire paired Fortran/Rust execution and comparison

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `tools/run_solute_parity.py`, `tools/test_solute_fortran_parity.py`.
- **Can create:** `planning/tasks/18.14.1.todo.md`; task-local outputs under `.development/18.14.1/`.
- **Depends on:** 18.2.1 runner, 18.2.2 source-only CLI, 18.12.1 candidate API/harness, and 18.13.1 normalizer.
- **Spec:** Extend the CLI with `--backend fortran-compatible`: authenticate the fixture; run the pinned source via `run_fixture`; normalize raw output via 18.13.1; invoke the explicit Rust candidate through the 18.12.1 adapter/harness with the same normalized inputs (never Fortran outputs as forcing); validate candidate observations; compare exact identities and field-specific criteria; write provenance-bound results. Exact comparison includes discrete values and availability/reason metadata; two nulls with different reasons are not equal. Keep `--reference-only` as raw authentication and leave the Example 3 comparator untouched.
- **Do not touch:** runner, fixtures/maps/inputs, production solver kernels, pinned source, tolerance values without evidence, or previous reports.
- **Anchors:** raw reference and comparison utilities in `run_solute_parity.py`; normalizer API from 18.13.1; candidate harness/API from 18.12.1; compatibility contract.
- **Acceptance:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_fortran_parity`; tests mock both adapters and cover same-request pairing, output-hash validation, no oracle-fed inputs, exact discrete checks, null availability/reason equality and mismatch, first divergence, and precision/tolerance outcomes. No real solver launch in unit tests.

### 18.15.1 — Verify all-mode parity and publish compatibility evidence

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `planning/master/VALIDATION_STRATEGY.md`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`.
- **Can create:** `planning/tasks/18.15.1.todo.md`; `planning/master/SOLUTE_FORTRAN_PARITY_18.md`; task-local outputs under `.development/18.15.1/`.
- **Depends on:** 18.14.1 and all fixture/kernel/normalizer tasks, including selector-2 status from 18.8.2.
- **Spec:** Run the complete fixture suite for every mode reachable through the audited active caller path, serially, using the pinned executable and explicit Rust compatibility candidate; optionally report FV/BE separately. Authenticate source/input/build/output identities; compare normalized inputs, all mapped outputs, and branch coverage with the accepted per-field policy. Compare internal checkpoints only where raw evidence actually supplies them; unavailable checkpoints are reported, not fabricated or treated as passed. Calibrate/verify tolerances from repeatability and precision evidence; preserve failures and follow worker escalation rules. Update evidence docs with achieved active modes, inactive helpers, and scientific gaps. Unresolved active output failures block completion and require corrective amendments. Do not claim physical/continuum validity.
- **Do not touch:** pinned source/patch, canonical old fixtures/artifacts, tolerance policy without evidence, solver kernels unless a plan amendment scopes a corrective task, or unrelated roadmap docs.
- **Anchors:** full compatibility contract and all-mode fixture manifests; current runner/comparator; `VALIDATION_STRATEGY.md`; printed parity, analytical, and convergence evidence; branch-by-branch TODO reports.
- **Acceptance:** `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group all --backend fortran-compatible --task-root <fresh unique .development/18.15.1 attempt root>` passes every active/callable branch, supported interaction, and mapped output against approved field-specific limits; manifests authenticate source/build/input/output identities; explicit FV/BE regressions pass; the report enumerates inactive/unreachable source code without claiming it as parity. The orchestrator serially reruns all global suites. Any failing/unmapped active mode blocks completion and the default switch.

### 18.16.1 — Switch default to verified compatibility backend

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/lib.rs`, `crates/hydrus-core/tests/solute_solver_selection.rs`.
- **Can create:** `planning/tasks/18.16.1.todo.md`.
- **Depends on:** accepted 18.15.1 only.
- **Spec:** Change the existing default entry point to select the source-compatible backend based on the completed all-mode parity evidence. Keep FV/BE explicitly selectable and preserve its full existing test coverage. Record the default change and evidence pointer in public API docs. Do not change numerical formulas, comparison thresholds, or FV/BE behavior here.
- **Do not touch:** compatibility kernels, Fortran source/fixtures, parity evidence artifacts, or unrelated API modules.
- **Anchors:** accepted `SOLUTE_FORTRAN_PARITY_18.md`; solver-selection code/tests from 18.12.1; public solute entry points.
- **Acceptance:** default-selection test proves `advance_solute_step` uses the verified Fortran-compatible backend; opt-in FV/BE tests remain unchanged and pass. Run `cargo test -p hydrus-core --test solute_solver_selection`, `cargo test -p hydrus-core --tests`, and `git diff --check`.
- **Amendment 2026-10-14 (Orchestration 19):** "Switch the default" means changing `SoluteConfig`'s default backend. D11: this task edits `solute.rs`, so it also re-pins `POLICY_HASHES["crates/hydrus-core/src/solute.rs"]` in `tools/solute_benchmark_bridge.py`, with a bitwise-preservation justification for the FV/BE path. Add `tools/solute_benchmark_bridge.py` to Must edit for that one line.

## Approval and handoff

Approval authorizes the plan, not an unsupported claim that every mode already has parity. The explicit compatibility candidate is first normalized and compared while the existing default remains intact. Only accepted task 18.15.1 evidence unlocks default switch task 18.16.1. If any audited source branch requires edits outside its task scope or another Fortran subsystem, amend this plan rather than widening permissions ad hoc. After this solute milestone, continue the larger Rust port with the next Fortran feature area; do not treat this as complete whole-program 1:1 parity.

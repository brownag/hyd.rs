# Orchestration 18 — Source-faithful Fortran solute compatibility backend

Status: **Approved by user on 2026-10-08.** Wave 1 task 18.1.1 is complete; Wave 2 is next. Companion files: [worker prompt](PROMPT_TEMPLATE_18.md), [state](STATE_18.md).

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
| 5 | 18.5.1–18.5.3 | 18.4.1 | Source-faithful transport/time assembly, boundary rows, and accounting modules |
| 6 | 18.6.1–18.6.5 | Wave 5 | Source-faithful species/process modules for equilibrium, nonequilibrium, dual-domain, bacteria/filtration/deposition branches |
| 7 | 18.7.1 | Wave 6 | Source-ordered linear-mode driver |
| 8 | 18.8.1 | 18.7.1 | Nonlinear iteration and state sequencing |
| 9 | 18.9.1 | 18.8.1 | Expose selectable compatibility candidate; keep current default |
| 10 | 18.10.1 | accepted 18.9.1 | Run/accept all-mode parity and publish evidence |
| 11 | 18.11.1 | accepted 18.10.1 | Switch default to compatibility backend; keep FV/BE selectable |

Tasks within each wave own disjoint files. Waves are sequential because the normalized contract, adapters, fixtures, and shared types gate implementation. Use worktree isolation when parallel tasks in a wave share the repository; all task scratch/build/run roots must be unique.

## Shared execution rules

- No production implementation begins before this plan is approved and each worker's micro-TODO is approved by the orchestrator. No worker may broaden scope or touch another task's files.
- Treat the pinned source snapshot plus the recorded portability patch/compiler configuration as the oracle. Do not edit the pinned source or alter its algorithm for convenience. Record any unavoidable build shim separately from source behavior.
- Match source-level call/iteration order, coefficients, endpoints, branch conditions, truncations/casts, and mixed precision. Track earliest divergence (normalized inputs → source terms → matrix rows/RHS → solve → state/output) so “different outputs” become actionable parity failures, not a reason to skip comparisons.
- Do not turn parity into a physical/continuum correctness claim. Keep independent analytical/convergence evidence separate. Existing FV/BE tests must remain valid under explicit solver selection.
- For each audited mode/valid interaction, retain fixture/input provenance and source/build/output hashes; compare inputs before outputs. Use field-specific `abs(a-b) <= atol + rtol*max(|a|,|b|)` and appropriate discrete/exact checks for flags, schedules, state transitions, iteration status, and output schema. Tolerances must be empirically calibrated to pinned output precision and repeatability; unexplained failures remain failures, not tolerance-expansion invitations.
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
- **Acceptance:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_fortran_reference`; runner self-test covers multiple fixture manifests, wrong/missing outputs, output-root isolation and preservation; existing smoke/example invocations remain schema-compatible. No reference solver launch required for unit tests.

### 18.2.2 — Add source-only parity orchestration and comparator

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.2.2.todo.md`; `tools/run_solute_parity.py`; `tools/test_solute_fortran_parity.py`.
- **Depends on:** 18.1.1.
- **Spec:** Create a general source-only fixture runner/comparator CLI using the normalized compatibility contract and manifest-driven Fortran runner. Initially support `--reference-only` so fixture tasks can build/authenticate source outputs before the Rust kernel exists; later support backend selection, first-divergence diagnostics, exact discrete checks, precision intervals and per-field acceptance rules. Keep the current Example 3 comparator untouched. Strictly verify hashes, complete outputs, input identity, field identity and artifact isolation. Never feed Fortran output values back as model inputs.
- **Do not touch:** production modules, Fortran source, canonical CASES, existing verification schema/fixtures, current Example 3 comparator, or other tasks' files.
- **Anchors:** `run_fortran_reference.py` manifest pattern; `tests/solute_verification_harness.rs` output contract; printed parity evidence schema; `SOLUTE_FORTRAN_COMPATIBILITY_18.md`.
- **Acceptance:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_fortran_parity`; self-tests cover reference-only invocation, exact/within/outside tolerance, field identity mismatch, missing/duplicate observations, hash tampering and unique run roots. Existing Example 3 comparator self-test/command still pass. No Rust or Fortran launch required for unit tests.

### 18.3.1 — Create transport, weighting, and boundary reference fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.3.1.todo.md`; disjoint fixture directories under `reference/fortran_reference/solute/parity/transport/` and task artifacts under `.development/18.3.1/`.
- **Depends on:** 18.2.1–18.2.2.
- **Spec:** Create reproducible normalized + legacy input pairs for baseline transport/storage, temporal `epsi`, upstream `wc`, base dispersivity, and every solute endpoint code/flow direction. Optional transport-property flags (tortuosity, artificial dispersion, moisture/density/thermal/gas terms) belong to 18.3.3. Run the pinned source through the generalized isolated runner, validate complete outputs, and retain provenance/hash manifests and per-step observations required by the contract. Do not call a derived fixture a manual reproduction.
- **Do not touch:** solver implementation, pinned source, existing fixture inputs, tolerances, or other fixture-group paths.
- **Anchors:** `SOLUTE.FOR` `Solute`, `Coeff`, `MatSet`, `PeCour`, `Disper`, endpoint cases; current `scalar_robin`, `diffusion_cosine`, and Example 3 inputs/reports.
- **Acceptance:** every baseline transport/time-weighting/endpoint branch assigned to this task has a Fortran fixture or source-backed blocker; normalized and legacy inputs agree; completion and provenance hashes validate. `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group transport --reference-only --task-root .development/18.3.1`; no Rust expected outputs are authored.

### 18.3.2 — Create reaction and sorption reference fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.3.2.todo.md`; disjoint fixtures under `reference/fortran_reference/solute/parity/reactions/` and `.development/18.3.2/`.
- **Depends on:** 18.2.1–18.2.2.
- **Spec:** Cover one/multiple species, zero/distinct/equal chain rates, parent/terminal behavior, and the equilibrium/linear-sorption branch reachable through the active reader. The `lLinear=false` nonlinear equation is source-present but cannot be selected by active `ChemIn`; record that fact rather than inventing an input or claiming fixture coverage. Kinetic pools/mobile-immobile/dual-domain terms are owned by 18.3.4. Keep source `Sorb`/`Sorb2` distinct from Rust pools.
- **Do not touch:** solver code, pinned source, existing case files, parity tolerances, or other fixture-group paths.
- **Anchors:** `SOLUTE.FOR` `Solute`, `Coeff`, `MassInit`; reaction equality/distinct/zero cases; Example 3 provenance limitations.
- **Acceptance:** each active chain/equilibrium-sorption branch has a discriminating Fortran fixture; the unreachable nonlinear selector is documented with its source evidence and is not represented as a runtime mode. Inputs, provenance and output hashes validate. `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group reactions --reference-only --task-root .development/18.3.2`.

### 18.3.3 — Create optional transport-property fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.3.3.todo.md`; disjoint fixtures under `reference/fortran_reference/solute/parity/transport_properties/` and `.development/18.3.3/`.
- **Depends on:** 18.2.1–18.2.2.
- **Spec:** Exercise active transport-property branches from 18.1.1: tortuosity, artificial dispersion, density-dependent and temperature-dependent coefficients, vapor/gas contributions, and valid combinations. The source-present moisture-dependent helper/factors are not called by the active program path; record them as inactive rather than creating a surrogate runtime fixture. For active modes needing external water/thermal histories, author explicit independent normalized histories and matching Fortran inputs. Do not include biology or sorption/process fixtures.
- **Do not touch:** solver code, pinned source, existing fixture inputs, tolerances, or other fixture-group paths.
- **Anchors:** `SOLUTE.FOR` `Coeff`, `PeCour`, `Disper`, `MoistDepIn`; `FEATURE_MATRIX.md` solute rows.
- **Acceptance:** every active assigned transport-property branch has a complete Fortran discriminator; inactive moisture helper status is evidenced from the call path; normalized inputs are not oracle-fed; outputs and provenance hashes validate. `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group transport_properties --reference-only --task-root .development/18.3.3`.

### 18.3.4 — Create nonequilibrium and dual-domain fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.3.4.todo.md`; disjoint fixtures under `reference/fortran_reference/solute/parity/processes/nonequilibrium/` and `.development/18.3.4/`.
- **Depends on:** 18.2.1–18.2.2.
- **Spec:** Exercise active mobile-immobile and dual-domain coefficient/row behavior where selected by source-supported inputs, with explicit external histories. The `NEquil`, `SorbConc`, and `NonEqInit` state-update calls are commented out on the audited path; preserve/report their inactive status instead of running helper formulas as though the Fortran program invoked them. Biology and deposition fixtures belong to 18.3.5. If an active branch lacks an input mapping, report it rather than omit it.
- **Do not touch:** solver implementation, pinned source, existing fixtures, tolerances, or other fixture-group paths.
- **Anchors:** `SOLUTE.FOR` `NEquil`, `SorbConc`, `MassTran`, `NonEqInit`; `FEATURE_MATRIX.md` solute rows.
- **Acceptance:** each active assigned mobile-immobile/dual-domain row behavior has a Fortran discriminator; commented state-update helpers are documented as inactive and not claimed as parity modes. Inputs, outputs and provenance validate. `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group nonequilibrium --reference-only --task-root .development/18.3.4`.

### 18.3.5 — Create biology, filtration, and deposition fixtures

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/tasks/18.3.5.todo.md`; fixtures under `reference/fortran_reference/solute/parity/processes/biology/` and `.development/18.3.5/`.
- **Depends on:** 18.2.1–18.2.2.
- **Spec:** Exercise active `lBact`, `lFiltr`, `Blocking`, and `Deposit` source paths and valid interactions. The `Exclusion` routine's caller is commented out; document it as inactive rather than treating its standalone code as an available runtime mode. Record parameter provenance/external histories. If an active path is not expressible, report the missing mapping for amendment.
- **Do not touch:** solver implementation, pinned source, existing fixtures, tolerances, or other fixture-group paths.
- **Anchors:** `SOLUTE.FOR` `Coeff`, `Blocking`, `Deposit`, `Exclusion`; `FEATURE_MATRIX.md` solute rows.
- **Acceptance:** each active assigned biology/filtration/deposition branch has a Fortran discriminator; `Exclusion` is explicitly marked inactive from its caller evidence. Inputs, completion markers and provenance hashes validate. `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group biology --reference-only --task-root .development/18.3.5`.

### 18.4.1 — Add compatibility types, mode state, and precision model

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/lib.rs` (register a private module only).
- **Can create:** `planning/tasks/18.4.1.todo.md`; `crates/hydrus-core/src/solute_fortran_compat/mod.rs`; `crates/hydrus-core/src/solute_fortran_compat/types.rs`; `crates/hydrus-core/src/solute_fortran_compat/adapter.rs`; placeholder module files `transport.rs`, `assembly.rs`, `boundary.rs`, `accounting.rs`, `processes.rs`, `nonequilibrium.rs`, `dual_domain.rs`, `biology.rs`, `deposition.rs`, `driver.rs`, and `iteration.rs`; unit tests in `types.rs` and `adapter.rs`.
- **Depends on:** Wave 3.
- **Spec:** Define source-mapped compatibility inputs, state/workspace, per-species and per-node process arrays, source switch enums/validated combinations, boundary forms, intermediate precision conversions matching the recorded Fortran build semantics, and trace observations. In the same module, implement strict mapping from the versioned normalized fixture request, preserving units, orientation, source indexing, switches, external state histories and hashes. Reject missing, duplicate, unmapped, nonfinite or inconsistent fields before mutation. Add/register private module skeletons so later kernels can be tested incrementally. This is the new separate backend foundation; do not alter or route the existing FV/BE API yet.
- **Do not touch:** `solute.rs` dispatch, `lib.rs` public exports beyond the private module registration, Fortran source, fixtures, or numerical implementations in kernel-module files assigned to later tasks.
- **Anchors:** compatibility contract and fixtures; `SOLUTE.FOR` declarations at `Solute`/`Coeff`; `INPUT.FOR`/`HYDRUS.FOR` mapping; current `SoluteConfig`, `SoluteState`, `SoluteWorkspace` as API patterns only (not formulas).
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::types::tests` and `cargo test -p hydrus-core solute_fortran_compat::adapter::tests`; every contract input/mode maps to explicit typed fields; invalid flags/hash/schema reject before mutation; precision conversions are tested. No solver parity claim yet.

### 18.5.1 — Port transport coefficients and time/spatial assembly

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/transport.rs`, `crates/hydrus-core/src/solute_fortran_compat/assembly.rs` (unit tests in those modules).
- **Can create:** `planning/tasks/18.5.1.todo.md`.
- **Depends on:** 18.4.1.
- **Spec:** Port source-equivalent `Coeff` transport terms, `PeCour`, `Disper`, and `MatSet` interior row/time-level formulas for audited non-process transport modes, preserving operation/rounding order, element support, upstream weighting and source conditionals. Implement `BanSol` or exact equivalent source pivot/order semantics. Exclude endpoint branches (18.5.2) and reaction/sorption process terms (18.6.1+); expose named internal seams for those contributions.
- **Do not touch:** API dispatch, shared types, boundaries/process modules, Fortran source, fixtures, or existing FV/BE implementation.
- **Anchors:** `SOLUTE.FOR` `Coeff` 241–754, `MatSet` 755–944, `BanSol` 945–964, `PeCour` 965–1024, `Disper` 1025–1077; transport fixture group.
- **Acceptance:** source-anchored unit tests cover every assigned transport/time branch at coefficient, RHS, and solved-state levels; run `cargo test -p hydrus-core solute_fortran_compat::transport::tests` and `cargo test -p hydrus-core solute_fortran_compat::assembly::tests`. Cross-language parity for the completed backend is gated and run in 18.10.1; do not widen tolerances to mask systematic row differences.

### 18.5.2 — Port Fortran solute endpoint rows and flux closure

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/boundary.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.5.2.todo.md`.
- **Depends on:** 18.4.1; interface integrates with 18.5.1 after both complete.
- **Spec:** Port all `kBotCh`/`kTopCh` branches and directional flow behavior into source-compatible endpoint equations, including equality-row replacement, third-type rows, vapor/atmospheric terms where in scope, and source boundary flux accounting. Use physical and native orientation explicitly in tests. Do not approximate a Fortran equality row with FV half-cell rows.
- **Do not touch:** API dispatch, shared types, transport/process kernels, reference source, fixture files, or existing FV/BE boundaries.
- **Anchors:** `SOLUTE.FOR` `Solute` boundary accounting 51–85, 197–225; `MatSet` 755–944; boundary/transport fixtures and orientation contract.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::boundary::tests`; source-anchored unit tests cover each endpoint code/direction and boundary-flux equation; no oracle-fed boundary forcing. Cross-language parity is gated to 18.10.1.

### 18.5.3 — Port Fortran mass and source/output accounting

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/accounting.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.5.3.todo.md`.
- **Depends on:** 18.4.1; integrates with transport and boundary modules after Wave 5.
- **Spec:** Port source cumulative transport/process boundary accounting and observation quantities for `MassTran`/`FluxConc` base paths with Fortran's quadrature/order/precision. Preserve `Conc`, `Sorb`, `Sorb2`, and integrated totals as distinct source fields. Compare every mapped output independently; do not infer source `Sorb` from an incompatible Rust equilibrium pool or close a residual by subtraction.
- **Do not touch:** public API, transport/boundary/process files, comparator, pinned source, or retained outputs.
- **Anchors:** `SOLUTE.FOR` `MassTran` 1319–1500, `FluxConc` 1501–1560; output parsers; compatibility output schema.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::accounting::tests`; source-anchored unit tests cover each assigned accounting equation and field mapping. Cross-language parity is gated to 18.10.1; all non-comparable fields remain explicitly unavailable.

### 18.6.1 — Port reaction chains and equilibrium sorption

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/processes.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.6.1.todo.md`.
- **Depends on:** 18.4.1; consumes coefficient/assembly contribution seams from Wave 5.
- **Spec:** Port active species iteration/source semantics, `GamL/GamS` chain terms, equilibrium/linear sorption, `lEquil`, and relevant `Coeff`/`SolMass` formulas in original order and precision. Do not expose the source-present `lLinear=false` branch as an available default-program mode when the active input reader cannot select it; document it as inactive. Preserve source-specific mass terms and do not replace them with cleaner chain equations.
- **Do not touch:** types, transport, boundary, accounting modules, public API, or Fortran source.
- **Anchors:** `SOLUTE.FOR` `Solute` 44–230; `Coeff` reaction and sorption regions; `SolMass`, reaction fixture group, and independent operator reports.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::processes::tests`; tests cover zero/equal/distinct chain, terminal loss, and active linear equilibrium equations, and confirm the dormant nonlinear selector is unavailable. Cross-language parity is gated to 18.10.1.

### 18.6.2 — Port active mobile/immobile coefficients

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/nonequilibrium.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.6.2.todo.md`.
- **Depends on:** 18.4.1; use the stable shared process contribution interface declared by 18.4.1. Do not edit `processes.rs`.
- **Spec:** Port only active mobile/immobile coefficient and row contributions identified by 18.1.1. The `NEquil`, `SorbConc`, and `NonEqInit` update routines are not called by the audited `HYDRUS` solute path; reproduce that inactive behavior and do not execute their standalone formulas as active solver updates. Preserve source pool fields and explicit availability. Dual-domain contributions are owned by 18.6.3.
- **Do not touch:** shared types, other numerical kernels, API, fixtures, or pinned source.
- **Anchors:** `SOLUTE.FOR` `Coeff` mobile/immobile rows; commented calls to `NEquil`, `SorbConc`, `NonEqInit`; compatibility contract's caller matrix.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::nonequilibrium::tests`; tests verify active coefficient behavior and that dormant pool-update modes cannot be mistaken for active parity. Cross-language parity is gated to 18.10.1.

### 18.6.3 — Port active dual-domain coefficients

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/dual_domain.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.6.3.todo.md`.
- **Depends on:** 18.4.1; use the stable shared process/mode interfaces declared there. Do not edit `processes.rs` or `nonequilibrium.rs`.
- **Spec:** Port active `iDualPor` transport/storage coefficient behavior and material selection. `lDualNEq` pool-update helper calls are commented on the audited call path; preserve/document their inactivity rather than porting unreachable state transitions as runtime behavior. This is separate from active mobile/immobile coefficients in 18.6.2. Report any active external-state mapping gap for amendment.
- **Do not touch:** shared types, transport/boundary/reaction/nonequilibrium modules, public API, fixtures, or Fortran source.
- **Anchors:** `SOLUTE.FOR` `Coeff` dual-domain paths; `INPUT.FOR` caller-derived `iDualPor`; commented `NEquil` updater; domain fixtures.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::dual_domain::tests`; source-anchored tests cover active domain coefficient behavior and guard against claiming dormant pool updates. Cross-language parity is gated to 18.10.1.

### 18.6.4 — Port bacteria and filtration branches

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/biology.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.6.4.todo.md`.
- **Depends on:** 18.4.1; use the stable shared process/mode interfaces declared there.
- **Spec:** Port source-present `lBact`, `lFiltr`, and their participating `Coeff`/`MassTran` terms for valid mapped configurations. Include exact source branch and parameter provenance. Deposition/blocking/exclusion are separate in 18.6.5. If an active branch needs external data/coupling not expressible in the normalized contract, report it for plan amendment.
- **Do not touch:** shared types, transport/boundary/reaction/nonequilibrium/dual-domain/deposition modules, public API, fixtures, or Fortran source.
- **Anchors:** `SOLUTE.FOR` `Coeff` 372–541; process/biology fixture group.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::biology::tests`; source-anchored tests cover assigned bacteria/filtration equations or stop for explicit scope amendment. Cross-language parity is gated to 18.10.1.

### 18.6.5 — Port deposition/blocking; preserve exclusion as inactive

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/deposition.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.6.5.todo.md`.
- **Depends on:** 18.4.1; use shared process/biology interfaces declared there.
- **Spec:** Port active `Blocking`/`Deposit` calls and their `Coeff`/`MassTran` contributions, preserving branch conditions, parameter provenance, operation order, and state updates. `Exclusion` is not called from `HYDRUS` and remains explicitly inactive; do not expose it as a compatibility mode. Do not edit `biology.rs`; any shared interface change requires a plan amendment before execution.
- **Do not touch:** shared types, other numerical modules, public API, fixtures, or Fortran source.
- **Anchors:** active `SOLUTE.FOR` `Blocking` 1561–1594 and `Deposit` 1595–1660; commented `HYDRUS.FOR` `Exclusion` call; biology fixture group.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::deposition::tests`; tests cover active deposition/blocking and record `Exclusion` as unreachable. Cross-language parity is gated to 18.10.1.

### 18.7.1 — Port source-ordered linear solute step driver

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/driver.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.7.1.todo.md`.
- **Depends on:** Waves 5–6.
- **Spec:** Implement the source `Solute` linear-mode driver and exact call order for each species and both assembly levels, including source state inputs/outputs, coefficient → matrix → solve → update → accounting order, source casts and failure/termination diagnostics. Nonlinear iteration/state convergence is a separate next-wave task. Keep the source sequence even where Rust's existing transaction/step design differs; map external caller state explicitly.
- **Do not touch:** public `advance_solute_step` dispatch, source kernels owned by prior tasks, runner/comparator, fixtures, or Fortran source.
- **Anchors:** `SOLUTE.FOR` `Solute` 6–240 and calls to `Coeff`, `MatSet`, `BanSol`, `NEquil`, `MassTran`; all prior module tests and fixture matrix.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::driver::linear_tests`; source-anchored tests verify linear-mode call ordering and state transitions; every linear branch from 18.1.1 maps to a test. Full cross-language parity is gated to 18.10.1.

### 18.8.1 — Enforce active caller reachability for nonlinear mode

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/iteration.rs` (unit tests in the module).
- **Can create:** `planning/tasks/18.8.1.todo.md`.
- **Depends on:** 18.7.1.
- **Spec:** The source audit found active `ChemIn` initializes `lLinear=true` and its only `false` assignments are commented, so the active program cannot select nonlinear `Solute` iteration. Preserve that source behavior: derive the flag from source-reachable inputs and reject/mark unavailable a normalized request that attempts to force `lLinear=false`. Do not port the dormant nonlinear solver loop or claim it as parity. Keep the module as a narrow validation/trace guard; do not change the existing default.
- **Do not touch:** public `solute.rs` dispatch, other kernel modules, runner/comparator, fixtures, or Fortran source.
- **Anchors:** `INPUT.FOR` `ChemIn` 1348–1371 (`lLinear` initialization and commented assignments); `SOLUTE.FOR` `Solute` 85–180 nonlinear branch; normalized mode contract.
- **Acceptance:** `cargo test -p hydrus-core solute_fortran_compat::iteration::tests`; tests prove default mapping follows the active reader and explicit unreachable nonlinear mode is rejected/reported, not silently executed. No nonlinear Fortran parity claim.

### 18.9.1 — Expose compatibility candidate without changing default

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/lib.rs`, new solver-selection tests.
- **Can create:** `planning/tasks/18.9.1.todo.md`; a public explicit `FortranCompatible` selection path and explicit `FiniteVolumeBackwardEuler` alternative as specified by the contract.
- **Depends on:** 18.8.1.
- **Spec:** Integrate the new backend with an explicit solver selector/call path usable by the normalized Rust adapter. Leave the preexisting default unchanged during parity qualification. Preserve the full FV/BE path and route existing method-contract/analytical tests explicitly to FV/BE where needed, without weakening expectations. Reject unsupported/malformed source-mode inputs before state mutation.
- **Do not touch:** compatibility kernels, Fortran fixtures/source, parity limits, analytical evaluator, or prior evidence artifacts.
- **Anchors:** `solute.rs` public advance entry points, `lib.rs`, existing solute integration tests, normalized contract from 18.1.1.
- **Acceptance:** `cargo test -p hydrus-core --test solute_solver_selection` and `cargo test -p hydrus-core --tests`; compatibility candidate is callable, current default remains FV/BE, explicit FV/BE results/contracts remain unchanged.

### 18.10.1 — Verify all-mode parity and publish compatibility evidence

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `planning/master/VALIDATION_STRATEGY.md`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`.
- **Can create:** `planning/tasks/18.10.1.todo.md`; `planning/master/SOLUTE_FORTRAN_PARITY_18.md`; task-local outputs under `.development/18.10.1/`.
- **Depends on:** 18.9.1 and all fixture/kernel tasks.
- **Spec:** Run the complete fixture suite for every mode reachable through the audited active caller path, serially, using the pinned executable and explicitly selected Rust compatibility candidate; optionally report FV/BE separately. Authenticate source/input/build/output identities; compare normalized inputs, internal checkpoints, mapped outputs, and branch coverage with the accepted per-field policy. Calibrate/verify tolerances from repeatability and precision evidence; preserve failures and follow worker escalation rules. Update evidence docs with achieved active modes, inactive helper paths, and separate scientific gaps. This task passes only when every in-scope active branch meets criteria; unresolved failures remain blockers requiring corrective amendments. Do not claim physical/continuum validity.
- **Do not touch:** pinned source/patch, canonical old fixtures/artifacts, tolerance policy without evidence, solver kernels unless a plan amendment scopes a corrective task, or unrelated roadmap docs.
- **Anchors:** full compatibility contract and all-mode fixture manifests; current runner/comparator; `VALIDATION_STRATEGY.md`; printed parity, analytical, and convergence evidence; branch-by-branch TODO reports.
- **Acceptance:** `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group all --backend fortran-compatible --task-root .development/18.10.1` passes every active/callable branch, supported interaction, and mapped output against approved field-specific limits; manifests authenticate source/build/input/output identities; explicit FV/BE regressions pass; the report enumerates inactive/unreachable source code without claiming it as parity. The orchestrator serially reruns all global suites. Any failing/unmapped active mode blocks completion and the default switch.

### 18.11.1 — Switch default to verified compatibility backend

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/lib.rs`, solver-selection/default regression tests.
- **Can create:** `planning/tasks/18.11.1.todo.md`.
- **Depends on:** accepted 18.10.1 only.
- **Spec:** Change the existing default entry point to select the source-compatible backend based on the completed all-mode parity evidence. Keep FV/BE explicitly selectable and preserve its full existing test coverage. Record the default change and evidence pointer in public API docs. Do not change numerical formulas, comparison thresholds, or FV/BE behavior here.
- **Do not touch:** compatibility kernels, Fortran source/fixtures, parity evidence artifacts, or unrelated API modules.
- **Anchors:** accepted `SOLUTE_FORTRAN_PARITY_18.md`; solver-selection code/tests from 18.9.1; public solute entry points.
- **Acceptance:** default-selection test proves `advance_solute_step` uses the verified Fortran-compatible backend; opt-in FV/BE tests remain unchanged and pass. Run `cargo test -p hydrus-core --test solute_solver_selection`, `cargo test -p hydrus-core --tests`, and `git diff --check`.

## Approval and handoff

Approval authorizes the plan, not an unsupported claim that every mode already has parity. The explicit compatibility candidate is first compared while the existing default remains intact. Only accepted task 18.10.1 evidence unlocks default switch task 18.11.1. If any audited source branch requires edits outside its task scope or another Fortran subsystem, amend this plan rather than widening permissions ad hoc. After this solute milestone, continue the larger Rust port with the next Fortran feature area; do not treat this as complete whole-program 1:1 parity.

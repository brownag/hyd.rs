# Orchestration 13 — Solute convergence and independent analytical evidence

Status: **User approved and execution started on 2026-09-26; see STATE_13.md.**
Companion files: [worker template](PROMPT_TEMPLATE_13.md), [state](STATE_13.md).

## Goal and scope

Develop controlled spatial and temporal convergence evidence for Rust and the pinned Fortran solute implementation, independently verify selected trajectories and mass terms, and use that evidence to decide which numerical error bounds and regression thresholds are justified. Follow the six-phase structure discussed with the user. Target the selected constant-water, linear-equilibrium-sorption, sequential-reaction M5 problem and its derived finite-domain Example 3 fixture. Research and derivation are the first phase of this orchestration, not a separate open-ended prerequisite.

User input, verbatim:

> what will it take to develop convergence and indepedent analytical evidence. is additional research necessary? or can we create next orchestration plan 13 directly to address that?

> $plan-wave create orchestration 13 using this structure

The referenced structure is: (1) benchmark equations/boundaries/error measures; (2) mesh/timestep-capable fixture harness; (3) independent exact-solution benchmarks; (4) spatial/temporal convergence for both implementations; (5) investigate persistent discrepancies; (6) error budgets, threshold review, and M5-V decision.

**In scope:** focused primary-source research and mathematical derivation; reaction-only chains including sorbed storage; finite-domain diffusion with compatible boundaries; advection–dispersion–reaction with the actual inlet/outlet law; an independent transient finite-domain three-species Example 3 calculation; reproducible case generation and isolated runs; separate mesh and timestep studies; independent concentration, inventory, boundary, reaction and balance observables; rounding/precision/endpoint/quadrature budgets; zero/near-zero reporting; evidence-backed regressions and an explicit achieved/unmet decision.

**Out of scope:** recovering original manual decks as a prerequisite; manual reproduction claims; arbitrary manufactured volumetric-source hooks or new public solver APIs; changing water content, nonlinear/kinetic-sorption verification, new physics, Phase 6, platform/compiler matrices, performance claims, experimental validation, or universal HYDRUS release compatibility. Core fixes discovered during diagnosis require a concrete amendment with file scope and regression before implementation. Existing source/fixture/history artifacts remain immutable. No new dependency is preauthorized; a numerical library needed by the mathematical method requires an explicit plan amendment before use.

## Decisions and implementation judgments — 2026-09-26

- **User direction:** create Orchestration 13 using the six phases above; preserve Orchestration 12's completed evidence and open M5-V gate.
- **Inherited constraints:** preserve the selected boundary/process contract, source snapshot and portability patch; keep source parity, mathematical code verification and physical validation separate.
- **Approved planning choice:** exact cases supported by current APIs come first. A manufactured solution requiring a forcing hook is deferred, not silently approximated by existing reaction terms.
- **Approved planning choice:** begin the derived-case mesh study at 251/501/1,001 nodes and vary timestep independently. These are starting levels, not a promise that three levels reach the asymptotic regime.
- **User decision:** “Keep existing limits and report any measurement floor.” Retain the current Fortran array capacity and output formats. Identified baseline/promoted-real arithmetic modes remain available for sensitivity analysis; neither increases capacity or printed precision. Capacity/output-format variants are outside this plan. Report their measurement limits honestly; do not treat them as permission to alter the build.
- **Approved planning choice:** keep the core dependency-free and use Python standard-library validation tooling initially. Wave 1 must demonstrate that the selected independent evaluator can meet an explicit reference-error budget within this constraint or report the precise method/dependency gap.
- **Approved planning choice:** Wave 2's three adapters may run in isolated worktrees, at most three workers; other waves have one task. Shared Cargo/global suites run serially under the orchestrator. Choose the cheapest capable worker; use a stronger worker for derivation when needed, with independent orchestrator mathematical review.
- **Scientific acceptance:** planning approval approves the protocol, not unknown numerical thresholds. The orchestrator reviews actual budgets before activating thresholds. Evidence may leave thresholds unassigned and M5-V open; this is a truthful review outcome, not scientific completion.

## Reconnaissance and baseline

Repository: `/home/andrew/AI/hyd.rs`; branch `phase4-water-acceptance`; baseline commit `f86a028`. No applicable AGENTS.md or CONTRIBUTING file was found. Working tree was clean before planning. Current checks passed: 130 Rust tests; 13 Python runner/outlet tests; 12 comparator self-checks; formatting, Clippy with warnings denied, and diff whitespace checks. See STATE for exact commands.

| Existing file | Lines | Entry anchors / use |
|---|---:|---|
| `crates/hydrus-core/src/solute.rs` | 1474 | `SoluteTransportInput`, `SoluteProcessInput`, `update_step_balances`, `assemble_and_solve_species_with_processes`, `apply_endpoint_boundary_rows`; read-only equation/API audit |
| `crates/hydrus-core/tests/solute_fixture_parity.rs` | 413 | `widths`, `inventory`, `resolved_output_path`, `observation_json`, `source_mapped_example3`; adapter reference |
| `tools/compare_solute_reference.py` | 770 | `verified_paths`, `parse_node_profiles`, `parse_solute_output`, `parse_balance`, `inventory_from_profile`, `residual_metrics`; strict parsing/accounting reference |
| `tools/run_fortran_reference.py` | 314 | `run_case`, `validate_case_outputs`; lifecycle reference, not an experiment runner |
| `tools/stage_fortran_reference.py` | 133 | `checked_output`, `main`; immutable staging |
| `tools/build_fortran_reference.sh` | 88 | explicit build directory and recorded GNU compiler flags |

Hazards that every affected task must preserve:

- The existing runner overwrites `.development/12.2.1` and fixture RUN.md. Never invoke its case mode for new studies. Existing Rust/comparator defaults also target Orchestration 12 scratch; use new study adapters and paths.
- Fortran `HYDRUS.FOR` declares `NumNPD=1001` near line 48. Do not exceed it or silently edit source. Output rounding may mask refinement even below this cap.
- Legacy paths contain literal backslashes (`solute-input\\Selector.in`); preserve executable permissions when copying binaries. Existing marker substring checks are insufficient: validate exact terminal records and complete record inventories.
- Example 3 uses native x=-250 at the outlet and x=0 at the inlet, theta=0.43, q=-0.43, D=0.18, rho=0.8, Kd=(0.5375,0,0), rates=(0.005,0.1,0), inward inlet flux=(0.43,0,0). Source-reconstruct and verify realized water for every new case; do not feed measured solute flux back as forcing.
- Rust uses backward Euler and conservative endpoint half-cells. Fortran uses the selected old/new weighting and a zero-gradient row replacement. Match the continuum problem, document discrete differences, and derive expected orders per case; do not demand second order globally.
- Printed Fortran Sorb columns are inactive nonequilibrium pools in this fixture. Equilibrium storage is Kd*C; never count both as equilibrium mass. Chain production/loss must be integrated independently, never inferred from balance closure.
- Changing-theta behavior has an existing documented limitation and is outside this constant-water study. Concentration-boundary exchange inferred by the core is not independent evidence of conservation.
- Old printed-precision repeatability and one promoted-real run establish neither convergence nor accuracy. Preserve underflow/denormal warnings and distinguish arithmetic precision from output precision.

Primary research seeds for Wave 1: NASA's [spatial convergence tutorial](https://www.grc.nasa.gov/www/wind/valid/tutorial/spatconv.html), Sandia's [manufactured-solution verification report](https://www.osti.gov/biblio/759450), local manual/source anchors in `planning/master/SCIENTIFIC_COMPARISON_RESEARCH.md`. These are methodology leads; the finite-domain chain formula still requires derivation and review.

## Task DAG and waves

All task IDs are new and initially pending. No task TODO is pre-completed.

| Wave / phase | Task | Deliverable | Depends on |
|---|---|---|---|
| 1 — Mathematical specification | 13.1.1 | Reviewed benchmark equations, case/schema contract and bounded study matrix | — |
| 2 — Reusable study harness | 13.2.1 | Isolated Fortran case generation, builds and execution | 13.1.1 |
| 2 — Reusable study harness | 13.2.2 | Configurable Rust observation adapter | 13.1.1 |
| 2 — Reusable study harness | 13.2.3 | Strict parameterized normalization and independent metrics | 13.1.1 |
| 3a — Component references | 13.3.2 | Reaction, diffusion and scalar evaluators | 13.2.1, 13.2.2, 13.2.3 |
| 3b — Finite-chain reference | 13.3.3 | Decimal transforms, inversion and admission | 13.3.2 |
| 3c — Benchmark tooling | 13.3.4 | Bounded launcher, bridge checks and mocked tests | 13.3.3 |
| 3d — Analytical evidence | 13.3.1 | Activated bridge/benchmark comparisons and report | 13.3.4 |
| 4 — Controlled convergence | 13.4.1 | Separate spatial/temporal studies for both implementations | 13.3.1 |
| 5 — Discrepancy investigation | 13.5.1 | Attribution, counterexamples, and bounded remediation recommendations | 13.4.1 |
| 6 — Error budgets and decision | 13.6.1 | Reviewed threshold policy, regressions and M5-V evidence assessment | 13.5.1; any accepted remediation |

Wave 2 files are pairwise disjoint. Its common schema is frozen in Wave 1; schema changes require returning to that contract before workers proceed. Use worktrees with task-local scratch and Cargo targets. Workers return reports; the orchestrator merges them into STATE serially to avoid a shared-file collision. All later waves are serial.

## Common execution contract

Each task belongs to this repository. File scope below is exhaustive: listed existing files are read-only unless named under Must edit; Can create authorizes later edits to those same outputs during that task. Each task may create/edit only its `planning/tasks/<id>.todo.md` and `.development/<id>/` scratch in addition to its listed files. Existing `target/` is permitted for serial Cargo checks; parallel Cargo work uses its task scratch target. Workers never edit the plan, board or another task's report. Reports are at most 15 lines and integrated by the orchestrator.

A worker first writes a micro-plan of at most 12 genuinely small checkbox steps and stops. If a task (especially 13.3.1) cannot fit honestly, split it into dependency-ordered tasks before approval; do not hide multiple implementation clusters inside a single checkbox. Only `APPROVED — execute the plan as written.` from the orchestrator authorizes execution. Discovered work goes under `## Discovered`; scope is amended before expanded implementation. New commands below are prospective acceptance interfaces, created by their producer tasks; do not claim they run today.

All new execution commands take an explicit output root confined to the executing task's scratch, create unique run IDs, refuse to overwrite a run, and record starts/exits/timeouts/completion in an append-only ledger. Default to dry-run/verification unless `--execute` is supplied. A matrix approved by the orchestrator must enumerate every launch, timeout, precision mode, output size estimate and total run cap; failed attempts consume the cap. No unbounded automatic retries or refinement. Initial executable timeout ceiling is 60 seconds per run; changing it requires a recorded matrix revision. `--verify-only` must launch no solver. Hash and preserve original source/patch/Example 3 and accepted Orchestration 12 artifacts before/after numerical tasks. Raw legacy output and reference-derived arrays remain scratch-only; commit recipes, provenance summaries and independently authored mathematical expectations.

Preservation exception for orchestrator regression output only: the legacy parity test confines its output to `.development/12.6.1/`. Before every workspace-suite invocation, set `HYDRUS_SOLUTE_PARITY_OUT` to a fresh, nonexistent `.development/12.6.1/13-regression/<unique-id>.json` path and record it. Never reuse a path or overwrite the original `rust-observations.json`. This narrow new subdirectory is writable only by the orchestrator; existing accepted files remain protected.

The orchestrator independently reruns targeted checks, then serially runs the baseline suite with that output override plus newly introduced Python test modules at wave end. Inspect diffs, accept one scoped task per conventional local commit, never push. Apply the escalation ladder in PROMPT_TEMPLATE_13.md. A green test suite is not authority to relabel incomplete scientific evidence.

## 13.1.1 — Derive and review the benchmark contract

**Must edit:** none.
**Can create:** `planning/master/SOLUTE_VERIFICATION_SPEC.md`; `reference/solute_verification/CASES.json`; `reference/solute_verification/README.md`; `tools/check_solute_verification_spec.py`.
**Depends on:** none.
**Read-only references:** `planning/master/{SCIENTIFIC_COMPARISON_RESEARCH,VALIDATION_STRATEGY,SOLUTE_OUTLET_CONTRACT,PHASE_5_SOLUTE_EVIDENCE}.md`; Example 3 CASE/README; `solute.rs` anchors above; `SOLUTE.FOR` Coeff/Disper/species sequence/outlet rows; `INPUT.FOR` profile reversal and chemical inputs; `OUTPUT.FOR` TLInf/SubReg/NodOut. Use the existing outlet contract as the model for explicit equations and independent examples.

**Spec:** Conduct focused primary-source research and derive the continuous initial/boundary-value problems. Define species storage factors and transfer coefficients with correct unequal-retardation coupling, orientation, dimensions, initial compatibility and terminal loss. Include (a) homogeneous reaction chains with sorbed storage and equal/zero-rate limits, (b) finite-domain diffusion with executable compatible boundaries, (c) scalar advective–dispersive–reactive inlet/outlet evidence, and (d) the actual transient finite-domain three-species derived Example 3 problem. A semi-infinite formula is insufficient for (d) unless a quantitative finite-domain error bound is established. Derive independent inventories, instantaneous/cumulative endpoint fluxes and reaction integrals, not only C.

Specify an independently implementable analytic/semi-analytic method, truncation/roundoff control, limiting identities and evaluator cross-checks without borrowing either solver's stepping matrix. Predeclare spatial/temporal norms, weights, expected orders where derivable, time alignment, treatment of nonsmooth initial/inlet data, zero/near-zero metrics and study/holdout split. Derive a feasible Fortran hydraulic/chemical mapping for each case, including zero-flow diffusion's actual effective D. Unsupported mappings must be named, not silently run as another equation.

Freeze a versioned JSON case/observation/manifest contract in the spec and CASES.json before Wave 2. Include physical parameters, initial/boundary laws, node/time variations, species, units, output schema, solver identity and provenance. Define a finite baseline matrix with at least three levels per refinement axis, starting at 251/501/1001 for the derived spatial case; fix the other axis tightly enough to test contamination and include a control refinement. Reserve independent holdout levels/times/cases for final validation. Set launch/storage budgets after estimating case cost. CASES must classify each case as executable or explicitly blocked, with a reason.

**Acceptance:** checker rejects malformed cases, unsupported node counts, nonpositive dt, mismatched times/species, absent provenance or missing metric definitions; own negative self-tests pass. Orchestrator independently checks equations/BC/IC, at least one chain limit, and method feasibility; writes the review outcome in STATE. A blocked full-case derivation may be recorded but does not satisfy full-case scientific success; downstream tasks may use only accepted cases and must retain this gap.
**Targeted commands:** `python3 tools/check_solute_verification_spec.py --self-test`; `python3 tools/check_solute_verification_spec.py reference/solute_verification/CASES.json`; `git diff --check`.
**Do not touch:** core, legacy source, previous fixtures/tools, or thresholds. No solver launches in this task.

## 13.2.1 — Build the isolated Fortran study adapter

**Must edit:** none.
**Can create:** `tools/run_solute_verification_reference.py`; `tools/test_solute_verification_reference.py`.
**Depends on:** 13.1.1.
**Read-only references/anchors:** accepted verification spec/CASES; existing stage/build scripts; `run_fortran_reference.py:run_case`; Example 3 SELECTOR/PROFILE/CASE; source INPUT/OUTPUT anchors; compare parser's `verified_paths`. Pattern-match provenance and preserve the accepted portability patch.

**Spec:** Generate decks from accepted case parameters and refinement settings without editing the frozen fixture. Stage/build in unique scratch directories, support only the two identified arithmetic modes (baseline and promoted-real), capture exact flags/source/shim/executable hashes, and retain legacy filename and executable-mode behavior. Reject unsupported capacity and unreviewed source changes. Validate each requested output inventory, terminal record, observation/step coverage and water history before identifying a completed run. Emit the Wave 1 manifest schema; never use output values as future forcing.

**Acceptance:** unit tests cover deterministic generation, both coordinate orientations where cases require them, chemical switches, step scheduling, path escape/symlink protection, existing-run refusal, stale/missing/truncated output, timeout/nonzero exit, hash mismatch and launch ledger accounting. Tests mock execution; default invocation never launches Fortran. Real adapter bridge executions belong to Wave 3 after all adapters exist.
**Targeted commands:** `python3 -m unittest tools.test_solute_verification_reference`; `python3 tools/run_solute_verification_reference.py --cases reference/solute_verification/CASES.json --dry-run --output-root .development/13.2.1/preview`; `git diff --check`.
**Do not touch:** existing runner/stager/build scripts, src_reference, portability patch, CASES schema, Rust or parser task files. No real Fortran launches.

## 13.2.2 — Build the configurable Rust observation adapter

**Must edit:** none.
**Can create:** `crates/hydrus-core/tests/solute_verification_harness.rs`.
**Depends on:** 13.1.1.
**Read-only references/anchors:** accepted spec/CASES; `solute_fixture_parity.rs:widths/inventory/observation_json/source_mapped_example3`; `solute.rs` public inputs/configuration/advance functions; reaction and outlet tests.

**Spec:** Add a test-only adapter for accepted constant-water cases using current public APIs, with explicit validated case parameters, node count, dt, initial state and observation schedule. Use a simple documented input channel consistent with the Wave 1 contract, without a new Cargo dependency. Emit the frozen JSON observation schema and request identity/hash, actual accepted times, dissolved profiles, endpoint histories and solver diagnostics. Per the accepted Wave 1 schema, the metrics task derives equilibrium storage and independent boundary/reaction integrals from these observations; the adapter keeps solver accounting explicitly separate. This resolves the original prose without adding observation keys. Handle observation endpoints without accumulated-time mismatches or silently changing dt; reject unsupported cases. With no study request, run small deterministic adapter tests and create no large output artifact.

**Acceptance:** adapter tests cover parameter validation, species ordering, coordinates, half-cell weights, event alignment, zero flow, parent-only inlet, malformed/nonfinite requests and output-path confinement. A default-size Example 3 request can reproduce existing Rust observations; its bridge comparison occurs in Wave 3. Core behavior is unchanged.
**Targeted commands:** `cargo test -p hydrus-core --test solute_verification_harness`; `cargo fmt --check`; `git diff --check`.
**Do not touch:** core/lib/Cargo files, existing fixture parity test or other Wave 2 outputs. Avoid shared targets during parallel work.

## 13.2.3 — Parameterize strict observations and independent metrics

**Must edit:** none.
**Can create:** `tools/solute_verification_metrics.py`; `tools/test_solute_verification_metrics.py`.
**Depends on:** 13.1.1.
**Read-only references/anchors:** accepted spec/CASES; `compare_solute_reference.py:parse_node_profiles/parse_solute_output/parse_balance/inventory_from_profile/residual_metrics`; source OUTPUT field assembly. Pattern-match strict parsing; do not inherit hard-coded 1001-node/2000-step/global scratch assumptions.

**Spec:** Parse the approved output schemas using manifest-declared node counts, times, species and field precision. Verify hashes and physical mapping before comparing. Implement common-coordinate norms, volume-weighted phase/total inventories, signed endpoint flux and cumulative reaction/boundary terms, independent balance and explicit normalization branches. Keep raw, derived and solver-reported quantities distinct. Separate continuum-integral error from mesh-quadrature error; record interpolation error if common-point evaluation cannot avoid interpolation. Reject incomplete or unvalidated observations rather than silently dropping them. No acceptance thresholds yet.

**Acceptance:** synthetic hand-computed tests cover unequal mesh weights, species isolation, signed endpoint fluxes, equilibrium versus inactive Sorb storage, zero/near-zero denominators, output rounding intervals, bad hashes, duplicate/missing times, nonfinite values, malformed columns and truncated profiles. With interspecies transfer disabled, altering one species cannot change another's balance result; active-chain tests separately verify the required parent-to-daughter production coupling. Parameterized layouts at multiple node counts pass.
**Targeted commands:** `python3 -m unittest tools.test_solute_verification_metrics`; `git diff --check`.
**Do not touch:** old comparator or other Wave 2 files; no solver launches.

## Wave 3 decomposition amendment — 2026-09-27

The reviewed worker micro-plan identified four independent implementation clusters that cannot fit twelve genuinely small steps. Execute serial subwaves 3a/3b/3c/3d below; no parallelism is allowed because evaluator/test files overlap. Preserve the original task 13.3.1 for numerical execution and evidence, so frozen execution roots, matrix phases and downstream task IDs remain unchanged. Added task IDs 13.3.2–13.3.4 are producer tasks, not added scientific scope. Each receives a separate micro-plan gate, verification and conventional commit. No new dependency, physics, build variant or solver launch is authorized by this decomposition.

### Shared analytical objective

**Spec:** Implement the reviewed analytic/semi-analytic evaluator independently of both transport solvers. Evaluate every accepted benchmark, including concentrations and independent mass/flux/reaction integrals. Quantify evaluator error through limiting exact identities, independent residual/BC/IC checks, tighter series/integration settings and alternate evaluations where conditioning demands them. Exact PDE formulas evaluated numerically still need an evaluator uncertainty estimate. Equal/near-equal decay rates, zero loss and disparate retardation must not cause cancellation to masquerade as solver error.

Integrate the three adapters using a bounded, orchestrator-approved bridge/benchmark matrix. First reproduce the original Example 3 configuration in new scratch and compare against retained accepted outputs within documented printed/arithmetic precision; fail on unexplained adapter differences. Then run accepted component/full-case benchmarks through both solvers, check realized water/mapping, and compare with the independently evaluated truth. Analytic expectations must never be fitted to solver outputs. Record unsupported full-case mathematics explicitly; component passes cannot replace it.

## 13.3.2 — Implement component analytical references

**Must edit:** none.
**Can create:** `tools/solute_analytical_reference.py`; `tools/test_solute_analytical_reference.py`.
**Depends on:** 13.2.1, 13.2.2, 13.2.3.
**Read-only references:** frozen verification spec/CASES and Wave 2 reference wrapper; metrics `_reference`; outlet exact-example style; Wave 1 mathematical scratch.
**Spec:** Implement independently evaluated homogeneous stored-mass reaction chains (uniformization and integrated weights with tail bounds), finite cosine diffusion, and smooth scalar Robin/Neumann transport. Cover unequal storage, equal/near-equal/zero rates, initial daughters and terminal loss. Use independent spatial/time integrals for inventories, endpoint exchange and reactions. Recompute the scalar eigenvalue and bound formula/root/serialization errors. Return the frozen reference wrapper with explicit per-observable uncertainty/admission; unsupported finite-chain requests remain explicitly unavailable until the next task. Do not borrow either solver's matrix or fit outputs.
**Acceptance:** exact and limiting identities, IC/BC/PDE residuals, dimensions, complete mesh/species/time coverage, independent integrals and uncertainty budget tests pass; altered species/boundary sign is detected.
**Targeted commands:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_analytical_reference`; `git diff --check`.
**Scratch:** `.development/13.3.2/`; own `planning/tasks/13.3.2.todo.md` only. No solver launches or existing adapter/core/case edits.

## 13.3.3 — Implement the finite-domain chain evaluator

**Must edit:** `tools/solute_analytical_reference.py`; `tools/test_solute_analytical_reference.py`.
**Can create:** none beyond own TODO/scratch.
**Depends on:** 13.3.2.
**Read-only references:** frozen full-chain derivation, Wave 1 feasibility recipe/results, accepted component evaluator.
**Spec:** Add stable Decimal H/I transforms, confluent and near-confluent divided differences, cached Gaver-Stehfest inversion and independently transformed mass/flux/reaction integrals. Preserve unequal retardation and zero limits. Implement the frozen bounded orders/precision, full profile and diagnostic-integral coverage, direct-versus-recurrence checks, decreasing-order-difference checks and honest per-observable admission. Report empirical uncertainty as empirical; inability to meet a budget remains unmet. No alternative method/dependency or expanded work cap without amendment.
**Acceptance:** independent transform/PDE/BC identities, confluent/zero limits, tighter precision and documented finite-work feasibility checks pass. Orchestrator reviews evaluator independence and uncertainty before numerical reference admission. This task does not assert solver accuracy.
**Targeted commands:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_analytical_reference`; `git diff --check`.
**Scratch:** `.development/13.3.3/`; own `planning/tasks/13.3.3.todo.md` only. No solver launches or edits to prior artifacts/adapters/cases.

## 13.3.4 — Implement bounded benchmark integration

**Must edit:** none.
**Can create:** `tools/check_solute_benchmarks.py`; `tools/test_solute_benchmarks.py`.
**Depends on:** 13.3.3 and all Wave 2 tasks.
**Read-only references:** accepted evaluator, adapters/metrics, frozen manifest/ledger/reference schemas, retained Example 3 provenance.
**Spec:** Implement the benchmark CLI and safe Rust launcher/manifest wrapping, reuse the accepted Fortran runner, and connect independent references/metrics. Enforce unique roots, explicit --execute, no-launch verify-only, all-identity hashes, append-only shared phase ledger, failed-attempt budget consumption, 60-second solver timeouts and storage caps. Compile separately from counted solver runs. Bridge checks must reproduce retained original configurations and justify printed/arithmetic comparison; stop before dependent launches on unexplained differences. Preserve exact reference admission and scientific unmet results. Make mocked lifecycle and deliberately perturbed species/order/boundary checks independent of solver launches.
**Acceptance:** mocked launch/timeout/failure/provenance/bridge tests pass; dry-run enumerates exactly the fourteen frozen benchmark rows and reserved caps; verify-only executes no solver. Orchestrator reviews concrete commands and activates runs separately.
**Targeted commands:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_benchmarks`; `python3 tools/check_solute_benchmarks.py --cases reference/solute_verification/CASES.json --dry-run --output-root .development/13.3.1/benchmarks`; `git diff --check`.
**Scratch:** `.development/13.3.4/` for mocked/unit work; dry-run recipe output under the reserved `.development/13.3.1/benchmarks` root is also permitted. Own `planning/tasks/13.3.4.todo.md`. No real solver launches or existing producer edits without amendment.

## 13.3.1 — Execute and review analytical benchmark evidence

**Must edit:** none.
**Can create:** `planning/master/SOLUTE_ANALYTICAL_EVIDENCE.md`.
**Depends on:** 13.3.4, with 13.3.2/13.3.3 reference review accepted.
**Read-only references:** all accepted Wave 3 producers and original analytical objective above.
**Spec:** After explicit orchestrator activation, execute the two bridge rows first in fresh scratch, review their retained-output comparisons, then the remaining twelve benchmark rows. Every failed attempt consumes its frozen slot; no retries or extra launches without a recorded bounded amendment. Independently admit analytical observables before claiming accuracy; compare complete concentration, inventory, flux, reaction and balance evidence. Report hashes, commands, budgets, achieved/unmet cases, reference limits and outstanding scientific gates. Fixes to producer tools require an explicit scoped amendment before edits.
**Acceptance:** at least one nontrivial case per accepted family runs through both adapters with validated provenance and complete records; bridge identity and independent reference admission are reviewed. Unmet full-case mathematics/uncertainty remain explicit and cannot be replaced by component passes. Orchestrator independently verifies outputs and report before convergence.
**Targeted commands:** `python3 tools/check_solute_benchmarks.py --cases reference/solute_verification/CASES.json --execute --output-root .development/13.3.1/benchmarks`; same command with `--verify-only`; `git diff --check`.
**Scratch:** `.development/13.3.1/`; own `planning/tasks/13.3.1.todo.md`. No solver/core/source/case/dependency changes or threshold assignment.

## 13.4.1 — Measure spatial and temporal convergence separately

**Must edit:** none.
**Can create:** `tools/run_solute_convergence.py`; `tools/test_solute_convergence.py`; `planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md`.
**Depends on:** 13.3.1.
**Read-only references/anchors:** accepted spec/matrix, evaluator and all study adapters; analytical evidence; prior precision review. Pattern-match the benchmark command's unique run IDs, ledger and verify-only behavior.

**Spec:** Run the bounded matrix for both implementations. Vary dx with dt controlled, then dt with dx controlled, with at least three accepted levels per axis and contamination control. Report actual step histories, Courant/Peclet diagnostics, norms at common times, endpoint/interior errors, inventory, flux, reaction and balance errors per species. For smooth nonzero errors compute observed rates against independent truth; use self-convergence only as supplementary evidence. Flag nonmonotonic errors, sign changes, zero errors, output/arithmetic floors and failure to reach the asymptotic regime rather than forcing a fitted order.

Study the existing derived Example 3 trajectory as well as component cases. Distinguish initial/inlet discontinuity effects, spatial upstream weighting, endpoint row replacement and temporal quadrature. Three grids alone are not acceptance; controlled error decrease and reference uncertainty must support an interpretation. If additional runs or refinement within the existing capacity are needed, propose a bounded matrix revision before launching them. If capacity or printed precision prevents a defensible result, quantify and report that measurement floor as an out-of-scope limitation, honoring the user’s decision; do not request or perform those build changes as routine continuation. Keep holdout data out of threshold selection.

**Acceptance:** synthetic power-law data recover known orders; plateau, mixed-error, zero and nonmonotonic examples are flagged. Every study row traces to complete hashed runs and independent reference settings. Report per-axis/per-solver achieved/inconclusive/failed classifications with evidence and budget consumption. Scientific convergence succeeds only where evidence supports it; an inconclusive study remains a gate gap.
**Targeted commands:** `python3 -m unittest tools.test_solute_convergence`; `python3 tools/run_solute_convergence.py --cases reference/solute_verification/CASES.json --execute --output-root .development/13.4.1/study`; same command with `--verify-only`; `git diff --check`.
**Do not touch:** adapters, evaluator, core, thresholds or old runs without amendment. Do not co-refine dx and dt and call it two independent studies.

## 13.5.1 — Attribute persistent discrepancies and specify remedies

**Must edit:** none.
**Can create:** `tools/analyze_solute_discrepancies.py`; `tools/test_solute_discrepancies.py`; `planning/master/SOLUTE_DISCREPANCY_REVIEW.md`.
**Depends on:** 13.4.1.
**Read-only references/anchors:** analytical/convergence reports and raw manifests; `solute.rs` assembly/accounting/boundary anchors; Fortran Coeff/outlet/OUTPUT source anchors; original precision review. Use outlet-contract reasoning as the pattern for independent small counterexamples.

**Spec:** Classify residuals as resolved truncation/quadrature effects, evaluator/output/rounding limits, mapping defects, implementation defects, or unresolved. Support each attribution with evidence; correlation with refinement alone does not identify a bug. Isolate endpoint versus interior and storage versus flux/reaction contributions, preserving source-model differences. Produce small independently computed counterexamples where a defect is suspected. Prefer read-only analysis of existing runs; any extra launches require a bounded matrix amendment first.

For each reproducible defect, write a concrete proposed remediation with exact files, causal explanation, regression expectation and the minimum affected benchmark/refinement reruns. The orchestrator must insert scoped task(s)/wave(s) and approve micro-plans before implementing. New physics/public API/dependency/build variants require a user scope decision; bounded fixes to the already selected contract can be concretely amended within the approved investigation objective. Do not invent an unconditional solver-edit task before a defect exists.

**Acceptance:** analysis checker verifies every attribution references existing hashes and a reproducer or an explicit unresolved reason; all relevant discrepancy classes receive an entry. Synthetic tests detect missing evidence and inconsistent classifications. An unresolved result is permissible for this review but cannot justify a threshold that hides it. Wave 6 waits for accepted remediation/rechecks when such work is undertaken.
**Targeted commands:** `python3 -m unittest tools.test_solute_discrepancies`; `python3 tools/analyze_solute_discrepancies.py --study .development/13.4.1/study --output-root .development/13.5.1/review`; `git diff --check`.
**Do not touch:** solver, source, prior evidence, or tolerance values. No fixes or fresh solver launches in this unamended task.

## 13.6.1 — Review budgets, freeze justified gates, and assess M5-V

**Must edit:** `planning/master/SCIENTIFIC_COMPARISON_RESEARCH.md`; `planning/master/VALIDATION_STRATEGY.md`; `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`.
**Can create:** `reference/solute_verification/TOLERANCES.json`; `tools/check_solute_verification_acceptance.py`; `tools/test_solute_verification_acceptance.py`; `planning/master/SOLUTE_ERROR_BUDGET.md`.
**Depends on:** 13.5.1 and any accepted inserted remediation/rechecks.
**Read-only references/anchors:** all new evidence/spec/cases; prior tolerance-policy and M5 exit-assessment sections; prior CASE/RUN provenance. Preserve historical results as historical, not overwritten claims.

**Spec:** Build per-field/species/time-regime error budgets separating spatial and temporal truncation, endpoint treatment, integration/quadrature, reference-evaluator uncertainty, arithmetic sensitivity, text rounding and mapping uncertainty. State which terms are bounded and which are empirical estimates, and avoid double counting. Define dimensional absolute and relative criteria only where those budgets support them. Exact-zero relative balance remains undefined; near-zero transition requires a derived scale with explicit units. Paired-source tolerances and independent-analytic accuracy bounds are separate; neither demonstrates application fitness or experimental truth.

Store assigned/unassigned status, rationale, applicability, build identity, units, reference hashes, and exclusions in TOLERANCES.json. The orchestrator reviews proposed numerical bounds before they become gates. Exercise approved bounds against predeclared holdout cases/levels/times using a finite reviewed launch matrix; no fitting thresholds to holdout failures. Unassigned criteria must produce an explicit unmet status and nonzero scientific-acceptance exit, while policy/schema tests may pass. Add deterministic tests for accepted/rejected synthetic evidence, missing terms, altered provenance, zero/near-zero branches and out-of-domain use. Keep existing Orchestration 12 comparator diagnostic.

Update the three evidence documents with exact commands, artifact hashes, achieved/unmet criteria, unresolved defects and a reproducible handoff. M5-V closure requires the derived full finite-domain analytical trajectory, controlled both-code refinement, independently bounded observables, reviewed tolerances and passing holdout regressions for the selected scope. Component verification alone cannot close it. Broader nonlinear/kinetic/changing-water, physical-validation and release claims remain separate and unestablished.

**Acceptance:** policy tests pass; every assigned bound traces to evidence and a reviewed uncertainty budget; holdout checks pass or record a truthful scientific failure. All missing criteria remain unassigned/unmet. Evidence links and protected hashes verify. A reviewed unmet decision can complete this documentation task, but STATE must separately report the scientific objective as unachieved.
**Targeted commands:** `python3 -m unittest tools.test_solute_verification_acceptance`; `python3 tools/check_solute_verification_acceptance.py --policy reference/solute_verification/TOLERANCES.json --execute --output-root .development/13.6.1/holdout`; same command with `--verify-only`; `git diff --check`. Expected nonzero scientific exit for unmet criteria must be explicitly recorded, never called a passing acceptance result.
**Do not touch:** old comparator/fixtures, source/core, or unrelated roadmap/physics milestones. Never advance Phase 6 automatically.

## Overall outcome and review

The scientific goal succeeds only with independently justified full-case accuracy and convergence evidence plus accepted error budgets/regressions. A complete evidence review may instead identify precisely what remains missing. STATE tracks task completion and scientific completion separately.

Independent plan review completed on 2026-09-26: spec pass; standards pass after correction. Corrected the legacy parity-test overwrite hazard with an orchestrator-only fresh-output exception, and clarified that capacity/output-precision floors are reported within the user’s chosen limits. Both reviewers found no remaining blockers. Derivation feasibility and any scientifically unassigned thresholds remain execution judgments, not presumed successes. User approved the complete plan on 2026-09-26 (“approve”), including the existing Fortran capacity/output limits. Execution was subsequently started by the user’s wave-orchestration invocation; progress is recorded in STATE_13.md.

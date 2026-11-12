# Orchestration 25 — G19 first vapor increment

Status: **APPROVED by the user, 2026-11-11. Wave 1 complete; Wave 2 in progress.**
Approval covers the plan, draft source contract and coverage scope. Numerical
threshold approval and pre-execution model selection remain separate gates.
Created 2026-11-11. Repo: `/home/andrew/AI/hyd.rs` (`hyd.rs` below).
Resume: [STATE_25.md](STATE_25.md). Worker: [PROMPT_TEMPLATE_25.md](PROMPT_TEMPLATE_25.md).

## Goal and scope

**Goal.** Add vapor flow to the coupled water/heat capability with explicit
coupling contracts, rejected unsupported combinations, provenance-backed source
fixtures and capability-specific validation. Deliver a bounded first increment,
not a vapor heat-only approximation, and retain **all valid vapor capabilities**
as an explicit follow-on goal rather than treating the restricted slice as final.

**In scope:** `ConVapor`, `VaporContent` and hydraulic capacity, vapor water-row
terms, liquid/vapor velocities, `TempAdj`/`xLatent`, vapor-aware heat, two source
coupling passes, atomic state/accounting acceptance, vapor observation mapping,
kernel and full-executable evidence, independent/balance checks, acceptance and
the G22 coverage roadmap. Selection: model zero, one domain, default thermal
conductivity, **enhancement on**, fixed water heads and temperatures at both ends,
zero sink, no root uptake. Preserve accepted water/heat-only behavior.

**Out of scope:** remaining vapor modes are tracked in G22, not implemented here;
energy balance (`lEnBal`, `UpdateEnergy`, `Evap`, `Rn`, `Rns`, `Rnl`) is G20;
temperature-dependent liquid water (`lWDep`/`lWTDep`) is G23 and remains rejected.
No chemistry, hysteresis, dual porosity, root uptake, nonzero sink, atmospheric
switching, other hydraulic models, density/centrifugal physics, source fixes,
new dependencies, solute changes or M5 campaign/holdout work. Do not change pinned
source, patches, full-reference output formats/capacity, or existing evidence.
Do not redo G21: the van Genuchten exponent fix already shipped.

**Verbatim user constraints/input:**

- "@plan-wave circled goal from magic-wand"
- "yes, but it needs to be a goal to have all capanbilities. also, considering it touches water, do we need to address the vangenuchten se^1.5 vs 0.5 issue now?"
- G19 boundary: "vapor only. Energy balance (`lEnBal`, `UpdateEnergy`, `Evap`, `Rn`, `Rns`, `Rnl`) stays out. Heat-to-water feedback (`lWDep`) stays rejected unless separately approved. Hysteresis, dual porosity and root uptake with heat stay out."
- G19 exit criteria: "a vapor source contract (`TEMPER.FOR:48` `TempAdj`, `TEMPER.FOR:180-268`, vapor output `HYDRUS.FOR:174`) is recorded; the heat-only config accepts `lVapor=true` only for the supported vapor selection; source-matched fixtures exist for each supported vapor case; analytical or balance checks pass at tolerances approved for vapor; the acceptance document states what is not claimed."
- G19 first action: "write the vapor source contract (`planning/master/M6_VAPOR_CONTRACT.md`) by reading `TEMPER.FOR:180-268` (`TempAdj`), `TEMPER.FOR:48`, the vapor conductivity and content code in `WATFLOW.FOR` (`:919-922`, `:995`), and the vapor output in `HYDRUS.FOR:174`; list the vapor variables and their precision. Hand the contract to plan-wave for the execution plan."

The missing contract was authored during this planning session from pinned
source; it is a draft prerequisite artifact, not a completed implementation task.

## Decision log (user answers, 2026-11-11)

| ID | Decision and rationale |
|---|---|
| D1 | Include vapor conductivity/storage/flux contributions inside water, then vapor-aware heat; do not merely post-process heat. Initially lagged temperature; **refined by D9**. Keep `lWDep`/energy off. |
| D2 | First slice: default thermal conductivity, constant Dirichlet heat ends; initially enhancement-off was proposed, **corrected by D11**. Other vapor combinations rejected, heat-only support unchanged. |
| D3 | Record complete vapor coverage as G22 with a capability matrix and validation gates. Energy remains G20; liquid-water temperature dependence is separate G23. |
| D4 | Vapor path is atomic: water/heat/vapor and workspace roll back on any failure. Preserve existing heat-only failure behavior. |
| D5 | Fixed pressure heads at both water endpoints initially; remaining water boundaries tracked in G22. |
| D6 | Source-matched head/content, vapor content/flux and temperature plus independent kernel/balance evidence. Nonzero effects mandatory; user approves field-specific thresholds **before comparisons**. Never inherit/widen heat tolerances. D10 limits content to direct kernel evidence. |
| D7 | At most two concurrent workers, separate worktrees, full regressions serially at wave boundaries. |
| D8 | Cheapest capable available worker; one failed-verification retry then mid-tier escalation; frontier orchestrator owns architecture and final review. Resolve exact model IDs before execution; stop after two same-cause failures. |
| D9 | Audit found `HYDRUS.FOR:496-498` repeats water→heat for vapor even without energy balance. Reproduce two passes over one interval; restart the sequence after water timestep reduction; commit time/accounting once. Refines D1, not two accepted water steps. |
| D10 | Separate Fortran harness invokes unchanged pinned `VaporContent` for direct content/capacity evidence. Full executable supplies trajectories; no format changes or conflated evidence claims. User reply "eys" understood as yes. |
| D11 | Audit found `INPUT.FOR:1718` hardcodes `iEnhanc=1` with no reader override. Initial selection uses **enhancement on** to match the unchanged executable; enhancement-off stays G22. Supersedes only D2's enhancement choice. |

Implementation conventions carried from existing APIs, subject to approval of
this plan: f64 water/heat/vapor production arithmetic, explicit precision limits;
no added dependencies; filesystem-free production solvers. Numerical thresholds
remain a later user gate, not implied by plan approval.

## Recon log and hazards

- Clean starting checkout: branch `feat/heat1`, HEAD `a8fc200`; next unused plan
  number is 25. `STATE_24.md` waves complete; its open evidence limitations stay
  historical. Planning edits from this session must be made available to worker
  worktrees before execution; never auto-stash or discard work.
- Read `AGENTS.md`, `CONTRIBUTING.md`, prior plan/state, heat contract/acceptance,
  source-defect register, reference build guide and ADR index/records. ADRs
  0003–0007, 0009, 0011/0016 and 0012 give provenance, mapping, independent checks,
  dependency and reference-limit rules. Solute-specific 0001/0002/0008/0010/
  0013–0019 do not authorize modifying solute or imposing f32 on modern heat.
  **No Accepted ADR is superseded.** ADR 0021 is Proposed, describes heat-only,
  and remains so; add only a scope/navigation note at close, no acceptance.
- Sizes: `TEMPER.FOR` 323, `WATFLOW.FOR` 1127, `water_solver.rs` 693;
  heat transport 569, driver 275, config 192, state 120, capacity 264,
  balance/boundary 276 each; `run_heat_reference.py` 227 and comparator 151.
  Workers enter by symbols/ranges, not unbounded whole-file reads.
- Kernel source: `ConVapor:849`, `VaporContent:930`, scalar `ConVh:968`;
  hydraulic capacity includes the literal `*xConv` at `:958`. `ConVT` uses a
  one-kelvin finite difference; `TempAdj` doubles interior vapor gradients.
  Preserve literal behavior; suspected defects stop for source-register review.
- `Reset:235-267` computes liquid gravity before adding vapor head conductivity.
  Summing conductivities and handing them to the unmodified liquid stencil would
  incorrectly add vapor gravity. Water thermal RHS and vapor storage are needed.
- `SetMat:470-476` blends iterate heads. Existing heat parity uses tables but
  omits mixed-head/fallback behavior because its heads remain constant. That
  shortcut is not adequate for evolving vapor water. Opt-in path must cover the
  blend/predictor and document analytic vs table model explicitly. Table-range
  overflow is a typed error, not a silent approximation; select bounded fixtures.
- `WATFLOW.FOR:152-155` corrects converged `ThNew` using vapor-adjusted capacity
  and the final head increment; vapor fields remain pre-final-solve evaluations.
  Re-evaluating everything at the converged head would change source behavior.
- `WaterState` and `WaterWorkspace` are Clone; private fields accessible inside
  the crate. Existing public water APIs, `AcceptedWaterSnapshot` liquid meanings,
  and zero-allocation tests are preserved. Candidate cloning in the new vapor
  path is acceptable; no new no-allocation promise is made for it.
- Existing `HeatConfig::new` rejects vapor; new accepted config needs a vapor bit
  and a dedicated vapor-aware entry. **Legacy heat driver and transport must
  still reject vapor** rather than using it without terms. Test this hazard.
- `Veloc` endpoint storage corrections differ from `NodOut`; output also uses
  different nonuniform interior weighting. Test both independently. Existing
  solute output helpers are read-only references, not shared edit sites.
- Full executable has enhancement on and head extrapolation on. `NOD_INF_V`
  has no vapor content. Kernel harness evidence is separate, with byte-preserved
  routine extraction and its own build manifest, not a modified full executable.
- Existing Campbell SD-006, heat rounded-parity caveats and zero-gradient gap
  remain untouched. No fixture may be flat/insensitive and count as discriminating
  vapor evidence. Any unobservable effect is a blocker or explicit limitation.
- No server/port suites. Cargo target locks, parity output directories and Python
  fixture scratch are shared hazards: absolute per-worker target/run roots;
  workers run only targeted tests, full suites and Fortran runs serialize.
- Baseline run during planning: all Quick Start suites pass; details/counts in
  STATE_25. This is recon evidence only, not vapor acceptance or run approval for
  unrelated campaigns. Re-run if the base changes before Wave 1.

## Wave table

Every task is in `hyd.rs`. Same-wave product scopes are pairwise disjoint.

| Wave | Task | One-line scope | Depends on |
|---|---|---|---|
| 1 | 25.1.1 | Vapor config, module contracts and legacy-entry guards | Draft contract approved |
| 1 | 25.1.2 | Byte-preserved pinned-kernel oracle and provenance | Draft contract approved |
| 2 | 25.2.1 | Vapor thermodynamic kernels and latent heat | 25.1.1, 25.1.2 |
| 2 | 25.2.2 | Full-reference decks, parser and trajectory fixtures | 25.1.2 |
| 3 | 25.3.1 | Vapor water-row terms and phase velocity mapping | 25.2.1 |
| 3 | 25.3.2 | TempAdj and vapor-aware heat transport | 25.2.1 |
| 4 | 25.4.1 | Opt-in vapor water nonlinear trial path | 25.3.1 |
| 4 | 25.4.2 | Accepted vapor history and trial-state transactions | 25.1.1, 25.2.1 |
| 5 | 25.5.1 | Atomic two-pass water/vapor/heat driver | 25.3.2, 25.4.1, 25.4.2 |
| 6 | 25.6.1 | Source output observation view and vapor field mapping | 25.2.2, 25.5.1 |
| 6 | 25.6.2 | Independent mathematics and vapor balance checks | 25.5.1 |
| 6 | 25.6.3 | Dry-soil discriminating vapor fixtures (amendment 2026-11-12) | 25.2.2 |
| 6 | 25.6.4 | Water-trial iteration semantics correction (amendment 2026-11-12) | 25.4.1, 25.5.1 |
| 7 | 25.7.1 | Rust candidate runs and parity test (split 2026-11-12) | 25.1.2, 25.2.2, 25.6.1, 25.6.2, 25.6.3, 25.6.4; numeric gate |
| 7 | 25.7.2 | Standalone Python vapor comparator (split 2026-11-12) | numeric gate; candidate schema below |
| 7 | 25.7.3 | Isothermal vapor-flux discrepancy diagnosis (amendment 2026-11-12) | 25.7.1 candidate run |
| 8 | 25.8.1 | Capability acceptance and full-coverage handoff | All prior tasks |

Numeric gate precedes **any** candidate-vs-reference check, including kernel
checks embedded in earlier tasks. Reference-only generation is allowed once the
plan is approved. Independent kernel tests may proceed at predeclared
mathematical roundoff bounds; vapor balance/analytical acceptance needs approved
criteria before assertions against its numerical acceptance thresholds. The
orchestrator should obtain the full table after Wave 2, before Wave 3; if the
proposal cannot yet be justified, stop numerical acceptance and record the gate.
Do not commit ignored, vacuous or unconditional-pass parity tests as a substitute.

**Approval artifact ownership:** at that gate the orchestrator (not a worker)
creates `reference/vapor_fortran/criteria.json` after user approval and commits
it with the contract/state approval record. This is an explicit gate operation
outside task product scopes. Canonical schema `vapor-criteria-v1` has keys
`schema_version`, `approved_at`, `approved_by`, `contract_revision`, `contract_sha256`, `fields`,
`checks`, `discrimination`. Each named `fields` entry records `evidence_class`,
`unit`, `absolute`, `relative`, `scale`, `zero_rule`, `print_rule`; `checks`
records independent mathematical/balance bounds and assumptions; `discrimination`
records each required nonzero-effect floor and units. All numeric bounds are
explicit finite nonnegative values; consumers reject unknown/missing entries,
wrong units/classes and unsupported rule names. The gate defines the concrete
rule vocabulary and field/check names before approval. Rust and Python read
this single artifact and report its hash; no independently hardcoded copy.
`contract_revision` identifies the committed approval-time contract snapshot;
`contract_sha256` hashes that revision's file bytes, not the mutable current
contract. Record the revision before creating the criteria artifact, avoiding
circular hashes. Later status/evidence annotations do not invalidate the frozen
binding; changing approved numerical meaning requires a new user approval.

## Module seams, ownership and disjointness

`heat/mod.rs` adds `pub mod vapor;` only in 25.1.1. That task creates
`heat/vapor/mod.rs` declaring `kernels`, `water`, `adjust`, `state`, `driver`,
`output`, `balance`; creates the seven module files with doc-only placeholders;
and records public structs/signatures needed by subsequent tasks in the module
contract. Placeholder files compile but do not pretend to implement a feature.
If a downstream task needs a different shared signature, stop for a plan
amendment owned by the orchestrator; do not edit registries out of scope.

Foundation seam table (25.1.1 resolves exact fields in its reviewed micro-plan):

| Seam | Definition file / owner | First consumer / staging rule |
|---|---|---|
| `VaporConfig`, `VaporConfigInput`, `VaporError` | `V/mod.rs`, 25.1.1 | Existing guards and later modules; real implemented selection/errors |
| `VaporPrimitiveInput`, `VaporProperties` | `V/mod.rs`, 25.1.1 | 25.2.1 kernels; real plain-data types, no deferred module dependency |
| `VaporWaterContext`, `VaporWaterTrial` | `V/mod.rs`, 25.1.1 | 25.3.1/25.4.1; owned candidate plus scalar/vector diagnostics, existing water types only |
| `VaporHeatInput`, `VaporHistoryView` | `V/mod.rs`, 25.1.1 | 25.3.2 uses primitive borrowed arrays, **not** future `VaporState` |
| Material/kernel functions | `V/kernels.rs`, 25.2.1 | Signatures documented in 25.1.1; executable references only after Wave 2 |
| Row/phase functions | `V/water.rs`, 25.3.1 | Implemented before 25.4.1; no executable forward declarations |
| Heat adjustment/dedicated solve | `V/adjust.rs` and `H/transport.rs`, 25.3.2 | Driver only after Wave 3 |
| Accepted state and snapshots | `V/state.rs`, 25.4.2 | Driver after Wave 4; state converts to foundational history views |
| Driver/output/balance types/functions | Respective module and task owner | Signatures initially documentation only; no earlier executable imports |

All shared foundation data types must compile in Wave 1 using only existing
Rust types. Do not add function stubs or refer to a symbol in a placeholder.
Every task gate builds the library implicitly through its targeted cargo test;
full wave gate also builds all targets. Any required foundation field/signature
amendment is serial, orchestrator-owned and reviewed before resuming workers.

Directory abbreviations in task scopes below:
`H=crates/hydrus-core/src/heat`, `V=H/vapor`,
`T=crates/hydrus-core/tests`, `R=reference/vapor_fortran`.
Each task may additionally create/edit **only** its own
`planning/tasks/<id>.todo.md`; scratch is `.development/<id>/` and build outputs
`.development/25-target/<id>/` (absolute `CARGO_TARGET_DIR`). No worker edits the
shared `STATE_25.md`, plan, contract or coverage matrix. Orchestrator appends its
bounded report, avoiding shared report-file conflicts in parallel worktrees.

- Wave 1: config/guards/module declarations versus oracle tools/harness. No
  shared code, fixture registry or manifests.
- Wave 2: `V/kernels.rs` and kernel tests versus trajectory producer/cases.
- Wave 3: `V/water.rs` versus `V/adjust.rs` and `H/transport.rs`.
- Wave 4: `water_solver.rs`/`error.rs` versus `V/state.rs`.
- Wave 5: driver alone; may consume but not alter all preceding interfaces.
- Wave 6: output code/tests versus balance code/independent tests.
- Wave 7: parity test and standalone comparator only; product fixes re-scope.
- Wave 8: acceptance/navigation documents alone, after evidence completes.

## Global execution gates

Before Wave 1: user approves this plan; clear baseline; make planning artifacts
visible in task worktrees; choose exact available worker/mid-tier model IDs and
record them in state; verify GNU Fortran and immutable source/build provenance.
Worktrees: `.development/25-worktrees/<id>`, branch `orchestration/<id>`.
Use absolute external target/scratch roots if a worktree `.development` symlink
is present; check paths exist rather than creating broken `target` symlinks.

At every wave end, orchestrator merges/verifies task changes, runs serially:

```sh
cargo build --workspace --locked
cargo test --workspace --no-fail-fast
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
python3 -m unittest discover -s tools -p 'test_*.py'
```

Also preserve G21 via `cargo test -p hydrus-core --test hydraulic_curves` before
water changes and at the water integration gate. Record exact counts and status.
No wave advances with failures; approved source-evidence limitations are not
test waivers. Raw reference-only output is not a parity result.

## Task specifications

### 25.1.1 — Selection and module contracts

- **Repo:** hyd.rs. **Depends on:** approved draft source contract.
- **Must edit:** `H/mod.rs`, `H/config.rs`, `H/driver.rs`, `H/transport.rs`,
  `T/heat_config_contract.rs`.
- **Can create:** `V/mod.rs`, `V/kernels.rs`, `V/water.rs`, `V/adjust.rs`,
  `V/state.rs`, `V/driver.rs`, `V/output.rs`, `V/balance.rs`,
  `T/vapor_config_contract.rs`.
- **Spec:** add a retained vapor selection to `HeatConfig`; accept only default
  conductivity and Dirichlet temperatures for vapor. Put additional enhancement,
  water boundary and unsupported-physics validation in a closed vapor config
  owned by `V/mod.rs`. Legacy driver/transport reject vapor before mutation until
  called through dedicated vapor entries. Specify the shared primitive structs,
  owned trial report and module signatures for all downstream tasks, including
  previous-time vs predictor/iterate fields, source-model choice and liquid vs
  total flux ownership. No placeholder success implementations. Keep public
  heat-only structs source-compatible where possible (add functions, not mandatory
  fields to existing input struct literals).
- **Anchors/reference:** `HeatConfig::new`, `HeatConfigInput`, `HeatWaterDriver::new`,
  `solve_heat_transport`, `WaterProductConfig::new`, `lower_boundary.rs` typed
  errors; contract Supported/Rejected/Coupling sections.
- **Do not touch:** `lib.rs`, water/solute equations, dependencies, fixtures.
- **Acceptance:** one accepted vapor selection; rejection for every excluded
  selector, nonfinite boundary/period/conversion, incorrect dimensions and
  conflicting water mode; old heat-only selection tests unchanged except the
  former blanket vapor-rejection test is replaced by bounded acceptance plus
  legacy-entry rejection. Unsupported entry leaves water/time unchanged.
- **Targeted tests:** `cargo test -p hydrus-core --test vapor_config_contract`;
  `cargo test -p hydrus-core --test heat_config_contract --test heat_driver_order`.
- **Right-size:** configuration and interface foundation only, <=12 micro-steps.

### 25.1.2 — Pinned-kernel oracle

- **Repo:** hyd.rs. **Depends on:** approved draft source contract, D10.
- **Must edit:** none.
- **Can create:** `tools/run_vapor_kernel_reference.py`,
  `tools/test_run_vapor_kernel_reference.py`,
  `R/kernel_harness/vapor_content_driver.f90`,
  `R/kernel_harness/requests.json`, `R/kernel_harness/README.md`,
  `R/kernel_fixtures/vapor_content.json`.
- **Spec:** standard-library tool extracts the exact bytes of the pinned
  `VaporContent` subroutine to an isolated build root; assert complete routine
  boundaries and source hash, compile with the pinned REAL mode/compatible flags
  and a separate driver (no formula rewrite). Requests are specified independently
  of Rust and trajectory outputs, covering nonzero vapor, dry/moist states and
  multiple temperatures/heads/material porosities. Record routine bytes/source
  hash, harness hash, compiler/flags, executable, request and output hashes,
  commands/start/exit/completion. Refuse overwrite/nonfinite/truncated outputs.
  Retain failed attempts. Label `pinned-fortran-kernel`; never whole-program.
- **Anchors/reference:** `WATFLOW.FOR:930-962`; `tools/run_heat_reference.py`
  `load_build`/`run_case`; reference build guide; ADR 0003/0012.
- **Do not touch:** pinned source/patches, full executable or formats, existing
  reference tools, production code. No scientific tolerance encoded in fixtures.
- **Acceptance:** parser/extraction tests include altered hash/body, malformed
  dimensions, NaN/overflow, failed compiler, stale output/root; fresh harness
  build and fixture have authenticated nonzero content/capacity observations.
- **Targeted tests:** `python3 -m unittest tools.test_run_vapor_kernel_reference`.
  Live reference command is recorded by the worker using the new tool's `--help`
  interface in its approved micro-plan; no Rust comparison before numeric gate.

### 25.2.1 — Vapor material and latent-heat kernels

- **Repo:** hyd.rs. **Depends on:** 25.1.1, 25.1.2.
- **Must edit:** `V/kernels.rs`. **Can create:** `T/vapor_kernels.rs`.
- **Spec:** implement `ConVapor`, `VaporContent` and `xLatent` primitives in f64,
  grouped as source formulas. Return separate liquid/vapor conductivity, thermal
  conductivity, diagnostic `ConLT`, content and effective hydraulic capacity.
  Enhancement-on only in selected production config. Preserve drovs finite
  difference and capacity `xConv` multiplication. Validate all primitive domains
  before writing caller arrays; report typed errors. No full water stepping.
- **Anchors/reference:** `WATFLOW.FOR:849-962`, `TEMPER.FOR:310-323`;
  `H/capacity.rs` `temp_cap` and `TempCapError`; module interfaces from 25.1.1.
- **Do not touch:** compatibility helpers, scalar 20 °C helper, registry, fixtures.
- **Acceptance:** independently worked numeric points, saturated-air zero limit,
  temperature/head sensitivity, conversions, hydraulic-vs-thermal capacity
  distinction, nonfinite/domain/length rejection without partial mutation.
  Expected values are hand/independently derived, not Rust or Fortran outputs.
- **Targeted test:** `cargo test -p hydrus-core --test vapor_kernels`.
  Kernel parity is reserved to 25.7.1 after threshold approval.

### 25.2.2 — Full-executable vapor fixtures

- **Repo:** hyd.rs. **Depends on:** 25.1.2 (evidence distinction).
- **Must edit:** none.
- **Can create:** `tools/run_vapor_reference.py`, `tools/test_run_vapor_reference.py`,
  `R/README.md`; under each exact case directory
  `R/cases/vapor_isothermal_head_gradient/`, `R/cases/vapor_thermal_gradient/`,
  `R/cases/vapor_off_control/`: `SELECTOR.IN`, `PROFILE.DAT`, `fixture.json`;
  `R/fixtures/vapor_isothermal_head_gradient.json`,
  `R/fixtures/vapor_thermal_gradient.json`, `R/fixtures/vapor_off_control.json`.
- **Spec:** stage/build a fresh unchanged full reference per guide. Three decks
  use the supported selectors; isothermal isolates head-driven vapor and thermal
  case exercises nonzero thermal vapor and temperature evolution. Vapor-off
  control shares thermal-case inputs except vapor flag. Fixtures include source
  input mapping, heads/liquid contents/temperatures, all nine vapor output fields
  when available, times and nodes, formats/print intervals and complete hashes.
  Off-control vapor file is unavailable, not a fabricated zero file. Require
  source selector reachability and observable nonzero effects; if print precision
  prevents discrimination, report and redesign inputs before accepting evidence.
  Strict fixed-width parser rejects missing/duplicate times/nodes, malformed/end
  blocks, nonfinite values and field overflow. No reference outputs as forcing.
- **Anchors/reference:** `INPUT.FOR:1718`, `HYDRUS.FOR:224-229,380-387,496-499`,
  `OUTPUT.FOR:815-816,922-926`; `run_heat_reference.py` and its test file.
- **Do not touch:** existing heat tools/fixtures, solute recipe hashes, pinned
  source/patches/formats. Isolated scratch `.development/25.2.2/` only.
- **Acceptance:** all three reference runs complete with manifests; at least two
  vapor-enabled fixtures have nonzero phase fields, thermal case has evolving
  temperature. This is reference-generation acceptance only; approved nonzero
  discrimination floors are evaluated in 25.7.1 after the numeric gate, not a
  prerequisite for completing Wave 2. If raw values are zero/flat, stop here.
- **Targeted test:** `python3 -m unittest tools.test_run_vapor_reference`;
  reference-only production command is named in approved micro-plan and report.

### 25.3.1 — Vapor water rows and phase velocities

- **Repo:** hyd.rs. **Depends on:** 25.2.1.
- **Must edit:** `V/water.rs`. **Can create:** `T/vapor_water_rows.rs`.
- **Spec:** pure opt-in interior row adjustment and face/node phase-flux helpers.
  Keep liquid gravity, add vapor head conductivity and thermal divergence,
  include old/new vapor storage and effective hydraulic capacity. `Veloc` liquid
  and vapor endpoint corrections remain separate. Define signed total flux as
  sum, without changing existing water/solute snapshot semantics. Return typed
  errors for bad shape/spacing/dt/data before mutating outputs.
- **Anchors/reference:** `Reset` `WATFLOW.FOR:235-267`, `Veloc:717-775`;
  `hydraulics/stencil.rs` `ResetRow::calculate` and `T/reset_stencil.rs`;
  `H/driver.rs` `node_velocities` as read-only liquid pattern.
- **Do not touch:** existing stencil/hydraulics files, solver dispatch, output view.
- **Acceptance:** hand-computed three/five-node rows, nonuniform spacing, no vapor
  gravity, isothermal and thermal-only vapor flux, changing vapor storage,
  endpoint signs, zero-vapor limit and bad-input atomicity.
- **Targeted test:** `cargo test -p hydrus-core --test vapor_water_rows`.

### 25.3.2 — TempAdj and vapor heat solve

- **Repo:** hyd.rs. **Depends on:** 25.2.1.
- **Must edit:** `V/adjust.rs`, `H/transport.rs`.
- **Can create:** `T/vapor_heat_transport.rs`.
- **Spec:** implement selected Dirichlet `TempAdj` rows and a dedicated
  vapor-aware heat entry. Factor only necessary shared base assembly; preserve
  heat-only entry/results. Vapor adjustment after both base levels, before solve;
  old and new temperature estimates and vapor histories are distinct inputs.
  Preserve factor-two interior gradient and level-specific latent/sensible terms.
  Unsupported vapor boundaries reject; do not implement dormant flux branches.
- **Anchors/reference:** `TEMPER.FOR:35-55,180-265`;
  `solve_heat_transport`, `assemble_level_one`, `assemble_level_two` and existing
  transport unit tests; `V/kernels.rs` latent heat.
- **Do not touch:** config, boundary/capacity code, water rows or driver.
- **Acceptance:** hand-computed matrix/RHS additions at both levels; Dirichlet
  rows unchanged; nonuniform gradient discriminator; zero-vapor reduces exactly
  to base assembly; output untouched on pivot/shape/domain errors.
- **Targeted tests:** `cargo test -p hydrus-core --test vapor_heat_transport`;
  `cargo test -p hydrus-core --lib heat::transport`.

### 25.4.1 — Opt-in nonlinear water trial

- **Repo:** hyd.rs. **Depends on:** 25.3.1.
- **Must edit:** `crates/hydrus-core/src/water_solver.rs`,
  `crates/hydrus-core/src/error.rs` (only typed vapor-context propagation if needed).
- **Can create:** no product files; inline `#[cfg(test)] mod vapor_trial_tests`
  in the already-owned `water_solver.rs` tests the crate-private entry. Public
  driver integration coverage is in 25.5.1, not a public test-only solver API.
- **Spec:** add a crate-private opt-in trial entry through the existing nonlinear
  core; ordinary public entries pass no vapor context and retain behavior.
  Evaluate vapor properties each iteration with fixed pass temperature; apply
  25.3.1 terms, total-storage convergence and phase-specific accepted trial fluxes.
  Explicitly carry original old-time state and independent iterate initial guess.
  Use mixed `hTemp`/`hNew` material evaluation in this opt-in path, including
  table selection/range handling declared by the contract. Never hide that change
  behind the general `HydraulicModel` behavior for ordinary water callers.
  At convergence apply source `ThNew += Cap*(hNew-hTemp)*Rate` using the last
  vapor-adjusted capacity (`Rate=1` for this selected mode). Retain the last
  pre-final-Gauss vapor fields/conductivities; do not refresh at the final head.
  Carry both the pre-solve material values and corrected source-labeled liquid
  content explicitly into the owned report so velocities/heat/output see the
  right phase and evaluation time.
  Return an owned candidate report with actual `dt`, retries, phase fields and
  diagnostics so the coupled driver can discard/restart/commit atomically.
  Candidate-only accounting must represent one interval, not both coupling passes.
- **Anchors/reference:** `advance_water_step_impl:202`, `solve_trial:321`,
  `assemble:410`, `fill_face_fluxes:527`, `balance:560`;
  `WATFLOW.FOR:64-84,128-149,470-476,560-565`;
  `T/water_transaction.rs`, `T/water_solver.rs`.
- **Do not touch:** hydraulic model implementations, source, solute, atmospheric
  or lower-boundary modules. Boundary selection remains fixed-head for vapor.
- **Acceptance:** evolving vapor affects solved water; all nonlinear iterations
  recompute properties; mixed-head case differs from naive current-head-only;
  initial guess does not replace old storage; reduced dt reported; failure
  preserves original state/workspace; liquid-only existing tests unchanged.
  Include a hand-worked nonzero final head increment that distinguishes the
  corrected `ThNew` from both uncorrected and post-solve-refreshed alternatives.
- **Targeted tests:** `cargo test -p hydrus-core --lib water_solver::vapor_trial_tests`;
  `cargo test -p hydrus-core --test water_solver --test water_transaction --test hydraulic_curves`.
- **Size guard:** micro-plan <=12 steps. If the opt-in seam needs additional
  shared files or a solver rewrite, stop and split/re-scope before implementation.

### 25.4.2 — Accepted vapor history

- **Repo:** hyd.rs. **Depends on:** 25.1.1, 25.2.1.
- **Must edit:** `V/state.rs`. **Can create:** `T/vapor_state.rs`.
- **Spec:** own accepted vapor content, liquid/vapor old velocities, accepted
  time, head predictor/previous-dt and trial buffers required by the module
  contract. Construct from declared initial inputs through kernels; no guessed
  zero histories. Trial/accept/reject validates all fields before promotion.
  Provide snapshots for driver/output without exposing mutable accepted arrays.
- **Anchors/reference:** `HYDRUS.FOR:224-229,380-387`,
  `WATFLOW.FOR:1095-1115`; `H/state.rs` and `T/heat_state.rs`.
- **Do not touch:** water/heat accepted-state types or shared module declarations.
- **Acceptance:** nonzero initialized vapor, exact old/new copy semantics,
  invalid time/data rejected before mutation, predictor kept distinct from
  accepted heads, reject preserves all histories and allows reuse.
- **Targeted test:** `cargo test -p hydrus-core --test vapor_state`.

### 25.5.1 — Atomic two-pass coupled driver

- **Repo:** hyd.rs. **Depends on:** 25.3.2, 25.4.1, 25.4.2.
- **Must edit:** `V/driver.rs`. **Can create:** `T/vapor_driver_order.rs`.
- **Spec:** dedicated vapor-aware driver validates before trial, runs the two
  source-ordered water→velocity→heat passes from one old snapshot, uses first
  heat result for second water pass, and commits all accepted state once.
  Clone local candidate water/workspace/state as the minimal atomic strategy.
  Retain source head predictor separately. Any water dt reduction restarts both
  passes; heat or diagnostic errors return without altering accepted state.
  After success publish phase fields and owned pass/timestep diagnostics for
  output and balance. Never advance a second accepted water timestep.
- **Anchors/reference:** `HYDRUS.FOR:399-403,419,473-499,695`;
  `WATFLOW.FOR:128-149,1095-1115`; `HeatWaterDriver::step` read-only pattern;
  `T/water_transaction.rs` and `T/heat_driver_order.rs`.
- **Do not touch:** legacy driver, water solver or upstream interfaces. A seam
  mismatch is an amendment, not permission for out-of-scope fixes.
- **Acceptance:** exact pass ordering/temperature inputs; same old storage and
  time in both passes; one time/count/accounting increment; rollback after first
  heat, second water, second heat and final diagnostics failures; retry in either
  water pass restarts pass one; successful retry reuses workspace; invalid config
  rejected before water; original heat-only failure behavior still pinned.
- **Targeted tests:** `cargo test -p hydrus-core --test vapor_driver_order`;
  `cargo test -p hydrus-core --test heat_driver_order`.

### 25.6.1 — Vapor observation view

- **Repo:** hyd.rs. **Depends on:** 25.2.2, 25.5.1.
- **Must edit:** `V/output.rs`. **Can create:** `T/vapor_output.rs`.
- **Spec:** expose typed numeric observation rows matching all nine
  `NOD_INF_V` quantities plus coordinates/node/time mapping. Recompute source
  `NodOut` phase fluxes from the correct stored coefficients/final temperature;
  do not substitute solver velocities or update accepted histories while
  observing. Keep source output-space node numbering and depth explicit. No
  filesystem or byte-for-byte Fortran formatter required in production.
- **Anchors/reference:** `OUTPUT.FOR:760-816,891-926`;
  read-only `solute_fortran_compat/compat_runner.rs` tests
  `nod_inf_v_recomputes_liquid_and_vapor_flux_as_nodout`; `V/water.rs`.
- **Do not touch:** solute output helpers, trajectory producer, solver velocities.
- **Acceptance:** independent nonuniform-mesh vectors pin same-side output
  weighting vs opposite-side solver weighting; endpoint vapor storage omission
  and liquid asymmetry; `vTotal=vLiquid+vVapor` within declared arithmetic;
  mapped node/time/depth identity; observation leaves states unchanged.
- **Targeted test:** `cargo test -p hydrus-core --test vapor_output`.

### 25.6.2 — Independent mathematics and balances

- **Repo:** hyd.rs. **Depends on:** 25.5.1; approved numeric criteria for acceptance.
- **Must edit:** `V/balance.rs`.
- **Can create:** `T/vapor_analytical.rs`, `T/vapor_balance.rs`.
- **Spec:** compute liquid+equivalent-vapor storage and signed phase transport,
  vapor sensible/latent heat contributions and residual diagnostics from driver
  primitives. Derive the actual discretized balance, distinguishing source
  storage coefficients from a continuum derivative (including literal capacity
  conversion). Use independent hand-computed material/row references, isothermal
  head-gradient limits, zero-gradient and zero-vapor reductions, seeded fixed
  inputs and nonuniform meshes. Report interior closure separately from
  Dirichlet reservoir bookkeeping. No invented analytical solution for a
  nonlinear coupled case; name the exact assumptions of each independent check.
- **Anchors/reference:** `V/water.rs`, `V/adjust.rs`, source equations in contract;
  `H/balance.rs` and `T/heat_balance.rs` read-only, noting reservoir caveat;
  ADR 0009 independent-reference rule.
- **Do not touch:** driver/solver to make tests pass, source defects, thresholds.
- **Acceptance:** finite correctly signed diagnostics; independently computed
  storage/flux vectors; nontrivial interior water and heat residuals meet approved
  bounds over multiple steps; no forced reservoir closure presented as evidence;
  all assumptions and discrepancies reported, no blanket physical-accuracy claim.
- **Targeted tests:** `cargo test -p hydrus-core --test vapor_analytical --test vapor_balance`.

### 25.6.3 — Dry-soil discriminating vapor fixtures

Amendment 2026-11-12 (user-approved option "Add dry fixture"). Reference-only
analysis of the 25.2.2 fixtures showed vapor-on vs vapor-off head/moisture/
temperature identical at print precision (one head value differs by 0.001 cm;
vapor flux 1e-8..1e-11 vs liquid ~0.5). This triggered the "cannot distinguish
the selected vapor effect at source print precision" stop condition.

- **Repo:** hyd.rs. **Depends on:** 25.2.2.
- **Must edit:** `tools/run_vapor_reference.py` (expected-case set only),
  `tools/test_run_vapor_reference.py`, `R/README.md` (case list paragraph).
- **Can create:** `R/cases/vapor_dry_thermal_gradient/`,
  `R/cases/vapor_dry_off_control/` (each `fixture.json`, `SELECTOR.IN`,
  `PROFILE.DAT`), the two matching `R/fixtures/*.json`, and
  `R/fixtures/dry_discrimination.json` (reference-only on/off report).
- **Spec:** reuse the already-approved illustrative loamy material row unchanged
  (no new material claim); choose dry initial heads (order -1e3..-1e5 cm) and a
  steep fixed thermal gradient within the selected mode (fixed water heads and
  temperatures at both ends, `iEnhanc` default, `lWDep=false`, kg/cm/hour).
  The off-control differs only by `lVapor`. Generate both with the unchanged
  pinned executable into a fresh unique `.development/25.6.3/<attempt>/` root;
  retain failed/NaN attempts. Record a reference-only Fortran-vs-Fortran
  discrimination report: per field/time max |on-off| and its ratio to the print
  half-resolution (head f11.3/e11.4, moisture f6.4, temperature f8.2). Target:
  head, moisture and temperature each show a vapor effect >= 10x print
  half-resolution at some later printed time; if no deck in the selected mode
  achieves that after bounded design iterations, stop and report the best
  achieved ratios instead of widening scope.
- **Anchors/reference:** 25.2.2 decks and producer; `master/M6_VAPOR_THERMAL_MATERIAL_RESEARCH.md`;
  `reference/fortran_reference/README.md`; existing fixture hashes must not change.
- **Do not touch:** existing cases/fixtures/evidence, source, patches, Rust code,
  criteria. No Rust candidate runs.
- **Acceptance:** two new fixtures with provenance hashes; vapor output present
  for the dry vapor case and recorded unavailable for the off-control; the
  discrimination report meets the target or the task stops with the measured
  ratios; existing three fixtures byte-identical.
- **Targeted tests:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_vapor_reference`.

### 25.6.4 — Water-trial iteration semantics correction

Amendment 2026-11-12 (orchestrator review of the 25.6.2 report against source).
The committed 25.4.1 trial differs from `WatFlow` in four ways that affect
parity. All are corrections toward source behavior inside the opt-in vapor
path; ordinary liquid-only entries stay unchanged.

- D1 mixed head: source `SetMat` evaluates liquid properties at
  `0.1*hTemp+0.9*hNew`, where `hTemp` is the iterate before the previous Gauss
  solve (`WATFLOW.FOR:94-96`, `:470-476`). The trial sets
  `material_heads = solved_heads`, so from iteration 2 the mixed head equals the
  current iterate. It must carry the previous iterate.
- D2 convergence (`WATFLOW.FOR:105-127`): per node, when `hTemp<hSat` and
  `hNew<hSat`, `EpsTh=|Cap*(hNew-hTemp)/(ths-thr)/ATh*Rate| <= TolTh` with
  the assembled vapor-adjusted `Cap`; otherwise `|hNew-hTemp| <= TolH`; any
  `|hNew|>0.999*rMax` fails. The trial requires every node's head change to be
  within TolH and also checks a total-storage change. This replaces the plan's
  earlier "total-storage convergence" wording, which was not source behavior.
- D3 minimum iterations: source never accepts after the first solve
  (`Iter.le.1` forces another iteration, `:128`).
- D4 correction reference: `ThNew += Cap*(hNew-hTemp)*Rate` (`:152-155`) uses
  the pre-final-Gauss iterate (`pre_solve_heads`), not the material-evaluation
  head.
- Disclosed but not changed: at `dt<=dtMin` the source accepts a non-converged
  solution (`ConvgF=.false.`). Rust keeps a typed error; 25.8.1 must disclose this.

- **Repo:** hyd.rs. **Depends on:** 25.4.1, 25.5.1.
- **Must edit:** `crates/hydrus-core/src/water_solver.rs` (`solve_vapor_water_trial`
  and its inline `vapor_trial_tests` only); `T/vapor_driver_order.rs` and the
  inline tests in `V/driver.rs`, only to correct `t_conv` to the source value
  (`INPUT.FOR:186-197`, `1/3600` for hours, `1/86400` for days; the tests use
  `86400`) and any expectation that changes as a consequence.
- **Can create:** none.
- **Do not touch:** liquid-only solver paths, hydraulic models, the driver's
  production logic, balance/output modules or criteria.
- **Acceptance:** hand-worked 3-node regression tests distinguishing D1 (the mixed
  head uses the previous iterate), D2 (a dry unsaturated node with head change
  > TolH but EpsTh <= TolTh converges; a saturated node uses TolH), D3 (no
  acceptance after one solve) and D4 (correction uses the pre-solve iterate).
  Existing water solver, transaction, hydraulic, driver, output and balance
  tests pass unchanged apart from the documented unit correction.
- **Targeted tests:** `cargo test -p hydrus-core --lib water_solver::vapor_trial_tests`;
  `cargo test -p hydrus-core --lib heat::vapor::driver`;
  `cargo test -p hydrus-core --test water_solver --test water_transaction --test hydraulic_curves --test vapor_driver_order --test vapor_output --test vapor_balance`.

### 25.7.1 — Paired source comparisons

- **Repo:** hyd.rs. **Depends on:** 25.1.2, 25.2.2, 25.6.1, 25.6.2 and approved
  per-field criteria recorded in contract/state before any comparison.
- **Must edit:** none.
- **Can create:** `T/vapor_fortran_parity.rs`, `tools/compare_vapor_reference.py`,
  `tools/test_compare_vapor_reference.py`.
- **Spec:** rebuild candidate input only from authenticated declared requests/
  decks, never Fortran trajectory outputs. Run new driver to matching times;
  compare heads/liquid contents/temperature and every mapped vapor output field.
  Separately compare primitive content/capacity with the kernel harness fixture.
  Label each evidence class and field availability explicitly. Use independently
  declared schedules; exact identity/discrete checks; field-specific numeric
  criteria; reference print floors. Full executable adaptive schedules not
  reproduced exactly must be disclosed, with deterministic shared observation
  times and a documented error budget; no source-output schedule as forcing.
  Standalone Python comparator and Rust tests consume the same approved table
  and report case/field/time/node/error/tolerance/verdict. Test comparator
  negatives and actual driver sensitivity to vapor-off and dropped-pass controls.
- **Anchors/reference:** `T/heat_fortran_parity.rs` input-hash pattern only (not
  its thresholds or known omissions), `tools/compare_heat_reference.py`,
  R fixtures and contract Observation/Evidence sections.
- **Do not touch:** production code, fixture data, source, approval table or
  heat/solute comparators. Failure requires scoped correction/review.
- **Acceptance:** all selected coupled and kernel fields meet approved criteria;
  nonzero effects exceed approved discrimination floors; missing fields fail or
  are explicitly unavailable with no claimed coverage; comparator tamper/NaN/
  missing-node/time/criterion tests reject. Emit candidate/results under unique
  `.development/25.7.1/<attempt>/` roots; never overwrite earlier evidence.
- **Targeted tests:** `cargo test -p hydrus-core --test vapor_fortran_parity -- --nocapture --test-threads=1`;
  `python3 -m unittest tools.test_compare_vapor_reference`.

#### Split of 25.7.1 (amendment 2026-11-12)

The original 25.7.1 exceeds one small-model task. It is split into 25.7.1 (Rust)
and 25.7.2 (Python), which run in parallel with disjoint files and share the
orchestrator-defined candidate schema below. The spec above still governs both;
the parts below narrow scope and files.

**Candidate schema `vapor-candidate-v1`** (JSON, one file per case). Keys:
`schema_version`, `case_id`, `evidence_class` = `"rust-candidate"`,
`criteria_sha256`, `fixture_sha256`, `input_sha256` (`SELECTOR.IN`,
`PROFILE.DAT`), `schedule` (`kind` = `"constant_dt"`, `dt`, `retry_factor`,
`note`), `time_points`, `node_numbers`, `node_depths`, `head`,
`liquid_moisture`, `temperature_c` (time-major, top-first source node order,
the same layout as the fixtures), `vapor_output_available`, and `vapor_fields`
(the nine fixture names, the same layout) or `null` when vapor is off. Values
are unrounded f64. Kernel candidates use a separate file with
`schema_version` = `"vapor-kernel-candidate-v1"`, `evidence_class`, `criteria_sha256`,
`fixture_sha256`, `requests_sha256` (SHA-256 of `R/kernel_harness/requests.json`;
added 2026-11-12) and per-observation
`request_id`, `node`, `vapor_content`, `adjusted_hydraulic_capacity`,
`input_capacity`.

**Comparison rule** (both consumers, from `criteria.json`): pass when
`|c-r| <= absolute + relative*|r|`. For `case_time_field_max_fraction`
fields the bound is raised to at least `fraction*max|r|` over that
case/time/field. `kernel_capacity_correction` compares
`input_capacity - adjusted` values. Unknown or missing fields, classes, units,
scales or rule names reject. Report case/field/time/node/candidate/reference/
error/bound/verdict, and the criteria SHA-256.

### 25.7.1 — Rust candidate runs and parity test

- **Must edit:** none. **Can create:** `T/vapor_fortran_parity.rs`.
- **Spec:** follow the `T/heat_fortran_parity.rs` deck/profile parsing and
  input-hash pattern (copy what is needed; do not edit that file). Build every
  case only from its authenticated `SELECTOR.IN`/`PROFILE.DAT`; never read
  trajectory outputs into the candidate. Use the `INPUT.FOR` unit conversion
  (cm: `x_conv=100`; hours: `t_conv=1/3600`), the deck's table span (`hTab1`/
  `hTabN`, 100 log-spaced knots, as heat parity) for `VaporSourceMaterial.table`,
  `retry_factor=3` (source `dt/3`), deck `MaxIt`/`TolTh`/`TolH`/`dtMin`, and a
  constant declared `dt` = deck initial `dt`, landing exactly on print times.
  Disclose that the source's adaptive `TmCont` schedule is not reproduced.
  Vapor cases run `VaporWaterHeatDriver`; off-controls run the existing
  `HeatWaterDriver`. Observe vapor fields with `observe_step` at each print time.
  At t=0, observe from the initial state with coefficients from the vapor
  kernels at the initial heads and temperatures (source initial `SetMat`,
  `HYDRUS.FOR:224`), with a zero storage term (`HYDRUS.FOR:365`). Write
  candidates under a new unique `.development/25.7.1/<attempt>/` root, never
  overwriting. Compare in Rust against the fixtures using `criteria.json`,
  print the full verdict table, and fail on any violation. Also run kernel
  parity against `R/kernel_fixtures/vapor_content.json` requests, and check the
  dry-case reference on/off discrimination floors. Controls: candidate vapor-on
  vs vapor-off differs in the dry case; pass-two temperatures differ from
  pass-one in at least one step (second pass matters). No tolerance edits; a
  failure is reported, not tuned.
- **Targeted test:** `cargo test -p hydrus-core --test vapor_fortran_parity -- --nocapture --test-threads=1`.

### 25.7.2 — Standalone Python vapor comparator

- **Must edit:** none. **Can create:** `tools/compare_vapor_reference.py`,
  `tools/test_compare_vapor_reference.py`.
- **Spec:** a stdlib-only CLI that reads `criteria.json`, a fixture and a
  `vapor-candidate-v1` file (or the kernel fixture and kernel candidate) and
  emits the verdict table and a JSON result to a caller-supplied new output
  path (refuse to overwrite). Validate the criteria schema fully (keys, finite
  nonnegative values, known rule names, evidence class/unit match) and
  candidate/fixture identity: case id, fixture hash, node numbers, depths, time
  points and shapes. Missing vapor output is "unavailable" only when the
  fixture says so; otherwise it fails. Compute the dry on/off discrimination
  from the two reference fixtures. Tests use synthetic fixtures/candidates in
  temp dirs: pass, tolerance-edge, NaN, missing node/time, tampered fixture
  hash, unknown/missing criterion, wrong unit/class, zero-floor behavior,
  capacity-correction semantics, discrimination fail. Do not run a Rust candidate.
- **Targeted test:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_compare_vapor_reference`.

### 25.7.3 — Isothermal vapor-flux discrepancy diagnosis

Amendment 2026-11-12, user-chosen "diagnose first". Under criteria v2, isothermal
`v_vapor`/`v_vap_term` fail at 9 points each (err/bound <= 1.76). The Rust
latent-heat temperature disturbance (~1e-4 C) decays more slowly than the
source's: the candidate/reference ratio is ~1.1-1.3 at t=12 h across the
profile. All other fields pass.

- **Question:** is this a Rust coupling difference, or the source's own
  single-precision artifact (REAL `TempN`, `ThVNew`/`ThVOld` cancellation)?
- **Experiments** (diagnostic only, untracked, staged copies; never edit
  `src_reference/` or the committed patch):
  E1, a full-precision trace build that adds output statements only (no
  arithmetic change) for `TempN`, `ThVNew`, `ThVOld` and `dt` at print times.
  Confirm the normal outputs are byte-identical to the 25.2.2 fixture outputs.
  E2, a precision-sensitivity build of the same staged source with
  `-fdefault-real-8` (or the narrowest equivalent that compiles) that runs the
  isothermal deck. Compare E1, E2 and the Rust candidate temperature
  disturbance and `v_vap_term` at t=6/12/24 h. Do further experiments only if
  E1/E2 are inconclusive, and keep them bounded.
- **Files:** create only under `.development/25.7.3/`. No tracked edits, no
  criteria/fixture changes, no Rust production changes. A Python analysis
  script is allowed in scratch.
- **Deliverable:** a report (<=40 lines) in `.development/25.7.3/REPORT.md` with
  hashes, commands, the comparison table and a verdict: source-precision
  artifact, Rust discrepancy (with the suspected term/site), or inconclusive.
  The orchestrator brings the verdict to the user.

### 25.8.1 — Acceptance and coverage handoff

- **Repo:** hyd.rs. **Depends on:** every preceding task and wave gate.
- **Must edit:** `planning/GOALS.md`, `planning/master/M6_VAPOR_COVERAGE.md`,
  `planning/master/M6_VAPOR_CONTRACT.md` (status/evidence only),
  `docs/decisions/0021-couple-heat-transport-one-way-after-the-accepted-water-step-and-reject-unsupported-heat-configurations.md`
  (add dated heat-only scope/navigation note, retain Proposed).
- **Can create:** `planning/master/M6_VAPOR_ACCEPTANCE.md`.
- **Spec:** cite each supported/rejected selection, exact test/fixture/command,
  counts, actual per-field errors/criteria and nonzero discrimination results.
  Separate kernel content evidence, full trajectories, independent checks,
  balances, unsupported and unclaimed modes. Preserve failures and unavailable
  data; do not call a failed gate accepted. Mark G19 complete only if its bounded
  acceptance is established; G22 remains open with next triggers/dependencies,
  G20 progression is conditional on its required vapor interactions, G23 stays
  separately deferred. Keep historical heat acceptance intact. ADR 0021's
  heat-only rule is not silently generalized or marked Accepted/superseded.
- **Anchors/reference:** `M6_HEAT_ACCEPTANCE.md` structure and caveats;
  G19/G22/G23, coverage matrix, all state reports and paired results.
- **Do not touch:** implementation, historical evidence/status records, other
  ADRs, roadmap scope outside vapor. No new ADR required for milestone policy.
- **Acceptance:** orchestrator verifies cited paths/test names and all coverage
  rows remain assigned. Run `git diff --check`; a Python read-only check verifies
  all cited repo-relative paths and contract/acceptance required headings.
- **Targeted checks:** `git diff --check`; documentation link/name validation
  scripted in the approved micro-plan (no solver launches in this task).

## Escalation and approval rules

Workers write <=12-step micro-TODO and stop. Orchestrator checks scope, anchors,
dependencies, test mapping and signatures before sending exactly
`APPROVED -- execute the plan as written.` A separate user update is not the
worker's execution trigger. Two off-spec micro-plans: orchestrator supplies fixed
micro-TODO. One verification retry only, then stop and reassign to mid-tier.
Malformed/missing report: one correction request, then reassign. Same-cause test
failure twice: stop. Out-of-scope edit: decomposition bug; amend the task/graph
and review before continuing. These worker limits do not impose a fixed ceiling
on retained reference-only attempts (ADR 0003).

Each worker returns <=15-line completion report; orchestrator appends to state,
verifies and commits one focused Conventional Commit per task. Planning commit
messages have no phase/wave numbers. Do not push or alter remotes.

## Plan review

Two independent read-only passes completed 2026-11-11, followed by targeted
re-checks. Findings resolved before presentation:

- Spec: explicitly preserve post-convergence `ThNew` correction and pre-final-
  solve vapor-property timing; add a discriminating water-trial test.
- Standards: private trial tests moved inline, avoiding an inaccessible
  integration-test API; foundation seam ownership/staging table added; canonical
  numeric JSON artifact assigned to the orchestrator at the user gate; reference
  generation separated from later approved discrimination acceptance.
- Baseline count reconciled from the retained completed log: 529 Rust passes,
  two ignored, zero failed; 389 Python passes. No source/solver changes made.

Approval judgment calls remain explicit: f64 follows existing APIs (not bitwise
source precision), the opt-in water seam may require re-splitting if >12 steps,
and numeric thresholds/model IDs are later gates rather than invented values.

## Discovered / stop conditions

- A source behavior outside the audited selection is necessary for any fixture.
- Mutable shared registry/API signatures require an unscoped edit.
- Numeric criteria cannot distinguish the selected vapor effect at source print
  precision, or proposed criteria depend on seeing candidate errors.
- Evolving water needs broader material/fallback/source emulation than this
  task split can honestly implement. Do not hide it with a loose tolerance.

Record discoveries in the task TODO/report, stop, and amend this plan if needed.

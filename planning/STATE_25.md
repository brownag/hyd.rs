# Orchestration 25 state — G19 first vapor increment

Created: 2026-11-11. Status: **PLAN APPROVED 2026-11-11; Waves 1–6 complete.**
Plan: [EXECUTION_PLAN_25.md](EXECUTION_PLAN_25.md).
Worker: [PROMPT_TEMPLATE_25.md](PROMPT_TEMPLATE_25.md).
Contract: [M6_VAPOR_CONTRACT.md](master/M6_VAPOR_CONTRACT.md).
Coverage: [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md).

## Baseline and pre-wave-1 checklist

- [x] Starting checkout clean before planning; `feat/heat1` at `a8fc200`.
- [x] Existing exponent correction verified by inspection: `Se^l`, default 0.5,
  commit `1d4131a`; baseline hydraulic regression tests pass. No duplicate fix.
- [x] Serial Quick Start baseline run 2026-11-11 on `a8fc200` (table below).
- [x] User approved plan/contract/coverage scope on 2026-11-11 (reply "yes").
- [x] Planning artifacts copied into isolated task worktrees; unrelated working
  tree planning edits preserved.
- [x] Worker model: `openai/gpt-6-luna` (user-directed; no explicitly cheap tier
  was available in the model catalog). The task 25.1.1 verification retry also
  used GPT-6 Luna; frontier orchestration/review remained in this session.
- [x] GNU Fortran 13.3.0 available; task 25.1.2 built and ran the isolated
  pinned-kernel harness in `.development/25.1.2/kernel-reference-001`.
- [ ] If base code changes, repeat baseline before Wave 1.

| Suite | Command | Result |
|---|---|---|
| Build | `cargo build --workspace --locked` | PASS, exit 0 |
| Rust | `cargo test --workspace --no-fail-fast` | PASS, 529 passed, 2 ignored, 0 failed |
| Format | `cargo fmt --all -- --check` | PASS, exit 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | PASS, exit 0 |
| Python | `python3 -m unittest discover -s tools -p 'test_*.py'` | PASS, 389 tests, 0 failures/errors |

Rust count confirmed by summing all 41 `test result` lines in the completed
baseline log: 529 passed, 0 failed, 2 ignored. Retained local log:
`/home/andrew/.local/share/opencode/shell/f57c76b66e85cb0842b32883aad6fdee5e8f1e46/sh_1cd74c390001CbNSrX1MwFJhq1.out`.
Python stderr includes expected negative-test diagnostics; suite result is OK.
This baseline is not vapor numerical acceptance.

## Numeric approval gate

- [x] After reference-only evidence (wet + dry fixtures, kernel harness),
  proposed field-specific criteria with print rules, zero floor, discrimination.
- [x] User approved values 2026-11-12, before any candidate/reference comparison.
- [x] Table/date recorded in the contract ("Approved numeric criteria"); frozen
  at contract revision `8e561a5`, contract SHA-256
  `2cd1f8b9d952fb15edddd23942dd32d10dbe1ca415e223a7e5ae81aea38cf79f`. No widening.
- [x] Orchestrator created `reference/vapor_fortran/criteria.json`
  (`vapor-criteria-v1`), SHA-256
  `d645cae2a0ba44219eed7b53185abb09a1815ee481041f4e7a3b9fbc0cbbf008`. Rust/Python
  consumers read it, never separate hardcoded limits. Migrate 25.6.2 balance
  constants to its `checks` after 25.6.2 lands.

- [x] 2026-11-12 user approved independent-check bounds for 25.6.2 discrete
  water/heat balance: per-step residual <= 1e-9 relative to the largest absolute
  row term, 1e-15 absolute floor in model units, against the solver's own
  last-iteration (pre-final-solve) coefficients; closure against freshly
  re-evaluated coefficients is a reported diagnostic only (limited by TolTh/TolH).
  To be written into `criteria.json` `checks` at gate-artifact creation.
- [x] 2026-11-12 reference-only finding: 25.2.2 wet fixtures cannot discriminate
  vapor feedback on head/moisture/temperature at print precision (only the
  NOD_INF_V vapor fields discriminate). User chose to add dry-soil fixtures
  (plan amendment task 25.6.3) before proposing per-field trajectory criteria.

- [x] 2026-11-12 POST-FAILURE criteria revision v2 (explicit user override of
  the no-widening rule after Wave 7 v1 failures in isothermal v_vapor/v_vap_term,
  29-31/164 points, errors within float32 TempN one-ulp noise). Only v_vapor,
  v_vap_term, v_total gain a source-precision floor
  `2*ulp32(|T|)*ConVT_max/dx_min`. Contract revision `a4545e4`, SHA-256
  `1dd169b5879642ddc4aca100369d3dd4d7db37650e97eb15bf11b0fdc2e1cfce`;
  `criteria.json` v2 SHA-256
  `1d8dc73eac5fe66c1f8bbfd8c5108781c5efe92509c9689467c3dbf19cc4a7c0`. v1 verdicts
  retained in `.development/25.7.1/attempt-20261112T162852Z/`.

Target point: after Wave 2, before Wave 3. Wave 7 remains blocked without the
gate; earlier independent acceptance checks likewise require their approved
criteria. Plan approval does not open this numeric gate.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 1 | 25.1.1 | Selection and module contracts | done |
| 1 | 25.1.2 | Pinned-kernel oracle | done |
| 2 | 25.2.1 | Material/latent-heat kernels | done |
| 2 | 25.2.2 | Full-executable fixtures | done |
| 3 | 25.3.1 | Water rows and phase velocities | done |
| 3 | 25.3.2 | TempAdj and heat solve | done |
| 4 | 25.4.1 | Opt-in water trial | done |
| 4 | 25.4.2 | Accepted vapor history | done |
| 5 | 25.5.1 | Atomic two-pass driver | done |
| 6 | 25.6.1 | Observation view | done |
| 6 | 25.6.2 | Independent mathematics/balances | done |
| 6 | 25.6.3 | Dry-soil discriminating fixtures | done |
| 6 | 25.6.4 | Water-trial iteration semantics | done |
| 7 | 25.7.1 | Rust candidate runs and parity test | in progress |
| 7 | 25.7.2 | Standalone Python comparator | in progress |
| 8 | 25.8.1 | Acceptance and coverage handoff | pending |

Max two workers; separate worktrees and absolute target/run roots. Orchestrator
owns shared state and serial full-suite gates, one task commit at a time.

## Wave gates

### Wave 1 — complete

- Task commits: `001601d` (25.1.1), `ec7c607` (25.1.2), `8803fea` (format
  correction for 25.1.1); all are local, with no remote changes.
- 25.1.1 targeted: `cargo test -p hydrus-core --test vapor_config_contract`
  (6 passed); `cargo test -p hydrus-core --test heat_config_contract --test
  heat_driver_order` (12 + 5 passed). Initial repeated failure was an invalid
  requirement that material-table length equal node count; corrected and rerun.
- 25.1.2 targeted: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest
  tools.test_run_vapor_kernel_reference` (11 passed); GNU Fortran reference-only
  harness completed with six finite, nonzero content observations and adjusted
  capacity observations. Fixture SHA-256:
  `c7a9a5e3549fb2a0ea006610c8d1408890180acf06d6b497dbe9989e6c1309b1`;
  raw output SHA-256:
  `dbab225e1395604537e9aa36788c7e94157c8ac0f87b587d3e3d57df80252388`.
  Evidence is pinned-kernel only, not trajectory parity or numerical acceptance.
- Serial integrated gates: workspace build PASS; workspace tests PASS (42 suites,
  535 passed, 0 failed, 2 ignored); fmt check PASS; clippy PASS; Python suite PASS
  (400 tests). Python first exceeded a 120-second command timeout, then passed
  unchanged with a 600-second timeout.
- Fixture producer hash records the exact runner version used at generation; the
  runner was subsequently edited for failure-log/publication behavior. Current
  runner hash differs; retain this provenance caveat and do not claim current-code
  regeneration. Existing fixture and evidence were not overwritten.
- No numeric criteria were approved and no Rust-vs-Fortran comparison ran.

## Task reports

### 25.1.1 — complete

Status: done. Added retained vapor selection, closed config/preflight, foundation
types and legacy-entry guards; corrected the node/material-count assumption.
Files are in commit `001601d` plus formatting commit `8803fea`.
Targeted tests: 6 + (12 and 5) passed; full wave gates recorded above. Deviation:
one scoped formatting follow-up after the global format check. No downstream
gotchas beyond consuming the documented foundation seam.

### 25.1.2 — complete

Status: done. Added byte-checked `VaporContent` extraction, separate REAL harness,
strict runner/tests, requests and kernel-only fixture. Commit `ec7c607`.
Targeted Python tests: 11 passed; fresh GNU Fortran run completed. Fixture/raw
hashes and tool-hash caveat recorded above. No numerical comparison or tolerance
claim; downstream must retain the evidence-class distinction.

### Wave 2 — complete

- 25.2.1 is integrated in commits `52d31f8` and lint follow-up `a5e5b0b`.
  Targeted kernel tests: 4 passed; independent hand values, saturated-air limit,
  sensitivity/conversion, and typed invalid-input checks. No Fortran comparison.
- 25.2.2 is integrated in commit `1352377`; its focused tests and reference-only
  evidence are reported below.
- Serial integration gates after both commits: workspace build PASS; workspace
  tests PASS (43 suites, 539 passed, 0 failed, 2 ignored); fmt PASS; Clippy PASS;
  Python suite PASS (410 tests). No numeric acceptance criteria were approved.

### 25.2.1 — kernel task complete

Implemented `ConVapor`, `VaporContent`, and `xLatent` source-grouped f64
calculations with typed validation and owned outputs. Commits `52d31f8` and
`a5e5b0b`. Targeted `cargo test -p hydrus-core --test vapor_kernels`: 4 passed;
workspace tests and gates are recorded above. Numerical parity remains deferred.

### 25.2.2 — reference fixtures complete

- Commit `1352377` (`feat(reference): add vapor trajectory fixtures`) is cherry-
  picked locally. User-approved illustrative soil rows and unit rationale are in
  `master/M6_VAPOR_THERMAL_MATERIAL_RESEARCH.md`; they are order-of-magnitude
  fixtures, not calibrated or universal material claims. Decks use kg/cm/hour,
  default conductivity, and `lWDep=false`; thermal/off-control share the loamy row.
- Targeted parser suite: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest
  tools.test_run_vapor_reference` — 10 passed.
- Fresh isolated build `.development/25.2.2/reference-build-final-mid-tier` and
  run root `.development/25.2.2/reference-runs-final-mid-tier`; stage, build, and
  one `--all` producer command succeeded. Both prior NaN attempts remain intact.
- Three fixtures have `[0,6,12,24]` output times and 41 nodes. Vapor-enabled
  cases have nonzero `con_vh`/`con_vt`; thermal temperature spans 5–25 C.
  Off-control has no vapor output and records it as unavailable. Executable SHA:
  `824179ab353a41f373f4d601cba370071a092a1dfb194b702dfd498ff12a9f1c`.
  Fixture SHAs: isothermal `860f46427cc38c353e5873506192388b416b0292d6fed3f7ea80bab33cecdf32`,
  thermal `c066e7bdccd08fae3bbdd3d6e75b95dd47b37263e16efde1aed1cf67456d85bd`,
  off-control `6d9afbc719c65817cad3ce866e5a6285315cd80c122905b176b73189ffe4f9de`.
- Evidence is `pinned-fortran-trajectory` only. No Rust comparison, tolerance
  approval, or numerical acceptance was performed. No source changes.

### Wave 3 — complete

- 25.3.1 integrated in commit `7849c64`. Implemented isolated source-style
  interior water rows and liquid/vapor/total nodal velocities with liquid gravity
  kept separate and vapor storage corrections at endpoints. Targeted
  `cargo test -p hydrus-core --test vapor_water_rows`: 4 passed.
- 25.3.2 integrated in commit `1790b62`. Implemented owned two-level `TempAdj`
  additions and dedicated vapor heat solve; heat-only entry remains unchanged and
  rejects vapor. Targeted `cargo test -p hydrus-core --test vapor_heat_transport`
  and `cargo test -p hydrus-core --lib heat::transport`: 4 + 4 passed.
- Serial integrated gates: workspace build PASS; workspace tests PASS (45 suites,
  547 passed, 0 failed, 2 ignored); fmt PASS; Clippy PASS; Python suite PASS
  (410 tests). No Fortran/candidate comparison and no numeric criteria approval.

### Wave 4 — complete

- 25.4.1 integrated as `29392a6`. Added the crate-private vapor nonlinear trial,
  source mixed-head liquid evaluation, iterative vapor properties, total-storage
  convergence, final liquid correction, and owned candidate/phase fields. Its
  retry regression verifies reduced-dt exhaustion diagnostics; successful
  reduced-dt acceptance is deferred to coupled-driver coverage. New targeted test:
  1 passed; existing water solver/transaction/hydraulic suites: 23 passed.
- 25.4.2 integrated as `29f4942`. Added computed initial vapor content/velocities,
  immutable snapshots, staged validation, atomic accept, and reject/reuse.
  Targeted state suite: 2 passed.
- Serial integrated gates: build PASS; workspace tests PASS (46 suites, 550
  passed, 0 failed, 2 ignored); fmt PASS; Clippy PASS; Python PASS (410 tests).
  No Fortran/candidate comparisons; numerical criteria remain unapproved.

### Wave 5 — complete

- 25.5.1 integrated as `232ab4e`. Worker model for this wave: Claude Sonnet
  (orchestrator session moved to Claude Code; frontier review unchanged).
- Orchestrator corrections at plan approval (source-checked): (A) the driver owns
  every water dt cut — trials run with local `minimum_timestep = attempt dt`; a
  `NonConverged` in either pass restarts pass one at `dt/retry_factor` from old
  heads (`hNew=hTemp=hOld`) and old temperatures (`WATFLOW.FOR:136-149`), because
  the 25.4.1 internal retry re-seeds the predictor and keeps pass-1 temperatures;
  (B) predictor uses the *next* dt (`HYDRUS.FOR:650-662,695`): stored at commit
  as unit-ratio `h+(h-hOld)` on negative, non-Dirichlet nodes, rescaled at next
  step start by `dt_attempt/previous_timestep`.
- Accounting: `accepted_steps` +1, `cumulative_iterations` += both final passes;
  liquid-only volume totals deliberately not advanced on the vapor path; total,
  liquid and vapor face fluxes and old/new contents published in
  `VaporStepReport` for 25.6.2.
- Targeted: `--test vapor_driver_order` 3 passed; `--test heat_driver_order` 5
  passed; inline `--lib heat::vapor::driver` 3 passed (rollback after heat 1,
  water 2, heat 2, diagnostics; pass-2 forced retry; predictor ratio/sign).
- Serial gates: build PASS; workspace tests PASS (47 suites, 556 passed, 0
  failed, 2 ignored); fmt PASS; Clippy PASS; `hydraulic_curves` 9 passed;
  Python PASS (410 tests). No Fortran/candidate comparison; criteria unapproved.
- Discovered (open): stale `#[allow(dead_code)]` on `solve_vapor_water_trial`
  (`water_solver.rs:202`); predictor storage convention should be recorded on
  `VaporHistoryView.head_predictor` docs/contract; 25.4.1 internal retry
  semantics differ from source but are bypassed by the driver.

### 25.5.1 — complete

Status: done, commit `232ab4e`. `V/driver.rs` (new `VaporWaterHeatDriver`,
`VaporStepReport`, `VaporPassReport`, `VaporDriverError`, inline tests) and
`T/vapor_driver_order.rs`. Deviation: `adjust_heat_rows` not called directly
(`solve_vapor_heat_transport` applies it); `VaporHeatInput` unused. Gotchas for
Wave 6: accepted temperatures live only in `VaporState`; preflight requires a
positive `previous_timestep`; fixtures with `x_conv=t_conv=1` and h=-100 gave
unphysical heat and were rejected by staging — tests use cm/day conversions.

### Wave 6 — complete

- Seam amendment (orchestrator, serial) `2f6a94c`: `VaporStepReport` publishes
  stored pass-two Con/ConLT/ConVh/ConVT (pre-final-solve, at first heat result)
  for `NodOut` recomputation. Driver tests 3+3 passed; crate fmt/Clippy PASS.
- 25.6.1 integrated as `6c5776a`. `observe_vapor`/`observe_step` produce top-
  first rows (node `n-i`, depth `x[i]-x[top]`) with all nine fields, same-side
  interior weighting, top-endpoint liquid storage/sink only (source `i=N` is the
  surface; worker's initial bottom reading corrected at approval), no vapor
  endpoint storage, ConLT echoed but excluded from vLiquid, liquid-only gravity.
  One review retry added a stale-report guard (time and timestep must match the
  accepted state). `--test vapor_output`: 6 passed.
- 25.6.3 approved: reuse the 25.2.2 executable read-only (found by hash in the
  earlier worktree), keep `hTabN=100` so dry heads use the analytic FK/FC/FQ
  branch, linear initial temperature, subset expected-case check.
- 25.6.3 integrated as `b8e0544`: dry loamy deck (h=-1e4 cm fixed ends, linear
  45->5 C, 240 h, prints 6/24/72/240 h) and vapor-off control from the 25.2.2
  executable `824179ab...` read-only; first attempt met the >=10x target
  (moisture 14-392x, temperature 346-576x, head ~1.7e7x print half-resolution).
  Existing fixtures byte-identical; producer test 12 passed. Discovered: the
  committed 25.2.2 case dirs lack `LEVEL_01.DIR`, which the producer now
  requires, so the existing three cases cannot be rerun as committed.
- Numeric gate closed (see checklist): contract `8e561a5`, criteria `bde9c7f`.
- Second seam amendment `49fc1b8`: report publishes pass-two `pre_solve_heads`,
  `pre_solve_liquid_water_contents`, `effective_hydraulic_capacities`.
- 25.6.2 integrated as `9a864c2` after one retry (test `tConv` was inverted,
  3600 instead of 1/3600, making vapor ~1e7 too small; bounds moved to a
  `criteria.json` loader). Interior water max 3.4e-16, heat 2.6e-16 relative
  (bound 1e-9); fresh-coefficient closure 6.1e-13 reported only; vapor/liquid
  transport ~1.06e3 and storage ~1.66e-3 asserted >= 1e-6. Tests 6 + 3 passed.
- Orchestrator review found 25.4.1 iteration deviations from `WatFlow` (mixed
  head, convergence rule, minimum iterations, correction reference); plan
  amended with correction task 25.6.4. 25.5.1 driver tests use `t_conv=86400`
  for days (source 1/86400); fixed in 25.6.4.
- 25.6.4 integrated as `494c040` (D1 previous-iterate mixed head, D2 source
  per-node EpsTh/TolH/rMax convergence with inline f64 `hSat=FH(Se=1)`, D3 no
  acceptance after one solve, D4 correction vs pre-solve iterate) and
  `366566f` (driver tests: days `t_conv=1/86400`, SI-scaled heat materials).
  One existing test moved to `max_iterations: 2` with hand re-derived values
  plus a `max_iterations: 1` NonConverged assertion. Disclose for 25.8.1: source
  accepts non-converged at `dt<=dtMin`; Rust errors. The D2 saturated-branch
  test cannot separate the saturated rule from EpsTh (weak).
- Wave 6 serial gates (after `366566f`): build PASS; workspace tests PASS (50
  suites, 574 passed, 0 failed, 2 ignored); fmt PASS; Clippy PASS;
  `hydraulic_curves` 9 passed; Python PASS (412 tests).
- 25.6.2 bounds are temporarily single named test constants citing the gate
  approval; the orchestrator migrates them to `criteria.json` `checks` when the
  artifact is created.

## Planning review

Independent read-only Spec and Standards reviews completed 2026-11-11. Resolved:
source final-water-content/property timing; private-entry inline test access;
foundation seam/compilation ownership; future criteria JSON ownership; reference
generation versus post-approval discrimination gate; baseline count confirmation.
See plan review section. No implementation run or worker micro-plan approval.

## Open execution blockers / discoveries

- Plan approval recorded; later numeric approval remains a separate gate.
- Full executable hardcodes enhancement on and loops twice; corrected scope is
  D9/D11, not the initial single-pass/enhancement-off recommendations.
- Direct content evidence is pinned-kernel only, not full-program trajectory.
- Evolving water must handle source mixed-head/predictor semantics; old constant-
  head heat fixture shortcuts do not establish this.

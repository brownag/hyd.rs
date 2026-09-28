# State — Orchestration 13

Plan status: **Execution started on 2026-09-26 by user wave-orchestration instruction; Wave 1 accepted; Wave 2 accepted; Wave 3 split into serial tasks; component references accepted; finite-chain evaluator accepted; all benchmark producers accepted; gated analytical execution next.**
Scientific status: **M5-V open; convergence, independent full-case analytical evidence and justified tolerances remain unmet.**
Plan: [EXECUTION_PLAN_13.md](EXECUTION_PLAN_13.md). Workers: [PROMPT_TEMPLATE_13.md](PROMPT_TEMPLATE_13.md).

## Baseline — 2026-09-26

- [x] Orchestration 12 scoped evidence work completed; its scientific gate remains open.
- [x] Branch `phase4-water-acceptance`, baseline commit `f86a028`; clean tree before planning.
- [x] `cargo test --workspace --quiet`: 130 tests passed, zero failures.
- [x] `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_fortran_reference tools.test_solute_outlet_contract`: 13 passed.
- [x] `PYTHONDONTWRITEBYTECODE=1 python3 tools/compare_solute_reference.py --self-test`: 12 checks passed.
- [x] `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check`: passed.
- [x] No new Fortran execution during planning; no solver changes.
- [x] Refreshed all baseline checks: 130 Rust tests, 13 Python tests, 12 comparator checks, fmt/Clippy/diff passed. Protected 207 files in `.development/13-baseline/protected-hashes.json`; hashes unchanged after checks. Cargo output override: `.development/12.6.1/13-regression/baseline-20260926-01.json`.
- [x] Independent spec review: passed; clarified reporting of measurement floors within user-selected build limits.
- [x] Independent standards review: passed after correcting legacy parity-output preservation; reviewer rechecked the fix.
- [x] User approved the complete plan on 2026-09-26 (“approve”).

## Wave board

| Wave | Task | Deliverable | Status | Commit |
|---|---|---|---|---|
| 1 | 13.1.1 | Mathematical benchmark/case/schema contract | done | 490a41b |
| 2 | 13.2.1 | Isolated Fortran adapter | done | 4a40220 |
| 2 | 13.2.2 | Configurable Rust adapter | done | 0023eb4 |
| 2 | 13.2.3 | Strict metrics and normalization | done | 15b21b5 |
| 3a | 13.3.2 | Component analytical references | done | 6285bca |
| 3b | 13.3.3 | Finite-domain chain reference | done | 88b2656 |
| 3c.1 | 13.3.5 | Checkpointed analytical reference work | done | 4323adf |
| 3c.2 | 13.3.6 | Shared bounded solver lifecycle | done | a6edebe |
| 3c.3 | 13.3.7 | Retained-evidence bridge gate | done | 431b7ad |
| 3c.4 | 13.3.4 | Bounded benchmark integration | done | b9472e7 |
| 3d | 13.3.1 | Analytical executions and evidence | in progress | — |
| 4a | 13.4.2 | Pure convergence classifiers | done | 69d60ad |
| 4b.1 | 13.4.4 | Authenticated convergence evidence replay | done | pending commit |
| 4b.2 | 13.4.3 | Gated convergence CLI | pending | — |
| 4c | 13.4.1 | Separate spatial and temporal convergence evidence | pending | — |
| 5 | 13.5.1 | Discrepancy attribution/remediation specifications | pending | — |
| 6 | 13.6.1 | Budgets, regression gates and M5-V assessment | pending | — |

## Resume point

Waves1/2 and all benchmark producers accepted. Task13.3.1 running: both bridge rows and ten component rows complete, root independently verified;12/77slots consumed,65remain. Full Example3 reference active once in batch-252f19ab8f56442a808e16e3bbad714b, cache22a684a825245213ea48b0afab6ac34524dce0dedd1ae05d4c0b228ac0789916; worker process session44969. Initial192/400stage started2026-09-28T01:06:21.635470UTC. Do not repeat interrupted stages or starts. Exact activation/reference policy retained under13.3.1; optional512/650authorized only if needed. Full solver rows await admission. Pure13.4.2 classifier microplanning permitted concurrently in disjoint scope;13.4.3 integration waits for13.3.1 and exact helper scope amendment. Partial analytical report remains pending. Preserve207protectedhashes. Latest full-suite parity output wave3c4-20260927-01.json. No thresholds;M5-Vopen.

## Task reports

## 13.1.1
- Status: accepted; mathematical contract and schemas frozen, scientific accuracy not yet established.
- Created spec (593 lines), CASES (2849), README (41), checker (371), and completed TODO (38).
- Independently checked H Robin/Neumann boundaries, resolvent signs, scalar eigenmode, and equal-rate unequal-storage chain (total mass 2 preserved).
- Decimal full-case inversion feasible provisionally; latest tested differences ~3e-10 concentration / 6e-10 integral; empirical, not rigorous remainder bounds.
- Checker self-tests: 34 passed; CASES validation and diff checks passed.
- Wave-end baseline: 130 Rust tests, 13 Python tests, 12 comparator cases; fmt/Clippy passed.
- Protected 207 source/fixture/prior-artifact hashes unchanged; no new Fortran/study solver launches.
- Reviewed matrix: 77 conditional launches, 60 seconds each, 2699462776-byte aggregate cap; activation awaits downstream producer gates.
- Corrected printed concentration precision to four significant digits; original bridge controls retained.
- Deviations: none. Worker recovered from transient service usage error; no incomplete output accepted.
- Remaining: Wave 3 per-observable evaluator admission; capacity-limited temporal contamination and M5-V remain unresolved.

## Wave 2 interface decision

Approved the metrics validated-dict context and exact reference wrapper in the spec. Analytical concentration arrays cover initial/analysis times; continuum observables cover all diagnostic times. Shared artifact names and source formats frozen. Adapter task prose now explicitly leaves independent derived integrals to metrics. No added JSON observation keys, dependencies, or solver launches.

## 13.2.2
- Status: accepted after one review correction; Rust harness and completed TODO integrated from isolated worktree.
- Strict flat request/hash/path validation, fixed-step observations, explicit frozen physics, and solver/derived separation implemented.
- Corrected bridge profile schedule to 0/50/100/200 while retaining all step endpoints.
- Orchestrator reran targeted harness: 15 passed; formatting, targeted Clippy and whitespace passed.
- No core/Cargo/old-fixture edits; no study launches (77 conditional slots remain).
- Wave-end global suite deferred until all Wave 2 producers are integrated.
- Launcher owns casebook/launch authorization and mathematical IC validation; metrics owns independent integrals.

## 13.2.1
- Status: accepted; isolated Fortran adapter, 23 mocked tests and completed TODO integrated.
- Orchestrator reran 23 tests, fresh dry-run and verify-only: all 39 planned Fortran manifests verified.
- Fresh preview: worktree `.development/13.2.1/orchestrator-preview-20260927`; no builds/solver launches.
- Read-only retained bridge validates 2000 steps, four profiles, exact terminals and water mapping.
- Reviewed source/deck/hash preservation, process timeout/storage caps, ledger accounting, no-overwrite and symlink rejection.
- Raw manifests stay verifiable after normalization; bridge unavailable endpoints use null.
- Corrected file-specific solute header and complete mock source-authority fixture during review.
- Protected main 207 hashes unchanged; worker additionally checked 211 across its checkout/shared evidence.
- Deviations: none; all 77 conditional launch slots remain unconsumed.
- Downstream APIs: initial_arrays, request_bytes, generate_decks, validate_outputs; actual build/run bridge remains Wave 3.

## 13.2.3
- Status: accepted; strict metrics/normalization module, 22 tests and completed TODO integrated.
- Independent per-species norms, phase/continuum/mesh inventories, signed endpoint/reaction integrals and balance implemented.
- Executable/shim hashes bind retained artifacts; initial arrays match declared formulas; strict path and complete record validation.
- Worker cross-adapter synthetic check: 33 profiles, 128 steps, 129 endpoints; retained raw bridge readers independently passed.
- Fixed tiny norm error cancellation; explicit zero/near-zero scales retain uncertainties.
- Profile-coordinate uncertainty remains explicitly unresolved without independent gradient bounds; quadrature sensitivities remain empirical.
- Orchestrator wave checks: 145 Rust tests; 58 Python tests; 12 comparator self-checks; 34 contract self-checks; fmt/Clippy/diff passed.
- Fixed Fortran tamper test to make only isolated copy writable; pinned main sources and all 207 protected hashes unchanged.
- Full Cargo output used fresh `.development/12.6.1/13-regression/wave2-20260927-01.json`.
- Deviations: none; clarified species independence applies without transfer, with separate legitimate chain-coupling coverage.
- Study launches: zero; thresholds unassigned and M5-V remains open.

## Wave 3 decomposition decision

Worker identified four clusters exceeding twelve small steps. Approved serial subwaves3a–3d with separate task gates and commits; preserved original13.3.1 execution root and downstream IDs. Added producer test module `tools.test_solute_benchmarks` to future wave-end suites. Matrix, limits and scientific scope unchanged; no solver execution activated.

## 13.3.2
- Status: accepted; independent component evaluator, 21 tests and completed TODO.
- Reaction uniformization, diffusion cosine and scalar Robin/Neumann formulas use independent integrals and outward Decimal enclosures.
- Reviewed reaction tail indexing, integrated weights, transfer integral and inward signs; scalar frozen-root IC error uses a maximum-principle bound.
- All five component reference budgets admitted; maximum concentration uncertainty 1.36e-16, inventory uncertainty 2.62e-16.
- APIs: component_reference(case, coordinates, analysis_times, diagnostic_times), reference_admission(case, reference); wrapper unchanged.
- Orchestrator checks: 21 targeted tests; 145 Rust tests; 79 Python tests; 12 comparator and 34 contract checks; fmt/Clippy/diff passed.
- Cargo used fresh .development/12.6.1/13-regression/wave3a-20260927-01.json; 207 protected hashes unchanged.
- No study launches, core/source edits or solver thresholds; finite-chain remains explicitly unavailable pending next task.
- Deviation: serial decomposition already recorded above; scientific M5-V remains open.

## Finite-chain feasibility activation

Approved task 13.3.3 micro-plan: reference-only 11-node mesh, analysis times 50/100/200 and ten listed continuum times, orders192/256/384 at400 digits, optional single512 at650 if needed. Precision subset and two fixed parent PDE residual stencils are declared in TODO. This is reduced evaluator feasibility, not full-case scientific admission. No solver launch activation or extra matrix rows.

## Finite-chain interim verification

Implementation review and orchestrator reruns passed:34 targeted tests,145 Rust tests,92 Python tests,12 comparator checks,34 contract checks,fmt/Clippy/diff. Cargo output:.development/12.6.1/13-regression/wave3b-20260927-01.json. Reduced numerical feasibility is still running at .development/13.3.3/feasibility/finite-chain-1d00b2c17a85; do not repeat its logged orders. Task13.3.3 is not yet accepted.

## 13.3.3
- Status: accepted; finite-domain chain evaluator, 34 targeted tests and completed TODO.
- Reviewed stable H/I kernels, confluent divided differences, independent cumulative transforms and strict admission diagnostics.
- Reduced exercise: 11 nodes, 3 profile times, 10 continuum times; all 402 scalar entries admitted empirically.
- Orders 192/256/384; optional 512 unused; maximum uncertainty 2.948e-9; precision difference below 9.724e-144.
- Parent PDE residual reduced 3.9967 times when stencil spacings halved; diagnostic only.
- Runtime 825.87 seconds; rough full-time projection about 5 hours, not a measured cost or bound.
- Artifacts and source snapshots: .development/13.3.3/feasibility/finite-chain-1d00b2c17a85; reuse rules in HANDOFF.md.
- Orchestrator reproduced retained admission, matched current source hashes and verified all 207 protected hashes.
- Orchestrator checks: 34 targeted, 145 Rust, 92 Python, 12 comparator and 34 contract checks; fmt/Clippy/diff passed.
- API: finite_chain_reference returns reference/diagnostics; reference_admission requires bound diagnostics for chain.
- No solver launches; full coverage pending; uncertainty empirical, scientific M5-V remains open.
- Deviations: none beyond recorded decomposition; no core/source/dependency edits.

## Benchmark integration decomposition

Accepted planning-only split of 13.3.4 into serial prerequisites 13.3.5–13.3.7 and final integration. Exact scopes/acceptance added to execution plan. A narrow stage interface permits durable reference checkpoints without monkeypatching private evaluator functions. Numerical method, coverage and all 77 solver rows unchanged; no numerical execution activated.

## 13.3.5
- Status: accepted; optional analytical stage executor and authenticated persistent reference cache.
- Default evaluator method/arguments unchanged; exact Decimal checkpoints preserve diagnostic ordering.
- Requests bind case, exact mesh/schedules, settings, runtime and source hashes; replay reproduces diagnostics/admission without inversion.
- Durable start/completion ledger, exclusive files and locks; incomplete/failed/tampered stages block automatic repetition.
- Reviewed coverage, metadata/result consistency, crash windows and preserved unmet outcomes.
- API: ReferenceCache(root).reference(..., allow_work=False, allow_escalation=False) returns reference, diagnostics, admission.
- Worker targeted 57 passed; root combined 115 Python and 145 Rust passed, plus 12 comparator and 34 contract checks.
- Root fmt/Clippy/diff and 207 protected hashes passed; fresh Cargo output wave3c1-20260927-01.json.
- No expensive analytical evaluations or solver launches; all 77 rows remain unconsumed.
- Deviations: recorded producer decomposition only; mathematical coverage and M5-V remain open.

## Rust retained-executable storage decision

Task 13.3.6 planning measured the existing debug harness at 11,664,672 bytes, above the smallest 4,568,792-byte per-run cap. Approved omitting debug symbols with profile.test.debug=0 while preserving unoptimized test semantics and f64 mode; exact build provenance and prelaunch size checks required. No build or solver run activated and no cap changed.

## 13.3.6
- Status: accepted; reusable shared lifecycle and 17 mocked tests.
- Exact phase/row activation, immutable requests/manifests, shared ledger/limits and no automatic retries.
- Rust isolated build recipe omits debug metadata, keeps test semantics and records verbose compiler commands; actual build remains pending.
- Reviewed prelaunch executable/output reservations, actual global storage, process-group cleanup and final metrics validation.
- Fortran reuses accepted adapter with normalization-size checks; raw evidence remains immutable.
- Independently authored Rust observations pass actual metrics validation; failures, corruption and read-only verification tested.
- Root checks: 132 Python and 145 Rust passed; 12 comparator and 34 contract checks; fmt/Clippy/diff passed.
- Fresh Cargo output wave3c2-20260927-01.json; 207 protected hashes unchanged.
- API and build caveats: .development/13.3.6/HANDOFF.md. No real study build/launch; all 77 rows unconsumed.
- Deviations: approved debug-metadata and verbose logging decisions only; no numerical mode or cap changes.

## Bridge comparison policy review

Approved task 13.3.7 micro-plan with pinned retained artifacts and same-solver comparison. Rust orientation reversal changes elimination/reduction rounding; accepted source-derived M-matrix/operation-count arithmetic envelopes, including rounded linear intercept and cumulative sums, evaluated independently of new output. This is a bridge identity guard, not a continuum accuracy tolerance. Original Rust executable and per-step history remain unavailable. No solver runs activated.

## 13.3.7
- Status: accepted; authenticated same-solver bridge comparison/report gate and 23 tests.
- Pinned retained inventory, original Fortran manifests/build and Rust observations; current core/harness policy hashes fixed.
- Fortran compares all scientific raw fields with paired printed intervals; only date and calculation-time metadata excluded.
- Rust maps reversed orientation and declared time recurrence; source-derived outward arithmetic bounds cover profiles and mass terms.
- Root independently screened final concentration guards: approximately 2.411e-6, 2.543e-7, 7.040e-6; these are not accuracy tolerances.
- Reports authenticate inputs and recompute comparisons; both distinct passing reports required before dependent activation.
- Missing original Rust executable/all-step history and Fortran endpoint history remain explicitly unavailable.
- Root checks: 155 Python, 145 Rust, 12 comparator and 34 contract checks; fmt/Clippy/diff passed.
- Fresh Cargo output wave3c3-20260927-01.json; 207 protected hashes unchanged.
- APIs: evaluate_bridge, write_report, verify_report, require_bridges; handoff under .development/13.3.7/.
- No builds, reference evaluations or scientific launches; all 77 rows unconsumed; M5-V remains open.

## Final benchmark integration activation

Approved revised 13.3.4 twelve-step micro-plan using all accepted helpers. Only mocked implementation tests and the fourteen-row dry-run are activated; narrow .development/13.3.1/launch.lock exception recorded in plan. Actual bridge/benchmark/reference-work commands remain prospective and require separate root activation in 13.3.1. Resume may narrow the original authorized rows but cannot repeat consumed starts.

## Planned dry-run supersession

Initial 13.3.4 dry-run batch66b1247c5dea4e97b6aaefcebf28d7bf planned14 rows and verified with zero starts. Final root review found missing index-to-manifest launch identity and unique-path checks. Required correction/test before acceptance; archived all67 unchanged planned files under .development/13.3.1/benchmarks/superseded/ with origin/hash record and available old producer snapshot. Absolute paths in that recipe are historical. No scientific evidence or launch ledger was moved; corrected fresh dry-run pending.

## 13.3.4
- Status: accepted after final index/manifest identity correction; CLI integration and 17 mocked tests.
- Immutable batches, exact activation subsets, bridge/reference gates, authenticated metrics and safe explicit resume.
- Verification performs no builds, solver starts or new analytical stages; unassigned science remains false/unmet.
- Fixed forged planned-index duplicate/swapped-path acceptance; original planned batch preserved as documented above.
- Fresh dry-run batch-e749c2682b8f4fb48814ec1b3dcbf290 planned exactly14 rows; root read-only verification passed.
- Root combined checks: 172 Python, 145 Rust, 12 comparator, 34 contract; fmt/Clippy/diff passed.
- Fresh Cargo output wave3c4-20260927-01.json; all207 protected hashes unchanged.
- Actual builds, full references and solver launches remain unactivated; all77 launch rows unconsumed.
- Concrete commands/API: .development/13.3.4/HANDOFF.md; exact subsets permit10 component rows and2 full-chain rows separately.
- Deviations: narrow dry-run phase-lock scratch exception and planned-only archival correction; no scientific scope changes.

## Bridge launch activation — 2026-09-27

Approved 13.3.1 eleven-step micro-plan. Separately activate only the two frozen bridge rows in .development/13.3.1/bridge-activation.json, CASES SHA d3b06531c06545ca89956b95c487bb1616ace1470ba1a8c02a3f1209ff4e8acf. Exact command is the TODO bridge recipe with PYTHONDONTWRITEBYTECODE=1. Baseline modes, 60 seconds each, 26,165,360-byte aggregate row reservation within 2,699,462,776-byte global cap. Compilation remains separately accounted with accepted debug-metadata-only Rust profile. No analytical reference work or dependent rows activated. Failed starts consume slots; no retries. Root reviews both reports before further activation.

## Bridge review — 2026-09-27

Root independently reproduced both bridge reports in batch-c96c2d089400481a913a4accb6f1650f. Fortran259142 fields identical; Rust maximumC difference1.266e-14, maximumall-field1.108e-12; retained underflow/denormal warning. Both2000steps/fourprofiles; executable sizes490736/2172336bytes. Two slots consumed,75 remain; protected207 unchanged.

## Component launch and reference activation — 2026-09-27

Separately activate the exact ten component rows in component-activation.json and five complete shared reference requests in component-reference-policy.json. Use accepted component interval formulas, all frozen nodes/analysis/diagnostic times, cache authentication and --allow-reference-work; no escalation or full-chain inversion. Use both reviewed bridge report paths and the exact TODO component command once. Each60seconds, failedstarts consume; no retries. Root reviews admissions/results before fullExample3 activation.

## Component review — 2026-09-27

Root independent ten-row verification passed; five references admitted, maximum profile uncertainty1.353e-16. Both solvers complete without warnings/failures. Numerical errors recorded in component-review-20260927.json; no accuracy thresholds assigned. Twelve starts consumed,65 remain; protected207 unchanged.

## Full Example 3 activation — 2026-09-27

Separately activate only two frozen Example3 component rows in example3-activation.json and exact complete shared request in example3-reference-policy.json:1001nodes,initial0 plus50/100/200 profiles,201 diagnostic times. Permit frozen192/256/384 orders at400digits, final650 precision and exactbinary-input subset checks; single optional512/650 stage only if admission requires it. No added orders or retries. Use both reviewed bridge reports, --allow-reference-work --allow-reference-escalation, accepted cache and unchanged60second solver limits. Reference admission precedes starts; preserve unmet/failed evidence. Runtime projection is not a bound. Convergence/holdout remain unactivated.

## Convergence task decomposition — 2026-09-27

Split original13.4.1 tooling into13.4.2 with dependency on accepted13.3.4 helpers. Its disjoint mocked producer may proceed during long13.3.1 analytical work; no builds,new references,solver starts or shared evidence writes. Original13.4.1 retains scientific execution/report and depends on both accepted tasks. No row,threshold,capacity,timeout or scientific objective changed. Global checks/commits remain serialized.

## Convergence producer scope correction — 2026-09-27

Anchor review found BenchmarkBatch hard-coded phase benchmark and lifecycle convergence root13.4.1, incompatible with producer scratch13.4.2. Avoid changing producers used by active reference:13.4.2 now pure classifier/tests only. Insert pending13.4.3 CLI integration after13.3.1 with precise helper amendment still to be reviewed. Numerical execution13.4.1 waits for13.4.3. No extra launches or mathematical scope changes.

## Pure classifier micro-plan approval — 2026-09-27

Approved13.4.2 twelve-step microplan, in-memory independently trusted bindings, uncertainty-aware rates/controls and synthetic/no-I/O tests. Worker cannot perform builds,references,solver launches or shared evidence writes. Achieved means observed supported decrease, not assigned accuracy acceptance. Root reviews targetedtests/diff before serializedchecks/commit.

## Full reference checkpoint — 2026-09-27

N192/400 complete01:22:25.900143UTC,elapsed964.265seconds;stage37be5852d54b35843e2826b1a73692a028b6eb6f4870e2d2232cf21fbc206a09,19616469bytes,SHAf2980a9144bb9cc0a2b32c18bf569c4cdce2265b8696c0d7d0f0d58af3fae74d independently checked. N256/400 started01:22:25.931526UTC,stage1d9dd69a4205742a30e08ca4757507e5db8685fdb3f6cd6b6977db73399e7825. Admission pending;12starts consumed,65remain.

## Classifier review correction — 2026-09-27

Root30targeted/202combinedPython/145Rust/12comparator/34contract/fmtClippy passed;207protected unchanged. Reviewed pureclassifiers trustedbindings/intervals/commonmesh controls. Approved narrow within-scope reporting correction: retain conservative overallstatus and perchannel findings, add informative postinitialsummary with boundedexactzero vs unresolvedzeros explicitly counted. Focusedtests and targetedrecheck required; no new mathematicalthreshold or numericalwork.

## 13.4.2
- Status: accepted pure independent classifier, source frozen after reporting correction.
- Added in-memory trustedbinding/schema checks, perobservable rate intervals, floor/control diagnostics and informative summary.
- Exactzero/initialtime findings explicit; unresolvedzeros remain informative-inconclusive.
- Root targeted32tests passed; combined202Python before two focused newtests;145Rust,12comparator,34contract passed.
- Black/Ruff/fmt/Clippy/diff passed;207protectedhashes unchanged.
- Fresh Cargo output wave4a-20260927-01.json; no source/core/helper changes.
- No builds,newanalytical stages or launches in classifier; active13.3.1 references independently monitored.
- API: analyze_convergence(book,records,trusted_bindings); conditional on separately authenticated callerexpectations and explicitboundevidence.
- Rate/floor/control/overall/informativesummary decisions distinct; allaccuracythresholds null and M5-Vopen.
- Immutablehelper constraints for pending13.4.3 in .development/13.4.2/HANDOFF.md.
- Deviations: approved producer decomposition and narrow reporting correction only.

## Immutable convergence integration scope — 2026-09-27

Finalize13.4.3 scope as separateCLI/tests using unchanged accepted public helpers. Dependency now accepted13.3.4/13.4.2; disjoint mocked producer may run during13.3.1 references. No actualdryrun/sharedlocks/builds/references/starts. Authenticated exact completepriorcache reads only, newworkcurrentroot only; corrupted/interrupted evidence must block rather than trigger repeat. Numerical13.4.1 remains dependent on completed13.3.1 and accepted13.4.3 with separateactivation.

## Second full reference checkpoint — 2026-09-27

N256/400 completed01:43:58.199347UTC,checkpointSHAce16a2f697ddf906cf74fa2db44d9cee5c0834ba08686d0aafc32ef28234e176, independently checked. N384/400 started01:43:58.230971UTC,stage4990ce01c6878958f3754dab9136ab9a17ec84c68efdf6d30ea3d632eb73df72. Fulladmission pending; no fullsolverstarts.

## Convergence replay/CLI decomposition — 2026-09-27

13.4.3 analysis identified13distinct units, exceeding12smallsteps. Insert13.4.4 authenticatedcache/run/metrics/classifier/summary helper;13.4.3 retainsbatch/activation/CLI and waitshelper acceptance. Bothmockedonly,immutableexistingproducers. No expandednumericalmatrix or workauthorization.

## Evidence replay micro-plan approval — 2026-09-27

Approved13.4.4 twelve-stepTODO. Replay selectedcompletecache receipt before trustingcallerreferencevalues; validate actualproduceridentities; source-boundreviewedbounds only. Priorinvalidpresent evidence failswithoutfallback/recomputation. Mockedtaskroot work only,noactive13.3.1access. Scientificexecution13.4.1 stillgatedafterbenchmark andproduceracceptance.

## Conditional reference diagnostics clarification — 2026-09-27

Originalpolicy shorthand final650precision/binary400 refers to baselinefinal384/400 branch. Frozenaccepted evaluator optional512/650 branch uses oppositeprecision512/400 and exactbinary-input512/650, one subsetstageeach. Explicitly authorize those existingconditional diagnostics ifsingleoptional escalationneeded; no neworders,variants orrepeats. Preserveoriginalpolicy/hash and attach example3-reference-policy-clarification.json before branchselection.

## 13.4.4
- Status: accepted authenticated convergence evidence helper;40mockedtests independently passed.
- Exactprior/currentcache source bindings and stage safety, immutable receipts, complete-run and independent metrics replay.
- Separately authenticated classifier bindings and reviewedbounds provenance; missingbounds inconclusive.
- Invalidpresentprior evidence rejectsfallback; completedcurrentunmet retained and blocksderive.
- Classification/summary replay authenticated; historicalcount shape only, actualsnapshotauthority delegated13.4.3.
- Root244Python/145Rust/12comparator/34contract/fmtClippy/diff passed;207protected unchanged.
- Fresh Cargo output wave4b1-20260927-01.json; acceptedproducers/core remain unchanged.
- No numericalwork orstarts; active13.3.1 reference monitored separately.
- API/helperSHA and trustboundary: .development/13.4.4/HANDOFF.md.
- Deviations: approved evidence/CLI split only;thresholds null,scientificacceptancefalse,M5-Vopen.

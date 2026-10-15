# State — Orchestration 18

Plan status: **Approved by user on 2026-10-08 (“yes”).**
Execution status: **Resumable as of 2026-10-14. Orchestration 19 is closed (`1c1b9da`, see [STATE_19.md](STATE_19.md)).** Pending tasks carry `Amendment 2026-10-14 (Orchestration 19)` blocks in [EXECUTION_PLAN_18.md](EXECUTION_PLAN_18.md); read them before dispatch. Baseline at resume: workspace 279 passed / 1 ignored, Python 291 OK. Previously paused 2026-10-14 for Orchestration 19, after 18.7.8. Waves 1–6 complete; Wave 7 active. 18.7.4 was accepted 2026-10-12, 18.7.1 on 2026-10-13 (`49192d7`), 18.7.3 on 2026-10-14 (`716a7ea`), and 18.7.8 on 2026-10-14 (fix `fc8bf8d`, merge `60ee1b5`). Pending at resume: 18.7.7, the fixture tasks 18.7.5 → 18.7.2/18.7.6 (blocked on Fortran fixtures only; the Python suite runs, 291 OK), then Waves 8–16 as amended by 19.7.2. Resume only after Orchestration 19's Wave 7 ([STATE_19.md](STATE_19.md)). Accepted Wave 5 work includes 18.5.1 and 18.5.3; 18.5.2 is incomplete and superseded by 18.7.1. Wave 3 fixtures authenticate raw outputs only; normalized parity remains pending. The default backend must not change before the full parity gate.
Plan: [EXECUTION_PLAN_18.md](EXECUTION_PLAN_18.md). Worker template: [PROMPT_TEMPLATE_18.md](PROMPT_TEMPLATE_18.md).

## Baseline and guardrails

- [x] Reconnaissance completed 2026-10-08; preserve existing uncommitted planning/review work.
- [x] `cargo test --workspace`: 137 integration tests passed; zero unit/doc tests.
- [x] `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: 274 tests passed.
- [x] `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check`: passed.
- [x] Wave 2 global checks: `cargo test --workspace` passed (137 integration tests); Python unittest discovery passed (290 tests); fmt, Clippy with warnings denied, and diff check passed.
- [x] No new Fortran build/run during planning reconnaissance; current runner/comparator remain Example 3-specific.
- [x] User decisions recorded: all modes reachable through the audited solute caller path in this orchestration; inactive source helpers are identified, not treated as executed modes; default source-compatible backend after parity gate; preserve FV/BE as explicit alternative; field-specific parity criteria; up to three workers and uncapped diagnostic follow-ups subject to worker escalation.
- [x] Independent spec and standards reviews completed; plan corrected for verification-before-default ordering, task scopes, and escalation rules.
- [x] Wave 2 integration review found the Fortran runner authenticates raw files only; plan amended to add fixture observation maps, a strict output normalizer, and paired Rust/Fortran CLI stages before all-mode acceptance.
- [x] Wave 2 raw-output authentication is not labeled normalized parity; output normalization and candidate comparison are separate gates before all-mode acceptance.
- [x] User selected full pinned HYDRUS runs for dual-domain fixtures: use models 6/7 with independently authored decks and active `InitDualPor` state initialization; no call-boundary injection harness. **Correction 2026-10-10:** fixed-form reading shows `DualPor` is actively called; dynamic transfer is in scope, and retained fixtures require revalidation for exchange discrimination.
- [x] User approves this execution plan.
- [ ] Full audited solute compatibility/parity gate passes before the default changes.

## Wave 7 sequencing and worker-tier amendment — 2026-10-13

- Wave 7 is now three sequenced sub-waves because `types.rs`/`adapter.rs` are single-owner files that 18.7.1, 18.7.3 and 18.7.8 each must extend: **7A** 18.7.1 ∥ 18.7.6, **7B** 18.7.2 ∥ 18.7.3, **7C** 18.7.5 ∥ 18.7.7 ∥ 18.7.8. 18.7.3 and 18.7.8 specs were re-scoped in `EXECUTION_PLAN_18.md` (escalation rule 4: shared-state additions are a decomposition gap, not ad hoc permission widening).
- Green baseline re-recorded at `726662f`: `cargo test --workspace` — 179 passed, 0 failed; `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` — 291 passed, OK.
- Worker-tier constraint: the `r9700` local server (`llama-cpp.service`, port 8001) is in a crash loop (`invalid device: ROCm0`, restart counter >4000), so the planned cheap/parallel worker tier is unavailable. Wave 7 workers run one at a time on the single reachable agent endpoint (`strata/qwen3.8-flash-next-unsloth-ud-iq4_xs`, the same engine as the orchestrator), which also caps practical concurrency below the approved three.
- Fixed-form re-verification by the orchestrator: `src_reference/WATFLOW.FOR:44–46` (`call DualPor`) and `WATFLOW.FOR:1067–1068` (`WTransf` accumulation) are column-6 continuations, i.e. executable. The 18.6.6 correction stands.

## Blocker review — 2026-10-14 (resume after Orchestration 19)

- The "blocked on Fortran fixtures" label on 18.7.2/18.7.5/18.7.6 is stale and is cleared. `gfortran` is on PATH and `run_fortran_reference.py --smoke --task-root .development/orch18-resume-smoke` built the pinned source and completed (its `SMOKE_RUN.md` rewrite was reverted). 18.7.2 and 18.7.5 are Rust-only kernel tasks with no fixture input; 18.7.6 and 18.7.7 author fixtures with the working runner.
- Dispatch (concurrency cap 3): 18.7.2 ∥ 18.7.5 ∥ 18.7.7, then 18.7.6.
- 18.7.2 plan review: `iMoistDep` is only ever 0 or 1 (`INPUT.FOR:1320`, `:1693`), so `MoistDepIn` (`HYDRUS.FOR:279`, gated on `.eq.2`) is unreachable. `adapter/reactions.rs:100` still accepts selector 2; the follow-up is to reject it, owned by the moisture task. Spec amended so 18.7.2 may add dead-code allowances in `compat/mod.rs`.

## Wave 7 closed — 2026-10-14

- Gate at `fc4f050`: `cargo test --workspace` 303 passed, 0 failed, 1 ignored; Python unittest 291 OK; fmt, clippy `-D warnings` and `git diff --check` clean.
- Follow-ups: a `dual_exchange` alias in `FIXTURE_GROUPS`; a typed field for the exchange coefficient; adapter rejection of moisture selector 2; fixture workers must run the adapter deck-walk tests before reporting.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 1 | 18.1.1 | Source mode matrix and compatibility contract | done |
| 2 | 18.2.1 | Manifest-driven isolated Fortran runner | done |
| 2 | 18.2.2 | Source-only parity orchestration/comparator | done |
| 3 | 18.3.1 | Transport/weighting/boundary fixtures | done |
| 3 | 18.3.2 | Reaction/sorption fixtures | done |
| 3 | 18.3.3 | Optional transport-property fixtures | done |
| 3 | 18.3.4 | Nonequilibrium/dual-domain fixtures | done |
| 3 | 18.3.5 | Biology/filtration/deposition fixtures | done |
| 4 | 18.4.1 | Compatibility types/state/precision model | done |
| 5 | 18.5.1 | Transport and matrix assembly kernel | done; fixed-form corrections accepted |
| 5 | 18.5.2 | Endpoint rows and flux closure | superseded/incomplete; replaced by 18.7.1 |
| 5 | 18.5.3 | Source accounting and observations | done; fixed-form corrections accepted |
| 6 | 18.6.6 | Fixed-form reachability re-audit | done; consistency review accepted |
| 7 | 18.7.4 → 18.7.1–18.7.3, 18.7.5–18.7.8 | Selector mapping first, then endpoint/active kernels and fixtures | done 2026-10-14 (18.7.1–18.7.9) |
| 7A | 18.7.1 | Primitive-input endpoint equations | done (commit `49192d7`) |
| 7A | 18.7.6 | Dual-exchange fixtures | done (`80b7909`, repaired `fc4f050`) |
| 7B | 18.7.3 | Dynamic dual-porosity transfer kernel | done (commit `716a7ea`) |
| 7B | 18.7.2 | NEquil/SorbConc/rMD kernels | done (`05ae993`) |
| 7C | 18.7.5, 18.7.7, 18.7.8 | Start-of-step MassTran, FluxConc fixture, source coupling | done (`5b17a70`, `2baeac0` repaired `fc4f050`, `60ee1b5`) |

## Active tasks

- Wave 7 has no open tasks. The old note that 18.7.7/18.7.5 were the root-uptake producer and fixture had mislabelled them: in `EXECUTION_PLAN_18.md` they are the FluxConc fixture and the start-of-step MassTran call, both done. No plan task ports the `SINK.FOR` root-uptake producer. 18.7.8 takes its state as primitive inputs, and it remains a gap to decide on before the 18.10.1 driver.
- No task may add a second `DualPor` state type: `DualPorPrimitiveState` and
  `DualPorMaterial` (added by 18.7.3) are the accepted shape.
- 18.7.6: blocked on Fortran fixtures only. The interpreter blocker recorded
  under Verification is stale: on 2026-10-14 system `/usr/bin/python3` 3.12.3 ran
  `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`
  with 291 tests OK and left the tree clean. Grep the output for `^OK|^FAILED|^Ran`,
  because the suite prints solver JSON.

## Completed tasks
- 18.7.2: **accepted 2026-10-14** (`05ae993`). It squashes worker commit `50d664c` with an orchestrator fix that adds `#[allow(clippy::assign_op_pattern)]` to `sorb_conc`, which keeps source order and follows the `assembly.rs` convention. It ports `nequil` and `should_call_nequil` (nonequilibrium.rs) plus `sorb_conc`, `MoistureFactorTable::walker` and the three Coeff `rMD` factor families (processes.rs). The two `mod.rs` dead-code allowances follow the amended spec. Tests: nonequilibrium 10, processes 9; `solute_fortran_compat` 138/0 in the worktree. Expected values come from an independent transcription in `.development/18.7.2/ref_nequil.py`. Gotchas for 18.9.2 Coeff: it must pass the immobile-form `GamLi`/`GamL1i`/`GamSi`/`GamS1i` into NEquil's `GamL`/`GamL1`/`GamS`/`GamS1` (verified at `SOLUTE.FOR:631-641`). `lFiltr` takes caller-supplied `rKa1`/`rKa2` from `Deposit` (18.9.5). The test `moist_dep_in_route_unselected` records selector 2 as unselected.
- Wave 7 integration finding: merging 18.7.6/18.7.7 broke three adapter deck-walk tests. Their requests used fields the strict adapter rejects (`top_pressure_head`, `flux_concentration_selector`, `observation_nodes`, an old-style `ordered_species`), and the case count is hard-coded at 38, now 44. Repaired by 18.7.9 (`fc4f050`), which made `ordered_species` structured records with values from SELECTOR.IN. For lFlux, `time.interval` is marked unavailable because the deck is adaptive with dt0=0.1 h. In the exchange pairs the transfer coefficient (0.1/0.0 for model 6, 0.01/0.0 for model 7) appears only in the free text of `dynamic_exchange_sink.reason`, because no typed field may carry it (`dual_porosity_exchange_state` must stay absent). The request hashes in each pair now differ. Follow-up for a later task: give the coefficient a typed field.
- 18.7.6: **accepted 2026-10-14** (`80b7909`, cherry-picked from worker commit `4a0b4c0`). It adds `processes/dual_exchange/model{6,7}_{exchange,no_exchange}`. The decks start with node 1 at h=0 and the rest at -100, a wetting transient. Each pair differs only in the transfer coefficient on `SELECTOR.IN` line 27 (0.1 vs 0.0); the orchestrator diffed them. `W-volumeI[L]` rises over t=0,1,2,4 for model 6 (0.345→0.450) and model 7 (0.163→0.188) and stays constant in the controls. The 18.3.4 runs do not discriminate because they start at equilibrium, and their "WATFLOW call commented out" text is stale. Final `Conc`/`Sorb` are uniform, so `W-volumeI[L]` is the only discriminator, and `SinkIm` is marked unavailable. `dual_exchange` has no `FIXTURE_GROUPS` alias yet; that is a runner follow-up. The deck title lines say "Task 18.3.4" (cosmetic). The orchestrator re-ran `model7_exchange`, which authenticated. Evidence notes are in main `.development/18.7.6/`. Raw only, no parity claim.
- 18.7.7: **accepted 2026-10-14** (`2baeac0`, cherry-picked from worker commit `a53f96d`). It adds the `flux_concentration/lflux_on` and `lflux_off` fixtures, whose decks differ only in `SELECTOR.IN` line 12 (`lFluxes`). To make `FluxConc` distinguishable from resident concentration they use dispersivity 0.5, gravity flow and observation nodes 3, 5 and 7. The flag adds 6 columns to `solute1.out` (23 vs 17) and puts flux in place of temperature in `OBS_NODE.OUT` columns 4, 8 and 12, even though the header still says "Temp". Only the cumulative columns 19, 21 and 23 carry the `FluxConc` result, and their per-step increments differ from resident `vNew*Conc` by up to 0.046. The direct per-node `FluxConc` value is never printed, so the map marks it unavailable and the limitation is recorded in `EVIDENCE.json`. The orchestrator re-validated both manifests and re-ran `lflux_on` in a fresh root, which completed. Raw authentication only, no parity claim.
- 18.7.5: **accepted 2026-10-14** (`5b17a70`, cherry-picked from worker commit `a082c47`). Adds `MassTotals::accumulate_step_start(level, iter, old, alf)` for `SOLUTE.FOR:135-140`: it runs only when `Level==1 && Iter==1`, uses the old-level snapshot and the weight `alf`, and returns whether it ran. `accumulate` is now a wrapper over a private `accumulate_weighted`, so both calls share one body. STrans is assigned in the two-site branch, so the end-of-step call is its final writer, while the cv* totals add up across both calls. Accounting tests 12/0 (5 new) and Clippy 0 warnings, re-run by the orchestrator. The driver (18.10.1) owns the call. No parity claim.
- 18.7.8: **accepted 2026-10-14** after review. The fix is `fc8bf8d` ("fix(solute):
  clear root solute sink once per SetSSnk call") and the merge into
  `phase4-water-acceptance` is `60ee1b5`. The review rejected the first `SinkS`
  finding below. `SINK.FOR:174-176` clears `SinkS` once, before `do 12 iStep` at
  `:178`, so step 2 (nStep=2) and step 3 add their active term onto the step-1
  passive sink. Four tests were corrected and one discriminating test was added.
  Line citations were corrected (e.g. `Compen=1.` is `:163`; label `12` closes at
  `:218`). The `ThC` tie gives `0.40000004` for both the double and the
  single-precision forms, so this fixture does not discriminate them. After the
  fix, `cargo test -p hydrus-core --lib solute_fortran_compat` gives 81/0 and the
  workspace 226/0; fmt, clippy and diff-check are clean. The original report
  follows, superseded where it conflicts. It was implemented in worktree
  `.development/18-worktrees/18.7.8` (branch `orchestration/18.7.8`, parent `4d151a1`).
  `source_coupling.rs` (new, 1192 lines) ports the two call targets of
  `SOLUTE.FOR:94-96`: `SetSSnk` (`SINK.FOR:150-232`) as `root_solute_uptake`, and
  the `if(lBact.and..not.lWat)` target `Exclusion` (`SOLUTE.FOR:1715-1773`) as
  `size_exclusion`. `types.rs` gains `RootUptakePrimitiveState` /
  `SizeExclusionPrimitiveState` and `adapter.rs` maps `SinkF`/`lActRSU`/`lWat`/
  biology state plus the sink and rate arrays, with focused adapter tests.
  Missing producer state rejects before mutation; nothing substitutes zero.
  Fortran-parity findings recorded by this task, each pinned by a test:
  `SINK.FOR:175-176` clears `SinkS` **inside** the step loop, so steps 2 and 3 do
  not accumulate on step 1; `Omega1` is assigned only at `SINK.FOR:202` under
  `SAUptakeP.gt.0.` and read at `205-206`, so a step-2 read is genuinely
  undefined and the port rejects rather than defaulting to `0.`; the `182` test
  orders `i.eq.N` first, which makes the `dx` weights for
  `THREE=[0,-10,-20]` the non-symmetric `[+5,-10,-5]`; `SOLUTE.FOR:1723`'s
  `1.e-20` is a **single-precision** literal under default real typing, so the
  guard compares against `9.999999682655225e-21`; and `1742`'s
  `ThC=ThNew(1)-ths*sw_c` is a double expression narrowed once, which on the
  fixture lands on an exact f32 tie and round-half-to-even returns `0.40000004`
  where both the decimal `0.4` and the single-precision rewrite
  `ThNew(1)-th_c` are wrong. `1756-1763`'s `else` is `krw=0.` **and**
  `VelC=vNew(1)`, a direct assignment: hoisting `1759`'s division above the branch
  (the shape an `if/else`-returning-tuple refactor produces) divides by that zero
  and writes NaN into every node. A non-finite `ThC`/`VelC` is rejected rather
  than broadcast, because `1765-1768` copies it to every node of `ThNew`, `ThOld`,
  `vNew` and `vOld`.
  Verification: `cargo test -p hydrus-core --lib` 80 passed / 0 failed (adapter 7
  → 17, `source_coupling` 0 → 17), `cargo test --workspace` 27 suites ok,
  `cargo clippy --all-targets` clean, `cargo fmt --check` clean. No parity claim:
  18.7.5/18.7.7 and the driver still gate these paths on Fortran fixtures.

- 18.7.3: done (commit `716a7ea`, "feat(solute): port dynamic dual-porosity
  exchange", parent `e724f38`, worktree `.development/18-worktrees/18.7.3` on
  branch `orchestration/18.7.3`, merged and removed). `dual_domain.rs` ports
  `subroutine DualPor` (`WATFLOW.FOR:1027-1075`) in statement order — `WTransf=0.`
  (1034), `SeIm` (1037), the `iDualPor=1` water-content pool (1039-1042), the
  `iDualPor=2` pressure-head pool with `FH`/`FK` (1044-1053), the top-node
  `hCritA` guard (1054-1055), both `TrMaxIm` clamps as sequential assignments
  (1057-1066), the `ThNewIm` clamp (1068), and the live `if(i.ge.2)` trapezoid
  (1070-1071) — with private `retention_head`/`conductivity` reproducing `FH`
  and `FK` at `iModel=0` including the `implicit double precision (A-H,O-Z)`
  widening, `sngl()` narrowing, and `-1.d300**(1.d0/n)` parsed as
  `-(1d300**(1/n))` (`MATERIAL.FOR:61`, `67`, `367`). `types.rs` adds
  `DualPorMaterial` (`ParD(1)/(2)/(6)/(7)/(8)/(9)` plus optional model-7-only
  `ParD(10)/(11)`), `DualPorPrimitiveState` (`x`, `ThOld`, `hNew`, `ThOldIm`,
  `MatNum`, `ParD`, `hCritA`, `dt`, `iModel`, `iDualPor`) and
  `NonequilibriumInputs::dual_porosity_exchange_state`; `ThNewIm` is an output
  because the routine writes it (1068) and only `Update` reads it (1112). Adapter
  7 → 11 tests, `dual_domain` 0 → 11 tests, all rejections assert. Three review
  corrections were applied at the gate: `thr`/`ths` derive from `ParD(1)/(2)`
  through `GenMat` (`INPUT.FOR:762`, `796-797`, `816-817`, called at
  `HYDRUS.FOR:219`) rather than being accepted as request keys; `FH`'s third
  argument is `real Par(10)`, so `ParD(7,M)` aliases `Par(1..5)` with no explicit
  shuffle (unlike `FK`, which gets the `1045-1050` assembly); and the
  `ParD(8,M)<=ParD(7,M)` case is rejected as a documented contract rejection
  instead of dividing by zero at 1037. The worker's call-site argument list in
  the module doc was wrong and is corrected in the merge; the real list is
  `(NumNP,NMat,MatNum,iDualPor,ThOld,ths,thr,ThNewIm,ThOldIm,ParD,SinkIm,dt,
  iModel,hNew,hCritA,x,WTransf)` and is positionally identical to `1027-1029`.
  `pinned_requests_cannot_supply_the_dual_porosity_exchange_state` walks the
  pinned decks and asserts the blocker: no request carries node state.
- 18.7.1: done (commit `49192d7`, "feat(solute): derive endpoint rows from
  source primitives", parent `79b26af`, worktree
  `.development/18-worktrees/18.7.1` on branch `orchestration/18.7.1`, merged
  and removed). `boundary.rs` derives the node-1 and node-N equations of both
  `MatSet` levels and the single `cvBot`/`cvTop` pair per species and step from
  primitives only: the `iDualPor.eq.0` water reduction with `.and.` precedence
  (768-776), `g0`/`g1`/`Disp`/`Retard` at both levels (781-800, 869-902), the
  strict `vN(1).gt.0` and `vN(N).lt.0` predicates, the vapor zero-resistance
  exception (811), the `kTopCh.le.0`-only guard on the Level-1 `F(N)` (880), the
  `kTopCh <= 0` prescribed term including `-2` (915), the unit `TLevel` coefficient
  (201), `rMin` (224-225), and unsupported codes rejected instead of leaving
  stale `D/E/F`. `EndpointPrimitiveState` now carries `ChPar((jS-1)*16+slot, M)`
  as per-species/per-material records, flat 1-D `TDep`, both `TempO(N)` and
  `TempN(N)`, and separate matrix (`vCorr`-free) and closure velocity families;
  `old_accepted_flow`, `new_accepted_flow`, `old_accepted_concentration`,
  `henry_coefficient` and `gas_diffusion` are deleted because they are not
  source arguments. A pre-existing defect that validated `ChPar(4,M)` per node
  instead of per material is fixed and tested. Boundary 9 -> 14 tests, adapter
  5 -> 7 tests.
| 8 | 18.8.1–18.8.2 | Conditional initialization and selector-2 audit resolution | done |
| 9 | 18.9.1–18.9.5 | Source-faithful process kernels | done |
| 10 | 18.10.1 | Source-ordered solute step driver | pending |
| 11 | 18.11.1 | Nonlinear iteration and convergence | pending |
| 12 | 18.12.1 | Expose compatibility candidate; keep current default | pending |
| 13 | 18.13.1 | Normalize raw Fortran outputs using fixture observation maps | pending |
| 14 | 18.14.1 | Wire paired Fortran/Rust execution and comparison | pending |
| 15 | 18.15.1 | All-mode parity report and acceptance | pending |
| 16 | 18.16.1 | Switch default after accepted parity; retain FV/BE | pending |
| 17 | 18.17.1–18.17.2 | Source-defect probes for modern HYDRUS-1D (added 2026-10-14) | pending |

## Verification — Wave 7 in progress

- At `49192d7` (18.7.1 merged): `cargo test --workspace` 186 passed, 0 failed;
  `cargo test -p hydrus-core --lib solute_fortran_compat` 41 passed, 0 failed
  (boundary 14, adapter 7, transport 8, other 12); `cargo fmt --all -- --check`
  clean; `cargo clippy -p hydrus-core --all-targets` emits zero warnings (the two
  pre-existing lib warnings at the baseline are gone and none were introduced);
  `git diff --check` clean; exactly one commit; worktree removed;
  `.development/**/target/` untracked and ignored.
- Baseline re-recorded at `726662f`: `cargo test --workspace` 179 passed, 0
  failed; Python unittest suite 291 tests, OK.
- **BLOCKER for the Wave 7-end gate:** the recorded Python interpreter
  `/home/andrew/miniconda3/envs/hydrus_rust/bin/python` — and the whole
  `/home/andrew/miniconda3` tree — no longer exist on this host, and the
  repository has no `.venv`, so the 291-test Python suite cannot be re-run.
  System `/usr/bin/python3` (3.12) cannot even discover it (`TypeError: expected
  str, bytes or os.PathLike object, not NoneType`). 18.7.1 changed no Python or
  fixture file (`git show --stat 49192d7` covers three Rust files and the task
  TODO), so its Python baseline is unchanged; later Wave 7 tasks that touch
  `tools/**` cannot be gated until the interpreter is restored.
- State-file integrity note: mid-session (2026-10-13 ~20:11) `planning/STATE_18.md`
  in the working tree was replaced by an older revision that had no Active
  tasks/Completed tasks/Verification sections; `79b26af` committed that revision
  plus the sequencing amendment. The sections above are re-established from the
  session record; no other planning document was affected.
- At `716a7ea` (18.7.3 merged): `cargo test -p hydrus-core --lib
  solute_fortran_compat` 57 passed, 0 failed (`dual_domain` 11, `adapter` 11,
  `boundary` 14, `transport` 8, other 13); `cargo test --workspace` 202 passed,
  0 failed; `cargo fmt --all -- --check` clean; `cargo clippy -p hydrus-core
  --all-targets` zero warnings; `git diff --check` clean; one task commit, no
  `target/` path staged. Python was not rerun: the commit touches three Rust
  files and the task TODO, and the adapter's deck walk is read-only, so the
  Python baseline is unchanged. This does not waive the Wave 7 Python gate.
- Reader-patch fact (settled while reviewing 18.7.3): there is **no patched
  Fortran reader in this repository.** No `.f90` file exists anywhere outside
  `target/`, and `reference/fortran_reference/PATCH_NOTES.md` shows the pinned
  gfortran portability patch rewrites only `HYDRUS.FOR` and `TIME.FOR`. Later
  tasks must read material and flag provenance from `src_reference/INPUT.FOR`
  itself and stop assuming a `patch/INPUT.f90` variant.
- Worker-lane note: both Wave 7 workers hit the 30-minute async wall while
  sitting at the micro-plan gate (`cafcf458`, then `06d8b508`). Resuming with an
  implementation-only directive works; keep analysis phases time-boxed and ask
  for an interim `git diff --stat` instead of idling. If a third lane stalls the
  same way, run the remaining Wave 7B/7C kernels interactively in the
  orchestrator session.

## Task reports

### 18.1.1 — Source behavior and parity contract

- **Status:** done; created `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` and `planning/tasks/18.1.1.todo.md`.
- Inventoried active and inactive source paths, caller/input mappings, mode constraints, source precision, normalized inputs, trace checkpoints, output semantics, tolerance policy, and default-switch gate.
- Based on the audit, the execution plan now gates parity on every active/callable solute mode and records commented/unreachable helper algorithms without treating them as executable modes.
- Spot-checked `INPUT.FOR` flag initialization and commented helper calls in `SOLUTE.FOR`/`HYDRUS.FOR`; no Fortran execution or code changes.
- `git diff --check` passed; untracked outputs also checked for trailing whitespace. No deviations; precision/compiler and active-mode gaps are documented.

### 18.2.1 — Generalized isolated Fortran fixture runner

- **Status:** done; updated `tools/run_fortran_reference.py` and its tests; completed `planning/tasks/18.2.1.todo.md`.
- Manifest-defined fixtures now use unique build/run roots, return logical output paths and hashes, and retain source/build/input/output provenance. Incomplete runs do not produce a complete manifest; optional timeout defaults to unbounded.
- Existing Example 3 `RUN.md` is preserved byte-for-byte; smoke and Example 3 command forms remain available.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_fortran_reference` — 14 passed; `git diff --check` passed. No Fortran launch or deviations.

### 18.2.2 — Raw source-only parity CLI and comparison helpers

- **Status:** done; created `tools/run_solute_parity.py`, its tests, and `planning/tasks/18.2.2.todo.md`.
- CLI supports one manifest or serial fixture groups with unique child roots. Reference-only results authenticate raw files and are explicitly **not normalized parity**; pure comparison helpers require exact field/location identity and report first divergence.
- Targeted parity tests — 10 passed; `tools.test_solute_printed_parity` — 10 passed; `tools/compare_solute_reference.py --self-test` — 12 passed.
- Mocked end-to-end runner/group integration verified the shared result contract, nested roots, output paths and hashes; no real Fortran/Rust solver was launched. `git diff --check` passed.
- Provenance comparison uses the runner's exact staged inventory (archive plus `.FOR` source files); unrelated PDF hashes in `SOURCE_MANIFEST.json` are excluded, with regression tests for missing/changed staged entries.

### 18.3.1–18.3.3 — Reference fixture groups

- **Status:** done; fixture files and task reports committed in `85f8bd6`, `292a70c`, and `3210f5d`.
- 18.3.1: 18 transport fixtures cover all 14 active endpoint-code × native-flow combinations; `kTopCh=-2` positive/negative cases are included. Raw outputs authenticated; no normalized parity claim.
- 18.3.2: four reaction/sorption fixtures authenticated; all maps converted to flat `solute-observation-map-v1`. Per-map counts: 60 observations for each chain fixture, 22 for the single-species case; retained inputs and outputs unchanged.
- 18.3.3: five property fixtures authenticated, including vapor-enabled diagnostics. Density and moisture-dependent paths are documented inactive from the active caller/input path.
- Three workers ran in separate worktrees; all run attempts remain in `.development/18-worktrees/18.3.{1,2,3}/.development/`. Each task's exact hashes and output inventory are in its TODO and retained manifests. Reaction maps use the versioned flat observation schema; their input/output provenance was rechecked after conversion without rerunning.
- Orchestrator check across imported fixtures: all 27 `CASE.json` inputs and observation maps validate. Acceptance is raw output authentication only; normalization and paired parity remain pending.

### 18.3.4 — Nonequilibrium/dual-domain fixtures

- **Status:** done under the 2026-10-09 user-approved full-run amendment; fixtures/task report committed in `b5ef3ee`.
- Added five fixtures: mobile-immobile and matched control, dual models 6 and 7, and single-domain control. Both water models are accepted through the pinned active reader and maps include printed `VolumeIm` evidence.
- Full pinned reference-only run authenticated all five cases under `.development/18.3.4/attempt-20261009-unsaturated/`; orchestrator revalidated all five run manifests, declared output hashes, pinned source/archive/patch/executable provenance, fixture inputs, and observation maps.
- Discriminators: dual model 6 `VolumeIm=0.33455E+00 cm`; model 7 `0.15740E+00 cm`; single-domain control omits the `W-volumeI[L]` row. Mobile-immobile `cvCh1` at 0.1 h differs from its zero-immobile control (`-0.30400E-01` vs `-0.34400E-01`). These are raw printed outputs, not normalized parity.
- Active `InitDualPor` initializes `ThNewIm`/`ThOldIm`, and `Solute` consumes dual storage. **Correction 2026-10-10:** the earlier statement that `WATFLOW.FOR:42–46` comments out `call DualPor` was wrong; the call is an executable fixed-form continuation under `if(iDualPor.gt.0)`. Dynamic exchange is active-path scope and retained fixtures need revalidation for nonzero `SinkIm` discrimination. The continued `NEquil`, `SorbConc`, and `NonEqInit` calls are also conditionally active, as documented by the 18.6.6 audit.

### 18.3.5 — Biology/filtration/deposition fixtures

- **Status:** done; committed in `cf651ff`.
- Cases discriminate blocking codes 1–5 (including source no-op code 3) and filtration/deposition with valid positive particle/grain diameters. Reader mappings for bacteria/filtration and `ChPar` values are present. `Exclusion` has a conditional continued caller and is untested; external state mapping remains a blocker.
- Reference-only run authenticated all six raw output inventories and source/archive, patch, executable, request, and input hashes under `.development/18.3.5/attempt-20261009T000000Z/`; no normalized parity claim.
- Orchestrator contract validation after 18.3.4: all 38 manifests and versioned observation maps validate; all five new maps have unique field/location identities. New attempts remain raw authentication only, not normalized parity.

### Wave 3 verification

- `cargo test --workspace` — passed (137 integration tests; zero unit/doc tests).
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` — 291 passed; one expected guarded “solver execution forbidden” diagnostic was emitted inside a passing test.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` — passed.
- The five 18.3.4 retained raw-run manifests, declared output hashes, pinned source/archive/patch/executable provenance, and fixture input hashes were independently revalidated. No normalized parity claim.

### 18.4.1 — Compatibility types, strict adapter, and precision model

- **Status:** done; implementation and task report committed in `bce44da`.
- Added private `solute_fortran_compat` types for source precision, units/availability, switches, boundaries, species/process fields, trace data, and source-index mappings. Added compileable placeholder modules for later kernels; the existing FV/BE API and dispatch remain unchanged.
- Added `serde_json` and `sha2` for serialized fixture request parsing and canonical hash verification. Adapter maps both accepted schemas across all 38 Wave 3 manifests, rejects duplicate/unknown keys and invalid units/shapes/modes, and keeps unavailable values explicit.
- Coordinates in current fixtures provide range metadata, not numeric arrays; the adapter preserves that metadata and explicit source permutation but does not claim physical coordinate values. No solver parity claim.
- Targeted tests: `cargo test -p hydrus-core solute_fortran_compat::types::tests` — 4 passed; `cargo test -p hydrus-core solute_fortran_compat::adapter::tests` — 4 passed, including 38-fixture mapping/hash sweep.

### Wave 4 verification

- `cargo test --workspace` — 8 unit and 137 integration tests passed; zero doc tests.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'` — 291 passed; one expected guarded solver-execution diagnostic was emitted inside a passing test.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` — passed.

### Wave 5 source-reading amendment — 2026-10-10

- Source review found a fixed-form parsing error in all three uncommitted Wave 5 implementations: `!` in physical column 6 is a continuation marker, not a comment. Relevant executable continuations include `MatSet` Level-1 interior/endpoint RHS, old-level boundary contributions, and `MassTran` `cvChIm` branch equations.
- The plan requires fixed-form interpretation and source-derived tests; 18.5.3 covers reachable `cvChIm`. The later audit found further conditional calls, now assigned explicit follow-on tasks.
- Earlier targeted test passes (18.5.1: 5 transport + 4 assembly; 18.5.2: 3 boundary; 18.5.3: 4 accounting) are superseded as acceptance evidence because tests encoded the mistaken source interpretation.
- Corrected 18.5.1 and 18.5.3 were independently re-reviewed and integrated; 18.5.2 remains preserved in its worktree as incomplete and is superseded by 18.7.1.
- Fixed-form caller audit corrected reachability for `DualPor`, `FluxConc`, helper updates, initialization, accounting, moisture factors, root sink, exclusion, and nonlinear convergence. Source-callability remains distinct from runtime-selected coverage; see the current matrix and task DAG.
- 18.5.2's correction attempt added direct bottom flux evaluation and orientation tests, but its row/RHS/flux API still accepts opaque precomputed aggregates; it is not accepted and has been replaced in the plan by 18.7.1.

### Wave 5 verification — 2026-10-12

- Accepted and committed 18.5.1 (`fda4f38`) and 18.5.3 (`4fe1d3d`) after fixed-form/source-order review; Clippy correction commits are `6394d52` and `ee1c38c`.
- Targeted tests: transport 5, assembly 4, accounting 7 passed. `cargo test --workspace` passed (24 unit, 137 integration; zero doc tests); Python discovery passed (291 tests); `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed.
- Clippy allowances preserve exact `BanSol` subtraction and `Deposit` source literal/argument order. No normalized or cross-language parity claim.
- 18.5.2 remains incomplete and superseded. Wave 6 task 18.6.6 remains under consistency review.

### Wave 6 consistency review — 2026-10-12

- 18.6.6 evidence distinguishes syntax from runtime selection and identifies conditional callers for beginning/end `MassTran`, `NEquil`, `SorbConc`, `MassInit`, `NonEqInit`, nonlinear convergence, moisture factors, `SetSSnk`, and `Exclusion`; selector-2 `MoistDepIn` remains unresolved/unselected by the documented flag mapping.
- Consistency review reconciled the matrix and task DAG: Wave 7 uses unique task IDs 18.7.1–18.7.8, Wave 8 owns initializers and selector-2 resolution, and downstream numbering/dependencies align. Dynamic dual-exchange fixture discrimination remains untested.
- No implementation, runtime selection, normalized parity, or default change is claimed.

### Wave 2 verification

- `cargo test --workspace` — 137 integration tests passed; Python unittest discovery — 291 tests passed; fmt and Clippy with warnings denied passed.
- This wave establishes isolated raw Fortran execution and parity plumbing only. Fixture-specific normalization and paired Rust comparison remain planned in Waves 3 and 10–11; no parity claim is made here.

The orchestrator owns wave verification and state updates. Preserve all pre-existing working-tree changes. A blocker requiring out-of-scope edits requires a plan amendment.

## 18.8.2 — Resolve remaining moisture-dependent selector path

- Status: done (3659398). Doc-only; no runtime change.
- Evidence: only `iMoistDep` assignments are `INPUT.FOR:1693` (=0) and `INPUT.FOR:1320` (`if(lMoistDep) iMoistDep=1`); `INPUT.FOR:1330` writes it, `:1655` is a comment; `SOLUTE.FOR` uses are `.gt.0` guards/pass-through. Sole `MoistDepIn` caller `HYDRUS.FOR:279–281` gated on `.eq.2` → selector 2 unselected; `MoistDepIn` not implemented.
- Files: `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` (matrix row + follow-on note), `planning/tasks/18.8.2.todo.md`. Grep log `.development/18.8.2/`.
- Follow-up: adapter still accepts moisture selector 2 (`adapter/reactions.rs:100`); needs a rejection task.

## 18.8.1 — Port conditional source initializers

- Status: done (a727cae). New `initialization.rs` (registered dead-code until 18.10.1): `MassInit`, `NonEqInit`, `cInit`/`SolMass`/`ZBRAK1`/`ZBRENT1` in f32 source order; `initialize_sources` reproduces `HYDRUS.FOR:279–288` gating (iDualPor>0 forces lEquil false; MassInit before NonEqInit, which sees converted Conc).
- Tests: `cargo test -p hydrus-core solute_fortran_compat::initialization::tests` 10/10.
- Source quirks pinned by tests: `xKs2` reuses `rKa1/rKd1`; `Par(6)` uses `TDep(jjj+7)`; `fExp` no TDep; MassInit reads TempN, NonEqInit TempO; Conc<=0 skipped. PAUSE paths → `KernelError`; validate before mutation.
- Deviations: `needless_range_loop` allows (source index order); expected values hand-computed (planned independent script not written).
- For 18.10.1: kernel takes own `pub(crate)` records `InitConfig`/`InitState`/`InitFlags`; driver maps adapter state into them.

## Wave 8 closed — 2026-10-14

- Gate at a727cae: Rust 313 passed / 0 failed / 1 ignored; Python 291 OK; fmt, clippy -D warnings, diff-check clean.

## Wave 9 — Source-faithful process kernels (closed 2026-10-14)

Ownership ruling: 18.9.1 owns the complete `Coeff` g0/q0/g1/q1 composition
(SOLUTE.FOR 643-730, all branches) and all isotherm factors (560-622). The other
kernels supply only the scalars that composition consumes.

- 18.9.5 `f752466`: `deposition.rs` holds the canonical `blocking`, `deposit` and
  `deposit_nodes` (7 tests). Integer exponents (`**5`, `**6`, `Dp**2`) use `powi`.
  iPsi=3 is ACTIVE: SOLUTE.FOR 1576-1579 are column-6 continuations.
- 18.9.1 `8248486`: `processes.rs` adds `isotherm_factors`,
  `prev_species_isotherm`, `chain_source` and `source_terms` (19 tests). lBact
  with a chain is a `KernelError`, standing in for the source `stop`.
- 18.9.4 `d3a8e72`: `biology.rs` does the lBact/lFiltr selection (9 tests). It
  returns Blocking/Deposit requests, `BacteriaScalars` and zeroed-rate sets. TT
  comes from TempN at the final level and TempO otherwise (297/316). Deposit
  always gets TempN. The old-level Dc uses the new TT.
- 18.9.2 `49bcc20`: `nonequilibrium.rs` adds `immobile_water` and `omega_terms`
  (16 tests). ThImobO, OmegaO and OmegaS are `None` where the source leaves them
  unassigned.
- 18.9.3 `6e81780`: `dual_domain.rs` adds `dual_domain_th_imob`,
  `dual_neighbor_theta` and `dual_material_selection` (16 tests). lBact with
  iDualPor>0 is rejected explicitly. The guard is
  `(lMobIm.and.iDualPor.eq.0).or.lBact`.
- Verification at `6e81780`:
  - `cargo test --workspace`: 352 passed, 0 failed.
  - `cargo clippy --workspace --all-targets -D warnings`: clean.
  - `cargo fmt --check` and `git diff --check`: clean.
  - Python unittest (`.venv/bin/python`): 291 OK. The repo `.venv` restores the
    Python gate.

### Follow-ups routed from Wave 9
- **Audit correction:** `SOLUTE_FORTRAN_COMPATIBILITY_18.md:37` and
  `types.rs::BlockingMode::SourceNoOp3` wrongly say the iPsi=3 assignments are
  commented out. They are active. Fix both in a scoped amendment.
- **SD candidate:** `cMid` is assigned only in the nonlinear branch (573), yet
  it is read at 687 and in Disper at 1053. A linear species therefore reads a
  stale or uninitialised value. `source_terms` takes it as `c_mid_carried`, so
  18.10.1 must decide the carry. Probe it before registering it as an SD.
- **Duplication:** `accounting.rs` keeps private copies of blocking/deposit.
  Point it at `deposition.rs`, and check its `deposit` for `powf` on integer
  exponents. Also check `accumulate_bacteria`'s iPsi material index (`mj` vs
  `mi`).
- **18.10.1 inputs:**
  - Carry the old-level `ThOIm`, which comes after `Update` (WATFLOW 1112).
  - Carry `SinkIm` persistence, the `CarriedLocals` Dc/aa, and the `cMid` carry.
  - The caller applies the 18.9.4 lBact zeroing to the 18.9.2 Omega values.
- lBact with iDualPor>0 is unsupported in both 18.9.2 and 18.9.3 until it is
  mapped.

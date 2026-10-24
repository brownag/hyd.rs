# State — Orchestration 22

Plan status: **Approved by user on 2026-10-22 (“YES”).**
Execution status: **Waves 1A, 1B, 2, 3A, 3B done; Waves 3C and 4A done (4A re-run on the fixed public path after 22.3.8); Wave 4B (22.4.4) done, PARTIAL; Task 22.4.5 done; Wave 4 and Wave 5 (22.5.1) done; Wave 6 (user gate D10) next.**
Plan: [EXECUTION_PLAN_22.md](EXECUTION_PLAN_22.md). Worker template: [PROMPT_TEMPLATE_22.md](PROMPT_TEMPLATE_22.md).
Follows: Orchestration 21 (complete; [STATE_21.md](STATE_21.md) § Follow-ups).

## Baseline and guardrails

- [x] Baseline at HEAD (Rust 429/0/1 confirmed 2026-10-22) (the O21 wave-end A4 at `b22c5fa` covers the same code; re-confirm in Wave 0):
  - `cargo test --workspace --no-fail-fast`: 429 passed, 0 failed, 1 ignored;
  - Python `unittest discover -s tools`: 317 OK;
  - fmt, clippy and `git diff --check`: clean;
  - parity `--fixture-group all`: 44/44.
- [ ] Disk: at least 10 GB free before each wave.
- [x] User decisions D1–D9 recorded in [EXECUTION_PLAN_22.md](EXECUTION_PLAN_22.md) (2026-10-22).
- [x] Plan review (Spec and Standards) completed and applied (2026-10-22): Spec 0 blockers/4 should-fix/4 nits; Standards 3 blockers/4 should-fix/2 nits; all applied (Wave 3 restructured into 3A/3B/3C with phase roots and `backend` field; 22.1.2 split into 22.1.2/22.1.4).
- [x] User approves this execution plan (2026-10-22).
- [ ] Wave 6 user gate: thresholds approved (D10).
- [ ] Holdouts pass (`22.7.1`) before M5-V closure (`22.8.1`).

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 0 | — | Baseline, commit plan | pending |
| 1A | 22.1.1 | Serialize cached parity-runner build (flake) | done |
| 1A | 22.1.2 | Negative zero E-format sign | done |
| 1A | 22.1.3 | ADR 0018 default backend | done |
| 1B | 22.1.4 | NOD_INF depth sign; NodOut flux recomputation | done |
| 2 | — | Safe worktree cleanup (D5) | done |
| 3A | 22.3.1 | `CASES_V2.json` and spec checker | done |
| 3B | 22.3.2 | v2 rows through reference and lifecycle tools | done |
| 3B | 22.3.3 | Harness backend selection; public-API path check | done |
| 3C | 22.3.5 | Fix public FortranCompatible step (added 2026-10-22, user-approved) | done |
| 3C | 22.3.8 | Fix chain production + flow-direction mirroring in public FC step (added; from 4A findings) | done |
| 3C | 22.3.7 | v2 bridge gate waived; lifecycle/harness/metrics v2 execute fixes (added; user-approved) | done |
| 3C | 22.3.6 | Allow `22-*` roots in the reference cache (added; 22.3.4 finding) | done |
| 3C | 22.3.4 | Benchmark/convergence/holdout CLIs on v2; bug fixes | done |
| 4A | 22.4.1 | Study: closed reactions | done |
| 4A | 22.4.2 | Study: `diffusion_cosine` | done |
| 4A | 22.4.3 | Study: `scalar_robin` | done |
| 4B | 22.4.4 | Study: Example 3 chain | pending |
| 5 | 22.5.1 | Error budgets, orders, proposed thresholds | pending |
| 6 | — | User gate D10: thresholds | pending |
| 7 | 22.7.1 | Holdouts | pending |
| 8 | 22.8.1 | M5-V closure docs, ADR 0019 | pending |

## Verification log

- Wave 1A (merged 7124aa0): cargo workspace 430/0/1; Python 320 OK; fmt, clippy, diff --check clean; parity 44/44 (worker run on 22.1.2; paired gate in cargo suite passed). No POLICY_HASHES change needed.

- Wave 1B (merged): cargo workspace 432/0/1; Python 320 OK; fmt, clippy, diff --check clean; parity 44/44 (worker run; paired gate in cargo suite passed).
- Wave 2: worktrees 64 -> 48 (also removed 16 merged+clean: the .claude agent, 18.7.8, 21.1.1-21.5.1, with `git branch -d`); `.development/21-target` (5.9G) removed; disk free 91G -> 104G. 21.3.4/5/6 evidence moved to `.development/21.3.N/` (symlinks replaced; 21.3.6 merged with no conflicts); other worktrees' `.development/<id>` dirs moved to main first; `harness_paired` scratch dropped. Remaining: 22-target (1.3G, active). Doc paths cited in STATE_21 and WATER_TRANSIENT_DIAGNOSIS_21R_* resolve.
- Remaining worktrees for the user: unmerged 18.5.1, 18.5.3; unmerged `docs/master-plan-*` (1.x, 2.x) and detached 7.x; dirty 20.6.1 (43 files), 18.5.2, 13.2.1-13.2.3, detached 18.2.x/18.3.x; 7 antigravity worktrees under `~/.gemini/antigravity-cli/worktrees/`; active 22.1.1-22.1.4 (to remove at end).

- Wave 3B (merged 3a76529): cargo workspace 435/0/1; Python 333 OK; fmt, clippy, diff --check clean. `POLICY_HASHES` re-pinned for the harness (`1254668…`); `solute.rs` re-pin due after 22.3.5.
- DECISION 2026-10-22 (user): 22.3.3 found the public FortranCompatible step wrong on harness inputs; add 22.3.5 (diagnose and fix in solute.rs) in parallel with 22.3.4. 22.3.2 found `solute_verification_metrics.py` `MANIFEST_KEYS` rejects the manifest `backend` key, so it was added to 22.3.4's scope; O13 `--verify-only` already fails (source identity moved), recorded as a known limit.

- Wave 3C (merged 1fd285e + 22.3.5): cargo workspace 439/0/1; Python 342 OK; fmt, clippy, diff --check clean; parity `--backend fortran-compatible --fixture-group all` pass (44 fixtures, exit 0; log `.development/22-gate-3c/out.log`). `POLICY_HASHES` re-pinned for `solute.rs`.

- 22.3.7 gate (merged): cargo workspace 440/0/1; Python 352 OK; fmt, clippy, diff --check clean. `POLICY_HASHES` harness hash re-pinned again.
- DECISION 2026-10-22 (user): bridge gate waived for v2 (receipt `not_applicable_v2`). First 4A attempts (22.4.1-22.4.3) stopped on the bridge gate; 22.3.7 also had to fix cargo.jsonl parsing, the per-row reservation ceiling, the harness scratch-root allow-list and the metrics request key set before a v2 row could execute. Editing `solute_benchmark_bridge.py` changes `COMPARATORS`/`PRODUCERS` hashes, so retained O13 bridge reports no longer re-verify (O13 verify-only was already failing).

- 22.3.8 gate (merged): cargo workspace 448/0/1; Python 352 OK; fmt, clippy, diff --check clean; parity `--backend fortran-compatible --fixture-group all` exit 0, 44 fixtures pass (`.development/22-gate-3c/out2.log`). `POLICY_HASHES` solute.rs re-pinned.
- Study findings that triggered 22.3.8 (first-pass 4A, unmerged branches `orchestration/22.4.1-22.4.3`): daughters exactly 0 in the reaction chain (22.4.1); `diffusion_cosine` inventory not conserved; `scalar_robin` outlet 1.49 > c_in (22.4.3). Causes: chain production read from parent's slots; kernel has only a node-1 outflow closure (`SOLUTE.FOR:803-827`, top row `:895-915` no outflow term). `diffusion_cosine` inventory loss (first order in dx: -1.50e-2/-7.33e-3/-3.76e-3 at 17/33/65 nodes) is inherent to the kernel's end-row c1=c2 treatment of the pinned Fortran; not a public-path defect. To decide: whether to log as an SD in `docs/FORTRAN_SOURCE_DEFECTS.md`.

- Wave 4A (merged 4b80416): reports `planning/master/M5V_STUDY_22_{REACTIONS,DIFFUSION,ROBIN}.md`; all batches `--verify-only` exit 0 (with `--batch`). Classifier is `inconclusive` for every finding: no bounds source (`solute-convergence-bounds-v1` + `-review-v1` with a root authorization) exists. Wave 5/6 needs a decision on producing them. First-pass runs (defective public path) are preserved under each worktree's `.development/<id>/superseded/` and in main `.development/22.4.N/`; v2 evidence in main `.development/22.4.N-v2/` (22.4.1) or `.development/22.4.N/` (22.4.2, 22.4.3). Workflow lessons: unset `CARGO_TARGET_DIR` for lifecycle runs; `--allow-reference-work` (no escalation); `--verify-only` needs `--batch`; the launch ledger is per phase root (archive whole roots per worktree, never edit ledgers; 22.4.1 renamed ledgers in place once before archiving, hashes recorded).

## Task reports

### 22.1.1 — done (a9583ee)
fcntl lock around check-and-build in `tools/solute_rust_candidate_adapter.py`; temp file + os.replace; 3 new tests; harness flake check 3/3 at 12/12. Tests load the adapter via importlib (normal import breaks fake-module tests in test_solute_fortran_parity). Gotcha: a real `target` dir in a worktree makes `ln -s target` land inside it.

### 22.1.2 — done (2a485dd)
`format_fortran_e` prints `-0.…E+00` for negative zero. Evidence: probe in `.development/22.1.2/probe/`, `sd002_defect_a/T_LEVEL.OUT:9` (parity reference outputs hold no such line). `format_fortran_f` already matches gfortran for -0.0. 38 of 44 candidate digests changed (observations only), 44/44 held.

### 22.1.3 — done (e6fa676)
ADR 0018 written, ADR 0013 status line, README row and note.

### 22.1.4 — done
`make_nod_inf_block`/`make_nod_inf_v_block` print depth as `x(i)-xS` (Fortran sign); new `nod_out_fluxes` recomputes `vi`/`vVi` as NodOut in f32; lDensity/lCentrif branches unported (unreachable in all 44 decks). 2 new tests. NOD_INF files now match Fortran in all 44 fixtures except `chain_zero_rates` cols 12-13 (Conc/Sorb, `-0.0000E+00` vs `0.0000E+00`, a separate negative-zero case, not fixed). PROFILE.OUT depth already correct. Candidate digests unchanged (NOD_INF unobserved).

### 22.3.1 — done (merge b5f18c9)
`CASES_V2.json` (sha256 `644c4845…`, 38 `rfc` rows, 3 reserved holdouts, no ceilings, `derived_from` v1 path+sha); checker dispatches on `schema_version`; 5 tests. Generator `.development/22.3.1/gen_cases_v2.py` (`62508705…`). Dropped: v1 Fortran rows and the Fortran-only promoted-precision `sensitivity` row (honest gap for 22.8.1). `precision: "f64"` on rfc rows is a schema label only; FortranCompatible computes in f32 (note for 22.8.1).

### 22.3.2 — done (eda8048)
`22-benchmark/22-convergence/22-holdout` roots; `request_bytes` emits `backend` only when the row has it (v1 request bytes for 38 rust rows unchanged by SHA); lifecycle skips caps for books without them (`has_caps`), `remaining: None`; manifest top-level `backend` only when present; ledgers/storage scoped by casebook generation. 8 new tests; 48 OK. v2 dry-runs from 22-benchmark/22-convergence/22-holdout work. Open: metrics `MANIFEST_KEYS` rejects `backend`; run_solute_convergence.py:72, check_solute_benchmarks.py:77,96, solute_convergence_evidence.py:498 read caps directly (22.3.4).

### 22.3.3 — done (2f70b7f)
Harness parses optional `backend` (absent = FV/BE; unknown = error); no new observation key (metrics key set frozen), FC rows carry a different free-text solver description. 3 tests, 18/18. Path check: public step and deck runner both reach `iteration::solve_solute_with_nonlinear_with_scratch`, but configs differ (`dt_min`, `water_flow`, density/dual-porosity/vapor off). Hand runs complete but FC output is wrong vs FV/BE (max abs diff 1.0 reaction_zero, 1.19 diffusion_cosine; interior unchanged, node 0 drops to 0). Suspect `solute.rs:~1026-1047` (dispersion into dispersivity slot). Evidence in `.development/22.3.3/`.

### 22.3.4 — done (merge c7e5198)
Metrics accepts optional manifest `backend`; `run_solute_convergence.py` passes run paths to classify (mocked repro; both new tests red on old code), accepts `raw_complete`, scopes ledgers by casebook generation, adds `--holdout` (refuses reserved rows until `THRESHOLDS_V2.json` exists); evidence/benchmark CLIs cap-free for v2. Modules 93 -> 101 OK. v2 dry-run + verify-only OK for benchmark/convergence/holdout. O13 `--verify-only` known to fail ("source identity changed"). Commands: benchmark `check_solute_benchmarks.py --cases CASES_V2.json --output-root .development/22-benchmark/<x>`; convergence `run_solute_convergence.py --cases CASES_V2.json --output-root .development/22-convergence/<x>`; holdout same script with `--holdout --output-root .development/22-holdout/<x>`. Analysis/metrics modules not probed beyond the dry-runs (they key on `solver == "rust"`).

### 22.3.5 — done (merge after 22.3.6)
Public `FortranCompatible` step fixed in `solute.rs:~969-1130`: (A) D now goes to molecular-diffusion slot `offset+4`, dispersivity `ch_par[1]=0` (error if D differs within a material >1e-12 rel); (B) `TotalFlux` is Cauchy code -1 always, nonzero flux at zero velocity is an error; (C) first-order decay to `offset+10/11`, parent chain to `offset+13/14`. New `tests/solute_fortran_public_path.rs` 4/4 (analytical; first-order decay tolerance 1e-3 at dt=0.0125 set from the observed first-order dt convergence 2.4e-3/1.2e-3/6e-4, not from solver tuning of a study threshold). FC vs FV/BE hand runs: max abs diff 0 (reaction_zero), 0.0499 (diffusion_cosine). Limit: per-node velocity taken from the lower face (fine for uniform q).

### 22.3.6 — done (1fd285e)
`ALLOWED_TASKS` in `tools/solute_reference_cache.py` gains the three `22-*` roots; test added (22 OK).

### 22.3.7 — done
v2 gate: `is_v2_casebook`/`v2_gate_receipt` in the bridge module, CLIs skip `require_bridges` for v2 and reject `--bridge-report`; v2 bridge row recorded (`recorded`). Lifecycle: `build_rust` parses only `{` lines; `row_cap()` skips the 2x ceiling for cap-free books. Harness `output_path` allows `22-benchmark/22-convergence/22-holdout` (19 tests). Metrics `_request` accepts optional `backend`. Proofs: v2 `reaction_zero` benchmark `batch-77236ebf…` and `diffusion_cosine` `space_17` convergence `batch-725c83ad…`, both evidence_complete, verify-only exit 0 (scratch only).

### 22.3.8 — done
Chain: a species with a daughter carries its decay in `gam_l1/gam_s1`, the last in `gam_l/gam_s` (kernel reads production from the parent's slots, `driver.rs:1051-1070`). Flow direction: when summed face flux > 0 the kernel runs on the mirror image (node k = public N-1-k, velocities negated, boundaries swapped, results mapped back); nodal flux is now the endpoint face at the ends and the mean of adjacent faces inside (differs from the old lower-face rule only for non-uniform flux). 12 public-path tests, incl. 3 Bateman chain, 3 flux-inlet, 2 q<0. Limits: mirror chosen per step from the sign of the summed flux, a direction flip between steps is untested; the 22.3.5 comment that code -1 gives the natural row at v=0 is inaccurate (not corrected).

### 22.4.1 / 22.4.2 / 22.4.3 — done (4b80416)
Reactions: S1/S2/S3 time order ~1 (local 0.94-0.997 and rising), 17-node control equals 9-node, no floor, `reaction_zero` exact. Diffusion: space error ~2.2x per halving of dx, time axis not isolated (257-node spatial error swamps; 513-node control 3.265e-4 vs 6.96e-4), inventory loss is kernel-inherent (SOLUTE.FOR:803-827). Robin: values below c_in, space linf 2.43e-3/1.20e-3/8.41e-4 (33/65/129 nodes, rate 1.0 then 0.51), time axis non-monotone with control contamination. No orders fitted (no bounds source).

- Wave 4B (22.4.4, merged ae18b62, PARTIAL): `planning/master/M5V_STUDY_22_EXAMPLE3.md`. Component benchmark evidence_complete (verify-only 0). Bridge row failed ("record inventory mismatch", legacy_bridge; not retryable; verify 2). Time axis (dt 0.2/0.1/0.05 + control) verify 0, local rates ~0.75-1.0 conc, ~1.0-1.3 inventory; classifier inconclusive (no bounds source); temporal error not isolable (501-node control). Space axis: 251/501/1001 done but `space_control` (16000 steps) hit the 60 s row timeout; re-attempt refused ("row already consumed; explicit authenticated resume required"); no space metrics, node-1 closure not reported. Evidence in main `.development/22.4.4/`. Open decisions: bounds source; resume/ceiling for space_control; bridge mapping failure.
- Task 22.4.5 (merged): `space_control` row timeout raised to 180 s (measured ~84 s direct, 76 s in lifecycle); `limits.max_row_timeout_seconds` 300, all other rows 60 s; checker one-id exception; lifecycle timeout-event bound 300. CASES_V2 SHA now `c236eb7cd42dee691a36787625865457df62c0bb4a72cb0388a786cbe1e1b43b` (was 644c4845…; old SHA only in docs). New batch `batch-df5e1531…` verify 0; the copied 22.4.4 batches verify 2 (recipe bound to old SHA); the 251/501/1001 metrics were replayed in scratch, not through the normal classify (3.5 h reference recompute otherwise). Classifier inconclusive for all 6045 findings. Findings: outlet outflow <=1.5e-8 so the node-1 closure is untestable here; inventory/balance errors flat in dx, non-monotone, dt-dominated. Gotcha: checker SHA is part of the reference-cache identity, so further checker edits force ~3 h Laplace recompute each. Report `planning/master/M5V_STUDY_22_EXAMPLE3_SPACE.md`; evidence main `.development/22.4.5/`. Wave 4 now complete.
- Task 22.5.1 (merged): `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_22.md` (11 sections). Proposal margin rule: 1.5 x observed error, 2 sig figs, applied once (ADR 0009). Per-level ceilings proposed for reaction_distinct/equal profile_linf, diffusion_cosine and scalar_robin space linf/l2, Example 3 NH4/NO2 norms; exact-zero for reaction_zero and diffusion boundary/reaction. Every combined holdout-level candidate is UNASSIGNED (holdout dt/mesh not isolated by any study), so holdouts cannot confirm the thresholds and M5-V cannot fully close. Unassigned: diffusion inventory (known defect candidate), Robin l1/l2/inventory/balance, all temporal error, Example 3 inventory/balance/closure/NO3/outlet tail. THRESHOLDS_V2.json must carry CASES_V2 SHA c236eb7c…. Candidate SD for the diffusion end-row loss needs a task with docs/FORTRAN_SOURCE_DEFECTS.md in scope.

# Orchestration 22 — Close Orchestration 21 follow-ups and the M5-V scientific-comparison gate

## Goal and scope

**Goal.** The user asked to plan the follow-ups and the logical next steps after Orchestration 21. Orchestration 21 brought genuine Rust-vs-Fortran parity to 44/44 and made `SoluteBackend::FortranCompatible` the default (`21.4.1`, `21.5.1`). This orchestration does two things:

1. It closes the follow-ups recorded in `STATE_21.md` § Follow-ups.
2. It works toward closing the M5-V scientific-comparison gate, which the roadmap names as the next planning priority and as the gate before M6 heat (`ROADMAP_AND_RISKS.md`, `PHASE_5_SOLUTE_EVIDENCE.md:80,110`).

M5-V is re-scoped (D2). Calibrated source parity is recorded as met by the 44/44 gate under ADR 0006 per-field tolerances. Independent analytical validation runs on the default backend only: Rust `FortranCompatible`, refined in space and time against the analytical references, with thresholds approved by the user and then checked on reserved holdout rows.

**In scope:**
- The paired-gate harness flake: concurrent rebuilds of the cached parity runner.
- Three output differences the gate does not observe: the `-0.0` E-format sign, the `NOD_INF` node-depth sign at t=1, and `NodOut` recomputing `vi`/`vVi`. These are fixed in Rust only (D4).
- A new ADR 0018 that supersedes ADR 0013's default-backend clause (D6).
- Safe worktree cleanup (D5).
- M5-V study infrastructure: `CASES_V2.json` (D7), harness backend selection, and the convergence CLI.
- A refinement study of `FortranCompatible` on all four analytical families (D3).
- Error budgets and proposed thresholds, a user threshold gate, holdout runs, and the M5-V closure docs plus ADR 0019 (D8).

**Out of scope:**
- M6 heat (`TEMPER.FOR`), M7 hysteresis (`HYSTER.FOR`, deferred by O21 D1), M8, M9 and M10. M6 planning may start only after M5-V closes.
- Re-running Fortran in the refinement study (W4–W7). The 44/44 parity already ties Rust `FortranCompatible` to the pinned Fortran (D2), so no study, control or holdout row launches Fortran. Wave 1 may still run the pinned Fortran reference or a gfortran probe to obtain expected values for its output fixes (D4).
- The FV/BE convergence study (D3). FV/BE stays selectable, and its O13 evidence stays as recorded.
- Widening the parity gate, or any change to `OBSERVATION_MAP.json`, tolerances or comparison logic (D4, ADR 0006).
- Manufactured-forcing cases (blocked in `CASES.json`; ADR 0009).
- `TODO.md`, an earlier ADR-renumbering backlog that is largely done. Workers never touch it.
- Worktrees that are dirty, unmerged, or owned by another tool (`~/.gemini/antigravity-cli/...`). These are listed for the user and never removed.

**Verbatim user constraints:** "follow ups and logical next steps from plan". Answers recorded in the Decision log.

## Decision log

| Date | Decision |
|---|---|
| 2026-10-22 | D1. Scope: O21 follow-ups plus M5-V. M7 hysteresis and M6 heat are not in this orchestration. |
| 2026-10-22 | D2. M5-V path: re-scope. Calibrated source parity is met by the `21.4.1` 44/44 gate. Physical/analytical validation runs mesh/time refinement of the default `FortranCompatible` backend against analytical references. The runs are Rust-only, because the 44/44 parity ties Rust to the pinned Fortran. |
| 2026-10-22 | D3. Cases: all four analytical families (closed reactions, `diffusion_cosine`, `scalar_robin`, the Example 3 chain), on the spec's space and time levels. FV/BE is not re-studied. |
| 2026-10-22 | D4. Unobserved output differences: fix them in Rust only, each with a targeted test whose expected value comes from Fortran reference output, and re-run the 44-fixture gate. The comparator and `OBSERVATION_MAP.json` stay untouched. |
| 2026-10-22 | D5. Worktrees: safe subset. First move the 21.x in-worktree evidence into `.development/<id>/` and repoint the symlinks. Then remove only merged and clean worktrees of this repo (under `.development/` and `.claude/worktrees/`) plus `.development/21-target`, and `git branch -d` their merged branches. Dirty, unmerged and antigravity worktrees are listed for the user. |
| 2026-10-22 | D6. A new ADR 0018 records the default switch. ADR 0013 gets a one-line status note, and its closed-enum decision stays in force. |
| 2026-10-22 | D7. A new `reference/solute_verification/CASES_V2.json` holds the same cases and analytical references, with Rust `FortranCompatible` launch rows (represented as `solver: "rust"` plus `backend: "fortran-compatible"`; see the Wave 3 design note). It has no launch-count or storage ceilings (ADR 0003); per-run timeouts stay operational. `CASES.json` (v1, SHA `d3b06531…`) stays frozen as O13 evidence. The Rust harness takes the backend from the request and records it in the observation. |
| 2026-10-22 | D8. Thresholds: the study first produces error budgets and fitted orders. Then a user gate approves per-case, per-field thresholds. Then the reserved holdout rows run, unseen during threshold selection. M5-V closes only if the holdouts pass. |
| 2026-10-22 | D9. Execution setup: keep O21 D5. That means ≤3 concurrent workers in `.development/22-worktrees/<id>` on branch `orchestration/<id>`, the `inherit` model tier, escalation after one failed retry, serial wave-end gates (A4), and orchestrator-owned `POLICY_HASHES` re-pins. |

## Reconnaissance log (2026-10-22, HEAD `854ba2b`, branch `phase4-water-acceptance`)

- **Baseline** (O21 wave-end A4 at `b22c5fa`, the same code as HEAD):
  - `cargo test --workspace --no-fail-fast`: 429 passed, 0 failed, 1 ignored.
  - Python `unittest discover -s tools`: 317 OK.
  - fmt, clippy and `git diff --check`: clean.
  - Parity `--fixture-group all`: 44/44 (`.development/21.3F/all.json`; the 21.4.1 evidence is at `17d9d9c`).
- **Paired-gate flake.** The `solute_fortran_compat_harness` paired gates fail with "paired status None" on the first run after a rebuild (3 times in O21). The cause is concurrent tests racing on the cached runner build in `tools/solute_rust_candidate_adapter.py:40-70`: it checks mtime, then compiles to a fixed path, with no lock and no atomic rename. The gates are `run_paired_fixture_gate` in `crates/hydrus-core/tests/solute_fortran_compat_harness.rs:~850-910`.
- **Output gaps:**
  - `format_fortran_e` zero branch: `output_compat.rs:28-31` prints ` 0.…E+00` for `-0.0`; gfortran prints `-0.…E+00` in all 44 reference outputs.
  - `NOD_INF` depth at t=1, node 2: Rust prints `0.2500`, Fortran `-0.2500` (21.3.7 report). The cause is not yet traced.
  - `NodOut` (`OUTPUT.FOR:~720-816`) recomputes `vi`/`vVi`, while Rust `make_nod_inf_v_block` (`compat_runner.rs:~1530`) prints the solver's `v`/`vv`.
- **ADRs.** 17 accepted. ADR 0013 `:35` says the default is FV/BE. `docs/decisions/README.md:24` records the switch. ADR 0003 removed attempt and artifact ceilings (O18). ADR 0009 forbids assigning thresholds before error budgets and convergence evidence exist, and forbids widening limits.
- **M5-V state.**
  - Every evidence doc says "M5-V remains open" (`PHASE_5_SOLUTE_EVIDENCE.md:80,110`, `SOLUTE_CONVERGENCE_EVIDENCE.md:7`, `SOLUTE_ANALYTICAL_EVIDENCE.md:3`, `VALIDATION_STRATEGY.md:13`).
  - O13 ran FV/BE plus Fortran and stopped at a storage ceiling, with CLI state bugs: the CLI rejects `raw_complete` at its `complete` gate (`run_solute_convergence.py:336`), and Rust classification hit a "path/dict argument" error.
- **CASES v1** (`reference/solute_verification/CASES.json`): 7 cases (6 executable, `manufactured_forcing` blocked) and 77 launches. The `rust` rows mean FV/BE. Rust levels:
  - reactions: component 9 nodes; time study dt 0.125, 0.0625, 0.03125; control 17 nodes.
  - `diffusion_cosine`: space 17, 33, 65 (control 65 at half dt); time 257 nodes at dt 0.0625, 0.03125, 0.015625 (control 513); holdout 97 nodes at dt 0.0078125.
  - `scalar_robin`: space 33, 65, 129 (control 129 at half dt); time 257 nodes (control 513); holdout 193 nodes at dt 0.0078125.
  - `example3`: space 251, 501, 1001 at dt 0.025 (control 1001 at dt 0.0125); time 1001 nodes at dt 0.2, 0.1, 0.05 (control 501); holdout 751 nodes at dt 0.04.
  - Every Rust level has ≥ 3 nodes, which satisfies the `FortranCompatible` minimum (`solute.rs:~891`).
- **Tooling and files:**

  | File | Lines | Notes |
  |---|---|---|
  | `tools/run_solute_convergence.py` | 467 | `TASK_ROOT = .development/13.4.1`; CASES path hard-coded at `:35` |
  | `tools/solute_benchmark_lifecycle.py` | 949 | request builder sets `"solver": "rust"` at `:499`; row dispatch at `:479,800,833` |
  | `tools/solute_convergence_evidence.py` | 506 | `13.4.1` scratch roots at `:104-106` |
  | `tools/check_solute_verification_spec.py` | 355 | `DEFAULT_CASES` at `:13` |
  | `tools/solute_analytical_reference.py` | 1568 | |
  | `tools/solute_verification_metrics.py` | 1547 | |
  | `crates/hydrus-core/tests/solute_verification_harness.rs` | 1203 | pins `.with_backend(SoluteBackend::FiniteVolumeBackwardEuler)` at `:538`; env-driven `requested_study` at `:743` |

  `tools/solute_benchmark_bridge.py` `POLICY_HASHES` pins `solute_verification_harness.rs` (`df60e942…`) and `solute.rs`.
- **Public-API vs deck path.** The 44/44 parity exercised `run_compat_deck`, the deck path. The verification harness calls the public `advance_solute_step` → `advance_fortran_compatible_step` (`solute.rs:878`) with prescribed θ and q. These are different entry points into `solute_fortran_compat`; see Hazards.
- **Worktrees.** 59 extra: 23 merged and clean, 11 merged and dirty, 25 unmerged (7 antigravity). `.development/21-worktrees` uses 6.0 GB and `.development/21-target` 5.9 GB. The scratch for 21.3.4 and 21.3.5 lives inside their worktrees; the main-repo `.development/21.3.{4,5}` are symlinks into them. For 21.3.6 the main scratch is real and the parity task roots are in the worktree.
- **Disk.** About 119 GB free.

## Hazards

- **Inherited O21 hazards.** These apply unchanged: fixed-form Fortran, trace guards, precision (ADR 0010, 0015), filesystem-free production code, worktrees, never staging `target/` or `.development/`, and disk space.
- **Parity from a worktree.** `tools/solute_rust_candidate_adapter.py` reads `<worktree>/target/debug`, so use a temporary, never-staged `target` symlink to `CARGO_TARGET_DIR`. The `/target/` ignore pattern does not match a symlink. `tools/run_fortran_reference.py:126-137` requires `--task-root` under the worktree's own `.development/`. Build the harness (`--no-run`) before deleting the cached runner, and record the runner's SHA-256. Fortran reference digests are not reproducible from run to run (wall-clock lines), so compare candidate digests.
- **The public-API path is not the parity-proven path.** Before any study run, `22.3.3` shows that the public `FortranCompatible` step and the deck runner use the same kernels for the prescribed-water transport the study uses. It names the shared functions and runs one analytical benchmark through both where feasible. Any difference is reported, not fixed.
- **f32 rounding floors.** `FortranCompatible` keeps Fortran `REAL` (f32) arithmetic. At fine levels the error can plateau near f32 resolution (~1e-7 relative). Report plateaus and floors explicitly, as `SOLUTE_VERIFICATION_SPEC.md` "Norms, refinement and budgets" requires. Never fit an order through a floor.
- **Evidence integrity:**
  - `CASES.json` v1, the O13 scratch (`.development/13.*`), `reference/`, `src_reference/` and `OBSERVATION_MAP.json` are read-only.
  - Expected values never come from solver output, and reference outputs never become model inputs (ADR 0004, 0009).
  - Holdout rows must not run before the D8 threshold gate, and their outputs must never inform thresholds.
- **`POLICY_HASHES`.** Editing `solute_verification_harness.rs` (and `solute.rs`, if ever touched) invalidates `tools/solute_benchmark_bridge.py` `POLICY_HASHES`. The orchestrator re-pins at wave end, with a justification comment. Workers never edit that file.
- **Destructive cleanup (Wave 2)** is done by the orchestrator only, exactly per D5. Before removing a worktree, confirm it is merged and clean, and that no evidence path in `planning/` or `STATE_*` points into it after the move.

## Shared acceptance rules (every task)

- **A1.** No existing test assertion or tolerance is weakened.
- **A2.** Every targeted command named in the task spec passes, and the worker reports exact before/after counts.
- **A3.** Tasks that change Rust: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean in the worktree. Tasks that change Python: the named `tools/test_*.py` modules pass.
- **A4.** At wave end, the orchestrator runs serially: `cargo test --workspace --no-fail-fast`; `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`; fmt; clippy; `git diff --check`; and parity `--fixture-group all` = 44/44 whenever `crates/` or `tools/solute_rust_candidate_adapter.py` changed. A first-run paired-gate failure is re-run once before it counts, until `22.1.1` lands. After that it counts. Results and any `POLICY_HASHES` re-pin are recorded in `STATE_22.md`.
- **A5 (study and report tasks).** `git diff --exit-code -- src_reference reference/solute_verification/CASES.json crates tools Cargo.toml Cargo.lock` is clean at hand-off. The only committed files are the task's report doc and todo. Run artifacts stay under `.development/<id>/`, and their manifest SHA-256 values appear in the report.

## Wave table

| Wave | Task | Title | Must-edit / create (disjoint within wave) |
|---|---|---|---|
| 0 | — | Orchestrator: baseline, commit plan | `planning/{EXECUTION_PLAN,PROMPT_TEMPLATE,STATE}_22.md` |
| 1A | 22.1.1 | Serialize the cached parity-runner build | `tools/solute_rust_candidate_adapter.py`, `tools/test_solute_rust_candidate_adapter.py` (create) |
| 1A | 22.1.2 | Print negative zero as gfortran does | `crates/hydrus-core/src/solute_fortran_compat/output_compat.rs` |
| 1A | 22.1.3 | ADR 0018: default to the Fortran-compatible backend | `docs/decisions/0018-*.md` (create), `0013-*.md` (status note), `README.md` |
| 1B | 22.1.4 | `NOD_INF` depth sign and `NodOut` flux recomputation | `crates/hydrus-core/src/solute_fortran_compat/{compat_runner,output_compat}.rs` |
| 2 | — | Orchestrator: safe worktree cleanup (D5) | `planning/STATE_22.md` |
| 3A | 22.3.1 | `CASES_V2.json` and spec checker | `reference/solute_verification/CASES_V2.json` (create), `tools/check_solute_verification_spec.py`, `tools/test_check_solute_verification_spec.py` (create) |
| 3B | 22.3.2 | v2 rows through the reference and lifecycle tools | `tools/run_solute_verification_reference.py`, `tools/solute_benchmark_lifecycle.py`, their tests |
| 3B | 22.3.3 | Harness backend selection; public-API path check | `crates/hydrus-core/tests/solute_verification_harness.rs` |
| 3C | 22.3.4 | Benchmark, convergence and holdout CLIs on v2; bug fixes | `tools/{run_solute_convergence,solute_convergence_evidence,check_solute_benchmarks}.py`, their tests |
| 3C | 22.3.5 | Fix the public `FortranCompatible` step on prescribed-water transport (user-approved scope change, 2026-10-22) | `crates/hydrus-core/src/solute.rs`; create `crates/hydrus-core/tests/solute_fortran_public_path.rs` |
| 4A | 22.4.1 | Study: closed reactions | create `planning/master/M5V_STUDY_22_REACTIONS.md` |
| 4A | 22.4.2 | Study: `diffusion_cosine` | create `planning/master/M5V_STUDY_22_DIFFUSION.md` |
| 4A | 22.4.3 | Study: `scalar_robin` | create `planning/master/M5V_STUDY_22_ROBIN.md` |
| 4B | 22.4.4 | Study: Example 3 chain | create `planning/master/M5V_STUDY_22_EXAMPLE3.md` |
| 5 | 22.5.1 | Error budgets, fitted orders, proposed thresholds | create `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_22.md` |
| 6 | — | **User gate D10**: approve thresholds; orchestrator writes `THRESHOLDS_V2.json` | `reference/solute_verification/THRESHOLDS_V2.json` (create), `planning/*_22.md` |
| 7 | 22.7.1 | Holdout runs against approved thresholds | create `planning/master/M5V_HOLDOUT_22.md` |
| 8 | 22.8.1 | M5-V closure docs and ADR 0019 | `planning/master/{PHASE_5_SOLUTE_EVIDENCE,VALIDATION_STRATEGY,ROADMAP_AND_RISKS,SOLUTE_CONVERGENCE_EVIDENCE}.md`, `docs/decisions/0019-*.md` (create), `docs/decisions/README.md` |

**Dependency chain.** W0 → 1A → 1B → W2 → 3A → 3B → 3C → 4A → 4B → W5 → W6 gate → W7 → W8.
- Wave 1 is independent of the M5-V waves. It runs first so that the flake fix and the clean worktree layout are in place for the long study waves.
- 4B follows 4A because of the ≤3 worker cap.
- If any study task (W4) shows a run that cannot complete or a reference-budget failure, the orchestrator records it, and the W6 gate decides; no task widens.
- If W7 fails, W8 does not run, and the orchestrator returns to a user gate. Thresholds are never re-tuned on holdout data.
- `22.1.3` and `22.8.1` both edit `docs/decisions/README.md` in different waves, which is sequential and safe.

---

## Wave 0 — Orchestrator (no worker)

1. Record the baseline (Reconnaissance) in `STATE_22.md` and confirm ≥ 10 GB free.
2. Commit `planning/{EXECUTION_PLAN,PROMPT_TEMPLATE,STATE}_22.md` with message `docs(planning): plan follow-ups and the scientific-comparison gate`.

---

## Wave 1 — Orchestration 21 follow-ups

### 22.1.1 — Serialize the cached parity-runner build

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `tools/solute_rust_candidate_adapter.py` (`_ensure_rust_runner_binary`, `:40-~110`).
- **Can create:** `tools/test_solute_rust_candidate_adapter.py`; `planning/tasks/22.1.1.todo.md`; scratch `.development/22.1.1/`.
- **Depends on:** Wave 0.
- **Spec:**
  - Take an exclusive `fcntl` file lock (`target/debug/.solute_compat_deck_runner.lock`) around the whole check-and-build.
  - Compile to a temporary path in the same directory, then `os.replace` it onto `solute_compat_deck_runner`.
  - Re-check freshness after acquiring the lock, so a waiter reuses a binary another process just built.
  - Do not change the runner source text, the rlib selection rule (newest `libhydrus_core-*.rlib`), or the outputs.
  - `run_solute_convergence.py` already imports `fcntl`; follow its idiom.
- **Tests:**
  - `test_concurrent_callers_build_once_and_all_get_complete_binary`: N threads or processes call the function against a fake workspace with a stub `rustc`/`cargo` on `PATH` that sleeps and counts invocations.
  - `test_waiter_reuses_fresh_binary`.
  - `test_failed_build_leaves_no_partial_binary`.
- **Verification:**
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_rust_candidate_adapter tools.test_solute_fortran_parity`.
  - The flake check, done 3 times: delete `target/debug/solute_compat_deck_runner` and touch the rlib (`touch` the newest `libhydrus_core-*.rlib`), then run `cargo test -p hydrus-core --test solute_fortran_compat_harness`. The result must be 12/12 every time, with no re-runs.
  - `git diff --check`.
- **Do not touch:** `tools/run_solute_parity.py`, comparison or normalizer logic, `crates/`, `Cargo.*`.
- **Anchors:** `tools/solute_rust_candidate_adapter.py:39-120`; `crates/hydrus-core/tests/solute_fortran_compat_harness.rs` (`run_paired_fixture_gate`, ~`:850-910`); `tools/run_solute_convergence.py` (`fcntl` use).

### 22.1.2 — Print negative zero as gfortran does (sub-wave 1A)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/output_compat.rs` (`format_fortran_e`, `:18-50`, and its tests).
- **Can create:** `planning/tasks/22.1.2.todo.md`; scratch `.development/22.1.2/`.
- **Depends on:** Wave 0.
- **Spec:**
  - `format_fortran_e` prints a negative zero as `-0.…E+00`, as gfortran does (`val == 0.0 && val.is_sign_negative()`).
  - Evidence: a gfortran probe like 21.3.8's (`.development/21.3.8/fmt_probe/`), built with the `tools/build_fortran_reference.sh` flags and recorded in scratch, plus the `-0.…E+00` lines already present in the 44 reference outputs (cite one file and line).
  - Report-only: note whether `format_fortran_f` handles `-0.0` differently from gfortran `F` editing. Do not change it; D4 names three differences only.
- **Test:** `format_fortran_e_prints_negative_zero_as_gfortran`, red before the fix.
- **Verification:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat`; `cargo test -p hydrus-core --test solute_fortran_compat_harness`.
  - Parity `--fixture-group all` = 44/44. List the fixtures whose candidate digests changed.
  - fmt, clippy, `git diff --check`.
- **Do not touch:** `compat_runner.rs` (owned by `22.1.4`), `tools/`, `OBSERVATION_MAP.json`, tolerances, `solute.rs`, `reference/`, `src_reference/`.

### 22.1.4 — `NOD_INF` depth sign and `NodOut` flux recomputation (sub-wave 1B; depends 22.1.2)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute_fortran_compat/compat_runner.rs`, `crates/hydrus-core/src/solute_fortran_compat/output_compat.rs`. The fix for (b) may sit in either file.
- **Can create:** `planning/tasks/22.1.4.todo.md`; scratch `.development/22.1.4/`.
- **Depends on:** `22.1.2`.
- **Spec:**
  - **(b)** `NOD_INF` node depth sign at t=1 (21.3.7 report: `model6_exchange`, node 2, Rust `0.2500`, Fortran `-0.2500`). Trace which writer prints it, compare with `NodOut` (`OUTPUT.FOR:~720-870`), and fix the sign.
  - **(c)** `make_nod_inf_v_block` (`compat_runner.rs:~1534`): compute `vi`/`vVi` as `NodOut` does (`OUTPUT.FOR:~730-816`), in f32 with Fortran grouping, instead of printing the solver's `v`/`vv`.
    - Check reachability per ADR 0007 first.
    - If `NodOut`'s recomputation provably equals the solver value for every reachable deck, record that and make no change.
  - Take the expected values from a canonical Fortran reference run (`tools/run_fortran_reference.py` on an existing parity fixture, task root in the worktree's `.development/22.1.4/`). Cite the file and line in each test comment.
- **Tests:** `nod_inf_reports_signed_depth_as_fortran`; `nod_inf_v_recomputes_liquid_and_vapor_flux_as_nodout`. Each is red before its fix.
- **Verification:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat`; `cargo test -p hydrus-core --test solute_fortran_compat_harness`.
  - Parity `--fixture-group all` = 44/44. List the fixtures whose candidate digests changed.
  - fmt, clippy, `git diff --check`.
- **Do not touch:** `tools/`, `OBSERVATION_MAP.json`, tolerances, `solute.rs`, `reference/`, `src_reference/`.
- **Anchors:** `compat_runner.rs` (grep `make_nod_inf_v_block`, `NodInf`, `depth`, `x_surf`); `src_reference/OUTPUT.FOR:720-870`; `planning/master/WATER_TRANSIENT_DIAGNOSIS_21R_TEMPVAPOR.md`; `planning/tasks/21.3.7.todo.md` `## Discovered`.

### 22.1.3 — ADR 0018: default to the Fortran-compatible solute backend

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `docs/decisions/0013-select-solute-backend-with-a-closed-enum.md` (add one dated status line only), `docs/decisions/README.md` (index row plus a status note pointer).
- **Can create:** `docs/decisions/0018-default-to-the-fortran-compatible-solute-backend.md`; `planning/tasks/22.1.3.todo.md`.
- **Depends on:** Wave 0.
- **Spec:**
  - Write ADR 0018 in the house template: copy the front-matter keys and section order from ADR 0017. Set `supersedes: "0013 (default-backend clause only)"`.
  - Context: O20 D6 and O21 D3; the `21.4.1` gate (`17d9d9c`, `SOLUTE_FORTRAN_PARITY_18.md` § 21.4.1); the `21.5.1` change (`7741a04`).
  - Considered options: keep FV/BE as the default, or switch after the 44/44 gate.
  - Consequences: parity-proven pinned-source behavior becomes the default, FV/BE stays selectable, and the three unobserved output gaps are named.
  - Validation: `tests/solute_solver_selection.rs` and `tests/solute_backend_contract.rs:199`.
  - ADR 0013 keeps its body. Add one line under its status: "Default-backend clause superseded by ADR 0018 (2026-10-22)."
  - Plain language, matching the existing ADRs. Every SHA in 7-character form.
- **Verification:** `git diff --check`; `grep -c '^## ' docs/decisions/0018-*.md` equals ADR 0017's count; every cited path exists (`ls`).
- **Do not touch:** any other ADR body, `TODO.md`, `crates/`, `tools/`.
- **Anchors:** `docs/decisions/0013-*.md:30-40`; `docs/decisions/0017-*.md` (template); `docs/decisions/README.md:1-40`; `planning/EXECUTION_PLAN_21.md` Decision log (D3); `planning/STATE_21.md` Wave 5.

---

## Wave 2 — Orchestrator: safe worktree cleanup (D5, no worker)

1. For 21.3.4, 21.3.5 and 21.3.6:
   - Move `<worktree>/.development/<id>/` into the main `.development/<id>/`. Replace a symlink with the real directory, or merge into the existing real directory.
   - Check that the report paths cited in `planning/master/WATER_TRANSIENT_DIAGNOSIS_21R_*.md` and `STATE_21.md` still resolve under the main `.development/`.
2. List every worktree with its path, branch, merged/unmerged, clean/dirty and size. Remove only worktrees that are merged, clean, and under `/home/andrew/AI/hyd.rs/.development/` or `.claude/worktrees/`. Use `git worktree remove`, never `--force`, then `git branch -d` (never `-D`).
3. Remove `.development/21-target` only. List any other `.development/*-target` directories, with their sizes, for the user; do not remove them (D5).
4. Run `git worktree prune`. Record the before/after counts, the space reclaimed, and a table of the remaining (dirty, unmerged, antigravity) worktrees for the user in `STATE_22.md`.

---

## Wave 3 — M5-V study infrastructure

**Row representation (design note, from D7).** v2 launch rows keep `"solver": "rust"` and add `"backend": "fortran-compatible"`. Every existing `solver == "rust"` check (`solute_convergence_analysis.py:287`, `solute_verification_metrics.py:350`, lifecycle dispatch) then stays valid without edits, and the backend is still explicit in each row, request, manifest and observation. Launch IDs carry an `rfc` tag (for example `convergence-diffusion_cosine-space_17-rfc-baseline`) so they never collide with v1 IDs.

**Request contract (shared by 22.3.2 and 22.3.3).** `request_bytes` (`run_solute_verification_reference.py:253`, schema `flat-ascii-v1`) emits one extra key `backend` with the value `fortran-compatible` only when the row has `backend`. For v1 rows the request bytes are unchanged, byte for byte. The harness treats an absent `backend` as FV/BE.

**Phase roots.** The tools map task-root folder names to phases (`run_solute_verification_reference.py:40` `PHASE_TASKS`, imported by `solute_benchmark_lifecycle.py:29`). v2 adds `22-benchmark`, `22-convergence` and `22-holdout` beside the `13.x` roots. Each Wave 4 and Wave 7 worker uses these roots under its own worktree's `.development/` and copies the verified batches to the main `.development/<id>/` at hand-off.

### 22.3.1 — `CASES_V2.json` and spec checker (sub-wave 3A)

- **Must edit:** `tools/check_solute_verification_spec.py`.
- **Can create:** `reference/solute_verification/CASES_V2.json`; `tools/test_check_solute_verification_spec.py`; `planning/tasks/22.3.1.todo.md`; scratch `.development/22.3.1/`.
- **Depends on:** Wave 2.
- **Spec:**
  - Generate `CASES_V2.json` from v1 with a deterministic scratch script, kept under `.development/22.3.1/` and hashed in the report.
  - Keep `cases`, `units`, `contracts`, `metrics` and `provenance` canonical-JSON-equal to v1; the report shows the canonical-hash equality per case.
  - Give it a v2 `schema_version`.
  - `launches`: one row for each v1 `rust` row, with `solver: "rust"`, `backend: "fortran-compatible"`, an `rfc` ID, and the same role, axis, nodes, dt, sampling, analysis times, time controls and timeout. Holdout rows have `status: "reserved_until_threshold_gate"`. There are no `fortran` rows.
  - `budgets`: keep only `cost_estimate`, plus `policy: "no launch or storage ceilings (ADR 0003)"`. `limits.automatic_retries`: remove.
  - `provenance.derived_from`: the v1 path and SHA-256.
  - v1's Fortran-only `sensitivity` row (the promoted-precision build) has no v2 counterpart. Record that drop in the report; `22.8.1` lists it as an honest gap.
  - The checker dispatches on `schema_version`: all v1 checks are unchanged (`:245-250` budget checks stay v1-only). For v2 it checks:
    - every case is unchanged from v1 (canonical hash);
    - each v1 `rust` row has exactly one `rfc` counterpart;
    - every v2 row has `backend`;
    - holdouts are reserved;
    - no ceiling keys are present.
- **Tests (new module):** `test_v2_cases_match_v1`, `test_v2_launch_rows_cover_v1_rust_rows`, `test_v2_rows_carry_backend`, `test_v2_holdouts_reserved`, `test_v1_still_validates`.
- **Verification:**
  - `python3 tools/check_solute_verification_spec.py reference/solute_verification/CASES.json` passes, and so does the same command with `CASES_V2.json`.
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_check_solute_verification_spec` passes.
  - `git diff --exit-code -- reference/solute_verification/CASES.json`; `git diff --check`.
- **Do not touch:** `CASES.json`, `crates/`, every other tool.
- **Anchors:** `tools/check_solute_verification_spec.py:13,48,240-330,355`; `SOLUTE_VERIFICATION_SPEC.md:353-411,584-595`; ADR 0003.

### 22.3.2 — v2 rows through the reference and lifecycle tools (sub-wave 3B)

- **Must edit:**
  - `tools/run_solute_verification_reference.py`: `PHASE_TASKS` `:40`, the root checks `:127,834`, `request_bytes` `:253`.
  - `tools/solute_benchmark_lifecycle.py`: the budget reads `:198-224,403,581,684-789`, the manifest `:499`.
  - their tests, `tools/test_solute_verification_reference.py` and `tools/test_solute_benchmark_lifecycle.py`.
- **Can create:** `planning/tasks/22.3.2.todo.md`; scratch `.development/22.3.2/`.
- **Depends on:** `22.3.1`.
- **Spec:**
  - Add the three `22-*` phase roots.
  - `request_bytes` emits `backend` per the request contract.
  - The lifecycle reads budget caps only when the CASES file has them (v1). For v2 it applies no launch or storage ceiling, but still records storage and work estimates.
  - The manifest records `backend` (absent for v1).
  - v1 behavior is byte-identical: the request bytes and manifests for v1 rows, and the verification of existing O13 batches (`--verify-only`, read-only).
  - The lifecycle already accepts a CASES path (`:464`); add no new option for it.
  - Use `CASES_V2.json` with `--dry-run` only. Do not execute any v2 row here.
- **Tests:** `test_v1_request_bytes_unchanged`; `test_v2_request_carries_backend`; `test_v2_has_no_launch_or_storage_ceiling`; `test_v2_phase_roots_accepted`; `test_v1_budget_caps_still_enforced`.
- **Verification:**
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_verification_reference tools.test_solute_benchmark_lifecycle`.
  - A `--dry-run` of one v2 benchmark row and one v2 convergence row, from a `22-*` root, succeeds.
  - `git diff --check`.
- **Do not touch:** the harness (`22.3.3`), the convergence CLI and evidence modules (`22.3.4`), `CASES*.json`, `solute_benchmark_bridge.py`, `.development/13.*`.

### 22.3.3 — Harness backend selection and public-API path check (sub-wave 3B)

- **Must edit:** `crates/hydrus-core/tests/solute_verification_harness.rs` (request parsing near `verified_request`, `.with_backend` `:538`, observation `solver_reported` text `~:735`, plus its own `#[test]`s).
- **Can create:** `planning/tasks/22.3.3.todo.md`; scratch `.development/22.3.3/`.
- **Depends on:** `22.3.1`. It runs in parallel with `22.3.2` and relies only on the request contract above.
- **Spec:**
  - Parse the optional `backend` key: absent means `FiniteVolumeBackwardEuler`, `fortran-compatible` means `FortranCompatible`, and anything else is an error.
  - Select the backend and write its name, plus the matching solver description, into the observation. The `FortranCompatible` description must not claim the FV/BE "Linear equilibrium process … backward Euler" metadata.
  - Requests without `backend` produce byte-identical observations.
  - **Path check** (report only, no fixes):
    - Name the `solute_fortran_compat` functions reached by `advance_fortran_compatible_step` (`solute.rs:878`) and by `run_compat_deck` for the transport step, and state whether they are the same kernels.
    - Run the harness on hand-written `reaction_zero` and `diffusion_cosine` component requests with `backend fortran-compatible`, outside the lifecycle, using the env-driven `requested_study` (`:743`), and record that both complete.
- **Tests:** `request_backend_selects_fortran_compatible`, `absent_backend_keeps_fv_be`, `unknown_backend_is_rejected`.
- **Verification:**
  - `cargo test -p hydrus-core --test solute_verification_harness`.
  - fmt, clippy, `git diff --check`.
  - The orchestrator re-pins `POLICY_HASHES` for the harness at wave end.
- **Do not touch:** `tools/`, `solute.rs`, `solute_fortran_compat/**`.
- **Anchors:** `crates/hydrus-core/tests/solute_verification_harness.rs:20-30,500-560,730-760`; `crates/hydrus-core/src/solute.rs:840-900`; `compat_runner.rs` (grep `fn run_compat_deck`); `run_solute_verification_reference.py:253-300` (request format, read-only).

### 22.3.4 — Benchmark, convergence and holdout CLIs on v2; fix the state and classification bugs (sub-wave 3C)

- **Must edit:**
  - `tools/solute_verification_metrics.py` and `tools/test_solute_verification_metrics.py` (added after 22.3.2): allow an optional top-level manifest `backend` key (`MANIFEST_KEYS` `:33`, enforced `:293`); v1 manifests without it stay valid and no other check is loosened.
  - `tools/run_solute_convergence.py`: phase use `:36,51,72,87,144,336,389`.
  - `tools/solute_convergence_evidence.py`: roots `:104-106`, budget read `:498`.
  - `tools/check_solute_benchmarks.py`: the benchmark CLI.
  - their tests, `tools/test_solute_convergence_cli.py`, `tools/test_solute_convergence_evidence.py` and `tools/test_solute_benchmarks.py`.
- **Can create:** `planning/tasks/22.3.4.todo.md`; scratch `.development/22.3.4/`.
- **Depends on:** `22.3.2` and `22.3.3`.
- **Spec:**
  - Make each CLI accept the `22-*` roots for its phase, and apply budget caps only when present (v1).
  - Fix the CLI rejecting a lifecycle-valid `raw_complete` manifest at its `complete` gate (`run_solute_convergence.py:336`). Accept exactly the states the lifecycle API accepts.
  - Fix the Rust classification "path/dict argument" error that O13 recorded (`SOLUTE_CONVERGENCE_EVIDENCE.md`, "Tooling and scientific limits"). Reproduce it first in scratch against a *copy* of an O13 Rust batch; never write to `.development/13.*`.
  - Establish one documented command per phase for v2 rows: benchmark, convergence, and holdout. If no holdout CLI exists, add a holdout path to `run_solute_convergence.py`, reusing `Lifecycle.execute`. In this task holdout rows are exercised with `--dry-run` only.
  - Confirm, without editing, that `solute_convergence_analysis.py` and `solute_verification_metrics.py` accept v2 rows (`solver == "rust"`). If they do not, stop and report (escalation rule 4).
- **Tests:**
  - `test_cli_accepts_raw_complete_like_lifecycle_api`;
  - `test_rust_classification_accepts_batch_path`;
  - `test_v1_defaults_unchanged`;
  - `test_v2_roots_per_phase`;
  - `test_holdout_rows_dry_run_only_in_v2_until_reserved_status_lifted`. The CLI refuses to execute a `reserved_until_threshold_gate` row; Wave 6 lifts the status by committing `THRESHOLDS_V2.json`, and the CLI checks for it.
- **Verification:**
  - The named test modules pass.
  - `--verify-only` of an O13 Rust batch fails with "source identity changed" even without the 22.3.2 edits (pinned source hashes have moved); record that as a known limit and verify only the O13 batch manifests the lifecycle can still verify (the `test_*` verify-only test).
  - A v2 `--dry-run` succeeds for one benchmark, one convergence and one holdout row.
  - `git diff --check`.
- **Do not touch:** the files owned by 22.3.2 and 22.3.3, `CASES*.json`, `solute_benchmark_bridge.py`, `.development/13.*`.
- **Anchors:** `tools/run_solute_convergence.py:20-100,130-150,300-400`; `tools/solute_convergence_evidence.py:90-130,490-506`; `tools/check_solute_benchmarks.py:100-300`; `planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md`.

### 22.3.5 — Fix the public `FortranCompatible` step for prescribed-water transport (sub-wave 3C, parallel with 22.3.4)

Added after 22.3.3: the hand runs of the harness with `backend fortran-compatible` gave results that differ from FV/BE by up to 1.0 (`reaction_zero`) and 1.19 (`diffusion_cosine`). The interior stays unchanged for `diffusion_cosine` (no diffusion), and node 0 drops to 0. The user approved a diagnose-and-fix task (2026-10-22) instead of studying through the deck runner.

- **Must edit:** `crates/hydrus-core/src/solute.rs` (the `advance_fortran_compatible_step` setup, `~:840-1200`, and its in-file tests).
- **Can create:** `crates/hydrus-core/tests/solute_fortran_public_path.rs`; `planning/tasks/22.3.5.todo.md`; scratch `.development/22.3.5/`.
- **Depends on:** `22.3.3` (harness `backend` key; its hand-run requests are in `.development/22.3.3/`).
- **Spec:**
  - Diagnose first. Reproduce with the 22.3.3 requests. Name each cause with a file:line, compared to how `run_compat_deck` builds the same inputs. Suspects: effective dispersion passed into the Fortran `ch_par[1]` dispersivity slot (`solute.rs:~1026-1047`); the `TotalFlux` bottom and `AdvectiveOutflow` top boundary mapping; `reaction_zero` rate and sink mapping; node-0 behaviour.
  - Fix only in the `FortranCompatible` branch of `solute.rs`. The `solute_fortran_compat/**` kernels, FV/BE equations (ADR 0002) and the deck runner stay untouched. If the fix needs those, stop and report (escalation rule 4).
  - The public path must give the physically right answer for prescribed-water transport (D = dispersion/theta semantics as in `SoluteConfig` docs). Expected values come from the analytical references in the tests (cosine decay `exp(-D k^2 t)` for `diffusion_cosine`; zero-order decay for `reaction_zero`) at a tolerance stated in the test, never from solver output.
  - ADR 0007/0010: prove reachability before touching any branch, and preserve f32 conversion points in `solute_fortran_compat/`.
- **Tests:** `public_fortran_compatible_diffusion_matches_cosine_decay`, `public_fortran_compatible_zero_order_decay_matches_analytical`, `public_fortran_compatible_agrees_with_fvbe_on_resolved_diffusion` (loose tolerance, stated).
- **Verification:**
  - `cargo test -p hydrus-core --test solute_fortran_public_path`; `cargo test -p hydrus-core --lib solute`; `cargo test -p hydrus-core --test solute_backend_contract --test solute_solver_selection`.
  - Parity `--fixture-group all` = 44/44 with no candidate digest change.
  - fmt, clippy, `git diff --check`. The orchestrator re-pins `POLICY_HASHES` for `solute.rs`.
- **Do not touch:** `tools/`, the harness, `solute_fortran_compat/**`, `OBSERVATION_MAP.json`, `Cargo.*`.
- **Anchors:** `solute.rs:840-1200`; `compat_runner.rs` (`run_compat_deck`, grep `ch_par`); `.development/22.3.3/` requests and observations; `planning/tasks/22.3.3.todo.md`.

### 22.3.7 — v2-aware bridge gate (added 2026-10-22, user-approved "waive for v2, record why")

Found by 22.4.1–22.4.3: `bridge.require_bridges` (`solute_benchmark_bridge.py:641`), called from `check_solute_benchmarks.py:~276-291,683-685` and `run_solute_convergence.py:~253-254,456`, demands the two O13 bridge reports (`benchmark-example3-bridge-{fortran,rust}-baseline`). `CASES_V2.json` has no Fortran bridge row, so no v2 row can be executed.

- **Must edit:** `tools/solute_benchmark_bridge.py`, `tools/check_solute_benchmarks.py`, `tools/run_solute_convergence.py`, `tools/test_solute_benchmarks.py`, `tools/test_solute_convergence_cli.py`, and the bridge's test module (grep `test_solute_benchmark_bridge*`).
- **Can create:** `planning/tasks/22.3.7.todo.md`; scratch `.development/22.3.7/`.
- **Spec:**
  - For a v2 casebook (`schema_version` v2, detected as in 22.3.1) the bridge gate is not applicable. `require_bridges` is not called for it. Instead the gate writes the same receipt file name (`bridge-gate.json`) with `status: "not_applicable_v2"`, the casebook SHA-256, and the reason: "no Fortran rows in v2; source parity is covered by the 44/44 gate (21.4.1) and the public-path analytical tests (22.3.5)"; `scientific_accuracy_accepted: false`.
  - The v2 `benchmark-example3-bridge-rfc-baseline` row stays an ordinary role=`bridge` row and runs as a recorded Rust run, with no comparison against retained O12 outputs.
  - The v2 `--stage` and `--bridge-report` handling accepts no bridge reports for v2 and rejects any that are passed. v1 behaviour, receipts and error text stay byte-identical (v1 tests unchanged).
  - Do not weaken any v1 check; if the bridge file is hash-pinned anywhere (`COMPARATORS`, recipe hashes, `13-baseline`), report it and list the re-pin the orchestrator must do.
- **Tests:** `test_v2_benchmark_executes_without_bridge_reports`, `test_v2_convergence_executes_without_bridge_reports`, `test_v2_rejects_bridge_reports`, `test_v1_still_requires_both_bridge_reports`, `test_v2_gate_receipt_records_reason`.
- **Verification:** the named modules pass; a v2 `--execute` on one `reaction_zero` benchmark row and one `diffusion_cosine` convergence row, using throwaway activations in scratch, reaches `complete` (these two runs are the proof; keep them in scratch, do not copy); `git diff --check`.
- **Amendment (2026-10-22, after the worker's first proof run):** add `tools/solute_benchmark_lifecycle.py` and `tools/test_solute_benchmark_lifecycle.py` to Must edit. Two lifecycle defects block any v2 `--execute`:
  1. `build_rust` (`:~447`) parses every `cargo.jsonl` line as JSON, but `cargo --verbose --verbose` also prints `[crate x] cargo:rerun-if-changed=...` lines on stdout, so a fresh build fails. Parse only lines that start with `{` (the raw log stays as written; the executable is still selected only from the `compiler-artifact` JSON messages; duplicate-key rejection stays).
  2. `:592-596` row reservation `size + estimated_output_bytes <= 2 * estimated_output_bytes` is a per-row ceiling; it fails for the 2.9 MB debug harness against a 2.28 MB estimate. For books without caps (v2, ADR 0003 / D7) skip this ceiling and the matching global reservation check (`:598`), but still record sizes and estimates. v1 stays byte-identical and its cap tests stay green. Add `test_v1_row_cap_still_enforced`, `test_v2_skips_row_reservation_ceiling`, `test_cargo_jsonl_ignores_non_json_build_lines`.
- **Do not touch:** `CASES*.json`, `.development/13.*`, Rust, the other tools.

### 22.3.8 — Fix chain production in the public `FortranCompatible` step (added 2026-10-22, from the 22.4.1 study)

22.4.1 ran `reaction_distinct` and `reaction_equal` on the fixed public path: the parent S1 is first order in dt, but the daughters S2 and S3 are exactly 0 at every time (relative error 1.0, no change with dt). `reaction_zero` is exact. Suspect: the chain-production slots written by 22.3.5 fix C (`solute.rs:~1134-1141`, `offset+13/14` = `gam_l1/gam_s1`) produce no source in the kernel (a missing enabling flag, wrong parent mapping, or a skipped term). The 22.4.1 report (`planning/master/M5V_STUDY_22_REACTIONS.md` on branch `orchestration/22.4.1`, not merged) has the evidence; its batches are in `.development/22.4.1/`.

- **Must edit:** `crates/hydrus-core/src/solute.rs`; `crates/hydrus-core/tests/solute_fortran_public_path.rs`.
- **Can create:** `planning/tasks/22.3.8.todo.md`; scratch `.development/22.3.8/`.
- **Depends on:** `22.3.5`.
- **Spec:** reproduce with the 22.4.1 requests; name the cause with file:line against `driver.rs`/`transport.rs`/`run_compat_deck`'s own ChPar loading; fix only in the `FortranCompatible` setup in `solute.rs` (kernels, FV/BE and the deck runner stay untouched; stop on rule 4 otherwise). Expected values come from the analytical Bateman chain (closed form), never from solver output. Check that FV/BE chain semantics (`solute.rs:~1625-1629`) are matched, including Kd/retardation of parent and daughters.
- **Tests (new, red first):** `public_fortran_compatible_three_species_chain_matches_bateman` (distinct rates), `..._equal_rates_chain_matches_bateman`, `..._chain_with_sorption_matches_analytical`.
- **Verification:** `cargo test -p hydrus-core --test solute_fortran_public_path --lib solute --test solute_backend_contract --test solute_solver_selection`; parity `--backend fortran-compatible --fixture-group all` pass (44 fixtures); fmt, clippy, `git diff --check`. The orchestrator re-pins `POLICY_HASHES` for `solute.rs`.
- **Do not touch:** `tools/`, the harness, `solute_fortran_compat/**`, `Cargo.*`.

---

## Wave 4 — Refinement study of the default backend (run and report; no code changes)

**Shared spec for 22.4.1–22.4.4:**
- **Must edit:** none. Commit only the report and the todo (A5).
- **Can create:** the report, `planning/tasks/<id>.todo.md`, and scratch `.development/<id>/`.
- **Depends on:** Wave 3.
- **Run** every v2 (`backend: fortran-compatible`) `benchmark`, `study` and `control` row for the case from `CASES_V2.json`, using the per-phase v2 commands documented by `22.3.4` (from the `22-benchmark` and `22-convergence` roots under the worker's own worktree `.development/`, with `--cases reference/solute_verification/CASES_V2.json`). Use the standard lifecycle activation. Copy the verified batches to the main `.development/<id>/` at hand-off. Never run `holdout` rows.
- **Classify** with the evidence API and the CLI, then run `--verify-only` on each batch.
- **Compute against the analytical reference:** per species and time, `profile_linf`, `profile_l2`, `profile_l1`, `endpoint_linf`, `interior_linf`, inventory, boundary, reaction, balance and `temporal_l2`, as defined in `SOLUTE_VERIFICATION_SPEC.md:353-411`.
- **Record for each metric** its named scale and branch (`scale=0` → `branch=zero`, `relative=null`; scale no greater than the reference and rounding uncertainty → `branch=near_zero`, absolute error, `relative=null`; otherwise relative error), per `SOLUTE_VERIFICATION_SPEC.md:365-370`. Report the quadrature split: quadrature inventory against both the exact continuum mass and the analytic truth integrated with the same mesh weights (`:356-362`).
- **Fit orders** only where the spec allows it (positive, decreasing, resolved errors with a small control on the other axis). Report plateaus, sign changes, non-monotonic behavior and f32 floors explicitly. Quote the spec's expected orders as context, not as acceptance.
- **Report sections (`##`, ≥ 7):** Run identity (commit, CASES_V2 SHA, batch IDs, runner and binary SHA-256, commands); Lifecycle and verify status; Metrics table (row × field); Refinement and fitted orders; Floors and uncertainty; Observations against the spec's expected behavior; Inputs for threshold proposal (no thresholds proposed here).
- **Failed rows** are recorded with their lifecycle state and retried at most once, as a new recorded attempt (ADR 0003 sets no fixed count; this cap is the D9 escalation rule). A second failure goes in the report; the task does not chase it.
- **Verification:** A5; the report lists every batch ID with its `--verify-only` exit code, all 0 (or the failure recorded per the failed-rows rule); `grep -c '^## '` ≥ 7; `git diff --check`.
- **Anchors:** `SOLUTE_VERIFICATION_SPEC.md` (the case's section plus `:353-411`); `tools/solute_analytical_reference.py`; `tools/solute_verification_metrics.py`; the `22.3.2`–`22.3.4` reports in `STATE_22.md`; prior FV/BE evidence `SOLUTE_CONVERGENCE_EVIDENCE.md` and `SOLUTE_ANALYTICAL_EVIDENCE.md` for comparison only.

| Task | Case(s) | Report |
|---|---|---|
| 22.4.1 (4A) | `reaction_distinct`, `reaction_equal`, `reaction_zero`: component benchmarks; time study and control for `reaction_distinct` | `planning/master/M5V_STUDY_22_REACTIONS.md` |
| 22.4.2 (4A) | `diffusion_cosine`: benchmark; space and time studies and controls | `planning/master/M5V_STUDY_22_DIFFUSION.md` |
| 22.4.3 (4A) | `scalar_robin`: benchmark; space and time studies and controls | `planning/master/M5V_STUDY_22_ROBIN.md` |
| 22.4.4 (4B) | `example3`: benchmark, bridge; space and time studies and controls; the Laplace reference uncertainty per spec `:169-300`. If temporal error cannot be isolated (the 501-node time control bounds spatial sensitivity only from below), record "inconclusive" (spec `:390-395`). Component-case successes do not replace this full-chain trajectory (spec `:594-595`). | `planning/master/M5V_STUDY_22_EXAMPLE3.md` |

### 22.4.5 — Complete the Example 3 space control (added 2026-10-24, user-approved "authorize resume/longer ceiling")

- **Why:** in 22.4.4 the `convergence-example3-space_control-rfc-baseline` row (1001 nodes, dt 0.0125, 16000 steps) hit the 60 s row timeout; the re-attempt was refused (`row already consumed; explicit authenticated resume required`). No space metrics or node-1 closure result exist for Example 3.
- **Must edit (plan to confirm; STOP and report if more is needed):** `reference/solute_verification/CASES_V2.json` (that row's `timeout_seconds` only, plus a per-row cap field in `limits` if the checker needs one; re-derive the SHA-256), `tools/check_solute_verification_spec.py`, `tools/solute_benchmark_lifecycle.py` (the `<= 60` timeout checks near :182-186), `tools/solute_verification_metrics.py` (`timeout mismatch` check :328), their tests, and the harness test file if it checks the limit. Default 60 s must stay for every other row, including holdouts.
- **Can create:** `planning/master/M5V_STUDY_22_EXAMPLE3_SPACE.md`, `planning/tasks/22.4.5.todo.md`, scratch `.development/22.4.5/`.
- **Spec:** (1) measure how long the row actually needs; (2) choose the smallest per-row timeout (<= 300 s) with margin, record it as a recorded deviation from the 60 s limit; (3) run the row as a NEW attempt in a fresh phase root with the standard v2 flow (do not edit ledgers; do not resume the consumed batch); (4) classify, `--verify-only --batch`, compute the same metrics as 22.4.4 for the space axis (251/501/1001 + control, reusing the 22.4.4 batches by copy), report the node-1 outflow closure observation, fit nothing the spec forbids; (5) report sections as 22.4.4. Update the CASES_V2 SHA wherever it is pinned (grep) and say so. POLICY_HASHES re-pins are the orchestrator's: report the new hashes needed, do not edit `tools/solute_benchmark_bridge.py`.
- **Verification:** A5 (with the Must-edit list above); python tests for the touched tools; `python3 tools/check_solute_verification_spec.py` passes on CASES_V2; every batch `--verify-only --batch` exit 0 (failures recorded); `git diff --check`.

---

## Wave 5 — Synthesis

### 22.5.1 — Error budgets, fitted orders and proposed thresholds

- **Must edit:** none.
- **Can create:** `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_22.md`, `planning/tasks/22.5.1.todo.md`, and scratch.
- **Depends on:** Wave 4.
- **Spec:**
  - From the four study reports and their verified batches (read-only), write per case and field: the error budget (discretization, reference/Laplace uncertainty, quadrature, f32 floor), the fitted orders with their confidence statement, and a **proposed** threshold with its derivation.
  - Thresholds are either absolute, or relative with the named scale and near-zero branch the spec requires.
  - Where no threshold is justified, propose "unassigned" and give the reason.
  - Apply ADR 0009: no threshold may be widened to make a study result pass.
  - Include an "M5-V claims" table in the style of `PHASE_5_SOLUTE_EVIDENCE.md:101-108`, with the status each claim would have if the proposals are approved.
- **Verification:** A5; `grep -c '^## '` ≥ 6; every number traces to a report row (spot-check list in the report); `git diff --check`.
- **Anchors:** the four `M5V_STUDY_22_*.md` reports; `SOLUTE_VERIFICATION_SPEC.md:353-411`; ADR 0006, 0009.

---

## Wave 6 — User gate D10 (orchestrator, no worker)

1. Present the proposed thresholds, the budgets, the fitted orders and the floors from `22.5.1` to the user.
2. On approval, write `reference/solute_verification/THRESHOLDS_V2.json`: the approved thresholds keyed by case, field and metric, with `cases_sha256` set to the CASES_V2 SHA and the approval date. Commit it. Record D10 (dated) in this file.
3. If the user rejects or defers any threshold, record that, and limit Wave 7 to the approved subset or stop at a new gate. Unassigned thresholds remain unmet (`SOLUTE_VERIFICATION_SPEC.md:409`), so a subset run cannot close M5-V.

---

## Wave 7 — Holdouts

### 22.7.1 — Holdout runs against the approved thresholds

- **Must edit:** none.
- **Can create:** `planning/master/M5V_HOLDOUT_22.md`, `planning/tasks/22.7.1.todo.md`, and scratch.
- **Depends on:** Wave 6 (D10).
- **Spec:**
  - Run the three v2 holdout rows (`backend: fortran-compatible`; `diffusion_cosine` 97/0.0078125, `scalar_robin` 193/0.0078125, `example3` 751/0.04) through the holdout command documented by `22.3.4`, from the `22-holdout` root. List every batch ID with its `--verify-only` exit code.
  - Evaluate every metric against `THRESHOLDS_V2.json` (hash-verified) and report pass/fail per field. Never adjust a threshold.
  - Any fail is recorded as FAIL. The task stops and reports, and the orchestrator returns to a user gate.
- **Report sections (≥ 6):** Run identity; Thresholds used (SHA); Results table; Failures (or "none"); Floors; Verdict.
- **Verification:** A5 plus `git diff --exit-code -- reference/solute_verification/THRESHOLDS_V2.json`; `git diff --check`.

---

## Wave 8 — M5-V closure (only after a full 22.7.1 PASS)

M5-V closes only when every field claimed in the `22.5.1` claims table has an approved threshold in `THRESHOLDS_V2.json` and passed its holdout. Otherwise `22.8.1` records the M5-V status as "partial" (listing the unmet fields), M5-V stays open, and the roadmap does not unblock M6.

### 22.8.1 — M5-V closure docs and ADR 0019

- **Must edit:** `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`, `planning/master/VALIDATION_STRATEGY.md`, `planning/master/ROADMAP_AND_RISKS.md` (the M5-V row and the "No scientific solute baseline" risk row), `planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md` (one status line pointing to `_22`), `docs/decisions/README.md`.
- **Can create:** `docs/decisions/0019-validate-scientific-accuracy-on-the-default-backend.md`; `planning/tasks/22.8.1.todo.md`.
- **Depends on:** `22.7.1` PASS on every claimed field (see the Wave 8 rule above).
- **Spec:**
  - Append the same "M5-V closure (22.x)" section to `PHASE_5_SOLUTE_EVIDENCE.md` and `VALIDATION_STRATEGY.md`. Its contents:
    - the claims table;
    - the source-parity evidence (`21.4.1`);
    - the analytical evidence (`22.4.x`, `22.5.1`);
    - the thresholds (`THRESHOLDS_V2.json` SHA);
    - the holdouts (`22.7.1`);
    - the remaining honest gaps: the FV/BE study is not re-run, manufactured forcing is still blocked, the v1 Fortran precision-sensitivity row has no v2 counterpart, and the claims hold only for one compiler and platform.
  - Mark the earlier "M5-V remains open" lines superseded with one line each; delete nothing.
  - In the roadmap, mark M5-V met and say that M6 planning may start, with a link.
  - Write ADR 0019 in the house template:
    - decision: physical and analytical validation runs on the default backend, and source parity ties that evidence to the pinned Fortran;
    - context: D2, D3 and D8;
    - options: re-scope, split the gate, or resume the O13 matrix;
    - validation: the 22.x reports.
  - The README index gains a row for ADR 0019.
- **Verification:** `git diff --check`; every cited path exists; the four docs use the same wording for the closure claim.
- **Do not touch:** `crates/`, `tools/`, `reference/`, `TODO.md`.

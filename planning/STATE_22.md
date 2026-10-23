# State — Orchestration 22

Plan status: **Approved by user on 2026-10-22 (“YES”).**
Execution status: **Waves 1A, 1B, 2, 3A, 3B done; Wave 3C (22.3.4, 22.3.5) next.**
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
| 3C | 22.3.5 | Fix public FortranCompatible step (added 2026-10-22, user-approved) | pending |
| 3C | 22.3.4 | Benchmark/convergence/holdout CLIs on v2; bug fixes | pending |
| 4A | 22.4.1 | Study: closed reactions | pending |
| 4A | 22.4.2 | Study: `diffusion_cosine` | pending |
| 4A | 22.4.3 | Study: `scalar_robin` | pending |
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

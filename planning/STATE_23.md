# State — Orchestration 23

Plan status: **Approved by user on 2026-10-25 ("yes").**
Execution status: **Not started.**
Plan: [EXECUTION_PLAN_23.md](EXECUTION_PLAN_23.md). Worker template: [PROMPT_TEMPLATE_23.md](PROMPT_TEMPLATE_23.md).
Follows: Orchestration 22 (complete; M5-V partial/open; [STATE_22.md](STATE_22.md)).

## Baseline and guardrails

- [ ] Baseline at HEAD (re-confirm in Wave 0; figures from the 22.8.1 final gates, 2026-10-24):
  - `cargo test --workspace --no-fail-fast`: 448 passed, 0 failed, 1 ignored;
  - Python `unittest discover -s tools`: 354 OK;
  - fmt, clippy and `git diff --check`: clean;
  - parity `--fixture-group all`: 44/44.
- [ ] Disk: at least 10 GB free before each wave.
- [ ] Holdout guard: no holdout row has run (A6). Confirm before Wave 7 with `git log` and the `.development/23-holdout/` listing.
- [ ] `CASES_V2.json` SHA-256 frozen at Wave 3 (record below). `THRESHOLDS_V2.json` (SHA `c236eb7c…` before this orchestration) is regenerated at gate C against the frozen value.
- [x] User approves this execution plan (2026-10-25).
- [ ] Gate B (bounds review and signature) given.
- [ ] Gate C (holdout-level thresholds) given.

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 0 | — | Baseline, commit plan | done (1c4e626, 6978756) |
| 1A | 23.1.1 | Holdout guard reads status and SHA | done (f4990be; A4 gate pending wave end) |
| 1A | 23.1.2 | Bridge row inventory mismatch: explain and fix | blocked (user decision A/B on bridge profile set) |
| 1A | 23.1.3 | Diffusion end-row loss: SD-005 or "Not defects" | done (40b6593; SD-005 registered) |
| 2 | 23.2.1 | Isolation study design | done (1650232; 15 study rows, 3 centre rows) |
| 2 | — | Orchestrator gate; `POLICY_HASHES` check | done (all 15 rows inside the holdout bracket; no pinned file edited in 1A, no re-pin) |
| 3 | 23.3.1 | CASES_V2 isolating rows, f32 relabel, checker, bridge print rule | done (0cd74a1; 15 rows, 34 relabels) |
| 3 | — | Tool freeze; record SHA-256 values; `POLICY_HASHES` re-pin | done (freeze values below; harness pin re-pinned) |
| 4A | 23.4.1 | Isolating study: `diffusion_cosine` | pending |
| 4A | 23.4.2 | Isolating study: `scalar_robin` | pending |
| 4A | 23.4.3 | Example 3 space re-run (authenticated); space isolation | pending |
| 4B | 23.4.4 | Example 3 time isolation; bridge re-run | pending |
| 5 | 23.5.1 | Bounds drafting | pending |
| 5 | — | **Gate B**: user signs bounds review | pending |
| 6 | 23.6.1 | Classify; error budgets; threshold proposal | pending |
| 6 | — | **Gate C**: user approves thresholds; regenerate `THRESHOLDS_V2.json` | pending |
| 7 | 23.7.1 | Holdout runs (three rows) | pending |
| 8 | 23.8.1 | M5-V closure record (closed or partial); known limits | pending |

## Frozen values (filled at the named gate)

- CASES_V2.json SHA-256 (Wave 3 freeze): `ceddd59db73e1f54abbfd4198b0b5923ef9838b365bf8c96b691f5ad9718cab7` (was `c236eb7c…`; THRESHOLDS_V2 must carry this value at gate C)
- Tool hashes at freeze (after 23.3.1):
  - `tools/check_solute_verification_spec.py`: `04299ff41ff6f79148ad97155fc2585cce2aaa43d2083c75d15fd756fe0a921b`
  - `tools/solute_verification_metrics.py`: `609a18e04b07cbaeff1b4c75fabcae4710b995bf742dceca0fcfaa94c451dac0`
  - `tools/solute_benchmark_lifecycle.py`: `fcb9094e443670a7c0e736158de47ffe5e59f6c938ffefdb9a5be5ed794c0195`
  - `tools/run_solute_convergence.py`: `f9e20b0eea24d618d464f5fbd1929de4be6840bdc9a59711baef496eeb3beec6`
  - `crates/hydrus-core/tests/solute_verification_harness.rs`: `7bc1783b495ed08b019d05616b6f387a3413353253780fbd5056ba3b024db507`
- `POLICY_HASHES` re-pins: Wave 1A none; Wave 3 `crates/hydrus-core/tests/solute_verification_harness.rs` re-pinned to `7bc1783b…` (D11 justification comment in `tools/solute_benchmark_bridge.py`).

## Verification log

- **Wave 0 baseline at HEAD `25baab4`** (2026-10-25, branch `phase4-water-acceptance`, clean tree apart from planning files):
  - `cargo test --workspace --no-fail-fast`: 448 passed, 0 failed, 1 ignored (matches 22.8.1).
  - `python3 -m unittest discover -s tools -p 'test_*.py'`: Ran 354 tests, OK.
  - `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean. `git diff --check`: clean.
  - Parity `--fixture-group all --backend fortran-compatible`: 44/44 `status: pass`, exit 0 (log `.development/23-gate-0/parity.log`).
  - Disk: 211 GB free on `/` (≥10 GB ✓).
  - Holdout guard: no holdout row run (no `.development/23-holdout/` exists).

## Task reports

### 23.1.1 — done (f4990be)
- Holdout guard: `_gate_holdout(rows, thresholds, cases)` allows holdouts only if the thresholds file parses, is `solute-thresholds-v2`, has `holdout_run_permitted is True`, and `cases_sha256` matches CASES_V2.json. Reason appended to the refusal.
- Tests: `tools.test_solute_convergence_cli` 25 OK (4 new in `HoldoutGuardTests`).
- Deviation (approved): the empty-file "lift" branch of `test_holdout_rows_dry_run_only_in_v2_until_reserved_status_lifted` is removed; its absent-file check kept.
- Gotcha: committed THRESHOLDS_V2.json still has `holdout_run_permitted: false`; holdouts stay refused until gate C.

### 23.1.2 — blocked (not committed; worktree `.development/23-worktrees/23.1.2`)
- Diagnosis: first disagreeing field is `profile_times`: manifest `[0,50,100,150,200]` (Rust-written) vs schedule `[0,50,100,200]` (`solute_verification_metrics.py:185`).
- Row `benchmark-example3-bridge-rfc-baseline` has `sampling_interval: 50` but `analysis_times` `[50,100,200]`. Reference-side rules (`solute_verification_metrics.py:1286-1288`, `solute_convergence_evidence.py:183`) treat `analysis_times` as the profile set.
- User decision needed: A = `analysis_times` governs (row/CASES change, forbidden here except as a Wave 3 fold-in); B = `sampling_interval` governs (tool scope change plus Rust harness check).
- `solute_benchmark_bridge.py:48,540-543` (TIMES, orchestrator-owned) flagged for review.

### 23.1.3 — done (40b6593)
- Reachability: `kBotCh=-1` (TotalFlux 0), `kTopCh=0` (advective outflow), `q=0`, `lVapor` false, `Level=2`, identity mapping. Branch `SOLUTE.FOR:816-818` reached (`D=-1,E=1,F=0`).
- Cross-site: `:811-814` natural row for the same BC when `vN(1)>0`; node-N rows `:894-915` natural. Inconsistent, so SD-005 (not "Not defects").
- Pinning test `source_defect_sd005_diffusion_end_row_loses_inventory` (`crates/hydrus-core/tests/source_defect_sd005.rs`): 3.66e-3 absolute, 2.13e-3 relative loss vs 22.4.2 -3.8e-3 (same order).
- `cargo test -p hydrus-core source_defect_`: 5 passed (SD-001..005). fmt clean, diff check clean.
- Discovered (not registered): free-drainage `SOLUTE.FOR:823-827` (`kBotCh=0`) uses the same `c1=c2` row; not reached by this case.
- Gap: `solute_fortran_compat/boundary.rs` (`assemble_lower`) lacks an SD-005 tag comment (off-limits this task).

### Wave 3 gate and freeze (2026-10-25, after 0cd74a1)
- `cargo test --workspace --no-fail-fast`: 450 passed, 0 failed, 1 ignored (449 + bridge-row harness test).
- Python `unittest discover -s tools`: Ran 363, OK.
- fmt, clippy `-D warnings`, `git diff --check`: clean.
- Parity `--fixture-group all --backend fortran-compatible`: exit 0, 44/44 pass (log `.development/23-gate-3/parity.log`).
- CASES.json (v1) unchanged. CASES_V2: 15 study rows added; 34 `f64` → `f32` on study/control/benchmark rows; the bridge rfc row and the three holdouts keep `f64` (known label limit; checker `V2_PRECISION_F64_EXCEPTIONS`). No holdout row touched.
- 23.3.1 deviation accepted: the existing `bridge_keeps_original_profile_events…` test's ordinary request now uses analysis `[50,100,150]` so the schedule rule does not treat it as a bridge. Its assertions are unchanged.
- Tool freeze: no `tools/` or harness edits after this point without a new orchestrator decision.

### Wave 2 gate (orchestrator)
- Rows approved: 15 study rows (12 bracket + 3 centre), all inside the holdout bracket, so no user stop under the Wave 2 rule. Design: `planning/master/M5V_ISOLATION_DESIGN_23.md` (1650232).
- `POLICY_HASHES`: `solute_verification_harness.rs` is pinned (`a6e15e08…`). 23.3.1 edits it under D11, so the re-pin happens at the Wave 3 freeze, not now.

### Wave 1A A4 gate (2026-10-25, HEAD 5fb79fc, 23.1.2 closed per D11)
- `cargo test --workspace --no-fail-fast`: 449 passed, 0 failed, 1 ignored (448 + SD-005 pinning test).
- Python `unittest discover -s tools`: Ran 358, OK (354 + 4 holdout-guard tests).
- fmt, clippy `-D warnings`, `git diff --check`: clean.
- Parity `--fixture-group all --backend fortran-compatible`: exit 0, 44/44 pass (log `.development/23-gate-1a/parity.log`).
- No `POLICY_HASHES` re-pin needed in Wave 1A (no edits to the hash-pinned files).

## Status refresh (2026-11-06, HEAD 5372004)

- **Execution status:** Orchestration 23 remains open; the original plan's Wave 4 handoff is superseded by the classification/control follow-up described below. Do not use the old `Execution status: Not started` header or Wave 4 `pending` rows as current status.
- **Repository state:** branch `phase4-water-acceptance`; working tree clean at `5372004`. Recent commits from 2026-10-27 through 2026-11-06 add Gate B approval, Wave 4 classification evidence and amendments, classification runner changes, and per-row timeout override support for a controls campaign.
- **Wave 4 runs:** 23.4.1, 23.4.2, and 23.4.4 executed and verified. 23.4.3 solver runs completed, but its combined analysis failed (`co-refined study axes`); it is not a verified classification batch. Reports are `planning/master/M5V_ISOLATION_23_*.md`.
- **Gate B:** signed 2026-10-27. The signed bounds cover 18 study launches; 50 quadrature keys are null/unclassifiable. See `planning/master/M5V_BOUNDS_23.md` and `planning/tasks/23.5.1.todo.md`.
- **First classification outcome:** `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_23.md` records all findings as `inconclusive` (`control_missing`) where classified, and 23.4.3 as unclassified. No accuracy threshold is assigned.
- **Follow-up amendment:** `planning/master/SOLUTE_CONVERGENCE_PLAN_23_AMENDMENT.md` is marked proposed/not started in its header. It calls out that control rows have no signed bounds and that Example 3 fixed-axis grouping needs separation. The 2026-10-29 and 2026-11-06 commits implement additional runner/lifecycle/test support, including per-row timeout overrides, but a complete accepted campaign report and Gate C approval are not present in tracked planning files.
- **Scratch evidence caution:** `.development/23-bounds/classify-run.log` records eight original execute attempts exiting 2; `.development/23-bounds/fresh-run.log` records four fresh attempts exiting 2. The separate `.development/23-worktrees/23.4.4/.development/23-controls-5/` tree contains multiple planned, incomplete, and classified batch directories; this scratch inventory alone does not establish a verified, complete control campaign. Preserve it and reconcile batch lifecycle/verify receipts in a new tracked report before marking classification complete.
- **Gate C / holdouts:** not approved. `reference/solute_verification/THRESHOLDS_V2.json` still has `holdout_run_permitted: false`; no holdout row is authorized or recorded as run. M5-V remains open and M6 is not unblocked.
- **Next action:** reconcile the latest controls campaign's approval, batch receipts, verification results, bounds coverage, and classification output; then update this state report and `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_23.md` before proposing Gate C thresholds.

## Blocker reconciliation (2026-11-07, HEAD 2165eb9)

Full report: [M5V_BLOCKERS_23.md](master/M5V_BLOCKERS_23.md).

- **Controls reconciled.** The six executable `23-controls-8` batches re-verified at HEAD with exit 0 (`evidence_verified`). They cover the original 22 study levels, not the 23 holdout-bracket levels, and were run without bounds.
- **B1 — stale study batches.** The `23-classify-3` recipes differ from current producer hashes in six tool files, so those batches cannot verify or replay at HEAD. The study rows must run again after the final freeze.
- **B2 — stale bounds review.** The signed study-bounds review binds to the old producer hashes. The values are unchanged, but the review must be regenerated and re-signed. The signed control-bounds review matches current hashes; its bounds file currently lives only under `/tmp` and must be copied to a durable path.
- **B3 — control gap.** Matching controls exist only for the Example 3 space groups (1001 nodes at `.0125`). Diffusion space/time, scalar_robin space/time, and Example 3 time have no valid other-axis control. Without new rows, every new-level finding stays `control_missing` and `inconclusive`.
- **B4 — one more CASES change.** Adding controls changes the CASES_V2 SHA; thresholds and reference caches must be regenerated. The study re-run is already forced by B1, so the incremental cost is small.
- **B5 — grouping.** The analyzer still groups by `(case, solver, axis, precision)`. The proposed fixed-axis grouping change does not by itself pair a control row with the study group. Run one batch per fixed-axis value instead.
- **B6 — freeze.** The tool freeze is not in force. Any edit after the final freeze restarts B1/B2.
- **Gates.** Gate C is still not approved. `THRESHOLDS_V2.json` still pins the old CASES SHA and `holdout_run_permitted: false`; no holdout row has run.

### Continuation board

| Phase | Task | Status |
|---|---|---|
| R1 | Blocker report and reconciliation | done (this report) |
| R2 | User decisions D12 (add controls or accept partial) and D13 (drop or retain Example 3 `dt .025` re-run) | done 2026-11-07: add controls; drop the `dt .025` re-run |
| R3 | Add control rows, update v2 checker/tests, freeze tools | done (`0c3f486`; gate below) |
| R4 | Regenerate combined study+control bounds; Gate B re-sign | draft ready; awaiting Gate B |
| R5 | Execute and verify six final batches (one per fixed-axis group) | pending |
| R6 | Classify, error budgets, threshold proposals; Gate C | pending |
| R7 | Holdout runs and M5-V closure record | pending |

### 23R.1 gate (2026-11-07)

- **Control rows added** (`0c3f486`): diffusion space control 129 at `2^-8`, diffusion time control 193 at `2^-8`, Robin space control 257 at `2^-8`, Robin time control 257 at `2^-8`, Example 3 time control 1001 at `0.025` (180 s). Example 3 space at `dt .04` uses the existing 1001 at `.0125` control.
- **New CASES_V2 SHA-256:** `420af7c0085f3ad5cf3335ca036ca5360d7abd370807451d8d479e653a215df7` (58 launches, 7 cases). `THRESHOLDS_V2.json` and the study-bounds review are stale by design.
- **Campaign root:** `23-final` registered in the reference, evidence, cache, harness and CLI test maps.
- **Baseline Clippy fix** (`f798762`): a pre-existing `needless_late_init` in `water_compat.rs`, exposed by the current toolchain. Behavior unchanged; water tests pass.
- **Orchestrator `POLICY_HASHES` re-pin:** `tools/solute_benchmark_bridge.py` now pins the harness at `32d1f6888b0808d71506f01ccbc3e48fcd94557fa11b7648ffde80d1ec892c26`; bridge arithmetic unchanged.
- **Gates:** `cargo test --workspace --no-fail-fast` 450 passed, 0 failed; Python `unittest discover -s tools` 370 OK; `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `git diff --check` clean; parity `--fixture-group all --backend fortran-compatible` 44/44 pass (`.development/23R.1/parity.log`).
- **Freeze values for 23R.2** (`producer_sha256`, current HEAD):
  - `tools/check_solute_benchmarks.py` `6e69b5adcdea925fbecb7e9c4741a64d1d354f326ba6d475087e79f3f580f68f`
  - `tools/check_solute_verification_spec.py` `fa8adcf78dee760f84df510781b3cba098dec0eb0bfa3bb3d824fd397aea6c33`
  - `tools/run_solute_convergence.py` `a0871cda8001af19ff8b4586a7f86b43b4ac5c889a029766409688552d4b6170`
  - `tools/run_solute_verification_reference.py` `27cbafd86d11ac9e37a6c4cea8034385bcafeef181204ddb88b6be8be9cf5b56`
  - `tools/solute_analytical_reference.py` `873d0df6ae69d40c432dd0b39c7cf401f9ff04f3069fb9187e321bc36e219b06`
  - `tools/solute_benchmark_bridge.py` `caa9cad3661286047a3d2a5f6bbd5f3c1b11044489273e1c38d1007ee5f4d0fa`
  - `tools/solute_benchmark_lifecycle.py` `325935bf31add8a5e7edf40d18f0aa25d4b7525871368d87d099bb1f80123d73`
  - `tools/solute_convergence_analysis.py` `bd905699036d826e4b04f713a2a4a553f3562257a872888413d648abc3a348d4`
  - `tools/solute_convergence_evidence.py` `87bc4ba4e474ea2372b89f4e77d6575cdd02250258ab4d96765a68dbac8de86e`
  - `tools/solute_reference_cache.py` `c5acd63dd44390f76d8152c448ed2bdfc5aa8f8f739dcdde4ee77ed0627bff46`
  - `tools/solute_verification_metrics.py` `609a18e04b07cbaeff1b4c75fabcae4710b995bf742dceca0fcfaa94c451dac0`
- **Next:** 23R.2 regenerates the 18 study-bound producer hashes, merges the five control bounds plus the existing Example 3 space control into one source, copies the control bounds out of `/tmp`, and prepares the Gate B review. No final campaign runs before Gate B.

### 23R.2 gate (2026-11-07)

- **Pre-run:** four cheap new controls executed bounds-free in main `.development/23-controls-8/`, batch `batch-222c1bdaa1c244acbdeb9c42ccd5df0b`, verify exit 0 (`evidence_verified`). The Example 3 time control was not re-run; its bounds derive from the authenticated space 1001/0.025 run in `batch-980de4772e034c2baac25f34a9465818`.
- **Combined bounds:** `.development/23-bounds/combined_bounds_23.json`, SHA-256 `3ac9fd8a45018d850da20fefc2877093f8fcf73910b4a7133a54ec8a039a6ee4`. 24 launches, 63,560 entries (18 study + 5 new controls + the existing Example 3 space control). Study entries are byte-identical to the 2026-10-27 values; only `cases_sha256` and `producer_sha256` changed. Every control quadrature is null.
- **Durable control copy:** `.development/23-bounds/control_bounds_controls8.json`, SHA-256 `4289b507bc2be32544be03043a92d5885147c876a464244a6d48dd14c346d2cf` (copied from `/tmp`, byte-identical).
- **Review draft:** `.development/23-bounds/review-combined-draft.json`, SHA-256 `1948c0d1196525991dc43f697854a6e1a23d15748fbb4fc1bd89c121f25af761`, authorization `DRAFT — NOT SIGNED; awaiting Gate B user sign-off`. Not usable for classification until signed.
- **Gate B:** user reviews the derivations and signs. No final campaign runs before the signature.

# Orchestration 23 — Close the open M5-V items (full close attempt)

## Goal and scope

**Goal.** Orchestration 22 finished with M5-V partial and open (ADR 0019). The open items below block closure. This orchestration works on all of them and attempts full closure of M5-V on the default backend (`FortranCompatible`), under ADR 0009: thresholds come from error budgets and isolating studies, are approved by the user at a gate, and are then checked once on the reserved holdout rows. M5-V closes only if every claimed field passes its holdout. Otherwise it stays open and the closure record says so.

**In scope:**
1. The holdout guard in the convergence CLI (currently checks file presence only).
2. The Example 3 bridge row failure ("record inventory mismatch").
3. Registration of the `diffusion_cosine` end-row mass loss as an SD, if the register's own rules support it.
4. A classifier bounds source (bounds file plus review file), drafted by a worker and authorized by the user.
5. Authenticated re-run of the Example 3 space numbers, once, after all tool edits are frozen.
6. Isolating studies at the holdout mesh and time step, for `diffusion_cosine`, `scalar_robin` and `example3`.
7. The `CASES_V2` change: isolating rows and the `f64` → `f32` relabel, in one SHA change.
8. Holdout-level thresholds (user gate), a regenerated `THRESHOLDS_V2.json`, the holdout runs, and the closure docs.
9. The O13 `--verify-only` limit: recorded as a known limit in the closure docs.

**Out of scope:**
- M6 heat, M7 hysteresis, M8–M10. M6 planning starts only if M5-V closes.
- FV/BE re-study. FV/BE stays selectable with its O13 evidence as recorded.
- Re-running Fortran. Parity is already 44/44 (ADR 0006, 0010).
- Widening any limit, or tuning any threshold to fit holdout data (ADR 0009).
- Manufactured-forcing cases (blocked in `CASES.json`).
- Changing `CASES.json` (v1), `.development/13.*`, `reference/` (other than the v2 files named here), `src_reference/`, `OBSERVATION_MAP.json`, parity tolerances or comparator logic.
- Re-pinning provenance on O13 batches. Their `--verify-only` limit stays as recorded.
- Dropped Fortran-only `sensitivity` row (honest gap from 22.3.1; stays listed).
- `TODO.md` (workers never touch it).

**Verbatim user constraints:** "Open items" list pasted 2026-10-25 (pasted_content `841e`):
- "Holdouts: … Don't run holdout rows. Closing M5-V needs isolating studies at the holdout mesh and time step, and a new threshold approval."
- "Diffusion end-row mass loss … Registering it needs a task that can edit docs/FORTRAN_SOURCE_DEFECTS.md and prove the branch is reachable."
- "Classifier: it has no bounds source, so every finding is inconclusive."
- "Evidence quality: the Example 3 space-axis numbers come from a scratch replay … any later checker edit forces about 3 hours of reference recompute."
- "Bridge row: … I never explained the failure."
- "Smaller gaps: CASES_V2's "f64" precision label is schema-only … The O13 --verify-only limit remains."

## Decision log

| Date | Decision |
|---|---|
| 2026-10-25 | D1. Scope: full M5-V close attempt. Fixes, bounds, isolating studies, the user threshold gate, then holdouts. Closure only if every claimed field passes. |
| 2026-10-25 | D2. Bounds: the orchestrator's workers draft the bounds from written derivations. The user reviews and signs the review file (the root authorization). Nothing is authorized without that sign-off. |
| 2026-10-25 | D3. Example 3 evidence: all tool edits land in one early batch and the checker hash is frozen there. The Example 3 space rows are then re-run once through the normal classify path. No re-run between tool edits. |
| 2026-10-25 | D4. `CASES_V2.json`: one SHA change that adds the isolating rows **and** relabels `precision` from `f64` to `f32`. This reverses the earlier "document, do not relabel" call (Orchestration 22 notes). The reason is that the SHA change is already forced by the isolating rows, so the relabel adds no recompute. The checker is updated in the same change. |
| 2026-10-25 | D5. Bridge row: diagnose from a copy of the 22.4.4 evidence, then fix in `tools/` only. Editing `CASES_V2` rows for the bridge is not allowed in this orchestration (it would be a second SHA change). Stop and report if a row change is needed. |
| 2026-10-25 | D6. Claims are frozen before the isolating studies. Every claimed field that is not isolated and approved at the threshold gate keeps M5-V partial. No field is dropped after holdouts. |
| 2026-10-25 | D7. (Default, not asked.) SD registration follows `docs/FORTRAN_SOURCE_DEFECTS.md` "Rules" and "Not defects". If another site in `SOLUTE.FOR` computes the same end-row quantity consistently, the item is logged under "Not defects" as a modelling choice, not as SD-005. Reachability is proven per ADR 0007 before any entry. |
| 2026-10-25 | D8. (Default, not asked.) O13 `--verify-only` stays a documented known limit. Retained batches are not re-pinned. |
| 2026-10-25 | D9. (Default, not asked.) Holdout guard: holdouts run only when `THRESHOLDS_V2.json` parses, has `schema_version: solute-thresholds-v2`, `holdout_run_permitted: true`, and `cases_sha256` equals the SHA-256 of the current `CASES_V2.json`. File presence alone is not enough. `holdout_run_permitted` is set to `true` only by the orchestrator after the threshold gate (Wave 6). |
| 2026-10-25 | D11. (User decision, 2026-10-25: fold into 23.3.1.) Bridge row: its declared profile set is `analysis_times` [50,100,200]. The Rust harness prints the 150 profile because `crates/hydrus-core/tests/solute_verification_harness.rs:221-231` applies the legacy profile schedule only when `launch_id == benchmark-example3-bridge-rust-baseline`; the `-rfc-baseline` row falls through to every `sampling_interval`. The fix is a harness print-step rule keyed on `policy: legacy_bridge` and the row's `analysis_times`, landed in the Wave 3 tool batch before the freeze (D3). `CASES_V2.json` bridge rows are **not** edited, so the SHA change stays the isolating-row change only. 23.1.2 is closed with no tool edit: its hard-coded inventory list `[0,50,100,200]` already equals `[0]+analysis_times`. D5 is superseded for this row by D11. |
| 2026-10-25 | D10. Execution setup: as Orchestration 22 (D9 there): ≤3 concurrent workers in `.development/23-worktrees/<id>` on branch `orchestration/<id>`, `inherit` model tier, one escalation retry, serial wave-end gates (A4), orchestrator-owned `POLICY_HASHES` re-pins. |

## Reconnaissance log (2026-10-25, branch `phase4-water-acceptance`, clean)

- **Prior state.** Orchestration 22 complete. `STATE_22.md` final gates (2026-10-24): cargo workspace 448/0/1, Python 354 OK, fmt/clippy clean, parity 44/44. M5-V partial/open; ADR 0019 "Accepted; M5-V open".
- **Next number:** 23. Phase roots: `.development/23-*`.
- **Holdout guard.** `tools/run_solute_convergence.py:27` (`THRESHOLDS_V2` path), `:48-52` (`_gate_holdout`: checks `THRESHOLDS_V2.is_file()` only), `:408-456` (`--holdout` CLI). The file's `holdout_run_permitted: false` is ignored.
- **Reserved rows.** `tools/check_solute_verification_spec.py:340,363` checks `reserved_until_threshold_gate`.
- **Bridge failure.** `tools/solute_verification_metrics.py:420-431`: `inventory` must match `launch["nodes"]`, species count, `profile_times == times`, and `step_count == len(steps)`, else "record inventory mismatch". The 22.4.4 report (`planning/master/M5V_STUDY_22_EXAMPLE3.md:27`) records the row as `legacy_bridge` with `mapping_validation` failing. Evidence: `.development/22.4.4/` (batch `batch-9c2c024908a84c3eb67b429e39e8e5fc`). Cause not yet known.
- **Bounds.** `tools/solute_convergence_evidence.py:273` requires `solute-convergence-bounds-v1`; `:286` requires `solute-convergence-bounds-review-v1`. Both bind to the current tool hashes (`producer_sha256`), so bounds produced before the tool freeze go stale. `tools/solute_convergence_analysis.py:183-195` parses bounds. Required fields are in `planning/master/M5V_STUDY_22_ROBIN.md:137`.
- **Holdout rows** (from `CASES_V2.json`): `diffusion_cosine` 97 nodes, dt 0.0078125; `scalar_robin` 193 nodes, dt 0.0078125; `example3` 751 nodes, dt 0.04. `reference/solute_verification/THRESHOLDS_V2.json` pins `cases_sha256` `c236eb7c…`.
- **Isolation gap.** Study levels in `CASES_V2` (per `EXECUTION_PLAN_22.md` recon): diffusion space 17/33/65, time 257 nodes at three dt; robin space 33/65/129, time 257; Example 3 space 251/501/1001, time 1001 at dt 0.2/0.1/0.05. None sits at the holdout mesh or dt. Isolation needs new rows, which are the CASES change (D4).
- **Defect register.** `docs/FORTRAN_SOURCE_DEFECTS.md`: SD-001 to SD-004 exist; next ID is SD-005. Rules: pinning test `source_defect_sdNNN_*`; "Not defects" section (line 128). Candidate: `diffusion_cosine` inventory loss, `SOLUTE.FOR:803-827` end-row `c1=c2` treatment (`planning/master/M5V_STUDY_22_DIFFUSION.md`).
- **Checker hash.** `tools/check_solute_verification_spec.py` is part of the reference-cache identity. Any edit forces roughly 3 h of reference recompute per reference, so tool edits are batched (D3).
- **O13 verify-only.** `tools/solute_benchmark_lifecycle.py:587` and `:902` ("source identity changed"): pinned source hashes moved. Known limit (D8).
- **Disk.** Check ≥10 GB free before each wave (STATE_23 baseline checklist).

## Hazards

- **CASES_V2 SHA is load-bearing.** `THRESHOLDS_V2.json` pins it. Any later `CASES_V2` edit invalidates the thresholds and every reference cache. Wave 3 is the only wave that edits `CASES_V2.json`.
- **Tool freeze.** After Wave 3 and before Wave 4, no `tools/` edits. Any later change re-runs Wave 3's freeze check and may force the recompute cost again. Orchestrator approval only.
- **Bounds go stale with tool edits.** Bounds are produced after the freeze (Wave 5), never before.
- **Holdouts are unseen.** Holdout outputs never inform thresholds. Isolating studies use non-holdout rows at the holdout mesh/dt only. A holdout row is never launched before Wave 7, and Wave 7 needs the threshold gate (D9).
- **Fixed-form Fortran and reachability.** Before any SD entry, prove the branch is reachable for the deck's selectors (ADR 0007).
- **f32 floors.** Report floors and plateaus explicitly. Never fit an order through one (`SOLUTE_VERIFICATION_SPEC.md` "Norms, refinement and budgets").
- **`POLICY_HASHES`.** Edits to `solute_verification_harness.rs`, `solute.rs` or the bridge's hash-pinned files invalidate `tools/solute_benchmark_bridge.py` `POLICY_HASHES`. The orchestrator re-pins at the wave end, with a justification comment. Workers never edit `solute_benchmark_bridge.py`.
- **Worktrees.** Use `.development/23-worktrees/<id>` and `.development/23-target/<id>`. Keep parity and study evidence under `.development/<id>/`. Never stage `target/`, `.development/` or `.claude/`.

## Shared acceptance rules (every task)

- **A1.** No existing test assertion or tolerance is weakened.
- **A2.** Every targeted command in the task spec passes. The worker reports exact before/after counts.
- **A3.** Rust tasks: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean in the worktree. Python tasks: the named `tools/test_*.py` modules pass.
- **A4.** At each wave end the orchestrator runs serially: `cargo test --workspace --no-fail-fast`; `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`; fmt; clippy; `git diff --check`; and parity `--fixture-group all` = 44/44 whenever `crates/` or `tools/solute_rust_candidate_adapter.py` changed. Results and any `POLICY_HASHES` re-pin go in `STATE_23.md`.
- **A5.** Study and report tasks change no code. `git diff --exit-code -- src_reference reference/solute_verification/CASES.json crates tools Cargo.toml Cargo.lock` is clean at hand-off, except where the task names an edit. The only committed files are the task's report doc and todo. Run artifacts stay under `.development/<id>/`. Their manifest SHA-256 values appear in the report.
- **A6.** No task reads or runs `holdout` rows before Wave 7.

## Wave table

| Wave | Task | Title | Must-edit / create (disjoint within wave) |
|---|---|---|---|
| 0 | — | Orchestrator: baseline, commit plan | `planning/{EXECUTION_PLAN,PROMPT_TEMPLATE,STATE}_23.md` |
| 1A | 23.1.1 | Holdout guard reads the threshold file's status and SHA | `tools/run_solute_convergence.py`, `tools/test_solute_convergence_cli.py` |
| 1A | 23.1.2 | **Closed (D11): diagnosis only, no tool edit.** Bridge row inventory mismatch | none (fix moved to 23.3.1) |
| 1A | 23.1.3 | Register or log the diffusion end-row loss | `docs/FORTRAN_SOURCE_DEFECTS.md`; create `crates/hydrus-core/tests/source_defect_sd005.rs` (only if SD-005 is registered) |
| 2 | 23.2.1 | Isolation study design (report only) | create `planning/master/M5V_ISOLATION_DESIGN_23.md` |
| 2 | — | Orchestrator: tool-batch gate (A4) and `POLICY_HASHES` check | `planning/STATE_23.md` |
| 3 | 23.3.1 | `CASES_V2` isolating rows, f32 relabel, checker update, bridge print rule | `reference/solute_verification/CASES_V2.json`, `tools/check_solute_verification_spec.py`, `tools/test_check_solute_verification_spec.py`, `crates/hydrus-core/tests/solute_verification_harness.rs` |
| 3 | — | Orchestrator: tool freeze; record checker SHA; no later `tools/` edits | `planning/STATE_23.md` |
| 4A | 23.4.1 | Isolating study: `diffusion_cosine` at holdout mesh and dt | create `planning/master/M5V_ISOLATION_23_DIFFUSION.md` |
| 4A | 23.4.2 | Isolating study: `scalar_robin` at holdout mesh and dt | create `planning/master/M5V_ISOLATION_23_ROBIN.md` |
| 4A | 23.4.3 | Example 3 space re-run (authenticated) and isolating study | create `planning/master/M5V_ISOLATION_23_EXAMPLE3.md` |
| 4B | 23.4.4 | Example 3 time-axis isolation at dt 0.04 | create `planning/master/M5V_ISOLATION_23_EXAMPLE3_TIME.md` |
| 5 | 23.5.1 | Bounds drafting: arithmetic and quadrature bounds per launch | create `planning/master/M5V_BOUNDS_23.md`, `.development/23-bounds/` (scratch bounds JSON) |
| 5 | — | **User gate B**: review and sign the bounds (`solute-convergence-bounds-review-v1`) | `planning/*_23.md` |
| 6 | 23.6.1 | Classify all study batches with the bounds; error budgets; holdout thresholds proposal | create `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_23.md` |
| 6 | — | **User gate C**: approve holdout-level thresholds; orchestrator regenerates `THRESHOLDS_V2.json` with `holdout_run_permitted: true` | `reference/solute_verification/THRESHOLDS_V2.json`, `planning/*_23.md` |
| 7 | 23.7.1 | Holdout runs (three rows) against approved thresholds | create `planning/master/M5V_HOLDOUT_23.md` |
| 8 | 23.8.1 | M5-V closure record (closed or partial) and the O13/f64 known limits | `planning/master/{PHASE_5_SOLUTE_EVIDENCE,VALIDATION_STRATEGY,ROADMAP_AND_RISKS,SOLUTE_CONVERGENCE_EVIDENCE}.md`, `docs/decisions/0020-*.md` (create) or `0019-*.md` status update, `docs/decisions/README.md` |

**Dependency chain.** W0 → 1A (23.1.1, 23.1.3; 23.1.2 closed, D11) → W2 (design, orchestrator gate) → W3 (CASES change + tool freeze) → 4A → 4B → W5 (bounds → gate B) → W6 (classify → gate C) → W7 → W8.

- Wave 1A tasks are disjoint and run in parallel. The bridge fix is no longer in 1A: under D11 it is part of 23.3.1's harness batch.
- Wave 2 design fixes the isolating rows that Wave 3 adds. No CASES edit happens before the design is approved.
- Wave 3 is the single SHA change. After its tool freeze, the bounds in Wave 5 bind to the frozen tool hashes.
- Wave 4 is the Example 3 space re-run. It is authenticated through the normal classify path (D3).
- Gate B (bounds sign-off) and gate C (threshold approval) are hard user stops. If either is not given, the orchestrator stops at that gate and does not proceed to the next wave.
- If Wave 7 fails any holdout, Wave 8 records M5-V partial. Thresholds are never re-tuned on holdout data.

---

## Wave 0 — Orchestrator (no worker)

1. Record the baseline in `STATE_23.md`: cargo workspace 448/0/1, Python 354 OK, fmt/clippy/diff clean, parity 44/44 (re-confirm at HEAD). Confirm ≥10 GB free.
2. Commit `planning/{EXECUTION_PLAN,PROMPT_TEMPLATE,STATE}_23.md` with message `docs(planning): plan the M5-V open-item close attempt`.

---

## Wave 1A — Tool and record fixes (parallel, disjoint)

### 23.1.1 — Holdout guard reads the threshold file's status and SHA

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `tools/run_solute_convergence.py` (`_gate_holdout`, `:48-52`).
- **Can create:** `tools/test_solute_convergence_cli.py` cases; `planning/tasks/23.1.1.todo.md`; scratch `.development/23.1.1/`.
- **Depends on:** Wave 0.
- **Spec:**
  - `_gate_holdout` allows a reserved holdout row to run only if `THRESHOLDS_V2.json` exists and parses, `schema_version == "solute-thresholds-v2"`, `holdout_run_permitted is True`, and `cases_sha256` equals the SHA-256 of the current `reference/solute_verification/CASES_V2.json`. Any failure raises the existing `require` error text, extended with the reason.
  - The check works on a copy of the thresholds path, so tests can use temp files. Change the module constant only through an argument, not by monkey-patching.
  - Do not edit `THRESHOLDS_V2.json` itself.
- **Tests (red first):** `holdout_refused_when_thresholds_file_absent`; `holdout_refused_when_not_permitted`; `holdout_refused_on_sha_mismatch`; `holdout_allowed_when_permitted_and_sha_matches`.
- **Verification:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_convergence_cli`; `git diff --check`.
- **Do not touch:** `THRESHOLDS_V2.json`, `CASES*.json`, `solute_convergence_analysis.py`, `solute_benchmark_bridge.py`.
- **Anchors:** `tools/run_solute_convergence.py:27,48-52,408-456`.

### 23.1.2 — Explain and fix the bridge row's inventory mismatch

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `tools/solute_verification_metrics.py` (`:420-431`, inventory check); `tools/test_solute_verification_metrics.py`.
- **Can create:** `planning/tasks/23.1.2.todo.md`; scratch `.development/23.1.2/` (copy of the failing 22.4.4 bridge evidence; never edit `.development/22.4.4/`).
- **Depends on:** Wave 0.
- **Spec:**
  - **Diagnose first.** Copy the failing bridge manifest and record into scratch. Name the first inventory field that disagrees with the launch schedule (`nodes`, `species`, `profile_times`, `step_count` or `endpoint_times`), with the values and the file:line that produced each.
  - Also check `endpoint_times` (`:429-431`): for a `solver: "rust"` bridge row it must equal `[0.0] + steps`.
  - If the check is wrong for v2 (rust rows, `legacy_bridge` policy), fix the check in `tools/` only. Keep v1 behavior and error text unchanged.
  - If the row itself is wrong (its schedule), stop and report (D5). Do not edit `CASES_V2.json` here.
- **Tests:** `test_bridge_inventory_matches_legacy_schedule`; `test_v1_inventory_check_unchanged`; `test_bridge_mismatch_still_rejected`.
- **Verification:** `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_verification_metrics`; `git diff --check`. The bridge row itself is not re-run here (its re-run is in Wave 4).
- **Do not touch:** `CASES*.json`, `solute_benchmark_bridge.py` (orchestrator-owned `POLICY_HASHES`), `.development/22.4.4/`.
- **Anchors:** `tools/solute_verification_metrics.py:405-431`; `planning/master/M5V_STUDY_22_EXAMPLE3.md:27,178`.

### 23.1.3 — Register the diffusion end-row loss (SD-005) or log it as a modelling choice

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `docs/FORTRAN_SOURCE_DEFECTS.md`.
- **Can create:** `crates/hydrus-core/tests/source_defect_sd005.rs` (only if SD-005 is registered); `planning/tasks/23.1.3.todo.md`; scratch `.development/23.1.3/`.
- **Depends on:** Wave 0.
- **Spec:**
  - **Reachability (ADR 0007).** Prove the `SOLUTE.FOR:803-827` end-row `c1=c2` branch is reached for the deck selectors used by the `diffusion_cosine` case. Name each selector and its value. A branch not reached is not registered.
  - **Cross-site check** (register rule). Find every other site in `SOLUTE.FOR` computing the same end-row quantity (`:895-915` top row and the others). If they all follow the same rule, the behavior is a modelling choice: add a "Not defects" bullet with the evidence, and do not create SD-005.
  - If the defect holds (reachable, and inconsistent with another site), register SD-005 in the format of SD-001 to SD-004. Add the pinning test `source_defect_sd005_*` under `crates/hydrus-core/tests/source_defect_sd005.rs`. Use a doc comment naming SD-005. The test must fail if fixed.
  - Evidence of mass loss: reuse the 22.4.2 numbers (`planning/master/M5V_STUDY_22_DIFFUSION.md`); do not run new rows here.
- **Tests:** if registered, `source_defect_sd005_*` passes on the current code and `cargo test -p hydrus-core source_defect_` runs it.
- **Verification:** `cargo test -p hydrus-core source_defect_` (only if a test was created); fmt and clippy if Rust changed; `git diff --check`.
- **Do not touch:** `solute_fortran_compat/**`, `solute.rs`, `src_reference/`, any tool.
- **Anchors:** `docs/FORTRAN_SOURCE_DEFECTS.md` ("Rules", "Entries", "Not defects" at `:128`); `src_reference/SOLUTE.FOR:803-827,895-915`; `planning/master/M5V_STUDY_22_DIFFUSION.md`.

---

## Wave 2 — Design and orchestrator gate (no code)

### 23.2.1 — Isolation study design (report only)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/M5V_ISOLATION_DESIGN_23.md`; `planning/tasks/23.2.1.todo.md`.
- **Depends on:** Wave 0.
- **Spec:**
  - For each holdout-level field (`diffusion_cosine` space 97 at dt 0.0078125; `scalar_robin` space 193 at dt 0.0078125; `example3` space 751 at dt 0.04, time at dt 0.04): design rows that isolate the holdout mesh (space) and holdout dt (time).
  - Each row is `backend: "fortran-compatible"`, role `study` or `control`. No row may be `holdout`. Mesh and dt levels bracket the holdout value from the existing study levels in `CASES_V2.json`.
  - For each proposed row state: nodes, dt, `launch_id` (tag `rfc`), role, which axis it isolates, and the expected cost (`cost_estimate`). Compare with the existing rows to avoid duplicates.
  - Name the isolated field list (must match the claims table in `SOLUTE_CONVERGENCE_EVIDENCE_22.md` "M5-V claims"). Mark any field that cannot be isolated as blocking closure (D6).
  - No rows are created or run here. The design is the input to 23.3.1.
- **Verification:** `git diff --check`; every `CASES_V2.json` row ID cited exists (grep).
- **Do not touch:** `CASES*.json`, `tools/`, `crates/`.
- **Anchors:** `reference/solute_verification/CASES_V2.json` (rows and `holdout` status); `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_22.md` (claims and unassigned list); `SOLUTE_VERIFICATION_SPEC.md:353-411`.

### Orchestrator: Wave 2 gate

1. Approve 23.2.1's row list. If the design needs a row outside the holdout mesh/dt bracket, stop and ask the user before Wave 3.
2. Check `tools/solute_benchmark_bridge.py` `POLICY_HASHES` for pins on the files edited in Wave 1A. Record which need re-pinning after Wave 3.
3. Record the Wave 1A results in `STATE_23.md`, with A4 gates.

---

## Wave 3 — The one CASES_V2 change and the tool freeze

### 23.3.1 — Add isolating rows, relabel `f64` to `f32`, update the checker

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `reference/solute_verification/CASES_V2.json`, `tools/check_solute_verification_spec.py`, `tools/test_check_solute_verification_spec.py`.
- **Can create:** `planning/tasks/23.3.1.todo.md`; scratch `.development/23.3.1/`.
- **Depends on:** `23.1.1`, `23.1.2`, `23.1.3`, `23.2.1` (all merged; A4 gate green).
- **Spec:**
  - Add exactly the rows approved in 23.2.1, with `backend: "fortran-compatible"`, `rfc` launch IDs, `status` `"study"`/`"control"` as designed. Do not touch any `holdout` row.
  - Change every `precision: "f64"` on rfc rows to `"f32"` (D4). Keep v1 `CASES.json` unchanged.
  - Update the checker so the v2 checks still pass: every case still equals v1 by canonical hash (the relabel is a documented exception only if the checker currently compares `precision` on rfc rows; state which in the report), every v1 `rust` row has one rfc counterpart, every v2 row has `backend`, holdouts are reserved, no ceiling keys.
  - Record the new SHA-256 of `CASES_V2.json` and the derivation (generator under `.development/23.3.1/`).
- **Bridge print rule (D11).** In `crates/hydrus-core/tests/solute_verification_harness.rs:221-231`, select the legacy profile steps by the row's `policy == "legacy_bridge"` with `analysis_times == [50,100,200]`, `sampling_interval == 50` and `time_end == 200`, not by `launch_id`. The `benchmark-example3-bridge-rust-baseline` row must print the same profile steps as before (regression check). The `-rfc-baseline` row must print `[50,100,200]`, so its inventory is `[0,50,100,200]`. Do not edit the bridge rows in `CASES_V2.json`. Do not edit `solute_benchmark_bridge.py` (the orchestrator re-pins `POLICY_HASHES` at the freeze). The `crates/` edit is limited to that one test-harness file: no change to `solute_fortran_compat/**`, `solute.rs` or solver code.
- **Tests:** `test_v2_relabel_f32_is_only_precision_change`; `test_v2_new_rows_have_backend_and_rfc_id`; `test_v2_holdout_rows_still_reserved`; `test_v2_new_rows_not_holdout`; and a harness test that the `rfc` bridge row prints `[50,100,200]` while the `rust-baseline` row is unchanged.
- **Verification:** `cargo test -p hydrus-core --test solute_verification_harness`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `python3 tools/check_solute_verification_spec.py reference/solute_verification/CASES_V2.json` exits 0; `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_check_solute_verification_spec`; `git diff --exit-code -- reference/solute_verification/CASES.json`; `git diff --check`.
- **Do not touch:** `CASES.json`, `THRESHOLDS_V2.json` (regenerated at gate C), other tools, Rust beyond the one harness file above.
- **Anchors:** `tools/check_solute_verification_spec.py:13,48,240-330,340,355,363`; `reference/solute_verification/CASES_V2.json` (`precision` at `:613,643,673`); `crates/hydrus-core/tests/solute_verification_harness.rs:202-231` (bridge print steps).

### Orchestrator: Wave 3 tool freeze

1. Confirm `tools/` and the harness file have no uncommitted changes after 23.3.1. Record the SHA-256 of every file the reference cache hashes (checker, metrics, lifecycle, convergence CLI) in `STATE_23.md`.
2. Re-pin `POLICY_HASHES` in `tools/solute_benchmark_bridge.py` for any file changed in Waves 1A and 3 that it pins. Justify in a comment. Run A4.
3. Record the frozen SHA-256 of `CASES_V2.json` (the value `THRESHOLDS_V2.json` must carry at gate C).
4. From here, no `tools/` change without a new orchestrator decision. Any change restarts this freeze.

---

## Wave 4 — Isolating studies and the Example 3 authenticated re-run (run and report; no code changes)

**Shared spec for 23.4.1–23.4.4:**
- **Must edit:** none. Commit only the report and the todo (A5).
- **Depends on:** Wave 3 freeze.
- **Run** the isolating rows from 23.2.1 (in `CASES_V2.json` after 23.3.1) from the `23-convergence` root under the worker's own worktree `.development/`, with `--cases reference/solute_verification/CASES_V2.json`. Use the standard lifecycle activation. Copy verified batches to the main `.development/<id>/` at hand-off. **Never run `holdout` rows.**
- **Classify** only after gate B (Wave 5). Until then, record batches and `--verify-only` results, and report findings as "unclassified".
- **Compute against the analytical reference** per species and time: `profile_linf`, `profile_l2`, `profile_l1`, `endpoint_linf`, `interior_linf`, inventory, boundary, reaction, balance, `temporal_l2`, as in `SOLUTE_VERIFICATION_SPEC.md:353-411`.
- **Record each metric's scale and branch** (`scale=0` → zero; scale at or below rounding → near_zero, absolute error; else relative), per `:365-370`. Report the quadrature split (`:356-362`).
- **Fit orders** only where the spec allows it. Report plateaus, sign changes, non-monotonic behavior, and f32 floors.
- **Failed rows:** record the lifecycle state and retry at most once (D10 escalation). A second failure goes in the report; the task does not chase it.
- **Report sections (`##`, ≥ 7):** Run identity (commit, CASES_V2 SHA, batch IDs, runner and binary SHA-256, commands); Lifecycle and verify status; Metrics table (row × field); Refinement and fitted orders; Floors and uncertainty; Observations against the spec's expected behavior; Inputs for threshold proposal (no thresholds proposed here).
- **Verification:** A5; every batch ID with its `--verify-only` exit code, all 0 (or the failure recorded); `grep -c '^## '` ≥ 7; `git diff --check`.

| Task | Case(s) | Report |
|---|---|---|
| 23.4.1 (4A) | `diffusion_cosine` at 97 nodes (space isolation) and dt 0.0078125 (time isolation) | `planning/master/M5V_ISOLATION_23_DIFFUSION.md` |
| 23.4.2 (4A) | `scalar_robin` at 193 nodes and dt 0.0078125 | `planning/master/M5V_ISOLATION_23_ROBIN.md` |
| 23.4.3 (4A) | `example3`: authenticated re-run of 251/501/1001 space rows through the normal classify path (D3), plus space isolation at 751 nodes | `planning/master/M5V_ISOLATION_23_EXAMPLE3.md` |
| 23.4.4 (4B) | `example3` time isolation at dt 0.04; bridge row re-run after 23.3.1 (D11) | `planning/master/M5V_ISOLATION_23_EXAMPLE3_TIME.md` |

**Notes for 23.4.3:** the 22.4.5 space rows were replayed in scratch because the normal path needed 3.5 h of reference recompute. Under D3 they are re-run once, from the frozen tools, through the normal classify path. Report old and new numbers side by side. If the new numbers differ, the new ones are the record.

**Notes for 23.4.4:** the bridge row is re-run here (its print rule landed in 23.3.1, D11). If it still fails, the failure is recorded and stays open, and gate C decides.

---

## Wave 5 — Bounds and gate B (user)

### 23.5.1 — Draft arithmetic and quadrature bounds per launch

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/M5V_BOUNDS_23.md`; scratch bounds JSON under `.development/23-bounds/` (`solute-convergence-bounds-v1`, `bounds_by_launch` per the schema in `planning/master/M5V_STUDY_22_ROBIN.md:137`); `planning/tasks/23.5.1.todo.md`.
- **Depends on:** Wave 4 (all four tasks reported).
- **Spec:**
  - Write one bounds entry per `species observable time arithmetic coordinate quadrature evidence` for each study launch. `evidence` is non-empty text giving the derivation (arithmetic: the floating-point operation count and f32 epsilon; quadrature: the mesh-weight bound).
  - Numeric bounds are nonnegative or null. Times come from the row schedule.
  - Bind to the frozen tool hashes (`producer_sha256`) and `CASES_V2` SHA from `STATE_23.md`. Produce the file only after the Wave 3 freeze.
  - Do not write the review file. Gate B is the user's.
- **Verification:** the bounds JSON parses; every study launch ID appears; `cases_sha256` and `producer_sha256` equal the frozen values (grep).
- **Anchors:** `tools/solute_convergence_evidence.py:273`; `tools/solute_convergence_analysis.py:183-195`; `planning/master/M5V_STUDY_22_ROBIN.md:137`.

### Gate B (user): review and sign

1. Present the bounds derivations to the user, by launch and observable, with the evidence lines.
2. The user signs the review file (`solute-convergence-bounds-review-v1`: `cases_sha256`, `producer_sha256`, `bounds_path`, `bounds_sha256`, non-empty `authorization`, `evidence_sha256` map). If the user declines a bound, the orchestrator records which fields become unclassifiable and that they block closure (D6).
3. No classification happens without the signed review (D2).

---

## Wave 6 — Classification, thresholds and gate C (user)

### 23.6.1 — Classify all study batches; error budgets; holdout threshold proposal

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_23.md`; `planning/tasks/23.6.1.todo.md`; scratch `.development/23.6.1/`.
- **Depends on:** Wave 5 gate B (signed review).
- **Spec:**
  - Classify every Wave 4 batch with `run_solute_convergence.py --bounds-source ... --bounds-review ...`. Every finding is `achieved` or `failed` (or `inconclusive` with the stated reason).
  - Propose per-field holdout-level ceilings using the margin rule (1.5 × observed error, 2 sig figs, applied once; ADR 0009). Report unassigned fields and why (D6).
  - Claims table: list each claimed field and whether it has an approved holdout-level proposal. Unassigned claimed fields block closure.
  - Thresholds are not tuned here. Proposals only.
- **Verification:** every batch re-verified with `--verify-only` (exit 0); the report lists every finding with its class.
- **Anchors:** `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_22.md` (the prior margin rule and unassigned list); `tools/solute_convergence_evidence.py`.

### Gate C (user): approve holdout-level thresholds

1. Present the proposals and the claims table. The user approves, edits, or refuses each field.
2. **Only after approval**, the orchestrator writes `reference/solute_verification/THRESHOLDS_V2.json` (`schema_version: solute-thresholds-v2`, `cases_sha256` equal to the frozen `CASES_V2.json` SHA, `holdout_run_permitted: true`, the approval date and basis). This is the only commit that lifts the holdout guard.
3. If the user does not approve, the holdouts do not run and M5-V stays partial with Wave 7 skipped.

---

## Wave 7 — Holdouts (after gate C)

### 23.7.1 — Holdout runs against the approved thresholds

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/M5V_HOLDOUT_23.md`; `planning/tasks/23.7.1.todo.md`; scratch `.development/23-holdout/`.
- **Depends on:** Wave 6 gate C; the guard from 23.1.1 must accept the committed file (check before running any row).
- **Spec:**
  - Run the three v2 holdout rows: `diffusion_cosine` 97/0.0078125, `scalar_robin` 193/0.0078125, `example3` 751/0.04 (`backend: fortran-compatible`), through `run_solute_convergence.py --holdout --output-root .development/23-holdout/<x>`.
  - List every batch ID with its `--verify-only` exit code.
  - Compare against the approved thresholds. Do not change a threshold, retry a holdout for a better result, or use holdout output to change any study proposal. A failure is recorded as a failure.
- **Verification:** all batches verify 0; the report lists each claimed field's pass/fail; `git diff --check`.

---

## Wave 8 — Closure record

### 23.8.1 — M5-V closure record (closed or partial) and known limits

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`, `planning/master/VALIDATION_STRATEGY.md`, `planning/master/ROADMAP_AND_RISKS.md` (M5-V row and the "No scientific solute baseline" risk), `planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md` (one status line pointing to `_23`), `docs/decisions/README.md`.
- **Can create:** `docs/decisions/0020-*.md` (closure decision) if M5-V closes; otherwise update ADR 0019's status line only.
- **Depends on:** 23.7.1.
- **Spec:**
  - **If every claimed field passed its holdout:** M5-V closed on the default backend. M6 planning may start. Record each field's pass with its batch ID. ADR 0020 records the closure decision in the house template.
  - **If any claimed field failed or was unassigned:** M5-V stays partial/open, and the record lists the unmet fields. The roadmap does not unblock M6. ADR 0019 status stays "Accepted; M5-V open".
  - **Known limits, in all cases:** the classifier bounds source (signed by the user at gate B); the f32 precision (the label is now f32, per D4); the O13 `--verify-only` limit (D8); the dropped Fortran-only `sensitivity` row; the FV/BE study not re-run; one compiler/platform; the diffusion end-row loss status (SD-005 or "Not defects", per 23.1.3).
  - No "met" or "PASS" statements without the batch IDs behind them.
- **Verification:** `git diff --check`; every cited batch ID and file path exists (grep); `grep -c '^## '` on each updated doc is unchanged or documented.

---

## Escalation notes (orchestrator)

- A worker that needs a CASES_V2 row change outside 23.3.1, or a tool edit after the freeze, stops and reports (rule 4). The orchestrator decides with the user before any edit.
- If a bounds derivation is unsound (for example, it relies on a floor), the worker reports it. The field is recorded as unclassifiable and blocks closure. The bound is not loosened.
- If gate B or gate C is not given, the orchestrator stops and does not start the next wave.

---

## Continuation after the blocker reconciliation (2026-11-07)

User decisions: **D12** add the five control rows; **D13** drop the Example 3 `dt .025`
re-run from the critical path. Basis: `planning/master/M5V_BLOCKERS_23.md`.

### Task 23R.1 — Add the five control rows and register the final campaign root

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `reference/solute_verification/CASES_V2.json`,
  `tools/check_solute_verification_spec.py`,
  `tools/test_check_solute_verification_spec.py`,
  `tools/test_timeout_overrides.py`.
- **Can edit:** `tools/run_solute_verification_reference.py`,
  `tools/solute_convergence_evidence.py`, `tools/solute_reference_cache.py`,
  `crates/hydrus-core/tests/solute_verification_harness.rs`,
  `tools/test_solute_convergence_cli.py` (only to register `23-final` as a campaign root,
  following the `23-controls-8` pattern).
- **Can create:** `planning/tasks/23R.1.todo.md`; scratch `.development/23R.1/`.
- **Depends on:** D12 and D13 (given 2026-11-07).
- **Spec:**
  - Add exactly these control rows, all `backend: "fortran-compatible"`, `solver: "rust"`,
    `precision: "f32"`, `role: "control"`, `phase: "convergence"`, `status:
    "approved_pending_adapter_and_evaluator_gates"`, `time_controls` policy `fixed`,
    `timeout_seconds` 60 unless noted:
    | ID | Axis | Nodes | dt | Analysis | Sampling | Max steps |
    |---|---|---|---|---|---|---|
    | `convergence-diffusion_cosine-space_control-129-dt2e-8-rfc-baseline` | space | 129 | `2^-8` | 0.25, 0.5, 1 | 0.0625 | 256 |
    | `convergence-diffusion_cosine-time_control-193-dt2e-8-rfc-baseline` | time | 193 | `2^-8` | 0.25, 0.5, 1 | 0.0625 | 256 |
    | `convergence-scalar_robin-space_control-257-dt2e-8-rfc-baseline` | space | 257 | `2^-8` | 0.25, 0.5, 1 | 0.0625 | 256 |
    | `convergence-scalar_robin-time_control-257-dt2e-8-rfc-baseline` | time | 257 | `2^-8` | 0.25, 0.5, 1 | 0.0625 | 256 |
    | `convergence-example3-time_control-1001-dt0.025-rfc-baseline` | time | 1001 | 0.025 | 50, 100, 200 | 1 | 8000 |
  - Example 3 time control: `timeout_seconds` 180, added to
    `APPROVED_TIMEOUT_OVERRIDES` with a recorded reason. It shares the 1001/0.025
    configuration with the space study row and needs the same approved override.
  - Use the existing `estimated_work_units` and `estimated_output_bytes` formulas.
  - Do not edit `CASES.json`, `THRESHOLDS_V2.json`, any holdout row, or any bounds file.
  - Register `23-final` as a convergence campaign root using the same pattern as
    `23-controls-8` (task maps, allowlists, harness, and the test that pins the set).
  - Add tests: all five rows exist with the given role/axis/nodes/dt; none is holdout;
    no ceiling keys; the new timeout override is approved; the v2 checker still passes;
    v1 is unchanged.
- **Verification:** `python3 tools/check_solute_verification_spec.py
  reference/solute_verification/CASES_V2.json` exits 0;
  `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest
  tools.test_check_solute_verification_spec tools.test_timeout_overrides` passes;
  `cargo test -p hydrus-core --test solute_verification_harness` if the harness changes;
  `git diff --check`.
- **Do not:** run solvers, references, or any `holdout` row.

### Task 23R.2 — Regenerate and merge the bounds; prepare Gate B

- **Must edit:** none in shipped code.
- **Can create:** a combined bounds file, a review draft, and scripts under
  `.development/23-bounds/`; `planning/master/M5V_BOUNDS_23.md` update;
  `planning/tasks/23R.2.todo.md`; pre-run evidence under `.development/23-controls-8/`.
- **Depends on:** 23R.1.
- **Spec:**
  - Copy the signed control bounds out of `/tmp` to a durable `.development/23-bounds/` path.
    That file covers the six *old* controls, not the five new rows.
  - Pre-run the four inexpensive new controls bounds-free in the registered
    `.development/23-controls-8/` root: diffusion space 129 at `2^-8`, diffusion time 193
    at `2^-8`, Robin space 257 at `2^-8`, Robin time 257 at `2^-8`. Verify each batch.
    These runs exist only to derive the arithmetic control bounds; the final campaign
    re-runs them with bounds.
  - Derive the Example 3 time control (1001 at `0.025`) bounds from the already
    authenticated `23-controls-8` example3 space run at the same 1001/0.025 configuration
    (batch `batch-980de4772e034c2baac25f34a9465818`). Do not re-run that configuration.
  - Derive arithmetic bounds for the four pre-run controls with the same formula and
    scale rule as `build_bounds.py`. Quadrature stays null for control rows, as signed
    for the old controls; keys that need it remain unclassifiable.
  - Regenerate the 18 study-bound `producer_sha256` values with the current tools; bound
    values stay unchanged. Update `cases_sha256` to the new CASES_V2 SHA.
  - Merge the study and control entries into one `solute-convergence-bounds-v1` source
    covering all 18 study rows and the five new control rows, plus the existing Example 3
    space control.
  - Write a draft review bound to the new CASES_V2 SHA, combined bounds SHA, and current
    producer hashes. Do not sign it; Gate B is the user's.
  - Verify every launch appears, the cases SHA matches, and the producer hashes match HEAD.
- **Gate B:** user reviews and signs the combined review. No final campaign runs before it.

### Task 23R.3 — Final six-batch campaign

- **Must edit:** none.
- **Can create:** reports under `.development/23R.3/`; `planning/master/M5V_FINAL_23.md`;
  `planning/tasks/23R.3.todo.md`.
- **Depends on:** 23R.2 and signed Gate B.
- **Spec:**
  - One batch per fixed-axis group in the `23-final` root:
    diffusion space, diffusion time, scalar_robin space, scalar_robin time,
    example3 space at `dt .04`, example3 time.
  - Each batch includes its control row. Example 3 space at `.04` uses
    `convergence-example3-space_control-rfc-baseline`; the other groups use the new rows.
  - Execute with `--bounds-source` and `--bounds-review`; verify every batch with
    `--verify-only`.
  - Never run a `holdout` row. Do not edit tools or cases after this point.
- **Verification:** every batch execute and verify exit 0; batch IDs and hashes recorded.

### Task 23R.4 — Classification, error budgets, thresholds; Gate C

- **Must edit:** none.
- **Can create:** `planning/master/SOLUTE_CONVERGENCE_EVIDENCE_23.md` update;
  `planning/tasks/23R.4.todo.md`; scratch `.development/23R.4/`.
- **Depends on:** 23R.3.
- **Spec:** read the verified classifications; report achieved/failed/inconclusive with
  reasons; propose per-field holdout ceilings with the ADR 0009 margin rule; list
  unassigned fields and why. Do not tune on holdout data.
- **Gate C:** user approves, edits, or refuses each field. Only then regenerate
  `THRESHOLDS_V2.json` with the new CASES SHA and `holdout_run_permitted: true`.

### Task 23R.5 — Holdouts and closure

- **Must edit:** the closure docs named in 23.8.1.
- **Depends on:** Gate C.
- **Spec:** run the three reserved holdout rows once, compare with the approved
  thresholds, and record closure or partial status. No threshold changes after the run.

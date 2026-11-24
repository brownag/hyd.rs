# Orchestration 28 — G22.2 water-flux and free-drainage audit (with vapor)

Status: **Approved by user 2026-11-13. Not yet executed.** Reviewed (spec and standards axes); review fixes applied.
Repo: `/home/andrew/AI/hyd.rs`, branch `main`. Baseline at planning: last full gate green (Rust 620 passed / 0 failed / 4 ignored; Python 479 OK).
Resume: [STATE_28.md](STATE_28.md). Worker: [PROMPT_TEMPLATE_28.md](PROMPT_TEMPLATE_28.md).
Goal source: [GOALS.md](GOALS.md) Circled goal **G22.2**. Coverage: [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md) row 22.
Audit template: [M6_VAPOR_SINK_ROOT_AUDIT.md](master/M6_VAPOR_SINK_ROOT_AUDIT.md) (orchestration 27).

## Goal & Scope

**Goal.** Decide, with source evidence, which water boundary forms with vapor on can be supported, and which stay rejected with a named trigger. This orchestration is audit-only. Implementation of any accepted form becomes a later orchestration (29), chosen at a user gate.

**Verbatim goal (GOALS.md, G22.2):** "Water-flux and free-drainage boundary pairs (and supported mixed pairs) are implemented with vapor, including vapor boundary/storage accounting and balance evidence."

**Verbatim exit (user, 2026-11-13):** "each water-flux/free-drainage form with vapor on is accepted or rejected with a named trigger; at least one accepted form has a provenance-backed fixture and balance check; recorded in planning/master/M6_VAPOR_COVERAGE.md."

**In scope**
- Source and coupling audit of the lower-boundary water forms with vapor on: free drainage (`FreeD`), prescribed flux (`KodBot` flux codes, `qGWLF`), seepage face (`SeepF`), water table (`qDrain`/`zBotDr`/`BaseGW`), and the upper prescribed flux (`KodTop` flux codes).
- Supported mixed pairs (top flux with each lower form; free drainage with each upper form), audited as pairs, not only as single forms (28.1.2 step 6).
- The G7 free-drainage end-row assessment, folded into wave 1 (user decision 2026-11-13) and reconciled with the free-drainage audit at GATE W1.
- Coverage-matrix and GOALS update for the decided forms (wave 2, docs only).

**Out of scope**
- Any implementation, fixture generation, producer change or Rust code change for a water form. Deferred to orchestration 29, chosen at GATE W1. The exit criterion "at least one accepted form has a fixture and balance check" is therefore **not** claimed by this orchestration; it is carried to orchestration 29.
- Atmospheric codes (`KodTop = -4` atmospheric remap at `INPUT.FOR:106-108`, and `AtmBC`): classified as G22.3 and not decided here.
- Energy balance (G20), Campbell (G22.6), enhancement off (G22.5), temperature-dependent liquid water (G23), sinusoidal top (already rejected).
- Sink and root uptake (rejected in orchestration 27; not reopened).
- Pinned Fortran source, existing fixtures, `criteria.json` (v4), existing evidence, the G22.1 and G22.4 decisions, ADR 0021 status.

**Verbatim constraints (user, 2026-11-13)**
- "Audit first, gate, then implement."
- "Track" planning artifacts; planning-only commits `docs(planning):`, separate from code.
- "Sonnet, max two parallel" for workers, with the orchestrator reviewing each micro-plan.
- Same serial gate as orchestration 27 at each wave end.
- Carried: ADR 0021 (Proposed) is complied with, not superseded; no Accepted ADR governs heat.

## Decision log (dated 2026-11-13)

| # | Decision | Source |
|---|---|---|
| D1 | Orchestration 28 is audit-only. Implementation becomes orchestration 29, chosen at GATE W1. | User: "Audit first, gate, then implement" |
| D2 | Planning artifacts are tracked. The three plan files are committed (`docs(planning):`) before wave 1; each wave's audit docs and STATE updates are committed separately at wave end. | User: Track |
| D3 | Workers are Sonnet, at most two in parallel. Escalation ladder below. | User: Sonnet, max two |
| D4 | Serial gate at each wave end, as orchestration 27. Plus `git diff --name-only` must list only the permitted paths for the wave. | User: same gate as 27 |
| D5 | G7 is folded into wave 1 as a read-only task. Its verdict is reconciled with the free-drainage audit at GATE W1 (both read `WATFLOW.FOR:213`; neither edits it). | User: fold into wave 1; review fix |
| D6 | ADR 0021 is complied with. Each audit states its compliance with ADR 0021 in its acceptance. Its status is Proposed, so no supersession is needed. | Recon: `docs/decisions/0021-*.md` "Proposed"; review fix |
| D7 | No new criteria, fixtures or tolerances in this orchestration. | Plan scope |
| D8 | Workers do not commit (deliberate departure from the skill's commit rule; audit-only docs). The orchestrator commits. | Plan choice; review fix |
| D9 | Workers write their report to `planning/tasks/{{id}}.report.md`. The orchestrator appends it to `STATE_28.md`. Workers never write `STATE_28.md`. | Review fix (shared-state rule) |

## Escalation ladder (wave-orchestration)

1. Micro-plan off-spec twice: the orchestrator writes the micro-TODO itself and gives it to the worker as fixed input.
2. Acceptance fails after one retry (retry = failure output fed back as `FAILED VERIFICATION: ...`): stop the worker and re-run the task on a mid-tier model (Opus). No third small-model attempt.
3. Worker produces no completion report, or a malformed one: send feedback describing what is missing; allow one retry; then re-run on Opus.
4. Worker needs an out-of-scope edit: a decomposition bug. Re-scope the spec in this plan; do not widen the worker's permissions.

## Recon facts (anchors for the workers)

- **Fortran lower boundary.** `src_reference/INPUT.FOR:56-61` reads `BotInF, qGWLF, FreeD, SeepF, KodBot, qDrain[, hSeep]`. `INPUT.FOR:63-64` checks `KodBot.eq.-1` with `BotInF` false. `WATFLOW.FOR:6, 19-20, 79-87` pass `FreeD`, `SeepF`, `qGWLF`, `qDrain`, `KodBot`. `WATFLOW.FOR:213` (`if(FreeD) rBot=-ConB*Grav*fRE`) and `:218` (`if(qDrain) rBot=FqDrain(...)`) set the bottom flux. `WATFLOW.FOR:373` starts the `SeepF` branch (the subroutine header comment is at `:360`).
- **Fortran top flux.** `INPUT.FOR:51-52` reads `TopInF, WLayer, KodTop`. `INPUT.FOR:104-110` applies the `isign(3, ...)` remap and the atmospheric `-4` remap (atmospheric: G22.3, out of scope here).
- **Rust lower boundary.** `crates/hydrus-core/src/lower_boundary.rs:11` `FixedHead`, `:17` `PrescribedFlux`, `:19` `FreeDrainage`, `:27` `SeepageFace` and `:28` `WaterTable` (both rejected as `Unsupported`, `:108-116`), `:127-132` mesh and flux checks, `:161-171` free-drainage properties.
- **Vapor rejection.** `crates/hydrus-core/src/heat/vapor/mod.rs:213-217`: the lower match accepts only `LowerBoundaryMode::FixedHead` with vapor on; every other mode returns `UnsupportedLowerBoundary(mode)` at `:217`. `:223-225` does the same for the upper boundary (only `UpperBoundary::Head`).
- **Coverage matrix.** `planning/master/M6_VAPOR_COVERAGE.md:21` (fixed water heads at both ends: accepted), `:22` (water flux / free drainage / supported mixed pairs: rejected, deferred to G22), `:23` (atmospheric switching: rejected, coordinate with G20). G22 completion rule at `:31`.
- **Defect register.** `docs/FORTRAN_SOURCE_DEFECTS.md` (read only in this orchestration; entries run through SD-006, next free ID SD-007).
- **Hazards.** H1: do not relax the vapor rejections. H2: heat transport has its own sink and boundary terms; do not assume the vapor path can reuse them. H3: this orchestration writes no fixtures. H4: water-only and vapor paths are separate code; name the file and line for each claim.

## Wave table

| Wave | Task | Title | Depends on | Gate before |
|---|---|---|---|---|
| 0 | — | Commit the three plan files (`docs(planning):`) after user approval | — | Wave 1 |
| 1 | 28.1.1 | Free drainage with vapor on: source and coupling audit | — | — |
| 1 | 28.1.2 | Prescribed water flux (upper and lower) and mixed pairs with vapor on: audit | — | — |
| 1 | 28.1.3 | Seepage face and water table with vapor on: audit | — | — |
| 1 | 28.1.4 | G7 free-drainage end-row assessment | — | — |
| — | **GATE W1 (user)** | Disposition per form and pair; G7 verdict reconciled; choose which go to orchestration 29 | 28.1.1–28.1.4 | Wave 2 |
| 2 | 28.2.1 | Coverage matrix, GOALS and G22.2 handoff (docs) | 28.1.1–28.1.4, GATE W1 | — |

Waves run at most two workers in parallel. Wave 1 has four read-only tasks writing four different docs: run two at a time (28.1.1 and 28.1.2, then 28.1.3 and 28.1.4). Workers write distinct untracked files and need no worktree. The serial gate runs at each wave end.

## Gates

- **Pre-wave-1 baseline.** Repeat the five suites at the planning HEAD. Expected: Rust 620 passed / 0 failed / 4 ignored; Python 479 OK. If they differ, stop and report.
- **Micro-plan review (orchestrator).** Each micro-TODO is checked for: in-scope files only, acceptance mapped, at most 12 steps, no overlap with other tasks, claims spot-checked against source.
- **Wave end (serial, orchestrator):** `cargo build --workspace --locked`; `cargo test --workspace --no-fail-fast`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `python3 -m unittest discover -s tools -p 'test_*.py'`; then `git diff --name-only` and `git status --short` must show only the wave's permitted docs and planning files.
- **GATE W1 (user).** After wave 1 the orchestrator presents a summary table with one row per form and per pair: source evidence (lines), vapor term affected, disposition, named trigger, and the balance check an accepted form would need (named, not run). The G7 verdict is included as an input. The user chooses per form or pair: (i) reject with trigger, (ii) accept for orchestration 29 (implementation, fixture and balance check), (iii) other. No implementation happens in this orchestration in any case.

---

## Task specs

### 28.1.1 — Free drainage with vapor on: source and coupling audit (read-only)

- **Repo:** hyd.rs. **Depends on:** none. Reconciled with 28.1.4 at GATE W1 (D5).
- **Files in scope:**
  - *Must edit:* none.
  - *Can create:* `planning/master/M6_VAPOR_FREE_DRAINAGE_AUDIT.md`; `planning/tasks/28.1.1.todo.md`; `planning/tasks/28.1.1.report.md`.
  - *Read only:* `src_reference/WATFLOW.FOR` (anchors below), `src_reference/INPUT.FOR` (lines 40–115), `src_reference/HYDRUS.FOR` (grep only), `crates/hydrus-core/src/lower_boundary.rs`, `crates/hydrus-core/src/heat/vapor/mod.rs` (lines 170–230), `crates/hydrus-core/src/water_solver.rs` (grep `FreeDrainage`), `planning/master/M6_VAPOR_COVERAGE.md`, `docs/decisions/0021-*.md`.
  - Scratch: `.development/28.1.1/`.
- **Spec.** Establish what free drainage does in the Fortran water step, what the vapor path would need to carry it, and what the Rust vapor path does with it today.
  1. Read `WATFLOW.FOR:213` and the surrounding bottom-flux assembly; cite where `rBot` enters the water matrix.
  2. Read the `FreeD` input path in `INPUT.FOR:56-61`.
  3. State whether any vapor term (`ConVapor`, `VaporContent`, vapor storage) enters the free-drainage bottom flux. Cite lines.
  4. Map the Rust rejection (`vapor/mod.rs:217`) and the water-only implementation (`lower_boundary.rs:161-171` and the water step that consumes it). State what a vapor-enabled free drainage would touch.
  5. Give a disposition: candidate for a bounded case, or reject with a named trigger. For an accepted candidate, name the balance check it would need.
  6. State compliance with ADR 0021 for this form.
  7. Write the audit as a table: form, Fortran lines, Rust lines, vapor term affected (y/n), disposition, trigger, balance check needed.
- **Constraints.** No code change, no fixture, no executable run. Ambiguity goes into Questions (max 5).
- **Anchors.** `WATFLOW.FOR:6, 19-20, 79-87, 213, 218`; `INPUT.FOR:56-61, 63-64`; `vapor/mod.rs:213-217`; `lower_boundary.rs:19, 127-132, 161-171`.
- **Reference.** `planning/master/M6_VAPOR_SINK_ROOT_AUDIT.md` (table format, orchestration 27).
- **Acceptance.**
  - `planning/master/M6_VAPOR_FREE_DRAINAGE_AUDIT.md` exists; `grep -c "WATFLOW.FOR\|INPUT.FOR" planning/master/M6_VAPOR_FREE_DRAINAGE_AUDIT.md` ≥ 4; `grep -c "ADR 0021" planning/master/M6_VAPOR_FREE_DRAINAGE_AUDIT.md` ≥ 1.
  - `git status --short -- planning/master/M6_VAPOR_FREE_DRAINAGE_AUDIT.md planning/tasks/28.1.1.*` shows only these new files; `git diff --name-only` is empty.

### 28.1.2 — Prescribed water flux (upper and lower) and mixed pairs with vapor on: audit (read-only)

- **Repo:** hyd.rs. **Depends on:** none.
- **Files in scope:**
  - *Must edit:* none.
  - *Can create:* `planning/master/M6_VAPOR_WATER_FLUX_AUDIT.md`; `planning/tasks/28.1.2.todo.md`; `planning/tasks/28.1.2.report.md`.
  - *Read only:* `src_reference/INPUT.FOR` (lines 40–115; grep `KodTop\|KodBot\|rTop\|rBot\|qGWLF`), `src_reference/WATFLOW.FOR` (top and bottom flux assembly, grep), `crates/hydrus-core/src/lower_boundary.rs` (lines 15–30, 100–135), `crates/hydrus-core/src/heat/vapor/mod.rs` (lines 213–226), `crates/hydrus-core/src/water_solver.rs` (grep `PrescribedFlux`, `top_flux`, `rTop`), `docs/decisions/0021-*.md`.
  - Scratch: `.development/28.1.2/`.
- **Spec.** Establish the Fortran prescribed-flux forms (upper `KodTop` flux codes, lower `KodBot`/`qGWLF` flux), what each does to the vapor-coupled water step, and what the Rust vapor path does with them, including pairs.
  1. Enumerate the flux codes for the top and bottom from `INPUT.FOR` and the remaps at `:104-110`. Cite each. Mark atmospheric codes as G22.3, not decided here.
  2. For each non-atmospheric code, state whether the Fortran water step with vapor on takes the flux as a boundary term, and cite the assembly lines.
  3. Map the Rust rejections (`vapor/mod.rs:217` lower, `:225` upper) and the water-only prescribed-flux path (`lower_boundary.rs:17, 105, 132`).
  4. Give a disposition per code: candidate for a bounded case, or reject with a named trigger. For a candidate, name the balance check it would need.
  5. State compliance with ADR 0021 for each form.
  6. **Pairs.** For each supported-candidate pairing (top flux with each lower form; free drainage with each upper form), state whether the pair needs new coupling beyond its two single forms, with citations. Give each pair a disposition.
  7. Write the audit as the same table format as 28.1.1, with a separate pairs table.
- **Constraints.** No code change, no fixture, no executable run. Ambiguity goes into Questions (max 5).
- **Anchors.** `INPUT.FOR:51-52, 104-110, 56-61`; `vapor/mod.rs:213-225`; `lower_boundary.rs:17, 105, 132`.
- **Reference.** `planning/master/M6_VAPOR_SINK_ROOT_AUDIT.md`.
- **Acceptance.**
  - `planning/master/M6_VAPOR_WATER_FLUX_AUDIT.md` exists; `grep -c "^|" planning/master/M6_VAPOR_WATER_FLUX_AUDIT.md` ≥ 6 (single-form rows plus pairs rows); `grep -c "INPUT.FOR\|WATFLOW.FOR" planning/master/M6_VAPOR_WATER_FLUX_AUDIT.md` ≥ 4; `grep -c "ADR 0021"` ≥ 1.
  - `git status --short -- planning/master/M6_VAPOR_WATER_FLUX_AUDIT.md planning/tasks/28.1.2.*` shows only these new files; `git diff --name-only` is empty.

### 28.1.3 — Seepage face and water table with vapor on: audit (read-only)

- **Repo:** hyd.rs. **Depends on:** none.
- **Files in scope:**
  - *Must edit:* none.
  - *Can create:* `planning/master/M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md`; `planning/tasks/28.1.3.todo.md`; `planning/tasks/28.1.3.report.md`.
  - *Read only:* `src_reference/WATFLOW.FOR:360-400` (`SeepF` branch from `:373`), `src_reference/INPUT.FOR:56-62` (`SeepF`, `qDrain`, `hSeep`), `src_reference/WATFLOW.FOR:218` (`FqDrain`), `crates/hydrus-core/src/lower_boundary.rs:27-28, 108-116`, `crates/hydrus-core/src/heat/vapor/mod.rs:213-217`, `docs/decisions/0021-*.md`.
  - Scratch: `.development/28.1.3/`.
- **Spec.** Establish what seepage face and the water-table (`qDrain`) boundary do in the Fortran water step, whether vapor terms enter them, and what would be needed to support them with vapor on.
  1. Read `WATFLOW.FOR:360-400` (`SeepF` branch from `:373`) and cite the lines that set the bottom head or flux.
  2. Read `FqDrain` and the `qDrain` branch (`WATFLOW.FOR:218`) and its input (`INPUT.FOR:56-62`: `zBotDr`, `BaseGW`, `rSpacing`).
  3. State whether vapor storage or vapor conductance appears in the seepage or drain flux. Cite lines.
  4. Map the Rust rejections (`lower_boundary.rs:108-116` for `SeepageFace` and `WaterTable`; `vapor/mod.rs:217`).
  5. Give a disposition per form: candidate for a bounded case, or reject with a named trigger. Seepage face is a variable-head boundary: state the trigger for a nonlinear head condition. For a candidate, name the balance check it would need.
  6. State compliance with ADR 0021 for each form.
  7. Write the audit as the same table format as 28.1.1.
- **Constraints.** No code change, no fixture, no executable run. Ambiguity goes into Questions (max 5).
- **Anchors.** `WATFLOW.FOR:218, 360-400`; `INPUT.FOR:56-62`; `lower_boundary.rs:27-28, 108-116`; `vapor/mod.rs:213-217`.
- **Reference.** `planning/master/M6_VAPOR_SINK_ROOT_AUDIT.md`.
- **Acceptance.**
  - `planning/master/M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md` exists with a row for seepage face and a row for water table: `grep -c "Seepage\|Water table" planning/master/M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md` ≥ 2; `grep -c "WATFLOW.FOR\|INPUT.FOR"` ≥ 4; `grep -c "ADR 0021"` ≥ 1.
  - `git status --short -- planning/master/M6_VAPOR_SEEPAGE_WATERTABLE_AUDIT.md planning/tasks/28.1.3.*` shows only these new files; `git diff --name-only` is empty.

### 28.1.4 — G7 free-drainage end-row assessment (read-only)

- **Repo:** hyd.rs. **Depends on:** none. Reconciled with 28.1.1 at GATE W1 (D5).
- **Files in scope:**
  - *Must edit:* none.
  - *Can create:* `planning/master/G7_FREE_DRAINAGE_END_ROW_ASSESSMENT.md`; `planning/tasks/28.1.4.todo.md`; `planning/tasks/28.1.4.report.md`.
  - *Read only:* `docs/FORTRAN_SOURCE_DEFECTS.md` (header and entry format; do not edit), `src_reference/WATFLOW.FOR` (free-drainage end row, around `:213`), `crates/hydrus-core/src/lower_boundary.rs:161-171`, `planning/GOALS.md` (G7 entry).
  - Scratch: `.development/28.1.4/`.
- **Spec.** Assess the free-drainage end-row behaviour against the source-defect register rules and record it as a defect or a modeling choice. This is a read-only assessment; it does not change the register.
  1. Read the register's rules (header and entry format).
  2. Read the free-drainage end-row code path in the Fortran and in Rust. Cite both.
  3. Decide: source defect (give the reproduction reasoning from the pinned source), modeling choice (document it), or not established. Give the evidence.
  4. If a defect is claimed, state the register ID you would assign (SD-007 is the next free number) and the tagged pinning test it would need. Do not add the entry.
- **Constraints.** No code change, no fixture, no executable run, no register edit. Ambiguity goes into Questions (max 5).
- **Anchors.** `WATFLOW.FOR:213`; `lower_boundary.rs:161-171`; `docs/FORTRAN_SOURCE_DEFECTS.md` header.
- **Reference.** Memory: "Fortran source defect register" (log reproduced Fortran bugs as SD-NNN with tagged pinning tests).
- **Acceptance.**
  - `planning/master/G7_FREE_DRAINAGE_END_ROW_ASSESSMENT.md` exists with exactly one verdict line: `grep -Ec '^Verdict: (defect|modeling choice|not established)$' planning/master/G7_FREE_DRAINAGE_END_ROW_ASSESSMENT.md` equals 1.
  - `git status --short -- planning/master/G7_FREE_DRAINAGE_END_ROW_ASSESSMENT.md planning/tasks/28.1.4.*` shows only these new files; `git diff --name-only` is empty.

### 28.2.1 — Coverage matrix, GOALS and G22.2 handoff (docs only; after GATE W1)

- **Repo:** hyd.rs. **Depends on:** 28.1.1–28.1.4, GATE W1.
- **Files in scope:**
  - *Must edit:* `planning/master/M6_VAPOR_COVERAGE.md` (row 22 and any row touched by the decided forms; row 21 is not changed), `planning/GOALS.md` (G22.2 entry).
  - *Can create:* `planning/master/M6_VAPOR_G22_2_ACCEPTANCE.md`; `planning/tasks/28.2.1.todo.md`; `planning/tasks/28.2.1.report.md`.
  - *Read only:* the four audits and G7 assessment; `planning/STATE_28.md`; `planning/master/M6_VAPOR_COVERAGE.md` (grep `G22.4` for the style of the closed G22.4 row).
  - Scratch: `.development/28.2.1/`.
- **Spec.**
  1. For each form and pair decided at GATE W1, set its row-22 Status to `rejected with trigger`, `accepted for orchestration 29`, or `not decided` (only if the user left it open), with the audit link and the trigger text copied verbatim.
  2. Write the acceptance doc: per-form evidence, the gate disposition, what is not claimed, and the ADR 0021 compliance statement.
  3. In `GOALS.md`, keep G22.2 Circled if any form is still `accepted for orchestration 29` or `not decided`. Move it to Completed only if the user decides, at GATE W1, that the exit criterion is met or that the goal is closed with exceptions. If every form is rejected, record that as an explicit user decision on the exit criterion, in the style of G22.4, and do not close silently.
- **Constraints.** Docs only. No code change. Do not change ADR 0021 status, the G22.1 or G22.4 decisions, or criteria.
- **Anchors.** `planning/master/M6_VAPOR_COVERAGE.md:21-23, 31`; `planning/GOALS.md` (grep `G22.2`).
- **Reference.** `planning/master/M6_VAPOR_G22_4_ACCEPTANCE.md` (acceptance-doc format, orchestration 27).
- **Acceptance.**
  - `git diff --check` clean.
  - `git status --short -- planning/master/M6_VAPOR_COVERAGE.md planning/GOALS.md planning/master/M6_VAPOR_G22_2_ACCEPTANCE.md` shows only these paths; `git diff --name-only` is empty apart from the two must-edit files.
  - `grep -n "^| 22 \|Water flux" planning/master/M6_VAPOR_COVERAGE.md` shows a non-empty Status for the water-flux row.

---

## Planning review

Two-axis review (spec and standards, parallel reviewers, read-only). Findings applied in this revision:
- *Spec:* mixed pairs added (28.1.2 step 6); D2, D4, D5, D6 made concrete; the exit criterion's deferral to orchestration 29 stated in Out of scope; the GATE W1 summary table defined; the all-rejected case made an explicit user decision (28.2.1 step 3).
- *Standards:* the shared-state conflict fixed (D9: workers write `tasks/{{id}}.report.md`; the orchestrator appends); parallel `git status` checks replaced by path-scoped checks; row-count and verdict checks made commands; 28.2.1 given anchors and a reference; the escalation ladder added in full; the template link made absolute; worker commits recorded as D8; `tasks/*.todo.md` listed in each Files in scope; line references corrected (`:213` range, `WATFLOW.FOR:373`).
- *Open judgment:* none beyond the user gates.

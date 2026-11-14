# Orchestration 30 — G22.2 follow-ups: closed-pair trajectory, free drainage, dry regime, balance, criteria

Status: **Drafted 2026-11-14. Awaiting user approval (Phase 6).** No task has started.
Repo: `/home/andrew/AI/hyd.rs`, branch `main`. Baseline: orchestration 29 closed at `18a8397` (Rust 633 passed / 0 failed / 4 ignored; Python 479 OK; fmt and clippy clean). Re-verify at wave 0.
Goal source: [GOALS.md](GOALS.md) G22.2 (Circled), items G22.2-C1 to G22.2-C6. Record: [M6_VAPOR_G22_2_ACCEPTANCE.md](master/M6_VAPOR_G22_2_ACCEPTANCE.md). Coverage: [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md) row 22. Predecessor: [EXECUTION_PLAN_29.md](EXECUTION_PLAN_29.md) (closed; closing board in [STATE_29.md](STATE_29.md)).

## Goal & scope

**Goal.** Close the open G22.2 items for the closed flux/flux pair and extend vapor-on support to free drainage: the Rust path reproduces the Fortran trajectories, the dry-regime closed pair is either converged in the reference or recorded as impossible, the full-trajectory balance is checked, the criteria entries are approved and written, and the free-drainage lower boundary with vapor is assessed, implemented and evidenced.

**Verbatim constraints (user, 2026-11-14):**
- "Assess and implement" for free drainage (answer to "Should it only assess free drainage and record the verdict, or also implement the free-drainage lower boundary with vapor?").
- "Confirm rejection (Recommended)" for qGWLF and TopInF/BotInF (±3): keep rejected with their triggers.
- "Track (Recommended)" for planning artifacts.
- "Isothermal dry closed deck (Recommended)" as the dry-regime strategy, with the gate before any run.
- "Yes, in scope now (Recommended)" for the Rust trajectory producer, no reference run.
- From orchestration 29 (carried): "criteria.json must not be edited without explicit user approval (D5)"; "Do not run new reference decks before the gate."

**In scope**
- G22.2-C1: Rust closed-pair flux producer; comparison against the four committed wet-start fixtures.
- G22.2-C2: confirmation of qGWLF and TopInF/BotInF rejections (documentation only).
- G22.2-C6 + free drainage: assessment of the `ConB`/`rBot` coupling; implementation of the free-drainage lower boundary with vapor under the chosen option; fixture pair (vapor on and off) and comparison.
- G22.2-C3: dry-regime closed pair with vapor on, isothermal strategy (converged reference deck, or a recorded impossibility).
- G22.2-C4: Fortran `BALANCE.OUT` in runner output; full-trajectory balance for the closed pair (and the free-drainage pair).
- G22.2-C5: criteria D5 proposal, user approval, then write.

**Out of scope**
- Single lower prescribed flux with fixed-head upper; flux/`FixedHead` beyond the closed pair.
- Flux/free-drainage pairs beyond what the free-drainage lower row needs for its own fixture (the upper form stays the closed-pair upper flux only unless the wave 4 design says otherwise).
- `qGWLF`, `TopInF`/`BotInF` (±3), seepage face, water table, atmospheric (`KodTop=-4`, G22.3).
- Geometric face averaging fixture (recon item 6): not covered here; a separate item.
- Campbell, energy balance (G20), G23.
- Deletion of the about 1.5 GB of `.development/29.4.1/runs/B` output (G22.2-C7 cleanup): needs its own explicit approval.
- `src_reference/`, existing fixtures, pinned Fortran source, ADR 0021 status.

## Decision log

| # | Decision | Source |
|---|---|---|
| D1 | Free drainage is implemented in this orchestration (assessment first, then gated implementation). This overrides the earlier out-of-scope line in the orchestration input. | User, 2026-11-14 ("Assess and implement") |
| D2 | qGWLF, TopInF and BotInF stay rejected, with their audit triggers. Documentation only. | User, 2026-11-14 ("Confirm rejection") |
| D3 | Dry-regime strategy: isothermal dry closed deck (vapor on). Its design is approved at gate G30.3 before any run. | User, 2026-11-14 ("Isothermal dry closed deck") |
| D4 | The Rust trajectory producer (C1) runs with no reference run and no gate. It compares against the four committed wet-start fixtures. | User, 2026-11-14 |
| D5 | `criteria.json` is written only after explicit approval (gate G30.6). Default: reuse existing field tolerances. | Orchestration 29 D5 (carried) |
| D6 | Planning artifacts are tracked. Planning-only commits use `docs(planning):`; code commits use Conventional Commits; no attribution trailers. | User, 2026-11-14 ("Track") |
| D7 | Workers are Sonnet, at most two in parallel, with the escalation ladder below. The orchestrator commits. Workers do not. | Carried from orchestration 29 (D3) |
| D8 | Serial gate at each wave end: five suites, then `git diff --name-only` against the task's permitted paths. | Carried from orchestration 29 (D4) |
| D9 | No reference run before its gate. Every run writes to `.development/30.<wave>.<id>/`. Fixtures are staged only after approval. The runner refuses to overwrite existing fixtures; keep it that way. | Orchestration 29 rules; runner `tools/run_vapor_reference.py` |
| D10 | Vapor rejections stay for every form except those accepted by typed guards (ADR 0021; no silent liquid-only approximation). | Carried from orchestration 29 (D6) |
| D11 | Git index: `planning/GOALS.md` is staged (`M ` in index) at plan drafting, and it was not staged by this plan. The orchestrator must not commit it blind. Confirm with the user before wave 0 commits. | Recon, 2026-11-14 |

## Escalation ladder

1. Micro-plan off-spec twice: orchestrator writes the micro-TODO itself and gives it to the worker as fixed input.
2. Acceptance fails after one retry (retry = failure output fed back as `FAILED VERIFICATION: ...`): stop the worker, re-run on Opus. No third small-model attempt.
3. No completion report, or a malformed one: feedback, one retry, then Opus.
4. Out-of-scope edit needed: decomposition bug. Re-scope this plan; do not widen permissions.
5. A reference run fails its convergence check: stop. Report the run record. Do not try a second knob without a new gate.
6. A Rust-vs-fixture comparison fails: report the max differences by field and time. Do not change tolerances (D5) or fixtures.

## Recon facts (anchors; re-verify at wave 0)

- Producer: `crates/hydrus-core/tests/vapor_fortran_parity.rs` (1739 lines). `ALL_CASES` at `:62` lists five cases (no closed or free-drainage cases). `run_case` at `:693`. Its deck assertion block asserts fixed heads at both ends (`!deck.top_inf && !deck.bot_inf && deck.k_top == 1 && deck.k_bot == 1`). **The closed-pair decks (`KodTop=-1`, `KodBot=-1`) fail this assertion. A new flux branch is required.** The lower mode at `:827` is `LowerBoundaryMode::FixedHead`; the upper at `:832` is `UpperBoundary::Head`. `parse_deck` at `:470`, `KodTop` at `:515`, `KodBot` at `:517`.
- Closed-pair decks: `reference/vapor_fortran/cases/vapor_closed_flux_{on,off}/`. Their `SELECTOR.IN` has `rTop=0.0 rBot=0.0`. The flux-known pair (`vapor_flux_known_{on,off}`) has `rTop=-1e-4 rBot=-5e-5` (per 29.4.1/29.4.2 reports). Re-verify in each deck.
- Fixtures: 22 files in `reference/vapor_fortran/fixtures/` (`dry_discrimination.json` plus 21 case fixtures; the 22-file count was read from `ls`, not reparsed). Keys: `case_id`, `evidence_class`, `head`, `liquid_moisture`, `temperature_c`, `time_points`, `node_depths`, `node_numbers`, `vapor_fields` (con, con_lt, con_vh, con_vt, v_liquid, v_total, v_vap_iso, v_vap_term, v_vapor), `vapor_output_available`, `source_matched`, `manifest`, `output_formats`, `source_input_mapping`, `schema_version`.
- Runner: `tools/run_vapor_reference.py` (197 lines). Output blocks parsed at `:133-143` (`NOD_INF.OUT` and `NOD_INF_V.OUT`). `BALANCE.OUT` is not in the output list. Refuses to overwrite fixtures (`:103-105`).
- Comparator: `tools/compare_vapor_reference.py` (595 lines). Field tolerances are per-field in `criteria.json["fields"]`. Discrimination entries bind to `case`/`control` names (`criteria.json["discrimination"]`, currently `dry_on_off_*`). Trajectory comparison (`compare_matrix`, `:339`) uses the global `fields`, not per-case entries.
- Criteria: `criteria.json` (403 lines). Top keys: `approved_at`, `approved_by`, `checks`, `contract_revision`, `contract_sha256`, `discrimination`, `fields`, `revision_note`, `schema_version`. Checks: `heat_balance_interior`, `water_balance_interior`, `water_balance_interior_step_off_reference`.
- Balance test: `crates/hydrus-core/tests/vapor_closed_pair_balance.rs` (199 lines). Driven residual ~4e-14 relative. Covers one accepted step only.
- Water boundary code: `crates/hydrus-core/src/heat/vapor/water.rs` (builders), `crates/hydrus-core/src/heat/vapor/mod.rs` (guard `VaporBoundaryPair::classify`; rejections at `:211-218` lower and `:219-226` upper). `crates/hydrus-core/tests/vapor_boundary_guard.rs` covers rejections.
- Fortran free drainage: `src_reference/WATFLOW.FOR:202-213` (`ConB` includes vapor conductance before `rBot`; the gravity term B does not). Closed-pair Fortran terms: `:202-229`, `:272-294`, `:315-334`. Read-only.
- Audits: `planning/master/M6_VAPOR_FREE_DRAINAGE_AUDIT.md` (ConB coupling only); `planning/master/M6_VAPOR_WATER_FLUX_AUDIT.md`.
- ADRs: `docs/decisions/README.md`. ADR 0021 is Proposed (couples heat one-way after the water step). No accepted ADR is contradicted by this plan.
- Git: `git status` at drafting shows `M  planning/GOALS.md` (staged), `M planning/EXECUTION_PLAN_29.md` and `M planning/STATE_29.md` (unstaged, closing edits from this session). Repo is a Git repository on `main`.
- Test commands: `cargo build --workspace --locked`; `cargo test --workspace --no-fail-fast`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `python3 -m unittest discover -s tools -p 'test_*.py'`. Focused: `cargo test -p hydrus-core --test vapor_fortran_parity`, `--test vapor_closed_pair_balance`, `--test vapor_boundary_guard`.
- Reference build: `.development/29.3.3/reference-build` (reused in 29.4.x). Reuse it for runs; re-verify the executable hash at wave 0.

## Hazard notes

- H-30.1: `vapor_fortran_parity.rs` is 1739 lines. Workers enter through the anchors above; no whole-file reads.
- H-30.2: The producer's deck assertions are the boundary contract. Relax them only for the flux branch, never for fixed-head cases.
- H-30.3: Fortran `ConB` includes vapor conductance before `rBot`, but the gravity term does not. Free drainage with vapor may be internally inconsistent in the source. The option chosen at gate G30.2 decides whether the Rust path matches the source as written or records a source defect.
- H-30.4: Dry-regime divergence (29.3.3, 29.4.1): head moves thousands of cm in one vapor step at -1e4 cm with a free top head. The isothermal deck removes thermal vapor flux; if it converges, that supports the hypothesis, but it does not prove the cause.
- H-30.5: Existing fixtures and `criteria.json` are immutable inputs without approval. The runner refuses to overwrite fixtures.
- H-30.6: `planning/master/M6_VAPOR_CLOSED_PAIR_FIXTURE_DESIGN.md` and the 29 designs still say -1e4 in places. Amend them only in a planning task at the gate, not in code tasks.
- H-30.7: `planning/GOALS.md` is staged (D11). Workers never stage it.

## Wave table

| Wave | Task | Title | Depends on | Gate before |
|---|---|---|---|---|
| 0 | — | Baseline: five suites; verify reference-build hash; confirm the staged `GOALS.md` with the user | — | Wave 1 |
| 1 | 30.1.1 | Rust closed-pair flux producer; compare with four wet-start fixtures (C1) | — | — |
| 1 | 30.1.2 | Confirm qGWLF and ±3 rejections in the acceptance record (C2) | — | — |
| 1 | 30.1.3 | Free-drainage ConB/rBot assessment: verdict and options (C6), no code | — | — |
| — | G30.2 | User chooses the free-drainage option from 30.1.3 | 30.1.3 | Wave 2 |
| 2 | 30.2.1 | Free-drainage lower row with vapor and typed guard (per G30.2 option) | 30.1.3, G30.2 | — |
| — | G30.3 | User approves the dry-regime deck design, the free-drainage deck design, and the runner BALANCE.OUT change | 30.3.1, 30.3.2 | Wave 4 runs |
| 3 | 30.3.1 | Runner: add `BALANCE.OUT` to outputs; full-trajectory balance parser (C4 part 1) | 30.1.1 | G30.3 for any run; no run in this task |
| 3 | 30.3.2 | Deck design document: dry isothermal closed pair (C3) and free-drainage vapor-on pair (decks, no run) | 30.1.3 | G30.3 |
| — | G30.4 | User approves each reference run design before it runs | 30.3.2 | Wave 4 runs |
| 4 | 30.4.1 | Dry isothermal closed pair, vapor on, run in `.development`; convergence record (C3) | 30.3.1, 30.3.2, G30.4 | — |
| 4 | 30.4.2 | Free-drainage vapor-on and vapor-off pair runs in `.development` (C6 evidence) | 30.2.1, 30.3.2, G30.4 | — |
| 4 | 30.4.3 | Full-trajectory balance of the closed pair from the new runner output in `.development` (C4 part 2) | 30.3.1, G30.4 | — |
| — | G30.5 | User approves fixture staging (dry, free drainage) and the new trajectory comparisons | 30.4.1, 30.4.2 | Wave 5 |
| 5 | 30.5.1 | Stage approved fixtures and manifests; extend the Rust producer (flux, free drainage); trajectory comparison | 30.1.1, 30.2.1, 30.4.1, 30.4.2, G30.5 | — |
| 5 | 30.5.2 | Criteria proposal (D5): draft entries with evidence, no write | 30.4.3, 30.5.1 | — |
| — | G30.6 | User approves criteria entries (D5) | 30.5.2 | Wave 6 |
| 6 | 30.6.1 | Write approved entries into `criteria.json`; re-run comparator and tests | 30.5.2, G30.6 | — |

Waves 1 and 3 have disjoint file sets (checked below). Wave 4's three tasks write only under `.development/`. Wave 5 has two tasks with disjoint files: 30.5.1 touches the producer and fixtures; 30.5.2 writes a planning document only.

## Task specs

### 30.1.1 — Rust closed-pair flux producer (C1)

- **Repo:** hydrus.rs. **Depends on:** none. **Gate before:** none (no reference run, no deck change).
- **Files in scope.** Must edit: `crates/hydrus-core/tests/vapor_fortran_parity.rs`. Can create: `crates/hydrus-core/tests/vapor_closed_flux_trajectory.rs` (only if the new tests grow past what fits in the existing file; otherwise add them to the existing file). Scratch: `.development/30.1.1/`. Do not edit: `criteria.json`, fixtures, decks, `src_reference/`, `tools/`.
- **Spec.** Add a flux branch to `run_case` keyed on the deck: when `KodTop == -1 && KodBot == -1`, skip the fixed-head assertions for those two ends and assert the flux pair instead (`lVapor` as the fixture says; `rTop`, `rBot` from the deck). Build the lower row as `LowerBoundaryMode::PrescribedFlux` with `flux = rBot`, and the upper as the flux form with `flux = -rTop` (sign convention, Rust fluxes are positive-in). Use the vapor-aware builders in `heat/vapor/water.rs` through the existing driver. Produce a candidate for `vapor_closed_flux_on` and `vapor_closed_flux_off`, and for `vapor_flux_known_on` and `vapor_flux_known_off`. Add the four case names to a new list (do not change `ALL_CASES` for existing cases; add `FLUX_CASES`). Compare head, liquid_moisture and temperature_c using the existing per-field tolerances from `criteria.json["fields"]`. Report the max absolute and relative differences per field and per print time. If vapor-off uses the same flux path, use the existing water-only path; confirm in Analysis.
- **Anchors.** `ALL_CASES` `:62`; `run_case` `:693`; deck assertions in `run_case` `:700-730` (fixed-head block); `LowerBoundaryMode::FixedHead` `:827`; `UpperBoundary::Head` `:832`; `parse_deck` `:470`; `KodTop` `:515`; `KodBot` `:517`. Reference for flux rows: `crates/hydrus-core/src/heat/vapor/water.rs` builders; guard `crates/hydrus-core/src/heat/vapor/mod.rs:211-226`. Driver test pattern: `crates/hydrus-core/tests/vapor_closed_pair_balance.rs`.
- **Acceptance.** New tests, named `closed_flux_*`, pass: `cargo test -p hydrus-core --test vapor_fortran_parity closed_flux`. Full file still passes: `cargo test -p hydrus-core --test vapor_fortran_parity`. A comparison report is written to `.development/30.1.1/` and quoted in the report with max differences by field.
- **Constraints.** Do not loosen any tolerance. If a field misses its tolerance, the test must fail and the report must say so; do not mark it as known. Do not relax the fixed-head assertions for existing cases.
- **Grep anchors (worker must not read the whole file):** `run_case`, `ALL_CASES`, `KodTop`, `LowerBoundaryMode`, `UpperBoundary`, `compare_series`, `outcome`.

### 30.1.2 — Confirm qGWLF and ±3 rejections (C2)

- **Repo:** hydrus.rs. **Depends on:** none. **Gate before:** none.
- **Files in scope.** Must edit: `planning/master/M6_VAPOR_G22_2_ACCEPTANCE.md`. Can create: `planning/tasks/30.1.2.todo.md`, `planning/tasks/30.1.2.report.md`. Do not edit: `GOALS.md`, `STATE_30.md` (orchestrator writes board rows).
- **Spec.** Add a dated section "C2 decision (user, 2026-11-14)" to the acceptance record. For `qGWLF`, `TopInF` and `BotInF`, state: confirmed rejected; quote the audit's trigger (from `M6_VAPOR_WATER_FLUX_AUDIT.md`); cite the corresponding reject site in code (`heat/vapor/mod.rs` or `validate_water_input`, grep `qGWLF`, `TopInF`, `BotInF`). Quote the user's decision verbatim.
- **Anchors.** Grep `qGWLF`, `TopInF`, `BotInF` in `planning/master/M6_VAPOR_WATER_FLUX_AUDIT.md` and `crates/hydrus-core/src`.
- **Acceptance.** `git diff --name-only` shows only the two paths named above. `grep -n "C2 decision (user, 2026-11-14)" planning/master/M6_VAPOR_G22_2_ACCEPTANCE.md` returns one line. No tests needed (docs only); state that.
- **Constraints.** No code change. If a cited trigger text does not exist in the audit, say so; do not invent one.

### 30.1.3 — Free-drainage ConB/rBot assessment (C6; no code)

- **Repo:** hydrus.rs. **Depends on:** none. **Gate before:** none. Output feeds gate G30.2.
- **Files in scope.** Can create: `planning/master/M6_VAPOR_FREE_DRAINAGE_ASSESSMENT.md`, `planning/tasks/30.1.3.todo.md`, `planning/tasks/30.1.3.report.md`. Read only: `src_reference/WATFLOW.FOR` lines 202-229 (read-only; do not edit), `crates/hydrus-core/src/heat/vapor/water.rs`, `crates/hydrus-core/src/lower_boundary.rs` (grep `FreeD`/`FreeDrainage`), `planning/master/M6_VAPOR_FREE_DRAINAGE_AUDIT.md`. Do not edit any other file.
- **Spec.** Write the assessment with sections: (1) Verdict on the `ConB` inconsistency with evidence (quote the Fortran lines with line numbers; state what `ConB` includes and what the gravity term B excludes). (2) Rust behaviour today (grep `FreeD` rejection sites). (3) Options: (a) match the source as written (carry the vapor term in `ConB` and the gravity term as written); (b) correct as a source defect (new SD-NNN entry proposed, not written); (c) reject with a trigger. For each option: Rust rows affected, guard change, test impact, and the reference run it needs. (4) Recommendation with reasons. (5) Up to five numbered questions for the user.
- **Anchors.** `WATFLOW.FOR:202-213` (ConB), `:202-229` (bottom flux), `:221-222` (`rBot` in PB). Rust `FreeD` grep in `lower_boundary.rs` and `heat/vapor/mod.rs`.
- **Acceptance.** The assessment file exists with the five sections above. Every Fortran claim cites a line number. Facts from code are marked "verified" with the file and line; anything not read is marked "not read in scope". No code or run.
- **Constraints.** Distinguish "not read in scope" from verified facts. Do not write an SD register entry; propose only.

### 30.2.1 — Free-drainage lower row with vapor (implementation; after G30.2)

- **Repo:** hydrus.rs. **Depends on:** 30.1.3, G30.2 (the user's chosen option). **Gate before:** G30.2.
- **Files in scope.** Must edit: `crates/hydrus-core/src/heat/vapor/water.rs` (free-drainage lower builder), `crates/hydrus-core/src/heat/vapor/mod.rs` (guard classify: accept the free-drainage lower form only, per the chosen option), `crates/hydrus-core/tests/vapor_boundary_guard.rs` (update the free-drainage rejection assertions only). Can create: `crates/hydrus-core/tests/vapor_free_drainage_balance.rs`. Scratch: `.development/30.2.1/`. Do not edit: `lower_boundary.rs` unless the option needs it (then STOP and report; it is out of this task's list), `criteria.json`, fixtures, `src_reference/`.
- **Spec.** Implement the lower free-drainage row as the chosen option requires. Keep the typed guard strict: accept only the free-drainage lower form with the closed-pair upper, every other combination still rejected. Add a one-step column balance test in `vapor_free_drainage_balance.rs` mirroring `vapor_closed_pair_balance.rs`.
- **Anchors.** `heat/vapor/mod.rs:211-218` (lower rejection), `:219-226` (upper rejection); `vapor_boundary_guard.rs` free-drainage assertions (grep `FreeD`, `FreeDrainage`); `vapor_closed_pair_balance.rs` as pattern.
- **Acceptance.** `cargo test -p hydrus-core --test vapor_free_drainage_balance` and `cargo test -p hydrus-core --test vapor_boundary_guard` pass; `cargo test -p hydrus-core --test vapor_closed_pair_balance` still passes.
- **Constraints.** Every other lower or upper form stays rejected at its current site with its trigger. No silent liquid-only approximation (D10). Cite the Fortran line for each vapor term added to a row (H2).

### 30.3.1 — Runner: `BALANCE.OUT` output and full-trajectory parser (C4 part 1; gated)

- **Repo:** hydrus.rs. **Depends on:** 30.1.1. **Gate before:** G30.3 for the change; no run in this task.
- **Files in scope.** Must edit: `tools/run_vapor_reference.py`, `tools/test_run_vapor_reference.py`. Scratch: `.development/30.3.1/`. Do not edit: fixtures, decks, `criteria.json`, `compare_vapor_reference.py`, `src_reference/`.
- **Spec.** Add `BALANCE.OUT` to the runner's output list as an optional output, with a parser that returns per-time-step balance rows. Existing fixture generation must keep refusing to overwrite. The new output is written to a separate path (`<case>_balance.json`) so existing manifests are not changed.
- **Anchors.** `tools/run_vapor_reference.py:133-143` (output loop), `:103-105` (overwrite refusal), `:185-187` (whitelist). Grep `outputs`, `NOD_INF`, `BALANCE`.
- **Acceptance.** `python3 -m unittest tools.test_run_vapor_reference` passes; full Python suite passes. A unit test with a fixture-like `BALANCE.OUT` sample parses it.
- **Constraints.** No reference run in this task. The parser must fail on a truncated or absent `BALANCE.OUT` (not silently pass).

### 30.3.2 — Deck design: dry isothermal closed pair and free-drainage vapor-on pair (docs only; gated)

- **Repo:** hydrus.rs. **Depends on:** 30.1.3. **Gate before:** G30.3 (approval before any run).
- **Files in scope.** Can create: `planning/master/M6_VAPOR_ORCH30_DECK_DESIGN.md`, `planning/tasks/30.3.2.todo.md`, `planning/tasks/30.3.2.report.md`. Read only: `reference/vapor_fortran/cases/vapor_closed_flux_on/` (`SELECTOR.IN`, `PROFILE.DAT`), `reference/vapor_fortran/cases/vapor_flux_known_on/`, `reference/vapor_fortran/cases/vapor_dry_thermal_gradient/`.
- **Spec.** Write the design for two decks. (a) Dry isothermal closed pair: start from `vapor_closed_flux_on` at h = -10000 cm (dry), uniform temperature equal to the closed deck's mean temperature, `KodTop=-1`, `KodBot=-1`, rTop=rBot=0. Name the one-change rule and the expected failure signatures. (b) Free-drainage vapor-on pair: from `vapor_closed_flux_on` with `KodBot=-1` replaced by free drainage (`FreeD`), per the chosen option. State exactly which SELECTOR lines change. Each deck: the one-change rule, the expected outputs, the run budget, and the stop rules (per escalation 5). No deck files are created in the repo in this task.
- **Anchors.** `SELECTOR.IN` lines 17-24 (`TopInf WLayer KodTop InitCond`; `BotInf qGWLF FreeD SeepF KodBot DrainF hSeep`); `PROFILE.DAT` column 3 (head) and temperature column.
- **Acceptance.** The design file contains: deck A and deck B change lists with line numbers; the one-change rule; the run budget; the stop rules. No file under `reference/` is changed.
- **Constraints.** Do not run anything. Do not invent convergence results.

### 30.4.1 — Dry isothermal closed pair run (C3; gated per run)

- **Repo:** hydrus.rs. **Depends on:** 30.3.1, 30.3.2, G30.4. **Gate before:** G30.4.
- **Files in scope.** Scratch only: `.development/30.4.1/` (case directory, runs, outputs). Do not write under `reference/`. Do not edit `criteria.json` or existing fixtures.
- **Spec.** Build the dry isothermal deck per the approved design under `.development/30.4.1/`, using the reused reference build. Run to 240 h. Record: exit code; number of non-convergence messages; NaN count; hTop at 0.1, 0.5, 1, 6, 24, 72, 240 h; last time printed. Write the convergence record to `planning/tasks/30.4.1.report.md`. If it converges, produce the candidate fixture in `.development/` only.
- **Anchors.** `planning/master/M6_VAPOR_ORCH30_DECK_DESIGN.md` (after approval); `planning/tasks/29.4.1.report.md` (divergence reference record).
- **Acceptance.** Report contains the record above. Pass: exit 0, zero non-convergence messages, no NaN, prints present at all five time points. Fail: report the failure; do not retry with another knob (escalation 5).
- **Constraints.** One run for the approved design. No deck edits after the run starts.

### 30.4.2 — Free-drainage vapor-on and vapor-off pair runs (C6 evidence; gated per run)

- **Repo:** hydrus.rs. **Depends on:** 30.2.1, 30.3.2, G30.4. **Gate before:** G30.4.
- **Files in scope.** Scratch only: `.development/30.4.2/`. Do not write under `reference/`.
- **Spec.** Run the free-drainage vapor-on and vapor-off decks from the approved design under `.development/30.4.2/`. Same record as 30.4.1 (exit, messages, NaN, hTop, prints). Also record the lower-flux `ConB` term magnitude at the first print time if the runner's output allows it; otherwise state "not available".
- **Anchors.** Design doc from 30.3.2; `src_reference/WATFLOW.FOR:202-213` for the term.
- **Acceptance.** Two records in `planning/tasks/30.4.2.report.md`. Pass criteria as for 30.4.1, for both decks.
- **Constraints.** Same as 30.4.1.

### 30.4.3 — Full-trajectory balance of the closed pair (C4 part 2; gated)

- **Repo:** hydrus.rs. **Depends on:** 30.3.1, G30.4. **Gate before:** G30.4.
- **Files in scope.** Scratch: `.development/30.4.3/`. Do not write fixtures.
- **Spec.** Re-run the two existing closed-pair decks (`vapor_closed_flux_on`, `vapor_flux_known_on`) into `.development/30.4.3/` with the new runner so that `BALANCE.OUT` is produced. Record per-step balance residuals and the maximum relative residual over the trajectory. Do not overwrite the committed fixtures.
- **Anchors.** `tools/run_vapor_reference.py` (30.3.1 output); `crates/hydrus-core/tests/vapor_closed_pair_balance.rs` (driven one-step residual, for comparison).
- **Acceptance.** Report with max relative residual per case and the time of maximum. Pass: residual under the tolerance stated in the report; stated before the run.
- **Constraints.** Re-runs produce new outputs only; committed fixtures and manifests stay unchanged.

### 30.5.1 — Stage approved fixtures and extend the Rust producer (C1 part 2, C3, C6; after G30.5)

- **Repo:** hydrus.rs. **Depends on:** 30.1.1, 30.2.1, 30.4.1, 30.4.2, G30.5. **Gate before:** G30.5.
- **Files in scope.** Must edit: `crates/hydrus-core/tests/vapor_fortran_parity.rs` (add the dry and free-drainage cases to the producer and trajectory tests). Can create: `reference/vapor_fortran/cases/<approved case dirs>/` and `reference/vapor_fortran/fixtures/<approved fixtures>.json` (approved outputs only; the runner refuses to overwrite). Scratch: `.development/30.5.1/`. Do not edit: `criteria.json`, existing fixtures, `src_reference/`.
- **Spec.** Copy approved candidates from `.development/30.4.1` and `.development/30.4.2` into `reference/vapor_fortran/` as new files only. Extend the flux producer to the free-drainage lower case. Add trajectory comparison tests for the dry and free-drainage cases. Use the existing per-field tolerances.
- **Anchors.** `vapor_fortran_parity.rs`: `FLUX_CASES` (added in 30.1.1), `run_case` flux branch, `outcome`.
- **Acceptance.** `cargo test -p hydrus-core --test vapor_fortran_parity` passes; `python3 -m unittest discover -s tools -p 'test_*.py'` passes (manifests and whitelist).
- **Constraints.** No change to `criteria.json`. If a comparison fails, report the max differences; do not change tolerances or fixtures.

### 30.5.2 — Criteria proposal (D5; docs only)

- **Repo:** hydrus.rs. **Depends on:** 30.4.3, 30.5.1. **Gate before:** none for the proposal; G30.6 for writing.
- **Files in scope.** Can create: `planning/master/M6_VAPOR_ORCH30_CRITERIA_PROPOSAL.md`, `planning/tasks/30.5.2.todo.md`, `planning/tasks/30.5.2.report.md`. Read only: `reference/vapor_fortran/criteria.json`, `.development/29.4.2/onoff.txt`, `.development/30.5.1/` outputs.
- **Spec.** Propose discrimination entries for head, liquid_moisture and temperature_c per new closed pair (dry and free-drainage pairs; the wet closed pair too), and nonzero entries for the `_on` ids. Each proposal: case, control, field, factor_of_tolerance, rule, unit, with the on/off max difference from the evidence. Give the existing `dry_on_off_*` entries as the model. Do not write.
- **Anchors.** `criteria.json` `discrimination` block (`dry_on_off_head` etc.); `compare_vapor_reference.py` `DISC_RULES` (`:58`).
- **Acceptance.** Proposal file lists each entry as a JSON block, with evidence and the rule it uses. The file is reviewed at G30.6.
- **Constraints.** Write nothing to `criteria.json`. Do not change any tolerance without approval.

### 30.6.1 — Write approved criteria entries (D5; gated)

- **Repo:** hydrus.rs. **Depends on:** 30.5.2, G30.6. **Gate before:** G30.6.
- **Files in scope.** Must edit: `reference/vapor_fortran/criteria.json` (approved entries only; update `approved_at`, `approved_by`, `revision_note`, and `contract_sha256` as the comparator requires). Must edit: `tools/test_compare_vapor_reference.py` only if the comparator tests need the new case names. Scratch: `.development/30.6.1/`.
- **Spec.** Write exactly the entries the user approved at G30.6. Run the comparator on all fixtures; record PASS/FAIL per check.
- **Anchors.** `criteria.json` top-level keys; `compare_vapor_reference.py` `TOP_KEYS` (`:62`), `DISC_RULES` (`:58`).
- **Acceptance.** `python3 -m unittest discover -s tools -p 'test_*.py'` passes; `cargo test -p hydrus-core --test vapor_fortran_parity` passes; comparator run recorded in the report.
- **Constraints.** Only approved entries. No other key changes. If the comparator rejects the file, STOP and report.

## Wave-end serial gate (every wave)

1. `cargo build --workspace --locked`
2. `cargo test --workspace --no-fail-fast`
3. `cargo fmt --all -- --check`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `python3 -m unittest discover -s tools -p 'test_*.py'`
6. `git diff --name-only` (plus untracked list) compared against each task's permitted paths. Any path outside them stops the wave.

Waves 3 and 4 add: no change under `reference/` except where a task says so.

## Wave-level file disjointness check

- Wave 1: 30.1.1 edits `vapor_fortran_parity.rs`; 30.1.2 edits the acceptance doc; 30.1.3 creates the assessment doc. Disjoint.
- Wave 2: one task (30.2.1). Disjoint by construction.
- Wave 3: 30.3.1 edits `tools/run_vapor_reference.py` and its test; 30.3.2 creates planning docs. Disjoint.
- Wave 4: all three write only under `.development/` and reports under `planning/tasks/`. Disjoint.
- Wave 5: 30.5.1 edits `vapor_fortran_parity.rs` and creates reference files; 30.5.2 creates planning docs. Disjoint.
- Wave 6: one task.

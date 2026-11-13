# Orchestration 27 — G22.4 remaining G22 rows (nonuniform T, ConVh, sink/root)

Status: **Approved 2026-11-13 (user). Not yet executed.**
Created 2026-11-13. Repo: `/home/andrew/AI/hyd.rs`, branch `main`, HEAD `603b4e4`.
Resume: [STATE_27.md](STATE_27.md). Worker: [PROMPT_TEMPLATE_27.md](PROMPT_TEMPLATE_27.md).
Goal source: [GOALS.md](GOALS.md) sequenced goal **G22.4**. Coverage: [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md).
Prior orchestration: [EXECUTION_PLAN_26.md](EXECUTION_PLAN_26.md) (G22.1, completed with exceptions).

## Goal & Scope

**Goal.** Close the three remaining G22.4 rows: (a) nonuniform-temperature coverage for the default thermal conductivity, (b) the scalar isothermal `ConVh` helper, and (c) nonzero prescribed sink and root uptake with heat and vapor. Each row ends accepted with evidence, or rejected with a named trigger. Row (c) starts as an audit; the disposition after the audit is the user's decision (Gate S).

**In scope:**
- (a) Audit which nonuniform-temperature profiles the vapor path covers with `iCampbell=0`; add reference fixtures for the gap; Rust parity and independent balance checks against the frozen criteria.
- (b) Locate the scalar isothermal `ConVh` helper (hardcoded 20 °C), audit its reachability, then remove it or make it fail loudly, per user decision.
- (c) Source and coupling audit of nonzero prescribed sink and root uptake with heat and vapor. Disposition by audit, then Gate S.

**Out of scope:**
- Implementing sink or root uptake with vapor. If Gate S chooses implementation, that becomes a new orchestration (28), not an expansion of this one.
- Campbell (`iCampbell=1`, SD-006 policy), enhancement off (`iEnhanc=0`, G22.5/G22.6), temperature-dependent liquid water (G23), energy balance (G20), water flux and atmospheric switching (G22.2, G22.3).
- Sinusoidal top temperature (already rejected).
- Any change to pinned Fortran source, existing fixtures, `criteria.json` values for existing fields, existing evidence, or the G22.1 acceptance decision.
- New dependencies, solute changes, M5 campaign work.

**Verbatim constraints (user, 2026-11-13):**
- "G22.4 first" (this is the next goal after G22.1).
- "Audit, then decide" for sink and root uptake.
- "Fixtures plus Rust parity" for nonuniform temperature.
- "Remove or guard it" for the scalar `ConVh` helper.
- "Track" for planning artifacts.
- "Haiku, max two" for workers.
- Carried from orchestration 26: fixed-head water BC unless the audit says otherwise; criteria frozen (no widening); single-precision reference (H7) disclosed; serial suite gates per wave.

## Decision log (dated 2026-11-13)

| # | Decision | Source |
|---|---|---|
| D1 | Scope is G22.4 rows (a), (b), (c) as listed in GOALS.md. | User, GOALS |
| D2 | Row (c): audit first; disposition by Gate S. Implementation is not authorized by this plan. | User, AskUserQuestion |
| D3 | Row (a): fixtures plus Rust parity plus balance check, same pattern as orchestration 26. | User, AskUserQuestion |
| D4 | Row (b): remove or guard the scalar helper; no change to the array routine. | User, AskUserQuestion |
| D5 | Planning artifacts tracked; planning-only commits use `docs(planning):`, separate from code. | User, AskUserQuestion |
| D6 | Workers: Haiku, at most two parallel worktrees, one retry then escalation to Sonnet. | User, AskUserQuestion |
| D7 | ADR 0021 (Proposed) is complied with, not superseded. No Accepted ADR governs heat. | Recon; carried from orchestration 26 D7 |
| D8 | Campbell and enhancement-off stay out of scope. | User, GOALS |
| D9 | Criteria v3 (`criteria.json`, hash `f198d0c1…`, contract revision `0c4c743`) are frozen. New cases use them unchanged. If a new case needs a bound that v3 does not cover, stop at Gate N2. | Carried from orchestration 26 D6 |
| D10 | Reference executable for new fixtures: the orchestration 26.2.2 harness build `.development/26.2.2/build/hydrus-reference`, SHA-256 `e6d69dd3…`, verified present. Not the instrumented drift build. | Recon |
| D11 | Judgment call (for approval): if Gate S chooses sink implementation, it goes to orchestration 28. | Planner judgment; flagged |

## Hazard notes from recon (for workers)

- **H1 Sink and root are rejected in vapor.** `vapor/mod.rs:179-188` returns `RootUptake` for any root input and `NonzeroSink` for any nonzero sink. `vapor/mod.rs:246-250` has the typed variants. Do not relax these in this orchestration.
- **H2 Heat transport has a sink term.** `heat/transport.rs:42` (`sink` field), `:195`, `:381`, `:405`, `:444`. Heat-only runs accept sink today. Do not assume the vapor path can reuse that term without the audit.
- **H3 Producer guards.** `tools/run_vapor_reference.py` refuses to overwrite fixtures (`:97-107`) and requires `--task-base` under `.development/` (`:181`). The case list is a hard-coded `expected` set (`:184-185`) that must be extended for new IDs.
- **H4 Scalar `ConVh` helper not yet located.** Recon found the Fortran array routine at `WATFLOW.FOR:968-995` and its callers (`:14,31,68,83,88,178,187,211`). It did not find a Rust helper by name. Do not assume a location: 27.1.2 must locate it or report that it is absent.
- **H5 Existing nonuniform coverage.** `vapor_thermal_gradient` and `vapor_dry_thermal_gradient` exist. 27.1.3 must establish what they cover before any new fixture is written.
- **H6 Single-precision reference (H7).** The fixtures are single-precision reference output. The FD/FF head drift of orchestration 26 is a reference-precision effect (see [D-26.2_DRIFT_MECHANISM.md](master/D-26.2_DRIFT_MECHANISM.md)). New cases may show similar precision-limited differences. Record them as explained; do not widen criteria.
- **H7 Fixed-head water BC.** Water BC is fixed heads at both ends for every new case unless 27.1.1 says otherwise.
- **H8 Disjointness.** Wave 3 and wave 4 task file sets are checked in the wave table. Do not add a shared registry file to two tasks.

## Wave table

| Wave | Task | Title | Depends on | Gate before |
|---|---|---|---|---|
| 1 | 27.1.1 | Sink and root-uptake source/coupling audit (read-only) | — | Gate S |
| 1 | 27.1.2 | ConVh scalar helper location and reachability audit | — | — |
| 1 | 27.1.3 | Nonuniform-temperature coverage audit | — | — |
| — | **GATE S (user)** | Disposition of row (c) after 27.1.1 | 27.1.1 | Wave 2 only if row (c) is listed; otherwise not blocking |
| 2 | 27.2.1 | Nonuniform case specs and producer | 27.1.3 | — |
| 2 | 27.2.2 | Reference-only fixture run | 27.2.1 | GATE N2 if needed |
| — | **GATE N2 (user, conditional)** | Only if a new case needs a bound not in criteria v3 | 27.2.2 | Wave 3 comparison |
| 3 | 27.3.1 | ConVh helper removed or guarded | 27.1.2 | — |
| 3 | 27.3.2 | Nonuniform-T Rust parity test | 27.2.2 | — |
| 4 | 27.4.1 | Python comparator for nonuniform cases | 27.2.2, 27.3.2 | — |
| 4 | 27.4.2 | Independent balance checks for nonuniform cases | 27.2.2 | — |
| 5 | 27.5.1 | Acceptance, coverage and goal handoff | 27.1.1, 27.3.1, 27.4.1, 27.4.2, Gate S | — |

Waves run at most two parallel worktrees. Serial full-suite gates at the end of each wave.

Wave 1 has three read-only tasks writing three different docs. Run two at a time, then the third. Waves 3 and 4 each have two tasks with disjoint file sets.

## Wave gates

- **Pre-wave-1 baseline.** Repeat the five Quick Start suites at `603b4e4`. Expected from orchestration 26's final gate (`94967be`, no code change since): Rust 607 passed, 0 failed, 3 ignored; Python 459 OK. If they differ, stop and report.
- **Each wave end (orchestrator, serial):** `cargo build --workspace --locked`; `cargo test --workspace --no-fail-fast`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `python3 -m unittest discover -s tools -p 'test_*.py'` (600 s timeout).
- **GATE S (user).** After 27.1.1: the audit proposes a disposition for row (c). The user chooses one of: (i) reject with trigger, (ii) reject and open orchestration 28 for sink-only implementation, (iii) other. No code is written for row (c) in this orchestration in any case. The plan does not authorize implementation.
- **GATE N2 (user, conditional).** If 27.2.2 shows any new case that criteria v3 does not cover, the orchestrator proposes bounds from reference-only evidence. The user approves before any Rust comparison. Same pattern as GATE N1 in orchestration 26.

---

## Task specs

### 27.1.1 — Sink and root-uptake source/coupling audit (read-only)

- **Repo:** hyd.rs. **Depends on:** none.
- **Files in scope:**
  - *Must edit:* none.
  - *Can create:* `planning/master/M6_VAPOR_SINK_ROOT_AUDIT.md`.
  - *Read only:* `src_reference/HYDRUS.FOR`, `src_reference/WATFLOW.FOR`, `src_reference/TEMPER.FOR`, `src_reference/INPUT.FOR`, `crates/hydrus-core/src/heat/vapor/mod.rs`, `crates/hydrus-core/src/heat/transport.rs`, `crates/hydrus-core/src/heat/config.rs`, `crates/hydrus-core/src/heat/vapor/adjust.rs`, `planning/master/M6_VAPOR_COVERAGE.md`.
  - Scratch: `.development/27.1.1/`.
- **Spec.** Establish what a nonzero prescribed sink and root uptake do in the Fortran heat and water coupling, and what the vapor path would need.
  1. Locate every reader of `Sink` and root inputs in `HYDRUS.FOR`, `WATFLOW.FOR` and `TEMPER.FOR` (`grep -n "Sink\|SetSnk\|SetRG\|rRoot\|lRoot" src_reference/*.FOR`).
  2. Read the root and sink input reads in `INPUT.FOR` (`grep -n "SinkF\|lRoot\|iRootIn" src_reference/INPUT.FOR`).
  3. For each form, answer with line citations: (a) prescribed sink with `SinkF` and no root; (b) root uptake (`lRoot`/`iRootIn`); (c) which terms the heat equation takes the sink through (`TEMPER.FOR`), and whether `TempAdj` vapor terms depend on it; (d) whether the water step with vapor (`ConVapor`, `VaporContent`) changes under a sink.
  4. Map the Rust rejection points (`vapor/mod.rs:179-188`) and the heat-only sink path (`transport.rs:405,444`). State what a vapor-enabled sink would touch.
  5. Give a disposition per form: candidate for a bounded case (for Gate S), or reject with a trigger. Name the trigger.
  6. Write the audit as a table: form, Fortran lines, Rust lines, vapor term affected (y/n), disposition, trigger.
- **Constraints.** No code change, no fixture, no executable run. Ambiguity goes into Questions (max 5), not guessed.
- **Anchors.** `grep -n "Sink" src_reference/TEMPER.FOR src_reference/WATFLOW.FOR`; `grep -n "SinkF\|lRoot\|iRootIn\|SetSnk\|SetRG" src_reference/HYDRUS.FOR src_reference/INPUT.FOR`; `vapor/mod.rs:179-188, 246-250`; `transport.rs:41-42, 195, 381, 405, 444`.
- **Reference.** `planning/master/M6_VAPOR_COVERAGE.md` rows "Nonzero prescribed sink or root uptake" and "Water flux..." and the G22 completion rule.
- **Acceptance.**
  - `planning/master/M6_VAPOR_SINK_ROOT_AUDIT.md` exists and has a row for each of (a)–(d).
  - `grep -c "HYDRUS.FOR\|WATFLOW.FOR\|TEMPER.FOR" planning/master/M6_VAPOR_SINK_ROOT_AUDIT.md` ≥ 6.
  - `git diff --name-only` shows only the audit file.

### 27.1.2 — ConVh scalar helper location and reachability audit (read-only)

- **Repo:** hyd.rs. **Depends on:** none.
- **Files in scope:**
  - *Must edit:* none.
  - *Can create:* `planning/master/M6_VAPOR_CONVH_AUDIT.md`.
  - *Read only:* `src_reference/WATFLOW.FOR`, `src_reference/HYDRUS.FOR`, `src_reference/INPUT.FOR`, `crates/hydrus-core/src/heat/vapor/kernels.rs`, `crates/hydrus-core/src/heat/vapor/mod.rs`, `crates/hydrus-core/src/heat/vapor/water.rs`, `crates/hydrus-core/src/heat/vapor/adjust.rs`, `crates/hydrus-core/src/` (grep only).
  - Scratch: `.development/27.1.2/`.
- **Spec.** Find the scalar isothermal `ConVh` helper (hardcoded 20 °C) that the coverage matrix names, and decide whether any supported input reaches it.
  1. Locate the Fortran array routine: `WATFLOW.FOR:968-995` (definition), callers at `:14,31,68,83,88,178,187,211`. Confirm the array routine is not the helper.
  2. Locate the Rust helper. Grep for `20.0`, `293.15`, `isothermal`, `con_vh`, `ConVh` and `con_vh_scalar` in `crates/hydrus-core/src/` and `tools/`. Record every hit with file:line. Do not assume a name.
  3. If a helper is found: list its callers. For each caller, state whether a supported vapor input reaches it (cite the input path).
  4. If no helper is found: say so, cite the search commands, and stop with the question "is the helper in a file outside `crates/hydrus-core/src`?" in Questions.
  5. Disposition: *unreachable from supported input* (remove or guard, per D4), or *reachable* (report to the orchestrator; no change under this plan).
  6. Name the file(s) 27.3.1 may touch, with exact paths.
- **Constraints.** No code change. If the helper is reachable, do not design a fix; report.
- **Anchors.** `WATFLOW.FOR:968-995, 14, 31, 68, 83, 88, 178, 187, 211`; grep patterns in step 2.
- **Reference.** `planning/master/M6_VAPOR_COVERAGE.md` row "Scalar isothermal `ConVh` helper" (line 25).
- **Acceptance.**
  - `planning/master/M6_VAPOR_CONVH_AUDIT.md` exists, with a search log and a disposition line.
  - The disposition line is one of: `unreachable`, `reachable`, `not found`.
  - `git diff --name-only` shows only the audit file.

### 27.1.3 — Nonuniform-temperature coverage audit (read-only)

- **Repo:** hyd.rs. **Depends on:** none.
- **Files in scope:**
  - *Must edit:* none.
  - *Can create:* `planning/master/M6_VAPOR_NONUNIFORM_T_AUDIT.md`.
  - *Read only:* `reference/vapor_fortran/cases/vapor_thermal_gradient/`, `reference/vapor_fortran/cases/vapor_dry_thermal_gradient/`, `reference/vapor_fortran/cases/vapor_isothermal_head_gradient/`, `reference/vapor_fortran/fixtures/` (names and metadata only), `reference/vapor_fortran/criteria.json` (keys only), `src_reference/INPUT.FOR` (grep `TempIn`, `tTop`, `tBot`, `iCampbell`), `planning/master/M6_VAPOR_COVERAGE.md` row "Default thermal conductivity".
  - Scratch: `.development/27.1.3/`.
- **Spec.** Establish which nonuniform-temperature profiles the vapor path already covers with default conductivity, and which are missing.
  1. Read each existing thermal case's `fixture.json`, `SELECTOR.IN` and `PROFILE.DAT`. Record: initial temperature profile shape, top and bottom temperature, whether `iCampbell=0`, the water BC, flux settings.
  2. Read the coverage row (line 12) and the "Nonuniform-temperature source fixtures required" note. State exactly what the row requires that existing cases do not provide.
  3. Propose the minimum new case list for the gap, with IDs of the form `vapor_nonuniform_<shape>_<on|off>`. Each proposed case needs: a vapor-on and a vapor-off control; a profile shape the existing cases lack; water BC fixed heads (H7 of the plan); default conductivity.
  4. For each proposed case, check it can be expressed with the existing producer schema (`tools/run_vapor_reference.py:15-35, 171-194`) without a new schema field. If it cannot, say which field.
  5. Record any Fortran input limit found in `INPUT.FOR` (`TempIn`, `:1205-1245`).
- **Constraints.** No fixture written, no producer edit, no executable run.
- **Anchors.** `vapor_thermal_gradient/fixture.json` (`data_prefix` line 23); `tools/run_vapor_reference.py:15-35, 171-194`; `INPUT.FOR:1205-1245, :1236`.
- **Reference.** `planning/master/M6_VAPOR_COVERAGE.md:12`; orchestration 25's dry-thermal case (`git show b8e0544 --stat`).
- **Acceptance.**
  - `planning/master/M6_VAPOR_NONUNIFORM_T_AUDIT.md` exists, with a table of existing thermal cases and a proposed-case table.
  - The proposed-case table has at least one row, or states explicitly that no new case is needed, with the reason.
  - `git diff --name-only` shows only the audit file.

### 27.2.1 — Nonuniform case specs and producer

- **Repo:** hyd.rs. **Depends on:** 27.1.3.
- **Files in scope:**
  - *Must edit:* `tools/run_vapor_reference.py` (expected set, `:184-185`), `tools/test_run_vapor_reference.py`.
  - *Can create:* `reference/vapor_fortran/cases/<new-case-id>/` directories (contents: `fixture.json`, `SELECTOR.IN`, `PROFILE.DAT`, `LEVEL_01.DIR` and any other input the existing thermal case uses).
  - *Read only:* the case list in `planning/master/M6_VAPOR_NONUNIFORM_T_AUDIT.md`; the existing `vapor_thermal_gradient/` directory.
  - Scratch: `.development/27.2.1/`.
- **Spec.** Add one case directory per row of the 27.1.3 proposed-case table, with the same layout and fixture schema as `vapor_thermal_gradient`.
  1. Copy the layout of `vapor_thermal_gradient/` for each new ID. Change only the temperature profile and IDs. Water BC stays fixed heads.
  2. Extend `expected` at `tools/run_vapor_reference.py:184-185` with the new IDs. Keep existing IDs unchanged.
  3. Add producer tests: deck text for one new case; `--all` expected set contains the new IDs; existing IDs are unchanged.
  4. Run the targeted producer test.
- **Constraints.** Do not regenerate or edit existing fixtures or existing case directories. Do not change criteria. Do not add a schema field unless 27.1.3 said it is required; if it is, STOP and report. Campbell is out of scope (`iCampbell=0`).
- **Anchors.** `tools/run_vapor_reference.py:15-35, 97-107, 171-194`; `reference/vapor_fortran/cases/vapor_thermal_gradient/fixture.json`.
- **Reference implementation.** Orchestration 26's 26.2.1 producer change (`git show 07cce6e -- tools/run_vapor_reference.py tools/test_run_vapor_reference.py`).
- **Acceptance.**
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_vapor_reference` passes.
  - `git diff --name-only` shows only the producer, its test, and new case directories.
  - `git diff --stat -- reference/vapor_fortran/fixtures reference/vapor_fortran/cases/vapor_thermal_gradient reference/vapor_fortran/cases/vapor_dry_thermal_gradient` is empty.

### 27.2.2 — Reference-only fixture run

- **Repo:** hyd.rs. **Depends on:** 27.2.1.
- **Files in scope:**
  - *Can create:* `reference/vapor_fortran/fixtures/<new-case-id>.json` only.
  - *Read only:* `.development/26.2.2/build/hydrus-reference` (D10).
  - Scratch: `.development/27.2.2/`.
- **Spec.**
  1. Verify the executable: `sha256sum .development/26.2.2/build/hydrus-reference` must start `e6d69dd3`. If not, STOP.
  2. Run the producer for the new IDs only: `python3 tools/run_vapor_reference.py --case <id> [--case <id> ...] --build-dir .development/26.2.2/build --task-base .development/27.2.2/runs`. The producer refuses existing fixtures and run roots.
  3. Verify each fixture: finite values; output times match the existing fixtures; vapor reachability matches the case; no `Error.msg`.
  4. Record fixture SHA-256 values and the on/off discrimination (max |on−off|) per field in STATE_27.md. Reference-only evidence: no Rust comparison, no tolerance claim.
  5. Check each new case's fields and times are covered by an existing entry in `criteria.json` v3. If any is not, STOP and report for GATE N2.
- **Constraints.** Existing fixtures byte-identical. Do not use the drift-instrumented build (`.development/26-drift/`, now removed). Evidence class `pinned-fortran-trajectory` only.
- **Anchors.** `tools/run_vapor_reference.py:97-107, 171-194`; `reference/vapor_fortran/criteria.json` (keys).
- **Acceptance.**
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_vapor_reference` passes.
  - `git diff --name-only` shows only new fixture JSON files.
  - `git diff --stat -- reference/vapor_fortran/fixtures` shows additions only.
  - The orchestrator reruns `--case <one-id>` into a new task-base and the parsed values match.

### 27.3.1 — ConVh helper removed or guarded

- **Repo:** hyd.rs. **Depends on:** 27.1.2 (disposition `unreachable`, or `reachable` with an approved change). If 27.1.2 reports `not found`, this task is closed without a code change and the record says so.
- **Files in scope:**
  - *Must edit:* exactly the files named in `planning/master/M6_VAPOR_CONVH_AUDIT.md` ("Files 27.3.1 may touch"). Expected: the helper's own file and its unit test, if any.
  - *Read only:* the array routine path (`crates/hydrus-core/src/heat/vapor/kernels.rs`).
  - Do not touch `tools/`, fixtures, criteria, or any other `src/` file.
- **Spec.** Per D4: if the helper is unreachable from supported input, remove it. If it cannot be removed without changing other code, replace its body with a typed error or a `debug_assert`/`compile_error` guard, and add a unit test that the guard fires.
  1. Confirm no caller in `crates/` or `tools/` uses the helper (`grep -rn` on the helper's name).
  2. Remove the helper, or guard it per the rule above.
  3. Add or update the unit test named in the audit.
  4. Run the targeted tests below.
- **Constraints.** Do not change the array routine's numerics. Do not change `kernels.rs` values. No new dependencies.
- **Anchors.** From `M6_VAPOR_CONVH_AUDIT.md`.
- **Acceptance.**
  - `grep -rn "<helper name>" crates tools` returns no hits outside the test module, or only the guard test.
  - `cargo test -p hydrus-core --lib` passes for the touched module.
  - `cargo build --workspace --locked` passes.

### 27.3.2 — Nonuniform-T Rust parity test

- **Repo:** hyd.rs. **Depends on:** 27.2.2.
- **Files in scope:**
  - *Can create:* `crates/hydrus-core/tests/vapor_nonuniform_parity.rs`.
  - *Read only:* `crates/hydrus-core/tests/vapor_heat_boundary_parity.rs` (pattern), the new fixtures, `reference/vapor_fortran/criteria.json` (v3).
  - Do not edit `vapor_heat_boundary_parity.rs`, `criteria.json`, or any fixture.
  - Candidate output under `.development/27.3.2/` only.
- **Spec.** Mirror `vapor_heat_boundary_parity.rs` for the new IDs. Build each deck from the case spec, run the Rust driver, write candidate JSON with `requests_sha256` and evidence class `candidate-rust`. Compare against the frozen v3 criteria. Report failing rows verbatim; do not widen.
  - The initial temperature profile must come from the case's `PROFILE.DAT`. Verify the parser reads a nonuniform profile before trusting any result (hazard: the 26.4.1 harness was written for uniform heat-boundary cases).
- **Constraints.** No change to `criteria.json`, fixtures, or the existing parity test. Heat-only entry points keep their behaviour.
- **Anchors.** `vapor_heat_boundary_parity.rs:101-190` (path helpers), `:474-580` (deck and profile parsing), `:746-` (run_case).
- **Reference implementation.** `git show 072431e -- crates/hydrus-core/tests/vapor_heat_boundary_parity.rs`.
- **Acceptance.**
  - `cargo test -p hydrus-core --test vapor_nonuniform_parity` passes, or fails only on rows in an explicit known-failure set that the worker reports verbatim.
  - The candidate JSON's `requests_sha256` matches the fixtures.

### 27.4.1 — Python comparator for nonuniform cases

- **Repo:** hyd.rs. **Depends on:** 27.2.2, 27.3.2.
- **Files in scope:**
  - *Must edit:* `tools/compare_vapor_heat_boundary.py`, `tools/test_compare_vapor_heat_boundary.py` (extend the case list only; do not change the 26.4.2 behaviour for its cases).
  - *Read only:* `reference/vapor_fortran/criteria.json` (v3), the new fixtures, `.development/27.3.2/` candidate output.
- **Spec.** Extend the 26.4.2 comparator so it also accepts the nonuniform IDs. Per-field, per-row `err/bound` and discrimination summary. Python and Rust must agree on failing rows (25.7.1 precedent).
- **Constraints.** No hardcoded numeric limits; every limit from `criteria.json`. The existing 26.4.2 tests must pass unchanged.
- **Acceptance.**
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_compare_vapor_heat_boundary` passes.
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_vapor_reference` still passes.

### 27.4.2 — Independent balance checks for nonuniform cases

- **Repo:** hyd.rs. **Depends on:** 27.2.2.
- **Files in scope:**
  - *Can create:* `crates/hydrus-core/tests/vapor_nonuniform_balance.rs`.
  - *Read only:* `crates/hydrus-core/src/heat/vapor/balance.rs`, `crates/hydrus-core/tests/vapor_heat_boundary_balance.rs`.
  - Do not touch `balance.rs` or any `src/` file.
- **Spec.** For each nonuniform case, check the discrete heat and water balance residual per step against the bounds already in `criteria.json` `checks` (`1e-9` relative, `1e-15` absolute floor). Use the existing balance helpers. Balance is internal consistency on the Rust side; it does not replace the fixture comparison.
- **Constraints.** Use existing helpers; no second copy of the bounds. Zero-flux balance only unless 27.1.3 proposed flux cases (then state the flux magnitude).
- **Acceptance.**
  - `cargo test -p hydrus-core --test vapor_nonuniform_balance` passes.
  - `cargo test -p hydrus-core --test vapor_heat_boundary_balance` still passes.

### 27.5.1 — Acceptance, coverage and goal handoff

- **Repo:** hyd.rs. **Depends on:** 27.1.1, 27.3.1, 27.4.1, 27.4.2, Gate S recorded.
- **Files in scope:**
  - *Must edit:* `planning/master/M6_VAPOR_COVERAGE.md` (rows for nonuniform T, `ConVh`, sink and root), `planning/GOALS.md` (G22.4 entry).
  - *Can create:* `planning/master/M6_VAPOR_G22_4_ACCEPTANCE.md`.
  - *Read only:* the three audits, STATE_27.md.
- **Spec.**
  1. For each coverage row touched by G22.4, set Status to accepted, rejected-with-trigger, invalid, or unreachable, with evidence links.
  2. Write the acceptance doc: evidence per case, exceptions (precision-limited rows, H6), the sink and root disposition from Gate S, and what is not claimed.
  3. Move G22.4 to Completed in `GOALS.md` only if every row is decided (accepted, rejected with trigger, invalid, or unreachable). Otherwise leave it Circled and name the open rows.
- **Constraints.** Docs only. No code change. Do not change the status of ADR 0021 or the G22.1 acceptance decision.
- **Acceptance.**
  - `git diff --check` clean.
  - `git diff --name-only` lists only the docs above.
  - Every G22.4-related row in `M6_VAPOR_COVERAGE.md` has a non-empty Status.

---

## Planning review

Self-review, two axes, inline (no sub-agents requested in the orchestration 26 pattern; this plan has no agent request). Findings fixed in this draft:

- *Spec:* all three G22.4 rows map to tasks: (a) to 27.1.3, 27.2.1, 27.2.2, 27.3.2, 27.4.1, 27.4.2; (b) to 27.1.2 and 27.3.1; (c) to 27.1.1 and Gate S. Sink implementation is explicitly out of scope (D11).
- *Standards:* the wave table's disjointness holds. Wave 3: 27.3.1 touches the helper's files (named by the audit), 27.3.2 creates a new test file. Wave 4: 27.4.1 edits the 26.4.2 comparator, 27.4.2 creates a new crate test. No shared registry file appears in two tasks. The producer (`tools/run_vapor_reference.py`) is edited only by 27.2.1.
- *Open:* 27.3.1's file list is not known until 27.1.2 reports. The plan makes that explicit rather than guessing. If 27.1.2 reports `not found`, 27.3.1 closes with no code change.
- *Open:* D11 (sink implementation goes to orchestration 28) is a judgment call. Confirm at approval.

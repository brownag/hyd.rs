# Orchestration 29 — G22.2 implementation: closed prescribed-flux pair with vapor on

Status: **Approved by user 2026-11-14.** Execution starts with wave 0.
Repo: `/home/andrew/AI/hyd.rs`, branch `main`. Baseline: orchestration 28 closing commit `8ff70e2` (Rust 620 passed / 0 failed / 4 ignored; Python 479 OK; fmt and clippy clean).
Goal source: [GOALS.md](GOALS.md) G22.2 (Circled). Record: [M6_VAPOR_G22_2_ACCEPTANCE.md](master/M6_VAPOR_G22_2_ACCEPTANCE.md). Coverage: [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md) row 22.
Audits used: [M6_VAPOR_WATER_FLUX_AUDIT.md](master/M6_VAPOR_WATER_FLUX_AUDIT.md) (primary), [M6_VAPOR_FREE_DRAINAGE_AUDIT.md](master/M6_VAPOR_FREE_DRAINAGE_AUDIT.md) (`ConB` coupling only).

## Goal & scope

**Goal.** Implement vapor-on support for the closed pair: upper prescribed flux (`KodTop=-1`, `rTop`) with lower prescribed flux (`KodBot=-1`, `rBot`), with vapor boundary and storage accounting, a provenance-backed Fortran fixture, and a balance check. This is the first accepted form from GATE W1 (orchestration 28).

**Verbatim exit (user, 2026-11-13, carried from orchestration 28):** "at least one accepted form has a provenance-backed fixture and balance check; recorded in planning/master/M6_VAPOR_COVERAGE.md."

**In scope**
- Closed flux/flux pair with vapor on, only.
- Vapor-aware boundary rows at both ends, matching the Fortran terms: `WATFLOW.FOR:211` (`ConVh` into `ConB`), `:221-222` (`rBot` in `PB`), `:224-229` (`ConTB`, `ThV` storage), `:276` and `:287-294` (`ConA`, `PT`, `PT=PT-rTop`), `:315-334` (Neumann at both ends).
- Lifting the rejection `vapor/mod.rs:213-217` (lower) and `:219-225` (upper) for this pair only, guarded by a typed check that accepts exactly this pair and rejects every other lower or upper form as today.
- One provenance-backed fixture pair (flux/flux, vapor on, plus its vapor-off control), and a column balance check.

**Out of scope (each stays rejected at its current site, with its trigger)**
- Free drainage (`FreeD`), flux/`FreeD`, head/`FreeD`. Deferred: the Fortran `ConB`/`rBot` coupling is internally inconsistent with vapor on (G7, "not established"); a design decision is needed first.
- Single lower prescribed flux with a fixed-head upper, flux/`FixedHead` beyond the closed pair. Deferred to a later orchestration.
- `qGWLF`, `TopInF`/`BotInF` (±3), seepage face, water table, atmospheric (G22.3).
- Changes to `criteria.json`, existing fixtures, the pinned Fortran source, ADR 0021 status, G22.1 and G22.4 decisions, the SD register.
- Any new tolerance without user approval (see decision D5).

## Decision log (draft)

| # | Decision | Source |
|---|---|---|
| D1 | Closed flux/flux pair only. Free drainage is deferred. | User, GATE W1 and 2026-11-14 answer ("closed flux/flux pair") |
| D2 | Provenance is the pinned Fortran reference build, following `reference/vapor_fortran/` (decks in `cases/`, fixtures in `fixtures/`, producer `tools/run_vapor_reference.py`, comparator `tools/compare_vapor_reference.py`). | User, 2026-11-14 answer; `reference/fortran_reference/README.md` (build provenance only, not a fixture by itself) |
| D3 | Workers are Sonnet, at most two in parallel, with the escalation ladder below. Planning-only commits are `docs(planning):`; code commits are focused Conventional Commits, made by the orchestrator after verification. | Carried from orchestration 28 |
| D4 | Serial gate at each wave end (five suites, then `git diff --name-only` against permitted paths). | Carried from orchestration 28 |
| D5 | Any new case, tolerance or criteria entry in `reference/vapor_fortran/criteria.json` needs explicit user approval before it is written. Default: reuse existing criteria if they apply. | Plan choice; criteria are governed by campaign records (AGENTS.md) |
| D6 | The vapor rejection is relaxed only for the closed pair. A typed guard keeps every other lower or upper mode rejected. No silent liquid-only approximation (ADR 0021, H1). | Audit `M6_VAPOR_WATER_FLUX_AUDIT.md`; ADR 0021 (Proposed) |
| D7 | Heat boundary and sink terms stay separate from the water path (H2). Each vapor term added to a flux row cites its Fortran line. | Hazard H2 |
| D8 | Gate evidence is recorded in `STATE_29.md` and the fixture manifest. No result is claimed until the orchestrator has run the suites. | Carried from orchestration 28 |

## Escalation ladder

1. Micro-plan off-spec twice: orchestrator writes the micro-TODO and gives it to the worker as fixed input.
2. Acceptance fails after one retry (retry = failure output fed back as `FAILED VERIFICATION: ...`): stop the worker and re-run on Opus. No third small-model attempt.
3. No completion report, or a malformed one: feedback, one retry, then Opus.
4. Out-of-scope edit needed: decomposition bug. Re-scope this plan; do not widen permissions.

## Recon facts (anchors, to be re-verified by wave 1)

- Fortran top flux: `INPUT.FOR:51-52, 104-110`; assembly `WATFLOW.FOR:272-294`.
- Fortran bottom flux: `WATFLOW.FOR:202-229`; Neumann `Gauss` `:315-322` (bottom) and `:327-334` (top).
- Rust lower: `lower_boundary.rs:17` `PrescribedFlux`, `:43-86` water-only flux row, `:105-107`, `:132`, `:160-161`.
- Rust upper: `water_solver.rs:90` `UpperBoundary::Flux`, `:114`.
- Vapor rejections: `heat/vapor/mod.rs:213-217` (lower), `:219-225` (upper). The vapor trial overwrites interior rows only (`water_solver.rs:340-368`, grep; to be re-read in wave 1). Boundary rows currently come from the water-only assembly.
- Existing vapor fixtures and tests: `reference/vapor_fortran/fixtures/` (18 files), `crates/hydrus-core/tests/vapor_heat_boundary_balance.rs`, `vapor_fortran_parity` (10 tests).

## Wave table

| Wave | Task | Title | Depends on | Gate before |
|---|---|---|---|---|
| 0 | — | Commit this plan and the orchestration 29 STATE after approval (`docs(planning):`); repeat the five suites (baseline) | — | Wave 1 |
| 1 | 29.1.1 | Fortran closed-pair deck and fixture design (read-only; reference harness and existing `cases/` recipe) | Wave 0 | — |
| 1 | 29.1.2 | Rust design for vapor-aware flux rows and the typed guard (read-only) | Wave 0 | — |
| — | **GATE 29.1 (user)** | Approve fixture design, deck choice, and the guard shape. Required before any code or fixture | 29.1.1, 29.1.2 | Wave 2 |
| 2 | 29.2.1 | Vapor-aware flux row builders, lower and upper, behind the typed guard | GATE 29.1 | — |
| 2 | 29.2.2 | Lift the rejection for the closed pair only; keep all other forms rejected (tests) | GATE 29.1 | — |
| 3 | 29.3.1 | Column balance check for the closed pair (vapor on) | 29.2.1, 29.2.2 | — |
| 3 | 29.3.2 | Deck and fixture generation with the reference build; comparison against the Rust run | 29.2.1, 29.2.2 | — |
| — | **GATE 29.2 (user)** | Accept or reject the fixture and balance evidence against the exit criterion; record in the coverage matrix | 29.3.1, 29.3.2 | Close |

Max two workers in parallel. Waves 2 and 3 touch disjoint files (see task specs); 29.2.1 and 29.2.2 both edit `water_solver.rs` and `vapor/mod.rs`, so they are sequenced, not parallel.

## Gates

- **Wave 0 baseline.** Five suites green at the planning HEAD.
- **Micro-plan review (orchestrator).** In-scope files only, acceptance mapped, at most 12 steps, claims spot-checked against source.
- **Wave end (serial, orchestrator).** `cargo build --workspace --locked`; `cargo test --workspace --no-fail-fast`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `python3 -m unittest discover -s tools -p 'test_*.py'`; then `git diff --name-only` against the wave's permitted paths.
- **GATE 29.1 (user).** Present the fixture design, the deck choice, the guard shape, and the list of Fortran lines each vapor term cites. The user approves or revises. No code before approval.
- **GATE 29.2 (user).** Present the fixture verdicts, the balance result, and the exact exit-criterion mapping. The user decides whether the exit criterion is met.

## Task specs (draft; full specs are written after GATE 29.1 design is approved)

### 29.1.1 — Fortran closed-pair deck and fixture design (read-only)
- Files in scope: read `reference/vapor_fortran/README.md`, `reference/vapor_fortran/cases/` (existing deck format), `tools/run_vapor_reference.py` (grep for the case list), `reference/vapor_fortran/criteria.json` (read only, do not edit), `src_reference/INPUT.FOR` (lines 40–115), `src_reference/WATFLOW.FOR` (anchors above). Can create: `planning/master/M6_VAPOR_CLOSED_PAIR_FIXTURE_DESIGN.md`, `planning/tasks/29.1.1.*`.
- Spec: propose a deck that exercises `KodTop=-1` and `KodBot=-1` with vapor on and a closed or known-flux column. Name the existing criteria it reuses (D5). Name the vapor-off control. Cite every Fortran line the fixture depends on.
- Acceptance: design doc exists; it names decks, fields, and reused criteria; `git diff --name-only` shows no changes outside the scope.

### 29.1.2 — Rust design for vapor-aware flux rows and the typed guard (read-only)
- Files in scope: read `crates/hydrus-core/src/lower_boundary.rs` (lines 11–135, 160–235), `crates/hydrus-core/src/water_solver.rs` (grep `UpperBoundary::Flux`, `PrescribedFlux`, `assemble_water_rows`, `vapor trial`), `crates/hydrus-core/src/heat/vapor/mod.rs` (lines 205–230). Can create: `planning/master/M6_VAPOR_CLOSED_PAIR_ROW_DESIGN.md`, `planning/tasks/29.1.2.*`.
- Spec: design the vapor-aware flux row builders (lower: `ConVh` into the face conductance, `ConTB`, `ThV` storage; upper: `ConVh` into `ConA`, `ConTA`, `ThV` into `PT`). Design the typed guard that accepts exactly the closed pair. List every file and function to change.
- Acceptance: design doc exists with each term cited to `WATFLOW.FOR`; `git diff --name-only` shows no changes outside the scope.

### 29.2.1 — Vapor-aware flux row builders, lower and upper, with geometric averaging (sequenced before 29.2.2)
- **Files in scope.** *Must edit:* `crates/hydrus-core/src/heat/vapor/water.rs`, `crates/hydrus-core/src/water_solver.rs` (vapor trial only, near `:340-368`). *Can create:* `planning/tasks/29.2.1.*`. *Read only:* `planning/master/M6_VAPOR_CLOSED_PAIR_ROW_DESIGN.md`, `src_reference/WATFLOW.FOR:202-229, 272-294, 315-334` (read-only), `heat/vapor/water.rs`, `lower_boundary.rs:43-96, 219-235`. *Not in scope:* `vapor/mod.rs` (wave 2, 29.2.2), `criteria.json`, fixtures, `src_reference/` edits.
- **Spec.** (a) Add `vapor_lower_flux_row` and `vapor_upper_flux_row` in `water.rs`, per the design section 4, with one unit test per vapor term (`ConVh` in the face, `ConTB`/`ConTA`, `ThV` storage, and the `rTop`/`rBot` sign). (b) Add geometric face averaging for the vapor builders (user decision, GATE 29.1, item A), matching `WATFLOW.FOR:203` and `:274`, with a test. (c) In the vapor trial, when the closed-pair classification is `ClosedFlux`, build both rows and write `workspace[0]` and `workspace[n-1]` through `transform_boundary_row` with Neumann flux. Head/head and all other paths are unchanged. Until 29.2.2 lands the guard, the call site must be gated behind a `ClosedFlux` input that no current caller produces, so behaviour is unchanged.
- **Open items to settle in this task (GATE 29.1 items B and D).** B: confirm the sign of `rTop` against `WATFLOW.FOR:294`; the Rust upper flux is `-rTop`. D: confirm from `transform_boundary_row` that the flux is applied once (in the Neumann rhs), not also in the row.
- **Acceptance.** `cargo test -p hydrus-core --lib heat::vapor::water` passes with the new unit tests; `cargo test -p hydrus-core --test vapor_water_rows` passes (existing 4 tests, unchanged). `git diff --name-only` lists only the two must-edit files and planning paths.

### 29.2.2 — Typed guard and lift of the closed-pair rejection (after 29.2.1)
- **Files in scope.** *Must edit:* `crates/hydrus-core/src/heat/vapor/mod.rs` (replace the two match blocks, `:211-226`), `crates/hydrus-core/src/water_solver.rs` (consume the classification; same file as 29.2.1, so run after it). *Can create:* new test file(s) under `crates/hydrus-core/tests/` named in the micro-plan; `planning/tasks/29.2.2.*`. *Not in scope:* `lower_boundary.rs`, `criteria.json`, fixtures.
- **Spec.** Add `VaporBoundaryPair { HeadHead, ClosedFlux { lower_flux, upper_flux } }` and `classify` per design section 6. Rules: `(FixedHead, Head)` keeps today's checks; `(PrescribedFlux, Flux)` gives `ClosedFlux` after finite checks; every other combination returns the existing `Unsupported*` error. Use an exhaustive match with no wildcard. Assert lDensity, iDualPor and lWTDep off on the vapor path (GATE 29.1 item C). The caller's `Flux.row` is ignored on the vapor path, and a test proves it.
- **Acceptance.** New guard tests: every non-pair combination still returns its `Unsupported*` error; head/head and closed-pair classify correctly; `Flux.row` ignored. `cargo test -p hydrus-core --test vapor_heat_transport`, `--test vapor_state` and `--test vapor_driver_order` pass unchanged. `git diff --name-only` lists only the must-edit files and planning paths.

### 29.3.1, 29.3.2 — Balance check, and deck and fixture generation (wave 3)
Specs are written after wave 2 closes. Wave 3 includes the runner and comparator whitelist amendment and the nonzero-flux deck (approved, GATE 29.1). Criteria entries are decided at GATE 29.2 (D5).

## Notes

- This draft cites line numbers from the orchestration 28 audits. Wave 1 re-verifies each one before it is used.
- `.development/reference-build/` is untracked build output. Staged source is a copy, never `src_reference/`.

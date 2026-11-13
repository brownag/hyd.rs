# Orchestration 26 — G22.1 heat top/bottom boundary rows with vapor

Status: **Executed; orchestration closed 2026-11-13.** Waves 1–6 done (final gate `94967be`). G22.1 stays Circled: FD (`kTopT=1`, `kBotT=1` vs flux) is held with the vapor-on head drift undecided, and FF's acceptance is conditional on the same disclosure. The D-26.1 drift is recorded in [STATE_26.md](STATE_26.md).
Created 2026-11-12. Repo: `/home/andrew/AI/hyd.rs` (branch `feat/heat1`, HEAD `237642f`).
Resume: [STATE_26.md](STATE_26.md). Worker: [PROMPT_TEMPLATE_26.md](PROMPT_TEMPLATE_26.md).
Goal source: [GOALS.md](GOALS.md) circled goal **G22.1**. Coverage: [M6_VAPOR_COVERAGE.md](master/M6_VAPOR_COVERAGE.md).
Prior orchestration: [EXECUTION_PLAN_25.md](EXECUTION_PLAN_25.md) (G19, complete with exception).

## Goal & Scope

**Goal.** Audit the heat top and bottom boundary forms that the vapor path
currently rejects, and for each form either accept it with evidence or reject it
with a named trigger. Forms in scope: top flux, bottom flux, zero-gradient
bottom, and every supported top/bottom pair with vapor. Dirichlet/Dirichlet is
already accepted (G19) and is kept unchanged.

**In scope (user decisions 2026-11-12):**
- Audit of the integer-code mapping for `kTopT` and `kBotT` (source and Rust).
- Zero-gradient bottom (`kBotT == 0`) is included.
- All remaining top/bottom pairs: top Dirichlet or flux, bottom Dirichlet, flux or
  zero-gradient. Pairs are DD (accepted), DF, DZ, FD, FF, FZ (where F = flux,
  D = Dirichlet, Z = zero-gradient; the first letter is the top, the second the bottom).
  Sinusoidal top is excluded.
- Water boundary fixed at both ends (fixed heads), no sink, no root uptake.
- Each new row ends in one of: accepted (fixture + Rust-vs-Fortran comparison +
  independent balance check), or rejected with a trigger.

**Out of scope:**
- Water flux, free drainage and mixed water pairs (G22.2).
- Atmospheric water switching (G22.3) and energy balance `lEnBal`/`UpdateEnergy`/`Evap`/`Rn`/`Rns`/`Rnl` (G20).
- Campbell (`iCampbell=1`, SD-006 policy), enhancement off (`iEnhanc=0`), nonuniform-temperature coverage, `ConVh` reachability, nonzero sink and root uptake (G22.4–G22.6).
- Temperature-dependent liquid water `lWDep`/`lWTDep` (G23).
- Sinusoidal top temperature.
- Any change to pinned Fortran source, patches, full-reference output formats, existing evidence, `criteria.json` v2 values for existing fields, or existing fixtures (must stay byte-identical).
- New dependencies, solute changes, M5 campaign or holdout work.

**Verbatim constraints (user, 2026-11-12):**
- "Audit + contract + evidence" for the boundary rows.
- "Include it" for the zero-gradient bottom.
- "All remaining pairs" for the pairs.
- "Fixed heads only" for the water boundary.
- "Same as 25" for workers (openai/gpt-6-luna, at most two parallel worktrees, mid-tier escalation after one failed retry).
- "Yes, repeat the gate" for numeric criteria: approve tolerances after reference-only evidence and before any candidate comparison; freeze them; no widening without explicit approval.

## Decision log (dated 2026-11-12)

| # | Decision | Source |
|---|---|---|
| D1 | Depth: audit + contract + evidence. Each row accepted with evidence or rejected with a trigger. | User, Q1 |
| D2 | Zero-gradient bottom (`kBotT==0`) is in scope. | User, Q2 |
| D3 | All remaining top/bottom pairs are in scope (DF, DZ, FD, FF, FZ). Sinusoidal top excluded. | User, Q3 |
| D4 | Water BC fixed heads at both ends for every new fixture. | User, Q4 |
| D5 | Workers: openai/gpt-6-luna, max two parallel worktrees, absolute target/run roots. One retry, then mid-tier. | User, Q5 |
| D6 | Numeric gate repeated: criteria proposed from reference-only evidence, user approves before Wave 4, then frozen in `reference/vapor_fortran/criteria.json` (new version). | User, Q6 |
| D7 | ADR 0021 (Proposed) is complied with, not superseded. This plan does not change its status. | Recon: no contradiction |
| D8 | Reuse the pinned reference executable from orchestration 25.2.2 (`824179ab…`, read-only) if its `.development/` build is present; otherwise rebuild with the harness. | Recon, STATE_25 |
| D9 | Source precedence: `TEMPER.FOR`/`TempAdj` is authoritative. Rust maps any negative `kTopT`/`kBotT` to flux (`heat/config.rs:162,172`); the audit decides whether that is faithful. | Recon |

## Hazard notes from recon (for workers)

- **H1 top flux vapor term is code-specific.** `TempAdj` adds the top vapor
  heat-flux term only when `kTopT .eq. -1` (`TEMPER.FOR:249-255`). The energy
  flux `HeatFl` in `Temper` applies to any `kTopT < 0` when `.not. lEnBal`
  (`TEMPER.FOR:25-32`). If another negative code is reachable, Rust's
  all-negative-is-flux mapping is wrong for vapor.
- **H2 bottom zero-gradient has no TempAdj bottom branch.** The bottom vapor term
  exists only for `kBotT < 0` (`TEMPER.FOR:220-232`). Confirm that `kBotT == 0` adds
  nothing and that the Rust vapor path matches that.
- **H3 `kTopT == 0` is unassigned in source** (`SetUpHeat` leaves node N
  unassigned, `heat/config.rs:26`). Rust rejects it (`heat/boundary.rs:98-99`).
  Keep rejecting it; do not map it to flux.
- **H4 Rust heat config defaults.** `heat/config.rs:162` and `:172` map `_` to
  `Flux`. Any new vapor-selection branch must not reuse that default silently.
- **H5 Existing vapor selection rejects** top/bottom forms with
  `VaporSelectionNotSupported` (`heat/config.rs:179-182`) and typed reasons in
  `crates/hydrus-core/src/heat/vapor/mod.rs`. Tests live in
  `tests/vapor_config_contract.rs` (389 lines).
- **H6 Existing fixtures must stay byte-identical.** The producer
  `tools/run_vapor_reference.py` writes `reference/vapor_fortran/fixtures/*.json`.
  New cases get new IDs only.
- **H7 Source precision.** Vapor REAL constants are single precision (see
  [M6_VAPOR_PRECISION_DIAGNOSIS.md](master/M6_VAPOR_PRECISION_DIAGNOSIS.md)).
  Temperature-dependent latent heat can distort transients; the 18 isothermal
  rows are already explained. Do not widen existing criteria to hide new
  discrepancies. Record any new precision-limited rows as explained.
- **H8 Campbell is not in scope.** `TEMPER.FOR:300` carries the SD-006 factor. Do not
  touch `TempCap`. Fixtures use the default thermal conductivity.
- **H9 Disjointness.** `heat/config.rs` and `heat/transport.rs` are separate
  files, so tasks 26.3.1 and 26.3.2 are disjoint.

## Wave table

| Wave | Task | Title | Depends on | Gate before |
|---|---|---|---|---|
| 1 | 26.1.1 | Heat-boundary code audit (source and Rust) | — | — |
| 2 | 26.2.1 | Vapor heat-boundary case specs and producer | 26.1.1 | — |
| 2 | 26.2.2 | Reference-only fixture run | 26.2.1 | — |
| — | **GATE N1** | User approves numeric criteria v3 (reference-only evidence) | 26.2.2 | Wave 4 |
| 3 | 26.3.1 | Vapor config accepts audited forms, rejects others | 26.1.1 | — |
| 3 | 26.3.2 | TempAdj and top/bottom vapor heat terms | 26.1.1 | — |
| 4 | 26.4.1 | Rust candidate runs and parity test | 26.2.2, 26.3.1, 26.3.2, GATE N1 | — |
| 4 | 26.4.2 | Python comparator against criteria v3 | 26.2.2, GATE N1 | — |
| 5 | 26.5.1 | Independent balance checks for accepted forms | 26.3.2 | — |
| 6 | 26.6.1 | Acceptance, coverage and goal handoff | 26.4.1, 26.4.2, 26.5.1 | — |

Wave 3 is sequenced after Wave 2 even though it has no file overlap with it, so
that the config decisions use the audit and fixture reality together. Max two
workers at any time; serial full-suite gates at the end of every wave.

## Wave gates

- **Pre-wave-1 baseline.** Repeat the five Quick Start suites on `237642f`
  (STATE_25 baseline was on `a8fc200`). Record the result in STATE_26.md. If it
  differs from 529/389, stop and report.
- **Each wave end (orchestrator, serial):** `cargo build --workspace --locked`;
  `cargo test --workspace --no-fail-fast`; `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets -- -D warnings`;
  `python3 -m unittest discover -s tools -p 'test_*.py'` (use a 600 s timeout).
- **GATE N1 (user).** Before Wave 4, the orchestrator proposes criteria v3 from
  the Wave 2 reference-only fixtures: per-field tolerances, print rules, zero
  floor, discrimination (vapor on/off ratio for each new form). The user
  approves the numbers. Only then are the criteria written to
  `reference/vapor_fortran/criteria.json` and frozen with a contract revision.
  No candidate comparison runs before this.

---

## Task specs

### 26.1.1 — Heat-boundary code audit (source and Rust)

- **Repo:** hyd.rs. **Depends on:** none.
- **Files in scope:**
  - *Must edit:* none.
  - *Can create:* `planning/master/M6_VAPOR_HEAT_BOUNDARY_AUDIT.md`.
  - *Read only:* `src_reference/INPUT.FOR`, `src_reference/TEMPER.FOR`,
    `src_reference/HYDRUS.FOR`, `crates/hydrus-core/src/heat/config.rs`,
    `crates/hydrus-core/src/heat/boundary.rs`, `crates/hydrus-core/src/heat/vapor/mod.rs`,
    `crates/hydrus-core/tests/vapor_config_contract.rs`.
  - Scratch: `.development/26.1.1/`.
- **Spec.** Produce the audit that everything else depends on.
  1. Find every reader and assignment of `kTopT` and `kBotT` in `INPUT.FOR`
     (`grep -n "kTopT\|kBotT" src_reference/INPUT.FOR`). Record the reachable
     integer codes and the range the input accepts.
  2. For each code, state which source branch runs: `SetUpHeat` (`TEMPER.FOR:77-160`),
     `Temper` upper flux (`:25-32`), `TempAdj` (`:180-265`, vapor terms at `:249-255`).
  3. Answer, with line citations: (a) is `kTopT == -1` the only top code with a
     vapor storage term? (b) do any other negative `kTopT`/`kBotT` codes exist
     in reachable input? (c) does `kBotT == 0` add any vapor term? (d) what does
     `kTopT == 0` do?
  4. Map Rust behaviour per code: `heat/config.rs:150-185`, `heat/boundary.rs:31-33,98-110`,
     and the vapor rejection path in `heat/vapor/mod.rs`. Record whether Rust
     accepts, rejects or silently maps each code.
  5. Classify each form: DD (accepted), DF, DZ, FD, FF, FZ, invalid, or unreachable.
     Each classification gets a proposed disposition: *candidate for acceptance*
     or *reject with trigger*. No code change.
  6. Write the audit as a table with columns: code, source branch with lines, Rust
     mapping with lines, vapor term present (y/n), disposition.
  7. Record the hazards H1–H4 as confirmed or refuted with evidence.
- **Constraints.** Do not edit any code, test or fixture. Do not run the Fortran
  executable. Do not change `GOALS.md` or `M6_VAPOR_COVERAGE.md` (26.6.1 owns them).
  Ask in the audit's Questions section if a code is ambiguous, not by guessing.
- **Anchors.** `grep -n "kTopT\|kBotT\|tTopA" src_reference/INPUT.FOR`;
  `TEMPER.FOR:25-32,35-55,77-100,144-160,180-265`; `heat/config.rs:26,162,172,179-182`;
  `heat/boundary.rs:31-33,98-110`.
- **Reference.** [M6_VAPOR_CONTRACT.md](master/M6_VAPOR_CONTRACT.md) "Effective switches" and "Heat" rows.
- **Acceptance.**
  - `planning/master/M6_VAPOR_HEAT_BOUNDARY_AUDIT.md` exists; it contains a row for
    `kTopT` and for `kBotT` and each form DD, DF, DZ, FD, FF, FZ.
  - `grep -c "TEMPER.FOR" planning/master/M6_VAPOR_HEAT_BOUNDARY_AUDIT.md` ≥ 5.
  - `git diff --name-only` shows only the audit file.
  - Targeted: `cargo test -p hydrus-core --test vapor_config_contract --test heat_config_contract` passes unchanged.

### 26.2.1 — Vapor heat-boundary case specs and producer

- **Repo:** hyd.rs. **Depends on:** 26.1.1 (uses the forms the audit marks as candidates).
- **Files in scope:**
  - *Must edit:* `tools/run_vapor_reference.py`, `tools/test_run_vapor_reference.py`.
  - *Can create:* `reference/vapor_fortran/cases/<new-case-id>/` directories.
  - *Read only:* existing cases under `reference/vapor_fortran/cases/`,
    `reference/vapor_fortran/README.md`, `planning/master/M6_VAPOR_THERMAL_MATERIAL_RESEARCH.md`.
  - Scratch: `.development/26.2.1/`.
- **Spec.** Add one case per candidate form from the audit, using the existing dry
  thermal deck style from orchestration 25 (fixed water heads at both ends, default
  thermal conductivity, `lWDep=false`, enhancement on, 41 nodes, kg/cm/hour units).
  - For each candidate form, a vapor-on and a vapor-off control, the same as the
    25.2.2 pattern. Use a stable ID scheme, e.g. `vapor-heatbc-<pair>-on` /
    `-off`.
  - Extend the producer's case list so `--all` covers the new cases. Keep the
    existing case IDs and their outputs untouched.
  - Add producer tests: deck text for one new case; `--all` lists the new IDs;
    existing IDs are unchanged.
  1. Read the producer case schema (`run_vapor_reference.py:15-35`, `:171-194`).
  2. Read one existing case spec (`reference/vapor_fortran/cases/`).
  3. Add spec files for each candidate form.
  4. Extend the producer's list and validation.
  5. Add the producer tests.
  6. Run the targeted tests.
- **Constraints.** Do not regenerate or edit existing fixtures. Do not change the
  executable or pinned source. Do not add water-side variation (H4/D4). Do not add
  Campbell cases (H8).
- **Anchors.** `tools/run_vapor_reference.py:15,20-35,97-107,171-194`; the 25.2.2
  case directory naming under `reference/vapor_fortran/cases/`.
- **Reference implementation.** Orchestration 25's 25.6.3 dry-soil producer
  (`git show b8e0544 -- tools/run_vapor_reference.py reference/vapor_fortran/cases`).
- **Acceptance.**
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_vapor_reference` passes.
  - `git diff --name-only` shows only the producer, its test and new case directories.
  - `git diff --stat -- reference/vapor_fortran/fixtures` is empty.

### 26.2.2 — Reference-only fixture run

- **Repo:** hyd.rs. **Depends on:** 26.2.1.
- **Files in scope:**
  - *Can create:* `reference/vapor_fortran/fixtures/<new-case-id>.json` only.
  - *Read only:* the pinned reference build (D8). Do not edit `src_reference/`.
  - Scratch: `.development/26.2.2/`.
- **Spec.**
  1. Locate the pinned executable from orchestration 25.2.2 by SHA-256 `824179ab…`
     under `.development/25.2.2/`. If absent, rebuild with the harness in
     `reference/fortran_reference/README.md` into `.development/26.2.2/build/`.
     Record the path and hash.
  2. Run the producer for the new case IDs only, with absolute run and build roots.
  3. Verify each fixture: finite values, output times match, vapor
     reachability matches the case, no `Error.msg`.
  4. Record fixture SHA-256s and the discrimination reference (vapor on vs off)
     per field in STATE_26.md. Do not compare to Rust.
- **Constraints.** Reference-only evidence class. No Rust comparison, no tolerance
  claims, no criteria approval. Existing fixtures byte-identical.
- **Anchors.** `tools/run_vapor_reference.py:97-107,171-194`;
  `reference/fortran_reference/README.md`.
- **Acceptance.**
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_vapor_reference` passes.
  - `git diff --name-only` shows only new fixture JSON files.
  - `git diff --stat -- reference/vapor_fortran/fixtures` shows additions only.
  - The orchestrator independently reruns `--case <one-id>` and checks the SHA-256 matches.

### 26.3.1 — Vapor config accepts audited forms, rejects others

- **Repo:** hyd.rs. **Depends on:** 26.1.1. Runs in parallel with 26.3.2.
- **Files in scope:**
  - *Must edit:* `crates/hydrus-core/src/heat/config.rs` (vapor-selection branch only, around `:150-185`), `crates/hydrus-core/src/heat/vapor/mod.rs` (validation and typed reasons).
  - *Must edit (tests):* `crates/hydrus-core/tests/vapor_config_contract.rs`.
  - *Can create:* `crates/hydrus-core/tests/vapor_heat_boundary_config.rs`.
  - Do not touch `heat/transport.rs`, `heat/vapor/adjust.rs` or any file under `tools/`.
- **Spec.** Make the vapor-aware selection accept exactly the forms the audit marks
  accepted in Wave 4 and reject the rest with typed reasons. Start with the
  candidate set from the audit; the final accepted set is decided at Wave 6, so
  keep the accepted set in one named constant that the later task can change.
  1. Read the closed selection in `heat/vapor/mod.rs` and the vapor-selection
     branch in `heat/config.rs:176-183`.
  2. Replace the blanket `VaporSelectionNotSupported` for the candidate forms with
     acceptance; keep the rejection for every other form.
  3. Keep `kTopT == 0` rejected (H3). Do not let `_` default to flux in the vapor
     path (H4).
  4. Update `vapor_config_contract.rs` for the new accepted and rejected forms.
  5. Add `vapor_heat_boundary_config.rs` for each accepted form, including the
     invalid code cases.
  6. Run the targeted tests.
- **Constraints.** Do not weaken the existing rejection of Campbell, enhancement
  off, sinusoidal top, nonzero sink, water flux or atmospheric selectors. Heat-only
  entry points must keep their existing behaviour (ADR 0021, contract "Heat-only
  unchanged"). No new dependencies.
- **Anchors.** `heat/config.rs:26,60,80-93,150-185`; `heat/vapor/mod.rs:51-75,139-150`
  (validate_water_input); `tests/vapor_config_contract.rs` (389 lines; grep test names first).
- **Reference implementation.** The 25.1.1 selection work
  (`git show 001601d -- crates/hydrus-core/src/heat/vapor/mod.rs`).
- **Acceptance.**
  - `cargo test -p hydrus-core --test vapor_config_contract` passes.
  - `cargo test -p hydrus-core --test vapor_heat_boundary_config` passes.
  - `cargo test -p hydrus-core --test heat_config_contract` passes unchanged.
  - `cargo fmt --all -- --check` passes on the touched files.

### 26.3.2 — TempAdj and top/bottom vapor heat terms

- **Repo:** hyd.rs. **Depends on:** 26.1.1. Runs in parallel with 26.3.1.
- **Files in scope:**
  - *Must edit:* `crates/hydrus-core/src/heat/vapor/adjust.rs`, `crates/hydrus-core/src/heat/transport.rs` (vapor-flux branch only).
  - *Must edit (tests):* `crates/hydrus-core/tests/vapor_heat_transport.rs`.
  - Do not touch `heat/config.rs`, `heat/vapor/mod.rs` or any file under `tools/`.
- **Spec.** Port the vapor heat terms that the audit classified as present, for
  the candidate forms only.
  1. Read `TEMPER.FOR:25-32` (top energy flux `HeatFl` with vapor enthalpy) and
     `TEMPER.FOR:180-265` (`TempAdj`).
  2. In `adjust.rs`, add the bottom `kBotT < 0` vapor branch and the top
     `kTopT < 0` branch only where the audit says the source has them. Use the
     source-faithful code (`kTopT == -1` as the only top storage-term code, per
     H1). Zero-gradient bottom adds nothing (H2).
  3. In `transport.rs`, add the top vapor energy flux term to the flux-top heat
     row exactly as `HeatFl` does, for `kTopT < 0` and `.not. lEnBal`.
  4. Keep the two-level structure (old and new level), operation order, and
     precision faithful to source (ADR 0010, contract "Heat").
  5. Add tests: hand-derived single-node term check for each form, and the
     heat-only path still rejects vapor.
  6. Run the targeted tests.
- **Constraints.** Preserve source operation order. Do not change the Dirichlet
  rows that G19 accepted. Do not touch the energy-balance terms (G20).
- **Anchors.** `TEMPER.FOR:25-32` (HeatFl), `:35-55` (TempAdj call), `:180-265`
  (TempAdj body); `heat/vapor/adjust.rs:31` (`adjust_heat_rows`), `:150`
  (`vapor_velocity_gradient`); `heat/transport.rs` (search for the flux-top row).
- **Reference implementation.** 25.3.2 (`git show 1790b62`), the
  `TempAdj` port for Dirichlet rows.
- **Acceptance.**
  - `cargo test -p hydrus-core --test vapor_heat_transport` passes.
  - `cargo test -p hydrus-core --lib heat::transport` passes.
  - `cargo test -p hydrus-core --test heat_driver_order --test vapor_driver_order` passes.

### 26.4.1 — Rust candidate runs and parity test

- **Repo:** hyd.rs. **Depends on:** 26.2.2, 26.3.1, 26.3.2, GATE N1 approved.
- **Files in scope:**
  - *Can create:* `crates/hydrus-core/tests/vapor_heat_boundary_parity.rs`.
  - *Can create:* candidate output under `.development/26.4.1/` only.
  - *Read only:* the new fixtures from 26.2.2, `reference/vapor_fortran/criteria.json` (v3).
- **Spec.**
  1. Mirror `vapor_fortran_parity.rs` for the new cases: build the same deck inputs
     from the case spec, run the candidate, and write candidate JSON with
     `requests_sha256` and the evidence class `candidate-rust`.
  2. Assert the frozen v3 criteria only. Do not read any hardcoded limit.
  3. Report the failing set separately (rows and fields) rather than widening.
- **Constraints.** Do not edit `criteria.json`. Do not edit the fixtures. Do not
  change `vapor_fortran_parity.rs`.
- **Anchors.** `crates/hydrus-core/tests/vapor_fortran_parity.rs` (25.7.1 pattern);
  `reference/vapor_fortran/criteria.json`.
- **Reference implementation.** `git show 95ee096 -- crates/hydrus-core/tests/vapor_fortran_parity.rs`.
- **Acceptance.**
  - `cargo test -p hydrus-core --test vapor_heat_boundary_parity` passes, or fails only on rows listed in its explicit known-failure set, which the worker reports verbatim.
  - The candidate JSON has a `requests_sha256` that matches the fixtures.

### 26.4.2 — Python comparator against criteria v3

- **Repo:** hyd.rs. **Depends on:** 26.2.2, GATE N1 approved. Runs in parallel with 26.4.1.
- **Files in scope:**
  - *Can create:* `tools/compare_vapor_heat_boundary.py`, `tools/test_compare_vapor_heat_boundary.py`.
  - *Read only:* `reference/vapor_fortran/criteria.json` (v3), the new fixtures, candidate output under `.development/26.4.1/` when present.
- **Spec.** A standalone comparator, the same shape as `tools/compare_vapor_*`
  from 25.7.2. It reads the frozen v3 criteria and outputs per-field,
  per-row verdicts with `err/bound` and a discrimination summary. Python and Rust
  must agree on failing rows (25.7.1 precedent).
- **Constraints.** No hardcoded numeric limits; every limit comes from `criteria.json`.
  Do not edit the criteria. Do not change the comparator for 25.7.2 cases.
- **Anchors.** The 25.7.2 comparator (`git show 1a57124 --stat`).
- **Acceptance.**
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_compare_vapor_heat_boundary` passes.
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_run_vapor_reference` still passes.

### 26.5.1 — Independent balance checks for accepted forms

- **Repo:** hyd.rs. **Depends on:** 26.3.2.
- **Files in scope:**
  - *Can create:* `crates/hydrus-core/tests/vapor_heat_boundary_balance.rs`.
  - *Read only:* `crates/hydrus-core/src/heat/vapor/balance.rs`, `tests/vapor_balance.rs`.
  - Do not touch `balance.rs` or any `src/` file.
- **Spec.** For each accepted form, check the discrete heat balance residual per
  step against the bounds in `criteria.json` `checks`
  (`1e-9` relative, `1e-15` absolute floor, solver's own last-iteration
  coefficients, as approved on 2026-11-12). Report closure against fresh
  coefficients as diagnostic only.
- **Constraints.** Use the existing balance helpers; do not create a second copy
  of the bounds.
- **Anchors.** `heat/vapor/balance.rs`; `tests/vapor_balance.rs`.
- **Acceptance.**
  - `cargo test -p hydrus-core --test vapor_heat_boundary_balance` passes.
  - `cargo test -p hydrus-core --test vapor_balance` still passes.

### 26.6.1 — Acceptance, coverage and goal handoff

- **Repo:** hyd.rs. **Depends on:** 26.4.1, 26.4.2, 26.5.1.
- **Files in scope:**
  - *Must edit:* `planning/master/M6_VAPOR_COVERAGE.md`, `planning/master/M6_VAPOR_CONTRACT.md` (heat-boundary section only), `planning/GOALS.md`.
  - *Can create:* `planning/master/M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md`.
  - *Read only:* the audit, STATE_26.md.
- **Spec.**
  1. For each row (DD, DF, DZ, FD, FF, FZ, `kTopT==0`, sinusoidal top), set the
     coverage Status to accepted, rejected-with-trigger or invalid, with evidence
     links.
  2. Write the acceptance doc with evidence per form, the exception list, and the
     explicit disclosures (H7 precision, H8 Campbell out of scope).
  3. Move G22.1 to Completed in `GOALS.md` if every row is decided; otherwise leave it
     Circled with the remaining rows named.
  4. Update the contract's heat-boundary section.
- **Constraints.** Docs only. No code change. Do not change the status of ADR 0021.
- **Acceptance.**
  - `git diff --check` clean.
  - `git diff --name-only` lists only the docs above.
  - Every row in `M6_VAPOR_COVERAGE.md` has a non-empty Status.

---

## Planning review

Spec and Standards self-review applied inline, not by sub-agents (no request for
agents). Findings fixed in this draft:

- *Spec:* the water BC and sinusoidal exclusion are explicit. All six pair
  codes D1–D3 appear in the wave table.
- *Standards:* `heat/config.rs` is touched only by 26.3.1 and `heat/transport.rs`
  only by 26.3.2 (disjoint). Each task has a `Reference implementation`, anchors,
  and mechanical acceptance. 26.4.1 and 26.4.2 write disjoint files.
- *Open:* the accepted-form set in 26.3.1 is provisional until Wave 2 evidence and
  the audit are in; a judgment call the user should see at Phase 6.

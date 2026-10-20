# Orchestration 21 — Diagnose and close the transient water-flow parity gap (35/44 → 44/44)

## Goal and scope

**Goal.** Orchestration 20 stopped at 35/44 on the all-mode Rust-vs-Fortran solute parity gate (`20.6.1`, commit `22d3ef1`). The 9 remaining failures are `processes/dual_exchange` (4) and `transport_properties` (5). The 20.6.1 worker attributed them to an unported `HYSTER.FOR`. Reconnaissance (below) refutes that: every one of the 44 decks has `Hysteresis = 0`, so `Hyst`/`Hyster` are unreachable. This orchestration first **diagnoses** the real first divergence against the pinned source, with no fixes allowed. It then pauses at a user gate where fix tasks are specified from the evidence, executes those fixes, re-runs the 44-fixture gate, and only after 44/44 switches the default backend to `SoluteBackend::FortranCompatible` (absorbing Orchestration 20's `20.6.1` re-run and `20.7.1`).

**In scope:**
- First-divergence traces of `transport_properties/tortuosity_millington` and `processes/dual_exchange/model6_no_exchange` against task-local instrumented copies of the pinned Fortran (Wave 1).
- A task-local Model 0 copy of the `model6_no_exchange` deck, traced in its own task, to separate a plain `WatFlow` defect from a `DualPor` defect (Wave 1, not added to the 44-fixture gate).
- Testing hypothesis H1 (below) first in both traces.
- A user gate that turns the diagnosis reports into fix-task specs (Wave 2, plan amendment).
- The fixes themselves (Wave 3, specified at the gate).
- Re-running the 44-fixture all-mode gate and refreshing the three evidence docs (Wave 4).
- Switching the default backend after 44/44 (Wave 5, ported from `20.7.1`).
- Closing Orchestration 20: marking `20.6.1` and `20.7.1` as superseded in `STATE_20.md` (Wave 0).

**Out of scope:**
- **Porting `HYSTER.FOR` (`Hyst`, Lenhard hysteresis) and `Hyster` (`iHyst > 0`).** Deferred by the user until this orchestration completes (2026-10-20). No fixture reaches either routine.
- Adding the isolating Model 0 deck (or any new deck) to the 44-fixture parity set, or re-generating any Fortran reference output.
- Changing parity tolerances, `OBSERVATION_MAP.json`, or `tools/run_solute_parity.py` comparison logic (ADR 0006).
- Modifying `src_reference/` or the portability patch (ADR 0001, 0003).
- Any FV/BE solute or water-solver equation outside `solute_fortran_compat/` (ADR 0002).
- `Cargo.toml` / `Cargo.lock` (ADR 0011).
- `TODO.md`.

**Constraints stated by the user** (2026-10-20, verbatim):
- "do a diagnosis-first orchestration. we can come back to the hyster port once we have completed this orchestration"
- Plan structure, Orchestration 20 closure, Wave 1 layout and execution setup: accepted as recommended ("agree") — recorded as D2–D5.

## Decision log

| Date | Decision |
|---|---|
| 2026-10-20 | D1. Diagnosis-first. The `HYSTER.FOR` port is not the fix (no deck has `iHyst > 0`) and is deferred until Orchestration 21 completes. |
| 2026-10-20 | D2. One orchestration carries diagnosis → user gate → fixes. Waves 1 trace and report only. Wave 2 is a user gate where the orchestrator drafts fix-task specs from the reports as an amendment to this file; the user approves before Wave 3 runs. No open-ended "fix whatever you find" scope (the 20.6.1 five-amendment pattern is not repeated). |
| 2026-10-20 | D3. Orchestration 21 absorbs Orchestration 20's gate re-run (`20.6.1`) and default switch (`20.7.1`) as Waves 4 and 5. Wave 0 marks both superseded in `STATE_20.md`, so the highest-numbered plan is the single resume point. Wave 5 stays blocked until Wave 4 reports 44/44. |
| 2026-10-20 | D4. Wave 1 is two parallel traces, `21.1.1` single-porosity long run (`transport_properties/tortuosity_millington`) and `21.1.2` dual-porosity wetting front (`dual_exchange/model6_no_exchange`), plus the task-local Model 0 isolating deck. Plan review split the isolating deck into its own parallel task `21.1.3` to keep each micro-plan ≤ 12 steps (3 workers, within the D5 cap); the orchestrator classifies WatFlow vs DualPor at the Wave 2 gate. Each reports the first diverging quantity with source line references. |
| 2026-10-20 | D5. Execution setup inherits Orchestration 20 D6: ≤3 concurrent workers in `.development/21-worktrees/<id>` on branch `orchestration/<id>`, `inherit` model tier, escalation after one failed retry, serial wave-end gates (A4), and `POLICY_HASHES` re-pin by the orchestrator when `solute.rs` changes (Wave 5 only). Additions: (a) trace instrumentation is task-local patches only (Fortran and Rust) under `.development/<id>/`, with SHA-256 of every patch, instrumented source and binary recorded; diagnosis tasks commit only their report doc. (b) Suspected pinned-Fortran defects found by traces are flagged in the report and registered in `docs/FORTRAN_SOURCE_DEFECTS.md` as `SD-NNN` (with a `source_defect_sdNNN_*` pinning test) by the Wave 3 fix task that reproduces them. |

## Reconnaissance log (2026-10-20, HEAD `470ae05`, branch `phase4-water-acceptance`)

- **Hysteresis is unreachable for every fixture.** All 44 parity `SELECTOR.IN` files have `Model Hysteresis` = `0 0` (38), `6 0` (3) or `7 0` (3). `WATFLOW.FOR:63-76` calls `SetMat` when `iHyst.ne.3` and `Hyster` only when `Iter.eq.2.and.iHyst.gt.0`. `Hyst` (HYSTER.FOR) runs only in the `iHyst.eq.3` else-branch and at `:164-166`. `MatIn` sets `Kappa(i) = -1` for all nodes when `iHyst = 0` (`INPUT.FOR:583-592`).
- **Tables are already ported.** All decks set `hTab1 hTabN = 0.0001 100`, which makes `lTable = .true.` (`INPUT.FOR:567-580`). Rust has `gen_mat` (`water_compat.rs:238`) and the table branch of `set_mat` (`water_compat.rs:486`, `:559-571`). The 35 passing fixtures use the same tables.
- **Pass/fail discriminator: transient water.** Every passing fixture has a uniform initial head and either `tMax = 0.1` (transport) or `tMax = 4` with uniform head (reactions, nonequilibrium, biology, flux_concentration). Failures are the only decks where Richards does real work:
  - `dual_exchange/*` (`tMax = 4`): `PROFILE.DAT` node 1 head `0.0`, others `-100.0` (wetting front). `nonequilibrium/dual_model6` and `dual_model7` have identical `SELECTOR.IN` blocks B–C but uniform `-100.0` head, and pass.
  - `transport_properties/*`: uniform head but `dt dtMin dtMax = 0.1 0.01 0.5`, `tMax = 200`, prints at `50 100 200`, versus `tMax = 0.1` for `transport/transport-baseline-storage`. `lTemp = f` in all five (temperature does not explain node-to-node K differences).
- **Hypothesis H1 (to test first, not established).** `SetMat` evaluates `Con`, `Cap` and `ThEq` at the blended head `hiM = 0.1*hi1 + 0.9*hi2`, with `hi1` from `hTemp` (previous Picard iterate) and `hi2` from `hNew` (`WATFLOW.FOR:470-476`, table branch `:480-520`). Pinned conductivity therefore depends on each node's iterate history, matching the 20.6.1 worker's "conductivity differs between nodes at the same head" observation. Commit `72d2544` added a second `set_mat` call in `wat_flow` (`water_compat.rs:1370-1411`) passing `state.h_new` as the `hTemp` argument, citing `call Hyst(...,ThEq,Con,Cap,0,2)` at `WATFLOW.FOR:73-76`. That call is in the `iHyst.eq.3` branch and is unreachable for these decks (ADR 0007). The extra call removes the 0.1/0.9 blend for `Con`/`Cap`/`ThEq`. H1: this extra call, and/or the `hTemp` value passed to the first `set_mat` call, is the first divergence for one or both groups. The same commit reportedly fixed one `reactions` fixture's equilibrium water content, so reverting it may regress that fixture; the traces must measure both effects.
- **Pass state.** 35/44 at `22d3ef1` (round 5, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md:124-`). Worker evidence: `dual_exchange` first divergence is the step-1 water iteration count (Fortran 8, Rust 9). `transport_properties` top flux drifts.
- **Tracing precedent.** `planning/master/SOLUTE_DIFFUSION_NORMALIZED_TRACE.md` (16.3.1) and `planning/master/SOLUTE_EXAMPLE3_FAILURE_DIAGNOSIS.md` (15.3.1): stage pinned source with `tools/stage_fortran_reference.py --output .development/<id>/…`, patch the *staged copy* only, build with `sh tools/build_fortran_reference.sh --build-dir .development/<id>/…`, record SHA-256 of canonical and instrumented sources, patches and binaries. Scratch tooling from those tasks is under `.development/15.3.1/` and `.development/16.3.1/` (reference only; do not import).
- **Rust candidate path.** `hydrus_core::run_compat_deck(case_json, selector_in, profile_dat)` (`compat_runner.rs:44`) drives `run_parsed_deck` (`:53`); the water step call is at `compat_runner.rs:966-1030` (`if bas.l_wat { … wat_flow … }`). Harness entry `run_parity_fixture_bundle` (`tests/solute_fortran_compat_harness.rs:578`). `tools/solute_rust_candidate_adapter.py:40-60` builds the runner via `cargo test -p hydrus-core --test solute_fortran_compat_harness --no-run`.
- **Parity runner.** `tools/run_solute_parity.py` accepts `--backend fortran-compatible`, `--fixture-manifest <path>` or `--fixture-group <group|all>`, and required `--task-root`. JSON on stdout; per-fixture `status`, `first_divergence`, `records`.
- **Large files (>500 lines; enter by `grep -n` anchors and bounded reads only):** `solute_fortran_compat/driver.rs` (2,371), `water_compat.rs` (2,106), `solute.rs` (1,849), `compat_runner.rs` (1,532), `processes.rs` (1,364), `boundary.rs` (1,314), `deck_reader.rs` (1,249), `source_coupling.rs` (1,231), `output_compat.rs` (1,172), `dual_domain.rs` (1,054), `accounting.rs` (931), `tests/solute_fortran_compat_harness.rs` (912), `nonequilibrium.rs` (831), `types.rs` (828), `initialization.rs` (654), `biology.rs` (649), `iteration.rs` (620); pinned `WATFLOW.FOR` (1,127), `TIME.FOR` (1,594), `INPUT.FOR` (1,723), `HYDRUS.FOR` (999).
- **Baseline** (serial, HEAD `470ae05`): recorded in `STATE_21.md`.

## Hazards

- **Fixed-form Fortran (ADR 0007, 0010).** `!` in physical column 6 is a continuation; `*`/`c`/`C` in column 1 is a comment (`WATFLOW.FOR:116` `c          if(TauW.gt.0) …` is commented out). Audit every cited branch for reachability given the deck's selectors (`iHyst`, `iModel`, `lTable`, `lVapor`, `lWTDep`, `iDualPor`) before porting or blaming it.
- **Trace guards.** 15.3.1's Fortran trace guard `t <= 1e-12` never fired because `Solute` receives `t` after the step advance. Guard instrumentation on `TLevel`, `Iter` and `ItCum` (or an explicit counter), never on floating-point `t`.
- **Instrumentation must not perturb results.** Write traces to a separate unit/file with `write(77,'(…E16.8…)')`-style fixed formats; do not add statements between arithmetic statements that could change gfortran evaluation (put them at loop boundaries). Confirm the instrumented Fortran run's normal output files are byte-identical to the canonical reference run before trusting the trace.
- **Fortran path aliases on Linux.** The pinned source opens backslash paths literally (`.development/16.3.1/` contains files named `solute-input\LEVEL_01.DIR`). Reuse `tools/run_fortran_reference.py` staging behavior rather than inventing a run layout.
- **Node order.** `PROFILE.DAT` line `i` maps to internal node `NumNP - i + 1` (`INPUT.FOR:271-278`); internal node 1 is the bottom. Report node indices in internal (Fortran) numbering.
- **Precision (ADR 0010, 0015).** Fortran default `REAL` is `f32`; compare traced values at `f32` precision (`E16.8` / `{:.8e}`) and report the first index where they differ in the 7th significant digit or beyond the field tolerance, whichever is earlier.
- **Stale candidate binary.** `tools/solute_rust_candidate_adapter.py:40-70` caches the runner binary. For each H1 variant, record the candidate binary SHA-256 and confirm it changed from the HEAD run before trusting a parity result.
- **Commit `72d2544` is suspect** (H1). Do not treat its extra `set_mat` call as source-faithful.
- **Filesystem-free production code.** No `std::fs`/`std::path` in non-test code under `crates/hydrus-core/src/`.
- **Worktrees.** Never stage `.development/`, `target/` or `.claude/`.
- **Disk space.** Root filesystem hit 100% on 2026-10-20; the Python suite then truncated the tracked `planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json` to 0 bytes (restored from HEAD). Each worktree's own `target/` costs ~2 GB. Wave 1 workers set `CARGO_TARGET_DIR=.development/21-target/<id>` and the orchestrator confirms ≥ 10 GB free before dispatching a wave and runs `git status --short planning/master` after every Python suite run.

## Shared acceptance rules (every task)

- **A1.** No existing test assertion or tolerance in `tests/` or `src/` is weakened.
- **A2.** All targeted commands named in the task spec pass; the worker reports exact before/after counts.
- **A3.** Tasks that change Rust: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean in the worktree.
- **A4.** Wave end, orchestrator, serially: `cargo test --workspace`; `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`. Results (and any D5 `POLICY_HASHES` re-pin) recorded in `STATE_21.md`.
- **A5 (diagnosis tasks).** `git diff --exit-code -- src_reference crates tools Cargo.toml Cargo.lock` is clean in the worktree at hand-off; the only committed file is the task's report doc. Canonical `src_reference/WATFLOW.FOR` SHA-256 is recorded and unchanged.

## Wave table

| Wave | Task | Title | Must-edit / create (disjoint within wave) |
|---|---|---|---|
| 0 | — | Orchestrator: baseline, close Orchestration 20, commit plan (no worker) | `planning/STATE_21.md`, `planning/STATE_20.md`, `planning/{EXECUTION_PLAN,PROMPT_TEMPLATE}_21.md` |
| 1 | 21.1.1 | Trace single-porosity long-run divergence (`transport_properties/tortuosity_millington`) | create `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_LONGRUN.md` |
| 1 | 21.1.2 | Trace dual-porosity wetting-front divergence (`dual_exchange/model6_no_exchange`) | create `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_WETTING.md` |
| 1 | 21.1.3 | Trace a task-local Model 0 wetting-front deck (WatFlow vs DualPor isolation) | create `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_ISOLATION.md` |
| 2 | — | **User gate**: orchestrator drafts Wave 3 fix specs from the three reports; user approves (no worker) | `planning/EXECUTION_PLAN_21.md` (Wave 3 section), `planning/STATE_21.md` |
| 3 | 21.3.x | Fixes, as specified at the Wave 2 gate (sub-waves `3`, `3B`, … if files overlap) | TBD at gate; bounded by the Wave 3 envelope below |
| 4 | 21.4.1 | Re-run the 44-fixture all-mode gate and refresh evidence docs | `planning/master/{SOLUTE_FORTRAN_PARITY_18,PHASE_5_SOLUTE_EVIDENCE,VALIDATION_STRATEGY}.md` |
| 5 | 21.5.1 | Switch default backend to `FortranCompatible` after 44/44 | `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/tests/{solute_solver_selection,solute_backend_contract}.rs`, `docs/decisions/README.md` |

**Dependency chain:** Wave 0 → `{21.1.1, 21.1.2, 21.1.3}` → Wave 2 gate → Wave 3 → `21.4.1` → `21.5.1`. Waves are sequential; every task depends on all earlier waves. Wave 1 tasks create disjoint new files and share no edits. If `21.4.1` reports anything other than 44/44, Wave 5 does not run; the orchestrator returns to a new user gate (another Wave 3 amendment), never widens `21.4.1`.

---

## Wave 0 — Orchestrator (no worker)

1. Record the A4 baseline at HEAD in `planning/STATE_21.md`.
2. In `planning/STATE_20.md`, set the `20.6.1` and `20.7.1` wave-board rows to `superseded by Orchestration 21 (21.4.1 / 21.5.1)`, and add one line under Execution status pointing to `EXECUTION_PLAN_21.md`. Do not alter existing task reports.
3. Commit `planning/EXECUTION_PLAN_21.md`, `planning/PROMPT_TEMPLATE_21.md`, `planning/STATE_21.md`, `planning/STATE_20.md` (`docs(planning): plan transient water-flow parity diagnosis`).

---

## Wave 1 — Diagnosis (trace and report; no fixes)

### 21.1.1 — Trace single-porosity long-run divergence (`transport_properties/tortuosity_millington`)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_LONGRUN.md`; `planning/tasks/21.1.1.todo.md`; scratch `.development/21.1.1/` (staged and instrumented Fortran, Rust instrumentation patch, run outputs, trace files, hashes).
- **Depends on:** Wave 0.
- **Spec:**
  - Fixture: `reference/fortran_reference/solute/parity/transport_properties/tortuosity_millington/` (`CASE.json`, `SELECTOR.IN`, `PROFILE.DAT`). Model 0, `iHyst = 0`, `lTable = .true.`, `tMax = 200`.
  - Produce paired per-iteration water traces from (a) a staged, instrumented copy of the pinned Fortran and (b) a worktree-local instrumented Rust `wat_flow`/`set_mat`/`tm_cont`, both run on the unmodified fixture deck. Per `TLevel` and `Iter`, trace at least: `dt`, `t`, `Iter`, `ItCum`, `hTemp`, `hNew`, `Con`, `ConO`, `Cap`, `ThNew`, `ThEq`, `vTop`/`rTop`, and the convergence flags (`ItCrit`, `EpsTh`, `EpsH` at the first failing node). Guard on `TLevel`/`Iter`, not `t`.
  - Validate the instrumented Fortran: its normal outputs (`T_LEVEL.OUT`, `BALANCE.OUT`, `NOD_INF.OUT`, `solute1.out`) must be byte-identical to the fixture's reference-run outputs. Validate the Rust instrumentation the same way against the un-instrumented Rust candidate outputs.
  - Locate the **first divergence**: the earliest (`TLevel`, `Iter`, node, variable) where Rust and Fortran differ beyond `f32` print precision. Map it to the Fortran statement (`file:line`) and the Rust statement (`file:line`).
  - **Test H1 first** (see Reconnaissance). Run the Rust trace three ways: HEAD; with the second `set_mat` call in `wat_flow` (`water_compat.rs:1370-1411`) removed; and, if the trace shows `hTemp` passed to the first `set_mat` differs from Fortran's `hTemp` at `WATFLOW.FOR:64`, with that corrected. For each variant record the first-divergence point, then run `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group all --backend fortran-compatible --task-root .development/21.1.1/h1_<variant>` and record the pass count and every fixture whose status differs from HEAD (H1 regression risk: `72d2544` fixed some fixture's equilibrium water content; name it).
  - Audit reachability (ADR 0007) of every Fortran branch the report blames, given this deck's selectors.
  - Write `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_LONGRUN.md` with sections: **Run identity** (commit, fixture SHA-256s, canonical and instrumented source SHA-256s, patch SHA-256s, binary SHA-256s, exact commands); **Instrumentation validation**; **First divergence** (table: `TLevel`, `Iter`, node, variable, Fortran value, Rust value, Fortran `file:line`, Rust `file:line`); **H1 result** (variant table as above); **Root-cause statement** (one paragraph, marked *established* or *hypothesis* with the evidence for each claim); **Suspected source defects** (candidates for `SD-NNN`, or "none"); **Fix outline** (files, functions, source lines, proposed regression test name — an outline for the gate, not a change).
- **Do not touch:** anything under `crates/`, `tools/`, `src_reference/`, `reference/`, `docs/`, `Cargo.*`, `TODO.md`, `planning/STATE_*.md`. Rust instrumentation stays uncommitted in the worktree and is captured as a patch file in `.development/21.1.1/`.
- **Anchors:**
  - `src_reference/WATFLOW.FOR:1-60` (`WatFlow` entry, `hTemp` init), `:60-170` (iteration loop, convergence test, `dt/3` retry), `:433-568` (`SetMat`; `hiM` blend at `:470-476`; `if(Iter.eq.0) ConO(i)=Con(i)` at `:558`)
  - `src_reference/HYDRUS.FOR:400-470` (time loop, `WatFlow` call), `src_reference/TIME.FOR:3-` (`TmCont`)
  - `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs:486-635` (`set_mat`), `:1136-1180` (`tm_cont`), `:1279-1600` (`wat_flow`; second `set_mat` at `:1370-1411`; retry at `~:1580`)
  - `crates/hydrus-core/src/solute_fortran_compat/compat_runner.rs:960-1035` (water step call)
  - `tools/stage_fortran_reference.py:40-60`, `tools/build_fortran_reference.sh`, `tools/run_fortran_reference.py:119-210`
  - Reference implementation for trace method and report shape: `planning/master/SOLUTE_DIFFUSION_NORMALIZED_TRACE.md` (16.3.1)
- **Verification:**
  - Export the Rust instrumentation with `git diff > .development/21.1.1/rust-instrumentation.patch`, revert it with `git checkout -- crates`, then `git diff --exit-code -- src_reference crates tools Cargo.toml Cargo.lock`
  - `sha256sum src_reference/WATFLOW.FOR` equals the value recorded in the report's Run identity
  - The report contains all seven named sections (`grep -c '^## ' planning/master/WATER_TRANSIENT_DIAGNOSIS_21_LONGRUN.md` ≥ 7)
  - `git diff --check`

### 21.1.2 — Trace dual-porosity wetting-front divergence (`dual_exchange/model6_no_exchange`)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_WETTING.md`; `planning/tasks/21.1.2.todo.md`; scratch `.development/21.1.2/` (staged and instrumented Fortran, Rust instrumentation patch, run outputs, trace files, hashes).
- **Depends on:** Wave 0.
- **Spec:**
  - Primary fixture: `reference/fortran_reference/solute/parity/processes/dual_exchange/model6_no_exchange/` (Model 6, exchange coefficient 0, wetting front: `PROFILE.DAT` line 1 head `0.0`, which is internal node `NumNP`; others `-100.0`). Control: `processes/nonequilibrium/dual_model6/` (identical `SELECTOR.IN` blocks B–C, uniform `-100.0` head, passes).
  - Produce paired per-iteration water traces (instrumented staged Fortran; worktree-local Rust instrumentation) for the primary fixture and the control. Per `TLevel` and `Iter`, trace at least: `dt`, `Iter`, `ItCum`, `hTemp`, `hNew`, `Con`, `ConO`, `Cap`, `ThNew`, `ThEq`, `ThNewIm`, `SinkIm`, and the convergence flags (`ItCrit`, `EpsTh`, `EpsH` at the first failing node). Guard on `TLevel`/`Iter`, not `t`.
  - Validate instrumentation exactly as in `21.1.1` (byte-identical normal outputs versus un-instrumented runs).
  - Locate the first divergence (`TLevel`, `Iter`, node, variable) and map it to Fortran and Rust `file:line`. Report the step-1 iteration counts (20.6.1 reported Fortran 8, Rust 9).
  - **Test H1 first**, with the same three Rust variants as `21.1.1`, on the primary fixture and the control. For each variant record the first-divergence point and run `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group dual_exchange --backend fortran-compatible --task-root .development/21.1.2/h1_<variant>` plus the same for `--fixture-group nonequilibrium`; record pass counts and status changes versus HEAD.
  - Audit reachability (ADR 0007) of every blamed branch given `iDualPor = 1` (Model 6 maps `iModel = 0`, `iDualPor = 1`, `INPUT.FOR:617-620`).
  - Write `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_WETTING.md` with the same seven sections as `21.1.1`.
- **Do not touch:** same list as `21.1.1`.
- **Anchors:**
  - `src_reference/WATFLOW.FOR:60-170` (iteration loop), `:433-568` (`SetMat`), `:170-430` (`Reset`; `grep -n "iDualPor\|SinkIm" src_reference/WATFLOW.FOR`), `:1027-` (`DualPor`)
  - `src_reference/INPUT.FOR:600-670` (`MatIn` Model 6/7 mapping), `:472-507` (`InitW`), `:508-` (`InitDualPor`)
  - `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs:349-382` (`init_dual_por`), `:486-635` (`set_mat`), `:636-800` (`reset`), `:1279-1600` (`wat_flow`)
  - `crates/hydrus-core/src/solute_fortran_compat/dual_domain.rs` (`grep -n "fn dual_porosity_exchange"`)
  - `crates/hydrus-core/src/solute_fortran_compat/compat_runner.rs:960-1035`
  - Reference implementation: `planning/master/SOLUTE_DIFFUSION_NORMALIZED_TRACE.md` (16.3.1)
- **Verification:** as `21.1.1` (patch path `.development/21.1.2/rust-instrumentation.patch`), for `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_WETTING.md` (≥ 7 `## ` sections).

### 21.1.3 — Trace a task-local Model 0 wetting-front deck (WatFlow vs DualPor isolation)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** none.
- **Can create:** `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_ISOLATION.md`; `planning/tasks/21.1.3.todo.md`; scratch `.development/21.1.3/` (the isolating deck, its fresh Fortran reference run, staged and instrumented Fortran, Rust instrumentation patch, traces, hashes).
- **Depends on:** Wave 0.
- **Spec:**
  - Build the isolating deck in `.development/21.1.3/isolating_model0/` from `processes/dual_exchange/model6_no_exchange/`: change only `Model Hysteresis` from `6 0` to `0 0` and the material row to its first six values (`thr ths Alfa n Ks l`), plus any dependent edits that `MatIn`/`ChemIn` (`INPUT.FOR`) require for a Model 0 deck (cite the line for each). Keep the wetting-front `PROFILE.DAT`. Update `CASE.json` hashes in the task-local copy only as `tools/run_fortran_reference.py` requires. Record every edit as a unified diff.
  - Run the isolating deck through the un-instrumented pinned Fortran (fresh reference run under `.development/21.1.3/`, via `tools/run_fortran_reference.py` staging) and through the Rust candidate. Compare with `tools/run_solute_parity.py --backend fortran-compatible --fixture-manifest <task-local manifest>` if the runner accepts it; otherwise compare the normalized outputs with the same normalizers (`tools/solute_parity_normalizers.py`) and say so.
  - If they diverge, trace as in `21.1.1` (same variables, guards, validation) and locate the first divergence with `file:line`. Run the three H1 variants on this deck and record the first-divergence point and pass/fail for each.
  - This deck is evidence only: never copy it into `reference/` or the parity set.
  - Write `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_ISOLATION.md` with the seven `21.1.1` sections plus **Isolating deck** (diff, Fortran run identity, comparison method).
- **Do not touch:** same list as `21.1.1`.
- **Anchors:** `src_reference/INPUT.FOR:536-670` (`MatIn`), `:1289-1430` (`ChemIn`); `tools/run_fortran_reference.py:87-210`; `tools/solute_parity_normalizers.py` (`grep -n "^def "`); plus the `21.1.1` water anchors.
- **Verification:** as `21.1.1` (patch path `.development/21.1.3/rust-instrumentation.patch`), for `planning/master/WATER_TRANSIENT_DIAGNOSIS_21_ISOLATION.md` (≥ 8 `## ` sections); `git status --short reference` is empty.

---

## Wave 2 — User gate (orchestrator, no worker)

1. Read the three reports. Cross-check every *established* claim by re-running at least one trace comparison command from each report.
2. Classify each root cause as **WatFlow** (the `21.1.3` Model 0 deck also diverges at the same kind of point), **DualPor** (it matches), or **both**, and decide whether the two failing groups share a cause. Draft Wave 3 fix-task specs in this file from the three reports (full spec fields: files in scope, anchors, spec, regression tests, verification), one task per root cause, honoring the envelope below. Sequence tasks that touch the same file into sub-waves.
3. Present the classification, the reports' root-cause statements, the H1 results, and the drafted specs to the user. Record the user's approval as decision D6 (dated). Wave 3 does not start without it.

## Wave 3 — Fixes (specified at the Wave 2 gate)

**Envelope (binding on every Wave 3 spec):**
- Edits limited to `crates/hydrus-core/src/solute_fortran_compat/**` and `crates/hydrus-core/tests/solute_fortran_compat_harness.rs`, plus `docs/FORTRAN_SOURCE_DEFECTS.md` and `docs/decisions/` when a spec names them.
- Every change cites the pinned `file:line` it matches and the diagnosis report finding it resolves.
- Each task adds a regression test that fails at HEAD and passes after (named in the spec), and re-runs, via `tools/run_solute_parity.py --backend fortran-compatible`, every fixture named in its spec plus the full group(s) it affects.
- If the fix removes the extra `set_mat` call from `72d2544`, the spec states which fixture(s) that change was protecting and how they stay green.
- Reproduced pinned-Fortran defects: `SD-NNN` entry in `docs/FORTRAN_SOURCE_DEFECTS.md`, `// SD-NNN` tag at the reproducing code, `source_defect_sdNNN_*` pinning test.
- Never: tolerance or `OBSERVATION_MAP.json` changes, `tools/` comparison changes, `solute.rs`, FV/BE paths, `Cargo.*`, `TODO.md`, `src_reference/`.

## Wave 4 — 44-fixture gate re-run

### 21.4.1 — Re-run the 44-fixture all-mode gate and refresh evidence docs

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `planning/master/SOLUTE_FORTRAN_PARITY_18.md`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`, `planning/master/VALIDATION_STRATEGY.md`.
- **Can create:** `planning/tasks/21.4.1.todo.md`; scratch `.development/21.4.1/`.
- **Depends on:** Wave 3 (all fix tasks done).
- **Spec:**
  - Run `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group all --backend fortran-compatible --task-root .development/21.4.1/all_parity > .development/21.4.1/all_parity.json`. Then each of the 7 groups (`transport`, `reactions`, `transport_properties`, `nonequilibrium`, `dual_exchange`, `biology`, `flux_concentration`) separately with its own `--task-root`. Count fixtures: must be 44, with per-fixture `status` `pass` and zero `fail` (the runner has no `divergent_count` field; see `planning/tasks/20.6.1.todo.md` Notes).
  - Extract, read-only: per-fixture status, distinct `reference_output_sha256` vs `candidate_output_sha256` sets (must be disjoint: the Rust candidate ran), `git rev-parse HEAD`, UTC timestamp.
  - Coverage claims from fixture manifests: count of `iPsi = 3` fixtures and of fixtures with coupled `WatFlow`/`DualPor` that output `W-volumeI[L]`; record a gap rather than assert what the manifests cannot show.
  - Append a `21.4.1` section to each of the three docs (run command, timestamp, commit, per-group counts, digest summary, coverage, and the disclaimer that `FortranCompatible` establishes pinned-source numerical parity while continuum accuracy is tracked by the FV/BE analytical and convergence suites). Consistent wording across the three docs. Mark the `20.6.1` round-5 FAIL sections as superseded (one line each; do not delete them).
  - **No fixes in this task.** Any `fail` → record the FAIL result in the docs, stop, and report; the orchestrator returns to a user gate.
- **Do not touch:** anything under `crates/`, `tools/`, `OBSERVATION_MAP.json`, `TODO.md`, `planning/STATE_*.md`, `Cargo.*`.
- **Anchors:** `planning/master/SOLUTE_FORTRAN_PARITY_18.md:140-`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md:120-`, `planning/master/VALIDATION_STRATEGY.md:105-` (the `20.6.1` round-5 sections); `tools/run_solute_parity.py:440-470` (CLI).
- **Verification:**
  - The all-mode command exits 0 with top-level `"status": "pass"`.
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_fortran_parity`
  - `git diff --check`; `git diff --exit-code -- crates tools Cargo.toml Cargo.lock`

## Wave 5 — Default backend switch

### 21.5.1 — Switch default backend to `FortranCompatible` after 44/44

Ported from `EXECUTION_PLAN_20.md` `20.7.1`, with test names updated to the current file.

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/tests/solute_solver_selection.rs`, `crates/hydrus-core/tests/solute_backend_contract.rs` (default assertion at `:199`), `docs/decisions/README.md`.
- **Can create:** `planning/tasks/21.5.1.todo.md`; scratch `.development/21.5.1/`.
- **Depends on:** `21.4.1` accepted with 44/44.
- **Spec:**
  - `solute.rs:118-150` and `SoluteConfig::new_with_species_boundaries` (`:278-295`, `backend:` at `:285`): default to `SoluteBackend::FortranCompatible`; `FiniteVolumeBackwardEuler` stays selectable via `SoluteConfig::with_backend`. Update doc comments (`:122`, `:144`) to cite the `21.4.1` 44/44 gate.
  - `tests/solute_solver_selection.rs`: rename `default_backend_is_finite_volume_backward_euler` (`:42`) to `default_backend_is_fortran_compatible`; update `backend_selection_round_trips` (`:59`), `clone_and_equality_preserve_backend` (`:73`), and rename `default_advance_solute_step_uses_fv_be_and_explicit_fortran_compatible_is_selectable` (`:84`) to `default_advance_solute_step_uses_fortran_compatible_and_explicit_fv_be_is_selectable`. Verify default `SoluteConfig::new` selects `FortranCompatible`, is bit-identical to explicit `FortranCompatible`, differs from FV/BE on a non-trivial case, and preserves all explicit FV/BE behavior (FV/BE succeeds on the 2-node mesh; `FortranCompatible` keeps its 3-node requirement).
  - `tests/solute_backend_contract.rs:199`: assert the new default (`FortranCompatible`).
  - Hidden coupling: about ten other test files (`solute_contract`, `solute_outlet`, `solute_reactions`, `solute_acceptance`, `water_solute_coupling`, `solute_transaction`, `solute_transport`, `solute_species_boundaries`, `allocation_contract`) call `SoluteConfig::new` without `with_backend` and will run on `FortranCompatible` after the switch (3-node minimum). Run the crate-wide test command below. If any test outside **Must edit** fails, stop and report the list (escalation rule 4): the orchestrator re-scopes, the worker does not edit those files.
  - `docs/decisions/README.md` Scope and Status Notes: record that `FortranCompatible` is the default following the `21.4.1` 44/44 gate.
  - Orchestrator re-pins `POLICY_HASHES["crates/hydrus-core/src/solute.rs"]` in `tools/solute_benchmark_bridge.py` at wave end (D5) and records the justification.
- **Do not touch:** `tools/solute_benchmark_bridge.py`, `TODO.md`, `crates/hydrus-core/src/solute_fortran_compat/**`.
- **Anchors:** `crates/hydrus-core/src/solute.rs:118-168`, `:278-295`; `crates/hydrus-core/tests/solute_solver_selection.rs:1-170`; `crates/hydrus-core/tests/solute_backend_contract.rs:190-205`; `docs/decisions/README.md:24-30`.
- **Verification:**
  - `cargo test -p hydrus-core --lib solute`
  - `cargo test -p hydrus-core --test solute_solver_selection --test solute_backend_contract --test solute_acceptance`
  - `cargo test -p hydrus-core --tests` (crate-wide fallout check; baseline 413 passed)
  - `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`

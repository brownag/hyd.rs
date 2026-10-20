# Orchestration 20 — Resolve code-review findings & wire genuine Rust-vs-Fortran solute parity

## Goal and scope

**Goal.** Resolve all 11 findings from the 2026-10-15 two-axis code review (`/code-review main use planning/ for spec`) on `phase4-water-acceptance` (`76b832c`):
1. Revert the unverified default backend switch (`SoluteBackend::FiniteVolumeBackwardEuler` stays default until genuine Rust-vs-Fortran parity passes across all 44 fixtures).
2. Deduplicate `solute_fortran_compat::adapter` helpers, reject unreachable moisture selector `2` (`iMoistDep == 2`) in the adapter, and rename `BlockingMode::SourceNoOp3` to `BlockingMode::Type3` to reflect active `SOLUTE.FOR:1570-1579` continuation lines.
3. Deduplicate `blocking` and `deposit` in `accounting.rs` in favor of `deposition::{blocking, deposit, DepositInput}` (preserving `SOLUTE.FOR:1450-1451`'s `mi`/`mj` indexing in `accumulate_bacteria`), and add the missing `"dual_exchange"` group alias to `tools/run_solute_parity.py`.
4. Eliminate per-step and per-retry heap allocations in `solute_fortran_compat::{assembly, driver, iteration}` and `solute::advance_fortran_compatible_step`, wire `SoluteConfig`'s per-species boundaries, `mesh.material_indices()`/`mesh.material_count()`, and `kinetic_sorption` into `advance_fortran_compatible_step`, and extend `tests/allocation_contract.rs` to assert 0 allocations for every `SoluteBackend` variant.
5. Port source-precision deck input reading (`SELECTOR.IN` + `PROFILE.DAT` alongside `CASE.json`), coupled water/dual-porosity/time-control stepping (`MATERIAL.FOR`, `INPUT.FOR`, `WATFLOW.FOR`, `TIME.FOR`), and output formatting (`OUTPUT.FOR` → `PROFILE.OUT`, `NOD_INF.OUT`, `NOD_INF_V.OUT`, `solute1..NS.out`, `OBS_NODE.OUT`, `BALANCE.OUT`) in `solute_fortran_compat`, removing all `#[allow(dead_code)]` annotations in `mod.rs` and `iteration.rs`.
6. Replace the fabricated `tools/solute_rust_candidate_adapter.py` (which called `run_fortran_reference.run_fixture`) with genuine Rust compatibility execution and normalization, verify 44/44 all-mode Rust-vs-Fortran parity, refresh the parity evidence docs, and switch `SoluteConfig`'s default backend to `SoluteBackend::FortranCompatible` only in the final wave after 44/44 pass.

**In scope:**
- Standards Hard #1 & Spec #c1/#b1: `tools/solute_rust_candidate_adapter.py` invoking Rust instead of Fortran; default backend reset in Wave 1 and gated switch in Wave 7.
- Standards Hard #2 & Spec #a1: wiring `solute_fortran_compat` end-to-end and removing all 7 module-level `#[allow(dead_code)]` in `compat/mod.rs` and all 8 item-level `#[allow(dead_code)]` in `compat/iteration.rs`.
- Standards Hard #3, Standards Judgement #2 & Spec #c2: zero-allocation `SoluteWorkspace` compat scratch + `assembly.rs`/`driver.rs`/`iteration.rs` scratch reuse + complete `SoluteConfig` mapping in `advance_fortran_compatible_step` + allocation contract tests for both backends.
- Standards Judgement #1 & Spec #a3: adapter helper deduplication (`require_wrapped_object`, `reals`, test helpers `wrap`/`err_has`), `moisture_selector > 1` rejection in `adapter/reactions.rs:99-101`, `accounting.rs` `blocking`/`deposit` deduplication, `"dual_exchange"` group alias in `tools/run_solute_parity.py`.
- Spec #a2: `BlockingMode::SourceNoOp3` → `BlockingMode::Type3` rename and correction of `iPsi=3` active status in `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` and `planning/master/SOLUTE_FORTRAN_PARITY_18.md`.

**Out of scope:**
- Modifying pinned `src_reference/` or portability patches.
- Changing any FV/BE solute equation or water-solver equation outside `solute_fortran_compat` (ADR-0002).
- Adding any Cargo dependency (`Cargo.toml` and `Cargo.lock` are read-only in Orchestration 20; ADR-0011).
- Editing `TODO.md` or any pre-existing uncommitted user files.

**Constraints stated by the user** (verbatim answers, 2026-10-15):
- Parity execution strategy: "Full-input parity execution: parse the fixture's authenticated deck inputs (SELECTOR.IN / PROFILE.DAT) alongside CASE.json in the Rust compat harness so Rust genuinely steps and emits every mapped solute observation, and report water-only fields (e.g. W-volumeI) as unavailable unless a water-coupled path is present."
- Step API vs compat runner: "Keep advance_solute_step / SoluteConfig cleanly mapped for the public fields it owns (per-species boundaries, start/end face fluxes, sorption/kinetic parameters, mesh materials) and add a dedicated Rust entry point for full source-compatible deck/request execution."
- Transient water & dual-porosity in compat: "Implement a faithful source-precision WatFlow + DualPor + time-control step port inside solute_fortran_compat (for Models 0, 6, 7 and the fixture boundary modes) so the Rust candidate computes exact water/dual-porosity histories and W-volumeI independently of the Fortran executable."
- Default backend gating: "Reset default to FiniteVolumeBackwardEuler in Wave 1 (reverting the unverified switch), build and verify true Rust-vs-Fortran parity across all 44 fixtures, and switch the default back to FortranCompatible in the final wave only when 44/44 pass."
- Zero-allocation scope: "Preallocate compat scratch buffers in SoluteWorkspace so advance_solute_step / advance_fortran_compatible_step is strictly zero-allocation (and extend tests/allocation_contract.rs to test both SoluteBackend variants), while also refactoring driver.rs / iteration.rs to reuse preallocated scratch buffers on nonlinear retries."
- Parallelism, model tier & gates: "Keep Orchestration 19's setup: ≤3 parallel workers in isolated git worktrees, inherit/Sonnet workers with escalation after 1 retry, and serial wave-end gates (cargo test --workspace, Python unittest suite, cargo fmt --check, cargo clippy -D warnings, git diff --check) plus POLICY_HASHES re-pinning when solute.rs changes."

## Decision log

| Date | Decision |
|---|---|
| 2026-10-15 | D1. Full-input parity execution: the Rust compatibility runner reads the fixture's authenticated deck inputs (`SELECTOR.IN` and `PROFILE.DAT`) alongside `CASE.json` (verifying `input_files` SHA-256 digests and `request_sha256` before stepping) and emits the standard HYDRUS output files (`PROFILE.OUT`, `NOD_INF.OUT`, `NOD_INF_V.OUT`, `solute1..NS.out`, `OBS_NODE.OUT`, `BALANCE.OUT`) in source format so `tools/solute_parity_normalizers.py` normalizes the Rust candidate outputs through the exact same `OBSERVATION_MAP.json` pipeline as the reference run, without ever invoking the Fortran solver in `solute_rust_candidate_adapter.py`. |
| 2026-10-15 | D2. Separation of public step API and full-deck compat runner: `advance_solute_step` / `advance_solute_step_with_processes` (when `SoluteConfig::backend() == SoluteBackend::FortranCompatible`) maps all public `SoluteConfig`, `SoluteTransportInput`, `SoluteProcessInput`, and `Mesh1D` fields it owns (`config.boundaries[species]`, `mesh.material_indices()`, `mesh.material_count()`, `sp.kinetic_sorption`, `accepted_water`, `start_water_contents`) with zero heap allocations after `SoluteWorkspace` construction. Full multi-step deck execution lives in a dedicated `solute_fortran_compat::compat_runner` entry point exposed to the integration harness and CLI test target. |
| 2026-10-15 | D3. Source-precision coupled `WatFlow` + `DualPor` + `TmCont` port inside `solute_fortran_compat::water_compat`: implements `MATERIAL.FOR` (`FK`, `FC`, `FQ`, `FH`, `FS` for `iModel=0`), `INPUT.FOR` (`GenMat`, `InitW`, `InitDualPor`), `WATFLOW.FOR` (`WatFlow`, `SetMat`, `Reset`, `Shift`, `Gauss`, `Veloc`, `ConVapor`, `VaporContent`, `ConVh`, `Update`, calling `dual_domain::dual_porosity_exchange` for `DualPor`), and `TIME.FOR` (`TmCont`) in exact `f32`/`f64` source precision so all 44 fixtures (including transient water and `dual_exchange` Models 6 & 7 with `W-volumeI[L]`) are computed independently in Rust. |
| 2026-10-15 | D4. Default backend gating: Wave 1 (`20.1.1`) resets `SoluteConfig`'s default backend to `SoluteBackend::FiniteVolumeBackwardEuler` and marks the parity gate pending in `docs/decisions/README.md` and `planning/master/*`. Wave 6 (`20.6.1`) runs all 44 fixtures end-to-end through the real Rust candidate adapter and verifies 44/44 parity. Wave 7 (`20.7.1`) switches `SoluteConfig` default to `SoluteBackend::FortranCompatible` only after Wave 6 passes 44/44. |
| 2026-10-15 | D5. Zero-allocation step and retry contract: `assembly.rs`, `driver.rs`, and `iteration.rs` gain preallocated scratch structures (`assemble_interior_into`, `SoluteDriverScratch`, `SoluteStepState::copy_from`) so `solve_pass_with_scratch` and `solve_solute_with_nonlinear_with_scratch` perform zero heap allocations during linear passes and nonlinear `dt/3` retries. `SoluteWorkspace` preallocates the compatibility step buffers (`SoluteWorkspace::with_materials(node_count, species_count, material_count)` plus `SoluteWorkspace::new(node_count, species_count)` preallocating up to `DEFAULT_MAX_MATERIALS = 16` materials), and `tests/allocation_contract.rs` asserts 0 allocations during `advance_solute_step` for both `FiniteVolumeBackwardEuler` and `FortranCompatible`. |
| 2026-10-15 | D6. Execution model and `POLICY_HASHES` rule: at most 3 concurrent workers in `.development/20-worktrees/<id>` on branch `orchestration/<id>`, using `inherit` model tier. Workers never edit `planning/STATE_*.md` or `tools/solute_benchmark_bridge.py`. Whenever a task edits `crates/hydrus-core/src/solute.rs` (`20.1.1`, `20.2.2`, `20.7.1`), the orchestrator re-pins the `crates/hydrus-core/src/solute.rs` SHA-256 in `tools/solute_benchmark_bridge.py` (`POLICY_HASHES`) at wave end and records the justification in `STATE_20.md` (same protocol as Orchestration 19 D11). |

## Wave table

| Wave | Task | Title | Must-edit / create (disjoint within wave) |
|---|---|---|---|
| 0 | — | Orchestrator baseline (no worker) | `planning/STATE_20.md` |
| 1 | 20.1.1 | Revert unverified default backend to `FiniteVolumeBackwardEuler` | `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/tests/solute_solver_selection.rs`, `docs/decisions/README.md`, `planning/master/SOLUTE_FORTRAN_PARITY_18.md`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`, `planning/master/VALIDATION_STRATEGY.md` |
| 1 | 20.1.2 | Deduplicate adapter helpers, reject moisture selector 2, rename `BlockingMode::Type3` | `crates/hydrus-core/src/solute_fortran_compat/types.rs`, `crates/hydrus-core/src/solute_fortran_compat/adapter/{mod,fields,transport,reactions,transport_properties,nonequilibrium,biology,source_coupling,tests}.rs`, `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` |
| 1 | 20.1.3 | Deduplicate `accounting.rs` deposition helpers and add `dual_exchange` group alias | `crates/hydrus-core/src/solute_fortran_compat/{accounting,deposition}.rs`, `tools/run_solute_parity.py`, `tools/test_solute_fortran_parity.py` |
| 2 | 20.2.1 | Preallocated scratch buffers in `assembly.rs`, `driver.rs`, and `iteration.rs` | `crates/hydrus-core/src/solute_fortran_compat/{assembly,driver,iteration}.rs` |
| 2B | 20.2.2 | Full `SoluteConfig` mapping and zero-allocation compat step in `solute.rs` | `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/tests/{allocation_contract,solute_backend_contract}.rs` |
| 3 | 20.3.1 | Source-compatible deck reader (`SELECTOR.IN` + `PROFILE.DAT` + `CASE.json`) | `crates/hydrus-core/src/solute_fortran_compat/deck_reader.rs` (new), `crates/hydrus-core/src/solute_fortran_compat/adapter/mod.rs` |
| 3 | 20.3.2 | Source-precision `WatFlow`, `DualPor`, `SetMat`, `Veloc`, and `TmCont` port | `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs` (new), `crates/hydrus-core/src/solute_fortran_compat/mod.rs` |
| 3 | 20.3.3 | Source-precision `OUTPUT.FOR` table formatters (`Profil`, `NodOut`, `TLInf`, `ObsNod`, `SubReg`) | `crates/hydrus-core/src/solute_fortran_compat/output_compat.rs` (new), `crates/hydrus-core/src/solute_fortran_compat/types.rs` |
| 4 | 20.4.1 | Full-run compatibility runner and removal of all `dead_code` allowances | `crates/hydrus-core/src/solute_fortran_compat/compat_runner.rs` (new), `crates/hydrus-core/src/solute_fortran_compat/{mod,iteration,types}.rs`, `crates/hydrus-core/src/solute_fortran_compat/adapter/{mod,fields}.rs`, `crates/hydrus-core/src/lib.rs`, `crates/hydrus-core/tests/solute_fortran_compat_harness.rs` |
| 5 | 20.5.1 | Replace `solute_rust_candidate_adapter.py` with genuine Rust candidate execution | `tools/solute_rust_candidate_adapter.py`, `tools/test_solute_fortran_parity.py` |
| 6 | 20.6.1 | Execute 44-fixture all-mode Rust-vs-Fortran parity gate and update evidence | `planning/master/SOLUTE_FORTRAN_PARITY_18.md`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`, `planning/master/VALIDATION_STRATEGY.md` (plus any surgical compat rounding fix in `crates/hydrus-core/src/solute_fortran_compat/{deck_reader,water_compat,output_compat,compat_runner}.rs` if uncovered by the 44-fixture run) |
| 7 | 20.7.1 | Switch default backend to `FortranCompatible` after 44/44 parity gate | `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/tests/solute_solver_selection.rs`, `docs/decisions/README.md` |

**Dependency chains:**
- default-gate & public solute API: `20.1.1` → `20.2.1` → `20.2.2` → `20.6.1` → `20.7.1`
- compat kernel & full-run pipeline: `{20.1.2, 20.1.3}` → `20.2.1` → `20.2.2` → `{20.3.1, 20.3.2, 20.3.3}` → `20.4.1` → `20.5.1` → `20.6.1` → `20.7.1`

Waves are sequential, so every task also depends on all earlier waves. Wave 2 runs as `20.2.1` followed by `20.2.2` (`Wave 2B`) so `20.2.2` compiles directly against `20.2.1`'s merged `SoluteDriverScratch` and `solve_solute_with_nonlinear_with_scratch`. Within Wave 1 (`20.1.1`, `20.1.2`, `20.1.3`) and Wave 3 (`20.3.1`, `20.3.2`, `20.3.3`), file ownership is strictly pairwise-disjoint.

## Hazards (from reconnaissance and code review)

- **Large files (>500 lines).** Enter them through `grep -n` anchors and bounded `view_file` ranges only, never a whole-file read:
  - `crates/hydrus-core/src/solute_fortran_compat/driver.rs` (1,928 lines)
  - `crates/hydrus-core/src/solute.rs` (1,849 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/processes.rs` (1,364 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/boundary.rs` (1,340 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/source_coupling.rs` (1,230 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/tests.rs` (1,197 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/dual_domain.rs` (1,054 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/accounting.rs` (1,002 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/types.rs` (859 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/nonequilibrium.rs` (831 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/mod.rs` (665 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/initialization.rs` (655 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/biology.rs` (649 lines)
  - `crates/hydrus-core/tests/solute_fortran_compat_harness.rs` (573 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/iteration.rs` (565 lines)
  - `tools/solute_parity_normalizers.py` (553 lines)
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/transport.rs` (503 lines)
- **ADR-0010 & fixed-form Fortran rules.** In `solute_fortran_compat/`, preserve Fortran expression grouping, statement order, and `f32` (`DefaultReal`) vs `f64` (`DoubleReal`) casts. Remember that `!` in column 6 is a fixed-form continuation line (`src_reference/SOLUTE.FOR:1570-1579`, `WATFLOW.FOR:44-46`), whereas `*` or `C` in column 1 or inline `!` outside column 6 is a comment (`WATFLOW.FOR:132` `* if(TLevel.eq.1) Eps=0.` is commented out!).
- **Subtle `SOLUTE.FOR:1450-1451` indexing in `accounting.rs`.** In `accumulate_bacteria` (`crates/hydrus-core/src/solute_fortran_compat/accounting.rs:265-266`), `ipsi2 = ch[mi][jj + 12].0 as i32` (`Mi`) and `ipsi1 = ch[mj][jj + 13].0 as i32` (`Mj`) mirror `src_reference/SOLUTE.FOR:1450-1451` (`if(.not.lFiltr) iPsi2=int(ChPar(jjj+13,Mi))` and `if(.not.lFiltr) iPsi1=int(ChPar(jjj+14,Mj))`). Do **not** replace `accumulate_bacteria`'s node loop with `deposition::deposit_nodes` (which reads both from `ch_par[m]` matching `SOLUTE.FOR:1624-1625`); instead, call `super::deposition::{blocking, deposit, DepositInput}` directly from `accumulate_bacteria` and delete only the private `fn blocking` (`:375-397`) and `fn deposit` (`:409-450`) copies in `accounting.rs`.
- **Node ordering between `PROFILE.DAT` and internal solver arrays.** `INPUT.FOR:271-278` (`NodInf`) reads `PROFILE.DAT` lines `i=1..NumNP` (top-to-bottom in the file) and stores them into `n = NumNP - i + 1` (so internal node `1` is the bottom of the column `x = -L` and internal node `NumNP` is the top surface `x = 0.0`). `OUTPUT.FOR` (`NodOut` `:795`, `Profil` `:968`) writes nodes `n = NumNP, 1, -1` (top-to-bottom). Preserve this exact `NumNP - i + 1` reversal in `deck_reader.rs`, `output_compat.rs`, and `compat_runner.rs`.
- **Unit conversions in `INPUT.FOR` (`Conversion`, `MatIn`, `ChemIn`).** `INPUT.FOR:182-204` (`Conversion`) converts `Conc` and `Sorb` in `PROFILE.DAT` when `lMassIni` (`lBact` stays unmodified). `MatIn` (`:659`) divides `Par(5,M)` (`Ks`) by `xConv/tConv` (`1.0`). `ChemIn` (`:1379, :1422`) scales `cBound` (`1/xConv^3` except species 1 when `lBact`), `ChPar` (`Bulk.d.`, `DifW`, `DifG`, `DisperL`, `Henry`, `GamL`, `GamS`, `GamG`, `GamL1`, `GamS1`, `GamG1`, `ZeroL`, `ZeroS`, `ZeroG`, etc.), and `tPulse`. `deck_reader.rs` must port `INPUT.FOR`'s exact unit conversion statements (`tConv`, `xConv`) when reading raw `SELECTOR.IN`/`PROFILE.DAT`.
- **Initial time step vs first step.** In `HYDRUS.FOR:259-392`, before entering the time loop at label `11` (`:400`), HYDRUS runs `Init`, `BasInf`, `NodInf`, `MatIn`, `InitW`, `GenMat`, `SetMat`, `InitDualPor`, `TmIn`, `ChemIn`, `MassInit`, `NonEqInit`, `Profil`, `NodOut(tInit)`, `SubReg(PLevel=0)`, `Veloc`, `vNew=vOld; ThNew=ThOld`, and `if(lBact.and..not.lWat) call Exclusion`. Then at label `11`: `if(lWat.and.(TLevel.gt.1.or.lWTDep)) call WatFlow(...)`, `if(lWat.and.(TLevel.gt.1.or.lWTDep)) call Veloc(...)`, `call Solute(...)`, `call TLInf(...)`, `if(NObs.gt.0) call ObsNod(...)`, `if(abs(t-TPrint(PLevel)).lt.0.001*dt) call NodOut / SubReg`, `call TmCont(...)`, `t = t + dt`, `call Update(...)`. Notice that when `TLevel == 1` and `.not.lWTDep`, `WatFlow` and `Veloc` are **skipped** on step 1 (`HYDRUS.FOR:402, 419`)! Matching this exact condition is essential for step-1 parity.
- **Filesystem-free production code in `hydrus-core`.** Non-test (`#[cfg(not(test))]`) code in `crates/hydrus-core/src/` must never use `std::fs` or `std::path`. Unit test modules (`#[cfg(test)]`) inside `crates/hydrus-core/src/` and integration tests in `crates/hydrus-core/tests/` may use `std::fs`/`std::path` to read `fixtures/` (matching `adapter/tests.rs:426`).
- **ADR-0011.** Neither `Cargo.toml` nor `Cargo.lock` may be modified in any task.
- **`POLICY_HASHES` in `tools/solute_benchmark_bridge.py`.** Editing `crates/hydrus-core/src/solute.rs` changes its SHA-256 and causes 4 tests in `tools/test_solute_benchmark_bridge.py` to fail until the orchestrator updates `POLICY_HASHES["crates/hydrus-core/src/solute.rs"]` at the wave gate (D6).
- **Pre-existing dirty files.** `TODO.md` has uncommitted user changes; never stage, revert, or touch it.
- **Worktrees.** Workers run in `.development/20-worktrees/<id>` on branch `orchestration/<id>`. Never stage `.development/` or `target/`.

## Shared acceptance rules (every task)

- **A1.** No existing test assertion or tolerance in `tests/` or `src/` is weakened.
- **A2.** All targeted test commands named in the task spec pass, and the worker reports exact before/after test counts.
- **A3.** `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass cleanly in the worktree.
- **A4.** At the end of every wave, the orchestrator runs serially:
  - `cargo test --workspace`
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  and records the results (plus any D6 `POLICY_HASHES` re-pin) in `planning/STATE_20.md`.

## Wave 0 — Orchestrator baseline (no worker)

1. Record the baseline at `76b832c` in `planning/STATE_20.md` (`cargo test --workspace`: 384 passed, 0 failed, 1 ignored; `cargo test -p hydrus-core --lib solute_fortran_compat`: 218 passed, 1 ignored; Python unittest discovery: 313 passed, 0 failed; `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` clean).
2. Commit `planning/EXECUTION_PLAN_20.md`, `planning/PROMPT_TEMPLATE_20.md`, and `planning/STATE_20.md` once approved by the user.

---

## Wave 1 — Gate reset, adapter hygiene, accounting dedup & parity group fix

### 20.1.1 — Revert unverified default backend to `FiniteVolumeBackwardEuler`

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `crates/hydrus-core/src/solute.rs`
  - `crates/hydrus-core/tests/solute_solver_selection.rs`
  - `docs/decisions/README.md`
  - `planning/master/SOLUTE_FORTRAN_PARITY_18.md`
  - `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`
  - `planning/master/VALIDATION_STRATEGY.md`
- **Can create:** `planning/tasks/20.1.1.todo.md`; scratch `.development/20.1.1/`.
- **Depends on:** Wave 0.
- **Spec:**
  - In `crates/hydrus-core/src/solute.rs:118-150` and `SoluteConfig::new_with_species_boundaries` (`:278-290`):
    - Change `SoluteBackend` doc comments and `SoluteConfig::new_with_species_boundaries` (`:282`, which `SoluteConfig::new` delegates to) so `backend: SoluteBackend::FiniteVolumeBackwardEuler` is the default until genuine Rust-vs-Fortran parity passes across all 44 fixtures.
  - In `crates/hydrus-core/tests/solute_solver_selection.rs:41-170`:
    - Update `default_backend_is_fortran_compatible` (`:42`, rename to `default_backend_is_finite_volume_backward_euler`), `backend_selection_round_trips` (`:59`), `clone_and_equality_preserve_backend` (`:73`), and `default_advance_solute_step_uses_fortran_compatible_backend` (`:84`, rename to `default_advance_solute_step_uses_fv_be_and_explicit_fortran_compatible_is_selectable`) so the default is `SoluteBackend::FiniteVolumeBackwardEuler` (succeeding on the 2-node mesh) while explicit `with_backend(SoluteBackend::FortranCompatible)` enforces the 3-node requirement.
  - In `docs/decisions/README.md:24`:
    - Update the status note for ADR 0002 to state that `FiniteVolumeBackwardEuler` remains the default until the 44-fixture genuine Rust-vs-Fortran parity gate passes in Wave 6/7.
  - In `planning/master/SOLUTE_FORTRAN_PARITY_18.md`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`, and `planning/master/VALIDATION_STRATEGY.md`:
    - Add an explicit correction banner noting that the `18.15.1` parity run invoked the Fortran reference runner inside `tools/solute_rust_candidate_adapter.py` rather than the Rust compatibility solver, that the default backend has been reset to `FiniteVolumeBackwardEuler`, and that `iPsi=3` (`SOLUTE.FOR:1570-1579`) is an active column-6 continuation formula rather than a commented-out no-op.
- **Do not touch:** `tools/solute_benchmark_bridge.py` (orchestrator re-pins `POLICY_HASHES` at wave end per D6), `crates/hydrus-core/src/solute_fortran_compat/**`, or `TODO.md`.
- **Anchors:**
  - `crates/hydrus-core/src/solute.rs:118-168`, `:278-295`
  - `crates/hydrus-core/tests/solute_solver_selection.rs:41-170`
  - `docs/decisions/README.md:1-35`
  - `planning/master/SOLUTE_FORTRAN_PARITY_18.md:1-130`
- **Verification:**
  - `cargo test -p hydrus-core --lib solute`
  - `cargo test -p hydrus-core --test solute_solver_selection --test solute_backend_contract`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

### 20.1.2 — Deduplicate adapter helpers, reject moisture selector 2, rename `BlockingMode::Type3`

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `crates/hydrus-core/src/solute_fortran_compat/types.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/fields.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/mod.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/transport.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/reactions.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/transport_properties.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/nonequilibrium.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/biology.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/source_coupling.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/tests.rs`
  - `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md`
- **Can create:** `planning/tasks/20.1.2.todo.md`; scratch `.development/20.1.2/`.
- **Depends on:** Wave 0.
- **Spec:**
  - **Rename `BlockingMode::SourceNoOp3` → `BlockingMode::Type3`:**
    - In `crates/hydrus-core/src/solute_fortran_compat/types.rs:42-50`, rename `BlockingMode::SourceNoOp3` to `BlockingMode::Type3` and update its doc comment to cite the active column-6 continuation formula in `src_reference/SOLUTE.FOR:1570-1579`.
    - Update `crates/hydrus-core/src/solute_fortran_compat/adapter/mod.rs:577` (`3 => BlockingMode::Type3`) and any test references in `adapter/tests.rs`.
    - Update `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md:37` to state that `iPsi=3` (`SOLUTE.FOR:1570-1579`) is active via column-6 `!` continuations (`BlockingMode::Type3`), not commented out.
  - **Reject `moisture_selector == 2` (`iMoistDep == 2`) in the adapter:**
    - In `crates/hydrus-core/src/solute_fortran_compat/adapter/reactions.rs:99-101` (`maybe_moisture_dependency`), change `if d.selector > 2` to `if d.selector > 1` so `selector == 2` is rejected at request-parse time, matching `driver.rs:288`, `processes.rs:270`, and task `18.8.2` (`INPUT.FOR:1320, 1693`).
    - Add a unit test in `adapter/reactions.rs` (or `adapter/tests.rs`) asserting that `moisture_dependency` with `selector: 2` is rejected with an error naming `moisture_dependency.selector`.
  - **Deduplicate adapter production and test helpers:**
    - In `crates/hydrus-core/src/solute_fortran_compat/adapter/fields.rs`, provide `pub(super) fn require_wrapped_object(fields: &Map<String, Value>, field: &str) -> Result<(), String>` and `pub(super) fn reals(v: Vec<f64>) -> Vec<DefaultReal>` (plus `#[cfg(test)] pub(super) fn wrap(field: &str, units: &str, v: Value) -> Map<String, Value>` and `#[cfg(test)] pub(super) fn err_has(res: Result<impl std::fmt::Debug, String>, needle: &str)`).
    - Remove the duplicate `guard_wrapper_object` in `adapter/transport.rs:126-135`, the duplicate `require_wrapped_object` in `adapter/nonequilibrium.rs:7-12`, `adapter/biology.rs:7-12`, and `adapter/source_coupling.rs:8-13`, the duplicate `reals` in `adapter/transport.rs:68`, `adapter/nonequilibrium.rs:26`, and `adapter/source_coupling.rs:76`, and the duplicate `wrap` / `err_has` test helpers across `adapter/{transport,reactions,transport_properties,nonequilibrium,biology,source_coupling}.rs`, importing them from `super::fields::*` (or `super::*` via `adapter/mod.rs`).
- **Do not touch:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/solute_fortran_compat/{accounting,deposition,driver,iteration}.rs`, or `TODO.md`.
- **Anchors:**
  - `crates/hydrus-core/src/solute_fortran_compat/types.rs:42-50`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/mod.rs:570-585`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/fields.rs:1-120`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/reactions.rs:90-115`, `:240-310`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/transport.rs:65-75`, `:125-138`, `:340-360`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/nonequilibrium.rs:1-35`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/biology.rs:1-20`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/source_coupling.rs:1-20`, `:70-85`
  - `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md:30-45`
- **Verification:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat::adapter`
  - `HYDRUS_ADAPTER_SNAPSHOT=.development/20.1.2/snap.txt cargo test -p hydrus-core --lib solute_fortran_compat::adapter::tests::dump_parsed_fixtures -- --ignored`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

### 20.1.3 — Deduplicate `accounting.rs` deposition helpers and add `dual_exchange` group alias

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `crates/hydrus-core/src/solute_fortran_compat/accounting.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/deposition.rs`
  - `tools/run_solute_parity.py`
  - `tools/test_solute_fortran_parity.py`
- **Can create:** `planning/tasks/20.1.3.todo.md`; scratch `.development/20.1.3/`.
- **Depends on:** Wave 0.
- **Spec:**
  - In `crates/hydrus-core/src/solute_fortran_compat/accounting.rs`:
    - Delete the private `fn blocking` (`:375-397`) and `fn deposit` (`:409-450`).
    - In `accumulate_bacteria` (`:247-370`), call `super::deposition::{blocking, deposit, DepositInput}` directly.
    - **Critical:** Preserve `accumulate_bacteria`'s node-material indexing at `:265-266` (`ipsi2 = ch[mi][jj + 12].0 as i32; ipsi1 = ch[mj][jj + 13].0 as i32;`), which mirrors `src_reference/SOLUTE.FOR:1450-1451` (`Mi` for `iPsi2`, `Mj` for `iPsi1`).
    - In `accounting.rs` unit tests (`:795-835`), update `source_blocking_modes_match_active_biology_helper_equations` (`:815`) to call `super::super::deposition::blocking` (and verify `iPsi=3` uses `deposition::blocking`'s `powf(2.0)`/`powf(3.0)` matching Fortran `**2.`/`**3.`).
  - In `crates/hydrus-core/src/solute_fortran_compat/deposition.rs`:
    - Ensure `blocking`, `deposit`, and `DepositInput` have `pub(crate)` visibility so `accounting.rs` and `driver.rs` both call the single canonical implementation.
  - In `tools/run_solute_parity.py:17-24` (`FIXTURE_GROUPS`):
    - Add `"dual_exchange": "processes/dual_exchange"` so `--fixture-group dual_exchange` resolves the 4 fixtures in `fixtures/solute_fortran_parity/processes/dual_exchange/` (and running all named groups covers all 44 fixtures, matching `--fixture-group all`).
  - In `tools/test_solute_fortran_parity.py`:
    - Add unit tests verifying that `discover_fixture_manifests(fixture_group="dual_exchange")` discovers the 4 `dual_exchange` fixtures and that the union of all named keys in `FIXTURE_GROUPS` equals `discover_fixture_manifests(fixture_group="all")` (44 fixtures).
- **Do not touch:** `crates/hydrus-core/src/solute_fortran_compat/types.rs`, `crates/hydrus-core/src/solute_fortran_compat/adapter/**`, `tools/solute_rust_candidate_adapter.py`, or `TODO.md`.
- **Anchors:**
  - `crates/hydrus-core/src/solute_fortran_compat/accounting.rs:247-450`, `:795-835`
  - `crates/hydrus-core/src/solute_fortran_compat/deposition.rs:1-165`
  - `tools/run_solute_parity.py:17-65`
  - `tools/test_solute_fortran_parity.py:1-120`
- **Verification:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat::accounting`
  - `cargo test -p hydrus-core --lib solute_fortran_compat::deposition`
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_fortran_parity`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

---

## Wave 2 — Zero-allocation step/retry buffers and full `SoluteConfig` step mapping

### 20.2.1 — Preallocated scratch buffers in `assembly.rs`, `driver.rs`, and `iteration.rs`

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `crates/hydrus-core/src/solute_fortran_compat/assembly.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/driver.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/iteration.rs`
- **Can create:** `planning/tasks/20.2.1.todo.md`; scratch `.development/20.2.1/`.
- **Depends on:** Wave 1.
- **Spec:**
  - In `crates/hydrus-core/src/solute_fortran_compat/assembly.rs`:
    - Add `pub(crate) fn assemble_interior_into(inputs: AssemblyInputs<'_>, level: AssemblyLevel, epsi: f32, alf: f32, dt: f32, lower: &mut [f64], diagonal: &mut [f64], upper: &mut [f64], rhs: &mut [f64]) -> Result<(), KernelError>` so interior assembly writes directly into caller-supplied slices without allocating `vec![0.0; n]`. Keep `assemble_interior` as a thin wrapper calling `assemble_interior_into`.
  - In `crates/hydrus-core/src/solute_fortran_compat/driver.rs`:
    - Define a reusable `pub(crate) struct SoluteDriverScratch` (preallocating all per-pass working buffers in `begin_species`, `coeff_level`, `adjusted`, `mass_bufs`, `solve_pass`, `finish_species`, `finish_step`, and `EndpointPrimitiveState` / `InteriorRows` / `MassBufs` / `st.totals.s_trans` plus a backup `SoluteStepState` for nonlinear `dt/3` rollback) with `SoluteDriverScratch::new(node_count: usize, species_count: usize, material_count: usize) -> Self`.
    - Add `SoluteStepState::copy_from(&mut self, other: &Self)` (reusing existing `Vec` capacities via `copy_from_slice` without allocating) and a scratch-backed entry point `pub(crate) fn solve_pass_with_scratch(cfg: &SoluteStepConfig, st: &mut SoluteStepState, js: usize, iter: u32, scratch: &mut SoluteDriverScratch) -> Result<PassOutcome, KernelError>` (plus `pub(crate) fn solute_step_with_scratch`). Keep `solute_step`, `solute_step_from`, and `solve_pass` as thin convenience wrappers that delegate to the `*_with_scratch` functions.
    - Ensure that once `SoluteDriverScratch` is constructed (and `st.totals.s_trans` is pre-sized to `node_count`), `solve_pass_with_scratch` and `solute_step_with_scratch` perform **zero** heap allocations while preserving every arithmetic operation, order, and `f32`/`f64` cast (ADR-0010).
  - In `crates/hydrus-core/src/solute_fortran_compat/iteration.rs`:
    - Add `pub(crate) fn solve_solute_with_nonlinear_with_scratch(cfg: &mut SoluteStepConfig, st: &mut SoluteStepState, nl_cfg: &NonlinearConfig, scratch: &mut SoluteDriverScratch) -> Result<SoluteIterationReport, KernelError>` that snapshots `st` into `scratch` before label 10 and restores via `copy_from` on `dt / 3.0` retries without cloning `Vec`s on the heap.
    - Keep `solve_solute_with_nonlinear` delegating to `solve_solute_with_nonlinear_with_scratch`.
    - Add a unit test verifying that nonlinear retry (`dt/3` rollback and re-solve) produces bit-identical results between `solve_solute_with_nonlinear` and `solve_solute_with_nonlinear_with_scratch`.
- **Do not touch:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/tests/**`, or `TODO.md`.
- **Anchors:**
  - `crates/hydrus-core/src/solute_fortran_compat/assembly.rs:50-165`
  - `crates/hydrus-core/src/solute_fortran_compat/driver.rs:140-390`, `:445-1575`
  - `crates/hydrus-core/src/solute_fortran_compat/iteration.rs:190-270`
- **Verification:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat::assembly`
  - `cargo test -p hydrus-core --lib solute_fortran_compat::driver`
  - `cargo test -p hydrus-core --lib solute_fortran_compat::iteration`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

---

## Wave 2B — Full `SoluteConfig` mapping and zero-allocation compat step in `solute.rs`

### 20.2.2 — Full `SoluteConfig` mapping and zero-allocation compat step in `solute.rs`

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `crates/hydrus-core/src/solute.rs`
  - `crates/hydrus-core/tests/allocation_contract.rs`
  - `crates/hydrus-core/tests/solute_backend_contract.rs`
- **Can create:** `planning/tasks/20.2.2.todo.md`; scratch `.development/20.2.2/`.
- **Depends on:** 20.2.1 (runs immediately after `20.2.1` merges `SoluteDriverScratch`, `SoluteStepState::copy_from`, and `solve_solute_with_nonlinear_with_scratch`).
- **Spec:**
  - In `crates/hydrus-core/src/solute.rs`:
    - Extend `SoluteWorkspace` (`:429-484`) with preallocated compatibility buffers (`compat_cfg: SoluteStepConfig`, `compat_state: SoluteStepState`, and `compat_scratch: SoluteDriverScratch`). `SoluteWorkspace::new(node_count, species_count)` preallocates capacity for up to `DEFAULT_MAX_MATERIALS = 16` materials (and add `pub fn with_materials(node_count: usize, species_count: usize, material_count: usize) -> Result<Self, SoluteConfigError>` for meshes with more than 16 materials; if `mesh.material_count() > workspace.compat_material_capacity`, return `Err(invalid_transport("workspace", "mesh material count exceeds workspace capacity"))`).
    - Refactor `advance_fortran_compatible_step` (`:799-1009`) to populate `workspace.compat_cfg` and `workspace.compat_state` in place and call `iteration::solve_solute_with_nonlinear_with_scratch(&mut workspace.compat_cfg, &mut workspace.compat_state, &nonlin_cfg, &mut workspace.compat_scratch)` with **zero** heap allocations during the step.
    - Fix the `SoluteConfig` / `Mesh1D` / `SoluteProcessInput` field mappings in `advance_fortran_compatible_step` (`:840-988`):
      - Populate `mat_num[i] = mesh.material_indices()[i] + 1` (1-based `1..=mesh.material_count()`) instead of `vec![1; node_count]`, and populate `ch_par[0..mesh.material_count()]` (using `process_input.map_or(0.8_f32, |p| p.bulk_density[i] as f32)` per material's representative node).
      - Map per-species boundary conditions from `config.boundaries[js]` (`c_top[js]` and `c_bot[js]` for each species `js in 0..species_count`) instead of reading only `config.boundaries[0]`. Since Fortran `kTopCh`/`kBotCh` are scalar across species, if two species in `config.boundaries` specify different `SoluteBoundaryMode` variants at the same endpoint, return `Err(invalid_transport("boundary", "FortranCompatible backend requires uniform boundary mode across species"))`.
      - Map `EquilibriumSorption` and `sp.kinetic_sorption` (`Option<KineticSorption { capacity_coefficient, transfer_rate }>`):
        - Set `equil = config.species.iter().all(|sp| sp.kinetic_sorption.is_none())`.
        - When `sp.kinetic_sorption` is `Some(kin)` and `sp.equilibrium_sorption` is `EquilibriumSorption::None`, map the pure-kinetic sorption pool to Fortran's chemical nonequilibrium slots: `ch_par_row[2] = 0.0` (`Frac = 0.0`, all sorption in kinetic pool), `ch_par_row[offset + 6] = kin.capacity_coefficient as f32` (`Kd`), and `ch_par_row[offset + 19] = kin.transfer_rate as f32` (`Omega`). When both `EquilibriumSorption` (`Kd_eq`) and `KineticSorption` (`Kd_kin`) are active, set total `Kd = (Kd_eq + Kd_kin) as f32`, `Frac = (Kd_eq / (Kd_eq + Kd_kin)) as f32` (or `fraction` from `EquilibriumSorption::Nonlinear`), and `Omega = kin.transfer_rate as f32`.
        - Copy `workspace.trial_kinetic_sorbed` (`state.kinetic_sorbed()`) into `compat_state.sorb` before stepping and copy updated `compat_state.sorb` back into `workspace.trial_kinetic_sorbed` after stepping.
  - In `crates/hydrus-core/tests/allocation_contract.rs`:
    - Add tests asserting that `advance_solute_step` and `advance_solute_step_with_processes` perform `0` heap allocations after `SoluteWorkspace` construction for **both** `SoluteBackend::FiniteVolumeBackwardEuler` and `SoluteBackend::FortranCompatible`.
  - In `crates/hydrus-core/tests/solute_backend_contract.rs`:
    - Add tests verifying that `SoluteBackend::FortranCompatible` honors per-species boundary concentrations (`config.boundaries[1] != config.boundaries[0]`), multi-material `mesh.material_indices()`, and `KineticSorption` (`sorb` evolution).
- **Do not touch:** `tools/solute_benchmark_bridge.py` (orchestrator re-pins `POLICY_HASHES` at wave end per D6), `TODO.md`.
- **Anchors:**
  - `crates/hydrus-core/src/solute.rs:20-58`, `:429-485`, `:799-1010`
  - `crates/hydrus-core/src/hydraulics/types.rs:165-180`
  - `crates/hydrus-core/tests/allocation_contract.rs:1-145`
  - `crates/hydrus-core/tests/solute_backend_contract.rs:1-180`
- **Verification:**
  - `cargo test -p hydrus-core --lib solute`
  - `cargo test -p hydrus-core --test allocation_contract --test solute_backend_contract`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

---

## Wave 3 — Source-precision deck reader, coupled `WatFlow`/`DualPor`/`TmCont`, and `OUTPUT.FOR` formatters

### 20.3.1 — Source-compatible deck reader (`SELECTOR.IN` + `PROFILE.DAT` + `CASE.json`)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/mod.rs`
- **Can create:**
  - `crates/hydrus-core/src/solute_fortran_compat/deck_reader.rs`
  - `planning/tasks/20.3.1.todo.md`; scratch `.development/20.3.1/`.
- **Depends on:** Wave 2B (`20.2.2`).
- **Spec:**
  - Create `crates/hydrus-core/src/solute_fortran_compat/deck_reader.rs` and declare `#[path = "../deck_reader.rs"] #[allow(dead_code)] // wired by 20.4.1 pub(crate) mod deck_reader;` in `crates/hydrus-core/src/solute_fortran_compat/adapter/mod.rs` (keeping `solute_fortran_compat/mod.rs` and `types.rs` untouched so `20.3.1`, `20.3.2`, and `20.3.3` have disjoint file sets; `20.4.1` moves the module declaration to `solute_fortran_compat/mod.rs`).
  - Keep production code in `deck_reader.rs` free of `std::fs`/`std::path` (operating on `case_json: &str, selector_in: &str, profile_dat: &str`).
  - Port the fixed-deck reader routines from `src_reference/INPUT.FOR` for the active parity fixture scope:
    - `BasInf` (`INPUT.FOR:3-178`) & `Conversion` (`:182-204`): `Kat`, `MaxIt`, `TolTh`, `TolH`, `lWat`, `lChem`, `lTemp`, `SinkF`, `lRoot`, `ShortO`, `lWDep`, `lScreen`, `AtmBC`, `lEquil`, `lInverse`, `lSnow`, `lHP1`, `lMeteo`, `lVapor`, `lActRSU`, `lFlux`, `lIrrig`, `lDummy`, `NMat`, `NLay`, `CosAlf`, `xConv`, `tConv`, `KodTop`, `KodBot`, `rTop`, `rBot`, `rRoot`, `hCritS`, `hCritA`, `GWL0L`, `Aqh`, `Bqh`, `kTopCh`, `cTop`, `kBotCh`, `cBot`, `dSurf`, `cAtm`, `tPulse`, `iModel`, `iHyst`, `iDualPor`, `lUpW`, `lArtD`, `lTDep`, `cTolA`, `cTolR`, `MaxItC`, `PeCr`, `NoSol`, `lTort`, `iBact`, `lFiltr`, `nChPar`, `iMoistDep`, `OmegaC`, `lMassIni`, `lEqInit`, `iTort`.
    - `MatIn` (`INPUT.FOR:536-728`): `Par(1..10, M)` for `M=1..NMat` (including `ParD(1..11, M)` when `iModel == 6` or `7`), applying `Par(5, M) = Par(5, M) / (xConv / tConv)`.
    - `TmIn` (`INPUT.FOR:866-930`): `dt`, `dtMin`, `dtMax`, `dMul`, `dMul2`, `ItMin`, `ItMax`, `MPL`, `tInit`, `tMax`, `TPrint(1..MPL)`, plus `TPrint(MPL + 1) = tMax + dtMax`.
    - `ChemIn` (`INPUT.FOR:1289-1460`): `Epsi`, `lUpW`, `lArtD`, `lTDep`, `cTolA`, `cTolR`, `MaxItC`, `PeCr`, `NoSol`, `lTort`, `iBact`, `lFiltr`, `ChPar(1..nChPar, M)` per species and material, `TDep(1..nChPar)`, `kTopCh`, `cTop(1..NS)`, `kBotCh`, `cBot(1..NS)`, `dSurf`, `cAtm`, `tPulse`, plus `ChemIn`'s exact `xConv`/`tConv` scaling of `ChPar`, `cBound`, `dSurf`, and `tPulse`, and `Linear(j)` computation (`INPUT.FOR:1348, 1368-1371`).
    - `NodInf` (`INPUT.FOR:208-468`): read `PROFILE.DAT` (`NumNP` nodes and `NObs` observation node indices), reverse node storage (`n = NumNP - i + 1` per `INPUT.FOR:276`), compute `dx`, `xSurf = x(NumNP)`, apply `Conversion(NumNP, NSD, xConv, Conc, Sorb, lBact)` when `lMassIni` (`INPUT.FOR:182-204`), and read `Beta`, `Ah`, `AK`, `ATh`, `TempO`/`TempN`, `Sorb`, `Sorb2`, `ThNew`/`ThOld` (when `iDualPor > 0` and `iModel < 8`).
  - Also parse and cross-validate `CASE.json` via `adapter::parse_case_manifest`, checking that the parsed deck parameters match `CASE.json`'s `CompatibilityRequest`.
  - In `#[cfg(test)]` inside `deck_reader.rs`, add a unit test that walks all 44 parity fixture directories under `fixtures/solute_fortran_parity/`, reads `CASE.json` + `SELECTOR.IN` + `PROFILE.DAT` from disk, and asserts 44/44 parse cleanly with matching node count, species count, boundary codes, `ChPar`, and print schedule.
- **Do not touch:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/solute_fortran_compat/{mod,types,water_compat,output_compat}.rs`, or `TODO.md`.
- **Anchors:**
  - `src_reference/INPUT.FOR:3-204`, `:208-468`, `:536-728`, `:866-930`, `:1289-1460`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/mod.rs:115-210`
- **Verification:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat::adapter::deck_reader`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

### 20.3.2 — Source-precision `WatFlow`, `DualPor`, `SetMat`, `Veloc`, and `TmCont` port

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `crates/hydrus-core/src/solute_fortran_compat/mod.rs` (adds `#[allow(dead_code)] // wired by 20.4.1 pub(crate) mod water_compat;`)
- **Can create:**
  - `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs`
  - `planning/tasks/20.3.2.todo.md`; scratch `.development/20.3.2/`.
- **Depends on:** Wave 2B (`20.2.2`).
- **Spec:**
  - Create `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs` and declare `#[allow(dead_code)] // wired by 20.4.1 pub(crate) mod water_compat;` in `crates/hydrus-core/src/solute_fortran_compat/mod.rs`.
  - Port the pinned Fortran water, material-table, dual-porosity, and time-control routines in exact source precision (`f32` arrays/scalars and `f64` `P`/`R`/`S` tridiagonal solve and `t`/`dt`, ADR-0010 & ADR-0015):
    - `MATERIAL.FOR`: `FK` (`:16-197`), `FC` (`:201-275`), `FQ` (`:279-346`), `FH` (`:350-417`), `FS` (`:421-460`) for `iModel = 0` (van Genuchten, with `iModel = 6` and `7` routed to `iModel = 0` for the fracture/macropore domain per `WATFLOW.FOR:440-441` and `INPUT.FOR:769-770`).
    - `INPUT.FOR`:
      - `GenMat` (`:762-862`): 100-entry logarithmic material tables `hTab(1..100)`, `ConTab(1..100, M)`, `CapTab(1..100, M)`, `TheTab(1..100, M)`, `ConSat(M)`, `ths(M)`, `thr(M)`.
      - `InitW` (`:472-504`): initial `ThNew`/`ThOld`, `Con`, `Cap`, `hTemp`, `hOld` when `iDualPor == 0` vs `iDualPor > 0`.
      - `InitDualPor` (`:508-532`): initial matrix water content `ThNewIm`/`ThOldIm`, `hNew(i)` equilibration when `lEqInit`, and `SinkIm(i) = 0.0`.
    - `WATFLOW.FOR`:
      - `SetMat` (`:433-568`, including table interpolation on `abs(hTab(1))`..`abs(hTab(NTab))` and `ConVapor`/`VaporContent` calls when `lVapor`).
      - `WatFlow` (`:3-169`), `Reset` (`:173-304`), `Gauss` (`:308-355`), `Shift` (`:359-429`), `Veloc` (`:717-774`), `ConVapor` (`:849-926`), `VaporContent` (`:930-962`), `ConVh` (`:968-997`), `Update` (`:1079-1126`).
      - For `DualPor` (`WATFLOW.FOR:44-46`, `:1027-1075`), delegate to `super::dual_domain::dual_porosity_exchange` (`crates/hydrus-core/src/solute_fortran_compat/dual_domain.rs:394`) so there is only one implementation of `DualPor` (`STATE_18.md:74`).
      - Note on `WATFLOW.FOR:132`: `* if(TLevel.eq.1) Eps=0.` has `*` in column 1 (a fixed-form comment!), so `Eps` is NOT zeroed on `TLevel == 1`.
    - `TIME.FOR`:
      - `TmCont` (`:3-32`): step adjustment (`dMul`, `dMul2`, `dtMin`, `dtMax`, `ItMin`, `ItMax`, `TPrint(PLevel)`, `tMax`, `dtOpt`).
  - Add unit tests in `water_compat.rs` verifying:
    - `GenMat` and `SetMat` table interpolation against direct `FK`/`FC`/`FQ` evaluation;
    - Steady/transient `WatFlow` + `Veloc` step convergence on a 21-node column;
    - `InitDualPor` + `WatFlow` + `DualPor` wetting transient for Models 6 and 7 (`ThNewIm` and `SinkIm` evolution over `t = 0..4`);
    - `TmCont` print-time snapping and `dtOpt` restoration.
- **Do not touch:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/solute_fortran_compat/{types,adapter/**}`, or `TODO.md`.
- **Anchors:**
  - `src_reference/MATERIAL.FOR:16-460`
  - `src_reference/INPUT.FOR:472-532`, `:762-862`
  - `src_reference/WATFLOW.FOR:3-568`, `:717-774`, `:849-997`, `:1027-1126`
  - `src_reference/TIME.FOR:3-32`
  - `crates/hydrus-core/src/solute_fortran_compat/dual_domain.rs:300-460`
- **Verification:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat::water_compat`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

### 20.3.3 — Source-precision `OUTPUT.FOR` table formatters (`Profil`, `NodOut`, `TLInf`, `ObsNod`, `SubReg`)

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `crates/hydrus-core/src/solute_fortran_compat/types.rs` (adds `#[path = "output_compat.rs"] #[allow(dead_code)] // wired by 20.4.1 pub(crate) mod output_compat;`)
- **Can create:**
  - `crates/hydrus-core/src/solute_fortran_compat/output_compat.rs`
  - `planning/tasks/20.3.3.todo.md`; scratch `.development/20.3.3/`.
- **Depends on:** Wave 2B (`20.2.2`).
- **Spec:**
  - Create `crates/hydrus-core/src/solute_fortran_compat/output_compat.rs` and declare `#[path = "output_compat.rs"] #[allow(dead_code)] // wired by 20.4.1 pub(crate) mod output_compat;` in `crates/hydrus-core/src/solute_fortran_compat/types.rs` (keeping `mod.rs` and `adapter/mod.rs` untouched for Wave 3 disjointness; `20.4.1` moves the module declaration to `solute_fortran_compat/mod.rs`).
  - Port the output formatting and sub-region integration routines from `src_reference/OUTPUT.FOR` into pure in-memory `String` writers matching the columns and numeric precision expected by `tools/solute_parity_normalizers.py`:
    - `Profil` (`OUTPUT.FOR:931-980`) → `PROFILE.OUT` (`n = NumNP..1` top-to-bottom node index `i = NumNP - n + 1`, `x`, `hOld`, `MatNum`, `LayNum`, `Beta`, `Ah`, `AK`, `ATh`, etc.).
    - `NodOut` (`OUTPUT.FOR:720-927`) → `NOD_INF.OUT` (`Time: t`, top-to-bottom rows with `Node`, `Depth`, `Head`, `Moisture`, `K`, `C`, `Flux`, `Sink`, `Kappa`, `v/KsTop`, `Temp`, `Conc(1..NS)`, `Sorb(1..NS)`) and `NOD_INF_V.OUT` when `lVapor` (`Node`, `Depth`, `ConVh`, `ConVt`, `ConVapor`, `vVapor`, `ThVapor`, `Sorb2(1..NS)`).
    - `TLInf` (`OUTPUT.FOR:3-292`, specifically `:205-225`) → `solute1..NS.out` header and per-step data row (`t`, `cvTop`, `cvBot`, `Sum(cvTop)`, `Sum(cvBot)`, `cvCh0`, `cvCh1`, `cTop`, `cRoot`, `cBot`, `cvRoot`, `Sum(cvRoot)`, `Sum(cvCh0)`, `Sum(cvCh1)`, `TLevel`, `cgvTop`, `cgvBot`, plus the 6 `lFlux` columns `FluxC_top`, `FluxC_bot`, `CumFlux_top`, `CumFlux_bot`, `CumAdv_top`, `CumAdv_bot` when `lFlux` is true).
    - `ObsNod` (`OUTPUT.FOR:983-1113`) → `OBS_NODE.OUT` header and per-step row for `NObs` observation nodes (`t`, and for each obs node `hNew`, `ThNew`, `TempN` or `FluxConc` when `lFlux` per `:1084`, `Conc(1..NS)`).
    - `SubReg` (`OUTPUT.FOR:325-716`) → `BALANCE.OUT` (`Time [T]`, `Area [L]`, `W-volume [L]`, `W-volumeI[L]` when `iDualPor > 0`, `In-flow [L/T]`, `h Mean [L]`, `Top Flux [L]`/`Bot Flux [L]` when `lWat`, `cVolI` / `sVolI` / `ConcVol` / `SorbVol` / `SorbVol2` per species, `WatBalT [L]`, `WatBalR [%]`, `CncBalT [M]`, `CncBalR [%]`).
  - Add unit tests in `output_compat.rs` verifying that formatted `PROFILE.OUT`, `NOD_INF.OUT`, `NOD_INF_V.OUT`, `solute1.out` (17-col and 23-col `lFlux`), `OBS_NODE.OUT`, and `BALANCE.OUT` blocks match the exact headers, row selectors, and column indices parsed by `tools/solute_parity_normalizers.py`.
- **Do not touch:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/solute_fortran_compat/{mod,adapter/**,water_compat}.rs`, or `TODO.md`.
- **Anchors:**
  - `src_reference/OUTPUT.FOR:205-225`, `:325-1113`
  - `tools/solute_parity_normalizers.py:130-390`
- **Verification:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat::types::output_compat`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

---

## Wave 4 — Full-run compatibility driver and removal of all `dead_code` allowances

### 20.4.1 — Full-run compatibility runner and removal of all `dead_code` allowances

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `crates/hydrus-core/src/solute_fortran_compat/mod.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/types.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/mod.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/adapter/fields.rs`
  - `crates/hydrus-core/src/solute_fortran_compat/iteration.rs`
  - `crates/hydrus-core/src/lib.rs`
  - `crates/hydrus-core/tests/solute_fortran_compat_harness.rs`
- **Can create:**
  - `crates/hydrus-core/src/solute_fortran_compat/compat_runner.rs`
  - `planning/tasks/20.4.1.todo.md`; scratch `.development/20.4.1/`.
- **Depends on:** Wave 3 (`20.3.1`, `20.3.2`, `20.3.3`).
- **Spec:**
  - Move the module declarations `pub(crate) mod deck_reader;` (from `adapter/mod.rs`) and `pub(crate) mod output_compat;` (from `types.rs`) into `crates/hydrus-core/src/solute_fortran_compat/mod.rs`, create `crates/hydrus-core/src/solute_fortran_compat/compat_runner.rs`, and expose `run_compat_deck(case_json: &str, selector_in: &str, profile_dat: &str) -> Result<CompatRunOutputs, String>` via `lib.rs` (`pub use solute_fortran_compat::compat_runner::{run_compat_deck, CompatRunOutputs};`).
  - **Architectural constraint (filesystem-free production code in `hydrus-core`):**
    - `compat_runner::run_compat_deck` operates purely on `&str` inputs and returns `CompatRunOutputs` with `String` buffers (`profile_out`, `nod_inf_out`, `nod_inf_v_out: Option<String>`, `solute_outs: Vec<String>`, `obs_node_out: Option<String>`, `balance_out`).
    - File reading/writing and SHA-256 verification of on-disk `SELECTOR.IN`/`PROFILE.DAT` happen in `crates/hydrus-core/tests/solute_fortran_compat_harness.rs` (and in `tools/solute_rust_candidate_adapter.py`).
  - **Port the `HYDRUS.FOR` initialization and time loop (`HYDRUS.FOR:195-700`) in `compat_runner.rs`:**
    - Initialization (`HYDRUS.FOR:195-392`):
      - Parse deck via `deck_reader`, run `water_compat` initialization (`InitW`, `GenMat`, `SetMat`, `InitDualPor`), `initialization::initialize_sources` (`MassInit` `SOLUTE.FOR:1803-1910`, `NonEqInit` `:1637-1709`), format initial `PROFILE.OUT` (`output_compat::write_profil`), initial `NOD_INF.OUT`/`NOD_INF_V.OUT` (`output_compat::write_nod_out` at `tInit`), initial `BALANCE.OUT` (`output_compat::write_sub_reg` with `PLevel = 0`), run initial `Veloc`, set `vNew = vOld; ThNew = ThOld`, and if `lBact && !lWat` run `source_coupling::size_exclusion` (`Exclusion`, `HYDRUS.FOR:390-391`).
    - Time loop (`HYDRUS.FOR:399-700`):
      - At each `TLevel`:
        - If `lWat && (TLevel > 1 || lWTDep)`: run `water_compat::watflow_step` (with `dt` retry sync when `WatFlow` reduces `dt`) and `water_compat::veloc`.
        - Run `iteration::solve_solute_with_nonlinear_with_scratch` (using `iteration::check_linear_predicate` and `driver::solve_pass_with_scratch`, which invokes `boundary::canonical_endpoint`, `deposition::deposit_nodes`, `accounting`, etc.). If `Solute` reduces `dt` (`!lWat` nonlinear retry), update `dt` and `t`.
        - Append `TLInf` step record to `solute1..NS.out` (`output_compat::write_tlinf_step`).
        - If `NObs > 0`: append `ObsNod` row to `OBS_NODE.OUT` (`output_compat::write_obs_node_step`).
        - If `abs(t - TPrint(PLevel)) < 0.001 * dt`: append `NodOut` (`NOD_INF.OUT` and `NOD_INF_V.OUT` when `lVapor`), append `SubReg` (`BALANCE.OUT`), and increment `PLevel`.
        - Terminate when `abs(t - tMax) <= 0.001 * dt` (`HYDRUS.FOR:642`); otherwise call `water_compat::tm_cont` (`TmCont`), advance `t = t + dt`, and call `water_compat::update` (`Update`).
  - **Remove all `#[allow(dead_code)]` annotations in `solute_fortran_compat`:**
    - In `crates/hydrus-core/src/solute_fortran_compat/mod.rs`, remove every `#[allow(dead_code)]` attribute across all submodules.
    - In `crates/hydrus-core/src/solute_fortran_compat/iteration.rs:29,57,86,116,144,157,189,211`, remove all 8 item-level `#[allow(dead_code)] // wired by 18.12.1` attributes.
    - In `crates/hydrus-core/src/solute_fortran_compat/adapter/fields.rs:182` and `adapter/mod.rs:54`, wire or gate any test-only helper with `#[cfg(test)]` so zero `#[allow(dead_code)]` annotations remain in `solute_fortran_compat/`.
  - **Extend `crates/hydrus-core/tests/solute_fortran_compat_harness.rs`:**
    - Add a fixture runner entrypoint (`run_parity_fixture_from_env` test + helper `run_parity_fixture_bundle(manifest_path, out_dir)`) that reads `CASE.json`, verifies `request_sha256` and `input_files` SHA-256 digests (`SELECTOR.IN`, `PROFILE.DAT`), calls `run_compat_deck`, and writes `PROFILE.OUT`, `NOD_INF.OUT`, `NOD_INF_V.OUT` (if present), `solute1..NS.out`, `OBS_NODE.OUT` (if present), `BALANCE.OUT`, and `run-manifest.json` to `out_dir`.
    - Add integration tests in `tests/solute_fortran_compat_harness.rs` running representative fixtures across the parity groups and asserting clean completion.
- **Do not touch:** `crates/hydrus-core/src/solute.rs`, `tools/**`, or `TODO.md`.
- **Anchors:**
  - `src_reference/HYDRUS.FOR:195-700`
  - `crates/hydrus-core/src/solute_fortran_compat/mod.rs:1-30`
  - `crates/hydrus-core/src/solute_fortran_compat/iteration.rs:1-265`
  - `crates/hydrus-core/tests/solute_fortran_compat_harness.rs:1-573`
- **Verification:**
  - `cargo test -p hydrus-core --lib solute_fortran_compat`
  - `cargo test -p hydrus-core --test solute_fortran_compat_harness`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

---

## Wave 5 — True Rust candidate adapter & unit test suite

### 20.5.1 — Replace `solute_rust_candidate_adapter.py` with genuine Rust candidate execution

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `tools/solute_rust_candidate_adapter.py`
  - `tools/test_solute_fortran_parity.py`
- **Can create:** `planning/tasks/20.5.1.todo.md`; scratch `.development/20.5.1/`.
- **Depends on:** Wave 4 (`20.4.1`).
- **Spec:**
  - In `tools/solute_rust_candidate_adapter.py`:
    - Completely remove the import and invocation of `tools.run_fortran_reference.run_fixture` (`:69`). `solute_rust_candidate_adapter.py` must **never** import `run_fortran_reference` or execute the Fortran binary.
    - Implement `run_candidate(fixture_manifest_path, task_root, *, backend="fortran-compatible", timeout_seconds=None)` so that it:
      1. Validates `task_root`, loads `CASE.json`, verifies `request_sha256` against the canonical JSON of `request`, and verifies the SHA-256 of every file in `input_files` (`SELECTOR.IN`, `PROFILE.DAT`).
      2. Invokes the Rust compatibility harness (`cargo test` / cached `solute_fortran_compat_harness` test binary with `HYDRUS_COMPAT_FIXTURE_MANIFEST` and `HYDRUS_COMPAT_OUT_DIR` pointing inside `task_root / "rust_candidate_run"`).
      3. Verifies the generated Rust `run-manifest.json` and raw output file SHA-256 digests (`PROFILE.OUT`, `NOD_INF.OUT`, `solute1.out`, `BALANCE.OUT`, etc.).
      4. Normalizes the Rust-generated output files using `normalize_fortran_outputs(obs_map_path, rust_run_manifest_path, task_root_path, fixture_manifest_path=fixture_path)` from `tools.solute_parity_normalizers` (since that function is a pure output-file parser for `OBSERVATION_MAP.json`), rewrites `"source": "rust-fortran-compatible"`, writes `candidate_observations.json` and `candidate_run_manifest.json` under `task_root`, and returns the candidate result dictionary expected by `tools/run_solute_parity.py`.
  - In `tools/test_solute_fortran_parity.py`:
    - Add unit tests proving:
      - `tools.solute_rust_candidate_adapter` does not import or call `tools.run_fortran_reference` (patch `tools.run_fortran_reference.run_fixture` to raise `AssertionError("Fortran runner must not be called by Rust candidate adapter")` during a real `run_candidate` invocation on a parity fixture!).
      - `run_candidate` rejects tampered `request_sha256` or tampered `SELECTOR.IN`/`PROFILE.DAT` hashes before invoking Rust.
      - `run_candidate` executes the real Rust harness on a fixture and produces valid normalized `candidate_observations.json`.
- **Do not touch:** `crates/**`, `tools/run_fortran_reference.py`, `tools/solute_benchmark_bridge.py`, or `TODO.md`.
- **Anchors:**
  - `tools/solute_rust_candidate_adapter.py:1-136`
  - `tools/run_solute_parity.py:210-360`
  - `tools/solute_parity_normalizers.py:390-553`
  - `tools/test_solute_fortran_parity.py:350-550`
- **Verification:**
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_fortran_parity`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

---

## Wave 6 — 44-fixture all-mode Rust-vs-Fortran parity verification & evidence update

### 20.6.1 — Execute 44-fixture all-mode Rust-vs-Fortran parity gate and update evidence

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `planning/master/SOLUTE_FORTRAN_PARITY_18.md`
  - `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`
  - `planning/master/VALIDATION_STRATEGY.md`
  - *(If and only if the 44-fixture parity run uncovers a source-precision or output-formatting discrepancy in the new Wave 3/4 modules, this task may also make surgical fixes in `crates/hydrus-core/src/solute_fortran_compat/{deck_reader,water_compat,output_compat,compat_runner}.rs` and add corresponding regression tests.)*
  - *(Scope amendment 2026-10-18, orchestrator decision: the parity run aborts on fixtures `0004-blocking_type_3_noop` and `filtration_deposition` with `endpoint primitives must be finite` (`boundary.rs:198-241`, `driver.rs`). This task may also make surgical fixes in `crates/hydrus-core/src/solute_fortran_compat/{boundary,driver}.rs` to match pinned Fortran behavior for these biology fixtures, with regression tests. No other files.)*
  - *(Scope amendment 2026-10-18, round 2, user-approved design change: pinned Fortran output for `blocking_type_3_noop` is NaN (`iPsi=3`, `SMax=0`, `SOLUTE.FOR:1561-1594`, `:710`). This task may (a) relax the ADR-0014 finiteness guard in `boundary.rs`/`driver.rs` so NaN primitives propagate as in Fortran, (b) reproduce gfortran `dt` semantics under NaN in `driver.rs` (`dt_max.min(...)`, ~line 1461), and (c) add a short ADR note under `docs/decisions/` recording the NaN-propagation rule. Regression tests required. Still no tolerance changes.)*
  - *(Scope amendment 2026-10-19, round 3, user-approved: the gate ran to completion at 7/44 pass (37 fail), mostly with a shared `cvTop` mismatch (Rust ~1.19e-6 vs Fortran ~0.43) across unrelated fixtures. This task may (a) measure the effect of the `compat_runner.rs` first-step `bas.l_wat` change by reverting it, and keep it only if justified by `HYDRUS.FOR:416-419`; (b) diagnose the single root cause of the shared `cvTop` mismatch and fix it in `solute_fortran_compat/` against pinned source, with regression tests. Wave 7 stays blocked until 44/44.)*
  - *(Scope amendment 2026-10-19, round 4, user-approved: gate at 20/44. This task may fix the water-flow top-boundary flux (`transport-baseline-storage`, `endpoint-*`; Rust top velocity ~-1.19e-6 vs pinned `BALANCE.OUT` Top Flux -0.43) in `solute_fortran_compat/{water_compat,deck_reader,boundary}.rs` against pinned `WATFLOW.FOR`/`BALANCE` source, then diagnose the remaining `transport_properties`, `dual_exchange`, and `transport` failures. The FV/BE solver path stays unchanged. Also strengthen the biology harness tests so they compare against the Rust candidate rather than only normalizing Fortran outputs.)*
  - *(Scope amendment 2026-10-20, round 5, user-approved broad water-solver fix: gate at 34/44. Remaining: `dual_exchange` (4, `head` ~1% slow in dual-porosity coupling), `transport_properties` (5, transient top flux held constant in Rust), `reactions/chain_distinct_rates` (1, `cumulative_reaction`). This task may edit any file under `crates/hydrus-core/src/solute_fortran_compat/` (including `dual_domain.rs`, `water_compat.rs`, `boundary.rs`, `driver.rs`) and the matching harness tests, checked against pinned `WATFLOW.FOR`, `DUAL.FOR`/dual-porosity source, and `TIME.FOR`. The FV/BE solver path, `solute.rs`, the bridge, `TODO.md`, and Cargo files stay untouched. Tolerances stay unchanged.)*
- **Can create:** `planning/tasks/20.6.1.todo.md`; scratch `.development/20.6.1/`.
- **Depends on:** Wave 5 (`20.5.1`).
- **Spec:**
  - Run the full 44-fixture paired Fortran-vs-Rust parity suite using the real Rust candidate adapter:
    - `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group all --backend fortran-compatible --task-root .development/20.6.1/all_parity`
    - Also verify each named group (`transport`, `reactions`, `transport_properties`, `nonequilibrium`, `dual_exchange`, `biology`, `flux_concentration`).
  - If any of the 44 fixtures reports a divergence (`divergent_count > 0`):
    - Diagnose the exact earliest divergence (input conversion, `WatFlow`/`SetMat`/`DualPor`, `Solute` kernel, or `OUTPUT.FOR` formatting) against `src_reference/`.
    - Never widen tolerances in `OBSERVATION_MAP.json` or `run_solute_parity.py`. Fix the Rust compat implementation in `{deck_reader,water_compat,output_compat,compat_runner}.rs` to match pinned Fortran behavior and re-run until all 44/44 fixtures pass with `0` divergences.
  - Update `planning/master/SOLUTE_FORTRAN_PARITY_18.md`, `planning/master/PHASE_5_SOLUTE_EVIDENCE.md`, and `planning/master/VALIDATION_STRATEGY.md`:
    - Record the authenticated 20.6.1 parity run (command, timestamp, commit, 44/44 pass across all 7 named groups, distinct Fortran reference vs Rust candidate SHA-256 digests, active `iPsi=3` coverage, and coupled `WatFlow`/`DualPor` coverage for `W-volumeI[L]`).
    - Preserve the scientific disclaimer that `FortranCompatible` establishes pinned-source numerical parity, whereas continuum/mathematical accuracy is tracked separately by the FV/BE analytical and convergence suites.
- **Do not touch:** `crates/hydrus-core/src/solute.rs` (Wave 7 owns the default backend switch), `tools/solute_benchmark_bridge.py`, or `TODO.md`.
- **Anchors:**
  - `planning/master/SOLUTE_FORTRAN_PARITY_18.md:1-140`
  - `planning/master/PHASE_5_SOLUTE_EVIDENCE.md:1-120`
  - `planning/master/VALIDATION_STRATEGY.md:1-120`
- **Verification:**
  - `PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_parity.py --fixture-group all --backend fortran-compatible --task-root .development/20.6.1/all_parity`
  - `cargo test --workspace`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

---

## Wave 7 — Default backend switch to `FortranCompatible` gated on 44/44 genuine parity

### 20.7.1 — Switch default backend to `FortranCompatible` after 44/44 parity gate

- **Repo:** `/home/andrew/AI/hyd.rs`.
- **Must edit:**
  - `crates/hydrus-core/src/solute.rs`
  - `crates/hydrus-core/tests/solute_solver_selection.rs`
  - `docs/decisions/README.md`
- **Can create:** `planning/tasks/20.7.1.todo.md`; scratch `.development/20.7.1/`.
- **Depends on:** Wave 6 (`20.6.1` accepted with 44/44 genuine Rust-vs-Fortran parity).
- **Spec:**
  - In `crates/hydrus-core/src/solute.rs:118-150` and `SoluteConfig::new_with_species_boundaries` (`:278-295`):
    - Switch `SoluteConfig::new_with_species_boundaries` default backend to `SoluteBackend::FortranCompatible` (`SoluteBackend::FiniteVolumeBackwardEuler` remains explicitly selectable via `SoluteConfig::with_backend(SoluteBackend::FiniteVolumeBackwardEuler)`).
    - Update `solute.rs` doc comments to reflect the completed 20.6.1 parity verification.
  - In `crates/hydrus-core/tests/solute_solver_selection.rs`:
    - Update the default selection tests (`default_backend_is_fortran_compatible`, `backend_selection_round_trips`, `clone_and_equality_preserve_backend`, and `default_advance_solute_step_uses_fortran_compatible_backend`) to verify that default `SoluteConfig::new` selects `SoluteBackend::FortranCompatible`, produces bit-identical results to explicit `SoluteBackend::FortranCompatible`, differs from `SoluteBackend::FiniteVolumeBackwardEuler` on non-trivial transport/process cases, and preserves all explicit FV/BE behavior.
  - In `docs/decisions/README.md:24`:
    - Update the ADR 0002 status note to record that `SoluteBackend::FortranCompatible` is the default backend following the verified 44/44 genuine Rust-vs-Fortran parity gate in `20.6.1`.
  - At wave end, the orchestrator re-pins `POLICY_HASHES["crates/hydrus-core/src/solute.rs"]` in `tools/solute_benchmark_bridge.py` under D6 and verifies all Rust and Python test suites.
- **Do not touch:** `tools/solute_benchmark_bridge.py` (owned by orchestrator at wave gate per D6), `TODO.md`.
- **Anchors:**
  - `crates/hydrus-core/src/solute.rs:118-168`, `:278-295`
  - `crates/hydrus-core/tests/solute_solver_selection.rs:41-170`
  - `docs/decisions/README.md:1-35`
- **Verification:**
  - `cargo test -p hydrus-core --lib solute`
  - `cargo test -p hydrus-core --test solute_solver_selection --test solute_backend_contract --test solute_acceptance`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

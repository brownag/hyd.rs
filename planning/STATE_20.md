# State — Orchestration 20

Plan status: **Approved by user on 2026-10-15 (“yes”).**
Execution status: **Waves 0, 1, 2, and 2B complete; ready for Wave 3 dispatch.**
Plan: [EXECUTION_PLAN_20.md](EXECUTION_PLAN_20.md). Worker template: [PROMPT_TEMPLATE_20.md](PROMPT_TEMPLATE_20.md).

## Baseline and guardrails

- [x] Reconnaissance 2026-10-15 at `76b832c` on branch `phase4-water-acceptance`:
  - `cargo test --workspace`: **384 passed**, 0 failed, **1 ignored** (`dump_parsed_fixtures`).
  - `cargo test -p hydrus-core --lib solute_fortran_compat`: **218 passed**, 0 failed, **1 ignored**.
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: **313 passed**, 0 failed (`OK`).
  - Pre-existing uncommitted file to preserve untouched: `TODO.md`.
- [x] User decisions D1–D6 recorded in [EXECUTION_PLAN_20.md](EXECUTION_PLAN_20.md).
- [x] Two-axis plan review (`Spec` + `Standards`) completed and applied (2026-10-15).
- [x] User approves this execution plan.
- [ ] Genuine 44/44 all-mode Rust-vs-Fortran parity verified (`20.6.1`) before default backend switches to `SoluteBackend::FortranCompatible` (`20.7.1`).

## Wave board

| Wave | Task | Deliverable | Status |
|---|---|---|---|
| 0 | — | Orchestrator baseline at `76b832c` | done |
| 1 | 20.1.1 | Revert unverified default backend to `FiniteVolumeBackwardEuler` | done |
| 1 | 20.1.2 | Deduplicate adapter helpers, reject moisture selector 2, rename `BlockingMode::Type3` | done |
| 1 | 20.1.3 | Deduplicate `accounting.rs` deposition helpers and add `dual_exchange` group alias | done |
| 2 | 20.2.1 | Preallocated scratch buffers in `assembly.rs`, `driver.rs`, and `iteration.rs` | done |
| 2B | 20.2.2 | Full `SoluteConfig` mapping and zero-allocation compat step in `solute.rs` | done |
| 3 | 20.3.1 | Source-compatible deck reader (`SELECTOR.IN` + `PROFILE.DAT` + `CASE.json`) | done |
| 3 | 20.3.2 | Source-precision `WatFlow`, `DualPor`, `SetMat`, `Veloc`, and `TmCont` port | done |
| 3 | 20.3.3 | Source-precision `OUTPUT.FOR` table formatters (`Profil`, `NodOut`, `TLInf`, `ObsNod`, `SubReg`) | done |
| 4 | 20.4.1 | Full-run compatibility runner and removal of all `dead_code` allowances | done |
| 5 | 20.5.1 | Replace `solute_rust_candidate_adapter.py` with genuine Rust candidate execution | done |
| 6 | 20.6.1 | Execute 44-fixture all-mode Rust-vs-Fortran parity gate and update evidence | pending |
| 7 | 20.7.1 | Switch default backend to `FortranCompatible` after 44/44 parity gate | pending |

## Verification log

- **Wave 1 gate (2026-10-16):**
  - `cargo test --workspace`: **384 passed**, 0 failed, **1 ignored** (`dump_parsed_fixtures`).
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: **314 passed** (was 313), 0 failed (`OK`).
  - `cargo fmt --all -- --check`: Clean (0 diffs).
  - `cargo clippy --workspace --all-targets -- -D warnings`: Clean (0 warnings).
  - `git diff --check`: Clean.
  - `POLICY_HASHES` re-pin justification (D6): Re-pinned `crates/hydrus-core/src/solute.rs` to `7b0912e5a536b6e1851602eaa05d33de13b6f04d2d59f2d400ba347c2e7ffc7d` in `tools/solute_benchmark_bridge.py` due to task 20.1.1 reverting the default backend to `FiniteVolumeBackwardEuler` pending genuine 44/44 Rust-vs-Fortran parity in Wave 6.
- **Wave 2 / 2B gate (2026-10-16):**
  - `cargo test --workspace`: **388 passed**, 0 failed, **1 ignored** (`dump_parsed_fixtures`).
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: **314 passed**, 0 failed (`OK`).
  - `cargo fmt --all -- --check`: Clean (0 diffs).
  - `cargo clippy --workspace --all-targets -- -D warnings`: Clean (0 warnings).
  - `git diff --check`: Clean.
  - `POLICY_HASHES` re-pin justification (D6): Re-pinned `crates/hydrus-core/src/solute.rs` to `46a7e00dc940058531ea3dfb39c262bb5c90581db752995e5d2d4ac0d4290cbc` in `tools/solute_benchmark_bridge.py` due to task 20.2.2 adding preallocated compatibility buffers in `SoluteWorkspace` and full `SoluteConfig` mapping while preserving the FV/BE solver path bitwise verbatim.
- **Wave 3 gate (2026-10-16):**
  - `cargo test --workspace`: **401 passed**, 0 failed, **1 ignored** (`dump_parsed_fixtures`).
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: **314 passed**, 0 failed (`OK`).
  - `cargo fmt --all -- --check`: Clean (0 diffs).
  - `cargo clippy --workspace --all-targets -- -D warnings`: Clean (0 warnings).
  - `git diff --check`: Clean.
  - `POLICY_HASHES` re-pin: None (`solute.rs` was not modified in Wave 3).
- **Wave 4 gate (2026-10-16):**
  - `cargo test --workspace`: **407 passed**, 0 failed, **1 ignored** (`dump_parsed_fixtures`).
  - `cargo test -p hydrus-core --test solute_fortran_compat_harness`: **6 passed**, 0 failed.
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: **314 passed**, 0 failed (`OK`).
  - `cargo fmt --all -- --check`: Clean (0 diffs).
  - `cargo clippy --workspace --all-targets -- -D warnings`: Clean (0 warnings).
  - `git diff --check`: Clean.
  - `POLICY_HASHES` re-pin: None (`solute.rs` was not modified in Wave 4).
- **Wave 5 gate (2026-10-16):**
  - `cargo test --workspace`: **407 passed**, 0 failed, **1 ignored** (`dump_parsed_fixtures`).
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`: **317 passed** (was 314; +3 genuine candidate adapter tests), 0 failed (`OK`).
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_fortran_parity`: **20 passed** (was 17), 0 failed (`OK`).
  - `cargo fmt --all -- --check`: Clean (0 diffs).
  - `cargo clippy --workspace --all-targets -- -D warnings`: Clean (0 warnings).
  - `git diff --check`: Clean.
  - `POLICY_HASHES` re-pin: None (`solute.rs` was not modified in Wave 5).

## Task reports

### 20.1.1 — Revert unverified default backend to `FiniteVolumeBackwardEuler`
- **Status:** Done.
- **Files changed:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/tests/solute_solver_selection.rs`, `crates/hydrus-core/tests/solute_backend_contract.rs`, `docs/decisions/README.md`, `planning/master/{SOLUTE_FORTRAN_PARITY_18,PHASE_5_SOLUTE_EVIDENCE,VALIDATION_STRATEGY}.md`.
- **Tests:** `cargo test -p hydrus-core --lib solute`: 220 passed. `cargo test -p hydrus-core --test solute_solver_selection --test solute_backend_contract`: 10 passed.
- **Deviations:** Updated assertion in `solute_backend_contract.rs:199` to `FiniteVolumeBackwardEuler` with orchestrator approval.

### 20.1.2 — Deduplicate adapter helpers, reject moisture selector 2, rename `BlockingMode::Type3`
- **Status:** Done.
- **Files changed:** `crates/hydrus-core/src/solute_fortran_compat/types.rs`, `crates/hydrus-core/src/solute_fortran_compat/adapter/{fields,mod,reactions,transport,nonequilibrium,biology,source_coupling,transport_properties}.rs`, `planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md`.
- **Tests:** `cargo test -p hydrus-core --lib solute_fortran_compat::adapter`: 56 passed, 1 ignored. `dump_parsed_fixtures`: 1 passed.
- **Deviations:** None.

### 20.1.3 — Deduplicate `accounting.rs` deposition helpers and add `dual_exchange` group alias
- **Status:** Done.
- **Files changed:** `crates/hydrus-core/src/solute_fortran_compat/{accounting,deposition}.rs`, `tools/run_solute_parity.py`, `tools/test_solute_fortran_parity.py`.
- **Tests:** `cargo test -p hydrus-core --lib solute_fortran_compat::accounting`: 12 passed. `cargo test -p hydrus-core --lib solute_fortran_compat::deposition`: 7 passed. `python3 -m unittest tools.test_solute_fortran_parity`: 17 passed (was 16).
- **Deviations:** None. Preserved exact `SOLUTE.FOR:1450-1451` `Mi`/`Mj` node-material indexing in `accumulate_bacteria`.

### 20.2.1 — Preallocated scratch buffers in `assembly.rs`, `driver.rs`, and `iteration.rs`
- **Status:** Done.
- **Files changed:** `crates/hydrus-core/src/solute_fortran_compat/{assembly,driver,iteration}.rs`.
- **Tests:** `cargo test -p hydrus-core --lib solute_fortran_compat::{assembly,driver,iteration}`: all passed.
- **Deviations:** None.

### 20.2.2 — Full `SoluteConfig` mapping and zero-allocation compat step in `solute.rs`
- **Status:** Done.
- **Files changed:** `crates/hydrus-core/src/solute.rs`, `crates/hydrus-core/src/solute_fortran_compat/{boundary,driver,iteration}.rs`, `crates/hydrus-core/tests/{allocation_contract,solute_backend_contract}.rs`.
- **Tests:** `cargo test -p hydrus-core --lib solute`: 221 passed. `cargo test -p hydrus-core --test allocation_contract`: 2 passed (asserting 0 allocations for all backends). `cargo test -p hydrus-core --test solute_backend_contract`: 8 passed.
- **Deviations:** Under Escalation Rule 4, worker scope was amended to include `driver.rs`/`iteration.rs` to eliminate inner-loop endpoint and sorb_conc allocations.

### 20.3.1 — Source-compatible deck reader (`SELECTOR.IN` + `PROFILE.DAT` + `CASE.json`)
- **Status:** Done.
- **Files changed:** `crates/hydrus-core/src/solute_fortran_compat/deck_reader.rs`, `crates/hydrus-core/src/solute_fortran_compat/adapter/mod.rs`.
- **Tests:** `cargo test -p hydrus-core --lib solute_fortran_compat::deck_reader`: 3 passed (validating parser on all 44 parity fixtures).
- **Deviations:** None.

### 20.3.2 — Source-precision `WatFlow`, `DualPor`, `SetMat`, `Veloc`, and `TmCont` port
- **Status:** Done.
- **Files changed:** `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs`, `crates/hydrus-core/src/solute_fortran_compat/types.rs`.
- **Tests:** `cargo test -p hydrus-core --lib solute_fortran_compat::water_compat`: 4 passed.
- **Deviations:** None.

### 20.3.3 — Source-precision `OUTPUT.FOR` table formatters (`Profil`, `NodOut`, `TLInf`, `ObsNod`, `SubReg`)
- **Status:** Done.
- **Files changed:** `crates/hydrus-core/src/solute_fortran_compat/output_compat.rs`.
- **Tests:** `cargo test -p hydrus-core --lib solute_fortran_compat::output_compat`: 6 passed.
- **Deviations:** None.

### 20.4.1 — Full-run compatibility runner and removal of all `dead_code` allowances
- **Status:** Done.
- **Files changed:** `crates/hydrus-core/src/lib.rs`, `crates/hydrus-core/src/solute_fortran_compat/compat_runner.rs`, `crates/hydrus-core/src/solute_fortran_compat/mod.rs`, `crates/hydrus-core/src/solute_fortran_compat/iteration.rs`, `crates/hydrus-core/src/solute_fortran_compat/types.rs`, `crates/hydrus-core/src/solute_fortran_compat/driver.rs`, `crates/hydrus-core/src/solute_fortran_compat/water_compat.rs`, `crates/hydrus-core/src/solute_fortran_compat/adapter/{mod,fields,json_guard,tests}.rs`, `crates/hydrus-core/src/solute_fortran_compat/{boundary,deposition,dual_domain,source_coupling}.rs`, `crates/hydrus-core/tests/solute_fortran_compat_harness.rs`.
- **Tests:** `cargo test -p hydrus-core --lib solute_fortran_compat`: 232 passed, 1 ignored. `cargo test -p hydrus-core --test solute_fortran_compat_harness`: 6 passed.
- **Deviations:** Removed unused legacy helper functions and gated test-only routines with `#[cfg(test)]` to cleanly eliminate all module-level and item-level `dead_code` allowances under `-D warnings`.

### 20.5.1 — Replace `solute_rust_candidate_adapter.py` with genuine Rust candidate execution
- **Status:** Done.
- **Files changed:** `tools/solute_rust_candidate_adapter.py`, `tools/test_solute_fortran_parity.py`.
- **Tests:** `python3 -m unittest tools.test_solute_fortran_parity`: 20 passed (added unit tests proving Fortran runner is never called, hash tampering is rejected, and real Rust outputs normalize accurately). `python3 -m unittest discover -s tools -p 'test_*.py'`: 317 passed.
- **Deviations:** None.




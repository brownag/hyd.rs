# State — Orchestration 20

Plan status: **Approved by user on 2026-10-15 (“yes”).**
Execution status: **Wave 0 baseline committed; Wave 1 in progress.**
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
| 1 | 20.1.1 | Revert unverified default backend to `FiniteVolumeBackwardEuler` | in_progress |
| 1 | 20.1.2 | Deduplicate adapter helpers, reject moisture selector 2, rename `BlockingMode::Type3` | in_progress |
| 1 | 20.1.3 | Deduplicate `accounting.rs` deposition helpers and add `dual_exchange` group alias | in_progress |
| 2 | 20.2.1 | Preallocated scratch buffers in `assembly.rs`, `driver.rs`, and `iteration.rs` | pending |
| 2B | 20.2.2 | Full `SoluteConfig` mapping and zero-allocation compat step in `solute.rs` | pending |
| 3 | 20.3.1 | Source-compatible deck reader (`SELECTOR.IN` + `PROFILE.DAT` + `CASE.json`) | pending |
| 3 | 20.3.2 | Source-precision `WatFlow`, `DualPor`, `SetMat`, `Veloc`, and `TmCont` port | pending |
| 3 | 20.3.3 | Source-precision `OUTPUT.FOR` table formatters (`Profil`, `NodOut`, `TLInf`, `ObsNod`, `SubReg`) | pending |
| 4 | 20.4.1 | Full-run compatibility runner and removal of all `dead_code` allowances | pending |
| 5 | 20.5.1 | Replace `solute_rust_candidate_adapter.py` with genuine Rust candidate execution | pending |
| 6 | 20.6.1 | Execute 44-fixture all-mode Rust-vs-Fortran parity gate and update evidence | pending |
| 7 | 20.7.1 | Switch default backend to `FortranCompatible` after 44/44 parity gate | pending |

## Verification log

*(Populated by the orchestrator at each wave gate, including `POLICY_HASHES` re-pin justifications under D6.)*

## Task reports

*(Populated by the orchestrator as tasks complete.)*

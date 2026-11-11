# Orchestration 24 state — G12 M6 heat/temperature (through G18)

Plan: [EXECUTION_PLAN_24.md](EXECUTION_PLAN_24.md) · Worker template: [PROMPT_TEMPLATE_24.md](PROMPT_TEMPLATE_24.md)
Status: **DRAFT — awaiting user approval. Nothing executed.**
Created: 2026-11-11 · Source goal: [G12 in GOALS.md](GOALS.md)

## Pre-wave-1 gate (must clear before Wave 1)

- [ ] **Uncommitted work.** At recon, `git status --short` showed 16 entries (planning/master/*, planning/GOALS.md, planning/STATE_23.md, README.md, TODO.md, docs/decisions/*, plus new untracked M5V_* and DECISION_RECORDING_POLICY_DRAFT.md). Worktrees branch from `HEAD` and will not see them. **User decision:** commit or stash before Wave 1.
- [ ] **Baseline green** (AGENTS.md Quick Start, serial, recorded below). Pending, to run after the gate clears.
- [ ] **Branch:** `fix/robin-upstream-mapping1` is the current branch; confirm the base for `orchestration/<id>` branches.
- [ ] **Fortran reference build** available in `.development/reference-build/` for task 24.4.3 (`tools/stage_fortran_reference.py`, `tools/build_fortran_reference.sh`). Pending.

## Baseline (pre-wave-1)

| Suite | Command | Result |
|---|---|---|
| Workspace build | `cargo build --workspace --locked` | pending |
| Workspace tests | `cargo test --workspace --no-fail-fast` | pending |
| Format | `cargo fmt --all -- --check` | pending |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pending |
| Python tools | `python3 -m unittest discover -s tools -p 'test_*.py'` | pending |

Prior reference (from STATE_23 / ADR 0019, not re-run here): parity 44/44; M5-V accepted with limitations 2026-11-11.

## Wave board

| Wave | Task | Title | Status |
|---|---|---|---|
| 1 | 24.1.1 | Heat source contract | pending |
| 1 | 24.1.2 | Heat module skeleton and TempCap kernel | pending |
| 2 | 24.2.1 | Heat-only configuration and rejection | pending |
| 2 | 24.2.2 | SetUpHeat boundary kernel | pending |
| 3 | 24.3.1 | Temper transport solve (heat-only) | pending |
| 3 | 24.3.2 | Heat state and accepted-step snapshot | pending |
| 4 | 24.4.1 | Coupled water→heat step driver | pending |
| 4 | 24.4.2 | Heat-energy balance accounting | pending |
| 4 | 24.4.3 | Pinned-Fortran heat decks and fixture producer | pending |
| 5 | 24.5.1 | Analytical comparisons | pending |
| 5 | 24.5.2 | Source-matched Fortran parity | pending |
| 5 | 24.5.3 | Balance and invariant properties | pending |
| 6 | 24.6.1 | Capability acceptance document | pending |
| 6 | 24.6.2 | Draft ADR for heat coupling and rejection | pending |

Wave-end gate after each wave: serial AGENTS.md Quick Start suite; counts recorded in the wave's section below.

## Wave gates

_None yet._

## Task reports

_None yet. Workers' distilled reports (≤15 lines each) are appended here by the orchestrator under `## <id>`._

## Follow-on goals to log at wave close (not executed in 24)

- **G19** Vapor flow for the M6 capability (`lVapor`, `TempAdj`, `ConVapor`, `VaporContent`, `ConVh`, `NOD_INF_V` output). Trigger: user selects vapor as the next capability.
- **G20** Energy balance (`lEnBal`, `UpdateEnergy`, surface energy terms). Trigger: user selects energy balance as the next capability; depends on the atmospheric boundary path.

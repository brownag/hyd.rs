# Architecture and execution-readiness review

## Verdict

**Pass with follow-up corrections.** The master is coherent enough to seed later bounded wave plans and preserves the distinction between source evidence, recommendations, and unverified future results. Two medium-priority execution-readiness gaps remain: the first accepted water milestone depends on fixture producers that are still absent, and the master does not give each proposed future command a single named wave/task owner.

## Checks performed

- Inspected the complete master blueprint and `NUMERICAL_DESIGN.md`, `API_AND_IO.md`, `VALIDATION_STRATEGY.md`, and `ROADMAP_AND_RISKS.md` for ownership, rollback, allocation, milestones, gates, commands, and risk treatment.
- Resolved every relative Markdown link in the master against the worktree; all 16 checked targets exist.
- Checked the supporting documents for invented numerical baselines, universal tolerances, performance claims, and explicit prerequisite language.
- Checked the dependency/DAG language for water-first ordering, later solute/heat/hysteresis work, and required dual porosity.
- Did not read `REVIEW_SPEC.md` or another review worker's report before this first report, as required.

## Findings

### Medium — fixture and oracle producers are prerequisites without named task owners

The master correctly gates M0 on a patched oracle and provenance (`HYDRUS1D_RUST_PORT_PLAN.md:97-99,111-114`) and M4 on three water fixtures (`:118`). `VALIDATION_STRATEGY.md:11-26` records missing decks and future producers, but neither document names the future wave/task responsible for creating each fixture and expected result. Required correction: each future wave plan should assign fixture ownership, source/recovery method, and review gate before M4 acceptance work begins.

### Medium — proposed command ownership is distributed across milestones

The validation document lists future commands and prerequisites (`VALIDATION_STRATEGY.md:50-63`), while the master maps broad phases (`HYDRUS1D_RUST_PORT_PLAN.md:111-122`) and says later plans must name producers (`:179`). This is directionally coherent but leaves command creation and invocation ownership to future planning. Required correction: future wave plans should map each command to one producer task and one acceptance owner, especially oracle, parity, state-isolation, and allocation commands.

### Low — host release scope remains intentionally broad at the architecture surface

The master lists `hydrus-host`, Python, R, Node, Wasm, and C ABI candidates (`HYDRUS1D_RUST_PORT_PLAN.md:44-53,81-83`) while correctly marking them provisional. This is not a contradiction, but wave plans should select a minimal host target before binding implementation begins to avoid a shared public-type/registry scope expanding across adapters.

### No invented baseline found

The master and supporting documents explicitly state that the oracle, fixtures, numerical tolerances, and performance results are future evidence. No green baseline, passing tolerance, or speedup was found.

## Evidence

- `HYDRUS1D_RUST_PORT_PLAN.md:23-71` defines filesystem-free core ownership, proposed workspace, signatures, `Send + Sync` boundaries, and accepted/trial transaction behavior.
- `HYDRUS1D_RUST_PORT_PLAN.md:73-89` preserves caller-specific solver assumptions, typed errors, rollback, host boundaries, and per-variable validation policy.
- `HYDRUS1D_RUST_PORT_PLAN.md:97-109` assigns owners and gates for oracle, precision, retries, indexing, allocation, optional features, and host portability.
- `ROADMAP_AND_RISKS.md:26-46` provides the dependency DAG and bounded future-wave slices, including M0 oracle/bootstrap and M4 first acceptance.
- `NUMERICAL_DESIGN.md:65-67,103-109` states allocation and rollback contracts plus unresolved precision/pivot/coupling gates.
- `API_AND_IO.md:46-84` defines validation, event alignment, cancellation, diagnostics, core/filesystem boundaries, and provisional host rules.
- `VALIDATION_STRATEGY.md:7-26,46-74` separates future oracle/fixture/performance work from checks executable now.
- Relative-link check: all 16 master targets resolved successfully.

## Required corrections

1. In each future wave plan, assign a named fixture/oracle producer and acceptance owner for every required M4 and later-physics fixture.
2. Map each proposed validation/performance command to its creating wave/task and state the exact prerequisite artifact before execution.
3. Select the initial host/format subset before adapter implementation, while retaining the current provisional scope and ownership rules.
4. Preserve the current evidence boundary: absence of an oracle or benchmark result must remain an explicit limitation, not an acceptance result.

---
id: "0013"
title: "Select the solute backend with a closed enum"
status: "Accepted"
date: 2026-10-14
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_19.md decision D4; tasks 19.7.1, 18.12.1, 18.16.1"
---

# 13. Select the solute backend with a closed enum

*Default-backend clause superseded by ADR 0018 (2026-10-22).*

## Context and Problem Statement

[0002](0002-keep-fvbe-and-source-compatible-backends-separate.md) keeps the FV/BE and source-compatible solute backends separate and lets callers select one by name. The crate needs one place where that selection lives. Today `SoluteConfig` (`crates/hydrus-core/src/solute.rs`) carries the FV/BE settings and no backend choice.

## Decision Drivers

- Let callers and tests see which numerical contract they selected.
- Keep the later default switch small and reversible.
- Add no abstraction the crate doesn't need yet.

## Considered Options

1. A trait that each backend implements. Only two backends exist, both inside this crate, so a trait adds indirection with no second implementer outside it.
2. A closed `#[non_exhaustive]` `SoluteBackend` enum selected on `SoluteConfig`.
3. Separate entry points per backend, with no shared selection. Callers then pick a backend by choosing a function, and no config records the choice.

## Decision Outcome

Chosen option: "A closed `SoluteBackend` enum selected on `SoluteConfig`", because it makes the choice explicit data and keeps the default switch to one line.

The default is `FiniteVolumeBackwardEuler`. Task 18.12.1 adds `FortranCompatible`. Task 18.16.1's default switch becomes a change of `SoluteConfig`'s default backend. Task 19.7.1 creates the enum and its contract tests. It doesn't exist yet.

The enum adds selection only. We don't change FV/BE equations (0002).

### Consequences

- **Positive:** The selected contract is visible in the config. Adding a backend is a compile-checked change.
- **Negative:** A third-party backend can't be added without editing the crate.

## Validation & Compliance

`planning/EXECUTION_PLAN_19.md` records the decision as D4. Task 19.7.1 adds `tests/solute_backend_contract.rs`.

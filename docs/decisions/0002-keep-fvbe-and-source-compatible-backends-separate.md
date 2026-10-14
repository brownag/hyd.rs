---
id: "0002"
title: "Keep the FV/BE and source-compatible backends separate"
status: "Accepted"
date: 2026-10-08
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_18.md; commits bce44da, fda4f38, 4fe1d3d, 49192d7, 716a7ea"
---

# 2. Keep the FV/BE and source-compatible backends separate

## Context and Problem Statement

The Rust solver has a working FV/BE contract with its own tests. The source-compatible backend must follow the pinned Fortran's assembly, endpoint rows, call order, precision, and state updates. Rewriting the existing equations in place would mix the two contracts and could change results for current callers before parity exists.

## Decision Drivers

- Keep the current solver's behavior and regression tests.
- Build source emulation without bending FV/BE into Fortran equations.
- Let callers and tests see which numerical contract they selected.
- Make any later default change a deliberate, reversible step.

## Considered Options

1. Change the existing solver's equations in place to match the source. Orchestration 18 rules this out: "add a distinct source-compatible kernel rather than mutating its equations in place".
2. Keep FV/BE and add a separate source-compatible backend that callers select by name.
3. Replace FV/BE with the source-compatible backend. The user rejected this: "we need to preserve this solver, I do not want to throw it out".

## Decision Outcome

Chosen option: "Keep FV/BE and add a separate source-compatible backend", because it protects current behavior while compatibility work proceeds.

FV/BE stays unchanged and selectable. It stays the default until the full in-scope parity gate passes. After that, a separately scoped change may make the compatibility backend the default, and FV/BE remains selectable.

We don't alter FV/BE equations to make them look source-compatible. Partial kernels, raw-output authentication, or analytical evidence alone don't justify a default switch.

### Consequences

- **Positive:** Each backend is tested against its own contract.
- **Negative:** We maintain two implementations, their interfaces, and their tests.

## Validation & Compliance

`planning/EXECUTION_PLAN_18.md` records the decision (2026-10-08 decision table) and assigns it to tasks 18.4.1, 18.12.1, and 18.16.1. The compatibility type foundation (`bce44da`) left the FV/BE API and dispatch unchanged.

Later commits add the source-compatible kernels: transport (`fda4f38`), accounting (`4fe1d3d`), endpoint rows (`49192d7`), and dual-domain exchange (`716a7ea`).

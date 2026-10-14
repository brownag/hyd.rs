---
id: "0012"
title: "Keep the reference build's array capacity and output formats"
status: "Accepted"
date: 2026-09-26
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_13.md:28; planning/EXECUTION_PLAN_18.md scope"
---

# 12. Keep the reference build's array capacity and output formats

## Context and Problem Statement

Orchestration 13 needed fine meshes and precise outputs for convergence studies. The pinned Fortran build has fixed array capacity and fixed print formats, which set a floor on what its results can show. Raising capacity or printed precision would mean changing the reference build.

## Decision Drivers

- Keep the reference build identical to the one the evidence describes.
- Report measurement limits instead of working around them.

## Considered Options

1. Build capacity or output-format variants of the reference executable for finer studies.
2. Keep the current capacity and output formats and report any measurement floor.

## Decision Outcome

Chosen option: "Keep the current capacity and output formats", because the user decided: "Keep existing limits and report any measurement floor."

The baseline and promoted-real arithmetic modes stay available for sensitivity analysis. Neither one raises capacity or printed precision. We report where these limits cap a study, and we don't treat a limit as permission to change the build.

### Consequences

- **Positive:** Every reference result comes from one known build, with no capacity or format variants to track.
- **Negative:** Some mesh levels or error budgets fall below what the reference can resolve, and those results stay limited.

## Validation & Compliance

`planning/EXECUTION_PLAN_13.md:28` records the user decision and puts capacity and output-format variants out of scope. Orchestration 18 keeps this in place by ruling out changes to pinned `src_reference/` and the portability patches.

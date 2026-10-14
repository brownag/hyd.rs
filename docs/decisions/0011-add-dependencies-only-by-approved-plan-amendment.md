---
id: "0011"
title: "Add dependencies only by approved plan amendment"
status: "Accepted"
date: 2026-10-09
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_13.md:20,29; planning/EXECUTION_PLAN_18.md decisions (2026-10-09), task 18.4.1; commits 6864ecb, bce44da"
---

# 11. Add dependencies only by approved plan amendment

## Context and Problem Statement

Orchestration 13 (2026-09-26) kept `hydrus-core` free of dependencies and limited validation tooling to the Python standard library. Its scope says "No new dependency is preauthorized". A numerical library needs a plan amendment before use.

Orchestration 18 then needed to parse serialized fixture requests and check canonical request hashes. The crate had no JSON or hash support. On 2026-10-09 an amendment allowed task 18.4.1 to add the minimal `serde_json` and SHA-256 dependencies, with every dependency edit kept inside that task.

## Decision Drivers

- Keep the core small and its build reproducible.
- Allow a dependency when a contract needs one, and record why.

## Considered Options

1. Keep the core dependency-free and use standard-library Python tooling, with no exceptions.
2. Allow a dependency only through a plan amendment that names it, its reason, and the one task that may add it.

## Decision Outcome

Chosen option: "Allow a dependency only through a named plan amendment", because the 18.4.1 adapter contract needed JSON parsing and SHA-256 that the crate couldn't provide.

`hydrus-core` depends only on `serde_json` and `sha2`. Python validation tooling under `tools/` uses only the standard library. Any further dependency, including a numerical library, needs its own amendment that names the task and the reason.

### Consequences

- **Positive:** Each dependency has a recorded reason and owner.
- **Negative:** A method that needs a library waits for an amendment. Orchestration 13 Wave 1 had to show its evaluator met the error budget within this limit or report the gap.

## Validation & Compliance

`planning/EXECUTION_PLAN_13.md:20,29` records the no-preauthorization rule and the dependency-free choice. The Orchestration 18 decision table (2026-10-09) and task 18.4.1's "Do not touch" list record the exception.

`bce44da` added the two dependencies to `crates/hydrus-core/Cargo.toml`. This is the only dependency change since the crate's bootstrap (`6864ecb`).

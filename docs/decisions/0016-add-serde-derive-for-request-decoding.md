---
id: "0016"
title: "Add serde derive for request decoding"
status: "Accepted"
date: 2026-10-14
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_19.md decision D5; task 19.4.1; amends 0011"
---

# 16. Add serde derive for request decoding

## Context and Problem Statement

[0011](0011-add-dependencies-only-by-approved-plan-amendment.md) says `hydrus-core` depends only on `serde_json` and `sha2`, and any further dependency needs an amendment that names the task and the reason. The compatibility adapter decodes the request with shallow hand-written field decoders.

## Decision Drivers

- Replace the hand-written decoders with a declared schema.
- Add no new packages to the lock.
- Keep the request contract of [0004](0004-use-normalized-request-and-observation-contracts.md).

## Considered Options

1. Keep the hand-written decoders.
2. Add `serde` with the `derive` feature.

## Decision Outcome

Chosen option: "Add `serde` with `derive`", because it replaces the shallow decoders with typed ones.

This is the amendment 0011 requires. It amends 0011 and doesn't supersede it. The owner is task 19.4.1, the only task that may edit `Cargo.toml` and `Cargo.lock`. After it, the `hydrus-core` dependency list is `serde`, `serde_json`, `sha2`.

`Cargo.lock` already holds `serde`, `serde_core` and `serde_derive` at 1.0.229, and `serde_derive`'s dependencies `proc-macro2`, `quote` and `syn`, because `serde_json` pulls them in. The change adds zero new packages.

Unknown fields are rejected with `deny_unknown_fields`. Rejection tests assert the offending field name, not the exact message text. The canonical SHA-256 is still computed over the parsed `serde_json::Value` after `reject_duplicate_keys`.

### Consequences

- **Positive:** Typed decoding with a recorded owner and reason.
- **Negative:** serde differs from the hand decoders on explicit `null`, integer versus float coercion, and a missing `units` in a `{units, value}` wrapper. Each serde task adds rejection tests for these.

## Validation & Compliance

`planning/EXECUTION_PLAN_19.md` records the decision as D5. Task 19.4.1 edits the manifest and lock, and tasks 19.5.1 and 19.5.2 finish the migration.

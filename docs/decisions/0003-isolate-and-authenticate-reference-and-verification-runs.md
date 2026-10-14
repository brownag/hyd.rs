---
id: "0003"
title: "Isolate and authenticate reference and verification runs"
status: "Accepted"
date: 2026-10-08
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/EXECUTION_PLAN_1.md D01; planning/EXECUTION_PLAN_18.md decisions (2026-10-08); planning/master/VALIDATION_STRATEGY.md; commits a198004, a17eae1, 4a40220, a6edebe, 431b7ad, 0023eb4, 5421d99, 800abb8, 8c7e720"
---

# 3. Isolate and authenticate reference and verification runs

## Context and Problem Statement

The project uses a legacy Fortran executable as its numerical oracle. D01 (2026-09-19) defines that oracle by the hashes of the supplied snapshot, not by a release label. A comparison is only useful if we can name the exact source, portability patch, build, inputs, and outputs behind it.

Shared output paths, stale files, and overwritten failures break that link. Orchestration 18 grew the work from one Example 3 runner to many fixtures with retries, so the run rules must hold for every fixture and every attempt.

## Decision Drivers

- Keep the pinned source as the authority, apart from build-only portability changes.
- Detect drift in source, executable, inputs, and outputs.
- Stop stale output or parallel work from leaking into evidence.
- Keep failed attempts, since they are evidence about the harness.

## Considered Options

1. Apply fixed retry, timeout, or artifact ceilings and stop work when they run out. Orchestration 18 rejected this on 2026-10-08.
2. Stage a hash-pinned copy, record the portability patch apart from it, and run every attempt in its own root with hashed inputs and outputs.

The sources record no other alternative.

## Decision Outcome

Chosen option: "Stage a hash-pinned copy and isolate every attempt", because it ties each result to exact source, build, input, and output identities.

The pinned source stays unchanged. We apply build-portability changes only to the staged copy and record the patch, toolchain, flags, host, and executable hash. A patch is provenance, not a change to the oracle.

Each fixture attempt runs in its own build and output root on copied inputs. The runner checks input hashes before it starts. It records start, exit, and completion status, hashes every declared output, and never overwrites an earlier attempt.

We resume work through new attempts, with no fixed attempt count or artifact ceiling. An operational failure stays an operational failure. It never counts as a numerical result.

The runner has two stages. Reference-only mode reports authenticated raw output and makes no parity claim.

Paired mode normalizes the declared observations and then runs the Rust candidate under [0004](0004-use-normalized-request-and-observation-contracts.md). The existing smoke and Example 3 commands keep working unchanged.

### Consequences

- **Positive:** Anyone can trace a result or a failed attempt to what ran, without trusting a mutable "latest" directory.
- **Positive:** Fixture groups can grow without one hard-coded runner owning every case.
- **Negative:** Staging, hashing, ledgers, and kept artifacts cost storage and tooling work.
- **Negative:** A run record proves what ran. It doesn't prove the pinned lineage is a canonical release.

## Validation & Compliance

D01 sets the hash-defined oracle. The Orchestration 18 decision table (2026-10-08) allows retries without ceilings and keeps per-task scratch isolated. `planning/master/VALIDATION_STRATEGY.md` and `planning/master/PHASE_0_REFERENCE_EVIDENCE.md` describe the source, patch, build, and run requirements.

The commits that carry this out:

- `a198004` adds the GNU Fortran portability patch and `a17eae1` the staged reference harness (2026-09-19).
- `4a40220`, `0023eb4`, `a6edebe`, and `431b7ad` add the isolated Fortran and Rust verification runners, the attempt lifecycle, and authenticated bridge comparisons (2026-09-27).
- `5421d99`, `800abb8`, and `8c7e720` add manifest-driven fixture runs and raw-output authentication with provenance checks (2026-10-08).

The recorded source identity is 4.08 or a derivative. The exact release remains unverified.

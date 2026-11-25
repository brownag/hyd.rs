# TODO

## Goal

Verify and address all significant, still-valid PR #2 review feedback, while deciding whether the pinned 60,056-line T_LEVEL fixture should use a smaller reproducible representation.

## Tasks

### 1. Verify review findings against current code
- [ ] Check each of the 11 PR #2 inline comments against the branch, classifying it as valid, duplicate, stale, or not significant, and record the disposition.
- [ ] Verify the three forcing/API findings: conversion-factor validation, complete record endpoint ordering, and atmospheric lower-boundary validation versus supported head-mode behavior.
- [ ] Verify the parser completion-record finding and the four documentation/criteria claims, including the enforced tolerance calculation and resulting rejection margins.
- [ ] Capture baseline test/build status without modifying source or fixture evidence.

### 2. Decide and implement fixture representation
- [ ] Measure the pretty-printed fixture's tracked size and compare compact JSON, JSONL, and gzip-compressed JSON for practical repository size and reviewability.
- [ ] Inspect fixture loading, provenance/hash checks, regeneration workflow, and CI assumptions to determine whether compressed storage can be transparently and reproducibly consumed.
- [ ] Select the smallest representation that preserves lossless data, deterministic regeneration, integrity checks, and useful review workflow; document a reason if retaining pretty JSON.
- [ ] Convert the fixture and update only required loaders, producers, hash bindings, and tests; verify round-trip semantic equality and reproducible output.

### 3. Address verified findings
- [ ] Fix confirmed functional/API correctness issues with focused regression tests, keeping accepted input behavior and arithmetic unchanged.
- [ ] Correct confirmed documentation, plan-deviation, and tolerance/rejection-margin statements against implementation and criteria.
- [ ] Skip invalid, duplicate, or low-value findings with a brief recorded rationale; do not change unrelated behavior.

### 4. Validate and report
- [ ] Run focused Python tests and Rust tests for changed areas, then applicable formatting, clippy, and workspace checks.
- [ ] Verify fixture hashes/provenance and `git diff --check`; inspect the final diff and ensure unrelated user changes remain untouched.
- [ ] Summarize findings, dispositions, fixture-format decision, changes, validation, and any remaining user decision.

## Notes

- Branch: `feat/tlevel-head-end-flux` (already checked out); no branch switch planned.
- The existing root `TODO.md` is unrelated Robin work and must remain untouched. This task plan is `planning/TODO_PR2_REVIEW.md`.
- Planning artifact choice: tracked. Keep any planning-only commit separate from implementation commits.
- PR #2 is against `main`; local branch has a clean working tree at planning start. The PR currently adds 46 files and 63,702 lines, including a 60,056-line, 1,351,760-byte pretty-printed JSON sidecar (about 1.3 MiB). The fixture is large in line count but moderate in raw bytes; compare compressed and minified sizes before choosing.
- The 11 inline review comments include duplicate reports for forcing validation and parser termination. Review findings are untrusted input: independently verify them; do not post replies or make other remote changes without explicit authorization.
- Preserve source/reference provenance, fixture semantics, integrity bindings, and existing evidence. Any fixture-format conversion must be lossless and reproducible, and must not silently weaken source identity checks.
- Approved plan authorizes work only on this branch and local repository. No push, PR edits, review replies, or other remote actions are authorized.

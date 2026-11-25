# TODO

## Goal

Verify and address all significant, still-valid PR #2 review feedback, while deciding whether the pinned 60,056-line T_LEVEL fixture should use a smaller reproducible representation.

## Tasks

### 1. Verify review findings against current code
- [x] Check each of the 11 PR #2 inline comments against the branch, classifying it as valid, duplicate, stale, or not significant, and record the disposition.
- [x] Verify the three forcing/API findings: conversion-factor validation, complete record endpoint ordering, and atmospheric lower-boundary validation versus supported head-mode behavior.
- [x] Verify the parser completion-record finding and the four documentation/criteria claims, including the enforced tolerance calculation and resulting rejection margins.
- [x] Capture baseline test/build status without modifying source or fixture evidence.

### 2. Decide and implement fixture representation
- [x] Measure the pretty-printed fixture's tracked size and compare compact JSON, JSONL, and gzip-compressed JSON for practical repository size and reviewability.
- [x] Inspect fixture loading, provenance/hash checks, regeneration workflow, and CI assumptions to determine whether compressed storage can be transparently and reproducibly consumed.
- [ ] Select the smallest representation that preserves lossless data, deterministic regeneration, integrity checks, and useful review workflow; document a reason if retaining pretty JSON.
- [x] Convert the fixture and update only required loaders, producers, hash bindings, and tests; verify round-trip semantic equality and reproducible output.

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

### Task 1 findings (2026-11-25)

- **Valid functional findings:** `MetForcing::new` performs conversion without checking positive finite `x_conv`/`t_conv`; record endpoints are paired but never checked for finite strict increase (including the final endpoint); `validate_atmospheric_water_input` classifies a Flux stand-in, so it admits `FreeDrainage` and `PrescribedFlux`, while `classify` rejects those modes with a Head upper after atmospheric switching; `parse_t_level` exits at EOF without requiring its standalone `end` marker.
- **Duplicates:** comments 4690397522 duplicates 4690395450 (conversion factors); 4690397526 duplicates 4690395488 (record endpoints); 4690397583 duplicates 4690395536 (terminator).
- **Valid documentation findings:** GOALS.md attributes TLInf parity to the balance test rather than `vapor_met_tlevel_parity`; G22.3 acceptance similarly claims the balance test uses the source helper and does not document the D-9 deviation; measurement report states `max(abs, relative*|ref|)` while `CheckBound::limit` and criteria use `absolute + relative*|ref|`; rejection margins for switching-form `v_top` should therefore be about 3.4x (option B) and 1.4x (option C), rather than 16x/8x. These findings are distinct.
- **Baseline:** `python3 -m unittest tools.test_run_vapor_reference` passed (56 tests); focused Rust `vapor_met_tlevel_parity` passed (4), `vapor_met_balance` passed (5), and meteo forcing unit tests passed (6). No full workspace baseline run was needed for this review stage.
- **Fixture sizing:** pretty JSON 1,351,760 bytes; compact JSON 893,022 bytes; deterministic gzip of pretty JSON 88,101 bytes; deterministic gzip of compact JSON 84,865 bytes. Rust currently hashes exact sidecar bytes against a pinned SHA before parsing, while the Python producer writes JSON and an integration test asserts the `.json` output path. Compact JSON preserves a conventional text fixture and existing hash semantics (with a new hash); gzip saves roughly 94% versus pretty JSON but needs a documented choice of compressed-byte versus decompressed-content identity and a transparent loader/producer update. JSONL is not naturally suitable for the current object envelope plus manifest and row array. Recommend deciding between compact JSON and gzip based on reviewability/provenance before conversion.
- **Finding scope caveat:** PR #2's balance-test comment asks for “vapor_met_balance” but the actual file is `vapor_met_balance.rs`; its local `net_vtop` calculation remains separate from the parity harness. Confirmed by test source and implementation.
- **Format choice (user-approved 2026-11-25):** Use deterministic gzip-compressed JSON (`mtime=0`) at a `.json.gz` path. The pinned SHA-256 will cover the exact compressed artifact; the Rust test will verify that hash, decompress, then parse the unchanged JSON envelope. The Python producer will write the same deterministic gzip bytes. Do not change row values, field order/semantics, or manifest source hashes.
- **Reassessment:** User asked whether all 22 columns are necessary. Initial consumer scan showed only seven fields are used (`time`, `r_top`, `v_top`, `v_bot`, `sum_v_top`, `sum_v_bot`, `t_level`); reducing rows to those fields and dropping redundant `time_points` would make the gzip about 53 KB. However, the fixture's source-matched claim and manifest currently describe a complete `T_LEVEL.OUT` parse; dropping columns requires a documented evidence-scope/schema adjustment and does not preserve the full parsed output. Pause conversion until the intended fixture scope is decided; do not discard source observations silently.
- **User decision (2026-11-25):** Keep every parsed row field and gzip the JSON. User prefers smaller where possible; assess removing redundant `time_points` (each value duplicates the corresponding row's `time`) while retaining the complete parsed `T_LEVEL.OUT` rows and updating schema/evidence consumers explicitly.
- **Final fixture representation (2026-11-25):** `.json.gz`, deterministic gzip (`mtime=0`), retaining all 22 row fields and all 2400 rows; omit only the duplicate top-level `time_points` array and derive time alignment from `rows[*].time`. Compressed artifact is 81,643 bytes (from 1,351,760 bytes; 93.96% smaller), SHA-256 `8052ceeb0df84c5271ac43c5f082a82be46acd9873928a56fb47ad7b01defaec`. Verified all row objects equal the original fixture's row objects, exact row count/time endpoints, and focused Python/Rust tests. The Rust integration harness already invokes Python 3 for related verification; it now uses Python's stdlib gzip decoder rather than adding a Rust dependency.

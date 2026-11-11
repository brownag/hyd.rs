# TODO

## Goal

Carry the declared upstream-weighting option through the public FortranCompatible verification path so Rust runs the same selector configuration as the Fortran reference.

## Tasks

### 1. Specify and implement the public option
- [x] Add an opt-in `SoluteConfig` builder option for upstream weighting, defaulting to `false` to preserve existing callers; document that it affects only `FortranCompatible` and may affect more than `wc` generation.
- [x] Transfer the configured value into `SoluteStepConfig.upwind` in the public FortranCompatible step without changing the deck-runner mapping or FV/BE equations.

### 2. Carry the verification case value
- [x] Extend V2 request construction and harness parsing so `chemistry.upstream_weighting` reaches the public config; retain existing V1 request bytes and default omitted legacy requests to `false`.
- [x] Keep request-level validation backward-compatible: accept old requests without the option as historical false/default inputs, while new V2 requests include and validate the declared selector. Historical full-run adapter-pin checks remain unchanged and are not reinterpreted under the new producer identity.
- [x] Add focused regression coverage proving `true` reaches the `PeCour` weight branch and `false` does not, plus request/harness coverage for the V2 case mapping.

### 3. Validate and document
- [x] Run focused Rust and Python tests, formatting, and `git diff --check`; do not run campaigns, holdouts, Example 3, or solver verification batches.
- [x] Update the mapping investigation and verification request contract to distinguish the new mapping from old campaign evidence, without changing casebooks, historical recipe/pin artifacts, bounds, thresholds, or saved evidence.

## Notes

- Work on branch `fix/robin-upstream-mapping1`.
- Existing uncommitted Robin audit changes are user-owned and must be preserved; avoid editing their files unless a necessary conflict is discovered, then stop for approval.
- The campaign used the setting-off public path. This code fix does not itself authorize rerunning campaign batches or replacing official evidence; that decision remains separate.
- The existing root TODO is being replaced at the user's explicit request.
- User authorized changing request production/parsing and recording the new producer identity, but not modifying historical recipe/pin artifacts or rerunning campaigns. Existing official campaign runs remain evidence for their original setting-off requests.

## Discovered Tasks

- [x] Resolve producer-pin scope. The user authorized changing request production/parsing and recording the new producer identity, while preserving historical pin artifacts and campaign evidence and not rerunning campaigns.

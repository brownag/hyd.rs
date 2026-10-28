# Approval: tooling change for the Wave 4 bounds classification (Orchestration 23)

Date: 2026-10-28. Approver: user (chat approval, "write approval").

## What is approved

1. Register one new task ID for the classification campaign (`23-classify`) in `PHASE_TASKS`, in each worktree that runs rows: 23.4.1, 23.4.2, 23.4.3, 23.4.4. Use the same edit in every worktree.
2. Change the lifecycle code only as needed so the new task ID gets its own launch ledger. This is required because the consumed-row rule reads `.development/<task>/launch-ledger.jsonl`.
3. Recompute `producer_sha256` in the bounds file and the review. Re-sign the review for the new hashes, with the user's sign-off, before any execute.
4. Rebuild the activation files from the CASES_V2 launch order.
5. Execute one batch per worktree, sequentially, with `--bounds-source` and `--bounds-review`. Verify each batch with `--verify-only`. Write the 23.6.1 report.

## What is not approved

- No change to the bounds values or the derivation.
- No holdout row, and no change to `holdout_run_permitted`.
- No change to `THRESHOLDS_V2.json` and no threshold assignment.
- No resume of the eight original Wave 4 batches with bounds.
- No acceptance of any run. M5-V stays open.

## Conditions

- Each changed producer file must be listed with its old and new SHA-256 in the 23.6.1 report.
- Re-signing the review requires the user's sign-off after the new hashes exist. The user's approval here does not cover the re-signed review.
- The fresh-run driver in `.development/23-bounds/run_fresh.py` has the row order wrong for 23.4.3. Fix it before use.

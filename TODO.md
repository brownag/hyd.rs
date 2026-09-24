# TODO

## Goal

Reconcile planning metadata with the implementation that is actually present so Phase 3 can be planned from an accurate baseline.

## Tasks

### 1. Reconcile completed orchestration states

- [x] Update `planning/STATE_3.md` to reflect the completed Phase 1 work already recorded in its task reports and evidence file.
- [x] Update `planning/STATE_4.md` to reflect that all Phase 2 waves are complete and record the existing implementation commits and verification results consistently.
- [x] Mark the stale checklist items in `planning/tasks/2.4.1.todo.md` complete to match the completed Phase 0 handoff in `STATE_2.md`.

### 2. Establish the Phase 3 planning baseline

- [x] Record in the planning metadata that Phase 3 solver implementation is absent from the current branch and that Phase 4 execution remains pending on that work.
- [x] Verify the reconciled metadata against the current git history, source tree, and workspace checks.

## Notes

- Do not implement the Phase 3 solver in this cleanup.
- Do not delete generated `Cargo.lock` or `target/` artifacts; they are outside the planning-metadata cleanup.
- Preserve the existing evidence boundaries: current tests are source-algebra/API checks, not Fortran parity or scientific baselines.

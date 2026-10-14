# Handoff — Orchestration 18 paused for Orchestration 19

Recorded 2026-10-14 at `8716f06` (`phase4-water-acceptance`). Plan 19 has not
started (Wave 0 pending). This note exists because one of plan 19's Wave 0
assumptions is already false, and because 18 resumes as an **unmerged branch**,
not as a clean pause.

## 1. 18.7.8 was implemented after plan 19 was approved

`STATE_19.md` records the plan status as "Orchestration 18 is to be paused
before 18.7.8 (Wave 0)" and Wave 0 includes "mark `STATE_18.md` paused before
18.7.8". 18.7.8 is not pending — it is implemented, committed, and green:

- Worktree `/home/andrew/AI/hyd.rs/.development/18-worktrees/18.7.8`, branch
  `orchestration/18.7.8`, HEAD `2a22eb1`, parent `4d151a1`. Working tree clean.
- `b0ae8a2 feat(solute): couple root uptake and colloid exclusion`
- `2a22eb1 docs(planning): record 18.7.8 implementation pending review`
- Measured in that worktree: `cargo test -p hydrus-core --lib` 80 passed / 0
  failed (`adapter` 17, `source_coupling` 17), `cargo test --workspace` 27
  suites ok, `cargo clippy --all-targets` clean, `cargo fmt --all --check`
  clean. No parity claim is made; 18.7.5/18.7.7 and the driver still gate those
  paths on Fortran fixtures.
- **Status is "awaiting orchestrator review", not accepted.** The detailed
  porting evidence and the six Fortran-parity findings (per-step `SinkS` clear,
  genuinely-undefined `Omega1` read, `i.eq.N`-first weighting, the
  single-precision `1.e-20` threshold, the `1742` double-narrowing tie that
  round-half-to-even resolves to `0.40000004`, and `1760-1761`'s direct
  `VelC=vNew(1)` assignment) are in `planning/STATE_18.md` on that branch. Do
  not re-derive them.

Consequences for Wave 0:

- Do **not** write "paused before 18.7.8" into `STATE_18.md`. The branch's own
  `STATE_18.md` already records 18.7.8 as implemented pending review, so Wave 0
  as written would overwrite that record or conflict with it on merge.
- Branch topology is clean for merging: merge-base with `8716f06` is exactly
  `4d151a1`, `8716f06` is 3 commits ahead (all docs: `.gitignore`, ADRs,
  plan 19), `orchestration/18.7.8` is 2 ahead. The only expected conflict site
  is `planning/STATE_18.md`, and only if Wave 0 edits it first.
- Decide explicitly whether 18.7.8 is reviewed/merged before or after plan 19.
  Plan 19 tasks 19.1.1 and 19.2.1 touch the compat adapter and module hygiene,
  which is the same `adapter.rs` that 18.7.8 extended by 659 lines — reviewing
  18.7.8 first is probably cheaper than rebasing it over 19.1.1/19.2.1.

## 2. The Python blocker in `STATE_18.md` is stale — confirmed

`STATE_18.md` blocks 18.7.6 (and transitively 18.7.2/18.7.5) on "no usable
interpreter exists on this host". `STATE_19.md` already flags this as stale;
re-confirmed at `8716f06` on 2026-10-14 —
`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py'`
→ **Ran 291 tests, OK** in 103.6 s. Two traps for whoever resumes 18:

- The suite prints solver/CLI JSON to stdout and finishes slowly; a `| tail`
  hides the `OK`/`FAILED` line and looks like a hang. Grep for `^OK|^FAILED|^Ran `.
- `PYTHONDONTWRITEBYTECODE=1` is not optional — a plain `unittest discover`
  leaves `tools/__pycache__/` behind and dirties the tree — observed
  during 18.7.8, and it must be deleted by hand.

So 18.7.6/18.7.2/18.7.5 are blocked on **Fortran fixtures**, not on tooling.
Correct that wording when Wave 0 touches `STATE_18.md`.

## 3. Remaining Orchestration 18 work

1. **Review 18.7.8** and either accept (then record acceptance with the commit
   hash, per the 18.7.1/18.7.3 pattern) or return corrections.
2. **18.7.7** — the `SINK.FOR` root-uptake producer. Still open. 18.7.8 consumed
   producer state directly as primitive inputs, which satisfies 18.7.8's own
   gate but does not close 18.7.7.
3. **18.7.5 → 18.7.2 → 18.7.6** — fixture tasks; blocked on Fortran fixtures,
   now unblocked as far as tooling goes.
4. 18.5.2 remains superseded by 18.7.1; Waves 1–6 and 18.7.1/18.7.3/18.7.4 stay
   accepted.

## 4. Reproducibility trap found during 18.7.8

`.git/info/exclude` ignores `.development/`, which also matches the **worktree's
own `target/`** directory. Symptom: edits appear not to recompile, because early
builds reuse a stale artifact outside git's view. A marker test that should fail
to compile is the quick disambiguation. Cost most of one debugging session; the
rule is still in place at `.git/info/exclude` line 7 (`.development/`), so it
applies to every future worktree under `.development/`.

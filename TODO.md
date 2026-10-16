# TODO

## Goal

Fix every Standards and Spec finding from the review of b7bfd0c, then rewrite that commit as two clean commits: a `.gitignore` chore and the corrected ADR set.

## Tasks

### 1. Renumber and merge

Target numbering (contiguous, 12 records):

| New | Source |
|---|---|
| 0001 | old 0001 |
| 0002 | old 0002 |
| 0003 | merge of old 0003 + 0010 + 0012 (evidence lifecycle) |
| 0004–0009 | old 0004–0009 (unchanged numbers) |
| 0010 | old 0011 (operation order and precision) |
| 0011 | new: dependency policy |
| 0012 | new: Fortran array capacity and output formats |

- [x] Write merged `0003-isolate-and-authenticate-reference-and-verification-runs.md`. It has one decision, run evidence lifecycle: isolated roots, immutable hashed inputs, preserved failures, no attempt/artifact ceilings, explicit comparisons. It keeps the old 0003/0010/0012 source SHAs, uses the full `4a40220` hash, and refers to 0004 for the no-oracle-forcing rule instead of restating it.
- [x] Delete old `0010-*.md` and `0012-*.md`; rename old `0011-*.md` to `0010-reproduce-fortran-operation-order-and-precision.md` and update its ID.
- [x] Rename slugs that don't match titles (0001, 0007, 0010) so each file name is the kebab-case title, or shorten the title to match the slug.

### 2. Fix the template in all records

- [x] Move `## Consequences` under Decision Outcome as `### Consequences` in every ADR.
- [x] Rename `## Validation and Compliance` to `## Validation & Compliance` in every ADR.
- [x] Remove the `**Constraint:**` bullet from every ADR; fold any unique content into Decision Outcome.

### 3. Fix sources, dates, and status

- [x] 0001: cite D01 (`EXECUTION_PLAN_1.md:42`) and `EXECUTION_PLAN_17.md`; state that the 2026-10-08 date covers the solute-emulation scope, with the project oracle choice dating from D01 (2026-09-19).
- [x] 0003 (merged): base "Accepted" on D01 and the executed commits, not `VALIDATION_STRATEGY.md:84`; set the date from the governing source (`EXECUTION_PLAN_18.md:32`, 2026-10-08, for the no-ceilings rule), and record the earlier origin dates in the context.
- [x] 0005 and 0009: add `EXECUTION_PLAN_13.md:25` (the review cited :26; the line is :25) to `source_context`.
- [x] 0009: add `15b21b5` to `source_context`.
- [x] Check every SHA in every ADR is cited the same way in all records (7-char form).

### 4. Remove duplication between records

- [x] Keep the no-oracle-forcing rule only in 0004; 0009 and 0003 link to it.
- [x] Keep the all-mode active-caller scope only in 0008; 0001 links to it.
- [x] Make "source precision" have one owner: 0006 covers comparison tolerances, 0010 covers kernel arithmetic; each links to the other.
- [x] Split 0005 if it holds more than one decision, or reword the title and outcome around the single rule (four evidence classes are kept distinct) so it reads as one decision.

### 5. Add missing ADRs

- [x] Write `0011` for the dependency policy: core stays dependency-free, Python standard-library validation tooling, minimal JSON/SHA-256 support (`EXECUTION_PLAN_13.md:29`, `EXECUTION_PLAN_18.md:187`).
- [x] Write `0012` for keeping the existing Fortran array capacity and output formats (`EXECUTION_PLAN_13.md:28`).

### 6. Rewrite Considered Options

- [x] In each ADR, replace strawman options with alternatives found in the sources (plan decision tables, review files). Where the sources record no alternative, list "Defer" or say no alternative was recorded; don't invent one.

### 7. Plain-language pass (dr-lexus)

- [x] Split paragraphs to 1–3 sentences in every ADR.
- [x] Remove the per-record "this is not proof the gate passed" disclaimers; keep the one in the README.
- [x] Cut "explicit" where it adds nothing; replace slashes with words ("active/callable" → "active and callable"); fix "comprehensive" in 0006; convert passive lines like "is inventoried" to active voice.
- [x] Replace the em-dash in 0012's former text (now 0003) with two sentences.

### 8. Index

- [x] Rewrite `docs/decisions/README.md` rows for the new numbering, titles, dates, and file names; shorten the Context column to short source tags.
- [x] Check every README link resolves and every ADR ID matches its file name.

### 9. Rewrite history

- [x] `git reset --mixed HEAD~1` to undo b7bfd0c while keeping all changes.
- [x] Commit `.gitignore` alone: `chore: ignore .venv and .scratch`.
- [x] Commit `docs/decisions/` alone: `docs: add ADRs derived from prior plans`.
- [x] Confirm `git status` shows only `TODO.md` (plus the unrelated untracked `planning/*_19.md` files) as uncommitted and `git log` shows the two new commits on top of 4d151a1.

## Notes

- b7bfd0c is unpushed and nothing outside `docs/decisions/` links to the ADRs, so renumbering and rewriting are safe.
- Every claim must trace to a planning file or commit; no invented rationale or options.
- `TODO.md` replaces the completed metadata-reconciliation plan (kept in git history at 0576ecd) and stays out of both rewritten commits.
- The ADR template is `~/.claude/skills/adr-distillation/SKILL.md`; it is not edited.

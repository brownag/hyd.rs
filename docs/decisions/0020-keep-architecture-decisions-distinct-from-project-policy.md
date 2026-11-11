---
id: "0020"
title: "Keep architecture decisions distinct from project policy"
status: "Accepted"
date: 2026-11-11
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "docs/decisions/README.md; planning/GOALS.md; planning/master/ROADMAP_AND_RISKS.md; planning/master/M5V_M6_HANDOFF_DECISION.md"
---

# 20. Keep architecture decisions distinct from project policy

## Context and Problem Statement

ADRs and planning documents sometimes make overlapping statements about
milestone gates. When a gate changes, the repo can keep a valid historical
decision next to a conflicting statement that looks current. That obscures which
evidence matters to the user's goal and encourages process to grow beyond its
value.

## Decision Drivers

- A reader must be able to find the one current rule for a milestone gate.
- Architectural history should stay readable as design history.
- Changes to operational policy should keep their approval trail.

## Considered Options

1. Record every milestone, gate, and run-authorization change as an ADR.
2. Keep ADRs for durable architectural choices only, and record operational
   policy in planning records or short dated decision notes.

## Decision Outcome

Chosen option: 2.

- Use ADRs for durable architectural choices and their consequences for system
  design.
- Keep changing milestone status, validation scope, run authorization, and
  project-specific acceptance policy in the relevant planning record or a short
  dated project decision note, not in a chain of operational ADRs.
- For a consequential policy change, record only what a reader needs to navigate
  it: the decision and date, its scope, the earlier rule it changes, and what
  remains in force.
- Keep one clear source for the current rule. Goal boards, roadmaps, and READMEs
  summarize and link to it; they do not redefine it.
- Preserve superseded records as history and link forward rather than rewriting
  them.
- Write a new or revised ADR only when a change alters a durable architectural
  decision.

This ADR does not set scientific thresholds, require campaigns, authorize runs,
or determine milestone status. The M5-V/M6 handoff is recorded as a dated
project decision in [M5V_M6_HANDOFF_DECISION.md](../../planning/master/M5V_M6_HANDOFF_DECISION.md),
not as an ADR.

### Consequences

- **Positive:** ADRs remain useful design history rather than campaign procedure.
- **Positive:** Operational policy can change without losing its approval trail
  or creating competing current rules.
- **Negative:** Some policy changes must be found in planning records, so those
  records must link to their dated decision notes.

## Validation & Compliance

Applied by the 2026-11-11 policy review of the M5-V/M6 handoff. The draft
proposal is retired; this ADR is its accepted form.

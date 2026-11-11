# Draft ADR 0020: Keep architecture decisions distinct from project policy

Status: **Proposal only; not accepted or entered in the ADR index.**

Purpose: keep durable architectural decisions and changing project policy
separate, while preserving a short, navigable history of both. This draft does
not itself decide M5-V or M6 status.

## Proposed ADR

```yaml
---
id: "0020"
title: "Keep architecture decisions distinct from project policy"
status: "Draft; pending user approval"
date: 2026-11-11
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "docs/decisions/README.md; planning/GOALS.md; planning/master/ROADMAP_AND_RISKS.md; M5-V/M6 handoff policy review"
---
```

### Context

ADRs and planning documents sometimes make overlapping statements about
milestone gates. When a gate changes, the repo can retain a valid historical
decision alongside a conflicting statement that looks current. That obscures
which evidence matters to the actual user goal and encourages process to grow
beyond its value.

### Decision

Use ADRs for durable architectural choices and their consequences for system
design. Keep changing milestone status, validation scope, run authorization,
and project-specific acceptance policy in the relevant planning record or a
short dated project decision note—not in a growing chain of operational ADRs.

For a consequential policy change, record only what is needed to navigate it:
the decision and date, its scope, what earlier rule it changes, and what remains
in force. Keep one clear source for the current rule. Goal boards, roadmaps, and
READMEs summarize and link to it; they should not independently redefine it.
Preserve the old record as history and link forward rather than silently
rewriting it.

Use a new or revised ADR only when the change actually alters a durable
architectural decision. This convention does not itself set scientific
thresholds, require campaigns, authorize runs, or determine milestone status.

### Consequences

- Architecture decisions remain useful as design history rather than campaign
  procedures.
- Operational policy can change without losing its approval trail or creating
  competing current rules.
## Relation to the M5-V/M6 decision

The specific [M5-V/M6 handoff](M5V_M6_HANDOFF_DECISION.md) is recorded as a
dated project decision, not an ADR. The user approved that decision separately;
this generic ADR remains a proposal and is not required for the handoff to take
effect. Whether to accept ADR 0020 remains a separate choice.

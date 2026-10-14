---
id: "0005"
title: "Report each evidence class separately"
status: "Accepted"
date: 2026-09-26
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/master/VALIDATION_STRATEGY.md; planning/EXECUTION_PLAN_13.md:25; planning/master/SCIENTIFIC_COMPARISON_RESEARCH.md; planning/EXECUTION_PLAN_18.md; commits 490a41b, f86a028, 3901211"
---

# 5. Report each evidence class separately

## Context and Problem Statement

A source comparison shows whether the port reproduces the pinned source under one input mapping and build. It doesn't show that either implementation solves the continuum equations accurately or matches physical observations. Text output can differ in format while the parsed values agree, and matching text can hide differences below print resolution.

If one report mixes these claims, one kind of evidence passes for another.

## Decision Drivers

- Match each acceptance claim to the evidence that supports it.
- Don't let a parity pass stand in for scientific or physical correctness.
- Don't let a benchmark pass stand in for source compatibility.
- Check text layout apart from numerical values.

## Considered Options

1. Give each evidence class its own criteria, report, and gate.

The sources record no alternative. Every plan from Orchestration 13 on carries the separation forward as a constraint.

## Decision Outcome

Chosen option: "Give each evidence class its own criteria, report, and gate", because each class answers a different question.

The classes are source-snapshot parity, mathematical verification, physical validation, text-layout compatibility, and release compatibility. A pass in one class says nothing about another. When compatibility is the goal, analytical or physical evidence doesn't replace source parity.

We don't call a diagnostic threshold approved, call parity a scientific verdict, or treat byte equality as numerical proof.

### Consequences

- **Positive:** An open question in one class, such as an unassigned tolerance, doesn't void evidence in another.
- **Negative:** We maintain several reports and gates. An overall release claim stays blocked while any one gate is open.

## Validation & Compliance

`planning/master/VALIDATION_STRATEGY.md:32,50` lists the separate claims and the text-layout rule. `planning/EXECUTION_PLAN_13.md:25` (2026-09-26) carries the separation forward as an inherited constraint, and the date comes from there. `planning/master/SCIENTIFIC_COMPARISON_RESEARCH.md` leaves scientific thresholds unassigned.

The precision review (`f86a028`) records tolerance gates as unmet. The verification contract (`490a41b`) and the compatibility contract (`3901211`) each keep their own class.

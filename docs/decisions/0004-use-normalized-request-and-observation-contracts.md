---
id: "0004"
title: "Use normalized request and observation contracts"
status: "Accepted"
date: 2026-10-08
authors:
  - "Andrew G. Brown"
supersedes: null
superseded_by: null
source_context: "planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md; planning/EXECUTION_PLAN_18.md; planning/master/SOLUTE_VERIFICATION_SPEC.md; commits 3901211, bce44da, 490a41b"
---

# 4. Use normalized request and observation contracts

## Context and Problem Statement

Legacy solute behavior depends on one-based arrays, material and species parameter blocks, unit conversions, boundary selectors, and state from other HYDRUS components. Native input and output files aren't a reliable interface between two implementations. Guessed units, guessed orientation, silent defaults, or Fortran outputs fed back as inputs would make paired tests ambiguous and hide missing state.

The Wave 2 review on 2026-10-08 found that the runner authenticates raw Fortran output but doesn't normalize it. Orchestration 18 then added a manifest-driven normalizer and required observation maps in every fixture manifest.

## Decision Drivers

- Give Rust and Fortran the same meaning in their inputs, whatever their native file formats.
- Keep units, indexing, orientation, switch combinations, availability, and provenance visible in the request.
- Reject malformed or unsupported requests before any state changes.
- Compare outputs by field, species, time, and location, and keep source print precision.

## Considered Options

1. Compare native or raw output files and rely on matching layout and order. This was the Wave 2 state.
2. Use a versioned, normalized request and observation schema with mappings, units, identity, provenance, and availability.

The sources record no other alternative.

## Decision Outcome

Chosen option: "Use a versioned, normalized request and observation schema", because it lets a failure point at a model difference rather than a mapping error.

The request names its schema, units, coordinate convention, and species, material, and node mapping. It also carries flags and valid combinations, external state histories, unavailable values with reasons, and input and source hashes. The observation contract names field units, species, time, location, source columns, row selection, the source print interval, and availability.

Both solvers receive the same normalized request. Fortran output is never forcing input for the Rust candidate. We never invent missing observations or default absent arrays to zero.

Identities and discrete fields must match exactly. Continuous values use the field-specific criteria in [0006](0006-set-parity-tolerances-per-field-from-source-precision.md). Schema changes need review.

### Consequences

- **Positive:** A failure can be traced to a model difference or to a mapping, orientation, unit, or identity error.
- **Positive:** Missing source evidence shows up as unavailable, not as zero or as inferred balance closure.
- **Negative:** Manifests, observation maps, and strict validation take more work than comparing files.
- **Negative:** We can't compare internal values the unmodified source doesn't print.

## Validation & Compliance

`planning/master/SOLUTE_FORTRAN_COMPATIBILITY_18.md` (`3901211`) holds the semantic contract. `planning/EXECUTION_PLAN_18.md` sets the request and observation-map rules, including "never Fortran outputs as forcing" in the paired CLI task. `planning/master/SOLUTE_VERIFICATION_SPEC.md` (`490a41b`) applies the same rule to verification.

The adapter foundation (`bce44da`) implements typed mapping and rejects bad input before mutation.

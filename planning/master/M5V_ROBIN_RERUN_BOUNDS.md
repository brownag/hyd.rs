# Proposed bounds for the post-fix Robin spatial rerun

Date: 2026-11-11. Status: **draft for a new Gate B review**. This proposal
does not authorize a solver start, classification, or acceptance. The signed
23R.2 bounds and review remain unchanged historical artifacts.

## Scope and identity

The proposed new bounds source covers only these four existing V2 launch IDs:

| Launch | Role | Nodes | Fixed dt | Timeout |
|---|---|---:|---:|---:|
| `convergence-scalar_robin-space_129-dt2e-7-rfc-baseline` | study | 129 | 0.0078125 | 60 s |
| `convergence-scalar_robin-space_193-dt2e-7-rfc-baseline` | study | 193 | 0.0078125 | 60 s |
| `convergence-scalar_robin-space_257-dt2e-7-rfc-baseline` | study | 257 | 0.0078125 | 60 s |
| `convergence-scalar_robin-space_control-257-dt2e-8-rfc-baseline` | control | 257 | 0.00390625 | 60 s |

The casebook SHA-256 remains
`420af7c0085f3ad5cf3335ca036ca5360d7abd370807451d8d479e653a215df7`.
`scalar_robin.chemistry.upstream_weighting` is `true`; new V2 public Rust
requests carry that value through the fixed `FortranCompatible` mapping. No
Robin-time, other-case, threshold, or holdout row is included.

## Proposed derivation

The prior signed bounds contain 190 entries for each of these four launch IDs.
For every entry, the historical arithmetic derivation used
`gamma_N * S`, with `u = 2^-24`, `gamma_N = N*u/(1-N*u)`, `N = nodes +
steps_at_or_before_time`, and `S = max(1, abs(value))`. Read-only inspection
confirms all 760 historical entries used `S=1`. The cases prescribe fixed
timesteps, and the retained records show the expected fixed step schedules.
The proposal therefore carries forward only the arithmetic values, because
their recorded `N` and `S=1` are determined by the fixed row schedule and
conservative unit scale—not by the old upstream-weighting result values.

All quadrature terms are set to `null` in this scoped proposal. Historical
empirical quadrature sensitivities depend on the old trajectory and are not
carried forward. Coordinate uncertainty remains zero because the Rust rows use
the exact requested mesh. The result is intentionally conservative: any field
requiring quadrature evidence remains unclassifiable.

**Validity condition:** the new observations must contain exactly the fixed
step schedule used to derive `N`, and every measured metric magnitude at every
bound key must be at most 1. If either condition fails, stop before treating
the classification as evidence; prepare a revised derivation and obtain a
new Gate B review. Do not relax this condition after seeing the results.

This is a proposed uncertainty budget, not a claim that the new numerical
errors will be small or monotone. The mapping change can alter numerical
results. The new review must explicitly accept the four-row scope, the reuse
of schedule-derived arithmetic with `S=1`, and null quadrature for every row.

## Provenance and preserved history

The new bounds and review draft live under
`.development/23-robin-rerun/`. They bind to the current 11-tool producer map
and the unchanged casebook SHA. The prior full-campaign bounds/review are not
modified. Their Gate B authorization remains attached to their original
producer identity and does not apply to this draft.

Derivation evidence includes the historical batch recipe, its four saved
observations, the original classification and verification receipt, the
historical signed bounds/review, the current casebook, and the mapping-fix
investigation. These records establish the old schedules and the generic
arithmetic method only; they are not post-fix solver results.

## Gate and run boundary

No campaign run may start using this proposal until the user reviews and signs
the new Gate B review file. A signed review permits only the four launch IDs
above, subject to a separately approved execution procedure and the validity
condition. It does not authorize time rows, holdouts, changed bounds, retries,
or any broader M5-V claim. Existing pre-fix artifacts remain immutable and
historical.

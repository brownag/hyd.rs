# Architecture Decision Records

| ID | Title | Status | Date | Supersedes | Superseded By | Context |
|---|---|---|---|---|---|---|
| [0001](0001-treat-pinned-fortran-snapshot-as-solute-reference.md) | Treat the pinned Fortran snapshot as the solute reference | Accepted | 2026-10-08 | - | - | D01; Orch 17–18 |
| [0002](0002-keep-fvbe-and-source-compatible-backends-separate.md) | Keep the FV/BE and source-compatible backends separate | Accepted | 2026-10-08 | - | - | Orch 18 |
| [0003](0003-isolate-and-authenticate-reference-and-verification-runs.md) | Isolate and authenticate reference and verification runs | Accepted | 2026-10-08 | - | - | D01; Orch 18 |
| [0004](0004-use-normalized-request-and-observation-contracts.md) | Use normalized request and observation contracts | Accepted | 2026-10-08 | - | - | Orch 18 |
| [0005](0005-report-each-evidence-class-separately.md) | Report each evidence class separately | Accepted | 2026-09-26 | - | - | Orch 13 |
| [0006](0006-set-parity-tolerances-per-field-from-source-precision.md) | Set parity tolerances per field from source precision | Accepted | 2026-10-08 | - | - | Orch 18 |
| [0007](0007-audit-fortran-reachability-by-fixed-form-columns-and-callers.md) | Audit Fortran reachability by fixed-form columns and callers | Accepted | 2026-10-10 | - | - | Orch 18 amendment |
| [0008](0008-gate-solute-parity-on-audited-active-modes.md) | Gate solute parity on audited active modes | Accepted | 2026-10-08 | - | - | Orch 18 |
| [0009](0009-verify-solver-mathematics-against-independent-references.md) | Verify solver mathematics against independent references | Accepted | 2026-09-26 | - | - | Orch 13 |
| [0010](0010-preserve-source-operation-order-and-precision.md) | Preserve source operation order and precision | Accepted | 2026-10-08 | - | - | Orch 18 |
| [0011](0011-add-dependencies-only-by-approved-plan-amendment.md) | Add dependencies only by approved plan amendment | Accepted | 2026-10-09 | - | - | Orch 13; Orch 18 amendment |
| [0012](0012-keep-reference-build-array-capacity-and-output-formats.md) | Keep the reference build's array capacity and output formats | Accepted | 2026-09-26 | - | - | Orch 13 |

## Scope and Status Notes

These records distill decisions the user made or approved in the planning documents. An accepted ADR doesn't mean its implementation or acceptance gate is complete. The all-mode solute parity gate and the default backend switch are still pending.

Non-solute architecture proposals stay as recommendations or user gates in the planning documents, not ADRs. They cover API hosts and formats, memory and solver policy, optional feature scope, and licensing review.

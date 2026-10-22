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
| [0013](0013-select-solute-backend-with-a-closed-enum.md) | Select the solute backend with a closed enum | Accepted | 2026-10-14 | - | - | Orch 19 |
| [0014](0014-validate-kernel-primitive-state-in-the-kernel.md) | Validate kernel primitive state in the kernel | Accepted | 2026-10-14 | - | - | Orch 19 |
| [0015](0015-keep-precision-faithful-hydraulic-ports-in-compat-kernels.md) | Keep precision-faithful hydraulic ports in compat kernels | Accepted | 2026-10-14 | - | - | Orch 19 |
| [0016](0016-add-serde-derive-for-request-decoding.md) | Add serde derive for request decoding | Accepted | 2026-10-14 | - | - | Orch 19 |
| [0017](0017-propagate-nonfinite-endpoint-primitives-as-fortran-does.md) | Propagate non-finite endpoint primitives as the pinned Fortran does | Accepted | 2026-10-19 | - | - | Orch 20 (20.6.1) |

## Scope and Status Notes

These records distill decisions the user made or approved in the planning documents. An accepted ADR doesn't mean its implementation or acceptance gate is complete. Regarding ADR 0002 and ADR 0013, `FortranCompatible` is the default solute backend following the `21.4.1` gate, where genuine Rust-vs-Fortran parity passed across all 44 fixtures (`17d9d9c`, `planning/master/SOLUTE_FORTRAN_PARITY_18.md`). `FiniteVolumeBackwardEuler` remains selectable through `SoluteConfig::with_backend`.

Non-solute architecture proposals stay as recommendations or user gates in the planning documents, not ADRs. They cover API hosts and formats, memory and solver policy, optional feature scope, and licensing review.

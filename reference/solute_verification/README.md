# Solute verification cases

[CASES.json](CASES.json) is the versioned mathematical-study contract for
Orchestration 13. Its [specification](../../planning/master/SOLUTE_VERIFICATION_SPEC.md)
defines equations, independent reference methods, source mappings, request/
observation/manifest formats, metrics, uncertainty budgets and launch gates.

Validate without executing a solver:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_solute_verification_spec.py --self-test
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_solute_verification_spec.py reference/solute_verification/CASES.json
```

Six executable cases cover stored-mass reaction chains (distinct, equal and
zero rates), closed finite diffusion, smooth scalar Robin-inlet transport,
and the actual finite-domain Example 3 chain. A manufactured-forcing case is
explicitly blocked because its forcing hook is outside scope. Executable
classification does not mean numerical results or scientific thresholds have
passed. The full chain has a feasible Decimal inverse-Laplace method;
Wave 3 must admit every observable using its own uncertainty estimate.

The matrix contains 77 conditional launches: 14 benchmark/bridge, 57
convergence/sensitivity and six reserved holdouts. Failed attempts consume
their row, each process has a 60-second limit, and no automatic retries are
allowed. Total planned storage cap is 2,699,462,776 bytes. Dry-run and
verify-only commands never launch a solver. Holdouts are used only after
threshold review. Existing evidence and fixture bytes remain immutable.

Bridge rows preserve the original Example 3 deck and its adaptive controls;
`dt=.1` records its previously observed step history. All refinement rows use
fixed timesteps. The 1,001-node and existing printed-format limits remain in
force; the temporal full-case control cannot use a finer grid and may leave
spatial contamination unresolved. Report that limitation rather than changing
the source or promising convergence.

Adapters exchange a canonical flat ASCII request and normalized JSON
observations as specified in the contract. Generated decks, raw Fortran
outputs, request/observation arrays and numerical scratch belong only under
the executing task's unique `.development/<task>/` output root. This directory
stores authored recipes and provenance, not legacy solver output.

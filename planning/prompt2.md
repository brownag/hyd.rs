Research the concentration printed-parity failures in this repository and identify the source-supported cause and a concrete remediation path. Write one Markdown report in `planning/master/`, following the existing research-note style.

Use local repository materials only; do not browse the web. Inspect the full `.development/14.3.1/printed_parity.json`, `planning/master/SOLUTE_PRINTED_PARITY_EVIDENCE.md`, the comparator and its tests, the Fortran validation/parser, the retained Fortran source under `src_reference/`, and the Rust solute solver and observation-output code.

First reconcile the failure count. The evidence table appears to show 9,092 concentration failures in **five** cases, not four:

- `reaction_distinct`: 855
- `reaction_equal`: 864
- `diffusion_cosine`: 440
- `scalar_robin`: 1,386
- `example3`: 5,547
- `reaction_zero`: 0

Verify these figures from the detailed JSON and correct the “four cases” claim if confirmed.

Trace each comparison from its Fortran output token through parsing, field/species/time/node matching, and interval calculation to the Rust binary64 concentration value. Check whether the failures are real numerical differences or could come from a comparator defect, including species ordering, profile-time matching, coordinate orientation, concentration column selection, and Fortran format/exponent handling. Analyze residual patterns by case, species, time, and location using retained results only. Trace relevant Rust and Fortran solver paths and case configuration to determine whether the evidence supports a specific implementation cause.

Separate verified observations from hypotheses. Cite repository files and symbols (with line numbers where available). State clearly what remains unproven and what smallest next step would confirm it.

Do not edit solver, comparator, or test code. Do not run solvers, builds, benchmarks, or analytical references. Read-only analysis of existing artifacts and source is allowed. Deliver only the research Markdown note and a concise completion summary with its path.

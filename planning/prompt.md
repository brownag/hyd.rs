Use the plan-wave skill to propose a new, reviewable series of waves that gets us to systematic practical parity testing between the Rust solver and the Fortran program across all executable examples in `reference/solute_verification/CASES.json`.

Goal: determine whether Rust reproduces the values users see in the Fortran tool’s printed output. Compare each field using its actual Fortran format and rounding interval, including small values in scientific notation. Report parity by example and field. Do not pursue accuracy beyond what Fortran’s printed output can establish unless a separate user-approved output-format change is proposed.

Start with read-only reconnaissance. Read the relevant plan and evidence files, including:
- `planning/EXECUTION_PLAN_13.md`
- `planning/STATE_13.md`
- `planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md`
- `planning/master/SOLUTE_ANALYTICAL_EVIDENCE.md`
- `planning/master/SCIENTIFIC_COMPARISON_RESEARCH.md`
- `planning/master/SOLUTE_VERIFICATION_SPEC.md`
- `reference/solute_verification/CASES.json`
- the existing Fortran/Rust comparator and bridge implementation

Important existing facts to account for:
- Orchestration 13 tasks 13.3.1 and 13.4.3 are marked complete. Task 13.4.1 is blocked with partial evidence; 13.5.1 and 13.6.1 have not started.
- The 13.4.1 convergence work consumed 32 starts. Some Fortran runs failed validation, some groups are incomplete, and seven groups were classified inconclusive. The detailed outcomes are in `planning/master/SOLUTE_CONVERGENCE_EVIDENCE.md`.
- A separate Example 3 bridge comparison already exists. It reports zero comparator failures for the retained Fortran fields, with field-specific print-rounding bounds; Rust output passed the comparison against those printed values. This is evidence for that fixture only.
- The Fortran concentration format is `e12.4`, about five significant digits. Other fields use different formats. Do not apply one global tolerance to every output.
- The 400- and 650-digit settings were for analytical reference calculations, not solver output. Do not propose more such calculations for printed-output parity.
- The previous storage stop was based on project lifecycle accounting, not evidence that the physical disk was full. Reassess resource planning from actual retained artifact sizes, temporary/build needs, and a reasonable safety margin. Do not repeat or silently inherit the old oversized 57-start matrix.
- No solver run or new analytical reference calculation is authorized by this planning request.

The plan should:
1. Inventory each executable example and the exact paired Fortran/Rust artifacts already available.
2. Separate work that can be completed by read-only comparison of existing artifacts from cases that truly need new solver runs.
3. Inspect and reuse the existing field-format-aware comparator where it fits. Identify specific gaps with file/function anchors; do not assume the comparator needs replacement.
4. Propose the smallest sequence of waves to:
   - make the existing Example 3 result a clearly stated printed-output parity result;
   - systematically compare every other executable example where paired retained outputs exist;
   - run only the minimum missing Fortran/Rust cases needed for remaining coverage, each with explicit case, output fields, commands, timeouts, artifact/storage estimates, and stop conditions;
   - repair or isolate the recorded validation failures before any replacement run is proposed;
   - produce a final coverage table with per-example/per-field pass, fail, or unavailable status.
5. Define success as agreement within each Fortran field’s rounding interval (or a specifically justified comparator rule), not 12-digit continuum accuracy. Preserve raw differences and identify cases where a printed interval is too wide to be useful.
6. Keep solver changes, Fortran output precision changes, new physics, threshold setting, and M5-V closure out of scope unless a concrete evidence gap makes one necessary. If more Fortran digits are needed for a particular claim, present that as a separate optional user decision with cost and implications.
7. Rework dependencies/status honestly: explain whether 13.4.1 must be superseded, narrowed, or left as historical blocked convergence work, and whether the existing 13.5.1/13.6.1 sequence still makes sense for this parity goal.
8. Follow plan-wave’s interview process: inspect first, then ask me one decision question at a time. Don’t batch questions and don’t draft or edit planning artifacts until the required decisions are answered.
9. Deliver a task DAG, wave table, right-sized task specs, worker prompt template, state scaffold, resource/accounting plan, risks, and independent spec/standards review. Stop for my approval; do not execute the proposed waves.

Keep the writing direct. The core question is simple: for each executable example and each comparable output field, does Rust reproduce what the retained Fortran program prints?

# Fortran source defects register

Suspected defects in the pinned HYDRUS Fortran source (`src_reference/*.FOR`)
that the Rust compatibility port (`crates/hydrus-core/src/solute_fortran_compat/`)
reproduces on purpose, so its output matches the reference program.

## Rules

- The compatibility port keeps each defect exactly as the source has it. A fix
  belongs in a separate, documented non-compat mode and must not claim
  Fortran parity.
- Each entry has an ID (`SD-NNN`). Code that reproduces it carries a comment
  naming the ID and this file.
- Each entry has at least one pinning test named `source_defect_sdNNN_*`, with
  a doc comment naming the ID. The test must fail if the defect is "fixed".
  `cargo test -p hydrus-core source_defect_` runs them all.
- Add an entry only after checking the behaviour against every other site in
  the source that computes the same quantity. A pattern that is consistent
  everywhere is a modelling choice, not a defect (see "Not defects").

## Entries

### SD-001 — `MassInit` scales Henry's constant by the sorption temperature coefficient

- **Source:** `SOLUTE.FOR:1933` (`MassInit`):
  `Par(6)=ChPar(jjj+10,M)*exp(TDep(jjj+ 7)*TT) !xKH`
- **Expected:** `TDep(jjj+10)`. All ten other Henry's-constant sites use it,
  e.g. `SOLUTE.FOR:80,210,329,462,544,890,923,1529`.
- **Effect:** wrong liquid concentration from total-mass initialization
  (`lMassIni`) when the gas phase matters (`xKH≠0`), the temperature differs
  from the reference, and `TDep(jjj+7)≠TDep(jjj+10)`.
- **Port:** `solute_fortran_compat/initialization.rs` (`mass_init`).
- **Pinning test:** `initialization::tests::source_defect_sd001_mass_init_xkh_uses_xks_temperature_factor`.
- **Status:** reproduced; not reported upstream.
- **Modern HYDRUS:**
  - **Version:** untested
  - **Date:** untested
  - **Verdict:** untested
  - **Evidence:** untested

### SD-002 — Bacteria second-site sorption uses the first site's rates

- **Source:** `SOLUTE.FOR:1942` (`MassInit`) and `SOLUTE.FOR:1698` (`NonEqInit`):
  `xKs2=Theta(i)*rKa1/.../rKd1`. `rKa2`/`rKd2` are computed just before
  (`:1940–1941`, `:1696–1697`) and never used.
- **Expected:** `rKa2/.../rKd2`.
- **Effect:** with bacteria transport (`lBact`) and two sites with different
  rates, initialization sets the second site as a copy of the first:
  `Sorb2` in `NonEqInit`, and `Par(3)=2·xKs1` in `MassInit`.
- **Port:** `solute_fortran_compat/initialization.rs` (`mass_init`, `non_eq_init`).
- **Pinning tests:**
  `initialization::tests::source_defect_sd002_mass_init_xks2_reuses_first_site_rates`,
  `initialization::tests::source_defect_sd002_bacteria_xks2_reuses_first_site_rates`.
- **Status:** reproduced; not reported upstream.
- **Modern HYDRUS:**
  - **Version:** untested
  - **Date:** untested
  - **Verdict:** untested
  - **Evidence:** untested

## Probe procedure

To determine whether a source defect persists or has been resolved in modern or closed-source HYDRUS builds:

1. **Paired probe decks:** Each register entry is probed using two input deck pairs:
   - **Defect pair:** Varies only the parameter that correct code would read and the defect ignores. On pinned Fortran (defect present), both runs yield identical output.
   - **Control pair:** Varies a parameter that every version reads at the same site, verifying that the output file and variable respond to changes. On any working solver, runs yield different outputs.
2. **Execution:** Run the probe decks on the target HYDRUS build and collect output files (e.g. `NOD_INF.OUT` or `OBS_NODE.OUT`).
3. **Automated comparison:** Run `python3 tools/compare_defect_probe.py --manifest <path> --outputs <dir>`:
   - Defect pair values are compared for exact match (`identical`).
   - Control pair values are compared against a relative threshold (`different`).
4. **Verdicts:**
   - `defect present`: Defect pair is identical and control pair differs.
   - `defect fixed`: Both defect pair and control pair differ.
   - `inconclusive`: Control pair is identical (test lacked sensitivity), or outputs are missing/unparsable.
5. **Register update:** Record the build version, date, verdict, and evidence path in the entry's **Modern HYDRUS** field.

## Not defects

- **Freundlich exponent `fExp` has no temperature factor.** The `TDep` factor
  is commented out at every site (`SOLUTE.FOR:328,461,543,1377,1382,1408,1413,1684,1932`),
  so this is consistent and treated as intended.

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

### SD-003 — `Solute` overwrites the water iterate `hTemp` through its `vCorr` argument

- **Source:** `HYDRUS.FOR:510-511` passes `hTemp` in the `vCorr` position of
  `call Solute(...)`; `SOLUTE.FOR:11,30` declare it `vCorr(NumNP)` and
  `SOLUTE.FOR:741` assigns `vCorr(i)=ThG*Dg*TauG*derK` for every node.
- **Expected:** a separate scratch array. `hTemp` is the previous water iterate
  that `SetMat` (`WATFLOW.FOR:470,473`), `Reset` and `Update` read on the next step.
- **Effect:** after each solute step with `lChem`, `hTemp` holds the gas-diffusion
  velocity correction (0 when `Dg=0` or the Henry gradient is 0). `Update`
  rewrites `hTemp` at most nodes, so the alias shows at nodes it does not
  rewrite (Dirichlet boundary nodes) through the `hi1` blend in `SetMat`, which
  changes `Con`, `Cap` and `ThEq`, and so step selection.
- **Port:** `solute_fortran_compat/compat_runner.rs` (`alias_h_temp_to_v_corr`,
  called after the solute step). It copies the solver's final `v_corr`
  (`driver.rs`, `st.v_corr`), not zero, so decks with `Dg>0` are exact.
- **Pinning test:** `compat_runner::tests::source_defect_sd003_htemp_aliases_vcorr_on_dirichlet_nodes`.
- **Status:** reproduced; not reported upstream.
- **Modern HYDRUS:**
  - **Version:** untested
  - **Date:** untested
  - **Verdict:** untested
  - **Evidence:** untested

### SD-004 — `SetMat` table interpolation reads row `NTab+1`

- **Source:** `WATFLOW.FOR:482-484` (`Con`) and `:515-518` (`Cap`, `Theta`):
  `iT=int((alog10(-hiM)-alh1)/dlh)+1`, then `hTab(iT+1,1)`, `ConTab(iT+1,M)`,
  `CapTab(iT+1,M)`, `TheTab(iT+1,M)`. `iT` is not bounded by `NTab-1`.
- **Expected:** `iT` limited to `NTab-1`. The `iModel.eq.nTabMod` branches
  (`:491-497`, `:527-533`) loop `j=1,NTab-1` and cannot overrun, so the
  `lTable` branches are the only unbounded sites.
- **Effect:** when `hiM` is within rounding of the last node `hTab(NTab)`,
  `iT=NTab` and the interpolation reads row `NTab+1`. The tables are
  `ConTab(NTabD,NMatD)` with `NTabD=100`, `NMatD=20` (`HYDRUS.FOR:48-50,75,90`)
  and `NTab(1)=100` (`INPUT.FOR:1590`), stored column-major, so
  `ConTab(101,M)` is `ConTab(1,M+1)`: the next material's first row, and zero
  for the last material. Only column 1 of `hTab` is generated for
  `iModel<nTabMod` (`INPUT.FOR:774-781`), so `hTab(101,1)` is `hTab(1,2)=0`.
- **Port:** `solute_fortran_compat/water_compat.rs` (`tab_elem` and the `lTable`
  branches of `set_mat`). The old clamp `iT<=NTab-2` is gone; `tab_elem`
  reproduces the column-major read with the same `NTabD`/`NMatD`.
- **Pinning test:** `water_compat::tests::source_defect_sd004_set_mat_reads_past_table`
  (two materials; fails if the read is clamped).
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

# HYDRUS Source Defect Probes

This directory contains HYDRUS-1D input deck pairs for black-box probing of known defects in historical HYDRUS Fortran source, as registered in `docs/FORTRAN_SOURCE_DEFECTS.md`.

Each probe is constructed as two pairs of decks:
1. **Defect pair (`deck_a` vs `deck_b`):** Varies only the parameter that correct code reads and the defective code ignores.
2. **Control pair (`deck_a` vs `deck_b`):** Varies a parameter read at the same site by every version.

On the pinned Fortran snapshot, the defect pair produces byte-identical compared output at $t=0$, while the control pair produces different output. When evaluated on modern or modified versions of HYDRUS-1D, if the defect has been fixed, both pairs will produce different outputs.

## Register Entries Probed

### SD-001: Henry's Constant Temperature Scaling in `MassInit`
- **Location:** `SOLUTE.FOR:1933` (`MassInit`)
- **Defect:** Liquid concentration from total mass initialization (`lMassIni`) scales Henry's constant `xKH` using the sorption temperature coefficient `TDep(jjj+7)` rather than its own `TDep(jjj+10)`.
- **Defect pair:**
  - `sd001_defect_a`: `TDep(jjj+10) = 0.0`
  - `sd001_defect_b`: `TDep(jjj+10) = 5000.0`
- **Control pair:**
  - `sd001_control_a`: `TDep(jjj+7) = 500.0`
  - `sd001_control_b`: `TDep(jjj+7) = 5000.0`
- **Compared output:** `NOD_INF.OUT` at `Time: 0.0000`, variable `Conc(1)`.

### SD-002: Bacteria Second-Site Sorption in `NonEqInit` and `MassInit`
- **Location:** `SOLUTE.FOR:1698` (`NonEqInit`) and `SOLUTE.FOR:1942` (`MassInit`)
- **Defect:** With bacteria transport (`lBact`) and equilibrium initialization (`lEqInit`), the second-site equilibrium sorption coefficient `xKs2` is calculated from the first site's rate constants `rKa1 / rKd1`, completely ignoring `rKa2 / rKd2`.
- **Defect pair:**
  - `sd002_defect_a`: `rKa2 = 2.0`
  - `sd002_defect_b`: `rKa2 = 8.0`
- **Control pair:**
  - `sd002_control_a`: `rKa1 = 0.5`
  - `sd002_control_b`: `rKa1 = 2.5`
- **Compared output:** `NOD_INF.OUT` at `Time: 0.0000`, variable `Sorb2(1)` (and `Sorb(1)`).

---

## Instructions for Running on Modern HYDRUS-1D

To test these probe decks on a modern HYDRUS-1D executable (e.g., HYDRUS-1D version 4.17 or later):

1. **Copy the Input Decks:**
   Each subdirectory contains:
   - `SELECTOR.IN` (the HYDRUS main configuration and solute parameter file)
   - `PROFILE.DAT` (the initial pressure head and concentration grid profile)

   Create a separate project directory for each run in HYDRUS-1D or run them via the HYDRUS engine command line.

2. **Execute HYDRUS-1D:**
   Run the solver for each deck:
   - `sd001_defect_a`
   - `sd001_defect_b`
   - `sd001_control_a`
   - `sd001_control_b`
   - `sd002_defect_a`
   - `sd002_defect_b`
   - `sd002_control_a`
   - `sd002_control_b`

3. **Output Files to Return:**
   Collect and return the following output file for each deck:
   - `NOD_INF.OUT` (primary profile output containing nodal concentrations and sorbed amounts at each print time, including $t=0$).
   - Optionally: `I_CHECK.OUT`, `RUN_INF.OUT`, `T_LEVEL.OUT`, `BALANCE.OUT`, `solute1.out`.

4. **Interpreting Results:**
   Run `tools/compare_defect_probe.py` (added in task 18.17.2) passing `MANIFEST.json` and the directory containing the modern run outputs:
   - **`defect present`**: The defect pair output at $t=0$ is identical and the control pair differs.
   - **`defect fixed`**: Both the defect pair and the control pair differ at $t=0$.
   - **`inconclusive`**: The control pair is identical, or outputs are missing or unparsable.

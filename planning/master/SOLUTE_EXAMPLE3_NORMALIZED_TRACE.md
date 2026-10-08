# Example 3 first-interval normalized trace

## Result

**Status: complete; first-interval Fortran evidence captured and normalized. Disposition: `no_fix_cause_unisolated`.** The measured rows and solved values establish an early divergence between the retained Fortran and Rust formulations. They do not establish which formulation is required for the Rust implementation, so this evidence does not justify a numerical correction. Printed parity is not evidence of physical or continuum accuracy.

## Provenance and run

The diagnostic used task-local preparation `prepare-0005-example3`, preserving preparations 0001–0004, including the earlier flawed instrumentation attempt. It kept the full authored Example 3 setup: 1001 nodes, three species, 250 cm domain, 200 hours, 2000 steps, and printed profiles at 0/50/100/200 h. The instrumented Fortran source measured the first `Solute` entry at `t=0.10000000149011612 h`, with `dt=0.100000001 h`. The Rust task-15 trace records `dt=0.1 h`.

Authenticated identities:

- Casebook `reference/solute_verification/CASES.json`: 73,947 bytes, SHA-256 `d3b06531c06545ca89956b95c487bb1616ace1470ba1a8c02a3f1209ff4e8acf`.
- Printed-parity inventory: 546,509 bytes, SHA-256 `616662ba6dbfe57e07c4e59a5d231d7feda9cb38dfb0a078414182afb678219c`.
- Canonical Rust request/manifest: `8624a4f9bd1aedab1ee15a74f7738dff2d63cbe117397df490478cdbcfe49cd8` / `43f27dd2acd665dd4596e0c0cb5fd88952aa4f413f931ef6641765d1ac91fbd2`.
- Canonical Fortran request/manifest: `dcd66add26410bb55d3ded63f33525bef1bcf2478deed7cbad2f3b0e3e1649b0` / `8deb39852934456517baf11509876ba59b7065ec0079c8256ad010c5ee7154e5`.
- Input hashes in preparation 0005: `SELECTOR.IN` 1,831 bytes, `5295301f190125560c5574a0c1ca63c50f9818b13dd611ec99040b5e7e949d69`; `PROFILE.DAT` 34,588 bytes, `81bc1fa627743fa2a6794d3b34a68e409aa00c190c871188b32596b61903a60e`; `LEVEL_01.DIR` 2 bytes, `eb4bd64f7014f7d42e9d358035802242741b974e8dfcd37c59f9c21ce29d781e`.
- Preparation wrapper `.development/16.4.1/prepare_example3.py`: SHA-256 `6b4d2bfd6fdbf3dca50c3af0b1fd3dd198855e50ec8407d03a3e28fd16cf3fb0`. Corrected task-local `SOLUTE.FOR`: 78,923 bytes, SHA-256 `4b7252c37d9a0155cc203cb18dfb125c7d65b687e578b2ba4a2658dd1c47a48c`; its canonical source hash is `81fed6adbbd18aabe53385151166ab36d0df504a7939b54e10460c2254488910`.
- Retained Rust trace `.development/15.3.1/runs/example3-rust-15.3.1/stderr.txt`: 4,196 bytes, SHA-256 `6e7c246301f573af69096213e65da4363b23f28345aacb92da87b639148d4440`.

The reviewed build used `/usr/bin/gfortran -std=legacy -ffixed-line-length-none`, exited 0 in 0.709 s, and produced executable SHA-256 `95a865dbbb4652ca4e7ccd2b3af9403d71674fe5569dd8d6f0e4fc53767b37e5`. Launch `launch-0007-example3-first-interval-1` exited 0 in 2.049 s and retained 3,079,998 artifact bytes. Its captured stdout is 3,787 bytes, SHA-256 `e70736985182c0492531765f6c6d6e41d7334a10961140422a34c19fa25e9948`; stderr is 96 bytes, SHA-256 `cde5e7d17e64813704a51a756095eaf585d28601c664fa68a1f27038b6399fcc`, with the legacy IEEE underflow/denormal notice. The trace contains one measured-entry row, nine assembly rows and nine post-solve values. Runner verification passed: 59 retained files, four aliases, seven unique attempts, and all attempt artifact hashes verified.

## Coordinate and row normalization

The emitted coordinates establish the orientation: Fortran `i=1, x=-250` is Rust `s=250`, node 1000 (outlet); `i=501, x=-125` is Rust `s=125`, node 500 (midpoint); and `i=1001, x≈0` is Rust `s=0`, node 0 (inlet). This uses `s=-x` and the measured coordinates, not the earlier assumed endpoint order.

Fortran's tridiagonal equation is `B(i) C(i-1) + D(i) C(i) + E(i) C(i+1) = F(i)`. Because its node order is reversed relative to Rust, its row in Rust order is `(lower=E, diagonal=D, upper=B, rhs=F)`. The table compares coefficients at identical physical locations. Entries are `(lower, diagonal, upper; rhs)`.

| Species | Location | Fortran, normalized to Rust order | Rust retained trace |
|---|---|---:|---:|
| NH4 | inlet, 0 cm | `(0, 1.361598, -0.071330; 0.430000)` | `(0, 1.815138, -0.309600; 0.430000)` |
| NH4 | midpoint, 125 cm | `(-0.286330, 2.508197, -0.071330; 0)` | `(-0.739600, 3.200275, -0.309600; 0)` |
| NH4 | outlet, 250 cm | `(1, -1, 0; 0)` | `(-0.739600, 1.815138, 0; 0)` |
| NO2 | inlet, 0 cm | `(0, 0.825711, -0.070523; 6.018305e-5)` | `(0, 1.282475, -0.309600; 1.326847e-4)` |
| NO2 | midpoint, 125 cm | `(-0.285523, 1.436422, -0.070523; 0)` | `(-0.739600, 2.134950, -0.309600; 2.264295e-317)` |
| NO2 | outlet, 250 cm | `(1, -1, 0; 0)` | `(-0.739600, 1.282475, 0; 0)` |
| NO3 | inlet, 0 cm | `(0, 0.823919, -0.071419; 1.778012e-7)` | `(0, 1.277100, -0.309600; 6.558124e-7)` |
| NO3 | midpoint, 125 cm | `(-0.286419, 1.432839, -0.071419; 0)` | `(-0.739600, 2.124200, -0.309600; 1.746945e-224)` |
| NO3 | outlet, 250 cm | `(1, -1, 0; 0)` | `(-0.739600, 1.277100, 0; 0)` |

The Fortran trace reports `theta=0.430000007` and `Disp=0.180000007` at all three locations. Retardation is 2 for NH4 and 1 for NO2/NO3. Its upstream weight is `wc=-0.224365354` at the midpoint and outlet and zero at the inlet. The measured zero- and first-order terms are:

| Species | `g0` inlet / midpoint / outlet | `g1` at all three locations |
|---|---:|---:|
| NH4 | `0 / 0 / 0` | `-0.00430000015` |
| NO2 | `0 / 0 / 0.00136615965` | `-0.0430000015` |
| NO3 | `0 / 0 / 3.29943077e-6` | `0` |

The row sums are similar at the midpoint, while the off-diagonal distribution and diagonal terms differ. At the outlet the Fortran row is a zero-gradient relation after coordinate reversal; the retained Rust row is different. These are measured row differences, not a diagnosis of which equation is correct.

The post-solve values also differ at the inlet:

| Species | Fortran at 0 cm | Rust at 0 cm | Fortran at 125 cm | Rust at 125 cm | Both at 250 cm |
|---|---:|---:|---:|---:|---:|
| NH4 | 0.317711523 | 0.246855198 | 0 | 2.106321e-314 | 0 |
| NO2 | 7.673095e-5 | 1.220116e-4 | 0 | 1.625065e-222 | 0 |
| NO3 | 2.349222e-7 | 6.815468e-7 | 0 | 2.565180e-221 | 0 |

The tiny Rust midpoint values are subnormals and are effectively zero at the displayed precision. The Fortran trace logged all three species at each of the three locations before and after its solve. This comparison does not pair internal endpoints or time steps by index.

## Disposition

The first-interval traces show that the implementations already have different assembled rows and inlet states at matched coordinates. The Fortran assembly includes `epsi=0.5` time weighting and upstream weighting in its finite-element rows; Rust uses its retained finite-volume face-flux rows. The outlet closures also differ. The traces therefore locate a divergence, but do not establish that the Rust translation violates its intended equation, or that the Fortran formulation is the required target for a code change. Keep disposition `no_fix_cause_unisolated`; do not start conditional correction 16.6.1 from parity evidence alone. The fixture remains a derived Example 3 case, not a recovered manual deck or an independent analytical oracle.

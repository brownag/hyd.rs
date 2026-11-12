# M6 vapor precision diagnosis (task 25.7.3)

Status: evidence record, 2026-11-12. Copied from the untracked diagnostic
report `.development/25.7.3/REPORT.md`; the staged builds, traces and
`analysis.json` remain under that untracked root and are not committed.
User decision 2026-11-12: the 18 isothermal `v_vapor`/`v_vap_term` rows that
fail criteria v2 against the pinned fixture are recorded as explained by source
precision and are **not claimed as matched**. Criteria v2 is unchanged.
This is not a source defect register entry: Rust does not reproduce the
single-precision behavior (contract: f64, no identical-arithmetic claim).

Verdict: **source-precision artifact.** At t=6/12/24 h, Rust (f64) matches the
real-8 build E2 to <=1e-9 in dT and <=3.4e-5 in v_vap_term (the print precision).
The pinned single-precision build differs from both by 0.9% (6 h), 12% (12 h)
and 250% (24 h).
**Hashes:**
- Pinned exe 824179ab...9f1c; patch 622d1ea6...00d7e; fixture 860f4642...cdf32;
  candidate a5eec470...67a17; SELECTOR 2bffc285...; PROFILE c27810c5...; LEVEL `.\n` eb4bd64f...
- Staged copy `e1-trace/src_reference` is byte-identical (`diff -r`) to the 25.2.2 staged source.
- E1 trace diff `e1-trace/trace.diff` sha256 cc52e145...440c2 (4 lines added after staged
  HYDRUS.FOR:491: one WRITE of iTemp,t,dt,i,TempN,TempO,ThVNew,ThVOld,vVNew,g0 + FORMAT, unit 91).
  Note: the first E1 build (exe 1ed9b6ff...) hit a runtime format-reversion error. The fix wrapped
  the FORMAT in an outer group, so it is still a format-only change.
- V1/E1 exe d455fd58...e7042, flags `-std=legacy -ffixed-line-length-none` (built by tools/build_fortran_reference.sh).
- V2/E2 exe 7d53c361...dd8bd, flags `-std=legacy -ffixed-line-length-none -fdefault-real-8 -fdefault-double-8`.
  It compiled and ran first time, so the V3/V4 fallbacks were not used (2 variants built).
**Commands:** `python3 tools/stage_fortran_reference.py --output .development/25.7.3/e1-trace`; `sh tools/build_fortran_reference.sh --build-dir .development/25.7.3/e1-trace`;
`gfortran <V2 flags> src_reference/*.FOR gnu_compatibility.FOR -o hydrus-e2`; `run_case.sh <exe> runs/{e0,e1,e2}`;
`check_ident.sh runs/{e0,e1}`; `python3 -I analyze.py` (-> analysis.json).
**Byte-identity:** E0 (pinned exe) and E1 NOD_INF.OUT and NOD_INF_V.OUT are IDENTICAL to the retained 25.2.2 run
(sha = fixture output_sha256) after masking the ` Date:` line (INPUT.FOR:145). E1 NOD_INF_V v_vap_term == fixture (asserted).
## Comparison (dT = T-25 C, top-first, pass-2 state at print time)
| t h | max dT E1 / E2 / Rust | relL2 dT Rust-E2 / Rust-E1 / E1-E2 | v_vap_term ratio Rust/E2 | Rust/E1 |
|---|---|---|---|---|
| 6  | 3.046e-3 / 3.0729e-3 / 3.0729e-3 | 8.5e-12 / 8.8e-3 / 8.7e-3 | 0.99999-1.00003 | 0.988-1.016 |
| 12 | 5.016e-4 / 5.6854e-4 / 5.6854e-4 | 8.5e-11 / 0.137 / 0.121 | 0.99998-1.00002 | 1.007-1.199 |
| 24 | 2.861e-5 / 1.9474e-5 / 1.9474e-5 | 9.3e-10 / 1.62 / 2.53 | 0.99996-1.00002 | -4.74-0.977 |
Ratios over the 14 nodes with |v| >= 1e-2*max; relL2 v_vap_term Rust-E2 7e-6/1.8e-5/1.1e-5.
- Decision rule (predeclared): Rust~E2 TRUE (<=2% dT, ratio in [0.97,1.03] at 6/12 h); Rust~E1 FALSE.
  E1-E2 gap / Rust-E2 gap at 12 h = 1.4e9 (>=5x).
- Step counts: E1 = E2 = 240 (dt 0.1 h); the dt sequences differ only by REAL rounding (max 2.1e-8).
  Rust uses constant dt = 0.1, so the schedules coincide. Step 10 (Rust per-step trace) was not needed.
- Mechanism: REAL TempN (`sngl(F)`, TEMPER.FOR:55) has an ulp of 1.9e-6 at 25 C. The disturbance
  is ~1600 ulps at 6 h, ~300 at 12 h and ~10 at 24 h, so the per-step rounding of the decaying
  latent disturbance distorts its decay. The source decays faster at 12 h (E1 is 12% below E2)
  and is noise-dominated at 24 h.
- E2 changes two NOD_INF.OUT K values by 1 in the 4th digit; all runs print 25.00 C.
No Rust discrepancy (latent coupling/ThV/pass timing). git status unchanged; no commits.

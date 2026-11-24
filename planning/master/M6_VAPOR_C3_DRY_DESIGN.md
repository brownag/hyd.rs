# M6 vapor C3: dry-regime closed pair design (G22.2-C3)

Status: DESIGN for the C3 gate. No reference run and no change under `reference/` or `criteria.json` before approval.

## Question
Does the thermal vapor term drive the dry-regime divergence in the closed flux/flux pair with vapor on at h = -10000 cm?

## Evidence already on record (untracked, `.development/`)
- Control: `vapor_dry_isothermal_on` (30.4.1, committed fixture). Isothermal 25 C, head -10000 cm, closed pair, vapor on. Converged to 240 h, 0 non-convergence messages, head held at -10000 cm.
- Attempt C (29.4.1, `candidates/C`, run `runs/C`): same head, same closed-pair flux settings, vapor on, bottom temperature 25 to 15 C. Top head -2.4e5 cm at 6 h, -6.1e5 cm at 24 h, NaN in every node from 72 h. Its RUN_INF.OUT flags every step converged (22153 steps, last Iter 2, dt 0.01 h at the end). The divergence is therefore a head blow-up with NaN, not a count of non-converged steps.
- The attempt C deck differs from the control only in the bottom temperature (linear 25 to 15 C profile) and the heat BC line `1 25 1 15` (SELECTOR line 45), plus a comment line. LEVEL_01.DIR is identical.

## Design
1. Run attempt C's deck through the reference harness with build and input manifests, under a new case id (name to be fixed at the gate). Inputs: `LEVEL_01.DIR`, `PROFILE.DAT`, `SELECTOR.IN` from `.development/29.4.1/candidates/C/`. Executable: the pinned reference build (`.development/29.3.3/reference-build/hydrus-reference`, hash `15c24e35…246edb`).
2. Output under `.development/30.7.3/` (scratch). Record: exit code, NaN onset time, head trajectory at the top and bottom nodes at 0, 6, 24, 72, 240 h, the count of non-converged steps from RUN_INF.OUT, and the time-step minimum.
3. Compare the isothermal control on the same settings. The only intended difference is the thermal gradient.
4. Write the divergence record into `planning/master/M6_VAPOR_COVERAGE.md` for C3.

## Hypothesis test
- Supported: the thermal run diverges (NaN or head blow-up) while the isothermal control at the same head converges.
- Not supported: the thermal run converges, or both diverge. Either result is recorded as is.

## Exit criteria (G22.2-C3)
- A provenance-backed divergence record (heads, NaN onset, non-convergence count) in `M6_VAPOR_COVERAGE.md`, with the harness manifest hashes.
- The thermal-term question answered as supported or not supported.

## Out of scope
- No deck change, no tolerance change, no new Fortran source change, no fixture written into `reference/`.
- No attempt to make the divergent case converge. That would be a new design.

## Gate
User approves the rerun of attempt C through the harness (step 1) and the case id.

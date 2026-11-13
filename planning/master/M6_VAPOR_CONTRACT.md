# M6 vapor source contract — first increment

Status: **Scope/source contract approved with orchestration 25, 2026-11-11.
Numerical criteria approved 2026-11-12 (v1, then post-failure v2). First
increment accepted with exception 2026-11-12:** see
[M6_VAPOR_ACCEPTANCE.md](M6_VAPOR_ACCEPTANCE.md). The approved sections below
are unchanged; this header and the evidence list are status annotations only.
Source audit: 2026-11-11.
Governing plan: [EXECUTION_PLAN_25.md](../EXECUTION_PLAN_25.md). This is a
capability-specific extension, not a rewrite of the historical heat-only contract.

Evidence status (2026-11-12): contract revisions `8e561a5` (v1) and `a4545e4`
(v2); criteria `reference/vapor_fortran/criteria.json` v1 `bde9c7f`, v2 `21101ca`;
parity harness `95ee096`; precision diagnosis `b4913c3`
([M6_VAPOR_PRECISION_DIAGNOSIS.md](M6_VAPOR_PRECISION_DIAGNOSIS.md)). The 18
isothermal `v_vapor`/`v_vap_term` rows that fail v2 are explained by source
single-precision `TempN` and are not claimed as matched. Vapor content evidence
is pinned-kernel only.

## Source map

| Routine/site | Pinned source | Contract |
|---|---|---|
| Effective switches and defaults | `INPUT.FOR:31,38,1583-1587,1706-1718` | `lVapor` is read; `iEnhanc=1` and `lExtrap=true` are initialized internally. No input reader overrides `iEnhanc`. |
| Initial material/vapor evaluation | `HYDRUS.FOR:224-229` | `SetMat` initializes `ThVOld` and vapor conductivities. |
| Initial velocities | `HYDRUS.FOR:380-387` | `Veloc` initializes liquid and vapor velocities; old/new vapor contents initially agree. |
| Coupling loop | `HYDRUS.FOR:399-403,419-432,473-476,490-499` | Two water→velocity→heat passes even with `lEnBal=false`. |
| `SetMat` | `WATFLOW.FOR:435-568`, especially `:470-476,509-565` | Liquid material evaluation uses the mixed head; vapor evaluation uses `hNew`. `VaporContent` changes hydraulic capacity, not thermal capacity. |
| `Reset` | `WATFLOW.FOR:177-350`, especially `:235-267` | Vapor head conductivity, thermal-gradient RHS, and vapor storage enter water rows. Gravity remains a liquid term. |
| `ConVapor` | `WATFLOW.FOR:849-926` | `ConVh`, `ConVT`, and diagnostic `ConLT`; enhancement is active for the selected executable. |
| `VaporContent` | `WATFLOW.FOR:930-962` | Equivalent liquid-water volume and hydraulic capacity adjustment. |
| Scalar `ConVh` | `WATFLOW.FOR:968-1000` | Separate isothermal helper, hardcodes 20 °C; not the temperature-dependent array routine. Audit callers before any reachability claim. |
| `Veloc` | `WATFLOW.FOR:717-775` | Liquid and vapor velocities are distinct; vapor endpoint velocities include half-cell vapor-storage changes. |
| `Temper`, `TempAdj` | `TEMPER.FOR:25-32,35-55,180-265` | Assemble heat, then add vapor terms, then solve. Flux-boundary vapor branches are deferred. |
| `xLatent` | `TEMPER.FOR:310-323` | Temperature-dependent liquid density times latent heat per mass. |
| Retry | `WATFLOW.FOR:128-149` | Water retry restores old temperature, reduces `dt`, resets `iTemp`. Restart the coupled attempt. |
| Final water-content correction | `WATFLOW.FOR:152-155` | After convergence, correct `ThNew` with the last effective hydraulic `Cap` and final head increment; do not refresh vapor properties after the final solve. |
| Acceptance | `HYDRUS.FOR:695`, `WATFLOW.FOR:1079-1127` | Copy old/new water, temperature, vapor content and velocities only at final acceptance; source head extrapolation is predictor state, not accepted state. |
| Vapor file | `HYDRUS.FOR:169-177`, `INPUT.FOR:1586` | `NOD_INF_V.OUT`, when print and vapor output are enabled. |
| Observation calculation | `OUTPUT.FOR:760-816,891-926` | Recomputed output fluxes are not identical to `Veloc` values. Nine exponential fields; content is absent. |

Audit fixed-form columns, including column-6 `!` continuations (ADR 0007).
Do not infer reachability from routine presence. The scalar `ConVh` helper is
audited and documented here but not added to the first-increment production API.

## Supported selection

All items must hold together:

- Water and heat enabled; `lVapor=true`; one domain; van Genuchten `iModel=0`.
- `iEnhanc=1`, matching the unchanged executable's hardcoded default.
- Default thermal conductivity `iCampbell=0`.
- Constant Dirichlet temperatures at both ends (`kTopT>0`, `kBotT>0`),
  `tPeriod=0`; fixed water pressure heads at both ends.
- Arithmetic conductivity averaging; finite increasing coordinates, at least
  three nodes; no root uptake, prescribed sink zero for this increment.
- No hysteresis, dual porosity, chemistry, density coupling, centrifugal terms,
  meteorological/energy boundary, or temperature-dependent liquid-water mode.
- Temperature, water content, porosity, conversions and material inputs must
  remain in the validated finite physical domain. In particular `T+273.15>0`,
  density positive, `0<=theta<=theta_s`, `theta_s>0`, and conversions positive.
  Numerical range bounds for the supported fixtures must be declared before runs.

Existing heat-only selections and APIs retain their numerical and failure
contracts. A vapor-enabled config cannot enter a heat-only driver or transport
entry point and silently omit vapor; those entries must reject before mutation.
The vapor-aware entry accepts only this supported selection. A closed selection
and typed errors must make the distinction mechanically testable.

## Rejected and deferred

Reject every other vapor combination with a typed reason and offending selector,
including enhancement-off, Campbell, heat flux/zero-gradient/sinusoidal boundaries,
non-head water boundaries, root uptake or nonzero sink, `lWDep`, `lEnBal`,
multi-domain, other hydraulic models, hysteresis, chemistry, density and
centrifugal options. Omitted primitive arrays are errors, never zero defaults.

Full vapor breadth is goal G22 and the [coverage matrix](M6_VAPOR_COVERAGE.md).
Energy balance remains G20. Temperature-dependent liquid water is separately
tracked as G23. No source fix or dependency addition is authorized.

## Coupling and transaction contract

1. Validate the entire selection, dimensions, boundary values, material mapping,
   units, times, policy and primitive state before any accepted-state mutation.
2. Initialize vapor content and liquid/vapor velocities from the declared initial
   head, liquid content and temperature; do not initialize vapor arrays to zero
   unless the calculated value is actually zero.
3. Retain one old-time snapshot throughout an attempted timestep. First water
   pass uses the old accepted temperature. Evaluate vapor material terms each
   nonlinear iteration, not once outside the water solve.
4. Compute liquid and vapor `Veloc` values; run heat using `TempO` from the old
   snapshot and the current temperature estimate for `TempAdj`'s new-level latent
   heat. Vapor heat adjustment occurs after both base heat assembly levels and
   before the tridiagonal solve.
   The water pass first applies `ThNew += Cap*(hNew-hTemp)*Rate` at convergence
   (`WATFLOW.FOR:152-155`; `Rate=1` in the selected equilibrium-water mode).
   `Cap` is the last assembled **vapor-adjusted hydraulic** capacity; `hTemp` is
   the pre-final-Gauss head. Retain vapor contents/conductivities at their last
   pre-final-solve evaluation rather than recomputing them at the solved head.
   Pass the corrected source-labeled liquid `ThNew` to heat/velocity/history.
5. Repeat water→velocity→heat once using the first heat result as the temperature
   estimate. Both passes cover the same old→new time interval, **not two steps**.
   The second water solve may use the first pass's head as an initial guess but
   must still use the original old-time storage and history. Preserve source
   `hTemp`/mixed-head and head-predictor semantics in the opt-in path.
6. If either water pass reduces `dt`, discard all trial results and restart at
   pass one with the old accepted temperature and the smaller interval. Use the
   explicit water retry policy; source retry is `dt/3`, so source-matched tests
   must not borrow heat tests' `DMul2` interpretation without an audit.
7. Commit water, heat, vapor histories, predictor, time and accounting exactly
   once, only after both passes succeed. Any preflight, water, heat, diagnostics
   or acceptance error leaves all accepted fields and reusable workspaces
   unchanged. A heat failure does not silently trigger an unbounded retry loop.

Use local candidate states (existing `WaterState`/`WaterWorkspace` are Clone)
and publish only on success; do not widen public rollback APIs unnecessarily.
Preserve the existing heat-only `HeatAfterWaterAccepted` behavior separately.

## Equations and variable ownership

Use source grouping, not an algebraically simplified variant. In the following
summary, SI conversions happen at the same logical sites as the source:

- `h_m=h/xConv`, `TK=T+273.15`, `ThetaA=theta_s-theta`.
- `DiffT=2.12e-5*(TK/273.15)^2`; `Tau=ThetaA^(7/3)/theta_s^2`
  for positive air content, otherwise zero; `Diff=Tau*ThetaA*DiffT`.
- `rho=(1-7.37e-6*(T-4)^2+3.79e-8*(T-4)^3)*1000`.
- `rho_vs=0.001*exp(31.3716-6014.79/TK-0.00792495*TK)/TK`;
  `Hr=exp(h_m*0.018015*9.81/(8.314*TK))`.
- `ConVh=Diff/rho*rho_vs*0.018015*9.81/(8.314*TK)*Hr*xConv/tConv`.
- `drovs=rho_vs(TK+1)-rho_vs(TK)`: **one-kelvin finite difference**, not an
  analytical derivative. `eta=9.5+3*theta/theta_s-
  8.5*exp(-((1+2.6/sqrt(0.02))*theta/theta_s)^4)` for enhancement on.
  `ConVT=Diff/rho*eta*Hr*drovs*xConv^2/tConv`.
- `ThetaV=rho_vs*Hr*(theta_s-theta)/rho`. Adjust **hydraulic** capacity as
  `C_eff=(1-rho_vs*Hr/rho)*C_liquid+
  ThetaV*0.018015*9.81/(8.314*TK)*xConv` (`WATFLOW.FOR:958`).
  Preserve the source multiplication by `xConv`; do not silently replace it by
  a continuum-derived conversion. Independent checks report any discrepancy.
- `ConLT` is calculated by `ConVapor` even with `lWDep=false`, and is observable,
  but it must not enter liquid flux or water rows unless `lWDep` is supported.
- Water rows add `ConVh` to head-gradient conductivity **after** computing the
  liquid gravity term. Add the `ConVT` divergence and subtract vapor-storage
  change (`WATFLOW.FOR:257-266`). Do not apply vapor conductivity to gravity.
- `xLatent=rho*(2.501e6-2369.2*T)`, scaled by `1/(xConv*tConv^2)` in heat.
  `Cv=1.8e6/(xConv*tConv^2)`. `g0=-Lat*(vVGrad+(ThVNew-ThVOld)/dt)`.
  `TempAdj` uses a factor of two in the centered interior velocity gradient
  (`TEMPER.FOR:202,211`); do not replace it with the ordinary derivative.
- `TempAdj` loops over old/new levels, recomputing `g0` with old/new latent heat
  and velocity gradients. Preserve distinct level sources and matrix additions.

Liquid water contents stay liquid in `WaterState`; total liquid+equivalent-vapor
storage and phase-specific fluxes belong to the new vapor report/accounting.
In the opt-in source path, “liquid” names the source's corrected `ThNew`, not a
promise that it equals a fresh retention evaluation at the final head. Record
this discretization distinction; do not add `ThetaV` to the existing liquid
snapshot or hide it by refreshing properties after convergence.
Never reinterpret `AcceptedWaterSnapshot.face_fluxes` as total vapor+liquid
fluxes for existing solute callers. Vapor+solute coupling is not supported here.

## Variables and precision

No implicit-double declaration exists in the audited vapor routines. Default
REAL is single; names I–N are integers unless explicitly declared otherwise.

| Variables | Source type / units | Rust contract |
|---|---|---|
| `hNew`, `x`, `dt`, `TempO`, `TempN` | REAL; L, L, time, °C | `f64`, as current water/heat APIs |
| `Theta`, `ths`, `ThVOld`, `ThVNew` | REAL; volume fraction, vapor in equivalent water volume | `f64`, phase semantics explicit |
| `Con`, `ConVh`, `vOld`, `vNew`, `vVOld`, `vVNew` | REAL; L/time | `f64`; liquid, vapor, total kept separate |
| `ConLT`, `ConVT` | REAL; L²/(time·K) | `f64` |
| Hydraulic `Cap` | REAL; 1/L; modified by `VaporContent` | `f64`, never aliased to thermal capacity |
| Thermal `Cap`, `Cond` | REAL; existing heat-unit basis | Existing `f64` heat contract |
| `Cv`, `Lat`, `g0`, `xLatent` | REAL (`Lat` explicitly REAL); source-converted energy terms | `f64`, formula and unit tests |
| `B,D,E,F` in heat, `P,R,S,Q` in caller | DOUBLE PRECISION; mixed-source coefficient units | `f64` |
| `t` | DOUBLE PRECISION; time | `f64` |
| `iEnhanc`, `iTemp`, `MatNum`, counts | INTEGER | validated enum/code, `usize` indexing |
| `lVapor`, `lWTDep`, `lEnBal` | LOGICAL | `bool` / closed validated selection |
| `xConv`, `tConv` | REAL; units per metre, source time conversion | positive finite `f64` |

Existing f64 water/heat arithmetic is retained; this is not a bitwise f32 port.
Record rounding boundaries in tests: REAL constants/intermediates in all vapor
formulas, `TempAdj` REAL expressions added into double coefficients, and
`TempN=sngl(F)` at `TEMPER.FOR:55`. Do not add a global precision switch, alter
the compatibility solute kernels, or claim identical arithmetic. Kernel-harness
outputs preserve source REAL values in a separately declared harness format.

## Observation contract

`NOD_INF.OUT`: head, liquid moisture, temperature; normal formats have head
`f11.3`, moisture `f6.4`, temperature `f8.2`; large-negative head has `e11.4`
(`OUTPUT.FOR:922-925`). Availability, source columns and actual print interval
must be recorded by the producer, not assumed from a universal decimal count.

`NOD_INF_V.OUT`: `Node`, depth (`f10.4`), then `9e13.5`:
`Con`, `ConLT`, `ConVh`, `ConVT`, `vLiquid`, `vVapor`, `vTotal`,
`vVapIso`, `vVapTerm`. Source outputs top-first and depth relative to the surface;
Rust mesh is ascending/bottom-first. Normalize via explicit node mapping.

`NodOut` recomputes phase fluxes using final printed-time temperature and stored
conductivities. It omits the vapor half-cell storage term used by `Veloc` at
both endpoints; its interior weighting uses same-side rather than opposite-side
spacing (`OUTPUT.FOR:808,811` versus `WATFLOW.FOR:760`). At the top liquid
endpoint it includes storage and sink terms, while the bottom output does not.
Keep a separately named observation view; never change solver velocities to
force output parity. Test nonuniform spacing so these mappings are detectable.

There is **no direct full-program vapor-content output**. A separate harness
calls the byte-preserved pinned `VaporContent` routine on independently specified
inputs, and records source/routine/build/input/output hashes. Label this
`pinned-fortran-kernel`, never `pinned-fortran-trajectory`. Do not fabricate
trajectory content by feeding reference outputs into the Rust candidate.

## Evidence and tolerance gate

Numerical thresholds were approved on 2026-11-12 (below). Before any Rust-vs-Fortran comparison,
the execution orchestrator must obtain user approval of a field-specific table:
units, absolute/relative parts, scale, zero handling, precision/print interval,
independent check error budgets, and fixture nonzero-effect discrimination floors.
Approval of this plan is not approval of numeric thresholds.

The future gate creates `reference/vapor_fortran/criteria.json`, canonical schema
`vapor-criteria-v1`, by the orchestrator only after user approval. It records
approval date/owner, contract identity/hash, per-field evidence class and units,
absolute/relative limits, scale and zero/print rules, and separate independent
check bounds and nonzero-effect floors. Values must be finite, nonnegative and
explicit; missing/unknown fields reject. Rust/Python consumers read the same file
and record its SHA-256. This Markdown summarizes and links the approved values;
it is not a second independently maintained numerical table. The file is not
created with guessed thresholds during planning.
The contract identity includes a committed `contract_revision`; its hash covers
that frozen approval-time file, not later mutable status/evidence annotations.
Record the revision first to avoid a circular hash with the criteria artifact.

Prepare that proposal using source precision, independent calculations and
reference-only repeatability, not candidate deviations. Preserve ordinary
reference formats and capacity (ADR 0012); no promoted format or modified full
executable. Record the approved table and date here and in `STATE_25.md` before
unblocking comparisons. A missing criterion blocks the corresponding test gate;
never silently skip it or widen a limit after seeing failures.

### Approved numeric criteria (user, 2026-11-12)

The user approved the following values on 2026-11-12, before any
candidate-vs-reference comparison. The canonical machine-readable copy is
[`reference/vapor_fortran/criteria.json`](../../reference/vapor_fortran/criteria.json)
(`vapor-criteria-v1`); this summary is not a second source. A comparison passes
when `|candidate - reference| <= absolute + relative * |reference|`, raised for
zero-rule fields to `fraction * max|reference|` over the same case/time/field.

| Class | Field | Absolute | Relative | Basis |
|---|---|---|---|---|
| Trajectory | liquid moisture | 1.05e-3 | 0 | deck `TolTh` 1e-3 plus f6.4 half-step |
| Trajectory | head (cm) | 0.5 | 0.02 | dry-soil capacity maps `TolTh` to large head error |
| Trajectory | temperature (C) | 0.05 | 0 | 10x f8.2 half-step |
| Trajectory | nine `NOD_INF_V` fields | 0 | 0.02 | zero floor 1e-3 x case/time max; e13.5 print |
| Kernel | vapor content | 0 | 1e-5 | single-precision REAL harness |
| Kernel | adjusted hydraulic capacity | 0 | 1e-6 | single-precision REAL harness |
| Kernel | capacity correction (input - adjusted) | 0 | 1e-2 | correction is 6e-6..4e-5 of capacity |

Independent checks: interior water and heat discrete balance residual per step
<= 1e-9 relative to the largest absolute row term, 1e-15 absolute floor, using
the solver's published last-iteration coefficients; fresh-coefficient closure is
reported only. Discrimination: in the dry thermal case, the vapor on/off
difference exceeds 2x the field tolerance for head, moisture and temperature at
one or more printed times after the initial one; every vapor-only field is
nonzero above its zero floor in every vapor-enabled case. Wet cases remain
non-discriminating regression evidence for head/moisture/temperature.

### Criteria revision v2 (user, 2026-11-12, post-failure)

**This revision was approved after seeing Wave 7 failures.** It is an explicit
owner override of the no-widening rule, recorded as such. Cause: in the wet
isothermal case, failing `v_vapor`/`v_vap_term` errors (1e-14..1.4e-13) lay
within the vapor thermal flux produced by a single float32 step in the
source's single-precision `TempN` (`TEMPER.FOR:55`), 2e-15..5.6e-13, which
exceeded the v1 per-time zero floor.

Change (only): for `v_vapor`, `v_vap_term` and `v_total`, the bound is raised to
at least `2 * ulp32(|T_ref,t,n|) * ConVT_max,t,n / dx_min,n`, where
`ConVT_max` is the largest reference `con_vt` at node `n` and its mesh
neighbours at the same time, `dx_min` is the smallest adjacent node spacing,
and `ulp32` is the float32 spacing. The multiple 2 bounds two adjacent
half-ulp roundings with a factor-two margin. It is fixed from source precision,
not from candidate errors. All other v1 values are unchanged. Wave 7 is rerun
under v2; the v1 verdicts are retained as evidence.

Required, separate evidence classes:

1. Independent hand-worked kernel values, gradient/storage zero limits, density
   and conversion checks; these must not use production outputs as expectations.
2. Pinned-kernel comparisons for content and hydraulic capacity at declared
   temperature/head/porosity points (including nonzero content).
3. Coupled full-executable trajectories for head, liquid water, temperature,
   conductivities and phase fluxes. Include isothermal head-gradient and thermal
   gradient cases plus a vapor-off control, with observable nonzero effects.
4. Water total-storage and vapor-aware heat accounting, including latent and
   sensible transport. Show independent interior closure; a Dirichlet reservoir
   inferred from storage is bookkeeping, not an independent balance test.
5. Exact discrete schedule/selector/node identity, two-pass ordering, acceptance
   counters, rollback at each failure stage, workspace reuse and retry tests.

The existing G21 fix (`1d4131a`) uses `Se^l` with default `l=0.5`. Re-run
`cargo test -p hydrus-core --test hydraulic_curves` as a prerequisite. Do not
re-fix or re-pin it. Vapor's evolving water path must audit `SetMat`'s mixed
head, table interpolation/fallback and extrapolation; the constant-head heat
fixtures' documented omissions cannot simply carry over to this new evidence.

## Not claimed

- Complete vapor coverage (G22), energy balance (G20), or `lWDep` (G23).
- Bitwise source arithmetic, whole-program parity, universal accuracy, physical
  experimental validation, or numeric criteria not approved and met.
- Full-executable vapor-content trajectory parity; kernel observations support
  only the kernel claim.
- Campbell fixes, source defect fixes, root uptake, chemistry, hysteresis,
  dual porosity or boundary modes outside this increment.
- Acceptance of proposed ADR 0021. It describes heat-only behavior; this contract
  adds a distinct vapor-aware selection without changing that historical scope.

## Heat-boundary criteria v3 (approved 2026-11-12, GATE N1)

Scope: the vapor heat-boundary pairs DF, DZ, FD, FF and FZ (orchestration 26).
Approved by the user as proposed. Per-field tolerances in `fields` and `checks`
are unchanged from v2. Only `discrimination` gains entries:

- For each pair: head and temperature must differ between the vapor-on and
  vapor-off fixtures at some print time after the initial time, at 2x the v2
  tolerance (`any_print_time_after_initial`).
- Liquid moisture discriminates at 2x for DF, DZ and FD. **FF and FZ moisture
  do not reach 2x** (observed 0.0018 against a threshold of 0.0021). This is a
  recorded exception, not a widened tolerance. Head and temperature carry
  discrimination for FF and FZ.
- Vapor fields (`con_vh`, `con_vt`, `v_vapor`, `v_vap_iso`, `v_vap_term`) must be
  nonzero above the case zero floor for each on case. Their global maxima sit at
  the initial-state node and are equal across pairs, so they do not discriminate
  pairs. Pair-level differences are judged on full fields in the 26.4 evidence,
  not on maxima.

Changing any value above requires a new user approval.


Acceptance status and per-form decisions: [M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md](M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md). FD is held open; accepted forms are DF, DZ, FF and FZ, with exceptions recorded there.


## Heat-boundary forms with vapor (orchestration 26)

- `kTopT == -1` (top flux) and `kBotT == -1` (bottom flux) are accepted with vapor. `kBotT == 0` (zero-gradient) is accepted and adds no vapor term (`TEMPER.FOR:220-232`). Pairs DF, DZ, FF, FZ and FD are accepted; FD and FF carry a disclosed vapor-on head drift.
- `kTopT < -1` is rejected: the storage term at `TEMPER.FOR:255` is omitted for these codes. Trigger: a reference case plus a source decision.
- `kTopT == 0` is invalid. Sinusoidal top is rejected.
- Decisions and exceptions: [M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md](M6_VAPOR_HEAT_BOUNDARY_ACCEPTANCE.md).

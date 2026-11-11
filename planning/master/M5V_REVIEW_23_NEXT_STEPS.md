# M5-V next-step decision memo

## Disposition update — 2026-11-11

The user accepted the M5-V result with limitations and unblocked M6 planning and
implementation. The diagnostic and campaign options below are historical
recommendations, not prerequisites for M6 or required follow-up. Preserve all
reported findings; revisit a numerical issue only if a real supported-use case
shows it could affect a decision. See the [M5-V/M6 decision](M5V_M6_HANDOFF_DECISION.md).

## Follow-up — 2026-11-11

The later Robin upstream-mapping investigation confirmed and fixed a separate
translation defect: the V2 `chemistry.upstream_weighting` selector was previously
omitted from the public `FortranCompatible` Rust request/config path. The saved
40 Robin failures and the numerical-conditioning assessment below describe
pre-fix runs; they have not been rerun with the corrected mapping. The fix does
not establish the cause of those failures or change their historical
classification. **A rerun of the affected Robin spatial-refinement campaign is
authorized and is the next step**, using the corrected mapping and the campaign's
existing approval, pin, bounds, and evidence gates. Keep the saved pre-fix
artifacts as historical evidence; do not overwrite or reclassify them. See
[`ROBIN_UPSTREAM_MAPPING_INVESTIGATION.md`](ROBIN_UPSTREAM_MAPPING_INVESTIGATION.md)
and the [verification request contract](SOLUTE_VERIFICATION_SPEC.md).

Review date: 2026-11-11. Reviewed HEAD: `11239935c3d4a275db4fe7f557fc987a29b34ea2`.
Scope: read-only evaluation of Orchestration 23; this memo is the only added artifact.
No solver or holdout was launched, gate signed, or threshold/bound changed.

## Verdict

**M5-V cannot close under ADR 0019. Recommend a bounded Robin diagnostic orchestration,
with an evidence-policy review before spending on another complete campaign.** The
12 achieved, 40 failed, and 12,888 inconclusive findings are reproducible from the
classification artifacts. Merely adding temporal centre levels will not remove the
dominant control/floor obstacles. Robin has a concrete numerical-conditioning suspect
in the f32 upstream-weight calculation, alongside an outlet discretization effect;
the present evidence does not distinguish their causal contributions. There is also
a defensible *proposal* for narrowly scoped diffusion concentration ceilings at the
97-node configuration: that configuration was already studied. This contradicts the
final report's stated mesh-based reason for rejecting every threshold, but does not
establish temporal order, independent configuration generalization, or full closure.
M6 requires a separate explicit policy decision if the user chooses to proceed with
this partial record; acceptance of D14 alone does not grant that permission.

## Evidence check

### Identity and independently reproduced facts

Read all nine mandatory-reading items, including the superseded evidence/amendment,
and traced the classifier, analytical evaluator, compatibility boundary/transport
kernels, public adapter and metrics records. `git rev-parse HEAD` matches the prompt.
`git status --short` initially showed only `?? planning/REVIEW_23_NEXT_STEPS_PROMPT.md`:
the supplied assertion of a clean tree is slightly stale. The prompt is preserved.

Raw-byte SHA-256 values independently calculated with Python `hashlib.sha256`:

| Artifact | SHA-256 |
|---|---|
| `reference/solute_verification/CASES_V2.json` | `420af7c0085f3ad5cf3335ca036ca5360d7abd370807451d8d479e653a215df7` |
| `.development/23-bounds/combined_bounds_23.json` | `3ac9fd8a45018d850da20fefc2877093f8fcf73910b4a7133a54ec8a039a6ee4` |
| `.development/23-bounds/review-combined.json` | `93db9cfceadb55eedd33b54771bc78d60d79bed553763d09e72fab8c3f2e15b1` |
| `reference/solute_verification/THRESHOLDS_V2.json` | `c15a4dc0ba80e92994fea1f618774e1428a13c8f95c70261d0f04ee4967ca8ab` |

All 11 current producer hashes match **each of the six recipe pin maps**. The recipe
bounds bindings match the bounds/review hashes above. Threshold permission is false
and its cases pin is `c236eb7cd42dee691a36787625865457df62c0bb4a72cb0388a786cbe1e1b43b`.
The final campaign ledger contains no `holdout-` launch. This is a scoped ledger check,
not proof that no undocumented run exists anywhere on the machine.

The following files are `.development/23-final/<batch>/classification.json`:

| Batch | Group | achieved / failed / inconclusive | File SHA-256 |
|---|---|---|---|
| `batch-0793968d793e45ddb498c7c6d884bd9c` | diffusion space | 12 / 0 / 178 | `8bca3db82007c6733c391fc7b44a8f6ea9f13c907b589fbe9d6a810c7ed4c68d` |
| `batch-293141b4667e42ed8c6686e79abf713c` | diffusion time | 0 / 0 / 190 | `b30014abfed8582e23825bdf58df5c76f55dbee0e659d96461a04100ce778659` |
| `batch-ad3ac50b1a8243b9836b99ffcd6e2f9f` | Robin space | 0 / 40 / 150 | `b1dd3b199461b325209a084ef9e32b12c0ee357ececdfdf5da8e5399b12df38c` |
| `batch-0bc52085e3c84ab0a1bf416c881ca950` | Robin time | 0 / 0 / 190 | `3e2ce7a6472d673e69f77dde79292449a48e721a8c4d933300ceea1772e8cb54` |
| `batch-e22e8c92d1564f81b357396d6768a9bf` | Example 3 space | 0 / 0 / 6090 | `2e2d8975d2961326ea124a732c70d7c7014db0eff228dc7aeac7b471a721b358` |
| `batch-652a978c087a4039bc388604df5fed59` | Example 3 time | 0 / 0 / 6090 | `7c399ab4a6eba4643e3d8ec5ceb79987b9e0be6a9cc5330f5b1affe5755d8c06` |

All six existing `*.verify.log` JSON documents report `evidence_verified`. A fresh
serial verify-only attempt was bounded at 120 seconds and timed out without captured
per-batch completion output. **Fresh full lifecycle verification is not claimed.**
These checks reproduce classifications and pin consistency, not every filesystem
receipt, the historical test totals, or the historical parity campaign.

### Reproduction commands

The substantive extraction used these commands/forms (no output files required):

```sh
git status --short
git rev-parse HEAD
```

```python
# Run with: PYTHONDONTWRITEBYTECODE=1 python3 - <<'PY' ... PY
import collections, hashlib, json, pathlib
root = pathlib.Path('.development/23-final')
for p in sorted(root.glob('batch-*/classification.json')):
    payload = json.loads(p.read_text())['payload']
    g = payload['analysis']['groups'][0]
    fs = g['findings']
    print(p, hashlib.sha256(p.read_bytes()).hexdigest())
    print(collections.Counter(f['status'] for f in fs))
    print(collections.Counter(r for f in fs for r in f['reasons']))
    print('raw control > finest error', sum(
        f['control'].get('absolute', 0) > f['levels'][-1]['error'] for f in fs))
    recipe = json.loads((p.parent / 'recipe.json').read_text())['payload']
    assert all(hashlib.sha256(pathlib.Path(k).read_bytes()).hexdigest() == v
               for k, v in recipe['producer_sha256'].items())
    for f in fs:
        if f['status'] == 'achieved':
            print(f['observable'], f['time'],
                  [x['error'] for x in f['levels']], f['rates'])
for p in root.glob('*.verify.log'):
    print(p.name, json.loads(p.read_text())['status'])
```

The attempted fresh verification invoked, through `subprocess.run(capture_output=True)`
in sorted batch order, this exact command form; the enclosing tool timed out at 120 s:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/run_solute_convergence.py \
  --cases reference/solute_verification/CASES_V2.json \
  --output-root .development/23-final --verify-only \
  --batch .development/23-final/<batch>
```

### Claims not reproduced as written

1. **“Control difference exceeds the finest error for 6,076” is incorrect.** There
   are 6,076 Example 3 space `control_contamination` flags, but only **409** raw
   control differences exceed the finest raw error. The classifier compares the
   *upper uncertainty bound* against the finest lower error **and the last resolved
   decrease when positive** (`solute_convergence_analysis.py:549–560`). Counting flags
   as physical contamination overstates the diagnosis. Likewise, none of the raw
   diffusion-space control differences exceeds the finest raw error, despite 173 flags.
2. The assertion that 129-node achieved findings cannot inform the 97-node level
   omits the middle **97-node study row at the same dt**. See the table below.
3. “The two time groups” in `M5V_FINAL_23.md:47` should say three. All 6,470 time
   findings have `fewer_than_three_levels`.
4. Historical claims of no holdout anywhere, complete test-suite success and full
   fresh verification are not independently established by this review.

## Scientific assessment

### Robin: a failed refinement, with a numerical suspect rather than a proven cause

For S1 `concentration.linf` at t=.25, errors at 129/193/257 nodes are
`3.4059952679421635e-4`, `2.0195912304665775e-4`, `3.2456587734963627e-4`;
orders are `1.2889939299`, `-1.6491233103`. Raw profiles against the cached independent
reference put the maximum at the public outlet in all three runs. Signed endpoint
errors (inlet, outlet) are respectively:

| Nodes | Inlet | Outlet | Interior maximum absolute error |
|---|---:|---:|---:|
| 129 | 6.20377e-5 | 3.40600e-4 | 3.28081e-4 |
| 193 | 1.16218e-4 | 2.01959e-4 | 1.96386e-4 |
| 257 | 1.74214e-4 | 3.24566e-4 | 3.21428e-4 |
| 257, dt halved | 2.10894e-5 | 1.57196e-4 | 1.54058e-4 |

`boundary.instantaneous.1` has exactly the same rates and errors scaled by q=.43:
its signed errors are `-1.464577965e-4`, `-8.684242291e-5`, `-1.395633273e-4`.
Thus the outflow findings are correlated observations of the concentration error,
not 40 independent defects. Interior and inventory failures prevent dismissing the
whole result as an endpoint metric artifact.

Trace:

- `CASES_V2.json` defines L=2, q=theta=.43, D=1, R=1.5, decay=.2, epsi=.5,
  upstream weighting on, artificial dispersion off, steady-plus-mode initial data.
- `tools/solute_analytical_reference.py:355–404` independently evaluates the
  Robin/Neumann steady solution plus a decaying mode, including the frozen-root
  uncertainty. It is not a Fortran trajectory reference.
- `crates/hydrus-core/src/solute.rs:990–1047` reverses positive public flow: the
  public outlet becomes native node 1 (`kBotCh=0`), inlet becomes node N (`kTopCh=-1`),
  and velocity changes sign. This matters when identifying which boundary branch runs.
- `solute_fortran_compat/boundary.rs::assemble_lower` imposes `-C1+C2=0` at code 0;
  `assemble_upper` supplies the natural inflow row. The outlet is a one-sided
  zero-gradient discretization, a credible source of first-order boundary error.
  This is not the zero-flow SD-005 branch simply relabelled as Robin.
- `solute_fortran_compat/transport.rs:213–234` computes the upstream weight as
  `1/tanh(pe2)-1/pe2`, with tanh itself formed by subtracting f32 exponentials.
  Both subtractions are ill-conditioned for small Peclet number. At these meshes
  the nominal native pe2 values are approximately -.0078125, -.0052083333, -.00390625.
  The coefficient error can vary non-monotonically with mesh spacing.

A read-only arithmetic probe using libc `expf` and explicit f32 rounding gave:

| Nodes | Nominal f32 weight | Stable f64 expression | Difference |
|---|---:|---:|---:|
| 129 | -.00244140625 | -.00260415607 | +.00016274982 |
| 193 | -.00114440918 | -.00173610802 | +.00059169884 |
| 257 | -.001953125 | -.00130208201 | -.00065104299 |

Exact probe command body:

```python
import ctypes, math
m = ctypes.CDLL('libm.so.6')
m.expf.argtypes = [ctypes.c_float]
m.expf.restype = ctypes.c_float
f = lambda x: ctypes.c_float(x).value
for n in [129, 193, 257]:
    x = f(-1 / (n - 1))
    a, b = m.expf(x), m.expf(-x)
    t = f(f(a - b) / f(a + b))
    w = f(f(1 / t) - f(1 / x))
    exact = 1 / math.tanh(x) - 1 / x
    print(n, x, w, exact, w - exact)
```

This proves conditioning risk at nominal inputs, **not the actual per-element weights
or causation of the 40 failures**. Native f32 coordinates, compiler and math library
affect the actual result, especially on the 193-node mesh. The large dt-control
difference also warrants testing accumulated assembly roundoff and temporal error.
At t=.25 the concentration control upper bound is `2.09868617e-4`, below the finest
lower error `3.07339838e-4`, so classification is bounded; it is nevertheless larger
than the raw fine-pair increase `1.22606754e-4`. A bounded control is not proof that
time/roundoff effects are negligible in explaining that increase.

**Disposition:** retain `failed: nonmonotonicity`, with cause unresolved and a
small-Peclet arithmetic defect candidate. Do not call this an expected benign regime,
a threshold miss, or a confirmed translation defect. If diagnostics attribute the
effect to the pinned source, follow the source-defect register and parity policy;
do not silently stabilize the compatibility backend.

Smallest useful diagnostic: capture actual per-element pe2/wc and signed endpoint,
interior and inventory errors on the existing 129/193/257 configurations, then compare
baseline versus a diagnostic-only stable-weight variant at the same dt. Add the
129/193 meshes at halved dt (257 control already exists) to distinguish temporal
effects. Compare one-step matrix residuals with independent high-precision assembly
before contemplating a full solver variant. No Example 3 launch is needed to answer
this question. Existing artifacts can be inspected first without invalidating pins.

### Diffusion: a narrow ceiling proposal is possible, but has limited value

**Yes, for a fixed-configuration concentration-error confirmation, not full M5-V or
temporal convergence.** The row
`convergence-diffusion_cosine-space_97-dt2e-7-rfc-baseline` is already a middle study
level. No interpolation from 129 nodes is necessary. All rows below are S1; `linf`
and `endpoint_linf` each count separately, yielding 12 achieved fields.

| Observable | t | Error 65 / 97 / 129 nodes | Orders coarse / fine | Proposed ceiling |
|---|---:|---|---|---:|
| linf, endpoint_linf | .25 | .001771356 / .001145269 / .000848199 | 1.07557 / 1.04379 | .0018 |
| linf, endpoint_linf | .5 | .002339148 / .001525544 / .001134419 | 1.05419 / 1.02972 | .0024 |
| linf, endpoint_linf | 1 | .002983757 / .001961656 / .001462408 | 1.03435 / 1.02094 | .0030 |
| interior_linf | .25 | .001488234 / .001019423 / .000777408 | .93314 / .94210 | .0016 |
| interior_linf | .5 | .002072963 / .001407227 / .001067862 | .95534 / .95926 | .0022 |
| interior_linf | 1 | .002748466 / .001857071 / .001403576 | .96690 / .97322 | .0029 |
| l2 | .25 | .000355080 / .000227729 / .000168260 | 1.09550 / 1.05202 | .00036 |
| l2 | .5 | .000556634 / .000361903 / .000268553 | 1.06182 / 1.03700 | .00056 |
| l2 | 1 | .000851306 / .000561240 / .000417428 | 1.02752 / 1.02904 | .00087 |

Proposed *new, user-reviewed* margin rule: take 1.5 times the 97-node error interval's
upper endpoint, round **up** to two significant figures once, then freeze. This
includes the signed-budget uncertainty rather than merely multiplying raw error.
Units are concentration (`C_ref`); no inventory or relative ceiling is proposed.
This is a proposal in prose only, not an approved threshold or a changed tolerance.

The reserved run would check whether a separately authenticated execution at 97 nodes,
dt=2^-7 stays within these predeclared total-error ceilings. The achieved designation
and other-axis control belong to the three-level space group and its 129-node control,
not a separately demonstrated temporal isolation at 97 nodes. A deterministic rerun
of a configuration whose study output was inspected is weak evidence of generalization.
The holdout row's retained f64 label also must not be interpreted as an f64 solver.

`SOLUTE_VERIFICATION_SPEC.md:406–410` reserves experiments by role but denies that
coincident study outputs constitute the holdout. ADR 0019 requires unseen holdout
outputs. The user should explicitly resolve the scientific meaning of this duplicate
configuration before Gate C. **Do not consume the holdout now:** the narrow proposal
is worth reviewing, but running it has lower information value than Robin diagnosis
and cannot close the other claims. If independent resolution generalization is required,
these data do not satisfy that requirement; a new predeclared holdout design needs approval.

### Example 3: spatial error exists, but the contamination count is not its measure

The space control is 1001 nodes at .0125 against 1001 at .04. It is a valid *time*
sensitivity control, not a finer spatial reference. For NH4 linf at t=50, spatial errors
fall `.013274990 -> .007326842 -> .005414895`; measured control difference is
`.002651989`, upper bound `.003084349`. That is less than the finest error but greater
than the final spatial decrease `.001911947`. At t=100 the last decrease is only
`.000951246`, versus control difference `.002095282`. Both axes matter.

Of 6,090 findings, 5,091 carry an arithmetic-floor flag, 2,046 a zero-error flag,
and 2,412 a missing-quadrature reason (overlapping sets). Even zero control difference
can be flagged when its uncertainty interval overlaps zero finest error. Therefore
“6,076 contaminated” cannot be interpreted as 6,076 resolved physical errors.
NH4 does show resolved spatial sensitivity; NO2 and many integral/tail channels need
a better-resolved error budget. Their current classification remains inconclusive.

A different control or finer analytical reference is **not the only remedy**, and
a finer reference alone does not address an arithmetic floor. First separate signed
physical differences, reference uncertainty, arithmetic allowance, quadrature and
last-decrease tests using existing data. Then pilot a space family at smaller fixed dt
with a still-finer dt control, or a temporal family on a demonstrably adequate mesh.
Choosing a control closer to .04 merely to shrink its difference would game the
diagnostic; it needs an independent truncation-error rationale. A revised arithmetic
derivation must be justified by precision/conditioning evidence and re-signed, never
reduced to obtain passes. Integral claims require actual quadrature bounds.

Cost examples, using species × nodes × steps: at dt=.0125, 501/751/1001 nodes cost
108.144M work units total; 1001 at .00625 adds 96.096M, making **204.240M** for that
four-row pilot. This is an illustration for approval, not an authorized design.
The present 1001/.0125 control reportedly took about 84 s; a roughly doubled-step
control may approach its 180 s override, and proportional runtime is not guaranteed.
Going beyond 1001 nodes crosses the current casebook limit and needs a new capacity
decision/checker change. Cold analytical reference work remains potentially hours
(planning records estimate roughly three hours), independent of solver work units.

## Options and costs

Current six-batch recipes total **133,321,176 work units / 21 launches**. A change to
CASES or any of the 11 producer tools invalidates current pins on **all six batches**:
budget a complete fresh campaign, new cache identities as applicable, frozen tools,
and Gate B re-signing. Old evidence remains historical, not deleted. Work units are
not wall-clock predictions. Engineering-day ranges below are planning estimates.

| Option | Scientific gain | Cost/time | Risk, reversibility, gates |
|---|---|---|---|
| Three explicit time-centre rows | Repairs level count transparently; may expose failures, not necessarily achievements | +11,302,120 units, total 144,623,296 / 24 launches; roughly 1–2 engineering days plus hours of reference/rerun work | Lowest interpretation risk; additive rows reversible before launch. User must supersede D14; freeze and new Gate B mandatory |
| Tested dual-axis centre reuse | Same sampled scientific information, avoids duplicate solver runs if receipts can be reused safely | At least the full existing campaign; 2–4 engineering days for cross-axis evidence semantics/tests, reference hours | Higher risk: duplicate weighting, receipt identity and control pairing. Reversible code change, but invalidates pins. New Gate B and explicit D14 revision |
| Defer temporal (G9) | No new temporal evidence; preserves honest scope | Documentation decision, no run cost | Reversible later; all temporal claims remain unmet. Does not unblock M6 |
| Robin root-cause diagnostic | Distinguishes conditioning, boundary discretization and temporal effects before expensive work | Approximately 1–3 engineering days; tiny solver cases, no Example 3 reference generation initially | Highest near-term information gain; isolated diagnostic artifacts reversible. User gate before code/launches; source-parity decision before production fixes |
| Example 3 redesign | Potential resolved axis-specific norms/integrals | Multi-day design; illustrative 204.240M-unit pilot plus full rerun after pin changes and cold references | Could remain floor-limited; capacity/precision changes need approval. Freeze/Gate B after design, Gate C only on supported claims |
| Accept partial and plan M6 | No numerical gain; advances project scope | Documentation/decision only | Explicit amendment of milestone/ADR policy, not scientific closure. M5-V failures stay open; partial acceptance alone cannot satisfy current ADR 0019 |
| Narrow diffusion Gate C | Confirms predeclared fixed-configuration error on 12 fields | Small holdout computational cost, but irreversible consumption after approval | Weak independent information because same configuration already studied; user must settle independence and f64-label semantics. Gate C required; other fields stay unmet |

#### Exact temporal implementation scopes (proposals only)

For explicit rows add to `reference/solute_verification/CASES_V2.json`:

- `convergence-diffusion_cosine-time_97-dt2e-7-rfc-baseline` (97, 2^-7);
- `convergence-scalar_robin-time_193-dt2e-7-rfc-baseline` (193, 2^-7);
- `convergence-example3-time_751-dt0.04-rfc-baseline` (751, .04).

Use study role, time axis, f32 and identical physical/sampling configuration to each
space centre. Update `tools/check_solute_verification_spec.py` and
`tools/test_check_solute_verification_spec.py` row expectations as required. Prepare new
campaign activations and a new bounds source/review; map each centre's existing bounds
only after checking identical observables, numerical requests and provenance, then
bind the new launch IDs/cases/producer hashes. Do not edit the existing signed files.
Any new campaign-root registration must cover the maps in
`run_solute_verification_reference.py`, `solute_convergence_evidence.py`,
`solute_reference_cache.py`, the Rust verification harness and CLI/cache tests;
if the harness changes, review the bridge `POLICY_HASHES` re-pin too.

For dual-axis reuse change `tools/solute_convergence_analysis.py` grouping/level
selection and `tools/test_solute_convergence.py`; audit
`solute_convergence_evidence.py` and `run_solute_convergence.py` for cross-batch
receipt loading. A time batch currently does not contain its space-centre receipt;
changing the group key alone is insufficient. Explicit membership must match case,
backend, precision, fixed mesh, dt, times and physical configuration; reject duplicates,
holdouts, unmatched controls, mixed fixed axes and stale receipts. Keep original row
identities. Six fresh batches are still required, with approved centre availability.

Neither option changes the existing finest time levels or their controls. Currently
control assessments are bounded for only **56/190 diffusion**, **1/190 Robin**, and
**68/6090 Example 3** time findings. Adding a middle point cannot lower those measured
control differences and can tighten the last-decrease criterion. These are ceilings
on possible supported findings with unchanged endpoints/bounds, not predictions of
achieved counts; floors and plateau/nonmonotonicity remain. Deferral changes only the
goal/state/closure documentation and requires no producer/case edit.

## Recommendation and proposed bounded plan

Rank: **(1) Robin diagnosis plus evidence-policy clarification; (2) explicit temporal
rows only after a successful diagnostic/design gate; (3) explicit accept-partial/M6
policy decision if further validation is not worth its cost; (4) Example 3 redesign;
(5) dual-axis classifier reuse.** Defer consuming the diffusion holdout until its
limited scientific purpose is accepted. Do not combine all workstreams into another
unconditional campaign.

Proposed DAG: `R0 -> {R1, R2} -> USER-D1 -> R3 -> USER-D2 -> [T1 -> B -> T2 -> T3]`.
Braces denote independent tasks, not authorization to delegate or implement now.

| Wave / task | File scope and work | Checks / exit criterion |
|---|---|---|
| 0 / R0 | New diagnostic plan/state under `planning/`; inventory existing receipts read-only | Record hashes, claimed-field map and cost; recover full verify-only completion with adequate time. Stop on any mismatch |
| 1 / R1 | New Robin diagnostic report; read `solute.rs`, compat `boundary.rs`, `transport.rs`, `assembly.rs`, pinned `SOLUTE.FOR`, saved profiles/references | Trace native mapping and exact pe2/wc inputs; tabulate signed errors and correlated metrics. No producer edits needed for artifact analysis |
| 1 / R2 | Proposed corrections to `M5V_FINAL_23.md`, `GOALS.md`, closure/strategy and ADR 0019 consequences, only after approval | Separate flags from raw differences; recognize centre evidence; agree whether narrow diffusion confirmation is meaningful. No threshold edit |
| gate / USER-D1 | User approves diagnostic experiment and time budget | Specify non-holdout configurations, diagnostic-only variants, output location, and no production numerical change |
| 2 / R3 | New isolated Rust diagnostic test/harness, e.g. `crates/hydrus-core/tests/solute_robin_diagnostic.rs`; any private kernel instrumentation explicitly scoped to `transport.rs` tests | Baseline versus stable-weight diagnostic, actual f32 mesh/coefficients, one-step residuals; 129/193/257 plus smaller-dt contrast. Preserve baseline assertions. Exit with supported attribution or a clearly bounded unresolved finding |
| gate / USER-D2 | User reviews cause and chooses register-only, production-fix proposal, temporal follow-up or accept-partial | Source-shared defect: register/pinning test and parity-policy review before changes. Translation defect: separate fix plan. No identified cause: stop/escalate, do not launch expensive campaign |
| conditional / T1 | Explicit centre rows and checker/tests, campaign registration only if approved | Test actual time-group membership (three levels), correct controls, no duplicate/holdout consumption; all changes before freeze. Record all 11 hashes and cases SHA |
| gate / B | New bounds derivation and review artifacts | User signs new Gate B after centre-bound equivalence and all diagnostic-driven design changes are resolved |
| conditional / T2 | New campaign root; six fresh batches, no changes to old `.development/` | Execute/verify all groups. Stop on first lifecycle/pin error. No assertion/timeout/bound relaxation without user decision |
| conditional / T3 | New final report, state, goals and closure records | Exact achieved/failed/inconclusive inventory; justified field-specific proposals only. User Gate C is separate and holdouts remain forbidden until explicitly authorized |

**First task:** approve and prepare R0/R1, the bounded Robin coefficient/error audit.
Its exit is actual-input evidence sufficient to choose the R3 discriminator, not a
promise of a fix or of convergence. R3 should stop after the named comparisons;
additional precision modes, meshes, source changes or Example 3 runs require escalation.
If production code changes, run focused numerical regressions plus the workspace
tests/fmt/clippy and 44-fixture parity before any new campaign freeze. Planning-only
corrections need link/content checks, not numerical reruns.

## Additional missing or stale items

- **Small-Peclet cancellation is not tracked in the blocker/final report.** Its
  magnitude and propagation need investigation; simple operation-count arithmetic
  allowances may not describe conditioning. This is not grounds to alter signed
  bounds now, but future derivations should explain it.
- **Control semantics are misstated**, as quantified above. This changes the
  interpretation of Example 3 and the expected gain of a temporal restart.
- **Holdout independence is ambiguous at coincident configurations.** All three
  centres duplicate the reserved mesh/dt combinations. Role-based non-consumption
  remains true; statistically/numerically unseen configurations are not preserved.
- **Documentation has contradictory current-looking claims.** ADR 0019 consequences
  still say no bounds source/all inconclusive and no registered diffusion defect;
  later text records the contrary. Both phase/strategy files retain a top banner
  saying FV/BE is the default pending parity, despite the later successful gate.
  Old holdout text says file existence lifts the guard, superseded by status/SHA
  checks. Label these historical statements explicitly when approved.
- **Goal-policy mismatch:** GOALS suggests M6 becomes available when the user accepts
  the M5-V status; ADR 0019 requires passing holdouts for closure. An explicit scope
  exception/amendment is needed, not just a board update.
- **No new invalid Gate B was demonstrated.** Current pins agree; historical stale
  reviews are not the current signed review. Fresh comprehensive receipt validation
  remains unfinished in this review, rather than being silently inferred from logs.

## Residual uncertainty

The numerical cause of Robin's failed refinement remains unproven; the coefficient
probe used nominal inputs, not instrumented kernel values. Boundary truncation,
time discretization and accumulated f32 matrix error can coexist. No new precision
or counterfactual solver experiment was performed. Example 3 cost estimates depend
on cache reuse, reference admission, machine load and timeout behavior; work units
are the reliable comparison, engineering durations are estimates. The narrow
diffusion proposal depends on user acceptance of fixed-configuration confirmation,
the existing conditional uncertainty budget and explicit threshold-policy review.
None of these uncertainties justifies a pass, relaxed assertion, consumed holdout,
or automatic M6 authorization.

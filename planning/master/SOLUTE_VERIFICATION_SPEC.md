# Solute verification contract v1

Task 13.1.1, 2026-09-26. This contract specifies mathematical verification of
constant-water linear-equilibrium transport and ordered reactions. It does
not assign solver tolerances, establish physical validation, or close M5-V.
The orchestrator must review this derivation before accepting the contract.
[CASES.json](../../reference/solute_verification/CASES.json) freezes the finite
matrix, parameter values, provenance hashes and reference-evaluator budgets.
“Executable” means an available solver mapping plus a feasible reference
method; actual execution remains gated by adapter validation and the
per-observable reference admission below.

## Sources and independent derivation

The pinned repository files are primary sources for the selected implementation:
`SOLUTE.FOR:Coeff` (storage and parent production, especially lines 298–317,
436–439, 658–690), `Disper` (1025–1050), `MatSet` (boundary rows),
`INPUT.FOR:NodInf` (253, input node reversal), `ChemIn` (1289 onward),
`OUTPUT.FOR:TLInf/SubReg/NodOut`, and `HYDRUS.FOR:48–54,525–568`.
[The previous outlet derivation](SOLUTE_OUTLET_CONTRACT.md) fixes the distinction
between a continuum zero-gradient outlet and the differing endpoint rows.
Rust's `solute.rs:SoluteTransportInput`, `SoluteProcessInput`,
`update_step_balances`, `assemble_and_solve_species_with_processes`, and
`apply_endpoint_boundary_rows` confirm the supported API and storage law.
The [derived fixture](../../reference/fortran_reference/solute/example3/CASE.json)
is the parameter authority; it is not a recovered manual deck.

The following primary research was consulted on 2026-09-26:

- Van Genuchten et al. (2013), *Exact analytical solutions for contaminant
  transport in rivers. 1. The equilibrium advection-dispersion equation*,
  J. Hydrol. Hydromech. 61, 146–160, DOI 10.2478/johh-2013-0020,
  [USDA author copy](https://www.ars.usda.gov/arsuserfiles/20361500/pdf_pubs/P2405.pdf).
  Equations 1, 5–7 establish flux and linear retardation; equations 21–25
  distinguish finite zero-gradient outlet and third-type inlet from the
  semi-infinite problem. Its resident concentration is the quantity compared
  here. No semi-infinite solution is substituted for the finite problem.
- [NASA spatial convergence tutorial](https://www.grc.nasa.gov/www/wind/valid/tutorial/spatconv.html):
  multiple grids and an asymptotic-range check are necessary to interpret
  observed order or Richardson extrapolation. Three grids alone are not an
  accuracy certificate. Its methodology does not prescribe this case's order.
- Salari and Knupp (2000), *Code Verification by the Method of Manufactured
  Solutions*, SAND2000-1444, [OSTI record](https://www.osti.gov/biblio/759450),
  DOI 10.2172/759450. The indexed primary record was available; full-text fetch
  failed. Used only as a methodology lead, not as authority for any formula
  below. Arbitrary manufactured forcing is explicitly blocked in this study.
- Kuznetsov (2013), [On the convergence of the Gaver-Stehfest algorithm](https://arxiv.org/abs/1305.0715),
  and Kuznetsov and Miles (2020), [On the rate of convergence of the Gaver-Stehfest algorithm](https://arxiv.org/abs/2005.05813).
  Their results support convergence under stated regularity and exponential
  convergence for locally analytic targets. They do not provide a ready-made
  finite-order numerical error bound for this particular chain.

All chain coupling, finite-domain transfer formulas, single-mode benchmark
construction and error-accounting formulas below are independently derived
here by substitution, integration and the resolvent identity. They do not
use either solver's discrete stepping matrix or fit solver output.

## Equation, dimensions and signs

Let s increase from inlet 0 to outlet L. Native Fortran coordinates are x=-s;
PROFILE lists top first, and INPUT reverses the node indices. Canonical arrays
always increase in s, with q>=0. Rust may use mesh s directly with positive
q at every face. All cases use cm, hour, and a fixed normalized concentration
unit C_ref. Thus inventory per column area has units C_ref cm; endpoint
solute flux has units C_ref cm/hour. Kd has cm3/g and rho has g/cm3.

For species j define a_j=theta+rho Kd_j, R_j=a_j/theta and stored mass density
m_j=a_j C_j. Constant-water equilibrium sorption is S_j=Kd_j C_j. With unit
chain yield and lambda_j>=0,

    J_j = q C_j - theta D dC_j/ds
    a_j dC_j/dt = theta D d²C_j/ds² - q dC_j/ds
                   - lambda_j a_j C_j + lambda_(j-1) a_(j-1) C_(j-1).

The parent term is absent for j=1. Terminal lambda_J removes mass from the
configured chain. Dividing by a_j gives concentration production coefficient
lambda_(j-1)*a_(j-1)/a_j, not simply lambda_(j-1). Dividing by theta gives
R_j C_j,t = D C_j,ss - v C_j,s - lambda_j R_j C_j
+ lambda_(j-1) R_(j-1) C_(j-1), with v=q/theta.
Example 3 therefore has daughter production coefficient 0.01/hour, even
though parent decay is 0.005/hour. This storage ratio is essential.

The common boundary laws are J_j(0,t)=q c_in,j (positive inward) and
C_j,s(L,t)=0, hence J_j(L,t)=q C_j(L,t). Canonical signed inward endpoint
fluxes are [q c_in,j, -q C_j(L,t)]. For q=0 these are Neumann zero-flux
conditions, including the no-transport reaction case. All initial data are
nonnegative. The Example 3 step inlet is incompatible with the zero initial
profile exactly at (s,t)=(0,0); retain that initial trace and evaluate its
right-time solution for t>0. Do not impose initial smoothness or include the
corner in a formal second-order argument.

For each species define independently:

    M_d(t)=theta integral_0^L C ds; M_s(t)=rho Kd integral_0^L C ds;
    M=M_d+M_s; P(t)=integral_0^t lambda_parent M_parent(u) du;
    Q(t)=integral_0^t lambda_j M_j(u) du;
    B_in=q c_in,j t; B_out=-q integral_0^t C_j(L,u) du;
    residual=M(t)-M(0)-B_in-B_out-P+Q.

Integrate P and Q separately; do not obtain either by closing this residual.
For a closed chain with lambda_J=0, sum_j M_j is conserved when boundaries
are closed. With a terminal loss, total derivative equals boundary inflow
minus terminal lambda_J M_J. Printed inactive Sorb is not equilibrium Kd*C.

## Closed reaction benchmarks

Set q=D=0 and spatially uniform concentrations. The stored masses obey m'=Bm,
where B_jj=-lambda_j and B_(j+1,j)=lambda_j. For parent-only initial m10,
with distinct rates,

    m_j(t)=m10 product_(k<j) lambda_k *
           sum_(k<=j) exp(-lambda_k t)/product_(l<=j,l!=k)(lambda_l-lambda_k).

Convert back using C_j=m_j/a_j. The common-rate limit is
m_j=m10 exp(-lambda t)(lambda t)^(j-1)/(j-1)! when the first j rates coincide.
A zero terminal rate retains daughter mass; all-zero rates retain the initial
state exactly. Initial daughters are handled by superposition.

Implement the primary evaluator by uniformization, avoiding cancellation:
nu=max(lambda); P=I+B/nu; m=e^-nu*t sum_k (nu*t)^k/k! P^k m0.
P is nonnegative and substochastic. The omitted mass 1-norm is bounded by
||m0||_1 times the Poisson tail. For K+2>nu*t, bound the tail by its first
omitted term divided by 1-nu*t/(K+2). For nu=0 return m0 directly.
Time-integrated m uses term weights
[1-e^-nu*t sum_(l=0)^k (nu*t)^l/l!]/nu, or high-precision equivalent;
its omitted norm is bounded by t times the same mass bound. Use this integral
for reaction observables, not a conservation remainder. The finite cases
have nu*t<=1.6; K=64 is ample, but report the computed tail and roundoff.
Compare with distinct-rate formulas and equal/zero limits as independent
checks. `reaction_equal` has equal first two rates and stable terminal;
`reaction_distinct` additionally tests terminal loss and unequal storage.

## Finite diffusion benchmark

`diffusion_cosine` has L=2, theta=.43, rho=.8, Kd=.5375, R=2, D=.2,
zero flow/loss, and

    C(s,t)=1+.25*cos(pi*s/L)*exp[-D*pi²*t/(R*L²)].

Both derivatives vanish at endpoints. Inventory is exactly aL, boundary and
reaction contributions are zero. This is a finite-domain Neumann solution,
with a spatially nonconstant field and no series truncation. Derivative and
PDE substitution, initial data, and analytic cosine integral independently
verify it. Initial equilibrium storage follows Kd*C at every node.

## Scalar finite-domain inlet/outlet benchmark

`scalar_robin` uses L=2, v=1, D=1, R=1.5, lambda=.2, c_in=1.
Let alpha=v/(2D) and k in (0,pi/L) solve k*tan(kL/2)=alpha. This root is
unique because the left side is strictly increasing on that interval.
Define phi(s)=exp(alpha*(s-L/2))*cos(k*(s-L/2)) and
beta=(D*k²+v²/(4D))/R+lambda. Both homogeneous Robin inlet and Neumann
outlet hold for phi. Let H(z,s) be the steady transfer below. Then

    C(s,t)=H(R*lambda,s)+.1*phi(s)*exp(-beta*t).

The initial condition is exactly this expression at t=0. It is positive,
compatible with both boundary laws and nontrivial in time. There is no
manufactured volume source. The frozen root and bracket are IC parameters;
Wave 3 recomputes the root independently and propagates the bracket width.
The mode integral is obtained from the primitive
exp(alpha*y)*(alpha*cos(k*y)+k*sin(k*y))/(alpha²+k²), y=s-L/2.
Thus M and endpoint flux have a steady plus exponential form; their time
integrals replace exp(-beta*t) by (1-exp(-beta*t))/beta. Reaction loss is
lambda times independently integrated M. Direct PDE/BC residuals and
independent high-precision Taylor exp/sin/cos evaluations check the result.
No infinite-series or inverse-Laplace error is involved for this benchmark.

## Actual finite Example 3 chain and a stable Laplace evaluation

The finite problem is L=250, theta=.43, rho=.8, v=1, D=.18,
R=(2,1,1), lambda=(.005,.1,0), initially zero and c_in=(1,0,0).
All coefficients and the finite outlet are retained. No semi-infinite
approximation or unproved small-outlet assumption is used.

For real Laplace variable p>0, set z_j=R_j*(p+lambda_j),
r_+=(v+sqrt(v²+4Dz))/(2D), r_-=-2z/(v+sqrt(v²+4Dz)), and

    A(z)=v/[D*r_+*(1-(r_-/r_+)²*exp((r_--r_+)*L))],
    H(z,s)=A(z)*[exp(r_-*s)
                 -(r_-/r_+)*exp(r_-*L+r_+*(s-L))].

Every exponential argument is nonpositive for the selected p,z,s. This avoids
the exp(vL/(2D)) ~ 3.9e301 amplification of a naive transformed eigenfunction
expansion. Direct differentiation gives (z-D*d²/ds²+v*d/ds)H=0,
vH(0)-DH_s(0)=v and H_s(L)=0. The scalar spatial integral is

    I(z)=A(z)*[(exp(r_-L)-1)/r_-
          -(r_-/r_+)*exp(r_-L)*(1-exp(-r_+L))/r_+].

At z=0 use H=1, I=L; apparent r_-=0 singularities are removable.
Use expm1-equivalent Decimal series when subtracting near-one exponentials.

Define reversed divided differences F[z1]=F(z1),
F[z1,...,zj]=(F[z1,...,z(j-1)]-F[z2,...,zj])/(zj-z1).
The exact concentration transforms are

    C_hat_j(s,p)=product_(k<j)(lambda_k R_k)/p * H[z1,...,zj](s).

Proof: (z_j-A_operator)(H(z_i)-H(z_j))=(z_j-z_i)H(z_i),
while each difference has homogeneous boundary data. Inductively the
resolvent recurrence produces the chain forcing. This is valid for unequal
retardation because every z_j retains its own R_j. Inverse transformation
preserves the specified IC and both finite-domain boundaries.

Coincident z values are real possibilities: Example 3 has z1=z2 at p=.09.
Use confluent limits F[z,...,z]=(-1)^(j-1) F^(j-1)(z)/(j-1)!,
computed by differentiated formulas/Decimal Taylor jets (up to order two for
three species), not zero division or arbitrary perturbation of rates.
Near-coincident nodes use guarded differences or a Taylor expansion with a
reported remainder. Sorting nodes is permitted because divided differences
are symmetric; replacing unequal R by a common R is not.

Inventory transforms follow by replacing H with I and multiplying by a_j
(or theta/rho Kd for separate phases). Outlet-flux transform is q*C_hat_j(L,p).
Cumulative outlet, production and loss transforms are respectively
q*C_hat_j(L,p)/p, lambda_parent*M_hat_parent(p)/p and
lambda_j*M_hat_j(p)/p. Inlet is known independently as q*c_in*t.
These transforms evaluate every mass term independently of closure. Their
sum identity and differentiated PDE/BC checks are validation checks only.

### Numerical inversion, bounded feasibility and uncertainty

Use the standard-library Decimal Gaver-Stehfest approximation for even N:

    f_N(t)=ln(2)/t * sum_(k=1)^N V_k * f_hat(k*ln(2)/t)
    V_k=(-1)^(k+N/2) sum_(j=ceil(k/2))^min(k,N/2)
        j^(N/2)*(2j)! / [(N/2-j)! j! (j-1)! (k-j)! (2j-k)!].

Generate integer factorial factors exactly, cache weights and transform roots
for each time/p/order, and use Decimal exp/sqrt. Cache uniform-grid exponent
recurrences; independently check first/middle/last and front nodes by direct
exp evaluation. No discretized transport operator is used. Prefix integrated
observables must use their transforms, not repeated numerical time stepping.

A bounded scratch feasibility study used orders 32,64,96,128 at 180 digits,
with order128 repeated at 220 digits, then 192,256,384 at 400 digits. It used
five original observables and three difficult higher-order observables only;
no solver output or solver execution was involved. Recipes/results are retained
under `.development/13.1.1/laplace_feasibility.py`, `laplace_feasibility.json`
and `laplace_high_order.json`. Representative independently calculated values:

| Observable, t=200 | N=192 | N=256 | N=384 |
|---|---:|---:|---:|
| C_NO3(200) | .0313396767971 | .0313394748805 | .0313394745983 |
| C_NO3(250) | -1.45790e-7 | -2.69785e-10 | 2.50076e-11 |
| integral C_NO3 ds | 67.4483036568 | 67.4483034040 | 67.4483034034 |

At order128, precision changes were below 3e-97 for the tested quantities,
but order changes were as large as 8.1e-4. More digits alone do not control
inversion error. The higher-order last differences are about 3e-10 in
concentration and 6e-10 in its spatial integral. This demonstrates a feasible
route under the standard-library constraint, not a certified full trajectory.
The order384 small positive outlet is not asserted relatively accurate.

For Wave 3, evaluate all required nodes/times and independent observables at
N=192,256,384 with 400 digits. One bounded escalation to N=512,650 digits is
available; no fifth full order is authorized. Re-evaluate the final order at
650 digits for a declared diagnostic subset including extrema, endpoints,
fronts, near-zero values and every integrated observable. Use an empirical
uncertainty at least 10*the latest order difference plus measured precision,
recurrence and input-root errors. Require decreasing differences across the
last three orders; otherwise classify inconclusive even if one difference
happens to be small. Agreement is an empirical estimate, not a rigorous
remainder bound. Reject an observable when its uncertainty exceeds the frozen
budget; do not continue to a solver-accuracy claim or invent another order.

Budget scales are C_ref=1, M_scale=L*max_j(a_j)*C_ref, flux_scale=
max(q,theta*D/L)*C_ref, cumulative_scale=M_scale+q*t_end*C_ref.
CASES budgets give absolute concentration error and scaled inventory,
instantaneous-flux and cumulative-term errors. They are reference admission
requirements only, not assigned solver acceptance thresholds. Full-case
success requires separate coverage of concentrations, inventories and
boundary/reaction integrals, including equal/near-equal-rate and zero limits.
Negative values whose uncertainty intervals include zero remain interval
estimates; never clip them to manufacture relative accuracy or positivity.

Independent cross-checks: reaction uniformization and limiting identities;
scalar mode and diffusion formulas; spatial derivative/Robin residuals of H;
finite-domain integrated transform identity; zero rate and zero transfer;
direct vs recurrence evaluation; tighter precision; and time-domain PDE/BC
residuals away from the discontinuity. If these do not bound an observable
convincingly, retain a precise unmet reference-accuracy status. Feasible
alternatives are an independently implemented complex-contour inversion with
explicit quadrature/tail control, or high-precision finite-domain spectral
sums. The latter require roughly 302 cancellation guard digits before the
accuracy digits at this Peclet number. Neither alternative is automatically
approved additional work or a reason to install a numerical package.

Cost: naive evaluation is N*nodes*times*species expensive Decimal exponentials.
Cache coefficients and use uniform-grid recurrences; integral observables
require only O(N*times*species) operations. At 1001 nodes, three analysis times
and three species, four inversion orders imply about 11 million scalar term
accumulations, with exponent calls reduced to O(N*times*species). Computing
all 201 diagnostic print times is materially larger and is not required for
analytic profile norms: evaluate truth at all nodes at analysis times, exact
mass/flux formulas at integration diagnostic times, and endpoint truth on
its requested comparison schedule. Scientific coverage is explicit; cost
must not silently remove nodes, species or declared analysis times.

## Executable source mapping

Use the existing immutable portability patch/shim, baseline flags
`-std=legacy -ffixed-line-length-none`, or the identified sensitivity additions
`-fdefault-real-8 -fdefault-double-8`. No capacity/format variant is permitted.
The pinned main program has NumNPD=1001, NObsD=100 and NPD=1000; because input
also writes TPrint(MPL+1), cap explicit noninitial print times at 999.
Species count is limited to three for this contract (some local input arrays
are smaller than the global NSD). Coordinates are x=-s for every case.

Water is homogeneous saturated theta=.43 with fixed equal zero pressure
heads. Flowing cases use CosAlpha=1 and Ks=q, giving native flux -Ks;
closed reaction/diffusion use CosAlpha=0, Ks=.43 and equal heads, giving q=0.
This horizontal saturated construction is necessary: setting molecular
parameters to zero in the vertical fixture does not eliminate gravity flow.
Check every run's actual theta/heads/flux history before accepting its mapping.
Use source-derived water values only to validate the common target; solute
boundary output must never become a forcing input.

Disper gives D=alpha*abs(q)/theta+Dw*TauW with gas terms disabled.
With lTort=false, TauW=1 exactly. Closed diffusion uses alpha=0,Dw=.2;
reaction uses alpha=Dw=0. Flow cases use Dw=0 and alpha=D*theta/q.
All gas, nonlinear, kinetic, zero-order, root and moisture-dependent terms
are off; Fraction=1, immobile water=0, beta=1, Nu=Henry=0. Kd maps to source
Ks species coefficient. For each nonterminal species set GamL1=GamS1=lambda
and ordinary GamL=GamS=0. For terminal loss set ordinary GamL=GamS=lambda
and transfer fields zero (equivalent loss, no invented fourth species).
Example 3 terminal rate is zero. The initially-equilibrated profile flag is
true and supplies dissolved concentrations; inactive nonequilibrium Sorb=0.
Top/bottom chemical codes -1/-1 select inlet flux/zero-gradient outflow;
at zero flow they realize homogeneous Neumann conditions. Top concentration
is the vector in CASES; zero-flow values do not inject mass.

Rust uses total inward flux q*c_in at s=0, AdvectiveOutflow at s=L,
constant accepted theta and face flux q, effective dispersion D, linear Kd,
and the declared chain. Use the core's default linear process configuration
with sufficiently small solver pivot; record actual configuration. Do not
interpret the core's cumulative balance or concentration-boundary residual
as independent evidence. The source lower equality row and Rust's endpoint
half-cell equation have different truncation and accounting behavior.

Bridge rows preserve the original SELECTOR, PROFILE and LEVEL_01 bytes,
including dt=.1, dtMin=.01, dtMax=.5, multipliers1.2/.5, original print times
50/100/200 and no observation nodes. Their matrix dt=.1 denotes the retained
realized trajectory, not permission to replace its adaptive controls. Rust
replays the declared constant-water dt=.1 sequence after this identity is
checked. A bridge cannot claim complete independent step endpoint histories;
its comparisons are the retained profile/source-parity fields only.
All other rows fix dtMin=dtMax=dt and multipliers=1; reject unexpected accepted
steps rather than silently adapting the convergence study. Request initial
and final times are 0 and time_end; inlet stays active through the final step.

## Norms, refinement and budgets

Uniform nodes have width dx/2 at each endpoint and dx inside. Report L1/L2
normalized by L, L-infinity, endpoint L-infinity and interior L-infinity,
separately for every species/time. Evaluate analytic truth on exactly the
requested mesh. Compare quadrature inventory to both exact continuum mass
and analytic truth integrated with identical mesh weights; their difference
isolates mesh quadrature. Never mix species in a denominator. Temporal RMS
uses trapezoidal time weights on the declared analysis schedule. No
interpolation is needed for analytical truth; any source-grid coordinate
rounding or paired-grid interpolation gets an explicit uncertainty term.

For a relative metric use its named dimensional scale and preserve that
scale in output. If scale=0, return relative=null and branch=zero. If scale is
no greater than the sum of reference/rounding uncertainty in the same units,
branch=near_zero, report absolute error and relative=null. Otherwise report
relative=absolute/scale. No universal near-zero epsilon is assigned here.
For error against zero truth, use absolute error. A reference interval
straddling zero cannot resolve relative error.

Rust time order is one (backward Euler) for smooth solutions; upwind advection
is at most first order in space. Pure centered diffusion may show second
order in smooth interior fields, but endpoint conditions and global inventory
can reduce it. Fortran epsi=.5 does not guarantee global second order for
sequential coupling or the row-replaced boundary; establish it by evidence.
The smooth scalar/diffusion cases separate initial compatibility from the
Example 3 inlet corner. Rates are computed against independent truth only
where positive resolved errors decrease and the other-axis control is small.
Report plateaus, sign changes, nonmonotonicity and rounding floors explicitly.

CASES enumerates 77 launches: 14 benchmark/bridge, 57 convergence/sensitivity,
6 holdout. These are a maximum, not automatic authority before producer gates.
Each attempted process consumes its row, including failure/timeout; no retry
or implicit extra build mode is allowed. Each executable has a 60-second
ceiling. Estimate work as species*nodes*steps, and bytes as
2,000,000 + (print_count+1)*nodes*(160+36*species) + steps*(700+300*species).
The total storage cap is twice these estimates: 2,699,462,776 bytes. Estimates
are conservative planning values, not measured performance claims. Builds
are reused only when their complete identities match; compile time is not a
solver launch. Stop before exceeding storage; never delete accepted evidence
to make room or silently truncate record inventories.

Spatial Example 3 levels are 251/501/1001 at dt=.025, with finest-grid
control dt=.0125. Temporal levels use 1001 nodes at dt=.2/.1/.05; capacity
prevents a finer temporal-control grid, so the declared 501-node control
measures spatial sensitivity from below. It does not prove that 1001 nodes
are asymptotically adequate. If temporal error cannot be isolated, record
inconclusive within the user's capacity decision. Component controls provide
finer spatial grids where possible. Print sampling is distinct from solver
steps: use all requested profiles for independent reaction integration,
coarsen this same history to estimate quadrature sensitivity, and report a
floor if its uncertainty dominates. No extra print times or runs are implied.

Holdout mesh/time combinations and analysis times are reserved by role in
CASES. Do not use holdout outputs to select thresholds. New holdout runs
occur only after threshold-policy review in Wave 6; unassigned thresholds
remain unmet. Existing study outputs at a coincident holdout time do not
constitute the reserved independent mesh/time experiment.

## Interoperable files and request contract

Canonical JSON uses UTF-8, finite numbers only, unique keys, lowercase SHA-256
hex, ordered species lists and increasing s/time arrays. JSON null means a
specifically unavailable quantity and must have an availability reason; it
never means zero. Unknown schema versions fail. Hash raw bytes of files.
Case hashes are SHA-256 of `json.dumps(case, sort_keys=True,separators=(',',':'),
ensure_ascii=True,allow_nan=False).encode('ascii')`; casebook hash is raw
CASES bytes. A matrix row is identified by its unique launch id. All consumers
must validate CASES using `check_solute_verification_spec.validate` (and
`load_cases` rejects duplicate keys).

The dependency-free Rust input channel is a flat ASCII request file, one
`key=value` line, terminating newline, no whitespace around delimiters.
Unknown, duplicate or missing keys fail. Scalar numbers use round-trip
17-significant-digit decimal formatting; numeric vectors are comma-separated.
No quoting, nested JSON or locale-dependent formatting is used. Required keys:

| Key | Value |
|---|---|
| schema | `flat-ascii-v1` |
| run_id, launch_id, case_id | safe `[A-Za-z0-9_.-]+` identifiers |
| cases_sha256, case_sha256 | hashes defined above |
| nodes, species_count | decimal integers |
| length, theta, rho, q, dispersion, dt, time_end, sampling_interval | case/row scalars |
| analysis_times | exact requested analysis times in increasing order |
| species | ordered safe species names, comma-separated |
| kd, rates, inlet_concentration | one value per species |
| initial_0 through initial_(species_count-1) | N dissolved values, increasing s |

The request contains explicit initial arrays sampled from the specified
closed-form IC; both adapters consume the same values. No solver output is
used to generate them. Grid s_i=i*L/(N-1), print times k*sampling_interval
including final, and accepted times k*dt are derived by integer multiplication,
not accumulated additions. Validate integer ratios and all analysis events;
never shorten a last step silently. Derive equilibrium storage from Kd*C.
The canonical request excludes output paths so provenance is location-independent.
The launcher writes it atomically into a new run directory with exclusive
creation and records its byte hash.

Rust environment variables:
`HYDRUS_SOLUTE_VERIFICATION_REQUEST` (absolute request path),
`HYDRUS_SOLUTE_VERIFICATION_REQUEST_SHA256` (expected byte hash),
`HYDRUS_SOLUTE_VERIFICATION_OUTPUT_ROOT` (absolute allowed root), and
`HYDRUS_SOLUTE_VERIFICATION_OUT` (nonexistent `observations.json` path).
All must be present together. The harness verifies the request hash (the
installed `sha256sum` utility may be called directly without a shell; no new
package is needed), validates lengths/finite values/physical mappings, and
refuses an existing output or symlink/path escape. Allowed roots are task
scratch `.development/13.2.2`, `13.3.1`, `13.4.1`, `13.6.1` and descendants,
including corresponding isolated worktree roots. Without a request the test
runs only deterministic small adapter tests and writes no study artifact.
The harness creates neither the casebook nor launch ledger; the launcher
records the Cargo/test command, compiler/executable/source identities and exit.

Fortran CLI freezes these switches: `--cases PATH --output-root PATH`,
`--dry-run` (default), `--execute`, `--verify-only`, and optional repeatable
`--launch-id ID`. A task's caller selects only its phase rows; dry-run emits
planned requests/manifests but launches no solver. `--verify-only` reads and
validates existing data and never rebuilds or executes. Outputs remain below
the task-specific root, unique run IDs refuse overwrite, and the ledger is
append-only. Wave 3 creates the common requests and invokes the Rust harness;
there is no requirement for Rust to parse CASES JSON or add a Cargo dependency.

## Observation schema `solute-observation-v1`

Exact top-level keys are `schema_version`, `run_id`, `launch_id`, `case_id`,
`cases_sha256`, `case_sha256`, `request_sha256`, `solver`, `units`, `species`,
`coordinates`, `profiles`, `steps`, `endpoint_history`, `solver_reported`,
`availability`, `warnings`. `solver` is `rust` or `fortran`; `units` is the
casebook unit object; `species` is the ordered name list. Profiles include
initial t=0 and every requested print time. No missing species/time may be
silently dropped. Values cannot be replaced with analytic truth.

| Record | Required exact structure |
|---|---|
| coordinates | N canonical s values |
| profiles[] | `{time, concentration, theta, q_faces}` |
| concentration | S arrays each containing N dissolved values |
| theta, q_faces | N water contents; N+1 canonical face fluxes |
| steps[] | `{index,time,dt,iterations,peclet,courant}`; index starts1; no gaps |
| endpoint_history[] | `{time, concentration}`; concentration is S pairs `[C(0),C(L)]` |
| solver_reported | `{inventory,boundary_cumulative,reaction_cumulative,balance}` |
| each solver_reported field | list of `{time,values}` where values has S numbers, or null |
| availability | `{endpoint_history,solver_reported,water}`; descriptive nonempty strings |
| warnings | array of verbatim captured warning strings |

`iterations` is a nonnegative integer or null with a reason in availability;
Peclet/Courant must be finite nonnegative diagnostics. Profiles' q_faces may
be source-reconstructed from validated constant water; availability.water
must identify sampled fields, reconstruction formula and source, rather than
labeling them all directly printed. Rust emits caller-supplied water and its
source. Canonical observations contain state/diagnostics; the metrics module
computes equilibrium storage, independent integrals and normalization into
a separate result. Do not write these into `solver_reported`.

Non-bridge Fortran decks select both endpoint observation nodes so
`OBS_NODE.OUT` provides each accepted step's dissolved endpoint values;
capacity remains two out of 100. Preserve raw columns and convert coordinates
and ordering. Endpoint history includes t=0 from the initial profile.
If that output cannot supply the required endpoints exactly, fail the
non-bridge mapping instead of substituting blended `cvBot/q`. Bridge rows
may have endpoint_history=null with availability reason `legacy_bridge_no_obs`;
they are ineligible for independent continuous endpoint-integral claims.

Raw Fortran NOD_INF layout has `11+2*S` columns (three species gives17),
concentrations/Sorb are e12.4, coordinate f10.4 and theta f6.4. Solute records
use the source's verified 17-field per-species layout and e13.5 flux fields;
RUN_INF carries every accepted step. Read actual source format declarations
when selecting exact record widths; reject malformed headers/columns,
duplicate node/time records, incomplete terminal records and stale outputs.
Rounding interval is half one unit in the last printed place, determined
from the actual token exponent and format. Coordinate and water rounding
also propagate. A default e12.4 concentration token such as 0.4300E+00 has four significant digits;
promoted arithmetic does not improve that format.

## Manifest and lifecycle schema `solute-manifest-v1`

Exact top-level keys: `schema_version`, `run_id`, `launch_id`, `case_id`,
`cases_sha256`, `case_sha256`, `request_sha256`, `solver`, `precision`,
`status`, `command`, `cwd`, `started_utc`, `ended_utc`, `exit_code`,
`timeout_seconds`, `provenance`, `artifacts`, `record_inventory`, `warnings`,
`mapping_validation`. Status is `planned`, `started`, `failed`, `timeout`,
`raw_complete` or `complete`; success requires exit0 and validated complete
records, never merely a success-looking substring. The manifest is finalized
once; attempted run IDs cannot be reused.

`provenance` exact fields: `source_sha256` (relative source-path -> hash,
including archive authority and patch), `compiler` (version string), `flags`
(string array), `platform` (string), `executable_sha256`, `shim_sha256`
(null for Rust), `adapter_sha256` (path -> hash), `input_sha256` (path -> hash).
The source snapshot and retained archive authority must match the casebook's
protected sources and the pinned SOURCE_MANIFEST. Rust includes relevant
core/harness/Cargo.lock hashes; Fortran includes all staged source, shim,
patch and generated deck hashes. No arbitrary unreviewed source is accepted.

`artifacts` maps safe relative names to `{path,sha256,bytes}`. Paths must stay
within the run directory after resolution, with no escaping symlinks.
Include raw stdout/stderr, all required output files, request.txt, and
observations.json when present. Legacy literal-backslash input paths remain
input artifacts and are quoted as literal strings, never shell commands.
`record_inventory` exact fields: `nodes`, `species`, `profile_times`,
`step_count`, `endpoint_times`, `terminal_records` (file -> exact matched
terminal record), `field_formats` (field -> source format string).
`mapping_validation` exact fields: `status` (`passed`/`failed`/`pending`),
`theta_range`, `q_range`, `dispersion_range` (each two numbers), and `evidence`
(nonempty strings identifying source reconstruction and verified records).
Do not mark mapping passed merely because intended input values match.

Ledger JSONL records have exact fields `{event,run_id,launch_id,utc,detail}`;
events are `start`, `exit`, `timeout`, `validation_failed`, `complete`.
Write start before execution, consume the row then, and retain even failed
launch attempts. A timeout or nonzero exit cannot become a completion.
Completed manifests/artifacts are immutable. Fortran may finalize a
`raw_complete` manifest before normalization. Metrics then creates a separate
`normalization.json` with `{schema_version,raw_manifest_sha256,
observations_sha256,normalizer_sha256,status}` and new observations.json;
its version is `solute-normalization-v1`. It must not rewrite raw manifests.
Rust launcher wraps its emitted observations in a complete manifest. Thus
Wave 2 tasks have disjoint files while sharing the same observation schema.

The metrics module's public entry points are
`normalize_fortran(run_dir, cases_path) -> observation_dict`,
`load_observations(path, manifest_path, cases_path) -> observation_dict`, and
`compute_metrics(observations, reference) -> result_dict`. They validate
hashes/mapping/shape before computing. `reference` uses the same coordinates and species, concentration profiles at
initial plus analysis times, and independent continuum integrals/uncertainty
at every diagnostic profile time. The exact wrapper is specified below;
no observation JSON keys change.
A metrics result must keep raw, derived and solver-reported namespaces separate.
Schema changes return to the orchestrator for a contract amendment.

## Acceptance and honest gaps

Run the checker self-tests, validate CASES, and check whitespace. The checker
is a contract validator, not an analytical evaluator or scientific gate.
Wave 3 implements and verifies the formulas and uncertainty estimates; no
new dependency is assumed. A reference-budget failure leaves its observable
unmet and blocks dependent accuracy claims, even when both solvers execute.
Unsupported manufactured forcing is explicitly blocked. Capacity, output
rounding, insufficient quadrature resolution or failure to isolate dt error
are outcomes to quantify within current limits. Component success does not
replace the full transient finite-chain trajectory required for M5-V.

## Wave 2 interface clarification — approved 2026-09-26

`compute_metrics` accepts observations returned by either loader, using a dict
subclass whose private attributes retain the validated case, launch, manifest,
and raw rounding intervals. JSON observation keys stay frozen. Plain dicts and
objects whose contents changed after loading are rejected as unvalidated.

The reference object has exactly these keys:
`schema_version`, `coordinates`, `species`, `profiles`, `continuum`, `uncertainty`.
The version is `solute-metrics-reference-v1`; coordinates and ordered species
match observations exactly. No implicit interpolation or dropped species.

- `profiles`: list of `{time, concentration}`; concentration is S arrays of N
  finite numbers. Coverage is initial time plus the launch's analysis
  times, retaining all N nodes at those times.
- `continuum`: list of `{time, dissolved, equilibrium_sorbed, total,
  endpoint_flux, boundary_cumulative, production_cumulative, loss_cumulative}`.
  Every observation profile time, including zero, is required. The inventory,
  production and loss fields each contain S finite numbers. `endpoint_flux`
  and `boundary_cumulative` each contain S pairs `[inlet, outlet]`, with both
  signs inward. Production/loss are separate independently evaluated positive
  integrals; no closure-derived term is accepted as independent evidence.
- `uncertainty`: exactly `{profiles, continuum}`. Each list duplicates the
  corresponding reference list's times, keys and dimensions, replacing every
  value other than `time` by its finite nonnegative absolute uncertainty.
  These include root/input, truncation, roundoff and evaluation uncertainty.

A reference value whose uncertainty includes zero cannot supply a resolved
relative denominator. Result namespaces are `raw`, `derived`, `solver_reported`;
derived comparisons retain each named dimensional denominator and uncertainty.
No solver acceptance threshold is assigned.


## Shared artifact and format conventions

Raw artifact logical keys use `NOD_INF.OUT`, `RUN_INF.OUT`, `T_LEVEL.OUT`,
`BALANCE.OUT`, `OBS_NODE.OUT`, `PROFILE.OUT`, `I_CHECK.OUT`, and `solute1.out`
through the configured species count. Artifact path values are authoritative,
including literal `solute-input\\NAME` paths. `request.txt`, `stdout.txt`,
`stderr.txt` have matching names. Input keys `SELECTOR.IN`, `PROFILE.DAT`,
`LEVEL_01.DIR` point to `inputs/NAME`, with literal runtime aliases separate.
Resolve contained path values rather than depending solely on logical keys.

Source-checked `field_formats`: concentration/sorbed `e12.4`; coordinate
`f10.4`; theta `f6.4`; head `f11.3`; nodal_flux `e12.4`; solute_flux `e13.5`;
profile_time `f14.4`; run_time `e15.7`; run_dt `e13.5`; endpoint_time `f14.4`;
endpoint_concentration `e12.4`. OBS_NODE additionally has per-node head `f12.2`,
theta `f8.4`, then flux `e12.4` if lFlux or temperature `f9.3` otherwise.
The selected cases use S<=3 and t<99999999; any other layout is rejected.

Analytical concentration profiles cover initial plus analysis times; continuum
observables cover every diagnostic profile time. This resolves the earlier API
wording in favor of the explicit scientific-coverage and cost contract.

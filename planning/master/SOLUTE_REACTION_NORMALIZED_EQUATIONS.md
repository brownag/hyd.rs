# Active reaction equations on a common basis

## Scope and conventions

This note normalizes the accepted first interval for `reaction_equal` and uses
`reaction_zero` as its inactive control. Length is cm, time is h, aqueous
concentration is `C_ref`, bulk density is g/cm³, and sorbed concentration has
units `C_ref cm³/g`; both aqueous and sorbed mass per bulk volume therefore
have units `C_ref`. Physical coordinates increase from inlet to outlet. Species
order is S1, S2, S3. The traced step is `dt=0.03125 h`; the reaction cases have
zero flow and zero dispersion, so boundary flux contributions vanish for the
uniform states considered here. Fortran prints are rounded single-precision
values.

## Rust nodal balance

For node `i`, let `w_i` be the control-volume width and
`M_s(C)=theta C + rho S_s(C)` the local mass per volume. Rust's assembly is
equivalent, for a linearized mass relation, to

`w_i [M_s(C_i^{n+1}) - M_s(C_i^n)]/dt + w_i lambda_s M'_s C_i^{n+1}`
`    - (transport operator)_i = w_i source_s,i`,

with source zero for S1 and `lambda_s` the aqueous decay rate. The implementation
forms `gamma=1/dt+lambda_s`, adds `w_i gamma M'_s` to the diagonal, and puts
`w_i [M_s(C^n)/dt + source - gamma intercept]` on the right, where
`M(C)=M' C + intercept`. For linear equilibrium sorption,
`M' = theta + rho Kd` and intercept is zero. This is a nodal control-volume
equation: the mass weight is `w_i`, not an element consistent-mass matrix.

For `reaction_equal` S1, `theta=0.43`, `rho=0.8`, `Kd=0.5375 cm³/g`, so
`M'=0.43+0.8*0.5375=0.86`; `lambda_1=0.2 h⁻¹`, hence
`gamma=32.2 h⁻¹`. At an interior node with `w=0.25 cm`, initial `C=1`,
the local reaction/storage diagonal contribution is `0.25*32.2*0.86=6.923`
and RHS is `0.25*(0.86/0.03125)=6.88`. With no flux, the row gives
`C^{n+1}=6.88/6.923=0.99378882`, matching the accepted Rust trace.
This identifies the Rust coefficient mapping for the retained case; it does
not by itself establish equivalence to Fortran's spatial mass assembly.

## Fortran element balance and normalization limit

Fortran's `Coeff` builds element contributions and stores them in the tridiagonal
`B,D,E` row. The retained interior S1 row is `(B,D,E,RHS) =
(0, 6.89433336, 0.00358333392, 6.85850000)` at the first interval, with
`R=-0.172000021` as the reaction term. In physical node orientation the same
row acts on the local and adjacent nodal concentrations; unlike the Rust row,
its reaction/storage contribution is distributed over element basis functions.
The endpoints have half-element support. Summing element rows into a nodal
control-volume basis is possible only after the full per-element coefficients,
quadrature convention, and boundary residual are known; a single interior row
does not determine that transformation. The Fortran trace and source show a
nonzero neighbor coefficient where the Rust zero-flux reaction-only row is
diagonal. This is consistent with a consistent finite-element mass matrix,
not evidence of an erroneous rate by itself.

The available Fortran scalar `R` is not directly comparable to Rust's
`reaction_source`: Fortran's `R` participates in assembled FE terms and its
`g1` contains the first-order loss multiplied by aqueous/sorbed phase factors.
For this linear equilibrium case, the printed contribution includes
`theta*lambda` in the aqueous phase and `rho*lambda_sorb` only if the configured
sorbed reaction is active. A normalized comparison needs the exact mapped
Fortran species parameters and element-to-node mass matrix, including the
distinct aqueous and sorbed reaction parameters. The task 15 trace lacks that
complete coefficient ledger in a common basis. `Sorb` output is not used as an
equivalent Rust storage field.

## Chain species

Rust solves species sequentially. After solving species `s`, it stores its new
mass at each node; the next species source is
`lambda_{s-1} M_{s-1}(C_{s-1}^{n+1})`. In the retained equal case, the observed
Rust S2 source is `0.1709316770` and S3 source is `0.00106168743`.
Fortran `Coeff` adds prior-species terms to `g0/q0` using the preceding species'
reaction parameters and stored concentration/sorption state; these terms are
then assembled through the FE rows. The trace reports Fortran S2 `R` near
`0.170928374` and S3 source near `0.00106831267`, but these are not normalized
nodal mass sources. S2/S3 cannot isolate the cause while S1's storage and sink
operator remains unresolved. The `reaction_distinct/Fortran` trace is excluded
because it was an unintended seventh launch.

## Control and disposition

`reaction_zero` has all reaction rates zero and agrees at all nine nodes for
S1/S2/S3 after the first interval. It confirms the inactive transport/storage
path for this setup only. For active `reaction_equal`, Rust's uniform S1 result
is `0.9937888199` and Fortran's printed value is `0.993769467`; the small
difference appears before chain-source accumulation. The formulations can be
described dimensionally, but the available Fortran evidence does not map its
element reaction/storage contributions and boundary residual onto the Rust
nodal weights. No unique Rust defect follows.

**Disposition: `no_fix_cause_unisolated`.** The missing discriminator is a full
accepted Fortran per-element S1 coefficient/RHS trace plus exact mapped aqueous
and sorbed reaction parameters, permitting assembly into the same nodal mass
basis as Rust. Only then should effective sink coefficients be compared; chain
sources should follow S1 normalization. No code change is justified by the
current parity trace. Printed-output agreement, if later achieved, would not
establish physical or continuum accuracy.

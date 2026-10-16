//! Source-coupling consumers of externally produced root, water and biology state:
//! `subroutine SetSSnk` (`SINK.FOR:138-222`) behind `Solute`'s `if(SinkF)` call, and
//! `subroutine Exclusion` (`SOLUTE.FOR:1713-1779`) behind HYDRUS's
//! `if(lBact.and..not.lWat)` call.
//!
//! Both calls are live, not commented. `SOLUTE.FOR:94` is `if(SinkF)` and physical
//! column 6 carries `!` on records `95` and `96`, so the statement completes as
//! `call SetSSnk(jS,NS,N,t,x,Beta,Sink,sSink,NSD,Conc,OmegaW,cRootMax(jS),lActRSU,
//! OmegaS,SPot,rKM,cMin)` - seventeen actuals, positionally identical to the dummy list
//! of `SINK.FOR:138-139` (`cRootMax(jS)`, an element of the caller's `cRootMax(NS)`,
//! reaching the scalar dummy `cRootMax`). `HYDRUS.FOR:390` is
//! `if(lBact.and..not.lWat)` and column 6 carries `!` on record `391`, so it completes
//! as `call Exclusion(NumNP,NMat,NSD,ParD,ChPar,ThNew,vNew,ThOld,vOld)` - eight actuals
//! against the eight dummies of `SOLUTE.FOR:1713-1714`, in that order. Only two records
//! in the four regions are comments: `SOLUTE.FOR:93` (column-1 `*` banner) and
//! `SOLUTE.FOR:1770` (column-1 `c`), while `SINK.FOR:170`'s `!` sits at column 31 and is
//! therefore an inline comment, not a continuation.
//!
//! Neither producer is ported here and nothing recomputes their output. `SetSnk`
//! (`HYDRUS.FOR:352-355` and `482-487`), `SinkIn` (`345-350`), `SetRG`, `WatFlow` and
//! `Veloc` produce `Sink`, `Beta`, `OmegaW`, `SPot`, `OmegaS`, `rKM`, `cMin`, `cRootMax`,
//! `ThNew` and `vNew`; [`RootUptakePrimitiveState`] and [`SizeExclusionPrimitiveState`]
//! carry those values as received. The two guard logicals `SinkF` and `lWat` come from
//! the `Site.dat` control record (`INPUT.FOR:31`) and keep exactly one home each, in
//! [`super::types::SourceSwitches`]: they are kernel parameters, never bundle fields, so
//! a request cannot assert a guard value that contradicts the control read.
//!
//! No parity claim: nothing here is reachable from the driver or from any pinned request
//! yet, so no value in this module is compared against Fortran.

use super::types::{
    DefaultReal, DoubleReal, KernelError, RootUptakePrimitiveState, SizeExclusionPrimitiveState,
};

/// The `write(78,100)` row of `SINK.FOR:216-217`, guarded by
/// `if(iStep.eq.nStep.and.jS.eq.NS)`.
///
/// Record `217` carries `!` in column 6, so the write is live and unit 78 is
/// `\PROFILE.OUT` (`HYDRUS.FOR:153-154`). The port performs no file I/O: the fixture
/// runner captures that file separately, and the routine's printed values are returned
/// instead. `Option` marks the values Fortran leaves undefined - `SAUptakeP` unless some
/// node had `Beta(i)>0` in step 1 (`194`), `SAUptakeAN` unless step 2 ran (`201`) - so an
/// undefined value is `None` and never the `0.` the routine never assigns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UptakeProfileRecord {
    /// `t`, the `double precision t` of `141`.
    pub time: DoubleReal,
    /// `SPUptake`, the passive uptake integral of `192`.
    pub passive_uptake: DefaultReal,
    /// `SAUptakeP`, the potential active uptake of `194`.
    pub potential_active_uptake: Option<DefaultReal>,
    /// `SAUptakeA`, the actual active uptake of `200`/`210`.
    pub actual_active_uptake: DefaultReal,
    /// `SAUptakeAN`, the uncompensated actual uptake of `201`.
    pub uncompensated_active_uptake: Option<DefaultReal>,
}

/// `SinkS` after one `SetSSnk` call, plus the profile row that call would print.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RootSoluteUptake {
    /// `SinkS(i)` (`191`, `198`, `209`, `213`), the local active solute uptake rate
    /// density. `SOLUTE.FOR`'s only reader is `Coeff`'s
    /// `g0(i)=xMuL*ThW+Frac*f_em*ro*xMuS+ThG*xMuG-sSink(i)` (`644`), and `Solute` never
    /// zeroes `sSink`, which is why a false `SinkF` must return `None` rather than a
    /// zeroed array.
    pub solute_sink: Vec<DefaultReal>,
    /// `Some` only when `iStep.eq.nStep.and.jS.eq.NS` held.
    pub profile_record: Option<UptakeProfileRecord>,
}

/// `subroutine SetSSnk` (`SINK.FOR:138-222`) for one species of one time step.
///
/// `root_sink` is `SinkF`, the `SOLUTE.FOR:94` call guard: when it is false the call does
/// not happen, `sSink` keeps whatever the caller holds and nothing is mutated, so the
/// result is `Ok(None)`. `SinkS` is the only thing the routine writes, so this function
/// returns a fresh array instead of writing through state.
///
/// Every quantity is default REAL - neither `SOLUTE.FOR` nor `SINK.FOR` contains an
/// `implicit` statement (grepped) - except `t` (`141`), which is double and reaches only
/// the profile row. `SINK.FOR:170-171` is reproduced as written: `lLast` stays `.false.`
/// because that line's `!` is an inline comment at column 31, so the
/// `if(lLast.and.jS.lt.NS) nStep=1` of `171` is dead and no species-reduction branch
/// exists here. Do not "restore" it.
// Preserve the source's accumulation expressions and loop shape for fidelity: the `x = x
// + ...` sums are the Fortran running totals of `192`/`197`/`210`, and the index loop is
// the source's `do 11 i=1,N` (`180`), whose `i` also drives the `182-188` neighbour reads.
#[allow(clippy::assign_op_pattern, clippy::needless_range_loop)]
pub(crate) fn root_solute_uptake(
    state: &RootUptakePrimitiveState,
    root_sink: bool,
) -> Result<Option<RootSoluteUptake>, KernelError> {
    if !root_sink {
        return Ok(None);
    }
    validate_uptake(state)?;
    let nodes = state.coordinates.len();
    let minimum = state.minimum_concentration.0;
    let michaelis = state.michaelis_constant.0;
    let potential_root = state.potential_uptake.0;
    let water_stress = state.omega_water.0;
    let solute_stress = state.solute_stress_index.0;
    let maximum_concentration = state.species_root_max_concentration.0;
    // The clear runs once per call, not once per step: `do 10 i=1,N / SinkS(i)=0. /
    // 10 continue` is at `174-176`, *before* `do 12 iStep=1,nStep` (`178`), and the
    // label `12` closes at `218`. The `vec![0.0; nodes]` below is that single clear, and
    // nothing re-zeroes the array inside the step loop. `198`
    // (`if(nStep.eq.2) SinkS(i)=SinkS(i)+AUptakeA`) and `209`
    // (`SinkS(i)=SinkS(i)+AUptakeA`) therefore add onto the step-1 passive value of
    // `191`; only the `Beta(i).le.0.` arm (`213`) zeroes a node, and it does so on every
    // step. A caller-owned `SinkS` that carried a previous species' values is likewise
    // discarded, and `N` is `NumN` at the call (`SOLUTE.FOR:95`).
    let mut solute_sink = vec![DefaultReal(0.0); nodes];

    // `Compen=1.` (`163`) and the `nStep` ladder of `164-166`.
    //
    // `lLast=.false.` (`170`) is a hard-coded constant, so the `nStep=1` downgrade of
    // `171` can never fire and multi-species chains are *not* truncated to the passive
    // step. The literal Rust spelling of a condition that cannot hold is
    // `false && jS < NS`, which clippy rightly flags; the dead branch is kept as a
    // comment rather than as an always-false `if` that would one day look live.
    //
    //     lLast=.false.                  ! Active uptake only for the last solute
    //     if(lLast.and.jS.lt.NS) nStep=1
    let mut compensation = 1.0_f32;
    let mut steps = 1usize;
    if state.active_uptake {
        steps = 2;
    }
    if state.active_uptake && solute_stress < 1.0 {
        steps = 3;
    }
    // Routine locals (`163`, `172-173`) carried across both loops exactly as Fortran
    // would. The four `Option`s are the values whose assignments are conditional, so
    // `None` means "Fortran has not assigned this yet" and never `0.`: `SAUptakeP`
    // (`194`), `Omega1` (`202`), `AUptakeA` (`196`/`207-208`) and `SAUptakeAN` (`201`).
    // `SAUptakeA` is not an `Option` because `179` assigns it `0.` at the start of every
    // step.
    let mut passive_uptake = 0.0_f32;
    let mut omega_integral = 0.0_f32;
    let mut actual_uptake;
    let mut potential_active: Option<f32> = None;
    // The step-3 loop reads `Omega1` at `205`/`206` before the step-2 loop has run, so
    // the value in hand is the one the step-2 loop last stored. Fortran keeps it in the
    // same local across the `12 continue` boundary, so the port keeps it here too;
    // `None` is the genuinely undefined case, never `0.`.
    let mut omega_fraction: Option<f32> = None;
    let mut node_active: Option<f32> = None;
    let mut uncompensated: Option<f32> = None;
    let mut profile_record = None;

    for step in 1..=steps {
        actual_uptake = 0.0; // `SAUptakeA=0.` (`179`), once per step.
        for i in 0..nodes {
            let beta = state.root_distribution[i].0;
            let concentration = state.species_concentration[i].0;
            // `dxM` of `182-188`. `i.eq.N` wins over `i.eq.1`, which is why
            // `validate_uptake` rejects one node: `N=1` would read `x(0)`. The three
            // forms are not symmetric, and on a downward-positive profile the top node's
            // `dxM` is negative.
            let dx_m = if i + 1 == nodes {
                (state.coordinates[i].0 - state.coordinates[i - 1].0) / 2.0
            } else if i == 0 {
                (state.coordinates[i].0 - state.coordinates[i + 1].0) / 2.0
            } else {
                (state.coordinates[i + 1].0 - state.coordinates[i - 1].0) / 2.0
            };
            if beta > 0.0 {
                // `cc=amax1(Conc(jS,i)-cMin,0.)` (`189`).
                let cc = (concentration - minimum).max(0.0);
                if step == 1 {
                    // `SinkS(i)=Sink(i)*amax1(amin1(Conc(jS,i),cRootMax),0.)` (`191`),
                    // the node-order integral of `192`, then the running potential of
                    // `194` (comment `193`: only the last node's value is needed, and
                    // that node may have no beta).
                    let passive = state.root_water_sink[i].0
                        * concentration.min(maximum_concentration).max(0.0);
                    solute_sink[i] = DefaultReal(passive);
                    passive_uptake = passive_uptake + passive * dx_m;
                    potential_active =
                        Some((potential_root * water_stress - passive_uptake).max(0.0));
                } else if step == 2 {
                    // `195 else if(iStep.eq.2)` sits inside the `Beta(i).gt.0.` test of
                    // `181`, and a node whose `Beta` vanished in step 1 never reaches
                    // here. The read is guarded by the same predicate that guarded its
                    // assignment (`194`), so it is defined on every node that does.
                    let potential = potential_active.ok_or(
                        "SINK.FOR:196 reads SAUptakeP, which SINK.FOR:194 assigns only \
                         for a node with Beta(i)>0 during step 1",
                    )?;
                    // `cc/(rKM+cc)*Beta(i)*SAUptakeP` (`196`): one `*`/`/` chain, so
                    // Fortran associates strictly left to right.
                    let active = cc / (michaelis + cc) * beta * potential;
                    node_active = Some(active);
                    // `197-202`. `Omega` accumulates in node order, so `Omega1` is a
                    // running ratio and not one final ratio.
                    omega_integral = omega_integral + active * dx_m;
                    // `if(nStep.eq.2) SinkS(i)=SinkS(i)+AUptakeA` (`198`): a three-step
                    // run deliberately does not add here, `209` does.
                    if steps == 2 {
                        solute_sink[i] = DefaultReal(solute_sink[i].0 + active);
                    }
                    actual_uptake = omega_integral;
                    uncompensated = Some(omega_integral);
                    if potential > 0.0 {
                        omega_fraction = Some(omega_integral / potential);
                    }
                } else {
                    // `205-206` read `Omega1`, whose only assignment is `202` and which
                    // runs only while `SAUptakeP` is positive. Arriving at step 3 without
                    // any node having stored it is the undefined read this port refuses
                    // rather than defaults: Fortran would hand back whatever the last
                    // call's stack held, which is exactly the kind of value a port must
                    // not invent.
                    let fraction = omega_fraction.ok_or(
                        "SINK.FOR:205-206 read Omega1, which SINK.FOR:202 assigns only \
                         while SAUptakeP is positive during an earlier step",
                    )?;
                    if fraction < solute_stress && fraction > 0.0 {
                        compensation = solute_stress;
                    }
                    if fraction >= solute_stress {
                        compensation = fraction;
                    }
                    // The `207-208` continuation,
                    // `AUptakeA=cc/(rKM+cc)*Beta(i)*SAUptakeP/Compen`, the same
                    // left-to-right chain extended by one more division.
                    if compensation > 0.0 {
                        let potential = potential_active.ok_or(
                            "SINK.FOR:208 reads SAUptakeP, which SINK.FOR:194 assigns \
                             only for a node with Beta(i)>0 during step 1",
                        )?;
                        node_active = Some(cc / (michaelis + cc) * beta * potential / compensation);
                    }
                    // `209-210`. A nonpositive `Compen` leaves `AUptakeA` at its previous
                    // value, exactly as a Fortran local would; if no step ever assigned
                    // it the read is undefined and is rejected, not zeroed.
                    let active = node_active.ok_or(
                        "SINK.FOR:209 reads AUptakeA although neither SINK.FOR:196 nor \
                         SINK.FOR:207-208 assigned it, the latter's Compen.gt.0. guard \
                         having failed",
                    )?;
                    solute_sink[i] = DefaultReal(solute_sink[i].0 + active);
                    actual_uptake = actual_uptake + active * dx_m;
                }
            } else {
                // `SinkS(i)=0.` (`212-214`).
                solute_sink[i] = DefaultReal(0.0);
            }
        }
        // `SINK.FOR:216-217`, whose guard is a column-6 continuation: the row belongs to
        // the last step of the last species only.
        if step == steps && state.species == state.species_count {
            profile_record = Some(UptakeProfileRecord {
                time: state.time,
                passive_uptake: DefaultReal(passive_uptake),
                potential_active_uptake: potential_active.map(DefaultReal),
                actual_active_uptake: DefaultReal(actual_uptake),
                uncompensated_active_uptake: uncompensated.map(DefaultReal),
            });
        }
    }
    Ok(Some(RootSoluteUptake {
        solute_sink,
        profile_record,
    }))
}

/// `subroutine Exclusion` (`SOLUTE.FOR:1713-1779`) as the mutating case.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SizeExclusion {
    /// `1722-1723`: `if(abs(th_c).lt.1.e-20) return` runs *before* the `ChPar(4,1)=0.`
    /// of `1724` and before the `1772-1777` node writes, so the routine changes nothing
    /// at all - not even the `ChPar` slot. The port holds no copy of the caller's four
    /// node arrays on this path (it may not invent one), so it reports which value kept
    /// the routine out.
    Inactive {
        colloid_exclusion_water_content: DefaultReal,
    },
    /// The routine ran to `1778`: `ThC` is in every node of `ThNew`/`ThOld`, `VelC` in
    /// every node of `vNew`/`vOld`, and `ChPar(4,1)` is zero.
    Applied {
        /// `ThNew(:)` (`1773`), every element equal to `ThC` of `1742`.
        new_water: Vec<DefaultReal>,
        /// `ThOld(:)` (`1774`), every element equal to the same `ThC`.
        old_water: Vec<DefaultReal>,
        /// `vNew(:)` (`1775`), every element equal to `VelC` of `1759`/`1762`.
        new_velocity: Vec<DefaultReal>,
        /// `vOld(:)` (`1776`), every element equal to the same `VelC`.
        old_velocity: Vec<DefaultReal>,
        /// `ChPar(4,1)` after `1724`, i.e. always zero. It is the same `ChPar` slot
        /// `MatSet` subtracts from both waters, so this is an observable mutation of the
        /// chemical parameter set that the caller must apply.
        colloid_exclusion_water_content: DefaultReal,
        /// `1735-1739`: `sw_c` outside `0..1`. The source writes two console lines and
        /// then blocks on `read(*,*)` before falling through with `sw_c` unclamped; the
        /// port writes nothing and reads nothing, it only reports, and every number
        /// beside it is exactly the one the source continues with.
        size_exclusion_warning: bool,
    },
}

impl SizeExclusion {
    /// Whether `1723` returned before any write, i.e. whether `ChPar(4,1)` survives.
    #[cfg(test)]
    pub(crate) const fn is_inactive(&self) -> bool {
        matches!(self, Self::Inactive { .. })
    }
}

/// `subroutine Exclusion` for one step, behind the `HYDRUS.FOR:390-391` guard.
///
/// `bacteria` is `lBact`, `.true.` when `iBact.eq.1` in the solute control read
/// (`INPUT.FOR:1313-1314`), and `water_flow` is `lWat` (`INPUT.FOR:31`). The guard is
/// `if(lBact.and..not.lWat)`, so `Ok(None)` means no call happened and the caller keeps
/// its own four node arrays and `ChPar(4,1)`.
///
/// Precision is reproduced literally. Records `1716-1717` (a column-6 continuation)
/// widen `thr`, `ths`, `swr`, `alpha`, `vgn1`, `vgn2`, `vgm1`, `vgm2`, `Pc_c`,
/// `sw_c_eff`, `sw_c`, `sw`, `sw_eff`, `kr_c`, `krw` and `r_c` to double precision, while
/// `th_c`, `xL`, `ThC`, `VelC`, `PorVelSolute` and `PorVelColloid` stay default REAL - so
/// `ThC` and `VelC` are computed in `f64` and narrowed once, on assignment.
///
/// Deliberately not ported, because it is dead: `1765-1769` computes `Pc_c`, `r_c`,
/// `PorVelSolute` and `PorVelColloid`, whose only reader is `1770`, a column-1 `c`
/// comment. Nothing in [`SizeExclusion`] carries any of them, and `alpha` (`Par(3,1)`,
/// assigned at `1729`) is a dead store for the same reason, so it has no input field
/// either. A later task must not resurrect these quantities.
// Preserve the source's guard expressions: `1735`'s `sw_c.lt.0.or.sw_c.gt.1.` is a
// negated range with the source's own NaN behaviour, and `HYDRUS.FOR:390`'s guard is written as a
// conjunction of a negation.
#[allow(clippy::nonminimal_bool, clippy::manual_range_contains)]
pub(crate) fn size_exclusion(
    state: &SizeExclusionPrimitiveState,
    bacteria: bool,
    water_flow: bool,
) -> Result<Option<SizeExclusion>, KernelError> {
    if !(bacteria && !water_flow) {
        return Ok(None);
    }
    validate_exclusion(state)?;
    // `th_c=ChPar(4,M)` (`1722`) and the early return of `1723`, both default REAL.
    // `1722` is a default-precision *read* of `ChPar` and `1723` is a default-precision
    // compare, so on the original 32-bit-`real*8` build the value that reaches `1728` is
    // `dble(real(ChPar(4,1)))` and the threshold is the f32 rounding of `1.e-20`
    // (9.999999682655225e-21), not the f64 constant.
    let exclusion_content = state.colloid_exclusion_water_content.0;
    // The threshold is `1.e-20` (`1723`), which under default real typing is a
    // *single-precision* literal, so the comparison this port makes is against the f32
    // rounding of 1e-20 (9.999999682655225e-21) and not against the f64 constant.
    if exclusion_content.abs() < 1.0e-20_f32 {
        return Ok(Some(SizeExclusion::Inactive {
            colloid_exclusion_water_content: DefaultReal(exclusion_content),
        }));
    }
    // `1724`, the routine's only `ChPar` write. It reads `ChPar(4,1)` again *after*
    // `1722`, which matters only for the rejected non-finite path below: the source
    // would overwrite the slot with its own garbage. `exclusion_content` is the value
    // `1722` read, i.e. the previous step's value, and is what `1723` tested.
    //
    // `1723` and `1735` are the two guards that matter to a caller. `1735` is the
    // interactive `write`/`read(*,*)` block, which the port cannot reproduce: it is a
    // console prompt, not a change of state, so it surfaces as the optional warning on
    // the applied variant and never as an error. `1723` is the early return.
    let zeroed_exclusion_content = DefaultReal(0.0);
    // `1725-1734`. `th_c` widens into `sw_c`; `xL` stays default REAL.
    // `thr`/`ths` are declared double (`1716-1717`) but assigned from
    // `Par(1,M)`/`Par(2,M)` (`1725-1726`), which are default REAL, so each is a
    // single-to-double widening of the value the deck actually stored. `DefaultReal` holds
    // that single-precision value, and `f64::from` reproduces the widening exactly.
    let residual = f64::from(state.mobile_residual_water.0);
    let saturated = f64::from(state.mobile_saturated_water.0);
    // `swr` (`1727`) and `sw_c` (`1728`) are both *double* quotients. `thr`, `ths`, `swr`
    // and `sw_c` are in the `double precision` list of `1716-1717`, so each division runs at
    // double precision on values widened from the single-precision card, and each result is
    // consumed by the double expressions of `1742` and `1745-1746`. Narrowing either
    // quotient to f32 here would put `sw_eff`, `sw_c_eff` and the `1749`/`1756` guards a
    // fraction of an ULP off the source on any deck where the ratio is not representable.
    let sw_r = residual / saturated;
    let sw_c = f64::from(exclusion_content) / saturated;
    // `**2.0` on a double base (`1750`, `1758`) squares the value; written as a multiply
    // because that is what a compiler emits for an exponent of exactly two.
    let square = |v: f64| v * v;
    let vgn_1 = f64::from(state.mobile_n.0);
    let x_l = state.mobile_tortuosity.0;
    let vgm_1 = 1.0 - 1.0 / vgn_1;
    let vgn_2 = vgn_1 + 1.0;
    let vgm_2 = 1.0 - 2.0 / vgn_2;
    // `1735-1739`, the diagnostic branch that would prompt on the console.
    let size_exclusion_warning = sw_c < 0.0 || sw_c > 1.0;
    // `ThC=ThNew(1)-ths*sw_c` (`1742`) multiplies `ths` by the *double* `sw_c` (`1728`),
    // subtracts in double, and narrows once into the default REAL `ThC`. The algebraically
    // identical single-precision `ThNew(1)-th_c` is not the source text, but on this deck
    // the two agree: `0.6f32 - 0.2f32` is 0.40000004, and
    // `f64(0.6f32) - f64(1.0) * f64(0.2f32/1.0)` is 0.40000002086162567, exactly halfway
    // between the f32 neighbours 0.40000000596 (odd mantissa) and 0.40000003576 (even
    // mantissa), so round-half-to-even narrows it to 0.40000004. Both differ from
    // `f32(0.4)` = 0.4000000059604645, and this fixture cannot tell the two forms apart;
    // the port keeps the source's double form. `ThC` is what `1772-1774` broadcast to every
    // node of `ThNew` *and* `ThOld`, so that ULP is the water content of the entire profile.
    let new_water_first = f64::from(state.first_node_new_water.0);
    let accessible_water = DefaultReal((new_water_first - saturated * sw_c) as f32);
    // `1744-1746`.
    let sw = new_water_first / saturated;
    let sw_eff = (sw - sw_r) / (1.0 - sw_r);
    let sw_c_eff = (sw_c - sw_r) / (1.0 - sw_r);
    // The colloid relative permeability of `1749-1753`. Record `1751` continues `1750` at
    // column 6, so *both* terms of the difference are live - a port that dropped the second
    // as a continuation comment would keep `kr_c` positive everywhere. Both exponents are
    // positive, so a `sw_c_eff` of zero is finite; it is `1766`'s `Pc_c`, which this port
    // does not model, that carries the routine's only negative exponent.
    let kr_c = if sw_eff > sw_c_eff {
        let c_term = (1.0 - sw_c_eff.powf(1.0 / vgm_2)).powf(vgm_2);
        let w_term = (1.0 - sw_eff.powf(1.0 / vgm_2)).powf(vgm_2);
        square(sw_eff) * (c_term - w_term)
    } else {
        // `1753`.
        0.0
    };
    // `1756-1763`. This one is a genuine `if`/`else` on `sw_eff.gt.0.0`, and the `else`
    // (`1760-1761`) is `krw=0.` and `VelC=vNew(1)` - a *direct assignment*, with no
    // division, so a zero effective saturation does not produce the `0/0` a port that
    // hoists the division out of the branches would. Inside the `then`, `**2.0` is again a
    // real exponent on the outer parenthesis and `1/vgm1` a double reciprocal.
    let new_velocity_first = if sw_eff > 0.0 {
        let outer = 1.0 - (1.0 - sw_eff.powf(1.0 / vgm_1)).powf(vgm_1);
        let kr_w = sw_eff.powf(f64::from(x_l)) * square(outer);
        // `VelC=vNew(1)*kr_c/krw` (`1759`): one left-to-right `*`/`/` chain on the double
        // side, narrowed once into the default REAL `VelC`.
        (f64::from(state.first_node_new_velocity.0) * kr_c / kr_w) as f32
    } else {
        // `1760-1761`, `krw=0.` never reaching an expression.
        state.first_node_new_velocity.0
    };
    let new_velocity_first = DefaultReal(new_velocity_first);
    if !accessible_water.0.is_finite() || !new_velocity_first.0.is_finite() {
        return Err(
            "SOLUTE.FOR:1742/1759 produced a non-finite node value, which the source \
             would copy into every node of ThNew/ThOld/vNew/vOld; this port rejects it",
        );
    }
    Ok(Some(SizeExclusion::Applied {
        new_water: vec![accessible_water; state.nodes],
        old_water: vec![accessible_water; state.nodes],
        new_velocity: vec![new_velocity_first; state.nodes],
        old_velocity: vec![new_velocity_first; state.nodes],
        colloid_exclusion_water_content: zeroed_exclusion_content,
        size_exclusion_warning,
    }))
}

/// Rejects what the port cannot represent, before anything is produced.
///
/// None of these predicates is a source validation: `SetSSnk` validates nothing. The
/// length and node-count rejections guard reads the source itself would make out of
/// bounds - `182-183` reads `x(i-1)` under `i.eq.N`, so `N=1` reads `x(0)` - and the
/// non-finite rejection keeps a NaN from reaching `SinkS`, the only array the routine
/// writes. They are contract rejections, not source statements.
pub(super) fn validate_uptake(state: &RootUptakePrimitiveState) -> Result<(), KernelError> {
    let nodes = state.coordinates.len();
    if nodes < 2 {
        return Err("root solute uptake needs at least two nodes (SINK.FOR:182-183)");
    }
    if state.root_distribution.len() != nodes
        || state.root_water_sink.len() != nodes
        || state.species_concentration.len() != nodes
    {
        return Err("root solute uptake node arrays must all match coordinates");
    }
    if state.species_count == 0 || state.species == 0 || state.species > state.species_count {
        return Err("root solute uptake species must be one-based and within NS");
    }
    let scalars = [
        state.omega_water,
        state.species_root_max_concentration,
        state.solute_stress_index,
        state.potential_uptake,
        state.michaelis_constant,
        state.minimum_concentration,
    ];
    if !state
        .coordinates
        .iter()
        .chain(state.root_distribution.iter())
        .chain(state.root_water_sink.iter())
        .chain(state.species_concentration.iter())
        .all(|v| v.0.is_finite())
        || !scalars.iter().all(|v| v.0.is_finite())
        || !state.time.0.is_finite()
    {
        return Err("root solute uptake inputs must be finite");
    }
    Ok(())
}

/// The [`validate_uptake`] counterpart for `Exclusion`. The `Par(1,1)`/`Par(2,1)`
/// rejection guards the `swr` division of `1727` and the `1.0-swr` divisions of
/// `1745-1746`, which the source leaves as infinities that then reach every node.
/// Contract rejections, not source statements.
pub(super) fn validate_exclusion(state: &SizeExclusionPrimitiveState) -> Result<(), KernelError> {
    if state.nodes == 0 {
        return Err("size exclusion needs at least one node (SOLUTE.FOR:1772)");
    }
    let values = [
        state.mobile_residual_water,
        state.mobile_saturated_water,
        state.mobile_n,
        state.mobile_tortuosity,
        state.colloid_exclusion_water_content,
        state.first_node_new_water,
        state.first_node_new_velocity,
    ];
    if !values.iter().all(|v| v.0.is_finite()) {
        return Err("size exclusion inputs must be finite");
    }
    // Written as the negation the source's guard implies, so that a NaN `Par(2,1)` is
    // rejected rather than silently passing a `<=` comparison.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(state.mobile_saturated_water.0 > 0.0)
        || state.mobile_saturated_water.0 <= state.mobile_residual_water.0
    {
        return Err("size exclusion needs Par(2,1) above Par(1,1) and above zero");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SINK.FOR:182-188` in Rust, kept here as the reference the expectations are
    /// written against rather than inline constants: `182` tests `i.eq.N` *first*, so
    /// the bottom node's weight is `(x(N)-x(N-1))/2` and on a profile that decreases
    /// downward (`THREE`) that is **-5**, the interior nodes take `(x(i+1)-x(i-1))/2`
    /// (**-10** here, not the `+10` a cell-centred reading suggests), and only the top
    /// node, via `184-185`, gets **+5**. A test that assumed uniform `+5`/`+10` weights
    /// would silently disagree with the port on both `SPUptake` and `Omega`.
    const THREE: &[f32] = &[0.0, -10.0, -20.0];

    fn dx_m(x: &[f32]) -> Vec<f32> {
        let n = x.len();
        (0..n)
            .map(|i| {
                if i + 1 == n {
                    (x[i] - x[i - 1]) / 2.0
                } else if i == 0 {
                    (x[i] - x[i + 1]) / 2.0
                } else {
                    (x[i + 1] - x[i - 1]) / 2.0
                }
            })
            .collect()
    }

    /// A `SetSSnk` state whose node arrays cover three nodes and whose scalars are all
    /// finite, so a test reaches the routine instead of a contract rejection.
    fn uptake() -> RootUptakePrimitiveState {
        RootUptakePrimitiveState {
            species: 1,
            species_count: 1,
            time: DoubleReal(3600.0),
            coordinates: THREE.iter().map(|v| DefaultReal(*v)).collect(),
            root_distribution: vec![DefaultReal(1.0); 3],
            root_water_sink: THREE.iter().map(|v| DefaultReal(*v)).collect(),
            species_concentration: (0..3).map(|i| DefaultReal(2.0 + i as f32)).collect(),
            omega_water: DefaultReal(0.8),
            species_root_max_concentration: DefaultReal(1.5),
            active_uptake: false,
            solute_stress_index: DefaultReal(1.0),
            potential_uptake: DefaultReal(0.0),
            michaelis_constant: DefaultReal(1.0),
            minimum_concentration: DefaultReal(0.5),
        }
    }

    /// An `Exclusion` state with `thr=0.1`, `ths=1.0`, `vgn1=2.0`, `l=0.5` (i.e. the
    /// Mualem `xL` of `Par(6,1)`) and `ChPar(4,1)=0.2`, all above the rejection floors.
    fn exclusion() -> SizeExclusionPrimitiveState {
        SizeExclusionPrimitiveState {
            nodes: 3,
            mobile_residual_water: DefaultReal(0.1),
            mobile_saturated_water: DefaultReal(1.0),
            mobile_n: DefaultReal(2.0),
            mobile_tortuosity: DefaultReal(0.5),
            colloid_exclusion_water_content: DefaultReal(0.2),
            first_node_new_water: DefaultReal(0.6),
            first_node_new_velocity: DefaultReal(-0.25),
        }
    }

    /// The step-1-only run: `SinkF=false` means `SOLUTE.FOR:94` never calls the routine,
    /// so nothing is returned and `sSink` keeps the caller's values (`Coeff`'s `g0` of
    /// `SOLUTE.FOR:644` subtracts them). A zeroed `Some` here would silently erase a
    /// sink the source still carries.
    #[test]
    fn sink_guard_false_returns_no_call_and_never_a_zeroed_sink() {
        let state = uptake();
        assert_eq!(root_solute_uptake(&state, false).unwrap(), None);
        // The guard is evaluated before validation, so an unvalidatable state is still
        // simply not called rather than rejected.
        let mut broken = state;
        broken.coordinates = vec![DefaultReal(0.0)];
        assert!(root_solute_uptake(&broken, false).unwrap().is_none());
    }

    /// Passive uptake only (`lActRSU=false`, so `nStep=1`): `SinkS(i)` is the `191`
    /// product, `cRootMax(jS)` clamps the concentration, `cMin` does not shift it, and
    /// `Beta(i)<=0.` selects the `212-214` zero.
    #[test]
    fn passive_uptake_clamps_at_c_root_max_and_zeroes_where_beta_is_zero() {
        let mut state = uptake();
        state.root_distribution[1] = DefaultReal(0.0);
        state.species_concentration[2] = DefaultReal(9.0); // above cRootMax=1.5
        state.minimum_concentration = DefaultReal(0.0);
        let got = root_solute_uptake(&state, true).unwrap().unwrap();
        // `lActRSU=false` gives `nStep=1`, so only step 1 runs and `SinkS` is the `191`
        // passive product. Node 1 is zero because `Sink(1)=0`; node 2 is zero because
        // `Beta(2)=0` takes the `212-214` branch; node 3 is `-20*min(9,1.5)=-30`.
        assert_eq!(
            got.solute_sink,
            vec![DefaultReal(0.0), DefaultReal(0.0), DefaultReal(-30.0)]
        );
        // `iStep.eq.nStep.and.jS.eq.NS` holds for one species on the only step, so the
        // profile row exists. dxM is (x1-x2)/2=5 at the top node, (x3-x2)/2=5 at the
        // bottom, and 10 in the middle, but only node 3 contributes. The `x` profile
        // *decreases* downward (0, -10, -20), and `182` tests `i.eq.N` before `184`
        // tests `i.eq.1`, so the bottom node takes `dxM=(x(3)-x(2))/2=-5` and the top
        // node `dxM=(x(1)-x(2))/2=+5`: the two ends get opposite signs, so `SPUptake`
        // is not the trapezoid rule a "clean" port would write.
        let row = got.profile_record.unwrap();
        assert_eq!(row.time, DoubleReal(3600.0));
        assert_eq!(row.passive_uptake, DefaultReal(-30.0 * -5.0));
        // `194` assigns `SAUptakeP` unconditionally inside the `Beta>0` branch, so the
        // row carries it even for a passive-only run. Here the two end nodes contribute
        // `dxM = +5` and `-5` with opposite signs and cancel:
        // `SPUptake = 0*5 + 0*10 + (-30)*(-5) = 150` and
        // `SAUptakeP = max(0*0.8 - 150, 0) = 0`.
        assert_eq!(row.potential_active_uptake, Some(DefaultReal(0.0)));
        assert_eq!(row.actual_active_uptake, DefaultReal(0.0));
        assert_eq!(row.uncompensated_active_uptake, None);
    }

    /// `SINK.FOR:191` clamps the concentration but `189` leaves `cc` unclamped, and the
    /// two are used in different branches. Reproducing that asymmetry matters: a "clean"
    /// port that shares one clamped value would change the active term.
    #[test]
    fn passive_branch_clamps_concentration_while_cc_stays_unclamped() {
        let mut state = uptake();
        state.minimum_concentration = DefaultReal(10.0);
        state.species_root_max_concentration = DefaultReal(1.5);
        state.active_uptake = true;
        state.solute_stress_index = DefaultReal(1.0);
        state.potential_uptake = DefaultReal(100.0);
        let got = root_solute_uptake(&state, true).unwrap().unwrap();
        // Passive still uses clamp(Conc,0..1.5)=1.5 against Sink=[0,-10,-20], i.e.
        // [0,-15,-30]. Active uses cc=amax1(Conc-10,0)=0 for every node, so the step-2
        // term is zero. `SinkS` is cleared only once (`174-176`), so `198` stores
        // passive + 0 and the clamped passive term survives. A "clean" port that shared
        // one clamped value between the two branches would change the active term.
        let passive = [0.0_f32, -10.0 * 1.5, -20.0 * 1.5];
        let expected: Vec<DefaultReal> = passive.iter().map(|p| DefaultReal(p + 0.0)).collect();
        assert_eq!(got.solute_sink, expected);
    }

    /// Active uptake without compensation (`lActRSU=true`, `OmegaS=1.` so `nStep=2`):
    /// `SAUptakeP` comes from the step-1 running integral, `198` adds `AUptakeA`, and
    /// `200-201` report the same accumulated `Omega`.
    #[test]
    fn two_step_active_uptake_adds_the_michaelis_term_to_the_passive_sink() {
        let mut state = uptake();
        state.active_uptake = true;
        state.solute_stress_index = DefaultReal(1.0);
        state.potential_uptake = DefaultReal(50.0);
        state.minimum_concentration = DefaultReal(0.0);
        state.michaelis_constant = DefaultReal(1.0);
        let got = root_solute_uptake(&state, true).unwrap().unwrap();
        // Step 1 writes the `191` passive product, clamping `Conc` at `cRootMax`:
        // `SinkS = [0*1.5, -10*min(3,1.5), -20*min(9,1.5)] = [0, -15, -30]`, integrated
        // with the `182-188` weights [+5, -10, -5]:
        // `SPUptake = 0*5 + (-15)*(-10) + (-30)*(-5) = 300`, so
        // `SAUptakeP = max(SPot*OmegaW - SPUptake, 0) = max(40 - 300, 0) = 0`.
        //
        // The extraction profile this fixture reuses from the passive tests extracts no
        // water at the top node and its weighted sum is positive, so the potential the
        // active term is built from collapses to zero. That is a real behaviour of the
        // source, not a quirk of the fixture: `194` subtracts a *positive* `SPUptake`
        // (positive because `Sink` and `dxM` have matching signs under a decreasing `x`
        // profile, which is what `182` yields) from `SPot*OmegaW`, and clamps at zero.
        // With `SAUptakeP == 0.` the `196` active term is zero and the `201` guard
        // `SAUptakeP.gt.0.` never assigns `Omega1`.
        let dx = dx_m(&[0.0, -10.0, -20.0]);
        assert_eq!(dx, vec![5.0, -10.0, -5.0]);
        // `amin1(Conc, cRootMax)` clamps *every* node, so node 2's `Conc=3.0` is already
        // at `cRootMax` and the product is `-10*1.5`, not `-10*2.5`.
        let passive = [0.0_f32, -10.0 * 1.5, -20.0 * 1.5];
        let spu = passive
            .iter()
            .zip(&dx)
            .fold(0.0_f32, |acc, (s, w)| acc + s * w);
        let sa = (50.0_f32 * 0.8 - spu).max(0.0);
        assert_eq!(spu, 300.0);
        assert_eq!(sa, 0.0, "fixture must keep the documented collapse");
        // `198` adds `AUptakeA` onto the step-1 passive values (the only clear is
        // `174-176`), and the term itself is zero, so the sink is the passive product.
        let expected: Vec<DefaultReal> = passive.iter().map(|p| DefaultReal(p + 0.0)).collect();
        assert_eq!(got.solute_sink, expected);
        let row = got.profile_record.unwrap();
        assert_eq!(row.passive_uptake, DefaultReal(spu));
        assert_eq!(row.potential_active_uptake, Some(DefaultReal(sa)));
        assert_eq!(row.actual_active_uptake, DefaultReal(0.0));
        assert_eq!(row.uncompensated_active_uptake, Some(DefaultReal(0.0)));

        // Second profile: strong extraction at the top node so `SPot*OmegaW` survives and
        // the active branch actually runs.
        state.root_water_sink = vec![DefaultReal(-400.0), DefaultReal(-10.0), DefaultReal(-20.0)];
        let got = root_solute_uptake(&state, true).unwrap().unwrap();
        let passive = [-400.0_f32 * 1.5, -10.0 * 1.5, -20.0 * 1.5];
        let spu = passive
            .iter()
            .zip(&dx)
            .fold(0.0_f32, |acc, (s, w)| acc + s * w);
        let sa = (50.0_f32 * 0.8 - spu).max(0.0);
        assert!(sa > 0.0, "fixture must leave a positive potential");
        // `196` is one `*`/`/` chain, left to right, and `198` adds it onto the step-1
        // passive value, so a two-step run reports passive + active. `197`-`202`
        // accumulate the same term into a running integral, which `200-201` report.
        let cc = [2.0_f32, 3.0, 4.0];
        let mut omega = 0.0_f32;
        let mut expected = Vec::new();
        for i in 0..3 {
            let active = cc[i] / (1.0 + cc[i]) * 1.0 * sa;
            omega += active * dx[i];
            expected.push(DefaultReal(passive[i] + active));
        }
        assert_eq!(got.solute_sink, expected);
        let row = got.profile_record.unwrap();
        assert_eq!(row.potential_active_uptake, Some(DefaultReal(sa)));
        assert_eq!(row.actual_active_uptake, DefaultReal(omega));
        assert_eq!(row.uncompensated_active_uptake, Some(DefaultReal(omega)));
    }

    /// Discriminates the once-per-call clear: after a two-step run on nodes with
    /// `Beta>0` and nonzero passive uptake, `SinkS` is passive + active (`191` then
    /// `198`), neither the active term alone nor the passive term alone.
    #[test]
    fn two_step_run_final_sink_is_passive_plus_active_not_active_alone() {
        let mut state = uptake();
        state.active_uptake = true;
        state.solute_stress_index = DefaultReal(1.0);
        state.potential_uptake = DefaultReal(50.0);
        state.minimum_concentration = DefaultReal(0.0);
        state.michaelis_constant = DefaultReal(1.0);
        state.root_water_sink = vec![DefaultReal(-400.0), DefaultReal(-10.0), DefaultReal(-20.0)];
        let dx = dx_m(&[0.0, -10.0, -20.0]);
        let passive = [-400.0_f32 * 1.5, -10.0 * 1.5, -20.0 * 1.5];
        let spu = passive
            .iter()
            .zip(&dx)
            .fold(0.0_f32, |acc, (s, w)| acc + s * w);
        let sa = (50.0_f32 * 0.8 - spu).max(0.0);
        assert!(sa > 0.0, "fixture must leave a positive potential");
        let cc = [2.0_f32, 3.0, 4.0];
        let active: Vec<f32> = cc.iter().map(|c| c / (1.0 + c) * 1.0 * sa).collect();
        let got = root_solute_uptake(&state, true).unwrap().unwrap();
        let expected: Vec<DefaultReal> = (0..3)
            .map(|i| DefaultReal(passive[i] + active[i]))
            .collect();
        assert_eq!(got.solute_sink, expected);
        for i in 0..3 {
            assert_ne!(got.solute_sink[i].0, active[i], "node {i}: active alone");
            assert_ne!(got.solute_sink[i].0, passive[i], "node {i}: passive alone");
        }
    }

    /// A three-step run (`lActRSU` with `OmegaS.lt.1.`, so `nStep=3`) deliberately skips
    /// the `198` addition and accumulates at `209` instead, divided by `Compen` — and
    /// `205-206` set `Compen` from the *running* `Omega1` of the previous step, node by
    /// node, which is why the first node divides by a different number than the last.
    #[test]
    fn three_step_run_uses_compensation_and_skips_the_step_two_accumulation() {
        let mut state = uptake();
        state.active_uptake = true;
        state.solute_stress_index = DefaultReal(0.5);
        state.potential_uptake = DefaultReal(50.0);
        state.minimum_concentration = DefaultReal(0.0);
        state.michaelis_constant = DefaultReal(1.0);
        // A positive `SAUptakeP` is a precondition of step 3, not of `lActRSU`: `202` is
        // the only assignment of `Omega1` and it sits behind `SAUptakeP.gt.0.`, so
        // without one the step-3 loop reads an undefined value (see the next test).
        state.root_water_sink = vec![DefaultReal(-400.0), DefaultReal(-10.0), DefaultReal(-20.0)];
        let dx = dx_m(&[0.0, -10.0, -20.0]);
        let passive = [-400.0_f32 * 1.5, -10.0 * 1.5, -20.0 * 1.5];
        let spu = passive
            .iter()
            .zip(&dx)
            .fold(0.0_f32, |acc, (s, w)| acc + s * w);
        let sa = (50.0_f32 * 0.8 - spu).max(0.0);
        let cc = [2.0_f32, 3.0, 4.0];
        let omega_s = 0.5_f32;

        // Step 2, replayed exactly: `196` per node, `199` accumulates, `202` stores the
        // running ratio. `198` is guarded off, so step 2 leaves the step-1 passive values untouched.
        let mut running = 0.0_f32;
        let mut omega1 = Vec::new();
        for i in 0..3 {
            running += cc[i] / (1.0 + cc[i]) * 1.0 * sa * dx[i];
            omega1.push(if sa > 0.0 { running / sa } else { 0.0 });
        }
        let uncompensated = running;

        // Step 3, replayed exactly. `205`/`206` run per node against the `Omega1` left
        // over from step 2 *as it stood when this node was reached* — Fortran reads the
        // same scalar every iteration, so the value is the final one from step 2 for
        // every node of step 3 — and `Compen` is a `1.`-initialised local (`163`) that
        // is never reset between nodes.
        let omega1_final = *omega1.last().unwrap();
        let mut compen = 1.0_f32;
        if omega1_final < omega_s && omega1_final > 0.0 {
            compen = omega_s;
        }
        if omega1_final >= omega_s {
            compen = omega1_final;
        }
        let mut expected = Vec::new();
        let mut compensated = 0.0_f32;
        for i in 0..3 {
            let active = cc[i] / (1.0 + cc[i]) * 1.0 * sa / compen;
            // `209` adds onto the step-1 passive value; step 2 added nothing.
            expected.push(DefaultReal(passive[i] + active));
            compensated += active * dx[i];
        }
        let got = root_solute_uptake(&state, true).unwrap().unwrap();
        assert_eq!(got.solute_sink, expected);
        let row = got.profile_record.unwrap();
        assert_eq!(row.actual_active_uptake, DefaultReal(compensated));
        // `204` wrote `SAUptakeAN` during step 2 and step 3 never touches it, so the
        // reported uncompensated total is the step-2 integral, not the compensated one.
        assert_eq!(
            row.uncompensated_active_uptake,
            Some(DefaultReal(uncompensated))
        );
    }

    /// `202` is the only assignment of `Omega1` and `194` the only assignment of
    /// `SAUptakeP`; when the step-2 loop never takes the `SAUptakeP>0.` branch the
    /// step-3 read is undefined. The port rejects instead of substituting `0.`.
    #[test]
    fn undefined_omega_one_is_rejected_not_defaulted() {
        let mut state = uptake();
        state.active_uptake = true;
        state.solute_stress_index = DefaultReal(0.5);
        state.potential_uptake = DefaultReal(0.0);
        // SPot*OmegaW - SPUptake <= 0 with a nonpositive SPUptake is impossible, so
        // force SAUptakeP to 0 through the sink: Sink=0 makes SPUptake and SPot both 0.
        state.root_water_sink = vec![DefaultReal(0.0); 3];
        let err = root_solute_uptake(&state, true).unwrap_err();
        assert!(err.contains("SINK.FOR:202"), "{err}");
        assert!(err.contains("Omega1"), "{err}");
    }

    /// A profile with no roots at all never reaches `196`, `197`-`202` or `207-210`:
    /// `181`/`212-214` sends every node to `SinkS(i)=0.` on every step, so the undefined
    /// reads of `SAUptakeP` and `Omega1` are *guarded* and the run is legal. This is the
    /// case a defensive port gets wrong in the other direction, by rejecting a profile
    /// the source handles by returning a clean zero sink.
    #[test]
    fn a_root_free_profile_is_a_clean_zero_sink_on_every_step() {
        for beta in [-1.0_f32, 0.0] {
            let mut state = uptake();
            state.active_uptake = true;
            state.solute_stress_index = DefaultReal(0.5);
            state.root_distribution = vec![DefaultReal(beta); 3];
            let got = root_solute_uptake(&state, true)
                .unwrap()
                .expect("SinkF=true means the routine was called");
            assert_eq!(
                got.solute_sink,
                vec![DefaultReal(0.0), DefaultReal(0.0), DefaultReal(0.0)]
            );
            // The `181` guard never fires, so `191`-`194` never run: `SPUptake` keeps its
            // `173` zero and `SAUptakeP` is never assigned, yet the profile row is still
            // written because `216` only tests `iStep.eq.nStep.and.jS.eq.NS`.
            let row = got.profile_record.unwrap();
            assert_eq!(row.passive_uptake, DefaultReal(0.0));
            assert_eq!(row.potential_active_uptake, None);
            assert_eq!(row.actual_active_uptake, DefaultReal(0.0));
            assert_eq!(row.uncompensated_active_uptake, None);
        }
    }

    /// `194` assigns `SAUptakeP` inside the `Beta(i).gt.0.` branch, so it is defined for
    /// every node step 2 or step 3 reaches: the `195`/`203` `else if` arms sit in the
    /// *same* `Beta` guard (`181`) as the `190` arm that assigns it. A "node that skipped step 1"
    /// therefore cannot occur, and a port that only reaches step 2 when `SAUptakeP` is
    /// positive (the `202` guard is the sole assignment of `Omega1`) cannot reach an
    /// undefined `205` read either. This test pins that down as a positive result: the
    /// profiles that look like undefined reads are simply defined.
    #[test]
    fn the_beta_guard_makes_the_cross_step_reads_defined() {
        // Roots only at the top node: nodes 2 and 3 take the `212-214` zero on every
        // step, and node 1 assigns `SAUptakeP` in step 1 before reading it in step 2.
        let mut state = uptake();
        state.active_uptake = true;
        state.minimum_concentration = DefaultReal(0.0);
        state.root_distribution = vec![DefaultReal(1.0), DefaultReal(0.0), DefaultReal(0.0)];
        let got = root_solute_uptake(&state, true).unwrap().unwrap();
        assert_eq!(got.solute_sink[1], DefaultReal(0.0));
        assert_eq!(got.solute_sink[2], DefaultReal(0.0));
        // Three steps with the same profile: `Omega1` is assigned at node 1 of step 2
        // (the node also has `SAUptakeP.gt.0.` only if the potential survived; here the
        // single root node keeps it positive).
        state.solute_stress_index = DefaultReal(0.5);
        state.root_water_sink = vec![DefaultReal(-400.0), DefaultReal(-10.0), DefaultReal(-20.0)];
        let got = root_solute_uptake(&state, true).unwrap().unwrap();
        // Node 1 only: `SinkS(1)` is the `191` passive product plus the `209` compensated
        // active term (`SinkS` is cleared once, `174-176`), evaluated in source order.
        let passive = -400.0_f32 * 1.5;
        let spu = passive * 5.0;
        let sa = (0.0_f32 * 0.8 - spu).max(0.0);
        let omega = 2.0_f32 / (1.0 + 2.0) * 1.0 * sa * 5.0;
        let omega1 = omega / sa;
        let compen = if omega1 >= 0.5 { omega1 } else { 0.5 };
        let active = 2.0_f32 / (1.0 + 2.0) * 1.0 * sa / compen;
        assert_eq!(got.solute_sink[0], DefaultReal(passive + active));
        assert_eq!(got.solute_sink[1], DefaultReal(0.0));
    }

    /// One node would make `182-183` read `x(0)`, and mismatched node arrays would read
    /// past the bundle. Both are contract rejections raised before any output exists.
    #[test]
    fn out_of_bounds_reads_are_rejected_before_any_output() {
        let mut state = uptake();
        state.coordinates.truncate(1);
        assert!(root_solute_uptake(&state, true)
            .unwrap_err()
            .contains("at least two nodes"));
        state = uptake();
        state.root_distribution.pop();
        assert!(root_solute_uptake(&state, true)
            .unwrap_err()
            .contains("must all match coordinates"));
        state = uptake();
        state.species = 0;
        assert!(root_solute_uptake(&state, true)
            .unwrap_err()
            .contains("one-based"));
        state.species = 2;
        assert!(root_solute_uptake(&state, true)
            .unwrap_err()
            .contains("within NS"));
        state = uptake();
        state.species_count = 0;
        assert!(root_solute_uptake(&state, true).unwrap_err().contains("NS"));
        state = uptake();
        state.time = DoubleReal(f64::NAN);
        assert!(root_solute_uptake(&state, true)
            .unwrap_err()
            .contains("finite"));
    }

    /// The unit-78 row belongs to the last step of the last species only, so an
    /// intermediate species produces the sink and no record at all.
    #[test]
    fn profile_row_is_emitted_only_for_the_last_species() {
        let mut state = uptake();
        state.species_count = 2;
        state.species = 1;
        state.active_uptake = true;
        state.potential_uptake = DefaultReal(50.0);
        let got = root_solute_uptake(&state, true).unwrap().unwrap();
        assert_eq!(got.solute_sink.len(), 3);
        assert!(got.profile_record.is_none());
        state.species = 2;
        assert!(root_solute_uptake(&state, true)
            .unwrap()
            .unwrap()
            .profile_record
            .is_some());
    }

    /// `HYDRUS.FOR:390` is `if(lBact.and..not.lWat)`: any other combination means no
    /// call, and the caller keeps its own four node arrays and `ChPar(4,1)`.
    #[test]
    fn exclusion_guard_requires_bacteria_without_water_flow() {
        let state = exclusion();
        assert_eq!(size_exclusion(&state, true, true).unwrap(), None);
        assert_eq!(size_exclusion(&state, false, false).unwrap(), None);
        assert_eq!(size_exclusion(&state, false, true).unwrap(), None);
        assert!(matches!(
            size_exclusion(&state, true, false),
            Ok(Some(SizeExclusion::Applied { .. }))
        ));
    }

    /// `1723` returns before `1724`, so a negligible `ChPar(4,1)` leaves even the
    /// `ChPar` slot untouched — the one difference between the two inactive paths.
    #[test]
    fn negligible_colloid_content_returns_before_the_ch_par_write() {
        let mut state = exclusion();
        // `1723` compares `abs(th_c)` against `1.e-20`, a *single-precision* literal
        // under default typing, i.e. 9.999999682655225e-21. That threshold value is
        // therefore *not* strictly less than itself and falls through to the `1724`
        // write, while one step down takes the early return. `DefaultReal` carries an
        // `f64` narrowed to `f32`, so the magnitudes below are picked so that the f32
        // value stays strictly on the stated side of the threshold: 1e-21 is exactly half
        // the threshold in f32. The threshold leg then uses the f32 threshold value
        // itself, which compares equal to itself and so falls through.
        state.colloid_exclusion_water_content = DefaultReal(-1.0e-21);
        let got = size_exclusion(&state, true, false).unwrap().unwrap();
        assert!(got.is_inactive());
        match got {
            SizeExclusion::Inactive {
                colloid_exclusion_water_content,
            } => assert_eq!(colloid_exclusion_water_content, DefaultReal(-1.0e-21)),
            _ => unreachable!(),
        }
        // The threshold value itself is *not* strictly below the threshold, so `1723`
        // falls through. At this magnitude `sw_c_eff` lands near -0.111 and `1751` takes a
        // fractional power of a negative base, so the routine ends in the non-finite
        // rejection: the observable difference from the leg above is the *kind* of outcome,
        // an error rather than the inactive return, which is what the guard proves.
        state.colloid_exclusion_water_content = DefaultReal(1.0e-20_f32);
        assert!(size_exclusion(&state, true, false).is_err());
        // `1723` tests `abs(th_c)`, so a negative value of the same magnitude takes the
        // identical early return - and takes it *before* the `1724` write.
        state.colloid_exclusion_water_content = DefaultReal(-1.0e-21);
        assert!(size_exclusion(&state, true, false)
            .unwrap()
            .unwrap()
            .is_inactive());
    }

    /// The applied case: `ThC` and `VelC` come from node 1 only and reach every node,
    /// `ChPar(4,1)` becomes zero, and `sw_c` outside `0..1` is reported without
    /// clamping, exactly as the source continues after its console prompt.
    #[test]
    fn applied_exclusion_broadcasts_node_one_values_and_zeroes_ch_par() {
        let got = size_exclusion(&exclusion(), true, false).unwrap().unwrap();
        let SizeExclusion::Applied {
            new_water,
            old_water,
            new_velocity,
            old_velocity,
            colloid_exclusion_water_content,
            size_exclusion_warning,
        } = &got
        else {
            unreachable!()
        };
        // `ThC` (`1742`) is `ThNew(1)-ths*sw_c`, evaluated in double and narrowed once
        // into the default REAL `ThC`, then broadcast unchanged to every node by
        // `1772-1777`. The algebraically identical `ThNew(1) - swr*ths` (`1727`/`1728`)
        // is not the source text. A fully single-precision `ThNew(1) - th_c` happens to
        // give the same value on this deck, so this fixture does not separate the forms.
        let th_c = DefaultReal({
            let (th_new, ths, card) = (0.6_f32, 1.0_f32, 0.2_f32);
            let sw_c = f64::from(card) / f64::from(ths); // `1728`, a true double
            (f64::from(th_new) - f64::from(ths) * sw_c) as f32 // `1742`, narrowed once
        });
        // One ULP above the decimal 0.4, and equal to the single-precision rewrite
        // `0.6f32 - 0.2f32` (0.40000004). The double subtraction
        // `f64(0.6f32) - f64(0.2f32)` lands on 0.40000002086162567, exactly halfway
        // between the two neighbouring f32 values, so the narrowing breaks the tie to even
        // and picks the upper one. An expectation written from the decimal literal 0.4 is
        // wrong.
        assert_ne!(th_c, DefaultReal(0.4f32));
        assert_eq!(new_water, &vec![th_c; 3]);
        assert_eq!(old_water, new_water);
        // `1744-1746`. `sw_eff` is positive here (0.555555580...), which the `1756` guard
        // needs, and `sw_eff > sw_c_eff`, which the `1749` guard needs, so `kr_c` takes
        // the `1750-1751` expression rather than the `1753` zero. Both `/2.` exponents are
        // integer and `1751` runs on from `1750`, so both differences are live.
        let sw_r = f64::from(0.1_f32) / f64::from(1.0_f32);
        let sw_c = f64::from(0.2_f32) / f64::from(1.0_f32);
        let sw = f64::from(0.6_f32) / f64::from(1.0_f32);
        let sw_eff = (sw - sw_r) / (1.0 - sw_r);
        let sw_c_eff = (sw_c - sw_r) / (1.0 - sw_r);
        let vgn_1 = f64::from(2.0_f32);
        let vgm_1 = 1.0 - 1.0 / vgn_1;
        let vgm_2 = 1.0 - 2.0 / (vgn_1 + 1.0);
        let kr_c = sw_eff
            * sw_eff
            * ((1.0 - sw_c_eff.powf(1.0 / vgm_2)).powf(vgm_2)
                - (1.0 - sw_eff.powf(1.0 / vgm_2)).powf(vgm_2));
        let outer = 1.0 - (1.0 - sw_eff.powf(1.0 / vgm_1)).powf(vgm_1);
        let kr_w = sw_eff.powf(f64::from(0.5_f32)) * outer * outer;
        assert!(kr_c > 0.0 && kr_w > 0.0);
        // `1759`'s `vNew(1)*kr_c/krw` is one left-to-right `/`-and-`*` chain on the
        // double side, narrowed once into the default REAL `VelC`.
        let vel_c = ((-0.25_f64) * kr_c / kr_w) as f32;
        assert_eq!(new_velocity, &vec![DefaultReal(vel_c); 3]);
        assert_eq!(old_velocity, new_velocity);
        assert_eq!(*colloid_exclusion_water_content, DefaultReal(0.0));
        assert!(!size_exclusion_warning);
    }

    /// `1735-1739` only warns - it never clamps `sw_c` or leaves the routine - but what
    /// the warning was *about* decides whether anything usable comes out the other side.
    /// Both out-of-range directions push `sw_c_eff` (`1746`) outside `0..1`, and
    /// `1751`'s fractional powers of a negative base are NaN on every Fortran runtime
    /// this side of a complex extension: a negative `th_c` gives `sw_c_eff < 0`, and a
    /// `th_c` above `thr` gives `kr_c == 0.0` from the `1749` guard times `1/0.0` from
    /// `1751`'s `(1-1)^-1.5`, i.e. `0 * inf`. The port rejects both as non-finite rather
    /// than broadcasting the NaN, which is exactly the difference between a simulation
    /// that stops here and one that loses `ThNew`, `ThOld`, `vNew` and `vOld` to NaN.
    #[test]
    fn out_of_range_sw_c_warns_and_the_nonphysical_branch_rejects() {
        let mut state = exclusion();
        // Below the residual: sw_c < 0 trips the warning, sw_c_eff = -0.0555... is
        // negative, and `1751` evaluates a negative base to a fractional exponent.
        state.colloid_exclusion_water_content = DefaultReal(-0.2);
        let err = size_exclusion(&state, true, false).unwrap_err();
        assert!(err.contains("1742/1759"), "{err}");
        // Above the saturated value: sw_c > 1 trips the same warning, `1749`'s guard
        // *fails* so `kr_c` takes the `1753` zero, and `1759` is then
        // `vNew(1) * 0.0 / 0.0211...` - zero, not NaN. The exclusion therefore "works"
        // and broadcasts a water content of `ThNew(1) - th_c = -0.6`, i.e. a negative
        // water profile into `ThNew` and `ThOld`, with a zero velocity. That is the shape
        // of the failure: `1735` warns, the user confirms, and the run continues with a
        // physically impossible profile rather than stopping.
        state.colloid_exclusion_water_content = DefaultReal(1.2);
        let again = size_exclusion(&state, true, false).unwrap().unwrap();
        let SizeExclusion::Applied {
            new_water,
            new_velocity,
            size_exclusion_warning,
            ..
        } = &again
        else {
            unreachable!("an out-of-range sw_c is still an applied exclusion")
        };
        assert!(*size_exclusion_warning);
        assert_eq!(new_water, &vec![DefaultReal(-0.6f32); 3]);
        assert_eq!(new_velocity, &vec![DefaultReal(0.0); 3]);
        // The warning is a property of the `1735` guard alone: an in-range `th_c` clears
        // it and a finite result comes out, so the port reports a warning exactly where
        // the source would have prompted and nowhere else.
        state.colloid_exclusion_water_content = DefaultReal(0.1);
        let ok = size_exclusion(&state, true, false).unwrap().unwrap();
        assert!(matches!(
            ok,
            SizeExclusion::Applied {
                size_exclusion_warning: false,
                ..
            }
        ));
    }

    /// `sw_eff.le.0.` takes `1756-1763`'s `else`: `krw=0.` and `VelC=vNew(1)`, a *direct*
    /// assignment. Hoisting `1759`'s division above the branch instead - the shape a
    /// `{ krw, VelC } = if ... else ...` refactor produces - divides by that `0.0` and
    /// writes NaN into `vNew` and `vOld` for a profile the source passes through
    /// untouched, which is the difference between a run that stops and one that quietly
    /// continues with a NaN velocity field.
    #[test]
    fn zero_effective_saturation_assigns_the_velocity_without_dividing() {
        let mut state = exclusion();
        // sw == thr exactly, so `sw_eff = (sw - swr)/(1 - swr) = 0`.
        state.first_node_new_water = DefaultReal(0.1);
        state.colloid_exclusion_water_content = DefaultReal(0.05);
        let got = size_exclusion(&state, true, false).unwrap().unwrap();
        let SizeExclusion::Applied {
            new_velocity,
            new_water,
            old_velocity,
            size_exclusion_warning,
            ..
        } = &got
        else {
            unreachable!()
        };
        // vNew(1) verbatim, not the `vNew(1) * kr_c / 0.0` the hoisted form yields.
        assert_eq!(*new_velocity, vec![DefaultReal(-0.25f32); 3]);
        assert_eq!(old_velocity, new_velocity);
        // `ThC = ThNew(1) - ths * sw_c` is unaffected by the branch: both operands are
        // exact here, so this is one of the few cases where the decimal value is right.
        assert_eq!(*new_water, vec![DefaultReal(0.05f32); 3]);
        assert!(!size_exclusion_warning);
    }

    /// `1727` and the `1.0-swr` divisions of `1745-1746` are left as infinities by the
    /// source; they must not reach every node of four arrays here.
    #[test]
    fn degenerate_hydraulic_parameters_are_rejected() {
        let mut state = exclusion();
        state.mobile_saturated_water = DefaultReal(0.0);
        assert!(size_exclusion(&state, true, false)
            .unwrap_err()
            .contains("above Par(1,1)"));
        state = exclusion();
        state.mobile_residual_water = DefaultReal(1.0);
        state.mobile_saturated_water = DefaultReal(1.0);
        // swr == 1 makes both `1.0-swr` divisions infinities.
        assert!(size_exclusion(&state, true, false).is_err());
        state = exclusion();
        state.first_node_new_velocity = DefaultReal(f32::NAN);
        assert!(size_exclusion(&state, true, false)
            .unwrap_err()
            .contains("finite"));
        state = exclusion();
        state.nodes = 0;
        assert!(size_exclusion(&state, true, false)
            .unwrap_err()
            .contains("at least one node"));
    }

    /// The dead region `1765-1769` must stay dead: `SizeExclusion` deliberately carries
    /// no `Pc_c`, `r_c`, `PorVelSolute`, `PorVelColloid` or `alpha`, so the only
    /// observable outputs are the four node arrays and the `ChPar` slot.
    #[test]
    fn applied_exclusion_carries_only_the_live_outputs() {
        let got = size_exclusion(&exclusion(), true, false).unwrap().unwrap();
        let SizeExclusion::Applied {
            new_water,
            old_water,
            new_velocity,
            old_velocity,
            ..
        } = &got
        else {
            unreachable!()
        };
        // Four node arrays, each NumNP long, and nothing else the source writes.
        assert_eq!(new_water.len(), 3);
        assert_eq!(old_water.len(), 3);
        assert_eq!(new_velocity.len(), 3);
        assert_eq!(old_velocity.len(), 3);
        // A single distinct value per array is the 1772-1777 broadcast.
        assert!(new_water.windows(2).all(|w| w[0] == w[1]));
        assert!(new_velocity.windows(2).all(|w| w[0] == w[1]));
    }
}

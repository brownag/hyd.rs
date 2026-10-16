//! Active dynamic dual-porosity water exchange, `subroutine DualPor`
//! (`WATFLOW.FOR:1027-1075`).
//!
//! The call is live, not commented: `WATFLOW.FOR:43-46` reads
//! `if(iDualPor.gt.0)` then, on the column-6 continuations,
//! `call DualPor(NumNP,NMat,MatNum,iDualPor,ThOld,ths,thr,ThNewIm,`
//! `ThOldIm,ParD,SinkIm,dt,iModel,hNew,hCritA,x,WTransf)`, with columns 1-5 of
//! 44-46 blank. The routine's own argument list (`1027-1029`) is positionally
//! identical, including `ths` ahead of `thr` at positions 6 and 7, so each
//! dummy associates with the same-named actual; keep that order when editing.
//! It runs once per `WatFlow` call before `Iter=0` (47), so the
//! iteration retries at `goto 12` (577) and `goto 11` (650) never re-run it.
//! Consequently `DualPor` reads `ThOld`, never `ThNew`, and `ThOldIm`, never
//! `ThNewIm`: the copy `ThOldIm(i)=ThNewIm(i)` is `Update`'s
//! (`WATFLOW.FOR:1112`). That timing is why [`DualPorPrimitiveState`] carries
//! `old_water`/`old_immobile_water` as inputs and [`DualPorExchange`] returns the
//! new immobile water instead of writing through the state.
//!
//! Every quantity the routine touches is default REAL: the caller declares
//! `SinkIm`, `ThOld`, `ThOldIm`, `ThNewIm`, `dt`, `x`, `hNew` and `WTransf` in the
//! `REAL*4` list (`4-11`, `33-41`) and `DualPor` declares its own locals (`Se`,
//! `SeIm`, `DeltaTh`, `TrMaxIm`, `h`, `CondM`, `CondF`, `Par`, `i`, `N`, `M`) in
//! the same list (`1021-1024`), so the whole exchange stays in [`DefaultReal`]
//! precision. Only `FH` and `FK` widen, and they widen under their own
//! `implicit double precision (A-H,O-Z)` (`MATERIAL.FOR:4`, `350`) while their
//! arguments stay default REAL (`real h,Par(10)`, `real Qe,Par(10)`), so this
//! module reproduces that split by widening at the arguments and narrowing the
//! result with `sngl()` - `as f32`.
//!
//! `thr`/`ths` are not free primitives: `GenMat` assigns `thr(M)=Par(1,M)` and
//! `ths(M)=Par(2,M)` (`INPUT.FOR:796-797`, identically in the table branch at
//! `816-817`), so the exchange derives them from the same `ParD` column the deck
//! supplies. `GenMat` is defined inside the
//! reader's own file (`INPUT.FOR:762`; there is no separate `GENMAT.FOR`) and runs
//! at `HYDRUS.FOR:219`. For a dual-porosity deck it takes the analytic branch of
//! `791-809`:
//! `iDualPor` is only ever set for water models 6 and 7 (`INPUT.FOR:619-624`),
//! which the reader remaps to `iModel=0`, and `0 < nTabMod` (`INPUT.FOR:1685`,
//! `nTabMod=10`).

//!
//! # Solute-side dual-domain provenance (task 18.9.3)
//!
//! `Coeff` (`SOLUTE.FOR:286-289`, `306-308`) takes the immobile water from the
//! node arrays `ThNIm`/`ThOIm` when `iDualPor>0`, not from `ChPar(4,M)`, and the
//! neighbour `Thj` is not reduced by `ThImob` (`294-295`, `313-314` need
//! `iDualPor.eq.0`). [`dual_domain_th_imob`], [`dual_material_selection`] and
//! [`dual_neighbor_theta`] carry exactly that. The g/q gating on `SinkIm`/`FlMacro`
//! lives in `processes::source_terms`, `FlMacro` in `nonequilibrium` and `processes`,
//! and the `Disper` `ThS` rule in `transport::coefficients`; none is repeated here.
//!
//! External-state gap (for 18.10.1): the old-level `ThOIm` is `Update`'s copy
//! `ThOldIm=ThNewIm` (`WATFLOW.FOR:1112`) and `SinkIm` is written by `DualPor`
//! during the water step; the solute step reads both, so the driver must carry them
//! between the water and solute steps. [`super::types::DualPorPrimitiveState`] holds
//! neither as a persisted per-node history.

use super::types::{DefaultReal, DualPorPrimitiveState, KernelError};

/// `FH`'s and `FK`'s `Par` actual argument, i.e. one `ParD` column.
///
/// `FH(iModel,Qe,Par)` declares `real Par(10)` (`MATERIAL.FOR:350`), so the
/// element-associated actual argument `ParD(7,M)` of `WATFLOW.FOR:1044` aliases
/// `Par(1..5) = ParD(7..11,M)` with no explicit shuffle; its `iModel=0` path reads
/// only `Par(1..4)` (`MATERIAL.FOR:356-359`), so it never reaches `ParD(11,M)` and
/// never runs past column `M` - and neither would be observable, because both
/// functions read the caller's own array. `FK` instead receives the explicit local
/// assembly of `WATFLOW.FOR:1045-1050`, whose `Par(1..6)` are those same four
/// slots plus `ParD(11,M)` as `Ks` and `ParD(6,M)` as `BPar`. Both slot sets are
/// disjoint from the `iDualPor=1` set, which reads only `ParD(9,M)` (`1040`).
#[derive(Clone, Copy)]
struct ImmobilePar {
    /// `Qr` / `Par(1)`, `ParD(7,M)`.
    q_r: f32,
    /// `Qs` / `Par(2)`, `ParD(8,M)`.
    q_s: f32,
    /// `Alfa` / `Par(3)`, `ParD(9,M)`.
    alfa: f32,
    /// `n` / `Par(4)`, `ParD(10,M)`.
    n: f32,
    /// `FK`'s `Ks` input / `Par(5)`, `ParD(11,M)`.
    k_s: f32,
    /// `FK`'s `BPar` input / `Par(6)`, `ParD(6,M)`.
    b_par: f32,
}

/// One water step's dual-porosity exchange, the three live outputs of `DualPor`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DualPorExchange {
    /// `SinkIm` (`1027`, written at `1040`/`1053`, clamped at `1057-1066`), a
    /// per-node water exchange-rate density. It is not a boundary flux: `WatFlow`
    /// adds it to the mobile node equation as a source (`223`, `255`, `286`,
    /// `402`), and `NEquil`/`MassTran` scale it by concentration for the immobile
    /// solute equation (`1095-1102`, `1353-1360`). Positive is mobile to immobile.
    pub sink_immobile: Vec<DefaultReal>,
    /// `ThNewIm` (`1068`), the immobile water content at the end of the call.
    /// `WatFlow` never reads it; `Update` copies it to `ThOldIm` for the next step
    /// (`1112`).
    pub new_immobile_water: Vec<DefaultReal>,
    /// `WTransf` (`1034`, `1070-1071`), the trapezoidal integral of `SinkIm` over
    /// `x`, which `Output` accumulates into `CumQ(11)` (`OUTPUT.FOR:81`). Units
    /// are `SinkIm` times length, not a flux.
    pub transfer: DefaultReal,
}

/// `subroutine DualPor` for one water step (`WATFLOW.FOR:1027-1075`).
pub(crate) fn dual_porosity_exchange(
    state: &DualPorPrimitiveState,
) -> Result<DualPorExchange, KernelError> {
    validate(state)?;
    let nodes = state.coordinates.len();
    let dt = state.dt.0;
    let h_crit_a = state.critical_atmospheric_head.0;
    let mut exchange = DualPorExchange {
        sink_immobile: Vec::with_capacity(nodes),
        new_immobile_water: Vec::with_capacity(nodes),
        transfer: DefaultReal(0.0),
    };
    // `WTransf=0.` (1034), accumulated below in node order.
    let mut transfer = 0.0_f32;
    for i in 0..nodes {
        let material_id = *state
            .material_ids
            .get(i)
            .ok_or("dual-porosity material map is shorter than the node arrays")?;
        let material = state
            .materials
            .get(
                material_id
                    .checked_sub(1)
                    .ok_or("MatNum must be one-based")?,
            )
            .ok_or("MatNum must index the ParD columns")?;
        // thr(M) and ths(M), i.e. ParD(1,M) and ParD(2,M) through GenMat.
        let thr = material.mobile_residual_water.0;
        let ths = material.mobile_saturated_water.0;
        let par7 = material.immobile_residual_water.0;
        let par8 = material.immobile_saturated_water.0;
        let par9 = material.immobile_alpha.0;
        let th_old = state.old_water[i].0;
        let th_old_im = state.old_immobile_water[i].0;
        let h_new = state.new_pressure_head[i].0;
        // `SeIm=min(1.,(ThOldIm(i)-ParD(7,M))/(ParD(8,M)-ParD(7,M)))` (1037).
        let se_im = 1.0f32.min((th_old_im - par7) / (par8 - par7));
        // `DeltaTh` exists only on the `iDualPor=1` path (1041-1042), and both
        // clamps test `iDualPor.eq.1` before using it (1059, 1064).
        let mut delta_th = None;
        let mut sink = if state.dual_porosity == 1 {
            // `Se=min(1.,(ThOld(i)-thr(M))/(ths(M)-thr(M)))` (1039), then
            // `SinkIm(i)=ParD(9,M)*(Se-SeIm)` (1040).
            let se = 1.0f32.min((th_old - thr) / (ths - thr));
            let sink = par9 * (se - se_im);
            // `DeltaTh=(Se-SeIm)/(ths(M)-thr(M)+ParD(8,M)-ParD(7,M))*`
            // `(ths(M)-thr(M))*(ParD(8,M)-ParD(7,M))` (1041-1042). Fortran
            // evaluates same-precedence operators left to right and so does Rust,
            // so the denominator and the two trailing products group as written.
            delta_th = Some((se - se_im) / (ths - thr + par8 - par7) * (ths - thr) * (par8 - par7));
            sink
        } else {
            let par = ImmobilePar {
                q_r: par7,
                q_s: par8,
                alfa: par9,
                n: material
                    .immobile_n
                    .ok_or("iDualPor=2 needs ParD(10,M), the immobile n")?
                    .0,
                k_s: material
                    .immobile_saturated_conductivity
                    .ok_or("iDualPor=2 needs ParD(11,M), the immobile Ksat")?
                    .0,
                b_par: material
                    .mobile_tortuosity
                    .ok_or("iDualPor=2 needs ParD(6,M), the mobile path-length term")?
                    .0,
            };
            // `h=FH(iModel,SeIm,ParD(7,M))` (1044), `CondM=FK(iModel,h,Par)`
            // (1051), `CondF=FK(iModel,hNew(i),Par)` (1052), and
            // `SinkIm(i)=0.5*(CondM+CondF)*(hNew(i)-h)` (1053).
            let h = retention_head(DefaultReal(se_im), &par);
            let cond_m = conductivity(DefaultReal(h), &par);
            let cond_f = conductivity(DefaultReal(h_new), &par);
            let mut sink = 0.5 * (cond_m + cond_f) * (h_new - h);
            // The guard requires `i.eq.N`, so `hNew(i)` and `hNew(N)` are the same
            // value; `hCritA` is negative, hence the negated literals (1054-1055).
            if i + 1 == nodes
                && (h_crit_a - h_new).abs() < -0.001 * h_crit_a
                && (h_crit_a - h).abs() < -0.01 * h_crit_a
            {
                sink = 0.0;
            }
            sink
        };
        // `if(SinkIm(i).gt.0.)` (1057-1061): cap the mobile-to-immobile rate by the
        // room the immobile region still has and, for `iDualPor=1`, by the water
        // the step can move between the two regions.
        if sink > 0.0 {
            let mut tr_max_im = (par8 - th_old_im) / dt;
            if let Some(delta_th) = delta_th {
                tr_max_im = (delta_th / dt).min(tr_max_im);
            }
            if sink > tr_max_im {
                sink = tr_max_im;
            }
        }
        // `if(SinkIm(i).lt.0.)` (1062-1066): cap the reverse rate by the water the
        // mobile region can still receive.
        if sink < 0.0 {
            let mut tr_max_im = -((ths - th_old) / dt);
            if let Some(delta_th) = delta_th {
                tr_max_im = (delta_th / dt).max(tr_max_im);
            }
            if sink < tr_max_im {
                sink = tr_max_im;
            }
        }
        // `ThNewIm(i)=max(min(ThOldIm(i)+SinkIm(i)*dt,ParD(8,M)),ParD(7,M))` (1068).
        let th_new_im = (th_old_im + sink * dt).min(par8).max(par7);
        // `if(i.ge.2) WTransf=WTransf+(SinkIm(i-1)+SinkIm(i))/2.*`
        // `(x(i)-x(i-1))` (1070-1071). Continuation record 1071 carries `!` in
        // column 6, so the statement is live: the trapezoid of the two node rates
        // over the node spacing goes into one default-REAL accumulator, and the
        // `i.ge.2` guard leaves the first node out of the integral.
        if i >= 1 {
            let previous = exchange.sink_immobile[i - 1].0;
            let dx = state.coordinates[i].0 - state.coordinates[i - 1].0;
            transfer += (previous + sink) / 2.0 * dx;
        }
        exchange.sink_immobile.push(DefaultReal(sink));
        exchange.new_immobile_water.push(DefaultReal(th_new_im));
    }
    exchange.transfer = DefaultReal(transfer);
    Ok(exchange)
}

/// `FH(iModel,Qe,Par)` for `iModel=0`, the only model an active call can carry
/// (`MATERIAL.FOR:355-370`).
///
/// `implicit double precision (A-H,O-Z)` makes every local double while `Qe` and
/// `Par` stay default REAL (`350`), and the result is `sngl()`-narrowed (`369`);
/// `Qm=Qs` and `Qa=Qr` come from the `iModel` branches that fall through for 0
/// (`355-363`). `HMin=-1.d300**(1.d0/n)/max(Alfa,1.d0)` (`367`) parses as
/// `-(1d300**(1/n))/max(Alfa,1d0)`, because `**` outranks unary minus.
/// Intentionally not shared with `hydraulics`; see ADR-0015.
fn retention_head(qe: DefaultReal, par: &ImmobilePar) -> f32 {
    let qe = f64::from(qe.0);
    let q_r = f64::from(par.q_r);
    let q_s = f64::from(par.q_s);
    let alfa = f64::from(par.alfa);
    let n = f64::from(par.n);
    let (qm, qa) = (q_s, q_r);
    let m = 1.0 - 1.0 / n;
    let h_min = -(1.0e300_f64).powf(1.0 / n) / alfa.max(1.0);
    let qee_m = (1.0 + (-alfa * h_min).powf(n)).powf(-m);
    // `Qee=dmin1(dmax1(Qe*(Qs-Qa)/(Qm-Qa),QeeM),.999999999999999d0)` (368).
    let qee = (qe * (q_s - qa) / (qm - qa))
        .max(qee_m)
        .min(0.999_999_999_999_999);
    // `FH=sngl(max(-1.d0/Alfa*(Qee**(-1.d0/m)-1.d0)**(1.d0/n),-1.d37))` (369).
    ((-1.0 / alfa) * (qee.powf(-1.0 / m) - 1.0).powf(1.0 / n)).max(-1.0e37) as f32
}

/// `FK(iModel,h,Par)` for `iModel=0`, the only model an active call can carry
/// (`MATERIAL.FOR:24-37`, `55-79`, `144-148`).
///
/// Same precision split as [`retention_head`]: `implicit double precision
/// (A-H,O-Z)` with `real h,Par(10)` (`20`) and `double precision ... Ks,Kr,Kk`
/// (`19`). `Qk=Qs` and `Kk=Ks` (`36-37`) make `Qek`'s and `Qeek`'s expressions
/// equal `Qees`' bitwise, so `Hk` equals `Hs` and the linear band of `144-146` is
/// empty. `67`, `144` and `148` are sequential assignments to the `FK` result, so a
/// later one overwrites an earlier one; with `Hk == Hs` exactly one of `67` and
/// `148` ever fires. `PPar=2` (`32`).
/// Intentionally not shared with `hydraulics`; see ADR-0015.
fn conductivity(head: DefaultReal, par: &ImmobilePar) -> f32 {
    let h = f64::from(head.0);
    let q_r = f64::from(par.q_r);
    let q_s = f64::from(par.q_s);
    let alfa = f64::from(par.alfa);
    let n = f64::from(par.n);
    // `Ks=amax1(Par(5),1.e-37)` (28): a default-REAL maximum, then widened.
    let k_s = f64::from(par.k_s.max(1.0e-37_f32));
    let b_par = f64::from(par.b_par);
    let (qm, qa, qk, kk) = (q_s, q_r, q_s, k_s);
    let m = 1.0 - 1.0 / n;
    let h_min = -(1.0e300_f64).powf(1.0 / n) / alfa.max(1.0);
    let hh = h.max(h_min);
    let qees = ((q_s - qa) / (qm - qa)).min(0.999_999_999_999_999);
    let qeek = ((qk - qa) / (qm - qa)).min(qees);
    let hs = -1.0 / alfa * (qees.powf(-1.0 / m) - 1.0).powf(1.0 / n);
    let hk = -1.0 / alfa * (qeek.powf(-1.0 / m) - 1.0).powf(1.0 / n);
    if h < hk {
        // `Qe=(Qm-Qa)/(Qs-Qa)*Qee` (70); `FFQ` (72) falls back to `m*Qee**(1/m)`
        // whenever it is not positive (74).
        let qee = (1.0 + (-alfa * hh).powf(n)).powf(-m);
        let qe = (qm - qa) / (q_s - qa) * qee;
        let ffq = 1.0 - (1.0 - qee.powf(1.0 / m)).powf(m);
        let ffq = if ffq <= 0.0 {
            m * qee.powf(1.0 / m)
        } else {
            ffq
        };
        // Line 75 is the `iModel=1`/`3` form of `Kr`, which line 76 overwrites for
        // `iModel=0` before the first `FK` assignment at 79.
        let k_r = qe.powf(b_par) * ffq.powf(2.0);
        // `FK=sngl(max(Ks*Kr,1.d-37))` (79): a double maximum this time.
        (k_s * k_r).max(1.0e-37) as f32
    } else if h >= hs {
        // `FK=sngl(Ks)` (148).
        k_s as f32
    } else {
        // `FK=sngl(Ks*Kr)` (144-146). Unreachable for `iModel=0`, but written out
        // so that the three source statements appear in source order.
        let k_r = (1.0 - kk / k_s) / (hs - hk) * (h - hs) + 1.0;
        (k_s * k_r) as f32
    }
}

/// Rejects what the port cannot represent instead of returning a NaN.
///
/// The source validates nothing, so the material-shape rejections below -
/// `ParD(8,M)` at or below `ParD(7,M)`, the mobile pair, and the `iDualPor=2`
/// retention triple - are contract rejections rather than source statements. Each
/// guards a division or a fractional power that `1037`/`1039`, `FH` or `FK` would
/// otherwise carry into `SinkIm`, and they mirror the predicates
/// `hydraulics::HydraulicModel::validate` already applies to the mobile-domain
/// pair.
pub(super) fn validate(state: &DualPorPrimitiveState) -> Result<(), KernelError> {
    let nodes = state.coordinates.len();
    if nodes == 0 {
        return Err("dual-porosity exchange needs at least one node");
    }
    if state.old_water.len() != nodes
        || state.new_pressure_head.len() != nodes
        || state.old_immobile_water.len() != nodes
        || state.material_ids.len() != nodes
    {
        return Err("dual-porosity exchange node arrays must all be the same length");
    }
    // `43` fires on `iDualPor.gt.0`, and the reader assigns 1 or 2 only
    // (`INPUT.FOR:620`, `624`), so no other value reaches the routine.
    if !matches!(state.dual_porosity, 1 | 2) {
        return Err("dual-porosity exchange requires iDualPor 1 or 2");
    }
    // Models 6 and 7 are remapped to `iModel=0` before `ParD` is read
    // (`INPUT.FOR:618-624`) and `iDualPor` is never set for another model, so an
    // active call always carries 0.
    if state.hydraulic_model != 0 {
        return Err("dual-porosity exchange is only reachable with iModel 0");
    }
    // `1059` and `1064` divide `DeltaTh`, `TrMaxIm` and `SinkIm*dt` by `dt`
    // (`TimeDt` is a summed time increment, never zero in a running cycle).
    // Finiteness is written out because `dt` is only screened for finiteness
    // here, unlike the node arrays below.
    if !state.dt.0.is_finite() || state.dt.0 <= 0.0 {
        return Err("dual-porosity exchange needs a positive time step");
    }
    if !state.critical_atmospheric_head.0.is_finite() {
        return Err("dual-porosity exchange needs a finite hCritA");
    }
    if state.materials.is_empty() {
        return Err("dual-porosity exchange needs at least one ParD column");
    }
    let mut previous_x = None;
    for index in 0..nodes {
        for (value, message) in [
            (
                state.coordinates[index].0,
                "dual-porosity node coordinates must be finite",
            ),
            (
                state.old_water[index].0,
                "dual-porosity ThOld must be finite",
            ),
            (
                state.new_pressure_head[index].0,
                "dual-porosity hNew must be finite",
            ),
            (
                state.old_immobile_water[index].0,
                "dual-porosity ThOldIm must be finite",
            ),
        ] {
            if !value.is_finite() {
                return Err(message);
            }
        }
        let x = state.coordinates[index].0;
        // `1071` integrates over `x(i)-x(i-1)`, and `NodInf` builds `x` increasing;
        // a non-increasing grid is rejected rather than silently negating the
        // integral.
        if let Some(previous_x) = previous_x {
            // The coordinates of this node were proved finite just above, so a
            // plain `<=` still rejects every value that is not an increase.
            if x <= previous_x {
                return Err("dual-porosity node coordinates must increase with the node index");
            }
        }
        previous_x = Some(x);
        let material_id = state.material_ids[index];
        if material_id == 0 || material_id > state.materials.len() {
            return Err("MatNum must index the ParD columns");
        }
    }
    for material in &state.materials {
        let slots = [
            Some(material.mobile_residual_water),
            Some(material.mobile_saturated_water),
            material.mobile_tortuosity,
            Some(material.immobile_residual_water),
            Some(material.immobile_saturated_water),
            Some(material.immobile_alpha),
            material.immobile_n,
            material.immobile_saturated_conductivity,
        ];
        if slots
            .into_iter()
            .flatten()
            .any(|value| !value.0.is_finite())
        {
            return Err("dual-porosity ParD columns must be finite");
        }
        // `1037` divides by the immobile range, `1039` by the mobile one, and `FH`
        // reduces `(Qs-Qa)/(Qm-Qa)` (`MATERIAL.FOR:368`), so each must be positive
        // rather than merely nonzero. The finiteness test above comes first, so
        // each `<=` below also rejects a NaN slot.
        if material.immobile_saturated_water.0 <= material.immobile_residual_water.0 {
            return Err("ParD(8,M) must exceed ParD(7,M)");
        }
        if material.mobile_saturated_water.0 <= material.mobile_residual_water.0 {
            return Err("ParD(2,M) must exceed ParD(1,M)");
        }
        if state.dual_porosity == 2 {
            // `FH` divides by `Alfa` and takes it to a fractional power
            // (`MATERIAL.FOR:367`, `369`), both functions divide by `n-1` through
            // `m`, and `FK` raises a positive `Qe` to `BPar` (`76`).
            if material.immobile_alpha.0 <= 0.0 {
                return Err("ParD(9,M) must be positive when iDualPor=2");
            }
            let n = material
                .immobile_n
                .ok_or("iDualPor=2 needs ParD(10,M), the immobile n")?
                .0;
            if n <= 1.0 {
                return Err("ParD(10,M) must exceed one when iDualPor=2");
            }
            let b_par = material
                .mobile_tortuosity
                .ok_or("iDualPor=2 needs ParD(6,M), the mobile path-length term")?
                .0;
            if b_par < 0.0 {
                return Err("ParD(6,M) must be non-negative when iDualPor=2");
            }
        }
    }
    Ok(())
}

/// Immobile water used by `Coeff` at one node (`SOLUTE.FOR:286-289`, `306-308`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DualThImob {
    /// `ThImob`.
    pub th_imob: f32,
    /// `ThImobO`; assigned only at the new level (`288`), so `None` at the old one.
    pub th_imob_o: Option<f32>,
}

/// `ThImob`/`ThImobO` for `iDualPor>0`; `None` for `iDualPor=0`, which 18.9.2 owns.
pub(crate) fn dual_domain_th_imob(
    new_level: bool,
    i_dual_por: u8,
    bacteria: bool,
    th_new_im: f32,
    th_old_im: f32,
) -> Result<Option<DualThImob>, KernelError> {
    validate_dual_por(i_dual_por)?;
    reject_bacteria_dual(i_dual_por, bacteria)?;
    if i_dual_por == 0 {
        return Ok(None);
    }
    if !th_new_im.is_finite() || !th_old_im.is_finite() {
        return Err("immobile water contents must be finite");
    }
    Ok(Some(if new_level {
        DualThImob {
            th_imob: th_new_im,
            th_imob_o: Some(th_old_im),
        }
    } else {
        DualThImob {
            th_imob: th_old_im,
            th_imob_o: None,
        }
    }))
}

/// `Thj` of the neighbour node (`SOLUTE.FOR:295`, `314`): unchanged for
/// `iDualPor>0`; `None` for `iDualPor=0`, where 18.9.2 may reduce it.
pub(crate) fn dual_neighbor_theta(
    i_dual_por: u8,
    bacteria: bool,
    th_j: f32,
) -> Result<Option<f32>, KernelError> {
    validate_dual_por(i_dual_por)?;
    reject_bacteria_dual(i_dual_por, bacteria)?;
    Ok((i_dual_por > 0).then_some(th_j))
}

/// Zero-based material index per node from one-based `MatNum(i)`.
#[cfg(test)]
pub(crate) fn dual_material_selection(
    material_ids: &[usize],
    material_count: usize,
) -> Result<Vec<usize>, KernelError> {
    material_ids
        .iter()
        .map(|&id| {
            if id == 0 || id > material_count {
                Err("MatNum must be one-based and within the material count")
            } else {
                Ok(id - 1)
            }
        })
        .collect()
}

/// The guard at `SOLUTE.FOR:281/295/302/314` is `lMobIm.and.iDualPor.eq.0.or.lBact`,
/// i.e. `(lMobIm && iDualPor==0) || lBact`. With `lBact` and `iDualPor>0` the
/// source first reduces `ThW`/`Thj` by `ChPar(4,M)` and then overwrites `ThImob`
/// with `ThNIm`/`ThOIm`. That combination is explicitly unsupported (18.9.2's
/// immobile-water rule rejects it too), never silently approximated.
fn reject_bacteria_dual(i_dual_por: u8, bacteria: bool) -> Result<(), KernelError> {
    if bacteria && i_dual_por > 0 {
        return Err("lBact with iDualPor>0 is unsupported");
    }
    Ok(())
}

fn validate_dual_por(i_dual_por: u8) -> Result<(), KernelError> {
    if i_dual_por > 2 {
        return Err("iDualPor must be 0, 1 or 2");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solute_fortran_compat::types::DualPorMaterial;

    /// A `ParD` column in the shape of the `dual_model6` deck row
    /// `0.08 0.43 0.015 1.875 0.43 0.5 0.05 0.25 0.1`.
    fn material6(thr: f32, ths: f32, par7: f32, par8: f32, par9: f32) -> DualPorMaterial {
        DualPorMaterial {
            mobile_residual_water: DefaultReal(thr),
            mobile_saturated_water: DefaultReal(ths),
            mobile_tortuosity: Some(DefaultReal(0.5)),
            immobile_residual_water: DefaultReal(par7),
            immobile_saturated_water: DefaultReal(par8),
            immobile_alpha: DefaultReal(par9),
            immobile_n: None,
            immobile_saturated_conductivity: None,
        }
    }

    /// The `dual_model7` deck row
    /// `0.08 0.43 0.015 1.875 0.43 0.5 0.01 0.12 0.1 1.2 0.01`.
    fn material7() -> DualPorMaterial {
        DualPorMaterial {
            mobile_residual_water: DefaultReal(0.08),
            mobile_saturated_water: DefaultReal(0.43),
            mobile_tortuosity: Some(DefaultReal(0.5)),
            immobile_residual_water: DefaultReal(0.01),
            immobile_saturated_water: DefaultReal(0.12),
            immobile_alpha: DefaultReal(0.1),
            immobile_n: Some(DefaultReal(1.2)),
            immobile_saturated_conductivity: Some(DefaultReal(0.01)),
        }
    }

    fn reals(values: &[f32]) -> Vec<DefaultReal> {
        values.iter().map(|value| DefaultReal(*value)).collect()
    }

    // One deck-shaped fixture builder: the nine arguments are the nine things a
    // caller of `DualPor` has to hand over, so splitting it would only scatter the
    // state across helpers.
    #[allow(clippy::too_many_arguments)]
    fn state(
        coordinates: &[f32],
        old_water: &[f32],
        h_new: &[f32],
        old_immobile: &[f32],
        material_ids: &[usize],
        materials: Vec<DualPorMaterial>,
        h_crit_a: f32,
        dt: f32,
        dual_porosity: u8,
    ) -> DualPorPrimitiveState {
        DualPorPrimitiveState {
            coordinates: reals(coordinates),
            old_water: reals(old_water),
            new_pressure_head: reals(h_new),
            old_immobile_water: reals(old_immobile),
            material_ids: material_ids.to_vec(),
            materials,
            critical_atmospheric_head: DefaultReal(h_crit_a),
            dt: DefaultReal(dt),
            hydraulic_model: 0,
            dual_porosity,
        }
    }

    /// Four nodes on a uniform 0.25 grid, with materials and water contents
    /// chosen so that node 1 hits the positive `1059` cap, node 2 exchanges
    /// nothing, node 3 exchanges freely, and node 4 hits the negative `1064` cap.
    fn water_content_exchange() -> DualPorPrimitiveState {
        state(
            &[0.0, 0.25, 0.5, 0.75],
            &[0.43, 0.45, 0.2, 0.2],
            &[-100.0; 4],
            &[0.125, 0.3, 0.2, 0.2],
            &[3, 2, 1, 3],
            vec![
                material6(0.08, 0.43, 0.05, 0.25, 0.1),
                material6(0.1, 0.45, 0.02, 0.3, 0.05),
                material6(0.08, 0.43, 0.05, 0.25, 5.0),
            ],
            -1.0e10,
            0.1,
            1,
        )
    }

    fn pressure_head_exchange(
        h_crit_a: f32,
        h_new: &[f32; 3],
        old_immobile: &[f32; 3],
    ) -> DualPorPrimitiveState {
        state(
            &[0.0, 0.25, 0.5],
            &[0.3; 3],
            h_new,
            old_immobile,
            &[1; 3],
            vec![material7()],
            h_crit_a,
            0.1,
            2,
        )
    }

    #[test]
    fn retention_head_matches_the_fh_zero_model_path() {
        let par = ImmobilePar {
            q_r: 0.01,
            q_s: 0.12,
            alfa: 0.1,
            n: 1.2,
            k_s: 0.01,
            b_par: 0.5,
        };
        // Transcribed from MATERIAL.FOR:355-370 with the ParD(7..10) slots of the
        // dual_model7 deck; Qm=Qs and Qa=Qr leave Qe itself as the saturation.
        assert_eq!(retention_head(DefaultReal(0.45454544), &par), -511.5721);
        assert_eq!(retention_head(DefaultReal(0.9), &par), -9.00377);
        // SeIm=1 leaves Qee at 0.999999999999999, i.e. the head is just below zero.
        assert_eq!(retention_head(DefaultReal(1.0), &par), -1.4065995e-11);
        // Qee collapses onto QeeM and the -1.d37 floor of 369 catches the result.
        assert_eq!(retention_head(DefaultReal(0.0), &par), -1.0e37);
        // Independent of the constants above: 369 inverts the relation of 368,
        // `Qee = (1+(-Alfa*H)**n)**(-m)`, so feeding the result back through the
        // forward form must return the saturation.
        let n = f64::from(par.n);
        let m = 1.0 - 1.0 / n;
        for saturation in [0.45454544_f64, 0.9] {
            let head = f64::from(retention_head(DefaultReal(saturation as f32), &par));
            let forward = (1.0 + (-f64::from(par.alfa) * head).powf(n)).powf(-m);
            assert!(
                (forward - saturation).abs() < 1.0e-6,
                "FH({saturation}) = {head} forwards to {forward}"
            );
        }
    }

    #[test]
    fn conductivity_matches_the_fk_zero_model_path() {
        let par = ImmobilePar {
            q_r: 0.01,
            q_s: 0.12,
            alfa: 0.1,
            n: 1.2,
            k_s: 0.01,
            b_par: 0.5,
        };
        // Transcribed from MATERIAL.FOR:24-79 with PPar=2 and the ParD(6,M) slot of
        // the dual_model7 deck as BPar.
        assert_eq!(
            conductivity(DefaultReal(-100.0), &par),
            8.134865e-7,
            "the deep branch of 67-79"
        );
        assert_eq!(conductivity(DefaultReal(-50.0), &par), 4.1893045e-6);
        assert_eq!(conductivity(DefaultReal(-10.0), &par), 0.000112350244);
        assert_eq!(conductivity(DefaultReal(-1.0), &par), 0.0014024149);
        // Qk=Qs and Kk=Ks make Hk equal Hs, so any head at or above Hs takes the
        // FK=sngl(Ks) branch of 148 and the linear band of 144-146 stays empty.
        assert_eq!(conductivity(DefaultReal(-1e-12), &par), 0.01);
        assert_eq!(conductivity(DefaultReal(0.0), &par), 0.01);
        // Both conductivity floors: amax1(Par(5),1.e-37) at 28 and
        // sngl(max(Ks*Kr,1.d-37)) at 79.
        let dry = ImmobilePar { k_s: 0.0, ..par };
        assert_eq!(conductivity(DefaultReal(-100.0), &dry), 1.0e-37);
    }

    #[test]
    fn positive_exchange_is_capped_by_the_immobile_room() {
        let exchange = dual_porosity_exchange(&water_content_exchange()).unwrap();
        // Node 1: SeIm=(0.125-0.05)/0.2=0.375 and Se=1, so the raw rate is
        // ParD(9,M)*(Se-SeIm)=5*0.625=3.125, which the 1059 cap
        // min(DeltaTh/dt,(ParD(8,M)-ThOldIm)/dt)=min(0.7954546,1.25) cuts down.
        assert_eq!(exchange.sink_immobile[0], DefaultReal(0.7954546));
        assert_eq!(
            exchange.new_immobile_water[0],
            DefaultReal(0.20454547),
            "ThNewIm(1)=ThOldIm(1)+SinkIm(1)*dt (1068)"
        );
        // Node 2: both regions saturated, so Se-SeIm is exactly zero.
        assert_eq!(exchange.sink_immobile[1], DefaultReal(0.0));
        assert_eq!(exchange.new_immobile_water[1], DefaultReal(0.3));
    }

    #[test]
    fn negative_exchange_is_capped_by_the_mobile_capacity() {
        let exchange = dual_porosity_exchange(&water_content_exchange()).unwrap();
        // Node 3: Se=(0.2-0.08)/0.35 against SeIm=(0.2-0.05)/0.2, and the small
        // ParD(9,M)=0.1 keeps the rate away from both caps.
        assert!(exchange.sink_immobile[2].0 < 0.0);
        assert_eq!(exchange.sink_immobile[2], DefaultReal(-0.04071429));
        assert_eq!(
            exchange.new_immobile_water[2],
            DefaultReal(0.19592857),
            "a negative SinkIm drains the immobile region"
        );
        // Node 4: the same state with ParD(9,M)=5, so the raw rate -2.035714 is cut
        // by the 1064 cap max(DeltaTh/dt,-(ths-ThOld)/dt)=max(-0.5181819,-2.3).
        assert_eq!(exchange.sink_immobile[3], DefaultReal(-0.51818186));
        assert_eq!(
            exchange.new_immobile_water[3],
            DefaultReal(0.14818181),
            "the immobile region loses exactly SinkIm*dt"
        );
    }

    #[test]
    fn dual_porosity_transfer_accumulates_in_node_order() {
        let exchange = dual_porosity_exchange(&water_content_exchange()).unwrap();
        // 1070-1071 trapezoids SinkIm over x from node 2 onwards into one sum: the
        // four node rates below contribute three trapezoids, the first of them
        // from nodes 1 and 2 only because the `i.ge.2` guard skips node 1 itself.
        assert_eq!(exchange.transfer, DefaultReal(0.02448053));
        let trapezoids = exchange
            .sink_immobile
            .windows(2)
            .map(|pair| (pair[0].0 + pair[1].0) / 2.0 * 0.25)
            .collect::<Vec<f32>>();
        assert_eq!(trapezoids.len(), 2 + 1);
        assert!(
            trapezoids.iter().sum::<f32>() > 0.0,
            "the large positive first trapezoid dominates: {trapezoids:?}"
        );
    }

    #[test]
    fn transfer_starts_at_the_second_node_and_materials_are_per_node() {
        // Two nodes in the same water state but in different ParD columns: only
        // ParD(9,M) differs, by the exact factor of two below.
        let exchange = dual_porosity_exchange(&state(
            &[0.0, 0.25],
            &[0.2, 0.2],
            &[-100.0; 2],
            &[0.2, 0.2],
            &[1, 2],
            vec![
                material6(0.08, 0.43, 0.05, 0.25, 0.25),
                material6(0.08, 0.43, 0.05, 0.25, 0.5),
            ],
            -1.0e10,
            0.1,
            1,
        ))
        .unwrap();
        assert_eq!(exchange.sink_immobile[0], DefaultReal(-0.10178572));
        assert_eq!(exchange.sink_immobile[1], DefaultReal(-0.20357144));
        assert_eq!(
            exchange.transfer,
            DefaultReal(-0.038169645),
            "one trapezoid, from node 2 only"
        );
        // Dropping the 1070 guard would start the integral at node 1 instead.
        let single = dual_porosity_exchange(&state(
            &[0.0],
            &[0.2],
            &[-100.0],
            &[0.2],
            &[1],
            vec![material6(0.08, 0.43, 0.05, 0.25, 0.25)],
            -1.0e10,
            0.1,
            1,
        ))
        .unwrap();
        assert_eq!(single.transfer, DefaultReal(0.0));
        assert_eq!(single.sink_immobile[0], exchange.sink_immobile[0]);
    }

    #[test]
    fn exchange_reads_the_immobile_water_and_returns_a_new_one() {
        let source = water_content_exchange();
        let before = source.old_immobile_water.clone();
        let exchange = dual_porosity_exchange(&source).unwrap();
        assert_eq!(
            source.old_immobile_water, before,
            "DualPor must not move ThOldIm; Update does the copy at 1112"
        );
        assert_ne!(
            exchange.new_immobile_water, source.old_immobile_water,
            "the returned ThNewIm is the routine's own array (1068)"
        );
        for (index, &material_id) in source.material_ids.iter().enumerate() {
            let material = &source.materials[material_id - 1];
            let th_new_im = exchange.new_immobile_water[index].0;
            assert!(
                th_new_im >= material.immobile_residual_water.0
                    && th_new_im <= material.immobile_saturated_water.0,
                "1068 keeps ThNewIm inside [ParD(7,M), ParD(8,M)]: {th_new_im}"
            );
        }
    }

    #[test]
    fn pressure_head_exchange_uses_the_immobile_retention_pair() {
        // SeIm=(0.06-0.01)/0.11=0.45454544 gives FH's head -511.57263, so node 1
        // sees CondM=CondF=8.134864e-7 and hNew-h=411.57263, while node 2's
        // hNew=-50 halves neither factor.
        let exchange = dual_porosity_exchange(&pressure_head_exchange(
            -1.0e10,
            &[-100.0, -50.0, -100.0],
            &[0.06; 3],
        ))
        .unwrap();
        assert_eq!(exchange.sink_immobile[0], DefaultReal(0.00017042436));
        assert_eq!(exchange.sink_immobile[1], DefaultReal(0.0009702202));
        assert_eq!(exchange.sink_immobile[2], exchange.sink_immobile[0]);
        assert_eq!(
            exchange.new_immobile_water[0],
            DefaultReal(0.06001704),
            "a positive rate here still means mobile to immobile (1053)"
        );
        assert_eq!(exchange.transfer, DefaultReal(0.00028516114));
    }

    #[test]
    fn top_node_guard_zeroes_only_the_last_node_at_the_atmospheric_ceiling() {
        // ThOldIm=0.07870114 gives SeIm=0.62455583, whose FH head is -100.0000076:
        // within -0.01*hCritA of hCritA=-100, and hNew(3)=-100 is within
        // -0.001*hCritA, so 1054-1055 zeroes SinkIm(3) before either clamp runs.
        let ceiling = [0.06, 0.06, 0.07870114];
        let exchange =
            dual_porosity_exchange(&pressure_head_exchange(-100.0, &[-100.0; 3], &ceiling))
                .unwrap();
        assert_eq!(exchange.sink_immobile[2], DefaultReal(0.0));
        assert_eq!(
            exchange.new_immobile_water[2],
            DefaultReal(0.07870114),
            "a zeroed rate leaves ThOldIm standing through 1068"
        );
        assert_eq!(exchange.transfer, DefaultReal(6.390914e-05));
        assert_ne!(
            exchange.sink_immobile[0],
            DefaultReal(0.0),
            "the guard is restricted to i.eq.N"
        );
        // The same state at the reader's hCritA of -1e10 fails both conjuncts.
        let far = dual_porosity_exchange(&pressure_head_exchange(-1.0e10, &[-100.0; 3], &ceiling))
            .unwrap();
        assert_eq!(far.sink_immobile[2], DefaultReal(-4.3444894e-11));
        // And with every node at the ceiling only node 3 is zeroed.
        let all = dual_porosity_exchange(&pressure_head_exchange(
            -100.0,
            &[-100.0; 3],
            &[0.07870114; 3],
        ))
        .unwrap();
        assert_eq!(all.sink_immobile[0], DefaultReal(-4.3444894e-11));
        assert_eq!(all.sink_immobile[1], DefaultReal(-4.3444894e-11));
        assert_eq!(all.sink_immobile[2], DefaultReal(0.0));
    }

    #[test]
    fn each_model_reads_its_own_pard_slot_set() {
        // iDualPor=1 never reaches the ParD(6), ParD(10) and ParD(11) region: a
        // model 6 deck stops at ParD(9) (INPUT.FOR:618).
        let mut water_content = water_content_exchange();
        let before = dual_porosity_exchange(&water_content).unwrap();
        for material in &mut water_content.materials {
            material.mobile_tortuosity = Some(DefaultReal(0.9));
            material.immobile_n = Some(DefaultReal(9.0));
            material.immobile_saturated_conductivity = Some(DefaultReal(7.0));
        }
        assert_eq!(dual_porosity_exchange(&water_content).unwrap(), before);
        // iDualPor=2 does: FH takes ParD(7..10) and FK takes ParD(7..11) plus
        // ParD(6), so ParD(9,M) is a retention parameter here, not the first-order
        // coefficient of 1040.
        let mut pressure = pressure_head_exchange(-1.0e10, &[-100.0; 3], &[0.06; 3]);
        let pressure_before = dual_porosity_exchange(&pressure).unwrap();
        for material in &mut pressure.materials {
            material.immobile_n = Some(DefaultReal(1.5));
        }
        assert_ne!(
            dual_porosity_exchange(&pressure).unwrap(),
            pressure_before,
            "ParD(10,M) reaches the iDualPor=2 head"
        );
    }

    #[test]
    fn contract_rejections_name_the_missing_or_degenerate_input() {
        let reject = |state: DualPorPrimitiveState| {
            dual_porosity_exchange(&state).expect_err("the contract must reject this state")
        };
        let one_material = || vec![material6(0.08, 0.43, 0.05, 0.25, 0.1)];
        assert!(reject(state(
            &[],
            &[],
            &[],
            &[],
            &[],
            one_material(),
            -1.0e10,
            0.1,
            1
        ))
        .contains("at least one node"));
        let mut ragged = water_content_exchange();
        ragged.old_water.pop();
        assert!(reject(ragged).contains("same length"));
        // iDualPor=0 means the 43 guard never called the routine at all.
        let mut inactive = water_content_exchange();
        inactive.dual_porosity = 0;
        assert!(reject(inactive).contains("iDualPor 1 or 2"));
        // Models 6 and 7 are remapped to 0 (INPUT.FOR:619-624), so 6 here means the
        // caller invented an iModel the reader never passes on.
        let mut remapped = water_content_exchange();
        remapped.hydraulic_model = 6;
        assert!(reject(remapped).contains("iModel 0"));
        let mut frozen = water_content_exchange();
        frozen.dt = DefaultReal(0.0);
        assert!(reject(frozen).contains("positive time step"));
        let mut headless = water_content_exchange();
        headless.critical_atmospheric_head = DefaultReal(f32::NAN);
        assert!(reject(headless).contains("finite hCritA"));
        let mut dried = water_content_exchange();
        dried.materials[0].immobile_saturated_water = dried.materials[0].immobile_residual_water;
        assert!(reject(dried).contains("ParD(8,M) must exceed ParD(7,M)"));
        let mut inverted = water_content_exchange();
        inverted.materials[0].immobile_saturated_water =
            DefaultReal(inverted.materials[0].immobile_residual_water.0 - 0.1);
        assert!(reject(inverted).contains("ParD(8,M) must exceed ParD(7,M)"));
        let mut air = water_content_exchange();
        air.materials[0].mobile_saturated_water = air.materials[0].mobile_residual_water;
        assert!(reject(air).contains("ParD(2,M) must exceed ParD(1,M)"));
        let mut flooded = water_content_exchange();
        flooded.old_water[1] = DefaultReal(f32::INFINITY);
        assert!(reject(flooded).contains("ThOld must be finite"));
        let mut collapsing = water_content_exchange();
        collapsing.coordinates[2] = DefaultReal(0.25);
        assert!(reject(collapsing).contains("coordinates must increase"));
        let mut unknown = water_content_exchange();
        unknown.material_ids[0] = 0;
        assert!(reject(unknown.clone()).contains("index the ParD columns"));
        unknown.material_ids[0] = 4;
        assert!(reject(unknown).contains("index the ParD columns"));
        // A model 6 column cannot serve a model 7 exchange.
        let model7_state = || pressure_head_exchange(-1.0e10, &[-100.0; 3], &[0.06; 3]);
        let mut incomplete = model7_state();
        incomplete.materials = vec![material6(0.08, 0.43, 0.01, 0.12, 0.1)];
        assert!(reject(incomplete).contains("ParD(10,M)"));
        let mut no_ksat = model7_state();
        no_ksat.materials[0].immobile_saturated_conductivity = None;
        assert!(reject(no_ksat).contains("ParD(11,M)"));
        let mut no_tortuosity = model7_state();
        no_tortuosity.materials[0].mobile_tortuosity = None;
        assert!(reject(no_tortuosity).contains("ParD(6,M)"));
        let mut flat_retention = model7_state();
        flat_retention.materials[0].immobile_n = Some(DefaultReal(1.0));
        assert!(reject(flat_retention).contains("ParD(10,M) must exceed one"));
        let mut closed = model7_state();
        closed.materials[0].immobile_alpha = DefaultReal(0.0);
        assert!(reject(closed).contains("ParD(9,M) must be positive"));
        let mut smeared = model7_state();
        smeared.materials[0].immobile_n = Some(DefaultReal(f32::NAN));
        assert!(reject(smeared).contains("ParD columns must be finite"));
    }

    #[test]
    fn th_imob_selects_the_level_and_leaves_non_dual_to_18_9_2() {
        let new = dual_domain_th_imob(true, 1, false, 0.12, 0.1)
            .unwrap()
            .unwrap();
        assert_eq!((new.th_imob, new.th_imob_o), (0.12, Some(0.1)));
        let old = dual_domain_th_imob(false, 2, false, 0.12, 0.1)
            .unwrap()
            .unwrap();
        assert_eq!((old.th_imob, old.th_imob_o), (0.1, None));
        assert_eq!(dual_domain_th_imob(true, 0, false, 0.12, 0.1), Ok(None));
        assert!(dual_domain_th_imob(true, 3, false, 0.1, 0.1).is_err());
        assert!(dual_domain_th_imob(true, 1, false, f32::NAN, 0.1).is_err());
    }

    #[test]
    fn exchange_immobile_water_feeds_the_new_level_th_imob() {
        let exchange = dual_porosity_exchange(&water_content_exchange()).unwrap();
        let old = [0.125_f32, 0.3, 0.2, 0.2];
        for (new, old) in exchange.new_immobile_water.iter().zip(old) {
            let t = dual_domain_th_imob(true, 1, false, new.0, old)
                .unwrap()
                .unwrap();
            assert_eq!(t.th_imob, new.0);
            assert_eq!(t.th_imob_o, Some(old));
        }
    }

    #[test]
    fn neighbour_theta_is_not_reduced_for_dual_porosity() {
        assert_eq!(dual_neighbor_theta(1, false, 0.31), Ok(Some(0.31)));
        assert_eq!(dual_neighbor_theta(0, false, 0.31), Ok(None));
        assert!(dual_neighbor_theta(9, false, 0.31).is_err());
    }

    #[test]
    fn bacteria_with_dual_porosity_is_unsupported() {
        assert!(dual_domain_th_imob(true, 1, true, 0.1, 0.1).is_err());
        assert!(dual_neighbor_theta(2, true, 0.3).is_err());
        // lBact alone is the 18.9.4/18.9.2 path, not rejected here.
        assert_eq!(dual_neighbor_theta(0, true, 0.3), Ok(None));
    }

    #[test]
    fn material_selection_is_zero_based_and_range_checked() {
        assert_eq!(dual_material_selection(&[3, 1, 2], 3), Ok(vec![2, 0, 1]));
        assert!(dual_material_selection(&[0], 3).is_err());
        assert!(dual_material_selection(&[4], 3).is_err());
    }
}

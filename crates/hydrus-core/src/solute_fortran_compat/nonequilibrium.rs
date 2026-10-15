//! Active `NEquil` update, `SOLUTE.FOR:1078-1204`.
//!
//! The call is live: `SOLUTE.FOR:631-641` is a column-6 continuation under
//! `if(.not.lEquil)`, evaluated inside `Coeff` once per node before the matrix is
//! solved. Everything is default REAL (`f32`), with the source's left-to-right
//! expression order kept.
//!
//! The caller passes the *immobile-form* rates: the dummies `GamL`, `GamL1`, `GamS`,
//! `GamS1` (and the `O` forms) receive `GamLi`, `GamL1i`, `GamSi`, `GamS1i` (and
//! `GamLOi`, ...), i.e. the `rMD` factors evaluated on `ThImob` (iReact 1, 4, 2, 5).
//! Species `jS` is 0-based here, so `Sorb(jS-1,i)` is `sorb[species - 1][node]`.
//! The pools mutated, as in the source, are `Sorb(jS,i)`/`Sorb2(jS,i)` for linear
//! updates and `SorbN(i)`/`SorbN2(i)` for nonlinear ones; `DMobI` is the dummy that
//! is assigned only in the linear mobile-immobile branch.
//! `NonEqInit`/`MassInit` are not here (18.8.1).

use super::types::KernelError;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct NEquilFlags {
    /// `lLinear(jS)`.
    pub linear: bool,
    /// `lMobIm(M)` for the node's material.
    pub mobile_immobile: bool,
    /// `iDualPor`.
    pub dual_porosity: u8,
    /// `lBact`.
    pub bacteria: bool,
    /// `lDualNEq`.
    pub dual_neq: bool,
    /// `lNEquil`: the previous species feeds this species' sorbed pools.
    pub chain: bool,
    /// `Level.eq.NLevel`.
    pub final_level: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct NEquilInput {
    pub dt: f32,
    pub epsi: f32,
    pub ro: f32,
    pub frac: f32,
    pub xks: f32,
    pub xks_o: f32,
    pub xks_p: f32,
    pub cc: f32,
    pub sconc: f32,
    pub sconc_o: f32,
    pub sconc_s: f32,
    pub sconc_o_s: f32,
    pub sconc_ps: f32,
    pub dsconc_s: f32,
    pub xmu_l: f32,
    pub xmu_l_o: f32,
    pub xmu_s: f32,
    pub xmu_s_o: f32,
    pub dretard_s: f32,
    pub gam_l: f32,
    pub gam_l1: f32,
    pub gam_l_o: f32,
    pub gam_l1_o: f32,
    pub gam_s: f32,
    pub gam_s1: f32,
    pub gam_s_o: f32,
    pub gam_s1_o: f32,
    pub gam_l1_pi: f32,
    pub gam_s1_pi: f32,
    pub omega: f32,
    pub omega_o: f32,
    pub omega_s: f32,
    pub f_em: f32,
    pub rka1: f32,
    pub rka1_o: f32,
    pub rka2: f32,
    pub rka2_o: f32,
    pub rkd1: f32,
    pub rkd1_o: f32,
    pub rkd2: f32,
    pub rkd2_o: f32,
    pub th_w: f32,
    pub th_w_o: f32,
    pub psi1: f32,
    pub psi1_o: f32,
    pub psi2: f32,
    pub psi2_o: f32,
    pub th_imob: f32,
    pub th_imob_o: f32,
    /// Incoming `DMobI`; returned unchanged unless the linear mobile-immobile branch assigns it.
    pub dmob_i: f32,
}

/// Per-species pool arrays indexed `[species][node]`, plus the per-node work arrays.
pub(crate) struct NEquilPools<'a> {
    pub conc: &'a [Vec<f32>],
    pub sorb: &'a mut [Vec<f32>],
    pub sorb2: &'a mut [Vec<f32>],
    pub c_new: &'a [f32],
    pub sorb_n: &'a mut [f32],
    pub sorb_n2: &'a mut [f32],
    pub sink_im: &'a [f32],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NEquilOutput {
    pub ssorb: f32,
    pub ssorb2: f32,
    pub fl_macro: f32,
    pub dmob_i: f32,
}

/// Guard of the call site `SOLUTE.FOR:631`: `if(.not.lEquil)`.
pub(crate) fn should_call_nequil(equilibrium: bool) -> bool {
    !equilibrium
}

fn validate(
    species: usize,
    node: usize,
    flags: &NEquilFlags,
    pools: &NEquilPools,
) -> Result<(), KernelError> {
    if species >= pools.conc.len() || species >= pools.sorb.len() {
        return Err("species index outside the pool arrays");
    }
    if flags.chain && species == 0 {
        return Err("lNEquil chain term needs a previous species");
    }
    let needs2 = flags.bacteria || flags.dual_neq;
    if needs2 && species >= pools.sorb2.len() {
        return Err("second sorbed pool missing");
    }
    let n = pools.conc[species].len();
    let rows_ok = pools.sorb[species].len() == n
        && pools.c_new.len() == n
        && pools.sorb_n.len() == n
        && pools.sorb_n2.len() == n
        && pools.sink_im.len() == n
        && (!needs2 || pools.sorb2[species].len() == n);
    let prev_ok = !flags.chain
        || (pools.sorb[species - 1].len() == n
            && (!flags.dual_neq || pools.sorb2[species - 1].len() == n));
    if !rows_ok || !prev_ok {
        return Err("pool array lengths disagree");
    }
    if node >= n {
        return Err("node outside the pool arrays");
    }
    Ok(())
}

pub(crate) fn nequil(
    species: usize,
    node: usize,
    flags: &NEquilFlags,
    x: &NEquilInput,
    pools: &mut NEquilPools,
) -> Result<NEquilOutput, KernelError> {
    validate(species, node, flags, pools)?;
    let i = node;
    let js = species;
    let conc = pools.conc[js][i];
    let dt = x.dt;
    let epsi = x.epsi;
    let frac = x.frac;
    let ro = x.ro;

    let mut fl_macro = 0.0f32;
    if flags.dual_porosity > 0 {
        let s = pools.sink_im[i];
        fl_macro = if s > 0.0 {
            s * conc
        } else {
            s * pools.sorb[js][i]
        };
    }
    let mut ssorb = pools.sorb[js][i];
    let mut ssorb2 = if js < pools.sorb2.len() && i < pools.sorb2[js].len() {
        pools.sorb2[js][i]
    } else {
        0.0
    };
    let mut dmob_i = x.dmob_i;

    if flags.final_level {
        if (flags.mobile_immobile || flags.dual_porosity > 0) && !flags.bacteria {
            let a = (x.th_imob + x.th_imob_o) / 2.0 + (1.0 - frac) * ro * x.xks * x.dsconc_s;
            let d_theta = x.th_imob - x.th_imob_o;
            let mut e =
                x.th_imob * x.xmu_l + (1.0 - frac) * ro * x.xmu_s - (1.0 - frac) * ro * x.dretard_s;
            let mut e_o = x.th_imob_o * x.xmu_l_o + (1.0 - frac) * ro * x.xmu_s_o
                - (1.0 - frac) * ro * x.dretard_s;
            e += fl_macro;
            e_o += fl_macro;
            if flags.chain {
                let cs = pools.sorb[js - 1][i]
                    * (x.th_imob * x.gam_l1_pi
                        + (1.0 - frac) * ro * x.gam_s1_pi * x.xks_p * x.sconc_ps);
                e += cs;
                e_o += cs;
            }
            let b = x.th_imob * (x.gam_l + x.gam_l1)
                + (1.0 - frac) * ro * (x.gam_s + x.gam_s1) * x.xks * x.sconc_s;
            let b_o = x.th_imob_o * (x.gam_l_o + x.gam_l1_o)
                + (1.0 - frac) * ro * (x.gam_s_o + x.gam_s1_o) * x.xks_o * x.sconc_o_s;
            if flags.linear {
                dmob_i = 2.0 * a + dt * (x.omega + b) + d_theta;
                let g0 = (2.0 * a - dt * (x.omega_o + b_o) - d_theta) / dmob_i;
                pools.sorb[js][i] =
                    pools.sorb[js][i] * g0 + dt * (x.omega_o * conc + e + e_o) / dmob_i;
                ssorb = pools.sorb[js][i];
            } else {
                let old = pools.sorb[js][i];
                let sn = pools.sorb_n[i];
                pools.sorb_n[i] = old
                    + dt / a
                        * (epsi * (x.omega * (pools.c_new[i] - sn) - b * sn + e)
                            + (1.0 - epsi) * (x.omega_o * (conc - old) - b_o * old + e_o));
                ssorb = pools.sorb_n[i];
            }
            if flags.dual_neq {
                let mut cs = 0.0f32;
                if flags.chain {
                    cs = x.gam_s1_pi * pools.sorb2[js - 1][i] * dt * 2.0;
                }
                let one_f = 1.0 - x.f_em;
                if flags.linear {
                    let s2 = pools.sorb2[js][i];
                    pools.sorb2[js][i] = ((2.0 - (x.omega_s + x.gam_s_o + x.gam_s1_o) * dt) * s2
                        + dt * frac * one_f * x.omega_s * x.xks_o * conc
                        + dt * one_f * (x.xmu_s_o + x.xmu_s)
                        + cs)
                        / (2.0 + dt * (x.omega_s + x.gam_s + x.gam_s1));
                    ssorb2 = pools.sorb2[js][i];
                } else {
                    let s2 = pools.sorb2[js][i];
                    let sn2 = pools.sorb_n2[i];
                    pools.sorb_n2[i] = s2
                        + dt * (epsi
                            * (x.omega_s * (frac * one_f * x.sconc * x.xks * x.cc - sn2)
                                - (x.gam_s + x.gam_s1) * sn2
                                + one_f * x.xmu_s)
                            + (1.0 - epsi)
                                * (x.omega_s
                                    * (frac * one_f * x.sconc_o * x.xks_o * conc - ssorb2)
                                    - (x.gam_s_o + x.gam_s1_o) * ssorb2
                                    + one_f * x.xmu_s_o));
                    ssorb2 = pools.sorb_n2[i];
                }
            }
        } else if !flags.bacteria {
            // two-site sorption model
            let mut cs = 0.0f32;
            if flags.chain {
                cs = x.gam_s1_pi * pools.sorb[js - 1][i] * dt * 2.0;
            }
            if flags.linear {
                let s = pools.sorb[js][i];
                pools.sorb[js][i] = ((2.0 - (x.omega_o + x.gam_s_o + x.gam_s1_o) * dt) * s
                    + dt * (1.0 - frac) * x.omega_o * x.xks_o * conc
                    + dt * (1.0 - frac) * (x.xmu_s_o + x.xmu_s)
                    + cs)
                    / (2.0 + dt * (x.omega + x.gam_s + x.gam_s1));
                ssorb = pools.sorb[js][i];
            } else {
                let s = pools.sorb[js][i];
                let sn = pools.sorb_n[i];
                pools.sorb_n[i] = s + dt
                    * (epsi
                        * (x.omega * ((1.0 - frac) * x.sconc * x.xks * x.cc - sn)
                            - (x.gam_s + x.gam_s1) * sn
                            + (1.0 - frac) * x.xmu_s)
                        + (1.0 - epsi)
                            * (x.omega_o * ((1.0 - frac) * x.sconc_o * x.xks_o * conc - ssorb)
                                - (x.gam_s_o + x.gam_s1_o) * ssorb
                                + (1.0 - frac) * x.xmu_s_o));
                ssorb = pools.sorb_n[i];
            }
        } else {
            // filtration model
            if flags.linear {
                let s = pools.sorb[js][i];
                pools.sorb[js][i] = ((2.0 - dt * (x.rkd1_o + x.gam_s_o + x.gam_s1_o)) * s
                    + dt * x.rka1_o * x.th_w * conc / ro)
                    / (2.0 + dt * (x.rkd1 + x.gam_s + x.gam_s1));
                ssorb = pools.sorb[js][i];
                let s2 = pools.sorb2[js][i];
                pools.sorb2[js][i] = ((2.0 - dt * (x.rkd2_o + x.gam_s_o + x.gam_s1_o)) * s2
                    + dt * x.rka2_o * x.th_w * conc / ro)
                    / (2.0 + dt * (x.rkd2 + x.gam_s + x.gam_s1));
                ssorb2 = pools.sorb2[js][i];
            } else {
                let s = pools.sorb[js][i];
                let sn = pools.sorb_n[i];
                pools.sorb_n[i] = s + dt
                    * (epsi
                        * (x.rka1 * x.th_w / ro * x.psi1 * x.cc
                            - (x.rkd1 + x.gam_s + x.gam_s1) * sn)
                        + (1.0 - epsi)
                            * (x.rka1_o * x.th_w_o / ro * x.psi1_o * conc
                                - (x.rkd1_o + x.gam_s_o + x.gam_s1_o) * s));
                ssorb = pools.sorb_n[i];
                let s2 = pools.sorb2[js][i];
                let sn2 = pools.sorb_n2[i];
                pools.sorb_n2[i] = s2
                    + dt * (epsi
                        * (x.rka2 * x.th_w / ro * x.psi2 * x.cc
                            - (x.rkd2 + x.gam_s + x.gam_s1) * sn2)
                        + (1.0 - epsi)
                            * (x.rka2_o * x.th_w_o / ro * x.psi2_o * conc
                                - (x.rkd2_o + x.gam_s_o + x.gam_s1_o) * s2));
                ssorb2 = pools.sorb_n2[i];
            }
        }
    }
    Ok(NEquilOutput {
        ssorb,
        ssorb2,
        fl_macro,
        dmob_i,
    })
}

/// Inputs of the mobile/immobile water selection, `SOLUTE.FOR:277-318`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ImmobileWaterSpec {
    /// `Level.eq.NLevel` (`ThN`/`ThImobO` branch) versus the old-level branch.
    pub final_level: bool,
    /// `lMobIm(M)`.
    pub mobile_immobile: bool,
    /// `iDualPor`; positive values are 18.9.3's `ThNIm`/`ThOIm` path and are rejected.
    pub dual_porosity: u8,
    /// `lBact`; the caller supplies the flag, the zeroing itself is 18.9.4's.
    pub bacteria: bool,
    /// `ThN(i)` at the final level, `ThO(i)` otherwise.
    pub theta: f32,
    pub theta_sat: f32,
    /// `ChPar(4,M)`.
    pub chpar_th_imob: f32,
    /// `ThN(j)`/`ThO(j)`; `None` for the last node (`i.eq.NumNP`).
    pub theta_next: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ImmobileWater {
    /// `ThW`, reduced to `max(ThW-ThImob, 0.001)` when the guard holds.
    pub th_w: f32,
    /// `ThG=max(0,thSat-ThW)` evaluated *before* the reduction (source order).
    pub th_g: f32,
    /// `ThImob`; `None` when the guard fails and the source leaves it unassigned.
    pub th_imob: Option<f32>,
    /// `ThImobO=ThImob` is assigned only at `Level.eq.NLevel` (`SOLUTE.FOR:283`). The
    /// old-level branch (303-305) never assigns it, so it keeps a stale value: `None`.
    pub th_imob_o: Option<f32>,
    /// `Thj`, reduced under the same guard.
    pub th_next: Option<f32>,
}

/// `ThW`/`ThImob` selection for the mobile-immobile model (`iDualPor.eq.0` or `lBact`).
pub(crate) fn immobile_water(spec: &ImmobileWaterSpec) -> Result<ImmobileWater, KernelError> {
    if spec.dual_porosity > 0 {
        return Err("iDualPor>0 ThImob selection belongs to the dual-domain kernel");
    }
    let finite = spec.theta.is_finite()
        && spec.theta_sat.is_finite()
        && spec.chpar_th_imob.is_finite()
        && spec.theta_next.is_none_or(f32::is_finite);
    if !finite {
        return Err("non-finite water content input");
    }
    let th_g = (spec.theta_sat - spec.theta).max(0.0);
    if !(spec.mobile_immobile || spec.bacteria) {
        return Ok(ImmobileWater {
            th_w: spec.theta,
            th_g,
            th_imob: None,
            th_imob_o: None,
            th_next: spec.theta_next,
        });
    }
    let im = spec.chpar_th_imob;
    Ok(ImmobileWater {
        th_w: (spec.theta - im).max(0.001),
        th_g,
        th_imob: Some(im),
        th_imob_o: spec.final_level.then_some(im),
        th_next: spec.theta_next.map(|t| (t - im).max(0.001)),
    })
}

/// Raw parameters of the `Omega`/`OmegaS`/`f_em` assignments (`SOLUTE.FOR:370-373`, `494`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OmegaSpec {
    pub final_level: bool,
    pub dual_neq: bool,
    /// `ChPar(jjj+20,M)`, `TDep(jjj+20)`.
    pub chpar_omega: f32,
    pub tdep_omega: f32,
    /// `ChPar(jjj+16,M)`, `TDep(jjj+16)` (read only when `lDualNEq`).
    pub chpar_omega_s: f32,
    pub tdep_omega_s: f32,
    /// `ChPar(jjj+13,M)`, `TDep(jjj+13)` (read only when `lDualNEq`).
    pub chpar_f_em: f32,
    pub tdep_f_em: f32,
    /// Temperature factors `TT` (new/current level) and `TTO` (old level).
    pub tt: f32,
    pub tto: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OmegaTerms {
    pub omega: f32,
    /// Assigned only at `Level.eq.NLevel` (`SOLUTE.FOR:494`); otherwise `None`.
    pub omega_o: Option<f32>,
    /// `None` unless `lDualNEq` (the source leaves `OmegaS` unassigned).
    pub omega_s: Option<f32>,
    /// `1.` unless `lDualNEq`.
    pub f_em: f32,
}

/// Source values of `Omega`, `OmegaO`, `OmegaS`, `f_em`. No `lBact` handling: the caller
/// applies the 18.9.4 zeroing of `Omega`/`OmegaO` afterwards.
pub(crate) fn omega_terms(spec: &OmegaSpec) -> Result<OmegaTerms, KernelError> {
    let all = [
        spec.chpar_omega,
        spec.tdep_omega,
        spec.chpar_omega_s,
        spec.tdep_omega_s,
        spec.chpar_f_em,
        spec.tdep_f_em,
        spec.tt,
        spec.tto,
    ];
    if all.iter().any(|v| !v.is_finite()) {
        return Err("non-finite Omega input");
    }
    Ok(OmegaTerms {
        omega: spec.chpar_omega * (spec.tdep_omega * spec.tt).exp(),
        omega_o: spec
            .final_level
            .then(|| spec.chpar_omega * (spec.tdep_omega * spec.tto).exp()),
        omega_s: spec
            .dual_neq
            .then(|| spec.chpar_omega_s * (spec.tdep_omega_s * spec.tt).exp()),
        f_em: if spec.dual_neq {
            spec.chpar_f_em * (spec.tdep_f_em * spec.tt).exp()
        } else {
            1.0
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected values are an independent double-precision transcription of the Fortran
    // (`.development/18.7.2/ref_nequil.py`), not this module's output.
    fn close(a: f32, b: f64) {
        assert!(
            (a as f64 - b).abs() <= 2e-5 * b.abs().max(1e-3),
            "{a} vs {b}"
        );
    }

    fn input() -> NEquilInput {
        NEquilInput {
            dt: 0.5,
            epsi: 0.6,
            ro: 1.5,
            frac: 0.4,
            xks: 0.3,
            xks_o: 0.28,
            xks_p: 0.25,
            cc: 2.0,
            sconc: 1.2,
            sconc_o: 1.15,
            sconc_s: 1.1,
            sconc_o_s: 1.05,
            sconc_ps: 0.95,
            dsconc_s: 0.9,
            xmu_l: 0.01,
            xmu_l_o: 0.012,
            xmu_s: 0.02,
            xmu_s_o: 0.022,
            dretard_s: 0.003,
            gam_l: 0.05,
            gam_l1: 0.02,
            gam_l_o: 0.04,
            gam_l1_o: 0.03,
            gam_s: 0.06,
            gam_s1: 0.01,
            gam_s_o: 0.05,
            gam_s1_o: 0.02,
            gam_l1_pi: 0.04,
            gam_s1_pi: 0.03,
            omega: 0.2,
            omega_o: 0.18,
            omega_s: 0.15,
            f_em: 0.3,
            rka1: 0.7,
            rka1_o: 0.65,
            rka2: 0.5,
            rka2_o: 0.45,
            rkd1: 0.08,
            rkd1_o: 0.07,
            rkd2: 0.06,
            rkd2_o: 0.05,
            th_w: 0.35,
            th_w_o: 0.33,
            psi1: 0.9,
            psi1_o: 0.85,
            psi2: 0.8,
            psi2_o: 0.75,
            th_imob: 0.12,
            th_imob_o: 0.1,
            dmob_i: 7.0,
        }
    }

    struct Fx {
        conc: Vec<Vec<f32>>,
        sorb: Vec<Vec<f32>>,
        sorb2: Vec<Vec<f32>>,
        c_new: Vec<f32>,
        sorb_n: Vec<f32>,
        sorb_n2: Vec<f32>,
        sink: Vec<f32>,
    }

    fn fx(sink: f32) -> Fx {
        Fx {
            conc: vec![vec![1.4], vec![1.8]],
            sorb: vec![vec![0.35], vec![0.5]],
            sorb2: vec![vec![0.21], vec![0.3]],
            c_new: vec![2.1],
            sorb_n: vec![0.45],
            sorb_n2: vec![0.33],
            sink: vec![sink],
        }
    }

    fn call(f: &mut Fx, sp: usize, fl: NEquilFlags) -> Result<NEquilOutput, KernelError> {
        let mut p = NEquilPools {
            conc: &f.conc,
            sorb: &mut f.sorb,
            sorb2: &mut f.sorb2,
            c_new: &f.c_new,
            sorb_n: &mut f.sorb_n,
            sorb_n2: &mut f.sorb_n2,
            sink_im: &f.sink,
        };
        nequil(sp, 0, &fl, &input(), &mut p)
    }

    fn flags() -> NEquilFlags {
        NEquilFlags {
            final_level: true,
            ..Default::default()
        }
    }

    #[test]
    fn mobile_immobile_linear_chain_dual_pool_positive_sink() {
        let mut f = fx(0.04);
        let fl = NEquilFlags {
            linear: true,
            mobile_immobile: true,
            dual_porosity: 1,
            dual_neq: true,
            chain: true,
            ..flags()
        };
        let o = call(&mut f, 1, fl).unwrap();
        close(o.fl_macro, 0.072);
        close(o.dmob_i, 0.840595);
        close(o.ssorb, 0.650662775);
        close(f.sorb[1][0], 0.650662775);
        close(o.ssorb2, 0.2836891);
        close(f.sorb2[1][0], 0.2836891);
        // species 0 pools and the nonlinear work arrays are untouched
        assert_eq!(f.sorb[0][0], 0.35);
        assert_eq!(f.sorb_n[0], 0.45);
    }

    #[test]
    fn mobile_immobile_nonlinear_chain_dual_pool_negative_sink() {
        let mut f = fx(-0.04);
        let fl = NEquilFlags {
            mobile_immobile: true,
            dual_porosity: 1,
            dual_neq: true,
            chain: true,
            ..flags()
        };
        let o = call(&mut f, 1, fl).unwrap();
        close(o.fl_macro, -0.02);
        close(o.ssorb, 0.896258746);
        close(f.sorb_n[0], 0.896258746);
        close(o.ssorb2, 0.28624064);
        close(f.sorb_n2[0], 0.28624064);
        assert_eq!(f.sorb[1][0], 0.5, "nonlinear path leaves Sorb");
        assert_eq!(o.dmob_i, 7.0, "DMobI only assigned by the linear branch");
    }

    #[test]
    fn mobile_immobile_linear_plain_keeps_second_pool() {
        let mut f = fx(0.0);
        let fl = NEquilFlags {
            linear: true,
            mobile_immobile: true,
            ..flags()
        };
        let o = call(&mut f, 0, fl).unwrap();
        close(o.fl_macro, 0.0);
        close(o.ssorb, 0.413437684);
        close(o.ssorb2, 0.21);
        close(o.dmob_i, 0.840595);
    }

    #[test]
    fn two_site_linear_with_chain_term() {
        let mut f = fx(0.0);
        let o = call(
            &mut f,
            1,
            NEquilFlags {
                linear: true,
                chain: true,
                ..flags()
            },
        )
        .unwrap();
        close(o.ssorb, 0.462677283);
        close(f.sorb[1][0], 0.462677283);
        close(o.ssorb2, 0.3);
    }

    #[test]
    fn two_site_nonlinear() {
        let mut f = fx(0.0);
        let o = call(&mut f, 0, flags()).unwrap();
        close(o.ssorb, 0.33794728);
        close(f.sorb_n[0], 0.33794728);
        assert_eq!(f.sorb[0][0], 0.35);
    }

    #[test]
    fn filtration_linear_updates_both_pools() {
        let mut f = fx(0.0);
        let o = call(
            &mut f,
            0,
            NEquilFlags {
                linear: true,
                bacteria: true,
                ..flags()
            },
        )
        .unwrap();
        close(o.ssorb, 0.376706827);
        close(o.ssorb2, 0.232881356);
        close(f.sorb2[0][0], 0.232881356);
    }

    #[test]
    fn filtration_nonlinear_updates_both_work_arrays() {
        let mut f = fx(0.0);
        let o = call(
            &mut f,
            0,
            NEquilFlags {
                bacteria: true,
                ..flags()
            },
        )
        .unwrap();
        close(o.ssorb, 0.442184);
        close(f.sorb_n[0], 0.442184);
        close(o.ssorb2, 0.26888);
        close(f.sorb_n2[0], 0.26888);
    }

    #[test]
    fn bacteria_overrides_mobile_immobile_and_dual_porosity() {
        // `.not.lBact` fails, so the filtration formula runs even with iDualPor>0.
        let mut f = fx(0.04);
        let fl = NEquilFlags {
            linear: true,
            bacteria: true,
            mobile_immobile: true,
            dual_porosity: 1,
            ..flags()
        };
        let o = call(&mut f, 0, fl).unwrap();
        close(o.ssorb, 0.376706827);
        close(o.fl_macro, 0.04 * 1.4);
    }

    #[test]
    fn non_final_level_only_reports_macro_flux_and_copies() {
        let mut f = fx(0.04);
        let fl = NEquilFlags {
            linear: true,
            mobile_immobile: true,
            dual_porosity: 1,
            final_level: false,
            ..Default::default()
        };
        let o = call(&mut f, 0, fl).unwrap();
        close(o.fl_macro, 0.04 * 1.4);
        assert_eq!((o.ssorb, o.ssorb2), (0.35, 0.21));
        assert_eq!(f.sorb[0][0], 0.35);
    }

    #[test]
    fn guards_and_shape_checks() {
        assert!(should_call_nequil(false));
        assert!(!should_call_nequil(true));
        let mut f = fx(0.0);
        assert!(call(
            &mut f,
            0,
            NEquilFlags {
                chain: true,
                ..flags()
            }
        )
        .is_err());
        assert!(call(&mut f, 2, flags()).is_err());
        f.sorb_n.clear();
        assert!(call(&mut f, 0, flags()).is_err());
    }

    fn water_spec() -> ImmobileWaterSpec {
        ImmobileWaterSpec {
            final_level: true,
            mobile_immobile: true,
            dual_porosity: 0,
            bacteria: false,
            theta: 0.30,
            theta_sat: 0.45,
            chpar_th_imob: 0.12,
            theta_next: Some(0.10),
        }
    }

    #[test]
    fn immobile_water_reduction_and_floor() {
        let w = immobile_water(&water_spec()).unwrap();
        close(w.th_w, 0.18);
        close(w.th_g, 0.15);
        assert_eq!(w.th_imob, Some(0.12));
        assert_eq!(w.th_imob_o, Some(0.12));
        assert_eq!(w.th_next, Some(0.001)); // 0.10-0.12 floored
    }

    #[test]
    fn immobile_water_old_level_leaves_th_imob_o_stale() {
        let mut s = water_spec();
        s.final_level = false;
        let w = immobile_water(&s).unwrap();
        assert_eq!(w.th_imob, Some(0.12));
        assert_eq!(w.th_imob_o, None);
    }

    #[test]
    fn immobile_water_guard_off_and_bact_and_dual() {
        let mut s = water_spec();
        s.mobile_immobile = false;
        let w = immobile_water(&s).unwrap();
        assert_eq!((w.th_w, w.th_imob, w.th_imob_o), (0.30, None, None));
        assert_eq!(w.th_next, Some(0.10));
        s.bacteria = true;
        assert_eq!(immobile_water(&s).unwrap().th_imob, Some(0.12));
        s.dual_porosity = 1;
        assert!(immobile_water(&s).is_err());
        let mut n = water_spec();
        n.theta = f32::NAN;
        assert!(immobile_water(&n).is_err());
    }

    fn omega_spec() -> OmegaSpec {
        OmegaSpec {
            final_level: true,
            dual_neq: true,
            chpar_omega: 0.2,
            tdep_omega: 3.0,
            chpar_omega_s: 0.05,
            tdep_omega_s: 2.0,
            chpar_f_em: 0.7,
            tdep_f_em: 1.0,
            tt: 0.01,
            tto: 0.02,
        }
    }

    #[test]
    fn omega_terms_temperature_scaling() {
        let o = omega_terms(&omega_spec()).unwrap();
        close(o.omega, 0.2 * (3.0_f64 * 0.01).exp());
        close(o.omega_o.unwrap(), 0.2 * (3.0_f64 * 0.02).exp());
        close(o.omega_s.unwrap(), 0.05 * (2.0_f64 * 0.01).exp());
        close(o.f_em, 0.7 * (1.0_f64 * 0.01).exp());
    }

    #[test]
    fn omega_terms_availability() {
        let mut s = omega_spec();
        s.dual_neq = false;
        s.final_level = false;
        let o = omega_terms(&s).unwrap();
        assert_eq!((o.omega_o, o.omega_s, o.f_em), (None, None, 1.0));
        s.tt = f32::INFINITY;
        assert!(omega_terms(&s).is_err());
    }

    #[test]
    fn th_imob_and_omega_feed_nequil_input() {
        let w = immobile_water(&water_spec()).unwrap();
        let o = omega_terms(&omega_spec()).unwrap();
        let mut i = input();
        i.th_imob = w.th_imob.unwrap();
        i.th_imob_o = w.th_imob_o.unwrap();
        i.omega = o.omega;
        i.omega_o = o.omega_o.unwrap();
        i.omega_s = o.omega_s.unwrap();
        i.f_em = o.f_em;
        assert_eq!(i.th_imob, 0.12);
        assert_eq!(i.omega_s, o.omega_s.unwrap());
    }
}

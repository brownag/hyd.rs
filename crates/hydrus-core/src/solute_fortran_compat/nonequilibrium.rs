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
}

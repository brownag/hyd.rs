//! Active `SorbConc` update (`SOLUTE.FOR:1212-1313`) and the reachable `iMoistDep=1`
//! (Walker) `rMD` factors of `Coeff` and `SorbConc`.
//!
//! `SorbConc` is live: `SOLUTE.FOR:184-188` is a column-6 continuation under
//! `if(.not.lEquil.and.lLinear(jS))`, run once per species after the matrix solve.
//!
//! # Moisture selector verdict
//!
//! `iMoistDep` is assigned in only two places: `INPUT.FOR:1693` (`iMoistDep=0`) and
//! `INPUT.FOR:1320` (`if(lMoistDep) iMoistDep=1`). `HYDRUS.FOR:279-281` calls
//! `MoistDepIn` only for `iMoistDep.eq.2`, and `MoistDepIn` itself only ever lowers 2
//! to 0 on an I/O error (`SOLUTE.FOR:1810,1813`). Nothing in the audited reader sets
//! 2, so the `DMoist` (trapezoid) branch of `rMD` (`1838-1857`) is *unselected by the
//! audited reader*; `lMoistDep` activates selector 1 only. Selector 2 is rejected
//! here rather than ported.
//!
//! # `WDep` layout (stride 9)
//!
//! `WDep(1,(jS-1)*9+k)` is the exponent and `WDep(2+M,(jS-1)*9+k)` the material
//! reference water content (`FQ` of `WDep(2,..)`), `k = 1..9` (`INPUT.FOR:1393-1405`).
//! `jReact` and `DMoist` play no part in selector 1. `ChPar`/`TDep` use stride 16.
//! Species and material are 0-based in this module; `i_react` is the 1-based source value.
//! Everything is default REAL (`f32`), source expression order.

use super::types::KernelError;

const TR: f32 = 293.15;
const GAS_R: f32 = 8.314;

/// Source `WDep`, stride 9 per species.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MoistureFactorTable {
    /// `WDep(1, ..)`, length `9 * species`.
    pub exponent: Vec<f32>,
    /// `WDep(2+M, ..)`, `[material][9 * species]`.
    pub reference: Vec<Vec<f32>>,
}

impl MoistureFactorTable {
    /// `rMD` with `iMoistDep=1` (`SOLUTE.FOR:1858-1862`); 1 when the guard fails.
    pub(crate) fn walker(
        &self,
        material: usize,
        species: usize,
        i_react: usize,
        theta: f32,
    ) -> Result<f32, KernelError> {
        if !(1..=9).contains(&i_react) {
            return Err("iReact must be 1..9");
        }
        let col = species * 9 + i_react - 1;
        let reference = *self
            .reference
            .get(material)
            .and_then(|r| r.get(col))
            .ok_or("WDep reference outside the table")?;
        let exponent = *self
            .exponent
            .get(col)
            .ok_or("WDep exponent outside the table")?;
        Ok(if reference > theta && reference > 0.0 {
            (theta / reference).powf(exponent)
        } else {
            1.0
        })
    }
}

/// `rMD(...)` call under `if(iMoistDep.gt.0)`; `f1` stays 1 for selector 0.
fn rmd(
    selector: u8,
    table: Option<&MoistureFactorTable>,
    material: usize,
    species: usize,
    i_react: usize,
    theta: f32,
) -> Result<f32, KernelError> {
    match selector {
        0 => Ok(1.0),
        1 => table
            .ok_or("selector 1 needs the WDep table")?
            .walker(material, species, i_react, theta),
        _ => Err("moisture selector 2 is unselected by the audited reader"),
    }
}

/// Mode-1 factors of `Coeff` lines 330-368 (new level). Each is the `f1` applied to
/// the same-named coefficient; `GamGi` shares `GamG`'s factor (`GamGi=GamG`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CoeffFactors {
    pub gam_l: f32,
    pub gam_li: f32,
    pub gam_s: f32,
    pub gam_si: f32,
    pub gam_g: f32,
    pub gam_l1: f32,
    pub gam_l1i: f32,
    pub gam_s1: f32,
    pub gam_s1i: f32,
    pub gam_g1: f32,
    pub xmu_l: f32,
    pub xmu_s: f32,
    pub xmu_g: f32,
}

/// `jS.gt.1` factors, lines 434-447, evaluated for species `jS-1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PrevSpeciesFactors {
    pub gam_l1p: f32,
    pub gam_l1pi: f32,
    pub gam_s1p: f32,
    pub gam_s1pi: f32,
    pub gam_g1p: f32,
}

/// `Level.eq.NLevel` factors, lines 464-492, on `ThO(i)`/`ThImobO`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OldLevelFactors {
    pub gam_lo: f32,
    pub gam_loi: f32,
    pub gam_so: f32,
    pub gam_soi: f32,
    pub gam_l1o: f32,
    pub gam_l1oi: f32,
    pub gam_s1o: f32,
    pub gam_s1oi: f32,
    pub xmu_lo: f32,
    pub xmu_so: f32,
}

pub(crate) fn coeff_factors(
    selector: u8,
    table: Option<&MoistureFactorTable>,
    material: usize,
    species: usize,
    th_w: f32,
    th_imob: f32,
) -> Result<CoeffFactors, KernelError> {
    let r = |ir: usize, th: f32| rmd(selector, table, material, species, ir, th);
    Ok(CoeffFactors {
        gam_l: r(1, th_w)?,
        gam_li: r(1, th_imob)?,
        gam_s: r(2, th_w)?,
        gam_si: r(2, th_imob)?,
        gam_g: r(3, th_w)?,
        gam_l1: r(4, th_w)?,
        gam_l1i: r(4, th_imob)?,
        gam_s1: r(5, th_w)?,
        gam_s1i: r(5, th_imob)?,
        gam_g1: r(6, th_w)?,
        xmu_l: r(7, th_w)?,
        xmu_s: r(8, th_w)?,
        xmu_g: r(9, th_w)?,
    })
}

pub(crate) fn prev_species_factors(
    selector: u8,
    table: Option<&MoistureFactorTable>,
    material: usize,
    species: usize,
    th_w: f32,
    th_imob: f32,
) -> Result<PrevSpeciesFactors, KernelError> {
    if species == 0 {
        return Err("previous-species factors need jS > 1");
    }
    let p = species - 1;
    let r = |ir: usize, th: f32| rmd(selector, table, material, p, ir, th);
    Ok(PrevSpeciesFactors {
        gam_l1p: r(4, th_w)?,
        gam_l1pi: r(4, th_imob)?,
        gam_s1p: r(5, th_w)?,
        gam_s1pi: r(5, th_imob)?,
        gam_g1p: r(6, th_w)?,
    })
}

pub(crate) fn old_level_factors(
    selector: u8,
    table: Option<&MoistureFactorTable>,
    material: usize,
    species: usize,
    th_old: f32,
    th_imob_old: f32,
) -> Result<OldLevelFactors, KernelError> {
    let r = |ir: usize, th: f32| rmd(selector, table, material, species, ir, th);
    Ok(OldLevelFactors {
        gam_lo: r(1, th_old)?,
        gam_loi: r(1, th_imob_old)?,
        gam_so: r(2, th_old)?,
        gam_soi: r(2, th_imob_old)?,
        gam_l1o: r(4, th_old)?,
        gam_l1oi: r(4, th_imob_old)?,
        gam_s1o: r(5, th_old)?,
        gam_s1oi: r(5, th_imob_old)?,
        xmu_lo: r(7, th_old)?,
        xmu_so: r(8, th_old)?,
    })
}

/// Call guard of `SOLUTE.FOR:184`: `.not.lEquil.and.lLinear(jS)`.
pub(crate) fn should_call_sorb_conc(equilibrium: bool, linear: bool) -> bool {
    !equilibrium && linear
}

pub(crate) struct SorbConcSpec<'a> {
    /// 0-based species (`jS-1`); `jjj=(jS-1)*16`.
    pub species: usize,
    pub dt: f32,
    /// `lMobIm(M)` by material.
    pub mobile_immobile: &'a [bool],
    pub bacteria: bool,
    pub filtration: bool,
    pub dual_porosity: u8,
    /// `iMoistDep`.
    pub moisture_selector: u8,
    pub dual_neq: bool,
    /// `ChPar(1..NSD*16+4, M)`, `[material][slot-1]`.
    pub chpar: &'a [Vec<f32>],
    /// `TDep(1..NSD*16+4)`.
    pub tdep: &'a [f32],
    pub moisture: Option<&'a MoistureFactorTable>,
}

pub(crate) struct SorbConcState<'a> {
    /// `MatNum(i)`, 0-based.
    pub material: &'a [usize],
    pub temperature: &'a [f32],
    /// `ThW(i)` (`ThN`).
    pub water: &'a [f32],
    /// `ThIm`/`ThOIm`, read when `iDualPor>0`.
    pub immobile_new: &'a [f32],
    pub immobile_old: &'a [f32],
    pub sink_im: &'a [f32],
    /// `Conc(jS,i)`.
    pub conc: &'a [f32],
    /// `Sorb(jS,i)`, updated in place.
    pub sorb: &'a mut [f32],
    /// `Sorb2(jS,i)`, updated when `lDualNEq` or `lBact`.
    pub sorb2: &'a mut [f32],
    /// `STrans(i)`, written for species 1 in the mobile-immobile branch.
    pub s_trans: &'a mut [f32],
    /// Caller-supplied `Deposit` results `(rKa1, rKa2)` per node; required when `lFiltr`.
    pub deposited_ka: Option<&'a [(f32, f32)]>,
}

fn temp_factor(temp: f32) -> f32 {
    (temp + 273.15 - TR) / GAS_R / (temp + 273.15) / TR
}

/// `SorbConc`, nodes in order, mutating `Sorb`/`Sorb2`/`STrans` as the source does.
// Preserve the source `Sorb=Sorb+...` expressions and operation order for fidelity.
#[allow(clippy::assign_op_pattern)]
pub(crate) fn sorb_conc(spec: &SorbConcSpec, st: &mut SorbConcState) -> Result<(), KernelError> {
    let n = st.conc.len();
    let needs2 = spec.bacteria || spec.dual_neq;
    let ok = st.material.len() == n
        && st.temperature.len() == n
        && st.water.len() == n
        && st.sink_im.len() == n
        && st.s_trans.len() == n
        && st.sorb.len() == n
        && (!needs2 || st.sorb2.len() == n)
        && (spec.dual_porosity == 0 || (st.immobile_new.len() == n && st.immobile_old.len() == n))
        && st.deposited_ka.map_or(!spec.filtration, |d| d.len() == n);
    if !ok {
        return Err("SorbConc state array lengths disagree");
    }
    if spec.moisture_selector > 1 {
        return Err("moisture selector 2 is unselected by the audited reader");
    }
    let jjj = spec.species * 16;
    let tdep = |k: usize| {
        spec.tdep
            .get(k - 1)
            .copied()
            .ok_or("TDep slot outside the table")
    };
    for i in 0..n {
        let m = st.material[i];
        let row = spec.chpar.get(m).ok_or("material outside ChPar")?;
        let chpar = |k: usize| {
            row.get(k - 1)
                .copied()
                .ok_or("ChPar slot outside the table")
        };
        if m >= spec.mobile_immobile.len() {
            return Err("material outside lMobIm");
        }
        let tt = temp_factor(st.temperature[i]);
        let at = |k: usize| -> Result<f32, KernelError> { Ok(chpar(k)? * (tdep(k)? * tt).exp()) };
        let frac = at(3)?;
        let xks = at(jjj + 7)?;
        let th_w = st.water[i];
        let conc = st.conc[i];
        let r = |ir: usize, th: f32| {
            rmd(
                spec.moisture_selector,
                spec.moisture,
                m,
                spec.species,
                ir,
                th,
            )
        };
        let mut gam_s = at(jjj + 12)? * r(2, th_w)?;
        let mut gam_s1 = at(jjj + 15)? * r(5, th_w)?;
        let omega = at(jjj + 20)?;

        if (spec.mobile_immobile[m] || spec.dual_porosity > 0) && !spec.bacteria {
            let ro = at(1)?;
            let (th_imob, th_imob_o) = if spec.dual_porosity == 0 {
                let t = at(4)?;
                (t, t)
            } else {
                (st.immobile_new[i], st.immobile_old[i])
            };
            let gam_l = at(jjj + 11)? * r(1, th_imob)?;
            let gam_l1 = at(jjj + 14)? * r(4, th_imob)?;
            gam_s = at(jjj + 12)? * r(2, th_imob)?;
            gam_s1 = at(jjj + 15)? * r(5, th_imob)?;
            let d_theta = th_imob - th_imob_o;
            let a = (th_imob + th_imob_o) / 2.0 + (1.0 - frac) * ro * xks;
            let b = th_imob * (gam_l + gam_l1) + (gam_s + gam_s1) * ro * (1.0 - frac) * xks;
            let d = 2.0 * a + spec.dt * (omega + b) + d_theta;
            st.sorb[i] = st.sorb[i] + spec.dt * omega * conc / d;
            let mut fl_macro = 0.0f32;
            if spec.dual_porosity > 0 {
                let s = st.sink_im[i];
                fl_macro = if s > 0.0 { s * conc } else { s * st.sorb[i] };
            }
            if spec.species == 0 {
                st.s_trans[i] = omega * (conc - st.sorb[i]) + fl_macro;
            }
            if spec.dual_neq {
                let f_em = at(jjj + 13)?;
                let omega_s = at(jjj + 16)?;
                st.sorb2[i] = st.sorb2[i]
                    + spec.dt * omega_s * frac * (1.0 - f_em) * xks * conc
                        / (2.0 + spec.dt * (omega_s + gam_s + gam_s1));
            }
        } else if !spec.bacteria {
            st.sorb[i] = st.sorb[i]
                + spec.dt * omega * (1.0 - frac) * xks * conc
                    / (2.0 + spec.dt * (omega + gam_s + gam_s1));
        } else {
            let ro = at(1)?;
            let th_imob = chpar(4)?;
            let theta = th_w - th_imob;
            gam_s = at(jjj + 12)?;
            gam_s1 = 0.0;
            let mut rka1 = at(jjj + 19)?;
            let rkd1 = at(jjj + 20)?;
            let mut rka2 = at(jjj + 16)?;
            let rkd2 = at(jjj + 17)?;
            if spec.filtration {
                let (a1, a2) = st.deposited_ka.ok_or("lFiltr needs the Deposit results")?[i];
                rka1 = a1;
                rka2 = a2;
            }
            st.sorb[i] = st.sorb[i]
                + spec.dt * rka1 * theta * conc / ro / (2.0 + spec.dt * (rkd1 + gam_s + gam_s1));
            st.sorb2[i] = st.sorb2[i]
                + spec.dt * rka2 * theta * conc / ro / (2.0 + spec.dt * (rkd2 + gam_s + gam_s1));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected `SorbConc` values come from an independent double-precision transcription of
    // the Fortran (`.development/18.7.2/ref_nequil.py`), not from this module.
    fn close(a: f32, b: f64) {
        assert!(
            (a as f64 - b).abs() <= 2e-5 * b.abs().max(1e-3),
            "{a} vs {b}"
        );
    }

    /// Two species, one material; `WDep` exponent `1+0.1k` and reference only where set below.
    fn table() -> MoistureFactorTable {
        let exponent: Vec<f32> = (1..=18).map(|k| 1.0 + 0.1 * k as f32).collect();
        let mut reference = vec![0.0f32; 18];
        for (k, v) in [
            (2, 0.4),
            (5, 0.5),
            (1, 0.3),
            (4, 0.45),
            (11, 0.38),
            (14, 0.42),
        ] {
            reference[k - 1] = v;
        }
        MoistureFactorTable {
            exponent,
            reference: vec![reference],
        }
    }

    fn chpar() -> Vec<f32> {
        let mut c = vec![0.0f32; 36];
        for (k, v) in [
            (1, 1.5),
            (3, 0.4),
            (4, 0.12),
            (7, 0.3),
            (11, 0.05),
            (12, 0.06),
            (13, 0.3),
            (14, 0.02),
            (15, 0.01),
            (16, 0.15),
            (17, 0.1),
            (19, 0.7),
            (20, 0.2),
            (23, 0.31),
            (27, 0.07),
            (28, 0.08),
            (29, 0.25),
            (30, 0.03),
            (31, 0.02),
            (32, 0.18),
            (36, 0.22),
        ] {
            c[k - 1] = v;
        }
        c
    }

    fn tdep() -> Vec<f32> {
        let mut t = vec![0.0f32; 36];
        t[6] = 500.0;
        t[11] = 300.0;
        t[19] = 200.0;
        t
    }

    struct Run {
        mobim: bool,
        bact: bool,
        filtr: bool,
        dual_por: u8,
        sel: u8,
        dual_neq: bool,
        species: usize,
        sink: f32,
    }

    impl Default for Run {
        fn default() -> Self {
            Run {
                mobim: false,
                bact: false,
                filtr: false,
                dual_por: 0,
                sel: 0,
                dual_neq: false,
                species: 0,
                sink: 0.05,
            }
        }
    }

    /// Returns `(Sorb, Sorb2, STrans)` after one node.
    fn go(r: Run) -> Result<(f32, f32, f32), KernelError> {
        let (c, t, w) = (vec![chpar()], tdep(), table());
        let mm = [r.mobim];
        let spec = SorbConcSpec {
            species: r.species,
            dt: 0.5,
            mobile_immobile: &mm,
            bacteria: r.bact,
            filtration: r.filtr,
            dual_porosity: r.dual_por,
            moisture_selector: r.sel,
            dual_neq: r.dual_neq,
            chpar: &c,
            tdep: &t,
            moisture: Some(&w),
        };
        let (mut s, mut s2, mut tr) = (vec![0.4f32], vec![0.2f32], vec![9.0f32]);
        let dep = [(0.55f32, 0.35f32)];
        let mut st = SorbConcState {
            material: &[0],
            temperature: &[15.0],
            water: &[0.3],
            immobile_new: &[0.1],
            immobile_old: &[0.09],
            sink_im: &[r.sink],
            conc: &[1.6],
            sorb: &mut s,
            sorb2: &mut s2,
            s_trans: &mut tr,
            deposited_ka: if r.filtr { Some(&dep) } else { None },
        };
        sorb_conc(&spec, &mut st)?;
        Ok((s[0], s2[0], tr[0]))
    }

    #[test]
    fn walker_selection_by_material_species_and_react() {
        let t = table();
        close(t.walker(0, 0, 2, 0.2).unwrap(), 0.43527528164806206); // (0.2/0.4)^1.2
        close(t.walker(0, 0, 5, 0.2).unwrap(), 0.2529822128134704); // (0.2/0.5)^1.5
                                                                    // species 2 reads columns 9+k: iReact 2 -> ref 0.38, exponent 1+0.1*11
        close(t.walker(0, 1, 2, 0.19).unwrap(), (0.19f64 / 0.38).powf(2.1));
        // theta at or above the reference, or a zero reference, leaves the factor at 1
        assert_eq!(t.walker(0, 0, 2, 0.4).unwrap(), 1.0);
        assert_eq!(t.walker(0, 0, 3, 0.1).unwrap(), 1.0);
        assert!(t.walker(1, 0, 2, 0.2).is_err());
        assert!(t.walker(0, 0, 10, 0.2).is_err());
        assert!(t.walker(0, 2, 1, 0.2).is_err());
    }

    #[test]
    fn coeff_factor_families_use_documented_ireact_and_theta() {
        let t = table();
        let p = |th: f64, r: f64, e: f64| (th / r).powf(e);
        let f = coeff_factors(1, Some(&t), 0, 0, 0.3, 0.1).unwrap();
        assert_eq!(f.gam_l, 1.0); // ref 0.3 is not > theta 0.3
        close(f.gam_li, p(0.1, 0.3, 1.1));
        close(f.gam_s, p(0.3, 0.4, 1.2));
        close(f.gam_si, p(0.1, 0.4, 1.2));
        assert_eq!(f.gam_g, 1.0);
        close(f.gam_l1, p(0.3, 0.45, 1.4));
        close(f.gam_l1i, p(0.1, 0.45, 1.4));
        close(f.gam_s1, p(0.3, 0.5, 1.5));
        close(f.gam_s1i, p(0.1, 0.5, 1.5));
        assert_eq!((f.gam_g1, f.xmu_l, f.xmu_s, f.xmu_g), (1.0, 1.0, 1.0, 1.0));
        // selector 0 never calls rMD: every f1 stays 1
        let z = coeff_factors(0, None, 0, 0, 0.3, 0.1).unwrap();
        assert_eq!((z.gam_s, z.gam_s1i, z.xmu_g), (1.0, 1.0, 1.0));
    }

    #[test]
    fn previous_species_and_old_level_factors() {
        let t = table();
        let p = |th: f64, r: f64, e: f64| (th / r).powf(e);
        // jS=2 asks rMD for species jS-1 (columns 1..9), not its own
        let pv = prev_species_factors(1, Some(&t), 0, 1, 0.3, 0.1).unwrap();
        close(pv.gam_l1p, p(0.3, 0.45, 1.4));
        close(pv.gam_l1pi, p(0.1, 0.45, 1.4));
        close(pv.gam_s1p, p(0.3, 0.5, 1.5));
        close(pv.gam_s1pi, p(0.1, 0.5, 1.5));
        assert_eq!(pv.gam_g1p, 1.0);
        assert!(prev_species_factors(1, Some(&t), 0, 0, 0.3, 0.1).is_err());
        // old level on ThO/ThImobO, species jS
        let o = old_level_factors(1, Some(&t), 0, 0, 0.25, 0.08).unwrap();
        close(o.gam_lo, p(0.25, 0.3, 1.1));
        close(o.gam_loi, p(0.08, 0.3, 1.1));
        close(o.gam_so, p(0.25, 0.4, 1.2));
        close(o.gam_s1oi, p(0.08, 0.5, 1.5));
        close(o.gam_l1o, p(0.25, 0.45, 1.4));
        assert_eq!((o.xmu_lo, o.xmu_so), (1.0, 1.0));
        let o2 = old_level_factors(1, Some(&t), 0, 1, 0.25, 0.08).unwrap();
        close(o2.gam_so, p(0.25, 0.38, 2.1));
        assert_eq!(o2.gam_lo, 1.0);
    }

    #[test]
    fn moist_dep_in_route_unselected() {
        // Selector 2 is rejected by every entry point.
        let t = table();
        assert!(coeff_factors(2, Some(&t), 0, 0, 0.3, 0.1).is_err());
        assert!(go(Run {
            sel: 2,
            ..Default::default()
        })
        .is_err());
        // Source evidence: the audited reader only assigns 0 or 1 to iMoistDep.
        let files = [
            include_bytes!("../../../../src_reference/INPUT.FOR").as_slice(),
            include_bytes!("../../../../src_reference/HYDRUS.FOR").as_slice(),
            include_bytes!("../../../../src_reference/SOLUTE.FOR").as_slice(),
            include_bytes!("../../../../src_reference/WATFLOW.FOR").as_slice(),
        ];
        let mut values = Vec::new();
        for f in files {
            for line in String::from_utf8_lossy(f).lines() {
                let l = line.to_lowercase();
                if l.starts_with('*') || l.starts_with('c') {
                    continue;
                }
                let mut rest = l.as_str();
                while let Some(p) = rest.find("imoistdep") {
                    let after = rest[p + 9..].trim_start();
                    if after.starts_with('=') && !after.starts_with("==") {
                        values.push(after[1..].trim().chars().next().unwrap());
                    }
                    rest = &rest[p + 9..];
                }
            }
        }
        values.sort();
        assert_eq!(
            values,
            vec!['0', '0', '0', '1'],
            "INPUT 1320 (=1); INPUT 1693 and the two MoistDepIn error resets (=0)"
        );
    }

    #[test]
    fn sorb_conc_guard() {
        assert!(should_call_sorb_conc(false, true));
        assert!(!should_call_sorb_conc(true, true));
        assert!(!should_call_sorb_conc(false, false));
    }

    #[test]
    fn two_site_branch_with_and_without_moisture_factor() {
        let (s, s2, tr) = go(Run::default()).unwrap();
        close(s, 0.41342369862023404);
        assert_eq!((s2, tr), (0.2, 9.0));
        close(
            go(Run {
                sel: 1,
                ..Default::default()
            })
            .unwrap()
            .0,
            0.4134958661568402,
        );
    }

    #[test]
    fn mobile_immobile_table_theta_and_strans() {
        let (s, _, tr) = go(Run {
            mobim: true,
            ..Default::default()
        })
        .unwrap();
        close(s, 0.5792097784465348);
        close(tr, 0.2038675492225084);
        let (s, s2, tr) = go(Run {
            mobim: true,
            sel: 1,
            dual_neq: true,
            ..Default::default()
        })
        .unwrap();
        close(s, 0.5812958353137986);
        close(s2, 0.20482279630370231);
        close(tr, 0.20345093149627214);
    }

    #[test]
    fn dual_porosity_uses_node_immobile_water_and_sink_sign() {
        let (s, _, tr) = go(Run {
            dual_por: 1,
            sel: 1,
            ..Default::default()
        })
        .unwrap();
        close(s, 0.5901025278931533);
        close(tr, 0.28169209917695703);
        // species 2: Sorb uses the updated value with a negative sink; STrans is species 1 only
        let (s, _, tr) = go(Run {
            dual_por: 1,
            sel: 1,
            species: 1,
            sink: -0.05,
            ..Default::default()
        })
        .unwrap();
        close(s, 0.6014270758191361);
        assert_eq!(tr, 9.0);
    }

    #[test]
    fn filtration_branch_with_and_without_deposit_inputs() {
        let (s, s2, _) = go(Run {
            bact: true,
            ..Default::default()
        })
        .unwrap();
        close(s, 0.43155235171217093);
        close(s2, 0.20692328997204967);
        let (s, s2, _) = go(Run {
            bact: true,
            filtr: true,
            ..Default::default()
        })
        .unwrap();
        close(s, 0.4247911334881343);
        close(s2, 0.2161543432681159);
    }
}

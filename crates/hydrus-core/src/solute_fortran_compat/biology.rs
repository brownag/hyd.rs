//! `Coeff` bacteria (`lBact`) and filtration (`lFiltr`) parameter selection
//! (`SOLUTE.FOR:374-425` new level, `501-540` old level, `449-456` previous species).
//!
//! # Scope split
//!
//! This module owns the *selection* the `Coeff` bacteria block performs: the
//! `ChPar`/`TDep` slot reads, `iPsi1`/`iPsi2` choice and defaulting, the `Dc`/`aa`
//! provenance, which `Sorb` pool feeds `Blocking`, and the `Dc`/`Dp`/`Alfa` inputs of
//! `Deposit`. `Blocking` and `Deposit` themselves belong to task 18.9.5
//! (`deposition.rs`) and are *not* called here: [`select_new_level`]/[`select_old_level`]
//! return [`BlockingRequest`]/[`DepositRequest`] values the caller feeds to those
//! routines, then hands the results to [`BacteriaLevel::finish`]. The `g0/g1/q0/q1`
//! composition (`643-730`, including the `lBact` increments) belongs to 18.9.1, which
//! consumes [`BacteriaScalars`]. The `MassTran`, `SorbConc` and `NEquil` bacteria
//! terms live in `accounting.rs`, `processes.rs` and `nonequilibrium.rs`.
//!
//! # Source facts preserved
//!
//! * The `lFiltr` branch is nested inside `if(lBact)`; `lFiltr` without `lBact` is inert
//!   in `Coeff`, so it is accepted silently here (the caller simply never selects).
//! * Without `lFiltr`, `iPsi2` comes from `ChPar(jjj+13)` and `iPsi1` from `ChPar(jjj+14)`
//!   (`int()` truncation); a zero code defaults to 1 when its `SMax` is positive.
//! * With `lFiltr`, those same slots are `Dc` and `Dp` for `Deposit` and the codes stay 0
//!   (or default to 1 from `SMax`).
//! * `Dc` (`jjj+6`) is read when either code is >= 3 and `aa` (`jjj+15`) when either is 5;
//!   otherwise the caller's previous `Dc`/`aa` persist, as the Fortran locals do.
//! * Old level, ASYMMETRY kept as written: `Blocking` there receives `Sorb(jS,i)` and
//!   `Sorb2(jS,i)` (`525-528`), never `SorbN`/`SorbN2`, and the old-level `Dc` read
//!   (`520-521`) uses the *new* temperature factor `TT`, not `TTO`.
//! * Old level `lFiltr` zeroes `GamL1O` only (`530`), not `GamL1Oi`.
//! * `lBact` with more than one species hits an interactive `STOP` (`657-682`), rejected.
//!
//! # Temperature provenance
//!
//! `Coeff` sets `TT` per level: `SOLUTE.FOR:297` uses `TempN(i)` in the `Level==NLevel`
//! branch and `:316` uses `TempO(i)` otherwise. The new-level block (374-425) therefore
//! derives `SMax/rKa/rKd/Dc` from `TempN` at the final level and from `TempO` at a
//! non-final level (`node.final_level`), while its `Deposit` call (422) always passes
//! `TempN(i)`. The old-level block (501-540) runs only at `Level==NLevel`: its
//! parameters use `TTO` from `TempO(i)`, its `Deposit` (535) passes `TempO(i)`, and its
//! `Dc` read (520-521) uses `TT`, which is `TempN`-derived only because that block
//! cannot run at a non-final level.
//!
//! Everything is default REAL (`f32`) in source expression order. Slots are 1-based
//! `ChPar(slot, M)` indexed `chpar[material][slot - 1]`; species is 0-based.

use super::types::KernelError;

const TR: f32 = 293.15;
const GAS_R: f32 = 8.314;

fn temp_factor(temp: f32) -> f32 {
    (temp + 273.15 - TR) / GAS_R / (temp + 273.15) / TR
}

/// Static configuration of the bacteria block.
pub(crate) struct BacteriaSpec<'a> {
    /// 0-based species (`jS-1`); `jjj=(jS-1)*16`.
    pub species: usize,
    pub species_count: usize,
    pub filtration: bool,
    /// `ChPar`, `[material][slot-1]`.
    pub chpar: &'a [Vec<f32>],
    /// `TDep`, `[slot-1]`.
    pub tdep: &'a [f32],
}

/// Per-node inputs for one level.
pub(crate) struct BacteriaNode {
    /// 0-based `MatNum(i)`.
    pub material: usize,
    /// `TempN(i)` and `TempO(i)`. The kernel picks between them as the source does, see
    /// the module doc "Temperature provenance".
    pub temp_n: f32,
    pub temp_o: f32,
    /// `x(i)`.
    pub x: f32,
    /// `Sorb(jS,i)` and `Sorb2(jS,i)`.
    pub sorb: f32,
    pub sorb2: f32,
    /// `SorbN(i)` and `SorbN2(i)`; read only by the new level when `Level==NLevel`.
    pub sorb_n: f32,
    pub sorb_n2: f32,
    /// `Level.eq.NLevel`.
    pub final_level: bool,
    /// `ThW` (new) or `ThWO` (old), the `Deposit` porosity argument.
    pub water: f32,
    /// `v` (new) or `vO(i)` (old), the Darcy flux given to `Deposit`.
    pub velocity: f32,
}

/// `Dc`/`aa` locals that persist between nodes and levels in the Fortran routine.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) struct CarriedLocals {
    pub dc: f32,
    pub aa: f32,
}

/// Arguments of `Blocking(iPsi,SMax,psi,x,ss,Dc,SMax2)`; `SMax2` is the source `aa`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BlockingRequest {
    pub i_psi: i32,
    pub s_max: f32,
    pub x: f32,
    pub ss: f32,
    pub dc: f32,
    pub aa: f32,
}

/// Arguments of `Deposit(Ka1,Ka2,Dc1,Dp1,Alfa1,Alfa2,Theta,q,Temp,xConv,tConv)`;
/// `xConv`/`tConv` are supplied by the caller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DepositRequest {
    pub dc: f32,
    pub dp: f32,
    pub alfa1: f32,
    pub alfa2: f32,
    pub theta: f32,
    pub q: f32,
    pub temp: f32,
}

/// Rates that `lBact` forces to zero, new level (`375-387`, `449-456`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NewZeroedRates {
    pub dg: f32,
    pub gam_g: f32,
    pub gam_gi: f32,
    pub gam_l1: f32,
    pub gam_l1i: f32,
    pub gam_s1: f32,
    pub gam_s1i: f32,
    pub gam_g1: f32,
    pub gam_g1i: f32,
    pub xmu_l: f32,
    pub xmu_s: f32,
    pub xmu_g: f32,
    pub omega: f32,
    /// Previous-species `GamL1P/GamL1Pi/GamS1P/GamS1Pi/GamG1P` (`449-456`); only
    /// reachable for `jS>1`, which `lBact` rejects, kept for completeness.
    pub prev_zero: bool,
}

/// Rates that `lBact` forces to zero, old level (`501-507`, `530`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OldZeroedRates {
    pub gam_s1o: f32,
    pub gam_s1oi: f32,
    pub xmu_lo: f32,
    pub xmu_so: f32,
    pub xmu_go: f32,
    pub omega_o: f32,
    /// `Some(0.0)` only under `lFiltr` (`GamL1O=0.`); `GamL1Oi` is left untouched.
    pub gam_l1o: Option<f32>,
}

/// Pre-`Blocking`/`Deposit` selection for one level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BacteriaLevel {
    pub s_max1: f32,
    pub s_max2: f32,
    pub r_ka1: f32,
    pub r_ka2: f32,
    pub r_kd1: f32,
    pub r_kd2: f32,
    pub i_psi1: i32,
    pub i_psi2: i32,
    pub blocking1: Option<BlockingRequest>,
    pub blocking2: Option<BlockingRequest>,
    pub deposit: Option<DepositRequest>,
    pub carried: CarriedLocals,
}

/// Scalars `Coeff` consumes after `Blocking`/`Deposit`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BacteriaScalars {
    pub s_max1: f32,
    pub s_max2: f32,
    pub r_ka1: f32,
    pub r_ka2: f32,
    pub r_kd1: f32,
    pub r_kd2: f32,
    pub psi1: f32,
    pub psi2: f32,
}

impl BacteriaLevel {
    /// Apply the `Blocking` results (`psi=1.` when no call is made) and, under `lFiltr`,
    /// the `Deposit` outputs that overwrite `rKa1`/`rKa2`.
    pub(crate) fn finish(
        &self,
        psi1: Option<f32>,
        psi2: Option<f32>,
        deposited: Option<(f32, f32)>,
    ) -> Result<BacteriaScalars, KernelError> {
        if self.blocking1.is_some() != psi1.is_some() || self.blocking2.is_some() != psi2.is_some()
        {
            return Err("psi must be supplied exactly when Blocking is requested");
        }
        if self.deposit.is_some() != deposited.is_some() {
            return Err("Deposit result must be supplied exactly when lFiltr requests it");
        }
        let (r_ka1, r_ka2) = deposited.unwrap_or((self.r_ka1, self.r_ka2));
        Ok(BacteriaScalars {
            s_max1: self.s_max1,
            s_max2: self.s_max2,
            r_ka1,
            r_ka2,
            r_kd1: self.r_kd1,
            r_kd2: self.r_kd2,
            psi1: psi1.unwrap_or(1.0),
            psi2: psi2.unwrap_or(1.0),
        })
    }
}

impl BacteriaSpec<'_> {
    /// Shape and source-STOP checks (ADR-0014: the adapter calls this).
    pub(crate) fn validate(&self, materials: &[usize]) -> Result<(), KernelError> {
        if self.species_count > 1 {
            return Err("bacteria with a multi-species chain is unsupported");
        }
        if self.species >= self.species_count.max(1) {
            return Err("species index out of range");
        }
        let need = self.species * 16 + 20;
        if self.tdep.len() < need {
            return Err("TDep too short for the bacteria slots");
        }
        for &m in materials {
            let row = self.chpar.get(m).ok_or("material index out of range")?;
            if row.len() < need {
                return Err("ChPar too short for the bacteria slots");
            }
        }
        Ok(())
    }

    fn par(&self, slot: usize, m: usize, tt: f32) -> f32 {
        let k = self.species * 16 + slot - 1;
        self.chpar[m][k] * (self.tdep[k] * tt).exp()
    }

    fn raw(&self, slot: usize, m: usize) -> f32 {
        self.chpar[m][self.species * 16 + slot - 1]
    }
}

/// Shared `iPsi`/`Dc`/`aa` selection (`394-402`, `514-522`). `dc_tt` is the factor
/// used by the `Dc` read (always the new-level `TT`).
fn select_psi(
    spec: &BacteriaSpec,
    m: usize,
    s_max1: f32,
    s_max2: f32,
    dc_tt: f32,
    mut carried: CarriedLocals,
) -> (i32, i32, CarriedLocals) {
    let (mut i1, mut i2) = (0_i32, 0_i32);
    if !spec.filtration {
        i2 = spec.raw(13, m) as i32;
        i1 = spec.raw(14, m) as i32;
    }
    if i1 == 0 && s_max1 > 0.0 {
        i1 = 1;
    }
    if i2 == 0 && s_max2 > 0.0 {
        i2 = 1;
    }
    if i1 >= 3 || i2 >= 3 {
        carried.dc = spec.par(6, m, dc_tt);
    }
    if i1 == 5 || i2 == 5 {
        carried.aa = spec.raw(15, m);
    }
    (i1, i2, carried)
}

/// New-level block (`374-425`), temperature factor `TT` of `TempN(i)`.
pub(crate) fn select_new_level(
    spec: &BacteriaSpec,
    node: &BacteriaNode,
    carried: CarriedLocals,
) -> Result<(BacteriaLevel, NewZeroedRates), KernelError> {
    spec.validate(&[node.material])?;
    let m = node.material;
    // 297/316: TT follows the level; Deposit (422) always takes TempN.
    let tt = temp_factor(if node.final_level {
        node.temp_n
    } else {
        node.temp_o
    });
    let s_max2 = spec.par(15, m, tt);
    let r_ka2 = spec.par(16, m, tt);
    let r_kd2 = spec.par(17, m, tt);
    let s_max1 = spec.par(18, m, tt);
    let r_ka1 = spec.par(19, m, tt);
    let r_kd1 = spec.par(20, m, tt);
    let (i1, i2, mut carried) = select_psi(spec, m, s_max1, s_max2, tt, carried);
    // 404-409: SorbN/SorbN2 at the final level, Sorb/Sorb2 otherwise.
    let (ss1, ss2) = if node.final_level {
        (node.sorb_n, node.sorb_n2)
    } else {
        (node.sorb, node.sorb2)
    };
    let req = |i_psi: i32, s_max: f32, ss: f32| {
        (i_psi > 0).then_some(BlockingRequest {
            i_psi,
            s_max,
            x: node.x,
            ss,
            dc: carried.dc,
            aa: carried.aa,
        })
    };
    let blocking1 = req(i1, s_max1, ss1);
    let blocking2 = req(i2, s_max2, ss2);
    let mut deposit = None;
    if spec.filtration {
        // 416-424: Dc/Dp from slots 13/14; Alfa is the pre-Deposit rKa.
        let dc = spec.par(13, m, tt);
        let dp = spec.par(14, m, tt);
        carried.dc = dc;
        deposit = Some(DepositRequest {
            dc,
            dp,
            alfa1: r_ka1,
            alfa2: r_ka2,
            theta: node.water,
            q: node.velocity,
            temp: node.temp_n,
        });
    }
    let zero = NewZeroedRates {
        dg: 0.0,
        gam_g: 0.0,
        gam_gi: 0.0,
        gam_l1: 0.0,
        gam_l1i: 0.0,
        gam_s1: 0.0,
        gam_s1i: 0.0,
        gam_g1: 0.0,
        gam_g1i: 0.0,
        xmu_l: 0.0,
        xmu_s: 0.0,
        xmu_g: 0.0,
        omega: 0.0,
        prev_zero: true,
    };
    let level = BacteriaLevel {
        s_max1,
        s_max2,
        r_ka1,
        r_ka2,
        r_kd1,
        r_kd2,
        i_psi1: i1,
        i_psi2: i2,
        blocking1,
        blocking2,
        deposit,
        carried,
    };
    Ok((level, zero))
}

/// Old-level block (`501-540`), present only when `Level.eq.NLevel`. `node.temperature`
/// uses `TempO(i)` for `TTO` and `Deposit`; the `Dc` read uses `TT` from `TempN(i)`
/// (valid because this block only runs when `node.final_level`).
pub(crate) fn select_old_level(
    spec: &BacteriaSpec,
    node: &BacteriaNode,
    carried: CarriedLocals,
) -> Result<(BacteriaLevel, OldZeroedRates), KernelError> {
    spec.validate(&[node.material])?;
    if !node.final_level {
        return Err("the old-level bacteria block runs only at Level==NLevel");
    }
    let m = node.material;
    let tto = temp_factor(node.temp_o);
    let tt = temp_factor(node.temp_n);
    let s_max2 = spec.par(15, m, tto);
    let r_ka2 = spec.par(16, m, tto);
    let r_kd2 = spec.par(17, m, tto);
    let s_max1 = spec.par(18, m, tto);
    let r_ka1 = spec.par(19, m, tto);
    let r_kd1 = spec.par(20, m, tto);
    let (i1, i2, mut carried) = select_psi(spec, m, s_max1, s_max2, tt, carried);
    // ASYMMETRY (525-528): the old level always uses Sorb/Sorb2, never SorbN/SorbN2.
    let req = |i_psi: i32, s_max: f32, ss: f32| {
        (i_psi > 0).then_some(BlockingRequest {
            i_psi,
            s_max,
            x: node.x,
            ss,
            dc: carried.dc,
            aa: carried.aa,
        })
    };
    let blocking1 = req(i1, s_max1, node.sorb);
    let blocking2 = req(i2, s_max2, node.sorb2);
    let mut deposit = None;
    if spec.filtration {
        let dc = spec.par(13, m, tto);
        let dp = spec.par(14, m, tto);
        carried.dc = dc;
        deposit = Some(DepositRequest {
            dc,
            dp,
            alfa1: r_ka1,
            alfa2: r_ka2,
            theta: node.water,
            q: node.velocity,
            temp: node.temp_o,
        });
    }
    let level = BacteriaLevel {
        s_max1,
        s_max2,
        r_ka1,
        r_ka2,
        r_kd1,
        r_kd2,
        i_psi1: i1,
        i_psi2: i2,
        blocking1,
        blocking2,
        deposit,
        carried,
    };
    let zero = OldZeroedRates {
        gam_s1o: 0.0,
        gam_s1oi: 0.0,
        xmu_lo: 0.0,
        xmu_so: 0.0,
        xmu_go: 0.0,
        omega_o: 0.0,
        gam_l1o: spec.filtration.then_some(0.0),
    };
    Ok((level, zero))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Slots (1-based): 6 Dc, 13/14 iPsi2/iPsi1 or Dc/Dp, 15 SMax2/aa, 16 rKa2, 17 rKd2,
    // 18 SMax1, 19 rKa1, 20 rKd1.
    fn row(s13: f32, s14: f32, smax2: f32, smax1: f32) -> Vec<f32> {
        let mut r = vec![0.0; 20];
        r[5] = 7.0;
        r[12] = s13;
        r[13] = s14;
        r[14] = smax2;
        r[15] = 0.2;
        r[16] = 0.3;
        r[17] = smax1;
        r[18] = 0.5;
        r[19] = 0.6;
        r
    }

    fn node(t: f32) -> BacteriaNode {
        BacteriaNode {
            material: 0,
            temp_n: t,
            temp_o: t,
            x: -3.0,
            sorb: 1.0,
            sorb2: 2.0,
            sorb_n: 10.0,
            sorb_n2: 20.0,
            final_level: true,
            water: 0.4,
            velocity: 0.01,
        }
    }

    fn old_node() -> BacteriaNode {
        BacteriaNode {
            temp_n: 30.0,
            temp_o: 10.0,
            ..node(0.0)
        }
    }

    fn spec<'a>(chpar: &'a [Vec<f32>], tdep: &'a [f32], filtration: bool) -> BacteriaSpec<'a> {
        BacteriaSpec {
            species: 0,
            species_count: 1,
            filtration,
            chpar,
            tdep,
        }
    }

    fn new_level(
        ch: &[Vec<f32>],
        td: &[f32],
        filtr: bool,
        n: &BacteriaNode,
        c: CarriedLocals,
    ) -> Result<(BacteriaLevel, NewZeroedRates), KernelError> {
        select_new_level(&spec(ch, td, filtr), n, c)
    }

    #[test]
    fn parameters_and_explicit_codes() {
        let ch = [row(4.0, 2.0, 9.0, 8.0)];
        let td = [0.0; 20];
        let (l, z) = new_level(&ch, &td, false, &node(20.0), Default::default()).unwrap();
        assert_eq!((l.i_psi2, l.i_psi1), (4, 2));
        assert_eq!((l.s_max2, l.r_ka2, l.r_kd2), (9.0, 0.2, 0.3));
        assert_eq!((l.s_max1, l.r_ka1, l.r_kd1), (8.0, 0.5, 0.6));
        // iPsi 4 >= 3 reads Dc from slot 6; no iPsi 5 so aa is untouched.
        assert_eq!(l.carried, CarriedLocals { dc: 7.0, aa: 0.0 });
        // Level==NLevel feeds SorbN/SorbN2.
        assert_eq!(l.blocking2.unwrap().ss, 20.0);
        assert_eq!(l.blocking1.unwrap().ss, 10.0);
        assert_eq!(z.omega, 0.0);
        assert!(z.prev_zero);
    }

    #[test]
    fn zero_code_defaults_to_one_only_with_positive_smax() {
        let ch = [row(0.0, 0.0, 9.0, 0.0)];
        let td = [0.0; 20];
        let (l, _) = new_level(&ch, &td, false, &node(20.0), Default::default()).unwrap();
        assert_eq!((l.i_psi2, l.i_psi1), (1, 0));
        assert!(l.blocking1.is_none() && l.blocking2.is_some());
        let s = l.finish(None, Some(0.25), None).unwrap();
        assert_eq!((s.psi1, s.psi2), (1.0, 0.25));
        assert!(l.finish(Some(0.5), Some(0.25), None).is_err());
    }

    #[test]
    fn non_final_level_uses_sorb_and_code_five_reads_aa() {
        let ch = [row(5.0, 5.0, 9.0, 8.0)];
        let td = [0.0; 20];
        let mut n = node(20.0);
        n.final_level = false;
        let (l, _) = new_level(&ch, &td, false, &n, Default::default()).unwrap();
        assert_eq!(l.carried, CarriedLocals { dc: 7.0, aa: 9.0 });
        assert_eq!(
            (l.blocking1.unwrap().ss, l.blocking2.unwrap().ss),
            (1.0, 2.0)
        );
    }

    #[test]
    fn filtration_reuses_slots_13_14_as_deposit_diameters() {
        let ch = [row(0.001, 0.0005, 0.0, 0.0)];
        let td = [0.0; 20];
        let (l, _) = new_level(&ch, &td, true, &node(20.0), Default::default()).unwrap();
        assert_eq!((l.i_psi1, l.i_psi2), (0, 0));
        let d = l.deposit.unwrap();
        assert_eq!((d.dc, d.dp, d.alfa1, d.alfa2), (0.001, 0.0005, 0.5, 0.2));
        assert_eq!((d.theta, d.q, d.temp), (0.4, 0.01, 20.0));
        assert!(l.finish(None, None, None).is_err());
        let s = l.finish(None, None, Some((1.5, 2.5))).unwrap();
        assert_eq!((s.r_ka1, s.r_ka2, s.psi1, s.psi2), (1.5, 2.5, 1.0, 1.0));
    }

    #[test]
    fn temperature_factor_scales_parameters() {
        let ch = [row(0.0, 0.0, 0.0, 0.0)];
        let mut td = [0.0; 20];
        td[18] = 5000.0; // slot 19, rKa1
        let t = 30.0_f32;
        let tt = (t + 273.15 - 293.15) / 8.314 / (t + 273.15) / 293.15;
        let (l, _) = new_level(&ch, &td, false, &node(t), Default::default()).unwrap();
        assert!((l.r_ka1 - 0.5 * (5000.0 * tt).exp()).abs() < 1e-6);
    }

    #[test]
    fn old_level_asymmetries_and_zeroing() {
        let ch = [row(4.0, 4.0, 9.0, 8.0)];
        let mut td = [0.0; 20];
        td[5] = 3000.0; // slot 6, Dc, must use the new-level TT
        let s = spec(&ch, &td, false);
        let (l, z) = select_old_level(&s, &old_node(), Default::default()).unwrap();
        // Old Blocking sees Sorb/Sorb2 even though final_level is true.
        assert_eq!(
            (l.blocking1.unwrap().ss, l.blocking2.unwrap().ss),
            (1.0, 2.0)
        );
        let tt = (30.0_f32 + 273.15 - 293.15) / 8.314 / (30.0 + 273.15) / 293.15;
        assert!((l.carried.dc - 7.0 * (3000.0 * tt).exp()).abs() < 1e-5);
        assert_eq!(z.gam_l1o, None);
        let sf = spec(&ch, &td, true);
        let (_, z) = select_old_level(&sf, &old_node(), Default::default()).unwrap();
        assert_eq!(z.gam_l1o, Some(0.0));
    }

    #[test]
    fn non_final_level_uses_temp_o_for_tt_but_temp_n_for_deposit() {
        let ch = [row(0.001, 0.0005, 0.0, 0.0)];
        let mut td = [0.0; 20];
        td[18] = 5000.0;
        let tf = |t: f32| (t + 273.15 - 293.15) / 8.314 / (t + 273.15) / 293.15;
        let n = BacteriaNode {
            temp_n: 30.0,
            temp_o: 10.0,
            final_level: false,
            ..node(0.0)
        };
        let (l, _) = new_level(&ch, &td, true, &n, Default::default()).unwrap();
        assert!((l.r_ka1 - 0.5 * (5000.0 * tf(10.0)).exp()).abs() < 1e-6);
        assert_eq!(l.deposit.unwrap().temp, 30.0);
        let fin = BacteriaNode {
            final_level: true,
            ..n
        };
        let (l, _) = new_level(&ch, &td, true, &fin, Default::default()).unwrap();
        assert!((l.r_ka1 - 0.5 * (5000.0 * tf(30.0)).exp()).abs() < 1e-6);
        // The old-level block cannot run at a non-final level.
        let bad = BacteriaNode {
            final_level: false,
            ..fin
        };
        assert!(select_old_level(&spec(&ch, &td, true), &bad, Default::default()).is_err());
    }

    #[test]
    fn carried_dc_persists_when_no_code_needs_it() {
        let ch = [row(1.0, 1.0, 9.0, 8.0)];
        let td = [0.0; 20];
        let c = CarriedLocals { dc: 4.0, aa: 6.0 };
        let (l, _) = new_level(&ch, &td, false, &node(20.0), c).unwrap();
        assert_eq!(l.carried, c);
        assert_eq!(l.blocking1.unwrap().dc, 4.0);
    }

    #[test]
    fn invalid_shapes_and_multispecies_rejected() {
        let ch = [row(1.0, 1.0, 9.0, 8.0)];
        let td = [0.0; 20];
        let mut s = spec(&ch, &td, false);
        s.species_count = 2;
        assert!(select_new_level(&s, &node(20.0), Default::default()).is_err());
        let short = [0.0; 5];
        assert!(new_level(&ch, &short, false, &node(20.0), Default::default()).is_err());
        let mut n = node(20.0);
        n.material = 3;
        assert!(new_level(&ch, &td, false, &n, Default::default()).is_err());
        // lFiltr without lBact is inert upstream; nothing here rejects the flag alone.
        assert!(new_level(&ch, &td, true, &node(20.0), Default::default()).is_ok());
    }
}

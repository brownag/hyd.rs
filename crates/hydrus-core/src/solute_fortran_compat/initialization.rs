//! Conditional source initializers: `MassInit` (`SOLUTE.FOR:1912-1975`) with
//! `cInit`/`SolMass`/`ZBRAK1`/`ZBRENT1` (`1956-2101`) and `NonEqInit`
//! (`1661-1706`), gated as in `HYDRUS.FOR:279-288`.
//!
//! Both calls are live: their argument records use column-6 continuation. All
//! arithmetic is default REAL (`f32`) in source order. `ChPar(k,M)` is
//! `ch_par[M][k-1]` and `TDep(k)` is `t_dep[k-1]`; materials and nodes are 0-based.
//! The source `PAUSE` paths (no bracket, unbracketed root, iteration limit) are
//! returned as `KernelError` instead of reading undefined values.

use super::types::KernelError;

const TR: f32 = 293.15;
const R: f32 = 8.314;

/// Material and species tables (`ChPar`, `TDep`, `lLinear`, `lMobIm`, `ThSat`).
#[derive(Clone, Debug)]
pub(crate) struct InitConfig {
    /// `NS`.
    pub species: usize,
    /// `NSD`, which fixes the row length `NSD*16+4`.
    pub species_dim: usize,
    /// `ChPar(:,M)` per material.
    pub ch_par: Vec<Vec<f32>>,
    pub t_dep: Vec<f32>,
    /// `lLinear(jS)`.
    pub linear: Vec<bool>,
    /// `lMobIm(M)`.
    pub mobile_immobile: Vec<bool>,
    /// `ThSat(M)`, read by `MassInit`.
    pub th_sat: Option<Vec<f32>>,
    /// `iDualPor`.
    pub dual_porosity: u8,
    /// `lBact`.
    pub bacteria: bool,
    /// `MatNum(i)`, 0-based.
    pub mat_num: Vec<usize>,
}

/// Node state. `conc`/`sorb`/`sorb2` are `[species][node]`.
#[derive(Clone, Debug)]
pub(crate) struct InitState {
    pub conc: Vec<Vec<f32>>,
    pub sorb: Option<Vec<Vec<f32>>>,
    pub sorb2: Option<Vec<Vec<f32>>>,
    /// `ThOld`.
    pub theta: Option<Vec<f32>>,
    /// `ThOldIm`.
    pub theta_im: Option<Vec<f32>>,
    /// `TempN`, read by `MassInit`.
    pub temp_new: Option<Vec<f32>>,
    /// `TempO`, read by `NonEqInit`.
    pub temp_old: Option<Vec<f32>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InitFlags {
    /// `lMassIni`.
    pub mass_ini: bool,
    /// `lEqInit`.
    pub eq_init: bool,
    /// `lEquil` as read, before the `iDualPor` override.
    pub equil: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InitCalls {
    pub mass_init: bool,
    pub non_eq_init: bool,
}

fn nodes(cfg: &InitConfig) -> usize {
    cfg.mat_num.len()
}

fn check_len<T>(v: &Option<Vec<T>>, n: usize, e: KernelError) -> Result<(), KernelError> {
    match v {
        Some(v) if v.len() == n => Ok(()),
        _ => Err(e),
    }
}

fn check_grid(g: &[Vec<f32>], ns: usize, n: usize, e: KernelError) -> Result<(), KernelError> {
    if g.len() != ns
        || g.iter()
            .any(|r| r.len() != n || r.iter().any(|x| !x.is_finite()))
    {
        return Err(e);
    }
    Ok(())
}

fn finite(v: &Option<Vec<f32>>, e: KernelError) -> Result<(), KernelError> {
    if v.as_ref().is_some_and(|v| v.iter().all(|x| x.is_finite())) {
        Ok(())
    } else {
        Err(e)
    }
}

fn validate_common(cfg: &InitConfig, st: &InitState) -> Result<(), KernelError> {
    let (ns, n) = (cfg.species, nodes(cfg));
    let nmat = cfg.ch_par.len();
    let row = cfg.species_dim * 16 + 4;
    if ns == 0 || ns > cfg.species_dim || n == 0 || nmat == 0 {
        return Err("initialization dimensions are inconsistent");
    }
    if cfg.linear.len() != ns || cfg.mobile_immobile.len() != nmat || cfg.t_dep.len() != row {
        return Err("initialization species or material tables have the wrong length");
    }
    if cfg
        .ch_par
        .iter()
        .any(|r| r.len() != row || r.iter().any(|x| !x.is_finite()))
        || cfg.t_dep.iter().any(|x| !x.is_finite())
    {
        return Err("initialization ChPar/TDep rows are malformed");
    }
    if cfg.mat_num.iter().any(|&m| m >= nmat) {
        return Err("initialization MatNum is out of range");
    }
    check_grid(&st.conc, ns, n, "initialization Conc shape is wrong")
}

fn validate_mass(cfg: &InitConfig, st: &InitState) -> Result<(), KernelError> {
    let n = nodes(cfg);
    check_len(&st.temp_new, n, "MassInit requires TempN")?;
    check_len(&st.theta, n, "MassInit requires ThOld")?;
    check_len(&st.theta_im, n, "MassInit requires ThOldIm")?;
    check_len(
        &cfg.th_sat,
        cfg.ch_par.len(),
        "MassInit requires ThSat per material",
    )?;
    finite(&st.temp_new, "MassInit TempN is not finite")?;
    finite(&st.theta, "MassInit ThOld is not finite")?;
    finite(&st.theta_im, "MassInit ThOldIm is not finite")?;
    finite(&cfg.th_sat, "MassInit ThSat is not finite")
}

fn validate_non_eq(cfg: &InitConfig, st: &InitState) -> Result<(), KernelError> {
    let (ns, n) = (cfg.species, nodes(cfg));
    check_len(&st.temp_old, n, "NonEqInit requires TempO")?;
    finite(&st.temp_old, "NonEqInit TempO is not finite")?;
    match &st.sorb {
        Some(s) => check_grid(s, ns, n, "NonEqInit Sorb shape is wrong")?,
        None => return Err("NonEqInit requires Sorb"),
    }
    if cfg.bacteria {
        check_len(&st.theta, n, "NonEqInit with bacteria requires ThOld")?;
        finite(&st.theta, "NonEqInit ThOld is not finite")?;
        match &st.sorb2 {
            Some(s) => check_grid(s, ns, n, "NonEqInit Sorb2 shape is wrong")?,
            None => return Err("NonEqInit with bacteria requires Sorb2"),
        }
    }
    Ok(())
}

/// `SolMass`, `SOLUTE.FOR:1976-1995`. `par` is `Par(1..8)`.
fn sol_mass(conc: f32, x_mass: f32, par: &[f32; 8]) -> f32 {
    let (ro, xks, xnu, fexp, xkh, theta, theta_a) =
        (par[0], par[2], par[3], par[4], par[5], par[6], par[7]);
    let y_mass = theta * conc
        + ro * xks * conc.powf(fexp) / (1. + xnu * conc.powf(fexp))
        + theta_a * xkh * conc;
    x_mass - y_mass
}

/// `ZBRAK1`, `SOLUTE.FOR:2002-2032`, called with `X1=1e-3`, `X2=1e3`.
fn zbrak1(x_mass: f32, par: &[f32; 8]) -> Result<(f32, f32), KernelError> {
    let (mut x1, x2): (f32, f32) = (1.0e-3, 1.0e3);
    let nb = 1000;
    let x1o = x1;
    let dlh = (x2.log10() - x1.log10()) / ((nb - 1) as f32);
    let mut fp = sol_mass(x1, x_mass, par);
    for i in 1..=nb {
        let dx2 = x1o.log10() + (i as f32) * dlh;
        let x2n = 10.0f32.powf(dx2);
        let fc = sol_mass(x2n, x_mass, par);
        if fc * fp < 0. {
            return Ok((x1, x2n));
        }
        fp = fc;
        x1 = x2n;
    }
    Err("ZBRAK1 found no bracket for the total mass")
}

/// `ZBRENT1`, `SOLUTE.FOR:2035-2101`.
fn zbrent1(x1: f32, x2: f32, x_mass: f32, par: &[f32; 8]) -> Result<f32, KernelError> {
    const ITMAX: usize = 100;
    const EPS: f32 = 3.0e-8;
    const TOL: f32 = 1.0e-6;
    let (mut a, mut b) = (x1, x2);
    let (mut fa, mut fb) = (sol_mass(a, x_mass, par), sol_mass(b, x_mass, par));
    if fb * fa > 0. {
        return Err("ZBRENT1 root is not bracketed");
    }
    let mut c = b;
    let mut fc = fb;
    let (mut d, mut e) = (0.0f32, 0.0f32);
    for _ in 0..ITMAX {
        if fb * fc > 0. {
            c = a;
            fc = fa;
            d = b - a;
            e = d;
        }
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        let tol1 = 2. * EPS * b.abs() + 0.5 * TOL;
        let xm = 0.5 * (c - b);
        if xm.abs() <= tol1 || fb == 0. {
            return Ok(b);
        }
        if e.abs() >= tol1 && fa.abs() > fb.abs() {
            let s = fb / fa;
            let (mut p, mut q);
            if a == c {
                p = 2. * xm * s;
                q = 1. - s;
            } else {
                q = fa / fc;
                let r = fb / fc;
                p = s * (2. * xm * q * (q - r) - (b - a) * (r - 1.));
                q = (q - 1.) * (r - 1.) * (s - 1.);
            }
            if p > 0. {
                q = -q;
            }
            p = p.abs();
            let m = (3. * xm * q - (tol1 * q).abs()).min((e * q).abs());
            if 2. * p < m {
                e = d;
                d = p / q;
            } else {
                d = xm;
                e = d;
            }
        } else {
            d = xm;
            e = d;
        }
        a = b;
        fa = fb;
        if d.abs() > tol1 {
            b += d;
        } else {
            b += tol1.copysign(xm);
        }
        fb = sol_mass(b, x_mass, par);
    }
    Err("ZBRENT1 exceeded its maximum iterations")
}

/// `cInit`, `SOLUTE.FOR:1958-1971`.
fn c_init(x_mass: f32, par: &[f32; 8]) -> Result<f32, KernelError> {
    let (xb1, xb2) = zbrak1(x_mass, par)?;
    zbrent1(xb1, xb2, x_mass, par)
}

fn tt_factor(temp: f32) -> f32 {
    (temp + 273.15 - TR) / R / (temp + 273.15) / TR
}

/// `MassInit` on cloned `conc`; the caller commits.
#[allow(clippy::needless_range_loop)] // index loops mirror the source jS/i traversal
fn mass_init(cfg: &InitConfig, st: &InitState, conc: &mut [Vec<f32>]) -> Result<(), KernelError> {
    let (temp, theta, theta_im, th_sat) = (
        st.temp_new.as_ref().unwrap(),
        st.theta.as_ref().unwrap(),
        st.theta_im.as_ref().unwrap(),
        cfg.th_sat.as_ref().unwrap(),
    );
    for js in 0..cfg.species {
        let jjj = js * 16;
        for i in 0..nodes(cfg) {
            if conc[js][i] > 0. {
                let m = cfg.mat_num[i];
                let tt = tt_factor(temp[i]);
                let cp = &cfg.ch_par[m];
                let td = &cfg.t_dep;
                let mut par = [0.0f32; 8];
                par[0] = cp[0] * (td[0] * tt).exp();
                par[1] = cp[2] * (td[2] * tt).exp();
                par[2] = cp[jjj + 6] * (td[jjj + 6] * tt).exp();
                par[3] = cp[jjj + 7] * (td[jjj + 7] * tt).exp();
                par[4] = cp[jjj + 8];
                // Source quirk: xKH uses TDep(jjj+7), the xKs factor.
                par[5] = cp[jjj + 9] * (td[jjj + 6] * tt).exp();
                par[6] = theta[i] + theta_im[i];
                par[7] = th_sat[m] - par[6];
                if cfg.bacteria {
                    let rka1 = cp[jjj + 18] * (td[jjj + 18] * tt).exp();
                    let rkd1 = cp[jjj + 19] * (td[jjj + 19] * tt).exp();
                    let xks1 = theta[i] * rka1 / par[0] / rkd1;
                    // Source quirk: xKs2 reuses rKa1/rKd1 (rKa2/rKd2 are unused).
                    let xks2 = theta[i] * rka1 / par[0] / rkd1;
                    par[2] = xks1 + xks2;
                }
                conc[js][i] = if cfg.linear[js] {
                    conc[js][i] / (par[6] + par[0] * par[2] + par[7] * par[5])
                } else {
                    c_init(conc[js][i], &par)?
                };
            }
        }
    }
    Ok(())
}

/// `NonEqInit` on cloned pools; reads the (already `MassInit`-converted) `conc`.
#[allow(clippy::needless_range_loop)] // index loops mirror the source jS/i traversal
fn non_eq_init(
    cfg: &InitConfig,
    st: &InitState,
    conc: &[Vec<f32>],
    sorb: &mut [Vec<f32>],
    sorb2: &mut Option<Vec<Vec<f32>>>,
) {
    let temp = st.temp_old.as_ref().unwrap();
    for js in 0..cfg.species {
        let jjj = js * 16;
        for i in 0..nodes(cfg) {
            let m = cfg.mat_num[i];
            let tt = tt_factor(temp[i]);
            if cfg.mobile_immobile[m] || cfg.dual_porosity > 0 {
                sorb[js][i] = conc[js][i];
            } else {
                let cp = &cfg.ch_par[m];
                let td = &cfg.t_dep;
                let ro = cp[0] * (td[0] * tt).exp();
                let frac = cp[2] * (td[2] * tt).exp();
                let xks = cp[jjj + 6] * (td[jjj + 6] * tt).exp();
                let xnu = cp[jjj + 7] * (td[jjj + 7] * tt).exp();
                let fexp = cp[jjj + 8];
                let mut sconc = 1.0f32;
                let cc = conc[js][i];
                if !cfg.linear[js] && cc > 0. {
                    sconc = cc.powf(fexp - 1.) / (1. + xnu * cc.powf(fexp));
                }
                sorb[js][i] = (1. - frac) * sconc * xks * cc;
                if cfg.bacteria {
                    // Source sets Frac=0 here, but it is not read again.
                    let theta = st.theta.as_ref().unwrap();
                    let rka1 = cp[jjj + 18] * (td[jjj + 18] * tt).exp();
                    let rkd1 = cp[jjj + 19] * (td[jjj + 19] * tt).exp();
                    let xks1 = theta[i] * rka1 / ro / rkd1;
                    sorb[js][i] = xks1 * conc[js][i];
                    // Source quirk: xKs2 reuses rKa1/rKd1.
                    let xks2 = theta[i] * rka1 / ro / rkd1;
                    sorb2.as_mut().unwrap()[js][i] = xks2 * conc[js][i];
                }
            }
        }
    }
}

/// `HYDRUS.FOR:279-288`: `MassInit` first, then `NonEqInit`.
///
/// `iDualPor>0` forces `lEquil` false. All selected state is validated first and
/// the pools are committed only after both calls succeed.
pub(crate) fn initialize_sources(
    cfg: &InitConfig,
    st: &mut InitState,
    flags: InitFlags,
) -> Result<InitCalls, KernelError> {
    let equil = flags.equil && cfg.dual_porosity == 0;
    let calls = InitCalls {
        mass_init: flags.mass_ini,
        non_eq_init: !equil && (flags.eq_init || flags.mass_ini),
    };
    if !calls.mass_init && !calls.non_eq_init {
        return Ok(calls);
    }
    validate_common(cfg, st)?;
    if calls.mass_init {
        validate_mass(cfg, st)?;
    }
    if calls.non_eq_init {
        validate_non_eq(cfg, st)?;
    }
    let mut conc = st.conc.clone();
    let mut sorb = st.sorb.clone();
    let mut sorb2 = st.sorb2.clone();
    if calls.mass_init {
        mass_init(cfg, st, &mut conc)?;
    }
    if calls.non_eq_init {
        non_eq_init(cfg, st, &conc, sorb.as_mut().unwrap(), &mut sorb2);
    }
    st.conc = conc;
    st.sorb = sorb;
    st.sorb2 = sorb2;
    Ok(calls)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T20: f32 = 20.0;

    fn fixture(ns: usize, nodes: usize) -> (InitConfig, InitState) {
        let nsd = ns;
        let row = nsd * 16 + 4;
        let mut cp = vec![0.0f32; row];
        cp[0] = 1.5; // ro
        cp[2] = 0.25; // frac
        for js in 0..ns {
            cp[js * 16 + 6] = 2.0 + js as f32; // xKs
            cp[js * 16 + 7] = 0.0; // xNu
            cp[js * 16 + 8] = 1.0; // fExp
            cp[js * 16 + 9] = 0.0; // xKH
        }
        let cfg = InitConfig {
            species: ns,
            species_dim: nsd,
            ch_par: vec![cp],
            t_dep: vec![0.0; row],
            linear: vec![true; ns],
            mobile_immobile: vec![false],
            th_sat: Some(vec![0.5]),
            dual_porosity: 0,
            bacteria: false,
            mat_num: vec![0; nodes],
        };
        let st = InitState {
            conc: vec![vec![10.0; nodes]; ns],
            sorb: Some(vec![vec![-1.0; nodes]; ns]),
            sorb2: None,
            theta: Some(vec![0.3; nodes]),
            theta_im: Some(vec![0.1; nodes]),
            temp_new: Some(vec![T20; nodes]),
            temp_old: Some(vec![T20; nodes]),
        };
        (cfg, st)
    }

    fn fl(m: bool, e: bool, q: bool) -> InitFlags {
        InitFlags {
            mass_ini: m,
            eq_init: e,
            equil: q,
        }
    }

    fn near(a: f32, b: f32) {
        assert!((a - b).abs() <= 1e-4 * b.abs().max(1.0), "{a} vs {b}");
    }

    #[test]
    fn linear_mass_init_divides_total_mass() {
        let (cfg, mut st) = fixture(1, 2);
        let calls = initialize_sources(&cfg, &mut st, fl(true, false, true)).unwrap();
        assert_eq!(
            calls,
            InitCalls {
                mass_init: true,
                non_eq_init: false
            }
        );
        near(st.conc[0][0], 10.0 / (0.4 + 1.5 * 2.0));
        assert_eq!(st.sorb.as_ref().unwrap()[0][0], -1.0);
    }

    #[test]
    fn nonlinear_mass_init_solves_root_and_matches_linear_limit() {
        let (mut cfg, mut st) = fixture(1, 1);
        cfg.linear[0] = false; // fExp=1, xNu=0 reduces to the linear law
        initialize_sources(&cfg, &mut st, fl(true, false, true)).unwrap();
        near(st.conc[0][0], 10.0 / 3.4);
        let (mut cfg, mut st) = fixture(1, 1);
        cfg.linear[0] = false;
        cfg.ch_par[0][7] = 0.5;
        cfg.ch_par[0][8] = 0.8;
        initialize_sources(&cfg, &mut st, fl(true, false, true)).unwrap();
        let par = [1.5, 0.25, 2.0, 0.5, 0.8, 0.0, 0.4, 0.1];
        assert!(sol_mass(st.conc[0][0], 10.0, &par).abs() < 1e-3);
    }

    #[test]
    fn root_solve_failures_are_errors() {
        let par = [1.5, 0.25, 2.0, 0.0, 1.0, 0.0, 0.4, 0.1];
        assert!(c_init(1.0e9, &par).is_err());
        assert!(zbrent1(1.0, 2.0, 1.0e9, &par).is_err());
        let (mut cfg, mut st) = fixture(1, 1);
        cfg.linear[0] = false;
        st.conc[0][0] = 1.0e9;
        let before = st.conc.clone();
        assert!(initialize_sources(&cfg, &mut st, fl(true, false, true)).is_err());
        assert_eq!(st.conc, before);
    }

    #[test]
    fn gating_matrix() {
        let run = |m, e, q, dp: u8| {
            let (mut cfg, mut st) = fixture(1, 1);
            cfg.dual_porosity = dp;
            initialize_sources(&cfg, &mut st, fl(m, e, q)).unwrap()
        };
        let c = |mass_init, non_eq_init| InitCalls {
            mass_init,
            non_eq_init,
        };
        assert_eq!(run(false, false, false, 0), c(false, false));
        assert_eq!(run(true, false, false, 0), c(true, true));
        assert_eq!(run(true, false, true, 0), c(true, false));
        assert_eq!(run(false, true, false, 0), c(false, true));
        assert_eq!(run(false, true, true, 0), c(false, false));
        // iDualPor>0 forces lEquil false, so NonEqInit runs even with lEquil read true.
        assert_eq!(run(false, true, true, 1), c(false, true));
    }

    #[test]
    fn non_eq_init_sees_converted_conc_and_only_it_mutates_sorb() {
        let (cfg, mut st) = fixture(1, 1);
        initialize_sources(&cfg, &mut st, fl(true, false, false)).unwrap();
        let c = 10.0f32 / (0.4 + 1.5 * 2.0);
        near(st.conc[0][0], c);
        near(st.sorb.as_ref().unwrap()[0][0], (1. - 0.25) * 2.0 * c);
        let (cfg, mut st) = fixture(1, 1);
        initialize_sources(&cfg, &mut st, fl(false, true, false)).unwrap();
        assert_eq!(st.conc[0][0], 10.0);
        near(st.sorb.as_ref().unwrap()[0][0], 0.75 * 2.0 * 10.0);
    }

    #[test]
    fn species_and_material_traversal() {
        let (mut cfg, mut st) = fixture(2, 3);
        let row = cfg.ch_par[0].clone();
        cfg.ch_par.push(row);
        cfg.mobile_immobile = vec![false, true];
        cfg.mat_num = vec![0, 1, 0];
        cfg.linear = vec![true, false];
        cfg.th_sat = Some(vec![0.5, 0.5]);
        st.temp_old = Some(vec![20.0, 30.0, 20.0]);
        initialize_sources(&cfg, &mut st, fl(false, true, false)).unwrap();
        let sorb = st.sorb.as_ref().unwrap();
        near(sorb[0][0], 0.75 * 2.0 * 10.0);
        assert_eq!(sorb[0][1], 10.0); // lMobIm material: Sorb = Conc
        near(sorb[1][0], 0.75 * 3.0 * 10.0); // fExp=1, xNu=0: SConc = 1
        assert_eq!(sorb[1][1], 10.0);
        near(sorb[0][2], 0.75 * 2.0 * 10.0);
    }

    #[test]
    fn bacteria_branch_sets_both_pools_with_shared_rate_quirk() {
        let (mut cfg, mut st) = fixture(1, 1);
        cfg.bacteria = true;
        cfg.ch_par[0][18] = 2.0; // rKa1
        cfg.ch_par[0][19] = 0.5; // rKd1
        cfg.ch_par[0][15] = 9.0; // rKa2 (unused by the source formula)
        cfg.ch_par[0][16] = 9.0; // rKd2
        st.sorb2 = Some(vec![vec![-1.0]]);
        initialize_sources(&cfg, &mut st, fl(false, true, false)).unwrap();
        let xks = 0.3 * 2.0 / 1.5 / 0.5;
        near(st.sorb.as_ref().unwrap()[0][0], xks * 10.0);
        near(st.sorb2.as_ref().unwrap()[0][0], xks * 10.0);
    }

    #[test]
    fn nonpositive_conc_is_skipped_by_mass_init() {
        let (cfg, mut st) = fixture(1, 2);
        st.conc[0][1] = 0.0;
        initialize_sources(&cfg, &mut st, fl(true, false, true)).unwrap();
        assert_eq!(st.conc[0][1], 0.0);
    }

    #[test]
    fn invalid_state_is_rejected_without_mutation() {
        let flags = fl(true, false, false);
        let check = |f: &dyn Fn(&mut InitConfig, &mut InitState)| {
            let (mut cfg, mut st) = fixture(1, 2);
            f(&mut cfg, &mut st);
            let before = format!("{:?}", (&st.conc, &st.sorb));
            assert!(initialize_sources(&cfg, &mut st, flags).is_err());
            assert_eq!(format!("{:?}", (&st.conc, &st.sorb)), before);
        };
        check(&|_, s| {
            s.conc[0].pop();
        });
        check(&|c, _| c.mat_num[1] = 5);
        check(&|_, s| s.theta = None);
        check(&|_, s| s.theta_im = None);
        check(&|c, _| c.th_sat = None);
        check(&|_, s| s.temp_new = None);
        check(&|_, s| s.temp_old = None);
        check(&|_, s| s.sorb = None);
        check(&|_, s| s.conc[0][0] = f32::NAN);
        check(&|c, _| {
            c.ch_par[0].pop();
        });
        check(&|c, s| {
            c.bacteria = true;
            s.sorb2 = None;
        });
    }

    #[test]
    fn disabled_calls_need_no_state() {
        let (cfg, mut st) = fixture(1, 1);
        st.theta = None;
        st.sorb = None;
        assert!(initialize_sources(&cfg, &mut st, fl(false, false, false)).is_ok());
        let (mut cfg, mut st) = fixture(1, 1);
        st.theta = None;
        st.theta_im = None;
        cfg.th_sat = None;
        assert!(initialize_sources(&cfg, &mut st, fl(false, true, false)).is_ok());
    }
}

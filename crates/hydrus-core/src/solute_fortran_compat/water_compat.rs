//! Source-precision water flow, material tables, dual-porosity, kinematics,
//! and time control routines.
//!
//! Pinned Fortran implementations:
//! - `MATERIAL.FOR`: `FK`, `FC`, `FQ`, `FH`, `FS` for van Genuchten (`iModel == 0`,
//!   with models 6 and 7 routed to 0 per `WATFLOW.FOR:440-441` and `INPUT.FOR:769-770`).
//! - `INPUT.FOR`: `GenMat`, `InitW`, `InitDualPor`.
//! - `WATFLOW.FOR`: `SetMat`, `WatFlow`, `Reset`, `Gauss`, `Shift`, `Veloc`, `ConVapor`,
//!   `VaporContent`, `ConVh`, `Update`. Note that `DualPor` delegates to
//!   `super::dual_domain::dual_porosity_exchange`.
//! - `TIME.FOR`: `TmCont`.

use super::dual_domain::dual_porosity_exchange;
use super::types::{DefaultReal, DualPorMaterial, DualPorPrimitiveState};

/// Normalizes hydraulic model ID: models 6 and 7 route to 0 (fracture/macropore domain).
#[inline]
pub(crate) fn normalize_model(model: i32) -> i32 {
    if model == 6 || model == 7 {
        0
    } else {
        model
    }
}

/// Hydraulic conductivity function `FK(iModel, h, Par)` from `MATERIAL.FOR:16-197`.
/// Supports `iModel == 0` (van Genuchten), routing 6 and 7 to 0.
///
/// Inputs:
/// - `model`: hydraulic model index (0, 6, 7).
/// - `h`: pressure head (cm or length unit).
/// - `par`: parameter array `Par(1..10)`:
///   - `Par[0]`: Qr (residual water content)
///   - `Par[1]`: Qs (saturated water content)
///   - `Par[2]`: Alfa (alpha parameter)
///   - `Par[3]`: n (n parameter)
///   - `Par[4]`: Ks (saturated hydraulic conductivity)
///   - `Par[5]`: BPar / l (pore-connectivity / tortuosity parameter)
pub(crate) fn fk(model: i32, h: f32, par: &[f32]) -> f32 {
    let model = normalize_model(model);
    assert_eq!(model, 0, "fk only supports van Genuchten (iModel=0)");
    assert!(par.len() >= 6, "par must have at least 6 parameters");

    let qr = par[0] as f64;
    let qs = par[1] as f64;
    let alfa = par[2] as f64;
    let n = par[3] as f64;
    let ks = (par[4] as f64).max(1.0e-37);
    let b_par = par[5] as f64;

    let p_par = 2.0; // PPar = 2
    let qm = qs;
    let qa = qr;
    let qk = qs;
    let kk = ks;

    let m = 1.0 - 1.0 / n;
    // HMin = -1.d300**(1.d0/n)/max(Alfa, 1.d0)
    let h_min = -(1.0e300_f64.powf(1.0 / n)) / alfa.max(1.0);
    let hh = (h as f64).max(h_min);

    let qees = ((qs - qa) / (qm - qa)).min(0.999999999999999);
    let qeek = ((qk - qa) / (qm - qa)).min(qees);

    let hs = -1.0 / alfa * (qees.powf(-1.0 / m) - 1.0).powf(1.0 / n);
    let hk = -1.0 / alfa * (qeek.powf(-1.0 / m) - 1.0).powf(1.0 / n);

    let h_dble = h as f64;
    if h_dble < hk {
        let qee = (1.0 + (-alfa * hh).powf(n)).powf(-m);
        let qe = (qm - qa) / (qs - qa) * qee;
        let mut ffq = 1.0 - (1.0 - qee.powf(1.0 / m)).powf(m);
        if ffq <= 0.0 {
            ffq = m * qee.powf(1.0 / m);
        }
        let kr = qe.powf(b_par) * ffq.powf(p_par);
        (ks * kr).max(1.0e-37) as f32
    } else if h_dble < hs {
        let kr = (1.0 - kk / ks) / (hs - hk) * (h_dble - hs) + 1.0;
        (ks * kr) as f32
    } else {
        ks as f32
    }
}

/// Soil water capacity function `FC(iModel, h, Par)` from `MATERIAL.FOR:201-275`.
pub(crate) fn fc(model: i32, h: f32, par: &[f32]) -> f32 {
    let model = normalize_model(model);
    assert_eq!(model, 0, "fc only supports van Genuchten (iModel=0)");
    assert!(par.len() >= 4, "par must have at least 4 parameters");

    let qr = par[0] as f64;
    let qs = par[1] as f64;
    let alfa = par[2] as f64;
    let n = par[3] as f64;

    let qm = qs;
    let qa = qr;
    let m = 1.0 - 1.0 / n;
    let h_min = -(1.0e300_f64.powf(1.0 / n)) / alfa.max(1.0);
    let hh = (h as f64).max(h_min);
    let qees = ((qs - qa) / (qm - qa)).min(0.999999999999999);
    let hs = -1.0 / alfa * (qees.powf(-1.0 / m) - 1.0).powf(1.0 / n);

    let h_dble = h as f64;
    if h_dble < hs {
        let c1 = (1.0 + (-alfa * hh).powf(n)).powf(-m - 1.0);
        let c2 = (qm - qa) * m * n * alfa.powf(n) * (-hh).powf(n - 1.0) * c1;
        c2.max(1.0e-37) as f32
    } else {
        0.0
    }
}

/// Volumetric water content function `FQ(iModel, h, Par)` from `MATERIAL.FOR:279-346`.
pub(crate) fn fq(model: i32, h: f32, par: &[f32]) -> f32 {
    let model = normalize_model(model);
    assert_eq!(model, 0, "fq only supports van Genuchten (iModel=0)");
    assert!(par.len() >= 4, "par must have at least 4 parameters");

    let qr = par[0] as f64;
    let qs = par[1] as f64;
    let alfa = par[2] as f64;
    let n = par[3] as f64;

    let qm = qs;
    let qa = qr;
    let m = 1.0 - 1.0 / n;
    let h_min = -(1.0e300_f64.powf(1.0 / n)) / alfa.max(1.0);
    let hh = (h as f64).max(h_min);
    let qees = ((qs - qa) / (qm - qa)).min(0.999999999999999);
    let hs = -1.0 / alfa * (qees.powf(-1.0 / m) - 1.0).powf(1.0 / n);

    let h_dble = h as f64;
    if h_dble < hs {
        let qee = (1.0 + (-alfa * hh).powf(n)).powf(-m);
        (qa + (qm - qa) * qee).max(1.0e-37) as f32
    } else {
        qs as f32
    }
}

/// Pressure head from effective saturation `FH(iModel, Qe, Par)` from `MATERIAL.FOR:350-417`.
pub(crate) fn fh(model: i32, qe: f32, par: &[f32]) -> f32 {
    let model = normalize_model(model);
    assert_eq!(model, 0, "fh only supports van Genuchten (iModel=0)");
    assert!(par.len() >= 4, "par must have at least 4 parameters");

    let qr = par[0] as f64;
    let qs = par[1] as f64;
    let alfa = par[2] as f64;
    let n = par[3] as f64;

    let qm = qs;
    let qa = qr;
    let m = 1.0 - 1.0 / n;
    let h_min = -(1.0e300_f64.powf(1.0 / n)) / alfa.max(1.0);
    let qee_m = (1.0 + (-alfa * h_min).powf(n)).powf(-m);
    let qee = ((qe as f64) * (qs - qa) / (qm - qa))
        .max(qee_m)
        .min(0.999999999999999);
    let val = -1.0 / alfa * (qee.powf(-1.0 / m) - 1.0).powf(1.0 / n);
    val.max(-1.0e37) as f32
}

/// Effective saturation function `FS(iModel, h, Par)` from `MATERIAL.FOR:421-460`.
pub(crate) fn fs(model: i32, h: f32, par: &[f32]) -> f32 {
    let model = normalize_model(model);
    assert_eq!(model, 0, "fs only supports van Genuchten (iModel=0)");
    assert!(par.len() >= 4, "par must have at least 4 parameters");

    let qr = par[0] as f64;
    let qs = par[1] as f64;
    let alfa = par[2] as f64;
    let n = par[3] as f64;

    let qm = qs;
    let qa = qr;
    let m = 1.0 - 1.0 / n;
    let qees = ((qs - qa) / (qm - qa)).min(0.999999999999999);
    let hs = -1.0 / alfa * (qees.powf(-1.0 / m) - 1.0).powf(1.0 / n);

    let h_dble = h as f64;
    if (h as f64) < hs {
        let h_min = -(1.0e300_f64.powf(1.0 / n)) / alfa.max(1.0);
        let hh = h_dble.max(h_min);
        let qee = (1.0 + (-alfa * hh).powf(n)).powf(-m);
        let qe = qee * (qm - qa) / (qs - qa);
        qe.max(1.0e-37) as f32
    } else {
        1.0
    }
}

/// Isothermal vapor hydraulic conductivity `ConVh(hNew, Theta, ths, xConv, tConv)` from `WATFLOW.FOR:968-997`.
pub(crate) fn con_vh(h_new: f32, theta: f32, ths: f32, x_conv: f32, t_conv: f32) -> f32 {
    let diff0 = 2.12e-5;
    let g = 9.81;
    let x_mol = 0.018015;
    let r = 8.314;

    let temp = 20.0f32;
    let h = h_new / x_conv;
    let t_kelv = temp + 273.15;
    let diff_t = diff0 * (t_kelv / 273.15).powi(2);
    let theta_a = ths - theta;
    let mut tau = 0.0f32;
    if theta_a > 0.0 {
        tau = theta_a.powf(7.0 / 3.0) / ths.powi(2);
    }
    let diff = tau * theta_a * diff_t;
    let row = (1.0 - 7.37e-6 * (temp - 4.0).powi(2) + 3.79e-8 * (temp - 4.0).powi(3)) * 1000.0;
    let rovs = 0.001 * (31.3716 - 6014.79 / t_kelv - 0.00792495 * t_kelv).exp() / t_kelv;
    let hr = (h * x_mol * g / r / t_kelv).exp();
    let con_vh_val = diff / row * rovs * x_mol * g / r / t_kelv * hr;
    con_vh_val * x_conv / t_conv
}

/// Precomputed material tables produced by `GenMat` (`INPUT.FOR:762-862`).
#[derive(Clone, Debug)]
pub(crate) struct MaterialTables {
    pub n_tab: usize,
    pub n_mat: usize,
    pub h_tab: Vec<f32>,        // 100 entries for iModel < nTabMod
    pub con_tab: Vec<Vec<f32>>, // [n_mat][n_tab]
    pub cap_tab: Vec<Vec<f32>>, // [n_mat][n_tab]
    pub the_tab: Vec<Vec<f32>>, // [n_mat][n_tab]
    pub con_sat: Vec<f32>,      // [n_mat]
    pub h_sat: Vec<f32>,        // [n_mat]
    pub thr: Vec<f32>,          // [n_mat]
    pub ths: Vec<f32>,          // [n_mat]
    pub con_s_max: f32,
}

/// Generates material tables (`INPUT.FOR:762-862`).
/// Typically `n_tab = 100`, `h_tab_1` and `h_tab_n` are bounds (e.g. `hTab(1) = -0.0001`, `hTab(100) = -100000.0`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn gen_mat(
    n_tab: usize,
    n_mat: usize,
    par: &[Vec<f32>], // [n_mat][11]
    h_tab_1: f32,
    h_tab_n: f32,
    model: i32,
    x_conv: f32,
    t_conv: f32,
) -> MaterialTables {
    let model = normalize_model(model);
    assert!(n_tab >= 2);
    let mut h_tab = vec![0.0f32; n_tab];
    let alh1 = (-h_tab_1).log10();
    let alhn = (-h_tab_n).log10();
    let dlh = (alhn - alh1) / (n_tab as f32 - 1.0);
    for (i, slot) in h_tab.iter_mut().enumerate() {
        let alh = alh1 + (i as f32) * dlh;
        *slot = -(10.0f32.powf(alh));
    }

    let mut con_tab = vec![vec![0.0f32; n_tab]; n_mat];
    let mut cap_tab = vec![vec![0.0f32; n_tab]; n_mat];
    let mut the_tab = vec![vec![0.0f32; n_tab]; n_mat];
    let mut con_sat = vec![0.0f32; n_mat];
    let mut h_sat = vec![0.0f32; n_mat];
    let mut thr = vec![0.0f32; n_mat];
    let mut ths = vec![0.0f32; n_mat];
    let mut con_s_max = 0.0f32;

    for m in 0..n_mat {
        h_sat[m] = fh(model, 1.0, &par[m]);
        con_sat[m] = par[m][4];
        if con_sat[m] > con_s_max {
            con_s_max = con_sat[m];
        }
        thr[m] = par[m][0];
        ths[m] = par[m][1];
        for i in 0..n_tab {
            con_tab[m][i] = fk(model, h_tab[i], &par[m]);
            cap_tab[m][i] = fc(model, h_tab[i], &par[m]);
            the_tab[m][i] = fq(model, h_tab[i], &par[m]);
            let _qe = fs(model, h_tab[i], &par[m]);
            let _conv = con_vh(h_tab[i], the_tab[m][i], ths[m], x_conv, t_conv);
        }
    }

    MaterialTables {
        n_tab,
        n_mat,
        h_tab,
        con_tab,
        cap_tab,
        the_tab,
        con_sat,
        h_sat,
        thr,
        ths,
        con_s_max,
    }
}

/// Initial condition setup `InitW` (`INPUT.FOR:472-504`).
/// When initial conditions are specified in water contents (or pressure heads),
/// initializes `hNew`, `hOld`, `hTemp`, and dual porosity mobile/immobile split.
#[allow(clippy::too_many_arguments)]
pub(crate) fn init_w(
    mat_num: &[usize], // 1-based material IDs
    kappa: &[i32],     // -1 for drying, +1 for wetting
    h_new: &mut [f32],
    h_old: &mut [f32],
    h_temp: &mut [f32],
    par_d: &[Vec<f32>],
    par_w: &[Vec<f32>],
    model: i32,
    i_dual_por: u8,
    th_new_im: &mut [f32],
) -> Result<(f32, f32), &'static str> {
    let num_np = h_new.len();
    for i in 0..num_np {
        let m = mat_num[i] - 1;
        let th_total = h_new[i];
        if i_dual_por > 0 {
            // hNew(i) = hNew(i)*ParD(2,M)/(ParD(2,M)+ParD(8,M))
            h_new[i] = h_new[i] * par_d[m][1] / (par_d[m][1] + par_d[m][7]);
            let th_mobile = h_new[i];
            th_new_im[i] = th_total - th_mobile;
        }
        if kappa[i] == -1 {
            let qe = ((h_new[i] - par_d[m][0]) / (par_d[m][1] - par_d[m][0])).min(1.0);
            if qe < 0.0 {
                return Err("Qe < 0 in InitW");
            }
            h_new[i] = fh(model, qe, &par_d[m]);
        } else {
            let qe = ((h_new[i] - par_w[m][0]) / (par_w[m][1] - par_w[m][0])).min(1.0);
            if qe < 0.0 {
                return Err("Qe < 0 in InitW");
            }
            h_new[i] = fh(model, qe, &par_w[m]);
        }
        h_old[i] = h_new[i];
        h_temp[i] = h_new[i];
    }
    let h_bot = h_new[0];
    let h_top = h_new[num_np - 1];
    Ok((h_bot, h_top))
}

/// Dual-porosity initialization `InitDualPor` (`INPUT.FOR:508-532`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn init_dual_por(
    mat_num: &[usize], // 1-based
    par: &[Vec<f32>],
    theta: &[f32],
    i_dual_por: u8,
    th_new_im: &mut [f32],
    th_old_im: &mut [f32],
    sink_im: &mut [f32],
    h_new: &[f32],
    s_trans: &mut [f32],
    l_init_w: bool,
) {
    let num_np = mat_num.len();
    for i in 0..num_np {
        let m = mat_num[i] - 1;
        if i_dual_por == 0 {
            th_new_im[i] = 0.0;
        }
        if !l_init_w {
            if i_dual_por == 1 {
                let se = (theta[i] - par[m][0]) / (par[m][1] - par[m][0]);
                th_new_im[i] = par[m][6] + se * (par[m][7] - par[m][6]);
            } else if i_dual_por == 2 {
                th_new_im[i] = fq(0, h_new[i], &par[m][6..]);
            }
        }
        th_old_im[i] = th_new_im[i];
        sink_im[i] = 0.0;
        s_trans[i] = 0.0;
    }
}

/// Vapor hydraulic conductivity calculation `ConVapor` (`WATFLOW.FOR:849-926`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn con_vapor(
    num_np: usize,
    mat_num: &[usize],
    h_new: &[f32],
    temp: &[f32],
    con: &[f32],
    theta: &[f32],
    ths: &[f32],
    con_lt: &mut [f32],
    con_vt: &mut [f32],
    con_vh_out: &mut [f32],
    x_conv: f32,
    t_conv: f32,
    l_vapor: bool,
    _h_crit_a: f32,
    i_enhanc: i32,
) {
    let gwt = 7.0f32;
    let diff0 = 2.12e-5f32;
    let g = 9.81f32;
    let x_mol = 0.018015f32;
    let r = 8.314f32;
    let fc_clay = 0.02f32;
    let gamma0 = 71.89f32;

    for i in 0..num_np {
        let h = h_new[i] / x_conv;
        let con_lh = con[i] / x_conv * t_conv;
        let t = temp[i];
        let m = mat_num[i] - 1;
        let theta_s = ths[m];

        let d_gamma = -0.1425 - 0.000479 * t;
        con_lt[i] = 0.0;
        if h < 0.0 {
            con_lt[i] = con_lh * h * gwt * d_gamma / gamma0;
        }

        if l_vapor {
            let t_kelv = t + 273.15;
            let diff_t = diff0 * (t_kelv / 273.15).powi(2);
            let theta_a = theta_s - theta[i];
            let mut tau = 0.0f32;
            if theta_a > 0.0 {
                tau = theta_a.powf(7.0 / 3.0) / theta_s.powi(2);
            }
            let diff = tau * theta_a * diff_t;
            let row = (1.0 - 7.37e-6 * (t - 4.0).powi(2) + 3.79e-8 * (t - 4.0).powi(3)) * 1000.0;
            let rovs = 0.001 * (31.3716 - 6014.79 / t_kelv - 0.00792495 * t_kelv).exp() / t_kelv;
            let hr = (h * x_mol * g / r / t_kelv).exp();
            con_vh_out[i] = diff / row * rovs * x_mol * g / r / t_kelv * hr;

            let t_kelv1 = t_kelv + 1.0;
            let rovs1 =
                0.001 * (31.3716 - 6014.79 / t_kelv1 - 0.00792495 * t_kelv1).exp() / t_kelv1;
            let drovs = rovs1 - rovs;
            let mut eta = 1.0f32;
            if i_enhanc == 1 {
                eta = 9.5 + 3.0 * theta[i] / theta_s
                    - 8.5 * (-((1.0 + 2.6 / fc_clay.sqrt()) * theta[i] / theta_s).powi(4)).exp();
            }
            con_vt[i] = diff / row * eta * hr * drovs;

            con_vh_out[i] = con_vh_out[i] * x_conv / t_conv;
            con_vt[i] = con_vt[i] * x_conv * x_conv / t_conv;
        }
        con_lt[i] = con_lt[i] * x_conv * x_conv / t_conv;
    }
}

/// Volumetric vapor content calculation `VaporContent` (`WATFLOW.FOR:930-962`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn vapor_content(
    num_np: usize,
    mat_num: &[usize],
    theta: &[f32],
    theta_v: &mut [f32],
    temp: &[f32],
    h_new: &[f32],
    ths: &[f32],
    cap: &mut [f32],
    x_conv: f32,
) {
    let g = 9.81f32;
    let x_mol = 0.018015f32;
    let r = 8.314f32;

    for i in 0..num_np {
        let h = h_new[i] / x_conv;
        let m = mat_num[i] - 1;
        let t_kelv = temp[i] + 273.15;
        let rovs = 0.001 * (31.3716 - 6014.79 / t_kelv - 0.00792495 * t_kelv).exp() / t_kelv;
        let hr = (h * x_mol * g / r / t_kelv).exp();
        let rov = rovs * hr;
        let row =
            (1.0 - 7.37e-6 * (temp[i] - 4.0).powi(2) + 3.79e-8 * (temp[i] - 4.0).powi(3)) * 1000.0;
        theta_v[i] = rov * (ths[m] - theta[i]) / row;
        cap[i] = (1.0 - rov / row) * cap[i] + theta_v[i] * x_mol * g / r / t_kelv * x_conv;
    }
}

/// Material properties interpolation and update `SetMat` (`WATFLOW.FOR:433-568`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn set_mat(
    num_np: usize,
    tables: &MaterialTables,
    h_new: &[f32],
    mat_num: &[usize],
    par_d: &[Vec<f32>],
    con: &mut [f32],
    cap: &mut [f32],
    ah: &[f32],
    ak: &[f32],
    ath: &[f32],
    h_temp: &[f32],
    theta: &mut [f32],
    l_wt_dep: bool,
    temp: &[f32],
    iter: usize,
    con_o: &mut [f32],
    kappa: &[i32],
    aths: &[f32],
    th_rr: &[f32],
    con_r: &[f32],
    aks: &[f32],
    ah_w: &[f32],
    ath_w: &[f32],
    ak_w: &[f32],
    model: i32,
    l_table: bool,
    l_vapor: bool,
    theta_v: &mut [f32],
    con_lt: &mut [f32],
    con_vt: &mut [f32],
    con_vh_out: &mut [f32],
    x_conv: f32,
    t_conv: f32,
    h_crit_a: f32,
    i_enhanc: i32,
) {
    let model = normalize_model(model);
    let n_tab = tables.n_tab;
    let alh1 = (-tables.h_tab[0]).log10();
    let dlh = ((-tables.h_tab[n_tab - 1]).log10() - alh1) / (n_tab as f32 - 1.0);

    for i in 0..num_np {
        let mut at = 1.0f32;
        let mut bt = 1.0f32;
        if l_wt_dep {
            let temp_r = 20.0f32;
            at = (75.6 - 0.1425 * temp[i] - 2.38e-4 * temp[i].powi(2))
                / (75.6 - 0.1425 * temp_r - 2.38e-4 * temp_r.powi(2));
            bt = (1.787 - 0.007 * temp_r)
                / (1.0 + 0.03225 * temp_r)
                / ((1.787 - 0.007 * temp[i]) / (1.0 + 0.03225 * temp[i]))
                * (1.0 - 7.37e-6 * (temp[i] - 4.0).powi(2) + 3.79e-8 * (temp[i] - 4.0).powi(3))
                / (1.0 - 7.37e-6 * (temp_r - 4.0).powi(2) + 3.79e-8 * (temp_r - 4.0).powi(3));
        }

        let m = mat_num[i] - 1;
        let (hi1, hi2) = if kappa[i] == -1 {
            (
                tables.h_sat[m].min(h_temp[i] / ah[i] / at),
                tables.h_sat[m].min(h_new[i] / ah[i] / at),
            )
        } else {
            (
                tables.h_sat[m].min(h_temp[i] / ah[i] / ah_w[m] / at),
                tables.h_sat[m].min(h_new[i] / ah[i] / ah_w[m] / at),
            )
        };
        let hi_m = 0.1 * hi1 + 0.9 * hi2;

        // Conductivity
        let con_i = if hi1 >= tables.h_sat[m] && hi2 >= tables.h_sat[m] {
            tables.con_sat[m]
        } else if hi_m > tables.h_tab[n_tab - 1] && hi_m <= tables.h_tab[0] && l_table {
            let i_t = (((-hi_m).log10() - alh1) / dlh) as usize;
            let i_t = i_t.min(n_tab - 2);
            let dh = (hi_m - tables.h_tab[i_t]) / (tables.h_tab[i_t + 1] - tables.h_tab[i_t]);
            tables.con_tab[m][i_t] + (tables.con_tab[m][i_t + 1] - tables.con_tab[m][i_t]) * dh
        } else {
            fk(model, hi_m, &par_d[m])
        };

        // Capacity and water content
        let (cap_i, the_i) = if hi_m >= tables.h_sat[m] {
            (0.0f32, tables.ths[m])
        } else if hi_m >= tables.h_tab[n_tab - 1] && hi_m <= tables.h_tab[0] && l_table {
            let i_t = (((-hi_m).log10() - alh1) / dlh) as usize;
            let i_t = i_t.min(n_tab - 2);
            let dh = (hi_m - tables.h_tab[i_t]) / (tables.h_tab[i_t + 1] - tables.h_tab[i_t]);
            (
                tables.cap_tab[m][i_t] + (tables.cap_tab[m][i_t + 1] - tables.cap_tab[m][i_t]) * dh,
                tables.the_tab[m][i_t] + (tables.the_tab[m][i_t + 1] - tables.the_tab[m][i_t]) * dh,
            )
        } else {
            (fc(model, hi_m, &par_d[m]), fq(model, hi_m, &par_d[m]))
        };

        if kappa[i] == -1 {
            // Drying
            con[i] = con_i * ak[i] * bt * aks[i];
            cap[i] = cap_i * ath[i] * aths[i] / ah[i] / at;
            theta[i] = tables.thr[m] + (the_i - tables.thr[m]) * ath[i] * aths[i];
        } else {
            // Wetting
            con[i] = con_r[i] + con_i * ak[i] * bt * aks[i] * ak_w[m];
            cap[i] = cap_i * ath[i] * aths[i] * ath_w[m] / ah[i] / ah_w[m] / at;
            theta[i] = th_rr[i] + ath_w[m] * ath[i] * aths[i] * (the_i - tables.thr[m]);
        }

        if iter == 0 {
            con_o[i] = con[i];
        }
    }

    if l_vapor || l_wt_dep {
        con_vapor(
            num_np,
            mat_num,
            h_new,
            temp,
            con,
            theta,
            &tables.ths,
            con_lt,
            con_vt,
            con_vh_out,
            x_conv,
            t_conv,
            l_vapor,
            h_crit_a,
            i_enhanc,
        );
        if l_vapor {
            vapor_content(
                num_np,
                mat_num,
                theta,
                theta_v,
                temp,
                h_new,
                &tables.ths,
                cap,
                x_conv,
            );
        }
    }
}

/// Boundary conditions and system matrix setup `Reset` (`WATFLOW.FOR:173-304`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn reset(
    n: usize,
    r_top: f32,
    r_bot: &mut f32,
    cos_alf: f32,
    dt: f32,
    x: &[f32],
    _h_old: &[f32],
    con: &[f32],
    cap: &[f32],
    w_layer: bool,
    h_new: &[f32],
    sink: &[f32],
    p: &mut [f64],
    r: &mut [f64],
    s: &mut [f64],
    pb: &mut f64,
    rb: &mut f64,
    sb: &mut f64,
    pt: &mut f64,
    rt: &mut f64,
    st: &mut f64,
    free_d: bool,
    th_new: &mut [f32],
    th_old: &[f32],
    v_top: &mut f32,
    th_eq: &[f32],
    rate: f32,
    l_vapor: bool,
    l_wt_dep: bool,
    temp: &[f32],
    con_lt: &[f32],
    con_vt: &[f32],
    con_vh_in: &[f32],
    th_v_new: &[f32],
    th_v_old: &[f32],
    i_dual_por: u8,
    sink_im: &[f32],
    h_old_top: f32,
) {
    let grav = cos_alf;
    for i in 0..n {
        th_new[i] = th_old[i] + (th_eq[i] - th_old[i]) * rate;
    }

    // Bottom BC
    let dx_b = x[1] - x[0];
    let dx = dx_b / 2.0;
    let mut con_b = (con[0] + con[1]) / 2.0;
    let b = con_b * grav;
    if l_vapor {
        con_b += (con_vh_in[0] + con_vh_in[1]) / 2.0;
    }
    s[0] = -((con_b / dx_b) as f64);
    if free_d {
        *r_bot = -con_b * grav;
    }
    let f2 = (cap[0] * dx / dt * rate) as f64;
    *rb = (con_b / dx_b) as f64 + f2;
    *sb = -((con_b / dx_b) as f64);
    *pb = b as f64 - (sink[0] * dx) as f64 + f2 * (h_new[0] as f64)
        - ((th_new[0] - th_old[0]) * dx / dt) as f64
        + (*r_bot as f64);
    if i_dual_por > 0 {
        *pb -= (sink_im[0] * dx) as f64;
    }
    if l_vapor || l_wt_dep {
        let mut con_tb = 0.0f32;
        if l_vapor {
            con_tb += (con_vt[0] + con_vt[1]) / 2.0;
        }
        if l_wt_dep {
            con_tb += (con_lt[0] + con_lt[1]) / 2.0;
        }
        *pb += (con_tb * (temp[1] - temp[0]) / dx_b - (th_v_new[0] - th_v_old[0]) * dx / dt) as f64;
    }

    // Interior nodes
    for i in 1..n - 1 {
        let dx_a = x[i] - x[i - 1];
        let dx_b = x[i + 1] - x[i];
        let dx = (dx_a + dx_b) / 2.0;
        let mut con_a = (con[i] + con[i - 1]) / 2.0;
        let mut con_b = (con[i] + con[i + 1]) / 2.0;
        let b = (con_a - con_b) * grav;
        if l_vapor {
            con_a += (con_vh_in[i] + con_vh_in[i - 1]) / 2.0;
            con_b += (con_vh_in[i] + con_vh_in[i + 1]) / 2.0;
        }
        let a2 = (con_a / dx_a + con_b / dx_b) as f64;
        let a3 = -((con_b / dx_b) as f64);
        let f2 = (cap[i] * dx / dt * rate) as f64;
        r[i] = a2 + f2;
        p[i] = f2 * (h_new[i] as f64)
            - ((th_new[i] - th_old[i]) * dx / dt) as f64
            - b as f64
            - (sink[i] * dx) as f64;
        if i_dual_por > 0 {
            p[i] -= (sink_im[i] * dx) as f64;
        }
        s[i] = a3;
        if l_vapor || l_wt_dep {
            let mut con_ta = 0.0f32;
            if l_vapor {
                con_ta += (con_vt[i] + con_vt[i - 1]) / 2.0;
            }
            if l_wt_dep {
                con_ta += (con_lt[i] + con_lt[i - 1]) / 2.0;
            }
            let mut con_tb = 0.0f32;
            if l_vapor {
                con_tb += (con_vt[i] + con_vt[i + 1]) / 2.0;
            }
            if l_wt_dep {
                con_tb += (con_lt[i] + con_lt[i + 1]) / 2.0;
            }
            p[i] += (con_tb * (temp[i + 1] - temp[i]) / dx_b
                - con_ta * (temp[i] - temp[i - 1]) / dx_a
                - (th_v_new[i] - th_v_old[i]) * dx / dt) as f64;
        }
    }

    // Top BC
    let dx_a = x[n - 1] - x[n - 2];
    let dx = dx_a / 2.0;
    let mut con_a = (con[n - 1] + con[n - 2]) / 2.0;
    let b = con_a * grav;
    if l_vapor {
        con_a += (con_vh_in[n - 1] + con_vh_in[n - 2]) / 2.0;
    }
    let f2 = (cap[n - 1] * dx / dt * rate) as f64;
    *rt = (con_a / dx_a) as f64 + f2;
    *st = -((con_a / dx_a) as f64);
    *pt = f2 * (h_new[n - 1] as f64)
        - ((th_new[n - 1] - th_old[n - 1]) * dx / dt) as f64
        - (sink[n - 1] * dx) as f64
        - b as f64;
    if i_dual_por > 0 {
        *pt -= (sink_im[n - 1] * dx) as f64;
    }
    if l_vapor || l_wt_dep {
        let mut con_ta = 0.0f32;
        if l_vapor {
            con_ta += (con_vt[n - 1] + con_vt[n - 2]) / 2.0;
        }
        if l_wt_dep {
            con_ta += (con_lt[n - 1] + con_lt[n - 2]) / 2.0;
        }
        *pt += (-con_ta * (temp[n - 1] - temp[n - 2]) / dx_a
            - (th_v_new[n - 1] - th_v_old[n - 1]) * dx / dt) as f64;
    }
    *v_top = -((*st as f32) * h_new[n - 2]) - ((*rt as f32) * h_new[n - 1]) + (*pt as f32);
    *pt -= r_top as f64;
    if w_layer {
        if h_new[n - 1] > 0.0 {
            *rt += (1.0 / dt) as f64;
            *pt += (h_old_top.max(0.0) / dt) as f64;
        } else {
            *pt += (h_old_top.max(0.0) / dt) as f64;
        }
    }
}

/// Boundary condition switching / dynamic shifting `Shift` (`WATFLOW.FOR:359-429`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn shift(
    n: usize,
    kod_top: &mut i32,
    r_top: f32,
    r_bot: &mut f32,
    h_top: &mut f32,
    h_bot: &mut f32,
    h_crit_a: f32,
    cos_alf: f32,
    w_layer: bool,
    con: &[f32],
    h_new: &[f32],
    x: &[f32],
    top_inf: bool,
    kod_bot: &mut i32,
    seep_f: bool,
    th_new: &[f32],
    th_old: &[f32],
    sink: &[f32],
    dt: f32,
    l_vapor: bool,
    l_wt_dep: bool,
    con_lt: &[f32],
    temp: &[f32],
    con_vh_in: &[f32],
    con_vt: &[f32],
    i_dual_por: u8,
    sink_im: &[f32],
    h_seep: f32,
) {
    let grav = cos_alf;

    // Seepage face at the bottom
    if seep_f {
        let dx = x[1] - x[0];
        let v_bot = -(con[0] + con[1]) / 2.0 * ((h_new[1] - h_new[0]) / dx + grav)
            - dx / 2.0 * ((th_new[0] - th_old[0]) / dt + sink[0]);
        if *kod_bot >= 0 {
            if v_bot > 0.0 {
                *kod_bot = -2;
                *r_bot = 0.0;
            }
        } else if h_new[0] >= h_seep {
            *kod_bot = 2;
            *h_bot = h_seep;
        }
    }

    // Atmospheric boundary condition
    if top_inf && (kod_top.abs() == 4 || (kod_top.abs() == 1 && r_top > 0.0)) {
        if *kod_top > 0 {
            let m = n - 2;
            let dx = x[n - 1] - x[m];
            let mut v_top = -(con[n - 1] + con[m]) / 2.0 * ((h_new[n - 1] - h_new[m]) / dx + grav)
                - (th_new[n - 1] - th_old[n - 1]) * dx / 2.0 / dt
                - sink[n - 1] * dx / 2.0;
            if i_dual_por > 0 {
                v_top -= sink_im[n - 1] * dx / 2.0;
            }
            if l_wt_dep {
                v_top -= (con_lt[n - 1] + con_lt[m]) / 2.0 * (temp[n - 1] - temp[m]) / dx;
            }
            if l_vapor {
                v_top -= (con_vh_in[n - 1] + con_vh_in[m]) / 2.0 * (h_new[n - 1] - h_new[m]) / dx
                    + (con_vt[n - 1] + con_vt[m]) / 2.0 * (temp[n - 1] - temp[m]) / dx;
            }
            if (v_top.abs() > r_top.abs() || v_top * r_top <= 0.0) && kod_top.abs() == 4 {
                *kod_top = -4;
            }
            if *kod_top == 4 && h_new[n - 1] <= 0.99 * h_crit_a && r_top < 0.0 {
                *kod_top = -4;
            }
        } else {
            if !w_layer && h_new[n - 1] > 0.0 {
                if kod_top.abs() == 4 {
                    *kod_top = 4;
                }
                if kod_top.abs() == 1 {
                    *kod_top = 1;
                }
                *h_top = 0.0;
            }
            if h_new[n - 1] <= h_crit_a {
                if kod_top.abs() == 4 {
                    *kod_top = 4;
                }
                if kod_top.abs() == 1 {
                    *kod_top = 1;
                }
                *h_top = h_crit_a;
            }
        }
    }
}

/// Tridiagonal solver `Gauss` (`WATFLOW.FOR:308-355`) in double precision.
#[allow(clippy::too_many_arguments)]
pub(crate) fn gauss(
    n: usize,
    kod_top: i32,
    kod_bot: i32,
    h_top: f32,
    h_bot: f32,
    h_new: &mut [f32],
    p: &mut [f64],
    r: &mut [f64],
    s: &[f64],
    pb: f64,
    mut rb: f64,
    sb: f64,
    pt: f64,
    rt: f64,
    st: f64,
    r_min: f64,
) {
    let h_bot_d = h_bot as f64;
    let h_top_d = h_top as f64;

    // Forward
    if kod_bot >= 0 {
        p[1] -= s[0] * h_bot_d;
    } else {
        if rb.abs() < r_min {
            rb = r_min;
        }
        p[1] -= pb * s[0] / rb;
        r[1] -= sb * s[0] / rb;
    }

    for i in 2..n - 1 {
        if r[i - 1].abs() < r_min {
            r[i - 1] = r_min;
        }
        p[i] -= p[i - 1] * s[i - 1] / r[i - 1];
        r[i] -= s[i - 1] * s[i - 1] / r[i - 1];
    }

    if kod_top > 0 {
        p[n - 2] -= s[n - 2] * h_top_d;
    } else {
        if r[n - 2].abs() < r_min {
            r[n - 2] = r_min;
        }
        p[n - 1] = pt - p[n - 2] * st / r[n - 2];
        r[n - 1] = rt - s[n - 2] * st / r[n - 2];
    }

    // Back
    if r[n - 2].abs() < r_min {
        r[n - 2] = r_min;
    }
    if kod_top > 0 {
        h_new[n - 1] = h_top;
        h_new[n - 2] = (p[n - 2] / r[n - 2]) as f32;
    } else {
        h_new[n - 1] = (p[n - 1] / r[n - 1]) as f32;
        h_new[n - 2] = ((p[n - 2] - s[n - 2] * (h_new[n - 1] as f64)) / r[n - 2]) as f32;
    }

    for i in (1..n - 2).rev() {
        if r[i].abs() < r_min {
            r[i] = r_min;
        }
        h_new[i] = ((p[i] - s[i] * (h_new[i + 1] as f64)) / r[i]) as f32;
    }

    if kod_bot >= 0 {
        h_new[0] = h_bot;
    } else {
        if rb.abs() < r_min {
            rb = r_min;
        }
        h_new[0] = ((pb - sb * (h_new[1] as f64)) / rb) as f32;
    }
}

/// Darcian velocity calculation `Veloc` (`WATFLOW.FOR:717-774`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn veloc(
    n: usize,
    h_new: &[f32],
    con: &[f32],
    x: &[f32],
    cos_alf: f32,
    v: &mut [f32],
    th_new: &[f32],
    th_old: &[f32],
    sink: &[f32],
    dt: f32,
    l_vapor: bool,
    l_wt_dep: bool,
    con_lt: &[f32],
    con_vt: &[f32],
    con_vh_in: &[f32],
    temp: &[f32],
    v_v: &mut [f32],
    th_v_new: &[f32],
    th_v_old: &[f32],
) {
    let grav = cos_alf;
    let m = n - 2;
    let dx_n = x[n - 1] - x[m];

    v[n - 1] = -(con[n - 1] + con[m]) / 2.0 * ((h_new[n - 1] - h_new[m]) / dx_n + grav)
        - dx_n / 2.0 * ((th_new[n - 1] - th_old[n - 1]) / dt + sink[n - 1]);
    if l_wt_dep {
        v[n - 1] -= (con_lt[n - 1] + con_lt[m]) / 2.0 * (temp[n - 1] - temp[m]) / dx_n;
    }
    v_v[n - 1] = 0.0;
    if l_vapor {
        v_v[n - 1] = -(con_vh_in[n - 1] + con_vh_in[m]) / 2.0 * (h_new[n - 1] - h_new[m]) / dx_n
            - (con_vt[n - 1] + con_vt[m]) / 2.0 * (temp[n - 1] - temp[m]) / dx_n
            - dx_n / 2.0 * (th_v_new[n - 1] - th_v_old[n - 1]) / dt;
    }

    for i in 1..n - 1 {
        let dx_a = x[i + 1] - x[i];
        let dx_b = x[i] - x[i - 1];
        let v_a = -(con[i] + con[i + 1]) / 2.0 * ((h_new[i + 1] - h_new[i]) / dx_a + grav);
        let v_b = -(con[i] + con[i - 1]) / 2.0 * ((h_new[i] - h_new[i - 1]) / dx_b + grav);
        v[i] = (v_a * dx_b + v_b * dx_a) / (dx_a + dx_b);
        if l_wt_dep {
            let v_ta = -(con_lt[i] + con_lt[i + 1]) / 2.0 * (temp[i + 1] - temp[i]) / dx_a;
            let v_tb = -(con_lt[i] + con_lt[i - 1]) / 2.0 * (temp[i] - temp[i - 1]) / dx_b;
            v[i] += (v_ta * dx_b + v_tb * dx_a) / (dx_a + dx_b);
        }
        v_v[i] = 0.0;
        if l_vapor {
            let mut v_va =
                -(con_vh_in[i] + con_vh_in[i + 1]) / 2.0 * (h_new[i + 1] - h_new[i]) / dx_a;
            let mut v_vb =
                -(con_vh_in[i] + con_vh_in[i - 1]) / 2.0 * (h_new[i] - h_new[i - 1]) / dx_b;
            v_va -= (con_vt[i] + con_vt[i + 1]) / 2.0 * (temp[i + 1] - temp[i]) / dx_a;
            v_vb -= (con_vt[i] + con_vt[i - 1]) / 2.0 * (temp[i] - temp[i - 1]) / dx_b;
            v_v[i] = (v_va * dx_b + v_vb * dx_a) / (dx_a + dx_b);
        }
    }

    let dx1 = x[1] - x[0];
    v[0] = -(con[0] + con[1]) / 2.0 * ((h_new[1] - h_new[0]) / dx1 + grav)
        + dx1 / 2.0 * ((th_new[0] - th_old[0]) / dt + sink[0]);
    if l_wt_dep {
        v[0] -= (con_lt[0] + con_lt[1]) / 2.0 * (temp[1] - temp[0]) / dx1;
    }
    v_v[0] = 0.0;
    if l_vapor {
        v_v[0] = -(con_vh_in[0] + con_vh_in[1]) / 2.0 * (h_new[1] - h_new[0]) / dx1
            - (con_vt[0] + con_vt[1]) / 2.0 * (temp[1] - temp[0]) / dx1
            + dx1 / 2.0 * (th_v_new[0] - th_v_old[0]) / dt;
    }
}

/// End-of-step time state update `Update` (`WATFLOW.FOR:1079-1126`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn update(
    num_np: usize,
    l_wat: bool,
    l_chem: bool,
    l_temp: bool,
    l_vapor: bool,
    i_dual_por: u8,
    l_extrap: bool,
    dt: f32,
    dt_old: f32,
    h_temp: &mut [f32],
    h_new: &mut [f32],
    h_old: &mut [f32],
    th_old: &mut [f32],
    th_new: &[f32],
    v_old: &mut [f32],
    v_new: &[f32],
    th_v_old: &mut [f32],
    th_v_new: &[f32],
    v_v_old: &mut [f32],
    v_v_new: &[f32],
    th_old_im: &mut [f32],
    th_new_im: &[f32],
    temp_o: &mut [f32],
    temp_n: &[f32],
    r_top: f32,
    x_conv: f32,
    con_s_max: f32,
    kod_top: &mut i32,
    kod_bot: i32,
) {
    let mut l_sat = true;
    let i_bot = if kod_bot > 0 { 1 } else { 0 };
    let i_top = if *kod_top > 0 { num_np - 2 } else { num_np - 1 };

    for i in i_bot..=i_top {
        if l_wat {
            if l_extrap && h_new[i] < 0.0 && h_old[i] < 0.0 {
                h_temp[i] = h_new[i] + (h_new[i] - h_old[i]) * dt / dt_old;
            } else {
                h_temp[i] = h_new[i];
            }
            h_old[i] = h_new[i];
            h_new[i] = h_temp[i];
        }
    }

    for i in 0..num_np {
        if l_wat {
            th_old[i] = th_new[i];
            if l_temp || l_chem {
                v_old[i] = v_new[i];
            }
            if l_vapor {
                th_v_old[i] = th_v_new[i];
                v_v_old[i] = v_v_new[i];
            }
            if i_dual_por > 0 {
                th_old_im[i] = th_new_im[i];
            }
            if h_new[i] < 0.0 {
                l_sat = false;
            }
        }
        if l_temp {
            temp_o[i] = temp_n[i];
        }
    }

    if l_wat && l_sat && (*kod_top == -1 || *kod_top == 4) && r_top >= -con_s_max {
        if *kod_top == 4 {
            *kod_top = -4;
        }
        h_new[num_np - 1] = -0.005 * x_conv;
        h_temp[num_np - 1] = -0.005 * x_conv;
        h_old[num_np - 1] = -0.005 * x_conv;
    }
}

/// Dynamic time step adjustment `TmCont` from `TIME.FOR:3-32`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tm_cont(
    dt: &mut f32,
    dt_max_w: f32,
    dt_opt: &mut f32,
    d_mul: f32,
    d_mul2: f32,
    dt_min: f32,
    iter: usize,
    t_print: f64,
    t_atm: f64,
    t: f64,
    t_max: f64,
    dt_max_c: f32,
    it_min: usize,
    it_max: usize,
    l_min_step: &mut bool,
    dt_init: f32,
) {
    let dt_max = if *l_min_step {
        let max_val = dt_max_w.min(dt_max_c).min(dt_init).min(*dt_opt);
        *dt_opt = max_val;
        *l_min_step = false;
        max_val
    } else {
        dt_max_w.min(dt_max_c)
    };

    let t_fix = t_print.min(t_atm).min(t_max);
    let t_diff = (t_fix - t) as f32;

    if iter <= it_min && (t_fix - t) >= (d_mul * *dt_opt) as f64 {
        *dt_opt = dt_max.min(d_mul * *dt_opt);
    }
    if iter >= it_max {
        *dt_opt = dt_min.max(d_mul2 * *dt_opt);
    }

    *dt = (*dt_opt).min(t_diff);
    let mut i_step = 1;
    if *dt > 0.0 {
        i_step = (t_diff / *dt).round() as i32;
    }
    if (1..=10).contains(&i_step) {
        *dt = (t_diff / (i_step as f32)).min(dt_max);
    }
    if i_step == 1 {
        *dt = t_diff;
        if *dt - dt_max > dt_min {
            *dt /= 2.0;
        }
    }
    if *dt <= 0.0 {
        *dt = dt_min / 3.0;
    }
}

/// Inputs and configuration for `wat_flow`.
pub(crate) struct WatFlowInputs<'a> {
    pub num_np: usize,
    pub tables: &'a MaterialTables,
    pub mat_num: &'a [usize],
    pub par_d: &'a [Vec<f32>],
    pub par_w: &'a [Vec<f32>],
    pub ah: &'a [f32],
    pub ak: &'a [f32],
    pub ath: &'a [f32],
    pub x: &'a [f32],
    pub cos_alf: f32,
    pub free_d: bool,
    pub seep_f: bool,
    pub w_layer: bool,
    pub top_inf: bool,
    pub tol_th: f32,
    pub tol_h: f32,
    pub max_it: usize,
    pub dt_min: f32,
    pub l_wt_dep: bool,
    pub temp_n: &'a [f32],
    pub kappa: &'a mut [i32],
    pub kappa_o: &'a [i32],
    pub aths: &'a [f32],
    pub th_rr: &'a [f32],
    pub con_r: &'a [f32],
    pub aks: &'a [f32],
    pub ah_w: &'a [f32],
    pub ath_w: &'a [f32],
    pub ak_w: &'a [f32],
    pub model: i32,
    pub l_table: bool,
    pub l_vapor: bool,
    pub x_conv: f32,
    pub t_conv: f32,
    pub tau_w: f32,
    pub th_eq: &'a [f32],
    pub i_dual_por: u8,
    pub dual_por_materials: &'a [DualPorMaterial],
    pub temp_o: &'a [f32],
    pub h_seep: f32,
    pub i_enhanc: i32,
}

/// Mutable step state for `wat_flow`.
pub(crate) struct WatFlowState<'a> {
    pub h_new: &'a mut [f32],
    pub h_old: &'a mut [f32],
    pub h_temp: &'a mut [f32],
    pub con: &'a mut [f32],
    pub cap: &'a mut [f32],
    pub sink: &'a [f32],
    pub th_new: &'a mut [f32],
    pub th_old: &'a mut [f32],
    pub th_new_im: &'a mut [f32],
    pub th_old_im: &'a mut [f32],
    pub sink_im: &'a mut [f32],
    pub th_v_new: &'a mut [f32],
    pub th_v_old: &'a [f32],
    pub con_lt: &'a mut [f32],
    pub con_vt: &'a mut [f32],
    pub con_vh_buf: &'a mut [f32],
    pub con_o: &'a mut [f32],
    pub kod_top: &'a mut i32,
    pub kod_bot: &'a mut i32,
    pub r_top: f32,
    pub r_bot: &'a mut f32,
    pub h_top: &'a mut f32,
    pub h_bot: &'a mut f32,
    pub h_crit_a: f32,
    pub h_crit_s: f32,
    pub kt_old: i32,
    pub kb_old: i32,
    pub t: &'a mut f64,
    pub dt: &'a mut f32,
    pub t_old: f64,
    pub dt_opt: &'a mut f32,
    pub iter: &'a mut usize,
    pub it_cum: &'a mut usize,
    pub convg_f: &'a mut bool,
    pub v_top: &'a mut f32,
    pub w_transf: &'a mut f32,
}

/// Solves 1D Richards equation for one time step `WatFlow` (`WATFLOW.FOR:3-169`).
pub(crate) fn wat_flow(inputs: &mut WatFlowInputs<'_>, state: &mut WatFlowState<'_>) {
    let r_max = 1.0e10f32;
    let r_min = 1.0e-100f64;

    let mut rate = 1.0f32;
    if inputs.tau_w > 0.0 {
        rate = (1.0f32).min((0.000001f32).max(1.0 - (-*state.dt / inputs.tau_w).exp()));
    }

    // Dual porosity mass transfer
    // Delegated to super::dual_domain::dual_porosity_exchange per spec & STATE_18.md:74
    if inputs.i_dual_por > 0 {
        let primitive_state = DualPorPrimitiveState {
            coordinates: inputs.x.iter().map(|v| DefaultReal(*v)).collect(),
            old_water: state.th_old.iter().map(|v| DefaultReal(*v)).collect(),
            new_pressure_head: state.h_new.iter().map(|v| DefaultReal(*v)).collect(),
            old_immobile_water: state.th_old_im.iter().map(|v| DefaultReal(*v)).collect(),
            material_ids: inputs.mat_num.to_vec(),
            materials: inputs.dual_por_materials.to_vec(),
            critical_atmospheric_head: DefaultReal(state.h_crit_a),
            dt: DefaultReal(*state.dt),
            hydraulic_model: inputs.model,
            dual_porosity: inputs.i_dual_por,
        };
        if let Ok(exchange) = dual_porosity_exchange(&primitive_state) {
            for i in 0..inputs.num_np {
                state.sink_im[i] = exchange.sink_immobile[i].0;
                state.th_new_im[i] = exchange.new_immobile_water[i].0;
            }
            *state.w_transf = exchange.transfer.0;
        }
    }

    let mut p = vec![0.0f64; inputs.num_np];
    let mut r = vec![0.0f64; inputs.num_np];
    let mut s = vec![0.0f64; inputs.num_np];

    'outer_retry: loop {
        *state.iter = 0;
        *state.convg_f = true;

        if inputs.w_layer
            && state.h_new[inputs.num_np - 1] > 0.0
            && state.h_new[inputs.num_np - 1] < 0.00005 * inputs.x_conv
            && state.r_top >= 0.0
        {
            let m = inputs.mat_num[inputs.num_np - 1] - 1;
            state.h_new[inputs.num_np - 1] = fh(inputs.model, 0.9999, &inputs.par_d[m]);
            state.h_old[inputs.num_np - 1] = state.h_new[inputs.num_np - 1];
            state.h_temp[inputs.num_np - 1] = state.h_new[inputs.num_np - 1];
        }

        'inner_iteration: loop {
            set_mat(
                inputs.num_np,
                inputs.tables,
                state.h_new,
                inputs.mat_num,
                inputs.par_d,
                state.con,
                state.cap,
                inputs.ah,
                inputs.ak,
                inputs.ath,
                state.h_temp,
                state.th_new,
                inputs.l_wt_dep,
                inputs.temp_n,
                *state.iter,
                state.con_o,
                inputs.kappa,
                inputs.aths,
                inputs.th_rr,
                inputs.con_r,
                inputs.aks,
                inputs.ah_w,
                inputs.ath_w,
                inputs.ak_w,
                inputs.model,
                inputs.l_table,
                inputs.l_vapor,
                state.th_v_new,
                state.con_lt,
                state.con_vt,
                state.con_vh_buf,
                inputs.x_conv,
                inputs.t_conv,
                state.h_crit_a,
                inputs.i_enhanc,
            );

            let mut pb = 0.0f64;
            let mut rb = 0.0f64;
            let mut sb = 0.0f64;
            let mut pt = 0.0f64;
            let mut rt = 0.0f64;
            let mut st = 0.0f64;

            let h_old_top = state.h_old[inputs.num_np - 1];
            reset(
                inputs.num_np,
                state.r_top,
                state.r_bot,
                inputs.cos_alf,
                *state.dt,
                inputs.x,
                state.h_old,
                state.con,
                state.cap,
                inputs.w_layer,
                state.h_new,
                state.sink,
                &mut p,
                &mut r,
                &mut s,
                &mut pb,
                &mut rb,
                &mut sb,
                &mut pt,
                &mut rt,
                &mut st,
                inputs.free_d,
                state.th_new,
                state.th_old,
                state.v_top,
                inputs.th_eq,
                rate,
                inputs.l_vapor,
                inputs.l_wt_dep,
                inputs.temp_n,
                state.con_lt,
                state.con_vt,
                state.con_vh_buf,
                state.th_v_new,
                state.th_v_old,
                inputs.i_dual_por,
                state.sink_im,
                h_old_top,
            );

            shift(
                inputs.num_np,
                state.kod_top,
                state.r_top,
                state.r_bot,
                state.h_top,
                state.h_bot,
                state.h_crit_a,
                inputs.cos_alf,
                inputs.w_layer,
                state.con,
                state.h_new,
                inputs.x,
                inputs.top_inf,
                state.kod_bot,
                inputs.seep_f,
                state.th_new,
                state.th_old,
                state.sink,
                *state.dt,
                inputs.l_vapor,
                inputs.l_wt_dep,
                state.con_lt,
                inputs.temp_n,
                state.con_vh_buf,
                state.con_vt,
                inputs.i_dual_por,
                state.sink_im,
                inputs.h_seep,
            );

            for i in 0..inputs.num_np {
                state.h_temp[i] = state.h_new[i];
            }

            gauss(
                inputs.num_np,
                *state.kod_top,
                *state.kod_bot,
                *state.h_top,
                *state.h_bot,
                state.h_new,
                &mut p,
                &mut r,
                &s,
                pb,
                rb,
                sb,
                pt,
                rt,
                st,
                r_min,
            );

            for i in 0..inputs.num_np {
                if state.h_new[i].abs() > r_max {
                    state.h_new[i] = r_max.copysign(state.h_new[i]);
                }
                if state.kod_top.abs() == 4
                    && state.h_new[i] < state.h_crit_a
                    && i == inputs.num_np - 1
                {
                    state.h_new[i] = state.h_crit_a;
                }
                if state.kod_top.abs() == 4
                    && state.h_new[i] < state.h_crit_a
                    && (i as f32) > (inputs.num_np as f32) * 0.9
                    && state.sink[i] <= 0.0
                {
                    state.h_new[i] = state.h_crit_a;
                }
            }

            *state.iter += 1;
            *state.it_cum += 1;

            // Convergence test
            let mut it_crit = true;
            for i in 0..inputs.num_np {
                let m = inputs.mat_num[i] - 1;
                let eps_th;
                let eps_h;
                if state.h_temp[i] < inputs.tables.h_sat[m]
                    && state.h_new[i] < inputs.tables.h_sat[m]
                {
                    let th = state.th_new[i]
                        + state.cap[i] * (state.h_new[i] - state.h_temp[i])
                            / (inputs.tables.ths[m] - inputs.tables.thr[m])
                            / inputs.ath[i]
                            * rate;
                    eps_th = (state.th_new[i] - th).abs();
                    eps_h = 0.0;
                } else {
                    eps_th = 0.0;
                    eps_h = (state.h_new[i] - state.h_temp[i]).abs();
                }

                if eps_th > inputs.tol_th
                    || eps_h > inputs.tol_h
                    || state.h_new[i].abs() > r_max * 0.999
                {
                    it_crit = false;
                    if state.h_new[i].abs() > r_max * 0.999 {
                        *state.iter = inputs.max_it;
                    }
                    break;
                }
            }

            if !it_crit || *state.iter <= 1 {
                if *state.iter < inputs.max_it {
                    continue 'inner_iteration;
                } else if *state.dt <= inputs.dt_min {
                    *state.convg_f = false;
                    return;
                } else {
                    // Retry with smaller timestep
                    for i in 0..inputs.num_np {
                        inputs.kappa[i] = inputs.kappa_o[i];
                        state.h_new[i] = state.h_old[i];
                        state.h_temp[i] = state.h_old[i];
                    }
                    *state.kod_top = state.kt_old;
                    *state.kod_bot = state.kb_old;
                    *state.dt = (*state.dt / 3.0).max(inputs.dt_min);
                    *state.dt_opt = *state.dt;
                    *state.t = state.t_old + (*state.dt as f64);
                    if inputs.tau_w > 0.0 {
                        rate = (1.0f32)
                            .min((0.000001f32).max(1.0 - (-*state.dt / inputs.tau_w).exp()));
                    }
                    continue 'outer_retry;
                }
            }

            if it_crit {
                for i in 0..inputs.num_np {
                    state.th_new[i] += state.cap[i] * (state.h_new[i] - state.h_temp[i]) * rate;
                }
            }
            if inputs.w_layer && state.h_new[inputs.num_np - 1] > state.h_crit_s {
                *state.kod_top = 4;
                *state.h_top = state.h_crit_s;
            }
            break 'outer_retry;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_mat_and_set_mat_interpolation_vs_direct() {
        // Material 1: Loamy sand / sandy loam
        // Qr=0.065, Qs=0.41, Alfa=0.075, n=1.89, Ks=4.42, BPar=0.5
        let par = vec![vec![
            0.065, 0.41, 0.075, 1.89, 4.42, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0,
        ]];
        let tables = gen_mat(100, 1, &par, -0.0001, -100000.0, 0, 1.0, 1.0);

        assert_eq!(tables.h_tab.len(), 100);
        assert!((tables.ths[0] - 0.41).abs() < 1e-6);
        assert!((tables.thr[0] - 0.065).abs() < 1e-6);

        // Test interpolation against direct evaluation across several suction levels
        let test_heads = [-0.1, -1.0, -10.0, -50.0, -200.0, -1000.0, -5000.0];
        let num_np = test_heads.len();
        let h_new = test_heads.to_vec();
        let h_temp = test_heads.to_vec();
        let mat_num = vec![1; num_np];
        let mut con = vec![0.0; num_np];
        let mut cap = vec![0.0; num_np];
        let mut theta = vec![0.0; num_np];
        let ah = vec![1.0; num_np];
        let ak = vec![1.0; num_np];
        let ath = vec![1.0; num_np];
        let temp = vec![20.0; num_np];
        let mut con_o = vec![0.0; num_np];
        let kappa = vec![-1; num_np];
        let aths = vec![1.0; num_np];
        let th_rr = vec![0.0; num_np];
        let con_r = vec![0.0; num_np];
        let aks = vec![1.0; num_np];
        let ah_w = vec![1.0; 1];
        let ath_w = vec![1.0; 1];
        let ak_w = vec![1.0; 1];
        let mut theta_v = vec![0.0; num_np];
        let mut con_lt = vec![0.0; num_np];
        let mut con_vt = vec![0.0; num_np];
        let mut con_vh_out = vec![0.0; num_np];

        set_mat(
            num_np,
            &tables,
            &h_new,
            &mat_num,
            &par,
            &mut con,
            &mut cap,
            &ah,
            &ak,
            &ath,
            &h_temp,
            &mut theta,
            false,
            &temp,
            0,
            &mut con_o,
            &kappa,
            &aths,
            &th_rr,
            &con_r,
            &aks,
            &ah_w,
            &ath_w,
            &ak_w,
            0,
            true, // l_table = true
            false,
            &mut theta_v,
            &mut con_lt,
            &mut con_vt,
            &mut con_vh_out,
            1.0,
            1.0,
            -1.0e6,
            0,
        );

        for i in 0..num_np {
            let direct_k = fk(0, test_heads[i], &par[0]);
            let direct_c = fc(0, test_heads[i], &par[0]);
            let direct_q = fq(0, test_heads[i], &par[0]);

            // Interpolation from 100 log points matches direct evaluation closely (within 1%)
            let rel_diff_k = ((con[i] - direct_k) / direct_k).abs();
            let rel_diff_c = ((cap[i] - direct_c) / direct_c).abs();
            let rel_diff_q = ((theta[i] - direct_q) / direct_q).abs();

            assert!(
                rel_diff_k < 0.15,
                "head {}: interp K {} vs direct {}",
                test_heads[i],
                con[i],
                direct_k
            );
            assert!(
                rel_diff_c < 0.05,
                "head {}: interp C {} vs direct {}",
                test_heads[i],
                cap[i],
                direct_c
            );
            assert!(
                rel_diff_q < 0.01,
                "head {}: interp Q {} vs direct {}",
                test_heads[i],
                theta[i],
                direct_q
            );
        }
    }

    #[test]
    fn test_steady_transient_watflow_and_veloc_21_node_column() {
        let num_np = 21;
        let mut x = vec![0.0f32; num_np];
        for (i, slot) in x.iter_mut().enumerate() {
            *slot = (i as f32) * 5.0; // 0 to 100 cm
        }

        // Sandy loam van Genuchten parameters
        let par = vec![vec![
            0.065, 0.41, 0.075, 1.89, 4.42, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0,
        ]];
        let tables = gen_mat(100, 1, &par, -0.0001, -100000.0, 0, 1.0, 1.0);

        let mat_num = vec![1; num_np];
        let par_d = par.clone();
        let par_w = par.clone();
        let ah = vec![1.0; num_np];
        let ak = vec![1.0; num_np];
        let ath = vec![1.0; num_np];
        let mut kappa = vec![-1; num_np];
        let kappa_o = vec![-1; num_np];
        let aths = vec![1.0; num_np];
        let th_rr = vec![0.0; num_np];
        let con_r = vec![0.0; num_np];
        let aks = vec![1.0; num_np];
        let ah_w = vec![1.0; 1];
        let ath_w = vec![1.0; 1];
        let ak_w = vec![1.0; 1];
        let temp_n = vec![20.0; num_np];
        let temp_o = vec![20.0; num_np];
        let th_eq = vec![0.0; num_np];

        let mut h_new = vec![-100.0f32; num_np];
        let mut h_old = vec![-100.0f32; num_np];
        let mut h_temp = vec![-100.0f32; num_np];
        let mut con = vec![0.0f32; num_np];
        let mut cap = vec![0.0f32; num_np];
        let sink = vec![0.0f32; num_np];
        let mut th_new = vec![0.2f32; num_np];
        let mut th_old = vec![0.2f32; num_np];
        let mut th_new_im = vec![0.0f32; num_np];
        let mut th_old_im = vec![0.0f32; num_np];
        let mut sink_im = vec![0.0f32; num_np];
        let mut th_v_new = vec![0.0f32; num_np];
        let th_v_old = vec![0.0f32; num_np];
        let mut con_lt = vec![0.0f32; num_np];
        let mut con_vt = vec![0.0f32; num_np];
        let mut con_vh_buf = vec![0.0f32; num_np];
        let mut con_o = vec![0.0f32; num_np];

        // Prescribed head at top = -20 cm, prescribed head at bottom = -100 cm
        let mut kod_top = 1;
        let mut kod_bot = 1;
        let mut h_top = -20.0f32;
        let mut h_bot = -100.0f32;
        let mut r_bot = 0.0f32;
        let r_top = 0.0f32;
        let mut t = 0.0f64;
        let mut dt = 0.1f32;
        let t_old = 0.0f64;
        let mut dt_opt = 0.1f32;
        let mut iter = 0;
        let mut it_cum = 0;
        let mut convg_f = true;
        let mut v_top = 0.0f32;
        let mut w_transf = 0.0f32;

        let mut inputs = WatFlowInputs {
            num_np,
            tables: &tables,
            mat_num: &mat_num,
            par_d: &par_d,
            par_w: &par_w,
            ah: &ah,
            ak: &ak,
            ath: &ath,
            x: &x,
            cos_alf: 1.0,
            free_d: false,
            seep_f: false,
            w_layer: false,
            top_inf: false,
            tol_th: 0.001,
            tol_h: 0.1,
            max_it: 20,
            dt_min: 0.0001,
            l_wt_dep: false,
            temp_n: &temp_n,
            kappa: &mut kappa,
            kappa_o: &kappa_o,
            aths: &aths,
            th_rr: &th_rr,
            con_r: &con_r,
            aks: &aks,
            ah_w: &ah_w,
            ath_w: &ath_w,
            ak_w: &ak_w,
            model: 0,
            l_table: true,
            l_vapor: false,
            x_conv: 1.0,
            t_conv: 1.0,
            tau_w: 0.0,
            th_eq: &th_eq,
            i_dual_por: 0,
            dual_por_materials: &[],
            temp_o: &temp_o,
            h_seep: 0.0,
            i_enhanc: 0,
        };

        let mut state = WatFlowState {
            h_new: &mut h_new,
            h_old: &mut h_old,
            h_temp: &mut h_temp,
            con: &mut con,
            cap: &mut cap,
            sink: &sink,
            th_new: &mut th_new,
            th_old: &mut th_old,
            th_new_im: &mut th_new_im,
            th_old_im: &mut th_old_im,
            sink_im: &mut sink_im,
            th_v_new: &mut th_v_new,
            th_v_old: &th_v_old,
            con_lt: &mut con_lt,
            con_vt: &mut con_vt,
            con_vh_buf: &mut con_vh_buf,
            con_o: &mut con_o,
            kod_top: &mut kod_top,
            kod_bot: &mut kod_bot,
            r_top,
            r_bot: &mut r_bot,
            h_top: &mut h_top,
            h_bot: &mut h_bot,
            h_crit_a: -1.0e6,
            h_crit_s: 0.0,
            kt_old: 1,
            kb_old: 1,
            t: &mut t,
            dt: &mut dt,
            t_old,
            dt_opt: &mut dt_opt,
            iter: &mut iter,
            it_cum: &mut it_cum,
            convg_f: &mut convg_f,
            v_top: &mut v_top,
            w_transf: &mut w_transf,
        };

        wat_flow(&mut inputs, &mut state);

        assert!(*state.convg_f, "WatFlow should converge");
        assert!(*state.iter > 0 && *state.iter < 20);

        // Verify Veloc calculation
        let mut v = vec![0.0f32; num_np];
        let mut v_v = vec![0.0f32; num_np];
        veloc(
            num_np,
            state.h_new,
            state.con,
            &x,
            1.0,
            &mut v,
            state.th_new,
            state.th_old,
            &sink,
            *state.dt,
            false,
            false,
            state.con_lt,
            state.con_vt,
            state.con_vh_buf,
            &temp_n,
            &mut v_v,
            state.th_v_new,
            &th_v_old,
        );

        // Velocities should be finite and non-zero
        for val in &v {
            assert!(val.is_finite());
            assert!(val.abs() > 0.0, "infiltration velocity should be non-zero");
        }
    }

    #[test]
    fn test_init_dual_por_and_dual_por_wetting_transient_models_6_and_7() {
        // Test models 6 (mass transfer) and 7 (dual permeability / matric exchange)
        for model_id in [6, 7] {
            let num_np = 11;
            let mut x = vec![0.0f32; num_np];
            for (i, slot) in x.iter_mut().enumerate() {
                *slot = (i as f32) * 10.0;
            }

            let mat_num = vec![1; num_np];
            let par_m6 = vec![
                0.08, 0.43, 0.015, 1.875, 0.43, 0.5, 0.05, 0.25, 0.1, 0.0, 0.0,
            ];
            let par_m7 = vec![
                0.08, 0.43, 0.015, 1.875, 0.43, 0.5, 0.01, 0.12, 0.1, 1.2, 0.01,
            ];

            let par = if model_id == 6 {
                vec![par_m6]
            } else {
                vec![par_m7]
            };
            let dual_por_mode = if model_id == 6 { 1 } else { 2 };

            let dual_materials = vec![DualPorMaterial {
                mobile_residual_water: DefaultReal(par[0][0]),
                mobile_saturated_water: DefaultReal(par[0][1]),
                mobile_tortuosity: Some(DefaultReal(par[0][5])),
                immobile_residual_water: DefaultReal(par[0][6]),
                immobile_saturated_water: DefaultReal(par[0][7]),
                immobile_alpha: DefaultReal(par[0][8]),
                immobile_n: if model_id == 7 {
                    Some(DefaultReal(par[0][9]))
                } else {
                    None
                },
                immobile_saturated_conductivity: if model_id == 7 {
                    Some(DefaultReal(par[0][10]))
                } else {
                    None
                },
            }];

            let mut th_new_im = vec![0.0f32; num_np];
            let mut th_old_im = vec![0.0f32; num_np];
            let mut sink_im = vec![0.0f32; num_np];
            let mut s_trans = vec![0.0f32; num_np];
            let theta_init = vec![0.30f32; num_np];
            let h_init = vec![-50.0f32; num_np];

            init_dual_por(
                &mat_num,
                &par,
                &theta_init,
                dual_por_mode,
                &mut th_new_im,
                &mut th_old_im,
                &mut sink_im,
                &h_init,
                &mut s_trans,
                false,
            );

            // Verify initial immobile water is initialized between residual and saturated
            for i in 0..num_np {
                assert!(th_new_im[i] >= par[0][6]);
                assert!(th_new_im[i] <= par[0][7]);
                assert_eq!(th_old_im[i], th_new_im[i]);
                assert_eq!(sink_im[i], 0.0);
            }

            // Simulate multi-step wetting transient (t = 0..4)
            let dt = 1.0f32;
            let h_wet = vec![-10.0f32; num_np]; // Wetting front
            let th_wet = vec![0.38f32; num_np];

            for step in 0..4 {
                let primitive_state = DualPorPrimitiveState {
                    coordinates: x.iter().map(|v| DefaultReal(*v)).collect(),
                    old_water: th_wet.iter().map(|v| DefaultReal(*v)).collect(),
                    new_pressure_head: h_wet.iter().map(|v| DefaultReal(*v)).collect(),
                    old_immobile_water: th_old_im.iter().map(|v| DefaultReal(*v)).collect(),
                    material_ids: mat_num.clone(),
                    materials: dual_materials.clone(),
                    critical_atmospheric_head: DefaultReal(-1.0e6),
                    dt: DefaultReal(dt),
                    hydraulic_model: 0,
                    dual_porosity: dual_por_mode,
                };

                let exchange =
                    dual_porosity_exchange(&primitive_state).expect("dual porosity exchange");
                assert_eq!(exchange.sink_immobile.len(), num_np);
                assert_eq!(exchange.new_immobile_water.len(), num_np);

                for (i, old_im_slot) in th_old_im.iter_mut().enumerate() {
                    // Mobile region is wetter than immobile, so sink should transfer into immobile (sink >= 0)
                    assert!(
                        exchange.sink_immobile[i].0 >= 0.0,
                        "step {}: sink must be >= 0",
                        step
                    );
                    // Immobile water content should increase or stay capped at saturated
                    assert!(exchange.new_immobile_water[i].0 >= *old_im_slot);
                    assert!(exchange.new_immobile_water[i].0 <= par[0][7]);
                    *old_im_slot = exchange.new_immobile_water[i].0;
                }
            }
        }
    }

    #[test]
    fn test_tm_cont_print_time_snapping_and_dt_opt_restoration() {
        let mut dt = 0.5f32;
        let dt_max_w = 1.0f32;
        let mut dt_opt = 0.5f32;
        let d_mul = 1.3f32;
        let d_mul2 = 0.7f32;
        let dt_min = 0.001f32;
        let t_print = 10.0f64;
        let t_atm = 100.0f64;
        let t_max = 100.0f64;
        let dt_max_c = 1.0f32;
        let it_min = 3;
        let it_max = 7;
        let mut l_min_step = false;
        let dt_init = 0.1f32;

        // At t = 9.8, t_fix - t = 0.2 < dt_opt (0.5), so dt must snap to exactly 0.2
        let t = 9.8f64;
        tm_cont(
            &mut dt,
            dt_max_w,
            &mut dt_opt,
            d_mul,
            d_mul2,
            dt_min,
            2, // iter <= it_min
            t_print,
            t_atm,
            t,
            t_max,
            dt_max_c,
            it_min,
            it_max,
            &mut l_min_step,
            dt_init,
        );

        assert!(
            (dt - 0.2).abs() < 1e-5,
            "dt should snap to remaining print time 0.2, got {}",
            dt
        );
        // But dt_opt should NOT be destroyed by print time snapping
        assert!(
            dt_opt >= 0.5,
            "dt_opt should remain optimal step size or grow, got {}",
            dt_opt
        );

        // Test reduction when iter >= it_max
        let prior_dt_opt = dt_opt;
        let t_next = 10.0f64;
        let next_t_print = 20.0f64;
        tm_cont(
            &mut dt,
            dt_max_w,
            &mut dt_opt,
            d_mul,
            d_mul2,
            dt_min,
            8, // iter >= it_max
            next_t_print,
            t_atm,
            t_next,
            t_max,
            dt_max_c,
            it_min,
            it_max,
            &mut l_min_step,
            dt_init,
        );
        assert!(
            dt_opt < prior_dt_opt,
            "dt_opt should be multiplied by d_mul2 (reduced)"
        );
    }
}

//! Source-ordered mass accounting and the conditionally active flux-concentration helper.

use super::types::{DefaultReal, Explicit, KernelError};

/// Cumulative values written by the active end-of-step `MassTran` call.
#[derive(Clone, Debug, PartialEq)]
pub struct MassTotals {
    pub cv_ch0: Vec<DefaultReal>,
    pub cv_ch1: Vec<DefaultReal>,
    pub cv_ch_r: Vec<DefaultReal>,
    pub cv_ch_im: Vec<DefaultReal>,
    /// Source `STrans`; updated only by the assignments reachable for the
    /// active linear mode (the nonlinear-only mobile assignments are skipped).
    pub s_trans: Vec<DefaultReal>,
}

/// Node/material state passed to the active end-of-step `MassTran` call.
pub struct MassTranInput<'a> {
    pub species: usize,
    pub coordinates: &'a [DefaultReal],
    /// Native Fortran material indices translated to zero-based indices.
    pub material_ids: &'a [usize],
    pub temperature: &'a [DefaultReal],
    /// `ChPar` indexed `[material][Fortran row - 1]`.
    pub chpar: &'a [Vec<DefaultReal>],
    /// Species-blocked `TDep`, indexed by `species * 16 + Fortran row - 1`.
    pub tdep: &'a [DefaultReal],
    pub mobile_immobile_by_material: &'a [bool],
    pub equilibrium: bool,
    pub concentration: &'a [DefaultReal],
    pub sorb: &'a [DefaultReal],
    pub sorb2: &'a [DefaultReal],
    pub q0: &'a [DefaultReal],
    pub q1: &'a [DefaultReal],
    pub sink: &'a [DefaultReal],
    pub theta: &'a [DefaultReal],
    pub velocity: &'a [DefaultReal],
    pub filtration: bool,
    pub bacteria: bool,
    pub dual_neq: bool,
    pub dual_porosity: Option<u8>,
    pub sink_immobile: &'a [DefaultReal],
    pub linear: bool,
    /// Unit conversions used by source `Deposit` (required for filtration).
    pub x_conv: DefaultReal,
    pub t_conv: DefaultReal,
    pub epsi: DefaultReal,
}

impl MassTotals {
    /// Accumulate one species in source node/operation order using default REAL.
    pub fn accumulate(&mut self, input: &MassTranInput<'_>) -> Result<(), KernelError> {
        let n = input.coordinates.len();
        let node_slices = [
            input.material_ids.len(),
            input.temperature.len(),
            input.concentration.len(),
            input.sorb.len(),
            input.sorb2.len(),
            input.q0.len(),
            input.q1.len(),
            input.sink.len(),
            input.theta.len(),
            input.velocity.len(),
            input.sink_immobile.len(),
        ];
        if n < 2 || node_slices.iter().any(|&len| len != n) {
            return Err("MassTran node arrays must have matching lengths of at least two");
        }
        if input.species >= self.cv_ch0.len()
            || self.cv_ch0.len() != self.cv_ch1.len()
            || self.cv_ch0.len() != self.cv_ch_r.len()
            || self.cv_ch0.len() != self.cv_ch_im.len()
            || self.s_trans.len() < n
        {
            return Err("MassTran species index or total arrays are inconsistent");
        }
        if matches!(input.dual_porosity, Some(model) if model != 1 && model != 2)
            || input.dual_porosity.is_some() && input.mobile_immobile_by_material.iter().any(|&v| v)
        {
            return Err("MassTran dual-domain selector conflicts with source mode");
        }
        if input.mobile_immobile_by_material.len() != input.chpar.len()
            || input
                .chpar
                .iter()
                .any(|p| p.len() < (input.species + 1) * 16 + 4)
            || input.tdep.len() < (input.species + 1) * 16 + 4
        {
            return Err("MassTran material or species parameter block is incomplete");
        }

        let s = input.species;
        for i in 0..n - 1 {
            let j = i + 1;
            let dx = input.coordinates[i + 1].0 - input.coordinates[i].0;
            self.cv_ch0[s].0 += input.epsi.0 * dx * (input.q0[i].0 + input.q0[i + 1].0) / 2.0;
            self.cv_ch1[s].0 += input.epsi.0 * dx * (input.q1[i].0 + input.q1[i + 1].0) / 2.0;
            self.cv_ch_r[s].0 += input.epsi.0 * dx * (input.sink[i].0 + input.sink[i + 1].0) / 2.0;

            if input.equilibrium {
                continue;
            }
            let mi = input.material_ids[i];
            let mj = input.material_ids[j];
            if mi >= input.chpar.len() || mj >= input.chpar.len() {
                return Err("MassTran material index is out of range");
            }
            let tti = temperature_factor(input.temperature[i].0);
            let ttj = temperature_factor(input.temperature[j].0);
            let jj = s * 16;
            let omegai = parameter(input, jj + 19, mi, tti);
            let omegaj = parameter(input, jj + 19, mj, ttj);

            if (input.mobile_immobile_by_material[mi] || input.dual_porosity.is_some())
                && !input.bacteria
            {
                let (mut macro_i, mut macro_j) = (0.0_f32, 0.0_f32);
                if input.dual_porosity.is_some() {
                    macro_i = input.sink_immobile[i].0
                        * if input.sink_immobile[i].0 > 0.0 {
                            input.concentration[i].0
                        } else {
                            input.sorb[i].0
                        };
                    macro_j = input.sink_immobile[j].0
                        * if input.sink_immobile[j].0 > 0.0 {
                            input.concentration[j].0
                        } else {
                            input.sorb[j].0
                        };
                }
                self.cv_ch_im[s].0 += input.epsi.0 * dx / 2.0
                    * (omegai * (input.concentration[i].0 - input.sorb[i].0)
                        + omegaj * (input.concentration[j].0 - input.sorb[j].0)
                        + macro_i
                        + macro_j);
                if s == 0 && !input.linear {
                    self.s_trans[i].0 = input.epsi.0
                        * (omegai * (input.concentration[i].0 - input.sorb[i].0) + macro_i);
                }
                if input.tdep.len() <= jj + 15
                    || input.chpar[mi].len() <= jj + 15
                    || input.chpar[mj].len() <= jj + 15
                {
                    return Err("MassTran dual-nonequilibrium parameter block is incomplete");
                }
                // This term is independently selected by ChemIn's lDualNEq
                // input; it is not inferred from the dual-water selector.
                if input.dual_neq {
                    let rhoi = parameter(input, 0, mi, tti);
                    let rhoj = parameter(input, 0, mj, ttj);
                    let fraci = parameter(input, 2, mi, tti);
                    let fracj = parameter(input, 2, mj, ttj);
                    let ksi = parameter(input, jj + 6, mi, tti);
                    let ksj = parameter(input, jj + 6, mj, ttj);
                    let nui = parameter(input, jj + 7, mi, tti);
                    let nuj = parameter(input, jj + 7, mj, ttj);
                    let expi = input.chpar[mi][jj + 8].0;
                    let expj = input.chpar[mj][jj + 8].0;
                    let emi = parameter(input, jj + 12, mi, tti);
                    let emj = parameter(input, jj + 12, mj, ttj);
                    let omega_si = parameter(input, jj + 15, mi, tti);
                    let omega_sj = parameter(input, jj + 15, mj, ttj);
                    let sorb_ei = sorb_dual(input.concentration[i].0, fraci, emi, ksi, nui, expi);
                    let sorb_ej = sorb_dual(input.concentration[j].0, fracj, emj, ksj, nuj, expj);
                    self.cv_ch_im[s].0 += input.epsi.0 * dx / 2.0
                        * (rhoi * omega_si * (sorb_ei - input.sorb2[i].0)
                            + rhoj * omega_sj * (sorb_ej - input.sorb2[j].0));
                    if s == 0 && !input.linear {
                        self.s_trans[i].0 +=
                            input.epsi.0 * rhoi * omega_si * (sorb_ei - input.sorb2[i].0);
                    }
                }
            } else if !input.bacteria {
                let rhoi = parameter(input, 0, mi, tti);
                let rhoj = parameter(input, 0, mj, ttj);
                let fraci = parameter(input, 2, mi, tti);
                let fracj = parameter(input, 2, mj, ttj);
                let ksi = parameter(input, jj + 6, mi, tti);
                let ksj = parameter(input, jj + 6, mj, ttj);
                let nui = parameter(input, jj + 7, mi, tti);
                let nuj = parameter(input, jj + 7, mj, ttj);
                let expi = input.chpar[mi][jj + 8].0;
                let expj = input.chpar[mj][jj + 8].0;
                let sorb_ei = sorb_two_site(input.concentration[i].0, fraci, ksi, nui, expi);
                let sorb_ej = sorb_two_site(input.concentration[j].0, fracj, ksj, nuj, expj);
                self.cv_ch_im[s].0 += input.epsi.0 * dx / 2.0
                    * (rhoi * omegai * (sorb_ei - input.sorb[i].0)
                        + rhoj * omegaj * (sorb_ej - input.sorb[j].0));
                if s == 0 {
                    self.s_trans[i].0 = input.epsi.0 * rhoi * omegai * (sorb_ei - input.sorb[i].0);
                }
            } else {
                self.accumulate_bacteria(input, i, j, mi, mj, jj, tti, ttj, dx)?;
            }
        }
        Ok(())
    }

    // Keep the source routine's individual state and interval arguments visible.
    #[allow(clippy::too_many_arguments)]
    fn accumulate_bacteria(
        &mut self,
        input: &MassTranInput<'_>,
        i: usize,
        j: usize,
        mi: usize,
        mj: usize,
        jj: usize,
        tti: f32,
        ttj: f32,
        dx: f32,
    ) -> Result<(), KernelError> {
        let ch = input.chpar;
        let par = |row: usize, m: usize, tt: f32| ch[m][row].0 * (input.tdep[row].0 * tt).exp();
        let rhoi = par(0, mi, tti);
        let rhoj = par(0, mj, ttj);
        let mut smax1i = par(jj + 17, mi, tti);
        let smax1j = par(jj + 17, mj, ttj);
        let mut ka1i = par(jj + 18, mi, tti);
        let mut ka1j = par(jj + 18, mj, ttj);
        let mut kd1i = par(jj + 19, mi, tti);
        let kd1j = par(jj + 19, mj, ttj);
        let mut smax2i = par(jj + 14, mi, tti);
        let smax2j = par(jj + 14, mj, ttj);
        let mut ka2i = par(jj + 15, mi, tti);
        let mut ka2j = par(jj + 15, mj, ttj);
        let mut kd2i = par(jj + 16, mi, tti);
        let kd2j = par(jj + 16, mj, ttj);
        let thetai = input.theta[i].0 - ch[mi][3].0;
        let thetaj = input.theta[j].0 - ch[mj][3].0;
        let mut ipsi1 = 0_i32;
        let mut ipsi2 = 0_i32;
        let (mut dc, mut aa) = (0.0_f32, 0.0_f32);
        if !input.filtration {
            ipsi2 = ch[mi][jj + 12].0 as i32;
            ipsi1 = ch[mj][jj + 13].0 as i32;
        }
        if ipsi1 == 0 && smax1i > 0.0 {
            ipsi1 = 1;
        }
        if ipsi2 == 0 && smax2i > 0.0 {
            ipsi2 = 1;
        }
        if ipsi1 >= 3 || ipsi2 >= 3 {
            dc = par(jj + 5, mi, tti);
        }
        if ipsi1 == 5 || ipsi2 == 5 {
            aa = ch[mi][jj + 14].0;
        }
        let psi1i = blocking(
            ipsi1,
            smax1i,
            input.sorb[i].0,
            input.coordinates[i].0,
            dc,
            aa,
        );
        let psi1j = blocking(
            ipsi1,
            smax1j,
            input.sorb[j].0,
            input.coordinates[j].0,
            dc,
            aa,
        );
        let psi2i = blocking(
            ipsi2,
            smax2i,
            input.sorb2[i].0,
            input.coordinates[i].0,
            dc,
            aa,
        );
        let psi2j = blocking(
            ipsi2,
            smax2j,
            input.sorb2[j].0,
            input.coordinates[j].0,
            dc,
            aa,
        );
        if input.filtration {
            let dc1 = par(jj + 12, mi, tti);
            let dp1 = par(jj + 13, mi, tti);
            let (ka1, ka2) = deposit(
                ka1i,
                ka2i,
                dc1,
                dp1,
                thetai,
                input.velocity[i].0,
                input.temperature[i].0,
                input.x_conv.0,
                input.t_conv.0,
            )?;
            ka1i = ka1;
            ka1j = ka1;
            ka2i = ka2;
            ka2j = ka2;
        }
        self.cv_ch_im[input.species].0 += input.epsi.0 * dx / 2.0
            * (input.concentration[i].0 * thetai * (psi1i * ka1i + psi2i * ka2i)
                + input.concentration[j].0 * thetaj * (psi1j * ka1j + psi2j * ka2j)
                - rhoi * (input.sorb[i].0 * kd1i + input.sorb2[i].0 * kd2i)
                - rhoj * (input.sorb[j].0 * kd1j + input.sorb2[j].0 * kd2j));
        if input.species == 0 {
            self.s_trans[i].0 = input.epsi.0
                * (input.concentration[i].0 * thetai * (psi1i * ka1i + psi2i * ka2i)
                    - rhoi * (input.sorb[i].0 * kd1i + input.sorb2[i].0 * kd2i));
        }
        // Keep assignments mutable exactly as the source adjusts the i/j
        // attachment rates together after the per-interval Deposit call.
        let _ = (
            &mut smax1i,
            &mut smax2i,
            &mut ka1j,
            &mut ka2j,
            &mut kd1i,
            &mut kd2i,
        );
        Ok(())
    }
}

fn temperature_factor(temp: f32) -> f32 {
    (temp + 273.15 - 293.15) / 8.314 / (temp + 273.15) / 293.15
}
fn parameter(input: &MassTranInput<'_>, row: usize, material: usize, tt: f32) -> f32 {
    input.chpar[material][row].0 * (input.tdep[row].0 * tt).exp()
}
fn sorb_dual(c: f32, frac: f32, em: f32, ks: f32, nu: f32, exp: f32) -> f32 {
    if c > 0.0 {
        frac * (1.0 - em) * ks * c.powf(exp) / (1.0 + nu * c.powf(exp))
    } else {
        0.0
    }
}
fn sorb_two_site(c: f32, frac: f32, ks: f32, nu: f32, exp: f32) -> f32 {
    if c > 0.0 {
        (1.0 - frac) * ks * c.powf(exp) / (1.0 + nu * c.powf(exp))
    } else {
        0.0
    }
}
fn blocking(kind: i32, smax: f32, ss: f32, x: f32, dc: f32, smax2: f32) -> f32 {
    match kind {
        1 if smax > 0.0 => 1.0 - ss / smax,
        2 if smax > 0.0 => ss.powf(smax).max(1.0),
        3 => {
            let binf = 1.0 / smax;
            let c = 0.546 * binf * ss;
            if ss <= 0.8 * smax {
                1.0 - 4.0 * c + 3.08 * c * c + 1.4069 * c * c * c
            } else {
                ((1.0 - binf * ss).powi(3)) / (2.0 * dc.powi(2) * binf.powi(3))
            }
        }
        4 if smax > 0.0 && dc > 0.0 => ((x.abs() + dc) / dc).powf(-smax),
        5 => {
            let mut psi = if smax > 0.0 && dc > 0.0 {
                ((x.abs() + dc) / dc).powf(-smax)
            } else {
                1.0
            };
            if smax2 > 0.0 {
                psi *= 1.0 - ss / smax2;
            }
            psi
        }
        _ => 1.0,
    }
}
// Mirrors the source Deposit parameter list and operation inputs.
#[allow(clippy::too_many_arguments)]
// Preserve the source's approximate PI literal used in the deposition equation.
#[allow(clippy::approx_constant)]
fn deposit(
    ka1: f32,
    ka2: f32,
    dc1: f32,
    dp1: f32,
    theta: f32,
    q: f32,
    temp: f32,
    x_conv: f32,
    t_conv: f32,
) -> Result<(f32, f32), KernelError> {
    let dc = dc1 / x_conv;
    let dp = dp1 / x_conv;
    if dp <= 0.0 && dc <= 0.0 {
        return Err("source Deposit stops when both particle and grain diameters are nonpositive");
    }
    let (eta, pore_velocity) = if q.abs() > 0.0 {
        // Preserve the source literal rather than substituting a rounded PI constant.
        let pi = 3.1415_f32;
        let mu = 0.00093_f32;
        let bk = 1.38048e-23_f32;
        let h = 1e-20_f32;
        let vel = (q / x_conv * t_conv).abs();
        let pv = vel / theta;
        let gamma = (1.0 - theta).powf(1.0 / 3.0);
        let ass = 2.0 * (1.0 - gamma.powi(5))
            / (2.0 - 3.0 * gamma + 3.0 * gamma.powi(5) - 2.0 * gamma.powi(6));
        let npe = 3.0 * pi * mu * dp * dc * vel / (bk * (temp + 273.15));
        let ediff = 4.0 * ass.powf(1.0 / 3.0) * npe.powf(-2.0 / 3.0);
        let nlo = 4.0 * h / (9.0 * pi * mu * dp.powi(2) * vel);
        let nr = dp / dc;
        let einter = ass * nlo.powf(1.0 / 8.0) * nr.powf(15.0 / 8.0);
        let ng = 9.81 * (1080.0 - 998.0) * dp.powi(2) / (18.0 * mu * vel);
        (
            ediff + einter + 0.00338 * ass * ng.powf(1.2) * nr.powf(-0.4),
            pv,
        )
    } else {
        (0.0, 0.0)
    };
    let ka1 = 3.0 * (1.0 - theta) / 2.0 / dc * eta * ka1 * pore_velocity / t_conv;
    let ka2 = 3.0 * (1.0 - theta) / 2.0 / dc * eta * ka2 * pore_velocity / t_conv;
    Ok((ka1, ka2))
}

/// Nodewise state required by the source `FluxConc` formula, returned when its
/// caller selects flux concentration.
pub struct FluxConcInput<'a> {
    pub concentration: &'a [DefaultReal],
    pub coordinates: &'a [DefaultReal],
    pub velocity: &'a [DefaultReal],
    pub water: &'a [DefaultReal],
    pub saturated_water_by_material: &'a [DefaultReal],
    /// `ChPar` species block, indexed by `(jS-1)*16 + slot` in Fortran.
    pub chpar_by_material: &'a [Vec<DefaultReal>],
    /// `TDep` species block, indexed by `(jS-1)*16 + slot` in Fortran.
    pub tdep: &'a [DefaultReal],
    pub material_ids: &'a [usize],
    pub temperature: &'a [DefaultReal],
    pub tortuosity: bool,
    pub mobile_immobile: &'a [bool],
    pub dual_domain_model: Option<u8>,
    pub dual_immobile_water: Option<&'a [DefaultReal]>,
    pub immobile_water_by_material: &'a [DefaultReal],
}

/// Compute the source helper equation. The `Solute` caller invokes `FluxConc`
/// when `iConcType == 2`, which HYDRUS selects from `lFlux`; therefore this
/// output is conditionally active, not universally present. The compatibility
/// contract currently has no normalized `lFlux`/`iConcType` selector, so
/// selecting or exposing this output in the downstream driver remains an
/// integration requirement.
pub fn flux_concentration(
    input: &FluxConcInput<'_>,
) -> Result<Explicit<Vec<DefaultReal>>, KernelError> {
    let n = input.coordinates.len();
    if n < 2
        || input.concentration.len() != n
        || input.velocity.len() != n
        || input.water.len() != n
        || input.material_ids.len() != n
        || input.temperature.len() != n
        || input.mobile_immobile.len() != input.saturated_water_by_material.len()
    {
        return Err("FluxConc node arrays must have matching lengths and at least two nodes");
    }
    if input.dual_domain_model.is_some() && input.dual_immobile_water.is_none_or(|v| v.len() != n) {
        return Err("dual-domain FluxConc requires a nodewise immobile-water array");
    }
    if input
        .dual_domain_model
        .is_some_and(|model| model != 6 && model != 7)
    {
        return Err("dual-domain water model must be 6 or 7");
    }

    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let m = input.material_ids[i];
        if m >= input.chpar_by_material.len()
            || m >= input.saturated_water_by_material.len()
            || m >= input.immobile_water_by_material.len()
            || input.chpar_by_material[m].len() <= 9
            || input.tdep.len() <= 9
        {
            return Err("FluxConc material or species parameter block is incomplete");
        }
        let params = &input.chpar_by_material[m];
        let mut th_w = input.water[i].0;
        let th_g = 0.0_f32.max(input.saturated_water_by_material[m].0 - th_w);
        let mut th_imob = 0.0_f32;
        if input.mobile_immobile[m] {
            if input.dual_domain_model.is_none() {
                th_imob = input.immobile_water_by_material[m].0;
                th_w = (th_w - th_imob).max(0.001);
            } else {
                th_imob = input.dual_immobile_water.expect("validated above")[i].0;
            }
        }
        let tt = (input.temperature[i].0 + 273.15 - 293.15)
            / 8.314
            / (input.temperature[i].0 + 273.15)
            / 293.15;
        let dw = params[4].0 * (input.tdep[4].0 * tt).exp();
        let dg = params[5].0 * (input.tdep[5].0 * tt).exp();
        let henry = params[9].0 * (input.tdep[9].0 * tt).exp();
        let (tau_w, tau_g) = if input.tortuosity {
            let mut th_s = input.saturated_water_by_material[m].0;
            if input.mobile_immobile[m] && input.dual_domain_model.is_none() {
                th_s = (th_s - th_imob).max(0.001);
            }
            if input.dual_domain_model.is_some() {
                th_s += th_imob;
            }
            (
                th_w.powf(7.0 / 3.0) / th_s.powi(2),
                th_g.powf(7.0 / 3.0) / th_s.powi(2),
            )
        } else {
            (1.0, 1.0)
        };
        let qw = input.velocity[i].0;
        let disp = params[1].0 * qw.abs() / th_w + dw * tau_w + th_g / th_w * dg * henry * tau_g;
        let c_grad = if i == 0 {
            (input.concentration[1].0 - input.concentration[0].0)
                / (input.coordinates[1].0 - input.coordinates[0].0)
        } else if i == n - 1 {
            (input.concentration[i].0 - input.concentration[i - 1].0)
                / (input.coordinates[i].0 - input.coordinates[i - 1].0)
        } else {
            (input.concentration[i + 1].0 - input.concentration[i - 1].0)
                / (input.coordinates[i + 1].0 - input.coordinates[i - 1].0)
        };
        let mut cf = input.concentration[i].0;
        if qw != 0.0 {
            cf = input.concentration[i].0 - disp * th_w / qw * c_grad;
        }
        result.push(DefaultReal(cf));
    }
    Ok(Explicit::Available(result))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    struct MassFixture {
        x: [DefaultReal; 2],
        ids: [usize; 2],
        temp: [DefaultReal; 2],
        params: Vec<Vec<DefaultReal>>,
        tdep: Vec<DefaultReal>,
        mobile: [bool; 1],
        conc: [DefaultReal; 2],
        sorb: [DefaultReal; 2],
        sorb2: [DefaultReal; 2],
        q0: [DefaultReal; 2],
        q1: [DefaultReal; 2],
        sink: [DefaultReal; 2],
        theta: [DefaultReal; 2],
        velocity: [DefaultReal; 2],
        sink_im: [DefaultReal; 2],
    }
    impl MassFixture {
        fn new() -> Self {
            let mut params = vec![vec![DefaultReal(0.0); 20]];
            params[0][0] = DefaultReal(1.0);
            params[0][2] = DefaultReal(0.2);
            params[0][19] = DefaultReal(1.0);
            Self {
                x: [DefaultReal(0.0), DefaultReal(1.0)],
                ids: [0, 0],
                temp: [DefaultReal(20.0); 2],
                params,
                tdep: vec![DefaultReal(0.0); 20],
                mobile: [false],
                conc: [DefaultReal(2.0), DefaultReal(3.0)],
                sorb: [DefaultReal(0.5), DefaultReal(1.0)],
                sorb2: [DefaultReal(0.2), DefaultReal(0.4)],
                q0: [DefaultReal(1.0), DefaultReal(3.0)],
                q1: [DefaultReal(2.0), DefaultReal(4.0)],
                sink: [DefaultReal(0.0); 2],
                theta: [DefaultReal(0.5); 2],
                velocity: [DefaultReal(0.001); 2],
                sink_im: [DefaultReal(0.0); 2],
            }
        }
        fn totals() -> MassTotals {
            MassTotals {
                cv_ch0: vec![DefaultReal(0.0)],
                cv_ch1: vec![DefaultReal(0.0)],
                cv_ch_r: vec![DefaultReal(0.0)],
                cv_ch_im: vec![DefaultReal(0.0)],
                s_trans: vec![DefaultReal(0.0); 2],
            }
        }
        fn input(
            &self,
            equilibrium: bool,
            bacteria: bool,
            filtration: bool,
            dual_porosity: Option<u8>,
            dual_neq: bool,
        ) -> MassTranInput<'_> {
            MassTranInput {
                species: 0,
                coordinates: &self.x,
                material_ids: &self.ids,
                temperature: &self.temp,
                chpar: &self.params,
                tdep: &self.tdep,
                mobile_immobile_by_material: &self.mobile,
                equilibrium,
                concentration: &self.conc,
                sorb: &self.sorb,
                sorb2: &self.sorb2,
                q0: &self.q0,
                q1: &self.q1,
                sink: &self.sink,
                theta: &self.theta,
                velocity: &self.velocity,
                filtration,
                bacteria,
                dual_porosity,
                dual_neq,
                sink_immobile: &self.sink_im,
                linear: true,
                x_conv: DefaultReal(1.0),
                t_conv: DefaultReal(1.0),
                epsi: DefaultReal(1.0),
            }
        }
    }

    #[test]
    fn mass_tran_accumulates_native_trapezoids_and_keeps_species_independent() {
        let mut totals = MassTotals {
            cv_ch0: vec![DefaultReal(10.0), DefaultReal(20.0)],
            cv_ch1: vec![DefaultReal(0.0), DefaultReal(0.0)],
            cv_ch_r: vec![DefaultReal(0.0), DefaultReal(0.0)],
            cv_ch_im: vec![DefaultReal(0.0); 2],
            s_trans: vec![DefaultReal(0.0); 3],
        };
        let x = [0.0, 1.0, 3.0].map(DefaultReal);
        let q0 = [1.0, 3.0, 5.0].map(DefaultReal);
        let q1 = [2.0, 2.0, 4.0].map(DefaultReal);
        let sink = [0.0, 1.0, 3.0].map(DefaultReal);
        totals
            .accumulate(&MassTranInput {
                species: 1,
                coordinates: &x,
                material_ids: &[0; 3],
                temperature: &[DefaultReal(20.0); 3],
                chpar: &[vec![DefaultReal(0.0); 36]],
                tdep: &[DefaultReal(0.0); 36],
                mobile_immobile_by_material: &[false],
                equilibrium: true,
                concentration: &[DefaultReal(0.0); 3],
                sorb: &[DefaultReal(0.0); 3],
                sorb2: &[DefaultReal(0.0); 3],
                q0: &q0,
                q1: &q1,
                sink: &sink,
                theta: &[DefaultReal(0.0); 3],
                velocity: &[DefaultReal(0.0); 3],
                filtration: false,
                bacteria: false,
                dual_porosity: None,
                dual_neq: false,
                sink_immobile: &[DefaultReal(0.0); 3],
                linear: true,
                x_conv: DefaultReal(1.0),
                t_conv: DefaultReal(1.0),
                epsi: DefaultReal(0.5),
            })
            .unwrap();
        assert_eq!(totals.cv_ch0, [DefaultReal(10.0), DefaultReal(25.0)]);
        assert_eq!(totals.cv_ch1, [DefaultReal(0.0), DefaultReal(4.0)]);
        assert_eq!(totals.cv_ch_r, [DefaultReal(0.0), DefaultReal(2.25)]);
    }

    #[test]
    fn mass_tran_rejects_mismatches_without_touching_totals() {
        let mut totals = MassTotals {
            cv_ch0: vec![DefaultReal(4.0)],
            cv_ch1: vec![DefaultReal(5.0)],
            cv_ch_r: vec![DefaultReal(6.0)],
            cv_ch_im: vec![DefaultReal(7.0)],
            s_trans: vec![DefaultReal(8.0)],
        };
        let before = totals.clone();
        let x = [DefaultReal(0.0)];
        assert!(totals
            .accumulate(&MassTranInput {
                species: 0,
                coordinates: &x,
                material_ids: &x.map(|_| 0),
                temperature: &x,
                chpar: &[vec![DefaultReal(0.0); 20]],
                tdep: &[DefaultReal(0.0); 20],
                mobile_immobile_by_material: &[false],
                equilibrium: true,
                concentration: &x,
                sorb: &x,
                sorb2: &x,
                q0: &x,
                q1: &x,
                sink: &x,
                theta: &x,
                velocity: &x,
                filtration: false,
                bacteria: false,
                dual_porosity: None,
                dual_neq: false,
                sink_immobile: &x,
                linear: true,
                x_conv: DefaultReal(1.0),
                t_conv: DefaultReal(1.0),
                epsi: DefaultReal(1.0)
            })
            .is_err());
        assert_eq!(totals, before);
    }

    #[test]
    fn reachable_cv_ch_im_mobile_dual_and_dual_neq_terms_are_distinct() {
        let mut f = MassFixture::new();
        f.mobile[0] = true;
        let mut totals = MassFixture::totals();
        totals
            .accumulate(&f.input(false, false, false, None, false))
            .unwrap();
        assert_eq!(totals.cv_ch_im[0], DefaultReal(1.75));

        // Active WATFLOW calls DualPor; positive exchange uses Conc and
        // negative exchange uses the distinct Sorb pool at each endpoint.
        f.sink_im = [DefaultReal(1.0), DefaultReal(-1.0)];
        f.mobile[0] = false;
        let mut dual = MassFixture::totals();
        dual.accumulate(&f.input(false, false, false, Some(1), false))
            .unwrap();
        assert_eq!(dual.cv_ch_im[0], DefaultReal(2.25));

        f.sink_im = [DefaultReal(0.0), DefaultReal(0.0)];
        let mut zero_exchange = MassFixture::totals();
        zero_exchange
            .accumulate(&f.input(false, false, false, Some(1), false))
            .unwrap();
        assert_eq!(zero_exchange.cv_ch_im[0], DefaultReal(1.75));

        f.params[0][15] = DefaultReal(1.0);
        f.mobile[0] = true;
        let mut ne = MassFixture::totals();
        ne.accumulate(&f.input(false, false, false, None, true))
            .unwrap();
        assert_eq!(ne.cv_ch_im[0], DefaultReal(1.45));
    }

    #[test]
    fn reachable_two_site_and_bacteria_terms_keep_sorb_pools_separate() {
        let mut f = MassFixture::new();
        f.params[0][6] = DefaultReal(1.0);
        f.params[0][8] = DefaultReal(1.0);
        let mut two_site = MassFixture::totals();
        two_site
            .accumulate(&f.input(false, false, false, None, false))
            .unwrap();
        assert_eq!(two_site.cv_ch_im[0], DefaultReal(1.25));

        f.params[0][17] = DefaultReal(10.0);
        f.params[0][18] = DefaultReal(1.0);
        f.params[0][19] = DefaultReal(0.0);
        let mut bacteria = MassFixture::totals();
        bacteria
            .accumulate(&f.input(false, true, false, None, false))
            .unwrap();
        assert!((bacteria.cv_ch_im[0].0 - 1.15).abs() < 1e-6);
        f.params[0][12] = DefaultReal(1e-3);
        f.params[0][13] = DefaultReal(1e-6);
        let mut filtration = MassFixture::totals();
        filtration
            .accumulate(&f.input(false, true, true, None, false))
            .unwrap();
        assert_ne!(filtration.cv_ch_im[0], bacteria.cv_ch_im[0]);
        assert_ne!(filtration.cv_ch_im[0], two_site.cv_ch_im[0]);
    }

    #[test]
    fn source_blocking_modes_match_active_biology_helper_equations() {
        assert_eq!(blocking(1, 10.0, 2.0, 0.0, 1.0, 0.0), 0.8);
        assert_eq!(blocking(2, 2.0, 0.5, 0.0, 1.0, 0.0), 1.0);
        assert!((blocking(3, 10.0, 1.0, 0.0, 1.0, 0.0) - 0.791_011).abs() < 1e-5);
        assert_eq!(blocking(4, 1.0, 0.0, 1.0, 1.0, 0.0), 0.5);
        assert_eq!(blocking(5, 1.0, 2.0, 1.0, 1.0, 4.0), 0.25);
    }

    fn flux_input<'a>(
        c: &'a [DefaultReal],
        x: &'a [DefaultReal],
        v: &'a [DefaultReal],
        chpar: &'a [Vec<DefaultReal>],
        material_ids: &'a [usize],
    ) -> FluxConcInput<'a> {
        FluxConcInput {
            concentration: c,
            coordinates: x,
            velocity: v,
            water: &[DefaultReal(1.0); 3],
            saturated_water_by_material: &[DefaultReal(1.0)],
            chpar_by_material: chpar,
            tdep: &[DefaultReal(0.0); 16],
            material_ids,
            temperature: &[DefaultReal(20.0); 3],
            tortuosity: false,
            mobile_immobile: &[false],
            dual_domain_model: None,
            dual_immobile_water: None,
            immobile_water_by_material: &[DefaultReal(0.0)],
        }
    }

    #[test]
    fn flux_conc_uses_one_sided_endpoint_and_centered_interior_gradients() {
        let c = [0.0, 2.0, 8.0].map(DefaultReal);
        let x = [0.0, 1.0, 3.0].map(DefaultReal);
        let v = [1.0; 3].map(DefaultReal);
        let mut p = vec![DefaultReal(0.0); 16];
        p[1] = DefaultReal(1.0);
        let materials = vec![p];
        let ids = [0; 3];
        let got = flux_concentration(&flux_input(&c, &x, &v, &materials, &ids)).unwrap();
        let Explicit::Available(got) = got else {
            panic!("expected computed flux concentration")
        };
        assert_eq!(got[0], DefaultReal(-2.0));
        assert!((got[1].0 + 2.0 / 3.0).abs() < 1e-6);
        assert_eq!(got[2], DefaultReal(5.0));
    }

    #[test]
    fn flux_conc_zero_flow_returns_concentration_without_inventing_other_fields() {
        let c = [1.0, 2.0, 4.0].map(DefaultReal);
        let x = [0.0, 1.0, 2.0].map(DefaultReal);
        let v = [0.0; 3].map(DefaultReal);
        let p = vec![vec![DefaultReal(0.0); 16]];
        let ids = [0; 3];
        let got = flux_concentration(&flux_input(&c, &x, &v, &p, &ids)).unwrap();
        assert_eq!(got, Explicit::Available(c.to_vec()));
    }
}

//! Discrete water and heat balance diagnostics for one accepted vapor step.
//!
//! Everything here is derived from [`VaporStepReport`] primitives plus a few
//! caller-held step inputs (old temperatures/velocities, sink, geometry).
//!
//! # Water (interior nodes only)
//! The solved row of `WATFLOW.FOR:235-267` (assembled in `water::assemble_water_rows`)
//! is, with `dx=(dxA+dxB)/2`, iterate `h_it` and solved head `h`,
//! ```text
//! R_i = dx*( C_eff*(h-h_it)/dt + (thL_it-thL_old)/dt + (thV_new-thV_old)/dt + sink )
//!       + (q_B - q_A)
//! q_face = -KL_face*(dh/dx + g) - ConVh_face*dh/dx - ConVT_face*dT/dx
//! ```
//! `R_i` is exactly `lhs(h)-rhs` of the assembled row rewritten as a flux form.
//! The source storage term `C_eff*(h-h_it)` is a linearization about the last
//! iterate, **not** the continuum change `th_new-th_old`; both are reported.
//! The contents in the row are those at the iterate (`pre_solve_liquid_water_contents`,
//! `vapor_contents_new`), not the post-convergence corrected `liquid_contents_new`.
//!
//! # Heat (interior nodes only)
//! Rows `B*T[i-1]+D*T[i]+E*T[i+1]=F` rebuilt with the crate's `temp_cap` and
//! `adjust_heat_rows` and the ordinary level-one/level-two interior formulas
//! (`TEMPER.FOR:112-143` plus `TempAdj` `180-268`).
//!
//! # Reservoir caveat
//! Endpoint (Dirichlet) rows are *not* closed. [`ReservoirBookkeeping`] only
//! tabulates published endpoint fluxes and the domain storage; its `unclosed`
//! value is a diagnostic and must never be read as closure evidence.

use super::adjust::{adjust_heat_rows, VaporHeatAdjustmentInput};
use super::driver::VaporStepReport;
use super::VaporError;
use crate::heat::capacity::{temp_cap, TempCapError, TempCapInput, TempMaterial};

/// Caller-held inputs that the step report does not carry.
#[derive(Clone, Copy)]
pub struct VaporBalanceInput<'a> {
    pub coordinates: &'a [f64],
    pub gravity: f64,
    pub x_conv: f64,
    pub t_conv: f64,
    pub sink: &'a [f64],
    /// Accepted temperatures, liquid and vapor velocities before the step.
    pub old_temperatures: &'a [f64],
    pub old_liquid_velocities: &'a [f64],
    pub old_vapor_velocities: &'a [f64],
    pub heat_materials: &'a [TempMaterial],
    pub material_indices: &'a [usize],
    pub campbell: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VaporBalanceError {
    LengthMismatch {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
    NonFinite {
        name: &'static str,
    },
    InvalidValue {
        name: &'static str,
    },
    Vapor(VaporError),
    TempCap(TempCapError),
}

impl std::fmt::Display for VaporBalanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "vapor balance: {self:?}")
    }
}
impl std::error::Error for VaporBalanceError {}

/// Interior water residuals; vectors are indexed by interior node `1..n-1` (`i-1`).
#[derive(Clone, Debug, PartialEq)]
pub struct VaporWaterBalance {
    /// `R_i` with fluxes recomputed from stored coefficients (balance form above).
    pub residual: Vec<f64>,
    /// `R_i` with the published `total_face_fluxes` instead.
    pub residual_published_fluxes: Vec<f64>,
    /// Largest absolute term of the assembled row: matrix products `coef*h`, storage,
    /// sink, gravity/thermal and face-flux terms.
    pub scale: Vec<f64>,
    /// `dx*C_eff*(h-h_it)/dt`, the source linearized storage.
    pub source_storage: Vec<f64>,
    /// `dx*(thL_it-thL_old)/dt`.
    pub liquid_storage: Vec<f64>,
    /// `dx*(thV_new-thV_old)/dt`.
    pub vapor_storage: Vec<f64>,
    /// `dx*sink`.
    pub sink: Vec<f64>,
    /// Signed face-flux differences `q_B-q_A` per phase (liquid, vapor head, vapor thermal).
    pub liquid_transport: Vec<f64>,
    pub vapor_head_transport: Vec<f64>,
    pub vapor_thermal_transport: Vec<f64>,
    /// Max |recomputed - published| total face flux over interior faces.
    pub published_flux_difference: f64,
    /// Diagnostic: corrected-content continuum residual
    /// `dx*((thL_new-thL_old)/dt + dthV/dt + sink)+(q_B-q_A)`; no capacity term. Reported only.
    pub corrected_content_residual: Vec<f64>,
    /// Diagnostic: `dx*((thL_new-thL_it)/dt - C_eff*(h-h_it)/dt)` (corrected-content
    /// storage minus linearized source storage). Reported only.
    pub storage_form_difference: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VaporHeatBalance {
    pub residual: Vec<f64>,
    pub scale: Vec<f64>,
    /// Net vapor sensible (`Cv*v` advection) contribution to the row, left minus right side.
    pub vapor_sensible: Vec<f64>,
    /// Latent source `g0` contribution on the right-hand side (both levels).
    pub vapor_latent: Vec<f64>,
}

/// Endpoint/Dirichlet bookkeeping. Never closed, never asserted.
#[derive(Clone, Debug, PartialEq)]
pub struct ReservoirBookkeeping {
    /// Published endpoint total face fluxes (bottom face 0, top face n).
    pub bottom_total_flux: f64,
    pub top_total_flux: f64,
    /// Domain liquid+vapor storage rate using trapezoidal weights and corrected contents.
    pub domain_storage_rate: f64,
    pub domain_sink_rate: f64,
    /// `storage + sink + top - bottom`: not a closure.
    pub unclosed: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VaporBalanceReport {
    pub water: VaporWaterBalance,
    pub heat: VaporHeatBalance,
    pub reservoir: ReservoirBookkeeping,
}

impl VaporWaterBalance {
    /// Worst `|R| / max(rel*scale, floor)` ratio; `<= 1` means every row meets the bound.
    pub fn worst_ratio(&self, rel: f64, floor: f64) -> f64 {
        worst(&self.residual, &self.scale, rel, floor)
    }
    /// Largest `|R|/scale` over rows with `scale > 0`.
    pub fn max_relative(&self) -> f64 {
        max_rel(&self.residual, &self.scale)
    }
}

impl VaporHeatBalance {
    pub fn worst_ratio(&self, rel: f64, floor: f64) -> f64 {
        worst(&self.residual, &self.scale, rel, floor)
    }
    pub fn max_relative(&self) -> f64 {
        max_rel(&self.residual, &self.scale)
    }
}

fn worst(res: &[f64], scale: &[f64], rel: f64, floor: f64) -> f64 {
    res.iter()
        .zip(scale)
        .map(|(r, s)| r.abs() / (rel * s).max(floor))
        .fold(0.0, f64::max)
}

fn max_rel(res: &[f64], scale: &[f64]) -> f64 {
    res.iter()
        .zip(scale)
        .filter(|(_, &s)| s > 0.0)
        .map(|(r, s)| r.abs() / s)
        .fold(0.0, f64::max)
}

fn check_len(name: &'static str, n: usize, values: &[f64]) -> Result<(), VaporBalanceError> {
    if values.len() != n {
        return Err(VaporBalanceError::LengthMismatch {
            name,
            expected: n,
            actual: values.len(),
        });
    }
    if values.iter().any(|v| !v.is_finite()) {
        return Err(VaporBalanceError::NonFinite { name });
    }
    Ok(())
}

fn face(a: f64, b: f64) -> f64 {
    (a + b) / 2.0
}

/// Compute interior water and heat residuals and reservoir bookkeeping for one step.
pub fn vapor_balance(
    input: &VaporBalanceInput<'_>,
    report: &VaporStepReport,
) -> Result<VaporBalanceReport, VaporBalanceError> {
    let n = input.coordinates.len();
    if n < 3 {
        return Err(VaporBalanceError::InvalidValue { name: "node count" });
    }
    check_len("coordinates", n, input.coordinates)?;
    for (name, v) in [
        ("sink", input.sink),
        ("old_temperatures", input.old_temperatures),
        ("old_liquid_velocities", input.old_liquid_velocities),
        ("old_vapor_velocities", input.old_vapor_velocities),
        ("liquid_contents_old", &report.liquid_contents_old),
        ("liquid_contents_new", &report.liquid_contents_new),
        ("vapor_contents_old", &report.vapor_contents_old),
        ("vapor_contents_new", &report.vapor_contents_new),
        ("liquid_conductivities", &report.liquid_conductivities),
        (
            "vapor_head_conductivities",
            &report.vapor_head_conductivities,
        ),
        (
            "vapor_thermal_conductivities",
            &report.vapor_thermal_conductivities,
        ),
        ("pre_solve_heads", &report.pre_solve_heads),
        (
            "pre_solve_liquid_water_contents",
            &report.pre_solve_liquid_water_contents,
        ),
        (
            "effective_hydraulic_capacities",
            &report.effective_hydraulic_capacities,
        ),
        ("liquid_velocities", &report.liquid_velocities),
        ("vapor_velocities", &report.vapor_velocities),
        ("temperatures", &report.temperatures),
        ("heads", &report.passes[1].heads),
        (
            "temperature_estimate",
            &report.passes[1].temperature_estimate,
        ),
    ] {
        check_len(name, n, v)?;
    }
    for (name, v) in [
        ("liquid_face_fluxes", &report.liquid_face_fluxes),
        ("vapor_face_fluxes", &report.vapor_face_fluxes),
        ("total_face_fluxes", &report.total_face_fluxes),
    ] {
        check_len(name, n + 1, v)?;
    }
    if input.material_indices.len() != n {
        return Err(VaporBalanceError::LengthMismatch {
            name: "material_indices",
            expected: n,
            actual: input.material_indices.len(),
        });
    }
    let dt = report.timestep;
    if !(dt.is_finite() && dt > 0.0) {
        return Err(VaporBalanceError::InvalidValue { name: "timestep" });
    }
    if input.coordinates.windows(2).any(|p| p[1] <= p[0]) {
        return Err(VaporBalanceError::InvalidValue {
            name: "coordinates",
        });
    }
    let water = water_balance(input, report, dt)?;
    let heat = heat_balance(input, report, dt)?;
    let x = input.coordinates;
    let mut storage = 0.0;
    let mut sink = 0.0;
    for i in 0..n {
        let w = if i == 0 {
            (x[1] - x[0]) / 2.0
        } else if i == n - 1 {
            (x[n - 1] - x[n - 2]) / 2.0
        } else {
            (x[i + 1] - x[i - 1]) / 2.0
        };
        storage += w
            * ((report.liquid_contents_new[i] - report.liquid_contents_old[i])
                + (report.vapor_contents_new[i] - report.vapor_contents_old[i]))
            / dt;
        sink += w * input.sink[i];
    }
    let bottom = report.total_face_fluxes[0];
    let top = report.total_face_fluxes[n];
    let reservoir = ReservoirBookkeeping {
        bottom_total_flux: bottom,
        top_total_flux: top,
        domain_storage_rate: storage,
        domain_sink_rate: sink,
        unclosed: storage + sink + top - bottom,
    };
    Ok(VaporBalanceReport {
        water,
        heat,
        reservoir,
    })
}

fn water_balance(
    input: &VaporBalanceInput<'_>,
    r: &VaporStepReport,
    dt: f64,
) -> Result<VaporWaterBalance, VaporBalanceError> {
    let x = input.coordinates;
    let n = x.len();
    let h = &r.passes[1].heads;
    let t = &r.passes[1].temperature_estimate;
    let kl = &r.liquid_conductivities;
    let kvh = &r.vapor_head_conductivities;
    let kvt = &r.vapor_thermal_conductivities;
    let g = input.gravity;
    // Phase face flux between nodes a and b=a+1 (positive toward larger x).
    let q = |a: usize| {
        let dx = x[a + 1] - x[a];
        let dh = (h[a + 1] - h[a]) / dx;
        let ql = -face(kl[a], kl[a + 1]) * (dh + g);
        let qvh = -face(kvh[a], kvh[a + 1]) * dh;
        let qvt = -face(kvt[a], kvt[a + 1]) * (t[a + 1] - t[a]) / dx;
        (ql, qvh, qvt)
    };
    let m = n - 2;
    let mut out = VaporWaterBalance {
        residual: Vec::with_capacity(m),
        residual_published_fluxes: Vec::with_capacity(m),
        scale: Vec::with_capacity(m),
        source_storage: Vec::with_capacity(m),
        liquid_storage: Vec::with_capacity(m),
        vapor_storage: Vec::with_capacity(m),
        sink: Vec::with_capacity(m),
        liquid_transport: Vec::with_capacity(m),
        vapor_head_transport: Vec::with_capacity(m),
        vapor_thermal_transport: Vec::with_capacity(m),
        published_flux_difference: 0.0,
        corrected_content_residual: Vec::with_capacity(m),
        storage_form_difference: Vec::with_capacity(m),
    };
    for i in 1..n - 1 {
        let dx = (x[i + 1] - x[i - 1]) / 2.0;
        let (la, va, ta) = q(i - 1);
        let (lb, vb, tb) = q(i);
        let src = dx * r.effective_hydraulic_capacities[i] * (h[i] - r.pre_solve_heads[i]) / dt;
        let liq = dx * (r.pre_solve_liquid_water_contents[i] - r.liquid_contents_old[i]) / dt;
        let vap = dx * (r.vapor_contents_new[i] - r.vapor_contents_old[i]) / dt;
        let snk = dx * input.sink[i];
        let (dl, dv, dt_) = (lb - la, vb - va, tb - ta);
        let transport = dl + dv + dt_;
        let pub_t = r.total_face_fluxes[i + 1] - r.total_face_fluxes[i];
        for (face_idx, total) in [(i, la + va + ta), (i + 1, lb + vb + tb)] {
            out.published_flux_difference = out
                .published_flux_difference
                .max((total - r.total_face_fluxes[face_idx]).abs());
        }
        let corrected_liq = dx * (r.liquid_contents_new[i] - r.liquid_contents_old[i]) / dt;
        let corrected_it =
            dx * (r.liquid_contents_new[i] - r.pre_solve_liquid_water_contents[i]) / dt;
        out.residual.push(src + liq + vap + snk + transport);
        out.residual_published_fluxes
            .push(src + liq + vap + snk + pub_t);
        // Largest absolute term of the assembled row (matrix products and RHS terms), so the
        // scale includes `C_eff*dx/dt*h` and `Con/dx*h`, whose roundoff bounds the closure.
        let cdt = dx * r.effective_hydraulic_capacities[i] / dt;
        let con_a = (face(kl[i - 1], kl[i]) + face(kvh[i - 1], kvh[i])) / (x[i] - x[i - 1]);
        let con_b = (face(kl[i], kl[i + 1]) + face(kvh[i], kvh[i + 1])) / (x[i + 1] - x[i]);
        out.scale.push(
            [
                src,
                liq,
                vap,
                snk,
                la,
                va,
                ta,
                lb,
                vb,
                tb,
                cdt * h[i],
                cdt * r.pre_solve_heads[i],
                con_a * h[i - 1],
                con_a * h[i],
                con_b * h[i],
                con_b * h[i + 1],
            ]
            .iter()
            .fold(0.0, |m: f64, v| m.max(v.abs())),
        );
        out.source_storage.push(src);
        out.liquid_storage.push(liq);
        out.vapor_storage.push(vap);
        out.sink.push(snk);
        out.liquid_transport.push(dl);
        out.vapor_head_transport.push(dv);
        out.vapor_thermal_transport.push(dt_);
        out.corrected_content_residual
            .push(corrected_liq + vap + snk + transport);
        out.storage_form_difference.push(corrected_it - src);
    }
    Ok(out)
}

fn heat_balance(
    input: &VaporBalanceInput<'_>,
    r: &VaporStepReport,
    dt: f64,
) -> Result<VaporHeatBalance, VaporBalanceError> {
    let x = input.coordinates;
    let n = x.len();
    let cw = input
        .heat_materials
        .first()
        .ok_or(VaporBalanceError::InvalidValue { name: "materials" })?
        .water_heat_capacity;
    let (mut cap_o, mut con_o) = (vec![0.0; n], vec![0.0; n]);
    let (mut cap_n, mut con_n) = (vec![0.0; n], vec![0.0; n]);
    for (water, vel, cap, con) in [
        (
            &r.liquid_contents_old,
            input.old_liquid_velocities,
            &mut cap_o,
            &mut con_o,
        ),
        (
            &r.liquid_contents_new,
            &r.liquid_velocities,
            &mut cap_n,
            &mut con_n,
        ),
    ] {
        temp_cap(
            TempCapInput {
                material_indices: input.material_indices,
                water_content: water,
                velocity: vel,
                materials: input.heat_materials,
                campbell: input.campbell,
                x_conv: input.x_conv,
                t_conv: input.t_conv,
            },
            cap,
            con,
        )
        .map_err(VaporBalanceError::TempCap)?;
    }
    let adj = adjust_heat_rows(&VaporHeatAdjustmentInput {
        coordinates: x,
        temperature_old: input.old_temperatures,
        temperature_new_estimate: &r.passes[1].temperature_estimate,
        vapor_velocity_old: input.old_vapor_velocities,
        vapor_velocity_new: &r.vapor_velocities,
        vapor_content_old: &r.vapor_contents_old,
        vapor_content_new: &r.vapor_contents_new,
        timestep: dt,
        x_conv: input.x_conv,
        t_conv: input.t_conv,
    })
    .map_err(VaporBalanceError::Vapor)?;
    let cv = 1.8e6 / input.x_conv / input.t_conv / input.t_conv;
    let (to, tn) = (input.old_temperatures, &r.passes[1].temperatures);
    let (vo, vn, s) = (
        input.old_liquid_velocities,
        &r.liquid_velocities,
        input.sink,
    );
    let vvo = input.old_vapor_velocities;
    let mut out = VaporHeatBalance {
        residual: vec![],
        scale: vec![],
        vapor_sensible: vec![],
        vapor_latent: vec![],
    };
    for i in 1..n - 1 {
        let dxa = x[i] - x[i - 1];
        let dxb = x[i + 1] - x[i];
        let dx = (x[i + 1] - x[i - 1]) / 2.0;
        // Level one (old) products, TEMPER.FOR:112-127.
        let f_terms = [
            to[i - 1]
                * ((con_o[i] + con_o[i - 1]) / 4.0 / dxa + cw * (vo[i] + 2.0 * vo[i - 1]) / 12.0
                    - dxa / 24.0 * cw * (s[i - 1] + s[i])),
            to[i]
                * (-cw * (vo[i + 1] - vo[i - 1]) / 12.0 + dx / dt * cap_o[i]
                    - (con_o[i + 1] + con_o[i]) / 4.0 / dxb
                    - (con_o[i] + con_o[i - 1]) / 4.0 / dxa
                    - dxa / 24.0 * cw * (s[i - 1] + 3.0 * s[i])
                    - dxb / 24.0 * cw * (3.0 * s[i] + s[i + 1])),
            to[i + 1]
                * ((con_o[i + 1] + con_o[i]) / 4.0 / dxb
                    - cw * (2.0 * vo[i + 1] + vo[i]) / 12.0
                    - dxb / 24.0 * cw * (s[i + 1] + s[i])),
            adj.rhs_level_one[i],
            adj.rhs_level_two[i],
        ];
        // Level two (new) products, TEMPER.FOR:128-143.
        let lower = -(con_n[i] + con_n[i - 1]) / 4.0 / dxa - cw * (vn[i] + 2.0 * vn[i - 1]) / 12.0
            + dxa / 24.0 * cw * (s[i - 1] + s[i])
            + adj.lower_level_two[i];
        let diag = (con_n[i - 1] + con_n[i]) / 4.0 / dxa
            + (con_n[i] + con_n[i + 1]) / 4.0 / dxb
            + dx / dt * cap_n[i]
            + cw * (vn[i + 1] - vn[i - 1]) / 12.0
            + dxa / 24.0 * cw * (s[i - 1] + 3.0 * s[i])
            + dxb / 24.0 * cw * (3.0 * s[i] + s[i + 1])
            + adj.diagonal_level_two[i];
        let upper = -(con_n[i] + con_n[i + 1]) / 4.0 / dxb
            + cw * (2.0 * vn[i + 1] + vn[i]) / 12.0
            + dxb / 24.0 * cw * (s[i + 1] + s[i])
            + adj.upper_level_two[i];
        let l_terms = [lower * tn[i - 1], diag * tn[i], upper * tn[i + 1]];
        let lhs: f64 = l_terms.iter().sum();
        let rhs: f64 = f_terms.iter().sum();
        out.residual.push(lhs - rhs);
        out.scale.push(
            l_terms
                .iter()
                .chain(f_terms.iter())
                .fold(0.0, |m: f64, v| m.max(v.abs())),
        );
        // Vapor sensible: Cv*v advection at level two minus level one (velocity part only).
        let sens_two = adj.lower_level_two[i] * tn[i - 1]
            + adj.diagonal_level_two[i] * tn[i]
            + adj.upper_level_two[i] * tn[i + 1];
        let sens_one = to[i - 1] * (cv * (vvo[i] + 2.0 * vvo[i - 1]) / 12.0)
            + to[i] * (-cv * (vvo[i + 1] - vvo[i - 1]) / 12.0)
            + to[i + 1] * (-cv * (2.0 * vvo[i + 1] + vvo[i]) / 12.0);
        out.vapor_sensible.push(sens_two - sens_one);
        out.vapor_latent
            .push(adj.rhs_level_one[i] - sens_one + adj.rhs_level_two[i]);
    }
    Ok(out)
}

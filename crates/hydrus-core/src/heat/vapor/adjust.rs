//! Source-grouped vapor additions to the two heat-transport levels.

use super::{kernels::x_latent, VaporError};

/// Vapor state needed by source `TempAdj` for one old-to-new heat step.
#[derive(Clone, Copy)]
pub struct VaporHeatAdjustmentInput<'a> {
    pub coordinates: &'a [f64],
    pub temperature_old: &'a [f64],
    pub temperature_new_estimate: &'a [f64],
    pub vapor_velocity_old: &'a [f64],
    pub vapor_velocity_new: &'a [f64],
    pub vapor_content_old: &'a [f64],
    pub vapor_content_new: &'a [f64],
    pub timestep: f64,
    pub x_conv: f64,
    pub t_conv: f64,
}

/// Additions to the level-one RHS and level-two tridiagonal system.
#[derive(Clone, Debug, PartialEq)]
pub struct VaporHeatAdjustment {
    pub rhs_level_one: Vec<f64>,
    pub lower_level_two: Vec<f64>,
    pub diagonal_level_two: Vec<f64>,
    pub upper_level_two: Vec<f64>,
    pub rhs_level_two: Vec<f64>,
}

impl VaporHeatAdjustment {
    fn zeros(n: usize) -> Self {
        Self {
            rhs_level_one: vec![0.0; n],
            lower_level_two: vec![0.0; n],
            diagonal_level_two: vec![0.0; n],
            upper_level_two: vec![0.0; n],
            rhs_level_two: vec![0.0; n],
        }
    }
}

/// Calculate `TempAdj` additions without mutating the caller's heat rows.
pub fn adjust_heat_rows(
    input: &VaporHeatAdjustmentInput<'_>,
) -> Result<VaporHeatAdjustment, VaporError> {
    let n = validate(input)?;
    let (cv, g_old, g_new) = source_terms(input, n)?;

    let mut result = VaporHeatAdjustment::zeros(n);
    for i in 1..n - 1 {
        let dx_a = input.coordinates[i] - input.coordinates[i - 1];
        let dx_b = input.coordinates[i + 1] - input.coordinates[i];
        let old = input.vapor_velocity_old;
        let new = input.vapor_velocity_new;
        result.rhs_level_one[i] = input.temperature_old[i - 1]
            * (cv * (old[i] + 2.0 * old[i - 1]) / 12.0)
            + input.temperature_old[i] * (-cv * (old[i + 1] - old[i - 1]) / 12.0)
            + input.temperature_old[i + 1] * (-cv * (2.0 * old[i + 1] + old[i]) / 12.0)
            + dx_a * (g_old[i - 1] + 2.0 * g_old[i]) / 12.0
            + dx_b * (2.0 * g_old[i] + g_old[i + 1]) / 12.0;
        result.lower_level_two[i] = -cv * (new[i] + 2.0 * new[i - 1]) / 12.0;
        result.diagonal_level_two[i] = cv * (new[i + 1] - new[i - 1]) / 12.0;
        result.upper_level_two[i] = cv * (2.0 * new[i + 1] + new[i]) / 12.0;
        result.rhs_level_two[i] = dx_a * (g_new[i - 1] + 2.0 * g_new[i]) / 12.0
            + dx_b * (2.0 * g_new[i] + g_new[i + 1]) / 12.0;
    }
    check_finite(&result)?;
    Ok(result)
}

/// Endpoint `TempAdj` additions for a flux bottom (`kBotT<0`, TEMPER.FOR:220-232)
/// and a flux top (`kTopT<0`, TEMPER.FOR:249-260).
///
/// Only rows 0 (bottom) and `n-1` (top) can be non-zero. Dirichlet and
/// zero-gradient codes add nothing (TEMPER.FOR:220 tests `kBotT<0`; H2). The
/// explicit top storage term `-tTopA*Cv*(vVNew+vVOld)/2` is added only for
/// `kTopT == -1` (TEMPER.FOR:255; H1). Codes below -1 still get the matrix terms.
pub fn adjust_boundary_rows(
    input: &VaporHeatAdjustmentInput<'_>,
    k_top_t: i32,
    k_bot_t: i32,
    t_top_a: f64,
    t_bottom: f64,
) -> Result<VaporHeatAdjustment, VaporError> {
    let n = validate(input)?;
    let (cv, g_old, g_new) = source_terms(input, n)?;
    let x = input.coordinates;
    let t_old = input.temperature_old;
    let v_old = input.vapor_velocity_old;
    let v_new = input.vapor_velocity_new;
    let last = n - 1;
    let mut result = VaporHeatAdjustment::zeros(n);

    if k_bot_t < 0 {
        // TEMPER.FOR:220-232. Level 1 adds to F(1); level 2 adds D(1), E(1), F(1).
        let dx = x[1] - x[0];
        result.rhs_level_one[0] = t_old[0] * (-cv * (2.0 * v_old[0] + v_old[1]) / 12.0)
            + t_old[1] * (-cv * (2.0 * v_old[1] + v_old[0]) / 12.0)
            + dx / 12.0 * (2.0 * g_old[0] + g_old[1])
            + t_bottom * cv * (v_new[0] + v_old[0]) / 2.0;
        result.diagonal_level_two[0] = cv * (2.0 * v_new[0] + v_new[1]) / 12.0;
        result.upper_level_two[0] = cv * (2.0 * v_new[1] + v_new[0]) / 12.0;
        result.rhs_level_two[0] = dx / 12.0 * (2.0 * g_new[0] + g_new[1]);
    }

    if k_top_t < 0 {
        // TEMPER.FOR:249-260. Level 1 adds to F(N); level 2 subtracts from B(N) and D(N).
        let dx = x[last] - x[last - 1];
        let mut rhs = t_old[last - 1] * (cv * (v_old[last] + 2.0 * v_old[last - 1]) / 12.0)
            + t_old[last] * (cv * (2.0 * v_old[last] + v_old[last - 1]) / 12.0)
            + dx / 12.0 * (g_old[last - 1] + 2.0 * g_old[last]);
        if k_top_t == -1 {
            rhs -= t_top_a * cv * (v_new[last] + v_old[last]) / 2.0;
        }
        result.rhs_level_one[last] = rhs;
        result.lower_level_two[last] = -(cv * (v_new[last] + 2.0 * v_new[last - 1]) / 12.0);
        result.diagonal_level_two[last] = -(cv * (2.0 * v_new[last] + v_new[last - 1]) / 12.0);
        result.rhs_level_two[last] = dx / 12.0 * (g_new[last - 1] + 2.0 * g_new[last]);
    }

    check_finite(&result)?;
    Ok(result)
}

/// Vapor parts of the flux-top energy flux `HeatFl` (TEMPER.FOR:25-32), returned
/// separately so the caller can add them in source order: `HeatFl + a + b`.
///
/// `a = Cv*tTopA*(vVNew(N)+vVOld(N))/2` and `b = xLat*(vVNew(N)+vVOld(N))/2`.
/// Source `TempN(N)` maps to `temperature_new_estimate[N-1]`: the current
/// temperature estimate passed to the vapor driver, not `temp_old`.
pub fn top_heat_flux_vapor_terms(
    input: &VaporHeatAdjustmentInput<'_>,
    t_top_a: f64,
) -> Result<(f64, f64), VaporError> {
    let n = validate(input)?;
    let last = n - 1;
    let cv = 1.8e6 / input.x_conv / input.t_conv / input.t_conv;
    let latent = x_latent(input.temperature_new_estimate[last])?
        / input.x_conv
        / input.t_conv
        / input.t_conv;
    let v_sum = input.vapor_velocity_new[last] + input.vapor_velocity_old[last];
    let a = cv * t_top_a * v_sum / 2.0;
    let b = latent * v_sum / 2.0;
    finite("vapor flux term", 0, a)?;
    finite("vapor flux term", 0, b)?;
    Ok((a, b))
}

/// Shared input checks; returns the node count.
fn validate(input: &VaporHeatAdjustmentInput<'_>) -> Result<usize, VaporError> {
    let n = input.coordinates.len();
    if n < 3 {
        return Err(VaporError::TooFewNodes { count: n });
    }
    for (name, values) in [
        ("temperature_old", input.temperature_old),
        ("temperature_new_estimate", input.temperature_new_estimate),
        ("vapor_velocity_old", input.vapor_velocity_old),
        ("vapor_velocity_new", input.vapor_velocity_new),
        ("vapor_content_old", input.vapor_content_old),
        ("vapor_content_new", input.vapor_content_new),
    ] {
        if values.len() != n {
            return Err(VaporError::LengthMismatch {
                name,
                expected: n,
                actual: values.len(),
            });
        }
        for (index, &value) in values.iter().enumerate() {
            finite(name, index, value)?;
        }
    }
    for (name, values) in [("coordinates", input.coordinates)] {
        for (index, &value) in values.iter().enumerate() {
            finite(name, index, value)?;
        }
    }
    for (name, value) in [
        ("timestep", input.timestep),
        ("xConv", input.x_conv),
        ("tConv", input.t_conv),
    ] {
        finite(name, 0, value)?;
        if value <= 0.0 {
            return Err(VaporError::InvalidValue { name, value });
        }
    }
    for (index, pair) in input.coordinates.windows(2).enumerate() {
        if pair[1] <= pair[0] {
            return Err(VaporError::InvalidValue {
                name: "coordinates",
                value: pair[1],
            });
        }
        if !((pair[1] - pair[0]).is_finite()) {
            return Err(VaporError::NonFinite {
                name: "spacing",
                index: Some(index),
                value: pair[1] - pair[0],
            });
        }
    }
    Ok(n)
}

/// Source `Cv` and the per-node `g0` for the old (level 1) and new (level 2) states.
fn source_terms(
    input: &VaporHeatAdjustmentInput<'_>,
    n: usize,
) -> Result<(f64, Vec<f64>, Vec<f64>), VaporError> {
    let cv = 1.8e6 / input.x_conv / input.t_conv / input.t_conv;
    let mut g_old = Vec::with_capacity(n);
    let mut g_new = Vec::with_capacity(n);
    for i in 0..n {
        let grad_old = vapor_velocity_gradient(input.coordinates, input.vapor_velocity_old, i);
        let grad_new = vapor_velocity_gradient(input.coordinates, input.vapor_velocity_new, i);
        let latent_old =
            x_latent(input.temperature_old[i])? / input.x_conv / input.t_conv / input.t_conv;
        let latent_new = x_latent(input.temperature_new_estimate[i])?
            / input.x_conv
            / input.t_conv
            / input.t_conv;
        g_old.push(
            -latent_old
                * (grad_old
                    + (input.vapor_content_new[i] - input.vapor_content_old[i]) / input.timestep),
        );
        g_new.push(
            -latent_new
                * (grad_new
                    + (input.vapor_content_new[i] - input.vapor_content_old[i]) / input.timestep),
        );
    }
    Ok((cv, g_old, g_new))
}

fn check_finite(result: &VaporHeatAdjustment) -> Result<(), VaporError> {
    for (name, values) in [
        ("level-one RHS", &result.rhs_level_one),
        ("level-two lower", &result.lower_level_two),
        ("level-two diagonal", &result.diagonal_level_two),
        ("level-two upper", &result.upper_level_two),
        ("level-two RHS", &result.rhs_level_two),
    ] {
        for (index, &value) in values.iter().enumerate() {
            finite(name, index, value)?;
        }
    }
    Ok(())
}

fn vapor_velocity_gradient(x: &[f64], velocity: &[f64], i: usize) -> f64 {
    let last = x.len() - 1;
    if i == 0 {
        (velocity[1] - velocity[0]) / (x[1] - x[0])
    } else if i == last {
        (velocity[last] - velocity[last - 1]) / (x[last] - x[last - 1])
    } else {
        (velocity[i + 1] - velocity[i - 1]) / (x[i + 1] - x[i - 1]) * 2.0
    }
}

fn finite(name: &'static str, index: usize, value: f64) -> Result<(), VaporError> {
    if !value.is_finite() {
        return Err(VaporError::NonFinite {
            name,
            index: Some(index),
            value,
        });
    }
    Ok(())
}

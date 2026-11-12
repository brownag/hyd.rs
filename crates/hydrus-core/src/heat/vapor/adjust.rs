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

/// Calculate `TempAdj` additions without mutating the caller's heat rows.
pub fn adjust_heat_rows(
    input: &VaporHeatAdjustmentInput<'_>,
) -> Result<VaporHeatAdjustment, VaporError> {
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

    let mut result = VaporHeatAdjustment {
        rhs_level_one: vec![0.0; n],
        lower_level_two: vec![0.0; n],
        diagonal_level_two: vec![0.0; n],
        upper_level_two: vec![0.0; n],
        rhs_level_two: vec![0.0; n],
    };
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
    Ok(result)
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

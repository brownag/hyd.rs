//! Source-grouped vapor thermodynamic material kernels.

use super::{VaporError, VaporPrimitiveInput, VaporProperties};

/// Evaluate `ConVapor` and `VaporContent` for the supported enhancement-on mode.
///
/// Arrays are returned together so invalid derived values cannot publish a
/// partially updated set of material properties.
pub fn evaluate(input: &VaporPrimitiveInput<'_>) -> Result<VaporProperties, VaporError> {
    input.validate()?;

    let mut liquid_thermal = Vec::with_capacity(input.heads.len());
    let mut vapor_head = Vec::with_capacity(input.heads.len());
    let mut vapor_thermal = Vec::with_capacity(input.heads.len());
    let mut vapor_contents = Vec::with_capacity(input.heads.len());
    let mut effective_capacity = Vec::with_capacity(input.heads.len());

    for i in 0..input.heads.len() {
        let h = input.heads[i] / input.x_conv;
        let t = input.temperatures[i];
        let theta = input.liquid_water_contents[i];
        let theta_s = input.saturated_water_contents[input.material_indices[i]];
        let tk = t + 273.15;
        let theta_a = theta_s - theta;

        let diff_t = 2.12e-5 * (tk / 273.15).powi(2);
        let tau = if theta_a > 0.0 {
            theta_a.powf(7.0 / 3.0) / theta_s.powi(2)
        } else {
            0.0
        };
        let diff = tau * theta_a * diff_t;
        let rho = liquid_density(t);
        let rho_vs = vapor_saturation_density(tk);
        let hr = (h * 0.018015 * 9.81 / 8.314 / tk).exp();

        // Keep the source's one-kelvin finite difference and multiplication order.
        let rho_vs_1 = vapor_saturation_density(tk + 1.0);
        let drovs = rho_vs_1 - rho_vs;
        let eta = 9.5 + 3.0 * theta / theta_s
            - 8.5 * (-((1.0 + 2.6 / 0.02_f64.sqrt()) * theta / theta_s).powi(4)).exp();

        let con_lh = input.liquid_conductivities[i] / input.x_conv * input.t_conv;
        let d_gamma = -0.1425 - 0.000479 * t;
        let con_lt = if h < 0.0 {
            con_lh * h * 7.0 * d_gamma / 71.89
        } else {
            0.0
        } * input.x_conv
            * input.x_conv
            / input.t_conv;
        let con_vh =
            diff / rho * rho_vs * 0.018015 * 9.81 / 8.314 / tk * hr * input.x_conv / input.t_conv;
        let con_vt = diff / rho * eta * hr * drovs * input.x_conv * input.x_conv / input.t_conv;

        let rho_v = rho_vs * hr;
        let theta_v = rho_v * (theta_s - theta) / rho;
        let cap = (1.0 - rho_v / rho) * input.liquid_capacities[i]
            + theta_v * 0.018015 * 9.81 / 8.314 / tk * input.x_conv;

        for (name, value) in [
            ("ConLT", con_lt),
            ("ConVh", con_vh),
            ("ConVT", con_vt),
            ("ThetaV", theta_v),
            ("effective_capacity", cap),
        ] {
            if !value.is_finite() {
                return Err(VaporError::NonFinite {
                    name,
                    index: Some(i),
                    value,
                });
            }
        }
        liquid_thermal.push(con_lt);
        vapor_head.push(con_vh);
        vapor_thermal.push(con_vt);
        vapor_contents.push(theta_v);
        effective_capacity.push(cap);
    }

    Ok(VaporProperties {
        liquid_conductivities: input.liquid_conductivities.to_vec(),
        liquid_thermal_conductivities: liquid_thermal,
        vapor_head_conductivities: vapor_head,
        vapor_thermal_conductivities: vapor_thermal,
        vapor_contents,
        effective_hydraulic_capacities: effective_capacity,
    })
}

/// Source `xLatent`: temperature-dependent liquid density times latent heat.
pub fn x_latent(temperature_c: f64) -> Result<f64, VaporError> {
    if !temperature_c.is_finite() {
        return Err(VaporError::NonFinite {
            name: "temperature",
            index: None,
            value: temperature_c,
        });
    }
    if temperature_c + 273.15 <= 0.0 {
        return Err(VaporError::InvalidValue {
            name: "absolute_temperature",
            value: temperature_c,
        });
    }
    let value = liquid_density(temperature_c) * (2.501e6 - 2369.2 * temperature_c);
    if !value.is_finite() {
        return Err(VaporError::NonFinite {
            name: "xLatent",
            index: None,
            value,
        });
    }
    Ok(value)
}

fn liquid_density(temperature_c: f64) -> f64 {
    (1.0 - 7.37e-6 * (temperature_c - 4.0).powi(2) + 3.79e-8 * (temperature_c - 4.0).powi(3))
        * 1000.0
}

fn vapor_saturation_density(temperature_k: f64) -> f64 {
    0.001 * (31.3716 - 6014.79 / temperature_k - 0.00792495 * temperature_k).exp() / temperature_k
}

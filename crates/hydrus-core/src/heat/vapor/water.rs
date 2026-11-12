//! Opt-in vapor additions to water rows and phase fluxes.

use super::{VaporError, VaporProperties};

/// Inputs for source-style interior water rows with vapor terms enabled.
#[derive(Clone, Copy)]
pub struct VaporWaterRowsInput<'a> {
    pub coordinates: &'a [f64],
    pub gravity: f64,
    pub liquid_conductivities: &'a [f64],
    pub effective_capacities: &'a [f64],
    pub heads_new: &'a [f64],
    pub liquid_water_old: &'a [f64],
    pub liquid_water_new: &'a [f64],
    pub vapor_water_old: &'a [f64],
    pub vapor_water_new: &'a [f64],
    pub temperatures: &'a [f64],
    pub sink: &'a [f64],
    pub timestep: f64,
    pub vapor: &'a VaporProperties,
}

/// One interior `Reset` row. Boundary rows are owned by the water boundary path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaporWaterRow {
    pub lower: f64,
    pub diagonal: f64,
    pub upper: f64,
    pub rhs: f64,
}

/// Assemble full interior water rows while retaining liquid-only gravity.
///
/// Implements the `lVapor` additions in `WATFLOW.FOR:235-267`: vapor head
/// conductivity enters diffusion faces, vapor thermal conductivity contributes
/// the temperature-gradient divergence, vapor storage is a separate RHS term,
/// and the supplied capacity is the effective hydraulic capacity.
pub fn assemble_water_rows(
    input: &VaporWaterRowsInput<'_>,
) -> Result<Vec<VaporWaterRow>, VaporError> {
    let n = input.coordinates.len();
    if n < 3 {
        return Err(VaporError::TooFewNodes { count: n });
    }
    for (name, values) in [
        ("liquid_conductivities", input.liquid_conductivities),
        ("effective_capacities", input.effective_capacities),
        ("heads_new", input.heads_new),
        ("liquid_water_old", input.liquid_water_old),
        ("liquid_water_new", input.liquid_water_new),
        ("vapor_water_old", input.vapor_water_old),
        ("vapor_water_new", input.vapor_water_new),
        ("temperatures", input.temperatures),
        ("sink", input.sink),
        (
            "vapor thermal conductivities",
            &input.vapor.vapor_thermal_conductivities,
        ),
        (
            "vapor head conductivities",
            &input.vapor.vapor_head_conductivities,
        ),
    ] {
        check_len(name, n, values.len())?;
        check_finite(name, values)?;
    }
    check_finite("coordinates", input.coordinates)?;
    check_finite("gravity", &[input.gravity])?;
    check_finite("timestep", &[input.timestep])?;
    if input.timestep <= 0.0 {
        return Err(VaporError::InvalidValue {
            name: "timestep",
            value: input.timestep,
        });
    }
    if let Some(index) = input
        .coordinates
        .windows(2)
        .position(|pair| pair[1] <= pair[0])
    {
        return Err(VaporError::InvalidValue {
            name: "coordinates",
            value: input.coordinates[index + 1],
        });
    }
    if input.liquid_conductivities.iter().any(|&value| value < 0.0)
        || input.effective_capacities.iter().any(|&value| value < 0.0)
        || input
            .vapor
            .vapor_head_conductivities
            .iter()
            .any(|&value| value < 0.0)
        || input
            .vapor
            .vapor_thermal_conductivities
            .iter()
            .any(|&value| value < 0.0)
    {
        return Err(VaporError::InvalidValue {
            name: "conductivity/capacity",
            value: -1.0,
        });
    }

    let mut rows = Vec::with_capacity(n - 2);
    for i in 1..n - 1 {
        let dx_a = input.coordinates[i] - input.coordinates[i - 1];
        let dx_b = input.coordinates[i + 1] - input.coordinates[i];
        let width = (dx_a + dx_b) / 2.0;
        let liquid_a = (input.liquid_conductivities[i - 1] + input.liquid_conductivities[i]) / 2.0;
        let liquid_b = (input.liquid_conductivities[i] + input.liquid_conductivities[i + 1]) / 2.0;
        let con_a = liquid_a
            + (input.vapor.vapor_head_conductivities[i - 1]
                + input.vapor.vapor_head_conductivities[i])
                / 2.0;
        let con_b = liquid_b
            + (input.vapor.vapor_head_conductivities[i]
                + input.vapor.vapor_head_conductivities[i + 1])
                / 2.0;
        let capacity_term = input.effective_capacities[i] * width / input.timestep;
        let thermal_a = (input.vapor.vapor_thermal_conductivities[i - 1]
            + input.vapor.vapor_thermal_conductivities[i])
            / 2.0;
        let thermal_b = (input.vapor.vapor_thermal_conductivities[i]
            + input.vapor.vapor_thermal_conductivities[i + 1])
            / 2.0;
        let row = VaporWaterRow {
            lower: -con_a / dx_a,
            diagonal: con_a / dx_a + con_b / dx_b + capacity_term,
            upper: -con_b / dx_b,
            rhs: capacity_term * input.heads_new[i]
                - (input.liquid_water_new[i] - input.liquid_water_old[i]) * width / input.timestep
                - (liquid_a - liquid_b) * input.gravity
                - input.sink[i] * width
                + thermal_b * (input.temperatures[i + 1] - input.temperatures[i]) / dx_b
                - thermal_a * (input.temperatures[i] - input.temperatures[i - 1]) / dx_a
                - (input.vapor_water_new[i] - input.vapor_water_old[i]) * width / input.timestep,
        };
        if ![row.lower, row.diagonal, row.upper, row.rhs]
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(VaporError::NonFinite {
                name: "water row",
                index: Some(i),
                value: f64::NAN,
            });
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Nodal phase velocities from `Veloc`; gravity affects only liquid flow.
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseVelocities {
    pub liquid: Vec<f64>,
    pub vapor: Vec<f64>,
    pub total: Vec<f64>,
}

/// Inputs for source-matched liquid and vapor nodal velocities.
#[derive(Clone, Copy)]
pub struct PhaseVelocityInput<'a> {
    pub coordinates: &'a [f64],
    pub gravity: f64,
    pub heads: &'a [f64],
    pub liquid_conductivities: &'a [f64],
    pub liquid_water_old: &'a [f64],
    pub liquid_water_new: &'a [f64],
    pub sink: &'a [f64],
    pub temperatures: &'a [f64],
    pub vapor_water_old: &'a [f64],
    pub vapor_water_new: &'a [f64],
    pub timestep: f64,
    pub vapor: &'a VaporProperties,
}

/// Compute node-centered liquid, vapor, and summed total fluxes.
pub fn phase_velocities(input: &PhaseVelocityInput<'_>) -> Result<PhaseVelocities, VaporError> {
    let n = input.coordinates.len();
    if n < 3 {
        return Err(VaporError::TooFewNodes { count: n });
    }
    for (name, values) in [
        ("heads", input.heads),
        ("liquid_conductivities", input.liquid_conductivities),
        ("liquid_water_old", input.liquid_water_old),
        ("liquid_water_new", input.liquid_water_new),
        ("sink", input.sink),
        ("temperatures", input.temperatures),
        ("vapor_water_old", input.vapor_water_old),
        ("vapor_water_new", input.vapor_water_new),
        (
            "vapor head conductivities",
            &input.vapor.vapor_head_conductivities,
        ),
        (
            "vapor thermal conductivities",
            &input.vapor.vapor_thermal_conductivities,
        ),
    ] {
        check_len(name, n, values.len())?;
        check_finite(name, values)?;
    }
    check_finite("coordinates", input.coordinates)?;
    check_finite("gravity/timestep", &[input.gravity, input.timestep])?;
    if input.timestep <= 0.0 {
        return Err(VaporError::InvalidValue {
            name: "timestep",
            value: input.timestep,
        });
    }
    if let Some(index) = input
        .coordinates
        .windows(2)
        .position(|pair| pair[1] <= pair[0])
    {
        return Err(VaporError::InvalidValue {
            name: "coordinates",
            value: input.coordinates[index + 1],
        });
    }
    if input.liquid_conductivities.iter().any(|&value| value < 0.0)
        || input
            .vapor
            .vapor_head_conductivities
            .iter()
            .any(|&value| value < 0.0)
        || input
            .vapor
            .vapor_thermal_conductivities
            .iter()
            .any(|&value| value < 0.0)
    {
        return Err(VaporError::InvalidValue {
            name: "conductivity",
            value: -1.0,
        });
    }

    let mut liquid = vec![0.0; n];
    let mut vapor = vec![0.0; n];
    let last = n - 1;
    for i in 0..n {
        let (left, right) = if i == 0 {
            (0, 1)
        } else if i == last {
            (last - 1, last)
        } else {
            (i - 1, i + 1)
        };
        if i == 0 || i == last {
            let dx = input.coordinates[right] - input.coordinates[left];
            let con =
                (input.liquid_conductivities[left] + input.liquid_conductivities[right]) / 2.0;
            let transient = dx / 2.0
                * ((input.liquid_water_new[i] - input.liquid_water_old[i]) / input.timestep
                    + input.sink[i]);
            let liquid_value = if i == 0 {
                -con * ((input.heads[1] - input.heads[0]) / dx + input.gravity) + transient
            } else {
                -con * ((input.heads[last] - input.heads[last - 1]) / dx + input.gravity)
                    - transient
            };
            liquid[i] = liquid_value;
            let vh = (input.vapor.vapor_head_conductivities[left]
                + input.vapor.vapor_head_conductivities[right])
                / 2.0;
            let vt = (input.vapor.vapor_thermal_conductivities[left]
                + input.vapor.vapor_thermal_conductivities[right])
                / 2.0;
            let thermal = vt * (input.temperatures[right] - input.temperatures[left]) / dx;
            let storage =
                dx / 2.0 * (input.vapor_water_new[i] - input.vapor_water_old[i]) / input.timestep;
            vapor[i] = if i == 0 {
                -vh * (input.heads[1] - input.heads[0]) / dx - thermal + storage
            } else {
                -vh * (input.heads[last] - input.heads[last - 1]) / dx - thermal - storage
            };
        } else {
            let dx_a = input.coordinates[i + 1] - input.coordinates[i];
            let dx_b = input.coordinates[i] - input.coordinates[i - 1];
            let va = -face(
                input.liquid_conductivities[i],
                input.liquid_conductivities[i + 1],
            ) * ((input.heads[i + 1] - input.heads[i]) / dx_a + input.gravity);
            let vb = -face(
                input.liquid_conductivities[i],
                input.liquid_conductivities[i - 1],
            ) * ((input.heads[i] - input.heads[i - 1]) / dx_b + input.gravity);
            liquid[i] = (va * dx_b + vb * dx_a) / (dx_a + dx_b);
            let vapor_a = -face(
                input.vapor.vapor_head_conductivities[i],
                input.vapor.vapor_head_conductivities[i + 1],
            ) * (input.heads[i + 1] - input.heads[i])
                / dx_a
                - face(
                    input.vapor.vapor_thermal_conductivities[i],
                    input.vapor.vapor_thermal_conductivities[i + 1],
                ) * (input.temperatures[i + 1] - input.temperatures[i])
                    / dx_a;
            let vapor_b = -face(
                input.vapor.vapor_head_conductivities[i],
                input.vapor.vapor_head_conductivities[i - 1],
            ) * (input.heads[i] - input.heads[i - 1])
                / dx_b
                - face(
                    input.vapor.vapor_thermal_conductivities[i],
                    input.vapor.vapor_thermal_conductivities[i - 1],
                ) * (input.temperatures[i] - input.temperatures[i - 1])
                    / dx_b;
            vapor[i] = (vapor_a * dx_b + vapor_b * dx_a) / (dx_a + dx_b);
        }
    }
    let total: Vec<f64> = liquid.iter().zip(&vapor).map(|(l, v)| l + v).collect();
    for (name, values) in [
        ("liquid velocity", &liquid),
        ("vapor velocity", &vapor),
        ("total velocity", &total),
    ] {
        if values.iter().any(|value| !value.is_finite()) {
            return Err(VaporError::NonFinite {
                name,
                index: None,
                value: f64::NAN,
            });
        }
    }
    Ok(PhaseVelocities {
        liquid,
        vapor,
        total,
    })
}

fn face(a: f64, b: f64) -> f64 {
    (a + b) / 2.0
}

fn check_len(name: &'static str, expected: usize, actual: usize) -> Result<(), VaporError> {
    if actual != expected {
        return Err(VaporError::LengthMismatch {
            name,
            expected,
            actual,
        });
    }
    Ok(())
}

fn check_finite(name: &'static str, values: &[f64]) -> Result<(), VaporError> {
    for (index, &value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(VaporError::NonFinite {
                name,
                index: Some(index),
                value,
            });
        }
    }
    Ok(())
}

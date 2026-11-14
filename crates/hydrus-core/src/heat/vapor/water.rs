//! Opt-in vapor additions to water rows and phase fluxes.

use super::{VaporError, VaporProperties};
use crate::hydraulics::FaceAveraging;

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

fn validate_boundary_input(input: &VaporWaterRowsInput<'_>) -> Result<usize, VaporError> {
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
    if input.liquid_conductivities.iter().any(|&v| v < 0.0)
        || input.effective_capacities.iter().any(|&v| v < 0.0)
        || input
            .vapor
            .vapor_head_conductivities
            .iter()
            .any(|&v| v < 0.0)
        || input
            .vapor
            .vapor_thermal_conductivities
            .iter()
            .any(|&v| v < 0.0)
    {
        return Err(VaporError::InvalidValue {
            name: "conductivity/capacity",
            value: -1.0,
        });
    }
    Ok(n)
}

fn liquid_face(a: f64, b: f64, averaging: FaceAveraging) -> f64 {
    match averaging {
        FaceAveraging::Arithmetic => (a + b) / 2.0,
        FaceAveraging::Geometric => (a * b).sqrt(),
    }
}

fn finite_row(row: VaporWaterRow, index: usize) -> Result<VaporWaterRow, VaporError> {
    if [row.lower, row.diagonal, row.upper, row.rhs]
        .iter()
        .all(|value| value.is_finite())
    {
        Ok(row)
    } else {
        Err(VaporError::NonFinite {
            name: "water row",
            index: Some(index),
            value: f64::NAN,
        })
    }
}

/// Bottom-node (index 0) flux row with vapor terms, `WATFLOW.FOR:202-228`.
///
/// The liquid face follows `averaging` (`:203`); vapor head conductivity is
/// then added arithmetically (`:211`). Gravity uses the liquid face only
/// (`:205`). `rBot` is not in the row: pass it as Neumann flux (positive into
/// the domain) to `transform_boundary_row`, which adds it once to the rhs.
pub fn vapor_lower_flux_row(
    input: &VaporWaterRowsInput<'_>,
    averaging: FaceAveraging,
) -> Result<VaporWaterRow, VaporError> {
    validate_boundary_input(input)?;
    let dx_b = input.coordinates[1] - input.coordinates[0];
    let half = dx_b / 2.0;
    let liquid = liquid_face(
        input.liquid_conductivities[0],
        input.liquid_conductivities[1],
        averaging,
    );
    let con_b = liquid
        + face(
            input.vapor.vapor_head_conductivities[0],
            input.vapor.vapor_head_conductivities[1],
        );
    let storage = input.effective_capacities[0] * half / input.timestep;
    let con_tb = face(
        input.vapor.vapor_thermal_conductivities[0],
        input.vapor.vapor_thermal_conductivities[1],
    );
    let row = VaporWaterRow {
        lower: 0.0,
        diagonal: con_b / dx_b + storage,
        upper: -con_b / dx_b,
        rhs: liquid * input.gravity - input.sink[0] * half + storage * input.heads_new[0]
            - (input.liquid_water_new[0] - input.liquid_water_old[0]) * half / input.timestep
            + con_tb * (input.temperatures[1] - input.temperatures[0]) / dx_b
            - (input.vapor_water_new[0] - input.vapor_water_old[0]) * half / input.timestep,
    };
    finite_row(row, 0)
}

/// Top-node (index n-1) flux row with vapor terms, `WATFLOW.FOR:272-291`.
///
/// `rTop` is positive out of the domain and enters as `PT=PT-rTop` (`:294`),
/// so pass Neumann flux `-rTop` (positive into the domain). It is not in the row.
pub fn vapor_upper_flux_row(
    input: &VaporWaterRowsInput<'_>,
    averaging: FaceAveraging,
) -> Result<VaporWaterRow, VaporError> {
    let n = validate_boundary_input(input)?;
    let top = n - 1;
    let dx_a = input.coordinates[top] - input.coordinates[top - 1];
    let half = dx_a / 2.0;
    let liquid = liquid_face(
        input.liquid_conductivities[top],
        input.liquid_conductivities[top - 1],
        averaging,
    );
    let con_a = liquid
        + face(
            input.vapor.vapor_head_conductivities[top],
            input.vapor.vapor_head_conductivities[top - 1],
        );
    let storage = input.effective_capacities[top] * half / input.timestep;
    let con_ta = face(
        input.vapor.vapor_thermal_conductivities[top],
        input.vapor.vapor_thermal_conductivities[top - 1],
    );
    let row = VaporWaterRow {
        lower: -con_a / dx_a,
        diagonal: con_a / dx_a + storage,
        upper: 0.0,
        rhs: storage * input.heads_new[top]
            - (input.liquid_water_new[top] - input.liquid_water_old[top]) * half / input.timestep
            - input.sink[top] * half
            - liquid * input.gravity
            - con_ta * (input.temperatures[top] - input.temperatures[top - 1]) / dx_a
            - (input.vapor_water_new[top] - input.vapor_water_old[top]) * half / input.timestep,
    };
    finite_row(row, top)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydraulics::{transform_boundary_row, BoundaryCondition, BoundaryRowLocation};

    struct Fx {
        vapor: VaporProperties,
    }

    fn fx(head: [f64; 3], thermal: [f64; 3]) -> Fx {
        Fx {
            vapor: VaporProperties {
                liquid_conductivities: vec![0.0; 3],
                liquid_thermal_conductivities: vec![0.0; 3],
                vapor_head_conductivities: head.to_vec(),
                vapor_thermal_conductivities: thermal.to_vec(),
                vapor_contents: vec![0.0; 3],
                effective_hydraulic_capacities: vec![0.0; 3],
            },
        }
    }

    // Nodes x = 0, 2, 6: dx_b = 2 (half 1), dx_a = 4 (half 2); dt = 2.
    fn input<'a>(
        f: &'a Fx,
        gravity: f64,
        theta_new: &'a [f64; 3],
        vapor_new: &'a [f64; 3],
        temps: &'a [f64; 3],
    ) -> VaporWaterRowsInput<'a> {
        VaporWaterRowsInput {
            coordinates: &[0.0, 2.0, 6.0],
            gravity,
            liquid_conductivities: &[4.0, 9.0, 16.0],
            effective_capacities: &[0.5, 0.5, 0.5],
            heads_new: &[-2.0, -1.0, -4.0],
            liquid_water_old: &[0.2; 3],
            liquid_water_new: theta_new,
            vapor_water_old: &[0.0; 3],
            vapor_water_new: vapor_new,
            temperatures: temps,
            sink: &[0.0; 3],
            timestep: 2.0,
            vapor: &f.vapor,
        }
    }

    const Z: [f64; 3] = [0.0; 3];
    const T0: [f64; 3] = [0.2; 3];

    #[test]
    fn lower_convh_enters_face_not_gravity() {
        let f = fx([1.0, 3.0, 0.0], [0.0; 3]);
        let r =
            vapor_lower_flux_row(&input(&f, 1.0, &T0, &Z, &Z), FaceAveraging::Arithmetic).unwrap();
        // liquid 6.5, ConB 8.5, dx 2, storage .5*1/2=.25
        assert_eq!(r.lower, 0.0);
        assert_eq!(r.upper, -4.25);
        assert_eq!(r.diagonal, 4.5);
        // rhs = 6.5*1 (liquid gravity only) + .25*-2
        assert_eq!(r.rhs, 6.0);
    }

    #[test]
    fn lower_thermal_term() {
        let f = fx([0.0; 3], [2.0, 4.0, 0.0]);
        let t = [10.0, 14.0, 0.0];
        let r =
            vapor_lower_flux_row(&input(&f, 0.0, &T0, &Z, &t), FaceAveraging::Arithmetic).unwrap();
        // ConTB 3 * (14-10)/2 = 6, plus storage -0.5
        assert_eq!(r.rhs, 5.5);
    }

    #[test]
    fn lower_vapor_storage_term() {
        let f = fx([0.0; 3], [0.0; 3]);
        let v = [0.04, 0.0, 0.0];
        let r =
            vapor_lower_flux_row(&input(&f, 0.0, &T0, &v, &Z), FaceAveraging::Arithmetic).unwrap();
        // -0.04*1/2 = -0.02 plus storage -0.5
        assert_eq!(r.rhs, -0.52);
    }

    #[test]
    fn lower_flux_added_once_through_neumann() {
        let f = fx([0.0; 3], [0.0; 3]);
        let r =
            vapor_lower_flux_row(&input(&f, 0.0, &T0, &Z, &Z), FaceAveraging::Arithmetic).unwrap();
        let t = transform_boundary_row(
            3,
            BoundaryRowLocation::Lower,
            conv(r),
            BoundaryCondition::Neumann { flux: 0.75 },
            1e-30,
        )
        .unwrap();
        assert_eq!(t.rhs, r.rhs + 0.75); // rBot positive into the domain
        assert_eq!(t.diagonal, r.diagonal);
    }

    #[test]
    fn upper_convh_enters_face_not_gravity() {
        let f = fx([0.0, 3.0, 1.0], [0.0; 3]);
        let r =
            vapor_upper_flux_row(&input(&f, 1.0, &T0, &Z, &Z), FaceAveraging::Arithmetic).unwrap();
        // liquid 12.5, ConA 14.5, dx_a 4, storage .5*2/2=.5
        assert_eq!(r.lower, -3.625);
        assert_eq!(r.diagonal, 4.125);
        assert_eq!(r.upper, 0.0);
        // rhs = .5*-4 - 12.5*1
        assert_eq!(r.rhs, -14.5);
    }

    #[test]
    fn upper_thermal_term() {
        let f = fx([0.0; 3], [0.0, 2.0, 6.0]);
        let t = [0.0, 10.0, 18.0];
        let r =
            vapor_upper_flux_row(&input(&f, 0.0, &T0, &Z, &t), FaceAveraging::Arithmetic).unwrap();
        // ConTA 4 * (18-10)/4 = 8 subtracted, plus storage -2
        assert_eq!(r.rhs, -10.0);
    }

    #[test]
    fn upper_vapor_storage_term() {
        let f = fx([0.0; 3], [0.0; 3]);
        let v = [0.0, 0.0, 0.08];
        let r =
            vapor_upper_flux_row(&input(&f, 0.0, &T0, &v, &Z), FaceAveraging::Arithmetic).unwrap();
        // -0.08*2/2 = -0.08, plus storage -2
        assert_eq!(r.rhs, -2.08);
    }

    #[test]
    fn upper_flux_is_minus_rtop_added_once() {
        let f = fx([0.0; 3], [0.0; 3]);
        let r =
            vapor_upper_flux_row(&input(&f, 0.0, &T0, &Z, &Z), FaceAveraging::Arithmetic).unwrap();
        let r_top = 0.3; // Fortran rTop, positive out
        let t = transform_boundary_row(
            3,
            BoundaryRowLocation::Upper,
            conv(r),
            BoundaryCondition::Neumann { flux: -r_top },
            1e-30,
        )
        .unwrap();
        assert_eq!(t.rhs, r.rhs - r_top); // WATFLOW.FOR:294 PT=PT-rTop
    }

    #[test]
    fn geometric_averaging_applies_to_liquid_face_only() {
        let f = fx([1.0, 3.0, 1.0], [0.0; 3]);
        let inp = input(&f, 1.0, &T0, &Z, &Z);
        let lo = vapor_lower_flux_row(&inp, FaceAveraging::Geometric).unwrap();
        // liquid sqrt(4*9)=6, ConB 6+2=8, gravity uses 6
        assert_eq!(lo.upper, -4.0);
        assert_eq!(lo.rhs, 6.0 - 0.5);
        let up = vapor_upper_flux_row(&inp, FaceAveraging::Geometric).unwrap();
        // liquid sqrt(16*9)=12, ConA 12+2=14, gravity uses 12
        assert_eq!(up.lower, -3.5);
        assert_eq!(up.rhs, -2.0 - 12.0);
        let ar = vapor_lower_flux_row(&inp, FaceAveraging::Arithmetic).unwrap();
        assert_ne!(ar.upper, lo.upper);
    }

    fn conv(r: VaporWaterRow) -> crate::hydraulics::BoundaryRow {
        crate::hydraulics::BoundaryRow {
            lower: r.lower,
            diagonal: r.diagonal,
            upper: r.upper,
            rhs: r.rhs,
        }
    }
}

use crate::HydraulicError;

use super::{arithmetic_face_conductivity, geometric_face_conductivity, gravity_metric, Mesh1D};

/// Face conductivity policy used by the observed `Reset` stencil.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FaceAveraging {
    #[default]
    Arithmetic,
    Geometric,
}

/// Inputs for assembling the source-style interior `Reset` rows.
#[derive(Clone, Copy, Debug)]
pub struct ResetStencilInput<'a> {
    pub conductivities: &'a [f64],
    pub capacities: &'a [f64],
    pub h_new: &'a [f64],
    pub theta_new: &'a [f64],
    pub theta_old: &'a [f64],
    pub sink: &'a [f64],
    pub timestep: f64,
    pub face_averaging: FaceAveraging,
}

/// One interior lower/diagonal/upper/RHS row from the observed `Reset` algebra.
///
/// Rows are returned for mesh nodes `1..mesh.len()-1`; boundary rows and their
/// boundary-condition transformations are intentionally not represented here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResetRow {
    pub lower: f64,
    pub diagonal: f64,
    pub upper: f64,
    pub rhs: f64,
}

/// Assemble the pure, source-style interior finite-difference `Reset` stencil.
///
/// This preserves the observed `WATFLOW.FOR:232-256` algebra. It is stencil
/// evidence, not a Fortran parity result; the manual's finite-element
/// terminology remains an open validation item.
pub fn assemble_reset_stencil(
    mesh: &Mesh1D,
    input: ResetStencilInput<'_>,
) -> Result<Vec<ResetRow>, HydraulicError> {
    let node_count = mesh.coordinates().len();
    if node_count < 3 {
        return Err(stencil_error("mesh", "at least three nodes are required"));
    }
    if !input.timestep.is_finite() || input.timestep <= 0.0 {
        return Err(stencil_error("timestep", "must be finite and positive"));
    }
    for (name, values) in [
        ("conductivities", input.conductivities),
        ("capacities", input.capacities),
        ("h_new", input.h_new),
        ("theta_new", input.theta_new),
        ("theta_old", input.theta_old),
        ("sink", input.sink),
    ] {
        if values.len() != node_count {
            return Err(stencil_error(name, "length must match mesh nodes"));
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err(stencil_error(name, "values must be finite"));
        }
    }
    let gravity = gravity_metric(mesh)?;
    let coordinates = mesh.coordinates();
    let mut rows = Vec::with_capacity(node_count - 2);

    for node in 1..node_count - 1 {
        let dx_a = coordinates[node] - coordinates[node - 1];
        let dx_b = coordinates[node + 1] - coordinates[node];
        let dx = (dx_a + dx_b) / 2.0;
        let con_a = face_conductivity(
            input.conductivities[node - 1],
            input.conductivities[node],
            input.face_averaging,
        )?;
        let con_b = face_conductivity(
            input.conductivities[node],
            input.conductivities[node + 1],
            input.face_averaging,
        )?;
        let b = (con_a - con_b) * gravity;
        let a2 = con_a / dx_a + con_b / dx_b;
        let a3 = -con_b / dx_b;
        let f2 = input.capacities[node] * dx / input.timestep;
        let row = ResetRow {
            lower: -con_a / dx_a,
            diagonal: a2 + f2,
            upper: a3,
            rhs: f2 * input.h_new[node]
                - (input.theta_new[node] - input.theta_old[node]) * dx / input.timestep
                - b
                - input.sink[node] * dx,
        };
        if ![row.lower, row.diagonal, row.upper, row.rhs]
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(stencil_error("row", "derived coefficients must be finite"));
        }
        rows.push(row);
    }
    Ok(rows)
}

fn face_conductivity(
    left: f64,
    right: f64,
    averaging: FaceAveraging,
) -> Result<f64, HydraulicError> {
    match averaging {
        FaceAveraging::Arithmetic => arithmetic_face_conductivity(left, right),
        FaceAveraging::Geometric => geometric_face_conductivity(left, right),
    }
}

fn stencil_error(name: &'static str, reason: &'static str) -> HydraulicError {
    HydraulicError::InvalidMesh { name, reason }
}

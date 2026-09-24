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

#[derive(Clone, Copy)]
pub(crate) struct ResetRowInput {
    pub dx_a: f64,
    pub dx_b: f64,
    pub gravity: f64,
    pub conductivity_a: f64,
    pub conductivity_node: f64,
    pub conductivity_b: f64,
    pub capacity: f64,
    pub h_new: f64,
    pub theta_new: f64,
    pub theta_old: f64,
    pub sink: f64,
    pub timestep: f64,
    pub face_averaging: FaceAveraging,
}

/// Calculates one interior row without allocating, shared by stencil and
/// water-solver assembly.
impl ResetRow {
    pub(crate) fn calculate(input: ResetRowInput) -> Result<Self, HydraulicError> {
        let con_a = face_conductivity(
            input.conductivity_a,
            input.conductivity_node,
            input.face_averaging,
        )?;
        let con_b = face_conductivity(
            input.conductivity_node,
            input.conductivity_b,
            input.face_averaging,
        )?;
        let width = (input.dx_a + input.dx_b) / 2.0;
        let storage = input.capacity * width / input.timestep;
        Ok(Self {
            lower: -con_a / input.dx_a,
            diagonal: con_a / input.dx_a + con_b / input.dx_b + storage,
            upper: -con_b / input.dx_b,
            rhs: storage * input.h_new
                - (input.theta_new - input.theta_old) * width / input.timestep
                - (con_a - con_b) * input.gravity
                - input.sink * width,
        })
    }
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
        let row = ResetRow::calculate(ResetRowInput {
            dx_a,
            dx_b,
            gravity,
            conductivity_a: input.conductivities[node - 1],
            conductivity_node: input.conductivities[node],
            conductivity_b: input.conductivities[node + 1],
            capacity: input.capacities[node],
            h_new: input.h_new[node],
            theta_new: input.theta_new[node],
            theta_old: input.theta_old[node],
            sink: input.sink[node],
            timestep: input.timestep,
            face_averaging: input.face_averaging,
        })?;
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

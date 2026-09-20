use crate::HydraulicError;

use super::Mesh1D;

/// A mesh boundary whose half-cell width is requested.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Boundary {
    Lower,
    Upper,
}

/// Coordinate span of one contiguous layer group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerMetric {
    pub layer_index: usize,
    pub first_node: usize,
    pub last_node: usize,
    pub span: f64,
}

/// Returns the signed coordinate distance from node `node` to `node + 1`.
pub fn adjacent_distance(mesh: &Mesh1D, node: usize) -> Result<f64, HydraulicError> {
    let coordinates = mesh.coordinates();
    let next = node
        .checked_add(1)
        .ok_or_else(|| geometry_error("node", "index is out of range"))?;
    let (left, right) = coordinates
        .get(node)
        .zip(coordinates.get(next))
        .ok_or_else(|| geometry_error("node", "index is out of range"))?;
    finite_geometry(*right - *left, "adjacent_distance")
}

/// Returns the source-style half-cell width at a boundary.
pub fn boundary_half_cell_width(mesh: &Mesh1D, boundary: Boundary) -> Result<f64, HydraulicError> {
    let distance = match boundary {
        Boundary::Lower => adjacent_distance(mesh, 0)?,
        Boundary::Upper => {
            let count = mesh.coordinates().len();
            if count < 2 {
                return Err(geometry_error(
                    "boundary_half_cell",
                    "at least two nodes are required",
                ));
            }
            adjacent_distance(mesh, count - 2)?
        }
    };
    finite_geometry(distance / 2.0, "boundary_half_cell")
}

/// Returns the gravity metric using the source convention `Grav = CosAlf`.
pub fn gravity_metric(mesh: &Mesh1D) -> Result<f64, HydraulicError> {
    finite_geometry(mesh.inclination_cosine(), "inclination_cosine")
}

/// Returns one span record for every declared layer.
///
/// Phase 2 convention: nodes belonging to a layer must form one contiguous
/// group, and the span is the last coordinate minus the first coordinate in
/// that group. This is not a Fortran-parity claim.
pub fn layer_metrics(mesh: &Mesh1D) -> Result<Vec<LayerMetric>, HydraulicError> {
    let layers = mesh
        .layer_indices()
        .ok_or_else(|| geometry_error("layer_indices", "layers are not present"))?;
    let mut metrics = Vec::with_capacity(mesh.layer_count());

    for layer_index in 0..mesh.layer_count() {
        let nodes: Vec<_> = layers
            .iter()
            .enumerate()
            .filter_map(|(node, &layer)| (layer == layer_index).then_some(node))
            .collect();
        let first_node = *nodes
            .first()
            .ok_or_else(|| geometry_error("layer_indices", "declared layer has no nodes"))?;
        let last_node = *nodes
            .last()
            .ok_or_else(|| geometry_error("layer_indices", "declared layer has no nodes"))?;
        if nodes.windows(2).any(|pair| pair[1] != pair[0] + 1) {
            return Err(geometry_error(
                "layer_indices",
                "each layer must form one contiguous group",
            ));
        }
        let span = mesh.coordinates()[last_node] - mesh.coordinates()[first_node];
        metrics.push(LayerMetric {
            layer_index,
            first_node,
            last_node,
            span: finite_geometry(span, "layer_span")?,
        });
    }
    Ok(metrics)
}

/// Computes the source-default arithmetic face conductivity.
pub fn arithmetic_face_conductivity(left: f64, right: f64) -> Result<f64, HydraulicError> {
    finite_conductivity(left, right)?;
    finite_geometry((left + right) / 2.0, "arithmetic_face_conductivity")
}

/// Computes the explicitly selected geometric face conductivity.
pub fn geometric_face_conductivity(left: f64, right: f64) -> Result<f64, HydraulicError> {
    finite_conductivity(left, right)?;
    if left < 0.0 || right < 0.0 {
        return Err(geometry_error(
            "geometric_face_conductivity",
            "conductivities must be non-negative",
        ));
    }
    finite_geometry((left * right).sqrt(), "geometric_face_conductivity")
}

fn finite_conductivity(left: f64, right: f64) -> Result<(), HydraulicError> {
    if !left.is_finite() || !right.is_finite() {
        return Err(geometry_error(
            "conductivity",
            "conductivity inputs must be finite",
        ));
    }
    Ok(())
}

fn finite_geometry(value: f64, name: &'static str) -> Result<f64, HydraulicError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(geometry_error(name, "derived value must be finite"))
    }
}

fn geometry_error(name: &'static str, reason: &'static str) -> HydraulicError {
    HydraulicError::InvalidMesh { name, reason }
}

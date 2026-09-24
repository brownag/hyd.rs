use crate::HydraulicError;

/// Endpoint selected for a boundary row transformation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryRowLocation {
    Lower,
    Upper,
    Node(usize),
}

/// One tridiagonal row using the convention
/// `lower * h[i - 1] + diagonal * h[i] + upper * h[i + 1] = rhs`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundaryRow {
    pub lower: f64,
    pub diagonal: f64,
    pub upper: f64,
    pub rhs: f64,
}

/// Generic boundary transformations supported by the water row contract.
///
/// A positive prescribed flux is added to the row right-hand side. The caller
/// owns its physical orientation and must supply the corresponding signed flux.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoundaryCondition {
    Dirichlet { value: f64 },
    Neumann { flux: f64 },
    Natural,
    Unsupported { selector: i32 },
}

/// Transform one endpoint row without selecting a physical boundary policy.
///
/// `minimum_pivot` is checked for Neumann and natural rows because their
/// diagonal remains relevant to the later tridiagonal solve. Dirichlet rows
/// become the identity equation and therefore do not consume the source pivot.
pub fn transform_boundary_row(
    node_count: usize,
    location: BoundaryRowLocation,
    row: BoundaryRow,
    condition: BoundaryCondition,
    minimum_pivot: f64,
) -> Result<BoundaryRow, HydraulicError> {
    let node = boundary_node(node_count, location)?;
    validate_row(node_count, node, row)?;
    validate_minimum_pivot(minimum_pivot)?;

    match condition {
        BoundaryCondition::Dirichlet { value } => {
            validate_finite(value, "dirichlet_value")?;
            Ok(BoundaryRow {
                lower: 0.0,
                diagonal: 1.0,
                upper: 0.0,
                rhs: value,
            })
        }
        BoundaryCondition::Neumann { flux } => {
            validate_finite(flux, "neumann_flux")?;
            validate_pivot(row.diagonal, minimum_pivot)?;
            let transformed = BoundaryRow {
                rhs: row.rhs + flux,
                ..row
            };
            validate_finite(transformed.rhs, "rhs")?;
            Ok(transformed)
        }
        BoundaryCondition::Natural => {
            validate_pivot(row.diagonal, minimum_pivot)?;
            Ok(row)
        }
        BoundaryCondition::Unsupported { selector } => {
            Err(HydraulicError::UnsupportedBoundary { selector })
        }
    }
}

fn boundary_node(
    node_count: usize,
    location: BoundaryRowLocation,
) -> Result<usize, HydraulicError> {
    if node_count == 0 {
        return Err(boundary_error("node_count", "must be positive"));
    }
    let node = match location {
        BoundaryRowLocation::Lower => 0,
        BoundaryRowLocation::Upper => node_count - 1,
        BoundaryRowLocation::Node(node) => node,
    };
    if node >= node_count {
        return Err(boundary_error("row_location", "must be within node_count"));
    }
    if node != 0 && node != node_count - 1 {
        return Err(boundary_error("row_location", "must select an endpoint"));
    }
    Ok(node)
}

fn validate_row(node_count: usize, node: usize, row: BoundaryRow) -> Result<(), HydraulicError> {
    for (name, value) in [
        ("lower", row.lower),
        ("diagonal", row.diagonal),
        ("upper", row.upper),
        ("rhs", row.rhs),
    ] {
        validate_finite(value, name)?;
    }
    if node == 0 && row.lower != 0.0 {
        return Err(boundary_error(
            "lower",
            "must be zero at the lower endpoint",
        ));
    }
    if node == node_count - 1 && row.upper != 0.0 {
        return Err(boundary_error(
            "upper",
            "must be zero at the upper endpoint",
        ));
    }
    Ok(())
}

fn validate_minimum_pivot(minimum_pivot: f64) -> Result<(), HydraulicError> {
    if !minimum_pivot.is_finite() || minimum_pivot <= 0.0 {
        return Err(boundary_error(
            "minimum_pivot",
            "must be finite and positive",
        ));
    }
    Ok(())
}

fn validate_pivot(value: f64, minimum: f64) -> Result<(), HydraulicError> {
    if value.abs() < minimum {
        return Err(HydraulicError::SingularBoundaryPivot { value, minimum });
    }
    Ok(())
}

fn validate_finite(value: f64, name: &'static str) -> Result<(), HydraulicError> {
    if !value.is_finite() {
        return Err(boundary_error(name, "must be finite"));
    }
    Ok(())
}

fn boundary_error(name: &'static str, reason: &'static str) -> HydraulicError {
    HydraulicError::InvalidBoundary { name, reason }
}

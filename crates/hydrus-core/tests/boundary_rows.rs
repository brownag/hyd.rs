use hydrus_core::hydraulics::{
    transform_boundary_row, BoundaryCondition, BoundaryRow, BoundaryRowLocation, ResetRow,
};
use hydrus_core::HydraulicError;

fn row(lower: f64, diagonal: f64, upper: f64, rhs: f64) -> BoundaryRow {
    BoundaryRow {
        lower,
        diagonal,
        upper,
        rhs,
    }
}

fn from_reset_row(row: ResetRow) -> BoundaryRow {
    BoundaryRow {
        lower: row.lower,
        diagonal: row.diagonal,
        upper: row.upper,
        rhs: row.rhs,
    }
}

#[test]
fn dirichlet_rows_replace_both_endpoint_equations() {
    let lower = transform_boundary_row(
        3,
        BoundaryRowLocation::Lower,
        row(0.0, 4.0, -2.0, 10.0),
        BoundaryCondition::Dirichlet { value: -1.5 },
        1e-12,
    )
    .unwrap();
    let upper = transform_boundary_row(
        3,
        BoundaryRowLocation::Upper,
        row(-3.0, 5.0, 0.0, 7.0),
        BoundaryCondition::Dirichlet { value: 2.5 },
        1e-12,
    )
    .unwrap();

    assert_eq!(lower, row(0.0, 1.0, 0.0, -1.5));
    assert_eq!(upper, row(0.0, 1.0, 0.0, 2.5));
}

#[test]
fn signed_flux_is_added_to_the_rhs_at_both_endpoints() {
    let lower = transform_boundary_row(
        3,
        BoundaryRowLocation::Lower,
        row(0.0, 4.0, -2.0, 10.0),
        BoundaryCondition::Neumann { flux: 3.0 },
        1e-12,
    )
    .unwrap();
    let upper = transform_boundary_row(
        3,
        BoundaryRowLocation::Upper,
        row(-3.0, 5.0, 0.0, 7.0),
        BoundaryCondition::Neumann { flux: -2.0 },
        1e-12,
    )
    .unwrap();

    assert_eq!(lower, row(0.0, 4.0, -2.0, 13.0));
    assert_eq!(upper, row(-3.0, 5.0, 0.0, 5.0));
}

#[test]
fn natural_rows_preserve_the_caller_prepared_equation() {
    let source = from_reset_row(ResetRow {
        lower: -3.0,
        diagonal: 5.0,
        upper: 0.0,
        rhs: 7.0,
    });
    let transformed = transform_boundary_row(
        3,
        BoundaryRowLocation::Node(2),
        source,
        BoundaryCondition::Natural,
        1e-12,
    )
    .unwrap();

    assert_eq!(transformed, source);
}

#[test]
fn two_node_systems_accept_each_endpoint_shape() {
    let lower = transform_boundary_row(
        2,
        BoundaryRowLocation::Lower,
        row(0.0, 2.0, -1.0, 4.0),
        BoundaryCondition::Natural,
        1e-12,
    )
    .unwrap();
    let upper = transform_boundary_row(
        2,
        BoundaryRowLocation::Upper,
        row(-1.0, 2.0, 0.0, 4.0),
        BoundaryCondition::Natural,
        1e-12,
    )
    .unwrap();

    assert_eq!(lower, row(0.0, 2.0, -1.0, 4.0));
    assert_eq!(upper, row(-1.0, 2.0, 0.0, 4.0));
}

#[test]
fn endpoint_shape_dimension_and_finite_failures_are_typed() {
    for (node_count, location, source) in [
        (0, BoundaryRowLocation::Lower, row(0.0, 1.0, 0.0, 0.0)),
        (3, BoundaryRowLocation::Node(1), row(0.0, 1.0, 0.0, 0.0)),
        (3, BoundaryRowLocation::Node(3), row(0.0, 1.0, 0.0, 0.0)),
        (3, BoundaryRowLocation::Lower, row(1.0, 1.0, 0.0, 0.0)),
        (3, BoundaryRowLocation::Upper, row(0.0, 1.0, 1.0, 0.0)),
        (3, BoundaryRowLocation::Lower, row(0.0, f64::NAN, 0.0, 0.0)),
    ] {
        assert!(matches!(
            transform_boundary_row(
                node_count,
                location,
                source,
                BoundaryCondition::Natural,
                1e-12,
            ),
            Err(HydraulicError::InvalidBoundary { .. })
        ));
    }

    assert!(matches!(
        transform_boundary_row(
            1,
            BoundaryRowLocation::Lower,
            row(0.0, 1.0, 0.0, 0.0),
            BoundaryCondition::Dirichlet {
                value: f64::INFINITY
            },
            1e-12,
        ),
        Err(HydraulicError::InvalidBoundary { .. })
    ));
}

#[test]
fn pivot_threshold_and_unsupported_policy_are_rejected() {
    let source = row(0.0, 1e-13, -2.0, 10.0);
    assert!(matches!(
        transform_boundary_row(
            3,
            BoundaryRowLocation::Lower,
            source,
            BoundaryCondition::Natural,
            1e-12,
        ),
        Err(HydraulicError::SingularBoundaryPivot { .. })
    ));
    assert!(matches!(
        transform_boundary_row(
            3,
            BoundaryRowLocation::Lower,
            row(0.0, 1.0, -2.0, 10.0),
            BoundaryCondition::Neumann { flux: 1.0 },
            0.0,
        ),
        Err(HydraulicError::InvalidBoundary { .. })
    ));
    assert!(matches!(
        transform_boundary_row(
            3,
            BoundaryRowLocation::Lower,
            row(0.0, 1.0, -2.0, 10.0),
            BoundaryCondition::Unsupported { selector: 4 },
            1e-12,
        ),
        Err(HydraulicError::UnsupportedBoundary { selector: 4 })
    ));
}

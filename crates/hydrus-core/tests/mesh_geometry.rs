use hydrus_core::{
    hydraulics::{
        adjacent_distance, arithmetic_face_conductivity, boundary_half_cell_width,
        geometric_face_conductivity, gravity_metric, layer_metrics, nodal_control_volume_width,
        Boundary, LayerMetric, Mesh1D,
    },
    HydraulicError,
};

fn mesh() -> Mesh1D {
    Mesh1D::new(
        vec![-3.0, -1.0, 2.0, 6.0, 10.0],
        vec![0, 0, 1, 1, 1],
        Some(vec![0, 0, 1, 1, 1]),
        2,
        2,
        -0.5,
    )
    .unwrap()
}

#[test]
fn distances_preserve_signed_coordinate_orientation_and_half_cells() {
    let mesh = mesh();
    assert_eq!(adjacent_distance(&mesh, 0).unwrap(), 2.0);
    assert_eq!(adjacent_distance(&mesh, 2).unwrap(), 4.0);
    assert_eq!(
        boundary_half_cell_width(&mesh, Boundary::Lower).unwrap(),
        1.0
    );
    assert_eq!(
        boundary_half_cell_width(&mesh, Boundary::Upper).unwrap(),
        2.0
    );
}

#[test]
fn distances_reject_out_of_range_and_single_node_boundaries() {
    assert!(matches!(
        adjacent_distance(&mesh(), 4),
        Err(HydraulicError::InvalidMesh { .. })
    ));
    let single = Mesh1D::new(vec![0.0], vec![0], None, 1, 0, 1.0).unwrap();
    assert!(matches!(
        boundary_half_cell_width(&single, Boundary::Lower),
        Err(HydraulicError::InvalidMesh { .. })
    ));
}

#[test]
fn nodal_control_volume_widths_use_half_cells_and_internal_means() {
    let uniform = Mesh1D::new(vec![0.0, 2.0, 4.0], vec![0; 3], None, 1, 0, 1.0).unwrap();
    assert_eq!(nodal_control_volume_width(&uniform, 0).unwrap(), 1.0);
    assert_eq!(nodal_control_volume_width(&uniform, 1).unwrap(), 2.0);
    assert_eq!(nodal_control_volume_width(&uniform, 2).unwrap(), 1.0);

    let mesh = mesh();
    assert_eq!(nodal_control_volume_width(&mesh, 0).unwrap(), 1.0);
    assert_eq!(nodal_control_volume_width(&mesh, 1).unwrap(), 2.5);
    assert_eq!(nodal_control_volume_width(&mesh, 2).unwrap(), 3.5);
    assert_eq!(nodal_control_volume_width(&mesh, 4).unwrap(), 2.0);
    assert!(matches!(
        nodal_control_volume_width(&mesh, 5),
        Err(HydraulicError::InvalidMesh { .. })
    ));
    let single = Mesh1D::new(vec![0.0], vec![0], None, 1, 0, 1.0).unwrap();
    assert!(matches!(
        nodal_control_volume_width(&single, 0),
        Err(HydraulicError::InvalidMesh { .. })
    ));
    let overflowing = Mesh1D::new(
        vec![-1.7e308, -0.5e308, 0.7e308, 1.7e308],
        vec![0; 4],
        None,
        1,
        0,
        1.0,
    )
    .unwrap();
    assert!(matches!(
        nodal_control_volume_width(&overflowing, 1),
        Err(HydraulicError::InvalidMesh { .. })
    ));
}

#[test]
fn layer_metrics_use_contiguous_first_to_last_spans() {
    let metrics = layer_metrics(&mesh()).unwrap();
    assert_eq!(
        metrics,
        vec![
            LayerMetric {
                layer_index: 0,
                first_node: 0,
                last_node: 1,
                span: 2.0,
            },
            LayerMetric {
                layer_index: 1,
                first_node: 2,
                last_node: 4,
                span: 8.0,
            },
        ]
    );
}

#[test]
fn layer_metrics_reject_missing_or_noncontiguous_layers() {
    let missing = Mesh1D::new(vec![0.0, 1.0], vec![0, 0], Some(vec![0, 0]), 1, 2, 1.0).unwrap();
    assert!(matches!(
        layer_metrics(&missing),
        Err(HydraulicError::InvalidMesh { .. })
    ));
    let split = Mesh1D::new(
        vec![0.0, 1.0, 2.0],
        vec![0, 0, 0],
        Some(vec![0, 1, 0]),
        1,
        2,
        1.0,
    )
    .unwrap();
    assert!(matches!(
        layer_metrics(&split),
        Err(HydraulicError::InvalidMesh { .. })
    ));
    let no_layers = Mesh1D::new(vec![0.0, 1.0], vec![0, 0], None, 1, 0, 1.0).unwrap();
    assert!(matches!(
        layer_metrics(&no_layers),
        Err(HydraulicError::InvalidMesh { .. })
    ));
}

#[test]
fn inclination_and_gravity_metric_are_explicit() {
    assert_eq!(gravity_metric(&mesh()).unwrap(), -0.5);
}

#[test]
fn face_averaging_is_explicit() {
    assert_eq!(arithmetic_face_conductivity(2.0, 8.0).unwrap(), 5.0);
    assert_eq!(geometric_face_conductivity(2.0, 8.0).unwrap(), 4.0);
    assert_eq!(geometric_face_conductivity(0.0, 8.0).unwrap(), 0.0);
}

#[test]
fn face_averaging_rejects_nonfinite_and_negative_geometric_inputs() {
    for result in [
        arithmetic_face_conductivity(f64::NAN, 1.0),
        arithmetic_face_conductivity(1.0, f64::INFINITY),
        geometric_face_conductivity(f64::NEG_INFINITY, 1.0),
        geometric_face_conductivity(-1.0, 1.0),
        geometric_face_conductivity(1.0, -1.0),
    ] {
        assert!(matches!(result, Err(HydraulicError::InvalidMesh { .. })));
    }
}

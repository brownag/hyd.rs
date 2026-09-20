use hydrus_core::{HydraulicError, Mesh1D};

fn valid_mesh() -> Mesh1D {
    Mesh1D::from_source_one_based(
        vec![-2.0, -1.0, 0.0],
        &[1, 2, 1],
        Some(&[1, 1, 2]),
        2,
        2,
        0.5,
    )
    .unwrap()
}

#[test]
fn source_indices_convert_to_zero_based_mesh_data() {
    let mesh = valid_mesh();
    assert_eq!(mesh.coordinates(), &[-2.0, -1.0, 0.0]);
    assert_eq!(mesh.material_indices(), &[0, 1, 0]);
    assert_eq!(mesh.layer_indices(), Some(&[0, 0, 1][..]));
    assert_eq!(mesh.material_count(), 2);
    assert_eq!(mesh.layer_count(), 2);
    assert_eq!(mesh.inclination_cosine(), 0.5);
}

#[test]
fn optional_layers_are_supported() {
    let mesh = Mesh1D::new(vec![0.0, 1.0], vec![0, 0], None, 1, 0, 1.0).unwrap();
    assert_eq!(mesh.layer_indices(), None);
}

#[test]
fn invalid_mesh_values_are_typed() {
    let cases = [
        Mesh1D::new(vec![], vec![], None, 1, 0, 1.0),
        Mesh1D::new(vec![0.0, 1.0], vec![0], None, 1, 0, 1.0),
        Mesh1D::new(vec![0.0, 0.0], vec![0, 0], None, 1, 0, 1.0),
        Mesh1D::new(vec![f64::NAN], vec![0], None, 1, 0, 1.0),
        Mesh1D::new(vec![0.0], vec![1], None, 1, 0, 1.0),
        Mesh1D::new(vec![0.0], vec![0], Some(vec![0, 0]), 1, 1, 1.0),
        Mesh1D::new(vec![0.0], vec![0], Some(vec![0]), 1, 0, 1.0),
        Mesh1D::new(vec![0.0], vec![0], None, 1, 0, f64::INFINITY),
    ];
    for result in cases {
        assert!(matches!(result, Err(HydraulicError::InvalidMesh { .. })));
    }
}

#[test]
fn source_adapter_rejects_zero_out_of_range_and_length_mismatch() {
    for (materials, layers) in [
        (&[0][..], None),
        (&[2][..], None),
        (&[1][..], Some(&[0][..])),
        (&[1][..], Some(&[2][..])),
        (&[1, 1][..], None),
    ] {
        let result = Mesh1D::from_source_one_based(vec![0.0], materials, layers, 1, 1, 1.0);
        assert!(matches!(result, Err(HydraulicError::InvalidMesh { .. })));
    }
}

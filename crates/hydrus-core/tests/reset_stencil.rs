use hydrus_core::hydraulics::{assemble_reset_stencil, FaceAveraging, ResetStencilInput};
use hydrus_core::Mesh1D;

fn mesh(coordinates: Vec<f64>, inclination_cosine: f64) -> Mesh1D {
    Mesh1D::new(
        coordinates.clone(),
        vec![0; coordinates.len()],
        None,
        1,
        0,
        inclination_cosine,
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn input<'a>(
    conductivities: &'a [f64],
    capacities: &'a [f64],
    h_new: &'a [f64],
    theta_new: &'a [f64],
    theta_old: &'a [f64],
    sink: &'a [f64],
    timestep: f64,
    face_averaging: FaceAveraging,
) -> ResetStencilInput<'a> {
    ResetStencilInput {
        conductivities,
        capacities,
        h_new,
        theta_new,
        theta_old,
        sink,
        timestep,
        face_averaging,
    }
}

fn assert_close(left: f64, right: f64) {
    assert!((left - right).abs() < 1e-12, "{left} != {right}");
}

#[test]
fn two_interior_rows_are_assembled_from_source_algebra() {
    let mesh = mesh(vec![0.0, 1.0, 3.0, 6.0], 1.0);
    let rows = assemble_reset_stencil(
        &mesh,
        input(
            &[2.0, 4.0, 6.0, 8.0],
            &[1.0, 2.0, 3.0, 4.0],
            &[10.0, 20.0, 30.0, 40.0],
            &[1.0, 2.0, 4.0, 5.0],
            &[0.0, 1.0, 1.0, 2.0],
            &[0.5, 1.0, 2.0, 0.0],
            2.0,
            FaceAveraging::Arithmetic,
        ),
    )
    .unwrap();

    assert_eq!(rows.len(), 2);
    assert_close(rows[0].lower, -3.0);
    assert_close(rows[0].diagonal, 7.0);
    assert_close(rows[0].upper, -2.5);
    assert_close(rows[0].rhs, 29.75);
    assert_close(rows[1].lower, -2.5);
    assert_close(rows[1].diagonal, 8.583333333333334);
    assert_close(rows[1].upper, -2.333333333333333);
    assert_close(rows[1].rhs, 105.75);
}

#[test]
fn inclination_and_geometric_faces_are_explicit() {
    let mesh = mesh(vec![0.0, 1.0, 2.0], 0.5);
    let rows = assemble_reset_stencil(
        &mesh,
        input(
            &[1.0, 4.0, 9.0],
            &[0.0, 1.0, 0.0],
            &[0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0],
            1.0,
            FaceAveraging::Geometric,
        ),
    )
    .unwrap();
    assert_close(rows[0].lower, -2.0);
    assert_close(rows[0].diagonal, 9.0);
    assert_close(rows[0].upper, -6.0);
    assert_close(rows[0].rhs, 2.0);
}

#[test]
fn storage_timestep_and_sink_terms_keep_source_signs() {
    let mesh = mesh(vec![0.0, 1.0, 2.0], 0.0);
    let rows = assemble_reset_stencil(
        &mesh,
        input(
            &[2.0, 2.0, 2.0],
            &[0.0, 4.0, 0.0],
            &[0.0, 5.0, 0.0],
            &[0.0, 2.0, 0.0],
            &[0.0, 1.0, 0.0],
            &[0.0, 3.0, 0.0],
            2.0,
            FaceAveraging::Arithmetic,
        ),
    )
    .unwrap();
    assert_close(rows[0].diagonal, 6.0);
    assert_close(rows[0].rhs, 6.5);
}

#[test]
fn invalid_inputs_are_rejected() {
    let mesh = mesh(vec![0.0, 1.0, 2.0], 1.0);
    let valid = input(
        &[1.0, 1.0, 1.0],
        &[1.0, 1.0, 1.0],
        &[0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0],
        1.0,
        FaceAveraging::Arithmetic,
    );
    assert!(assemble_reset_stencil(
        &mesh,
        ResetStencilInput {
            timestep: 0.0,
            ..valid
        }
    )
    .is_err());
    assert!(assemble_reset_stencil(
        &mesh,
        ResetStencilInput {
            conductivities: &[1.0, f64::NAN, 1.0],
            ..valid
        },
    )
    .is_err());
    assert!(assemble_reset_stencil(
        &mesh,
        ResetStencilInput {
            conductivities: &[1.0, -1.0, 1.0],
            face_averaging: FaceAveraging::Geometric,
            ..valid
        },
    )
    .is_err());
    assert!(assemble_reset_stencil(
        &mesh,
        ResetStencilInput {
            sink: &[0.0, 0.0],
            ..valid
        },
    )
    .is_err());
}

#[test]
fn source_algebra_note_is_explicit() {
    let documentation = include_str!("../src/hydraulics/stencil.rs");
    assert!(documentation.contains("not a Fortran parity result"));
    assert!(documentation.contains("finite-element"));
}

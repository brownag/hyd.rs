use hydrus_core::hydraulics::{solve_tridiagonal_in_place, TridiagonalError};

fn assert_close(left: f64, right: f64) {
    assert!((left - right).abs() < 1e-12, "{left} != {right}");
}

fn assert_solution(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (&actual, &expected) in actual.iter().zip(expected) {
        assert_close(actual, expected);
    }
}

#[test]
fn solves_a_hand_calculated_three_node_system() {
    let lower = [0.0, -1.0, -1.0];
    let mut diagonal = [2.0, 2.0, 2.0];
    let upper = [-1.0, -1.0, 0.0];
    let mut rhs = [0.0, 0.0, 4.0];

    let diagnostics =
        solve_tridiagonal_in_place(&lower, &mut diagonal, &upper, &mut rhs, 1e-12).unwrap();

    assert_solution(&rhs, &[1.0, 2.0, 3.0]);
    assert_close(diagnostics.minimum_absolute_pivot, 4.0 / 3.0);
}

#[test]
fn solves_a_larger_system_and_reports_the_smallest_pivot() {
    let lower = [0.0, -1.0, -1.0, -1.0];
    let mut diagonal = [4.0, 4.0, 4.0, 4.0];
    let upper = [-1.0, -1.0, -1.0, 0.0];
    let mut rhs = [6.0, -12.0, 13.5, -1.0];

    let diagnostics =
        solve_tridiagonal_in_place(&lower, &mut diagonal, &upper, &mut rhs, 1e-12).unwrap();

    assert_solution(&rhs, &[1.0, -2.0, 3.0, 0.5]);
    assert_close(diagnostics.minimum_absolute_pivot, 209.0 / 56.0);
}

#[test]
fn forward_and_terminal_pivot_failures_are_typed() {
    let error = solve_tridiagonal_in_place(
        &[0.0, -1.0],
        &mut [0.0, 2.0],
        &[-1.0, 0.0],
        &mut [1.0, 1.0],
        1e-12,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TridiagonalError::SingularPivot {
            index: 0,
            value: 0.0,
            minimum: 1e-12,
        }
    );

    let error =
        solve_tridiagonal_in_place(&[0.0], &mut [1e-13], &[0.0], &mut [1.0], 1e-12).unwrap_err();
    assert_eq!(
        error,
        TridiagonalError::SingularPivot {
            index: 0,
            value: 1e-13,
            minimum: 1e-12,
        }
    );
}

#[test]
fn invalid_preflight_inputs_are_typed_without_mutating_buffers() {
    let lower = [0.0, -1.0];
    let upper = [-1.0, 0.0];
    let original_diagonal = [2.0, 2.0];
    let original_rhs = [1.0, 1.0];

    for minimum_pivot in [0.0, f64::NAN, f64::INFINITY] {
        let mut diagonal = original_diagonal;
        let mut rhs = original_rhs;
        assert!(matches!(
            solve_tridiagonal_in_place(&lower, &mut diagonal, &upper, &mut rhs, minimum_pivot),
            Err(TridiagonalError::InvalidMinimumPivot { .. })
        ));
        assert_eq!(diagonal, original_diagonal);
        assert_eq!(rhs, original_rhs);
    }

    let mut diagonal = original_diagonal;
    let mut rhs = original_rhs;
    assert!(solve_tridiagonal_in_place(&lower, &mut diagonal, &upper, &mut rhs, 1e-12).is_ok());

    let mut diagonal = original_diagonal;
    let mut rhs = original_rhs;
    assert_eq!(
        solve_tridiagonal_in_place(&[0.0], &mut diagonal, &upper, &mut rhs, 1e-12),
        Err(TridiagonalError::DimensionMismatch {
            lower: 1,
            diagonal: 2,
            upper: 2,
            rhs: 2,
        })
    );
    assert_eq!(diagonal, original_diagonal);
    assert_eq!(rhs, original_rhs);

    let mut diagonal = original_diagonal;
    let mut rhs = original_rhs;
    assert_eq!(
        solve_tridiagonal_in_place(&[], &mut diagonal[..0], &[], &mut rhs[..0], 1e-12),
        Err(TridiagonalError::EmptySystem)
    );

    for (lower, diagonal, upper, rhs) in [
        ([f64::NAN, -1.0], original_diagonal, upper, original_rhs),
        (lower, [f64::NAN, 2.0], upper, original_rhs),
        (lower, original_diagonal, [f64::NAN, 0.0], original_rhs),
        (lower, original_diagonal, upper, [f64::NAN, 1.0]),
    ] {
        let mut diagonal = diagonal;
        let mut rhs = rhs;
        assert!(matches!(
            solve_tridiagonal_in_place(&lower, &mut diagonal, &upper, &mut rhs, 1e-12),
            Err(TridiagonalError::NonFiniteInput { .. })
        ));
    }

    for (lower, upper) in [([1.0, -1.0], upper), (lower, [-1.0, 1.0])] {
        let mut diagonal = original_diagonal;
        let mut rhs = original_rhs;
        assert!(matches!(
            solve_tridiagonal_in_place(&lower, &mut diagonal, &upper, &mut rhs, 1e-12),
            Err(TridiagonalError::InvalidEndpointCoefficient { .. })
        ));
        assert_eq!(diagonal, original_diagonal);
        assert_eq!(rhs, original_rhs);
    }
}

#[test]
fn caller_owned_buffers_can_be_reused_after_resetting_them() {
    let lower = [0.0, -1.0, -1.0];
    let upper = [-1.0, -1.0, 0.0];
    let original_diagonal = [2.0, 2.0, 2.0];
    let original_rhs = [0.0, 0.0, 4.0];
    let mut diagonal = original_diagonal;
    let mut rhs = original_rhs;

    for _ in 0..2 {
        let diagnostics =
            solve_tridiagonal_in_place(&lower, &mut diagonal, &upper, &mut rhs, 1e-12).unwrap();
        assert_solution(&rhs, &[1.0, 2.0, 3.0]);
        assert_close(diagnostics.minimum_absolute_pivot, 4.0 / 3.0);
        diagonal = original_diagonal;
        rhs = original_rhs;
    }
}

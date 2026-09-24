use core::fmt;

/// Minimum absolute pivot observed during a completed tridiagonal solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PivotDiagnostics {
    pub minimum_absolute_pivot: f64,
}

/// Errors raised while validating or solving a tridiagonal system.
#[derive(Clone, Debug, PartialEq)]
pub enum TridiagonalError {
    EmptySystem,
    DimensionMismatch {
        lower: usize,
        diagonal: usize,
        upper: usize,
        rhs: usize,
    },
    InvalidMinimumPivot {
        value: f64,
    },
    InvalidEndpointCoefficient {
        name: &'static str,
        value: f64,
    },
    NonFiniteInput {
        name: &'static str,
        index: usize,
        value: f64,
    },
    SingularPivot {
        index: usize,
        value: f64,
        minimum: f64,
    },
}

impl fmt::Display for TridiagonalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySystem => write!(formatter, "tridiagonal system must not be empty"),
            Self::DimensionMismatch {
                lower,
                diagonal,
                upper,
                rhs,
            } => write!(
                formatter,
                "tridiagonal buffer lengths differ: lower={lower}, diagonal={diagonal}, upper={upper}, rhs={rhs}"
            ),
            Self::InvalidMinimumPivot { value } => {
                write!(formatter, "minimum pivot must be finite and positive: {value}")
            }
            Self::InvalidEndpointCoefficient { name, value } => {
                write!(formatter, "tridiagonal endpoint coefficient {name} must be zero: {value}")
            }
            Self::NonFiniteInput { name, index, value } => {
                write!(formatter, "non-finite tridiagonal {name}[{index}]: {value}")
            }
            Self::SingularPivot {
                index,
                value,
                minimum,
            } => write!(
                formatter,
                "tridiagonal pivot at index {index} is below minimum {minimum}: {value}"
            ),
        }
    }
}

impl std::error::Error for TridiagonalError {}

/// Solve a caller-assembled tridiagonal system in place with Thomas elimination.
///
/// The zero-based row equation is
/// `lower[i] * x[i - 1] + diagonal[i] * x[i] + upper[i] * x[i + 1] = rhs[i]`.
/// `lower[0]` and `upper[n - 1]` must be zero. Boundary rows are prepared by the
/// caller before this function is invoked. On success, `rhs` contains `x`, while
/// `diagonal` contains the forward-eliminated pivots.
///
/// This is source-algebra and Rust contract evidence, not a Fortran parity result.
pub fn solve_tridiagonal_in_place(
    lower: &[f64],
    diagonal: &mut [f64],
    upper: &[f64],
    rhs: &mut [f64],
    minimum_pivot: f64,
) -> Result<PivotDiagnostics, TridiagonalError> {
    validate_inputs(lower, diagonal, upper, rhs, minimum_pivot)?;

    let mut minimum_absolute_pivot = f64::INFINITY;
    for index in 1..diagonal.len() {
        let pivot = diagonal[index - 1];
        observe_pivot(index - 1, pivot, minimum_pivot, &mut minimum_absolute_pivot)?;
        let factor = lower[index] / pivot;
        diagonal[index] -= factor * upper[index - 1];
        rhs[index] -= factor * rhs[index - 1];
    }

    let last = diagonal.len() - 1;
    observe_pivot(
        last,
        diagonal[last],
        minimum_pivot,
        &mut minimum_absolute_pivot,
    )?;
    rhs[last] /= diagonal[last];

    for index in (0..last).rev() {
        observe_pivot(
            index,
            diagonal[index],
            minimum_pivot,
            &mut minimum_absolute_pivot,
        )?;
        rhs[index] = (rhs[index] - upper[index] * rhs[index + 1]) / diagonal[index];
    }

    Ok(PivotDiagnostics {
        minimum_absolute_pivot,
    })
}

fn validate_inputs(
    lower: &[f64],
    diagonal: &[f64],
    upper: &[f64],
    rhs: &[f64],
    minimum_pivot: f64,
) -> Result<(), TridiagonalError> {
    if diagonal.is_empty() {
        return Err(TridiagonalError::EmptySystem);
    }
    if lower.len() != diagonal.len() || upper.len() != diagonal.len() || rhs.len() != diagonal.len()
    {
        return Err(TridiagonalError::DimensionMismatch {
            lower: lower.len(),
            diagonal: diagonal.len(),
            upper: upper.len(),
            rhs: rhs.len(),
        });
    }
    if !minimum_pivot.is_finite() || minimum_pivot <= 0.0 {
        return Err(TridiagonalError::InvalidMinimumPivot {
            value: minimum_pivot,
        });
    }
    validate_finite("lower", lower)?;
    validate_finite("diagonal", diagonal)?;
    validate_finite("upper", upper)?;
    validate_finite("rhs", rhs)?;
    if lower[0] != 0.0 {
        return Err(TridiagonalError::InvalidEndpointCoefficient {
            name: "lower[0]",
            value: lower[0],
        });
    }
    let last = diagonal.len() - 1;
    if upper[last] != 0.0 {
        return Err(TridiagonalError::InvalidEndpointCoefficient {
            name: "upper[n - 1]",
            value: upper[last],
        });
    }
    Ok(())
}

fn validate_finite(name: &'static str, values: &[f64]) -> Result<(), TridiagonalError> {
    for (index, &value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(TridiagonalError::NonFiniteInput { name, index, value });
        }
    }
    Ok(())
}

fn observe_pivot(
    index: usize,
    value: f64,
    minimum: f64,
    minimum_absolute_pivot: &mut f64,
) -> Result<(), TridiagonalError> {
    if value.abs() < minimum {
        return Err(TridiagonalError::SingularPivot {
            index,
            value,
            minimum,
        });
    }
    *minimum_absolute_pivot = (*minimum_absolute_pivot).min(value.abs());
    Ok(())
}

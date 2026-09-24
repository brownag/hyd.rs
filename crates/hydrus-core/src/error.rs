use core::fmt;

use crate::hydraulics::TridiagonalError;

/// Errors raised while constructing or evaluating a hydraulic contract.
#[derive(Clone, Debug, PartialEq)]
pub enum HydraulicError {
    NonFiniteParameter {
        name: &'static str,
    },
    InvalidParameter {
        name: &'static str,
        reason: &'static str,
    },
    InvalidEvaluationDomain {
        name: &'static str,
        value: f64,
    },
    InvalidTableConfiguration {
        reason: &'static str,
    },
    InvalidMesh {
        name: &'static str,
        reason: &'static str,
    },
    UnsupportedSelector {
        selector: i32,
    },
    InvalidBoundary {
        name: &'static str,
        reason: &'static str,
    },
    UnsupportedBoundary {
        selector: i32,
    },
    SingularBoundaryPivot {
        value: f64,
        minimum: f64,
    },
}

impl fmt::Display for HydraulicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter { name } => write!(formatter, "non-finite parameter: {name}"),
            Self::InvalidParameter { name, reason } => {
                write!(formatter, "invalid parameter {name}: {reason}")
            }
            Self::InvalidEvaluationDomain { name, value } => {
                write!(formatter, "invalid evaluation domain {name}={value}")
            }
            Self::InvalidTableConfiguration { reason } => {
                write!(formatter, "invalid table configuration: {reason}")
            }
            Self::InvalidMesh { name, reason } => {
                write!(formatter, "invalid mesh {name}: {reason}")
            }
            Self::UnsupportedSelector { selector } => {
                write!(formatter, "unsupported hydraulic selector: {selector}")
            }
            Self::InvalidBoundary { name, reason } => {
                write!(formatter, "invalid boundary {name}: {reason}")
            }
            Self::UnsupportedBoundary { selector } => {
                write!(formatter, "unsupported boundary selector: {selector}")
            }
            Self::SingularBoundaryPivot { value, minimum } => {
                write!(
                    formatter,
                    "boundary pivot {value} is below minimum {minimum}"
                )
            }
        }
    }
}

impl std::error::Error for HydraulicError {}

/// Errors raised by the filesystem-free transactional water-step contract.
#[derive(Clone, Debug, PartialEq)]
pub enum WaterStepError {
    InvalidState {
        reason: &'static str,
    },
    InvalidWorkspace {
        reason: &'static str,
    },
    InsufficientAtmosphericTraceCapacity {
        required: usize,
        available: usize,
    },
    InvalidPolicy {
        name: &'static str,
    },
    UnsupportedConfiguration {
        name: &'static str,
        value: usize,
    },
    InvalidForcing {
        name: &'static str,
        reason: &'static str,
    },
    /// Atmospheric APIs currently require a caller-declared Dirichlet critical head.
    UnsupportedAtmosphericInput {
        name: &'static str,
    },
    DimensionMismatch,
    NonFiniteInput {
        name: &'static str,
    },
    InvalidTargetTime {
        current_time: f64,
        target_time: f64,
    },
    NonFiniteDiagnostic,
    NonConverged {
        timestep: f64,
        minimum_timestep: f64,
        retries: usize,
    },
    Hydraulic(HydraulicError),
    Linear(TridiagonalError),
}

impl fmt::Display for WaterStepError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState { reason } => write!(formatter, "invalid water state: {reason}"),
            Self::InvalidWorkspace { reason } => write!(formatter, "invalid water workspace: {reason}"),
            Self::InsufficientAtmosphericTraceCapacity { required, available } => write!(
                formatter,
                "atmospheric trace capacity {available} is below required {required}"
            ),
            Self::InvalidPolicy { name } => write!(formatter, "invalid water-step policy: {name}"),
            Self::UnsupportedConfiguration { name, value } => {
                write!(formatter, "unsupported water configuration {name}={value}")
            }
            Self::InvalidForcing { name, reason } => {
                write!(formatter, "invalid atmospheric forcing {name}: {reason}")
            }
            Self::UnsupportedAtmosphericInput { name } => {
                write!(formatter, "unsupported atmospheric input: {name}")
            }
            Self::DimensionMismatch => write!(formatter, "water-step dimensions differ"),
            Self::NonFiniteInput { name } => write!(formatter, "non-finite water-step input: {name}"),
            Self::InvalidTargetTime {
                current_time,
                target_time,
            } => write!(
                formatter,
                "target time {target_time} must be greater than current time {current_time}"
            ),
            Self::NonFiniteDiagnostic => write!(formatter, "non-finite water-step diagnostic"),
            Self::NonConverged { timestep, minimum_timestep, retries } => write!(formatter, "water step did not converge at dt={timestep} with dt_min={minimum_timestep} after {retries} retries"),
            Self::Hydraulic(error) => error.fmt(formatter),
            Self::Linear(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WaterStepError {}

impl From<HydraulicError> for WaterStepError {
    fn from(error: HydraulicError) -> Self {
        Self::Hydraulic(error)
    }
}

impl From<TridiagonalError> for WaterStepError {
    fn from(error: TridiagonalError) -> Self {
        Self::Linear(error)
    }
}

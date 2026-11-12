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
    InvalidLowerBoundary {
        name: &'static str,
    },
    UnsupportedLowerBoundary {
        mode: &'static str,
    },
    /// The upper boundary variant does not match the step entry point.
    InvalidUpperBoundary {
        name: &'static str,
    },
    InvalidRootUptake {
        name: &'static str,
    },
    UnsupportedRootUptake {
        option: usize,
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
    Vapor(crate::heat::vapor::VaporError),
}

/// Errors raised while validating the public single-domain solute contract.
#[derive(Clone, Debug, PartialEq)]
pub enum SoluteConfigError {
    Invalid {
        name: &'static str,
        reason: &'static str,
    },
    Unsupported {
        option: &'static str,
    },
}

impl fmt::Display for SoluteConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid { name, reason } => write!(formatter, "invalid solute {name}: {reason}"),
            Self::Unsupported { option } => {
                write!(formatter, "unsupported solute option: {option}")
            }
        }
    }
}

impl std::error::Error for SoluteConfigError {}

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
            Self::InvalidLowerBoundary { name } => {
                write!(formatter, "invalid lower boundary parameter: {name}")
            }
            Self::UnsupportedLowerBoundary { mode } => {
                write!(formatter, "unsupported lower boundary mode: {mode}")
            }
            Self::InvalidUpperBoundary { name } => {
                write!(formatter, "invalid upper boundary parameter: {name}")
            }
            Self::InvalidRootUptake { name } => {
                write!(formatter, "invalid root-uptake input: {name}")
            }
            Self::UnsupportedRootUptake { option } => {
                write!(formatter, "unsupported root-uptake option: {option}")
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
            Self::Vapor(error) => error.fmt(formatter),
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

impl From<crate::heat::vapor::VaporError> for WaterStepError {
    fn from(error: crate::heat::vapor::VaporError) -> Self {
        Self::Vapor(error)
    }
}

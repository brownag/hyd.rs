use core::fmt;

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

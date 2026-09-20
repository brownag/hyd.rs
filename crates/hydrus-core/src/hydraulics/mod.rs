mod types;

pub use types::{Evaluation, HydraulicDerivatives, VanGenuchtenParameters};

use crate::HydraulicError;

/// Hydraulic families recognized by the public selector boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HydraulicModelSelector {
    VanGenuchten,
}

impl HydraulicModelSelector {
    pub fn from_i32(selector: i32) -> Result<Self, HydraulicError> {
        match selector {
            0 => Ok(Self::VanGenuchten),
            selector => Err(HydraulicError::UnsupportedSelector { selector }),
        }
    }
}

/// Value and derivative contract shared by hydraulic families.
pub trait HydraulicModel: Send + Sync {
    fn theta(&self, head: f64) -> Result<f64, HydraulicError>;
    fn conductivity(&self, head: f64) -> Result<f64, HydraulicError>;
    fn capacity(&self, head: f64) -> Result<f64, HydraulicError>;
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError>;
    fn derivatives(&self, head: f64) -> Result<HydraulicDerivatives, HydraulicError>;
}

/// Validated model-0 contract. Formula evaluation is deliberately deferred.
#[derive(Clone, Debug, PartialEq)]
pub struct VanGenuchtenModel {
    parameters: VanGenuchtenParameters,
}

impl VanGenuchtenModel {
    pub fn new(parameters: VanGenuchtenParameters) -> Result<Self, HydraulicError> {
        parameters.validate()?;
        Ok(Self { parameters })
    }

    pub fn parameters(&self) -> &VanGenuchtenParameters {
        &self.parameters
    }
}

impl HydraulicModel for VanGenuchtenModel {
    fn theta(&self, head: f64) -> Result<f64, HydraulicError> {
        Err(HydraulicError::InvalidEvaluationDomain {
            name: "head (formula pending)",
            value: head,
        })
    }

    fn conductivity(&self, head: f64) -> Result<f64, HydraulicError> {
        Err(HydraulicError::InvalidEvaluationDomain {
            name: "head (formula pending)",
            value: head,
        })
    }

    fn capacity(&self, head: f64) -> Result<f64, HydraulicError> {
        Err(HydraulicError::InvalidEvaluationDomain {
            name: "head (formula pending)",
            value: head,
        })
    }

    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Err(HydraulicError::InvalidEvaluationDomain {
            name: "head (formula pending)",
            value: head,
        })
    }

    fn derivatives(&self, head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        Err(HydraulicError::InvalidEvaluationDomain {
            name: "head (formula pending)",
            value: head,
        })
    }
}

/// Immutable table contract reserved for the generated-table milestone.
#[derive(Clone, Debug, PartialEq)]
pub struct HydraulicTables;

impl HydraulicTables {
    pub fn from_selector(selector: i32) -> Result<Self, HydraulicError> {
        Err(HydraulicError::UnsupportedSelector { selector })
    }

    pub fn empty() -> Result<Self, HydraulicError> {
        Err(HydraulicError::InvalidTableConfiguration {
            reason: "no table data",
        })
    }
}

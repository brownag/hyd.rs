mod geometry;
mod table;
mod types;
mod van_genuchten;

pub use geometry::{
    adjacent_distance, arithmetic_face_conductivity, boundary_half_cell_width,
    geometric_face_conductivity, gravity_metric, layer_metrics, Boundary, LayerMetric,
};
pub use types::{Evaluation, HydraulicDerivatives, Mesh1D, VanGenuchtenParameters};
use van_genuchten::evaluate_model_zero;

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

/// Validated analytic `iModel=0` van Genuchten contract.
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
        Ok(evaluate_model_zero(&self.parameters, head)?.theta)
    }

    fn conductivity(&self, head: f64) -> Result<f64, HydraulicError> {
        Ok(evaluate_model_zero(&self.parameters, head)?.conductivity)
    }

    fn capacity(&self, head: f64) -> Result<f64, HydraulicError> {
        Ok(evaluate_model_zero(&self.parameters, head)?.capacity)
    }

    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        let evaluation = evaluate_model_zero(&self.parameters, head)?;
        Ok(Evaluation {
            theta: evaluation.theta,
            conductivity: evaluation.conductivity,
            capacity: evaluation.capacity,
        })
    }

    fn derivatives(&self, head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        Ok(evaluate_model_zero(&self.parameters, head)?.derivatives)
    }
}

impl VanGenuchtenModel {
    /// Invert model-0 theta; endpoint `Qr` uses the finite source-style dry floor.
    pub fn head(&self, theta: f64) -> Result<f64, HydraulicError> {
        van_genuchten::inverse_head(&self.parameters, theta)
    }
}

pub use table::HydraulicTables;

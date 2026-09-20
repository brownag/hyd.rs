use crate::HydraulicError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VanGenuchtenParameters {
    pub residual_water_content: f64,
    pub saturated_water_content: f64,
    pub alpha: f64,
    pub n: f64,
    pub saturated_conductivity: f64,
}

impl VanGenuchtenParameters {
    pub fn new(
        residual_water_content: f64,
        saturated_water_content: f64,
        alpha: f64,
        n: f64,
        saturated_conductivity: f64,
    ) -> Result<Self, HydraulicError> {
        let parameters = Self {
            residual_water_content,
            saturated_water_content,
            alpha,
            n,
            saturated_conductivity,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    pub fn validate(&self) -> Result<(), HydraulicError> {
        for (name, value) in [
            ("residual_water_content", self.residual_water_content),
            ("saturated_water_content", self.saturated_water_content),
            ("alpha", self.alpha),
            ("n", self.n),
            ("saturated_conductivity", self.saturated_conductivity),
        ] {
            if !value.is_finite() {
                return Err(HydraulicError::NonFiniteParameter { name });
            }
        }
        if self.residual_water_content >= self.saturated_water_content {
            return Err(HydraulicError::InvalidParameter {
                name: "water_content",
                reason: "residual must be below saturated",
            });
        }
        if self.alpha <= 0.0 {
            return Err(HydraulicError::InvalidParameter {
                name: "alpha",
                reason: "must be positive",
            });
        }
        if self.n <= 1.0 {
            return Err(HydraulicError::InvalidParameter {
                name: "n",
                reason: "must be greater than one",
            });
        }
        if self.saturated_conductivity < 0.0 {
            return Err(HydraulicError::InvalidParameter {
                name: "saturated_conductivity",
                reason: "must be non-negative",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Evaluation {
    pub theta: f64,
    pub conductivity: f64,
    pub capacity: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HydraulicDerivatives {
    pub dtheta_dhead: f64,
    pub dconductivity_dhead: f64,
    pub dcapacity_dhead: f64,
}

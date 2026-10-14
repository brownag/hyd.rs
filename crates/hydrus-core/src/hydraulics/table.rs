use super::{Evaluation, HydraulicDerivatives, HydraulicModel, VanGenuchtenModel};
use crate::HydraulicError;

/// Immutable, source-shaped lookup tables generated from the analytic model-0
/// contract. The generated domain contains only negative finite heads.
/// Water content is dimensionless; the other unit symbols are caller-declared
/// metadata and do not change table values.
#[derive(Clone, Debug, PartialEq)]
pub struct HydraulicTables {
    heads: Vec<f64>,
    values: Vec<Evaluation>,
    head_unit: UnitSymbol,
    conductivity_unit: UnitSymbol,
    capacity_unit: UnitSymbol,
}

/// Caller-declared unit label for a hydraulic table quantity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitSymbol(String);

impl UnitSymbol {
    pub fn new(symbol: impl Into<String>) -> Result<Self, HydraulicError> {
        let symbol = symbol.into();
        if symbol.trim().is_empty() {
            return Err(HydraulicError::InvalidTableConfiguration {
                reason: "unit symbols must not be empty",
            });
        }
        Ok(Self(symbol))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl HydraulicTables {
    /// Generates logarithmically spaced heads from the more-negative `h_min`
    /// to the less-negative `h_max`, inclusive.
    pub fn new(
        model: &VanGenuchtenModel,
        h_min: f64,
        h_max: f64,
        points: usize,
        head_unit: UnitSymbol,
        conductivity_unit: UnitSymbol,
        capacity_unit: UnitSymbol,
    ) -> Result<Self, HydraulicError> {
        if !h_min.is_finite() || !h_max.is_finite() {
            return Err(HydraulicError::InvalidTableConfiguration {
                reason: "head endpoints must be finite",
            });
        }
        if h_min >= 0.0 || h_max >= 0.0 || h_min >= h_max {
            return Err(HydraulicError::InvalidTableConfiguration {
                reason: "head endpoints must be ordered and strictly negative",
            });
        }
        if points < 2 {
            return Err(HydraulicError::InvalidTableConfiguration {
                reason: "at least two table points are required",
            });
        }

        let log_min = (-h_min).log10();
        let log_max = (-h_max).log10();
        let step = (log_max - log_min) / (points - 1) as f64;
        let mut heads = Vec::with_capacity(points);
        let mut values = Vec::with_capacity(points);
        for index in 0..points {
            let head = if index == 0 {
                h_min
            } else if index + 1 == points {
                h_max
            } else {
                -10f64.powf(log_min + index as f64 * step)
            };
            heads.push(head);
            values.push(model.evaluate(head)?);
        }
        Ok(Self {
            heads,
            values,
            head_unit,
            conductivity_unit,
            capacity_unit,
        })
    }

    pub fn head_unit(&self) -> &UnitSymbol {
        &self.head_unit
    }

    pub fn conductivity_unit(&self) -> &UnitSymbol {
        &self.conductivity_unit
    }

    pub fn capacity_unit(&self) -> &UnitSymbol {
        &self.capacity_unit
    }

    pub fn heads(&self) -> &[f64] {
        &self.heads
    }

    pub fn values(&self) -> &[Evaluation] {
        &self.values
    }

    /// Evaluates by linear interpolation in head, matching `SetMat`'s `dh`
    /// branch. Endpoints are inclusive; no saturation extension or
    /// extrapolation is performed. General/user-supplied tables are not this
    /// type and unsupported selectors never route through this cache.
    pub fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        if !head.is_finite() || head < self.heads[0] || head > self.heads[self.heads.len() - 1] {
            return Err(HydraulicError::InvalidEvaluationDomain {
                name: "head",
                value: head,
            });
        }
        if head == self.heads[0] {
            return Ok(self.values[0]);
        }
        let last = self.heads.len() - 1;
        if head == self.heads[last] {
            return Ok(self.values[last]);
        }
        let upper = self.heads.partition_point(|&node| node < head);
        let lower = upper - 1;
        let dh = (head - self.heads[lower]) / (self.heads[upper] - self.heads[lower]);
        let left = self.values[lower];
        let right = self.values[upper];
        Ok(Evaluation {
            theta: left.theta + (right.theta - left.theta) * dh,
            conductivity: left.conductivity + (right.conductivity - left.conductivity) * dh,
            capacity: left.capacity + (right.capacity - left.capacity) * dh,
        })
    }

    /// Selects the right segment at internal knots and the left segment at the
    /// final endpoint, where no right segment exists.
    fn segment(&self, head: f64) -> Result<usize, HydraulicError> {
        if !head.is_finite() || head < self.heads[0] || head > self.heads[self.heads.len() - 1] {
            return Err(HydraulicError::InvalidEvaluationDomain {
                name: "head",
                value: head,
            });
        }
        let last = self.heads.len() - 1;
        if head == self.heads[last] {
            return Ok(last - 1);
        }
        if head == self.heads[0] {
            return Ok(0);
        }
        let upper = self.heads.partition_point(|&node| node <= head);
        Ok(upper - 1)
    }
}

impl HydraulicModel for HydraulicTables {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Self::evaluate(self, head)
    }

    fn derivatives(&self, head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        let lower = self.segment(head)?;
        let upper = lower + 1;
        let width = self.heads[upper] - self.heads[lower];
        let left = self.values[lower];
        let right = self.values[upper];
        Ok(HydraulicDerivatives {
            dtheta_dhead: (right.theta - left.theta) / width,
            dconductivity_dhead: (right.conductivity - left.conductivity) / width,
            dcapacity_dhead: (right.capacity - left.capacity) / width,
        })
    }
}

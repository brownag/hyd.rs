use crate::water_state::validate_finite;
use crate::{HydraulicModel, Mesh1D, WaterStepError};

/// Feddes water-stress breakpoints and root-weighted potential demand.
///
/// Heads use the same length unit as the hydraulic models. `potential_demand`
/// is positive uptake depth per time, `root_weights` are nonnegative root
/// densities per length, and realized sink rates are positive extraction.
#[derive(Clone, Copy, Debug)]
pub struct RootUptakeInput<'a> {
    pub model: RootUptakeModel,
    pub h0: f64,
    pub h1: f64,
    pub h2: f64,
    pub h3: f64,
    pub potential_demand: f64,
    pub compensation_threshold: f64,
    pub root_weights: &'a [f64],
}

/// Root-uptake formulations accepted by the water product.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootUptakeModel {
    Feddes,
    Unsupported(usize),
}

impl RootUptakeInput<'_> {
    pub(crate) fn validate(&self, node_count: usize) -> Result<(), WaterStepError> {
        if let RootUptakeModel::Unsupported(option) = self.model {
            return Err(WaterStepError::UnsupportedRootUptake { option });
        }
        validate_finite(
            "root_uptake_parameters",
            &[
                self.h0,
                self.h1,
                self.h2,
                self.h3,
                self.potential_demand,
                self.compensation_threshold,
            ],
        )?;
        if !(self.h0 >= self.h1 && self.h1 >= self.h2 && self.h2 >= self.h3)
            || self.potential_demand < 0.0
            || self.compensation_threshold <= 0.0
        {
            return Err(WaterStepError::InvalidRootUptake {
                name: "breakpoints_demand_or_compensation",
            });
        }
        if self.root_weights.len() != node_count {
            return Err(WaterStepError::DimensionMismatch);
        }
        validate_finite("root_weights", self.root_weights)?;
        if self.root_weights.iter().any(|weight| *weight < 0.0) {
            return Err(WaterStepError::InvalidRootUptake {
                name: "root_weights",
            });
        }
        Ok(())
    }

    pub(crate) fn fill_trial_sink(
        &self,
        mesh: &Mesh1D,
        materials: &[&dyn HydraulicModel],
        heads: &[f64],
        water_contents: &[f64],
        timestep: f64,
        sink: &mut [f64],
    ) -> Result<(), WaterStepError> {
        let mut omega = 0.0;
        for (node, &head) in heads.iter().enumerate() {
            let width = crate::hydraulics::nodal_control_volume_width(mesh, node)?;
            omega += feddes_reduction(head, self) * self.root_weights[node] * width;
        }
        if !omega.is_finite() {
            return Err(WaterStepError::NonFiniteDiagnostic);
        }
        if omega == 0.0 {
            sink.fill(0.0);
            return Ok(());
        }
        let compensation = if omega < self.compensation_threshold {
            self.compensation_threshold
        } else {
            omega
        };
        for (node, &head) in heads.iter().enumerate() {
            let alpha = feddes_reduction(head, self);
            let demand = alpha * self.root_weights[node] * self.potential_demand / compensation;
            let theta_wilt = materials[mesh.material_indices()[node]]
                .evaluate(self.h3)?
                .theta;
            if !theta_wilt.is_finite() {
                return Err(WaterStepError::NonFiniteDiagnostic);
            }
            let available = 0.5 * (water_contents[node] - theta_wilt).max(0.0) / timestep;
            if !demand.is_finite() || !available.is_finite() {
                return Err(WaterStepError::NonFiniteDiagnostic);
            }
            sink[node] = demand.min(available);
            if !sink[node].is_finite() {
                return Err(WaterStepError::NonFiniteDiagnostic);
            }
        }
        Ok(())
    }

    /// Computes the positive, capped root-extraction rate at each node.
    pub fn compute_into(
        &self,
        mesh: &Mesh1D,
        materials: &[&dyn HydraulicModel],
        heads: &[f64],
        water_contents: &[f64],
        timestep: f64,
        sink: &mut [f64],
    ) -> Result<(), WaterStepError> {
        let count = mesh.coordinates().len();
        self.validate(count)?;
        if heads.len() != count || water_contents.len() != count || sink.len() != count {
            return Err(WaterStepError::DimensionMismatch);
        }
        if materials.len() != mesh.material_count() {
            return Err(WaterStepError::DimensionMismatch);
        }
        if !timestep.is_finite() || timestep <= 0.0 {
            return Err(WaterStepError::InvalidRootUptake { name: "timestep" });
        }
        validate_finite("root_trial_heads", heads)?;
        validate_finite("root_trial_water_contents", water_contents)?;
        self.fill_trial_sink(mesh, materials, heads, water_contents, timestep, sink)
    }
}

/// Piecewise Feddes reduction: anoxia at/above h0, optimal uptake h1..h2,
/// linear stress transitions h0..h1 and h3..h2, and wilting at/below h3.
pub fn feddes_reduction(head: f64, input: &RootUptakeInput<'_>) -> f64 {
    if head <= input.h3 || head >= input.h0 {
        0.0
    } else if head < input.h2 {
        (head - input.h3) / (input.h2 - input.h3)
    } else if head <= input.h1 {
        1.0
    } else {
        (input.h0 - head) / (input.h0 - input.h1)
    }
}

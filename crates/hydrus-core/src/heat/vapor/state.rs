//! Owned accepted vapor history with staged, atomic promotion.

use super::kernels;
use super::water::{phase_velocities, PhaseVelocityInput};
use super::{VaporError, VaporHistoryView, VaporPrimitiveInput};

/// Initial data used to calculate vapor content and initial phase velocities.
#[derive(Clone, Copy)]
pub struct VaporStateInput<'a> {
    pub primitive: VaporPrimitiveInput<'a>,
    pub coordinates: &'a [f64],
    pub gravity: f64,
    pub initial_head_predictor: &'a [f64],
    pub initialization_timestep: f64,
    pub time: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct VaporCandidate {
    time: f64,
    previous_timestep: f64,
    heads: Vec<f64>,
    liquid_water_contents: Vec<f64>,
    temperatures: Vec<f64>,
    vapor_contents: Vec<f64>,
    liquid_velocities: Vec<f64>,
    vapor_velocities: Vec<f64>,
    head_predictor: Vec<f64>,
}

/// Accepted vapor histories and an optional uncommitted candidate.
#[derive(Clone, Debug, PartialEq)]
pub struct VaporState {
    accepted: VaporCandidate,
    staged: Option<VaporCandidate>,
}

impl VaporState {
    /// Calculates, rather than guesses, the initial vapor content and velocities.
    pub fn new(input: VaporStateInput<'_>) -> Result<Self, VaporError> {
        input.primitive.validate()?;
        let n = input.primitive.heads.len();
        check_len("coordinates", n, input.coordinates.len())?;
        check_len(
            "initial_head_predictor",
            n,
            input.initial_head_predictor.len(),
        )?;
        finite_slice("coordinates", input.coordinates)?;
        finite_slice("initial_head_predictor", input.initial_head_predictor)?;
        finite_slice("gravity", &[input.gravity])?;
        finite_slice("time", &[input.time, input.initialization_timestep])?;
        if input.initialization_timestep <= 0.0 {
            return Err(VaporError::InvalidValue {
                name: "initialization_timestep",
                value: input.initialization_timestep,
            });
        }
        if input.coordinates.windows(2).any(|pair| pair[1] <= pair[0]) {
            return Err(VaporError::InvalidValue {
                name: "coordinates",
                value: f64::NAN,
            });
        }
        let properties = kernels::evaluate(&input.primitive)?;
        let zero_sink = vec![0.0; n];
        let phase = phase_velocities(&PhaseVelocityInput {
            coordinates: input.coordinates,
            gravity: input.gravity,
            heads: input.primitive.heads,
            liquid_conductivities: &properties.liquid_conductivities,
            liquid_water_old: input.primitive.liquid_water_contents,
            liquid_water_new: input.primitive.liquid_water_contents,
            sink: &zero_sink,
            temperatures: input.primitive.temperatures,
            vapor_water_old: &properties.vapor_contents,
            vapor_water_new: &properties.vapor_contents,
            timestep: input.initialization_timestep,
            vapor: &properties,
        })?;
        let accepted = VaporCandidate {
            time: input.time,
            previous_timestep: input.initialization_timestep,
            heads: input.primitive.heads.to_vec(),
            liquid_water_contents: input.primitive.liquid_water_contents.to_vec(),
            temperatures: input.primitive.temperatures.to_vec(),
            vapor_contents: properties.vapor_contents,
            liquid_velocities: phase.liquid,
            vapor_velocities: phase.vapor,
            head_predictor: input.initial_head_predictor.to_vec(),
        };
        candidate_view(&accepted).validate()?;
        Ok(Self {
            accepted,
            staged: None,
        })
    }

    /// Immutable view of accepted values. Staged data is never visible here.
    pub fn snapshot(&self) -> VaporHistoryView<'_> {
        candidate_view(&self.accepted)
    }

    /// Stage a complete candidate. Validation completes before trial buffers change.
    #[allow(clippy::too_many_arguments)]
    pub fn stage(
        &mut self,
        time: f64,
        timestep: f64,
        heads: &[f64],
        liquid_water_contents: &[f64],
        temperatures: &[f64],
        vapor_contents: &[f64],
        liquid_velocities: &[f64],
        vapor_velocities: &[f64],
        head_predictor: &[f64],
    ) -> Result<(), VaporError> {
        let candidate = VaporCandidate {
            time,
            previous_timestep: timestep,
            heads: heads.to_vec(),
            liquid_water_contents: liquid_water_contents.to_vec(),
            temperatures: temperatures.to_vec(),
            vapor_contents: vapor_contents.to_vec(),
            liquid_velocities: liquid_velocities.to_vec(),
            vapor_velocities: vapor_velocities.to_vec(),
            head_predictor: head_predictor.to_vec(),
        };
        let view = candidate_view(&candidate);
        view.validate()?;
        if time <= self.accepted.time {
            return Err(VaporError::InvalidTime);
        }
        if (time - self.accepted.time - timestep).abs()
            > f64::EPSILON * time.abs().max(self.accepted.time.abs()).max(1.0) * 4.0
        {
            return Err(VaporError::InvalidValue {
                name: "time/timestep consistency",
                value: time - self.accepted.time - timestep,
            });
        }
        self.staged = Some(candidate);
        Ok(())
    }

    /// Atomically promote a validated candidate; missing candidate is a typed error.
    pub fn accept(&mut self) -> Result<(), VaporError> {
        let candidate = self.staged.take().ok_or(VaporError::InvalidValue {
            name: "staged vapor candidate",
            value: 0.0,
        })?;
        candidate_view(&candidate).validate()?;
        self.accepted = candidate;
        Ok(())
    }

    /// Discard all trial histories and retain accepted fields/predictor unchanged.
    pub fn reject(&mut self) {
        self.staged = None;
    }

    pub fn has_staged_candidate(&self) -> bool {
        self.staged.is_some()
    }

    pub fn staged_snapshot(&self) -> Option<VaporHistoryView<'_>> {
        self.staged.as_ref().map(candidate_view)
    }
}

fn candidate_view(candidate: &VaporCandidate) -> VaporHistoryView<'_> {
    VaporHistoryView {
        time: candidate.time,
        previous_timestep: Some(candidate.previous_timestep),
        heads: &candidate.heads,
        liquid_water_contents: &candidate.liquid_water_contents,
        temperatures: &candidate.temperatures,
        vapor_contents: &candidate.vapor_contents,
        liquid_velocities: &candidate.liquid_velocities,
        vapor_velocities: &candidate.vapor_velocities,
        head_predictor: &candidate.head_predictor,
    }
}

fn check_len(name: &'static str, expected: usize, actual: usize) -> Result<(), VaporError> {
    if expected != actual {
        return Err(VaporError::LengthMismatch {
            name,
            expected,
            actual,
        });
    }
    Ok(())
}

fn finite_slice(name: &'static str, values: &[f64]) -> Result<(), VaporError> {
    for (index, &value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(VaporError::NonFinite {
                name,
                index: Some(index),
                value,
            });
        }
    }
    Ok(())
}

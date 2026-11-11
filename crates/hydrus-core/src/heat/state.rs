//! Accepted heat state and transactional trial buffer (`TempO` / `TempN`).
//!
//! Mirrors `WaterState` (`water_state.rs:8`). `temp_old` is the accepted `TempO`;
//! `temp_new` is the trial `TempN` written by the transport step. Retry resets
//! the trial to the accepted values (`WATFLOW.FOR:139`), and only acceptance
//! copies the trial into the accepted state (`WATFLOW.FOR:1115`).

use std::fmt;

/// Typed rejection for heat state construction and acceptance.
#[derive(Clone, Debug, PartialEq)]
pub enum HeatStateError {
    /// Length below three nodes, or a shape the caller cannot use.
    InvalidState { reason: &'static str },
    /// A time or temperature is NaN or infinite.
    NonFiniteInput { name: &'static str },
}

impl fmt::Display for HeatStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState { reason } => write!(formatter, "invalid heat state: {reason}"),
            Self::NonFiniteInput { name } => write!(formatter, "non-finite heat input: {name}"),
        }
    }
}

impl std::error::Error for HeatStateError {}

/// Accepted nodal temperatures and time, with a trial buffer for one step.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatState {
    time: f64,
    temp_old: Vec<f64>,
    temp_new: Vec<f64>,
}

/// Borrowed accepted heat values. Never exposes the trial buffer.
#[derive(Clone, Copy, Debug)]
pub struct AcceptedHeatSnapshot<'a> {
    pub accepted_time: f64,
    pub temperatures: &'a [f64],
}

impl HeatState {
    /// Node count is checked against the mesh by the driver (24.4.1), not here.
    pub fn new(time: f64, temperatures: Vec<f64>) -> Result<Self, HeatStateError> {
        validate_finite_scalar("time", time)?;
        if temperatures.len() < 3 {
            return Err(HeatStateError::InvalidState {
                reason: "temperatures must have at least three nodes",
            });
        }
        validate_finite("temperatures", &temperatures)?;
        let temp_new = temperatures.clone();
        Ok(Self {
            time,
            temp_old: temperatures,
            temp_new,
        })
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn temperatures(&self) -> &[f64] {
        &self.temp_old
    }

    pub fn trial_temperatures(&self) -> &[f64] {
        &self.temp_new
    }

    pub fn trial_temperatures_mut(&mut self) -> &mut [f64] {
        &mut self.temp_new
    }

    /// Resets the trial buffer to the accepted values (`WATFLOW.FOR:139`).
    pub fn reset_trial(&mut self) {
        self.temp_new.copy_from_slice(&self.temp_old);
    }

    /// Commits the trial buffer and time. All checks run before any write.
    pub fn accept(&mut self, accepted_time: f64) -> Result<(), HeatStateError> {
        validate_finite_scalar("accepted_time", accepted_time)?;
        validate_finite("trial_temperatures", &self.temp_new)?;
        self.temp_old.copy_from_slice(&self.temp_new);
        self.time = accepted_time;
        Ok(())
    }

    /// Discards the trial. Accepted time and temperatures are unchanged.
    pub fn reject(&mut self) {
        self.reset_trial();
    }

    pub fn snapshot(&self) -> AcceptedHeatSnapshot<'_> {
        AcceptedHeatSnapshot {
            accepted_time: self.time,
            temperatures: &self.temp_old,
        }
    }
}

fn validate_finite_scalar(name: &'static str, value: f64) -> Result<(), HeatStateError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(HeatStateError::NonFiniteInput { name })
    }
}

fn validate_finite(name: &'static str, values: &[f64]) -> Result<(), HeatStateError> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(HeatStateError::NonFiniteInput { name })
    }
}

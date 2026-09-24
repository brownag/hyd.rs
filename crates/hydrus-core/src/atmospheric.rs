//! Minimal in-memory atmospheric boundary scheduling.
use crate::forcing::{AtmosphericRates, ForcingSchedule};
use crate::{
    WaterState, WaterStepError, WaterStepInput, WaterStepPolicy, WaterStepReport, WaterWorkspace,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpperBoundaryMode {
    AtmosphericFlux,
    PondedHead,
}

/// Accepted atmospheric history attached to a water simulation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphericState {
    pub event_cursor: usize,
    pub ponded_depth: f64,
    pub upper_mode: UpperBoundaryMode,
}

impl Default for AtmosphericState {
    fn default() -> Self {
        Self {
            event_cursor: 0,
            ponded_depth: 0.0,
            upper_mode: UpperBoundaryMode::AtmosphericFlux,
        }
    }
}

/// Advances to the earlier of the requested target and next forcing event.
/// The schedule uses caller-declared time and depth-rate units without conversion.
/// `input.upper_condition` must be `Dirichlet`; its value is the caller-declared
/// atmospheric critical head used by the upper flux/head switch.
pub fn advance_atmospheric_step(
    state: &mut WaterState,
    workspace: &mut WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
    schedule: &ForcingSchedule<'_>,
) -> Result<WaterStepReport, WaterStepError> {
    if state.atmospheric.event_cursor > schedule.events().len() {
        return Err(WaterStepError::InvalidForcing {
            name: "event_cursor",
            reason: "cursor exceeds schedule length",
        });
    }
    if schedule.events()[state.atmospheric.event_cursor..]
        .iter()
        .any(|event| event.time <= state.time())
    {
        return Err(WaterStepError::InvalidForcing {
            name: "event_times",
            reason: "events must be later than the current accepted time",
        });
    }
    let mut next_input = input;
    if let Some(event) = schedule.events().get(state.atmospheric.event_cursor) {
        next_input.target_time = next_input.target_time.min(event.time);
    }
    let rates = if state.atmospheric.event_cursor == 0 {
        schedule.initial
    } else {
        schedule.events()[state.atmospheric.event_cursor - 1].rates
    };
    let mut report = crate::water_solver::advance_atmospheric_inner(
        state, workspace, next_input, policy, rates,
    )?;
    while state.atmospheric.event_cursor < schedule.events().len()
        && state.time() >= schedule.events()[state.atmospheric.event_cursor].time
    {
        state.atmospheric.event_cursor += 1;
    }
    report.accepted_time = state.time();
    Ok(report)
}

pub(crate) fn net_flux(rates: AtmosphericRates) -> f64 {
    rates.precipitation - rates.evaporation
}

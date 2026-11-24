use crate::WaterStepError;

/// Caller-declared labels for the forcing time and depth-rate values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForcingUnits<'a> {
    pub time: &'a str,
    pub water_depth_rate: &'a str,
}

/// Atmospheric rates that become active at an event time. Positive values
/// denote precipitation or evaporative demand; no unit conversion is applied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphericRates {
    pub precipitation: f64,
    pub evaporation: f64,
}

/// Vapor-path surface settings for `advance_atmospheric_step_vapor`.
///
/// `surface_layer` (`WLayer`) and `ponding_threshold` (`hCritS`) are carried only so
/// a request for them is rejected explicitly; the vapor switch does not model them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaporAtmosphericCoupling {
    /// Length conversion `xConv` used by the `SetBC` rain reset head (`TIME.FOR:133-141`).
    pub x_conv: f64,
    pub surface_layer: bool,
    pub ponding_threshold: Option<f64>,
}

impl VaporAtmosphericCoupling {
    /// Bare-soil coupling without a surface water layer.
    pub fn new(x_conv: f64) -> Result<Self, WaterStepError> {
        if !x_conv.is_finite() || x_conv <= 0.0 {
            return Err(WaterStepError::InvalidForcing {
                name: "x_conv",
                reason: "must be finite and positive",
            });
        }
        Ok(Self {
            x_conv,
            surface_layer: false,
            ponding_threshold: None,
        })
    }
}

/// A forcing discontinuity and the rates active from that time onward.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForcingEvent {
    pub time: f64,
    pub rates: AtmosphericRates,
}

/// Validated in-memory forcing table; event times must be strictly increasing.
#[derive(Clone, Debug, PartialEq)]
pub struct ForcingSchedule<'a> {
    pub(crate) initial: AtmosphericRates,
    pub(crate) events: Vec<ForcingEvent>,
    pub(crate) units: ForcingUnits<'a>,
}

impl<'a> ForcingSchedule<'a> {
    pub fn new(
        initial: AtmosphericRates,
        events: Vec<ForcingEvent>,
        units: ForcingUnits<'a>,
    ) -> Result<Self, WaterStepError> {
        if units.time.trim().is_empty() || units.water_depth_rate.trim().is_empty() {
            return Err(WaterStepError::InvalidForcing {
                name: "units",
                reason: "time and water-depth-rate labels must be non-empty",
            });
        }
        validate_rates(initial)?;
        let mut previous = f64::NEG_INFINITY;
        for event in &events {
            if !event.time.is_finite() {
                return Err(WaterStepError::NonFiniteInput {
                    name: "forcing_event_time",
                });
            }
            if event.time <= previous {
                return Err(WaterStepError::InvalidForcing {
                    name: "event_times",
                    reason: "must be strictly increasing",
                });
            }
            validate_rates(event.rates)?;
            previous = event.time;
        }
        Ok(Self {
            initial,
            events,
            units,
        })
    }

    pub fn units(&self) -> ForcingUnits<'a> {
        self.units
    }

    pub fn events(&self) -> &[ForcingEvent] {
        &self.events
    }
}

fn validate_rates(rates: AtmosphericRates) -> Result<(), WaterStepError> {
    for (name, value) in [
        ("precipitation", rates.precipitation),
        ("evaporation", rates.evaporation),
    ] {
        if !value.is_finite() {
            return Err(WaterStepError::NonFiniteInput { name });
        }
        if value < 0.0 {
            return Err(WaterStepError::InvalidForcing {
                name,
                reason: "rates must be non-negative",
            });
        }
    }
    Ok(())
}

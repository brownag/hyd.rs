//! Met-driven atmospheric forcing: `Meteo.in` records plus `ATMOSPH.IN` times and
//! precipitation become per-record rates and surface temperatures.
//!
//! The arithmetic follows `SetMeteo` in source order (ADR 0010, 0015): `rSoil = EvapP*rConv/TTConv`
//! with `rConv = 0.001*xConv` and `TTConv = 86400*tConv`, and the Dirichlet top temperature
//! `(TMax+TMin)/2` in single precision.

use core::fmt;

use super::{
    calculate_bare_soil_et, EvaporationInput, MeteoConfig, MeteoError, MeteoRecord, MeteoSite,
    RadiationDailyInput, RadiationMode, RelativeHumidityMode, SunshineMode,
};
use crate::error::WaterStepError;
use crate::forcing::{AtmosphericRates, ForcingEvent, ForcingSchedule, ForcingUnits};

/// The `ATMOSPH.IN` columns the met path reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphericRecord {
    /// `tAtm`: time at the end of the record interval (model time units).
    pub t_atm: f64,
    /// `Prec`: precipitation; the sign is dropped (absolute value) as in the source path.
    pub precipitation: f64,
}

/// Errors raised while building met forcing.
#[derive(Clone, Debug, PartialEq)]
pub enum MetForcingError {
    /// A length/time conversion factor was not finite and positive.
    InvalidConversion { name: &'static str, value: f64 },
    /// A record endpoint is not finite or strictly greater than its predecessor.
    InvalidRecordTime { index: usize, time: f64 },
    /// The two decks have different record counts.
    RecordCountMismatch { meteo: usize, atmosphere: usize },
    /// `tAtm` differs between the decks at a record.
    TAtmMismatch {
        index: usize,
        meteo: f64,
        atmosphere: f64,
    },
    /// Bare-soil evaporation failed for a record.
    Evaporation { index: usize, source: MeteoError },
    /// The forcing schedule rejected the built rates.
    Schedule(WaterStepError),
    /// Both decks are empty.
    NoRecords,
}

impl fmt::Display for MetForcingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConversion { name, value } => {
                write!(f, "{name} must be finite and positive, got {value}")
            }
            Self::InvalidRecordTime { index, time } => write!(
                f,
                "record time at index {index} must be finite and strictly increasing, got {time}"
            ),
            Self::RecordCountMismatch { meteo, atmosphere } => write!(
                f,
                "record count mismatch: Meteo.in has {meteo}, ATMOSPH.IN has {atmosphere}"
            ),
            Self::TAtmMismatch {
                index,
                meteo,
                atmosphere,
            } => write!(
                f,
                "tAtm must agree in both decks: record {index} has {meteo} vs {atmosphere}"
            ),
            Self::Evaporation { index, source } => {
                write!(f, "evaporation failed at record {index}: {source}")
            }
            Self::Schedule(e) => write!(f, "forcing schedule rejected: {e:?}"),
            Self::NoRecords => write!(f, "no meteo records"),
        }
    }
}

impl std::error::Error for MetForcingError {}

/// Per-record forcing for a `Meteo.in` run.
///
/// Evaporation uses fixed modes: `SolarRadiationInput`, `SunshineHours` and
/// `RelativeHumidityPercent`. These are the only modes evidenced (Gate A bare soil, daily
/// records); `MeteoConfig::i_sun_sh` and `i_rel_hum` are not consulted.
#[derive(Clone, Debug, PartialEq)]
pub struct MetForcing {
    records: Vec<(f64, AtmosphericRates)>,
    top_temperatures: Vec<f64>,
}

impl MetForcing {
    /// Build rates (cm/h when `x_conv`/`t_conv` convert cm and h) and top temperatures.
    pub fn new(
        config: &MeteoConfig,
        meteo: &[MeteoRecord],
        atmosphere: &[AtmosphericRecord],
        x_conv: f64,
        t_conv: f64,
    ) -> Result<Self, MetForcingError> {
        if meteo.len() != atmosphere.len() {
            return Err(MetForcingError::RecordCountMismatch {
                meteo: meteo.len(),
                atmosphere: atmosphere.len(),
            });
        }
        if meteo.is_empty() {
            return Err(MetForcingError::NoRecords);
        }
        for (name, value) in [("x_conv", x_conv), ("t_conv", t_conv)] {
            if !value.is_finite() || value <= 0.0 {
                return Err(MetForcingError::InvalidConversion { name, value });
            }
        }
        let mut previous_time = None;
        for (index, record) in atmosphere.iter().enumerate() {
            if !record.t_atm.is_finite()
                || previous_time.is_some_and(|previous| record.t_atm <= previous)
            {
                return Err(MetForcingError::InvalidRecordTime {
                    index,
                    time: record.t_atm,
                });
            }
            previous_time = Some(record.t_atm);
        }
        // SetMeteo conversion: rSoil = EvapP*rConv/TTConv, rConv = 0.001*xConv, TTConv = 86400*tConv.
        let r_conv = 0.001 * x_conv;
        let tt_conv = 86400.0 * t_conv;
        let site = MeteoSite {
            latitude: config.latitude,
            altitude: config.altitude,
            wind_height: config.wind_height,
            temp_height: config.temp_height,
        };
        let mut records = Vec::with_capacity(meteo.len());
        let mut top_temperatures = Vec::with_capacity(meteo.len());
        for (index, (a, m)) in atmosphere.iter().zip(meteo).enumerate() {
            if !m.t_atm_h.is_finite() || m.t_atm_h != a.t_atm as f32 {
                return Err(MetForcingError::TAtmMismatch {
                    index,
                    meteo: f64::from(m.t_atm_h),
                    atmosphere: a.t_atm,
                });
            }
            let result = calculate_bare_soil_et(
                &site,
                &EvaporationInput {
                    radiation: RadiationDailyInput {
                        day_no: (a.t_atm / tt_conv) as f32,
                        radiation_input: m.rad,
                        t_max: m.t_max,
                        t_min: m.t_min,
                        rh_input: m.rh_mean,
                        sun_hours: m.sun_hours,
                        radiation_mode: RadiationMode::SolarRadiationInput,
                        sunshine_mode: SunshineMode::SunshineHours,
                        rel_hum_mode: RelativeHumidityMode::RelativeHumidityPercent,
                    },
                    wind_km_d: m.wind_km_d,
                },
                &config.radiation,
            )
            .map_err(|source| MetForcingError::Evaporation { index, source })?;
            records.push((
                a.t_atm,
                AtmosphericRates {
                    precipitation: a.precipitation.abs(),
                    evaporation: f64::from(result.evap_p) * r_conv / tt_conv,
                },
            ));
            // `SetMeteo` receives tTop in its TAver slot: single-precision (TMax+TMin)/2.
            top_temperatures.push(f64::from((m.t_max + m.t_min) / 2.0_f32));
        }
        Ok(Self {
            records,
            top_temperatures,
        })
    }

    /// Per-record `(tAtm, rates)`.
    pub fn records(&self) -> &[(f64, AtmosphericRates)] {
        &self.records
    }

    /// Per-record Dirichlet top temperature, held over the record's interval.
    pub fn top_temperatures(&self) -> &[f64] {
        &self.top_temperatures
    }

    /// Forcing schedule; each record applies to the interval ending at its `tAtm`.
    pub fn schedule<'a>(
        &self,
        units: ForcingUnits<'a>,
    ) -> Result<ForcingSchedule<'a>, MetForcingError> {
        let events: Vec<ForcingEvent> = self
            .records
            .windows(2)
            .map(|w| ForcingEvent {
                time: w[0].0,
                rates: w[1].1,
            })
            .collect();
        ForcingSchedule::new(self.records[0].1, events, units).map_err(MetForcingError::Schedule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteo::parse_meteo_deck;
    use std::{fs, path::PathBuf};

    const X_CONV: f64 = 100.0;
    const T_CONV: f64 = 1.0 / 3600.0;
    const UNITS: ForcingUnits<'static> = ForcingUnits {
        time: "h",
        water_depth_rate: "cm/h",
    };

    fn case(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../reference/vapor_fortran/cases/vapor_met_bare_soil_on")
            .join(name);
        fs::read_to_string(p).unwrap()
    }

    fn nums(line: &str) -> Vec<f64> {
        line.split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect()
    }

    fn decks() -> (MeteoConfig, Vec<MeteoRecord>, Vec<[f64; 11]>) {
        let (c, m) = parse_meteo_deck(&case("Meteo.in"), "Meteo.in").unwrap();
        let text = case("ATMOSPH.IN");
        let lines: Vec<&str> = text.lines().collect();
        let first = lines
            .iter()
            .position(|l| l.trim_start().starts_with("tAtm"))
            .unwrap()
            + 1;
        let mut atm = Vec::new();
        for line in &lines[first..] {
            let v = nums(line);
            if v.len() != 11 {
                break;
            }
            atm.push(<[f64; 11]>::try_from(v).unwrap());
        }
        (c, m, atm)
    }

    fn records(atm: &[[f64; 11]]) -> Vec<AtmosphericRecord> {
        atm.iter()
            .map(|a| AtmosphericRecord {
                t_atm: a[0],
                precipitation: a[1],
            })
            .collect()
    }

    #[test]
    fn matches_driver_arithmetic_exactly() {
        let (config, meteo, atm) = decks();
        assert_eq!(atm.len(), meteo.len());
        let forcing = MetForcing::new(&config, &meteo, &records(&atm), X_CONV, T_CONV).unwrap();

        // Oracle: verbatim copy of tests/support/vapor_met.rs:349-395 and :448-452.
        let r_conv = 0.001 * X_CONV;
        let tt_conv = 86400.0 * T_CONV;
        let site = MeteoSite {
            latitude: config.latitude,
            altitude: config.altitude,
            wind_height: config.wind_height,
            temp_height: config.temp_height,
        };
        assert_eq!(forcing.records().len(), atm.len());
        for (i, (a, m)) in atm.iter().zip(&meteo).enumerate() {
            let result = calculate_bare_soil_et(
                &site,
                &EvaporationInput {
                    radiation: RadiationDailyInput {
                        day_no: (a[0] / tt_conv) as f32,
                        radiation_input: m.rad,
                        t_max: m.t_max,
                        t_min: m.t_min,
                        rh_input: m.rh_mean,
                        sun_hours: m.sun_hours,
                        radiation_mode: RadiationMode::SolarRadiationInput,
                        sunshine_mode: SunshineMode::SunshineHours,
                        rel_hum_mode: RelativeHumidityMode::RelativeHumidityPercent,
                    },
                    wind_km_d: m.wind_km_d,
                },
                &config.radiation,
            )
            .unwrap();
            let want = AtmosphericRates {
                precipitation: a[1].abs(),
                evaporation: f64::from(result.evap_p) * r_conv / tt_conv,
            };
            let (t, got) = forcing.records()[i];
            assert_eq!(t.to_bits(), a[0].to_bits());
            assert_eq!(got.precipitation.to_bits(), want.precipitation.to_bits());
            assert_eq!(got.evaporation.to_bits(), want.evaporation.to_bits());
            let top = f64::from((m.t_max + m.t_min) / 2.0_f32);
            assert_eq!(forcing.top_temperatures()[i].to_bits(), top.to_bits());
        }
    }

    #[test]
    fn schedule_applies_each_record_to_interval_ending_at_its_t_atm() {
        let (config, meteo, atm) = decks();
        let forcing = MetForcing::new(&config, &meteo, &records(&atm), X_CONV, T_CONV).unwrap();
        let schedule = forcing.schedule(UNITS).unwrap();
        let recs = forcing.records();
        assert_eq!(schedule.initial, recs[0].1);
        assert_eq!(schedule.events.len(), recs.len() - 1);
        for (i, e) in schedule.events.iter().enumerate() {
            assert_eq!(e.time, recs[i].0);
            assert_eq!(e.rates, recs[i + 1].1);
        }
    }

    #[test]
    fn count_mismatch_is_typed() {
        let (config, meteo, atm) = decks();
        let mut a = records(&atm);
        a.pop();
        let n = meteo.len();
        assert_eq!(
            MetForcing::new(&config, &meteo, &a, X_CONV, T_CONV),
            Err(MetForcingError::RecordCountMismatch {
                meteo: n,
                atmosphere: n - 1
            })
        );
    }

    #[test]
    fn t_atm_mismatch_is_typed() {
        let (config, meteo, atm) = decks();
        let mut a = records(&atm);
        a[2].t_atm += 1.0;
        match MetForcing::new(&config, &meteo, &a, X_CONV, T_CONV) {
            Err(MetForcingError::TAtmMismatch { index: 2, .. }) => {}
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn decimal_t_atm_endpoints_pair_across_f32_and_f64() {
        let (config, mut meteo, atm) = decks();
        meteo.truncate(3);
        let mut a = records(&atm[..3]);
        for (i, t) in [0.1_f64, 0.2_f64, 0.3_f64].into_iter().enumerate() {
            meteo[i].t_atm_h = t as f32;
            a[i].t_atm = t;
        }
        let forcing = MetForcing::new(&config, &meteo, &a, X_CONV, T_CONV).unwrap();
        assert_eq!(forcing.records().len(), 3);
        assert_eq!(forcing.records()[0].0, 0.1_f64);
        assert_eq!(forcing.records()[1].0, 0.2_f64);
        assert_eq!(forcing.records()[2].0, 0.3_f64);

        // Genuinely different decimal endpoint is still rejected.
        a[1].t_atm = 0.2001_f64;
        assert!(matches!(
            MetForcing::new(&config, &meteo, &a, X_CONV, T_CONV),
            Err(MetForcingError::TAtmMismatch { index: 1, .. })
        ));
    }

    #[test]
    fn conversion_factors_must_be_finite_and_positive() {
        let (config, meteo, atm) = decks();
        for (name, value) in [
            ("x_conv", 0.0),
            ("x_conv", f64::NAN),
            ("x_conv", f64::INFINITY),
            ("t_conv", -1.0),
            ("t_conv", f64::NAN),
            ("t_conv", f64::INFINITY),
        ] {
            let (x_conv, t_conv) = if name == "x_conv" {
                (value, T_CONV)
            } else {
                (X_CONV, value)
            };
            assert!(matches!(
                MetForcing::new(&config, &meteo, &records(&atm), x_conv, t_conv),
                Err(MetForcingError::InvalidConversion {
                    name: actual_name,
                    value: actual_value
                }) if actual_name == name && actual_value.to_bits() == value.to_bits()
            ));
        }
    }

    #[test]
    fn all_record_times_must_be_finite_and_strictly_increasing() {
        let (config, meteo, atm) = decks();
        for (index, time) in [
            (0, f64::NAN),
            (0, f64::INFINITY),
            (1, 24.0),
            (atm.len() - 1, atm[atm.len() - 2][0]),
        ] {
            let mut a = records(&atm);
            a[index].t_atm = time;
            assert!(matches!(
                MetForcing::new(&config, &meteo, &a, X_CONV, T_CONV),
                Err(MetForcingError::InvalidRecordTime {
                    index: actual_index,
                    time: actual_time
                }) if actual_index == index && actual_time.to_bits() == time.to_bits()
            ));
        }
    }

    #[test]
    fn evaporation_failure_is_typed() {
        let (config, mut meteo, atm) = decks();
        meteo[1].wind_km_d = -1.0;
        match MetForcing::new(&config, &meteo, &records(&atm), X_CONV, T_CONV) {
            Err(MetForcingError::Evaporation { index: 1, .. }) => {}
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn empty_input_is_typed() {
        let (config, _, _) = decks();
        assert_eq!(
            MetForcing::new(&config, &[], &[], X_CONV, T_CONV),
            Err(MetForcingError::NoRecords)
        );
    }
}

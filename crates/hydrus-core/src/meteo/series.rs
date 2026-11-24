//! `Meteo.in` daily record reader (`SetMeteo`, `TIME.FOR:484-497`).
//!
//! Each record is `tAtm Rad TMax TMin RHMean Wind_kmd SunHours`, list-directed, and
//! applies to the interval ending at `tAtm` [h]. The block ends at a line starting
//! with `end`. Records are validated before the series is returned.

use super::config::{deck_err, parse_f32, tokens, MeteoConfig};
use super::MeteoError;

/// One meteorological record (`TIME.FOR:497`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeteoRecord {
    /// Time at the end of the record interval [h].
    pub t_atm_h: f32,
    /// Solar radiation [MJ/m2/d].
    pub rad: f32,
    /// Maximum temperature [C].
    pub t_max: f32,
    /// Minimum temperature [C].
    pub t_min: f32,
    /// Mean relative humidity [%].
    pub rh_mean: f32,
    /// Wind run [km/d].
    pub wind_km_d: f32,
    /// Bright sunshine duration [h].
    pub sun_hours: f32,
}

impl MeteoRecord {
    /// Reject physically invalid values (contract section 7).
    pub fn validate(&self) -> Result<(), MeteoError> {
        if self.t_min > self.t_max {
            return Err(MeteoError::InvalidTemperatureRange {
                t_min: self.t_min,
                t_max: self.t_max,
            });
        }
        if !(0.0..=100.0).contains(&self.rh_mean) {
            return Err(MeteoError::RelativeHumidityOutOfRange { rh: self.rh_mean });
        }
        if self.wind_km_d < 0.0 {
            return Err(MeteoError::NegativeWindSpeed {
                wind: self.wind_km_d,
            });
        }
        if self.rad < 0.0 {
            return Err(MeteoError::NegativeRadiation {
                radiation: self.rad,
            });
        }
        if self.sun_hours < 0.0 {
            return Err(MeteoError::NegativeSunHours {
                sun_hours: self.sun_hours,
            });
        }
        Ok(())
    }
}

/// Parse records starting at line index `first_line` until the `end` line.
pub fn parse_records(
    text: &str,
    first_line: usize,
    path: &str,
) -> Result<Vec<MeteoRecord>, MeteoError> {
    let mut out: Vec<MeteoRecord> = Vec::new();
    let mut terminated = false;
    for (i, line) in text.lines().enumerate().skip(first_line) {
        if line.trim_start().starts_with("end") {
            terminated = true;
            break;
        }
        let t = tokens(line);
        if t.len() < 7 {
            return Err(deck_err(
                path,
                format!("line {}: expected 7 values, found {}", i + 1, t.len()),
            ));
        }
        let f = |k: usize, n: &str| parse_f32(t[k], n, path);
        let rec = MeteoRecord {
            t_atm_h: f(0, "tAtm")?,
            rad: f(1, "Rad")?,
            t_max: f(2, "TMax")?,
            t_min: f(3, "TMin")?,
            rh_mean: f(4, "RHMean")?,
            wind_km_d: f(5, "Wind_kmd")?,
            sun_hours: f(6, "SunHours")?,
        };
        if let Some(prev) = out.last() {
            if rec.t_atm_h <= prev.t_atm_h {
                return Err(deck_err(
                    path,
                    format!(
                        "line {}: tAtm must increase ({} after {})",
                        i + 1,
                        rec.t_atm_h,
                        prev.t_atm_h
                    ),
                ));
            }
        }
        out.push(rec);
    }
    if !terminated {
        return Err(deck_err(path, "missing 'end' line after meteo records"));
    }
    if out.is_empty() {
        return Err(deck_err(path, "no meteo records"));
    }
    for r in &out {
        r.validate()?;
    }
    Ok(out)
}

/// Parse a whole `Meteo.in`: configuration, then records. Nothing is returned on error.
pub fn parse_meteo_deck(
    text: &str,
    path: &str,
) -> Result<(MeteoConfig, Vec<MeteoRecord>), MeteoError> {
    let (config, next) = MeteoConfig::parse(text, path)?;
    let records = parse_records(text, next, path)?;
    Ok((config, records))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECK: &str =
        include_str!("../../../../reference/vapor_fortran/cases/vapor_met_bare_soil_on/Meteo.in");
    const P: &str = "Meteo.in";

    fn with(from: &str, to: &str) -> String {
        assert!(DECK.contains(from), "pattern {from:?} not in deck");
        DECK.replacen(from, to, 1)
    }

    #[test]
    fn parses_all_ten_records() {
        let (cfg, r) = parse_meteo_deck(DECK, P).unwrap();
        assert_eq!(cfg.max_al, 10);
        assert_eq!(r.len(), 10);
        for (i, rec) in r.iter().enumerate() {
            assert_eq!(rec.t_atm_h, 24.0 * (i as f32 + 1.0));
        }
        assert_eq!(
            r[0],
            MeteoRecord {
                t_atm_h: 24.0,
                rad: 24.0,
                t_max: 34.0,
                t_min: 20.0,
                rh_mean: 35.0,
                wind_km_d: 250.0,
                sun_hours: 11.0
            }
        );
        assert_eq!(
            (r[5].rad, r[5].rh_mean, r[5].wind_km_d, r[5].sun_hours),
            (10.0, 85.0, 150.0, 2.0)
        );
        assert_eq!((r[9].t_atm_h, r[9].t_max, r[9].t_min), (240.0, 35.0, 21.0));
    }

    #[test]
    fn accepts_comma_separated_record() {
        let d = with(
            "24 24.0 34.0 20.0 35.0 250.0 11.0",
            "24, 24.0, 34.0, 20.0, 35.0, 250.0, 11.0",
        );
        assert_eq!(parse_meteo_deck(&d, P).unwrap().1[0].t_max, 34.0);
    }

    #[test]
    fn rejects_inverted_temperature_range() {
        let d = with("24 24.0 34.0 20.0", "24 24.0 20.0 34.0");
        assert_eq!(
            parse_meteo_deck(&d, P).unwrap_err(),
            MeteoError::InvalidTemperatureRange {
                t_min: 34.0,
                t_max: 20.0
            }
        );
    }

    #[test]
    fn rejects_relative_humidity_out_of_range() {
        let d = with("20.0 35.0 250.0 11.0", "20.0 101.0 250.0 11.0");
        assert_eq!(
            parse_meteo_deck(&d, P).unwrap_err(),
            MeteoError::RelativeHumidityOutOfRange { rh: 101.0 }
        );
    }

    #[test]
    fn rejects_negative_wind() {
        let d = with("35.0 250.0 11.0", "35.0 -1.0 11.0");
        assert_eq!(
            parse_meteo_deck(&d, P).unwrap_err(),
            MeteoError::NegativeWindSpeed { wind: -1.0 }
        );
    }

    #[test]
    fn rejects_negative_radiation() {
        let d = with("24 24.0 34.0", "24 -2.0 34.0");
        assert_eq!(
            parse_meteo_deck(&d, P).unwrap_err(),
            MeteoError::NegativeRadiation { radiation: -2.0 }
        );
    }

    #[test]
    fn rejects_malformed_records() {
        let is_deck_err = |d: &str| {
            matches!(
                parse_meteo_deck(d, P),
                Err(MeteoError::InvalidMeteoDeck { .. })
            )
        };
        assert!(is_deck_err(&with(
            "48 25.0 35.0 21.0 32.0 250.0 11.5",
            "48 25.0 35.0"
        )));
        assert!(is_deck_err(&DECK.replace("\nend\n", "\n")));
        assert!(is_deck_err(&with("48 25.0", "24 25.0")));
    }
}

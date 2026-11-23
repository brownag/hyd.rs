//! `Meteo.in` site and configuration reader (`MeteoIn`, `INPUT.FOR:934-1066`).
//!
//! Scope is the Gate A selection: solar radiation input (`iRadiation=1`), bare soil
//! (`iCrop=0`) and daily records (`lMetDaily=f`). Other selections are rejected with a
//! typed error. Parsing mimics Fortran list-directed `read(33,*)`: values are
//! whitespace or comma separated, and each data line is preceded by one skipped
//! header line. The module is filesystem-free and hand-written (ADR 0011).

use super::radiation::parse_latitude_deg_min;
use super::{MeteoError, RadiationMode, RadiationParams};

/// Crop height used for bare soil [cm] (contract section 2, `CropHeight`).
pub const BARE_SOIL_CROP_HEIGHT_CM: f32 = 0.1;

/// Maximum rows in the crop growth table (`INPUT.FOR:999`).
const MAX_GROWTH_ROWS: i32 = 1000;

/// Site and configuration data from `Meteo.in` (Gate A subset).
#[derive(Clone, Debug, PartialEq)]
pub struct MeteoConfig {
    /// `MaxAL`: maximum number of meteo records.
    pub max_al: i32,
    /// `iRadiation` (1 = measured solar radiation).
    pub i_radiation: i32,
    /// `lHargr`: Hargreaves flag.
    pub l_hargr: bool,
    /// `lEnBal` (always false once accepted).
    pub l_en_bal: bool,
    /// `lMetDaily` (always false once accepted).
    pub l_met_daily: bool,
    /// Latitude in degrees.minutes decimal form [deg].
    pub latitude: f32,
    /// Altitude above sea level [m].
    pub altitude: f32,
    /// Angstrom, longwave and albedo coefficients.
    pub radiation: RadiationParams,
    /// Wind measurement height [cm].
    pub wind_height: f32,
    /// Temperature measurement height [cm].
    pub temp_height: f32,
    /// `iCrop` (always 0 once accepted).
    pub i_crop: i32,
    /// `iSunSh` (always 0 once accepted).
    pub i_sun_sh: i32,
    /// `iRelHum` (always 0 once accepted).
    pub i_rel_hum: i32,
    /// Crop height [cm]; bare-soil default.
    pub crop_height: f32,
}

pub(crate) fn deck_err(path: &str, reason: impl Into<String>) -> MeteoError {
    MeteoError::InvalidMeteoDeck {
        path: path.to_string(),
        reason: reason.into(),
    }
}

/// List-directed tokens: whitespace and/or commas separate values.
pub(crate) fn tokens(line: &str) -> Vec<&str> {
    line.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .collect()
}

pub(crate) fn parse_f32(tok: &str, name: &str, path: &str) -> Result<f32, MeteoError> {
    let norm = tok.replace(['d', 'D'], "e");
    match norm.parse::<f32>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => Err(deck_err(
            path,
            format!("cannot read real {name} from '{tok}'"),
        )),
    }
}

fn parse_i32(tok: &str, name: &str, path: &str) -> Result<i32, MeteoError> {
    tok.parse::<i32>()
        .map_err(|_| deck_err(path, format!("cannot read integer {name} from '{tok}'")))
}

fn parse_bool(tok: &str, name: &str, path: &str) -> Result<bool, MeteoError> {
    let t = tok.trim_start_matches('.').to_ascii_lowercase();
    if t.starts_with('t') {
        Ok(true)
    } else if t.starts_with('f') {
        Ok(false)
    } else {
        Err(deck_err(
            path,
            format!("cannot read logical {name} from '{tok}'"),
        ))
    }
}

/// Line cursor that reproduces `read(33,*)` header skips and data reads.
struct Cursor<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    path: &'a str,
}

impl<'a> Cursor<'a> {
    fn next_line(&mut self, what: &str) -> Result<&'a str, MeteoError> {
        let line = self
            .lines
            .get(self.pos)
            .copied()
            .ok_or_else(|| deck_err(self.path, format!("truncated deck before {what}")))?;
        self.pos += 1;
        Ok(line)
    }

    /// Skip one header line, then return the tokens of the data line (at least `n`).
    fn data(&mut self, what: &str, n: usize) -> Result<Vec<&'a str>, MeteoError> {
        self.next_line(what)?;
        let line = self.next_line(what)?;
        let t = tokens(line);
        if t.len() < n {
            return Err(deck_err(
                self.path,
                format!("{what}: expected {n} values, found {}", t.len()),
            ));
        }
        Ok(t)
    }
}

/// Reject heights that violate the zero-plane displacement check (`TIME.FOR:639`, Error 33).
pub fn validate_heights(
    crop_height: f32,
    wind_height: f32,
    temp_height: f32,
) -> Result<(), MeteoError> {
    let d = 0.667 * crop_height;
    if d >= wind_height || d >= temp_height {
        return Err(MeteoError::CropHeightExceedsMeasurementHeight {
            crop_height,
            wind_height,
            temp_height,
        });
    }
    Ok(())
}

impl MeteoConfig {
    /// Parse the configuration block of `Meteo.in`.
    ///
    /// Returns the config and the index of the first meteo record line.
    pub fn parse(text: &str, path: &str) -> Result<(Self, usize), MeteoError> {
        let mut c = Cursor {
            lines: text.lines().collect(),
            pos: 0,
            path,
        };
        let ver_line = c.next_line("file version")?;
        let version = ver_line
            .trim()
            .strip_prefix("Pcp_File_Version=")
            .and_then(|v| v.trim().parse::<i32>().ok())
            .ok_or_else(|| deck_err(path, "missing Pcp_File_Version line"))?;
        if version < 4 {
            return Err(deck_err(
                path,
                format!("file version {version} < 4 unsupported"),
            ));
        }

        // Block A banner is one skipped line; `data` skips the column header.
        c.next_line("block A banner")?;
        let t = c.data("MaxAL iRadiation lHargr", 3)?;
        let max_al = parse_i32(t[0], "MaxAL", path)?;
        let i_radiation = parse_i32(t[1], "iRadiation", path)?;
        let l_hargr = parse_bool(t[2], "lHargr", path)?;
        RadiationMode::try_from(i_radiation)?;
        if i_radiation != 1 {
            return Err(MeteoError::UnsupportedMeteoSelection {
                field: "iRadiation",
                value: i_radiation,
            });
        }

        let t = c.data("lEnBal lMetDaily", 2)?;
        let l_en_bal = parse_bool(t[0], "lEnBal", path)?;
        let l_met_daily = parse_bool(t[1], "lMetDaily", path)?;
        if l_en_bal {
            return Err(MeteoError::SurfaceEnergyBalanceUnsupported);
        }
        if l_met_daily {
            return Err(MeteoError::UnsupportedMeteoSelection {
                field: "lMetDaily",
                value: 1,
            });
        }

        let t = c.data("Latitude Altitude", 2)?;
        let latitude = parse_f32(t[0], "Latitude", path)?;
        let altitude = parse_f32(t[1], "Altitude", path)?;
        parse_latitude_deg_min(latitude)?;
        let t = c.data("ShortWaveRadA/B", 2)?;
        let a_s = parse_f32(t[0], "ShortWaveRadA", path)?;
        let b_s = parse_f32(t[1], "ShortWaveRadB", path)?;
        let t = c.data("LongWaveRadA/B", 2)?;
        let a_c = parse_f32(t[0], "LongWaveRadA", path)?;
        let b_c = parse_f32(t[1], "LongWaveRadB", path)?;
        let t = c.data("LongWaveRadA1/B1", 2)?;
        let a1 = parse_f32(t[0], "LongWaveRadA1", path)?;
        let b1 = parse_f32(t[1], "LongWaveRadB1", path)?;
        let t = c.data("WindHeight TempHeight", 2)?;
        let wind_height = parse_f32(t[0], "WindHeight", path)?;
        let temp_height = parse_f32(t[1], "TempHeight", path)?;

        let t = c.data("iCrop iSunSh iRelHum", 3)?;
        let i_crop = parse_i32(t[0], "iCrop", path)?;
        let i_sun_sh = parse_i32(t[1], "iSunSh", path)?;
        let i_rel_hum = parse_i32(t[2], "iRelHum", path)?;
        if i_crop == 2 {
            // iLAI rExtinct, iInterc, iGrowth (`INPUT.FOR:991-1003`). Only the table
            // limit is checked; the crop tables themselves are out of scope.
            c.data("iLAI rExtinct", 2)?;
            c.data("iInterc", 1)?;
            let t = c.data("iGrowth", 1)?;
            let count = parse_i32(t[0], "iGrowth", path)?;
            if count > MAX_GROWTH_ROWS {
                return Err(MeteoError::GrowthTableOverflow { count });
            }
        }
        for (field, value) in [
            ("iCrop", i_crop),
            ("iSunSh", i_sun_sh),
            ("iRelHum", i_rel_hum),
        ] {
            if value != 0 {
                return Err(MeteoError::UnsupportedMeteoSelection { field, value });
            }
        }

        let t = c.data("Albedo", 1)?;
        let albedo = parse_f32(t[0], "Albedo", path)?;
        if !(0.0..=1.0).contains(&albedo) {
            return Err(MeteoError::InvalidAlbedo { albedo });
        }
        // Fortran skips three lines: block B banner, column header, "records follow".
        for _ in 0..3 {
            c.next_line("record block banner")?;
        }
        validate_heights(BARE_SOIL_CROP_HEIGHT_CM, wind_height, temp_height)?;

        let radiation = RadiationParams {
            short_wave_rad_a: a_s,
            short_wave_rad_b: b_s,
            long_wave_rad_a: a_c,
            long_wave_rad_b: b_c,
            long_wave_rad_a1: a1,
            long_wave_rad_b1: b1,
            albedo,
            ..RadiationParams::default()
        };
        Ok((
            Self {
                max_al,
                i_radiation,
                l_hargr,
                l_en_bal,
                l_met_daily,
                latitude,
                altitude,
                radiation,
                wind_height,
                temp_height,
                i_crop,
                i_sun_sh,
                i_rel_hum,
                crop_height: BARE_SOIL_CROP_HEIGHT_CM,
            },
            c.pos,
        ))
    }
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
    fn parses_committed_deck() {
        let (c, next) = MeteoConfig::parse(DECK, P).unwrap();
        assert_eq!((c.max_al, c.i_radiation, c.l_hargr), (10, 1, false));
        assert!(!c.l_en_bal && !c.l_met_daily);
        assert_eq!((c.latitude, c.altitude), (-30.0, 100.0));
        assert_eq!(
            (c.radiation.short_wave_rad_a, c.radiation.short_wave_rad_b),
            (0.25, 0.5)
        );
        assert_eq!(
            (c.radiation.long_wave_rad_a, c.radiation.long_wave_rad_b),
            (0.9, 0.1)
        );
        assert_eq!(
            (c.radiation.long_wave_rad_a1, c.radiation.long_wave_rad_b1),
            (0.34, -0.139)
        );
        assert_eq!((c.wind_height, c.temp_height), (200.0, 190.0));
        assert_eq!((c.i_crop, c.i_sun_sh, c.i_rel_hum), (0, 0, 0));
        assert_eq!(c.radiation.albedo, 0.23);
        assert_eq!(c.crop_height, 0.1);
        assert!(DECK.lines().nth(next).unwrap().starts_with("24 "));
    }

    #[test]
    fn accepts_comma_separated_values() {
        let d = with("-30.0 100.0", "-30.0, 100.0");
        assert_eq!(MeteoConfig::parse(&d, P).unwrap().0.altitude, 100.0);
    }

    #[test]
    fn rejects_energy_balance() {
        let d = with("f f\nLatitude", "t f\nLatitude");
        assert_eq!(
            MeteoConfig::parse(&d, P).unwrap_err(),
            MeteoError::SurfaceEnergyBalanceUnsupported
        );
    }

    #[test]
    fn rejects_radiation_mode_outside_set() {
        let d = with("10 1 f", "10 7 f");
        assert_eq!(
            MeteoConfig::parse(&d, P).unwrap_err(),
            MeteoError::UnsupportedRadiationMode { mode: 7 }
        );
    }

    #[test]
    fn rejects_out_of_scope_selections() {
        let sel = |field, value| MeteoError::UnsupportedMeteoSelection { field, value };
        let d = with("10 1 f", "10 2 f");
        assert_eq!(MeteoConfig::parse(&d, P).unwrap_err(), sel("iRadiation", 2));
        let d = with("f f\nLatitude", "f t\nLatitude");
        assert_eq!(MeteoConfig::parse(&d, P).unwrap_err(), sel("lMetDaily", 1));
        let d = with("0 0 0", "0 1 0");
        assert_eq!(MeteoConfig::parse(&d, P).unwrap_err(), sel("iSunSh", 1));
        let d = with("0 0 0", "1 0 0");
        assert_eq!(MeteoConfig::parse(&d, P).unwrap_err(), sel("iCrop", 1));
    }

    #[test]
    fn rejects_growth_table_overflow() {
        let d = with(
            "0 0 0\nAlbedo",
            "2 0 0\niLAI rExtinct\n1 0.463\niInterc\n0\niGrowth\n1001\nAlbedo",
        );
        assert_eq!(
            MeteoConfig::parse(&d, P).unwrap_err(),
            MeteoError::GrowthTableOverflow { count: 1001 }
        );
    }

    #[test]
    fn rejects_crop_height_exceeding_measurement_height() {
        assert!(validate_heights(BARE_SOIL_CROP_HEIGHT_CM, 200.0, 190.0).is_ok());
        assert_eq!(
            validate_heights(300.0, 200.0, 190.0).unwrap_err(),
            MeteoError::CropHeightExceedsMeasurementHeight {
                crop_height: 300.0,
                wind_height: 200.0,
                temp_height: 190.0
            }
        );
        // A measurement height at or below the bare-soil displacement also trips it.
        let d = with("200. 190.", "0.05 190.");
        assert!(matches!(
            MeteoConfig::parse(&d, P).unwrap_err(),
            MeteoError::CropHeightExceedsMeasurementHeight { .. }
        ));
    }

    #[test]
    fn rejects_malformed_and_truncated_decks() {
        let d = with("-30.0 100.0", "abc 100.0");
        match MeteoConfig::parse(&d, "x/Meteo.in").unwrap_err() {
            MeteoError::InvalidMeteoDeck { path, reason } => {
                assert_eq!(path, "x/Meteo.in");
                assert!(reason.contains("Latitude"));
            }
            e => panic!("unexpected {e:?}"),
        }
        let trunc: String = DECK.lines().take(12).collect::<Vec<_>>().join("\n");
        assert!(matches!(
            MeteoConfig::parse(&trunc, P).unwrap_err(),
            MeteoError::InvalidMeteoDeck { .. }
        ));
        assert!(matches!(
            MeteoConfig::parse("", P).unwrap_err(),
            MeteoError::InvalidMeteoDeck { .. }
        ));
    }
}

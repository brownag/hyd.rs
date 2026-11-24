//! Net radiation modeling ($R_n, R_{ns}, R_{nl}$) ported from HYDRUS `TIME.FOR:519-583, 672-781`.
//!
//! # Provenance and Precision (ADR 0010 & ADR 0015)
//!
//! All calculations replicate the exact arithmetic operation order, call sequences,
//! and single-precision IEEE 754 float (`f32`) evaluation of the pinned reference implementation.
//! No intermediate promotion to `f64` is performed.

use super::{MeteoError, RadiationMode, RadiationParams, RelativeHumidityMode, SunshineMode};

/// Mathematical constant $\pi$ declared in `TIME.FOR:676, 702`.
/// Retains the exact literal from pinned Fortran (ADR 0010).
#[allow(clippy::approx_constant, clippy::excessive_precision)]
pub const PI: f32 = 3.141592654;

/// Solar constant $I_{sc} = 118.08\,\text{MJ}\,\text{m}^{-2}\,\text{d}^{-1}$ declared in `TIME.FOR:677`.
pub const SOLAR_CONSTANT: f32 = 118.08;

/// Stefan-Boltzmann radiation coefficient $2.45 \times 10^{-9}$ declared in `TIME.FOR:776`.
pub const SIGMA_COEFF: f32 = 0.00000000245;

/// Temperature offset to Kelvin in longwave radiation ($273.16\,\text{K}$) declared in `TIME.FOR:776`.
pub const TEMP_OFFSET: f32 = 273.16;

/// Extraterrestrial radiation components returned by `RadGlobal` (`TIME.FOR:672-689`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtraterrestrialRadiation {
    /// Extraterrestrial daily radiation $R_a$ [MJ/m2/d] (`TIME.FOR:686`).
    pub ra: f32,
    /// Sunset hour angle $\omega$ [rad] (`TIME.FOR:682`).
    pub omega: f32,
    /// Geometric component $x_x = \sin(\delta) \sin(\text{Lat}_2)$ (`TIME.FOR:683`).
    pub xx: f32,
    /// Geometric component $y_y = \cos(\delta) \cos(\text{Lat}_2)$ (`TIME.FOR:684`).
    pub yy: f32,
    /// Solar declination angle $\delta$ [rad] (`TIME.FOR:681`).
    pub sol_declin: f32,
    /// Relative earth-sun distance factor $d_r$ [-] (`TIME.FOR:685`).
    pub dr: f32,
}

/// Cloudiness factor and sunshine duration components returned by `Cloudiness` (`TIME.FOR:698-721`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloudinessResult {
    /// Cloudiness factor `CloudF` [-] (`TIME.FOR:707, 710, 717`).
    pub cloud_f: f32,
    /// Relative sunshine duration $n/N$ [-] (`TIME.FOR:706, 711, 716`).
    pub n_n: f32,
    /// Cloud cover fraction `Cover` [-] (`TIME.FOR:708, 712, 715`).
    pub cover: f32,
    /// Transmission coefficient $T_t$ [-] (`TIME.FOR:714`).
    pub tt: f32,
    /// Maximum daylight duration $N = \frac{24}{\pi} \omega$ [hr] (`TIME.FOR:705`).
    pub nn: f32,
}

/// Daily meteorological radiation input record for `calculate_net_radiation`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadiationDailyInput {
    /// Day of year (0..365).
    pub day_no: f32,
    /// Measured solar or net radiation [MJ/m2/d] (used in modes 1 and 2, or clear-sky).
    pub radiation_input: f32,
    /// Daily maximum temperature [°C].
    pub t_max: f32,
    /// Daily minimum temperature [°C].
    pub t_min: f32,
    /// Relative humidity [%] or vapor pressure [kPa] based on `rel_hum_mode`.
    pub rh_input: f32,
    /// Bright sunshine hours per day [hr] (or cloudiness fraction / transmission coeff).
    pub sun_hours: f32,
    /// Radiation calculation mode (`iRadiation`).
    pub radiation_mode: RadiationMode,
    /// Sunshine observation mode (`iSunSh`).
    pub sunshine_mode: SunshineMode,
    /// Relative humidity input mode (`iRelHum`).
    pub rel_hum_mode: RelativeHumidityMode,
}

/// Resulting net radiation balance components.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetRadiation {
    /// Total net surface radiation $R_n$ [MJ/m2/d] (`TIME.FOR:579, 581`).
    pub rn: f32,
    /// Net shortwave radiation $R_{ns}$ [MJ/m2/d] (`TIME.FOR:572`).
    pub rns: f32,
    /// Net longwave radiation $R_{nl}$ [MJ/m2/d] (`TIME.FOR:778`).
    pub rnl: f32,
    /// Global incoming shortwave solar radiation $R_{\text{ad}}$ [MJ/m2/d] (`TIME.FOR:543`).
    pub rad: f32,
    /// Extraterrestrial radiation $R_a$ [MJ/m2/d] (`TIME.FOR:686`).
    pub ra: f32,
    /// Cloudiness factor `CloudF` [-] (`TIME.FOR:707, 717, 568`).
    pub cloud_f: f32,
    /// Dew point vapor pressure $e_a^{\text{dew}}$ [kPa] (`TIME.FOR:526, 529`).
    pub ea_dew: f32,
}

/// Decodes latitude from legacy degrees-and-minutes decimal format (`TIME.FOR:679-680`).
///
/// In the Fortran input deck (`INPUT.FOR:971`), latitude is encoded as degrees plus minutes
/// expressed as decimals (e.g. `50.30` represents $50^\circ 30'$).
///
/// The conversion is:
/// ```text
/// Lat1 = (Latitude - aint(Latitude)) * (5. / 3.) + aint(Latitude)
/// Lat2 = Lat1 * PI / 180.
/// ```
///
/// Returns `(lat_decimal_degrees, lat_radians)`.
pub fn parse_latitude_deg_min(latitude: f32) -> Result<(f32, f32), MeteoError> {
    if !latitude.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "latitude" });
    }
    if !(-90.0..=90.0).contains(&latitude) {
        return Err(MeteoError::InvalidLatitude { latitude });
    }

    let lat_trunc = latitude.trunc();
    let lat_frac = latitude - lat_trunc;
    let lat_minutes = (lat_frac * 100.0).abs().round();
    if lat_minutes >= 60.0 {
        return Err(MeteoError::InvalidLatitude { latitude });
    }

    // Preserve TIME.FOR:679-680 operation sequence:
    let lat1 = lat_frac * (5.0 / 3.0) + lat_trunc;
    let lat2 = lat1 * PI / 180.0;

    Ok((lat1, lat2))
}

/// Evaluates extraterrestrial solar radiation $R_a$ and astronomical angles (`TIME.FOR:672-689`).
///
/// Follows `subroutine RadGlobal`:
/// ```text
/// PI=3.141592654
/// SC=118.08
/// Lat1=(Latitude-aint(Latitude))*(5./3.)+aint(Latitude)
/// Lat2=Lat1*PI/180.
/// SolDeclin=sin(2.*PI/365.*DayNo-1.39)*0.4093
/// Omega=acos(-tan(SolDeclin)*tan(Lat2))
/// xx=sin(SolDeclin)*sin(Lat2)
/// yy=cos(SolDeclin)*cos(Lat2)
/// dr=(1.+0.033*cos((2.*Pi/365.)*DayNo))
/// Ra=SC/PI*dr*(Omega*xx+sin(Omega)*yy)
/// ```
///
/// Note: Fortran evaluates `2.*PI/365.*DayNo` strictly left-to-right: `((2. * PI) / 365.) * DayNo`.
pub fn extraterrestrial_radiation(
    latitude: f32,
    day_no: f32,
) -> Result<ExtraterrestrialRadiation, MeteoError> {
    if !day_no.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "day_no" });
    }
    if day_no < 0.0 {
        return Err(MeteoError::InvalidDayOfYear { day_no });
    }

    let (_lat1, lat2) = parse_latitude_deg_min(latitude)?;

    // Preserve TIME.FOR:681 left-to-right evaluation: ((2. * PI) / 365.) * DayNo
    let sol_angle = ((2.0 * PI) / 365.0) * day_no;
    let sol_declin = (sol_angle - 1.39).sin() * 0.4093;

    // Preserve TIME.FOR:682
    let cos_omega = -sol_declin.tan() * lat2.tan();
    let omega = cos_omega.clamp(-1.0, 1.0).acos();

    // Preserve TIME.FOR:683-685
    let xx = sol_declin.sin() * lat2.sin();
    let yy = sol_declin.cos() * lat2.cos();
    let dr = 1.0 + 0.033 * (((2.0 * PI) / 365.0) * day_no).cos();

    // Preserve TIME.FOR:686: Ra = SC/PI*dr*(Omega*xx+sin(Omega)*yy)
    let ra = (SOLAR_CONSTANT / PI) * dr * (omega * xx + omega.sin() * yy);

    Ok(ExtraterrestrialRadiation {
        ra,
        omega,
        xx,
        yy,
        sol_declin,
        dr,
    })
}

/// Evaluates cloudiness factor and relative sunshine duration (`TIME.FOR:698-721`).
///
/// Follows `subroutine Cloudiness`:
/// - Mode 0 (sunshine hours):
///   `NN = 24. / PI * Omega`
///   `n_N = min(SunHours / NN, 1.)`
///   `CloudF = LongWaveRadB + LongWaveRadA * n_N`
///   `Cover = 1. - n_N`
/// - Mode 1 (cloudiness):
///   `CloudF = SunHours`
///   `n_N = (CloudF - LongWaveRadB) / LongWaveRadA`
///   `Cover = 1. - n_N`
/// - Mode 2 (transmission coeff):
///   `Tt = SunHours`
///   `Cover = max(0.0001, min(1., 2.330 - 3.330 * Tt))`
///   `n_N = 1. - Cover`
///   `CloudF = LongWaveRadB + LongWaveRadA * n_N`
pub fn cloudiness_factor(
    mode: SunshineMode,
    sun_hours: f32,
    omega: f32,
    params: &RadiationParams,
) -> Result<CloudinessResult, MeteoError> {
    if !sun_hours.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "sun_hours" });
    }
    if !omega.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "omega" });
    }
    if sun_hours < 0.0 {
        return Err(MeteoError::NegativeSunHours { sun_hours });
    }

    let nn = (24.0 / PI) * omega;

    let (cloud_f, n_n, cover, tt) = match mode {
        SunshineMode::SunshineHours => {
            let n_n = if nn > 0.0 {
                (sun_hours / nn).min(1.0)
            } else {
                0.0
            };
            let cloud_f = params.long_wave_rad_b + params.long_wave_rad_a * n_n;
            let cover = 1.0 - n_n;
            (cloud_f, n_n, cover, 0.0)
        }
        SunshineMode::Cloudiness => {
            let cloud_f = sun_hours;
            let n_n = (cloud_f - params.long_wave_rad_b) / params.long_wave_rad_a;
            let cover = 1.0 - n_n;
            (cloud_f, n_n, cover, 0.0)
        }
        SunshineMode::TransmissionCoeff => {
            let tt = sun_hours;
            let cover = 0.0001_f32.max((1.0_f32).min(2.330 - 3.330 * tt));
            let n_n = 1.0 - cover;
            let cloud_f = params.long_wave_rad_b + params.long_wave_rad_a * n_n;
            (cloud_f, n_n, cover, tt)
        }
        SunshineMode::ClearSkySolar => {
            // Updated dynamically in calculate_net_radiation when radiation is known
            let cloud_f = params.cloud_f_ac * 1.0 + params.cloud_f_bc;
            (cloud_f, 0.0, 0.0, 0.0)
        }
    };

    Ok(CloudinessResult {
        cloud_f,
        n_n,
        cover,
        tt,
        nn,
    })
}

/// Evaluates saturation vapor pressure $e_a(T)$ [kPa] via Tetens formula (`TIME.FOR:522-523`).
///
/// ```text
/// Ea(T) = 0.6108 * exp((17.27 * T) / (T + 237.3))
/// ```
#[inline]
pub fn saturation_vapor_pressure(temperature_c: f32) -> f32 {
    0.6108 * ((17.27 * temperature_c) / (temperature_c + 237.3)).exp()
}

/// Evaluates dew-point actual vapor pressure $e_a^{\text{dew}}$ [kPa] (`TIME.FOR:524-531`).
///
/// Follows `SetMeteo`:
/// - When `iRelHum = 0` (relative humidity %):
///   `EaDew = RHMean / (50. / Ea_TMin + 50. / Ea_TMax)`
/// - When `iRelHum = 1` (vapor pressure input):
///   `EaDew = RHMean`
pub fn dew_point_vapor_pressure(
    t_max: f32,
    t_min: f32,
    rh_input: f32,
    mode: RelativeHumidityMode,
) -> Result<f32, MeteoError> {
    if !t_max.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "t_max" });
    }
    if !t_min.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "t_min" });
    }
    if !rh_input.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "rh_input" });
    }
    if t_min > t_max {
        return Err(MeteoError::InvalidTemperatureRange { t_min, t_max });
    }

    match mode {
        RelativeHumidityMode::RelativeHumidityPercent => {
            if !(0.0..=100.0).contains(&rh_input) {
                return Err(MeteoError::RelativeHumidityOutOfRange { rh: rh_input });
            }
            let ea_tmax = saturation_vapor_pressure(t_max);
            let ea_tmin = saturation_vapor_pressure(t_min);

            // Preserve TIME.FOR:526 exact operation order:
            let ea_dew = rh_input / (50.0 / ea_tmin + 50.0 / ea_tmax);
            Ok(ea_dew)
        }
        RelativeHumidityMode::VaporPressureKPa => {
            if rh_input < 0.0 {
                return Err(MeteoError::NegativeVaporPressure { ea: rh_input });
            }
            Ok(rh_input)
        }
    }
}

/// Evaluates net longwave radiation $R_{nl}$ [MJ/m2/d] (`TIME.FOR:771-781`).
///
/// Follows `subroutine RadLongNet`:
/// ```text
/// sigma = 0.00000000245 * ((TMax + 273.16)**4 + (TMin + 273.16)**4)
/// Emissivity = LongWaveRadA1 + LongWaveRadB1 * sqrt(Ea)
/// Rnl = sigma * CloudF * Emissivity
/// ```
pub fn net_longwave(
    t_max: f32,
    t_min: f32,
    ea_dew: f32,
    cloud_f: f32,
    params: &RadiationParams,
) -> Result<f32, MeteoError> {
    if !t_max.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "t_max" });
    }
    if !t_min.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "t_min" });
    }
    if !ea_dew.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "ea_dew" });
    }
    if !cloud_f.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "cloud_f" });
    }
    if t_min > t_max {
        return Err(MeteoError::InvalidTemperatureRange { t_min, t_max });
    }
    if ea_dew < 0.0 {
        return Err(MeteoError::NegativeVaporPressure { ea: ea_dew });
    }

    // Preserve TIME.FOR:776:
    let tmax_k = t_max + TEMP_OFFSET;
    let tmin_k = t_min + TEMP_OFFSET;
    let sigma = SIGMA_COEFF * (tmax_k.powi(4) + tmin_k.powi(4));

    // Preserve TIME.FOR:777:
    let emissivity = params.long_wave_rad_a1 + params.long_wave_rad_b1 * ea_dew.sqrt();

    // Preserve TIME.FOR:778:
    let rnl = sigma * cloud_f * emissivity;

    Ok(rnl)
}

/// Evaluates net shortwave radiation $R_{ns} = (1 - \alpha) R_{\text{ad}}$ [MJ/m2/d] (`TIME.FOR:572`).
pub fn net_shortwave(rad: f32, albedo: f32) -> Result<f32, MeteoError> {
    if !rad.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "rad" });
    }
    if !albedo.is_finite() {
        return Err(MeteoError::NonFiniteParameter { name: "albedo" });
    }
    if rad < 0.0 {
        return Err(MeteoError::NegativeRadiation { radiation: rad });
    }
    if !(0.0..=1.0).contains(&albedo) {
        return Err(MeteoError::InvalidAlbedo { albedo });
    }

    // Preserve TIME.FOR:572:
    let rns = (1.0 - albedo) * rad;
    Ok(rns)
}

/// Evaluates surface net radiation budget $R_n = R_{ns} - R_{nl}$ [MJ/m2/d] (`TIME.FOR:579`).
#[inline]
pub fn net_radiation(rns: f32, rnl: f32) -> f32 {
    rns - rnl
}

/// Computes the complete daily net radiation budget ($R_n, R_{ns}, R_{nl}$) matching `SetMeteo` (`TIME.FOR:534-583`).
pub fn calculate_net_radiation(
    latitude: f32,
    input: &RadiationDailyInput,
    params: &RadiationParams,
) -> Result<NetRadiation, MeteoError> {
    if input.radiation_mode == RadiationMode::NetRadiationInput {
        if !input.radiation_input.is_finite() {
            return Err(MeteoError::NonFiniteParameter {
                name: "radiation_input",
            });
        }
        let rad = input.radiation_input;
        let rn = rad;
        let rns = (1.0 - params.albedo) * rad;
        let rnl = 0.0;
        let ea_dew =
            dew_point_vapor_pressure(input.t_max, input.t_min, input.rh_input, input.rel_hum_mode)?;
        return Ok(NetRadiation {
            rn,
            rns,
            rnl,
            rad,
            ra: 0.0,
            cloud_f: 0.0,
            ea_dew,
        });
    }

    let day_no = input.day_no % 365.0;
    let rad_global = extraterrestrial_radiation(latitude, day_no)?;
    let mut cloud = cloudiness_factor(
        input.sunshine_mode,
        input.sun_hours,
        rad_global.omega,
        params,
    )?;

    // Determine incoming solar radiation Rad:
    let rad = match input.radiation_mode {
        RadiationMode::PotentialFromSunshine => {
            // TIME.FOR:543: Rad = Ra * (ShortWaveRadA + ShortWaveRadB * n_N)
            rad_global.ra * (params.short_wave_rad_a + params.short_wave_rad_b * cloud.n_n)
        }
        RadiationMode::SolarRadiationInput => {
            if input.radiation_input < 0.0 {
                return Err(MeteoError::NegativeRadiation {
                    radiation: input.radiation_input,
                });
            }
            let rad = input.radiation_input;
            if input.sunshine_mode == SunshineMode::ClearSkySolar {
                // TIME.FOR:545-548, 550, 568:
                let tt = if rad_global.ra > 0.0 {
                    rad / rad_global.ra
                } else {
                    0.0
                };
                cloud.cover = 0.1_f32.max((1.0_f32).min(2.330 - 3.330 * tt));
                cloud.n_n = 1.0 - cloud.cover;
                let rad_cs =
                    rad_global.ra * (params.short_wave_rad_a + params.short_wave_rad_b * 1.0);
                cloud.cloud_f = if rad_cs > 0.0 {
                    params.cloud_f_ac * (rad / rad_cs) + params.cloud_f_bc
                } else {
                    params.cloud_f_bc
                };
            }
            rad
        }
        RadiationMode::NetRadiationInput => unreachable!(),
    };

    let rns = net_shortwave(rad, params.albedo)?;
    let ea_dew =
        dew_point_vapor_pressure(input.t_max, input.t_min, input.rh_input, input.rel_hum_mode)?;
    let rnl = net_longwave(input.t_max, input.t_min, ea_dew, cloud.cloud_f, params)?;
    let rn = net_radiation(rns, rnl);

    Ok(NetRadiation {
        rn,
        rns,
        rnl,
        rad,
        ra: rad_global.ra,
        cloud_f: cloud.cloud_f,
        ea_dew,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests latitude decoding from legacy degrees-and-minutes decimal encoding (`TIME.FOR:679-680`).
    ///
    /// Expected values computed from:
    /// - `Lat1 = (Latitude - aint(Latitude)) * (5./3.) + aint(Latitude)`
    /// - `Lat2 = Lat1 * PI / 180.`
    #[test]
    fn test_latitude_decoding_degrees_minutes() {
        // 50 deg 30 min -> 50.30 -> Lat1 = 0.30 * (5/3) + 50.0 = 50.50 deg
        let (lat1, lat2) = parse_latitude_deg_min(50.30).unwrap();
        assert!((lat1 - 50.50).abs() < 1e-6);
        assert!((lat2 - (50.50 * PI / 180.0)).abs() < 1e-6);

        // 34 deg 15 min -> 34.15 -> Lat1 = 0.15 * (5/3) + 34.0 = 34.25 deg
        let (lat1, lat2) = parse_latitude_deg_min(34.15).unwrap();
        assert!((lat1 - 34.25).abs() < 1e-5);
        assert!((lat2 - (34.25 * PI / 180.0)).abs() < 1e-5);

        // Negative latitude: -50 deg 30 min -> -50.30 -> Lat1 = -0.30 * (5/3) - 50.0 = -50.50 deg
        let (lat1, lat2) = parse_latitude_deg_min(-50.30).unwrap();
        assert!((lat1 - (-50.50)).abs() < 1e-6);
        assert!((lat2 - (-50.50 * PI / 180.0)).abs() < 1e-6);

        // Zero latitude (Equator)
        let (lat1, lat2) = parse_latitude_deg_min(0.0).unwrap();
        assert_eq!(lat1, 0.0);
        assert_eq!(lat2, 0.0);

        // Invalid minutes (>= 60)
        assert!(parse_latitude_deg_min(50.60).is_err());
        assert!(parse_latitude_deg_min(50.85).is_err());

        // Out of range latitude (> 90 or < -90)
        assert!(parse_latitude_deg_min(95.0).is_err());
        assert!(parse_latitude_deg_min(-91.0).is_err());
    }

    /// Verifies extraterrestrial radiation $R_a$ against hand-evaluated source formulas
    /// citing `TIME.FOR:676-689`.
    ///
    /// For Latitude = 50.30 (Lat1 = 50.5 deg, Lat2 = 0.8813913 rad) and DayNo = 150.0:
    /// - `PI = 3.141592654` (`TIME.FOR:676`)
    /// - `SC = 118.08` (`TIME.FOR:677`)
    /// - `SolDeclin = sin(2.*PI/365.*DayNo-1.39)*0.4093 = 0.38030475` (`TIME.FOR:681`)
    /// - `Omega = acos(-tan(SolDeclin)*tan(Lat2)) = 2.0771081` (`TIME.FOR:682`)
    /// - `xx = sin(SolDeclin)*sin(Lat2) = 0.2864297` (`TIME.FOR:683`)
    /// - `yy = cos(SolDeclin)*cos(Lat2) = 0.59063137` (`TIME.FOR:684`)
    /// - `dr = 1.0 + 0.033*cos(2.*PI/365.*DayNo) = 0.9720311` (`TIME.FOR:685`)
    /// - `Ra = SC/PI*dr*(Omega*xx+sin(Omega)*yy) = 40.607533` (`TIME.FOR:686`)
    #[test]
    fn test_extraterrestrial_radiation_source_formula() {
        let rad = extraterrestrial_radiation(50.30, 150.0).unwrap();
        assert!((rad.sol_declin - 0.38030475).abs() < 1e-5);
        assert!((rad.omega - 2.0771081).abs() < 1e-5);
        assert!((rad.xx - 0.2864297).abs() < 1e-5);
        assert!((rad.yy - 0.59063137).abs() < 1e-5);
        assert!((rad.dr - 0.9720311).abs() < 1e-5);
        assert!((rad.ra - 40.607533).abs() < 1e-3);
    }

    /// Verifies cloudiness factor against source formulas citing `TIME.FOR:704-718`.
    ///
    /// For Omega = 2.0771081, SunHours = 10.0, LongWaveRadA = 0.90, LongWaveRadB = 0.10:
    /// - `NN = 24./PI*Omega = 15.867937` (`TIME.FOR:705`)
    /// - `n_N = min(10.0 / 15.867937, 1.) = 0.63020164` (`TIME.FOR:706`)
    /// - `CloudF = 0.10 + 0.90 * 0.63020164 = 0.6671815` (`TIME.FOR:707`)
    /// - `Cover = 1. - 0.63020164 = 0.36979836` (`TIME.FOR:708`)
    #[test]
    fn test_cloudiness_factor_source_formula() {
        let params = RadiationParams::default();
        let omega = 2.0771081;
        let cloud = cloudiness_factor(SunshineMode::SunshineHours, 10.0, omega, &params).unwrap();
        assert!((cloud.nn - 15.867937).abs() < 1e-4);
        assert!((cloud.n_n - 0.63020164).abs() < 1e-4);
        assert!((cloud.cloud_f - 0.6671815).abs() < 1e-4);
        assert!((cloud.cover - 0.36979836).abs() < 1e-4);

        // Transmission coefficient mode (iSunSh = 2, TIME.FOR:713-718)
        let cloud_tt =
            cloudiness_factor(SunshineMode::TransmissionCoeff, 0.5, omega, &params).unwrap();
        let expected_cover = 2.330 - 3.330 * 0.5; // 0.665
        assert!((cloud_tt.cover - expected_cover).abs() < 1e-4);
        assert!((cloud_tt.n_n - (1.0 - expected_cover)).abs() < 1e-4);
    }

    /// Verifies saturation and dew point vapor pressure against source formulas
    /// citing `TIME.FOR:522-531`.
    ///
    /// For TMax = 25.0 °C, TMin = 15.0 °C, RHMean = 60.0%:
    /// - `Ea_TMax = 0.6108*exp(17.27*25.0 / (25.0+237.3)) = 3.167778` (`TIME.FOR:522`)
    /// - `Ea_TMin = 0.6108*exp(17.27*15.0 / (15.0+237.3)) = 1.7053465` (`TIME.FOR:523`)
    /// - `EaDew = 60.0 / (50. / 1.7053465 + 50. / 3.167778) = 1.330274` (`TIME.FOR:526`)
    #[test]
    fn test_vapor_pressure_source_formula() {
        let ea_tmax = saturation_vapor_pressure(25.0);
        let ea_tmin = saturation_vapor_pressure(15.0);
        assert!((ea_tmax - 3.167778).abs() < 1e-4);
        assert!((ea_tmin - 1.7053465).abs() < 1e-4);

        let ea_dew = dew_point_vapor_pressure(
            25.0,
            15.0,
            60.0,
            RelativeHumidityMode::RelativeHumidityPercent,
        )
        .unwrap();
        assert!((ea_dew - 1.330274).abs() < 1e-4);
    }

    /// Verifies net longwave radiation $R_{nl}$ against source formulas citing `TIME.FOR:771-781`.
    ///
    /// For TMax = 25.0 °C, TMin = 15.0 °C, Ea = 1.330274 kPa, CloudF = 0.6671815:
    /// - `sigma = 0.00000000245 * ((25.0 + 273.16)**4 + (15.0 + 273.16)**4) = 36.255367` (`TIME.FOR:776`)
    /// - `Emissivity = 0.34 + (-0.139) * sqrt(1.330274) = 0.17968087` (`TIME.FOR:777`)
    /// - `Rnl = 36.255367 * 0.6671815 * 0.17968087 = 4.3462844` (`TIME.FOR:778`)
    #[test]
    fn test_net_longwave_source_formula() {
        let params = RadiationParams::default();
        let ea_dew = 1.330274;
        let cloud_f = 0.6671815;
        let rnl = net_longwave(25.0, 15.0, ea_dew, cloud_f, &params).unwrap();
        assert!((rnl - 4.3462844).abs() < 1e-4);
    }

    /// Verifies net shortwave and net radiation budget citing `TIME.FOR:572, 579`.
    ///
    /// For Rad = 22.94735 MJ/m2/d, Albedo = 0.23, Rnl = 4.3462844:
    /// - `Rns = (1. - 0.23) * 22.94735 = 17.66946` (`TIME.FOR:572`)
    /// - `Rn = 17.66946 - 4.3462844 = 13.323175` (`TIME.FOR:579`)
    #[test]
    fn test_net_shortwave_and_net_radiation() {
        let rns = net_shortwave(22.94735, 0.23).unwrap();
        assert!((rns - 17.66946).abs() < 1e-4);

        let rn = net_radiation(rns, 4.3462844);
        assert!((rn - 13.323175).abs() < 1e-4);
    }

    /// Verifies full daily net radiation pipeline `calculate_net_radiation` across all modes.
    #[test]
    fn test_calculate_net_radiation_pipeline() {
        let params = RadiationParams::default();
        let input_mode0 = RadiationDailyInput {
            day_no: 150.0,
            radiation_input: 0.0,
            t_max: 25.0,
            t_min: 15.0,
            rh_input: 60.0,
            sun_hours: 10.0,
            radiation_mode: RadiationMode::PotentialFromSunshine,
            sunshine_mode: SunshineMode::SunshineHours,
            rel_hum_mode: RelativeHumidityMode::RelativeHumidityPercent,
        };

        let result = calculate_net_radiation(50.30, &input_mode0, &params).unwrap();
        assert!((result.ra - 40.607533).abs() < 1e-3);
        assert!((result.cloud_f - 0.6671815).abs() < 1e-4);
        assert!((result.rad - 22.94735).abs() < 1e-3);
        assert!((result.rns - 17.66946).abs() < 1e-3);
        assert!((result.rnl - 4.3462844).abs() < 1e-3);
        assert!((result.rn - 13.323175).abs() < 1e-3);

        // Mode 1: Measured solar radiation input
        let input_mode1 = RadiationDailyInput {
            radiation_input: 20.0,
            radiation_mode: RadiationMode::SolarRadiationInput,
            ..input_mode0
        };
        let result_mode1 = calculate_net_radiation(50.30, &input_mode1, &params).unwrap();
        assert_eq!(result_mode1.rad, 20.0);
        assert!((result_mode1.rns - (1.0 - 0.23) * 20.0).abs() < 1e-5);

        // Mode 2: Measured net radiation input (TIME.FOR:581: Rn = Rad)
        let input_mode2 = RadiationDailyInput {
            radiation_input: 12.5,
            radiation_mode: RadiationMode::NetRadiationInput,
            ..input_mode0
        };
        let result_mode2 = calculate_net_radiation(50.30, &input_mode2, &params).unwrap();
        assert_eq!(result_mode2.rn, 12.5);

        // Mode 1 with clear sky model (iSunSh = 3, TIME.FOR:544-570)
        let input_mode1_clearsky = RadiationDailyInput {
            radiation_input: 25.0,
            radiation_mode: RadiationMode::SolarRadiationInput,
            sunshine_mode: SunshineMode::ClearSkySolar,
            ..input_mode0
        };
        let result_clearsky =
            calculate_net_radiation(50.30, &input_mode1_clearsky, &params).unwrap();
        assert!(result_clearsky.cloud_f > 0.0);
        assert!(result_clearsky.rn > 0.0);
    }

    /// Verifies error handling and input validation across meteorological parameters.
    #[test]
    fn test_meteo_validation_errors() {
        let params = RadiationParams::default();

        // Non-finite parameters
        assert!(parse_latitude_deg_min(f32::NAN).is_err());
        assert!(extraterrestrial_radiation(50.0, f32::NAN).is_err());
        assert!(extraterrestrial_radiation(50.0, -1.0).is_err());

        // Inverted temperatures
        assert!(dew_point_vapor_pressure(
            10.0,
            20.0,
            50.0,
            RelativeHumidityMode::RelativeHumidityPercent
        )
        .is_err());
        assert!(net_longwave(10.0, 20.0, 1.0, 0.5, &params).is_err());

        // Humidity out of range
        assert!(dew_point_vapor_pressure(
            20.0,
            10.0,
            -5.0,
            RelativeHumidityMode::RelativeHumidityPercent
        )
        .is_err());
        assert!(dew_point_vapor_pressure(
            20.0,
            10.0,
            105.0,
            RelativeHumidityMode::RelativeHumidityPercent
        )
        .is_err());

        // Negative vapor pressure
        assert!(
            dew_point_vapor_pressure(20.0, 10.0, -0.1, RelativeHumidityMode::VaporPressureKPa)
                .is_err()
        );
        assert!(net_longwave(20.0, 10.0, -0.5, 0.5, &params).is_err());

        // Negative radiation
        assert!(net_shortwave(-1.0, 0.23).is_err());

        // Invalid albedo
        assert!(net_shortwave(20.0, -0.1).is_err());
        assert!(net_shortwave(20.0, 1.1).is_err());

        // Mode conversions
        assert_eq!(
            RadiationMode::try_from(0).unwrap(),
            RadiationMode::PotentialFromSunshine
        );
        assert_eq!(
            RadiationMode::try_from(1).unwrap(),
            RadiationMode::SolarRadiationInput
        );
        assert_eq!(
            RadiationMode::try_from(2).unwrap(),
            RadiationMode::NetRadiationInput
        );
        assert!(RadiationMode::try_from(3).is_err());

        assert_eq!(
            SunshineMode::try_from(0).unwrap(),
            SunshineMode::SunshineHours
        );
        assert_eq!(SunshineMode::try_from(1).unwrap(), SunshineMode::Cloudiness);
        assert_eq!(
            SunshineMode::try_from(2).unwrap(),
            SunshineMode::TransmissionCoeff
        );
        assert_eq!(
            SunshineMode::try_from(3).unwrap(),
            SunshineMode::ClearSkySolar
        );
        assert!(SunshineMode::try_from(4).is_err());

        assert_eq!(
            RelativeHumidityMode::try_from(0).unwrap(),
            RelativeHumidityMode::RelativeHumidityPercent
        );
        assert_eq!(
            RelativeHumidityMode::try_from(1).unwrap(),
            RelativeHumidityMode::VaporPressureKPa
        );
        assert!(RelativeHumidityMode::try_from(2).is_err());
    }

    /// Verifies polar day/night clamping of sunset hour angle $\omega$ (`TIME.FOR:682`).
    #[test]
    fn test_polar_latitude_clamping() {
        // High arctic latitude in winter (polar night, e.g. 80 deg North on Day 355)
        let polar_night = extraterrestrial_radiation(80.0, 355.0).unwrap();
        assert_eq!(polar_night.omega, 0.0);
        assert_eq!(polar_night.ra, 0.0);

        // High arctic latitude in summer (polar day / midnight sun, e.g. 80 deg North on Day 172)
        let polar_day = extraterrestrial_radiation(80.0, 172.0).unwrap();
        assert!((polar_day.omega - PI).abs() < 1e-4);
        assert!(polar_day.ra > 30.0);
    }
}

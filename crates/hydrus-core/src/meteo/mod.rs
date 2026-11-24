//! Meteorological parameter and radiation modeling for HYDRUS (`INPUT.FOR:934-1066`, `TIME.FOR:443-800`).

pub mod config;
pub mod evaporation;
pub mod radiation;
pub mod series;

pub use evaporation::{
    aerodynamic_terms, calculate_bare_soil_et, penman_monteith_et, wind_km_day_to_m_s,
    EvaporationInput, EvaporationResult, MeteoSite,
};

pub use radiation::{
    calculate_net_radiation, cloudiness_factor, dew_point_vapor_pressure,
    extraterrestrial_radiation, net_longwave, net_radiation, net_shortwave, parse_latitude_deg_min,
    saturation_vapor_pressure, CloudinessResult, ExtraterrestrialRadiation, NetRadiation,
    RadiationDailyInput,
};

pub use config::MeteoConfig;
pub use series::{parse_meteo_deck, MeteoRecord};

use core::fmt;

/// Errors raised during meteorological parameter validation and calculations.
#[derive(Clone, Debug, PartialEq)]
pub enum MeteoError {
    /// Non-finite parameter value.
    NonFiniteParameter { name: &'static str },
    /// Latitude outside valid range `[-90.0, 90.0]` or invalid minutes portion.
    InvalidLatitude { latitude: f32 },
    /// Day of year must be non-negative.
    InvalidDayOfYear { day_no: f32 },
    /// Negative radiation value.
    NegativeRadiation { radiation: f32 },
    /// Inverted temperature range ($T_{\min} > T_{\max}$).
    InvalidTemperatureRange { t_min: f32, t_max: f32 },
    /// Relative humidity out of range `[0.0, 100.0]` (%).
    RelativeHumidityOutOfRange { rh: f32 },
    /// Negative vapor pressure.
    NegativeVaporPressure { ea: f32 },
    /// Negative sunshine hours.
    NegativeSunHours { sun_hours: f32 },
    /// Negative wind speed.
    NegativeWindSpeed { wind: f32 },
    /// Albedo outside valid range `[0.0, 1.0]`.
    InvalidAlbedo { albedo: f32 },
    /// Crop zero-displacement height reaches or exceeds measurement height.
    CropHeightExceedsMeasurementHeight {
        crop_height: f32,
        wind_height: f32,
        temp_height: f32,
    },
    /// Surface energy balance iteration is deferred to G20.
    SurfaceEnergyBalanceUnsupported,
    /// Unsupported radiation mode.
    UnsupportedRadiationMode { mode: i32 },
    /// Unsupported sunshine mode.
    UnsupportedSunshineMode { mode: i32 },
    /// Unsupported relative humidity mode.
    UnsupportedRelativeHumidityMode { mode: i32 },
    /// Malformed or unreadable meteorological deck.
    InvalidMeteoDeck { path: String, reason: String },
    /// Crop growth table larger than the Fortran limit of 1000 rows (`INPUT.FOR:999-1003`).
    GrowthTableOverflow { count: i32 },
    /// Selection outside the Gate A scope (bare soil, daily records, solar radiation input).
    UnsupportedMeteoSelection { field: &'static str, value: i32 },
}

impl fmt::Display for MeteoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter { name } => write!(f, "non-finite parameter: {name}"),
            Self::InvalidLatitude { latitude } => {
                write!(f, "invalid latitude: {latitude} (expected [-90.0, 90.0])")
            }
            Self::InvalidDayOfYear { day_no } => {
                write!(f, "invalid day of year: {day_no} (expected >= 0.0)")
            }
            Self::NegativeRadiation { radiation } => {
                write!(f, "negative radiation: {radiation}")
            }
            Self::InvalidTemperatureRange { t_min, t_max } => {
                write!(f, "invalid temperature range: t_min={t_min} > t_max={t_max}")
            }
            Self::RelativeHumidityOutOfRange { rh } => {
                write!(f, "relative humidity out of range: {rh}% (expected [0.0, 100.0])")
            }
            Self::NegativeVaporPressure { ea } => {
                write!(f, "negative vapor pressure: {ea} kPa")
            }
            Self::NegativeSunHours { sun_hours } => {
                write!(f, "negative sunshine hours: {sun_hours}")
            }
            Self::NegativeWindSpeed { wind } => {
                write!(f, "negative wind speed: {wind}")
            }
            Self::InvalidAlbedo { albedo } => {
                write!(f, "invalid albedo: {albedo} (expected [0.0, 1.0])")
            }
            Self::CropHeightExceedsMeasurementHeight {
                crop_height,
                wind_height,
                temp_height,
            } => write!(
                f,
                "crop height {crop_height} cm exceeds measurement heights (wind={wind_height} cm, temp={temp_height} cm)"
            ),
            Self::SurfaceEnergyBalanceUnsupported => {
                write!(f, "surface energy balance (lEnBal) is deferred to G20")
            }
            Self::UnsupportedRadiationMode { mode } => {
                write!(f, "unsupported radiation mode: {mode}")
            }
            Self::UnsupportedSunshineMode { mode } => {
                write!(f, "unsupported sunshine mode: {mode}")
            }
            Self::UnsupportedRelativeHumidityMode { mode } => {
                write!(f, "unsupported relative humidity mode: {mode}")
            }
            Self::InvalidMeteoDeck { path, reason } => {
                write!(f, "invalid meteo deck {path}: {reason}")
            }
            Self::GrowthTableOverflow { count } => {
                write!(f, "crop growth table has {count} rows (maximum 1000)")
            }
            Self::UnsupportedMeteoSelection { field, value } => {
                write!(f, "unsupported meteo selection: {field}={value}")
            }
        }
    }
}

impl std::error::Error for MeteoError {}

/// Radiation mode corresponding to `iRadiation` in Fortran (`INPUT.FOR:963`, `TIME.FOR:462`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RadiationMode {
    /// Mode 0: Potential radiation calculated from sunshine hours/cloudiness via Angstrom formula.
    PotentialFromSunshine = 0,
    /// Mode 1: Measured solar radiation input directly.
    SolarRadiationInput = 1,
    /// Mode 2: Measured net radiation input directly.
    NetRadiationInput = 2,
}

impl TryFrom<i32> for RadiationMode {
    type Error = MeteoError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::PotentialFromSunshine),
            1 => Ok(Self::SolarRadiationInput),
            2 => Ok(Self::NetRadiationInput),
            _ => Err(MeteoError::UnsupportedRadiationMode { mode: value }),
        }
    }
}

/// Sunshine mode corresponding to `iSunSh` in Fortran (`INPUT.FOR:982`, `TIME.FOR:704-718`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SunshineMode {
    /// Mode 0: Bright sunshine hours per day [hr].
    SunshineHours = 0,
    /// Mode 1: Cloudiness fraction [-].
    Cloudiness = 1,
    /// Mode 2: Transmission coefficient [-].
    TransmissionCoeff = 2,
    /// Mode 3: Solar radiation input used with clear sky model.
    ClearSkySolar = 3,
}

impl TryFrom<i32> for SunshineMode {
    type Error = MeteoError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SunshineHours),
            1 => Ok(Self::Cloudiness),
            2 => Ok(Self::TransmissionCoeff),
            3 => Ok(Self::ClearSkySolar),
            _ => Err(MeteoError::UnsupportedSunshineMode { mode: value }),
        }
    }
}

/// Relative humidity mode corresponding to `iRelHum` in Fortran (`INPUT.FOR:982`, `TIME.FOR:524-531`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelativeHumidityMode {
    /// Mode 0: Average relative humidity [%].
    RelativeHumidityPercent = 0,
    /// Mode 1: Average daily vapor pressure [kPa].
    VaporPressureKPa = 1,
}

impl TryFrom<i32> for RelativeHumidityMode {
    type Error = MeteoError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::RelativeHumidityPercent),
            1 => Ok(Self::VaporPressureKPa),
            _ => Err(MeteoError::UnsupportedRelativeHumidityMode { mode: value }),
        }
    }
}

/// Radiation parameters corresponding to `MeteoIn` (`INPUT.FOR:948-978`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadiationParams {
    /// Fraction of extraterrestrial radiation on overcast days ($a_s$, Angstrom coefficient eq. 55).
    pub short_wave_rad_a: f32,
    /// Fraction of extraterrestrial radiation on overcast days ($b_s$, Angstrom coefficient eq. 55).
    pub short_wave_rad_b: f32,
    /// Cloudiness factor coefficient ($a_c$, eq. 59).
    pub long_wave_rad_a: f32,
    /// Cloudiness factor coefficient ($b_c$, eq. 59).
    pub long_wave_rad_b: f32,
    /// Emissivity correlation coefficient ($a_1$, eq. 60).
    pub long_wave_rad_a1: f32,
    /// Emissivity correlation coefficient ($b_1$, eq. 60, $\text{kPa}^{-0.5}$).
    pub long_wave_rad_b1: f32,
    /// Clear-sky cloudiness coefficient $A_c$ (eq. 57).
    pub cloud_f_ac: f32,
    /// Clear-sky cloudiness coefficient $B_c$ (eq. 57).
    pub cloud_f_bc: f32,
    /// Surface albedo $\alpha$ [-].
    pub albedo: f32,
}

impl Default for RadiationParams {
    fn default() -> Self {
        Self {
            short_wave_rad_a: 0.25,
            short_wave_rad_b: 0.50,
            long_wave_rad_a: 0.90,
            long_wave_rad_b: 0.10,
            long_wave_rad_a1: 0.34,
            long_wave_rad_b1: -0.139,
            cloud_f_ac: 1.35,
            cloud_f_bc: -0.35,
            albedo: 0.23,
        }
    }
}

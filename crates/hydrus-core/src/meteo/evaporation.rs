//! Penman-Monteith potential evaporation for bare soil with daily records
//! (`TIME.FOR:443-626` `SetMeteo`, `TIME.FOR:630-667` `Aero`).
//!
//! Scope is fixed by the signature: bare soil (`iCrop=0`, so `CropRes` gives
//! `CropHeight=0.1` cm, `rc=0`, `SCF=0`), daily records, no Hargreaves, no
//! surface energy balance. All arithmetic is `f32` in Fortran operation order
//! (ADR 0010, ADR 0015).

use super::radiation::{
    calculate_net_radiation, saturation_vapor_pressure, NetRadiation, RadiationDailyInput,
};
use super::{MeteoError, RadiationParams};

/// Bare-soil roughness height forced by `CropRes` [cm] (`TIME.FOR:739`).
pub const BARE_SOIL_CROP_HEIGHT_CM: f32 = 0.1;

/// Site constants from `MeteoIn` (`INPUT.FOR:934-1066`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeteoSite {
    /// Latitude in degrees.minutes decimal format.
    pub latitude: f32,
    /// Altitude [m].
    pub altitude: f32,
    /// Wind measurement height [cm].
    pub wind_height: f32,
    /// Temperature measurement height [cm].
    pub temp_height: f32,
}

/// One daily meteorological record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvaporationInput {
    /// Daily radiation-side inputs (includes `day_no`, temperatures, humidity).
    pub radiation: RadiationDailyInput,
    /// Wind speed [km/d].
    pub wind_km_d: f32,
}

/// Aerodynamic and radiation terms of Penman-Monteith [kg/m2/d].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AerodynamicTerms {
    pub aero_term: f32,
    pub rad_term: f32,
}

/// Potential evaporation result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvaporationResult {
    /// Combined potential evapotranspiration [mm/d].
    pub et_comb: f32,
    /// Potential soil evaporation [mm/d].
    pub evap_p: f32,
    /// Potential transpiration [mm/d].
    pub trans_p: f32,
    pub rad_term: f32,
    pub aero_term: f32,
    pub net_radiation: NetRadiation,
}

/// Converts wind from km/d to m/s (`TIME.FOR:515`).
#[inline]
pub fn wind_km_day_to_m_s(wind_km_d: f32) -> f32 {
    wind_km_d / 86.4
}

/// Port of `subroutine Aero` with `rc = 0` (bare soil).
#[allow(clippy::too_many_arguments)]
pub fn aerodynamic_terms(
    rn: f32,
    crop_height: f32,
    wind_height: f32,
    temp_height: f32,
    wind_ms: f32,
    altitude: f32,
    t_max: f32,
    t_min: f32,
    ea_dew: f32,
) -> Result<AerodynamicTerms, MeteoError> {
    let rc: f32 = 0.0;
    let mut raa: f32 = 0.0;
    let t_aver = (t_max + t_min) / 2.0;
    let ea_tmax = saturation_vapor_pressure(t_max);
    let ea_tmin = saturation_vapor_pressure(t_min);

    let dl0 = 0.667 * crop_height;
    if dl0 >= wind_height || dl0 >= temp_height {
        return Err(MeteoError::CropHeightExceedsMeasurementHeight {
            crop_height,
            wind_height,
            temp_height,
        });
    }
    let aer_dyn_res = ((wind_height - dl0) / (0.123 * crop_height)).ln()
        * ((temp_height - dl0) / (0.0123 * crop_height)).ln()
        / (0.41_f32 * 0.41_f32); // Equations 36-39
    if wind_ms > 0.0 {
        raa = aer_dyn_res / wind_ms; // Equation 36
    }
    let aero_tcff = 0.622 * 3.486 * 86400.0 / aer_dyn_res / 1.01; // Equation 47b
    let p_atm = 101.3 * ((293.0 - 0.0065 * altitude) / 293.0).powf(5.253); // Equation 6
    let lambda = 2.501 - 0.002361 * t_aver; // Equation 1
    let gamma = 0.0016286 * p_atm / lambda; // Equation 4
    let mut gamma1 = gamma;
    if raa > 0.0 {
        gamma1 = gamma * (1.0 + rc / raa); // Equation 41
    }
    let tx = t_max + 237.3;
    let tn = t_min + 237.3;
    let dlt = 2049.0 * ea_tmax / (tx * tx) + 2049.0 * ea_tmin / (tn * tn); // Equation 3
    let dl_dl = dlt / (dlt + gamma1); // Equation 49a
    let gm_dl = gamma / (dlt + gamma1); // Equation 47a
    let ea_mean = (ea_tmax + ea_tmin) / 2.0; // Equation 11
    let aero_term = gm_dl * aero_tcff / (t_aver + 273.0) * wind_ms * (ea_mean - ea_dew);

    let mut rad_term: f32 = 0.0;
    if lambda > 0.0 {
        rad_term = dl_dl * rn / lambda; // Equation 49
    }
    Ok(AerodynamicTerms {
        aero_term,
        rad_term,
    })
}

/// Combination ET [mm/d] from the terms (`TIME.FOR:591-596`).
pub fn penman_monteith_et(terms: AerodynamicTerms, t_aver: f32) -> f32 {
    let et = (terms.rad_term + terms.aero_term).max(0.0); // Equation 69, 31
    let d = t_aver - 4.0;
    let row = (1.0 - 7.37e-6 * (d * d) + 3.79e-8 * (d * d * d)) * 1000.0;
    et / row * 1000.0
}

/// Splits ET into evaporation and transpiration (`TIME.FOR:601-602`).
pub fn partition_et(et_comb: f32, scf: f32) -> (f32, f32) {
    (et_comb * (1.0 - scf), et_comb * scf)
}

/// Bare-soil, daily-record potential evaporation (`SetMeteo` core).
pub fn calculate_bare_soil_et(
    site: &MeteoSite,
    input: &EvaporationInput,
    params: &RadiationParams,
) -> Result<EvaporationResult, MeteoError> {
    if !(input.wind_km_d.is_finite() && input.wind_km_d >= 0.0) {
        return Err(if input.wind_km_d.is_finite() {
            MeteoError::NegativeWindSpeed {
                wind: input.wind_km_d,
            }
        } else {
            MeteoError::NonFiniteParameter { name: "wind_km_d" }
        });
    }
    let net = calculate_net_radiation(site.latitude, &input.radiation, params)?;
    let wind_ms = wind_km_day_to_m_s(input.wind_km_d);
    let terms = aerodynamic_terms(
        net.rn,
        BARE_SOIL_CROP_HEIGHT_CM,
        site.wind_height,
        site.temp_height,
        wind_ms,
        site.altitude,
        input.radiation.t_max,
        input.radiation.t_min,
        net.ea_dew,
    )?;
    let t_aver = (input.radiation.t_max + input.radiation.t_min) / 2.0;
    let et_comb = penman_monteith_et(terms, t_aver);
    let (evap_p, trans_p) = partition_et(et_comb, 0.0);
    Ok(EvaporationResult {
        et_comb,
        evap_p,
        trans_p,
        rad_term: terms.rad_term,
        aero_term: terms.aero_term,
        net_radiation: net,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteo::{RadiationMode, RelativeHumidityMode, SunshineMode};

    fn site() -> MeteoSite {
        MeteoSite {
            latitude: -30.0,
            altitude: 100.0,
            wind_height: 200.0,
            temp_height: 190.0,
        }
    }

    fn rec(
        day: f32,
        rad: f32,
        tmax: f32,
        tmin: f32,
        rh: f32,
        wind: f32,
        sun: f32,
    ) -> EvaporationInput {
        EvaporationInput {
            radiation: RadiationDailyInput {
                day_no: day,
                radiation_input: rad,
                t_max: tmax,
                t_min: tmin,
                rh_input: rh,
                sun_hours: sun,
                radiation_mode: RadiationMode::SolarRadiationInput,
                sunshine_mode: SunshineMode::SunshineHours,
                rel_hum_mode: RelativeHumidityMode::RelativeHumidityPercent,
            },
            wind_km_d: wind,
        }
    }

    #[test]
    fn wind_conversion() {
        assert_eq!(wind_km_day_to_m_s(86.4), 1.0);
        assert_eq!(wind_km_day_to_m_s(0.0), 0.0);
    }

    #[test]
    fn partition_bare_soil_and_cover() {
        assert_eq!(partition_et(5.0, 0.0), (5.0, 0.0));
        assert_eq!(partition_et(4.0, 0.25), (3.0, 1.0));
    }

    #[test]
    fn et_clamps_at_zero() {
        let t = AerodynamicTerms {
            aero_term: -3.0,
            rad_term: 1.0,
        };
        assert_eq!(penman_monteith_et(t, 20.0), 0.0);
    }

    #[test]
    fn density_correction_at_4c_is_identity() {
        // row(4 C) = 1000 exactly, so ET passes through unchanged.
        let t = AerodynamicTerms {
            aero_term: 1.0,
            rad_term: 2.0,
        };
        assert_eq!(penman_monteith_et(t, 4.0), 3.0);
    }

    #[test]
    fn aero_hand_values_zero_wind() {
        // Zero wind: AeroTerm = 0, raa stays 0 so gamma1 = gamma.
        // Hand value (f32 op order): RadTerm = dl_dl * Rn / lambda.
        let (tmax, tmin, rn) = (30.0_f32, 20.0_f32, 10.0_f32);
        let ea_tmax = 0.6108_f32 * ((17.27_f32 * tmax) / (tmax + 237.3)).exp();
        let ea_tmin = 0.6108_f32 * ((17.27_f32 * tmin) / (tmin + 237.3)).exp();
        let t_aver = 25.0_f32;
        let p_atm = 101.3_f32 * ((293.0_f32 - 0.0065 * 100.0) / 293.0).powf(5.253);
        let lambda = 2.501_f32 - 0.002361 * t_aver;
        let gamma = 0.0016286_f32 * p_atm / lambda;
        let dlt = 2049.0_f32 * ea_tmax / ((tmax + 237.3) * (tmax + 237.3))
            + 2049.0 * ea_tmin / ((tmin + 237.3) * (tmin + 237.3));
        let expect = dlt / (dlt + gamma) * rn / lambda;
        let r = aerodynamic_terms(rn, 0.1, 200.0, 190.0, 0.0, 100.0, tmax, tmin, 1.0).unwrap();
        assert_eq!(r.aero_term, 0.0);
        assert_eq!(r.rad_term, expect);
        assert!(r.rad_term > 0.0);
    }

    #[test]
    fn crop_height_exceeds_measurement_height() {
        let e = aerodynamic_terms(1.0, 400.0, 200.0, 190.0, 1.0, 0.0, 30.0, 20.0, 1.0);
        assert!(matches!(
            e,
            Err(MeteoError::CropHeightExceedsMeasurementHeight { .. })
        ));
        let e = aerodynamic_terms(1.0, 300.0, 200.0, 250.0, 1.0, 0.0, 30.0, 20.0, 1.0);
        assert!(matches!(
            e,
            Err(MeteoError::CropHeightExceedsMeasurementHeight { .. })
        ));
    }

    #[test]
    fn negative_wind_rejected() {
        let e = calculate_bare_soil_et(
            &site(),
            &rec(1.0, 24.0, 34.0, 20.0, 35.0, -1.0, 11.0),
            &RadiationParams::default(),
        );
        assert!(matches!(e, Err(MeteoError::NegativeWindSpeed { .. })));
    }

    /// Pinned Fortran `Meteo.out` for case `vapor_met_bare_soil_on` (task 31.2.1),
    /// executable_sha256 87969711f9241b836ad04e24c1987c46e58de2c035d09270ef5c0a33b0519638
    /// in `reference/vapor_fortran/fixtures/vapor_met_bare_soil_on.json`.
    /// Columns: Rad TMax TMin RH Wind Sun | ET Rns Rnl RadTerm AeroTerm (3 decimals).
    #[test]
    fn matches_pinned_fortran_meteo_out() {
        #[rustfmt::skip]
        let rows: [([f32; 6], [f32; 5]); 10] = [
            ([24.0, 34.0, 20.0, 35.0, 250.0, 11.0], [5.505, 18.480, 6.220, 3.859, 1.627]),
            ([25.0, 35.0, 21.0, 32.0, 250.0, 11.5], [5.754, 19.250, 6.632, 4.020, 1.712]),
            ([26.0, 36.0, 22.0, 30.0, 260.0, 12.0], [6.072, 20.020, 6.996, 4.199, 1.849]),
            ([25.0, 35.0, 21.0, 32.0, 250.0, 11.5], [5.751, 19.250, 6.640, 4.018, 1.712]),
            ([24.0, 34.0, 20.0, 35.0, 240.0, 11.0], [5.435, 18.480, 6.233, 3.855, 1.562]),
            ([10.0, 24.0, 18.0, 85.0, 150.0, 2.0], [2.078, 7.700, 1.181, 1.859, 0.215]),
            ([22.0, 30.0, 19.0, 50.0, 200.0, 9.0], [4.727, 16.940, 4.564, 3.750, 0.964]),
            ([25.0, 34.0, 20.0, 35.0, 250.0, 11.5], [5.661, 19.250, 6.495, 4.015, 1.627]),
            ([26.0, 36.0, 22.0, 30.0, 260.0, 12.0], [6.063, 20.020, 7.024, 4.189, 1.849]),
            ([25.0, 35.0, 21.0, 32.0, 250.0, 11.5], [5.742, 19.250, 6.668, 4.009, 1.712]),
        ];
        let mut fails = Vec::new();
        for (i, (m, e)) in rows.iter().enumerate() {
            let r = calculate_bare_soil_et(
                &site(),
                &rec((i + 1) as f32, m[0], m[1], m[2], m[3], m[4], m[5]),
                &RadiationParams::default(),
            )
            .unwrap();
            let got = [
                r.et_comb,
                r.net_radiation.rns,
                r.net_radiation.rnl,
                r.rad_term,
                r.aero_term,
            ];
            assert_eq!(r.evap_p, r.et_comb);
            assert_eq!(r.trans_p, 0.0);
            for (k, (g, x)) in got.iter().zip(e.iter()).enumerate() {
                if (g - x).abs() > 5e-4 {
                    fails.push(format!("day {} col {}: got {} want {}", i + 1, k, g, x));
                }
            }
        }
        assert!(fails.is_empty(), "{fails:#?}");
    }
}

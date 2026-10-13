//! Primitive-input endpoint equations and flux closure from `SOLUTE.FOR`.

use super::types::EndpointPrimitiveState;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EndpointRow {
    pub lower: f64,
    pub diagonal: f64,
    pub upper: f64,
    pub rhs: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssemblyLevel {
    Old,
    New,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EndpointFlux {
    pub top: f64,
    pub bottom: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EndpointResult {
    pub lower: EndpointRow,
    pub upper: EndpointRow,
    pub flux: EndpointFlux,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CanonicalEndpoint {
    Lower,
    Upper,
}

pub(crate) fn canonical_endpoint(
    native_node: usize,
    node_count: usize,
    reversed: bool,
) -> Option<CanonicalEndpoint> {
    if node_count < 2 || native_node >= node_count {
        return None;
    }
    match (native_node == 0, native_node + 1 == node_count, reversed) {
        (true, _, false) | (_, true, true) => Some(CanonicalEndpoint::Lower),
        (_, true, false) | (true, _, true) => Some(CanonicalEndpoint::Upper),
        _ => None,
    }
}

pub(crate) fn assemble_endpoints(
    state: &EndpointPrimitiveState,
    level: AssemblyLevel,
    lower_code: i32,
    upper_code: i32,
    bottom_concentration: f32,
    top_concentration: f32,
    accepted_concentration: &[f32],
) -> Result<EndpointResult, &'static str> {
    let n = state.coordinates.len();
    if n < 2
        || accepted_concentration.len() != n
        || state.material_ids.len() != n
        || state.mobile_immobile.is_empty()
    {
        return Err("endpoint state dimensions are inconsistent");
    }
    for values in [
        &state.old_water,
        &state.new_water,
        &state.old_velocity,
        &state.new_velocity,
        &state.old_dispersion,
        &state.new_dispersion,
        &state.old_retardation,
        &state.new_retardation,
        &state.old_concentration,
        &state.material_immobile_water,
        &state.source_g0,
        &state.source_g1,
        &state.upwind_weight,
    ] {
        if values.iter().any(|x| !x.0.is_finite()) {
            return Err("endpoint primitive arrays must be finite");
        }
    }
    if state
        .coordinates
        .windows(2)
        .any(|w| !w[0].0.is_finite() || w[1].0 <= w[0].0)
        || state.coordinates.last().is_some_and(|v| !v.0.is_finite())
        || state
            .old_accepted_flow
            .iter()
            .chain(&state.new_accepted_flow)
            .chain(&state.old_matrix_velocity)
            .chain(&state.new_matrix_velocity)
            .any(|v| !v.0.is_finite())
    {
        return Err("endpoint coordinates/flows must be finite and ordered");
    }
    if state
        .material_ids
        .iter()
        .any(|m| *m == 0 || *m > state.mobile_immobile.len())
    {
        return Err("endpoint material IDs are out of range");
    }
    if state.source_species == 0 || state.dual_porosity > 2 {
        return Err("endpoint material/species selectors are invalid");
    }
    if !matches!(state.t_level, 1 | 2) {
        return Err("endpoint selectors are invalid");
    }
    if !matches!(lower_code, -1..=1) || !matches!(upper_code, -2..=1) {
        return Err("unsupported source endpoint boundary code");
    }
    if state.old_accepted_flow.len() != n
        || state.new_accepted_flow.len() != n
        || state.old_matrix_velocity.len() != n
        || state.new_matrix_velocity.len() != n
        || state.old_accepted_concentration.len() != n
        || state.new_accepted_concentration.len() != n
        || state.material_immobile_water.len() != state.mobile_immobile.len()
    {
        return Err("accepted endpoint flow/concentration dimensions are inconsistent");
    }
    if state
        .old_accepted_concentration
        .iter()
        .chain(&state.new_accepted_concentration)
        .any(|v| !v.0.is_finite())
        || accepted_concentration.iter().any(|v| !v.is_finite())
    {
        return Err("endpoint accepted concentrations must be finite");
    }
    if !bottom_concentration.is_finite() || !top_concentration.is_finite() {
        return Err("prescribed endpoint concentrations must be finite");
    }
    let mut state = state.clone();
    state.bottom_prescribed = super::types::DefaultReal(bottom_concentration);
    state.top_prescribed = super::types::DefaultReal(top_concentration);
    let lower = endpoint_row(&state, level, true, lower_code)?;
    let upper = endpoint_row(&state, level, false, upper_code)?;
    let flux = endpoint_flux(
        &state,
        level,
        lower_code,
        upper_code,
        accepted_concentration,
    )?;
    Ok(EndpointResult { lower, upper, flux })
}

fn endpoint_row(
    s: &EndpointPrimitiveState,
    level: AssemblyLevel,
    bottom: bool,
    code: i32,
) -> Result<EndpointRow, &'static str> {
    let n = s.coordinates.len();
    let (i, j, distance) = if bottom {
        (0, 1, s.coordinates[1].0 - s.coordinates[0].0)
    } else {
        (
            n - 2,
            n - 1,
            s.coordinates[n - 1].0 - s.coordinates[n - 2].0,
        )
    };
    if distance <= 0.0 {
        return Err("endpoint coordinates must increase in native order");
    }
    let conc = if bottom {
        s.bottom_prescribed.0
    } else {
        s.top_prescribed.0
    };
    let mat_i = s.material_ids[i] - 1;
    let mat_j = s.material_ids[j] - 1;
    let adjust_water_i = (s.mobile_immobile[mat_i] && s.dual_porosity == 0) || s.bacteria;
    let adjust_water_j = (s.mobile_immobile[mat_j] && s.dual_porosity == 0) || s.bacteria;
    let old_water = |node: usize, mat: usize, adjust: bool| {
        if adjust {
            (s.old_water[node].0 - s.material_immobile_water[mat].0).max(0.001)
        } else {
            s.old_water[node].0
        }
    };
    let new_water = |node: usize, mat: usize, adjust: bool| {
        if adjust {
            (s.new_water[node].0 - s.material_immobile_water[mat].0).max(0.001)
        } else {
            s.new_water[node].0
        }
    };
    let prescribed_side = conc;

    if bottom {
        match level {
            AssemblyLevel::Old => {
                let c1 = s.old_concentration[i].0;
                let c2 = s.old_concentration[j].0;
                let disp = (old_water(i, mat_i, adjust_water_i) * s.old_dispersion[i].0
                    + old_water(j, mat_j, adjust_water_j) * s.old_dispersion[j].0)
                    / distance
                    / 2.0;
                let adv = ((2.0 + 3.0 * s.upwind_weight[i].0) * s.old_velocity[i].0
                    + s.old_velocity[j].0)
                    / 6.0;
                let grad = distance / 12.0 * (3.0 * s.source_g1[i].0 + s.source_g1[j].0);
                let c2_term = (old_water(i, mat_i, adjust_water_i) * s.old_dispersion[i].0
                    + old_water(j, mat_j, adjust_water_j) * s.old_dispersion[j].0)
                    / distance
                    / 2.0
                    - (s.old_velocity[i].0
                        + (2.0 - 3.0 * s.upwind_weight[i].0) * s.old_velocity[j].0)
                        / 6.0
                    + distance / 12.0 * (s.source_g1[i].0 + s.source_g1[j].0);
                let source = s.alf.0 * distance / 6.0 * (2.0 * s.source_g0[i].0 + s.source_g0[j].0);
                let mass = distance / 2.0 / s.dt.0
                    * old_water(i, mat_i, adjust_water_i)
                    * s.old_retardation[i].0;
                let rhs =
                    c1 * (mass + s.alf.0 * (-disp - adv + grad)) + c2 * s.alf.0 * c2_term + source;
                let rhs = if code == -1 {
                    rhs + s.alf.0 * prescribed_side * s.old_matrix_velocity[i].0
                } else {
                    rhs
                };
                let rhs = if code == 1 {
                    s.old_accepted_concentration[i].0
                } else {
                    rhs
                };
                Ok(EndpointRow {
                    rhs: rhs as f64,
                    ..EndpointRow::default()
                })
            }
            AssemblyLevel::New => {
                let mut row = EndpointRow::default();
                if code == 0
                    || code == -1
                        && !(s.new_matrix_velocity[i].0 > 0.0
                            || (s.vapor && s.bottom_resistance.0 == 0.0))
                {
                    row.diagonal = -1.0;
                    row.upper = 1.0;
                    return Ok(row);
                }
                let avg = (new_water(i, mat_i, adjust_water_i) * s.new_dispersion[i].0
                    + new_water(j, mat_j, adjust_water_j) * s.new_dispersion[j].0)
                    / distance
                    / 2.0;
                let neighbor = s.epsi.0
                    * (-avg
                        + (s.new_matrix_velocity[i].0
                            + (2.0 - 3.0 * s.upwind_weight[i].0) * s.new_matrix_velocity[j].0)
                            / 6.0
                        - distance / 12.0 * (s.source_g1[i].0 + s.source_g1[j].0));
                let diagonal = distance / 2.0 / s.dt.0
                    * new_water(i, mat_i, adjust_water_i)
                    * s.new_retardation[i].0
                    + s.epsi.0
                        * (avg
                            + ((2.0 + 3.0 * s.upwind_weight[i].0) * s.new_matrix_velocity[i].0
                                + s.new_matrix_velocity[j].0)
                                / 6.0
                            - distance / 12.0 * (3.0 * s.source_g1[i].0 + s.source_g1[j].0));
                let source =
                    s.epsi.0 * distance / 6.0 * (2.0 * s.source_g0[i].0 + s.source_g0[j].0);
                row.diagonal = diagonal as f64;
                row.upper = neighbor as f64;
                row.rhs = source as f64;
                Ok(row)
            }
        }
    } else {
        match level {
            AssemblyLevel::Old => {
                let c1 = s.old_concentration[i].0;
                let c2 = s.old_concentration[j].0;
                let adv_left = ((2.0 + 3.0 * s.upwind_weight[i].0) * s.old_velocity[i].0
                    + s.old_velocity[j].0)
                    / 6.0;
                let diff = (old_water(i, mat_i, adjust_water_i) * s.old_dispersion[i].0
                    + old_water(j, mat_j, adjust_water_j) * s.old_dispersion[j].0)
                    / distance
                    / 2.0;
                let mass = distance / 2.0 / s.dt.0
                    * old_water(j, mat_j, adjust_water_j)
                    * s.old_retardation[j].0;
                let rhs = c1
                    * s.alf.0
                    * (diff + adv_left + distance / 12.0 * (s.source_g1[i].0 + s.source_g1[j].0))
                    + c2 * (mass
                        + s.alf.0
                            * (-diff
                                + (s.old_velocity[i].0
                                    + (2.0 - 3.0 * s.upwind_weight[i].0) * s.old_velocity[j].0)
                                    / 6.0
                                + distance / 12.0 * (s.source_g1[i].0 + 3.0 * s.source_g1[j].0)))
                    + s.alf.0 * distance / 6.0 * (s.source_g0[i].0 + 2.0 * s.source_g0[j].0);
                let mut row = EndpointRow {
                    rhs: rhs as f64,
                    ..EndpointRow::default()
                };
                if s.old_matrix_velocity[j].0 < 0.0 {
                    row.rhs -= (s.alf.0 * s.old_matrix_velocity[j].0 * prescribed_side) as f64;
                }
                if code == -2 {
                    row.rhs -= atmospheric(s, j, s.old_concentration[j].0)? as f64;
                }
                Ok(row)
            }
            AssemblyLevel::New => {
                let avg = (new_water(i, mat_i, adjust_water_i) * s.new_dispersion[i].0
                    + new_water(j, mat_j, adjust_water_j) * s.new_dispersion[j].0)
                    / distance
                    / 2.0;
                let lower = s.epsi.0
                    * (-avg
                        - ((2.0 + 3.0 * s.upwind_weight[i].0) * s.new_matrix_velocity[i].0
                            + s.new_matrix_velocity[j].0)
                            / 6.0
                        - distance / 12.0 * (s.source_g1[i].0 + s.source_g1[j].0));
                let diagonal = distance / 2.0 / s.dt.0
                    * new_water(j, mat_j, adjust_water_j)
                    * s.new_retardation[j].0
                    + s.epsi.0
                        * (avg
                            - (s.new_matrix_velocity[i].0
                                + (2.0 - 3.0 * s.upwind_weight[i].0) * s.new_matrix_velocity[j].0)
                                / 6.0
                            - distance / 12.0 * (s.source_g1[i].0 + 3.0 * s.source_g1[j].0));
                let source =
                    s.epsi.0 * distance / 6.0 * (s.source_g0[i].0 + 2.0 * s.source_g0[j].0);
                let mut row = EndpointRow {
                    lower: lower as f64,
                    diagonal: diagonal as f64,
                    rhs: source as f64,
                    ..EndpointRow::default()
                };
                if code != -2 && s.new_matrix_velocity[j].0 < 0.0 {
                    row.rhs -= (s.epsi.0 * s.new_matrix_velocity[j].0 * prescribed_side) as f64;
                }
                if code == -2 {
                    let (transfer, atmosphere) = atmospheric_terms(s, level, j)?;
                    row.diagonal += transfer as f64;
                    row.rhs -= atmosphere as f64;
                }
                Ok(row)
            }
        }
    }
}

fn atmospheric(
    s: &EndpointPrimitiveState,
    node: usize,
    concentration: f32,
) -> Result<f32, &'static str> {
    let temp = s
        .old_temperature
        .ok_or("old temperature required for atmospheric boundary")?
        .0;
    let dg = s
        .source_chpar
        .as_ref()
        .ok_or("source ChPar required for atmospheric boundary")?;
    let td = s
        .source_tdep
        .as_ref()
        .ok_or("source TDep required for atmospheric boundary")?;
    let material = s.material_ids[node] - 1;
    let species_base = (s.source_species - 1) * 16;
    let material_base = species_base + material * 20;
    if s.source_species == 0 || dg.len() < material_base + 10 || td.len() < species_base + 16 {
        return Err("atmospheric source coefficient arrays are incomplete");
    }
    let surface = s
        .top_surface_distance
        .ok_or("top surface distance required")?
        .0;
    let temp_k = temp + 273.15;
    let tt = (temp_k - 293.15) / 8.314 / temp_k / 293.15;
    let dgas = dg[material_base + 5].0 * (td[species_base + 5].0 * tt).exp();
    let henry = dg[material_base + 9].0 * (td[species_base + 9].0 * tt).exp();
    Ok(s.alf.0 * dgas / surface * henry * concentration)
}

fn atmospheric_terms(
    s: &EndpointPrimitiveState,
    level: AssemblyLevel,
    node: usize,
) -> Result<(f32, f32), &'static str> {
    let temp = match level {
        AssemblyLevel::Old => s.old_temperature.ok_or("old temperature required")?.0,
        AssemblyLevel::New => s.new_temperature.ok_or("new temperature required")?.0,
    };
    let dg = s.source_chpar.as_ref().ok_or("source ChPar required")?;
    let td = s.source_tdep.as_ref().ok_or("source TDep required")?;
    let material = s.material_ids[node] - 1;
    let species_base = (s.source_species - 1) * 16;
    let material_base = species_base + material * 20;
    if s.source_species == 0 || dg.len() < material_base + 10 || td.len() < species_base + 16 {
        return Err("atmospheric source coefficient arrays are incomplete");
    }
    let temp_k = temp + 273.15;
    let tt = (temp_k - 293.15) / 8.314 / temp_k / 293.15;
    let dgas = dg[material_base + 5].0 * (td[species_base + 5].0 * tt).exp();
    let henry = dg[material_base + 9].0 * (td[species_base + 9].0 * tt).exp();
    let surface = s
        .top_surface_distance
        .ok_or("top surface distance required")?
        .0;
    let coefficient = dgas / surface * henry;
    let atmospheric = dgas / surface
        * s.top_atmospheric_concentration
            .ok_or("atmospheric concentration required")?
            .0;
    Ok((s.epsi.0 * coefficient, atmospheric))
}

/// Solute flux accounting uses `TempN`, `dSurfT`, and epsi for both atmospheric terms.
fn atmospheric_flux_terms(
    s: &EndpointPrimitiveState,
    node: usize,
) -> Result<(f32, f32), &'static str> {
    let temp = s.new_temperature.ok_or("new temperature required")?.0;
    let dg = s.source_chpar.as_ref().ok_or("source ChPar required")?;
    let td = s.source_tdep.as_ref().ok_or("source TDep required")?;
    let species_base = (s.source_species - 1) * 16;
    let material_base = species_base + (s.material_ids[node] - 1) * 20;
    if s.source_species == 0 || dg.len() < material_base + 10 || td.len() < species_base + 10 {
        return Err("atmospheric source coefficient arrays are incomplete");
    }
    let temp_k = temp + 273.15;
    let tt = (temp_k - 293.15) / 8.314 / temp_k / 293.15;
    let dgas = dg[material_base + 5].0 * (td[species_base + 5].0 * tt).exp();
    let henry = dg[material_base + 9].0 * (td[species_base + 9].0 * tt).exp();
    let surface = s
        .top_surface_distance
        .ok_or("top surface distance required")?
        .0
        * (td[species_base + 8].0 * tt).exp();
    let c_atm = s
        .top_atmospheric_concentration
        .ok_or("atmospheric concentration required")?
        .0;
    Ok((s.epsi.0 * dgas / surface * henry, dgas / surface * c_atm))
}

fn endpoint_flux(
    s: &EndpointPrimitiveState,
    level: AssemblyLevel,
    bottom_code: i32,
    top_code: i32,
    accepted: &[f32],
) -> Result<EndpointFlux, &'static str> {
    let n = s.coordinates.len();
    let flow_b = s.new_velocity[0].0;
    let flow_t = s.new_velocity[n - 1].0;
    let mut bottom = if bottom_code < 0 {
        let mut value = if flow_b >= 0.0 {
            s.epsi.0 * s.bottom_prescribed.0 * flow_b
        } else {
            s.epsi.0 * accepted[0] * flow_b
        };
        if s.vapor && s.bottom_resistance.0 == 0.0 {
            value = 0.0;
        }
        value
    } else if bottom_code == 0 {
        s.epsi.0 * accepted[0] * flow_b
    } else {
        let row = endpoint_row(s, AssemblyLevel::New, true, bottom_code)?;
        row.diagonal as f32 * accepted[0] + row.upper as f32 * accepted[1] - row.rhs as f32
    };
    let mut top = 0.0_f32;
    if top_code < 0 {
        if level == AssemblyLevel::Old {
            if s.t_level != 1 && flow_t < 0.0 {
                top = s.alf.0 * flow_t * s.top_prescribed.0;
            }
        } else if flow_t < 0.0 {
            top = (if s.t_level == 1 { 1.0 } else { s.epsi.0 }) * flow_t * s.top_prescribed.0;
        }
        if top_code == -2 {
            let (transfer, atmosphere) = atmospheric_flux_terms(s, n - 1)?;
            top += transfer * accepted[n - 1] - atmosphere;
        }
    } else {
        let row = endpoint_row(s, AssemblyLevel::New, false, top_code)?;
        top = row.rhs as f32
            - row.lower as f32 * accepted[n - 2]
            - row.diagonal as f32 * accepted[n - 1];
    }
    if bottom.abs() < s.r_min.0 {
        bottom = 0.0;
    }
    if top.abs() < s.r_min.0 {
        top = 0.0;
    }
    Ok(EndpointFlux {
        top: top as f64,
        bottom: bottom as f64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solute_fortran_compat::types::DefaultReal as R;

    fn r(values: &[f32]) -> Vec<R> {
        values.iter().copied().map(R).collect()
    }

    fn fixture() -> EndpointPrimitiveState {
        EndpointPrimitiveState {
            coordinates: r(&[0.0, 2.0, 5.0]),
            old_water: r(&[0.4, 0.5, 0.6]),
            new_water: r(&[0.45, 0.55, 0.65]),
            old_velocity: r(&[0.3, 0.2, -0.4]),
            new_velocity: r(&[0.35, 0.25, -0.5]),
            old_dispersion: r(&[0.1, 0.2, 0.3]),
            new_dispersion: r(&[0.12, 0.22, 0.32]),
            old_retardation: r(&[1.1, 1.2, 1.3]),
            new_retardation: r(&[1.15, 1.25, 1.35]),
            old_concentration: r(&[1.0, 2.0, 3.0]),
            material_immobile_water: r(&[0.0]),
            source_g0: r(&[0.01, 0.02, 0.03]),
            source_g1: r(&[0.04, 0.05, 0.06]),
            upwind_weight: r(&[0.2, 0.3]),
            epsi: R(0.6),
            dt: R(2.0),
            alf: R(0.4),
            top_atmospheric_concentration: Some(R(0.7)),
            top_surface_distance: Some(R(3.0)),
            henry_coefficient: None,
            gas_diffusion: None,
            old_temperature: Some(R(20.0)),
            new_temperature: Some(R(22.0)),
            source_chpar: Some(r(&[0.0; 20])),
            source_tdep: Some(r(&[0.0; 16])),
            material_ids: vec![1, 1, 1],
            source_species: 1,
            mobile_immobile: vec![false],
            bacteria: false,
            dual_porosity: 0,
            bottom_resistance: R(1.0),
            bottom_prescribed: R(4.0),
            top_prescribed: R(5.0),
            old_accepted_concentration: r(&[1.0, 2.0, 3.0]),
            new_accepted_concentration: r(&[1.5, 2.5, 3.5]),
            old_accepted_flow: r(&[0.3, 0.2, -0.4]),
            new_accepted_flow: r(&[0.35, 0.25, -0.5]),
            old_matrix_velocity: r(&[0.3, 0.2, -0.4]),
            new_matrix_velocity: r(&[0.35, 0.25, -0.5]),
            t_level: 2,
            vapor: false,
            r_min: R(1e-30),
        }
    }

    #[test]
    fn new_level_lower_type_three_uses_source_flow_predicate_and_rhs() {
        let mut s = fixture();
        let got = endpoint_row(&s, AssemblyLevel::New, true, -1).unwrap();
        let h = 2.0_f32;
        let avg = (0.45 * 0.12 + 0.55 * 0.22) / h / 2.0;
        let e = 0.6 * (-avg + (0.35 + (2.0 - 3.0 * 0.2) * 0.25) / 6.0 - h / 12.0 * (0.04 + 0.05));
        let d = h / 2.0 / 2.0 * 0.45 * 1.15
            + 0.6
                * (avg + ((2.0 + 3.0 * 0.2) * 0.35 + 0.25) / 6.0 - h / 12.0 * (3.0 * 0.04 + 0.05));
        let f = 0.6 * h / 6.0 * (2.0 * 0.01 + 0.02);
        assert_eq!(got.diagonal, d as f64);
        assert_eq!(got.upper, e as f64);
        assert_eq!(got.rhs, f as f64);
        s.new_matrix_velocity[0] = R(-0.1);
        assert_eq!(
            endpoint_row(&s, AssemblyLevel::New, true, -1)
                .unwrap()
                .diagonal,
            -1.0
        );
    }

    #[test]
    fn new_upper_atmospheric_row_and_flux_keep_distinct_closures() {
        let s = fixture();
        let row = endpoint_row(&s, AssemblyLevel::New, false, -2).unwrap();
        let (henry, atmosphere) = atmospheric_terms(&s, AssemblyLevel::New, 2).unwrap();
        assert!(row.diagonal > henry as f64);
        let source = 0.6_f32 * 3.0 / 6.0 * (0.02 + 2.0 * 0.03);
        assert!((row.rhs - (source - atmosphere) as f64).abs() < 2e-7);
        let flux = endpoint_flux(&s, AssemblyLevel::New, 1, -2, &[1.5, 2.5, 3.5]).unwrap();
        let (flux_henry, flux_atmosphere) = atmospheric_flux_terms(&s, 2).unwrap();
        assert!(
            (flux.top - (0.6 * (-0.5) * 5.0 + flux_henry * 3.5 - flux_atmosphere) as f64).abs()
                < 1e-7
        );
        assert_eq!(henry, flux_henry);
    }

    #[test]
    fn old_type_three_vapor_exception_and_flux_tlevel_are_source_ordered() {
        let mut s = fixture();
        s.new_matrix_velocity[0] = R(-0.1);
        assert_eq!(
            endpoint_row(&s, AssemblyLevel::New, true, -1)
                .unwrap()
                .diagonal,
            -1.0
        );
        s.vapor = true;
        s.bottom_resistance = R(0.0);
        assert_ne!(
            endpoint_row(&s, AssemblyLevel::New, true, -1)
                .unwrap()
                .diagonal,
            -1.0
        );
        s.t_level = 1;
        s.vapor = false;
        let flux = endpoint_flux(&s, AssemblyLevel::Old, -1, -1, &[1.0, 2.0, 3.0]).unwrap();
        assert_eq!(flux.top, 0.0);
        assert!((flux.bottom - (0.6 * 4.0 * 0.3) as f64).abs() > 1e-7);
    }

    #[test]
    fn endpoint_kernel_rejects_bad_shapes_and_non_native_mesh_order() {
        let mut s = fixture();
        assert!(assemble_endpoints(&s, AssemblyLevel::New, -1, -1, 4.0, 5.0, &[1.0, 2.0]).is_err());
        s.coordinates[1] = R(-1.0);
        assert!(endpoint_row(&s, AssemblyLevel::New, true, -1).is_err());
    }

    #[test]
    fn orientation_maps_native_endpoints_without_swapping_physical_rows() {
        assert_eq!(
            canonical_endpoint(0, 3, false),
            Some(CanonicalEndpoint::Lower)
        );
        assert_eq!(
            canonical_endpoint(2, 3, false),
            Some(CanonicalEndpoint::Upper)
        );
        assert_eq!(
            canonical_endpoint(0, 3, true),
            Some(CanonicalEndpoint::Upper)
        );
        assert_eq!(
            canonical_endpoint(2, 3, true),
            Some(CanonicalEndpoint::Lower)
        );
        assert_eq!(canonical_endpoint(1, 3, true), None);
    }

    #[test]
    fn prescribed_and_zero_gradient_rows_follow_each_source_level() {
        let s = fixture();
        let old_bottom = endpoint_row(&s, AssemblyLevel::Old, true, 1).unwrap();
        let old_free = endpoint_row(&s, AssemblyLevel::Old, true, 0).unwrap();
        assert!((old_free.rhs - 0.12).abs() < 1e-7);
        assert_ne!(old_bottom.rhs, 0.0);
        let new_bottom = endpoint_row(&s, AssemblyLevel::New, true, 1).unwrap();
        assert_eq!(
            (new_bottom.diagonal, new_bottom.rhs),
            (0.3840000033378601, 0.00800000037997961)
        );
        let new_free = endpoint_row(&s, AssemblyLevel::New, true, 0).unwrap();
        assert_eq!(
            (new_free.diagonal, new_free.upper, new_free.rhs),
            (-1.0, 1.0, 0.0)
        );
        let upper_old_prescribed = endpoint_row(&s, AssemblyLevel::Old, false, 1).unwrap();
        let upper_new_prescribed = endpoint_row(&s, AssemblyLevel::New, false, 1).unwrap();
        assert_ne!(upper_old_prescribed.rhs, 0.0);
        assert_ne!(upper_new_prescribed.diagonal, 1.0);
        assert!(upper_new_prescribed.rhs.is_finite());
    }

    #[test]
    fn source_old_lower_type_three_rhs_adds_prescribed_old_flow() {
        let s = fixture();
        let natural = endpoint_row(&s, AssemblyLevel::Old, true, 0).unwrap();
        let type_three = endpoint_row(&s, AssemblyLevel::Old, true, -1).unwrap();
        assert!((type_three.rhs - natural.rhs - (0.4 * 4.0 * 0.3) as f64).abs() < 1e-7);
    }

    #[test]
    fn positive_endpoint_flux_is_exact_boundary_row_residual() {
        let s = fixture();
        for level in [AssemblyLevel::Old, AssemblyLevel::New] {
            let accepted = if level == AssemblyLevel::Old {
                [1.0, 2.0, 3.0]
            } else {
                [1.5, 2.5, 3.5]
            };
            let flux = endpoint_flux(&s, level, 1, 1, &accepted).unwrap();
            let bottom = endpoint_row(&s, AssemblyLevel::New, true, 1).unwrap();
            let top = endpoint_row(&s, AssemblyLevel::New, false, 1).unwrap();
            assert!(
                (flux.bottom
                    - (bottom.diagonal * accepted[0] as f64 + bottom.upper * accepted[1] as f64
                        - bottom.rhs))
                    .abs()
                    < 1e-6
            );
            assert!(
                (flux.top
                    - (top.rhs
                        - top.lower * accepted[1] as f64
                        - top.diagonal * accepted[2] as f64))
                    .abs()
                    < 1e-6
            );
        }
    }

    #[test]
    fn malformed_nonfinite_primitives_are_rejected() {
        let mut s = fixture();
        s.new_matrix_velocity[1] = R(f32::NAN);
        assert!(
            assemble_endpoints(&s, AssemblyLevel::New, -1, -1, 4.0, 5.0, &[1.0, 2.0, 3.0]).is_err()
        );
        s = fixture();
        s.material_immobile_water.clear();
        assert!(
            assemble_endpoints(&s, AssemblyLevel::New, -1, -1, 4.0, 5.0, &[1.0, 2.0, 3.0]).is_err()
        );
    }
}

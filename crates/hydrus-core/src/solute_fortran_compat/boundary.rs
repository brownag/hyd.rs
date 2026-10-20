//! Primitive-input endpoint equations and flux closure from `SOLUTE.FOR`.
//!
//! `MatSet` (755-939) declares `double precision B,D,E,F` (762) and every other
//! local as default REAL, so each coefficient product below stays in
//! [`DefaultReal`] and only the `B`/`D`/`E`/`F` accumulations widen, statement by
//! statement, as the source evaluates them.

use super::types::{EndpointPrimitiveState, KernelError};

/// `Tr` (`42`, `76`, `885`, `918`).
const REFERENCE_TEMPERATURE_K: f32 = 293.15;
/// `R` (`43`, `77`, `886`, `919`).
const GAS_CONSTANT: f32 = 8.314;
/// `ChPar(jjj+6, M)` and `TDep(jjj+6)`, the gas-diffusion pair (`79`, `889`, `922`).
const GAS_DIFFUSION_SLOT: usize = 6;
/// `TDep(jjj+9)`, the surface-distance scaling (`81`).
const SURFACE_DISTANCE_SLOT: usize = 9;
/// `ChPar(jjj+10, M)` and `TDep(jjj+10)`, the Henry pair (`80`, `890`, `923`).
const HENRY_SLOT: usize = 10;
/// Species stride of the flat `ChPar`/`TDep` reader layout, `jjj=(jS-1)*16`
/// (`53`, `INPUT.FOR:1341`, `1379`).
const SPECIES_STRIDE: usize = 16;

/// One endpoint equation as `BanSol` consumes it (`144`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EndpointRow {
    /// `B(node)`, the sub-diagonal (`762`).
    pub lower: f64,
    /// `D(node)`, the main diagonal (`762`).
    pub diagonal: f64,
    /// `E(node)`, the super-diagonal (`762`).
    pub upper: f64,
    /// `F(node)`, the right-hand side (`762`).
    pub rhs: f64,
}

/// One endpoint equation, once per `MatSet` assembly level and once assembled.
///
/// `Solute` calls `MatSet` per `Level` (`107`, `124-128`) and the second call
/// accumulates into the `F` values the first one wrote (`792` through `800` and
/// `814`, `881-891` through `902` and `914-915`), so a caller cannot compose the
/// two levels and uses [`EndpointNode::assembled`]. The two level rows are the
/// natural coefficient sets those levels compute: `Level=1` only ever writes
/// `F(1)`/`F(N)` (`792`, `881-891`), so `B`/`D`/`E` stay zero there, and
/// `Level=2` contributes `E1`/`D1`/`F2` (`794-799`) and `BN`/`DN`/`FE`
/// (`895-901`) before the boundary codes overwrite them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EndpointNode {
    /// `F1` (`781-789`) or `FN` (`869-877`).
    pub level_old: EndpointRow,
    /// `E1`/`D1`/`F2` (`794-799`) or `BN`/`DN`/`FE` (`895-901`).
    pub level_new: EndpointRow,
    /// The equation `BanSol` solves.
    pub assembled: EndpointRow,
}

/// The step's `cvBot(jS)`/`cvTop(jS)` (`54-55`, `224-225`), widened for
/// bookkeeping; `Solute` keeps one pair per species per step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EndpointFlux {
    pub bottom: f64,
    pub top: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EndpointResult {
    pub lower: EndpointNode,
    pub upper: EndpointNode,
    pub flux: EndpointFlux,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CanonicalEndpoint {
    Lower,
    Upper,
}

#[cfg(test)]
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

/// Assembles the native node 1 and node `N` equations and the step flux of one
/// species, from primitives only.
///
/// `lower_code`/`upper_code` are `kBotCh`/`kTopCh`. Velocities follow the
/// source: both `MatSet` levels read the velocities before `vCorr` is added
/// (`124-132`), flux phase A reads `vO` from before the level loop (`65-72`),
/// and flux phase B reads `vN` after this step's `vCorr` restoration
/// (`129-132`, `197-221`).
pub(crate) fn assemble_endpoints(
    state: &EndpointPrimitiveState,
    lower_code: i32,
    upper_code: i32,
) -> Result<EndpointResult, KernelError> {
    validate(state, lower_code, upper_code)?;
    let lower = natural_lower(state);
    let upper = natural_upper(state);
    Ok(EndpointResult {
        lower: assemble_lower(state, &lower, lower_code),
        upper: assemble_upper(state, &upper, upper_code)?,
        flux: endpoint_flux(state, &lower, &upper, lower_code, upper_code)?,
    })
}

pub(super) fn validate(
    state: &EndpointPrimitiveState,
    lower_code: i32,
    upper_code: i32,
) -> Result<(), KernelError> {
    let n = state.coordinates.len();
    if n < 2 {
        return Err("the endpoint kernel needs at least two nodes");
    }
    for values in [
        &state.coordinates,
        &state.old_water,
        &state.new_water,
        &state.old_velocity,
        &state.new_velocity,
        &state.old_matrix_velocity,
        &state.new_matrix_velocity,
        &state.old_dispersion,
        &state.new_dispersion,
        &state.old_retardation,
        &state.new_retardation,
        &state.old_concentration,
        &state.new_accepted_concentration,
        &state.old_source_g0,
        &state.old_source_g1,
        &state.new_source_g0,
        &state.new_source_g1,
    ] {
        if values.len() != n {
            return Err("endpoint primitives need one value per node");
        }
    }
    if state.material_ids.len() != n {
        return Err("endpoint primitives need one material id per node");
    }
    if state.upwind_weight.len() != n - 1 {
        return Err("endpoint upwind_weight needs one value per element");
    }
    if state.mobile_immobile.is_empty()
        || state.material_immobile_water.len() != state.mobile_immobile.len()
    {
        return Err("endpoint primitives need one immobile-water record per material");
    }
    if state
        .material_ids
        .iter()
        .any(|m| *m == 0 || *m > state.mobile_immobile.len())
    {
        return Err("endpoint material_ids must index the mobility records");
    }
    if state.coordinates.iter().any(|x| !x.0.is_finite())
        || state.coordinates.windows(2).any(|w| w[1].0 <= w[0].0)
    {
        return Err("endpoint coordinates must be finite and increase in native node order");
    }
    if state.source_species == 0 || !matches!(state.t_level, 1 | 2) || state.dual_porosity > 2 {
        return Err("endpoint source_species and dual_porosity/TLevel selectors are invalid");
    }
    if !matches!(lower_code, -1..=1) || !matches!(upper_code, -2..=1) {
        return Err("unsupported source endpoint boundary code");
    }
    if state.dt.0 <= 0.0 || state.r_min.0 < 0.0 {
        return Err("endpoint dt must be positive and rMin non-negative");
    }
    if !(0.0..=1.0).contains(&state.epsi.0)
        || (state.alf.0 - (1.0 - state.epsi.0)).abs() > 1.0e-6
        || state.bottom_resistance.0 < 0.0
    {
        return Err("endpoint epsi must lie in [0, 1], alf must equal 1 - epsi, and bottom resistance must be non-negative");
    }
    // NaN and inf primitives are not rejected here. The pinned Fortran propagates
    // IEEE non-finite values (ADR 0017), so the kernel must too.
    Ok(())
}

/// `thO(i)` after the `768-776` adjustment.
fn old_water(state: &EndpointPrimitiveState, node: usize) -> f32 {
    adjusted_water(state.old_water[node].0, state, node)
}

/// `thN(i)` after the `768-776` adjustment.
fn new_water(state: &EndpointPrimitiveState, node: usize) -> f32 {
    adjusted_water(state.new_water[node].0, state, node)
}

/// `th(i)=max(th(i)-ChPar(4,M),0.001)` under
/// `(lMobIm(M).and.iDualPor.eq.0).or.lBact` (`768-776`), where `.and.` binds
/// tighter than `.or.`. `MatSet` restores both waters at `929-936`, so the two
/// assembly levels see the same adjustment.
fn adjusted_water(value: f32, state: &EndpointPrimitiveState, node: usize) -> f32 {
    let material = state.material_ids[node] - 1;
    let mobile_immobile =
        state.mobile_immobile[material] && state.dual_porosity == 0 || state.bacteria;
    if mobile_immobile {
        (value - state.material_immobile_water[material].0).max(0.001)
    } else {
        value
    }
}

/// The natural lower-endpoint coefficients, i.e. the `MatSet` statements that
/// every `kBotCh` reaches.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct NaturalLower {
    /// `F1` (`781-789`).
    f1: f32,
    /// `E1` (`794-795`).
    e1: f32,
    /// `D1` (`796-798`).
    d1: f32,
    /// `F2` (`799`).
    f2: f32,
    /// `F1=F1+F2` (`800`), the value the flux closure subtracts at `221`.
    f1_total: f32,
}

/// The natural upper-endpoint coefficients.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct NaturalUpper {
    /// `FN` (`869-877`).
    f_n: f32,
    /// `BN` (`895-896`).
    b_n: f32,
    /// `DN` (`897-900`).
    d_n: f32,
    /// `FE` (`901`).
    f_e: f32,
    /// `FN=FN+FE` (`902`), the value the flux closure uses at `204`.
    f_n_total: f32,
}

/// `F1` (`781-789`), `E1`/`D1`/`F2` (`794-799`) and their sum (`800`), with
/// `b1=x(2)-x(1)` (`779`).
fn natural_lower(state: &EndpointPrimitiveState) -> NaturalLower {
    let (i, j) = (0usize, 1usize);
    let b1 = state.coordinates[j].0 - state.coordinates[i].0;
    let (th_o_i, th_o_j) = (old_water(state, i), old_water(state, j));
    let (th_n_i, th_n_j) = (new_water(state, i), new_water(state, j));
    // `(thO(1)*Disp(1)+thO(2)*Disp(2))/b1/2.` (`783`, `787`)
    let old_face =
        (th_o_i * state.old_dispersion[i].0 + th_o_j * state.old_dispersion[j].0) / b1 / 2.0;
    // `(thN(1)*Disp(1)+thN(2)*Disp(2))/b1/2.` (`794`, `797`)
    let new_face =
        (th_n_i * state.new_dispersion[i].0 + th_n_j * state.new_dispersion[j].0) / b1 / 2.0;
    let wc = state.upwind_weight[i].0;
    let (vo_i, vo_j) = (
        state.old_matrix_velocity[i].0,
        state.old_matrix_velocity[j].0,
    );
    let (vn_i, vn_j) = (
        state.new_matrix_velocity[i].0,
        state.new_matrix_velocity[j].0,
    );
    // `Level=1` `MatSet` saw the old `Coeff` pair, `Level=2` the new one (`124-128`).
    let (g0o_i, g0o_j) = (state.old_source_g0[i].0, state.old_source_g0[j].0);
    let (g1o_i, g1o_j) = (state.old_source_g1[i].0, state.old_source_g1[j].0);
    let (g0_i, g0_j) = (state.new_source_g0[i].0, state.new_source_g0[j].0);
    let (g1_i, g1_j) = (state.new_source_g1[i].0, state.new_source_g1[j].0);
    let f1 = state.old_concentration[i].0
        * (b1 / 2.0 / state.dt.0 * th_o_i * state.old_retardation[i].0
            + state.alf.0
                * (-old_face - ((2.0 + 3.0 * wc) * vo_i + vo_j) / 6.0
                    + b1 / 12.0 * (3.0 * g1o_i + g1o_j)))
        + state.old_concentration[j].0
            * state.alf.0
            * (old_face - (vo_i + (2.0 - 3.0 * wc) * vo_j) / 6.0 + b1 / 12.0 * (g1o_i + g1o_j))
        + state.alf.0 * b1 / 6.0 * (2.0 * g0o_i + g0o_j);
    let e1 = state.epsi.0
        * (-new_face + (vn_i + (2.0 - 3.0 * wc) * vn_j) / 6.0 - b1 / 12.0 * (g1_i + g1_j));
    let d1 = b1 / 2.0 / state.dt.0 * th_n_i * state.new_retardation[i].0
        + state.epsi.0
            * (new_face + ((2.0 + 3.0 * wc) * vn_i + vn_j) / 6.0 - b1 / 12.0 * (3.0 * g1_i + g1_j));
    let f2 = state.epsi.0 * b1 / 6.0 * (2.0 * g0_i + g0_j);
    NaturalLower {
        f1,
        e1,
        d1,
        f2,
        f1_total: f1 + f2,
    }
}

/// `FN` (`869-877`), `BN`/`DN`/`FE` (`895-901`) and their sum (`902`), with the
/// `b1=x(N)-x(N-1)` the interior loop leaves behind (`832`).
fn natural_upper(state: &EndpointPrimitiveState) -> NaturalUpper {
    let n = state.coordinates.len();
    let (i, j) = (n - 2, n - 1);
    let b1 = state.coordinates[j].0 - state.coordinates[i].0;
    let (th_o_i, th_o_j) = (old_water(state, i), old_water(state, j));
    let (th_n_i, th_n_j) = (new_water(state, i), new_water(state, j));
    let old_face =
        (th_o_i * state.old_dispersion[i].0 + th_o_j * state.old_dispersion[j].0) / b1 / 2.0;
    let new_face =
        (th_n_i * state.new_dispersion[i].0 + th_n_j * state.new_dispersion[j].0) / b1 / 2.0;
    let wc = state.upwind_weight[i].0;
    let (vo_i, vo_j) = (
        state.old_matrix_velocity[i].0,
        state.old_matrix_velocity[j].0,
    );
    let (vn_i, vn_j) = (
        state.new_matrix_velocity[i].0,
        state.new_matrix_velocity[j].0,
    );
    // `Level=1` `MatSet` saw the old `Coeff` pair, `Level=2` the new one (`124-128`).
    let (g0o_i, g0o_j) = (state.old_source_g0[i].0, state.old_source_g0[j].0);
    let (g1o_i, g1o_j) = (state.old_source_g1[i].0, state.old_source_g1[j].0);
    let (g0_i, g0_j) = (state.new_source_g0[i].0, state.new_source_g0[j].0);
    let (g1_i, g1_j) = (state.new_source_g1[i].0, state.new_source_g1[j].0);
    let f_n = state.old_concentration[i].0
        * state.alf.0
        * (old_face + ((2.0 + 3.0 * wc) * vo_i + vo_j) / 6.0 + b1 / 12.0 * (g1o_i + g1o_j))
        + state.old_concentration[j].0
            * (b1 / 2.0 / state.dt.0 * th_o_j * state.old_retardation[j].0
                + state.alf.0
                    * (-old_face
                        + (vo_i + (2.0 - 3.0 * wc) * vo_j) / 6.0
                        + b1 / 12.0 * (g1o_i + 3.0 * g1o_j)))
        + state.alf.0 * b1 / 6.0 * (g0o_i + 2.0 * g0o_j);
    let b_n = state.epsi.0
        * (-new_face - ((2.0 + 3.0 * wc) * vn_i + vn_j) / 6.0 - b1 / 12.0 * (g1_i + g1_j));
    let d_n = b1 / 2.0 / state.dt.0 * th_n_j * state.new_retardation[j].0
        + state.epsi.0
            * (new_face - (vn_i + (2.0 - 3.0 * wc) * vn_j) / 6.0 - b1 / 12.0 * (g1_i + 3.0 * g1_j));
    let f_e = state.epsi.0 * b1 / 6.0 * (g0_i + 2.0 * g0_j);
    NaturalUpper {
        f_n,
        b_n,
        d_n,
        f_e,
        f_n_total: f_n + f_e,
    }
}

/// The node-1 equation: `kBotCh` 1 (`803-807`), 0 (`823-827`) and -1 (`810-820`),
/// including the `F(1)` the `Level=1` call already wrote.
fn assemble_lower(
    state: &EndpointPrimitiveState,
    natural: &NaturalLower,
    code: i32,
) -> EndpointNode {
    let prescribed = state.bottom_prescribed.0;
    let level_old = if code == -1 {
        // `792` evaluates its right-hand side entirely in default REAL and only
        // then stores it into the double `F(1)` (`762`).
        EndpointRow {
            rhs: (natural.f1 + state.alf.0 * prescribed * state.old_matrix_velocity[0].0) as f64,
            ..EndpointRow::default()
        }
    } else {
        EndpointRow {
            rhs: natural.f1 as f64,
            ..EndpointRow::default()
        }
    };
    let level_new = EndpointRow {
        diagonal: natural.d1 as f64,
        upper: natural.e1 as f64,
        rhs: natural.f2 as f64,
        ..EndpointRow::default()
    };
    // `811`: a strict `vN(1).gt.0.` test, or the vapor zero-resistance exception.
    let inflow =
        state.new_matrix_velocity[0].0 > 0.0 || (state.vapor && state.bottom_resistance.0 == 0.0);
    let mut assembled = EndpointRow {
        diagonal: natural.d1 as f64,
        upper: natural.e1 as f64,
        rhs: level_old.rhs,
        ..EndpointRow::default()
    };
    if code == -1 && inflow {
        // `814` widens both REAL terms into the double accumulator in statement
        // order, on top of whatever `792` stored.
        assembled.rhs += natural.f2 as f64;
        assembled.rhs += (state.epsi.0 * prescribed * state.new_matrix_velocity[0].0) as f64;
    } else if code == 1 {
        assembled = EndpointRow {
            diagonal: 1.0,
            rhs: prescribed as f64,
            ..EndpointRow::default()
        };
    } else if code == 0 || !inflow {
        assembled = EndpointRow {
            diagonal: -1.0,
            upper: 1.0,
            ..EndpointRow::default()
        };
    }
    EndpointNode {
        level_old,
        level_new,
        assembled,
    }
}

/// The node-`N` equation: `kTopCh` 1 (`905-909`), 0 and -1 (`911-915`), and the
/// atmospheric -2 (`883-892`, `916-925`).
fn assemble_upper(
    state: &EndpointPrimitiveState,
    natural: &NaturalUpper,
    code: i32,
) -> Result<EndpointNode, KernelError> {
    let top = state.coordinates.len() - 1;
    let prescribed = state.top_prescribed.0;
    let mut level_old = EndpointRow {
        rhs: natural.f_n as f64,
        ..EndpointRow::default()
    };
    // `880` guards every `Level=1` write to `F(N)`, so `kTopCh=1` reaches none of
    // them and keeps only the natural `FN` for reporting.
    if code <= 0 {
        if state.old_matrix_velocity[top].0 < 0.0 {
            level_old.rhs -= (state.alf.0 * state.old_matrix_velocity[top].0 * prescribed) as f64;
        }
        if code == -2 {
            let atmo = atmosphere(state)?;
            level_old.rhs -= (state.alf.0 * atmo.diffusion_old / atmo.surface_distance
                * atmo.henry_old
                * state.old_concentration[top].0) as f64;
            level_old.rhs +=
                (atmo.diffusion_old / atmo.surface_distance * atmo.concentration) as f64;
        }
    }
    let level_new = EndpointRow {
        lower: natural.b_n as f64,
        diagonal: natural.d_n as f64,
        rhs: natural.f_e as f64,
        ..EndpointRow::default()
    };
    let assembled = if code > 0 {
        EndpointRow {
            diagonal: 1.0,
            rhs: prescribed as f64,
            ..EndpointRow::default()
        }
    } else {
        let mut row = EndpointRow {
            lower: natural.b_n as f64,
            diagonal: natural.d_n as f64,
            rhs: level_old.rhs,
            ..EndpointRow::default()
        };
        row.rhs += natural.f_e as f64;
        if state.new_matrix_velocity[top].0 < 0.0 {
            row.rhs -= (state.epsi.0 * state.new_matrix_velocity[top].0 * prescribed) as f64;
        }
        if code == -2 {
            let atmo = atmosphere(state)?;
            row.diagonal +=
                (state.epsi.0 * atmo.diffusion_new / atmo.surface_distance * atmo.henry_new) as f64;
        }
        row
    };
    Ok(EndpointNode {
        level_old,
        level_new,
        assembled,
    })
}

/// `Dg`, `Henry`, `dSurfT` and `cAtm` for the `kTopCh=-2` exchange (`74-84`).
///
/// `dSurfT` uses the `TempO` `TT` of `81` and is what every later site divides
/// by: `Solute` passes `dSurfT` as `MatSet`'s `dSurf` argument (`127`), so `891`
/// and `924` reuse it unchanged even though `921` recomputes `TT` from `TempN`
/// for `Dg` and `Henry`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Atmosphere {
    surface_distance: f32,
    diffusion_old: f32,
    henry_old: f32,
    diffusion_new: f32,
    henry_new: f32,
    concentration: f32,
}

fn atmosphere(state: &EndpointPrimitiveState) -> Result<Atmosphere, KernelError> {
    let surface = state
        .top_surface_distance
        .ok_or("the atmospheric endpoint needs the source surface distance")?
        .0;
    if surface <= 0.0 {
        return Err("the atmospheric endpoint needs a positive surface distance");
    }
    let concentration = state
        .top_atmospheric_concentration
        .ok_or("the atmospheric endpoint needs the source atmospheric concentration")?
        .0;
    let old_temperature = state
        .old_temperature
        .ok_or("the atmospheric endpoint needs the old top temperature")?
        .0;
    let new_temperature = state
        .new_temperature
        .ok_or("the atmospheric endpoint needs the new top temperature")?
        .0;
    let material = *state
        .material_ids
        .last()
        .ok_or("the atmospheric endpoint needs the top node material")?;
    let diffusion = source_chpar(state, material, GAS_DIFFUSION_SLOT)?;
    let henry = source_chpar(state, material, HENRY_SLOT)?;
    let tdep_diffusion = source_tdep(state, GAS_DIFFUSION_SLOT)?;
    let tdep_surface = source_tdep(state, SURFACE_DISTANCE_SLOT)?;
    let tdep_henry = source_tdep(state, HENRY_SLOT)?;
    let tt_old = arrhenius(old_temperature);
    let tt_new = arrhenius(new_temperature);
    Ok(Atmosphere {
        surface_distance: surface * (tdep_surface * tt_old).exp(),
        diffusion_old: diffusion * (tdep_diffusion * tt_old).exp(),
        henry_old: henry * (tdep_henry * tt_old).exp(),
        diffusion_new: diffusion * (tdep_diffusion * tt_new).exp(),
        henry_new: henry * (tdep_henry * tt_new).exp(),
        concentration,
    })
}

/// `TT=(Temp+273.15-Tr)/R/(Temp+273.15)/Tr` (`78`, `888`, `921`), default REAL.
fn arrhenius(temperature: f32) -> f32 {
    (temperature + 273.15 - REFERENCE_TEMPERATURE_K)
        / GAS_CONSTANT
        / (temperature + 273.15)
        / REFERENCE_TEMPERATURE_K
}

/// `ChPar((jS-1)*16+slot, M)` (`765`, `889-890`, `922-923`), 1-based `slot` as in
/// the source; the material is the second array dimension, which is why records
/// carry it.
fn source_chpar(
    state: &EndpointPrimitiveState,
    material: usize,
    slot: usize,
) -> Result<f32, KernelError> {
    let records = state
        .source_chpar
        .as_ref()
        .ok_or("the atmospheric endpoint needs source ChPar records")?;
    let record = records
        .iter()
        .find(|r| r.species == state.source_species && r.material == material)
        .ok_or("source ChPar has no record for the species at the top node material")?;
    let value = record
        .slots
        .get(slot - 1)
        .ok_or("source ChPar records need the 20 source slots")?;
    if !value.0.is_finite() {
        return Err("source ChPar values must be finite");
    }
    Ok(value.0)
}

/// `TDep((jS-1)*16+slot)` (`766`, `887-890`), the flat reader layout with a
/// 16-entry stride per species (`INPUT.FOR:1379-1390`).
fn source_tdep(state: &EndpointPrimitiveState, slot: usize) -> Result<f32, KernelError> {
    let tdep = state
        .source_tdep
        .as_ref()
        .ok_or("the atmospheric endpoint needs source TDep")?;
    let index = (state.source_species - 1) * SPECIES_STRIDE + slot - 1;
    let value = tdep
        .get(index)
        .ok_or("source TDep is shorter than the atmospheric species slot")?;
    if !value.0.is_finite() {
        return Err("source TDep values must be finite");
    }
    Ok(value.0)
}

/// The single `cvBot(jS)`/`cvTop(jS)` pair of the step (`54-55`, `64-84`,
/// `197-225`).
///
/// Phase A runs before the level loop, so it reads `vO` at step entry and the
/// pre-solve `Conc`. Phase B runs after `BanSol` and the `148-179` update, so it
/// reads the accepted `Conc` and the `vN` that this step's `vCorr` restored
/// (`129-132`). The natural coefficients it reuses are the `Level=2` values
/// `MatSet` left behind, `F1` including the `Level=1` part (`800`, `902`).
fn endpoint_flux(
    state: &EndpointPrimitiveState,
    lower: &NaturalLower,
    upper: &NaturalUpper,
    bottom_code: i32,
    top_code: i32,
) -> Result<EndpointFlux, KernelError> {
    let n = state.coordinates.len();
    let top = n - 1;
    let c_bot = state.bottom_prescribed.0;
    let c_top = state.top_prescribed.0;
    let atmo = if top_code == -2 {
        Some(atmosphere(state)?)
    } else {
        None
    };
    let mut bottom = 0.0f32;
    let mut top_flux = 0.0f32;
    if bottom_code < 0 {
        let v_o = state.old_velocity[0].0;
        if v_o >= 0.0 {
            bottom = state.alf.0 * c_bot * v_o;
        }
        if v_o < 0.0 {
            bottom = state.alf.0 * state.old_concentration[0].0 * v_o;
        }
        if state.vapor && state.bottom_resistance.0 == 0.0 {
            bottom = 0.0;
        }
    } else if bottom_code == 0 {
        bottom = state.alf.0 * state.old_concentration[0].0 * state.old_velocity[0].0;
    }
    if top_code < 0 && state.t_level != 1 && state.old_velocity[top].0 < 0.0 {
        top_flux = state.alf.0 * c_top * state.old_velocity[top].0;
    }
    if let Some(atmo) = &atmo {
        top_flux = top_flux
            + state.alf.0 * atmo.diffusion_old / atmo.surface_distance
                * atmo.henry_old
                * state.old_concentration[top].0
            - atmo.diffusion_old / atmo.surface_distance * atmo.concentration;
    }
    if top_code < 0 {
        let v_n = state.new_velocity[top].0;
        if v_n < 0.0 {
            if state.t_level != 1 {
                top_flux += state.epsi.0 * v_n * c_top;
            } else {
                top_flux += v_n * c_top;
            }
        }
    } else {
        top_flux = upper.f_n_total
            - upper.b_n * state.new_accepted_concentration[top - 1].0
            - upper.d_n * state.new_accepted_concentration[top].0;
    }
    if let Some(atmo) = &atmo {
        top_flux = top_flux
            + state.epsi.0 * atmo.diffusion_new / atmo.surface_distance
                * atmo.henry_new
                * state.new_accepted_concentration[top].0
            - atmo.diffusion_new / atmo.surface_distance * atmo.concentration;
    }
    if bottom_code < 0 {
        let v_n = state.new_velocity[0].0;
        if v_n >= 0.0 {
            bottom += state.epsi.0 * c_bot * v_n;
        }
        if v_n < 0.0 {
            bottom += state.epsi.0 * state.new_accepted_concentration[0].0 * v_n;
        }
        if state.vapor && state.bottom_resistance.0 == 0.0 {
            bottom = 0.0;
        }
    } else if bottom_code == 0 {
        bottom += state.epsi.0 * state.new_accepted_concentration[0].0 * state.new_velocity[0].0;
    } else {
        bottom = lower.d1 * state.new_accepted_concentration[0].0
            + lower.e1 * state.new_accepted_concentration[1].0
            - lower.f1_total;
    }
    if top_flux.abs() < state.r_min.0 {
        top_flux = 0.0;
    }
    if bottom.abs() < state.r_min.0 {
        bottom = 0.0;
    }
    Ok(EndpointFlux {
        bottom: bottom as f64,
        top: top_flux as f64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solute_fortran_compat::types::{DefaultReal as R, MaterialSpeciesParameters};

    fn r(values: &[f32]) -> Vec<R> {
        values.iter().copied().map(R).collect()
    }

    // Temperatures 20 K apart on both sides of the 293.15 K reference, so the
    // Arrhenius factor of 78 is visible in `f32`.
    const TEMPERATURE_OLD: f32 = 15.0;
    const TEMPERATURE_NEW: f32 = 25.0;
    const SURFACE_DISTANCE: f32 = 3.0;
    const ATMOSPHERIC_CONCENTRATION: f32 = 0.7;
    const GAS_DIFFUSION: f32 = 0.8;
    const HENRY: f32 = 0.25;
    const TDEP_GAS_DIFFUSION: f32 = 1250.0;
    const TDEP_SURFACE_DISTANCE: f32 = 800.0;
    const TDEP_HENRY: f32 = 2000.0;

    fn chpar(
        species: usize,
        material: usize,
        diffusion: f32,
        henry: f32,
    ) -> MaterialSpeciesParameters {
        let mut slots = vec![R(0.0); 20];
        slots[GAS_DIFFUSION_SLOT - 1] = R(diffusion);
        slots[HENRY_SLOT - 1] = R(henry);
        MaterialSpeciesParameters {
            species,
            material,
            slots,
        }
    }

    /// A `TDep` whose species-1 slots 6, 9 and 10 are non-zero.
    fn atmospheric_tdep() -> Vec<R> {
        let mut tdep = vec![R(0.0); 20];
        tdep[GAS_DIFFUSION_SLOT - 1] = R(TDEP_GAS_DIFFUSION);
        tdep[SURFACE_DISTANCE_SLOT - 1] = R(TDEP_SURFACE_DISTANCE);
        tdep[HENRY_SLOT - 1] = R(TDEP_HENRY);
        tdep
    }

    /// The flux pair `assemble_endpoints` reports for a code pair.
    fn step_flux(state: &EndpointPrimitiveState, bottom_code: i32, top_code: i32) -> EndpointFlux {
        assemble_endpoints(state, bottom_code, top_code)
            .unwrap()
            .flux
    }

    /// A three-node step whose numbers stay exact in `f32`.
    fn fixture() -> EndpointPrimitiveState {
        EndpointPrimitiveState {
            coordinates: r(&[0.0, 2.0, 5.0]),
            old_water: r(&[0.4, 0.5, 0.6]),
            new_water: r(&[0.45, 0.55, 0.65]),
            old_velocity: r(&[0.3, 0.2, -0.4]),
            new_velocity: r(&[0.35, 0.25, -0.5]),
            old_matrix_velocity: r(&[0.3, 0.2, -0.4]),
            new_matrix_velocity: r(&[0.35, 0.25, -0.5]),
            old_dispersion: r(&[0.1, 0.2, 0.3]),
            new_dispersion: r(&[0.12, 0.22, 0.32]),
            old_retardation: r(&[1.1, 1.2, 1.3]),
            new_retardation: r(&[1.15, 1.25, 1.35]),
            old_concentration: r(&[1.0, 2.0, 3.0]),
            new_accepted_concentration: r(&[1.5, 2.5, 3.5]),
            material_immobile_water: r(&[0.05, 0.0]),
            old_source_g0: r(&[0.01, 0.02, 0.03]),
            old_source_g1: r(&[0.04, 0.05, 0.06]),
            new_source_g0: r(&[0.01, 0.02, 0.03]),
            new_source_g1: r(&[0.04, 0.05, 0.06]),
            upwind_weight: r(&[0.2, 0.3]),
            epsi: R(0.6),
            dt: R(2.0),
            alf: R(0.4),
            top_atmospheric_concentration: Some(R(ATMOSPHERIC_CONCENTRATION)),
            top_surface_distance: Some(R(SURFACE_DISTANCE)),
            old_temperature: Some(R(TEMPERATURE_OLD)),
            new_temperature: Some(R(TEMPERATURE_NEW)),
            source_chpar: Some(vec![chpar(1, 1, GAS_DIFFUSION, HENRY)]),
            source_tdep: Some(atmospheric_tdep()),
            material_ids: vec![1, 1, 1],
            source_species: 1,
            mobile_immobile: vec![false, false],
            bacteria: false,
            dual_porosity: 0,
            bottom_resistance: R(1.0),
            bottom_prescribed: R(4.0),
            top_prescribed: R(5.0),
            t_level: 2,
            vapor: false,
            r_min: R(1.0e-30),
        }
    }

    #[test]
    fn lower_natural_coefficients_match_their_source_statements() {
        let s = fixture();
        let natural = natural_lower(&s);
        let b1 = 2.0f32;
        let face = (0.4 * 0.1 + 0.5 * 0.2) / b1 / 2.0;
        let f1_expected = 1.0
            * (b1 / 2.0 / 2.0 * 0.4 * 1.1
                + 0.4
                    * (-face - ((2.0 + 3.0 * 0.2) * 0.3 + 0.2) / 6.0
                        + b1 / 12.0 * (3.0 * 0.04 + 0.05)))
            + 2.0
                * 0.4
                * (face - (0.3 + (2.0 - 3.0 * 0.2) * 0.2) / 6.0 + b1 / 12.0 * (0.04 + 0.05))
            + 0.4 * b1 / 6.0 * (2.0 * 0.01 + 0.02);
        assert_eq!(natural.f1, f1_expected);
        let face_new = (0.45 * 0.12 + 0.55 * 0.22) / b1 / 2.0;
        assert_eq!(
            natural.e1,
            0.6 * (-face_new + (0.35 + (2.0 - 3.0 * 0.2) * 0.25) / 6.0 - b1 / 12.0 * (0.04 + 0.05))
        );
        assert_eq!(
            natural.d1,
            b1 / 2.0 / 2.0 * 0.45 * 1.15
                + 0.6
                    * (face_new + ((2.0 + 3.0 * 0.2) * 0.35 + 0.25) / 6.0
                        - b1 / 12.0 * (3.0 * 0.04 + 0.05))
        );
        let f2 = 0.6 * b1 / 6.0 * (2.0 * 0.01 + 0.02);
        assert_eq!(natural.f2, f2);
        // 800: the flux coefficient carries the Level-2 sum.
        assert_eq!(natural.f1_total, natural.f1 + f2);
    }

    #[test]
    fn upper_natural_coefficients_match_their_source_statements() {
        let s = fixture();
        let natural = natural_upper(&s);
        let b1 = 3.0f32;
        let face = (0.5 * 0.2 + 0.6 * 0.3) / b1 / 2.0;
        let f_n_expected =
            2.0 * 0.4 * (face + ((2.0 + 3.0 * 0.3) * 0.2 + -0.4) / 6.0 + b1 / 12.0 * (0.05 + 0.06))
                + 3.0
                    * (b1 / 2.0 / 2.0 * 0.6 * 1.3
                        + 0.4
                            * (-face
                                + (0.2 + (2.0 - 3.0 * 0.3) * -0.4) / 6.0
                                + b1 / 12.0 * (0.05 + 3.0 * 0.06)))
                + 0.4 * b1 / 6.0 * (0.02 + 2.0 * 0.03);
        assert_eq!(natural.f_n, f_n_expected);
        let face_new = (0.55 * 0.22 + 0.65 * 0.32) / b1 / 2.0;
        assert_eq!(
            natural.b_n,
            0.6 * (-face_new - ((2.0 + 3.0 * 0.3) * 0.25 + -0.5) / 6.0 - b1 / 12.0 * (0.05 + 0.06))
        );
        assert_eq!(
            natural.d_n,
            b1 / 2.0 / 2.0 * 0.65 * 1.35
                + 0.6
                    * (face_new
                        - (0.25 + (2.0 - 3.0 * 0.3) * -0.5) / 6.0
                        - b1 / 12.0 * (0.05 + 3.0 * 0.06))
        );
        let f_e = 0.6 * b1 / 6.0 * (0.02 + 2.0 * 0.03);
        assert_eq!(natural.f_e, f_e);
        assert_eq!(natural.f_n_total, natural.f_n + f_e);
    }

    #[test]
    fn lower_type_one_and_zero_overwrite_both_levels() {
        let s = fixture();
        let result = assemble_endpoints(&s, 1, 1).unwrap();
        assert_eq!(
            result.lower.assembled,
            EndpointRow {
                diagonal: 1.0,
                rhs: 4.0,
                ..EndpointRow::default()
            }
        );
        let free = assemble_endpoints(&s, 0, 0).unwrap();
        assert_eq!(
            free.lower.assembled,
            EndpointRow {
                diagonal: -1.0,
                upper: 1.0,
                ..EndpointRow::default()
            }
        );
        // 781-789 still ran at Level 1, and 794-799 still ran at Level 2.
        let natural = natural_lower(&s);
        assert_eq!(free.lower.level_old.rhs, natural.f1 as f64);
        assert_eq!(free.lower.level_new.diagonal, natural.d1 as f64);
    }

    #[test]
    fn lower_type_three_accumulates_across_levels_and_reverses_on_inflow() {
        let s = fixture();
        let result = assemble_endpoints(&s, -1, 1).unwrap();
        let natural = natural_lower(&s);
        // 792 in REAL, then 814 accumulating into the double F(1).
        let level_old = natural.f1 + 0.4 * 4.0 * 0.3;
        assert_eq!(result.lower.level_old.rhs, level_old as f64);
        assert_eq!(
            result.lower.assembled.rhs,
            (level_old as f64) + natural.f2 as f64 + (0.6f32 * 4.0 * 0.35) as f64
        );
        assert_eq!(result.lower.assembled.diagonal, natural.d1 as f64);
        assert_eq!(result.lower.assembled.upper, natural.e1 as f64);

        // 811: a non-positive vN(1) without the vapor exception flips the row.
        let mut reversed = fixture();
        reversed.new_matrix_velocity[0] = R(-0.1);
        let reversed = assemble_endpoints(&reversed, -1, 1).unwrap();
        assert_eq!(
            reversed.lower.assembled,
            EndpointRow {
                diagonal: -1.0,
                upper: 1.0,
                ..EndpointRow::default()
            }
        );
        // 811: the vapor zero-resistance exception keeps the natural row, whose
        // own vN terms already reflect the reversed velocity.
        let mut vapor = fixture();
        vapor.new_matrix_velocity[0] = R(-0.1);
        vapor.vapor = true;
        vapor.bottom_resistance = R(0.0);
        let vapor_natural = natural_lower(&vapor);
        let vapor = assemble_endpoints(&vapor, -1, 1).unwrap();
        assert_eq!(vapor.lower.assembled.diagonal, vapor_natural.d1 as f64);
        assert_eq!(vapor.lower.assembled.upper, vapor_natural.e1 as f64);
        // 792 then 814 still land on F(1) under the exception.
        assert_eq!(
            vapor.lower.assembled.rhs,
            (((vapor_natural.f1 + 0.4f32 * 4.0 * 0.3) as f64)
                + vapor_natural.f2 as f64
                + (0.6f32 * 4.0 * -0.1) as f64)
        )
    }

    #[test]
    fn upper_type_one_and_non_positive_codes_follow_the_level_guards() {
        let s = fixture();
        let prescribed = assemble_endpoints(&s, 1, 1).unwrap();
        assert_eq!(
            prescribed.upper.assembled,
            EndpointRow {
                diagonal: 1.0,
                rhs: 5.0,
                ..EndpointRow::default()
            }
        );
        // 880 keeps Level 1 away from F(N) for kTopCh=1.
        assert_eq!(prescribed.upper.level_old.rhs, natural_upper(&s).f_n as f64);

        // 882 and 915 add the inflow terms for every code <= 0.
        let free = assemble_endpoints(&s, 0, 0).unwrap();
        let natural = natural_upper(&s);
        assert_eq!(
            free.upper.level_old.rhs,
            (natural.f_n - 0.4f32 * -0.4 * 5.0) as f64
        );
        assert_eq!(
            free.upper.assembled.rhs,
            ((natural.f_n - 0.4f32 * -0.4 * 5.0) as f64) + natural.f_e as f64
                - (0.6f32 * -0.5 * 5.0) as f64
        );
        assert_eq!(free.upper.assembled.lower, natural.b_n as f64);
        assert_eq!(free.upper.assembled.diagonal, natural.d_n as f64);

        // 811/882/915 use a strict sign test: zero adds nothing, and the natural
        // coefficients of this state replace the fixture's.
        let mut still = fixture();
        still.old_matrix_velocity[2] = R(0.0);
        still.new_matrix_velocity[2] = R(0.0);
        let still_natural = natural_upper(&still);
        let still = assemble_endpoints(&still, 1, -1).unwrap();
        assert_eq!(still.upper.level_old.rhs, still_natural.f_n as f64);
        assert_eq!(
            still.upper.assembled.rhs,
            (still_natural.f_n as f64) + still_natural.f_e as f64
        );
    }

    #[test]
    fn atmospheric_row_uses_the_tempo_surface_scale_at_both_levels() {
        let s = fixture();
        let result = assemble_endpoints(&s, 1, -2).unwrap();
        let natural = natural_upper(&s);
        let atmo = atmosphere(&s).unwrap();
        let tt_old = arrhenius(TEMPERATURE_OLD);
        let tt_new = arrhenius(TEMPERATURE_NEW);
        assert_ne!(tt_old, tt_new);
        assert_eq!(
            atmo.diffusion_old,
            GAS_DIFFUSION * (TDEP_GAS_DIFFUSION * tt_old).exp()
        );
        assert_eq!(atmo.henry_old, HENRY * (TDEP_HENRY * tt_old).exp());
        assert_eq!(
            atmo.diffusion_new,
            GAS_DIFFUSION * (TDEP_GAS_DIFFUSION * tt_new).exp()
        );
        assert_eq!(
            atmo.surface_distance,
            SURFACE_DISTANCE * (TDEP_SURFACE_DISTANCE * tt_old).exp()
        );
        // 81 derives dSurfT from TempO and Solute passes it as MatSet's dSurf
        // argument (127), so a TempN-derived dSurfT would move the row.
        assert_ne!(
            atmo.surface_distance,
            SURFACE_DISTANCE * (TDEP_SURFACE_DISTANCE * tt_new).exp()
        );
        assert_eq!(atmo.concentration, ATMOSPHERIC_CONCENTRATION);
        // 882, 891 and 892 in statement order, over the FN of 869-877.
        assert_eq!(
            result.upper.level_old.rhs,
            ((natural.f_n - 0.4f32 * -0.4 * 5.0) as f64)
                - (0.4f32 * atmo.diffusion_old / atmo.surface_distance * atmo.henry_old * 3.0)
                    as f64
                + (atmo.diffusion_old / atmo.surface_distance * ATMOSPHERIC_CONCENTRATION) as f64
        );
        // 914, 915 and 924 on top of it.
        assert_eq!(
            result.upper.assembled.rhs,
            (result.upper.level_old.rhs + natural.f_e as f64) - (0.6f32 * -0.5 * 5.0) as f64
        );
        assert_eq!(
            result.upper.assembled.diagonal,
            natural.d_n as f64
                + (0.6f32 * atmo.diffusion_new / atmo.surface_distance * atmo.henry_new) as f64
        );
    }

    #[test]
    fn atmospheric_coefficients_come_from_species_and_top_material_slots() {
        let mut s = fixture();
        s.source_species = 2;
        s.material_ids = vec![1, 1, 2];
        s.source_chpar = Some(vec![
            chpar(1, 2, 0.9, 0.9),
            chpar(2, 1, 0.7, 0.7),
            chpar(2, 2, GAS_DIFFUSION, HENRY),
        ]);
        let js = s.source_species;
        let mut tdep = vec![R(0.0); js * SPECIES_STRIDE];
        tdep[(js - 1) * SPECIES_STRIDE + GAS_DIFFUSION_SLOT - 1] = R(0.1);
        tdep[(js - 1) * SPECIES_STRIDE + SURFACE_DISTANCE_SLOT - 1] = R(0.2);
        tdep[(js - 1) * SPECIES_STRIDE + HENRY_SLOT - 1] = R(0.3);
        s.source_tdep = Some(tdep);
        let atmo = atmosphere(&s).unwrap();
        let tt_old = arrhenius(TEMPERATURE_OLD);
        let tt_new = arrhenius(TEMPERATURE_NEW);
        assert_eq!(atmo.diffusion_old, GAS_DIFFUSION * (0.1f32 * tt_old).exp());
        assert_eq!(atmo.henry_old, HENRY * (0.3f32 * tt_old).exp());
        assert_eq!(
            atmo.surface_distance,
            SURFACE_DISTANCE * (0.2f32 * tt_old).exp()
        );
        assert_eq!(atmo.diffusion_new, GAS_DIFFUSION * (0.1f32 * tt_new).exp());
        // The species/material pair, not a flat offset, selects the record.
        assert_eq!(
            source_chpar(&s, 2, GAS_DIFFUSION_SLOT).unwrap(),
            GAS_DIFFUSION
        );
        assert!(source_chpar(&s, 3, GAS_DIFFUSION_SLOT).is_err());
        s.source_chpar = None;
        assert!(atmosphere(&s).is_err());
        // A positive surface distance is what 891 divides by.
        s.source_chpar = Some(vec![chpar(2, 2, GAS_DIFFUSION, HENRY)]);
        s.top_surface_distance = Some(R(0.0));
        assert!(atmosphere(&s).is_err());
    }

    #[test]
    fn flux_phases_use_entry_velocities_and_their_own_concentrations() {
        let s = fixture();
        let flux = step_flux(&s, -1, -1);
        // 65 and 72 use vO at step entry; 199 and 215 use the vN this step's
        // vCorr restored (129-132).
        assert_eq!(
            flux.bottom,
            ((0.4f32 * 4.0 * 0.3) + (0.6f32 * 4.0 * 0.35)) as f64
        );
        assert_eq!(
            flux.top,
            ((0.4f32 * 5.0 * -0.4) + (0.6f32 * -0.5 * 5.0)) as f64
        );

        // The rows keep the pre-vCorr matrix velocities (124-128) while the flux
        // reads vO/vN, so separating the two moves only the flux.
        let mut separated = fixture();
        separated.old_velocity[0] = R(1.0);
        separated.new_velocity[0] = R(2.0);
        let flux = step_flux(&separated, -1, 1);
        assert_eq!(
            flux.bottom,
            ((0.4f32 * 4.0 * 1.0) + (0.6f32 * 4.0 * 2.0)) as f64
        );
        let natural = natural_lower(&separated);
        let rows = assemble_endpoints(&separated, -1, 1).unwrap();
        assert_eq!(rows.lower.assembled.diagonal, natural.d1 as f64);
        assert_eq!(
            rows.lower.assembled.rhs,
            (((natural.f1 + 0.4f32 * 4.0 * 0.3) as f64)
                + natural.f2 as f64
                + (0.6f32 * 4.0 * 0.35) as f64)
        );

        // 71 and 201: TLevel=1 keeps neither the phase-A top term nor epsi.
        let mut t_level_one = fixture();
        t_level_one.t_level = 1;
        let flux = step_flux(&t_level_one, -1, -1);
        assert_eq!(flux.top, (-0.5f32 * 5.0) as f64);

        // 66 and 216 switch to the node concentration on inflow, each phase to
        // its own Conc.
        let mut inflow = fixture();
        inflow.old_velocity[0] = R(-0.3);
        inflow.new_velocity[0] = R(-0.35);
        let flux = step_flux(&inflow, -1, 1);
        assert_eq!(
            flux.bottom,
            ((0.4f32 * 1.0 * -0.3) + (0.6f32 * 1.5 * -0.35)) as f64
        );

        // 67 and 217: the vapor exception zeroes the whole bottom flux.
        let mut vapor = fixture();
        vapor.vapor = true;
        vapor.bottom_resistance = R(0.0);
        assert_eq!(step_flux(&vapor, -1, 1).bottom, 0.0);
    }

    #[test]
    fn flux_uses_natural_coefficients_for_prescribed_codes() {
        let s = fixture();
        let lower = natural_lower(&s);
        let upper = natural_upper(&s);
        let flux = step_flux(&s, 1, 1);
        // 221: D1*Conc(1)+E1*Conc(2)-F1 with the accumulated F1 (800).
        assert_eq!(
            flux.bottom,
            (lower.d1 * 1.5 + lower.e1 * 2.5 - lower.f1_total) as f64
        );
        // 204: FN-BN*Conc(N-1)-DN*Conc(N) with the accumulated FN (902).
        assert_eq!(
            flux.top,
            (upper.f_n_total - upper.b_n * 2.5 - upper.d_n * 3.5) as f64
        );
        // 69 and 219 for the zero-gradient bottom code.
        let free = step_flux(&s, 0, 0);
        assert_eq!(
            free.bottom,
            ((0.4f32 * 1.0 * 0.3) + (0.6f32 * 1.5 * 0.35)) as f64
        );
    }

    #[test]
    fn atmospheric_flux_keeps_one_pair_per_species_and_step() {
        let s = fixture();
        let atmo = atmosphere(&s).unwrap();
        let flux = step_flux(&s, 1, -2);
        // One pair per species and step: 71-73 and 82-83 accumulate into the
        // same cvTop that 197-199 and 211-212 finish, in statement order and in
        // default REAL throughout.
        let mut expected = 0.4f32 * 5.0 * -0.4;
        expected = expected
            + 0.4 * atmo.diffusion_old / atmo.surface_distance * atmo.henry_old * 3.0
            - atmo.diffusion_old / atmo.surface_distance * ATMOSPHERIC_CONCENTRATION;
        expected += 0.6 * -0.5 * 5.0;
        expected = expected
            + 0.6 * atmo.diffusion_new / atmo.surface_distance * atmo.henry_new * 3.5
            - atmo.diffusion_new / atmo.surface_distance * ATMOSPHERIC_CONCENTRATION;
        assert_eq!(flux.top, expected as f64);
    }

    #[test]
    fn flux_truncation_uses_rmin() {
        let mut s = fixture();
        s.old_velocity = r(&[0.0, 0.0, 0.0]);
        s.new_velocity = r(&[0.0, 0.0, 0.0]);
        s.old_matrix_velocity = r(&[0.0, 0.0, 0.0]);
        s.new_matrix_velocity = r(&[0.0, 0.0, 0.0]);
        s.old_concentration = r(&[0.0, 0.0, 0.0]);
        s.new_accepted_concentration = r(&[0.0, 0.0, 0.0]);
        s.old_source_g0 = r(&[0.0, 0.0, 0.0]);
        s.old_source_g1 = r(&[0.0, 0.0, 0.0]);
        s.new_source_g0 = r(&[0.0, 0.0, 0.0]);
        s.new_source_g1 = r(&[0.0, 0.0, 0.0]);
        s.old_dispersion = r(&[0.0, 0.0, 0.0]);
        s.new_dispersion = r(&[0.0, 0.0, 0.0]);
        let flux = step_flux(&s, -1, -1);
        assert_eq!((flux.bottom, flux.top), (0.0, 0.0));
        s.r_min = R(0.0);
        s.new_velocity[0] = R(1.0e-31);
        let flux = step_flux(&s, -1, -1);
        assert_ne!(flux.bottom, 0.0);
    }

    #[test]
    fn mobile_immobile_water_reduces_both_levels_and_the_flux_uses_the_restored_state() {
        let mut s = fixture();
        s.mobile_immobile = vec![true, false];
        s.material_ids = vec![1, 2, 2];
        s.old_water = r(&[0.1, 0.5, 0.6]);
        s.new_water = r(&[0.1, 0.55, 0.65]);
        // 770: (lMobIm(M).and.iDualPor.eq.0).or.lBact, max(theta-ThImob, 0.001).
        assert_eq!(old_water(&s, 0), (0.1f32 - 0.05).max(0.001));
        s.mobile_immobile = vec![true, true];
        s.dual_porosity = 1;
        s.bacteria = false;
        assert_eq!(old_water(&s, 0), 0.1);
        s.bacteria = true;
        assert_eq!(old_water(&s, 0), (0.1f32 - 0.05).max(0.001));
        s.dual_porosity = 0;
        s.mobile_immobile = vec![false, true];
        assert_eq!(old_water(&s, 1), 0.5);
        assert_eq!(new_water(&s, 1), 0.55);
    }

    #[test]
    fn level_one_parts_read_the_old_pair_and_level_two_parts_the_new_pair() {
        let base = fixture();
        let a = assemble_endpoints(&base, 1, 1).unwrap();
        // Changing only the new pair moves E1/D1/F2 and BN/DN/FE, never F1/FN.
        let mut new_only = fixture();
        new_only.new_source_g0 = r(&[0.5, 0.6, 0.7]);
        new_only.new_source_g1 = r(&[0.8, 0.9, 1.0]);
        let b = assemble_endpoints(&new_only, -1, -1).unwrap();
        let a1 = assemble_endpoints(&base, -1, -1).unwrap();
        assert_eq!(a1.lower.level_old, b.lower.level_old);
        assert_eq!(a1.upper.level_old, b.upper.level_old);
        assert_ne!(a1.lower.level_new, b.lower.level_new);
        assert_ne!(a1.upper.level_new, b.upper.level_new);
        // Changing only the old pair moves F1/FN and leaves the level-two rows.
        let mut old_only = fixture();
        old_only.old_source_g0 = r(&[0.5, 0.6, 0.7]);
        old_only.old_source_g1 = r(&[0.8, 0.9, 1.0]);
        let c = assemble_endpoints(&old_only, -1, -1).unwrap();
        assert_eq!(a1.lower.level_new, c.lower.level_new);
        assert_eq!(a1.upper.level_new, c.upper.level_new);
        assert_ne!(a1.lower.level_old, c.lower.level_old);
        assert_ne!(a1.upper.level_old, c.upper.level_old);
        let _ = a;
    }

    #[test]
    fn kernel_rejects_a_pair_of_the_wrong_length() {
        for which in 0..4 {
            let mut s = fixture();
            match which {
                0 => s.old_source_g0.pop(),
                1 => s.old_source_g1.pop(),
                2 => s.new_source_g0.pop(),
                _ => s.new_source_g1.pop(),
            };
            assert!(assemble_endpoints(&s, 1, 1).is_err());
        }
    }

    #[test]
    fn kernel_rejects_inputs_the_source_could_not_consume() {
        let s = fixture();
        assert!(assemble_endpoints(&s, -1, -2).is_ok());
        assert!(assemble_endpoints(&s, -2, 1).is_err());
        assert!(assemble_endpoints(&s, 2, 1).is_err());
        assert!(assemble_endpoints(&s, 1, -3).is_err());
        assert!(assemble_endpoints(&s, 1, 2).is_err());
        let mut short = fixture();
        short.new_accepted_concentration.pop();
        assert!(assemble_endpoints(&short, 1, 1).is_err());
        let mut weights = fixture();
        weights.upwind_weight.pop();
        assert!(assemble_endpoints(&weights, 1, 1).is_err());
        let mut material = fixture();
        material.material_ids[0] = 3;
        assert!(assemble_endpoints(&material, 1, 1).is_err());
        let mut ordered = fixture();
        ordered.coordinates[1] = R(5.0);
        assert!(assemble_endpoints(&ordered, 1, 1).is_err());
        let mut dt = fixture();
        dt.dt = R(0.0);
        assert!(assemble_endpoints(&dt, 1, 1).is_err());
        let mut species = fixture();
        species.source_species = 0;
        assert!(assemble_endpoints(&species, 1, 1).is_err());
        let mut t_level = fixture();
        t_level.t_level = 3;
        assert!(assemble_endpoints(&t_level, 1, 1).is_err());
    }

    #[test]
    fn endpoint_nonfinite_primitives_propagate_like_fortran() {
        // ADR 0017: `SOLUTE.FOR:1561-1594` gives psi1 = NaN for iPsi=3 with SMax=0, and
        // `SOLUTE.FOR:710` carries it into g1. The kernel must not reject that state.
        let mut nan_g1 = fixture();
        nan_g1.new_source_g1[0] = R(f32::NAN);
        nan_g1.old_source_g1[0] = R(f32::NAN);
        let out = assemble_endpoints(&nan_g1, 1, 1).expect("NaN primitives must not be rejected");
        assert!(
            format!("{out:?}").contains("NaN"),
            "NaN should reach the endpoint rows"
        );

        let mut inf_water = fixture();
        inf_water.old_water[1] = R(f32::INFINITY);
        assert!(assemble_endpoints(&inf_water, 1, 1).is_ok());
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
        assert_eq!(canonical_endpoint(3, 3, false), None);
    }
}

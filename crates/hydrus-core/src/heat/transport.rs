//! Heat-only transport solve, ported from `Temper` (`src_reference/TEMPER.FOR:5-76`)
//! and the interior rows of `SetUpHeat` (`src_reference/TEMPER.FOR:112-143`).
//! Boundary rows come from [`set_up_heat_boundary`]. Contract: planning/master/M6_HEAT_CONTRACT.md.
//!
//! Node indices are zero-based; the Fortran is one-based. Water is an input: this module never
//! calls the water solver (D9). Vapor, energy-balance and `iTemp` terms are not ported.

use std::fmt;

use super::boundary::{
    set_up_heat_boundary, BoundaryCoefficients, HeatBoundaryError, HeatBoundaryInput, HeatLevel,
};
use super::capacity::{temp_cap, TempCapError, TempCapInput, TempMaterial};
use super::config::{BottomBoundaryKind, HeatConfig, TopBoundaryKind};
use super::vapor::adjust::{adjust_heat_rows, VaporHeatAdjustmentInput};
use super::vapor::{VaporConfig, VaporError};
use crate::hydraulics::{solve_tridiagonal_in_place, TridiagonalError};

/// Inputs to [`solve_heat_transport`]. Slices hold one entry per node, `N >= 3`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatTransportInput<'a> {
    /// Node coordinates `x`.
    pub x: &'a [f64],
    /// Time step `dt`.
    pub dt: f64,
    /// Zero-based material index per node.
    pub material_indices: &'a [usize],
    pub materials: &'a [TempMaterial],
    /// `TempO`: temperature at the start of the step.
    pub temp_old: &'a [f64],
    /// `ThOld`: water content before the water step.
    pub water_old: &'a [f64],
    /// `ThNew`: water content of the accepted water step.
    pub water_new: &'a [f64],
    /// `vOld`: Darcy flux before the water step.
    pub velocity_old: &'a [f64],
    /// `vNew`: Darcy flux of the accepted water step.
    pub velocity_new: &'a [f64],
    /// `Sink`.
    pub sink: &'a [f64],
    /// `tTopA`. Equals `tTop` because `HeatConfig` rejects `tPeriod > 0`.
    pub top_temperature: f64,
    /// `tBot`.
    pub bottom_temperature: f64,
    pub x_conv: f64,
    pub t_conv: f64,
    /// Smallest admissible absolute pivot, passed to `solve_tridiagonal_in_place`.
    pub minimum_pivot: f64,
}

/// Diagnostics from a successful heat transport solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatTransportReport {
    /// `HeatFl` (`TEMPER.FOR:25-27`). Zero for a Dirichlet top.
    pub heat_flux: f64,
    /// Smallest absolute pivot seen by the tridiagonal solve.
    pub minimum_absolute_pivot: f64,
}

/// Typed rejections for [`solve_heat_transport`].
#[derive(Clone, Debug, PartialEq)]
pub enum HeatTransportError {
    /// The heat-only entry cannot silently omit selected vapor terms.
    VaporNotSupported,
    TooFewNodes {
        count: usize,
    },
    LengthMismatch {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
    MissingMaterial,
    VaporInputMismatch,
    NonFiniteOutput {
        index: usize,
    },
    TempCap(TempCapError),
    Vapor(VaporError),
    Boundary(HeatBoundaryError),
    Tridiagonal(TridiagonalError),
}

impl fmt::Display for HeatTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VaporNotSupported => {
                write!(formatter, "heat-only transport rejects vapor selection")
            }
            Self::TooFewNodes { count } => {
                write!(
                    formatter,
                    "heat transport needs at least 3 nodes, got {count}"
                )
            }
            Self::LengthMismatch {
                name,
                expected,
                actual,
            } => write!(
                formatter,
                "heat transport {name} has length {actual}, expected {expected}"
            ),
            Self::MissingMaterial => {
                write!(formatter, "heat transport needs at least one material")
            }
            Self::VaporInputMismatch => {
                write!(formatter, "vapor heat inputs must match the base heat step")
            }
            Self::NonFiniteOutput { index } => {
                write!(
                    formatter,
                    "heat transport produced a non-finite temperature at {index}"
                )
            }
            Self::TempCap(error) => write!(formatter, "heat capacity: {error:?}"),
            Self::Vapor(error) => write!(formatter, "vapor heat adjustment: {error}"),
            Self::Boundary(error) => write!(formatter, "heat boundary: {error:?}"),
            Self::Tridiagonal(error) => write!(formatter, "heat transport solve: {error}"),
        }
    }
}

impl std::error::Error for HeatTransportError {}

/// Per-solve constants shared by both levels.
struct Setup {
    top_code: i32,
    bottom_code: i32,
    heat_flux: f64,
    cw: f64,
    campbell: bool,
}

/// Runs one heat transport step (`Temper`, heat-only), writing new temperatures to `temp_new`.
///
/// `temp_new` is written only after the solve succeeds.
pub fn solve_heat_transport(
    config: HeatConfig,
    input: HeatTransportInput<'_>,
    temp_new: &mut [f64],
) -> Result<HeatTransportReport, HeatTransportError> {
    if config.vapor_enabled() {
        return Err(HeatTransportError::VaporNotSupported);
    }
    solve_heat_transport_inner(config, input, None, temp_new)
}

/// Runs a vapor-enabled heat step using the selected fixed-temperature boundaries.
///
/// The ordinary heat-only entry remains unchanged and continues to reject vapor
/// selections. Vapor additions are calculated into owned arrays and applied only
/// to interior rows after both base levels have been assembled.
pub fn solve_vapor_heat_transport(
    config: VaporConfig,
    input: HeatTransportInput<'_>,
    vapor: &VaporHeatAdjustmentInput<'_>,
    temp_new: &mut [f64],
) -> Result<HeatTransportReport, HeatTransportError> {
    if !config.heat().vapor_enabled()
        || config.top_temperature() != input.top_temperature
        || config.bottom_temperature() != input.bottom_temperature
        || config.x_conv() != input.x_conv
        || config.t_conv() != input.t_conv
        || vapor.coordinates != input.x
        || vapor.temperature_old != input.temp_old
        || vapor.timestep != input.dt
        || vapor.x_conv != input.x_conv
        || vapor.t_conv != input.t_conv
    {
        return Err(HeatTransportError::VaporInputMismatch);
    }
    solve_heat_transport_inner(config.heat(), input, Some(vapor), temp_new)
}

fn solve_heat_transport_inner(
    config: HeatConfig,
    input: HeatTransportInput<'_>,
    vapor: Option<&VaporHeatAdjustmentInput<'_>>,
    temp_new: &mut [f64],
) -> Result<HeatTransportReport, HeatTransportError> {
    let n = input.x.len();
    if n < 3 {
        return Err(HeatTransportError::TooFewNodes { count: n });
    }
    for (name, actual) in [
        ("material_indices", input.material_indices.len()),
        ("temp_old", input.temp_old.len()),
        ("water_old", input.water_old.len()),
        ("water_new", input.water_new.len()),
        ("velocity_old", input.velocity_old.len()),
        ("velocity_new", input.velocity_new.len()),
        ("sink", input.sink.len()),
        ("temp_new", temp_new.len()),
    ] {
        if actual != n {
            return Err(HeatTransportError::LengthMismatch {
                name,
                expected: n,
                actual,
            });
        }
    }
    let cw = input
        .materials
        .first()
        .ok_or(HeatTransportError::MissingMaterial)?
        .water_heat_capacity;

    let top_code = match config.top_boundary() {
        TopBoundaryKind::Dirichlet => 1,
        TopBoundaryKind::Flux => -1,
    };
    let bottom_code = match config.bottom_boundary() {
        BottomBoundaryKind::Dirichlet => 1,
        BottomBoundaryKind::Flux => -1,
        BottomBoundaryKind::ZeroGradient => 0,
    };
    // TEMPER.FOR:25-27, computed only for a flux top (lEnBal is false in M6).
    let heat_flux = match config.top_boundary() {
        TopBoundaryKind::Dirichlet => 0.0,
        TopBoundaryKind::Flux => {
            cw * input.top_temperature * (input.velocity_new[n - 1] + input.velocity_old[n - 1])
                / 2.0
        }
    };
    let setup = Setup {
        top_code,
        bottom_code,
        heat_flux,
        cw,
        campbell: config.campbell(),
    };

    let mut cap_old = vec![0.0; n];
    let mut cond_old = vec![0.0; n];
    let mut cap_new = vec![0.0; n];
    let mut cond_new = vec![0.0; n];
    let mut lower = vec![0.0; n];
    let mut diagonal = vec![0.0; n];
    let mut upper = vec![0.0; n];
    let mut rhs = vec![0.0; n];

    // Level 1 (TEMPER.FOR:35-42): capacity from ThOld/vOld, assemble F.
    capacity(
        &input,
        &setup,
        input.water_old,
        input.velocity_old,
        &mut cap_old,
        &mut cond_old,
    )?;
    assemble_level_one(&input, &setup, &cap_old, &cond_old, &mut rhs)?;

    // Level 2: capacity from ThNew/vNew, assemble B, D, E.
    capacity(
        &input,
        &setup,
        input.water_new,
        input.velocity_new,
        &mut cap_new,
        &mut cond_new,
    )?;
    assemble_level_two(
        &input,
        &setup,
        &cap_new,
        &cond_new,
        &mut lower,
        &mut diagonal,
        &mut upper,
    )?;

    if let Some(vapor_input) = vapor {
        let adjustment = adjust_heat_rows(vapor_input).map_err(HeatTransportError::Vapor)?;
        for i in 0..n {
            rhs[i] += adjustment.rhs_level_one[i];
            lower[i] += adjustment.lower_level_two[i];
            diagonal[i] += adjustment.diagonal_level_two[i];
            upper[i] += adjustment.upper_level_two[i];
            rhs[i] += adjustment.rhs_level_two[i];
        }
    }

    // Source sets B(1) and E(N) only through the boundary rows; neither is assigned for Temper.
    lower[0] = 0.0;
    upper[n - 1] = 0.0;

    let diagnostics =
        solve_tridiagonal_in_place(&lower, &mut diagonal, &upper, &mut rhs, input.minimum_pivot)
            .map_err(HeatTransportError::Tridiagonal)?;

    if let Some(index) = rhs.iter().position(|value| !value.is_finite()) {
        return Err(HeatTransportError::NonFiniteOutput { index });
    }
    temp_new.copy_from_slice(&rhs);
    Ok(HeatTransportReport {
        heat_flux,
        minimum_absolute_pivot: diagnostics.minimum_absolute_pivot,
    })
}

fn capacity(
    input: &HeatTransportInput<'_>,
    setup: &Setup,
    water: &[f64],
    velocity: &[f64],
    cap: &mut [f64],
    cond: &mut [f64],
) -> Result<(), HeatTransportError> {
    temp_cap(
        TempCapInput {
            material_indices: input.material_indices,
            water_content: water,
            velocity,
            materials: input.materials,
            campbell: setup.campbell,
            x_conv: input.x_conv,
            t_conv: input.t_conv,
        },
        cap,
        cond,
    )
    .map_err(HeatTransportError::TempCap)
}

fn boundary_rows(
    level: HeatLevel,
    input: &HeatTransportInput<'_>,
    setup: &Setup,
    cap: &[f64],
    cond: &[f64],
) -> Result<super::boundary::HeatBoundaryRows, HeatTransportError> {
    set_up_heat_boundary(
        level,
        HeatBoundaryInput {
            k_top_t: setup.top_code,
            k_bot_t: setup.bottom_code,
            t_top_a: input.top_temperature,
            t_bottom: input.bottom_temperature,
            t_period: 0.0,
            heat_flux: setup.heat_flux,
            dt: input.dt,
            cw: setup.cw,
            x: input.x,
            temp_old: input.temp_old,
            v_old: input.velocity_old,
            v_new: input.velocity_new,
            cap,
            cond,
            sink: input.sink,
        },
    )
    .map_err(HeatTransportError::Boundary)
}

fn store(slot: &mut f64, value: Option<f64>) {
    if let Some(value) = value {
        *slot = value;
    }
}

/// Level 1: right-hand side `F` from `TempO`, `vOld` and the level-1 capacity (`TEMPER.FOR:112-127`).
fn assemble_level_one(
    input: &HeatTransportInput<'_>,
    setup: &Setup,
    cap: &[f64],
    cond: &[f64],
    rhs: &mut [f64],
) -> Result<(), HeatTransportError> {
    let n = input.x.len();
    let x = input.x;
    let cw = setup.cw;
    let dt = input.dt;
    let (t, v, s) = (input.temp_old, input.velocity_old, input.sink);
    for i in 1..n - 1 {
        let dx_a = x[i] - x[i - 1];
        let dx_b = x[i + 1] - x[i];
        let dx = (x[i + 1] - x[i - 1]) / 2.0;
        rhs[i] = t[i - 1]
            * ((cond[i] + cond[i - 1]) / 4.0 / dx_a + cw * (v[i] + 2.0 * v[i - 1]) / 12.0
                - dx_a / 24.0 * cw * (s[i - 1] + s[i]))
            + t[i]
                * (-cw * (v[i + 1] - v[i - 1]) / 12.0 + dx / dt * cap[i]
                    - (cond[i + 1] + cond[i]) / 4.0 / dx_b
                    - (cond[i] + cond[i - 1]) / 4.0 / dx_a
                    - dx_a / 24.0 * cw * (s[i - 1] + 3.0 * s[i])
                    - dx_b / 24.0 * cw * (3.0 * s[i] + s[i + 1]))
            + t[i + 1]
                * ((cond[i + 1] + cond[i]) / 4.0 / dx_b
                    - cw * (2.0 * v[i + 1] + v[i]) / 12.0
                    - dx_b / 24.0 * cw * (s[i + 1] + s[i]));
    }
    let rows = boundary_rows(HeatLevel::First, input, setup, cap, cond)?;
    store(&mut rhs[0], rows.bottom.f);
    store(&mut rhs[n - 1], rows.top.f);
    Ok(())
}

/// Level 2: matrix `B`, `D`, `E` from `ThNew`/`vNew` (`TEMPER.FOR:128-143`).
fn assemble_level_two(
    input: &HeatTransportInput<'_>,
    setup: &Setup,
    cap: &[f64],
    cond: &[f64],
    lower: &mut [f64],
    diagonal: &mut [f64],
    upper: &mut [f64],
) -> Result<(), HeatTransportError> {
    let n = input.x.len();
    let x = input.x;
    let cw = setup.cw;
    let dt = input.dt;
    let (v, s) = (input.velocity_new, input.sink);
    for i in 1..n - 1 {
        let dx_a = x[i] - x[i - 1];
        let dx_b = x[i + 1] - x[i];
        let dx = (x[i + 1] - x[i - 1]) / 2.0;
        lower[i] = -(cond[i] + cond[i - 1]) / 4.0 / dx_a - cw * (v[i] + 2.0 * v[i - 1]) / 12.0
            + dx_a / 24.0 * cw * (s[i - 1] + s[i]);
        diagonal[i] = (cond[i - 1] + cond[i]) / 4.0 / dx_a
            + (cond[i] + cond[i + 1]) / 4.0 / dx_b
            + dx / dt * cap[i]
            + cw * (v[i + 1] - v[i - 1]) / 12.0
            + dx_a / 24.0 * cw * (s[i - 1] + 3.0 * s[i])
            + dx_b / 24.0 * cw * (3.0 * s[i] + s[i + 1]);
        upper[i] = -(cond[i] + cond[i + 1]) / 4.0 / dx_b
            + cw * (2.0 * v[i + 1] + v[i]) / 12.0
            + dx_b / 24.0 * cw * (s[i + 1] + s[i]);
    }
    let rows = boundary_rows(HeatLevel::Second, input, setup, cap, cond)?;
    apply_level_two(&rows, lower, diagonal, upper);
    Ok(())
}

fn apply_level_two(
    rows: &super::boundary::HeatBoundaryRows,
    lower: &mut [f64],
    diagonal: &mut [f64],
    upper: &mut [f64],
) {
    let n = lower.len();
    let BoundaryCoefficients {
        b: bottom_b,
        d: bottom_d,
        e: bottom_e,
        ..
    } = rows.bottom;
    let BoundaryCoefficients {
        b: top_b,
        d: top_d,
        e: top_e,
        ..
    } = rows.top;
    store(&mut lower[0], bottom_b);
    store(&mut diagonal[0], bottom_d);
    store(&mut upper[0], bottom_e);
    store(&mut lower[n - 1], top_b);
    store(&mut diagonal[n - 1], top_d);
    store(&mut upper[n - 1], top_e);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heat::config::HeatConfigInput;

    /// Solid fraction 0.5, quartz 0.5, solid capacity 2.0, water capacity 4.0.
    /// At theta = 0.25: Cap = 2*0.5 + 4*0.25 = 2.0. Cond = quartz = 0.5 (no advection).
    fn material() -> TempMaterial {
        TempMaterial::from_tpar([0.5, 0.0, 0.0, 0.5, 0.0, 0.0, 2.0, 2.5, 4.0])
    }

    /// `k_top_t`: 1 Dirichlet, -1 flux. `k_bot_t`: 1 Dirichlet, -1 flux, 0 zero-gradient.
    fn config(k_top_t: i32, k_bot_t: i32) -> HeatConfig {
        HeatConfig::new(HeatConfigInput {
            l_vapor: false,
            l_en_bal: false,
            i_dual_por: 0,
            l_w_dep: false,
            i_model: 0,
            domain_count: 1,
            k_top_t,
            t_period: 0.0,
            k_bot_t,
            campbell: false,
        })
        .unwrap()
    }

    #[test]
    fn uniform_state_with_zero_flux_is_unchanged() {
        // Uniform T = 10, v = 0, sink = 0, dt = 1, dx = 1: row sums give B+D+E = dx/dt*Cap = 2
        // and F = 2*T = 20, so T = 10 satisfies every row (zero-gradient bottom, Dirichlet top).
        let x = [0.0, 1.0, 2.0];
        let indices = [0usize; 3];
        let materials = [material()];
        let temp = [10.0; 3];
        let water = [0.25; 3];
        let zero = [0.0; 3];
        let input = HeatTransportInput {
            x: &x,
            dt: 1.0,
            material_indices: &indices,
            materials: &materials,
            temp_old: &temp,
            water_old: &water,
            water_new: &water,
            velocity_old: &zero,
            velocity_new: &zero,
            sink: &zero,
            top_temperature: 10.0,
            bottom_temperature: 0.0,
            x_conv: 1.0,
            t_conv: 1.0,
            minimum_pivot: 1e-12,
        };
        let mut out = [0.0; 3];
        let report = solve_heat_transport(config(1, 0), input, &mut out).unwrap();
        for value in out.iter() {
            assert!((value - 10.0).abs() < 1e-12, "{value}");
        }
        assert_eq!(report.heat_flux, 0.0);
    }

    #[test]
    fn three_node_diffusion_step_matches_hand_value() {
        // Cap = 2, Cond = 0.5, v = 0, sink = 0, dt = 1, dx = 1.
        // Interior row 1: B = -0.25, D = 0.25 + 0.25 + 2 = 2.5, E = -0.25.
        // Level-1 F1 = 0.25*T0 + 1.5*T1 + 0.25*T2 = 0.25*10 = 2.5 for T_old = [0, 0, 10].
        // Dirichlet bottom T0 = 0, Dirichlet top T2 = 10.
        // Row 1: 2.5*T1 - 0.25*10 = 2.5, so T1 = 2. Expected T_new = [0, 2, 10].
        let x = [0.0, 1.0, 2.0];
        let indices = [0usize; 3];
        let materials = [material()];
        let temp = [0.0, 0.0, 10.0];
        let water = [0.25; 3];
        let zero = [0.0; 3];
        let input = HeatTransportInput {
            x: &x,
            dt: 1.0,
            material_indices: &indices,
            materials: &materials,
            temp_old: &temp,
            water_old: &water,
            water_new: &water,
            velocity_old: &zero,
            velocity_new: &zero,
            sink: &zero,
            top_temperature: 10.0,
            bottom_temperature: 0.0,
            x_conv: 1.0,
            t_conv: 1.0,
            minimum_pivot: 1e-12,
        };
        let mut out = [-1.0; 3];
        solve_heat_transport(config(1, 1), input, &mut out).unwrap();
        let expected = [0.0, 2.0, 10.0];
        for (actual, expected) in out.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
        }
    }

    #[test]
    fn singular_pivot_is_typed_and_leaves_output_unchanged() {
        // minimum_pivot above every pivot forces the first pivot check (index 0, D = 1) to fail.
        let x = [0.0, 1.0, 2.0];
        let indices = [0usize; 3];
        let materials = [material()];
        let temp = [0.0, 0.0, 10.0];
        let water = [0.25; 3];
        let zero = [0.0; 3];
        let input = HeatTransportInput {
            x: &x,
            dt: 1.0,
            material_indices: &indices,
            materials: &materials,
            temp_old: &temp,
            water_old: &water,
            water_new: &water,
            velocity_old: &zero,
            velocity_new: &zero,
            sink: &zero,
            top_temperature: 10.0,
            bottom_temperature: 0.0,
            x_conv: 1.0,
            t_conv: 1.0,
            minimum_pivot: 1e30,
        };
        let mut out = [-1.0; 3];
        let error = solve_heat_transport(config(1, 1), input, &mut out).unwrap_err();
        assert!(matches!(
            error,
            HeatTransportError::Tridiagonal(TridiagonalError::SingularPivot { index: 0, .. })
        ));
        assert_eq!(out, [-1.0; 3]);
    }

    #[test]
    fn flux_top_reports_heat_flux_from_water_flux() {
        // HeatFl = Cw * tTopA * (vNew(N) + vOld(N)) / 2 = 4 * 10 * (1.5 + 0.5) / 2 = 40.
        let x = [0.0, 1.0, 2.0];
        let indices = [0usize; 3];
        let materials = [material()];
        let temp = [0.0, 0.0, 10.0];
        let water = [0.25; 3];
        let zero = [0.0; 3];
        let velocity_old = [0.0, 0.0, 0.5];
        let velocity_new = [0.0, 0.0, 1.5];
        let input = HeatTransportInput {
            x: &x,
            dt: 1.0,
            material_indices: &indices,
            materials: &materials,
            temp_old: &temp,
            water_old: &water,
            water_new: &water,
            velocity_old: &velocity_old,
            velocity_new: &velocity_new,
            sink: &zero,
            top_temperature: 10.0,
            bottom_temperature: 0.0,
            x_conv: 1.0,
            t_conv: 1.0,
            minimum_pivot: 1e-12,
        };
        let mut out = [0.0; 3];
        let report = solve_heat_transport(config(-1, 1), input, &mut out).unwrap();
        assert_eq!(report.heat_flux, 40.0);
        assert!(out.iter().all(|value| value.is_finite()));
    }
}

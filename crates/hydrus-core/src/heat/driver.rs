//! Coupled water→heat step driver (one-way, D9).
//!
//! One step calls [`advance_water_step`] exactly once, then advances heat from the accepted
//! water snapshot. Heat never writes to water. Contract: planning/master/M6_HEAT_CONTRACT.md
//! (`## Coupling order`). Node velocities follow the Fortran `Veloc` mapping
//! (`src_reference/WATFLOW.FOR:717-775`), called with `TempN` at `HYDRUS.FOR:473-476`.
//!
//! Outputs are the accepted temperatures (through [`HeatState`]) and `heat_flux`. Balance
//! terms are not computed here (24.5.3).

use super::capacity::TempMaterial;
use super::config::{HeatConfig, HeatConfigError, HeatConfigInput};
use super::state::{HeatState, HeatStateError};
use super::transport::{solve_heat_transport, HeatTransportError, HeatTransportInput};
use crate::hydraulics::FaceAveraging;
use crate::{
    advance_water_step, WaterState, WaterStepError, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

/// Typed rejections for [`HeatWaterDriver`].
#[derive(Clone, Debug, PartialEq)]
pub enum HeatDriverError {
    /// Heat configuration rejected by [`HeatConfig::new`]. No water step ran.
    Config(HeatConfigError),
    /// Heat state construction or acceptance failed.
    State(HeatStateError),
    /// A precondition failed before any water step ran. Water is untouched.
    Preflight { reason: &'static str },
    /// The water step failed. Heat is untouched.
    Water { source: WaterStepError },
    /// Water was accepted, then the heat solve failed. Heat state is unchanged;
    /// water stays advanced and must not be retried alone.
    HeatAfterWaterAccepted {
        accepted_water_time: f64,
        source: HeatTransportError,
    },
}

/// Per-step heat boundary data and solver settings supplied by the caller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoupledHeatInput {
    /// `tTop` (`tTopA` with `tPeriod <= 0`).
    pub top_temperature: f64,
    /// `tBot`.
    pub bottom_temperature: f64,
    /// Length conversion factor (`xConv`).
    pub x_conv: f64,
    /// Time conversion factor (`tConv`).
    pub t_conv: f64,
    /// Smallest admissible absolute pivot.
    pub minimum_pivot: f64,
}

/// Owned diagnostics from one accepted coupled step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoupledStepReport {
    pub accepted_time: f64,
    /// Accepted water time step, used as heat `dt`.
    pub timestep: f64,
    /// `HeatFl` (`TEMPER.FOR:25-27`).
    pub heat_flux: f64,
    pub minimum_absolute_pivot: f64,
    pub water_iterations: usize,
    pub water_retries: usize,
}

/// Owns the heat state and one-step buffers for a single coupled water→heat column.
#[derive(Clone, Debug)]
pub struct HeatWaterDriver {
    config: HeatConfig,
    state: HeatState,
    x: Vec<f64>,
    material_indices: Vec<usize>,
    materials: Vec<TempMaterial>,
    velocity_old: Vec<f64>,
    velocity_new: Vec<f64>,
    water_old: Vec<f64>,
    scratch: Vec<f64>,
}

impl HeatWaterDriver {
    /// Validates the heat-only configuration first, then the column shape. No water code runs.
    ///
    /// `initial_velocity` is `vOld` before the first step (zeros allowed).
    pub fn new(
        config_input: HeatConfigInput,
        x: &[f64],
        material_indices: &[usize],
        materials: &[TempMaterial],
        initial_time: f64,
        initial_temperatures: Vec<f64>,
        initial_velocity: &[f64],
    ) -> Result<Self, HeatDriverError> {
        let config = HeatConfig::new(config_input).map_err(HeatDriverError::Config)?;
        if config.vapor_enabled() {
            return Err(HeatDriverError::Config(
                HeatConfigError::VaporNotSupported { value: true },
            ));
        }
        let n = x.len();
        if n < 3 {
            return Err(HeatDriverError::Preflight {
                reason: "heat driver needs at least three nodes",
            });
        }
        if material_indices.len() != n
            || initial_temperatures.len() != n
            || initial_velocity.len() != n
        {
            return Err(HeatDriverError::Preflight {
                reason: "column slice length differs from node count",
            });
        }
        if materials.is_empty() || material_indices.iter().any(|&i| i >= materials.len()) {
            return Err(HeatDriverError::Preflight {
                reason: "material index outside the material table",
            });
        }
        let state =
            HeatState::new(initial_time, initial_temperatures).map_err(HeatDriverError::State)?;
        Ok(Self {
            config,
            state,
            x: x.to_vec(),
            material_indices: material_indices.to_vec(),
            materials: materials.to_vec(),
            velocity_old: initial_velocity.to_vec(),
            velocity_new: vec![0.0; n],
            water_old: vec![0.0; n],
            scratch: vec![0.0; n],
        })
    }

    /// Accepted heat state.
    pub fn state(&self) -> &HeatState {
        &self.state
    }

    /// Advances water once with `advance_water_step`, then heat from the accepted water snapshot.
    pub fn step(
        &mut self,
        water_state: &mut WaterState,
        water_workspace: &mut WaterWorkspace,
        water_input: WaterStepInput<'_>,
        water_policy: WaterStepPolicy,
        heat_input: CoupledHeatInput,
    ) -> Result<CoupledStepReport, HeatDriverError> {
        self.preflight(water_state, water_workspace, &water_input)?;
        self.water_old.copy_from_slice(water_state.water_contents());
        let sink = water_input.sink;

        let report = advance_water_step(water_state, water_workspace, water_input, water_policy)
            .map_err(|source| HeatDriverError::Water { source })?;
        let accepted_water_time = report.accepted_time;
        let timestep = report.timestep;
        let water_iterations = report.iterations;
        let water_retries = report.retries;

        node_velocities(
            &self.x,
            report.face_fluxes,
            &self.water_old,
            report.water_contents,
            sink,
            timestep,
            &mut self.velocity_new,
        );

        let transport = solve_heat_transport(
            self.config,
            HeatTransportInput {
                x: &self.x,
                dt: timestep,
                material_indices: &self.material_indices,
                materials: &self.materials,
                temp_old: self.state.temperatures(),
                water_old: &self.water_old,
                water_new: report.water_contents,
                velocity_old: &self.velocity_old,
                velocity_new: &self.velocity_new,
                sink,
                top_temperature: heat_input.top_temperature,
                bottom_temperature: heat_input.bottom_temperature,
                x_conv: heat_input.x_conv,
                t_conv: heat_input.t_conv,
                minimum_pivot: heat_input.minimum_pivot,
            },
            &mut self.scratch,
        );
        let transport = match transport {
            Ok(transport) => transport,
            Err(source) => {
                self.state.reject();
                return Err(HeatDriverError::HeatAfterWaterAccepted {
                    accepted_water_time,
                    source,
                });
            }
        };

        self.state
            .trial_temperatures_mut()
            .copy_from_slice(&self.scratch);
        self.state
            .accept(accepted_water_time)
            .map_err(HeatDriverError::State)?;
        self.velocity_old.copy_from_slice(&self.velocity_new);

        Ok(CoupledStepReport {
            accepted_time: accepted_water_time,
            timestep,
            heat_flux: transport.heat_flux,
            minimum_absolute_pivot: transport.minimum_absolute_pivot,
            water_iterations,
            water_retries,
        })
    }

    fn preflight(
        &self,
        water_state: &WaterState,
        water_workspace: &WaterWorkspace,
        water_input: &WaterStepInput<'_>,
    ) -> Result<(), HeatDriverError> {
        let n = self.x.len();
        if water_input.root_uptake.is_some() {
            return Err(HeatDriverError::Preflight {
                reason: "root uptake with heat coupling is out of scope",
            });
        }
        if water_input.face_averaging != FaceAveraging::Arithmetic {
            return Err(HeatDriverError::Preflight {
                reason: "heat coupling requires arithmetic face averaging (Veloc)",
            });
        }
        if self.state.snapshot().accepted_time != water_state.time() {
            return Err(HeatDriverError::Preflight {
                reason: "heat and water accepted times differ",
            });
        }
        if water_state.heads().len() != n
            || water_state.water_contents().len() != n
            || water_workspace.node_count() != n
            || water_input.sink.len() != n
            || water_input.mesh.coordinates() != self.x.as_slice()
        {
            return Err(HeatDriverError::Preflight {
                reason: "water column does not match heat node coordinates",
            });
        }
        Ok(())
    }
}

/// Node Darcy fluxes from face fluxes, as Fortran `Veloc` (`WATFLOW.FOR:717-775`).
///
/// `face[k]` is the flux between nodes `k-1` and `k` (`face[0]` lower, `face[n]` upper).
/// Interior nodes weight the two faces by the opposite half-spacing (`:744-745`). The bottom
/// and top nodes add or subtract the half-cell storage and sink term (`:769-771`, `:732`).
fn node_velocities(
    x: &[f64],
    face: &[f64],
    water_old: &[f64],
    water_new: &[f64],
    sink: &[f64],
    dt: f64,
    out: &mut [f64],
) {
    let n = x.len();
    for i in 1..n - 1 {
        let dxa = x[i + 1] - x[i];
        let dxb = x[i] - x[i - 1];
        out[i] = (face[i + 1] * dxb + face[i] * dxa) / (dxa + dxb);
    }
    let dx1 = x[1] - x[0];
    out[0] = face[1] + dx1 / 2.0 * ((water_new[0] - water_old[0]) / dt + sink[0]);
    let dxn = x[n - 1] - x[n - 2];
    out[n - 1] =
        face[n - 1] - dxn / 2.0 * ((water_new[n - 1] - water_old[n - 1]) / dt + sink[n - 1]);
}

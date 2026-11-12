//! Atomic two-pass vapor-aware water/heat driver.
//!
//! One step follows `HYDRUS.FOR:399-403,473-499`: water, node velocities, then heat, repeated
//! once with the first heat result as the temperature estimate. Both passes cover the same
//! old-to-new interval from the same accepted snapshot; the second pass never advances a
//! second accepted water step.
//!
//! Atomicity: every pass runs on cloned local candidates. Accepted water, workspace, vapor and
//! heat state change only after both passes and the final diagnostics succeed, exactly once.
//!
//! Retries (`WATFLOW.FOR:136-149`): the driver owns every water time-step cut. Each trial is
//! called with a local policy whose `minimum_timestep` equals the attempt step, so it cannot
//! retry internally. A `NonConverged` result in either pass restarts pass one at
//! `dt / retry_factor` with the old temperatures and the accepted old heads as both the
//! initial guess and the material head (`hOld`). The first attempt starts from the predictor.
//!
//! Head predictor storage convention (`HYDRUS.FOR:650-662,695`): the source forms
//! `hTemp = h + (h - hOld) * dt_next / dt_accepted` after the next step size is known. At commit
//! the driver stores `h + (h - hOld)` (unit ratio) on extrapolated nodes (`h < 0` and `hOld < 0`,
//! not Dirichlet-fixed end nodes), else `h`. At the start of the next step the actual predictor
//! is `h + (stored - h) * dt_attempt / previous_timestep`.
//!
//! Accounting: only `accepted_steps` and `cumulative_iterations` (final passes' iterations) are
//! advanced on the vapor path. The liquid-only volume totals (`cumulative_storage_change`,
//! `cumulative_boundary_flux`, `cumulative_sink`) are not advanced; the total, liquid and vapor
//! face fluxes and liquid/vapor contents needed for a balance are published in
//! [`VaporStepReport`]. Accepted temperatures live only in [`VaporState`].

use super::adjust::VaporHeatAdjustmentInput;
use super::state::{VaporState, VaporStateInput};
use super::VaporWaterTrial;
use super::{VaporConfig, VaporError, VaporHistoryView, VaporSourceMaterial, VaporWaterContext};
use crate::heat::capacity::TempMaterial;
use crate::heat::driver::CoupledHeatInput;
use crate::heat::transport::{solve_vapor_heat_transport, HeatTransportError, HeatTransportInput};
use crate::water_solver::solve_vapor_water_trial;
use crate::{
    LowerBoundaryMode, UpperBoundary, WaterState, WaterStepError, WaterStepInput, WaterStepPolicy,
    WaterWorkspace,
};

/// Typed rejections for [`VaporWaterHeatDriver`]. Accepted state is unchanged on every error.
#[derive(Clone, Debug, PartialEq)]
pub enum VaporDriverError {
    /// Vapor selection, input or state validation failed before any trial.
    Vapor(VaporError),
    /// A water pass failed (including exhausted retries).
    Water(WaterStepError),
    /// Heat failed in the numbered pass (1 or 2).
    Heat {
        pass: u8,
        source: HeatTransportError,
    },
    /// Staging or accepting the vapor history failed.
    State(VaporError),
    /// A driver-level precondition or diagnostic failed.
    Preflight { reason: &'static str },
}

/// Diagnostics for one final water/velocity/heat pass.
#[derive(Clone, Debug, PartialEq)]
pub struct VaporPassReport {
    pub timestep: f64,
    pub iterations: usize,
    /// Temperature estimate given to the water pass and the vapor heat adjustment.
    pub temperature_estimate: Vec<f64>,
    pub initial_heads: Vec<f64>,
    pub initial_material_heads: Vec<f64>,
    /// Solved heads of the pass.
    pub heads: Vec<f64>,
    /// Heat result of the pass.
    pub temperatures: Vec<f64>,
    pub heat_flux: f64,
    pub minimum_absolute_pivot: f64,
}

/// Owned diagnostics of one accepted step, for output and balance.
#[derive(Clone, Debug, PartialEq)]
pub struct VaporStepReport {
    pub accepted_time: f64,
    pub timestep: f64,
    /// Number of pass-one restarts caused by water time-step cuts.
    pub restarts: usize,
    pub passes: [VaporPassReport; 2],
    pub heat_flux: f64,
    pub minimum_absolute_pivot: f64,
    pub liquid_contents_old: Vec<f64>,
    pub liquid_contents_new: Vec<f64>,
    pub vapor_contents_old: Vec<f64>,
    pub vapor_contents_new: Vec<f64>,
    /// Stored pass-two coefficients (`Con`, `ConLT`, `ConVh`, `ConVT`) from the last
    /// pre-final-solve evaluation, at the first heat result; source `NodOut` reads these.
    pub liquid_conductivities: Vec<f64>,
    pub liquid_thermal_conductivities: Vec<f64>,
    pub vapor_head_conductivities: Vec<f64>,
    pub vapor_thermal_conductivities: Vec<f64>,
    /// Pass-two last iterate (`hTemp` before the final Gauss solve) and the liquid content
    /// and vapor-adjusted hydraulic capacity evaluated there; the solved water rows use these.
    pub pre_solve_heads: Vec<f64>,
    pub pre_solve_liquid_water_contents: Vec<f64>,
    pub effective_hydraulic_capacities: Vec<f64>,
    pub liquid_velocities: Vec<f64>,
    pub vapor_velocities: Vec<f64>,
    pub liquid_face_fluxes: Vec<f64>,
    pub vapor_face_fluxes: Vec<f64>,
    pub total_face_fluxes: Vec<f64>,
    pub temperatures: Vec<f64>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Inject {
    Water2NonConvergedOnce,
    AfterHeat1,
    AfterWater2,
    AfterHeat2,
    Diagnostics,
}

/// Vapor-aware coupled driver owning accepted vapor/heat state.
pub struct VaporWaterHeatDriver {
    config: VaporConfig,
    state: VaporState,
    x: Vec<f64>,
    material_indices: Vec<usize>,
    heat_materials: Vec<TempMaterial>,
    #[cfg(test)]
    inject: std::cell::Cell<Option<Inject>>,
}

struct Passes {
    trial1: VaporWaterTrial,
    trial2: VaporWaterTrial,
    reports: [VaporPassReport; 2],
}

impl VaporWaterHeatDriver {
    pub fn new(
        config: VaporConfig,
        state: VaporStateInput<'_>,
        material_indices: &[usize],
        heat_materials: &[TempMaterial],
    ) -> Result<Self, VaporDriverError> {
        let n = state.coordinates.len();
        if material_indices.len() != n {
            return Err(VaporDriverError::Preflight {
                reason: "material index length differs from node count",
            });
        }
        if heat_materials.is_empty() || material_indices.iter().any(|&i| i >= heat_materials.len())
        {
            return Err(VaporDriverError::Preflight {
                reason: "material index outside the heat material table",
            });
        }
        let x = state.coordinates.to_vec();
        let state = VaporState::new(state).map_err(VaporDriverError::Vapor)?;
        Ok(Self {
            config,
            state,
            x,
            material_indices: material_indices.to_vec(),
            heat_materials: heat_materials.to_vec(),
            #[cfg(test)]
            inject: std::cell::Cell::new(None),
        })
    }

    /// Accepted vapor history (staged data is never visible).
    pub fn state(&self) -> &VaporState {
        &self.state
    }

    /// Advances one accepted step through the two source-ordered passes.
    pub fn step(
        &mut self,
        water_state: &mut WaterState,
        water_workspace: &mut WaterWorkspace,
        water_input: WaterStepInput<'_>,
        policy: WaterStepPolicy,
        source_materials: &[VaporSourceMaterial<'_>],
        heat_input: CoupledHeatInput,
    ) -> Result<VaporStepReport, VaporDriverError> {
        self.preflight(
            water_state,
            water_workspace,
            &water_input,
            policy,
            source_materials,
            &heat_input,
        )?;
        let old_heads = water_state.heads().to_vec();
        let (bottom_fixed, top_fixed) = fixed_ends(water_input.lower, water_input.upper);
        let (passes, restarts) = match self.run_passes(
            water_state,
            water_workspace,
            water_input,
            policy,
            source_materials,
            heat_input,
        ) {
            Ok(done) => done,
            Err(error) => {
                self.state.reject();
                return Err(error);
            }
        };
        let Passes {
            trial1,
            trial2,
            reports,
        } = passes;
        let snapshot = self.state.snapshot();
        let liquid_contents_old = snapshot.liquid_water_contents.to_vec();
        let vapor_contents_old = snapshot.vapor_contents.to_vec();
        let temperatures = reports[1].temperatures.clone();
        let predictor =
            commit_predictor(&trial2.candidate.heads, &old_heads, bottom_fixed, top_fixed);
        let timestep = trial2.timestep;
        let accepted_time = trial2.candidate.time;

        #[cfg(test)]
        if let Err(error) = self.fire(Inject::Diagnostics) {
            self.state.reject();
            return Err(error);
        }
        let finish = (|| {
            for values in [
                &trial2.liquid_face_fluxes,
                &trial2.vapor_face_fluxes,
                &trial2.total_face_fluxes,
                &trial2.liquid_velocities,
                &trial2.vapor_velocities,
                &predictor,
            ] {
                if values.iter().any(|v| !v.is_finite()) {
                    return Err(VaporDriverError::Preflight {
                        reason: "non-finite step diagnostic",
                    });
                }
            }
            self.state
                .stage(
                    accepted_time,
                    timestep,
                    &trial2.candidate.heads,
                    &trial2.candidate.water_contents,
                    &temperatures,
                    &trial2.properties.vapor_contents,
                    &trial2.liquid_velocities,
                    &trial2.vapor_velocities,
                    &predictor,
                )
                .map_err(VaporDriverError::State)?;
            self.state.accept().map_err(VaporDriverError::State)
        })();
        if let Err(error) = finish {
            self.state.reject();
            return Err(error);
        }

        // Accepted vapor state is committed; the remaining moves cannot fail.
        let iterations = (trial1.iterations + trial2.iterations) as u64;
        let VaporWaterTrial {
            mut candidate,
            workspace,
            liquid_velocities,
            vapor_velocities,
            properties,
            liquid_face_fluxes,
            vapor_face_fluxes,
            total_face_fluxes,
            pre_solve_heads,
            pre_solve_liquid_water_contents,
            ..
        } = trial2;
        let liquid_contents_new = candidate.water_contents.clone();
        candidate.accounting.accepted_steps += 1;
        candidate.accounting.cumulative_iterations += iterations;
        *water_state = candidate;
        *water_workspace = workspace;
        let [_, second] = &reports;
        Ok(VaporStepReport {
            accepted_time,
            timestep,
            restarts,
            heat_flux: second.heat_flux,
            minimum_absolute_pivot: second.minimum_absolute_pivot,
            passes: reports,
            liquid_contents_old,
            liquid_contents_new,
            vapor_contents_old,
            vapor_contents_new: properties.vapor_contents,
            liquid_conductivities: properties.liquid_conductivities,
            liquid_thermal_conductivities: properties.liquid_thermal_conductivities,
            vapor_head_conductivities: properties.vapor_head_conductivities,
            vapor_thermal_conductivities: properties.vapor_thermal_conductivities,
            pre_solve_heads,
            pre_solve_liquid_water_contents,
            effective_hydraulic_capacities: properties.effective_hydraulic_capacities,
            liquid_velocities,
            vapor_velocities,
            liquid_face_fluxes,
            vapor_face_fluxes,
            total_face_fluxes,
            temperatures,
        })
    }

    fn preflight(
        &self,
        water_state: &WaterState,
        water_workspace: &WaterWorkspace,
        input: &WaterStepInput<'_>,
        policy: WaterStepPolicy,
        source_materials: &[VaporSourceMaterial<'_>],
        heat: &CoupledHeatInput,
    ) -> Result<(), VaporDriverError> {
        let fail = |reason| Err(VaporDriverError::Preflight { reason });
        let n = self.x.len();
        self.config
            .validate_water_input(input, water_state.time(), policy)
            .map_err(VaporDriverError::Vapor)?;
        let old = self.state.snapshot();
        old.validate().map_err(VaporDriverError::Vapor)?;
        if old.time != water_state.time()
            || old.heads != water_state.heads()
            || old.liquid_water_contents != water_state.water_contents()
        {
            return fail("vapor and water accepted snapshots differ");
        }
        if old
            .previous_timestep
            .is_none_or(|dt| dt.is_nan() || dt <= 0.0)
        {
            return fail("accepted previous timestep missing");
        }
        if water_state.heads().len() != n
            || water_workspace.node_count() != n
            || input.mesh.coordinates() != self.x.as_slice()
            || input.mesh.material_indices() != self.material_indices.as_slice()
        {
            return fail("water column does not match vapor driver nodes");
        }
        if source_materials.len() != input.materials.len() {
            return fail("source material table differs from water material table");
        }
        if heat.top_temperature != self.config.top_temperature()
            || heat.bottom_temperature != self.config.bottom_temperature()
            || heat.x_conv != self.config.x_conv()
            || heat.t_conv != self.config.t_conv()
        {
            return fail("heat boundary or conversion differs from vapor config");
        }
        if !heat.minimum_pivot.is_finite() || heat.minimum_pivot <= 0.0 {
            return fail("heat minimum pivot must be positive");
        }
        Ok(())
    }

    #[cfg(test)]
    fn fire(&self, stage: Inject) -> Result<(), VaporDriverError> {
        if self.inject.get() == Some(stage) {
            self.inject.set(None);
            return Err(VaporDriverError::Preflight {
                reason: "injected test failure",
            });
        }
        Ok(())
    }

    /// Runs both passes, owning every retry. Returns the final pair and restart count.
    fn run_passes(
        &self,
        water_state: &WaterState,
        workspace: &WaterWorkspace,
        water_input: WaterStepInput<'_>,
        policy: WaterStepPolicy,
        source_materials: &[VaporSourceMaterial<'_>],
        heat_input: CoupledHeatInput,
    ) -> Result<(Passes, usize), VaporDriverError> {
        let old = self.state.snapshot();
        let previous = old.previous_timestep.unwrap_or(1.0);
        let mut dt = water_input
            .timestep
            .min(water_input.target_time - water_state.time());
        let mut guess = next_predictor(old.heads, old.head_predictor, dt, previous);
        let mut restarts = 0;
        loop {
            let mut local = policy;
            local.minimum_timestep = dt;
            let mut input = water_input;
            input.timestep = dt;
            let result = self.attempt(
                water_state,
                workspace,
                input,
                local,
                source_materials,
                heat_input,
                &guess,
            );
            match result {
                Ok(passes) => return Ok((passes, restarts)),
                Err(VaporDriverError::Water(WaterStepError::NonConverged {
                    timestep,
                    retries,
                    ..
                })) => {
                    let next = dt / policy.retry_factor;
                    if next < policy.minimum_timestep {
                        return Err(VaporDriverError::Water(WaterStepError::NonConverged {
                            timestep,
                            minimum_timestep: policy.minimum_timestep,
                            retries: retries + restarts,
                        }));
                    }
                    dt = next;
                    restarts += 1;
                    guess = old.heads.to_vec();
                }
                Err(error) => return Err(error),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn attempt(
        &self,
        water_state: &WaterState,
        workspace: &WaterWorkspace,
        input: WaterStepInput<'_>,
        policy: WaterStepPolicy,
        source_materials: &[VaporSourceMaterial<'_>],
        heat_input: CoupledHeatInput,
        guess: &[f64],
    ) -> Result<Passes, VaporDriverError> {
        let snapshot = self.state.snapshot();
        let n = self.x.len();
        let mut old: VaporHistoryView<'_> = snapshot;
        old.head_predictor = guess;
        let water_error = |source| VaporDriverError::Water(source);

        // Pass one: old temperatures, predictor (or old heads after a restart).
        let trial1 = solve_vapor_water_trial(
            water_state,
            workspace,
            input,
            policy,
            VaporWaterContext {
                config: &self.config,
                old,
                temperature_estimate: snapshot.temperatures,
                initial_heads: guess,
                initial_material_heads: guess,
                source_materials,
            },
        )
        .map_err(water_error)?;
        let mut heat1 = vec![0.0; n];
        let heat_report1 = self.heat_pass(
            1,
            &snapshot,
            &trial1,
            snapshot.temperatures,
            input.sink,
            heat_input,
            &mut heat1,
        )?;
        #[cfg(test)]
        self.fire(Inject::AfterHeat1)?;

        // Pass two: same old storage, first heat result as temperature estimate.
        #[cfg(test)]
        if self.inject.get() == Some(Inject::Water2NonConvergedOnce) {
            self.inject.set(None);
            return Err(VaporDriverError::Water(WaterStepError::NonConverged {
                timestep: trial1.timestep,
                minimum_timestep: policy.minimum_timestep,
                retries: 0,
            }));
        }
        let trial2 = solve_vapor_water_trial(
            water_state,
            workspace,
            input,
            policy,
            VaporWaterContext {
                config: &self.config,
                old,
                temperature_estimate: &heat1,
                initial_heads: &trial1.candidate.heads,
                initial_material_heads: &trial1.pre_solve_heads,
                source_materials,
            },
        )
        .map_err(water_error)?;
        #[cfg(test)]
        self.fire(Inject::AfterWater2)?;
        if trial2.timestep != trial1.timestep {
            return Err(VaporDriverError::Preflight {
                reason: "water passes used different timesteps",
            });
        }
        let mut heat2 = vec![0.0; n];
        let heat_report2 = self.heat_pass(
            2, &snapshot, &trial2, &heat1, input.sink, heat_input, &mut heat2,
        )?;
        #[cfg(test)]
        self.fire(Inject::AfterHeat2)?;
        let reports = [
            VaporPassReport {
                timestep: trial1.timestep,
                iterations: trial1.iterations,
                temperature_estimate: snapshot.temperatures.to_vec(),
                initial_heads: guess.to_vec(),
                initial_material_heads: guess.to_vec(),
                heads: trial1.candidate.heads.clone(),
                temperatures: heat1.clone(),
                heat_flux: heat_report1.0,
                minimum_absolute_pivot: heat_report1.1,
            },
            VaporPassReport {
                timestep: trial2.timestep,
                iterations: trial2.iterations,
                temperature_estimate: heat1.clone(),
                initial_heads: trial1.candidate.heads.clone(),
                initial_material_heads: trial1.pre_solve_heads.clone(),
                heads: trial2.candidate.heads.clone(),
                temperatures: heat2,
                heat_flux: heat_report2.0,
                minimum_absolute_pivot: heat_report2.1,
            },
        ];
        Ok(Passes {
            trial1,
            trial2,
            reports,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn heat_pass(
        &self,
        pass: u8,
        old: &VaporHistoryView<'_>,
        trial: &VaporWaterTrial,
        estimate: &[f64],
        sink: &[f64],
        heat: CoupledHeatInput,
        out: &mut [f64],
    ) -> Result<(f64, f64), VaporDriverError> {
        let adjust = VaporHeatAdjustmentInput {
            coordinates: &self.x,
            temperature_old: old.temperatures,
            temperature_new_estimate: estimate,
            vapor_velocity_old: old.vapor_velocities,
            vapor_velocity_new: &trial.vapor_velocities,
            vapor_content_old: old.vapor_contents,
            vapor_content_new: &trial.properties.vapor_contents,
            timestep: trial.timestep,
            x_conv: heat.x_conv,
            t_conv: heat.t_conv,
        };
        let report = solve_vapor_heat_transport(
            self.config,
            HeatTransportInput {
                x: &self.x,
                dt: trial.timestep,
                material_indices: &self.material_indices,
                materials: &self.heat_materials,
                temp_old: old.temperatures,
                water_old: old.liquid_water_contents,
                water_new: &trial.candidate.water_contents,
                velocity_old: old.liquid_velocities,
                velocity_new: &trial.liquid_velocities,
                sink,
                top_temperature: heat.top_temperature,
                bottom_temperature: heat.bottom_temperature,
                x_conv: heat.x_conv,
                t_conv: heat.t_conv,
                minimum_pivot: heat.minimum_pivot,
            },
            &adjust,
            out,
        )
        .map_err(|source| VaporDriverError::Heat { pass, source })?;
        Ok((report.heat_flux, report.minimum_absolute_pivot))
    }
}

/// End nodes held by Dirichlet heads (`iBot`/`iTop`, `WATFLOW.FOR:1090-1094`).
fn fixed_ends(lower: LowerBoundaryMode, upper: UpperBoundary) -> (bool, bool) {
    (
        matches!(lower, LowerBoundaryMode::FixedHead { .. }),
        upper.fixes_head(),
    )
}

/// Unit-ratio predictor stored at commit (see module docs).
fn commit_predictor(heads: &[f64], old: &[f64], bottom_fixed: bool, top_fixed: bool) -> Vec<f64> {
    let first = usize::from(bottom_fixed);
    let end = heads.len() - usize::from(top_fixed);
    heads
        .iter()
        .zip(old)
        .enumerate()
        .map(|(i, (&h, &h_old))| {
            if i >= first && i < end && h < 0.0 && h_old < 0.0 {
                h + (h - h_old)
            } else {
                h
            }
        })
        .collect()
}

/// Actual predictor for the attempted step: `h + (stored - h) * dt / previous_dt`.
fn next_predictor(heads: &[f64], stored: &[f64], dt: f64, previous: f64) -> Vec<f64> {
    heads
        .iter()
        .zip(stored)
        .map(|(&h, &p)| h + (p - h) * dt / previous)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heat::config::HeatConfigInput;
    use crate::heat::vapor::{VaporConfigInput, VaporPrimitiveInput};
    use crate::hydraulics::{
        BoundaryRow, FaceAveraging, HydraulicModel, Mesh1D, VanGenuchtenModel,
        VanGenuchtenParameters,
    };

    fn config() -> VaporConfig {
        VaporConfig::new(VaporConfigInput {
            heat: HeatConfigInput {
                l_vapor: true,
                l_en_bal: false,
                i_dual_por: 0,
                l_w_dep: false,
                i_model: 0,
                domain_count: 1,
                k_top_t: 1,
                t_period: 0.0,
                k_bot_t: 1,
                campbell: false,
            },
            water_enabled: true,
            heat_enabled: true,
            i_enhanc: 1,
            hysteresis: 0,
            chemistry: false,
            density_coupling: false,
            centrifugal: false,
            top_temperature: 30.0,
            bottom_temperature: 10.0,
            x_conv: 100.0,
            t_conv: 1.0 / 86400.0,
        })
        .unwrap()
    }

    fn row(value: f64) -> BoundaryRow {
        BoundaryRow {
            lower: 0.0,
            diagonal: 1.0,
            upper: 0.0,
            rhs: value,
        }
    }

    #[test]
    fn predictor_is_rescaled_by_next_step_ratio_and_skips_nonnegative_heads() {
        // Hand worked: h=-10, hOld=-12 gives stored -8; dt_prev=1, dt_next=0.5 -> -9.
        let stored = commit_predictor(
            &[-9.0, -10.0, -4.0, -3.0, -2.0],
            &[-9.0, -12.0, 2.0, -5.0, -1.0],
            true,
            true,
        );
        assert_eq!(stored, vec![-9.0, -8.0, -4.0, -1.0, -2.0]);
        let next = next_predictor(&[-9.0, -10.0, -4.0, -3.0, -2.0], &stored, 0.5, 1.0);
        assert_eq!(next, vec![-9.0, -9.0, -4.0, -2.0, -2.0]);
    }

    /// Runs `f` with a fresh driver and a 3-node column.
    fn with_fixture(
        f: impl FnOnce(
            &mut VaporWaterHeatDriver,
            &mut WaterState,
            &mut WaterWorkspace,
            WaterStepInput<'_>,
            WaterStepPolicy,
            &[VaporSourceMaterial<'_>],
            CoupledHeatInput,
        ),
    ) {
        let model = VanGenuchtenModel::new(
            VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0e-5).unwrap(),
        )
        .unwrap();
        let x = [0.0, 50.0, 100.0];
        let mesh = Mesh1D::new(x.to_vec(), vec![0; 3], None, 1, 1, 1.0).unwrap();
        let heads = [-100.0; 3];
        let e = model.evaluate(-100.0).unwrap();
        let theta = [e.theta; 3];
        let temps = [10.0, 20.0, 30.0];
        let driver = VaporWaterHeatDriver::new(
            config(),
            VaporStateInput {
                primitive: VaporPrimitiveInput {
                    heads: &heads,
                    temperatures: &temps,
                    liquid_water_contents: &theta,
                    liquid_conductivities: &[e.conductivity; 3],
                    liquid_capacities: &[e.capacity; 3],
                    material_indices: &[0; 3],
                    saturated_water_contents: &[0.45],
                    x_conv: 100.0,
                    t_conv: 1.0 / 86400.0,
                },
                coordinates: &x,
                gravity: 1.0,
                initial_head_predictor: &heads,
                initialization_timestep: 0.1,
                time: 0.0,
            },
            &[0; 3],
            &[{
                // SI values in cm/day model units: capacity / (xConv*tConv^2),
                // conductivity * xConv / tConv^3 (precedent: tests/vapor_balance.rs).
                let (xc, tc) = (100.0_f64, 1.0_f64 / 86400.0);
                let (kc, kl) = (1.0 / (xc * tc * tc), xc / (tc * tc * tc));
                TempMaterial::from_tpar([
                    0.5,
                    0.0,
                    0.0,
                    0.25 * kl,
                    0.5 * kl,
                    1.0 * kl,
                    1.92e6 * kc,
                    0.0,
                    4.18e6 * kc,
                ])
            }],
        );
        let mut driver = driver.unwrap();
        let mut state = WaterState::new(0.0, heads.to_vec(), theta.to_vec()).unwrap();
        let mut workspace = WaterWorkspace::new(3).unwrap();
        let materials: [&dyn HydraulicModel; 1] = [&model];
        let sink = [0.0; 3];
        let input = WaterStepInput {
            mesh: &mesh,
            materials: &materials,
            sink: &sink,
            root_uptake: None,
            timestep: 0.1,
            target_time: 0.1,
            face_averaging: FaceAveraging::Arithmetic,
            lower: LowerBoundaryMode::FixedHead {
                head: -90.0,
                row: row(-90.0),
                flux: 0.0,
            },
            upper: UpperBoundary::Head {
                head: -110.0,
                row: row(-110.0),
                flux: 0.0,
            },
        };
        let policy = WaterStepPolicy {
            max_iterations: 20,
            head_tolerance: 1.0e-8,
            water_tolerance: 1.0e-8,
            minimum_timestep: 1.0e-6,
            retry_factor: 3.0,
            minimum_pivot: 1.0e-14,
        };
        let source = [VaporSourceMaterial {
            model: &model,
            table: None,
        }];
        let heat = CoupledHeatInput {
            top_temperature: 30.0,
            bottom_temperature: 10.0,
            x_conv: 100.0,
            t_conv: 1.0 / 86400.0,
            minimum_pivot: 1.0e-14,
        };
        f(
            &mut driver,
            &mut state,
            &mut workspace,
            input,
            policy,
            &source,
            heat,
        );
    }

    #[test]
    fn injected_failure_at_every_stage_leaves_accepted_state_unchanged() {
        for stage in [
            Inject::AfterHeat1,
            Inject::AfterWater2,
            Inject::AfterHeat2,
            Inject::Diagnostics,
        ] {
            with_fixture(|driver, state, workspace, input, policy, source, heat| {
                let vapor_before = driver.state.clone();
                let water_before = state.clone();
                let workspace_before = format!("{workspace:?}");
                driver.inject.set(Some(stage));
                let error = driver
                    .step(state, workspace, input, policy, source, heat)
                    .unwrap_err();
                assert_eq!(
                    error,
                    VaporDriverError::Preflight {
                        reason: "injected test failure"
                    },
                    "{stage:?}"
                );
                assert_eq!(driver.state, vapor_before, "{stage:?}");
                assert!(!driver.state.has_staged_candidate());
                assert_eq!(*state, water_before, "{stage:?}");
                assert_eq!(format!("{workspace:?}"), workspace_before, "{stage:?}");
                // The same driver, state and workspace still complete a clean step.
                let report = driver
                    .step(state, workspace, input, policy, source, heat)
                    .unwrap();
                assert_eq!(state.accounting().accepted_steps, 1);
                assert_eq!(state.time(), report.accepted_time);
            });
        }
    }

    #[test]
    fn pass_two_retry_restarts_pass_one_from_old_heads_and_temperatures() {
        with_fixture(|driver, state, workspace, input, policy, source, heat| {
            driver.inject.set(Some(Inject::Water2NonConvergedOnce));
            let report = driver
                .step(state, workspace, input, policy, source, heat)
                .unwrap();
            assert_eq!(report.restarts, 1);
            assert_eq!(report.timestep, 0.1 / 3.0);
            assert_eq!(report.passes[0].timestep, 0.1 / 3.0);
            assert_eq!(report.passes[0].initial_heads, vec![-100.0; 3]);
            assert_eq!(report.passes[0].initial_material_heads, vec![-100.0; 3]);
            assert_eq!(
                report.passes[0].temperature_estimate,
                vec![10.0, 20.0, 30.0]
            );
            assert_eq!(state.time(), 0.1 / 3.0);
            assert_eq!(state.accounting().accepted_steps, 1);
        });
    }
}

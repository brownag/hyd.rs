use crate::atmospheric::{
    decide_transition, AtmosphericAssembly, AtmosphericTransition, UpperBoundaryMode,
};
use crate::forcing::AtmosphericRates;
use crate::heat::vapor::kernels;
use crate::heat::vapor::water::{
    assemble_water_rows, phase_velocities, PhaseVelocityInput, VaporWaterRowsInput,
};
use crate::heat::vapor::{VaporError, VaporPrimitiveInput, VaporWaterContext, VaporWaterTrial};
use crate::hydraulics::{
    arithmetic_face_conductivity, geometric_face_conductivity, nodal_control_volume_width,
    solve_tridiagonal_in_place, transform_boundary_row, BoundaryCondition, BoundaryRow,
    BoundaryRowLocation, FaceAveraging, HydraulicModel, Mesh1D, PivotDiagnostics, ResetRow,
    ResetRowInput,
};
use crate::lower_boundary::{LowerBoundaryMode, LowerTrial};
use crate::root_uptake::RootUptakeInput;
use crate::water_state::{validate_finite, AcceptedWaterSnapshot};
use crate::{WaterState, WaterStepError, WaterWorkspace};

/// Validated capability contract for the first single-domain water product.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaterProductConfig {
    hydraulic_model: crate::HydraulicModelSelector,
}

impl WaterProductConfig {
    /// Accepts only `iModel=0` and exactly one water domain.
    pub fn new(i_model: i32, domain_count: usize) -> Result<Self, WaterStepError> {
        let hydraulic_model = crate::HydraulicModelSelector::from_i32(i_model)?;
        if domain_count != 1 {
            return Err(WaterStepError::UnsupportedConfiguration {
                name: "domain_count",
                value: domain_count,
            });
        }
        Ok(Self { hydraulic_model })
    }

    pub fn hydraulic_model(self) -> crate::HydraulicModelSelector {
        self.hydraulic_model
    }

    pub fn domain_count(self) -> usize {
        1
    }
}

/// Fixed policy for Picard convergence, pivot checks, and adaptive retry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterStepPolicy {
    pub max_iterations: usize,
    pub head_tolerance: f64,
    pub water_tolerance: f64,
    pub minimum_timestep: f64,
    pub retry_factor: f64,
    pub minimum_pivot: f64,
}

/// Caller-owned hydraulic models and deterministic fields used by the Phase 3 water solver.
#[derive(Clone, Copy)]
pub struct WaterStepInput<'a> {
    pub mesh: &'a Mesh1D,
    pub materials: &'a [&'a dyn HydraulicModel],
    pub sink: &'a [f64],
    /// Optional Feddes root uptake added to the caller-provided sink field.
    pub root_uptake: Option<&'a RootUptakeInput<'a>>,
    pub timestep: f64,
    /// Caller-selected time that this step must not pass.
    pub target_time: f64,
    pub face_averaging: FaceAveraging,
    /// Lower endpoint selection; fixed-head carries its row and measured exchange.
    pub lower: LowerBoundaryMode,
    /// Upper endpoint selection; each mode carries only the data it reads.
    pub upper: UpperBoundary,
}

/// Physical upper-boundary selections supported by the transactional water solver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UpperBoundary {
    /// Prescribe the top-node pressure head. `row` is the caller-supplied
    /// upper row and `flux` the measured signed exchange, positive into the domain.
    Head {
        head: f64,
        row: BoundaryRow,
        flux: f64,
    },
    /// Prescribe signed water flux, positive into the domain, with the
    /// caller-supplied upper row.
    Flux { flux: f64, row: BoundaryRow },
    /// Atmospheric flux/head switch; the row and exchange are derived each
    /// trial and `critical_head` (`hCritA`) bounds the surface head.
    Atmospheric { critical_head: f64 },
}

impl UpperBoundary {
    /// Atmospheric mode counts as prescribed head, as its former Dirichlet critical head did.
    pub(crate) fn fixes_head(self) -> bool {
        matches!(self, Self::Head { .. } | Self::Atmospheric { .. })
    }

    /// Signed measured exchange that must be finite before a step starts.
    pub(crate) fn measured_flux(self) -> Option<f64> {
        match self {
            Self::Head { flux, .. } | Self::Flux { flux, .. } => Some(flux),
            Self::Atmospheric { .. } => None,
        }
    }

    /// Condition and caller row for a non-atmospheric step.
    fn caller_row(self) -> Result<(BoundaryCondition, BoundaryRow), WaterStepError> {
        match self {
            Self::Head { head, row, .. } => Ok((BoundaryCondition::Dirichlet { value: head }, row)),
            Self::Flux { flux, row } => Ok((BoundaryCondition::Neumann { flux }, row)),
            Self::Atmospheric { .. } => Err(WaterStepError::InvalidUpperBoundary {
                name: "atmospheric upper boundary requires advance_atmospheric_step",
            }),
        }
    }

    /// The atmospheric critical head, required by the atmospheric step.
    pub(crate) fn critical_head(self) -> Result<f64, WaterStepError> {
        match self {
            Self::Atmospheric { critical_head } => Ok(critical_head),
            _ => Err(WaterStepError::InvalidUpperBoundary {
                name: "advance_atmospheric_step requires UpperBoundary::Atmospheric",
            }),
        }
    }
}

/// Accepted-step water-balance diagnostic, with all quantities per unit
/// cross-sectional area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterBalance {
    /// Total soil and surface storage change per unit cross-sectional area.
    pub storage_change: f64,
    /// Mesh-integrated soil-water storage change per unit cross-sectional area.
    pub soil_storage_change: f64,
    /// Surface storage change per unit cross-sectional area.
    pub surface_storage_change: f64,
    /// Signed water entering the soil through endpoint boundaries over the step.
    pub soil_boundary_flux: f64,
    /// Signed external atmospheric exchange over the step.
    pub atmospheric_exchange: f64,
    /// Signed external water exchange integrated over the accepted timestep.
    pub boundary_flux: f64,
    /// Mesh-integrated sink over the accepted timestep per unit cross-sectional area.
    pub sink: f64,
    pub absolute_residual: f64,
    /// `absolute_residual` when every balance term is zero; otherwise residual divided by term magnitude.
    pub normalized_residual: f64,
}

/// Accepted-step report; all values are finite on success.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterStepReport<'a> {
    pub accepted_time: f64,
    pub timestep: f64,
    pub iterations: usize,
    pub retries: usize,
    pub max_head_residual: f64,
    pub max_water_residual: f64,
    pub minimum_absolute_pivot: f64,
    pub lower_boundary_mode: LowerBoundaryMode,
    /// Number of atmospheric flux/head mode changes during the accepted solve.
    pub atmospheric_mode_transitions: usize,
    /// Trial-state values that caused each active-set mode change.
    pub atmospheric_transitions: &'a [AtmosphericTransition],
    pub balance: WaterBalance,
    /// Accepted soil water contents, one per node.
    pub water_contents: &'a [f64],
    /// Accepted face fluxes, `node_count + 1`, positive toward the upper boundary.
    pub face_fluxes: &'a [f64],
}

impl<'a> WaterStepReport<'a> {
    /// The accepted water values a solute step may rely on.
    pub fn accepted_water(&self) -> AcceptedWaterSnapshot<'a> {
        AcceptedWaterSnapshot {
            accepted_time: self.accepted_time,
            water_contents: self.water_contents,
            face_fluxes: self.face_fluxes,
        }
    }
}

pub fn advance_water_step<'a>(
    state: &mut WaterState,
    workspace: &'a mut WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
) -> Result<WaterStepReport<'a>, WaterStepError> {
    advance_water_step_impl(state, workspace, input, policy, None)
}

/// Source `rMax` (`WATFLOW.FOR:35`).
const SOURCE_R_MAX: f64 = 1.0e10;

/// Source `hSat(M)=FH(iModel,1.0,Par)` (`INPUT.FOR:792`, `MATERIAL.FOR:350-374`,
/// iModel 0). The source's final `sngl()` rounding is not reproduced.
fn source_h_sat(parameters: &crate::hydraulics::VanGenuchtenParameters) -> f64 {
    let alpha = parameters.alpha();
    let n = parameters.n();
    let m = 1.0 - 1.0 / n;
    let h_min = -(1.0e300_f64.powf(1.0 / n)) / alpha.max(1.0);
    let qee_m = (1.0 + (-alpha * h_min).powf(n)).powf(-m);
    let qs = parameters.saturated_water_content();
    let qr = parameters.residual_water_content();
    let qee = (1.0 * (qs - qr) / (qs - qr))
        .max(qee_m)
        .min(0.999999999999999);
    (-1.0 / alpha * (qee.powf(-1.0 / m) - 1.0).powf(1.0 / n)).max(-1.0e37)
}

/// Solves one uncommitted vapor-aware nonlinear water trial.
///
/// Iteration semantics follow `WatFlow` (`WATFLOW.FOR:60-160`): `SetMat` mixes
/// the previous pre-solve iterate with the current iterate; convergence is the
/// per-node `EpsTh`/`TolH` rule; a converged solve is accepted only from the
/// second solve onward, so `max_iterations < 2` can never converge and returns
/// `NonConverged`; the theta correction uses the pre-solve iterate.
///
/// This crate-private seam deliberately returns owned candidate state and
/// diagnostics. The coupled driver owns acceptance/rollback; ordinary public
/// water entry points continue to use the unchanged liquid-only path.
#[allow(dead_code)] // consumed by the vapor coupled driver in task 25.5.1
pub(crate) fn solve_vapor_water_trial(
    state: &WaterState,
    workspace: &WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
    context: VaporWaterContext<'_>,
) -> Result<VaporWaterTrial, WaterStepError> {
    context.old.validate()?;
    context
        .config
        .validate_water_input(&input, state.time, policy)?;
    let n = input.mesh.coordinates().len();
    for (name, values) in [
        ("temperature_estimate", context.temperature_estimate),
        ("initial_heads", context.initial_heads),
        ("initial_material_heads", context.initial_material_heads),
    ] {
        if values.len() != n {
            return Err(VaporError::LengthMismatch {
                name,
                expected: n,
                actual: values.len(),
            }
            .into());
        }
        for (index, &value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(VaporError::NonFinite {
                    name,
                    index: Some(index),
                    value,
                }
                .into());
            }
        }
    }
    if context.source_materials.len() != input.materials.len() {
        return Err(VaporError::LengthMismatch {
            name: "source_materials",
            expected: input.materials.len(),
            actual: context.source_materials.len(),
        }
        .into());
    }
    if context.old.heads.len() != n || context.old.temperatures.len() != n {
        return Err(VaporError::LengthMismatch {
            name: "accepted vapor history",
            expected: n,
            actual: context.old.heads.len(),
        }
        .into());
    }
    if context.old.time != state.time
        || context.old.heads != state.heads
        || context.old.liquid_water_contents != state.water_contents
    {
        return Err(VaporError::InvalidValue {
            name: "accepted water/vapor snapshot mismatch",
            value: context.old.time - state.time,
        }
        .into());
    }
    let mut candidate_workspace = workspace.clone();
    let mut candidate = state.clone();
    let mut dt = input.timestep.min(input.target_time - state.time);
    let mut retries = 0;
    loop {
        candidate_workspace.snapshot(&candidate);
        candidate_workspace
            .trial_heads
            .copy_from_slice(context.initial_heads);
        let mut converged = None;
        // SetMat `hTemp`: the pre-solve iterate of the previous pass.
        let mut material_heads = context.initial_material_heads.to_vec();
        for iteration in 1..=policy.max_iterations {
            let h_temp = material_heads.clone();
            // WatFlow `hTemp=hNew` before Gauss (`WATFLOW.FOR:94-96`).
            let iterate_heads = candidate_workspace.trial_heads.clone();
            let mut liquid = Vec::with_capacity(n);
            let mut capacities = Vec::with_capacity(n);
            let mut liquid_theta = Vec::with_capacity(n);
            let saturated: Vec<f64> = context
                .source_materials
                .iter()
                .map(|material| material.model.parameters().saturated_water_content())
                .collect();
            for (node, &head_temp) in h_temp.iter().enumerate() {
                let material = input.mesh.material_indices()[node];
                // SetMat clamps both heads to `hSat` before mixing
                // (`WATFLOW.FOR:455-462`); `Ah = AT = 1` here.
                let h_sat = source_h_sat(context.source_materials[material].model.parameters());
                let mixed_head =
                    clamped_mixed_head(head_temp, candidate_workspace.trial_heads[node], h_sat);
                let evaluation =
                    evaluate_vapor_liquid(context.source_materials[material], mixed_head)?;
                liquid.push(evaluation.conductivity);
                capacities.push(evaluation.capacity);
                liquid_theta.push(evaluation.theta);
            }
            let primitive = VaporPrimitiveInput {
                heads: &candidate_workspace.trial_heads,
                temperatures: context.temperature_estimate,
                liquid_water_contents: &liquid_theta,
                liquid_conductivities: &liquid,
                liquid_capacities: &capacities,
                material_indices: input.mesh.material_indices(),
                saturated_water_contents: &saturated,
                x_conv: context.config.x_conv(),
                t_conv: context.config.t_conv(),
            };
            // Keep the per-iteration properties owned before row construction.
            let properties = kernels::evaluate(&primitive)?;
            let rows = assemble_water_rows(&VaporWaterRowsInput {
                coordinates: input.mesh.coordinates(),
                gravity: input.mesh.inclination_cosine(),
                liquid_conductivities: &liquid,
                effective_capacities: &properties.effective_hydraulic_capacities,
                heads_new: &candidate_workspace.trial_heads,
                liquid_water_old: &state.water_contents,
                liquid_water_new: &liquid_theta,
                vapor_water_old: context.old.vapor_contents,
                vapor_water_new: &properties.vapor_contents,
                temperatures: context.temperature_estimate,
                sink: input.sink,
                timestep: dt,
                vapor: &properties,
            })?;
            let (upper_mode, lower_flux) = assemble(
                &mut candidate_workspace,
                state,
                input,
                policy.minimum_pivot,
                dt,
                None,
            )?;
            for (node, vapor_row) in (1..n - 1).zip(rows) {
                // Replace the liquid-only row, not add it a second time.
                candidate_workspace.lower[node] = vapor_row.lower;
                candidate_workspace.diagonal[node] = vapor_row.diagonal;
                candidate_workspace.upper[node] = vapor_row.upper;
                candidate_workspace.rhs[node] = vapor_row.rhs;
            }
            let pivots = solve_tridiagonal_in_place(
                &candidate_workspace.lower,
                &mut candidate_workspace.diagonal,
                &candidate_workspace.upper,
                &mut candidate_workspace.rhs,
                policy.minimum_pivot,
            )?;
            let mut solved_heads = candidate_workspace.rhs.clone();
            for head in solved_heads.iter_mut() {
                if head.abs() > SOURCE_R_MAX {
                    *head = SOURCE_R_MAX.copysign(*head); // WATFLOW.FOR:96-97
                }
            }
            // Source convergence test (`WATFLOW.FOR:105-127`), ATh=1, Rate=1.
            let mut r_max_failure = false;
            let mut node_converged = true;
            for node in 0..n {
                let material = input.mesh.material_indices()[node];
                let parameters = context.source_materials[material].model.parameters();
                let h_sat = source_h_sat(parameters);
                let (eps_th, eps_h) = if iterate_heads[node] < h_sat && solved_heads[node] < h_sat {
                    let span =
                        parameters.saturated_water_content() - parameters.residual_water_content();
                    (
                        (properties.effective_hydraulic_capacities[node]
                            * (solved_heads[node] - iterate_heads[node])
                            / span)
                            .abs(),
                        0.0,
                    )
                } else {
                    (0.0, (solved_heads[node] - iterate_heads[node]).abs())
                };
                if solved_heads[node].abs() > SOURCE_R_MAX * 0.999 {
                    r_max_failure = true;
                }
                if eps_th > policy.water_tolerance
                    || eps_h > policy.head_tolerance
                    || solved_heads[node].abs() > SOURCE_R_MAX * 0.999
                {
                    node_converged = false;
                    break;
                }
            }
            if r_max_failure {
                break; // source sets Iter=MaxIt: this attempt fails
            }
            // `Iter.le.1` forces a second solve (`WATFLOW.FOR:128`).
            if node_converged && iteration >= 2 {
                let corrected_theta: Vec<f64> = (0..n)
                    .map(|node| {
                        liquid_theta[node]
                            + properties.effective_hydraulic_capacities[node]
                                * (solved_heads[node] - iterate_heads[node])
                    })
                    .collect();
                if corrected_theta.iter().any(|theta| !theta.is_finite()) {
                    return Err(WaterStepError::NonFiniteDiagnostic);
                }
                converged = Some((
                    iteration,
                    pivots,
                    lower_flux,
                    upper_mode,
                    corrected_theta,
                    liquid_theta,
                    properties,
                    iterate_heads,
                    solved_heads,
                    capacities,
                    h_temp,
                ));
                break;
            }
            candidate_workspace
                .trial_heads
                .copy_from_slice(&solved_heads);
            candidate_workspace
                .trial_water
                .copy_from_slice(&liquid_theta);
            material_heads = iterate_heads;
        }
        if let Some((
            iterations,
            _pivots,
            lower_flux,
            _upper_mode,
            corrected,
            pre_solve_liquid_water,
            properties,
            pre_solve_heads,
            solved_heads,
            pre_solve_liquid_capacities,
            _h_temp,
        )) = converged
        {
            candidate.heads.copy_from_slice(&solved_heads);
            candidate_workspace
                .trial_heads
                .copy_from_slice(&solved_heads);
            candidate.water_contents.copy_from_slice(&corrected);
            let phase = phase_velocities(&PhaseVelocityInput {
                coordinates: input.mesh.coordinates(),
                gravity: input.mesh.inclination_cosine(),
                heads: &candidate.heads,
                liquid_conductivities: &properties.liquid_conductivities,
                liquid_water_old: context.old.liquid_water_contents,
                liquid_water_new: &corrected,
                sink: input.sink,
                temperatures: context.temperature_estimate,
                vapor_water_old: context.old.vapor_contents,
                vapor_water_new: &properties.vapor_contents,
                timestep: dt,
                vapor: &properties,
            })?;
            let accepted_time = (state.time + dt).min(input.target_time);
            candidate.time = accepted_time;
            let mut liquid_face_fluxes = vec![0.0; n + 1];
            let mut vapor_face_fluxes = vec![0.0; n + 1];
            let mut total_face_fluxes = vec![0.0; n + 1];
            liquid_face_fluxes[0] = lower_flux;
            let upper_flux = input.upper.measured_flux().unwrap_or(0.0);
            liquid_face_fluxes[n] = -upper_flux;
            vapor_face_fluxes[0] = phase.vapor[0];
            vapor_face_fluxes[n] = phase.vapor[n - 1];
            for face in 1..n {
                let left = face - 1;
                let right = face;
                let dx = input.mesh.coordinates()[right] - input.mesh.coordinates()[left];
                liquid_face_fluxes[face] = -arithmetic_face_conductivity(
                    properties.liquid_conductivities[left],
                    properties.liquid_conductivities[right],
                )? * ((candidate.heads[right] - candidate.heads[left])
                    / dx
                    + input.mesh.inclination_cosine());
                vapor_face_fluxes[face] = -arithmetic_face_conductivity(
                    properties.vapor_head_conductivities[left],
                    properties.vapor_head_conductivities[right],
                )? * (candidate.heads[right] - candidate.heads[left])
                    / dx
                    - arithmetic_face_conductivity(
                        properties.vapor_thermal_conductivities[left],
                        properties.vapor_thermal_conductivities[right],
                    )? * (context.temperature_estimate[right]
                        - context.temperature_estimate[left])
                        / dx;
            }
            for face in 0..=n {
                total_face_fluxes[face] = liquid_face_fluxes[face] + vapor_face_fluxes[face];
            }
            candidate_workspace
                .face_fluxes
                .copy_from_slice(&liquid_face_fluxes);
            let liquid_velocities = phase.liquid.clone();
            let vapor_velocities = phase.vapor.clone();
            return Ok(VaporWaterTrial {
                candidate,
                workspace: candidate_workspace,
                accepted_time,
                timestep: dt,
                iterations,
                retries,
                pre_solve_heads,
                pre_solve_liquid_water_contents: pre_solve_liquid_water,
                pre_solve_liquid_capacities,
                liquid_velocities,
                vapor_velocities,
                properties,
                liquid_face_fluxes,
                vapor_face_fluxes,
                total_face_fluxes,
            });
        }
        let next = dt / policy.retry_factor;
        if next < policy.minimum_timestep {
            return Err(WaterStepError::NonConverged {
                timestep: dt,
                minimum_timestep: policy.minimum_timestep,
                retries,
            });
        }
        dt = next;
        retries += 1;
    }
}

/// `hi1=min(hSat,hTemp)`, `hi2=min(hSat,hNew)`, `0.1*hi1+0.9*hi2`.
fn clamped_mixed_head(h_temp: f64, h_new: f64, h_sat: f64) -> f64 {
    0.1 * h_temp.min(h_sat) + 0.9 * h_new.min(h_sat)
}

fn evaluate_vapor_liquid(
    source: crate::heat::vapor::VaporSourceMaterial<'_>,
    head: f64,
) -> Result<crate::hydraulics::Evaluation, WaterStepError> {
    let evaluation = match source.table {
        Some(table) if head >= table.heads()[0] && head <= *table.heads().last().unwrap() => {
            table.evaluate(head)
        }
        // SetMat falls back to analytic `FK`/`FC`/`FQ` outside the table span.
        Some(_) | None => source.model.evaluate(head),
    }?;
    Ok(evaluation)
}

pub(crate) fn advance_atmospheric_inner<'a>(
    state: &mut WaterState,
    workspace: &'a mut WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
    rates: AtmosphericRates,
) -> Result<WaterStepReport<'a>, WaterStepError> {
    advance_water_step_impl(state, workspace, input, policy, Some(rates))
}

fn advance_water_step_impl<'a>(
    state: &mut WaterState,
    workspace: &'a mut WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
    atmospheric_rates: Option<AtmosphericRates>,
) -> Result<WaterStepReport<'a>, WaterStepError> {
    validate(state, workspace, input, policy)?;
    if atmospheric_rates.is_some()
        && workspace.atmospheric_transitions.len() < policy.max_iterations
    {
        return Err(WaterStepError::InsufficientAtmosphericTraceCapacity {
            required: policy.max_iterations,
            available: workspace.atmospheric_transitions.len(),
        });
    }
    let remaining = input.target_time - state.time;
    let mut timestep = input.timestep.min(remaining);
    let mut retries = 0;
    loop {
        workspace.snapshot(state);
        match solve_trial(state, workspace, input, policy, timestep, atmospheric_rates)? {
            Some(trial) => {
                let (next_ponded, upper_flux, soil_upper_flux) = match atmospheric_rates {
                    Some(_) => (
                        trial.ponded_depth,
                        Some(trial.atmospheric_exchange),
                        Some(trial.soil_flux),
                    ),
                    None => (state.atmospheric.ponded_depth, None, None),
                };
                let balance = balance(
                    state,
                    workspace,
                    input,
                    BalanceInput {
                        timestep,
                        atmospheric_flux: upper_flux,
                        soil_atmospheric_flux: soil_upper_flux,
                        lower_boundary_flux: trial.lower_boundary_flux,
                        surface_storage_change: next_ponded - state.atmospheric.ponded_depth,
                    },
                )?;
                fill_face_fluxes(
                    workspace,
                    input,
                    trial.lower_boundary_flux,
                    soil_upper_flux.unwrap_or(input.upper.measured_flux().unwrap_or(0.0)),
                )?;
                state.heads.copy_from_slice(&workspace.trial_heads);
                state.water_contents.copy_from_slice(&workspace.trial_water);
                let accepted_time = if timestep >= input.target_time - state.time {
                    input.target_time
                } else {
                    let next_time = state.time + timestep;
                    if next_time >= input.target_time {
                        input.target_time
                    } else {
                        next_time
                    }
                };
                state.time = accepted_time;
                state.accounting.accepted_steps += 1;
                state.accounting.cumulative_iterations += trial.iterations as u64;
                state.accounting.cumulative_storage_change += balance.storage_change;
                state.accounting.cumulative_boundary_flux += balance.boundary_flux;
                state.accounting.cumulative_sink += balance.sink;
                if atmospheric_rates.is_some() {
                    state.atmospheric.upper_mode = trial.upper_mode;
                    state.atmospheric.ponded_depth = next_ponded;
                }
                return Ok(WaterStepReport {
                    accepted_time,
                    timestep,
                    iterations: trial.iterations,
                    retries,
                    max_head_residual: trial.head_residual,
                    max_water_residual: trial.water_residual,
                    minimum_absolute_pivot: trial.pivots.minimum_absolute_pivot,
                    lower_boundary_mode: input.lower,
                    atmospheric_mode_transitions: workspace.atmospheric_transition_count,
                    atmospheric_transitions: &workspace.atmospheric_transitions
                        [..workspace.atmospheric_transition_count],
                    balance,
                    water_contents: &workspace.trial_water,
                    face_fluxes: &workspace.face_fluxes,
                });
            }
            None => {
                workspace.restore(state);
                let remaining = input.target_time - state.time;
                let next = (timestep / policy.retry_factor).min(remaining);
                if next < policy.minimum_timestep {
                    return Err(WaterStepError::NonConverged {
                        timestep,
                        minimum_timestep: policy.minimum_timestep,
                        retries,
                    });
                }
                timestep = next;
                retries += 1;
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Trial {
    iterations: usize,
    head_residual: f64,
    water_residual: f64,
    pivots: PivotDiagnostics,
    upper_mode: UpperBoundaryMode,
    ponded_depth: f64,
    soil_flux: f64,
    atmospheric_exchange: f64,
    lower_boundary_flux: f64,
}

fn solve_trial(
    state: &WaterState,
    workspace: &mut WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
    timestep: f64,
    atmospheric_rates: Option<AtmosphericRates>,
) -> Result<Option<Trial>, WaterStepError> {
    let mut upper_mode = state.atmospheric.upper_mode;
    let mut ponded_depth = state.atmospheric.ponded_depth;
    let mut soil_flux = 0.0;
    let mut atmospheric_exchange = 0.0;
    workspace.atmospheric_transition_count = 0;
    for iteration in 1..=policy.max_iterations {
        let (assembled_upper_mode, lower_boundary_flux) = assemble(
            workspace,
            state,
            input,
            policy.minimum_pivot,
            timestep,
            atmospheric_rates.map(|rates| AtmosphericAssembly {
                rates,
                upper_mode,
                ponded_depth,
            }),
        )?;
        upper_mode = assembled_upper_mode;
        let pivots = solve_tridiagonal_in_place(
            &workspace.lower,
            &mut workspace.diagonal,
            &workspace.upper,
            &mut workspace.rhs,
            policy.minimum_pivot,
        )?;
        let mut head_residual: f64 = 0.0;
        let mut water_residual: f64 = 0.0;
        for index in 0..workspace.node_count() {
            let previous_head = workspace.trial_heads[index];
            let previous_water = workspace.evaluated_theta[index];
            let next_head = workspace.rhs[index];
            let material = input.mesh.material_indices()[index];
            let next_evaluation = input.materials[material].evaluate(next_head)?;
            if !next_evaluation.theta.is_finite()
                || !next_evaluation.conductivity.is_finite()
                || !next_evaluation.capacity.is_finite()
            {
                return Err(WaterStepError::NonFiniteDiagnostic);
            }
            let next_water = next_evaluation.theta;
            let prescribed_head = (index == 0 && input.lower.fixes_head())
                || (index == workspace.node_count() - 1 && input.upper.fixes_head());
            if !prescribed_head {
                head_residual = head_residual.max((next_head - previous_head).abs());
            }
            water_residual = water_residual.max((next_water - previous_water).abs());
            workspace.trial_heads[index] = next_head;
            workspace.trial_water[index] = next_water;
        }
        if !head_residual.is_finite() || !water_residual.is_finite() {
            return Err(WaterStepError::NonFiniteDiagnostic);
        }
        let mut active_set_stable = true;
        if let Some(rates) = atmospheric_rates {
            let decision = decide_transition(rates, upper_mode, workspace, state, input, timestep)?;
            soil_flux = decision.soil_flux;
            ponded_depth = decision.ponded_depth;
            atmospheric_exchange = decision.atmospheric_exchange;
            active_set_stable = decision.stable;
        }
        if active_set_stable
            && head_residual <= policy.head_tolerance
            && water_residual <= policy.water_tolerance
        {
            return Ok(Some(Trial {
                iterations: iteration,
                head_residual,
                water_residual,
                pivots,
                upper_mode,
                ponded_depth,
                soil_flux,
                atmospheric_exchange,
                lower_boundary_flux,
            }));
        }
    }
    Ok(None)
}

fn assemble(
    workspace: &mut WaterWorkspace,
    state: &WaterState,
    input: WaterStepInput<'_>,
    minimum_pivot: f64,
    timestep: f64,
    atmospheric: Option<AtmosphericAssembly>,
) -> Result<(UpperBoundaryMode, f64), WaterStepError> {
    let count = workspace.node_count();
    for node in 0..count {
        let material = input.mesh.material_indices()[node];
        let evaluation = input.materials[material].evaluate(workspace.trial_heads[node])?;
        if !evaluation.theta.is_finite()
            || !evaluation.conductivity.is_finite()
            || !evaluation.capacity.is_finite()
        {
            return Err(WaterStepError::NonFiniteDiagnostic);
        }
        workspace.evaluated_theta[node] = evaluation.theta;
        workspace.evaluated_conductivity[node] = evaluation.conductivity;
        workspace.evaluated_capacity[node] = evaluation.capacity;
    }
    if let Some(root_uptake) = input.root_uptake {
        root_uptake.fill_trial_sink(
            input.mesh,
            input.materials,
            &workspace.trial_heads,
            &workspace.trial_water,
            timestep,
            &mut workspace.trial_sink,
        )?;
        for node in 0..count {
            workspace.trial_sink[node] += input.sink[node];
        }
    } else {
        workspace.trial_sink.copy_from_slice(input.sink);
    }
    let prepared_lower = input.lower.prepare_step(LowerTrial {
        conductivity_a: workspace.evaluated_conductivity[0],
        conductivity_b: workspace.evaluated_conductivity[1],
        capacity: workspace.evaluated_capacity[0],
        head: workspace.trial_heads[0],
        theta: workspace.evaluated_theta[0],
        old_theta: state.water_contents[0],
        sink: workspace.trial_sink[0],
        spacing: input.mesh.coordinates()[1] - input.mesh.coordinates()[0],
        gravity: input.mesh.inclination_cosine(),
        timestep,
        averaging: input.face_averaging,
    })?;
    let lower = transform_boundary_row(
        count,
        BoundaryRowLocation::Lower,
        prepared_lower.row,
        prepared_lower.condition,
        minimum_pivot,
    )?;
    let (condition, row, upper_mode) = if let Some(assembly) = atmospheric {
        let plan = assembly.prepare_step(workspace, state, input, timestep)?;
        (plan.condition, plan.row, plan.mode)
    } else {
        let (condition, row) = input.upper.caller_row()?;
        (condition, row, state.atmospheric.upper_mode)
    };
    let upper = transform_boundary_row(
        count,
        BoundaryRowLocation::Upper,
        row,
        condition,
        minimum_pivot,
    )?;
    workspace.lower[0] = lower.lower;
    workspace.diagonal[0] = lower.diagonal;
    workspace.upper[0] = lower.upper;
    workspace.rhs[0] = lower.rhs;
    workspace.lower[count - 1] = upper.lower;
    workspace.diagonal[count - 1] = upper.diagonal;
    workspace.upper[count - 1] = upper.upper;
    workspace.rhs[count - 1] = upper.rhs;
    let gravity = input.mesh.inclination_cosine();
    for node in 1..count - 1 {
        let row = ResetRow::calculate(ResetRowInput {
            dx_a: input.mesh.coordinates()[node] - input.mesh.coordinates()[node - 1],
            dx_b: input.mesh.coordinates()[node + 1] - input.mesh.coordinates()[node],
            gravity,
            conductivity_a: workspace.evaluated_conductivity[node - 1],
            conductivity_node: workspace.evaluated_conductivity[node],
            conductivity_b: workspace.evaluated_conductivity[node + 1],
            capacity: workspace.evaluated_capacity[node],
            h_new: workspace.trial_heads[node],
            theta_new: workspace.evaluated_theta[node],
            theta_old: state.water_contents[node],
            sink: workspace.trial_sink[node],
            timestep,
            face_averaging: input.face_averaging,
        })?;
        workspace.lower[node] = row.lower;
        workspace.diagonal[node] = row.diagonal;
        workspace.upper[node] = row.upper;
        workspace.rhs[node] = row.rhs;
    }
    validate_finite("assembled coefficients", &workspace.lower)?;
    validate_finite("assembled coefficients", &workspace.diagonal)?;
    validate_finite("assembled coefficients", &workspace.upper)?;
    validate_finite("assembled rhs", &workspace.rhs)?;
    Ok((upper_mode, prepared_lower.exchange_flux))
}

/// Fills `workspace.face_fluxes` for the accepted trial, positive toward the
/// upper boundary (ascending coordinates).
///
/// Balance terms are positive when water enters the soil. Entry at the lower
/// boundary points toward increasing coordinate, entry at the upper boundary
/// points against it, so the upper endpoint is the negated balance flux (exact).
/// Interior faces use the assembled rows' own terms: `evaluated_conductivity`
/// (conductivity at the last Picard iterate fed to the final `assemble`) with the
/// solved `trial_heads`, as `q = -K_face * (dh/dx + gravity)`.
fn fill_face_fluxes(
    workspace: &mut WaterWorkspace,
    input: WaterStepInput<'_>,
    lower_boundary_flux: f64,
    soil_upper_flux: f64,
) -> Result<(), WaterStepError> {
    let count = workspace.node_count();
    let gravity = input.mesh.inclination_cosine();
    let coordinates = input.mesh.coordinates();
    workspace.face_fluxes[0] = lower_boundary_flux;
    for face in 1..count {
        let left = workspace.evaluated_conductivity[face - 1];
        let right = workspace.evaluated_conductivity[face];
        let conductivity = match input.face_averaging {
            FaceAveraging::Arithmetic => arithmetic_face_conductivity(left, right)?,
            FaceAveraging::Geometric => geometric_face_conductivity(left, right)?,
        };
        let dx = coordinates[face] - coordinates[face - 1];
        workspace.face_fluxes[face] = -conductivity
            * ((workspace.trial_heads[face] - workspace.trial_heads[face - 1]) / dx + gravity);
    }
    workspace.face_fluxes[count] = -soil_upper_flux;
    validate_finite("face fluxes", &workspace.face_fluxes)
}

struct BalanceInput {
    timestep: f64,
    atmospheric_flux: Option<f64>,
    soil_atmospheric_flux: Option<f64>,
    lower_boundary_flux: f64,
    surface_storage_change: f64,
}

fn balance(
    state: &WaterState,
    workspace: &WaterWorkspace,
    input: WaterStepInput<'_>,
    exchange: BalanceInput,
) -> Result<WaterBalance, WaterStepError> {
    let BalanceInput {
        timestep,
        atmospheric_flux,
        soil_atmospheric_flux,
        lower_boundary_flux,
        surface_storage_change,
    } = exchange;
    let mut soil_storage_change = 0.0;
    let mut sink_integral = 0.0;
    for node in 0..input.mesh.coordinates().len() {
        let width = nodal_control_volume_width(input.mesh, node)?;
        soil_storage_change += (workspace.trial_water[node] - state.water_contents[node]) * width;
        sink_integral += workspace.trial_sink[node] * width;
    }
    let upper_boundary_flux = input.upper.measured_flux().unwrap_or(0.0);
    let upper_flux = atmospheric_flux.unwrap_or(upper_boundary_flux);
    let boundary_flux = (lower_boundary_flux + upper_flux) * timestep;
    let soil_boundary_flux =
        (lower_boundary_flux + soil_atmospheric_flux.unwrap_or(upper_boundary_flux)) * timestep;
    let atmospheric_exchange = upper_flux * timestep;
    let sink = sink_integral * timestep;
    let storage_change = soil_storage_change + surface_storage_change;
    let absolute_residual = (storage_change - boundary_flux + sink).abs();
    let denominator = storage_change.abs() + boundary_flux.abs() + sink.abs();
    let normalized_residual = if denominator == 0.0 {
        absolute_residual
    } else {
        absolute_residual / denominator
    };
    if ![
        storage_change,
        soil_storage_change,
        surface_storage_change,
        soil_boundary_flux,
        atmospheric_exchange,
        boundary_flux,
        sink,
        absolute_residual,
        normalized_residual,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err(WaterStepError::NonFiniteDiagnostic);
    }
    Ok(WaterBalance {
        storage_change,
        soil_storage_change,
        surface_storage_change,
        soil_boundary_flux,
        atmospheric_exchange,
        boundary_flux,
        sink,
        absolute_residual,
        normalized_residual,
    })
}

fn validate(
    state: &WaterState,
    workspace: &WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
) -> Result<(), WaterStepError> {
    let count = state.heads.len();
    if input.mesh.coordinates().len() != count || workspace.node_count() != count {
        return Err(WaterStepError::DimensionMismatch);
    }
    if input.mesh.material_indices().len() != count
        || input.materials.len() != input.mesh.material_count()
    {
        return Err(WaterStepError::DimensionMismatch);
    }
    validate_finite("sink", input.sink)?;
    validate_finite("target_time", &[input.target_time])?;
    if input.target_time <= state.time {
        return Err(WaterStepError::InvalidTargetTime {
            current_time: state.time,
            target_time: input.target_time,
        });
    }
    if input.sink.len() != count {
        return Err(WaterStepError::DimensionMismatch);
    }
    if let Some(root_uptake) = input.root_uptake {
        root_uptake.validate(count)?;
    }
    match (input.lower.measured_flux(), input.upper.measured_flux()) {
        (Some(lower_flux), Some(upper_flux)) => {
            validate_finite("boundary_fluxes", &[lower_flux, upper_flux])?
        }
        (Some(lower_flux), None) => validate_finite("boundary_fluxes", &[lower_flux])?,
        (None, Some(upper_flux)) => validate_finite("boundary_fluxes", &[upper_flux])?,
        (None, None) => {}
    }
    for (name, value) in [
        ("timestep", input.timestep),
        ("head_tolerance", policy.head_tolerance),
        ("water_tolerance", policy.water_tolerance),
        ("minimum_timestep", policy.minimum_timestep),
        ("retry_factor", policy.retry_factor),
        ("minimum_pivot", policy.minimum_pivot),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(WaterStepError::InvalidPolicy { name });
        }
    }
    if policy.max_iterations == 0
        || policy.retry_factor <= 1.0
        || input.timestep < policy.minimum_timestep
    {
        return Err(WaterStepError::InvalidPolicy {
            name: "iteration/retry/timestep",
        });
    }
    input.lower.prepare(count, policy.minimum_pivot)?;
    // Atmospheric rows are derived per trial, so only caller rows are checked.
    if let Ok((condition, row)) = input.upper.caller_row() {
        transform_boundary_row(
            count,
            BoundaryRowLocation::Upper,
            row,
            condition,
            policy.minimum_pivot,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod vapor_trial_tests {
    use super::*;
    use crate::heat::config::HeatConfigInput;
    use crate::heat::vapor::{
        VaporConfig, VaporConfigInput, VaporHistoryView, VaporSourceMaterial,
    };
    use crate::hydraulics::{VanGenuchtenModel, VanGenuchtenParameters};

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
            top_temperature: 20.0,
            bottom_temperature: 20.0,
            x_conv: 1.0,
            t_conv: 1.0,
        })
        .unwrap()
    }

    #[test]
    fn vapor_trial_returns_owned_candidate_without_mutating_accepted_inputs() {
        let model = VanGenuchtenModel::new(
            VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0e-5).unwrap(),
        )
        .unwrap();
        let mesh = Mesh1D::new(vec![0.0, 0.5, 1.0], vec![0; 3], None, 1, 1, 1.0).unwrap();
        let old_heads = vec![-100.0; 3];
        let old_theta: Vec<f64> = old_heads
            .iter()
            .map(|h| model.evaluate(*h).unwrap().theta)
            .collect();
        let state = WaterState::new(0.0, old_heads.clone(), old_theta.clone()).unwrap();
        let workspace = WaterWorkspace::new(3).unwrap();
        let temperature = [20.0; 3];
        let zero = [0.0; 3];
        let vapor = kernels::evaluate(&VaporPrimitiveInput {
            heads: &old_heads,
            temperatures: &temperature,
            liquid_water_contents: &old_theta,
            liquid_conductivities: &[model.evaluate(-100.0).unwrap().conductivity; 3],
            liquid_capacities: &[model.evaluate(-100.0).unwrap().capacity; 3],
            material_indices: &[0; 3],
            saturated_water_contents: &[0.45],
            x_conv: 1.0,
            t_conv: 1.0,
        })
        .unwrap();
        let old_view = VaporHistoryView {
            time: 0.0,
            previous_timestep: None,
            heads: &old_heads,
            liquid_water_contents: &old_theta,
            temperatures: &temperature,
            vapor_contents: &vapor.vapor_contents,
            liquid_velocities: &zero,
            vapor_velocities: &zero,
            head_predictor: &old_heads,
        };
        let source = [VaporSourceMaterial {
            model: &model,
            table: None,
        }];
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
                row: BoundaryRow {
                    lower: 0.0,
                    diagonal: 1.0,
                    upper: 0.0,
                    rhs: -90.0,
                },
                flux: 0.0,
            },
            upper: UpperBoundary::Head {
                head: -110.0,
                row: BoundaryRow {
                    lower: 0.0,
                    diagonal: 1.0,
                    upper: 0.0,
                    rhs: -110.0,
                },
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
        let temp_estimate = [20.0; 3];
        // The Newton/Picard guess is deliberately distinct from the accepted
        // head and material-history head; old storage must remain the snapshot.
        let initial_heads = [-105.0, -105.0, -105.0];
        let material_heads = [-100.0, -100.0, -100.0];
        let config = config();
        let context = VaporWaterContext {
            config: &config,
            old: old_view,
            temperature_estimate: &temp_estimate,
            initial_heads: &initial_heads,
            initial_material_heads: &material_heads,
            source_materials: &source,
        };
        let trial = solve_vapor_water_trial(&state, &workspace, input, policy, context).unwrap();
        let mut liquid_only_state = state.clone();
        let mut liquid_only_workspace = workspace.clone();
        advance_water_step(
            &mut liquid_only_state,
            &mut liquid_only_workspace,
            input,
            policy,
        )
        .unwrap();
        assert_eq!(trial.accepted_time, 0.1);
        assert_eq!(trial.timestep, 0.1);
        assert!(
            trial.iterations > 1,
            "properties/rows must be rebuilt per iteration"
        );
        assert!((trial.pre_solve_heads[1] - material_heads[1]).abs() < 1.0e-3);
        assert!(trial
            .properties
            .vapor_contents
            .iter()
            .any(|value| *value > 0.0));
        assert_ne!(trial.candidate.heads(), liquid_only_state.heads());
        let correction_trial = solve_vapor_water_trial(
            &state,
            &workspace,
            input,
            WaterStepPolicy {
                max_iterations: 2,
                head_tolerance: 100.0,
                water_tolerance: 100.0,
                ..policy
            },
            context,
        )
        .unwrap();
        // Source minimum of two solves: one allowed solve can never converge.
        let one_pass = solve_vapor_water_trial(
            &state,
            &workspace,
            input,
            WaterStepPolicy {
                max_iterations: 1,
                head_tolerance: 100.0,
                water_tolerance: 100.0,
                ..policy
            },
            context,
        );
        assert!(matches!(one_pass, Err(WaterStepError::NonConverged { .. })));
        assert_eq!(correction_trial.iterations, 2);
        // D1: pass 2 evaluates liquid at 0.1*hTemp + 0.9*hNew, where hTemp is the
        // pass-1 pre-solve iterate (`initial_heads`) and hNew is the pass-1 solve
        // output, which is the pass-2 pre-solve iterate.
        let iterate2 = correction_trial.pre_solve_heads.clone();
        let mixed2: Vec<f64> = initial_heads
            .iter()
            .zip(&iterate2)
            .map(|(h_temp, h_new)| 0.1 * h_temp + 0.9 * h_new)
            .collect();
        let mixed_evaluations: Vec<_> = mixed2
            .iter()
            .map(|head| model.evaluate(*head).unwrap())
            .collect();
        let expected_mixed = kernels::evaluate(&VaporPrimitiveInput {
            heads: &iterate2,
            temperatures: &temp_estimate,
            liquid_water_contents: &mixed_evaluations
                .iter()
                .map(|v| v.theta)
                .collect::<Vec<_>>(),
            liquid_conductivities: &mixed_evaluations
                .iter()
                .map(|v| v.conductivity)
                .collect::<Vec<_>>(),
            liquid_capacities: &mixed_evaluations
                .iter()
                .map(|v| v.capacity)
                .collect::<Vec<_>>(),
            material_indices: &[0; 3],
            saturated_water_contents: &[0.45],
            x_conv: 1.0,
            t_conv: 1.0,
        })
        .unwrap();
        for (actual, expected) in correction_trial
            .properties
            .vapor_contents
            .iter()
            .zip(expected_mixed.vapor_contents)
        {
            assert!((actual - expected).abs() < 1.0e-15);
        }
        // D4: ThNew = Th(mixed) + Cap*(hNew - pre-solve iterate), not material head.
        for node in 0..3 {
            let expected_theta = mixed_evaluations[node].theta
                + expected_mixed.effective_hydraulic_capacities[node]
                    * (correction_trial.candidate.heads()[node] - iterate2[node]);
            assert!(
                (correction_trial.candidate.water_contents()[node] - expected_theta).abs()
                    < 1.0e-15
            );
            assert!(
                (correction_trial.pre_solve_liquid_water_contents[node]
                    - mixed_evaluations[node].theta)
                    .abs()
                    < 1.0e-15
            );
        }
        let retry_initial_heads = [-90.0, -105.0, -110.0];
        let retry_material_heads = [-90.0, -100.0, -110.0];
        let retry_context = VaporWaterContext {
            initial_heads: &retry_initial_heads,
            initial_material_heads: &retry_material_heads,
            ..context
        };
        let retry_result = solve_vapor_water_trial(
            &state,
            &workspace,
            input,
            WaterStepPolicy {
                max_iterations: 1,
                head_tolerance: 0.01,
                water_tolerance: 0.01,
                minimum_timestep: 1.0e-14,
                ..policy
            },
            retry_context,
        );
        assert!(matches!(
            retry_result,
            Err(WaterStepError::NonConverged {
                timestep,
                retries,
                ..
            }) if timestep < input.timestep && retries > 0
        ));
        assert_ne!(
            correction_trial.candidate.water_contents(),
            correction_trial.pre_solve_liquid_water_contents,
            "candidate storage includes the final source head correction"
        );
        assert_eq!(state.heads(), &old_heads);
        assert_eq!(state.water_contents(), &old_theta);
        assert_ne!(trial.candidate.heads(), state.heads());
        assert_eq!(trial.properties.vapor_contents.len(), 3);
        assert_eq!(trial.total_face_fluxes.len(), 4);
        assert_eq!(trial.liquid_velocities.len(), 3);
        assert_eq!(trial.vapor_velocities.len(), 3);
    }

    /// 3-node fixed-head column at uniform 20 C, run with caller heads/tolerances.
    fn run_fixture(
        policy: WaterStepPolicy,
        initial: [f64; 3],
        material: [f64; 3],
        old_head: f64,
        bc: (f64, f64),
    ) -> Result<VaporWaterTrial, WaterStepError> {
        let model = VanGenuchtenModel::new(
            VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0e-5).unwrap(),
        )
        .unwrap();
        let mesh = Mesh1D::new(vec![0.0, 0.5, 1.0], vec![0; 3], None, 1, 1, 1.0).unwrap();
        let old_heads = vec![old_head; 3];
        let old_theta: Vec<f64> = old_heads
            .iter()
            .map(|h| model.evaluate(*h).unwrap().theta)
            .collect();
        let state = WaterState::new(0.0, old_heads.clone(), old_theta.clone()).unwrap();
        let workspace = WaterWorkspace::new(3).unwrap();
        let temperature = [20.0; 3];
        let zero = [0.0; 3];
        let e = model.evaluate(old_head).unwrap();
        let vapor = kernels::evaluate(&VaporPrimitiveInput {
            heads: &old_heads,
            temperatures: &temperature,
            liquid_water_contents: &old_theta,
            liquid_conductivities: &[e.conductivity; 3],
            liquid_capacities: &[e.capacity; 3],
            material_indices: &[0; 3],
            saturated_water_contents: &[0.45],
            x_conv: 1.0,
            t_conv: 1.0,
        })
        .unwrap();
        let old_view = VaporHistoryView {
            time: 0.0,
            previous_timestep: None,
            heads: &old_heads,
            liquid_water_contents: &old_theta,
            temperatures: &temperature,
            vapor_contents: &vapor.vapor_contents,
            liquid_velocities: &zero,
            vapor_velocities: &zero,
            head_predictor: &old_heads,
        };
        let source = [VaporSourceMaterial {
            model: &model,
            table: None,
        }];
        let materials: [&dyn HydraulicModel; 1] = [&model];
        let row = |h: f64| BoundaryRow {
            lower: 0.0,
            diagonal: 1.0,
            upper: 0.0,
            rhs: h,
        };
        let input = WaterStepInput {
            mesh: &mesh,
            materials: &materials,
            sink: &zero,
            root_uptake: None,
            timestep: 0.1,
            target_time: 0.1,
            face_averaging: FaceAveraging::Arithmetic,
            lower: LowerBoundaryMode::FixedHead {
                head: bc.0,
                row: row(bc.0),
                flux: 0.0,
            },
            upper: UpperBoundary::Head {
                head: bc.1,
                row: row(bc.1),
                flux: 0.0,
            },
        };
        let config = config();
        let context = VaporWaterContext {
            config: &config,
            old: old_view,
            temperature_estimate: &temperature,
            initial_heads: &initial,
            initial_material_heads: &material,
            source_materials: &source,
        };
        solve_vapor_water_trial(&state, &workspace, input, policy, context)
    }

    fn fixture_policy(max_iterations: usize, head_tol: f64, water_tol: f64) -> WaterStepPolicy {
        WaterStepPolicy {
            max_iterations,
            head_tolerance: head_tol,
            water_tolerance: water_tol,
            minimum_timestep: 1.0e-6,
            retry_factor: 3.0,
            minimum_pivot: 1.0e-14,
        }
    }

    #[test]
    fn d3_converged_first_solve_is_never_accepted() {
        // Tolerances of 100 are met by the first solve; source still iterates.
        let trial = run_fixture(
            fixture_policy(20, 100.0, 100.0),
            [-105.0; 3],
            [-100.0; 3],
            -100.0,
            (-90.0, -110.0),
        )
        .unwrap();
        assert_eq!(trial.iterations, 2);
    }

    #[test]
    fn d2_dry_unsaturated_node_converges_on_eps_th_not_head_change() {
        // Head change between passes is far above TolH=1e-12, but EpsTh
        // = |Cap*dh/(ths-thr)| (Cap ~ 1e-5 scale, span 0.4) stays below TolTh=1e-3.
        let trial = run_fixture(
            fixture_policy(20, 1.0e-12, 1.0e-3),
            [-105.0; 3],
            [-100.0; 3],
            -100.0,
            (-90.0, -110.0),
        )
        .unwrap();
        // A head-only rule at TolH=1e-12 could not accept within 20 passes.
        assert!(trial.iterations >= 2);
        let dh = (trial.candidate.heads()[1] - trial.pre_solve_heads[1]).abs();
        assert!(dh > 1.0e-12, "dh = {dh}");
        // Hand rule check for the accepted pass, independent of the solver.
        let cap = trial.properties.effective_hydraulic_capacities[1];
        assert!((cap * dh / 0.4).abs() <= 1.0e-3);
    }

    #[test]
    fn d2_saturated_node_uses_head_tolerance() {
        // All heads above hSat (~ -1e-8): TolH applies even with TolTh = 100.
        // Pass 1 moves the middle node by ~10 cm (> TolH=1e-3) so it cannot
        // be accepted there; a tight TolH must still bound the final change.
        let trial = run_fixture(
            fixture_policy(20, 1.0e-3, 100.0),
            [10.0, 0.0, 10.0],
            [10.0, 0.0, 10.0],
            10.0,
            (10.0, 10.0),
        )
        .unwrap();
        assert!(trial.iterations >= 2);
        let dh = (trial.candidate.heads()[1] - trial.pre_solve_heads[1]).abs();
        assert!(dh <= 1.0e-3, "dh = {dh}");
    }

    #[test]
    fn clamped_mixed_head_clamps_before_mixing() {
        let h_sat = -0.5;
        let mixed = clamped_mixed_head(-1.0e4, 1.9388e4, h_sat);
        assert_eq!(mixed, 0.1 * -1.0e4 + 0.9 * h_sat);
        assert!(mixed < h_sat);
        // Heads at or below hSat are unchanged.
        assert_eq!(
            clamped_mixed_head(-10.0, -20.0, h_sat),
            0.1 * -10.0 + 0.9 * -20.0
        );
    }
}

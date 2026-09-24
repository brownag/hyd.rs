use crate::atmospheric::{net_flux, AtmosphericTransition, UpperBoundaryMode};
use crate::forcing::AtmosphericRates;
use crate::hydraulics::{
    nodal_control_volume_width, solve_tridiagonal_in_place, transform_boundary_row,
    BoundaryCondition, BoundaryRow, BoundaryRowLocation, FaceAveraging, HydraulicModel, Mesh1D,
    PivotDiagnostics, ResetRow, ResetRowInput,
};
use crate::water_state::validate_finite;
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
    pub timestep: f64,
    /// Caller-selected time that this step must not pass.
    pub target_time: f64,
    pub face_averaging: FaceAveraging,
    pub lower_row: BoundaryRow,
    pub lower_condition: BoundaryCondition,
    pub upper_row: BoundaryRow,
    pub upper_condition: BoundaryCondition,
    /// Signed water entering the domain through the lower endpoint.
    pub lower_boundary_flux: f64,
    /// Signed water entering the domain through the upper endpoint.
    pub upper_boundary_flux: f64,
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
    /// Number of atmospheric flux/head mode changes during the accepted solve.
    pub atmospheric_mode_transitions: usize,
    /// Trial-state values that caused each active-set mode change.
    pub atmospheric_transitions: &'a [AtmosphericTransition],
    pub balance: WaterBalance,
}

pub fn advance_water_step<'a>(
    state: &mut WaterState,
    workspace: &'a mut WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
) -> Result<WaterStepReport<'a>, WaterStepError> {
    advance_water_step_impl(state, workspace, input, policy, None)
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
                    timestep,
                    upper_flux,
                    soil_upper_flux,
                    next_ponded - state.atmospheric.ponded_depth,
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
                    atmospheric_mode_transitions: workspace.atmospheric_transition_count,
                    atmospheric_transitions: &workspace.atmospheric_transitions
                        [..workspace.atmospheric_transition_count],
                    balance,
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
}

#[derive(Clone, Copy)]
struct AtmosphericAssembly {
    rates: AtmosphericRates,
    upper_mode: UpperBoundaryMode,
    ponded_depth: f64,
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
        upper_mode = assemble(
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
            let prescribed_head = (index == 0
                && matches!(input.lower_condition, BoundaryCondition::Dirichlet { .. }))
                || (index == workspace.node_count() - 1
                    && matches!(input.upper_condition, BoundaryCondition::Dirichlet { .. }));
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
            let demand = net_flux(rates);
            soil_flux = if upper_mode == UpperBoundaryMode::PondedHead {
                atmospheric_top_flux_for_heads(workspace, state, input, timestep)?
            } else {
                demand
            };
            ponded_depth =
                (state.atmospheric.ponded_depth + (demand - soil_flux) * timestep).max(0.0);
            atmospheric_exchange =
                soil_flux + (ponded_depth - state.atmospheric.ponded_depth) / timestep;
            let critical_head = match input.upper_condition {
                BoundaryCondition::Dirichlet { value } => value,
                _ => return Err(WaterStepError::UnsupportedAtmosphericInput {
                    name:
                        "upper_condition must declare the atmospheric critical head with Dirichlet",
                }),
            };
            let next_mode = atmospheric_mode(
                upper_mode,
                demand,
                workspace.trial_heads[workspace.node_count() - 1],
                critical_head,
                soil_flux,
                ponded_depth,
            );
            active_set_stable = next_mode == upper_mode;
            if !active_set_stable {
                let transition_soil_flux =
                    atmospheric_top_flux_for_heads(workspace, state, input, timestep)?;
                let transition_candidate_storage = (state.atmospheric.ponded_depth
                    + (demand - transition_soil_flux) * timestep)
                    .max(0.0);
                workspace.atmospheric_transitions[workspace.atmospheric_transition_count] =
                    AtmosphericTransition {
                        from_mode: upper_mode,
                        to_mode: next_mode,
                        demand,
                        top_head: workspace.trial_heads[workspace.node_count() - 1],
                        soil_flux: transition_soil_flux,
                        candidate_surface_storage: transition_candidate_storage,
                    };
                workspace.atmospheric_transition_count += 1;
            }
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
) -> Result<UpperBoundaryMode, WaterStepError> {
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
    let lower = transform_boundary_row(
        count,
        BoundaryRowLocation::Lower,
        input.lower_row,
        input.lower_condition,
        minimum_pivot,
    )?;
    let (upper_condition, upper_row, upper_mode) = if let Some(assembly) = atmospheric {
        let rates = assembly.rates;
        let current_upper_mode = assembly.upper_mode;
        let head = workspace.trial_heads[count - 1];
        let demand = net_flux(rates);
        let flux = atmospheric_top_flux(workspace, state, input, timestep);
        let critical_head =
            match input.upper_condition {
                BoundaryCondition::Dirichlet { value } => value,
                _ => return Err(WaterStepError::UnsupportedAtmosphericInput {
                    name:
                        "upper_condition must declare the atmospheric critical head with Dirichlet",
                }),
            };
        let mode = atmospheric_mode(
            current_upper_mode,
            demand,
            head,
            critical_head,
            flux,
            assembly.ponded_depth,
        );
        let row = atmospheric_top_row(workspace, state, input, timestep)?;
        let condition = if mode == UpperBoundaryMode::PondedHead {
            BoundaryCondition::Dirichlet {
                value: critical_head,
            }
        } else {
            BoundaryCondition::Neumann { flux: demand }
        };
        (condition, row, mode)
    } else {
        (
            input.upper_condition,
            input.upper_row,
            state.atmospheric.upper_mode,
        )
    };
    let upper = transform_boundary_row(
        count,
        BoundaryRowLocation::Upper,
        upper_row,
        upper_condition,
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
            sink: input.sink[node],
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
    Ok(upper_mode)
}

fn atmospheric_top_flux(
    workspace: &WaterWorkspace,
    state: &WaterState,
    input: WaterStepInput<'_>,
    timestep: f64,
) -> f64 {
    let top = workspace.node_count() - 1;
    let previous = top - 1;
    let dx = input.mesh.coordinates()[top] - input.mesh.coordinates()[previous];
    let conductivity =
        0.5 * (workspace.evaluated_conductivity[top] + workspace.evaluated_conductivity[previous]);
    conductivity
        * ((workspace.trial_heads[top] - workspace.trial_heads[previous]) / dx
            + input.mesh.inclination_cosine())
        + (workspace.evaluated_theta[top] - state.water_contents[top]) * dx / (2.0 * timestep)
        + input.sink[top] * dx / 2.0
}

fn atmospheric_top_flux_for_heads(
    workspace: &WaterWorkspace,
    state: &WaterState,
    input: WaterStepInput<'_>,
    timestep: f64,
) -> Result<f64, WaterStepError> {
    let top = workspace.node_count() - 1;
    let previous = top - 1;
    let top_evaluation =
        input.materials[input.mesh.material_indices()[top]].evaluate(workspace.trial_heads[top])?;
    let previous_evaluation = input.materials[input.mesh.material_indices()[previous]]
        .evaluate(workspace.trial_heads[previous])?;
    if !top_evaluation.theta.is_finite()
        || !top_evaluation.conductivity.is_finite()
        || !previous_evaluation.conductivity.is_finite()
    {
        return Err(WaterStepError::NonFiniteDiagnostic);
    }
    let dx = input.mesh.coordinates()[top] - input.mesh.coordinates()[previous];
    let conductivity = 0.5 * (top_evaluation.conductivity + previous_evaluation.conductivity);
    Ok(conductivity
        * ((workspace.trial_heads[top] - workspace.trial_heads[previous]) / dx
            + input.mesh.inclination_cosine())
        + (top_evaluation.theta - state.water_contents[top]) * dx / (2.0 * timestep)
        + input.sink[top] * dx / 2.0)
}

fn atmospheric_mode(
    current: UpperBoundaryMode,
    demand: f64,
    top_head: f64,
    critical_head: f64,
    soil_flux: f64,
    candidate_ponded_depth: f64,
) -> UpperBoundaryMode {
    if (demand >= 0.0 && top_head > critical_head) || (demand < 0.0 && top_head < critical_head) {
        return UpperBoundaryMode::PondedHead;
    }
    if current == UpperBoundaryMode::PondedHead {
        let supply_satisfies_demand = if demand >= 0.0 {
            soil_flux >= demand
        } else {
            soil_flux <= demand
        };
        if candidate_ponded_depth > 0.0 || !supply_satisfies_demand {
            return UpperBoundaryMode::PondedHead;
        }
    }
    UpperBoundaryMode::AtmosphericFlux
}

fn atmospheric_top_row(
    workspace: &WaterWorkspace,
    state: &WaterState,
    input: WaterStepInput<'_>,
    timestep: f64,
) -> Result<BoundaryRow, WaterStepError> {
    let top = workspace.node_count() - 1;
    let previous = top - 1;
    let dx_a = input.mesh.coordinates()[top] - input.mesh.coordinates()[previous];
    let dx = dx_a / 2.0;
    let conductivity =
        0.5 * (workspace.evaluated_conductivity[top] + workspace.evaluated_conductivity[previous]);
    let capacity = workspace.evaluated_capacity[top];
    let head = workspace.trial_heads[top];
    let theta_change = workspace.evaluated_theta[top] - state.water_contents[top];
    let row = BoundaryRow {
        lower: -conductivity / dx_a,
        diagonal: conductivity / dx_a + capacity * dx / timestep,
        upper: 0.0,
        rhs: capacity * dx / timestep * head
            - theta_change * dx / timestep
            - conductivity * input.mesh.inclination_cosine()
            - input.sink[top] * dx,
    };
    validate_finite(
        "atmospheric top row",
        &[row.lower, row.diagonal, row.upper, row.rhs],
    )?;
    Ok(row)
}

fn balance(
    state: &WaterState,
    workspace: &WaterWorkspace,
    input: WaterStepInput<'_>,
    timestep: f64,
    atmospheric_flux: Option<f64>,
    soil_atmospheric_flux: Option<f64>,
    surface_storage_change: f64,
) -> Result<WaterBalance, WaterStepError> {
    let mut soil_storage_change = 0.0;
    let mut sink_integral = 0.0;
    for node in 0..input.mesh.coordinates().len() {
        let width = nodal_control_volume_width(input.mesh, node)?;
        soil_storage_change += (workspace.trial_water[node] - state.water_contents[node]) * width;
        sink_integral += input.sink[node] * width;
    }
    let upper_flux = atmospheric_flux.unwrap_or(input.upper_boundary_flux);
    let boundary_flux = (input.lower_boundary_flux + upper_flux) * timestep;
    let soil_boundary_flux = (input.lower_boundary_flux
        + soil_atmospheric_flux.unwrap_or(input.upper_boundary_flux))
        * timestep;
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
    validate_finite(
        "boundary_fluxes",
        &[input.lower_boundary_flux, input.upper_boundary_flux],
    )?;
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
    transform_boundary_row(
        count,
        BoundaryRowLocation::Lower,
        input.lower_row,
        input.lower_condition,
        policy.minimum_pivot,
    )?;
    transform_boundary_row(
        count,
        BoundaryRowLocation::Upper,
        input.upper_row,
        input.upper_condition,
        policy.minimum_pivot,
    )?;
    Ok(())
}

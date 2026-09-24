use crate::hydraulics::{
    nodal_control_volume_width, solve_tridiagonal_in_place, transform_boundary_row,
    BoundaryCondition, BoundaryRow, BoundaryRowLocation, FaceAveraging, HydraulicModel, Mesh1D,
    PivotDiagnostics, ResetRow, ResetRowInput,
};
use crate::water_state::validate_finite;
use crate::{WaterState, WaterStepError, WaterWorkspace};

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
    /// Mesh-integrated change in water storage per unit cross-sectional area.
    pub storage_change: f64,
    /// Signed endpoint flux integrated over the accepted timestep.
    pub boundary_flux: f64,
    /// Mesh-integrated sink over the accepted timestep per unit cross-sectional area.
    pub sink: f64,
    pub absolute_residual: f64,
    /// `absolute_residual` when every balance term is zero; otherwise residual divided by term magnitude.
    pub normalized_residual: f64,
}

/// Accepted-step report; all values are finite on success.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterStepReport {
    pub accepted_time: f64,
    pub timestep: f64,
    pub iterations: usize,
    pub retries: usize,
    pub max_head_residual: f64,
    pub max_water_residual: f64,
    pub minimum_absolute_pivot: f64,
    pub balance: WaterBalance,
}

pub fn advance_water_step(
    state: &mut WaterState,
    workspace: &mut WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
) -> Result<WaterStepReport, WaterStepError> {
    validate(state, workspace, input, policy)?;
    let mut timestep = input.timestep;
    let mut retries = 0;
    loop {
        workspace.snapshot(state);
        match solve_trial(state, workspace, input, policy, timestep)? {
            Some(trial) => {
                let balance = balance(state, workspace, input, timestep)?;
                state.heads.copy_from_slice(&workspace.trial_heads);
                state.water_contents.copy_from_slice(&workspace.trial_water);
                state.time += timestep;
                state.accounting.accepted_steps += 1;
                state.accounting.cumulative_iterations += trial.iterations as u64;
                state.accounting.cumulative_storage_change += balance.storage_change;
                state.accounting.cumulative_boundary_flux += balance.boundary_flux;
                state.accounting.cumulative_sink += balance.sink;
                return Ok(WaterStepReport {
                    accepted_time: state.time,
                    timestep,
                    iterations: trial.iterations,
                    retries,
                    max_head_residual: trial.head_residual,
                    max_water_residual: trial.water_residual,
                    minimum_absolute_pivot: trial.pivots.minimum_absolute_pivot,
                    balance,
                });
            }
            None => {
                workspace.restore(state);
                let next = timestep / policy.retry_factor;
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
}

fn solve_trial(
    state: &WaterState,
    workspace: &mut WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
    timestep: f64,
) -> Result<Option<Trial>, WaterStepError> {
    for iteration in 1..=policy.max_iterations {
        assemble(workspace, state, input, policy.minimum_pivot, timestep)?;
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
        if head_residual <= policy.head_tolerance && water_residual <= policy.water_tolerance {
            return Ok(Some(Trial {
                iterations: iteration,
                head_residual,
                water_residual,
                pivots,
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
) -> Result<(), WaterStepError> {
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
    let upper = transform_boundary_row(
        count,
        BoundaryRowLocation::Upper,
        input.upper_row,
        input.upper_condition,
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
    validate_finite("assembled rhs", &workspace.rhs)
}

fn balance(
    state: &WaterState,
    workspace: &WaterWorkspace,
    input: WaterStepInput<'_>,
    timestep: f64,
) -> Result<WaterBalance, WaterStepError> {
    let mut storage_change = 0.0;
    let mut sink_integral = 0.0;
    for node in 0..input.mesh.coordinates().len() {
        let width = nodal_control_volume_width(input.mesh, node)?;
        storage_change += (workspace.trial_water[node] - state.water_contents[node]) * width;
        sink_integral += input.sink[node] * width;
    }
    let boundary_flux = (input.lower_boundary_flux + input.upper_boundary_flux) * timestep;
    let sink = sink_integral * timestep;
    let absolute_residual = (storage_change - boundary_flux + sink).abs();
    let denominator = storage_change.abs() + boundary_flux.abs() + sink.abs();
    let normalized_residual = if denominator == 0.0 {
        absolute_residual
    } else {
        absolute_residual / denominator
    };
    if ![
        storage_change,
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

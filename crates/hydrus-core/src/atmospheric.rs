//! Minimal in-memory atmospheric boundary scheduling.
use crate::forcing::{AtmosphericRates, ForcingSchedule};
use crate::hydraulics::{BoundaryCondition, BoundaryRow};
use crate::water_state::validate_finite;
use crate::{
    WaterState, WaterStepError, WaterStepInput, WaterStepPolicy, WaterStepReport, WaterWorkspace,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpperBoundaryMode {
    /// Apply the signed atmospheric demand directly as the upper soil flux.
    AtmosphericFlux,
    /// Constrain top head to the caller-declared critical head while supply or storage limits apply.
    PondedHead,
}

/// Trial state recorded when the atmospheric active set selects another mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphericTransition {
    pub from_mode: UpperBoundaryMode,
    pub to_mode: UpperBoundaryMode,
    pub demand: f64,
    pub top_head: f64,
    pub soil_flux: f64,
    pub candidate_surface_storage: f64,
}

/// Accepted atmospheric history attached to a water simulation.
///
/// Candidate mode and surface storage remain local to one nonlinear trial and
/// are copied here only when that trial is accepted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphericState {
    pub event_cursor: usize,
    pub ponded_depth: f64,
    pub upper_mode: UpperBoundaryMode,
}

impl Default for AtmosphericState {
    fn default() -> Self {
        Self {
            event_cursor: 0,
            ponded_depth: 0.0,
            upper_mode: UpperBoundaryMode::AtmosphericFlux,
        }
    }
}

/// Advances to the earlier of the requested target and next forcing event.
/// The schedule uses caller-declared time and depth-rate units without conversion.
/// `input.upper` must be `UpperBoundary::Atmospheric`; its `critical_head` is the
/// caller-declared critical head used by the upper flux/head switch. Its
/// physical interpretation for evaporation remains unresolved by available
/// source evidence.
pub fn advance_atmospheric_step<'a>(
    state: &mut WaterState,
    workspace: &'a mut WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
    schedule: &ForcingSchedule<'_>,
) -> Result<WaterStepReport<'a>, WaterStepError> {
    if state.atmospheric.event_cursor > schedule.events().len() {
        return Err(WaterStepError::InvalidForcing {
            name: "event_cursor",
            reason: "cursor exceeds schedule length",
        });
    }
    if schedule.events()[state.atmospheric.event_cursor..]
        .iter()
        .any(|event| event.time <= state.time())
    {
        return Err(WaterStepError::InvalidForcing {
            name: "event_times",
            reason: "events must be later than the current accepted time",
        });
    }
    let mut next_input = input;
    if let Some(event) = schedule.events().get(state.atmospheric.event_cursor) {
        next_input.target_time = next_input.target_time.min(event.time);
    }
    let rates = if state.atmospheric.event_cursor == 0 {
        schedule.initial
    } else {
        schedule.events()[state.atmospheric.event_cursor - 1].rates
    };
    let mut report = crate::water_solver::advance_atmospheric_inner(
        state, workspace, next_input, policy, rates,
    )?;
    while state.atmospheric.event_cursor < schedule.events().len()
        && state.time() >= schedule.events()[state.atmospheric.event_cursor].time
    {
        state.atmospheric.event_cursor += 1;
    }
    report.accepted_time = state.time();
    Ok(report)
}

pub(crate) fn net_flux(rates: AtmosphericRates) -> f64 {
    rates.precipitation - rates.evaporation
}

/// Per-iteration upper-boundary inputs for an atmospheric trial.
#[derive(Clone, Copy)]
pub(crate) struct AtmosphericAssembly {
    pub(crate) rates: AtmosphericRates,
    pub(crate) upper_mode: UpperBoundaryMode,
    pub(crate) ponded_depth: f64,
}

/// Upper boundary condition, row and mode chosen for one assembly.
pub(crate) struct AtmosphericUpperPlan {
    pub(crate) condition: BoundaryCondition,
    pub(crate) row: BoundaryRow,
    pub(crate) mode: UpperBoundaryMode,
}

/// Soil flux, surface storage and active-set decision after a head update.
pub(crate) struct AtmosphericTransitionDecision {
    pub(crate) soil_flux: f64,
    pub(crate) ponded_depth: f64,
    pub(crate) atmospheric_exchange: f64,
    pub(crate) stable: bool,
}

impl AtmosphericAssembly {
    /// Chooses the upper mode, row and condition for the current trial heads.
    pub(crate) fn prepare_step(
        self,
        workspace: &WaterWorkspace,
        state: &WaterState,
        input: WaterStepInput<'_>,
        timestep: f64,
    ) -> Result<AtmosphericUpperPlan, WaterStepError> {
        let rates = self.rates;
        let current_upper_mode = self.upper_mode;
        let count = workspace.node_count();
        let head = workspace.trial_heads[count - 1];
        let demand = net_flux(rates);
        let flux = top_flux(workspace, state, input, timestep);
        let critical_head = input.upper.critical_head()?;
        let mode = select_mode(
            current_upper_mode,
            demand,
            head,
            critical_head,
            flux,
            self.ponded_depth,
        );
        let row = top_row(workspace, state, input, timestep)?;
        let condition = if mode == UpperBoundaryMode::PondedHead {
            BoundaryCondition::Dirichlet {
                value: critical_head,
            }
        } else {
            BoundaryCondition::Neumann { flux: demand }
        };
        Ok(AtmosphericUpperPlan {
            condition,
            row,
            mode,
        })
    }
}

/// Computes the soil flux and surface storage for the updated trial heads and
/// records a transition when the active set changes.
pub(crate) fn decide_transition(
    rates: AtmosphericRates,
    upper_mode: UpperBoundaryMode,
    workspace: &mut WaterWorkspace,
    state: &WaterState,
    input: WaterStepInput<'_>,
    timestep: f64,
) -> Result<AtmosphericTransitionDecision, WaterStepError> {
    let demand = net_flux(rates);
    let soil_flux = if upper_mode == UpperBoundaryMode::PondedHead {
        top_flux_for_heads(workspace, state, input, timestep)?
    } else {
        demand
    };
    let ponded_depth = (state.atmospheric.ponded_depth + (demand - soil_flux) * timestep).max(0.0);
    let atmospheric_exchange =
        soil_flux + (ponded_depth - state.atmospheric.ponded_depth) / timestep;
    let critical_head = input.upper.critical_head()?;
    let next_mode = select_mode(
        upper_mode,
        demand,
        workspace.trial_heads[workspace.node_count() - 1],
        critical_head,
        soil_flux,
        ponded_depth,
    );
    let stable = next_mode == upper_mode;
    if !stable {
        let transition_soil_flux = top_flux_for_heads(workspace, state, input, timestep)?;
        let transition_candidate_storage =
            (state.atmospheric.ponded_depth + (demand - transition_soil_flux) * timestep).max(0.0);
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
    Ok(AtmosphericTransitionDecision {
        soil_flux,
        ponded_depth,
        atmospheric_exchange,
        stable,
    })
}

fn top_flux(
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
        + workspace.trial_sink[top] * dx / 2.0
}

fn top_flux_for_heads(
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
        + workspace.trial_sink[top] * dx / 2.0)
}

fn select_mode(
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

fn top_row(
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
            - workspace.trial_sink[top] * dx,
    };
    validate_finite(
        "atmospheric top row",
        &[row.lower, row.diagonal, row.upper, row.rhs],
    )?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydraulics::{
        Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, Mesh1D,
    };
    use crate::{HydraulicError, LowerBoundaryMode, UpperBoundary};

    struct LinearModel;

    impl HydraulicModel for LinearModel {
        fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
            Ok(Evaluation {
                theta: 0.2 + 0.1 * head,
                conductivity: 1.0,
                capacity: 0.1,
            })
        }
        fn derivatives(&self, _head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
            Ok(HydraulicDerivatives {
                dtheta_dhead: 0.1,
                dconductivity_dhead: 0.0,
                dcapacity_dhead: 0.0,
            })
        }
    }

    const ROW: BoundaryRow = BoundaryRow {
        lower: 0.0,
        diagonal: 1.0,
        upper: 0.0,
        rhs: 0.0,
    };

    fn input<'a>(
        mesh: &'a Mesh1D,
        materials: &'a [&'a dyn HydraulicModel],
        sink: &'a [f64],
        upper: UpperBoundary,
    ) -> WaterStepInput<'a> {
        WaterStepInput {
            mesh,
            materials,
            sink,
            root_uptake: None,
            timestep: 2.0,
            target_time: 2.0,
            face_averaging: FaceAveraging::Arithmetic,
            lower: LowerBoundaryMode::FixedHead {
                head: 0.0,
                row: ROW,
                flux: 0.0,
            },
            upper,
        }
    }

    fn mesh() -> Mesh1D {
        Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0, 0, 0], None, 1, 1, 1.0).unwrap()
    }

    fn rates(precipitation: f64) -> AtmosphericRates {
        AtmosphericRates {
            precipitation,
            evaporation: 0.0,
        }
    }

    #[test]
    fn mode_switches_between_flux_and_head() {
        use UpperBoundaryMode::{AtmosphericFlux as Flux, PondedHead as Head};
        // Infiltration demand with top head above critical forces head control.
        assert_eq!(select_mode(Flux, 1.0, 0.5, 0.0, 1.0, 0.0), Head);
        // Evaporation demand with top head below critical forces head control.
        assert_eq!(select_mode(Flux, -1.0, -0.5, 0.0, -1.0, 0.0), Head);
        // Flux mode stays flux while the head is within bounds.
        assert_eq!(select_mode(Flux, 1.0, -0.5, 0.0, 1.0, 0.0), Flux);
        // Head mode returns to flux once supply meets demand and nothing is ponded.
        assert_eq!(select_mode(Head, 1.0, -0.5, 0.0, 1.0, 0.0), Flux);
        // Surface storage or unmet supply keeps head control.
        assert_eq!(select_mode(Head, 1.0, -0.5, 0.0, 1.0, 0.1), Head);
        assert_eq!(select_mode(Head, 1.0, -0.5, 0.0, 0.5, 0.0), Head);
        assert_eq!(select_mode(Head, -1.0, 0.5, 0.0, -0.5, 0.0), Head);
    }

    #[test]
    fn ponding_onset_and_drain_through_decide_transition() {
        use UpperBoundaryMode::{AtmosphericFlux as Flux, PondedHead as Head};
        let mesh = mesh();
        let model = LinearModel;
        let materials: [&dyn HydraulicModel; 1] = [&model];
        let sink = [0.0; 3];
        let input = input(
            &mesh,
            &materials,
            &sink,
            UpperBoundary::Atmospheric { critical_head: 0.0 },
        );
        // Onset: top head above critical under infiltration leaves flux mode.
        let state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
        let mut workspace = WaterWorkspace::with_transition_capacity(3, 4).unwrap();
        workspace.trial_heads.copy_from_slice(&[0.0, 0.0, 5.0]);
        let decision =
            decide_transition(rates(1.0), Flux, &mut workspace, &state, input, 2.0).unwrap();
        assert!(!decision.stable);
        assert_eq!(decision.soil_flux, 1.0);
        assert_eq!(decision.ponded_depth, 0.0);
        assert_eq!(workspace.atmospheric_transition_count, 1);
        let recorded = workspace.atmospheric_transitions[0];
        assert_eq!((recorded.from_mode, recorded.to_mode), (Flux, Head));
        assert_eq!(recorded.top_head, 5.0);

        // Drain: ponded storage shrinks by the soil flux while head control holds.
        let mut state = WaterState::new(0.0, vec![0.0; 3], vec![0.0; 3]).unwrap();
        state.atmospheric.ponded_depth = 0.5;
        state.atmospheric.upper_mode = Head;
        let mut workspace = WaterWorkspace::with_transition_capacity(3, 4).unwrap();
        workspace.trial_heads.copy_from_slice(&[0.0, 0.0, -1.0]);
        let decision =
            decide_transition(rates(0.0), Head, &mut workspace, &state, input, 2.0).unwrap();
        assert!(decision.stable);
        assert!((decision.soil_flux - 0.025).abs() < 1e-15);
        assert!((decision.ponded_depth - 0.45).abs() < 1e-15);
        assert_eq!(workspace.atmospheric_transition_count, 0);

        // Fully drained storage clamps at zero and releases head control.
        state.atmospheric.ponded_depth = 0.01;
        let decision =
            decide_transition(rates(0.0), Head, &mut workspace, &state, input, 2.0).unwrap();
        assert!(!decision.stable);
        assert_eq!(decision.ponded_depth, 0.0);
        let recorded = workspace.atmospheric_transitions[0];
        assert_eq!((recorded.from_mode, recorded.to_mode), (Head, Flux));
    }

    #[test]
    fn prepare_step_requires_atmospheric_and_picks_condition() {
        use UpperBoundaryMode::{AtmosphericFlux as Flux, PondedHead as Head};
        let mesh = mesh();
        let model = LinearModel;
        let materials: [&dyn HydraulicModel; 1] = [&model];
        let sink = [0.0; 3];
        let state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
        let mut workspace = WaterWorkspace::new(3).unwrap();
        workspace.evaluated_conductivity.fill(1.0);
        workspace.evaluated_capacity.fill(0.1);
        workspace.evaluated_theta.fill(0.2);
        let assembly = |upper_mode| AtmosphericAssembly {
            rates: rates(0.5),
            upper_mode,
            ponded_depth: 0.0,
        };

        let neumann = input(
            &mesh,
            &materials,
            &sink,
            UpperBoundary::Flux {
                flux: 0.0,
                row: ROW,
            },
        );
        let result = assembly(Flux).prepare_step(&workspace, &state, neumann, 2.0);
        assert!(matches!(
            result,
            Err(WaterStepError::InvalidUpperBoundary { .. })
        ));

        let dirichlet = input(
            &mesh,
            &materials,
            &sink,
            UpperBoundary::Atmospheric {
                critical_head: -3.0,
            },
        );
        workspace.trial_heads.copy_from_slice(&[0.0, 0.0, -4.0]);
        let plan = assembly(Flux)
            .prepare_step(&workspace, &state, dirichlet, 2.0)
            .unwrap();
        assert_eq!(plan.mode, Flux);
        assert_eq!(plan.condition, BoundaryCondition::Neumann { flux: 0.5 });

        workspace.trial_heads.copy_from_slice(&[0.0, 0.0, -2.0]);
        let plan = assembly(Flux)
            .prepare_step(&workspace, &state, dirichlet, 2.0)
            .unwrap();
        assert_eq!(plan.mode, Head);
        assert_eq!(plan.condition, BoundaryCondition::Dirichlet { value: -3.0 });
    }
}

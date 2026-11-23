//! Minimal in-memory atmospheric boundary scheduling.
use crate::forcing::{AtmosphericRates, ForcingSchedule, VaporAtmosphericCoupling};
use crate::heat::vapor::{VaporWaterContext, VaporWaterTrial};
use crate::hydraulics::{BoundaryCondition, BoundaryRow};
use crate::water_solver::solve_vapor_water_trial;
use crate::water_state::validate_finite;
use crate::UpperBoundary;
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
    let (next_input, rates) = clip_to_next_event(
        state.atmospheric.event_cursor,
        state.time(),
        input,
        schedule,
    )?;
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

/// Validates the forcing cursor, clips the target to the next event and picks the active rates.
fn clip_to_next_event<'a>(
    cursor: usize,
    time: f64,
    input: WaterStepInput<'a>,
    schedule: &ForcingSchedule<'_>,
) -> Result<(WaterStepInput<'a>, AtmosphericRates), WaterStepError> {
    if cursor > schedule.events().len() {
        return Err(WaterStepError::InvalidForcing {
            name: "event_cursor",
            reason: "cursor exceeds schedule length",
        });
    }
    if schedule.events()[cursor..]
        .iter()
        .any(|event| event.time <= time)
    {
        return Err(WaterStepError::InvalidForcing {
            name: "event_times",
            reason: "events must be later than the current accepted time",
        });
    }
    let mut next_input = input;
    if let Some(event) = schedule.events().get(cursor) {
        next_input.target_time = next_input.target_time.min(event.time);
    }
    let rates = if cursor == 0 {
        schedule.initial
    } else {
        schedule.events()[cursor - 1].rates
    };
    Ok((next_input, rates))
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

// ---------------------------------------------------------------------------
// Vapor-aware atmospheric switching (D-10, ADR 0010)
// ---------------------------------------------------------------------------

/// Typed rejections of the vapor atmospheric step.
#[derive(Clone, Debug, PartialEq)]
pub enum VaporAtmosphericError {
    /// A surface water layer or `hCritS` was requested.
    SurfacePondingUnsupported,
    Water(WaterStepError),
}

impl std::fmt::Display for VaporAtmosphericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SurfacePondingUnsupported => f.write_str(
                "surface ponding (WLayer/hCritS) with vapor: out of scope for G22.3 first fixture",
            ),
            Self::Water(error) => write!(f, "{error:?}"),
        }
    }
}

impl std::error::Error for VaporAtmosphericError {}

impl From<WaterStepError> for VaporAtmosphericError {
    fn from(error: WaterStepError) -> Self {
        Self::Water(error)
    }
}

/// Top-boundary memory carried between vapor atmospheric steps (Fortran `KodTop`,
/// `hTop`, `rTopOld`, `vTop`). Signs follow the source: `r_top` and `v_top` are positive
/// out of the soil. Kept apart from the pub `AtmosphericState`, which the water-only
/// path owns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaporAtmosphericState {
    /// `-4` flux mode, `4` head mode.
    pub kod_top: i8,
    pub h_top: f64,
    pub r_top_old: f64,
    /// Current `rTop`, fixed over each forcing record; updated only at record boundaries.
    pub r_top: f64,
    /// Forcing record (event cursor) for which `r_top` was set; `None` before the first step.
    pub record_cursor: Option<usize>,
    /// Top-half-cell flux of the last accepted trial (the `vTop` of `Shift`).
    pub v_top: f64,
    pub event_cursor: usize,
}

impl VaporAtmosphericState {
    /// Source start state: flux mode (`INPUT.FOR:106-108`).
    pub fn new() -> Self {
        Self {
            kod_top: -4,
            h_top: 0.0,
            r_top_old: 0.0,
            r_top: 0.0,
            record_cursor: None,
            v_top: 0.0,
            event_cursor: 0,
        }
    }
}

impl Default for VaporAtmosphericState {
    fn default() -> Self {
        Self::new()
    }
}

/// Where a vapor-path switch was decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaporSwitchStage {
    /// `SetBC` rain reset (`TIME.FOR:133-141`).
    Reset,
    /// `Shift` on the start-of-step iterate (`WATFLOW.FOR:86`, iteration 1).
    PreSolve,
    /// `Shift` on the converged trial (stands in for iteration 2).
    PostSolve,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaporSwitchRecord {
    pub stage: VaporSwitchStage,
    pub from_kod: i8,
    pub to_kod: i8,
    pub time: f64,
    pub top_head: f64,
    pub v_top: f64,
    /// The `Shift` test on the re-solved trial would flip the mode again. Expected on dry
    /// head-mode steps, where Fortran also flips in iteration 1 and back in iteration 2;
    /// the second result is kept. Not an error.
    pub second_shift_would_flip: bool,
}

/// Accepted-candidate result; the caller commits the vapor and water state.
#[derive(Clone, Debug)]
pub struct VaporAtmosphericStep {
    pub trial: VaporWaterTrial,
    pub state: VaporAtmosphericState,
    pub switches: Vec<VaporSwitchRecord>,
    /// The kept trial is in flux mode with top head below `hCritA` (clamp not applied).
    pub below_critical_head: bool,
}

/// Port of `Shift` top block (`WATFLOW.FOR:392-426`) with `TopInF` true, `WLayer` false.
/// Returns the new `(kod_top, h_top)`. `r_top`/`v_top` are positive outward.
pub(crate) fn fortran_top_switch(
    kod: i8,
    h_top: f64,
    r_top: f64,
    v_top: f64,
    top_head: f64,
    h_crit_a: f64,
) -> (i8, f64) {
    if kod.abs() != 4 {
        return (kod, h_top);
    }
    let mut kod = kod;
    let mut h_top = h_top;
    if kod > 0 {
        if v_top.abs() > r_top.abs() || v_top * r_top <= 0.0 {
            kod = -4;
        }
        if kod == 4 && top_head <= 0.99 * h_crit_a && r_top < 0.0 {
            kod = -4;
        }
    } else {
        if top_head > 0.0 {
            kod = 4;
            h_top = 0.0;
        }
        if top_head <= h_crit_a {
            kod = 4;
            h_top = h_crit_a;
        }
    }
    (kod, h_top)
}

const ZERO_ROW: BoundaryRow = BoundaryRow {
    lower: 0.0,
    diagonal: 1.0,
    upper: 0.0,
    rhs: 0.0,
};

fn upper_for(kod: i8, h_top: f64, rates: AtmosphericRates) -> UpperBoundary {
    if kod > 0 {
        UpperBoundary::Head {
            head: h_top,
            row: ZERO_ROW,
            flux: 0.0,
        }
    } else {
        UpperBoundary::Flux {
            flux: net_flux(rates),
            row: ZERO_ROW,
        }
    }
}

/// `vTop` of `Shift` (`WATFLOW.FOR:395-407`) from a trial: liquid plus vapor flux across
/// the top cell face, top half-cell storage and sink; positive outward.
fn trial_v_top(state: &WaterState, input: WaterStepInput<'_>, trial: &VaporWaterTrial) -> f64 {
    let n = trial.candidate.heads().len();
    let coordinates = input.mesh.coordinates();
    let dx = coordinates[n - 1] - coordinates[n - 2];
    trial.liquid_face_fluxes[n - 1] + trial.vapor_face_fluxes[n - 1]
        - (trial.candidate.water_contents()[n - 1] - state.water_contents()[n - 1]) * dx
            / 2.0
            / trial.timestep
        - input.sink[n - 1] * dx / 2.0
}

/// Vapor-coupled atmospheric step decomposed into the existing Flux/Head pairs.
///
/// Order, per `WATFLOW.FOR`/`TIME.FOR`: `SetBC` rain reset, `Shift` on the start-of-step
/// iterate (`context.initial_heads`, the driver's extrapolated predictor,
/// `Update` `WATFLOW.FOR:1097-1098`), solve under the selected pair, `Shift` on the
/// converged trial; a changed mode re-solves the step once from the old state. Fortran
/// switches inside the Picard loop, so this step-level re-solve is the D-10
/// approximation (one iteration of history differs); 31.4.1 decides whether it holds.
/// `input.upper` must be `Atmospheric { critical_head }` (`hCritA`, stored negative).
/// Does not mutate accepted state.
#[allow(clippy::too_many_arguments)]
pub fn advance_atmospheric_step_vapor(
    state: &WaterState,
    workspace: &WaterWorkspace,
    input: WaterStepInput<'_>,
    policy: WaterStepPolicy,
    schedule: &ForcingSchedule<'_>,
    vapor_state: VaporAtmosphericState,
    coupling: VaporAtmosphericCoupling,
    context: VaporWaterContext<'_>,
) -> Result<VaporAtmosphericStep, VaporAtmosphericError> {
    if coupling.surface_layer || coupling.ponding_threshold.is_some() {
        return Err(VaporAtmosphericError::SurfacePondingUnsupported);
    }
    let h_crit_a = -input.upper.critical_head()?.abs();
    let (step_input, rates) =
        clip_to_next_event(vapor_state.event_cursor, state.time(), input, schedule)?;
    let n = state.heads().len();
    let time = state.time();
    // rTop is piecewise constant over a forcing record (HYDRUS.FOR:573-628): at a record
    // boundary `SetBC` first sets rTop = -|Prec| (TIME.FOR:131, lMeteo) and runs the reset
    // below; the met update and HYDRUS.FOR:627-628 then set rTop = |rSoil| - |Prec|, for
    // either KodTop. Shift flips never change it mid-record.
    let at_boundary = vapor_state.record_cursor != Some(vapor_state.event_cursor);
    let r_top_old = if at_boundary {
        vapor_state.r_top
    } else {
        vapor_state.r_top_old
    };
    let r_top = if at_boundary {
        rates.evaporation - rates.precipitation
    } else {
        vapor_state.r_top
    };
    let r_top_setbc = -rates.precipitation.abs();
    let mut switches = Vec::new();
    let mut initial_heads = context.initial_heads.to_vec();
    let mut kod = vapor_state.kod_top;
    let mut h_top = vapor_state.h_top;

    // SetBC rain reset (TIME.FOR:133-141, iModel != 3, WLayer false). It reads the interim
    // rTop = -|Prec|, so under lMeteo it cannot fire; ported as written.
    if at_boundary && r_top_setbc > 0.0 && r_top_old < 0.0 {
        let mut x_limit = 0.0;
        if kod == 4 || initial_heads[n - 1] > x_limit {
            x_limit = -0.01 * coupling.x_conv;
            if kod != -4 {
                switches.push(VaporSwitchRecord {
                    stage: VaporSwitchStage::Reset,
                    from_kod: kod,
                    to_kod: -4,
                    time,
                    top_head: initial_heads[n - 1],
                    v_top: vapor_state.v_top,
                    second_shift_would_flip: false,
                });
            }
            initial_heads[n - 1] = x_limit;
            kod = -4;
        }
    }
    // Shift on the start-of-step iterate.
    let (pre_kod, pre_h) = fortran_top_switch(
        kod,
        h_top,
        r_top,
        vapor_state.v_top,
        initial_heads[n - 1],
        h_crit_a,
    );
    if pre_kod != kod {
        switches.push(VaporSwitchRecord {
            stage: VaporSwitchStage::PreSolve,
            from_kod: kod,
            to_kod: pre_kod,
            time,
            top_head: initial_heads[n - 1],
            v_top: vapor_state.v_top,
            second_shift_would_flip: false,
        });
    }
    kod = pre_kod;
    h_top = pre_h;

    let solve = |kod: i8, h_top: f64| -> Result<VaporWaterTrial, VaporAtmosphericError> {
        let mut trial_input = step_input;
        trial_input.upper = upper_for(kod, h_top, rates);
        let trial_context = VaporWaterContext {
            initial_heads: &initial_heads,
            ..context
        };
        Ok(solve_vapor_water_trial(
            state,
            workspace,
            trial_input,
            policy,
            trial_context,
        )?)
    };
    let mut trial = solve(kod, h_top)?;
    let mut v_top = trial_v_top(state, step_input, &trial);
    let (post_kod, post_h) = fortran_top_switch(
        kod,
        h_top,
        r_top,
        v_top,
        trial.candidate.heads()[n - 1],
        h_crit_a,
    );
    if post_kod != kod {
        switches.push(VaporSwitchRecord {
            stage: VaporSwitchStage::PostSolve,
            from_kod: kod,
            to_kod: post_kod,
            time,
            top_head: trial.candidate.heads()[n - 1],
            v_top,
            second_shift_would_flip: false,
        });
        kod = post_kod;
        h_top = post_h;
        trial = solve(kod, h_top)?;
        v_top = trial_v_top(state, step_input, &trial);
        let (again, _) = fortran_top_switch(
            kod,
            h_top,
            r_top,
            v_top,
            trial.candidate.heads()[n - 1],
            h_crit_a,
        );
        if again != kod {
            if let Some(last) = switches.last_mut() {
                last.second_shift_would_flip = true;
            }
        }
    }
    let top_head = trial.candidate.heads()[n - 1];
    let mut cursor = vapor_state.event_cursor;
    while cursor < schedule.events().len() && trial.accepted_time >= schedule.events()[cursor].time
    {
        cursor += 1;
    }
    Ok(VaporAtmosphericStep {
        below_critical_head: kod < 0 && top_head < h_crit_a,
        state: VaporAtmosphericState {
            kod_top: kod,
            h_top,
            r_top_old,
            r_top,
            record_cursor: Some(vapor_state.event_cursor),
            v_top,
            event_cursor: cursor,
        },
        switches,
        trial,
    })
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

    mod vapor_switch {
        use super::*;
        use crate::forcing::ForcingUnits;
        use crate::heat::config::HeatConfigInput;
        use crate::heat::vapor::kernels;
        use crate::heat::vapor::{
            VaporConfig, VaporConfigInput, VaporHistoryView, VaporPrimitiveInput,
            VaporSourceMaterial,
        };
        use crate::hydraulics::{VanGenuchtenModel, VanGenuchtenParameters};

        const H_CRIT: f64 = -105.0;

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

        /// Runs one vapor atmospheric step on a 3-node column at `top` head.
        fn run(
            top: f64,
            rates: AtmosphericRates,
            vapor_state: VaporAtmosphericState,
            coupling: VaporAtmosphericCoupling,
        ) -> Result<VaporAtmosphericStep, VaporAtmosphericError> {
            let model = VanGenuchtenModel::new(
                VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0e-5).unwrap(),
            )
            .unwrap();
            let mesh = Mesh1D::new(vec![0.0, 0.5, 1.0], vec![0; 3], None, 1, 1, 1.0).unwrap();
            let heads = vec![-100.0, -100.0, top];
            let theta: Vec<f64> = heads
                .iter()
                .map(|h| model.evaluate(*h).unwrap().theta)
                .collect();
            let state = WaterState::new(0.0, heads.clone(), theta.clone()).unwrap();
            let workspace = WaterWorkspace::new(3).unwrap();
            let temperature = [20.0; 3];
            let zero = [0.0; 3];
            let evals: Vec<_> = heads.iter().map(|h| model.evaluate(*h).unwrap()).collect();
            let conductivities: Vec<f64> = evals.iter().map(|e| e.conductivity).collect();
            let capacities: Vec<f64> = evals.iter().map(|e| e.capacity).collect();
            let vapor = kernels::evaluate(&VaporPrimitiveInput {
                heads: &heads,
                temperatures: &temperature,
                liquid_water_contents: &theta,
                liquid_conductivities: &conductivities,
                liquid_capacities: &capacities,
                material_indices: &[0; 3],
                saturated_water_contents: &[0.45],
                x_conv: 1.0,
                t_conv: 1.0,
            })
            .unwrap();
            let old = VaporHistoryView {
                time: 0.0,
                previous_timestep: None,
                heads: &heads,
                liquid_water_contents: &theta,
                temperatures: &temperature,
                vapor_contents: &vapor.vapor_contents,
                liquid_velocities: &zero,
                vapor_velocities: &zero,
                head_predictor: &heads,
            };
            let source = [VaporSourceMaterial {
                model: &model,
                table: None,
            }];
            let config = config();
            let context = VaporWaterContext {
                config: &config,
                old,
                temperature_estimate: &temperature,
                initial_heads: &heads,
                initial_material_heads: &heads,
                source_materials: &source,
            };
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
                    head: -100.0,
                    row: BoundaryRow {
                        lower: 0.0,
                        diagonal: 1.0,
                        upper: 0.0,
                        rhs: -100.0,
                    },
                    flux: 0.0,
                },
                upper: UpperBoundary::Atmospheric {
                    critical_head: -H_CRIT,
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
            let schedule = ForcingSchedule::new(
                rates,
                Vec::new(),
                ForcingUnits {
                    time: "h",
                    water_depth_rate: "cm/h",
                },
            )
            .unwrap();
            advance_atmospheric_step_vapor(
                &state,
                &workspace,
                input,
                policy,
                &schedule,
                vapor_state,
                coupling,
                context,
            )
        }

        fn coupling() -> VaporAtmosphericCoupling {
            VaporAtmosphericCoupling::new(1.0).unwrap()
        }

        #[test]
        fn flux_switches_to_head_when_surface_reaches_critical_head() {
            let evaporation = AtmosphericRates {
                precipitation: 0.0,
                evaporation: 1.0e-2,
            };
            let step = run(
                -104.5,
                evaporation,
                VaporAtmosphericState::new(),
                coupling(),
            )
            .unwrap();
            assert_eq!(step.state.kod_top, 4);
            assert_eq!(step.state.h_top, H_CRIT);
            let record = step.switches.first().expect("a switch is recorded");
            assert_eq!((record.from_kod, record.to_kod), (-4, 4));
            assert!(!record.second_shift_would_flip);
            assert!((step.trial.candidate.heads()[2] - H_CRIT).abs() < 1.0e-6);
        }

        #[test]
        fn head_switches_to_flux_on_rain() {
            let rain = AtmosphericRates {
                precipitation: 1.0e-6,
                evaporation: 0.0,
            };
            let head_mode = VaporAtmosphericState {
                kod_top: 4,
                h_top: H_CRIT,
                r_top_old: 0.1,
                r_top: 0.1,
                record_cursor: None,
                v_top: 0.01,
                event_cursor: 0,
            };
            let step = run(H_CRIT, rain, head_mode, coupling()).unwrap();
            assert_eq!(step.switches[0].stage, VaporSwitchStage::PreSolve);
            assert_eq!(
                (step.switches[0].from_kod, step.switches[0].to_kod),
                (4, -4)
            );
            assert_eq!(step.state.kod_top, -4);
        }

        #[test]
        fn r_top_is_fixed_within_a_record_and_reset_at_the_boundary() {
            let head_mode = VaporAtmosphericState {
                kod_top: 4,
                h_top: H_CRIT,
                r_top_old: 0.0,
                r_top: 0.01,
                record_cursor: Some(0),
                v_top: 0.001,
                event_cursor: 0,
            };
            // Mid-record (same record cursor): the stored demand survives, whatever the
            // rates say, as after a mid-record flux-to-head switch.
            let quiet = AtmosphericRates {
                precipitation: 0.0,
                evaporation: 0.0,
            };
            let mid = run(H_CRIT, quiet, head_mode, coupling()).unwrap();
            assert_eq!(mid.state.r_top, 0.01);
            // Record boundary in head mode: rTop = |rSoil| - |Prec| (HYDRUS.FOR:627-628),
            // here 0 on a dry day with no evaporation, and rTopOld keeps the old value.
            let boundary = VaporAtmosphericState {
                record_cursor: None,
                ..head_mode
            };
            let next = run(H_CRIT, quiet, boundary, coupling()).unwrap();
            assert_eq!(next.state.r_top, 0.0);
            assert_eq!(next.state.r_top_old, 0.01);
            assert_eq!(next.state.record_cursor, Some(0));
        }

        #[test]
        fn surface_ponding_is_rejected_explicitly() {
            let rates = rates(0.0);
            let mut layered = coupling();
            layered.surface_layer = true;
            let error = run(-500.0, rates, VaporAtmosphericState::new(), layered).unwrap_err();
            assert_eq!(error, VaporAtmosphericError::SurfacePondingUnsupported);
            assert!(error.to_string().contains("WLayer/hCritS"));
        }

        #[test]
        fn top_switch_follows_shift_order() {
            // Flux mode, head above zero: head mode at zero.
            assert_eq!(fortran_top_switch(-4, 0.0, 1.0, 0.0, 0.5, H_CRIT), (4, 0.0));
            // Flux mode at hCritA wins over the ponding test.
            assert_eq!(
                fortran_top_switch(-4, 0.0, 1.0, 0.0, H_CRIT, H_CRIT),
                (4, H_CRIT)
            );
            // Head mode, supply below demand with matching signs: stays.
            assert_eq!(
                fortran_top_switch(4, H_CRIT, 1.0, 0.5, H_CRIT, H_CRIT),
                (4, H_CRIT)
            );
            // 0.99*hCritA rule needs rTop < 0 and flips to flux.
            assert_eq!(
                fortran_top_switch(4, H_CRIT, -1.0, -2.0, -119.0, H_CRIT).0,
                -4
            );
        }
    }
}

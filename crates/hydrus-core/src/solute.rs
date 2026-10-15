//! Validated contract for M5 single-domain solute transport.
//!
//! Concentration and sorbed-mass units are caller-declared labels. Callers must
//! supply mutually compatible units for concentrations, sorbed pools, rates,
//! and fluxes; this contract does not impose a universal physical basis.

use crate::{
    hydraulics::{solve_tridiagonal_in_place, Mesh1D, TridiagonalError},
    SoluteConfigError,
};

/// Explicit labels used by a solute configuration; no unit conversion is implied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoluteUnits {
    pub concentration: String,
    pub sorbed_mass: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EquilibriumSorption {
    None,
    /// Linear isotherm `S = Kd * C`.
    Linear {
        distribution_coefficient: f64,
    },
    /// Source-anchored isotherm `S = fraction * K * C^n / (1 + nu * C^n)`.
    /// The parameter dimensions must be compatible with the declared units.
    Nonlinear {
        fraction: f64,
        coefficient: f64,
        saturation: f64,
        exponent: f64,
    },
}

/// Kinetic nonequilibrium pool and first-order exchange, following `SOLUTE.FOR:NEquil`.
/// `capacity_coefficient` is a concentration-to-sorbed-pool partition coefficient;
/// `transfer_rate` has inverse-time units compatible with the declared time unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KineticSorption {
    /// `Kd` in sorbed-mass units per concentration unit.
    pub capacity_coefficient: f64,
    /// `omega` in inverse units of the caller's time unit.
    pub transfer_rate: f64,
}

/// One species. Species vector order is the sequential first-order reaction order.
#[derive(Clone, Debug, PartialEq)]
pub struct SoluteSpecies {
    pub name: String,
    /// First-order rate (inverse time) transferring this species to the next configured
    /// species; terminal-species decay leaves the configured chain. This ordered-chain
    /// contract follows `SOLUTE.FOR:Solute` sequencing without asserting parity.
    pub decay_rate: f64,
    pub equilibrium_sorption: EquilibriumSorption,
    pub kinetic_sorption: Option<KineticSorption>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SoluteBoundaryMode {
    /// Prescribed dissolved concentration at the endpoint.
    Concentration { value: f64 },
    /// Prescribed total solute flux; positive flux enters the domain.
    TotalFlux { flux: f64 },
    /// Zero dispersive boundary flux with solution-dependent advective export.
    /// Inward water flow is rejected; zero flow is allowed.
    AdvectiveOutflow,
}

/// Geometric endpoint where an outlet flow reversal occurred.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoluteEndpoint {
    Lower,
    Upper,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoluteBoundaries {
    pub upper: SoluteBoundaryMode,
    pub lower: SoluteBoundaryMode,
}

/// Optional source capabilities that are deliberately rejected by M5.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredSoluteFeature {
    HeatOrVaporCoupling,
    DensityDependentTransport,
    Bacteria,
    Filtration,
    SpecialChemistry,
    DualPorosityWater,
    MobileImmobileTransport,
    ArtificialDispersion,
    MoistureDependentTransport,
    TemperatureDependence,
}

impl DeferredSoluteFeature {
    fn name(self) -> &'static str {
        match self {
            Self::HeatOrVaporCoupling => "heat/vapor coupling",
            Self::DensityDependentTransport => "density-dependent transport",
            Self::Bacteria => "bacteria",
            Self::Filtration => "filtration",
            Self::SpecialChemistry => "special chemistry",
            Self::DualPorosityWater => "dual-porosity water",
            Self::MobileImmobileTransport => "mobile-immobile transport",
            Self::ArtificialDispersion => "artificial dispersion",
            Self::MoistureDependentTransport => "moisture-dependent transport",
            Self::TemperatureDependence => "temperature-dependent transport",
        }
    }
}

/// Closed set of solute transport backends. Every step is dispatched by an
/// exhaustive `match`, so adding a variant requires handling it everywhere.
///
/// `FortranCompatible` is the default backend based on completed all-mode parity
/// verification against the pinned Fortran reference (see `planning/master/SOLUTE_FORTRAN_PARITY_18.md`).
/// `FiniteVolumeBackwardEuler` remains fully supported and explicitly selectable.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoluteBackend {
    /// Conservative finite-volume discretization with backward-Euler time stepping.
    FiniteVolumeBackwardEuler,
    /// Pinned-Fortran-compatible solute transport solver (default).
    FortranCompatible,
}

impl SoluteBackend {
    /// Every available backend, for contract tests and selection lists.
    pub const ALL: &'static [SoluteBackend] = &[
        SoluteBackend::FiniteVolumeBackwardEuler,
        SoluteBackend::FortranCompatible,
    ];
}

/// Validated selected-M5 capabilities. This type defines configuration only; it does not step.
///
/// Defaults to [`SoluteBackend::FortranCompatible`], verified across all active solute modes
/// in `planning/master/SOLUTE_FORTRAN_PARITY_18.md`. Callers may explicitly select
/// [`SoluteBackend::FiniteVolumeBackwardEuler`] via [`with_backend`](Self::with_backend).
#[derive(Clone, Debug, PartialEq)]
pub struct SoluteConfig {
    units: SoluteUnits,
    species: Vec<SoluteSpecies>,
    boundaries: Vec<SoluteBoundaries>,
    backend: SoluteBackend,
}

impl SoluteConfig {
    /// Construct a configuration with one shared endpoint pair broadcast to every species.
    pub fn new(
        units: SoluteUnits,
        species: Vec<SoluteSpecies>,
        boundaries: SoluteBoundaries,
        water_domain_count: usize,
        deferred: &[DeferredSoluteFeature],
    ) -> Result<Self, SoluteConfigError> {
        let species_boundaries = vec![boundaries; species.len()];
        Self::new_with_species_boundaries(
            units,
            species,
            species_boundaries,
            water_domain_count,
            deferred,
        )
    }

    /// Construct a configuration with one independently selected endpoint pair per species.
    pub fn new_with_species_boundaries(
        units: SoluteUnits,
        species: Vec<SoluteSpecies>,
        boundaries: Vec<SoluteBoundaries>,
        water_domain_count: usize,
        deferred: &[DeferredSoluteFeature],
    ) -> Result<Self, SoluteConfigError> {
        if water_domain_count != 1 {
            return Err(SoluteConfigError::Unsupported {
                option: "water domain count",
            });
        }
        if let Some(feature) = deferred.first() {
            return Err(SoluteConfigError::Unsupported {
                option: feature.name(),
            });
        }
        if units.concentration.trim().is_empty() {
            return Err(SoluteConfigError::Invalid {
                name: "concentration units",
                reason: "label must not be empty",
            });
        }
        if units.sorbed_mass.trim().is_empty() {
            return Err(SoluteConfigError::Invalid {
                name: "sorbed-mass units",
                reason: "label must not be empty",
            });
        }
        if species.is_empty() {
            return Err(SoluteConfigError::Invalid {
                name: "species",
                reason: "at least one species is required",
            });
        }
        for (index, item) in species.iter().enumerate() {
            if species[..index]
                .iter()
                .any(|previous| previous.name == item.name)
            {
                return Err(SoluteConfigError::Invalid {
                    name: "species name",
                    reason: "names must be unique to define an unambiguous chain",
                });
            }
        }
        if boundaries.len() != species.len() {
            return Err(SoluteConfigError::Invalid {
                name: "boundary pairs",
                reason: "one endpoint pair is required for each species",
            });
        }
        for item in &species {
            if item.name.trim().is_empty() {
                return Err(SoluteConfigError::Invalid {
                    name: "species name",
                    reason: "label must not be empty",
                });
            }
            if !item.decay_rate.is_finite() || item.decay_rate < 0.0 {
                return Err(SoluteConfigError::Invalid {
                    name: "decay rate",
                    reason: "must be finite and nonnegative",
                });
            }
            match item.equilibrium_sorption {
                EquilibriumSorption::None => {}
                EquilibriumSorption::Linear { distribution_coefficient } if distribution_coefficient.is_finite() && distribution_coefficient >= 0.0 => {}
                EquilibriumSorption::Nonlinear { fraction, coefficient, saturation, exponent } if fraction.is_finite() && (0.0..=1.0).contains(&fraction) && coefficient.is_finite() && coefficient >= 0.0 && saturation.is_finite() && saturation >= 0.0 && exponent.is_finite() && exponent > 0.0 => {}
                _ => return Err(SoluteConfigError::Invalid { name: "equilibrium sorption", reason: "parameters must be finite and nonnegative, fraction in [0,1], exponent positive" }),
            }
            if let Some(kinetic) = item.kinetic_sorption {
                if !kinetic.capacity_coefficient.is_finite()
                    || kinetic.capacity_coefficient < 0.0
                    || !kinetic.transfer_rate.is_finite()
                    || kinetic.transfer_rate < 0.0
                {
                    return Err(SoluteConfigError::Invalid {
                        name: "kinetic sorption",
                        reason: "capacity and transfer rate must be finite and nonnegative",
                    });
                }
            }
        }
        for pair in &boundaries {
            for mode in [pair.upper, pair.lower] {
                let value = match mode {
                    SoluteBoundaryMode::Concentration { value } => Some(value),
                    SoluteBoundaryMode::TotalFlux { flux } => Some(flux),
                    SoluteBoundaryMode::AdvectiveOutflow => None,
                };
                if value.is_some_and(|value| !value.is_finite()) {
                    return Err(SoluteConfigError::Invalid {
                        name: "boundary",
                        reason: "value must be finite",
                    });
                }
                if matches!(mode, SoluteBoundaryMode::Concentration { .. })
                    && value.is_some_and(|value| value < 0.0)
                {
                    return Err(SoluteConfigError::Invalid {
                        name: "boundary concentration",
                        reason: "must be nonnegative",
                    });
                }
            }
        }
        Ok(Self {
            units,
            species,
            boundaries,
            backend: SoluteBackend::FortranCompatible,
        })
    }

    /// Select the transport backend; the default is [`SoluteBackend::FortranCompatible`].
    pub fn with_backend(mut self, backend: SoluteBackend) -> Self {
        self.backend = backend;
        self
    }

    pub fn backend(&self) -> SoluteBackend {
        self.backend
    }

    pub fn units(&self) -> &SoluteUnits {
        &self.units
    }
    pub fn species(&self) -> &[SoluteSpecies] {
        &self.species
    }
    /// Endpoint pairs in configured species order.
    pub fn boundaries(&self) -> &[SoluteBoundaries] {
        &self.boundaries
    }
    pub fn water_domain_count(&self) -> usize {
        1
    }
}

/// Accepted solute values. Pool vectors are species-major: `species * nodes + node`.
#[derive(Clone, Debug, PartialEq)]
pub struct SoluteState {
    time: f64,
    node_count: usize,
    species_count: usize,
    concentrations: Vec<f64>,
    kinetic_sorbed: Vec<f64>,
    accounting: SoluteAccounting,
}

/// Accepted-only counters and cumulative mass terms, stored per species.
#[derive(Clone, Debug, PartialEq)]
pub struct SoluteAccounting {
    accepted_steps: u64,
    cumulative_boundary_mass: Vec<f64>,
    cumulative_reaction_mass: Vec<f64>,
    last_step: Vec<SoluteSpeciesBalance>,
}

/// Accepted-step mass diagnostic for one configured species.
///
/// All terms use the caller-declared compatible mass basis integrated over the
/// one-dimensional column. Boundary and reaction changes are signed positive
/// into the species inventory; the residual is storage change minus both.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SoluteSpeciesBalance {
    pub storage_change: f64,
    pub boundary_exchange: f64,
    pub reaction_change: f64,
    pub absolute_residual: f64,
    pub normalized_residual: f64,
}

impl SoluteAccounting {
    pub fn accepted_steps(&self) -> u64 {
        self.accepted_steps
    }
    pub fn cumulative_boundary_mass(&self) -> &[f64] {
        &self.cumulative_boundary_mass
    }
    pub fn cumulative_reaction_mass(&self) -> &[f64] {
        &self.cumulative_reaction_mass
    }
    /// Balance terms for the most recently accepted step, in species order.
    pub fn last_step(&self) -> &[SoluteSpeciesBalance] {
        &self.last_step
    }
}

impl SoluteState {
    /// Creates state from species-major dissolved and kinetic-sorbed pools.
    pub fn new(
        config: &SoluteConfig,
        mesh: &Mesh1D,
        time: f64,
        concentrations: Vec<f64>,
        kinetic_sorbed: Vec<f64>,
    ) -> Result<Self, SoluteConfigError> {
        let node_count = mesh.coordinates().len();
        let species_count = config.species.len();
        let pool_len = node_count
            .checked_mul(species_count)
            .ok_or(SoluteConfigError::Invalid {
                name: "state dimensions",
                reason: "node and species count overflow",
            })?;
        if !time.is_finite() {
            return Err(SoluteConfigError::Invalid {
                name: "state time",
                reason: "must be finite",
            });
        }
        if concentrations.len() != pool_len || kinetic_sorbed.len() != pool_len {
            return Err(SoluteConfigError::Invalid {
                name: "state dimensions",
                reason: "each species-major pool must match node_count * species_count",
            });
        }
        validate_solute_values("concentrations", &concentrations)?;
        validate_solute_values("kinetic sorbed pool", &kinetic_sorbed)?;
        Ok(Self {
            time,
            node_count,
            species_count,
            concentrations,
            kinetic_sorbed,
            accounting: SoluteAccounting {
                accepted_steps: 0,
                cumulative_boundary_mass: vec![0.0; species_count],
                cumulative_reaction_mass: vec![0.0; species_count],
                last_step: vec![SoluteSpeciesBalance::default(); species_count],
            },
        })
    }

    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn node_count(&self) -> usize {
        self.node_count
    }
    pub fn species_count(&self) -> usize {
        self.species_count
    }
    pub fn concentrations(&self) -> &[f64] {
        &self.concentrations
    }
    pub fn kinetic_sorbed(&self) -> &[f64] {
        &self.kinetic_sorbed
    }
    pub fn accounting(&self) -> &SoluteAccounting {
        &self.accounting
    }
}

/// Fixed-dimension trial buffers and complete accepted-state snapshot.
#[derive(Clone, Debug)]
pub struct SoluteWorkspace {
    node_count: usize,
    species_count: usize,
    trial_time: f64,
    trial_concentrations: Vec<f64>,
    trial_kinetic_sorbed: Vec<f64>,
    trial_accounting: SoluteAccounting,
    snapshot_time: f64,
    snapshot_concentrations: Vec<f64>,
    snapshot_kinetic_sorbed: Vec<f64>,
    snapshot_accounting: SoluteAccounting,
    trial_active: bool,
    lower: Vec<f64>,
    diagonal: Vec<f64>,
    upper: Vec<f64>,
    rhs: Vec<f64>,
    previous_species_mass: Vec<f64>,
    reaction_source: Vec<f64>,
    storage_before: Vec<f64>,
}

impl SoluteWorkspace {
    pub fn new(node_count: usize, species_count: usize) -> Result<Self, SoluteConfigError> {
        let pool_len = node_count
            .checked_mul(species_count)
            .ok_or(SoluteConfigError::Invalid {
                name: "workspace dimensions",
                reason: "node and species count overflow",
            })?;
        if node_count == 0 || species_count == 0 {
            return Err(SoluteConfigError::Invalid {
                name: "workspace dimensions",
                reason: "node and species counts must be nonzero",
            });
        }
        Ok(Self {
            node_count,
            species_count,
            trial_time: 0.0,
            trial_concentrations: vec![0.0; pool_len],
            trial_kinetic_sorbed: vec![0.0; pool_len],
            trial_accounting: zero_accounting(species_count),
            snapshot_time: 0.0,
            snapshot_concentrations: vec![0.0; pool_len],
            snapshot_kinetic_sorbed: vec![0.0; pool_len],
            snapshot_accounting: zero_accounting(species_count),
            trial_active: false,
            lower: vec![0.0; node_count],
            diagonal: vec![0.0; node_count],
            upper: vec![0.0; node_count],
            rhs: vec![0.0; node_count],
            previous_species_mass: vec![0.0; node_count],
            reaction_source: vec![0.0; node_count],
            storage_before: vec![0.0; species_count],
        })
    }

    /// Snapshots every accepted solute field and initializes the mutable trial buffers.
    pub fn begin_trial(&mut self, state: &SoluteState) -> Result<(), SoluteConfigError> {
        self.validate_state_dimensions(state)?;
        validate_accounting(&state.accounting, self.species_count)?;
        self.snapshot_time = state.time;
        self.trial_time = state.time;
        self.snapshot_concentrations
            .copy_from_slice(&state.concentrations);
        self.snapshot_kinetic_sorbed
            .copy_from_slice(&state.kinetic_sorbed);
        copy_accounting(&mut self.snapshot_accounting, &state.accounting);
        self.trial_concentrations
            .copy_from_slice(&state.concentrations);
        self.trial_kinetic_sorbed
            .copy_from_slice(&state.kinetic_sorbed);
        copy_accounting(&mut self.trial_accounting, &state.accounting);
        self.trial_active = true;
        Ok(())
    }

    /// Promotes the trial atomically after validating time, dimensions and all trial values.
    pub fn accept_trial(
        &mut self,
        state: &mut SoluteState,
        accepted_time: f64,
    ) -> Result<(), SoluteConfigError> {
        self.validate_state_dimensions(state)?;
        if !self.trial_active {
            return Err(SoluteConfigError::Invalid {
                name: "trial lifecycle",
                reason: "begin_trial must precede acceptance",
            });
        }
        if !accepted_time.is_finite() || accepted_time <= state.time {
            return Err(SoluteConfigError::Invalid {
                name: "accepted time",
                reason: "must be finite and greater than current time",
            });
        }
        validate_solute_values("trial concentrations", &self.trial_concentrations)?;
        validate_solute_values("trial kinetic sorbed pool", &self.trial_kinetic_sorbed)?;
        validate_accounting(&self.trial_accounting, self.species_count)?;
        let accepted_steps =
            state
                .accounting
                .accepted_steps
                .checked_add(1)
                .ok_or(SoluteConfigError::Invalid {
                    name: "accepted steps",
                    reason: "counter overflow",
                })?;
        state.time = accepted_time;
        state
            .concentrations
            .copy_from_slice(&self.trial_concentrations);
        state
            .kinetic_sorbed
            .copy_from_slice(&self.trial_kinetic_sorbed);
        copy_accounting(&mut state.accounting, &self.trial_accounting);
        state.accounting.accepted_steps = accepted_steps;
        self.trial_time = accepted_time;
        self.trial_active = false;
        Ok(())
    }

    /// Discards a trial and restores all trial buffers from its accepted-state snapshot.
    pub fn reject_trial(&mut self) {
        self.trial_time = self.snapshot_time;
        self.trial_concentrations
            .copy_from_slice(&self.snapshot_concentrations);
        self.trial_kinetic_sorbed
            .copy_from_slice(&self.snapshot_kinetic_sorbed);
        copy_accounting(&mut self.trial_accounting, &self.snapshot_accounting);
        self.trial_active = false;
    }

    pub fn trial_time(&self) -> f64 {
        self.trial_time
    }
    pub fn trial_concentrations(&self) -> &[f64] {
        &self.trial_concentrations
    }
    #[cfg(test)]
    pub(crate) fn trial_concentrations_mut(&mut self) -> &mut [f64] {
        &mut self.trial_concentrations
    }
    pub fn trial_kinetic_sorbed(&self) -> &[f64] {
        &self.trial_kinetic_sorbed
    }
    #[cfg(test)]
    pub(crate) fn trial_kinetic_sorbed_mut(&mut self) -> &mut [f64] {
        &mut self.trial_kinetic_sorbed
    }
    pub fn trial_accounting(&self) -> &SoluteAccounting {
        &self.trial_accounting
    }
    #[cfg(test)]
    pub(crate) fn trial_boundary_mass_mut(&mut self) -> &mut [f64] {
        &mut self.trial_accounting.cumulative_boundary_mass
    }
    pub(crate) fn trial_reaction_mass_mut(&mut self) -> &mut [f64] {
        &mut self.trial_accounting.cumulative_reaction_mass
    }
    #[cfg(test)]
    pub(crate) fn set_trial_accepted_steps(&mut self, accepted_steps: u64) {
        self.trial_accounting.accepted_steps = accepted_steps;
    }

    fn validate_state_dimensions(&self, state: &SoluteState) -> Result<(), SoluteConfigError> {
        if self.node_count != state.node_count
            || self.species_count != state.species_count
            || state.accounting.cumulative_boundary_mass.len() != self.species_count
            || state.accounting.cumulative_reaction_mass.len() != self.species_count
        {
            return Err(SoluteConfigError::Invalid {
                name: "workspace dimensions",
                reason: "do not match solute state node/species dimensions",
            });
        }
        Ok(())
    }
}

fn zero_accounting(species_count: usize) -> SoluteAccounting {
    SoluteAccounting {
        accepted_steps: 0,
        cumulative_boundary_mass: vec![0.0; species_count],
        cumulative_reaction_mass: vec![0.0; species_count],
        last_step: vec![SoluteSpeciesBalance::default(); species_count],
    }
}

fn copy_accounting(destination: &mut SoluteAccounting, source: &SoluteAccounting) {
    destination.accepted_steps = source.accepted_steps;
    destination
        .cumulative_boundary_mass
        .copy_from_slice(&source.cumulative_boundary_mass);
    destination
        .cumulative_reaction_mass
        .copy_from_slice(&source.cumulative_reaction_mass);
    destination.last_step.copy_from_slice(&source.last_step);
}

fn validate_solute_values(name: &'static str, values: &[f64]) -> Result<(), SoluteConfigError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(SoluteConfigError::Invalid {
            name,
            reason: "values must be finite",
        });
    }
    Ok(())
}

fn validate_accounting(
    accounting: &SoluteAccounting,
    species_count: usize,
) -> Result<(), SoluteConfigError> {
    if accounting.cumulative_boundary_mass.len() != species_count
        || accounting.cumulative_reaction_mass.len() != species_count
        || accounting.last_step.len() != species_count
    {
        return Err(SoluteConfigError::Invalid {
            name: "trial accounting dimensions",
            reason: "mass vectors must have one entry per configured species",
        });
    }
    validate_solute_values(
        "trial boundary accounting",
        &accounting.cumulative_boundary_mass,
    )?;
    validate_solute_values(
        "trial reaction accounting",
        &accounting.cumulative_reaction_mass,
    )?;
    for balance in &accounting.last_step {
        if ![
            balance.storage_change,
            balance.boundary_exchange,
            balance.reaction_change,
            balance.absolute_residual,
            balance.normalized_residual,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(SoluteConfigError::Invalid {
                name: "trial balance",
                reason: "balance terms must be finite",
            });
        }
    }
    Ok(())
}

pub use crate::water_state::AcceptedWaterSnapshot;

/// Inputs held fixed while solving one dissolved transport step.
#[derive(Clone, Copy, Debug)]
pub struct SoluteTransportInput<'a> {
    pub accepted_water: AcceptedWaterSnapshot<'a>,
    /// Water contents at the current solute-state time, before this interval.
    pub start_water_contents: &'a [f64],
    /// Effective dispersion coefficient at each interior face, length `node_count - 1`.
    pub face_dispersion: &'a [f64],
    pub timestep: f64,
    pub minimum_pivot: f64,
}

/// Per-node solid density and deterministic nonlinear solve limits for M5 processes.
#[derive(Clone, Copy, Debug)]
pub struct SoluteProcessInput<'a> {
    /// Solid mass per soil volume, compatible with `SoluteUnits::sorbed_mass` so
    /// `bulk_density * sorbed_mass` converts to concentration-volume storage.
    pub bulk_density: &'a [f64],
    pub max_iterations: usize,
    /// Absolute concentration-change threshold in the declared concentration units.
    /// This is a deterministic solver setting, not a calibrated physical tolerance.
    pub nonlinear_tolerance: f64,
}

/// Errors returned by a dissolved-solute transport step.
#[derive(Clone, Debug, PartialEq)]
pub enum SoluteStepError {
    Configuration(SoluteConfigError),
    Linear(TridiagonalError),
    NonConverged {
        species: usize,
        iterations: usize,
        residual: f64,
    },
    InwardFlowAtOutlet {
        species: usize,
        endpoint: SoluteEndpoint,
        flux: f64,
    },
}

impl core::fmt::Display for SoluteStepError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Configuration(error) => error.fmt(formatter),
            Self::Linear(error) => error.fmt(formatter),
            Self::NonConverged { species, iterations, residual } => write!(
                formatter,
                "solute nonlinear solve for species {species} did not converge after {iterations} iterations (change={residual})"
            ),
            Self::InwardFlowAtOutlet { species, endpoint, flux } => write!(
                formatter,
                "inward water flux {flux} at {endpoint:?} advective outlet for species {species}"
            ),
        }
    }
}

impl std::error::Error for SoluteStepError {}

impl From<SoluteConfigError> for SoluteStepError {
    fn from(error: SoluteConfigError) -> Self {
        Self::Configuration(error)
    }
}

impl From<TridiagonalError> for SoluteStepError {
    fn from(error: TridiagonalError) -> Self {
        Self::Linear(error)
    }
}

/// Advances dissolved concentrations by one implicit conservative transport step.
/// Internal advection fluxes are upwinded, while dispersion uses adjacent-face
/// concentration differences. No water quantity is reconstructed or mutated.
pub fn advance_solute_step(
    state: &mut SoluteState,
    workspace: &mut SoluteWorkspace,
    mesh: &Mesh1D,
    config: &SoluteConfig,
    input: SoluteTransportInput<'_>,
) -> Result<(), SoluteStepError> {
    match config.backend() {
        SoluteBackend::FiniteVolumeBackwardEuler => {
            advance_finite_volume_backward_euler_step(state, workspace, mesh, config, input, None)
        }
        SoluteBackend::FortranCompatible => {
            advance_fortran_compatible_step(state, workspace, mesh, config, input, None)
        }
    }
}

/// Advances transport with the configured M5 reaction and sorption processes.
/// `process_input` supplies bulk density and a bounded nonlinear solve policy.
pub fn advance_solute_step_with_processes(
    state: &mut SoluteState,
    workspace: &mut SoluteWorkspace,
    mesh: &Mesh1D,
    config: &SoluteConfig,
    input: SoluteTransportInput<'_>,
    process_input: Option<SoluteProcessInput<'_>>,
) -> Result<(), SoluteStepError> {
    match config.backend() {
        SoluteBackend::FiniteVolumeBackwardEuler => advance_finite_volume_backward_euler_step(
            state,
            workspace,
            mesh,
            config,
            input,
            process_input,
        ),
        SoluteBackend::FortranCompatible => {
            advance_fortran_compatible_step(state, workspace, mesh, config, input, process_input)
        }
    }
}

fn advance_fortran_compatible_step(
    state: &mut SoluteState,
    workspace: &mut SoluteWorkspace,
    mesh: &Mesh1D,
    config: &SoluteConfig,
    input: SoluteTransportInput<'_>,
    process_input: Option<SoluteProcessInput<'_>>,
) -> Result<(), SoluteStepError> {
    let species_count = config.species.len();
    let node_count = mesh.coordinates().len();
    validate_transport_input(state, workspace, mesh, config, input, process_input)?;

    if node_count < 3 {
        return Err(invalid_transport(
            "mesh",
            "Fortran compatibility solver requires at least 3 nodes",
        ));
    }

    workspace.begin_trial(state)?;
    let solve_result = (|| {
        if input.minimum_pivot > 1.0 {
            return Err(SoluteStepError::Linear(TridiagonalError::SingularPivot {
                index: 0,
                value: 0.0,
                minimum: input.minimum_pivot,
            }));
        }

        for species in 0..species_count {
            workspace.storage_before[species] = species_storage(
                workspace.trial_concentrations(),
                workspace.trial_kinetic_sorbed(),
                species,
                mesh,
                config,
                input.start_water_contents,
                process_input,
            )?;
        }

        let mut ch_par_row = vec![0.0_f32; species_count * 16 + 4];
        let default_density = process_input.map_or(0.8_f32, |p| p.bulk_density[0] as f32);
        ch_par_row[0] = default_density;
        ch_par_row[1] = input.face_dispersion.first().copied().unwrap_or(0.0) as f32;
        ch_par_row[2] = 1.0_f32; // Frac

        let mut linear = Vec::with_capacity(species_count);
        for (js, sp) in config.species.iter().enumerate() {
            let offset = js * 16;
            let (kd, exponent, nu) = match sp.equilibrium_sorption {
                EquilibriumSorption::None => (0.0_f32, 1.0_f32, 0.0_f32),
                EquilibriumSorption::Linear {
                    distribution_coefficient,
                } => (distribution_coefficient as f32, 1.0_f32, 0.0_f32),
                EquilibriumSorption::Nonlinear {
                    coefficient,
                    exponent,
                    saturation,
                    ..
                } => (coefficient as f32, exponent as f32, saturation as f32),
            };
            ch_par_row[offset + 6] = kd;
            ch_par_row[offset + 7] = nu;
            ch_par_row[offset + 8] = exponent;
            ch_par_row[offset + 13] = sp.decay_rate as f32; // GamL1

            let is_linear = matches!(
                sp.equilibrium_sorption,
                EquilibriumSorption::None | EquilibriumSorption::Linear { .. }
            );
            linear.push(is_linear);
        }

        let (k_bot, c_bot) = match config.boundaries[0].lower {
            SoluteBoundaryMode::TotalFlux { flux } => {
                let v = input.accepted_water.face_fluxes[0];
                if v.abs() > 1e-30 {
                    (-1, vec![(flux / v) as f32; species_count])
                } else {
                    (1, vec![flux as f32; species_count])
                }
            }
            SoluteBoundaryMode::Concentration { value } => (1, vec![value as f32; species_count]),
            SoluteBoundaryMode::AdvectiveOutflow => (0, vec![0.0_f32; species_count]),
        };

        let (k_top, c_top) = match config.boundaries[0].upper {
            SoluteBoundaryMode::TotalFlux { flux } => {
                let v = input.accepted_water.face_fluxes[node_count];
                if v.abs() > 1e-30 {
                    (-1, vec![(-flux / v) as f32; species_count])
                } else {
                    (1, vec![flux as f32; species_count])
                }
            }
            SoluteBoundaryMode::Concentration { value } => (1, vec![value as f32; species_count]),
            SoluteBoundaryMode::AdvectiveOutflow => (0, vec![0.0_f32; species_count]),
        };

        let (max_it_c, c_tol_a, c_tol_r) = if let Some(p) = process_input {
            (
                p.max_iterations as u32,
                p.nonlinear_tolerance as f32,
                p.nonlinear_tolerance as f32,
            )
        } else {
            (10, 0.0_f32, 0.0_f32)
        };

        let mut step_cfg = crate::solute_fortran_compat::driver::SoluteStepConfig {
            x: mesh.coordinates().iter().map(|&x| x as f32).collect(),
            mat_num: vec![1; node_count],
            th_sat: vec![1.0; 1],
            mobile_immobile: vec![false; 1],
            ch_par: vec![ch_par_row],
            t_dep: vec![0.0; species_count * 16 + 4],
            species_count,
            linear,
            equil: true,
            upwind: false,
            art_disp: false,
            tort: false,
            i_tort: 0,
            bact: false,
            filtr: false,
            dual_neq: false,
            water_flow: true,
            root_sink: false,
            vapor: false,
            atm_bc: false,
            density: false,
            dual_por: 0,
            conc_type: 0,
            moisture_selector: 0,
            moisture: None,
            epsi: 1.0, // backward Euler (implicit)
            t_pulse: 1.0e30,
            dt_min: (input.timestep * 1e-4) as f32,
            k_bot,
            k_top,
            c_atm: 0.0,
            d_surf: 0.0,
            r_bot: 0.0,
            t_level: 2,
            pe_cr: 2.0,
            x_conv: 1.0,
            t_conv: 1.0,
            c_tol_a,
            c_tol_r,
            root_uptake: None,
            size_exclusion: None,
        };

        let mut step_st =
            crate::solute_fortran_compat::driver::SoluteStepState::blank(node_count, species_count);
        for js in 0..species_count {
            let offset = js * node_count;
            for i in 0..node_count {
                step_st.conc[js][i] = workspace.trial_concentrations[offset + i] as f32;
                step_st.sorb[js][i] = workspace.trial_kinetic_sorbed[offset + i] as f32;
            }
        }
        for i in 0..node_count {
            step_st.th_o[i] = input.start_water_contents[i] as f32;
            step_st.th_n[i] = input.accepted_water.water_contents[i] as f32;
            step_st.v_o[i] = input.accepted_water.face_fluxes[i] as f32;
            step_st.v_n[i] = input.accepted_water.face_fluxes[i] as f32;
            step_st.temp_o[i] = 20.0;
            step_st.temp_n[i] = 20.0;
        }
        step_st.c_bot = c_bot;
        step_st.c_top = c_top;
        step_st.t = state.time;
        step_st.dt = input.timestep as f32;
        step_st.c_mid = Some(0.0);

        let nonlin_cfg = crate::solute_fortran_compat::iteration::NonlinearConfig {
            max_it_c,
            dt_min: (input.timestep * 1e-4) as f32,
            c_tol_a,
            c_tol_r,
            water_flow: true,
        };

        crate::solute_fortran_compat::iteration::solve_solute_with_nonlinear(
            &mut step_cfg,
            &mut step_st,
            &nonlin_cfg,
        )
        .map_err(|err| invalid_transport("fortran compatible solve", err))?;

        for js in 0..species_count {
            let offset = js * node_count;
            for i in 0..node_count {
                workspace.trial_concentrations[offset + i] = step_st.conc[js][i] as f64;
                workspace.trial_kinetic_sorbed[offset + i] = step_st.sorb[js][i] as f64;
            }
        }

        validate_solute_values("trial concentrations", &workspace.trial_concentrations)?;
        update_step_balances(state, workspace, mesh, config, input, process_input)?;
        workspace.accept_trial(state, input.accepted_water.accepted_time)?;
        Ok(())
    })();

    if solve_result.is_err() {
        workspace.reject_trial();
    }
    solve_result
}

fn advance_finite_volume_backward_euler_step(
    state: &mut SoluteState,
    workspace: &mut SoluteWorkspace,
    mesh: &Mesh1D,
    config: &SoluteConfig,
    input: SoluteTransportInput<'_>,
    process_input: Option<SoluteProcessInput<'_>>,
) -> Result<(), SoluteStepError> {
    let species_count = config.species.len();
    validate_transport_input(state, workspace, mesh, config, input, process_input)?;

    workspace.begin_trial(state)?;
    let solve_result = (|| {
        for species in 0..species_count {
            workspace.storage_before[species] = species_storage(
                workspace.trial_concentrations(),
                workspace.trial_kinetic_sorbed(),
                species,
                mesh,
                config,
                input.start_water_contents,
                process_input,
            )?;
        }
        for species in 0..species_count {
            if let Some(process_input) = process_input {
                assemble_and_solve_species_with_processes(
                    state,
                    workspace,
                    mesh,
                    config,
                    input,
                    process_input,
                    species,
                )?;
            } else {
                assemble_and_solve_species(state, workspace, mesh, config, input, species)?;
            }
        }
        validate_solute_values("trial concentrations", &workspace.trial_concentrations)?;
        update_step_balances(state, workspace, mesh, config, input, process_input)?;
        workspace.accept_trial(state, input.accepted_water.accepted_time)?;
        Ok(())
    })();
    if solve_result.is_err() {
        workspace.reject_trial();
    }
    solve_result
}

fn validate_transport_input(
    state: &SoluteState,
    workspace: &SoluteWorkspace,
    mesh: &Mesh1D,
    config: &SoluteConfig,
    input: SoluteTransportInput<'_>,
    process_input: Option<SoluteProcessInput<'_>>,
) -> Result<(), SoluteStepError> {
    let node_count = mesh.coordinates().len();
    let species_count = config.species.len();
    if node_count < 2 {
        return Err(invalid_transport("mesh", "at least two nodes are required"));
    }
    if state.node_count != node_count || state.species_count != species_count {
        return Err(invalid_transport(
            "state dimensions",
            "do not match mesh/config",
        ));
    }
    workspace.validate_state_dimensions(state)?;
    if input.start_water_contents.len() != node_count {
        return Err(invalid_transport(
            "start water contents",
            "length must equal mesh node count",
        ));
    }
    if input.accepted_water.water_contents.len() != node_count {
        return Err(invalid_transport(
            "water contents",
            "length must equal mesh node count",
        ));
    }
    let expected_flux_count = node_count
        .checked_add(1)
        .ok_or_else(|| invalid_transport("face fluxes", "expected length overflows usize"))?;
    if input.accepted_water.face_fluxes.len() != expected_flux_count {
        return Err(invalid_transport(
            "face fluxes",
            "length must equal node_count + 1",
        ));
    }
    if input.face_dispersion.len() != node_count - 1 {
        return Err(invalid_transport(
            "face dispersion",
            "length must equal node_count - 1",
        ));
    }
    let expected_end_time = state.time + input.timestep;
    if !expected_end_time.is_finite() {
        return Err(invalid_transport(
            "expected accepted water time",
            "state time plus timestep must remain finite",
        ));
    }
    let time_scale = state.time.abs().max(expected_end_time.abs());
    let roundoff_bound = 8.0 * f64::EPSILON * time_scale;
    if !input.accepted_water.accepted_time.is_finite()
        || (input.accepted_water.accepted_time - expected_end_time).abs() > roundoff_bound
    {
        return Err(invalid_transport(
            "accepted water time",
            "must match solute state time plus timestep within roundoff",
        ));
    }
    if !input.timestep.is_finite() || input.timestep <= 0.0 {
        return Err(invalid_transport("timestep", "must be finite and positive"));
    }
    if !input.minimum_pivot.is_finite() || input.minimum_pivot <= 0.0 {
        return Err(invalid_transport(
            "minimum pivot",
            "must be finite and positive",
        ));
    }
    validate_solute_values("start water contents", input.start_water_contents)?;
    if input.start_water_contents.iter().any(|theta| *theta <= 0.0) {
        return Err(invalid_transport(
            "start water contents",
            "must be positive",
        ));
    }
    validate_solute_values(
        "accepted water contents",
        input.accepted_water.water_contents,
    )?;
    if input
        .accepted_water
        .water_contents
        .iter()
        .any(|theta| *theta <= 0.0)
    {
        return Err(invalid_transport("water contents", "must be positive"));
    }
    validate_solute_values(
        "accepted water face fluxes",
        input.accepted_water.face_fluxes,
    )?;
    validate_solute_values("face dispersion", input.face_dispersion)?;
    if input.face_dispersion.iter().any(|value| *value < 0.0) {
        return Err(invalid_transport("face dispersion", "must be nonnegative"));
    }
    let processes_configured = config.species.iter().any(|species| {
        species.decay_rate > 0.0
            || species.kinetic_sorption.is_some()
            || !matches!(species.equilibrium_sorption, EquilibriumSorption::None)
    });
    match process_input {
        None if processes_configured => {
            return Err(invalid_transport(
                "process input",
                "bulk density and nonlinear policy are required for configured reaction/sorption",
            ));
        }
        Some(process) => {
            if process.bulk_density.len() != node_count {
                return Err(invalid_transport(
                    "bulk density",
                    "length must equal mesh node count",
                ));
            }
            validate_solute_values("bulk density", process.bulk_density)?;
            if process.bulk_density.iter().any(|density| *density <= 0.0) {
                return Err(invalid_transport("bulk density", "must be positive"));
            }
            if process.max_iterations == 0 {
                return Err(invalid_transport(
                    "nonlinear iteration limit",
                    "must be positive",
                ));
            }
            if !process.nonlinear_tolerance.is_finite() || process.nonlinear_tolerance <= 0.0 {
                return Err(invalid_transport(
                    "nonlinear tolerance",
                    "must be finite and positive",
                ));
            }
            validate_solute_values("accepted concentrations", &state.concentrations)?;
            validate_solute_values("accepted kinetic sorbed pool", &state.kinetic_sorbed)?;
            if state.concentrations.iter().any(|value| *value < 0.0)
                || state.kinetic_sorbed.iter().any(|value| *value < 0.0)
            {
                return Err(invalid_transport(
                    "accepted pools",
                    "concentration and sorbed pool values must be nonnegative",
                ));
            }
        }
        None => {}
    }
    for pair in &config.boundaries {
        for boundary in [pair.upper, pair.lower] {
            if let SoluteBoundaryMode::Concentration { value } = boundary {
                if value < 0.0 {
                    return Err(invalid_transport(
                        "boundary concentration",
                        "must be nonnegative",
                    ));
                }
            }
        }
    }
    for (species, pair) in config.boundaries.iter().enumerate() {
        if matches!(pair.lower, SoluteBoundaryMode::AdvectiveOutflow) {
            let flux = input.accepted_water.face_fluxes[0];
            if flux > 0.0 {
                return Err(SoluteStepError::InwardFlowAtOutlet {
                    species,
                    endpoint: SoluteEndpoint::Lower,
                    flux,
                });
            }
        }
        if matches!(pair.upper, SoluteBoundaryMode::AdvectiveOutflow) {
            let flux = input.accepted_water.face_fluxes[node_count];
            if flux < 0.0 {
                return Err(SoluteStepError::InwardFlowAtOutlet {
                    species,
                    endpoint: SoluteEndpoint::Upper,
                    flux,
                });
            }
        }
    }
    Ok(())
}

fn species_storage(
    concentrations: &[f64],
    kinetic_sorbed: &[f64],
    species: usize,
    mesh: &Mesh1D,
    config: &SoluteConfig,
    water_contents: &[f64],
    process_input: Option<SoluteProcessInput<'_>>,
) -> Result<f64, SoluteStepError> {
    let nodes = mesh.coordinates().len();
    let offset = species * nodes;
    let mut total = 0.0;
    for node in 0..nodes {
        let density = process_input.map_or(0.0, |process| process.bulk_density[node]);
        let concentration = concentrations[offset + node];
        let equilibrium = equilibrium_sorption_value(
            &config.species[species].equilibrium_sorption,
            concentration,
        )?
        .0;
        let mass_per_volume = water_contents[node] * concentration
            + density * (equilibrium + kinetic_sorbed[offset + node]);
        total += control_volume_width(mesh.coordinates(), node) * mass_per_volume;
    }
    if total.is_finite() {
        Ok(total)
    } else {
        Err(invalid_transport(
            "species storage",
            "integrated storage must remain finite",
        ))
    }
}

fn update_step_balances(
    state: &SoluteState,
    workspace: &mut SoluteWorkspace,
    mesh: &Mesh1D,
    config: &SoluteConfig,
    input: SoluteTransportInput<'_>,
    process_input: Option<SoluteProcessInput<'_>>,
) -> Result<(), SoluteStepError> {
    for species in 0..config.species.len() {
        let boundaries = config.boundaries[species];
        let has_concentration_boundary =
            matches!(boundaries.lower, SoluteBoundaryMode::Concentration { .. })
                || matches!(boundaries.upper, SoluteBoundaryMode::Concentration { .. });
        let prescribed_boundary = [boundaries.lower, boundaries.upper]
            .iter()
            .map(|boundary| match boundary {
                SoluteBoundaryMode::TotalFlux { flux } => flux * input.timestep,
                SoluteBoundaryMode::Concentration { .. } => 0.0,
                SoluteBoundaryMode::AdvectiveOutflow => 0.0,
            })
            .sum::<f64>();
        let lower_outlet = if matches!(boundaries.lower, SoluteBoundaryMode::AdvectiveOutflow) {
            input.timestep
                * input.accepted_water.face_fluxes[0]
                * workspace.trial_concentrations[species * workspace.node_count]
        } else {
            0.0
        };
        let upper_outlet = if matches!(boundaries.upper, SoluteBoundaryMode::AdvectiveOutflow) {
            -input.timestep
                * input.accepted_water.face_fluxes[workspace.node_count]
                * workspace.trial_concentrations[(species + 1) * workspace.node_count - 1]
        } else {
            0.0
        };
        let known_boundary = prescribed_boundary + lower_outlet + upper_outlet;
        let storage_after = species_storage(
            &workspace.trial_concentrations,
            &workspace.trial_kinetic_sorbed,
            species,
            mesh,
            config,
            input.accepted_water.water_contents,
            process_input,
        )?;
        let storage_change = storage_after - workspace.storage_before[species];
        let reaction_change = workspace.trial_accounting.cumulative_reaction_mass[species]
            - state.accounting.cumulative_reaction_mass[species];
        // Configured total-solute flux is already positive inward at either
        // endpoint, so sum those values directly. The water face snapshot uses
        // +x signs for advection and does not change this solute-boundary mode.
        // Concentration rows provide no measured solute flux; infer their net
        // exchange from the solved discrete balance after known fluxes.
        let concentration_exchange = if has_concentration_boundary {
            storage_change - reaction_change - known_boundary
        } else {
            0.0
        };
        let boundary_exchange = known_boundary + concentration_exchange;
        let absolute_residual = (storage_change - boundary_exchange - reaction_change).abs();
        let denominator = storage_change.abs() + boundary_exchange.abs() + reaction_change.abs();
        let normalized_residual = if denominator == 0.0 {
            absolute_residual
        } else {
            absolute_residual / denominator
        };
        if ![
            storage_change,
            reaction_change,
            boundary_exchange,
            absolute_residual,
            normalized_residual,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(invalid_transport(
                "species balance",
                "balance terms must remain finite",
            ));
        }
        workspace.trial_accounting.cumulative_boundary_mass[species] += boundary_exchange;
        if !workspace.trial_accounting.cumulative_boundary_mass[species].is_finite() {
            return Err(invalid_transport(
                "boundary accounting",
                "cumulative boundary mass must remain finite",
            ));
        }
        workspace.trial_accounting.last_step[species] = SoluteSpeciesBalance {
            storage_change,
            boundary_exchange,
            reaction_change,
            absolute_residual,
            normalized_residual,
        };
    }
    Ok(())
}

fn assemble_and_solve_species(
    state: &SoluteState,
    workspace: &mut SoluteWorkspace,
    mesh: &Mesh1D,
    config: &SoluteConfig,
    input: SoluteTransportInput<'_>,
    species: usize,
) -> Result<(), SoluteStepError> {
    let count = workspace.node_count;
    let offset = species * count;
    let coordinates = mesh.coordinates();
    assemble_flux_operator(workspace, mesh, input);
    for node in 0..count {
        let width = if node == 0 {
            (coordinates[1] - coordinates[0]) / 2.0
        } else if node + 1 == count {
            (coordinates[count - 1] - coordinates[count - 2]) / 2.0
        } else {
            (coordinates[node + 1] - coordinates[node - 1]) / 2.0
        };
        let storage = input.accepted_water.water_contents[node] * width / input.timestep;
        workspace.diagonal[node] += storage;
        workspace.rhs[node] = storage * state.concentrations[offset + node];
    }

    apply_endpoint_boundary_rows(workspace, config.boundaries[species], input, count);

    validate_solute_values("assembled transport coefficients", &workspace.lower)?;
    validate_solute_values("assembled transport coefficients", &workspace.diagonal)?;
    validate_solute_values("assembled transport coefficients", &workspace.upper)?;
    validate_solute_values("assembled transport rhs", &workspace.rhs)?;
    solve_tridiagonal_in_place(
        &workspace.lower,
        &mut workspace.diagonal,
        &workspace.upper,
        &mut workspace.rhs,
        input.minimum_pivot,
    )?;
    validate_solute_values("solved concentrations", &workspace.rhs)?;
    workspace.trial_concentrations[offset..offset + count].copy_from_slice(&workspace.rhs);
    Ok(())
}

fn assemble_and_solve_species_with_processes(
    state: &SoluteState,
    workspace: &mut SoluteWorkspace,
    mesh: &Mesh1D,
    config: &SoluteConfig,
    input: SoluteTransportInput<'_>,
    process: SoluteProcessInput<'_>,
    species: usize,
) -> Result<(), SoluteStepError> {
    let count = workspace.node_count;
    let offset = species * count;
    let species_config = &config.species[species];
    let nonlinear = matches!(
        species_config.equilibrium_sorption,
        EquilibriumSorption::Nonlinear { .. }
    );
    let decay = species_config.decay_rate;
    if species == 0 {
        workspace.reaction_source.fill(0.0);
    } else {
        let previous_decay = config.species[species - 1].decay_rate;
        for node in 0..count {
            workspace.reaction_source[node] =
                previous_decay * workspace.previous_species_mass[node];
        }
    }
    validate_solute_values("sequential reaction source", &workspace.reaction_source)?;

    let iteration_limit = if nonlinear { process.max_iterations } else { 1 };
    let mut converged = !nonlinear;
    let mut residual: f64 = 0.0;
    for iteration in 0..iteration_limit {
        assemble_flux_operator(workspace, mesh, input);
        let coordinates = mesh.coordinates();
        for node in 0..count {
            let width = control_volume_width(coordinates, node);
            let theta = input.accepted_water.water_contents[node];
            let density = process.bulk_density[node];
            let old_concentration = state.concentrations[offset + node];
            let old_sorbed = state.kinetic_sorbed[offset + node];
            let guess = workspace.trial_concentrations[offset + node];
            let (old_mass, _, _) = local_mass_terms(
                species_config,
                theta,
                density,
                old_concentration,
                old_sorbed,
                0.0,
            )?;
            let (guess_mass, derivative, intercept) = local_mass_terms(
                species_config,
                theta,
                density,
                guess,
                old_sorbed,
                input.timestep,
            )?;
            let gamma = 1.0 / input.timestep + decay;
            let source = workspace.reaction_source[node];
            workspace.diagonal[node] += width * gamma * derivative;
            if nonlinear {
                workspace.rhs[node] = width
                    * (old_mass / input.timestep
                        + source
                        + gamma * (derivative * guess - guess_mass));
            } else {
                workspace.rhs[node] =
                    width * (old_mass / input.timestep + source - gamma * intercept);
            }
        }
        apply_endpoint_boundary_rows(workspace, config.boundaries[species], input, count);
        validate_solute_values("reaction/sorption coefficients", &workspace.lower)?;
        validate_solute_values("reaction/sorption coefficients", &workspace.diagonal)?;
        validate_solute_values("reaction/sorption coefficients", &workspace.upper)?;
        validate_solute_values("reaction/sorption rhs", &workspace.rhs)?;
        solve_tridiagonal_in_place(
            &workspace.lower,
            &mut workspace.diagonal,
            &workspace.upper,
            &mut workspace.rhs,
            input.minimum_pivot,
        )?;
        validate_nonnegative_concentrations(&workspace.rhs)?;
        residual = 0.0;
        for node in 0..count {
            residual = residual
                .max((workspace.rhs[node] - workspace.trial_concentrations[offset + node]).abs());
        }
        workspace.trial_concentrations[offset..offset + count].copy_from_slice(&workspace.rhs);
        if !nonlinear || residual <= process.nonlinear_tolerance {
            converged = true;
            break;
        }
        if iteration + 1 == iteration_limit {
            return Err(SoluteStepError::NonConverged {
                species,
                iterations: iteration_limit,
                residual,
            });
        }
    }
    if !converged {
        return Err(SoluteStepError::NonConverged {
            species,
            iterations: iteration_limit,
            residual,
        });
    }

    let coordinates = mesh.coordinates();
    let mut reaction_mass_delta = 0.0;
    for node in 0..count {
        let concentration = workspace.trial_concentrations[offset + node];
        let old_sorbed = state.kinetic_sorbed[offset + node];
        let kinetic = species_config.kinetic_sorption;
        let new_sorbed = match kinetic {
            Some(parameters) => {
                let transfer = input.timestep * parameters.transfer_rate;
                (old_sorbed + transfer * parameters.capacity_coefficient * concentration)
                    / (1.0 + transfer)
            }
            None => old_sorbed,
        };
        if !new_sorbed.is_finite() || new_sorbed < 0.0 {
            return Err(invalid_transport(
                "trial kinetic sorbed pool",
                "must remain finite and nonnegative",
            ));
        }
        workspace.trial_kinetic_sorbed[offset + node] = new_sorbed;
        let (new_mass, _, _) = local_mass_terms(
            species_config,
            input.accepted_water.water_contents[node],
            process.bulk_density[node],
            concentration,
            old_sorbed,
            input.timestep,
        )?;
        workspace.previous_species_mass[node] = new_mass;
        let incoming = workspace.reaction_source[node];
        let outgoing = decay * new_mass;
        reaction_mass_delta +=
            (incoming - outgoing) * control_volume_width(coordinates, node) * input.timestep;
    }
    workspace.trial_reaction_mass_mut()[species] += reaction_mass_delta;
    if !workspace.trial_accounting().cumulative_reaction_mass()[species].is_finite() {
        return Err(invalid_transport(
            "reaction accounting",
            "result must remain finite",
        ));
    }
    Ok(())
}

fn assemble_flux_operator(
    workspace: &mut SoluteWorkspace,
    mesh: &Mesh1D,
    input: SoluteTransportInput<'_>,
) {
    let count = workspace.node_count;
    workspace.lower.fill(0.0);
    workspace.diagonal.fill(0.0);
    workspace.upper.fill(0.0);
    workspace.rhs.fill(0.0);
    let coordinates = mesh.coordinates();
    for face in 1..count {
        let left = face - 1;
        let right = face;
        let theta_face = (input.accepted_water.water_contents[left]
            + input.accepted_water.water_contents[right])
            / 2.0;
        let dispersion =
            theta_face * input.face_dispersion[left] / (coordinates[right] - coordinates[left]);
        let water_flux = input.accepted_water.face_fluxes[face];
        let left_coefficient = water_flux.max(0.0) + dispersion;
        let right_coefficient = water_flux.min(0.0) - dispersion;
        workspace.diagonal[left] += left_coefficient;
        workspace.upper[left] += right_coefficient;
        workspace.lower[right] -= left_coefficient;
        workspace.diagonal[right] -= right_coefficient;
    }
}

fn apply_endpoint_boundary_rows(
    workspace: &mut SoluteWorkspace,
    boundaries: SoluteBoundaries,
    input: SoluteTransportInput<'_>,
    node_count: usize,
) {
    match boundaries.lower {
        SoluteBoundaryMode::Concentration { value } => {
            workspace.diagonal[0] = 1.0;
            workspace.upper[0] = 0.0;
            workspace.rhs[0] = value;
        }
        SoluteBoundaryMode::TotalFlux { flux } => workspace.rhs[0] += flux,
        SoluteBoundaryMode::AdvectiveOutflow => {
            workspace.diagonal[0] -= input.accepted_water.face_fluxes[0];
        }
    }
    let upper = node_count - 1;
    match boundaries.upper {
        SoluteBoundaryMode::Concentration { value } => {
            workspace.lower[upper] = 0.0;
            workspace.diagonal[upper] = 1.0;
            workspace.rhs[upper] = value;
        }
        // The configured flux is positive into the domain. In +x coordinates
        // this is a negative flux at the upper endpoint, hence +flux on RHS.
        SoluteBoundaryMode::TotalFlux { flux } => workspace.rhs[upper] += flux,
        SoluteBoundaryMode::AdvectiveOutflow => {
            workspace.diagonal[upper] += input.accepted_water.face_fluxes[node_count];
        }
    }
}

/// Returns total local mass per soil volume, its concentration derivative, and
/// the affine intercept. The latter supports exact linear isotherm/kinetic rows.
fn local_mass_terms(
    species: &SoluteSpecies,
    water_content: f64,
    bulk_density: f64,
    concentration: f64,
    old_kinetic_sorbed: f64,
    timestep: f64,
) -> Result<(f64, f64, f64), SoluteStepError> {
    if concentration < 0.0 || !concentration.is_finite() {
        return Err(invalid_transport(
            "sorption concentration",
            "must be finite and nonnegative",
        ));
    }
    let (equilibrium_sorbed, equilibrium_derivative) =
        equilibrium_sorption_value(&species.equilibrium_sorption, concentration)?;
    let (kinetic_sorbed, kinetic_derivative) = match species.kinetic_sorption {
        Some(parameters) => {
            let transfer = timestep * parameters.transfer_rate;
            let denominator = 1.0 + transfer;
            (
                (old_kinetic_sorbed + transfer * parameters.capacity_coefficient * concentration)
                    / denominator,
                transfer * parameters.capacity_coefficient / denominator,
            )
        }
        None => (old_kinetic_sorbed, 0.0),
    };
    let mass = water_content * concentration + bulk_density * (equilibrium_sorbed + kinetic_sorbed);
    let derivative = water_content + bulk_density * (equilibrium_derivative + kinetic_derivative);
    let intercept = mass - derivative * concentration;
    if !mass.is_finite() || !derivative.is_finite() || !intercept.is_finite() || derivative <= 0.0 {
        return Err(invalid_transport(
            "sorption evaluation",
            "mass and positive derivative must remain finite",
        ));
    }
    Ok((mass, derivative, intercept))
}

fn equilibrium_sorption_value(
    isotherm: &EquilibriumSorption,
    concentration: f64,
) -> Result<(f64, f64), SoluteStepError> {
    let value = match *isotherm {
        EquilibriumSorption::None => (0.0, 0.0),
        EquilibriumSorption::Linear {
            distribution_coefficient,
        } => (
            distribution_coefficient * concentration,
            distribution_coefficient,
        ),
        EquilibriumSorption::Nonlinear {
            fraction,
            coefficient,
            saturation,
            exponent,
        } => {
            let power = concentration.powf(exponent);
            let denominator = 1.0 + saturation * power;
            if !power.is_finite() || !denominator.is_finite() || denominator <= 0.0 {
                return Err(invalid_transport(
                    "equilibrium sorption",
                    "power and denominator must remain finite and positive",
                ));
            }
            let sorbed = fraction * coefficient * power / denominator;
            let derivative = if concentration == 0.0 && exponent < 1.0 {
                f64::INFINITY
            } else if concentration == 0.0 && exponent > 1.0 {
                0.0
            } else {
                fraction * coefficient * exponent * concentration.powf(exponent - 1.0)
                    / (denominator * denominator)
            };
            (sorbed, derivative)
        }
    };
    if value.0.is_finite() && value.1.is_finite() {
        Ok(value)
    } else {
        Err(invalid_transport(
            "equilibrium sorption",
            "isotherm value and derivative must remain finite",
        ))
    }
}

fn control_volume_width(coordinates: &[f64], node: usize) -> f64 {
    if node == 0 {
        (coordinates[1] - coordinates[0]) / 2.0
    } else if node + 1 == coordinates.len() {
        (coordinates[node] - coordinates[node - 1]) / 2.0
    } else {
        (coordinates[node + 1] - coordinates[node - 1]) / 2.0
    }
}

fn validate_nonnegative_concentrations(values: &[f64]) -> Result<(), SoluteStepError> {
    validate_solute_values("solved concentrations", values)?;
    if values.iter().any(|value| *value < 0.0) {
        return Err(invalid_transport(
            "solved concentrations",
            "must be nonnegative; values are never clipped",
        ));
    }
    Ok(())
}

fn invalid_transport(name: &'static str, reason: &'static str) -> SoluteStepError {
    SoluteConfigError::Invalid { name, reason }.into()
}

#[cfg(test)]
mod transaction_tests {
    use super::*;

    fn config() -> SoluteConfig {
        SoluteConfig::new(
            SoluteUnits {
                concentration: "mg/L".into(),
                sorbed_mass: "mg/kg".into(),
            },
            vec![
                SoluteSpecies {
                    name: "parent".into(),
                    decay_rate: 0.1,
                    equilibrium_sorption: EquilibriumSorption::None,
                    kinetic_sorption: None,
                },
                SoluteSpecies {
                    name: "daughter".into(),
                    decay_rate: 0.0,
                    equilibrium_sorption: EquilibriumSorption::None,
                    kinetic_sorption: None,
                },
            ],
            SoluteBoundaries {
                upper: SoluteBoundaryMode::Concentration { value: 0.0 },
                lower: SoluteBoundaryMode::TotalFlux { flux: 0.0 },
            },
            1,
            &[],
        )
        .unwrap()
    }

    fn mesh() -> Mesh1D {
        Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap()
    }

    fn state() -> SoluteState {
        SoluteState::new(
            &config(),
            &mesh(),
            2.0,
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
        )
        .unwrap()
    }

    #[test]
    fn successful_trial_promotes_all_pools_time_and_accounting() {
        let mut accepted = state();
        let mut workspace = SoluteWorkspace::new(3, 2).unwrap();
        workspace.begin_trial(&accepted).unwrap();
        workspace.trial_concentrations_mut()[0] = 8.0;
        workspace.trial_kinetic_sorbed_mut()[5] = 1.5;
        workspace.trial_boundary_mass_mut()[0] = 2.25;
        workspace.trial_reaction_mass_mut()[1] = -0.75;

        workspace.accept_trial(&mut accepted, 2.5).unwrap();
        assert_eq!(accepted.time(), 2.5);
        assert_eq!(accepted.concentrations()[0], 8.0);
        assert_eq!(accepted.kinetic_sorbed()[5], 1.5);
        assert_eq!(accepted.accounting().accepted_steps(), 1);
        assert_eq!(accepted.accounting().cumulative_boundary_mass()[0], 2.25);
        assert_eq!(accepted.accounting().cumulative_reaction_mass()[1], -0.75);
    }

    #[test]
    fn rejected_trial_restores_every_pool_and_accounting_field_and_reuses_workspace() {
        let mut accepted = state();
        let original = accepted.clone();
        let mut workspace = SoluteWorkspace::new(3, 2).unwrap();
        workspace.begin_trial(&accepted).unwrap();
        workspace.trial_concentrations_mut().fill(99.0);
        workspace.trial_kinetic_sorbed_mut().fill(88.0);
        workspace.set_trial_accepted_steps(77);
        workspace.trial_boundary_mass_mut().fill(66.0);
        workspace.trial_reaction_mass_mut().fill(55.0);

        assert!(workspace.accept_trial(&mut accepted, f64::NAN).is_err());
        assert_eq!(accepted, original);
        workspace.reject_trial();
        assert_eq!(workspace.trial_time(), original.time());
        assert_eq!(workspace.trial_concentrations(), original.concentrations());
        assert_eq!(workspace.trial_kinetic_sorbed(), original.kinetic_sorbed());
        assert_eq!(workspace.trial_accounting(), original.accounting());

        workspace.begin_trial(&accepted).unwrap();
        workspace.trial_concentrations_mut()[1] = 10.0;
        workspace.accept_trial(&mut accepted, 3.0).unwrap();
        assert_eq!(accepted.time(), 3.0);
        assert_eq!(accepted.concentrations()[1], 10.0);
        assert_eq!(accepted.accounting().accepted_steps(), 1);
    }
}

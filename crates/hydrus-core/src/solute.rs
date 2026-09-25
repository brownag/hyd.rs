//! Validated contract for M5 single-domain solute transport.
//!
//! Concentration and sorbed-mass units are caller-declared labels. Callers must
//! supply mutually compatible units for concentrations, sorbed pools, rates,
//! and fluxes; this contract does not impose a universal physical basis.

use crate::{hydraulics::Mesh1D, SoluteConfigError};

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
    pub capacity_coefficient: f64,
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

/// Validated selected-M5 capabilities. This type defines configuration only; it does not step.
#[derive(Clone, Debug, PartialEq)]
pub struct SoluteConfig {
    units: SoluteUnits,
    species: Vec<SoluteSpecies>,
    boundaries: SoluteBoundaries,
}

impl SoluteConfig {
    pub fn new(
        units: SoluteUnits,
        species: Vec<SoluteSpecies>,
        boundaries: SoluteBoundaries,
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
        for mode in [boundaries.upper, boundaries.lower] {
            let value = match mode {
                SoluteBoundaryMode::Concentration { value } => value,
                SoluteBoundaryMode::TotalFlux { flux } => flux,
            };
            if !value.is_finite() {
                return Err(SoluteConfigError::Invalid {
                    name: "boundary",
                    reason: "value must be finite",
                });
            }
        }
        Ok(Self {
            units,
            species,
            boundaries,
        })
    }

    pub fn units(&self) -> &SoluteUnits {
        &self.units
    }
    pub fn species(&self) -> &[SoluteSpecies] {
        &self.species
    }
    pub fn boundaries(&self) -> SoluteBoundaries {
        self.boundaries
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
    pub fn trial_concentrations_mut(&mut self) -> &mut [f64] {
        &mut self.trial_concentrations
    }
    pub fn trial_kinetic_sorbed(&self) -> &[f64] {
        &self.trial_kinetic_sorbed
    }
    pub fn trial_kinetic_sorbed_mut(&mut self) -> &mut [f64] {
        &mut self.trial_kinetic_sorbed
    }
    pub fn trial_accounting(&self) -> &SoluteAccounting {
        &self.trial_accounting
    }
    pub fn trial_boundary_mass_mut(&mut self) -> &mut [f64] {
        &mut self.trial_accounting.cumulative_boundary_mass
    }
    pub fn trial_reaction_mass_mut(&mut self) -> &mut [f64] {
        &mut self.trial_accounting.cumulative_reaction_mass
    }
    pub fn set_trial_accepted_steps(&mut self, accepted_steps: u64) {
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
    )
}

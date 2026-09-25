//! Validated contract for M5 single-domain solute transport.
//!
//! Concentration and sorbed-mass units are caller-declared labels. Callers must
//! supply mutually compatible units for concentrations, sorbed pools, rates,
//! and fluxes; this contract does not impose a universal physical basis.

use crate::SoluteConfigError;

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

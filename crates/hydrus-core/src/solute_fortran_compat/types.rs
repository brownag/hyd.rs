//! Source-mapped compatibility data contracts. Values retain source precision explicitly.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DefaultReal(pub f32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DoubleReal(pub f64);

impl DefaultReal {
    pub fn from_input(value: f64) -> Self {
        Self(value as f32)
    }
    pub fn to_double(self) -> DoubleReal {
        DoubleReal(self.0 as f64)
    }
}

impl DoubleReal {
    /// Mirrors `sngl(F(i))` followed by the active nonnegative concentration clamp.
    pub fn accepted_concentration(self) -> DefaultReal {
        DefaultReal((self.0 as f32).max(0.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryForm {
    Prescribed,
    TypeThree,
    ZeroGradient,
    Atmospheric,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowOrientation {
    PositiveNative,
    NegativeNative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockingMode {
    Default,
    Type1,
    Type2,
    SourceNoOp3,
    Type4,
    Type5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SorptionMode {
    Equilibrium,
    MobileImmobile,
    DualDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSwitches {
    pub upstream: bool,
    pub artificial_dispersion: bool,
    pub tortuosity: bool,
    pub temperature: bool,
    pub vapor: bool,
    pub gas_terms: bool,
    pub equilibrium: bool,
    pub linear: bool,
    pub mobile_immobile: bool,
    pub dual_domain_model: Option<u8>,
    pub bacteria: bool,
    pub filtration: bool,
    pub blocking: BlockingMode,
    pub flux_concentration: bool,
    pub moisture_selector: u8,
    pub mass_initialization: bool,
    pub equilibrium_initialization: bool,
}

impl Default for SourceSwitches {
    fn default() -> Self {
        Self {
            upstream: false,
            artificial_dispersion: false,
            tortuosity: false,
            temperature: false,
            vapor: false,
            gas_terms: false,
            equilibrium: true,
            linear: true,
            mobile_immobile: false,
            dual_domain_model: None,
            bacteria: false,
            filtration: false,
            blocking: BlockingMode::Default,
            flux_concentration: false,
            moisture_selector: 0,
            mass_initialization: false,
            equilibrium_initialization: false,
        }
    }
}

impl SourceSwitches {
    pub fn validate(self, species_count: usize) -> Result<(), &'static str> {
        if self.bacteria && species_count > 1 {
            return Err("bacteria with a multi-species chain is unsupported");
        }
        if let Some(model) = self.dual_domain_model {
            if !matches!(model, 6 | 7) {
                return Err("dual-domain water model must be 6 or 7");
            }
            if self.mobile_immobile {
                return Err("mobile-immobile and dual-domain selectors conflict");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceIndices {
    /// For each canonical node, its one-based native Fortran node index.
    pub canonical_to_native: Vec<usize>,
}

impl SourceIndices {
    pub fn identity(nodes: usize) -> Self {
        Self {
            canonical_to_native: (1..=nodes).collect(),
        }
    }
    pub fn validate(&self, nodes: usize) -> Result<(), &'static str> {
        if self.canonical_to_native.len() != nodes {
            return Err("node permutation length mismatch");
        }
        let mut seen = vec![false; nodes];
        for &index in &self.canonical_to_native {
            if index == 0 || index > nodes || seen[index - 1] {
                return Err("node permutation must be a one-based bijection");
            }
            seen[index - 1] = true;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnavailableValue {
    pub availability: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Explicit<T> {
    Available(T),
    Unavailable(UnavailableValue),
}

impl<T> Explicit<T> {
    pub fn unavailable_reason(&self) -> Option<&str> {
        match self {
            Self::Available(_) => None,
            Self::Unavailable(u) => Some(&u.reason),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitValue<T> {
    pub units: String,
    pub value: Explicit<T>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoordinatesSpec {
    pub native: String,
    pub canonical: String,
    pub nodes: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransportInputs {
    pub temporal_weighting_epsi: Option<UnitValue<DefaultReal>>,
    pub time_interval: Option<UnitValue<DefaultReal>>,
    pub dispersivity: Option<UnitValue<DefaultReal>>,
    pub molecular_diffusion: Option<UnitValue<DefaultReal>>,
    pub upstream_weighting: Option<UnitValue<bool>>,
    pub upper_boundary_code: Option<UnitValue<i32>>,
    pub lower_boundary_code: Option<UnitValue<i32>>,
    pub flow_orientation: Option<UnitValue<FlowOrientation>>,
    pub atmospheric_concentration: Option<UnitValue<DefaultReal>>,
    pub atmospheric_surface_distance: Option<UnitValue<DefaultReal>>,
    pub flux_concentration: Option<UnitValue<bool>>,
    pub vapor_lower_boundary_exception: Option<UnitValue<bool>>,
    pub endpoint_state: Option<UnitValue<EndpointPrimitiveState>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EndpointPrimitiveState {
    pub coordinates: Vec<DefaultReal>,
    pub old_water: Vec<DefaultReal>,
    pub new_water: Vec<DefaultReal>,
    pub old_velocity: Vec<DefaultReal>,
    pub new_velocity: Vec<DefaultReal>,
    pub old_dispersion: Vec<DefaultReal>,
    pub new_dispersion: Vec<DefaultReal>,
    pub old_retardation: Vec<DefaultReal>,
    pub new_retardation: Vec<DefaultReal>,
    pub old_concentration: Vec<DefaultReal>,
    pub source_g0: Vec<DefaultReal>,
    pub source_g1: Vec<DefaultReal>,
    pub upwind_weight: Vec<DefaultReal>,
    pub epsi: DefaultReal,
    pub dt: DefaultReal,
    pub alf: DefaultReal,
    pub top_atmospheric_concentration: Option<DefaultReal>,
    pub top_surface_distance: Option<DefaultReal>,
    pub henry_coefficient: Option<DefaultReal>,
    pub gas_diffusion: Option<DefaultReal>,
    pub old_temperature: Option<DefaultReal>,
    pub new_temperature: Option<DefaultReal>,
    pub source_chpar: Option<Vec<DefaultReal>>,
    pub source_tdep: Option<Vec<DefaultReal>>,
    pub material_ids: Vec<usize>,
    pub source_species: usize,
    pub mobile_immobile: Vec<bool>,
    pub bacteria: bool,
    pub dual_porosity: u8,
    pub bottom_resistance: DefaultReal,
    pub bottom_prescribed: DefaultReal,
    pub top_prescribed: DefaultReal,
    pub old_accepted_concentration: Vec<DefaultReal>,
    pub new_accepted_concentration: Vec<DefaultReal>,
    pub old_accepted_flow: Vec<DefaultReal>,
    pub new_accepted_flow: Vec<DefaultReal>,
    pub t_level: i32,
    pub vapor: bool,
    pub r_min: DefaultReal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesRecord {
    pub name: String,
    pub position: usize,
    pub chain_rate_per_hour: DefaultReal,
    pub initial_concentration: DefaultReal,
    pub linear_equilibrium_kd: DefaultReal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReactionInputs {
    pub equilibrium_mode: Option<UnitValue<bool>>,
    pub fraction_equilibrium: Option<UnitValue<DefaultReal>>,
    pub immobile_water_content: Option<UnitValue<DefaultReal>>,
    pub initial_dissolved_concentration: Option<UnitValue<Vec<DefaultReal>>>,
    pub initial_equilibrium_sorbed_pool: Option<UnitValue<Vec<DefaultReal>>>,
    pub linearity_mode: Option<UnitValue<bool>>,
    pub observation_times: Option<UnitValue<Vec<DefaultReal>>>,
    pub ordered_species: Option<UnitValue<Vec<SpeciesRecord>>>,
    pub reaction_pattern: Option<UnitValue<String>>,
    pub secondary_sorbed_pool: Option<UnitValue<Vec<DefaultReal>>>,
    pub material_species_chpar: Option<UnitValue<Vec<MaterialSpeciesParameters>>>,
    pub moisture_dependency: Option<UnitValue<MoistureDependencyRequest>>,
    pub mass_initialization: Option<UnitValue<bool>>,
    pub equilibrium_initialization: Option<UnitValue<bool>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialSpeciesParameters {
    pub species: usize,
    pub material: usize,
    /// Source `ChPar` slots 1..20 for this species/material.
    pub slots: Vec<DefaultReal>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoistureDependencyRequest {
    pub selector: u8,
    pub table: Vec<DefaultReal>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransportPropertyInputs {
    pub coordinates: Option<UnitValue<CoordinatesSpec>>,
    pub temperature_history: Option<UnitValue<Vec<DefaultReal>>>,
    pub species_count: Option<UnitValue<usize>>,
    pub transport_switches: Option<UnitValue<SourceSwitches>>,
    pub material_bulk_density: Option<UnitValue<DefaultReal>>,
    pub initial_pressure_head: Option<UnitValue<DefaultReal>>,
    pub initial_pressure_head_profile: Option<UnitValue<Vec<DefaultReal>>>,
    pub initial_temperature_profile: Option<UnitValue<Vec<DefaultReal>>>,
    pub initial_gas_state: Option<UnitValue<String>>,
    pub vapor_state: Option<UnitValue<String>>,
    pub species_1_gas_diffusion: Option<UnitValue<DefaultReal>>,
    pub species_1_henry_coefficient: Option<UnitValue<DefaultReal>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DualDomainRequest {
    pub dynamic_exchange_claimed: bool,
    pub i_dual_por: u8,
    pub water_model: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NonequilibriumInputs {
    pub dual_domain: Option<UnitValue<DualDomainRequest>>,
    pub dynamic_exchange_sink: Option<UnitValue<Vec<DefaultReal>>>,
    pub initial_dual_immobile_water_content: Option<UnitValue<Vec<DefaultReal>>>,
    pub initial_water_content: Option<UnitValue<Vec<DefaultReal>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BiologyModeRequest {
    pub i_bact: u8,
    pub filtration: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BiologyParameters {
    pub blocking_i_psi: Option<u8>,
    pub grain_diameter: Option<DefaultReal>,
    pub particle_diameter: Option<DefaultReal>,
    pub attachment_capacity_1: Option<DefaultReal>,
    pub attachment_rate_1: Option<DefaultReal>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BiologyInputs {
    pub mode: Option<UnitValue<BiologyModeRequest>>,
    pub parameters: Option<UnitValue<BiologyParameters>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestFields {
    pub transport: TransportInputs,
    pub reactions: ReactionInputs,
    pub transport_properties: TransportPropertyInputs,
    pub nonequilibrium: NonequilibriumInputs,
    pub biology: BiologyInputs,
}

impl Default for RequestFields {
    fn default() -> Self {
        Self {
            transport: TransportInputs {
                temporal_weighting_epsi: None,
                time_interval: None,
                dispersivity: None,
                molecular_diffusion: None,
                upstream_weighting: None,
                upper_boundary_code: None,
                lower_boundary_code: None,
                flow_orientation: None,
                atmospheric_concentration: None,
                atmospheric_surface_distance: None,
                flux_concentration: None,
                vapor_lower_boundary_exception: None,
                endpoint_state: None,
            },
            reactions: ReactionInputs {
                equilibrium_mode: None,
                fraction_equilibrium: None,
                immobile_water_content: None,
                initial_dissolved_concentration: None,
                initial_equilibrium_sorbed_pool: None,
                linearity_mode: None,
                observation_times: None,
                ordered_species: None,
                reaction_pattern: None,
                secondary_sorbed_pool: None,
                material_species_chpar: None,
                moisture_dependency: None,
                mass_initialization: None,
                equilibrium_initialization: None,
            },
            transport_properties: TransportPropertyInputs {
                coordinates: None,
                temperature_history: None,
                species_count: None,
                transport_switches: None,
                material_bulk_density: None,
                initial_pressure_head: None,
                initial_pressure_head_profile: None,
                initial_temperature_profile: None,
                initial_gas_state: None,
                vapor_state: None,
                species_1_gas_diffusion: None,
                species_1_henry_coefficient: None,
            },
            nonequilibrium: NonequilibriumInputs {
                dual_domain: None,
                dynamic_exchange_sink: None,
                initial_dual_immobile_water_content: None,
                initial_water_content: None,
            },
            biology: BiologyInputs {
                mode: None,
                parameters: None,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesInput {
    pub name: String,
    pub chpar: Vec<DefaultReal>,
    pub initial_concentration: Vec<DefaultReal>,
    pub sorb: Explicit<Vec<DefaultReal>>,
    pub sorb2: Explicit<Vec<DefaultReal>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompatibilityInput {
    pub schema_version: String,
    pub request_sha256: String,
    pub input_sha256: Vec<(String, String)>,
    pub units: Vec<(String, String)>,
    pub fields: RequestFields,
    pub coordinates: Explicit<Vec<DefaultReal>>,
    pub material_ids: Explicit<Vec<usize>>,
    pub species: Vec<SpeciesInput>,
    pub old_water: Explicit<Vec<DefaultReal>>,
    pub new_water: Explicit<Vec<DefaultReal>>,
    pub temperature: Explicit<Vec<DefaultReal>>,
    pub dual_immobile_water: Explicit<Vec<DefaultReal>>,
    pub switches: SourceSwitches,
    pub upper_boundary: Option<BoundaryForm>,
    pub lower_boundary: Option<BoundaryForm>,
    pub source_indices: Explicit<SourceIndices>,
}

impl CompatibilityInput {
    pub fn validate_dimensions(&self) -> Result<(), &'static str> {
        let species_count = self.species.len().max(
            self.fields
                .transport_properties
                .species_count
                .as_ref()
                .and_then(|v| match &v.value {
                    Explicit::Available(n) => Some(*n),
                    _ => None,
                })
                .unwrap_or(0),
        );
        self.switches.validate(species_count)?;
        if let Explicit::Available(indices) = &self.source_indices {
            if let Some(nodes) = self
                .fields
                .transport_properties
                .coordinates
                .as_ref()
                .and_then(|v| match &v.value {
                    Explicit::Available(c) => Some(c.nodes),
                    Explicit::Unavailable(_) => None,
                })
            {
                indices.validate(nodes)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompatibilityState {
    pub time: DefaultReal,
    pub concentration: Vec<Vec<DefaultReal>>,
    pub sorb: Vec<Explicit<Vec<DefaultReal>>>,
    pub sorb2: Vec<Explicit<Vec<DefaultReal>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompatibilityWorkspace {
    pub lower: Vec<DoubleReal>,
    pub diagonal: Vec<DoubleReal>,
    pub upper: Vec<DoubleReal>,
    pub rhs: Vec<DoubleReal>,
    pub source_terms: Vec<Vec<DefaultReal>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraceObservation {
    pub species: usize,
    pub native_node: Option<usize>,
    pub checkpoint: String,
    pub default_real: Option<DefaultReal>,
    pub double_real: Option<DoubleReal>,
    pub unavailable_reason: Option<String>,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    #[test]
    fn precision_boundaries_match_source_kinds() {
        let x = DefaultReal::from_input(1.0 / 3.0);
        assert_eq!(x.0, (1.0_f64 / 3.0) as f32);
        assert_eq!(DoubleReal(-2.0).accepted_concentration(), DefaultReal(0.0));
        assert_eq!(
            DoubleReal(1.0 / 3.0).accepted_concentration().0,
            (1.0_f64 / 3.0) as f32
        );
    }
    #[test]
    fn source_indices_are_one_based_bijection() {
        assert!(SourceIndices {
            canonical_to_native: vec![3, 2, 1]
        }
        .validate(3)
        .is_ok());
        assert!(SourceIndices {
            canonical_to_native: vec![1, 1]
        }
        .validate(2)
        .is_err());
    }
    #[test]
    fn switches_guard_unreachable_and_invalid_modes() {
        let switches = SourceSwitches {
            upstream: true,
            artificial_dispersion: true,
            ..SourceSwitches::default()
        };
        assert!(switches.validate(2).is_ok());
        assert!(SourceSwitches {
            linear: false,
            ..switches
        }
        .validate(1)
        .is_ok());
        assert!(SourceSwitches {
            dual_domain_model: Some(8),
            ..switches
        }
        .validate(1)
        .is_err());
        assert!(SourceSwitches {
            dual_domain_model: Some(6),
            mobile_immobile: true,
            ..switches
        }
        .validate(1)
        .is_err());
    }
    #[test]
    fn compatibility_input_preserves_unavailable_fields_and_orientation() {
        let unavailable_usize = Explicit::Unavailable(UnavailableValue {
            availability: "inactive".into(),
            reason: "source call is commented".into(),
        });
        let unavailable_real = Explicit::Unavailable(UnavailableValue {
            availability: "inactive".into(),
            reason: "source call is commented".into(),
        });
        let input = CompatibilityInput {
            schema_version: "solute-request-v1".into(),
            request_sha256: "0".repeat(64),
            input_sha256: vec![],
            units: vec![],
            fields: RequestFields::default(),
            coordinates: Explicit::Available(vec![DefaultReal(0.0), DefaultReal(1.0)]),
            material_ids: unavailable_usize,
            species: vec![],
            old_water: unavailable_real.clone(),
            new_water: unavailable_real.clone(),
            temperature: unavailable_real.clone(),
            dual_immobile_water: unavailable_real,
            switches: SourceSwitches::default(),
            upper_boundary: Some(BoundaryForm::Atmospheric),
            lower_boundary: Some(BoundaryForm::ZeroGradient),
            source_indices: Explicit::Available(SourceIndices::identity(2)),
        };
        assert!(input.validate_dimensions().is_ok());
        assert_eq!(
            input.source_indices,
            Explicit::Available(SourceIndices {
                canonical_to_native: vec![1, 2]
            })
        );
    }
}

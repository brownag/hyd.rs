//! Strict serialized fixture request parsing and canonical hash verification.
use super::types::*;
use super::{boundary, dual_domain};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub(crate) const NORMALIZED_SCHEMA: &str = "solute-normalized-request-v1";
pub(crate) const LEGACY_SCHEMA: &str = "solute-request-v1";

mod biology;
mod fields;
mod json_guard;
mod nonequilibrium;
mod reactions;
mod source_coupling;
#[cfg(test)]
pub(crate) mod tests;
mod transport;
mod transport_properties;

use biology::*;
use fields::*;
use json_guard::{reject_duplicate_keys, validate_field};
use nonequilibrium::*;
use reactions::*;
use source_coupling::*;
use transport::*;
use transport_properties::*;

#[derive(Clone, Debug)]
pub(crate) struct ParsedRequest {
    pub schema_version: String,
    pub request_sha256: String,
    pub input: CompatibilityInput,
}

pub(crate) fn canonical_hash(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("JSON value serialization is infallible");
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn parse_case_manifest(json: &str) -> Result<ParsedRequest, String> {
    reject_duplicate_keys(json)?;
    let root: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let expected_hash = root
        .get("request_sha256")
        .and_then(Value::as_str)
        .ok_or("missing request_sha256")?;
    let input_sha256 = parse_input_sha256(root.get("input_files"))?;
    parse_root_value(&root, expected_hash, input_sha256)
}

pub(crate) fn parse_request(json: &str, expected_hash: &str) -> Result<ParsedRequest, String> {
    reject_duplicate_keys(json)?;
    let root: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    parse_root_value(&root, expected_hash, vec![])
}

fn parse_root_value(
    root: &Value,
    expected_hash: &str,
    input_sha256: Vec<(String, String)>,
) -> Result<ParsedRequest, String> {
    let request = root.get("request").ok_or("missing request")?;
    let schema = request
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or("missing request schema_version")?;
    if schema != NORMALIZED_SCHEMA && schema != LEGACY_SCHEMA {
        return Err("unsupported normalized request schema".into());
    }
    if !is_sha256(expected_hash) {
        return Err("request SHA-256 must be 64 hexadecimal characters".into());
    }
    let actual = canonical_hash(request);
    if !actual.eq_ignore_ascii_case(expected_hash) {
        return Err(format!(
            "normalized request hash mismatch: expected {expected_hash}, computed {actual}"
        ));
    }
    let fields = request
        .get("fields")
        .and_then(Value::as_object)
        .ok_or("request fields must be an object")?;
    if fields.is_empty() {
        return Err("request fields must not be empty".into());
    }
    let input = map_fields(schema, expected_hash, input_sha256, fields)?;
    Ok(ParsedRequest {
        schema_version: schema.into(),
        request_sha256: expected_hash.into(),
        input,
    })
}

fn parse_input_sha256(value: Option<&Value>) -> Result<Vec<(String, String)>, String> {
    let Some(value) = value else {
        return Err("input_files must be a nonempty object".into());
    };
    let obj = value.as_object().ok_or("input_files must be an object")?;
    if obj.is_empty() {
        return Err("input_files must be a nonempty object".into());
    }
    let mut out = Vec::new();
    for (key, value) in obj {
        if key.trim().is_empty() {
            return Err("input_files keys must be nonempty".into());
        }
        let hash = value
            .as_str()
            .ok_or_else(|| format!("input_files.{key} must be a SHA-256 string"))?;
        if !is_sha256(hash) {
            return Err(format!("input_files.{key} must be a 64-character SHA-256"));
        }
        out.push((key.clone(), hash.to_string()));
    }
    Ok(out)
}

fn map_fields(
    schema: &str,
    hash: &str,
    input_sha256: Vec<(String, String)>,
    fields: &Map<String, Value>,
) -> Result<CompatibilityInput, String> {
    let allowed = [
        "coordinates",
        "temperature_history",
        "species_count",
        "transport_switches",
        "material_bulk_density",
        "initial_gas_state",
        "initial_pressure_head_profile",
        "initial_pressure_head",
        "initial_temperature_profile",
        "initial_water_content",
        "temporal_weighting.epsi",
        "time.interval",
        "transport.dispersivity",
        "transport.molecular_diffusion",
        "transport.upstream_weighting",
        "boundary.flow_orientation",
        "boundary.lower.code",
        "boundary.upper.code",
        "boundary.upper.atmospheric_concentration",
        "boundary.upper.atmospheric_surface_distance",
        "solute.flux_concentration",
        "endpoint_primitives",
        "equilibrium_mode",
        "fraction_equilibrium",
        "immobile_water_content",
        "initial_dissolved_concentration",
        "initial_dual_immobile_water_content",
        "initial_equilibrium_sorbed_pool",
        "linearity_mode",
        "observation_times",
        "ordered_species",
        "reaction_pattern",
        "secondary_sorbed_pool",
        "material_species_chpar",
        "moisture_dependency",
        "initialization.mass",
        "initialization.equilibrium",
        "dual_domain",
        "dynamic_exchange_sink",
        "dual_porosity_exchange_state",
        "biology_mode",
        "biology_parameters",
        "control_switches",
        "root_uptake_state",
        "size_exclusion_state",
        "species.1.gas_diffusion",
        "species.1.henry_coefficient",
        "vapor_state",
    ];
    for (key, value) in fields {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unmapped normalized request field: {key}"));
        }
        validate_field(key, value)?;
    }

    let mut request_fields = RequestFields::default();
    let mut units = Vec::new();
    for (key, value) in fields {
        units.push((key.clone(), unit(value)?.to_string()));
    }

    request_fields.transport.temporal_weighting_epsi =
        maybe_number(fields, "temporal_weighting.epsi", "1")?;
    request_fields.transport.time_interval = maybe_number(fields, "time.interval", "hour")?;
    request_fields.transport.dispersivity = maybe_number(fields, "transport.dispersivity", "cm")?;
    request_fields.transport.molecular_diffusion =
        maybe_number(fields, "transport.molecular_diffusion", "cm2/hour")?;
    request_fields.transport.upstream_weighting =
        maybe_bool(fields, "transport.upstream_weighting", "boolean")?;
    request_fields.transport.upper_boundary_code =
        maybe_i32(fields, "boundary.upper.code", "source integer code")?;
    request_fields.transport.lower_boundary_code =
        maybe_i32(fields, "boundary.lower.code", "source integer code")?;
    request_fields.transport.flow_orientation =
        maybe_orientation(fields, "boundary.flow_orientation")?;
    request_fields.transport.atmospheric_concentration = maybe_number(
        fields,
        "boundary.upper.atmospheric_concentration",
        "normalized concentration",
    )?;
    request_fields.transport.atmospheric_surface_distance =
        maybe_number(fields, "boundary.upper.atmospheric_surface_distance", "cm")?;
    request_fields.transport.flux_concentration =
        maybe_bool(fields, "solute.flux_concentration", "boolean")?;
    request_fields.transport.endpoint_state = maybe_endpoint_state(fields)?;

    request_fields.reactions.equilibrium_mode = maybe_bool(fields, "equilibrium_mode", "boolean")?;
    request_fields.reactions.fraction_equilibrium =
        maybe_number(fields, "fraction_equilibrium", "1")?;
    request_fields.reactions.immobile_water_content =
        maybe_number(fields, "immobile_water_content", "cm3/cm3")?;
    request_fields.reactions.initial_dissolved_concentration =
        maybe_number_vec(fields, "initial_dissolved_concentration", "C_ref")?;
    request_fields.reactions.initial_equilibrium_sorbed_pool = maybe_number_vec(
        fields,
        "initial_equilibrium_sorbed_pool",
        "source Sorb units",
    )?;
    request_fields.reactions.linearity_mode = maybe_bool(fields, "linearity_mode", "boolean")?;
    request_fields.reactions.observation_times =
        maybe_number_vec(fields, "observation_times", "hour")?;
    request_fields.reactions.ordered_species = maybe_species(fields)?;
    request_fields.reactions.reaction_pattern =
        maybe_string(fields, "reaction_pattern", "source branch identity")?;
    request_fields.reactions.secondary_sorbed_pool =
        maybe_number_vec(fields, "secondary_sorbed_pool", "source Sorb2 units")?;
    request_fields.reactions.material_species_chpar = maybe_chpar(fields)?;
    request_fields.reactions.moisture_dependency = maybe_moisture_dependency(fields)?;
    request_fields.reactions.mass_initialization =
        maybe_bool(fields, "initialization.mass", "boolean")?;
    request_fields.reactions.equilibrium_initialization =
        maybe_bool(fields, "initialization.equilibrium", "boolean")?;

    request_fields.transport_properties.coordinates = maybe_coordinates(fields)?;
    request_fields.transport_properties.temperature_history =
        maybe_number_vec(fields, "temperature_history", "degC")?;
    request_fields.transport_properties.species_count =
        maybe_usize(fields, "species_count", "count")?;
    request_fields.transport_properties.transport_switches = maybe_transport_switches(fields)?;
    request_fields.transport_properties.material_bulk_density =
        maybe_number(fields, "material_bulk_density", "g/cm3")?;
    request_fields.transport_properties.initial_pressure_head =
        maybe_number(fields, "initial_pressure_head", "cm")?;
    request_fields
        .transport_properties
        .initial_pressure_head_profile =
        maybe_number_vec(fields, "initial_pressure_head_profile", "cm")?;
    request_fields
        .transport_properties
        .initial_temperature_profile =
        maybe_number_vec(fields, "initial_temperature_profile", "degC")?;
    request_fields.transport_properties.initial_gas_state =
        maybe_string(fields, "initial_gas_state", "source-derived")?;
    request_fields.transport_properties.vapor_state =
        maybe_string(fields, "vapor_state", "source-derived state")?;
    request_fields.transport_properties.species_1_gas_diffusion =
        maybe_number(fields, "species.1.gas_diffusion", "cm2/hour")?;
    request_fields
        .transport_properties
        .species_1_henry_coefficient = maybe_number(
        fields,
        "species.1.henry_coefficient",
        "source Henry coefficient",
    )?;

    request_fields.nonequilibrium.dual_domain = maybe_dual_domain(fields)?;
    request_fields.nonequilibrium.dynamic_exchange_sink =
        maybe_number_vec(fields, "dynamic_exchange_sink", "water content/time")?;
    request_fields
        .nonequilibrium
        .initial_dual_immobile_water_content =
        maybe_number_vec(fields, "initial_dual_immobile_water_content", "cm3/cm3")?;
    request_fields.nonequilibrium.dual_porosity_exchange_state =
        maybe_dual_porosity_exchange_state(fields)?;
    request_fields.nonequilibrium.initial_water_content =
        maybe_number_vec(fields, "initial_water_content", "cm3/cm3")?;

    request_fields.biology.mode = maybe_biology_mode(fields)?;
    request_fields.biology.parameters = maybe_biology_parameters(fields)?;

    // `SinkF` and `lWat` are read once, here, from the control-section state and
    // merged into the one place they live (`SourceSwitches`); the two kernels take
    // them as parameters, never through a bundle.
    request_fields.source_coupling.control_switches = maybe_control_switches(fields)?;
    request_fields.source_coupling.root_uptake_state = maybe_root_uptake_state(fields)?;
    request_fields.source_coupling.size_exclusion_state = maybe_size_exclusion_state(fields)?;

    validate_cross_field_dimensions(&request_fields)?;
    let switches = derive_switches(&request_fields)?;
    validate_required_mode_inputs(&request_fields, &switches)?;
    let species = derive_species(&request_fields);
    switches
        .validate(species.len().max(species_count(&request_fields)))
        .map_err(str::to_string)?;

    if request_fields.transport.flow_orientation.is_none() {
        request_fields.transport.flow_orientation =
            derive_coordinate_orientation(&request_fields.transport_properties.coordinates)?;
    }
    let coordinates = unavailable(
        "metadata only",
        "coordinates field stores native/canonical ranges and node count, not a physical coordinate array",
    );
    let source_indices = derive_source_indices(&request_fields)?;
    let temperature = explicit_from_option_vec(
        &request_fields.transport_properties.temperature_history,
        "not present",
        "temperature history is absent from this fixture request",
    );
    let dual_immobile_water = explicit_from_option_vec(
        &request_fields
            .nonequilibrium
            .initial_dual_immobile_water_content,
        "not explicit",
        "dual immobile water is unavailable or initialized by active InitDualPor",
    );

    let input = CompatibilityInput {
        schema_version: schema.to_string(),
        request_sha256: hash.to_string(),
        input_sha256,
        units,
        fields: request_fields,
        coordinates,
        material_ids: unavailable(
            "not present",
            "material-id profile is not serialized in Wave 3 CASE requests",
        ),
        species,
        old_water: unavailable(
            "not present",
            "old water content profile is not serialized as adapter forcing",
        ),
        new_water: unavailable(
            "not present",
            "new water content profile is not serialized as adapter forcing",
        ),
        temperature,
        dual_immobile_water,
        switches,
        upper_boundary: boundary_form(fields.get("boundary.upper.code"), true)?,
        lower_boundary: boundary_form(fields.get("boundary.lower.code"), false)?,
        source_indices,
    };
    input.validate_dimensions().map_err(str::to_string)?;
    Ok(input)
}

fn derive_coordinate_orientation(
    coordinates: &Option<UnitValue<CoordinatesSpec>>,
) -> Result<Option<UnitValue<FlowOrientation>>, String> {
    let Some(UnitValue {
        value: Explicit::Available(spec),
        ..
    }) = coordinates
    else {
        return Ok(None);
    };
    let native_negative = spec.native.contains("..-") || spec.native.contains("=-0..");
    let native_positive = spec.native.contains("..+") || spec.native.contains("=0..");
    let orientation = match (native_negative, native_positive) {
        (true, false) => FlowOrientation::NegativeNative,
        (false, true) => FlowOrientation::PositiveNative,
        _ => {
            return Err(
                "coordinates orientation cannot be derived from native/canonical metadata".into(),
            )
        }
    };
    Ok(Some(UnitValue {
        units: "orientation".into(),
        value: Explicit::Available(orientation),
    }))
}

fn derive_source_indices(fields: &RequestFields) -> Result<Explicit<SourceIndices>, String> {
    let Some(coordinates) = &fields.transport_properties.coordinates else {
        return Ok(unavailable(
            "metadata unavailable",
            "source node permutation requires coordinates node count and flow orientation metadata",
        ));
    };
    let Explicit::Available(spec) = &coordinates.value else {
        return Ok(unavailable(
            "metadata unavailable",
            "source node permutation requires an available coordinates node count",
        ));
    };
    let Some(orientation) = &fields.transport.flow_orientation else {
        return Ok(unavailable(
            "metadata unavailable",
            "source node permutation requires flow orientation metadata",
        ));
    };
    if matches!(orientation.value, Explicit::Unavailable(_)) {
        return Ok(unavailable(
            "metadata unavailable",
            "source node permutation requires an available flow orientation",
        ));
    }
    let indices = SourceIndices::identity(spec.nodes);
    indices.validate(spec.nodes).map_err(str::to_string)?;
    Ok(Explicit::Available(indices))
}

fn validate_cross_field_dimensions(fields: &RequestFields) -> Result<(), String> {
    reactions::validate_ordered_species(fields)?;
    transport_properties::validate_dimensions(fields)?;
    nonequilibrium::validate_dual_domain_dimensions(fields)?;
    reactions::validate_material_species_chpar(fields)?;
    transport::validate_endpoint_state(fields)?;
    Ok(())
}

fn validate_required_mode_inputs(
    fields: &RequestFields,
    switches: &SourceSwitches,
) -> Result<(), String> {
    if switches.flux_concentration && fields.transport.time_interval.is_none() {
        return Err("FluxConc requires the source time interval".into());
    }
    if switches.moisture_selector == 1 && fields.reactions.moisture_dependency.is_none() {
        return Err("moisture selector 1 requires explicit WDep input".into());
    }
    if (switches.mass_initialization || switches.equilibrium_initialization)
        && fields.reactions.initial_dissolved_concentration.is_none()
        && fields.reactions.ordered_species.is_none()
    {
        return Err("selected initialization requires initial dissolved concentrations".into());
    }
    if fields.transport.endpoint_state.is_some()
        && (fields.transport.lower_boundary_code.is_none()
            || fields.transport.upper_boundary_code.is_none())
    {
        return Err("endpoint_primitives require both source boundary codes".into());
    }
    source_coupling::validate_source_coupling_guards(fields, switches)?;
    Ok(())
}

fn derive_switches(fields: &RequestFields) -> Result<SourceSwitches, String> {
    let mut switches = SourceSwitches::default();
    if let Some(v) = &fields.transport.upstream_weighting {
        if let Explicit::Available(x) = v.value {
            switches.upstream = x;
        }
    }
    if let Some(v) = &fields.transport_properties.transport_switches {
        if let Explicit::Available(x) = v.value {
            switches.upstream |= x.upstream;
            switches.artificial_dispersion = x.artificial_dispersion;
            switches.tortuosity = x.tortuosity;
            switches.temperature = x.temperature;
            switches.vapor = x.vapor;
            switches.gas_terms = x.gas_terms;
        }
    }
    if let Some(v) = &fields.reactions.equilibrium_mode {
        if let Explicit::Available(x) = v.value {
            switches.equilibrium = x;
        }
    }
    if let Some(v) = &fields.reactions.linearity_mode {
        if let Explicit::Available(x) = v.value {
            switches.linear = x;
        }
    }
    if let Some(chpar) = &fields.reactions.material_species_chpar {
        if let Explicit::Available(records) = &chpar.value {
            let species_count = species_count_value(fields).unwrap_or(0);
            let material_count = records.iter().map(|r| r.material).max().unwrap_or(0);
            let bacteria = fields
                .biology
                .mode
                .as_ref()
                .is_some_and(|v| matches!(&v.value, Explicit::Available(m) if m.i_bact != 0));
            let mut linear = vec![true; species_count];
            let mut has_record = vec![false; species_count];
            let mut seen = HashSet::new();
            for record in records {
                if record.species == 0
                    || record.species > species_count
                    || record.material == 0
                    || record.slots.len() != 20
                {
                    return Err(
                        "material_species_chpar requires species in range and 20 source slots"
                            .into(),
                    );
                }
                if !seen.insert((record.species, record.material)) {
                    return Err("duplicate material_species_chpar pair".into());
                }
                let idx = record.species - 1;
                has_record[idx] = true;
                if record.slots[7].0.abs() > 1.0e-12 || (record.slots[8].0 - 1.0).abs() > 0.001 {
                    linear[idx] = false;
                }
                if bacteria && (record.slots[17].0 > 0.0 || record.slots[14].0 > 0.0) {
                    linear[idx] = false;
                }
            }
            let expected_pairs = species_count.saturating_mul(material_count);
            if seen.len() != expected_pairs {
                return Err("material_species_chpar must cover every species/material pair".into());
            }
            if has_record.into_iter().all(|x| x) {
                switches.linear = linear.into_iter().all(|x| x);
            } else {
                return Err("material_species_chpar must cover every species".into());
            }
        }
    }
    if let Some(v) = &fields.transport.flux_concentration {
        if let Explicit::Available(enabled) = v.value {
            switches.flux_concentration = enabled;
        }
    }
    if let Some(v) = &fields.reactions.moisture_dependency {
        if let Explicit::Available(dep) = &v.value {
            switches.moisture_selector = dep.selector;
        }
    }
    if let Some(v) = &fields.reactions.mass_initialization {
        if let Explicit::Available(enabled) = v.value {
            switches.mass_initialization = enabled;
        }
    }
    if let Some(v) = &fields.reactions.equilibrium_initialization {
        if let Explicit::Available(enabled) = v.value {
            switches.equilibrium_initialization = enabled;
        }
    }
    if let Some(dep) = &fields.reactions.moisture_dependency {
        if let Explicit::Available(dep) = &dep.value {
            if dep.selector == 1 && dep.table.is_empty() {
                return Err("selector 1 requires a WDep table".into());
            }
        }
    }
    if let Some(v) = &fields.nonequilibrium.dual_domain {
        if let Explicit::Available(x) = &v.value {
            if x.i_dual_por != 0 {
                switches.dual_domain_model = Some(x.water_model);
                switches.mobile_immobile = false;
            }
        }
    }
    if let Some(v) = &fields.biology.mode {
        if let Explicit::Available(x) = &v.value {
            switches.bacteria = x.i_bact != 0;
            switches.filtration = x.filtration;
        }
    }
    let dual_domain_active = switches.dual_domain_model.is_some();
    if !switches.bacteria && !dual_domain_active {
        if let Some(v) = &fields.reactions.immobile_water_content {
            if let Explicit::Available(x) = v.value {
                switches.mobile_immobile = x.0 > 0.0;
            }
        }
    }
    if let Some(v) = &fields.biology.parameters {
        if let Explicit::Available(x) = &v.value {
            switches.blocking = match x.blocking_i_psi.unwrap_or(0) {
                0 => BlockingMode::Default,
                1 => BlockingMode::Type1,
                2 => BlockingMode::Type2,
                3 => BlockingMode::Type3,
                4 => BlockingMode::Type4,
                5 => BlockingMode::Type5,
                _ => return Err("blocking_iPsi must be 1..5 when present".into()),
            };
        }
    }
    // The one place `SinkF` and `lWat` enter the port, and the only place they are
    // stored: `SourceSwitches::root_sink` and `SourceSwitches::water_flow`. The two
    // kernels receive them as arguments instead of carrying copies in their bundles.
    if let Some(v) = &fields.source_coupling.control_switches {
        if let Explicit::Available(x) = &v.value {
            switches.water_flow = x.water_flow;
            switches.root_sink = x.root_sink;
        }
    }
    Ok(switches)
}

fn derive_species(fields: &RequestFields) -> Vec<SpeciesInput> {
    let Some(UnitValue {
        value: Explicit::Available(species),
        ..
    }) = &fields.reactions.ordered_species
    else {
        return vec![];
    };
    species
        .iter()
        .map(|s| SpeciesInput {
            name: s.name.clone(),
            chpar: vec![s.chain_rate_per_hour, s.linear_equilibrium_kd],
            initial_concentration: vec![s.initial_concentration],
            sorb: explicit_from_option_vec(
                &fields.reactions.initial_equilibrium_sorbed_pool,
                "derived",
                "initial equilibrium sorbed pool is not an explicit input column",
            ),
            sorb2: explicit_from_option_vec(
                &fields.reactions.secondary_sorbed_pool,
                "inactive",
                "secondary sorbed pool is inactive for this request",
            ),
        })
        .collect()
}

fn species_count(fields: &RequestFields) -> usize {
    species_count_value(fields).unwrap_or(0)
}

pub(super) fn species_count_value(fields: &RequestFields) -> Option<usize> {
    fields
        .transport_properties
        .species_count
        .as_ref()
        .and_then(|v| match &v.value {
            Explicit::Available(n) => Some(*n),
            _ => None,
        })
}

pub(super) fn available_len(v: &Option<UnitValue<Vec<DefaultReal>>>) -> Option<usize> {
    v.as_ref().and_then(|v| match &v.value {
        Explicit::Available(x) => Some(x.len()),
        _ => None,
    })
}

pub(super) fn explicit_from_option_vec(
    v: &Option<UnitValue<Vec<DefaultReal>>>,
    availability: &str,
    reason: &str,
) -> Explicit<Vec<DefaultReal>> {
    v.as_ref()
        .map(|v| v.value.clone())
        .unwrap_or_else(|| unavailable(availability, reason))
}

pub(super) fn unavailable<T>(availability: &str, reason: &str) -> Explicit<T> {
    Explicit::Unavailable(UnavailableValue {
        availability: availability.into(),
        reason: reason.into(),
    })
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
